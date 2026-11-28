use std::{fs, future::Future, task::Poll};

use super::*;

#[tokio::test]
async fn preservation_requires_completed_startup_failure_and_does_not_retry_it() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("broken.sqlite");
    let folder = root.path().join("copies");
    fs::create_dir(&folder).unwrap();
    let bytes = b"damaged SQLite state";
    fs::write(&path, bytes).unwrap();
    let mut startup = Startup::new(&path);
    assert_eq!(
        startup
            .preserve_problem_state(folder.clone(), 1024)
            .await
            .unwrap_err(),
        StorageError::RetryRequired
    );
    assert!(startup.automatic(None).await.is_err());
    let failure = startup.state();
    assert!(matches!(failure, StartupState::Failed(_)));
    let copy = startup
        .preserve_problem_state(folder.clone(), 1024)
        .await
        .unwrap();
    assert_eq!(startup.state(), failure);
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert!(startup.automatic(None).await.is_err());
    assert!(startup.retry(None).await.is_err());
    copy.verify().unwrap();
    fs::remove_file(&path).unwrap();
    let storage = startup.retry(None).await.unwrap();
    assert_eq!(startup.state(), StartupState::Ready);
    assert_eq!(
        startup
            .preserve_problem_state(folder, 1024)
            .await
            .unwrap_err(),
        StorageError::RetryRequired
    );
    storage.shutdown().await.unwrap();
    copy.verify().unwrap();
}

#[tokio::test]
async fn cancelled_wait_retains_capture_and_blocks_retry_until_same_result_is_collected() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("broken.sqlite");
    fs::write(&path, b"damaged").unwrap();
    let mut startup = Startup::new(&path);
    assert!(startup.automatic(None).await.is_err());
    let (reply, ready) = tokio::sync::oneshot::channel();
    let folder = root.path().to_owned();
    startup.problem = Some(PendingProblem {
        folder: folder.clone(),
        max_total_bytes: 1024,
        ready,
    });
    {
        let future = startup.preserve_problem_state(folder.clone(), 1024);
        tokio::pin!(future);
        std::future::poll_fn(|cx| {
            assert!(future.as_mut().poll(cx).is_pending());
            Poll::Ready(())
        })
        .await;
    }
    assert!(startup.retry(None).await.is_err());
    assert_eq!(
        startup
            .preserve_problem_state(folder.clone(), 2048)
            .await
            .unwrap_err(),
        StorageError::RetryRequired
    );
    let copy = crate::problem_state::preserve(&path, &folder, 1024).unwrap();
    let expected = copy.path().to_owned();
    reply.send(Ok(copy)).unwrap();
    let copy = startup.preserve_problem_state(folder, 1024).await.unwrap();
    assert_eq!(copy.path(), expected);
    assert!(startup.problem.is_none());
    copy.discard().unwrap();
}
