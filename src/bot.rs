use ahash::{HashMapExt as _, RandomState};
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Input, Select};
use plugin_builder::Listen;
use serde::{Deserialize, Serialize};
use serde_json::{self, Value};
use std::collections::HashMap;
use std::env;
use std::fmt::{Debug, Display};
use std::future::Future;
use std::io::Write as _;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::pin::Pin;
use std::{fs, net::IpAddr, sync::Arc};
use tokio::sync::mpsc::{self};
use tokio::sync::{oneshot, watch};
use tokio::task::JoinHandle;

use crate::error::{BotBuildError, BotError};
use crate::task::TASK_MANAGER;

pub use crate::bot::runtimebot::kovi_api::AccessControlMode;

pub(crate) mod connect;
pub(crate) mod handler;
pub(crate) mod run;

pub mod message;
pub mod plugin_builder;
pub mod runtimebot;

tokio::task_local! {
    pub static PLUGIN_BUILDER: crate::PluginBuilder;
}

tokio::task_local! {
    pub static PLUGIN_NAME: Arc<String>;
}

/// kovi的配置
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KoviConf {
    pub config: Config,
    pub server: Server,
}

impl AsRef<KoviConf> for KoviConf {
    fn as_ref(&self) -> &KoviConf {
        self
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub main_admin: i64,
    pub admins: Vec<i64>,
    pub debug: bool,
}

impl KoviConf {
    pub fn new(main_admin: i64, admins: Option<Vec<i64>>, server: Server, debug: bool) -> Self {
        KoviConf {
            config: Config {
                main_admin,
                admins: admins.unwrap_or_default(),
                debug,
            },
            server,
        }
    }
}

type KoviAsyncFn = dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync;

impl Drop for Bot {
    fn drop(&mut self) {
        for i in self.run_abort.iter() {
            i.abort();
        }
    }
}

/// bot结构体
#[derive(Clone)]
pub struct Bot {
    pub information: BotInformation,
    pub(crate) plugins: HashMap<String, BotPlugin, RandomState>,
    pub(crate) run_abort: Vec<tokio::task::AbortHandle>,
}

#[derive(Clone)]
pub(crate) struct BotPlugin {
    pub(crate) enable_on_startup: bool,
    pub(crate) enabled: watch::Sender<bool>,

    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) main: Arc<KoviAsyncFn>,
    pub(crate) listen: Listen,

    #[cfg(feature = "plugin-access-control")]
    pub(crate) access_control: bool,
    #[cfg(feature = "plugin-access-control")]
    pub(crate) list_mode: AccessControlMode,
    #[cfg(feature = "plugin-access-control")]
    pub(crate) access_list: AccessList,
}

#[cfg(feature = "plugin-access-control")]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct AccessList {
    pub(crate) friends: Vec<i64>,
    pub(crate) groups: Vec<i64>,
}

#[derive(Clone)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub enabled: bool,
    pub enable_on_startup: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct PluginStatus {
    enable_on_startup: bool,
    access_control: bool,
    list_mode: AccessControlMode,
    access_list: AccessList,
}

/// bot信息结构体
#[derive(Debug, Clone)]
pub struct BotInformation {
    pub main_admin: i64,
    pub deputy_admins: Vec<i64>,
    pub server: Server,
}
/// server信息
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Server {
    pub host: Host,
    pub port: u16,
    pub access_token: String,
    pub secure: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum Host {
    IpAddr(IpAddr),
    Domain(String),
}

impl Display for Host {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Host::IpAddr(ip) => write!(f, "{}", ip),
            Host::Domain(domain) => write!(f, "{}", domain),
        }
    }
}

impl Server {
    pub fn new(host: Host, port: u16, access_token: String, secure: bool) -> Self {
        Server {
            host,
            port,
            access_token,
            secure,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SendApi {
    pub action: String,
    pub params: Value,
    pub echo: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiReturn {
    pub status: String,
    pub retcode: i32,
    pub data: Value,
    pub echo: String,
}

pub(crate) type ApiAndOneshot = (
    SendApi,
    Option<oneshot::Sender<Result<ApiReturn, ApiReturn>>>,
);

impl std::fmt::Display for ApiReturn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "status: {}, retcode: {}, data: {}, echo: {}",
            self.status, self.retcode, self.data, self.echo
        )
    }
}

impl std::fmt::Display for SendApi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", serde_json::to_string(self).unwrap())
    }
}

impl SendApi {
    pub fn new(action: &str, params: Value, echo: &str) -> Self {
        SendApi {
            action: action.to_string(),
            params,
            echo: echo.to_string(),
        }
    }
}

impl BotPlugin {
    fn shutdown(&mut self) -> JoinHandle<()> {
        log::info!("Plugin '{}' is dropping.", self.name,);

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

        TASK_MANAGER.disable_plugin(&self.name);

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

impl Bot {
    /// 构建一个bot实例
    /// # Examples
    /// ```
    /// let conf = KoviConf::new(
    ///     123456,
    ///     None,
    ///     Server {
    ///         host: "127.0.0.1".parse(),
    ///         port: 8081,
    ///         access_token: "",
    ///     },
    ///     false,
    ///     None,
    /// );
    /// let bot = Bot::build(conf);
    /// bot.run()
    /// ```
    pub fn build<C>(conf: C) -> Bot
    where
        C: AsRef<KoviConf>,
    {
        let conf = conf.as_ref();
        Bot {
            information: BotInformation {
                main_admin: conf.config.main_admin,
                deputy_admins: conf.config.admins.clone(),
                server: conf.server.clone(),
            },
            plugins: HashMap::<_, _, RandomState>::new(),
            run_abort: Vec::new(),
        }
    }

    /// 挂载插件的启动函数。
    pub fn mount_main<T>(&mut self, name: T, version: T, main: Arc<KoviAsyncFn>)
    where
        String: From<T>,
    {
        let name = String::from(name);
        let version = String::from(version);
        let (tx, _rx) = watch::channel(true);
        let bot_plugin = BotPlugin {
            enable_on_startup: true,
            enabled: tx,
            name: name.clone(),
            version,
            main,
            listen: Listen::default(),

            #[cfg(feature = "plugin-access-control")]
            access_control: false,
            #[cfg(feature = "plugin-access-control")]
            list_mode: AccessControlMode::WhiteList,
            #[cfg(feature = "plugin-access-control")]
            access_list: AccessList::default(),
        };
        self.plugins.insert(name, bot_plugin);
    }

    /// 读取本地Kovi.conf.toml文件
    pub fn load_local_conf() -> Result<KoviConf, BotBuildError> {
        //检测文件是kovi.conf.json还是kovi.conf.toml
        let kovi_conf_file_exist = fs::metadata("kovi.conf.toml").is_ok();

        let conf_json: KoviConf = if kovi_conf_file_exist {
            match fs::read_to_string("kovi.conf.toml") {
                Ok(v) => match toml::from_str(&v) {
                    Ok(conf) => conf,
                    Err(err) => {
                        eprintln!("Configuration file parsing error: {}", err);
                        config_file_write_and_return()
                            .map_err(|e| BotBuildError::FileCreateError(e.to_string()))?
                    }
                },
                Err(err) => {
                    return Err(BotBuildError::FileReadError(err.to_string()));
                }
            }
        } else {
            config_file_write_and_return()
                .map_err(|e| BotBuildError::FileCreateError(e.to_string()))?
        };

        unsafe {
            if env::var("RUST_LOG").is_err() {
                if conf_json.config.debug {
                    env::set_var("RUST_LOG", "debug");
                } else {
                    env::set_var("RUST_LOG", "info");
                }
            }
        }

        Ok(conf_json)
    }
}

impl Bot {
    /// 使用KoviConf设置插件在Bot启动时的状态
    ///
    /// 如果配置文件中没有对应的插件，将会被忽略，保留插件默认状态
    ///
    /// 如果配置文件读取失败或者解析toml失败，将会保留插件默认状态
    pub fn set_plugin_startup_use_file(mut self) -> Self {
        let file_path = "kovi.plugin.toml";
        let content = match fs::read_to_string(file_path) {
            Ok(v) => {
                log::debug!("Set plugin startup use file successfully");
                v
            }
            Err(e) => {
                log::debug!("Failed to read file: {}", e);
                return self;
            }
        };
        let plugin_status_map: HashMap<String, PluginStatus> = match toml::from_str(&content) {
            Ok(v) => v,
            Err(e) => {
                log::debug!("Failed to parse toml: {}", e);
                return self;
            }
        };

        for (name, plugin) in self.plugins.iter_mut() {
            if let Some(plugin_status) = plugin_status_map.get(name) {
                plugin.enable_on_startup = plugin_status.enable_on_startup;
                plugin.access_control = plugin_status.access_control;
                plugin.list_mode = plugin_status.list_mode;
                plugin.access_list = plugin_status.access_list.clone();
            }
        }

        self
    }

    /// 使用KoviConf设置插件在Bot启动时的状态
    ///
    /// 如果配置文件中没有对应的插件，将会被忽略，保留插件默认状态
    ///
    /// 如果配置文件读取失败或者解析toml失败，将会保留插件默认状态
    pub fn set_plugin_startup_use_file_ref(&mut self) {
        let file_path = "kovi.plugin.toml";
        let content = match fs::read_to_string(file_path) {
            Ok(v) => {
                log::debug!("Set plugin startup use file successfully");
                v
            }
            Err(e) => {
                log::debug!("Failed to read file: {}", e);
                return;
            }
        };
        let plugin_status_map: HashMap<String, PluginStatus> = match toml::from_str(&content) {
            Ok(v) => v,
            Err(e) => {
                log::debug!("Failed to parse toml: {}", e);
                return;
            }
        };

        for (name, plugin) in self.plugins.iter_mut() {
            if let Some(plugin_status) = plugin_status_map.get(name) {
                plugin.enable_on_startup = plugin_status.enable_on_startup;
                plugin.access_control = plugin_status.access_control;
                plugin.list_mode = plugin_status.list_mode;
                plugin.access_list = plugin_status.access_list.clone();
            }
        }
    }

    /// 设置全部插件在Bot启动时的状态
    pub fn set_all_plugin_startup(mut self, enabled: bool) -> Self {
        for plugin in self.plugins.values_mut() {
            plugin.enable_on_startup = enabled
        }
        self
    }

    /// 设置全部插件在Bot启动时的状态
    pub fn set_all_plugin_startup_ref(&mut self, enabled: bool) {
        for plugin in self.plugins.values_mut() {
            plugin.enable_on_startup = enabled
        }
    }

    /// 设置单个插件在Bot启动时的状态
    pub fn set_plugin_startup<T: AsRef<str>>(
        mut self,
        name: T,
        enabled: bool,
    ) -> Result<Self, BotError> {
        let name = name.as_ref();
        if let Some(n) = self.plugins.get_mut(name) {
            n.enable_on_startup = enabled;
            Ok(self)
        } else {
            Err(BotError::PluginNotFound(format!(
                "Plugin {} not found",
                name
            )))
        }
    }

    /// 设置单个插件在Bot启动时的状态
    pub fn set_plugin_startup_ref<T: AsRef<str>>(
        &mut self,
        name: T,
        enabled: bool,
    ) -> Result<(), BotError> {
        let name = name.as_ref();
        if let Some(n) = self.plugins.get_mut(name) {
            n.enable_on_startup = enabled;
            Ok(())
        } else {
            Err(BotError::PluginNotFound(format!(
                "Plugin {} not found",
                name
            )))
        }
    }

    #[cfg(any(feature = "save_plugin_status", feature = "save_bot_admin"))]
    pub(crate) fn save_bot_status(&self) {
        #[cfg(feature = "save_plugin_status")]
        {
            let _file_path = "kovi.plugin.toml";

            let mut plugin_status = HashMap::new();
            for (name, plugin) in self.plugins.iter() {
                plugin_status.insert(name.clone(), PluginStatus {
                    enable_on_startup: plugin.enable_on_startup,
                    access_control: plugin.access_control,
                    list_mode: plugin.list_mode,
                    access_list: plugin.access_list.clone(),
                });
            }

            let serialized =
                toml::to_string(&plugin_status).expect("Failed to serialize plugin status");
            fs::write(_file_path, serialized).expect("Failed to write plugin status to file");
        }

        #[cfg(feature = "save_bot_admin")]
        {
            let file_path = "kovi.conf.toml";
            let existing_content = fs::read_to_string(file_path).unwrap_or_default();

            let mut doc = existing_content
                .parse::<toml_edit::DocumentMut>()
                .unwrap_or_else(|_| toml_edit::DocumentMut::new());

            // 确保 "config" 存在
            if !doc.contains_key("config") {
                doc["config"] = toml_edit::table();
            }

            // 更新 "config" 中的 admin 信息
            doc["config"]["main_admin"] = toml_edit::value(self.information.main_admin);
            doc["config"]["admins"] = toml_edit::Item::Value(toml_edit::Value::Array(
                self.information
                    .deputy_admins
                    .iter()
                    .map(|&x| toml_edit::Value::from(x))
                    .collect(),
            ));

            let file = fs::File::create(file_path).unwrap();
            let mut writer = std::io::BufWriter::new(file);
            writer.write_all(doc.to_string().as_bytes()).unwrap();
        }
    }
}

/// 将配置文件写入磁盘
fn config_file_write_and_return() -> Result<KoviConf, std::io::Error> {
    enum HostType {
        IPv4,
        IPv6,
        Domain,
    }

    let host_type: HostType = {
        let items = ["IPv4", "IPv6", "Domain"];
        let select = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("What is the type of the host of the OneBot server?")
            .items(&items)
            .default(0)
            .interact()
            .unwrap();

        match select {
            0 => HostType::IPv4,
            1 => HostType::IPv6,
            2 => HostType::Domain,
            _ => panic!(), //不可能的事情
        }
    };

    let host = match host_type {
        HostType::IPv4 => {
            let ip = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("What is the IP of the OneBot server?")
                .default(Ipv4Addr::new(127, 0, 0, 1))
                .interact_text()
                .unwrap();
            Host::IpAddr(IpAddr::V4(ip))
        }
        HostType::IPv6 => {
            let ip = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("What is the IP of the OneBot server?")
                .default(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1))
                .interact_text()
                .unwrap();
            Host::IpAddr(IpAddr::V6(ip))
        }
        HostType::Domain => {
            let domain = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("What is the domain of the OneBot server?")
                .default("localhost".to_string())
                .interact_text()
                .unwrap();
            Host::Domain(domain)
        }
    };

    let port: u16 = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("What is the port of the OneBot server?")
        .default(8081)
        .interact_text()
        .unwrap();

    let access_token: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("What is the access_token of the OneBot server? (Optional)")
        .default("".to_string())
        .show_default(false)
        .interact_text()
        .unwrap();

    let main_admin: i64 = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("What is the ID of the main administrator? (Not used yet)")
        .allow_empty(true)
        .interact_text()
        .unwrap();

    // 是否查看更多可选选项
    let more: bool = {
        let items = ["No", "Yes"];
        let select = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Do you want to view more optional options?")
            .items(&items)
            .default(0)
            .interact()
            .unwrap();

        match select {
            0 => false,
            1 => true,
            _ => panic!(), //不可能的事情
        }
    };

    let mut secure = false;
    if more {
        // wss https? tls?
        secure = {
            let items = vec!["No", "Yes"];
            let select = Select::with_theme(&ColorfulTheme::default())
                // .with_prompt("Enable secure connection? (HTTPS/WSS)")
                .with_prompt("Enable secure connection? (WSS)")
                .items(&items)
                .default(0)
                .interact()
                .unwrap();

            match select {
                0 => false,
                1 => true,
                _ => panic!(), //不可能的事情
            }
        };
    }

    let config = KoviConf::new(
        main_admin,
        None,
        Server::new(host, port, access_token, secure),
        false,
    );

    let mut doc = toml_edit::DocumentMut::new();
    doc["config"] = toml_edit::table();
    doc["config"]["main_admin"] = toml_edit::value(config.config.main_admin);
    doc["config"]["admins"] = toml_edit::Item::Value(toml_edit::Value::Array(
        config
            .config
            .admins
            .iter()
            .map(|&x| toml_edit::Value::from(x))
            .collect(),
    ));
    doc["config"]["debug"] = toml_edit::value(config.config.debug);

    doc["server"] = toml_edit::table();
    doc["server"]["host"] = match &config.server.host {
        Host::IpAddr(ip) => toml_edit::value(ip.to_string()),
        Host::Domain(domain) => toml_edit::value(domain.clone()),
    };
    doc["server"]["port"] = toml_edit::value(config.server.port as i64);
    doc["server"]["access_token"] = toml_edit::value(config.server.access_token.clone());
    doc["server"]["secure"] = toml_edit::value(config.server.secure);

    let file = fs::File::create("kovi.conf.toml")?;
    let mut writer = std::io::BufWriter::new(file);
    writer.write_all(doc.to_string().as_bytes())?;

    Ok(config)
}

#[macro_export]
macro_rules! build_bot {
    ($( $plugin:ident ),* $(,)* ) => {
        {
            let conf = match kovi::bot::Bot::load_local_conf() {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error loading config: {}", e);
                    panic!("Failed to load config");
                }
            };
            kovi::logger::try_set_logger();
            let mut bot = kovi::bot::Bot::build(&conf);

            $(
                let (crate_name, crate_version) = $plugin::__kovi_get_plugin_info();
                kovi::log::info!("Mounting plugin: {}", crate_name);
                bot.mount_main(crate_name, crate_version, std::sync::Arc::new($plugin::__kovi_run_async_plugin));
            )*

            bot.set_plugin_startup_use_file_ref();
            bot
        }
    };
}

#[test]
fn build_bot() {
    let conf = KoviConf::new(
        123456,
        None,
        Server {
            host: Host::IpAddr("127.0.0.1".parse().unwrap()),
            port: 8081,
            access_token: "".to_string(),
            secure: false,
        },
        false,
    );
    let _ = Bot::build(conf);
}
