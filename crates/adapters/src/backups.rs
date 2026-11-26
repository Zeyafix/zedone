use std::{sync::Arc, time::Duration};
use tokio::{sync::Notify, task::JoinHandle};
use zedone_storage::Storage;
pub(crate) struct Worker {
    wake: Arc<Notify>,
    task: JoinHandle<()>,
}
impl Worker {
    pub(crate) fn new(storage: Storage) -> Self {
        let wake = Arc::new(Notify::new());
        let notify = wake.clone();
        let task = tokio::spawn(async move {
            let _ = storage.initialize_daily_backups().await;
            let _ = storage.cleanup_daily_backups().await;
            let mut changed = true;
            let mut failed = true;
            let mut last_day = None;
            let mut cleanup = std::time::Instant::now();
            loop {
                if cleanup.elapsed() >= Duration::from_secs(300) {
                    let _ = storage.retry_daily_cleanup().await;
                    cleanup = std::time::Instant::now();
                }
                let day = zedone_storage::daily_backup::local_day().ok();
                let calendar = day != last_day;
                if changed || calendar || failed {
                    failed = storage
                        .daily_tick(changed || calendar)
                        .await
                        .unwrap_or(true);
                }
                last_day = day;
                tokio::select! {
                    _=notify.notified()=>changed=true,
                    _=tokio::time::sleep(Duration::from_secs(1))=>changed=false,
                }
            }
        });
        Self { wake, task }
    }
    pub(crate) fn notify(&self) {
        self.wake.notify_one();
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.task.abort();
    }
}
