use std::sync::Mutex;

use tokio::task::JoinHandle;

#[derive(Debug)]
pub struct BackgroundTask(Mutex<Option<JoinHandle<()>>>);

impl BackgroundTask {
    pub const fn new(handle: JoinHandle<()>) -> Self {
        Self(Mutex::new(Some(handle)))
    }

    pub async fn shutdown(&self) {
        let handle = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();

        if let Some(handle) = handle {
            handle.abort();
            let _ = handle.await;
        }
    }
}

impl Drop for BackgroundTask {
    fn drop(&mut self) {
        if let Some(handle) = self
            .0
            .get_mut()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
        {
            handle.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    use super::*;

    struct DropFlag(Arc<AtomicBool>);

    impl Drop for DropFlag {
        fn drop(&mut self) {
            self.0.store(true, Ordering::Release);
        }
    }

    #[tokio::test]
    async fn shutdown_aborts_and_joins_the_task() {
        let started = Arc::new(AtomicBool::new(false));
        let dropped = Arc::new(AtomicBool::new(false));
        let task = BackgroundTask::new(tokio::spawn({
            let started = started.clone();
            let dropped = dropped.clone();

            async move {
                let _drop_flag = DropFlag(dropped);
                started.store(true, Ordering::Release);
                futures::future::pending::<()>().await;
            }
        }));

        while !started.load(Ordering::Acquire) {
            tokio::task::yield_now().await;
        }

        task.shutdown().await;
        task.shutdown().await;

        assert!(dropped.load(Ordering::Acquire));
    }
}
