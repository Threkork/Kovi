use crate::{
    Bot, PluginBuilder,
    bot::handler::{InternalEvent, KoviEvent},
    runtime::RT,
    task::TASK_MANAGER,
    types::ApiAndOneshot,
};
use log::error;
use parking_lot::RwLock;
use std::borrow::Borrow;
use std::future::Future;
use std::process::exit;
use std::sync::{Arc, LazyLock};
use tokio::sync::{
    mpsc::{self, Sender},
    watch,
};
use tokio::task::JoinHandle;

struct InitManager {}
impl InitManager {
    pub(crate) fn init() {
        if RT.get().is_none() {
            crate::runtime::Runtime::once_init().unwrap();
        }

        if TASK_MANAGER.get().is_none() {
            crate::task::TaskManager::once_init().unwrap();
        }
    }
}

impl Bot {
    pub fn spawn<F>(&mut self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let join = RT.get().unwrap().spawn(future);
        self.run_abort.push(join.abort_handle());
        join
    }

    /// 运行bot
    ///
    /// **注意此函数会阻塞, 直到Bot连接失效，或者有退出信号传入程序**
    pub fn run(self) {
        InitManager::init();

        let server = self.information.server.clone();

        let bot = Arc::new(RwLock::new(self));

        let async_task = async {
            //处理连接，从msg_tx返回消息
            let (event_tx, mut event_rx): (
                mpsc::Sender<InternalEvent>,
                mpsc::Receiver<InternalEvent>,
            ) = mpsc::channel(32);

            // 接收插件的api
            let (api_tx, api_rx): (mpsc::Sender<ApiAndOneshot>, mpsc::Receiver<ApiAndOneshot>) =
                mpsc::channel(32);

            // 连接
            let connect_task = RT.get().unwrap().spawn({
                let event_tx = event_tx.clone();
                Self::ws_connect(server, api_rx, event_tx, bot.clone())
            });

            let connect_res = connect_task.await.unwrap();

            if let Err(e) = connect_res {
                error!(
                    "{e}\nBot connection failed, please check the configuration and restart the bot"
                );
                return;
            }

            {
                let mut bot_write = bot.write();

                // drop检测
                bot_write.spawn({
                    let event_tx = event_tx;
                    exit_signal_check(event_tx)
                });

                // 运行所有的main
                #[cfg(not(feature = "dylib-plugin"))]
                bot_write.spawn({
                    let bot = bot.clone();
                    let api_tx = api_tx.clone();
                    async move { Self::run_mains(bot, api_tx) }
                });
            }

            let mut drop_task = None;
            //处理事件，每个事件都会来到这里
            while let Some(event) = event_rx.recv().await {
                let api_tx = api_tx.clone();
                let bot = bot.clone();

                // Drop为关闭事件，所以要等待，其他的不等待
                if let InternalEvent::KoviEvent(KoviEvent::Drop) = event {
                    drop_task = Some(
                        RT.get()
                            .unwrap()
                            .spawn(Self::handler_event(bot, event, api_tx)),
                    );
                    break;
                } else {
                    RT.get()
                        .unwrap()
                        .spawn(Self::handler_event(bot, event, api_tx));
                }
            }
            if let Some(drop_task) = drop_task {
                match drop_task.await {
                    Ok(_) => {}
                    Err(e) => {
                        error!("{}", e)
                    }
                };
            }
        };

        RT.get().unwrap().block_on(async_task);
    }

    /// 运行所有main()
    #[cfg(not(feature = "dylib-plugin"))]
    fn run_mains(bot: Arc<RwLock<Self>>, api_tx: mpsc::Sender<ApiAndOneshot>) {
        let bot_ = bot.read();
        let main_job_map = bot_.plugins.borrow();

        let (host, port) = {
            (
                bot_.information.server.host.clone(),
                bot_.information.server.port,
            )
        };

        for (name, plugin) in main_job_map.iter() {
            if !plugin.enable_on_startup {
                continue;
            }
            let plugin_builder = PluginBuilder::new(
                name.clone(),
                bot.clone(),
                host.clone(),
                port,
                api_tx.clone(),
            );

            plugin.run(plugin_builder);
        }
    }
}

pub(crate) static DROP_CHECK: LazyLock<ExitCheck> = LazyLock::new(ExitCheck::init);

pub struct ExitCheck {
    watch_rx: watch::Receiver<bool>,
    join_handle: tokio::task::JoinHandle<()>,
}

impl Drop for ExitCheck {
    fn drop(&mut self) {
        self.join_handle.abort();
    }
}

impl ExitCheck {
    fn init() -> ExitCheck {
        let (tx, watch_rx) = watch::channel(false);

        // 启动 drop check 任务
        let join_handle = RT.get().unwrap().spawn(async move {
            Self::await_exit_signal().await;

            let _ = tx.send(true);

            Self::await_exit_signal().await;

            handler_second_time_exit_signal().await;
        });

        ExitCheck {
            watch_rx,
            join_handle,
        }
    }

    async fn await_exit_signal() {
        #[cfg(unix)]
        use tokio::signal::unix::{SignalKind, signal};
        #[cfg(windows)]
        use tokio::signal::windows;

        #[cfg(windows)]
        {
            let mut sig_ctrl_break = windows::ctrl_break().unwrap();
            let mut sig_ctrl_c = windows::ctrl_c().unwrap();
            let mut sig_ctrl_close = windows::ctrl_close().unwrap();
            let mut sig_ctrl_logoff = windows::ctrl_logoff().unwrap();
            let mut sig_ctrl_shutdown = windows::ctrl_shutdown().unwrap();

            tokio::select! {
                _ = sig_ctrl_break.recv() => {}
                _ = sig_ctrl_c.recv() => {}
                _ = sig_ctrl_close.recv() => {}
                _ = sig_ctrl_logoff.recv() => {}
                _ = sig_ctrl_shutdown.recv() => {}
            }
        }

        #[cfg(unix)]
        {
            let mut sig_hangup = signal(SignalKind::hangup()).unwrap();
            let mut sig_alarm = signal(SignalKind::alarm()).unwrap();
            let mut sig_interrupt = signal(SignalKind::interrupt()).unwrap();
            let mut sig_quit = signal(SignalKind::quit()).unwrap();
            let mut sig_terminate = signal(SignalKind::terminate()).unwrap();

            tokio::select! {
                _ = sig_hangup.recv() => {}
                _ = sig_alarm.recv() => {}
                _ = sig_interrupt.recv() => {}
                _ = sig_quit.recv() => {}
                _ = sig_terminate.recv() => {}
            }
        }
    }

    pub async fn await_exit_signal_change(&self) {
        let mut rx = self.watch_rx.clone();
        rx.changed().await.unwrap();
    }
}

pub(crate) async fn exit_signal_check(tx: Sender<InternalEvent>) {
    DROP_CHECK.await_exit_signal_change().await;

    tx.send(InternalEvent::KoviEvent(KoviEvent::Drop))
        .await
        .unwrap();
}

async fn handler_second_time_exit_signal() {
    exit(1)
}
