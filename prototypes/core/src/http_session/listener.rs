use std::{
    io,
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::net::{TcpListener, TcpSocket, TcpStream};

pub(super) fn bind(address: SocketAddr) -> io::Result<TcpListener> {
    let socket = if address.is_ipv4() {
        TcpSocket::new_v4()?
    } else {
        TcpSocket::new_v6()?
    };
    #[cfg(unix)]
    socket.set_reuseaddr(true)?;
    socket.bind(address)?;
    socket.listen(128)
}

pub(super) struct RecoveringListener {
    listener: Option<TcpListener>,
    address: SocketAddr,
    recovering: Arc<AtomicBool>,
}

impl RecoveringListener {
    pub fn new(listener: TcpListener, recovering: Arc<AtomicBool>) -> io::Result<Self> {
        Ok(Self {
            address: listener.local_addr()?,
            listener: Some(listener),
            recovering,
        })
    }
}

impl axum::serve::Listener for RecoveringListener {
    type Io = TcpStream;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> (TcpStream, SocketAddr) {
        loop {
            if self.listener.is_none() {
                match bind(self.address) {
                    Ok(listener) => {
                        self.listener = Some(listener);
                        self.recovering.store(false, Ordering::SeqCst);
                    }
                    Err(_) => {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        continue;
                    }
                }
            }
            let listener = self.listener.as_ref().expect("listener was bound above");
            match listener.accept().await {
                Ok(connection) => return connection,
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::InvalidInput | io::ErrorKind::NotConnected
                    ) =>
                {
                    self.recovering.store(true, Ordering::SeqCst);
                    self.listener = None;
                }
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::ConnectionRefused
                            | io::ErrorKind::ConnectionAborted
                            | io::ErrorKind::ConnectionReset
                    ) => {}
                Err(_) => tokio::time::sleep(Duration::from_secs(1)).await,
            }
        }
    }

    fn local_addr(&self) -> io::Result<SocketAddr> {
        Ok(self.address)
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use axum::serve::Listener;
    use std::net::Shutdown;

    async fn wait_for_recovery(recovering: &AtomicBool) {
        tokio::time::timeout(Duration::from_secs(2), async {
            while !recovering.load(Ordering::SeqCst) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    }

    fn destroyed_listener() -> (RecoveringListener, TcpListener, Arc<AtomicBool>) {
        let original = bind("127.0.0.1:0".parse().unwrap()).unwrap();
        let address = original.local_addr().unwrap();
        socket2::SockRef::from(&original)
            .shutdown(Shutdown::Both)
            .unwrap();
        let occupied = bind(address).unwrap();
        let recovering = Arc::new(AtomicBool::new(false));
        (
            RecoveringListener::new(original, recovering.clone()).unwrap(),
            occupied,
            recovering,
        )
    }

    #[tokio::test]
    async fn destroyed_socket_rebinds_the_exact_endpoint_after_bind_failures() {
        let (mut listener, occupied, recovering) = destroyed_listener();
        let address = listener.local_addr().unwrap();
        let accept = tokio::spawn(async move { listener.accept().await });
        wait_for_recovery(&recovering).await;
        assert!(!accept.is_finished());
        drop(occupied);
        let client = tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                if let Ok(client) = TcpStream::connect(address).await {
                    break client;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        let (server, _) = accept.await.unwrap();
        assert_eq!(server.local_addr().unwrap(), address);
        assert_eq!(client.peer_addr().unwrap(), address);
        assert!(!recovering.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn graceful_shutdown_cancels_a_pending_rebind() {
        let (listener, occupied, recovering) = destroyed_listener();
        let address = listener.local_addr().unwrap();
        let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
        let task = tokio::spawn(async move {
            axum::serve(listener, axum::Router::new())
                .with_graceful_shutdown(async {
                    let _ = stopped.await;
                })
                .await
                .unwrap();
        });
        wait_for_recovery(&recovering).await;
        stop.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(2), task)
            .await
            .unwrap()
            .unwrap();
        drop(occupied);
        assert!(TcpStream::connect(address).await.is_err());
        assert!(bind(address).is_ok());
    }
}
