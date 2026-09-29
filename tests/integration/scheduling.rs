use alloc::sync::Arc;
use core::{
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};
use futures::future::join_all;
use std::sync::mpsc;
use tokio::sync::oneshot;
use web_rs::mcp::processing::BlockingPool;
#[tokio::test]
async fn cloned_pools_share_one_limit() {
    let pool = BlockingPool::new(2).unwrap();
    let running = Arc::new(AtomicUsize::new(0));
    let peak = Arc::new(AtomicUsize::new(0));
    let jobs = core::iter::repeat_with(|| {
        let shared = pool.clone();
        let active = Arc::clone(&running);
        let maximum = Arc::clone(&peak);
        async move {
            shared
                .run(move || {
                    let count = active.fetch_add(1, Ordering::SeqCst) + 1;
                    maximum.fetch_max(count, Ordering::SeqCst);
                    std::thread::sleep(Duration::from_millis(10));
                    active.fetch_sub(1, Ordering::SeqCst);
                    Ok(())
                })
                .await
                .unwrap();
        }
    })
    .take(12);
    join_all(jobs).await;
    assert_eq!(peak.load(Ordering::SeqCst), 2);
    assert_eq!(running.load(Ordering::SeqCst), 0);
}
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
