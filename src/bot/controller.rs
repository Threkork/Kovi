use std::sync::Arc;

use crate::{
    error::BotError,
    task::{PLUGIN_NAME, TASK_MANAGER},
};

use super::Bot;

impl Bot {
    /// 卸载传入的插件
    ///
    /// 并且因为要运行插件可能存在的 drop 闭包，所以需要异步。
    ///
    /// # error
    ///
    /// 如果寻找不到插件，会报错 BotError::PluginNotFound
    pub async fn disable_plugin<T: AsRef<str>>(&mut self, plugin_name: T) -> Result<(), BotError> {
        let plugin_name = plugin_name.as_ref();

        let bot_plugin = match self.plugins.get_mut(plugin_name) {
            Some(v) => v,
            None => return Err(BotError::PluginNotFound(plugin_name.to_string())),
        };


        let mut join_handles = Vec::new();
        let plugin_name_ = Arc::new(plugin_name.to_string());
        for listen in &bot_plugin.listen.drop {
            let listen_clone = listen.clone();
            let plugin_name_ = plugin_name_.clone();
            let handle = tokio::spawn(async move {
                PLUGIN_NAME.scope(plugin_name_, Self::handler_drop(listen_clone));
            });
            join_handles.push(handle);
        }

        for handle in join_handles {
            handle.await.unwrap();
        }

        TASK_MANAGER.disable_plugin(plugin_name);

        bot_plugin.enabled.send(false).unwrap();
        bot_plugin.listen.clear();

        Ok(())
    }

    pub fn enable_plugin<T: AsRef<str>>(&mut self, plugin_name: T) -> Result<(), BotError> {
        let plugin_name = plugin_name.as_ref();

        let bot_plugin = match self.plugins.get(plugin_name) {
            Some(v) => v,
            None => return Err(BotError::PluginNotFound(plugin_name.to_string())),
        };

        bot_plugin.enabled.send(true).unwrap();

        Self::run_plugin_main(bot_plugin);

        Ok(())
    }
}
