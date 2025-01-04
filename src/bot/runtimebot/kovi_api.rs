use super::RuntimeBot;
use crate::{
    bot::{ApiAndOneshot, PluginInfo},
    error::BotError,
    Bot, PluginBuilder,
};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{Arc, RwLock},
};
use tokio::sync::mpsc;

#[deprecated(since = "0.11.0", note = "弃用，直接删掉就好了")]
pub trait KoviApi {}

#[derive(Debug, Clone)]
pub enum SetAdmin {
    /// 增加一个管理员
    Add(i64),
    /// 增加多个管理员
    Adds(Vec<i64>),
    /// 移除一个管理员
    Remove(i64),
    /// 移除多个管理员
    Removes(Vec<i64>),
    /// 替换管理员成此管理员
    Changes(Vec<i64>),
}

#[derive(Debug, Clone)]
pub enum SetAccessControlList {
    /// 增加一个名单
    Add(i64),
    /// 增加多个名单
    Adds(Vec<i64>),
    /// 移除一个名单
    Remove(i64),
    /// 移除多个名单
    Removes(Vec<i64>),
    /// 替换名单成此名单
    Changes(Vec<i64>),
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum AccessControlMode {
    BlackList,
    WhiteList,
}

/// 黑白名单
#[cfg(feature = "plugin-access-control")]
impl RuntimeBot {
    /// 为某一插件启动名单
    ///
    /// # error
    ///
    /// 如果寻找不到插件，会返回Err `BotError::PluginNotFound`
    ///
    /// 如果此 `RuntimeBot` 实例内部的 `Bot` 中已经不存在，将会返回Err `BotError::RefExpired` 。
    /// 这通常出现在 `Bot` 已经关闭，可有个不受 Kovi 管理的线程仍然拥有此 `RuntimeBot`。
    pub fn set_plugin_access_control<T: AsRef<str>>(
        &self,
        plugin_name: T,
        enable: bool,
    ) -> Result<(), BotError> {
        let bot = match self.bot.upgrade() {
            Some(b) => b,
            None => return Err(BotError::RefExpired),
        };

        let mut bot = bot.write().unwrap();

        let plugin_name = plugin_name.as_ref();

        let plugin = match bot.plugins.get_mut(plugin_name) {
            Some(v) => v,
            None => return Err(BotError::PluginNotFound(plugin_name.to_string())),
        };

        plugin.access_control = enable;

        Ok(())
    }

    /// 更改名单为其他模式，插件默认为白名单模式
    ///
    /// # error
    ///
    /// 如果寻找不到插件，会返回Err `BotError::PluginNotFound`
    ///
    /// 如果此 `RuntimeBot` 实例内部的 `Bot` 中已经不存在，将会返回Err `BotError::RefExpired` 。
    /// 这通常出现在 `Bot` 已经关闭，可有个不受 Kovi 管理的线程仍然拥有此 `RuntimeBot`。
    pub fn set_plugin_access_control_mode<T: AsRef<str>>(
        &self,
        plugin_name: T,
        access_control_mode: AccessControlMode,
    ) -> Result<(), BotError> {
        let bot = match self.bot.upgrade() {
            Some(b) => b,
            None => return Err(BotError::RefExpired),
        };

        let mut bot = bot.write().unwrap();

        let plugin_name = plugin_name.as_ref();

        let plugin = match bot.plugins.get_mut(plugin_name) {
            Some(v) => v,
            None => return Err(BotError::PluginNotFound(plugin_name.to_string())),
        };

        plugin.list_mode = access_control_mode;

        Ok(())
    }

    /// 为某一插件添加名单
    ///
    /// is_group为true时，为群组名单，为false时为好友名单
    ///
    /// # error
    ///
    /// 如果寻找不到插件，会返回Err `BotError::PluginNotFound`
    ///
    /// 如果此 `RuntimeBot` 实例内部的 `Bot` 中已经不存在，将会返回Err `BotError::RefExpired` 。
    /// 这通常出现在 `Bot` 已经关闭，可有个不受 Kovi 管理的线程仍然拥有此 `RuntimeBot`。
    pub fn set_plugin_access_control_list<T: AsRef<str>>(
        &self,
        plugin_name: T,
        is_group: bool,
        change: SetAccessControlList,
    ) -> Result<(), BotError> {
        let bot = match self.bot.upgrade() {
            Some(b) => b,
            None => return Err(BotError::RefExpired),
        };

        let mut bot = bot.write().unwrap();

        let plugin_name = plugin_name.as_ref();

        let plugin = match bot.plugins.get_mut(plugin_name) {
            Some(v) => v,
            None => return Err(BotError::PluginNotFound(plugin_name.to_string())),
        };

        match (change, is_group) {
            // 添加一个群组到名单
            (SetAccessControlList::Add(id), true) => {
                plugin.access_list.groups.insert(id);
            }
            // 添加多个群组到名单
            (SetAccessControlList::Adds(ids), true) => {
                for id in ids {
                    plugin.access_list.groups.insert(id);
                }
            }
            // 从名单中移除一个群组
            (SetAccessControlList::Remove(id), true) => {
                plugin.access_list.groups.remove(&id);
            }
            // 从名单中移除多个群组
            (SetAccessControlList::Removes(ids), true) => {
                for id in ids {
                    plugin.access_list.groups.remove(&id);
                }
            }
            // 替换名单为新的群组列表
            (SetAccessControlList::Changes(ids), true) => {
                plugin.access_list.groups = ids.into_iter().collect();
            }
            // 添加一个用户到名单
            (SetAccessControlList::Add(id), false) => {
                plugin.access_list.friends.insert(id);
            }
            // 添加多个用户到名单
            (SetAccessControlList::Adds(ids), false) => {
                for id in ids {
                    plugin.access_list.friends.insert(id);
                }
            }
            // 从名单中移除一个用户
            (SetAccessControlList::Remove(id), false) => {
                plugin.access_list.friends.remove(&id);
            }
            // 从名单中移除多个用户
            (SetAccessControlList::Removes(ids), false) => {
                for id in ids {
                    plugin.access_list.friends.remove(&id);
                }
            }
            // 替换名单为新的用户列表
            (SetAccessControlList::Changes(ids), false) => {
                plugin.access_list.friends = ids.into_iter().collect();
            }
        }

        Ok(())
    }
}

/// 管理员控制
impl RuntimeBot {
    /// 修改Bot的管理员
    ///
    /// # Error
    ///
    /// 如果此 `RuntimeBot` 实例内部的 `Bot` 中已经不存在，将会返回 `BotError::RefExpired` 错误。
    /// 这通常出现在Bot已经关闭，可有个不受Kovi管理的线程仍然拥有此 RuntimeBot。
    pub fn set_deputy_admins(&self, change: SetAdmin) -> Result<(), BotError> {
        let bot = match self.bot.upgrade() {
            Some(b) => b,
            None => return Err(BotError::RefExpired),
        };

        let mut bot = bot.write().unwrap();
        match change {
            SetAdmin::Add(id) => bot.information.deputy_admins.push(id),
            SetAdmin::Adds(ids) => bot.information.deputy_admins.extend(ids),
            SetAdmin::Remove(id) => bot.information.deputy_admins.retain(|&x| x != id),
            SetAdmin::Removes(ids) => bot.information.deputy_admins.retain(|&x| !ids.contains(&x)),
            SetAdmin::Changes(ids) => bot.information.deputy_admins = ids,
        }

        Ok(())
    }

    /// 获取Bot的主管理员
    ///
    /// # Error
    ///
    /// 如果此 `RuntimeBot` 实例内部的 `Bot` 中已经不存在，将会返回 `BotError::RefExpired` 错误。
    /// 这通常出现在Bot已经关闭，可有个不受Kovi管理的线程仍然拥有此 RuntimeBot。
    pub fn get_main_admin(&self) -> Result<i64, BotError> {
        let bot = match self.bot.upgrade() {
            Some(b) => b,
            None => return Err(BotError::RefExpired),
        };

        let id = bot.read().unwrap().information.main_admin;
        Ok(id)
    }

    /// 获取Bot的副管理员
    ///
    /// # Error
    ///
    /// 如果此 `RuntimeBot` 实例内部的 `Bot` 中已经不存在，将会返回 `BotError::RefExpired` 错误。
    /// 这通常出现在Bot已经关闭，可有个不受Kovi管理的线程仍然拥有此 RuntimeBot。
    pub fn get_deputy_admins(&self) -> Result<Vec<i64>, BotError> {
        let bot = match self.bot.upgrade() {
            Some(b) => b,
            None => return Err(BotError::RefExpired),
        };

        let ids = bot.read().unwrap().information.deputy_admins.clone();
        Ok(ids)
    }

    /// 获取Bot的所有管理员
    ///
    /// # Error
    ///
    /// 如果此 `RuntimeBot` 实例内部的 `Bot` 中已经不存在，将会返回 `BotError::RefExpired` 错误。
    /// 这通常出现在Bot已经关闭，可有个不受Kovi管理的线程仍然拥有此 RuntimeBot。
    pub fn get_all_admin(&self) -> Result<Vec<i64>, BotError> {
        let bot = match self.bot.upgrade() {
            Some(b) => b,
            None => return Err(BotError::RefExpired),
        };

        let mut admins = Vec::with_capacity(1);

        let bot = bot.read().unwrap();

        admins.push(bot.information.main_admin);

        admins.extend(bot.information.deputy_admins.clone());

        Ok(admins)
    }
}

/// 工具
impl RuntimeBot {
    /// 获取插件自己的路径
    pub fn get_data_path(&self) -> PathBuf {
        let mut current_dir = std::env::current_dir().unwrap();

        current_dir.push(format!("data/{}", self.plugin_name));
        current_dir
    }
}

/// 插件控制
impl RuntimeBot {
    /// 获取Bot的插件信息。
    ///
    /// # Error
    ///
    /// 如果此 `RuntimeBot` 实例内部的 `Bot` 中已经不存在，将会返回 `BotError::RefExpired` 错误。
    /// 这通常出现在Bot已经关闭，可有个不受Kovi管理的线程仍然拥有此 RuntimeBot。
    pub fn get_plugin_info(&self) -> Result<Vec<PluginInfo>, BotError> {
        let bot = match self.bot.upgrade() {
            Some(b) => b,
            None => return Err(BotError::RefExpired),
        };

        let bot = bot.read().unwrap();

        let plugins_info: Vec<PluginInfo> = bot
            .plugins
            .iter()
            .map(|(name, plugin)| PluginInfo {
                name: name.clone(),
                version: plugin.version.clone(),
                enabled: *plugin.enabled.borrow(),
                enable_on_startup: plugin.enable_on_startup,
                access_control: plugin.access_control,
                list_mode: plugin.list_mode,
                access_list: plugin.access_list.clone(),
            })
            .collect();

        Ok(plugins_info)
    }

    /// 重载传入的插件
    ///
    /// # error
    ///
    /// 如果寻找不到插件，会返回Err `BotError::PluginNotFound`
    ///
    /// 如果此 `RuntimeBot` 实例内部的 `Bot` 中已经不存在，将会返回Err `BotError::RefExpired` 。
    /// 这通常出现在 `Bot` 已经关闭，可有个不受 Kovi 管理的线程仍然拥有此 `RuntimeBot`。
    pub async fn restart_plugin<T: AsRef<str>>(&self, plugin_name: T) -> Result<(), BotError> {
        if self.is_plugin_enable(&plugin_name)? {
            let join = self.disable_plugin(&plugin_name)?;

            if let Some(join) = join {
                join.await.unwrap()
            }
        }

        self.enable_plugin(plugin_name)
    }

    /// 卸载传入的插件
    ///
    /// # error
    ///
    /// 如果寻找不到插件，会返回Err `BotError::PluginNotFound`
    ///
    /// 如果此 `RuntimeBot` 实例内部的 `Bot` 中已经不存在，将会返回Err `BotError::RefExpired` 。
    /// 这通常出现在 `Bot` 已经关闭，可有个不受 Kovi 管理的线程仍然拥有此 `RuntimeBot`。
    pub fn disable_plugin<T: AsRef<str>>(
        &self,
        plugin_name: T,
    ) -> Result<Option<tokio::task::JoinHandle<()>>, BotError> {
        if !self.is_plugin_enable(&plugin_name)? {
            return Ok(None);
        }

        let bot = match self.bot.upgrade() {
            Some(b) => b,
            None => return Err(BotError::RefExpired),
        };

        Ok(Some(disable_plugin(bot, plugin_name)?))
    }

    /// 启用传入的插件
    ///
    /// # error
    ///
    /// 如果寻找不到插件，会返回Err `BotError::PluginNotFound`
    ///
    /// 如果此 `RuntimeBot` 实例内部的 `Bot` 中已经不存在，将会返回Err `BotError::RefExpired` 。
    /// 这通常出现在 `Bot` 已经关闭，可有个不受 Kovi 管理的线程仍然拥有此 `RuntimeBot`。
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
    /// 如果寻找不到插件，会返回Err `BotError::PluginNotFound`
    ///
    /// 如果此 `RuntimeBot` 实例内部的 `Bot` 中已经不存在，将会返回Err `BotError::RefExpired` 。
    /// 这通常出现在 `Bot` 已经关闭，可有个不受 Kovi 管理的线程仍然拥有此 `RuntimeBot`。
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
) -> Result<tokio::task::JoinHandle<()>, BotError> {
    let join;
    {
        let mut bot = bot.write().unwrap();

        let plugin_name = plugin_name.as_ref();

        let bot_plugin = match bot.plugins.get_mut(plugin_name) {
            Some(v) => v,
            None => return Err(BotError::PluginNotFound(plugin_name.to_string())),
        };
        join = bot_plugin.shutdown();
    }

    Ok(join)
}

fn enable_plugin<T: AsRef<str>>(
    bot: Arc<RwLock<Bot>>,
    plugin_name: T,
    api_tx: mpsc::Sender<ApiAndOneshot>,
) -> Result<(), BotError> {
    let bot_read = bot.read().unwrap();
    let plugin_name = plugin_name.as_ref();

    let (host, port) = {
        (
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

    let plugin_builder =
        PluginBuilder::new(plugin_name.to_string(), bot.clone(), host, port, api_tx);

    tokio::spawn(async move { Bot::run_plugin_main(&plugin_, plugin_builder) });

    Ok(())
}
