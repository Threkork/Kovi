use super::ApiMpsc;
use super::{runtimebot::RuntimeBot, Bot};
use event::{AllMsgEvent, AllNoticeEvent, AllRequestEvent, OneBotEvent};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::{net::IpAddr, sync::mpsc};


pub mod event;

pub type OneBotEventFn = Arc<dyn Fn(&OneBotEvent) + Send + Sync + 'static>;
pub type KoviDropEventFn = Arc<dyn Fn() + Send + Sync + 'static>;

#[derive(Clone)]
pub enum ListenFn {
    MsgFn(OneBotEventFn),
    AdminMsg(OneBotEventFn),
    AllNotice(OneBotEventFn),
    AllRequest(OneBotEventFn),
    KoviEventDrop(KoviDropEventFn),
}


pub struct PluginBuilder {
    pub name: String,
    pub host: IpAddr,
    pub port: u16,

    bot: Arc<RwLock<Bot>>,
    api_tx: mpsc::Sender<ApiMpsc>,
}

impl PluginBuilder {
    pub fn new(name: String, bot: Arc<RwLock<Bot>>, api_tx: mpsc::Sender<ApiMpsc>) -> Self {
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
    pub fn on_msg<F>(&mut self, handler: F)
    where
        F: Fn(&AllMsgEvent) + Send + Sync + 'static,
    {
        let mut bot = self.bot.write().unwrap();

        let all_listen = bot.plugins.get_mut(&self.name).unwrap();

        all_listen.push(ListenFn::MsgFn(Arc::new(move |event| {
            if let OneBotEvent::OnMsg(e) = event {
                handler(e)
            } else {
                panic!()
            }
        })));
    }

    /// 注册管理员消息处理函数。
    ///
    /// 注册一个处理程序，用于处理接收到的消息事件（`AllMsgEvent`）。
    pub fn on_admin_msg<F>(&mut self, handler: F)
    where
        F: Fn(&AllMsgEvent) + Send + Sync + 'static,
    {
        let mut bot = self.bot.write().unwrap();

        let all_listen = bot.plugins.get_mut(&self.name).unwrap();

        all_listen.push(ListenFn::AdminMsg(Arc::new(move |event| {
            if let OneBotEvent::OnMsg(e) = event {
                handler(e)
            } else {
                panic!()
            }
        })));
    }

    /// 注册消息处理函数。
    ///
    /// 注册一个处理程序，用于处理接收到的消息事件（`AllNoticeEvent`）。
    pub fn on_all_notice<F>(&mut self, handler: F)
    where
        F: Fn(&AllNoticeEvent) + Send + Sync + 'static,
    {
        let mut bot = self.bot.write().unwrap();

        let all_listen = bot.plugins.get_mut(&self.name).unwrap();

        all_listen.push(ListenFn::AllNotice(Arc::new(move |event| {
            if let OneBotEvent::OnAllNotice(e) = event {
                handler(e)
            } else {
                panic!()
            }
        })));
    }

    /// 注册消息处理函数。
    ///
    /// 注册一个处理程序，用于处理接收到的消息事件（`AllRequestEvent`）。
    pub fn on_all_request<F>(&mut self, handler: F)
    where
        F: Fn(&AllRequestEvent) + Send + Sync + 'static,
    {
        let mut bot = self.bot.write().unwrap();

        let all_listen = bot.plugins.get_mut(&self.name).unwrap();

        all_listen.push(ListenFn::AllRequest(Arc::new(move |event| {
            if let OneBotEvent::OnAllRequest(e) = event {
                handler(e)
            } else {
                panic!()
            }
        })));
    }

    /// 注册程序结束事件处理函数。
    ///
    /// 注册处理程序，用于处理接收到的程序结束事件。
    pub fn drop<F>(&mut self, handler: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        let mut bot = self.bot.write().unwrap();

        let all_listen = bot.plugins.get_mut(&self.name).unwrap();

        all_listen.push(ListenFn::KoviEventDrop(Arc::new(handler)));
    }
}
