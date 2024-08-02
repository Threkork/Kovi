use super::{runtimebot::RuntimeBot, Bot};
use event::{Event, OnAllNoticeEvent, OnMsgEvent};
use serde_json::Value;
use std::sync::{Arc, RwLock};
use std::{net::IpAddr, sync::mpsc};
pub mod event;

#[derive(Clone)]
pub struct Plugin {
    pub name: String,
    pub all_listen: Vec<Listen>,
}

#[derive(Copy, Clone)]
pub enum OnType {
    OnMsg,
    OnAdminMsg,
    OnAllNotice,
}

#[derive(Clone)]
pub struct Listen {
    pub on_type: OnType,
    pub handler: Arc<dyn Fn(&Event) -> Result<(), ()> + Send + Sync + 'static>,
}

pub struct PluginBuilder {
    pub name: Option<String>,

    bot: Arc<RwLock<Bot>>,
    host: IpAddr,
    port: u16,
    api_tx: mpsc::Sender<Value>,
}

impl PluginBuilder {
    pub fn new(bot: Arc<RwLock<Bot>>, api_tx: mpsc::Sender<Value>) -> Self {
        let (host, port) = {
            let bot_lock = bot.read().unwrap();
            (
                bot_lock.information.server.host,
                bot_lock.information.server.port,
            )
        };
        let bot = bot.clone();
        PluginBuilder {
            name: Option::None,

            bot,
            host,
            port,
            api_tx,
        }
    }

    pub fn set_info(&mut self, name: &str) {
        self.name = Some(name.to_string());

        let mut bot_lock = self.bot.write().unwrap();
        bot_lock.plugins.push(Plugin {
            name: name.to_string(),
            all_listen: Vec::new(),
        });
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
}

impl PluginBuilder {
    /// 注册消息处理函数。
    ///
    /// 注册一个处理程序（handler），用于处理接收到的消息事件（`OnMsgEvent`）。
    /// 接收闭包，要求函数接受 `OnMsgEvent` 类型的参数，并返回 `Result` 类型。
    /// 闭包必须实现 `Send` 、 `Sync`和 `'static`，因为要保证多线程安全以及在确保闭包在整个程序生命周期有效。
    pub fn on_msg<F>(&mut self, handler: F) -> Result<(), ()>
    where
        F: Fn(&OnMsgEvent) -> Result<(), ()> + Send + Sync + 'static,
    {
        if self.name == None {
            return Err(());
        }
        let bot = self.bot.clone();
        for plugin in &mut bot.write().unwrap().plugins {
            if let Some(name) = &self.name {
                if plugin.name != *name {
                    continue;
                }
            }
            plugin.all_listen.push(Listen {
                on_type: OnType::OnMsg,
                handler: Arc::new(move |event| {
                    if let Event::OnMsg(e) = event {
                        handler(e)
                    } else {
                        panic!()
                    }
                }),
            });
            return Ok(());
        }
        return Ok(());
    }

    /// 注册消息处理函数。
    ///
    /// 注册一个处理程序（handler），用于处理接收到的消息事件（`OnMsgEvent`）。
    /// 接收闭包，要求函数接受 `OnMsgEvent` 类型的参数，并返回 `Result` 类型。
    /// 闭包必须实现 `Send` 、 `Sync`和 `'static`，因为要保证多线程安全以及在确保闭包在整个程序生命周期有效。
    pub fn on_admin_msg<F>(&mut self, handler: F) -> Result<(), ()>
    where
        F: Fn(&OnMsgEvent) -> Result<(), ()> + Send + Sync + 'static,
    {
        if self.name == None {
            return Err(());
        }
        let bot = self.bot.clone();
        for plugin in &mut bot.write().unwrap().plugins {
            if let Some(name) = &self.name {
                if plugin.name != *name {
                    continue;
                }
            }
            plugin.all_listen.push(Listen {
                on_type: OnType::OnAdminMsg,
                handler: Arc::new(move |event| {
                    if let Event::OnMsg(e) = event {
                        handler(e)
                    } else {
                        panic!()
                    }
                }),
            });
            return Ok(());
        }
        return Ok(());
    }

    /// 注册消息处理函数。
    ///
    /// 注册一个处理程序（handler），用于处理接收到的消息事件（`on_all_notice`）。
    /// 接收闭包，要求函数接受 `OnAllNoticeEvent` 类型的参数，并返回 `Result` 类型。
    /// 闭包必须实现 `Send` 、 `Sync`和 `'static`，因为要保证多线程安全以及在确保闭包在整个程序生命周期有效。
    pub fn on_all_notice<F>(&mut self, handler: F) -> Result<(), ()>
    where
        F: Fn(&OnAllNoticeEvent) -> Result<(), ()> + Send + Sync + 'static,
    {
        if self.name == None {
            return Err(());
        }
        let bot = self.bot.clone();
        for plugin in &mut bot.write().unwrap().plugins {
            if let Some(name) = &self.name {
                if plugin.name != *name {
                    continue;
                }
            }
            plugin.all_listen.push(Listen {
                on_type: OnType::OnAllNotice,
                handler: Arc::new(move |event| {
                    if let Event::OnAllNotice(e) = event {
                        handler(e)
                    } else {
                        panic!()
                    }
                }),
            });
            return Ok(());
        }
        return Ok(());
    }
}
