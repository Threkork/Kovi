use ahash::RandomState;
use parking_lot::Mutex;
use std::{
    borrow::BorrowMut,
    collections::HashMap,
    future::Future,
    sync::{Arc, LazyLock},
    time::Duration,
};
use tokio::{
    task::{AbortHandle, JoinHandle},
    time::interval,
};

pub(crate) static TASK_MANAGER: LazyLock<TaskManager> = LazyLock::new(TaskManager::init);


pub(crate) struct TaskManager {
    pub(crate) handles: Arc<Mutex<TaskAbortHandles>>,
}

impl TaskManager {
    pub(crate) fn init() -> Self {
        let handles = Arc::new(Mutex::new(TaskAbortHandles::default()));

        let handles_clone = handles.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(20)); // 每<?>秒清理一次
            loop {
                interval.tick().await;
                log::debug!("Kovi task thread is cleaning up task handles");

                let mut handles_lock = handles_clone.lock();

                handles_lock.clear();
            }
        });

        Self { handles }
    }

    pub(crate) fn disable_plugin(&self, plugin_name: &str) {
        let mut task_manager = self.handles.lock();

        let map = task_manager.map.borrow_mut();
        println!("{:?}", map);
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


tokio::task_local! {
    pub(crate) static PLUGIN_NAME: Arc<String>;
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
pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    PLUGIN_NAME.with(|name| {
        let join = {
            let name = name.clone();
            tokio::spawn(PLUGIN_NAME.scope(name, future))
        };

        let about_join = join.abort_handle();


        if TASK_MANAGER.handles.is_locked() {
            task_manager_handler(name, about_join);
        } else {
            tokio::spawn({
                let name = name.clone();
                async move {
                    task_manager_handler(&name, about_join);
                }
            });
        }

        join
    })
}

pub fn task_manager_handler(name: &str, about_join: AbortHandle) {
    let mut task_abort_handles = TASK_MANAGER.handles.lock();

    let aborts = task_abort_handles.map.entry(name.to_string()).or_default();

    aborts.push(about_join);
}
