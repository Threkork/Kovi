use super::RuntimeBot;
use crate::{bot::ApiAndOneshot, error::BotError, Bot, PluginBuilder};
use std::{
    path::PathBuf,
    sync::{Arc, RwLock},
};
use tokio::sync::mpsc;

impl RuntimeBot {
    /// 获取插件自己的路径
    pub fn get_data_path(&self) -> PathBuf {
        let mut current_dir = std::env::current_dir().unwrap();

        current_dir.push(format!("data/{}", self.plugin_name));
        current_dir
    }

    /// 卸载传入的插件
    ///
    /// # error
    ///
    /// 如果寻找不到插件，会报错 BotError::PluginNotFound
    pub fn disable_plugin<T: AsRef<str> + std::marker::Send>(
        &self,
        plugin_name: T,
    ) -> Result<(), BotError> {
        if !self.is_plugin_enable(&plugin_name)? {
            return Ok(());
        }

        let bot = match self.bot.upgrade() {
            Some(b) => b,
            None => return Err(BotError::RefExpired),
        };

        disable_plugin(bot, plugin_name)
    }

    /// 启用传入的插件
    ///
    /// # error
    ///
    /// 如果寻找不到插件，会报错 BotError::PluginNotFound
    pub fn enable_plugin<T: AsRef<str>>(&self, plugin_name: T) -> Result<(), BotError> {
        if self.is_plugin_enable(&plugin_name)? {
            return Ok(());
        }

        let bot = match self.bot.upgrade() {
            Some(b) => b,
            None => return Err(BotError::RefExpired),
        };

        enable_plugin(bot, plugin_name, self.api_tx.clone())
    }

    /// 插件是否开启
    ///
    /// # error
    ///
    /// 如果寻找不到插件，会报错 BotError::PluginNotFound
    pub fn is_plugin_enable<T: AsRef<str>>(&self, plugin_name: T) -> Result<bool, BotError> {
        let bot = match self.bot.upgrade() {
            Some(b) => b,
            None => return Err(BotError::RefExpired),
        };

        let bot = bot.read().unwrap();
        let plugin_name = plugin_name.as_ref();

        let bot_plugin = match bot.plugins.get(plugin_name) {
            Some(v) => v,
            None => return Err(BotError::PluginNotFound(plugin_name.to_string())),
        };
        let bool_ = *bot_plugin.enabled.borrow();
        Ok(bool_)
    }
}

pub(crate) fn disable_plugin<T: AsRef<str>>(
    bot: Arc<RwLock<Bot>>,
    plugin_name: T,
) -> Result<(), BotError> {
    {
        let mut bot = bot.write().unwrap();

        let plugin_name = plugin_name.as_ref();

        let bot_plugin = match bot.plugins.get_mut(plugin_name) {
            Some(v) => v,
            None => return Err(BotError::PluginNotFound(plugin_name.to_string())),
        };
        bot_plugin.shutdown();
    }

    Ok(())
}

fn enable_plugin<T: AsRef<str>>(
    bot: Arc<RwLock<Bot>>,
    plugin_name: T,
    api_tx: mpsc::Sender<ApiAndOneshot>,
) -> Result<(), BotError> {
    let bot_read = bot.read().unwrap();
    let plugin_name = plugin_name.as_ref();

    let (main_admin, admin, host, port) = {
        (
            bot_read.information.main_admin,
            bot_read.information.admin.clone(),
            bot_read.information.server.host.clone(),
            bot_read.information.server.port,
        )
    };

    let bot_plugin = match bot_read.plugins.get(plugin_name) {
        Some(v) => v,
        None => return Err(BotError::PluginNotFound(plugin_name.to_string())),
    };

    bot_plugin.enabled.send_modify(|v| {
        *v = true;
    });

    let plugin_ = bot_plugin.clone();

    let plugin_builder = PluginBuilder::new(
        plugin_name.to_string(),
        bot.clone(),
        main_admin,
        admin,
        host,
        port,
        api_tx,
    );

    tokio::spawn(async move { Bot::run_plugin_main(&plugin_, plugin_builder) });

    Ok(())
}
