use dialoguer::theme::ColorfulTheme;
use dialoguer::Input;
use handler::InternalEvent;
use log::{debug, error};
use plugin_builder::{ListenFn, PluginBuilder};
use runtimebot::onebot_api::ApiReturn;
use runtimebot::ApiOneshot;
use serde::{Deserialize, Serialize};
use serde_json::{self, json, Value};
use std::collections::HashMap;
use std::env;
use std::future::Future;
use std::net::Ipv4Addr;
use std::pin::Pin;
use std::{fs, net::IpAddr, process::exit, sync::Arc};
use tokio::sync::mpsc::{self, Sender};

mod run;

mod connect;
mod handler;
pub mod message;
pub mod plugin_builder;
pub mod runtimebot;

impl Bot {
    /// 构建一个bot实例
    /// # Examples
    /// ```
    /// let bot = Bot::build();
    /// bot.run()
    /// ```
    pub fn build(conf: KoviConf) -> Bot {
        Bot {
            information: BotInformation {
                id: 0,
                nickname: "".to_string(),
                main_admin: conf.main_admin,
                admin: conf.admins,
                server: conf.server,
            },
            main: Vec::new(),
            plugins: HashMap::new(),
        }
    }

    pub fn mount_main<T>(&mut self, name: T, version: T, main: Arc<AsyncFn>)
    where
        String: From<T>,
    {
        let name = String::from(name);
        let version = String::from(version);
        let bot_main = BotMain {
            name,
            version,
            main,
        };
        self.main.push(bot_main)
    }

    pub fn load_local_conf() -> KoviConf {
        let conf_json: KoviConf = match fs::read_to_string("kovi.conf.json") {
            Ok(v) => match serde_json::from_str(&v) {
                Ok(v) => v,
                Err(err) => {
                    error!("{err}");
                    exit(1)
                }
            },
            Err(_) => match config_file_write_and_return() {
                Ok(v) => v,
                Err(err) => {
                    error!("{err}");
                    exit(1)
                }
            },
        };

        unsafe {
            if env::var("RUST_LOG").is_err() {
                if conf_json.debug {
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

    let config = json!({
        "main_admin": main_admin,
        "admins": [],
        "server": {
            "host": host,
            "port": port,
            "access_token": access_token
        },
        "debug": false
    });

    let config: KoviConf = serde_json::from_value(config).unwrap();

    let file = fs::File::create("kovi.conf.json")?;
    serde_json::to_writer_pretty(file, &config)?;
    Ok(config)
}

/// kovi的配置
#[derive(Deserialize, Serialize)]
pub struct KoviConf {
    pub main_admin: i64,
    pub admins: Vec<i64>,
    pub server: Server,
    pub debug: bool,
}
impl KoviConf {
    pub fn new(main_admin: i64, admins: Option<Vec<i64>>, server: Server, debug: bool) -> Self {
        KoviConf {
            main_admin,
            admins: if let Some(v) = admins { v } else { Vec::new() },
            server,
            debug,
        }
    }
}

type AsyncFn =
    dyn Fn(PluginBuilder) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync + 'static;

/// bot结构体
#[derive(Clone)]
pub struct Bot {
    pub information: BotInformation,
    pub main: Vec<BotMain>,
    pub plugins: HashMap<String, Vec<ListenFn>>,
}


#[derive(Clone)]
pub struct BotSyncMain {
    pub name: String,
    pub version: String,
    pub main: Arc<dyn Fn(PluginBuilder) + Send + Sync + 'static>,
}

#[derive(Clone)]
pub struct BotMain {
    pub name: String,
    pub version: String,
    pub main: Arc<AsyncFn>,
}

/// bot信息结构体
#[derive(Debug, Clone)]
pub struct BotInformation {
    pub id: i64,
    pub nickname: String,
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

#[derive(Debug, Serialize, Clone)]
pub struct SendApi {
    pub action: String,
    pub params: Value,
    pub echo: String,
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


async fn exit_and_eprintln<E>(e: E, event_tx: Sender<InternalEvent>)
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
            kovi::logger::set_logger();
            let conf = kovi::bot::Bot::load_local_conf();
            let mut bot = kovi::bot::Bot::build(conf);

            $(
                let (crate_name, crate_version) = $plugin::__kovi__get_plugin_info();
                kovi::log::info!("Mounting plugin: {}", crate_name);
                bot.mount_main(crate_name, crate_version, std::sync::Arc::new($plugin::__kovi__run_async_plugin));
            )*

            bot
        }
    };
}
