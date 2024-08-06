use super::ApiMpsc;
use super::{runtimebot::RuntimeBot, Bot};
use event::{Event, OnAllNoticeEvent, OnMsgEvent};
use std::error::Error as stdError;
use std::fmt;
use std::sync::{Arc, RwLock};
use std::{net::IpAddr, sync::mpsc};
use thiserror::Error;

pub mod event;

pub type ListenFn = Arc<dyn Fn(&Event) -> Result<(), PluginError> + Send + Sync + 'static>;

#[derive(Error, Debug)]
pub enum PluginBuilderError {
    #[error("The information of the plugin is not set correctly")]
    InfoError(),
}

#[derive(Debug)]
pub struct PluginError {
    id: String,
    error: String,
}

impl PluginError {
    pub fn new<E>(id: String, error: E) -> Self
    where
        E: fmt::Display + stdError,
    {
        PluginError {
            id,
            error: error.to_string(),
        }
    }
}

impl fmt::Display for PluginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PluginError {}: {}", self.id, self.error)
    }
}

impl stdError for PluginError {
}

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
    pub handler: ListenFn,
}

pub struct PluginBuilder {
    pub name: Option<String>,

    bot: Arc<RwLock<Bot>>,
    host: IpAddr,
    port: u16,
    api_tx: mpsc::Sender<ApiMpsc>,
}

impl PluginBuilder {
    pub fn new(bot: Arc<RwLock<Bot>>, api_tx: mpsc::Sender<ApiMpsc>) -> Self {
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

    /// 创建 plugin 错误, 但是还是要注意 set_info() 要设置好, 否则这个闭包线程会 **panic!()**
    pub fn error<E>(&self, error: E) -> PluginError
    where
        E: fmt::Display + stdError,
    {
        let id = self.name.clone();
        PluginError::new(id.unwrap(), error)
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
    pub fn on_msg<F>(&mut self, handler: F) -> Result<(), PluginBuilderError>
    where
        F: Fn(&OnMsgEvent) -> Result<(), PluginError> + Send + Sync + 'static,
    {
        if self.name.is_none() {
            return Err(PluginBuilderError::InfoError());
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
        Ok(())
    }

    /// 注册消息处理函数。
    ///
    /// 注册一个处理程序（handler），用于处理接收到的消息事件（`OnMsgEvent`）。
    /// 接收闭包，要求函数接受 `OnMsgEvent` 类型的参数，并返回 `Result` 类型。
    /// 闭包必须实现 `Send` 、 `Sync`和 `'static`，因为要保证多线程安全以及在确保闭包在整个程序生命周期有效。
    pub fn on_admin_msg<F>(&mut self, handler: F) -> Result<(), PluginBuilderError>
    where
        F: Fn(&OnMsgEvent) -> Result<(), PluginError> + Send + Sync + 'static,
    {
        if self.name.is_none() {
            return Err(PluginBuilderError::InfoError());
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
        Ok(())
    }

    /// 注册消息处理函数。
    ///
    /// 注册一个处理程序（handler），用于处理接收到的消息事件（`on_all_notice`）。
    /// 接收闭包，要求函数接受 `OnAllNoticeEvent` 类型的参数，并返回 `Result` 类型。
    /// 闭包必须实现 `Send` 、 `Sync`和 `'static`，因为要保证多线程安全以及在确保闭包在整个程序生命周期有效。
    pub fn on_all_notice<F>(&mut self, handler: F) -> Result<(), PluginBuilderError>
    where
        F: Fn(&OnAllNoticeEvent) -> Result<(), PluginError> + Send + Sync + 'static,
    {
        if self.name.is_none() {
            return Err(PluginBuilderError::InfoError());
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
        Ok(())
    }
}
