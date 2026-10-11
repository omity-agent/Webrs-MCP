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
    let (entered, mut observed) = tokio::sync::mpsc::unbounded_channel();
    let mut releases = Vec::new();
    let mut jobs = Vec::new();
    for _ in 0_usize..8_usize {
        let shared = pool.clone();
        let active = Arc::clone(&running);
        let maximum = Arc::clone(&peak);
        let started = entered.clone();
        let (release, wait) = mpsc::channel();
        releases.push(release);
        jobs.push(tokio::spawn(async move {
            shared
                .run(move || {
                    let count = active.fetch_add(1, Ordering::SeqCst) + 1;
                    maximum.fetch_max(count, Ordering::SeqCst);
                    started.send(()).unwrap();
                    wait.recv_timeout(Duration::from_secs(5)).unwrap();
                    active.fetch_sub(1, Ordering::SeqCst);
                    Ok(())
                })
                .await
                .unwrap();
        }));
    }
    for _ in 0_usize..2_usize {
        tokio::time::timeout(Duration::from_secs(5), observed.recv())
            .await
            .unwrap()
            .unwrap();
    }
    let extra = tokio::time::timeout(Duration::from_millis(50), observed.recv()).await;
    for release in releases {
        release.send(()).unwrap();
    }
    for result in join_all(jobs).await {
        result.unwrap();
    }
    assert!(
        extra.is_err(),
        "cloned pools admitted work above their shared limit"
    );
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
