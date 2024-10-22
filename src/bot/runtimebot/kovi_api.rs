use tokio::sync::mpsc;

use super::RuntimeBot;
use crate::{
    bot::ApiAndOneshot,
    error::BotError,
    task::{PLUGIN_NAME, TASK_MANAGER},
    Bot, PluginBuilder,
};
use std::{
    path::PathBuf,
    sync::{Arc, RwLock},
};

pub trait KoviApi {
    /// 获取插件自己的路径
    fn get_data_path(&self) -> PathBuf;

    /// 卸载传入的插件
    ///
    /// 并且因为要运行插件可能存在的 drop 闭包，所以需要异步。
    ///
    /// # error
    ///
    /// 如果寻找不到插件，会报错 BotError::PluginNotFound
    fn disable_plugin<T: AsRef<str> + std::marker::Send>(
        &self,
        plugin_name: T,
    ) -> Result<(), BotError>;

    /// 启用传入的插件
    ///
    /// # error
    ///
    /// 如果寻找不到插件，会报错 BotError::PluginNotFound
    fn enable_plugin<T: AsRef<str>>(&self, plugin_name: T) -> Result<(), BotError>;

    fn is_plugin_enable<T: AsRef<str>>(&self, plugin_name: T) -> Result<bool, BotError>;
}

impl KoviApi for RuntimeBot {
    fn get_data_path(&self) -> PathBuf {
        let mut current_dir = std::env::current_dir().unwrap();

        current_dir.push(format!("data/{}", self.plugin_name));
        current_dir
    }

    fn disable_plugin<T: AsRef<str> + std::marker::Send>(
        &self,
        plugin_name: T,
    ) -> Result<(), BotError> {
        if !self.is_plugin_enable(&plugin_name)? {
            return Ok(());
        }

        disable_plugin(self.bot.clone(), plugin_name)
    }

    fn enable_plugin<T: AsRef<str>>(&self, plugin_name: T) -> Result<(), BotError> {
        if self.is_plugin_enable(&plugin_name)? {
            return Ok(());
        }

        enable_plugin(self.bot.clone(), plugin_name, self.api_tx.clone())
    }

    fn is_plugin_enable<T: AsRef<str>>(&self, plugin_name: T) -> Result<bool, BotError> {
        let bot = self.bot.read().unwrap();
        let plugin_name = plugin_name.as_ref();

        let bot_plugin = match bot.plugins.get(plugin_name) {
            Some(v) => v,
            None => return Err(BotError::PluginNotFound(plugin_name.to_string())),
        };
        let bool_ = *bot_plugin.enabled.borrow();
        Ok(bool_)
    }
}


fn disable_plugin<T: AsRef<str>>(bot: Arc<RwLock<Bot>>, plugin_name: T) -> Result<(), BotError> {
    let mut join_handles = Vec::new();
    {
        let mut bot = bot.write().unwrap();

        let plugin_name = plugin_name.as_ref();

        let bot_plugin = match bot.plugins.get_mut(plugin_name) {
            Some(v) => v,
            None => return Err(BotError::PluginNotFound(plugin_name.to_string())),
        };


        let plugin_name_ = Arc::new(plugin_name.to_string());
        for listen in &bot_plugin.listen.drop {
            let listen_clone = listen.clone();
            let plugin_name_ = plugin_name_.clone();
            let handle = tokio::spawn(async move {
                PLUGIN_NAME
                    .scope(plugin_name_, Bot::handler_drop(listen_clone))
                    .await;
            });
            join_handles.push(handle);
        }

        TASK_MANAGER.disable_plugin(plugin_name);

        bot_plugin.enabled.send_modify(|v| {
            *v = false;
        });
        bot_plugin.listen.clear();
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
            bot_read.information.server.host,
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
