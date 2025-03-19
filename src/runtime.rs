use crate::error::InitManagerError;
use std::sync::{Arc, OnceLock};
use tokio::{runtime::Runtime as TokioRuntime, task::JoinHandle};

pub static RUNTIME: OnceLock<Runtime> = OnceLock::new();

pub(crate) use RUNTIME as RT;

#[derive(Debug)]
pub struct Runtime {
    rt: Arc<TokioRuntime>,
}
impl Runtime {
    fn new() -> Self {
        let rt = TokioRuntime::new().unwrap();
        Self { rt: Arc::new(rt) }
    }

    pub(crate) fn once_init() -> Result<(), InitManagerError> {
        RUNTIME
            .set(Self::new())
            .map_err(|_| InitManagerError::AlreadyInitialized)
    }

    pub fn block_on<F: Future>(&self, future: F) -> F::Output {
        self.rt.block_on(future)
    }

    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.rt.spawn(future)
    }
}
