use core::time::Duration;
use std::sync::mpsc;
use tokio::sync::oneshot;
use web_rs::mcp::processing::BlockingPool;
#[tokio::test]
async fn cancellation_keeps_running_work_inside_the_budget() {
    let pool = BlockingPool::new(1).unwrap();
    let first_pool = pool.clone();
    let (started, observed) = oneshot::channel();
    let (release, wait) = mpsc::channel();
    let caller = tokio::spawn(async move {
        first_pool
            .run(move || {
                started.send(()).unwrap();
                wait.recv_timeout(Duration::from_secs(5)).unwrap();
                Ok(())
            })
            .await
    });
    observed.await.unwrap();
    caller.abort();
    assert!(caller.await.unwrap_err().is_cancelled());
    let (entered, mut notified) = oneshot::channel();
    let second = tokio::spawn(async move {
        pool.run(move || {
            entered.send(()).unwrap();
            Ok(())
        })
        .await
    });
    let early = tokio::time::timeout(Duration::from_millis(50), &mut notified).await;
    release.send(()).unwrap();
    assert!(
        early.is_err(),
        "cancellation released a permit for work still running"
    );
    notified.await.unwrap();
    second.await.unwrap().unwrap();
}
