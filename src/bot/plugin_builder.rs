use super::{runtimebot::RuntimeBot, Bot};
use super::{ApiAndOneshot, Host, PLUGIN_BUILDER, PLUGIN_NAME};
use croner::errors::CronError;
use croner::Cron;
use event::{AllMsgEvent, AllNoticeEvent, AllRequestEvent};
use log::error;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;

pub mod event;

pub type PinFut = Pin<Box<dyn Future<Output = ()> + Send>>;

pub type AllMsgFn = Arc<dyn Fn(Arc<AllMsgEvent>) -> PinFut + Send + Sync>;

pub type AllNoticeFn = Arc<dyn Fn(Arc<AllNoticeEvent>) -> PinFut + Send + Sync>;

pub type AllRequestFn = Arc<dyn Fn(Arc<AllRequestEvent>) -> PinFut + Send + Sync>;

pub type NoArgsFn = Arc<dyn Fn() -> PinFut + Send + Sync>;

#[derive(Clone, Default)]
pub(crate) struct Listen {
    pub(crate) msg: Vec<Arc<ListenMsgFn>>,
    #[cfg(feature = "message_sent")]
    pub(crate) msg_sent: Vec<AllMsgFn>,
    pub(crate) notice: Vec<AllNoticeFn>,
    pub(crate) request: Vec<AllRequestFn>,
    pub(crate) drop: Vec<NoArgsFn>,
}

#[derive(Clone)]
pub(crate) enum ListenMsgFn {
    Msg(AllMsgFn),
    PrivateMsg(AllMsgFn),
    GroupMsg(AllMsgFn),
    AdminMsg(AllMsgFn),
}

impl Listen {
    pub fn clear(&mut self) {
        self.msg.clear();
        self.notice.clear();
        self.request.clear();
        self.drop.clear();
        self.msg.shrink_to_fit();
        self.notice.shrink_to_fit();
        self.request.shrink_to_fit();
        self.drop.shrink_to_fit();
        #[cfg(feature = "message_sent")]
        self.msg_sent.clear();
        #[cfg(feature = "message_sent")]
        self.msg_sent.shrink_to_fit();
    }
}

#[derive(Clone)]
pub struct PluginBuilder {
    pub(crate) runtime_bot: Arc<RuntimeBot>,
}

impl PluginBuilder {
    pub(crate) fn new(
        name: String,
        bot: Arc<RwLock<Bot>>,
        main_admin: i64,
        admin: Vec<i64>,
        host: Host,
        port: u16,
        api_tx: mpsc::Sender<ApiAndOneshot>,
    ) -> Self {
        let runtime_bot = Arc::new(RuntimeBot {
            main_admin,
            admin,
            host,
            port,

            bot,
            plugin_name: name,
            api_tx,
        });

        PluginBuilder { runtime_bot }
    }

    pub fn get_runtime_bot() -> Arc<RuntimeBot> {
        PLUGIN_BUILDER.with(|p| p.runtime_bot.clone())
    }

    pub fn get_plugin_name() -> String {
        PLUGIN_BUILDER.with(|p| p.runtime_bot.plugin_name.to_string())
    }

    pub fn get_plugin_host() -> (Host, u16) {
        PLUGIN_BUILDER.with(|p| (p.runtime_bot.host.clone(), p.runtime_bot.port))
    }
}

impl PluginBuilder {
    /// 注册消息处理函数。
    ///
    /// 注册一个处理程序，用于处理接收到的消息事件（`AllMsgEvent`）。
    pub fn on_msg<F, Fut>(handler: F)
    where
        F: Fn(Arc<AllMsgEvent>) -> Fut + Send + Sync + 'static,
        Fut: Future + Send,
        Fut::Output: Send,
    {
        PLUGIN_BUILDER.with(|p| {
            let mut bot = p.runtime_bot.bot.write().unwrap();
            let bot_plugin = bot.plugins.get_mut(&p.runtime_bot.plugin_name).unwrap();

            let handler = Arc::new(handler);

            let listen_fn = ListenMsgFn::Msg(Arc::new({
                move |event| {
                    Box::pin({
                        let handler = handler.clone();
                        async move {
                            handler(event).await;
                        }
                    })
                }
            }));

            bot_plugin.listen.msg.push(Arc::new(listen_fn));
        })
    }

    /// 注册管理员消息处理函数。
    ///
    /// 注册一个处理程序，用于处理接收到的消息事件（`AllMsgEvent`）。
    pub fn on_admin_msg<F, Fut>(handler: F)
    where
        F: Fn(Arc<AllMsgEvent>) -> Fut + Send + Sync + 'static,
        Fut: Future + Send,
        Fut::Output: Send,
    {
        PLUGIN_BUILDER.with(|p| {
            let mut bot = p.runtime_bot.bot.write().unwrap();
            let bot_plugin = bot.plugins.get_mut(&p.runtime_bot.plugin_name).unwrap();

            bot_plugin
                .listen
                .msg
                .push(Arc::new(ListenMsgFn::AdminMsg(Arc::new({
                    let handler = Arc::new(handler);
                    move |event| {
                        Box::pin({
                            let handler = handler.clone();
                            async move {
                                handler(event).await;
                            }
                        })
                    }
                }))));
        })
    }

    /// 注册管理员消息处理函数。
    ///
    /// 注册一个处理程序，用于处理接收到的消息事件（`AllMsgEvent`）。
    pub fn on_private_msg<F, Fut>(handler: F)
    where
        F: Fn(Arc<AllMsgEvent>) -> Fut + Send + Sync + 'static,
        Fut: Future + Send,
        Fut::Output: Send,
    {
        PLUGIN_BUILDER.with(|p| {
            let mut bot = p.runtime_bot.bot.write().unwrap();
            let bot_plugin = bot.plugins.get_mut(&p.runtime_bot.plugin_name).unwrap();

            bot_plugin
                .listen
                .msg
                .push(Arc::new(ListenMsgFn::PrivateMsg(Arc::new({
                    let handler = Arc::new(handler);
                    move |event| {
                        Box::pin({
                            let handler = handler.clone();
                            async move {
                                handler(event).await;
                            }
                        })
                    }
                }))));
        })
    }

    pub fn on_group_msg<F, Fut>(handler: F)
    where
        F: Fn(Arc<AllMsgEvent>) -> Fut + Send + Sync + 'static,
        Fut: Future + Send,
        Fut::Output: Send,
    {
        PLUGIN_BUILDER.with(|p| {
            let mut bot = p.runtime_bot.bot.write().unwrap();
            let bot_plugin = bot.plugins.get_mut(&p.runtime_bot.plugin_name).unwrap();

            bot_plugin
                .listen
                .msg
                .push(Arc::new(ListenMsgFn::GroupMsg(Arc::new({
                    let handler = Arc::new(handler);
                    move |event| {
                        Box::pin({
                            let handler = handler.clone();
                            async move {
                                handler(event).await;
                            }
                        })
                    }
                }))));
        })
    }

    #[cfg(feature = "message_sent")]
    /// 注册 message_sent 消息处理函数。
    pub fn on_msg_send<F, Fut>(handler: F)
    where
        F: Fn(Arc<AllMsgEvent>) -> Fut + Send + Sync + 'static,
        Fut: Future + Send,
        Fut::Output: Send,
    {
        PLUGIN_BUILDER.with(|p| {
            let mut bot = p.runtime_bot.bot.write().unwrap();
            let bot_plugin = bot.plugins.get_mut(&p.runtime_bot.plugin_name).unwrap();

            bot_plugin.listen.msg_sent.push(Arc::new({
                let handler = Arc::new(handler);
                move |event| {
                    Box::pin({
                        let handler = handler.clone();
                        async move {
                            handler(event).await;
                        }
                    })
                }
            }));
        })
    }

    /// 注册消息处理函数。
    ///
    /// 注册一个处理程序，用于处理接收到的消息事件（`AllNoticeEvent`）。
    pub fn on_all_notice<F, Fut>(handler: F)
    where
        F: Fn(Arc<AllNoticeEvent>) -> Fut + Send + Sync + 'static,
        Fut: Future + Send,
        Fut::Output: Send,
    {
        PLUGIN_BUILDER.with(|p| {
            let mut bot = p.runtime_bot.bot.write().unwrap();
            let bot_plugin = bot.plugins.get_mut(&p.runtime_bot.plugin_name).unwrap();

            bot_plugin.listen.notice.push(Arc::new({
                let handler = Arc::new(handler);
                move |event| {
                    Box::pin({
                        let handler = handler.clone();
                        async move {
                            handler(event).await;
                        }
                    })
                }
            }));
        })
    }

    /// 注册异步消息处理函数。
    ///
    /// 注册一个处理程序，用于处理接收到的消息事件（`AllRequestEvent`）。
    pub fn on_all_request<F, Fut>(handler: F)
    where
        F: Fn(Arc<AllRequestEvent>) -> Fut + Send + Sync + 'static,
        Fut: Future + Send,
        Fut::Output: Send,
    {
        PLUGIN_BUILDER.with(|p| {
            let mut bot = p.runtime_bot.bot.write().unwrap();
            let bot_plugin = bot.plugins.get_mut(&p.runtime_bot.plugin_name).unwrap();

            bot_plugin.listen.request.push(Arc::new({
                let handler = Arc::new(handler);
                move |event| {
                    Box::pin({
                        let handler = handler.clone();
                        async move {
                            handler(event).await;
                        }
                    })
                }
            }));
        })
    }

    /// 注册程序结束事件处理函数。
    ///
    /// 注册处理程序，用于处理接收到的程序结束事件。
    pub fn drop<F, Fut>(handler: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future + Send,
        Fut::Output: Send,
    {
        PLUGIN_BUILDER.with(|p| {
            let mut bot = p.runtime_bot.bot.write().unwrap();
            let bot_plugin = bot.plugins.get_mut(&p.runtime_bot.plugin_name).unwrap();

            bot_plugin.listen.drop.push(Arc::new({
                let handler = Arc::new(handler);
                move || {
                    Box::pin({
                        let handler = handler.clone();
                        async move {
                            handler().await;
                        }
                    })
                }
            }));
        })
    }

    /// 注册定时任务。
    ///
    /// 传入 Cron 。
    pub fn cron<F, Fut>(cron: &str, handler: F) -> Result<(), CronError>
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future + Send,
        Fut::Output: Send,
    {
        PLUGIN_BUILDER.with(|p| {
            let cron = match Cron::new(cron).with_seconds_optional().parse() {
                Ok(v) => v,
                Err(e) => return Err(e),
            };
            Self::run_cron_task(p, cron, handler);
            Ok(())
        })
    }

    /// 注册定时任务。
    ///
    /// 传入 Cron 。
    pub fn cron_use_croner<F, Fut>(cron: Cron, handler: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future + Send,
        Fut::Output: Send,
    {
        PLUGIN_BUILDER.with(|p| {
            Self::run_cron_task(p, cron, handler);
        })
    }

    fn run_cron_task<F, Fut>(p: &PluginBuilder, cron: Cron, handler: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future + Send,
        Fut::Output: Send,
    {
        let name = Arc::new(p.runtime_bot.plugin_name.clone());
        let mut enabled = {
            let bot = p.runtime_bot.bot.read().unwrap();
            let plugin = bot.plugins.get(&*name).unwrap();
            plugin.enabled.subscribe()
        };
        tokio::spawn(PLUGIN_NAME.scope(name.clone(), async move {

            tokio::select! {
                _ = async {
                        loop {
                            let now = chrono::Local::now();
                            let next = match cron.find_next_occurrence(&now, false) {
                                Ok(v) => v,
                                Err(e) => {
                                    error!("{name} cron task error: {}", e);
                                    break;
                                }
                            };
                            let time = next - now;
                            let duration = std::time::Duration::from_millis(time.num_milliseconds() as u64);
                            tokio::time::sleep(duration).await;
                            handler().await;
                        }
                } => {}
                _ = async {
                        loop {
                            enabled.changed().await.unwrap();
                            if !*enabled.borrow_and_update() {
                                break;
                            }
                        }
                } => {}
            }
        }));
    }
}

#[macro_export]
macro_rules! async_move {
    // 匹配没有事件参数的情况
    (;$($var:ident),*; $($body:tt)*) => {
        {
            $(let $var = $var.clone();)*
            move || {
                $(let $var = $var.clone();)*
                async move
                    $($body)*
            }
        }
    };

    // 匹配有事件参数的情况
    ($event:ident; $($var:ident),*; $($body:tt)*) => {
        {
            $(let $var = $var.clone();)*
            move |$event| {
                $(let $var = $var.clone();)*
                async move
                    $($body)*
            }
        }
    };

    // 匹配只要一次clone的情况（自己tokio::spawn一个新线程）
    ($($var:ident),*;$($body:tt)*) => {
        {
            $(let $var = $var.clone();)*
            async move
                $($body)*
        }
    };
}

#[cfg(test)]
mod on_is_ture {
    use crate::{
        bot::{plugin_builder::ListenMsgFn, ApiAndOneshot, PLUGIN_BUILDER},
        Bot, PluginBuilder,
    };
    use std::{
        net::{IpAddr, Ipv4Addr},
        sync::{Arc, RwLock},
    };
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn on_is_ture() {
        let conf = crate::bot::KoviConf::new(
            123,
            None,
            crate::bot::Server::new(
                crate::bot::Host::IpAddr(IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1))),
                8081,
                "".to_string(),
                false,
            ),
            false,
        );

        let (api_tx, _): (mpsc::Sender<ApiAndOneshot>, mpsc::Receiver<ApiAndOneshot>) =
            mpsc::channel(1);

        async fn test_something() {
            PluginBuilder::on_msg(|_| async {});
            PluginBuilder::on_admin_msg(|_| async {});
            PluginBuilder::on_group_msg(|_| async {});
            PluginBuilder::on_private_msg(|_| async {});
            PluginBuilder::on_all_notice(|_| async {});
            PluginBuilder::on_all_request(|_| async {});
            PluginBuilder::drop(|| async {});
        }

        fn pin_something() -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
            Box::pin(async {
                test_something().await;
            })
        }

        let mut bot = Bot::build(conf);
        bot.mount_main("some", "0.0.1", Arc::new(pin_something));
        let main_foo = bot.plugins.get("some").unwrap().main.clone();
        let bot = Arc::new(RwLock::new(bot));

        let p = PluginBuilder::new(
            "some".to_string(),
            bot.clone(),
            123,
            vec![],
            crate::bot::Host::IpAddr(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1))),
            8081,
            api_tx,
        );
        PLUGIN_BUILDER.scope(p, (main_foo)()).await;

        let bot_lock = bot.write().unwrap();
        let bot_plugin = bot_lock.plugins.get("some").unwrap();

        // 检测里面是不是每个类型的闭包都是一个
        let mut counts = std::collections::HashMap::new();
        counts.insert(
            "MsgFn",
            bot_plugin
                .listen
                .msg
                .iter()
                .filter(|&msg| matches!(msg.as_ref(), ListenMsgFn::Msg(_)))
                .count(),
        );
        counts.insert(
            "PrivateMsgFn",
            bot_plugin
                .listen
                .msg
                .iter()
                .filter(|&msg| matches!(msg.as_ref(), ListenMsgFn::PrivateMsg(_)))
                .count(),
        );
        counts.insert(
            "GroupMsgFn",
            bot_plugin
                .listen
                .msg
                .iter()
                .filter(|&msg| matches!(msg.as_ref(), ListenMsgFn::GroupMsg(_)))
                .count(),
        );
        counts.insert(
            "AdminMsgFn",
            bot_plugin
                .listen
                .msg
                .iter()
                .filter(|&msg| matches!(msg.as_ref(), ListenMsgFn::AdminMsg(_)))
                .count(),
        );
        counts.insert("AllNoticeFn", bot_plugin.listen.notice.len());
        counts.insert("AllRequestFn", bot_plugin.listen.request.len());
        counts.insert("KoviEventDropFn", bot_plugin.listen.drop.len());

        for (key, &count) in counts.iter() {
            assert_eq!(count, 1, "{} should have exactly one closure", key);
        }
    }
}
