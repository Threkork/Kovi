use ahash::{HashMapExt as _, RandomState};
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Input, Select};
use handler::InternalEvent;
use log::{debug, error};
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
use std::{fs, net::IpAddr, process::exit, sync::Arc};
use tokio::sync::mpsc::{self, Sender};
use tokio::sync::{oneshot, watch};
mod connect;
mod handler;
mod run;

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

type KoviAsyncFn = dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync;

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
        };
        self.plugins.insert(name, bot_plugin);
    }


    pub fn load_local_conf() -> KoviConf {
        //检测文件是kovi.conf.json还是kovi.conf.toml
        let kovi_conf_file_exist = fs::metadata("kovi.conf.toml").is_ok();

        let conf_json: KoviConf = if kovi_conf_file_exist {
            match fs::read_to_string("kovi.conf.toml") {
                Ok(v) => match toml::from_str(&v) {
                    Ok(conf) => conf,
                    Err(err) => {
                        eprintln!(
                            "Failed to parse TOML:\n{}\nPlease reload the config file",
                            err
                        );
                        match config_file_write_and_return() {
                            Ok(conf) => conf,
                            Err(err) => {
                                eprintln!("Failed to create config file: {}", err);
                                exit(1);
                            }
                        }
                    }
                },
                Err(err) => {
                    eprintln!("Failed to read TOML file: {}", err);
                    exit(1);
                }
            }
        } else {
            match config_file_write_and_return() {
                Ok(conf) => conf,
                Err(err) => {
                    eprintln!("Failed to create config file: {}", err);
                    exit(1);
                }
            }
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
    enum HostType {
        IPv4,
        IPv6,
        Domain,
    }

    let host_type: HostType = {
        let items = vec!["IPv4", "IPv6", "Domain"];
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
        let items = vec!["No", "Yes"];
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
            host: Host::IpAddr("127.0.0.1".parse().unwrap()),
            port: 8081,
            access_token: "".to_string(),
            secure: false,
        },
        false,
    );
    let _ = Bot::build(conf);
}
