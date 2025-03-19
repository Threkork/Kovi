use crate::{error::InitManagerError, plugin::PLUGIN_NAME, runtime::RT};
use ahash::RandomState;
use parking_lot::Mutex;
use std::{
    borrow::BorrowMut,
    collections::HashMap,
    future::Future,
    sync::{Arc, OnceLock},
    time::Duration,
};
use tokio::{
    task::{AbortHandle, JoinHandle},
    time::interval,
};
#[cfg(not(feature = "dylib-plugin"))]
pub(crate) static TASK_MANAGER: OnceLock<TaskManager> = OnceLock::new();

#[cfg(feature = "dylib-plugin")]
pub static TASK_MANAGER: OnceLock<TaskManager> = OnceLock::new();

#[derive(Debug)]
pub struct TaskManager {
    pub(crate) handles: Arc<Mutex<TaskAbortHandles>>,
}

impl TaskManager {
    /// 必须先初始化Runtime才能初始化TaskManager
    pub(crate) fn once_init() -> Result<(), InitManagerError> {
        let handles = Arc::new(Mutex::new(TaskAbortHandles::default()));

        let handles_clone = handles.clone();
        RT.get().unwrap().spawn(async move {
            let mut interval = interval(Duration::from_secs(20)); // 每20秒清理一次
            loop {
                interval.tick().await;
                log::debug!("Kovi task thread is cleaning up task handles");

                let mut handles_lock = handles_clone.lock();

                handles_lock.clear();
            }
        });

        let task_manager = Self { handles };

        TASK_MANAGER
            .set(task_manager)
            .map_err(|_| InitManagerError::AlreadyInitialized)
    }

    pub(crate) fn disable_plugin(&self, plugin_name: &str) {
        let mut task_manager = self.handles.lock();

        let map = task_manager.map.borrow_mut();
        let vec = match map.get(plugin_name) {
            Some(v) => v,
            None => return,
        };

        for abort in vec {
            abort.abort();
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct TaskAbortHandles {
    map: HashMap<String, Vec<AbortHandle>, RandomState>,
}

impl Default for TaskAbortHandles {
    fn default() -> Self {
        Self {
            map: HashMap::with_hasher(RandomState::new()),
        }
    }
}

impl TaskAbortHandles {
    pub(crate) fn clear(&mut self) {
        for vec in self.map.values_mut() {
            vec.retain(|abort| !abort.is_finished());
            vec.shrink_to_fit();
        }
    }
}

/// 生成一个新的异步线程并立即运行，另外，这个线程关闭句柄会被交给 Kovi 管理。
///
/// **如果在 Kovi 管理之外的地方（新的tokio线程或者系统线程）运行此函数，此函数会 panic!**
///
/// 由 Kovi 管理的地方：
///
/// 1. 有 #[kovi::plugin] 的插件入口函数。
/// 2. 插件的监听闭包。
/// 3. 由 kovi::spawn() 创建的新线程。
///
/// # panic!
///
/// 如果在 Kovi 管理之外的地方（tokio线程或者系统线程）运行此函数，此函数会 panic!
#[cfg(not(feature = "dylib-plugin"))]
pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    PLUGIN_NAME.with(|name| {
        let join = {
            let name = name.clone();
            RT.get().unwrap().spawn(PLUGIN_NAME.scope(name, future))
        };

        let about_join = join.abort_handle();

        task_manager_handler(name, about_join);

        join
    })
}

pub(crate) fn task_manager_handler(name: &str, about_join: AbortHandle) {
    let mut task_abort_handles = TASK_MANAGER.get().unwrap().handles.lock();

    let aborts = task_abort_handles.map.entry(name.to_string()).or_default();

    aborts.push(about_join);
}

///////////////////////////////////// 以下为dylib特殊对待

/// 生成一个新的异步线程并立即运行，另外，这个线程关闭句柄会被交给 Kovi 管理。
///
/// **如果在 Kovi 管理之外的地方（新的tokio线程或者系统线程）运行此函数，此函数会 panic!**
///
/// 由 Kovi 管理的地方：
///
/// 1. 有 #[kovi::plugin] 的插件入口函数。
/// 2. 插件的监听闭包。
/// 3. 由 kovi::spawn() 创建的新线程。
///
/// # panic!
///
/// 如果在 Kovi 管理之外的地方（tokio线程或者系统线程）运行此函数，此函数会 panic!
#[cfg(feature = "dylib-plugin")]
pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let join = RT.get().unwrap().spawn(future);

    let about_join = join.abort_handle();

    let name = PLUGIN_NAME.get().unwrap();

    task_manager_handler(name, about_join);

    join
}
