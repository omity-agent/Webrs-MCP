use crate::{Result, config::validation, error::AppError};
use alloc::sync::Arc;
use tokio::sync::Semaphore;
#[derive(Clone)]
pub struct BlockingPool {
    permits: Arc<Semaphore>,
}
impl BlockingPool {
    #[inline]
    pub fn new(max_concurrent_tasks: usize) -> Result<Self> {
        let permits =
            validation::concurrency_limit(max_concurrent_tasks, "chunking.max_concurrent_tasks")?;
        Ok(Self {
            permits: Arc::new(Semaphore::new(permits.get())),
        })
    }
    #[expect(
        clippy::missing_inline_in_public_items,
        reason = "Blocking work is scheduled through a shared concurrency budget."
    )]
    pub async fn run<Task, Output>(&self, task: Task) -> Result<Output>
    where
        Task: FnOnce() -> Result<Output> + Send + 'static,
        Output: Send + 'static,
    {
        let permit = Arc::clone(&self.permits)
            .acquire_owned()
            .await
            .map_err(|error| AppError::internal(format!("CPU task limiter was closed: {error}")))?;
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            task()
        })
        .await
        .map_err(|error| {
            tracing :: error ! (% error , "blocking processing task failed");
            AppError::internal(format!("page processing task failed: {error}"))
        })?
    }
}
