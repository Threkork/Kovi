use super::{
    handler::{InternalEvent, KoviEvent},
    runtimebot::ApiOneshot,
    Bot,
};
use crate::PluginBuilder;
use log::error;
use std::{
    process::exit,
    sync::{Arc, RwLock},
};
use tokio::{
    runtime::Runtime,
    sync::mpsc::{self, Sender},
};


impl Bot {
    /// 运行bot
    /// **注意此函数会阻塞**
    pub fn run(self) {
        let (host, port, access_token) = (
            self.information.server.host,
            self.information.server.port,
            self.information.server.access_token.clone(),
        );

        let bot = Arc::new(RwLock::new(self));

        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            //处理连接，从msg_tx返回消息
            let (event_tx, mut event_rx): (
                mpsc::Sender<InternalEvent>,
                mpsc::Receiver<InternalEvent>,
            ) = mpsc::channel(32);
            // 接收插件的api
            let (api_tx, api_rx): (mpsc::Sender<ApiOneshot>, mpsc::Receiver<ApiOneshot>) =
                mpsc::channel(32);

            // 事件连接
            tokio::spawn({
                let event_tx = event_tx.clone();
                let access_token = access_token.clone();
                async move {
                    Self::ws_connect(host, port, access_token, event_tx).await;
                }
            });

            // drop检测
            tokio::spawn({
                let event_tx = event_tx.clone();
                async move {
                    drop_check(event_tx, false).await;
                }
            });

            // api连接
            tokio::spawn({
                let access_token = access_token.clone();
                async move {
                    Self::ws_send_api(host, port, access_token, api_rx, event_tx).await;
                }
            });


            // 运行所有的main
            tokio::spawn({
                let bot = bot.clone();
                let api_tx = api_tx.clone();
                async { Self::plugin_main(bot, api_tx).await }
            });


            let mut drop_task = None;
            //处理事件，每个事件都会来到这里
            while let Some(event) = event_rx.recv().await {
                let api_tx = api_tx.clone();
                let bot = bot.clone();

                // Drop为关闭事件，所以要等待，其他的不等待
                if let InternalEvent::KoviEvent(KoviEvent::Drop) = event {
                    drop_task = Some(tokio::spawn(async {
                        Self::handler_event(bot, event, api_tx).await;
                    }));
                    break;
                } else {
                    tokio::spawn(async {
                        Self::handler_event(bot, event, api_tx).await;
                    });
                }
            }
            if let Some(drop_task) = drop_task {
                match drop_task.await {
                    Ok(_) => {}
                    Err(e) => {
                        error!("{}", e)
                    }
                };
                exit(0)
            }
        });
    }

    async fn plugin_main(bot: Arc<RwLock<Self>>, api_tx: mpsc::Sender<ApiOneshot>) {
        // 运行所有main()
        let bot_main_job_clone = bot.clone();
        let api_tx_main_job_clone = api_tx.clone();

        let mut main_job_vec;
        {
            let bot = bot_main_job_clone.read().unwrap();
            main_job_vec = bot.main.clone();
        }

        //储存所有main()
        let mut handler_main_job = Vec::new();

        while let Some(main_job) = main_job_vec.pop() {
            let bot_main_job_clone = bot_main_job_clone.clone();
            let api_tx = api_tx_main_job_clone.clone();
            handler_main_job.push(tokio::spawn(async move {
                let plugin_builder =
                    PluginBuilder::new(main_job.name.clone(), bot_main_job_clone.clone(), api_tx);
                // 异步运行 main()
                (main_job.main)(plugin_builder).await;
            }));
        }
        //等待所有main()结束
        for handler in handler_main_job {
            match handler.await {
                Ok(_) => {}
                Err(e) => {
                    error!("plugin main error: {}", e);
                }
            }
        }
    }
}


#[cfg(windows)]
use tokio::signal::windows;

#[cfg(unix)]
use tokio::signal::unix::{signal, SignalKind};

async fn drop_check(tx: Sender<InternalEvent>, exit: bool) {
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

    if exit {
        std::process::exit(1);
    }

    tx.send(InternalEvent::KoviEvent(KoviEvent::Drop))
        .await
        .unwrap();

    //递归运行本函数，第二次就会结束进程
    Box::pin(drop_check(tx, true)).await;
}
