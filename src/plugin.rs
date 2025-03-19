use crate::Bot;
use crate::runtime::RT;
use crate::task::TASK_MANAGER;
use crate::types::KoviAsyncFn;
use plugin_builder::listen::Listen;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, OnceLock};
use tokio::sync::watch;
use tokio::task::JoinHandle;

#[cfg(not(feature = "dylib-plugin"))]
use crate::PluginBuilder;

#[cfg(feature = "plugin-access-control")]
pub use crate::bot::runtimebot::kovi_api::AccessControlMode;

#[cfg(feature = "plugin-access-control")]
use crate::bot::runtimebot::kovi_api::AccessList;

pub(crate) mod dylib_plugin;
pub mod plugin_builder;

#[cfg(not(feature = "dylib-plugin"))]
tokio::task_local! {
    pub static PLUGIN_BUILDER: crate::PluginBuilder;
}

#[cfg(not(feature = "dylib-plugin"))]
tokio::task_local! {
    pub static PLUGIN_NAME: Arc<String>;
}

#[cfg(feature = "dylib-plugin")]
pub static PLUGIN_BUILDER: OnceLock<crate::PluginBuilder> = OnceLock::new();

#[cfg(feature = "dylib-plugin")]
pub static PLUGIN_NAME: OnceLock<String> = OnceLock::new();

#[derive(Clone)]
pub struct Plugin {
    pub(crate) enable_on_startup: bool,
    pub(crate) enabled: watch::Sender<bool>,

    pub name: String,
    pub version: String,
    pub(crate) main: Arc<KoviAsyncFn>,
    pub(crate) listen: Listen,

    #[cfg(feature = "plugin-access-control")]
    pub(crate) access_control: bool,
    #[cfg(feature = "plugin-access-control")]
    pub(crate) list_mode: AccessControlMode,
    #[cfg(feature = "plugin-access-control")]
    pub(crate) access_list: AccessList,
}
impl std::fmt::Debug for Plugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Plugin")
            .field("name", &self.name)
            .field("version", &self.version)
            .field("enabled", &self.enabled)
            .field("enable_on_startup", &self.enable_on_startup)
            .finish()
    }
}

impl Plugin {
    pub fn new<S>(name: S, version: S, main: Arc<KoviAsyncFn>) -> Self
    where
        S: Into<String>,
    {
        Self {
            enable_on_startup: true,
            enabled: watch::channel(true).0,
            name: name.into(),
            version: version.into(),
            main,
            listen: Listen::default(),
            #[cfg(feature = "plugin-access-control")]
            access_control: false,
            #[cfg(feature = "plugin-access-control")]
            list_mode: AccessControlMode::WhiteList,
            #[cfg(feature = "plugin-access-control")]
            access_list: AccessList::default(),
        }
    }

    // 运行插件的main()
    #[cfg(not(feature = "dylib-plugin"))]
    pub(crate) fn run(&self, plugin_builder: PluginBuilder) {
        let plugin_name = plugin_builder.runtime_bot.plugin_name.clone();

        let mut enabled = self.enabled.subscribe();
        let main = self.main.clone();

        RT.get().unwrap().spawn(async move {
            tokio::select! {
                _ = PLUGIN_NAME.scope(
                        Arc::new(plugin_name),
                        PLUGIN_BUILDER.scope(plugin_builder, main()),
                ) =>{}
                _ = async {
                        loop {
                            enabled.changed().await.unwrap();
                            if !*enabled.borrow_and_update() {
                                break;
                            }
                        }
                } => {}
            }
        });
    }

    // 运行插件的main()
    #[cfg(feature = "dylib-plugin")]
    pub(crate) fn run(&self) {
        let mut enabled = self.enabled.subscribe();
        let main = self.main.clone();

        RT.get().unwrap().spawn(async move {
            tokio::select! {
                _ = main() => {}
                _ = async {
                        loop {
                            enabled.changed().await.unwrap();
                            if !*enabled.borrow_and_update() {
                                break;
                            }
                        }
                } => {}
            }
        });
    }

    #[cfg(not(feature = "dylib-plugin"))]
    pub(crate) fn shutdown(&mut self) -> JoinHandle<()> {
        log::debug!("Plugin '{}' is dropping.", self.name,);

        let plugin_name_ = Arc::new(self.name.clone());

        let mut task_vec = Vec::new();

        for listen in &self.listen.drop {
            let listen_clone = listen.clone();

            let plugin_name_ = plugin_name_.clone();

            let task = tokio::spawn(async move {
                PLUGIN_NAME
                    .scope(plugin_name_, Bot::handler_drop(listen_clone))
                    .await;
            });

            task_vec.push(task);
        }

        TASK_MANAGER.get().unwrap().disable_plugin(&self.name);

        self.enabled.send_modify(|v| {
            *v = false;
        });
        self.listen.clear();
        tokio::spawn(async move {
            for task in task_vec {
                let _ = task.await;
            }
        })
    }

    #[cfg(feature = "dylib-plugin")]
    pub(crate) fn shutdown(&mut self) -> JoinHandle<()> {
        log::debug!("Plugin '{}' is dropping.", self.name,);

        let mut task_vec = Vec::new();

        for listen in &self.listen.drop {
            let listen_clone = listen.clone();

            let task = tokio::spawn(async move {
                Bot::handler_drop(listen_clone).await;
            });

            task_vec.push(task);
        }

        TASK_MANAGER.get().unwrap().disable_plugin(&self.name);

        self.enabled.send_modify(|v| {
            *v = false;
        });
        self.listen.clear();
        tokio::spawn(async move {
            for task in task_vec {
                let _ = task.await;
            }
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct PluginStatus {
    pub(crate) enable_on_startup: bool,
    #[cfg(feature = "plugin-access-control")]
    pub(crate) access_control: bool,
    #[cfg(feature = "plugin-access-control")]
    pub(crate) list_mode: AccessControlMode,
    #[cfg(feature = "plugin-access-control")]
    pub(crate) access_list: AccessList,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    /// 插件是否启用
    pub enabled: bool,
    /// 插件是否在Bot启动时启用
    pub enable_on_startup: bool,
    /// 插件是否启用框架级访问控制
    #[cfg(feature = "plugin-access-control")]
    pub access_control: bool,
    /// 插件的访问控制模式
    #[cfg(feature = "plugin-access-control")]
    pub list_mode: AccessControlMode,
    /// 插件的访问控制列表
    #[cfg(feature = "plugin-access-control")]
    pub access_list: AccessList,
}
