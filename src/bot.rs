use ahash::{HashMapExt as _, RandomState};
use dialoguer::theme::ColorfulTheme;
use dialoguer::Input;
use handler::InternalEvent;
use log::{debug, error};
use plugin_builder::Listen;
use serde::{Deserialize, Serialize};
use serde_json::{self, Value};
use std::collections::HashMap;
use std::env;
use std::future::Future;
use std::io::Write as _;
use std::net::Ipv4Addr;
use std::pin::Pin;
use std::{fs, net::IpAddr, process::exit, sync::Arc};
use tokio::sync::mpsc::{self, Sender};
use tokio::sync::{oneshot, watch};

use crate::PluginBuilder;

mod connect;
mod handler;
mod run;

pub mod controller;
pub mod message;
pub mod plugin_builder;
pub mod runtimebot;

/// kovi的配置
#[derive(Deserialize, Serialize)]
pub struct KoviConf {
    pub config: Config,
    pub server: Server,
}

#[derive(Deserialize, Serialize)]
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

type KoviAsyncFn = dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync + 'static;

/// bot结构体
#[derive(Clone)]
pub struct Bot {
    pub information: BotInformation,
    pub plugins: HashMap<String, BotPlugin, RandomState>,
}

#[derive(Clone)]
pub struct BotPlugin {
    pub(crate) enabled: watch::Sender<bool>,
    pub version: String,
    pub(crate) main: Arc<KoviAsyncFn>,
    pub(crate) listen: Listen,
    pub(crate) plugin_builder: Option<PluginBuilder>,
}

/// bot信息结构体
#[derive(Debug, Clone)]
pub struct BotInformation {
    pub main_admin: i64,
    pub admin: Vec<i64>,
    pub server: Server,
}
/// server信息
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Server {
    pub host: IpAddr,
    pub port: u16,
    pub access_token: String,
}

impl Server {
    pub fn new(host: IpAddr, port: u16, access_token: String) -> Self {
        Server {
            host,
            port,
            access_token,
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

#[derive(Deserialize)]
struct OldConf {
    main_admin: i64,
    admin: Vec<i64>,
    server: Server,
    debug: bool,
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
    /// );
    /// let bot = Bot::build(conf);
    /// bot.run()
    /// ```
    pub fn build(conf: KoviConf) -> Bot {
        Bot {
            information: BotInformation {
                main_admin: conf.config.main_admin,
                admin: conf.config.admins,
                server: conf.server,
            },
            plugins: HashMap::<_, _, RandomState>::new(),
        }
    }

    pub fn mount_main<T>(&mut self, name: T, version: T, main: Arc<KoviAsyncFn>)
    where
        String: From<T>,
    {
        let name = String::from(name);
        let version = String::from(version);
        let (tx, _rx) = watch::channel(true);
        let bot_plugin = BotPlugin {
            enabled: tx,
            version,
            main,
            listen: Listen::default(),
            plugin_builder: None,
        };
        self.plugins.insert(name, bot_plugin);
    }

    pub fn load_local_conf() -> KoviConf {
        enum KoviConfFile {
            Json,
            Toml,
            None,
        }

        //检测文件是kovi.conf.json还是kovi.conf.toml
        let kovi_conf_file = if fs::metadata("kovi.conf.toml").is_ok() {
            KoviConfFile::Toml
        } else if fs::metadata("kovi.conf.json").is_ok() {
            KoviConfFile::Json
        } else {
            KoviConfFile::None
        };

        let conf_json: KoviConf = match kovi_conf_file {
            KoviConfFile::Toml => match fs::read_to_string("kovi.conf.toml") {
                Ok(v) => match toml::from_str(&v) {
                    Ok(conf) => conf,
                    Err(err) => {
                        error!("Failed to parse TOML: {}", err);
                        exit(1);
                    }
                },
                Err(err) => {
                    error!("Failed to read TOML file: {}", err);
                    exit(1);
                }
            },
            KoviConfFile::Json => old_json_conf_to_toml_conf(),
            KoviConfFile::None => match config_file_write_and_return() {
                Ok(conf) => conf,
                Err(err) => {
                    error!("Failed to create config file: {}", err);
                    exit(1);
                }
            },
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

        conf_json
    }
}

/// 将配置文件写入磁盘
fn config_file_write_and_return() -> Result<KoviConf, std::io::Error> {
    let host = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("What is the IP of the OneBot server?")
        .default(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)))
        .interact_text()
        .unwrap();

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

    let config = KoviConf::new(
        main_admin,
        None,
        Server::new(host, port, access_token),
        false,
    );

    let file = fs::File::create("kovi.conf.toml")?;

    //写入文件
    let mut writer = std::io::BufWriter::new(file);
    let config_str = toml::to_string_pretty(&config).unwrap();
    writer.write_all(config_str.as_bytes())?;

    Ok(config)
}

fn old_json_conf_to_toml_conf() -> KoviConf {
    let old_conf: OldConf = match fs::read_to_string("kovi.conf.json") {
        Ok(v) => match serde_json::from_str(&v) {
            Ok(conf) => conf,
            Err(_) => {
                error!("Load config error, please try deleting kovi.conf.json and reconfigure.\n 加载出错，请尝试删掉kovi.conf.json，重新配置。");
                exit(1);
            }
        },
        Err(err) => {
            error!("Failed to read JSON file: {}", err);
            error!("Load config error, please try deleting kovi.conf.json and reconfigure.\n 加载出错，请尝试删掉kovi.conf.json，重新配置。");
            exit(1);
        }
    };

    let new_conf = KoviConf::new(
        old_conf.main_admin,
        Some(old_conf.admin),
        old_conf.server,
        old_conf.debug,
    );

    let toml_str = toml::to_string_pretty(&new_conf).unwrap();
    fs::write("kovi.conf.toml", toml_str).unwrap();

    if let Err(e) = fs::remove_file("kovi.conf.json") {
        error!("Failed to remove old JSON config file: {}", e);
    }

    new_conf
}


pub(crate) async fn exit_and_eprintln<E>(e: E, event_tx: Sender<InternalEvent>)
where
    E: std::fmt::Display,
{
    error!("{e}\nBot connection failed, please check the configuration and restart KoviBot");
    if let Err(e) = event_tx
        .send(InternalEvent::KoviEvent(handler::KoviEvent::Drop))
        .await
    {
        debug!("通道关闭,{e}")
    };
}

#[macro_export]
macro_rules! build_bot {
    ($( $plugin:ident ),* $(,)* ) => {
        {
            let conf = kovi::bot::Bot::load_local_conf();
            kovi::logger::try_set_logger();
            let mut bot = kovi::bot::Bot::build(conf);

            $(
                let (crate_name, crate_version) = $plugin::__kovi_get_plugin_info();
                kovi::log::info!("Mounting plugin: {}", crate_name);
                bot.mount_main(crate_name, crate_version, std::sync::Arc::new($plugin::__kovi_run_async_plugin));
            )*

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
            host: "127.0.0.1".parse().unwrap(),
            port: 8081,
            access_token: "".to_string(),
        },
        false,
    );
    let _ = Bot::build(conf);
}
