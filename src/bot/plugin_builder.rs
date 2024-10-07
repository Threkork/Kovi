use super::run::PLUGIN_BUILDER;
use super::ApiAndOneshot;
use super::{runtimebot::RuntimeBot, Bot};
use croner::errors::CronError;
use croner::Cron;
use event::{AllMsgEvent, AllNoticeEvent, AllRequestEvent};
use log::error;
use std::future::Future;
use std::net::IpAddr;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;


pub mod event;

pub type PinFut = Pin<Box<dyn Future<Output = ()> + Send>>;

pub type AllMsgFn = Arc<dyn Fn(Arc<AllMsgEvent>) -> PinFut + Send + Sync + 'static>;

pub type AllNoticeFn = Arc<dyn Fn(Arc<AllNoticeEvent>) -> PinFut + Send + Sync + 'static>;

pub type AllRequestFn = Arc<dyn Fn(Arc<AllRequestEvent>) -> PinFut + Send + Sync + 'static>;

pub type NoArgsFn = Arc<dyn Fn() -> PinFut + Send + Sync + 'static>;

#[derive(Clone)]
pub enum ListenFn {
    MsgFn(AllMsgFn),

    PrivateMsgFn(AllMsgFn),

    GroupMsgFn(AllMsgFn),

    AdminMsgFn(AllMsgFn),

    AllNoticeFn(AllNoticeFn),

    AllRequestFn(AllRequestFn),

    KoviEventDropFn(NoArgsFn),
}


pub struct PluginBuilder {
    // pub(crate) cron_tx: mpsc::Sender<MpscCronTask>,
    runtime_bot: Arc<RuntimeBot>,
}

impl PluginBuilder {
    pub(crate) fn new(
        name: String,
        bot: Arc<RwLock<Bot>>,
        api_tx: mpsc::Sender<ApiAndOneshot>,
    ) -> Self {
        let (main_admin, admin, host, port) = {
            let bot_lock = bot.read().unwrap();
            (
                bot_lock.information.main_admin,
                bot_lock.information.admin.clone(),
                bot_lock.information.server.host,
                bot_lock.information.server.port,
            )
        };

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

    #[deprecated(note = "请使用 get_runtime_bot() 代替")]
    pub fn build_runtime_bot() -> RuntimeBot {
        PLUGIN_BUILDER.with(|p| (*p.runtime_bot).clone())
    }

    pub fn get_runtime_bot() -> Arc<RuntimeBot> {
        PLUGIN_BUILDER.with(|p| p.runtime_bot.clone())
    }

    #[deprecated(note = "请使用 RuntimeBot 的 get_data_path() 代替")]
    pub fn get_data_path() -> PathBuf {
        let mut current_dir = std::env::current_dir().unwrap();
        PLUGIN_BUILDER.with(|p| {
            current_dir.push(format!("data/{}", p.runtime_bot.plugin_name));
            current_dir
        })
    }

    pub fn get_kovi_bot() -> Arc<RwLock<Bot>> {
        PLUGIN_BUILDER.with(|p| p.runtime_bot.bot.clone())
    }

    pub fn get_plugin_name() -> String {
        PLUGIN_BUILDER.with(|p| p.runtime_bot.plugin_name.clone())
    }

    pub fn get_plugin_host() -> (IpAddr, u16) {
        PLUGIN_BUILDER.with(|p| (p.runtime_bot.host, p.runtime_bot.port))
    }
}

impl PluginBuilder {
    /// 注册消息处理函数。
    ///
    /// 注册一个处理程序，用于处理接收到的消息事件（`AllMsgEvent`）。
    pub fn on_msg<F, Fut>(handler: F)
    where
        F: Fn(Arc<AllMsgEvent>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        PLUGIN_BUILDER.with(|p| {
            let mut bot = p.runtime_bot.bot.write().unwrap();

            let bot_plugin = bot.plugins.get_mut(&p.runtime_bot.plugin_name).unwrap();

            bot_plugin
                .listen
                .push(ListenFn::MsgFn(Arc::new(move |event| {
                    Box::pin(handler(event))
                })));
        })
    }


    /// 注册管理员消息处理函数。
    ///
    /// 注册一个处理程序，用于处理接收到的消息事件（`AllMsgEvent`）。
    pub fn on_admin_msg<F, Fut>(handler: F)
    where
        F: Fn(Arc<AllMsgEvent>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        PLUGIN_BUILDER.with(|p| {
            let mut bot = p.runtime_bot.bot.write().unwrap();

            let bot_plugin = bot.plugins.get_mut(&p.runtime_bot.plugin_name).unwrap();

            bot_plugin
                .listen
                .push(ListenFn::AdminMsgFn(Arc::new(move |event| {
                    Box::pin(handler(event))
                })));
        })
    }

    /// 注册管理员消息处理函数。
    ///
    /// 注册一个处理程序，用于处理接收到的消息事件（`AllMsgEvent`）。
    pub fn on_private_msg<F, Fut>(handler: F)
    where
        F: Fn(Arc<AllMsgEvent>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        PLUGIN_BUILDER.with(|p| {
            let mut bot = p.runtime_bot.bot.write().unwrap();

            let bot_plugin = bot.plugins.get_mut(&p.runtime_bot.plugin_name).unwrap();

            bot_plugin
                .listen
                .push(ListenFn::PrivateMsgFn(Arc::new(move |event| {
                    Box::pin(handler(event))
                })));
        })
    }

    pub fn on_group_msg<F, Fut>(handler: F)
    where
        F: Fn(Arc<AllMsgEvent>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        PLUGIN_BUILDER.with(|p| {
            let mut bot = p.runtime_bot.bot.write().unwrap();

            let bot_plugin = bot.plugins.get_mut(&p.runtime_bot.plugin_name).unwrap();

            bot_plugin
                .listen
                .push(ListenFn::GroupMsgFn(Arc::new(move |event| {
                    Box::pin(handler(event))
                })));
        })
    }


    /// 注册消息处理函数。
    ///
    /// 注册一个处理程序，用于处理接收到的消息事件（`AllNoticeEvent`）。
    pub fn on_all_notice<F, Fut>(handler: F)
    where
        F: Fn(Arc<AllNoticeEvent>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        PLUGIN_BUILDER.with(|p| {
            let mut bot = p.runtime_bot.bot.write().unwrap();

            let bot_plugin = bot.plugins.get_mut(&p.runtime_bot.plugin_name).unwrap();

            bot_plugin
                .listen
                .push(ListenFn::AllNoticeFn(Arc::new(move |event| {
                    Box::pin(handler(event))
                })));
        })
    }

    /// 注册异步消息处理函数。
    ///
    /// 注册一个处理程序，用于处理接收到的消息事件（`AllRequestEvent`）。
    pub fn on_all_request<F, Fut>(handler: F)
    where
        F: Fn(Arc<AllRequestEvent>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        PLUGIN_BUILDER.with(|p| {
            let mut bot = p.runtime_bot.bot.write().unwrap();

            let bot_plugin = bot.plugins.get_mut(&p.runtime_bot.plugin_name).unwrap();

            bot_plugin
                .listen
                .push(ListenFn::AllRequestFn(Arc::new(move |event| {
                    Box::pin(handler(event))
                })));
        })
    }

    /// 注册程序结束事件处理函数。
    ///
    /// 注册处理程序，用于处理接收到的程序结束事件。
    pub fn drop<F, Fut>(handler: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        PLUGIN_BUILDER.with(|p| {
            let mut bot = p.runtime_bot.bot.write().unwrap();

            let bot_plugin = bot.plugins.get_mut(&p.runtime_bot.plugin_name).unwrap();

            bot_plugin
                .listen
                .push(ListenFn::KoviEventDropFn(Arc::new(move || {
                    Box::pin(handler())
                })));
        })
    }

    /// 注册定时任务。
    ///
    /// 传入 Cron 。
    pub fn cron<F, Fut>(cron: &str, handler: F) -> Result<(), CronError>
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        PLUGIN_BUILDER.with(|p| {
            let cron = match Cron::new(cron).with_seconds_optional().parse() {
                Ok(v) => v,
                Err(e) => return Err(e),
            };
            let name = p.runtime_bot.plugin_name.clone();
            tokio::spawn(async move {
                loop {
                    let now = chrono::Local::now();
                    let next = match cron.find_next_occurrence(&now, false) {
                        Ok(v) => v,
                        Err(e) => {
                            error!("{name} cron task error: {}", e);
                            continue;
                        }
                    };
                    let time = next - now;
                    let duration = std::time::Duration::from_millis(time.num_milliseconds() as u64);
                    tokio::time::sleep(duration).await;
                    handler().await;
                }
            });
            Ok(())
        })
    }

    /// 注册定时任务。
    ///
    /// 传入 Cron 。
    pub fn cron_use_croner<F, Fut>(cron: Cron, handler: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        PLUGIN_BUILDER.with(|p| {
            let name = p.runtime_bot.plugin_name.clone();
            tokio::spawn(async move {
                loop {
                    let now = chrono::Local::now();
                    let next = match cron.find_next_occurrence(&now, false) {
                        Ok(v) => v,
                        Err(e) => {
                            error!("{name} cron task error: {}", e);
                            continue;
                        }
                    };
                    let time = next - now;
                    let duration = std::time::Duration::from_millis(time.num_milliseconds() as u64);
                    tokio::time::sleep(duration).await;
                    handler().await;
                }
            });
        })
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
