use super::{
    handler::{InternalEvent, KoviEvent},
    ApiAndOneshot, Bot,
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

tokio::task_local! {
    pub(crate) static PLUGIN_BUILDER: PluginBuilder;
}

// pub(crate) type MpscCronTask = (
//     Cron,
//     String,
//     Arc<dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>,
// );


impl Bot {
    /// 运行bot
    /// **注意此函数会阻塞并且接管程序退出, 程序不会运行后续所有代码**
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
            let (api_tx, api_rx): (mpsc::Sender<ApiAndOneshot>, mpsc::Receiver<ApiAndOneshot>) =
                mpsc::channel(32);

            // let (cron_tx, mut cron_rx): (mpsc::Sender<MpscCronTask>, mpsc::Receiver<MpscCronTask>) =
            //     mpsc::channel(32);

            // 事件连接
            tokio::spawn({
                let event_tx = event_tx.clone();
                let access_token = access_token.clone();
                Self::ws_connect(host, port, access_token, event_tx)
            });

            // drop检测
            tokio::spawn({
                let event_tx = event_tx.clone();
                drop_check(event_tx, false)
            });

            // api连接
            tokio::spawn({
                let access_token = access_token.clone();
                Self::ws_send_api(host, port, access_token, api_rx, event_tx)
            });


            // 运行所有的main
            tokio::spawn({
                let bot = bot.clone();
                let api_tx = api_tx.clone();
                Self::plugin_main(bot, api_tx)
            });

            //定时任务
            // tokio::spawn(async move {
            //     while let Some(v) = cron_rx.recv().await {
            //         let (cron, plugin_name, task) = v;
            //         let now = chrono::Local::now();
            //         let next = cron.find_next_occurrence(&now, false).unwrap();
            //     }
            // });


            let mut drop_task = None;
            //处理事件，每个事件都会来到这里
            while let Some(event) = event_rx.recv().await {
                let api_tx = api_tx.clone();
                let bot = bot.clone();

                // Drop为关闭事件，所以要等待，其他的不等待
                if let InternalEvent::KoviEvent(KoviEvent::Drop) = event {
                    drop_task = Some(tokio::spawn(Self::handler_event(bot, event, api_tx)));
                    break;
                } else {
                    tokio::spawn(Self::handler_event(bot, event, api_tx));
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

    // 运行所有main()
    async fn plugin_main(bot: Arc<RwLock<Self>>, api_tx: mpsc::Sender<ApiAndOneshot>) {
        let main_job_map = bot.read().unwrap().plugins.clone();

        for (name, plugins) in main_job_map {
            tokio::spawn({
                let plugin_builder = PluginBuilder::new(name.clone(), bot.clone(), api_tx.clone());

                // 异步运行 main()
                PLUGIN_BUILDER.scope(plugin_builder, (plugins.main)())
            });
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
