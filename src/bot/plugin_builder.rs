use super::ApiOneshot;
use super::{runtimebot::RuntimeBot, Bot};
use event::{AllMsgEvent, AllNoticeEvent, AllRequestEvent};
use std::future::Future;
use std::net::IpAddr;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;


pub mod event;

pub type PinFut = Pin<Box<dyn Future<Output = ()> + Send>>;

pub type MsgFn = Arc<dyn Fn(Arc<AllMsgEvent>) -> PinFut + Send + Sync + 'static>;

pub type AdminMsgFn = Arc<dyn Fn(Arc<AllMsgEvent>) -> PinFut + Send + Sync + 'static>;

pub type AllNoticeFn = Arc<dyn Fn(Arc<AllNoticeEvent>) -> PinFut + Send + Sync + 'static>;

pub type AllRequestFn = Arc<dyn Fn(Arc<AllRequestEvent>) -> PinFut + Send + Sync + 'static>;

pub type KoviDropEventFn = Arc<dyn Fn() -> PinFut + Send + Sync + 'static>;

#[derive(Clone)]
pub enum ListenFn {
    MsgFn(MsgFn),

    AdminMsgFn(AdminMsgFn),

    AllNoticeFn(AllNoticeFn),

    AllRequestFn(AllRequestFn),

    KoviEventDropFn(KoviDropEventFn),
}


pub struct PluginBuilder {
    pub name: String,
    pub host: IpAddr,
    pub port: u16,

    bot: Arc<RwLock<Bot>>,
    api_tx: mpsc::Sender<ApiOneshot>,
}

impl PluginBuilder {
    pub fn new(name: String, bot: Arc<RwLock<Bot>>, api_tx: mpsc::Sender<ApiOneshot>) -> Self {
        let (host, port) = {
            let bot_lock = bot.read().unwrap();
            (
                bot_lock.information.server.host,
                bot_lock.information.server.port,
            )
        };
        {
            let bot = bot.clone();
            let mut bot_lock = bot.write().unwrap();
            bot_lock.plugins.insert(name.clone(), Vec::new());
        }
        PluginBuilder {
            name,
            bot,
            host,
            port,
            api_tx,
        }
    }

    pub fn build_runtime_bot(&self) -> RuntimeBot {
        RuntimeBot {
            main_admin: self.bot.read().unwrap().information.main_admin,
            admin: self.bot.read().unwrap().information.admin.clone(),
            host: self.host,
            port: self.port,
            api_tx: self.api_tx.clone(),
        }
    }

    pub fn get_data_path(&self) -> PathBuf {
        let mut current_dir = std::env::current_dir().unwrap();
        current_dir.push(format!("data/{}", self.name));
        current_dir
    }
}

impl PluginBuilder {
    /// 注册消息处理函数。
    ///
    /// 注册一个处理程序，用于处理接收到的消息事件（`AllMsgEvent`）。
    pub fn on_msg<F, Fut>(&mut self, handler: F)
    where
        F: Fn(Arc<AllMsgEvent>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let mut bot = self.bot.write().unwrap();

        let all_listen = bot.plugins.get_mut(&self.name).unwrap();

        all_listen.push(ListenFn::MsgFn(Arc::new(move |event| {
            Box::pin(handler(event))
        })));
    }

    /// 注册管理员消息处理函数。
    ///
    /// 注册一个处理程序，用于处理接收到的消息事件（`AllMsgEvent`）。
    pub fn on_admin_msg_async<F, Fut>(&mut self, handler: F)
    where
        F: Fn(Arc<AllMsgEvent>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let mut bot = self.bot.write().unwrap();

        let all_listen = bot.plugins.get_mut(&self.name).unwrap();

        all_listen.push(ListenFn::AdminMsgFn(Arc::new(move |event| {
            Box::pin(handler(event))
        })));
    }


    /// 注册消息处理函数。
    ///
    /// 注册一个处理程序，用于处理接收到的消息事件（`AllNoticeEvent`）。
    pub fn on_all_notice<F, Fut>(&mut self, handler: F)
    where
        F: Fn(Arc<AllNoticeEvent>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let mut bot = self.bot.write().unwrap();

        let all_listen = bot.plugins.get_mut(&self.name).unwrap();

        all_listen.push(ListenFn::AllNoticeFn(Arc::new(move |event| {
            Box::pin(handler(event))
        })));
    }

    /// 注册异步消息处理函数。
    ///
    /// 注册一个处理程序，用于处理接收到的消息事件（`AllRequestEvent`）。
    pub fn on_all_request<F, Fut>(&mut self, handler: F)
    where
        F: Fn(Arc<AllRequestEvent>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let mut bot = self.bot.write().unwrap();

        let all_listen = bot.plugins.get_mut(&self.name).unwrap();

        all_listen.push(ListenFn::AllRequestFn(Arc::new(move |event| {
            Box::pin(handler(event))
        })));
    }

    /// 注册程序结束事件处理函数。
    ///
    /// 注册处理程序，用于处理接收到的程序结束事件。
    pub fn drop<F, Fut>(&mut self, handler: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let mut bot = self.bot.write().unwrap();

        let all_listen = bot.plugins.get_mut(&self.name).unwrap();

        all_listen.push(ListenFn::KoviEventDropFn(Arc::new(move || {
            Box::pin(handler())
        })));
    }
}
