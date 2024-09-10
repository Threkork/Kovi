use dialoguer::theme::ColorfulTheme;
use dialoguer::Input;
use futures_util::lock::Mutex;
use futures_util::{SinkExt, StreamExt};
use handler::InternalEvent;
use log::{debug, error, warn};
use plugin_builder::{ListenFn, PluginBuilder};
use reqwest::header::HeaderValue;
use runtimebot::onebot_api::ApiReturn;
use runtimebot::ApiMpsc;
use serde::{Deserialize, Serialize};
use serde_json::{self, json, Value};
use std::collections::HashMap;
use std::env;
use std::future::Future;
use std::net::Ipv4Addr;
use std::pin::Pin;
use std::sync::mpsc::{self, Sender};
use std::{fs, net::IpAddr, process::exit, sync::Arc};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;

mod run;

#[cfg(feature = "logger")]
use crate::logger::set_logger;

mod handler;
pub mod message;
pub mod plugin_builder;
pub mod runtimebot;

/// 将配置文件写入磁盘
fn config_file_write_and_return() -> Result<ConfigJson, std::io::Error> {
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
        "plugins": [],
        "server": {
            "host": host,
            "port": port,
            "access_token": access_token
        },
        "debug": false
    });

    let config: ConfigJson = serde_json::from_value(config).unwrap();

    let file = fs::File::create("kovi.conf.json")?;
    serde_json::to_writer_pretty(file, &config)?;
    Ok(config)
}

#[derive(Deserialize, Serialize)]
struct ConfigJson {
    main_admin: i64,
    admins: Vec<i64>,
    plugins: Vec<String>,
    server: Server,
    debug: bool,
}

type AsyncFn =
    dyn Fn(PluginBuilder) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync + 'static;

/// bot结构体
#[derive(Clone)]
pub struct Bot {
    information: BotInformation,
    main: Vec<BotMain>,
    plugins: HashMap<String, Vec<ListenFn>>,
    life: BotLife,
}

#[derive(Clone)]
pub enum BotMain {
    BotSyncMain(BotSyncMain),
    BotAsyncMain(BotAsyncMain),
}

#[derive(Clone)]
pub struct BotSyncMain {
    pub name: String,
    pub version: String,
    pub main: Arc<dyn Fn(PluginBuilder) + Send + Sync + 'static>,
}

#[derive(Clone)]
pub struct BotAsyncMain {
    pub name: String,
    pub version: String,
    pub main: Arc<AsyncFn>,
}

/// bot信息结构体
#[derive(Debug, Clone)]
pub struct BotInformation {
    id: i64,
    nickname: String,
    main_admin: i64,
    admin: Vec<i64>,
    server: Server,
}
/// server信息
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Server {
    host: IpAddr,
    port: u16,
    access_token: String,
}

impl Bot {
    /// 构建一个bot实例
    /// # Examples
    /// ```
    /// let bot = Bot::build();
    /// bot.run()
    /// ```
    pub fn build() -> Bot {
        let config_json: ConfigJson = match fs::read_to_string("kovi.conf.json") {
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
        if env::var("RUST_LOG").is_err() {
            if config_json.debug {
                env::set_var("RUST_LOG", "debug");
            } else {
                env::set_var("RUST_LOG", "info");
            }
        }

        #[cfg(feature = "logger")]
        set_logger();


        Bot {
            information: BotInformation {
                id: 0,
                nickname: "".to_string(),
                main_admin: config_json.main_admin,
                admin: config_json.admins,
                server: config_json.server,
            },
            main: Vec::new(),
            plugins: HashMap::new(),
            life: BotLife {
                status: LifeStatus::Initial,
                //现在的时间
            },
        }
    }

    /// 向bot挂载插件，须传入Arc\<Fn\>
    pub fn mount_main<T>(
        &mut self,
        name: T,
        version: T,
        main: Arc<dyn Fn(PluginBuilder) + Send + Sync + 'static>,
    ) where
        String: From<T>,
    {
        let name = String::from(name);
        let version = String::from(version);
        let bot_main = BotSyncMain {
            name,
            version,
            main,
        };
        self.main.push(BotMain::BotSyncMain(bot_main))
    }

    pub fn mount_async_main<T>(&mut self, name: T, version: T, main: Arc<AsyncFn>)
    where
        String: From<T>,
    {
        let name = String::from(name);
        let version = String::from(version);
        let bot_main = BotAsyncMain {
            name,
            version,
            main,
        };
        self.main.push(BotMain::BotAsyncMain(bot_main))
    }
}

#[derive(Debug, Clone)]
struct BotLife {
    status: LifeStatus,
}

#[derive(Debug, Clone)]
enum LifeStatus {
    Initial,
    Running,
}

type ApiTxMap = Arc<Mutex<HashMap<String, Sender<Result<ApiReturn, ApiReturn>>>>>;

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

impl Bot {
    async fn ws_connect(
        host: IpAddr,
        port: u16,
        access_token: String,
        event_tx: mpsc::Sender<InternalEvent>,
    ) {
        //增加Authorization头
        let mut request = format!("ws://{}:{}/event", host, port)
            .into_client_request()
            .unwrap();

        if !access_token.is_empty() {
            request.headers_mut().insert(
                "Authorization",
                HeaderValue::from_str(&format!("Bearer {}", access_token)).unwrap(),
            );
        }

        let (ws_stream, _) = match connect_async(request).await {
            Ok(v) => v,
            Err(e) => {
                exit_and_eprintln(e, event_tx);
                return;
            }
        };

        let (_, read) = ws_stream.split();
        read.for_each(|msg| {
            let event_tx = event_tx.clone();
            async {
                match msg {
                    Ok(msg) => {
                        if !msg.is_text() {
                            return;
                        }

                        let text = msg.to_text().unwrap();
                        if let Err(e) = event_tx.send(InternalEvent::OneBotEvent(text.to_string()))
                        {
                            debug!("通道关闭：{e}")
                        }
                    }
                    Err(e) => exit_and_eprintln(e, event_tx),
                }
            }
        })
        .await;
    }

    async fn ws_send_api(
        host: IpAddr,
        port: u16,
        access_token: String,
        api_rx: mpsc::Receiver<ApiMpsc>,
        event_tx: mpsc::Sender<InternalEvent>,
    ) {
        //增加Authorization头
        let mut request = format!("ws://{}:{}/api", host, port)
            .into_client_request()
            .unwrap();

        if !access_token.is_empty() {
            request.headers_mut().insert(
                "Authorization",
                HeaderValue::from_str(&format!("Bearer {}", access_token)).unwrap(),
            );
        }


        let (ws_stream, _) = match connect_async(request).await {
            Ok(v) => v,
            Err(e) => {
                exit_and_eprintln(e, event_tx);
                return;
            }
        };

        let (write, read) = ws_stream.split();
        let write = Arc::new(Mutex::new(write));

        let api_tx_map: ApiTxMap = Arc::new(Mutex::new(HashMap::new()));

        //读
        tokio::spawn({
            let api_tx_map = Arc::clone(&api_tx_map);
            let event_tx = event_tx.clone();
            async move {
                read.for_each(|msg| {
                    let event_tx = event_tx.clone();
                    async {
                        match msg {
                            Ok(msg) => {
                                if msg.is_close() {
                                    exit_and_eprintln(
                                        format!("{msg}\nBot connection failed"),
                                        event_tx,
                                    );
                                    return;
                                }
                                if !msg.is_text() {
                                    return;
                                }

                                let text = msg.to_text().unwrap();

                                debug!("{}", text);

                                let return_value: ApiReturn = match serde_json::from_str(text) {
                                    Ok(v) => v,
                                    Err(_) => {
                                        warn!("Unknow api return： {text}");
                                        return;
                                    }
                                };

                                if return_value.status != "ok" {
                                    warn!("Api return error: {text}")
                                }


                                if return_value.echo == "None" {
                                    return;
                                }


                                let mut api_tx_map = api_tx_map.lock().await;

                                let api_tx = api_tx_map.remove(&return_value.echo).unwrap();
                                if return_value.status.to_lowercase() == "ok" {
                                    api_tx.send(Ok(return_value)).unwrap();
                                } else {
                                    api_tx.send(Err(return_value)).unwrap();
                                }
                            }
                            Err(e) => exit_and_eprintln(e, event_tx),
                        }
                    }
                })
                .await;
            }
        });


        //写
        tokio::spawn({
            let write = Arc::clone(&write);
            let api_tx_map = Arc::clone(&api_tx_map);
            async move {
                for (api_msg, return_api_tx) in api_rx {
                    let event_tx = event_tx.clone();
                    debug!("{}", api_msg);

                    if api_msg.echo.as_str() != "None" {
                        api_tx_map
                            .lock()
                            .await
                            .insert(api_msg.echo.clone(), return_api_tx.unwrap());
                    }

                    let msg = Message::text(api_msg.to_string());
                    let mut write_lock = write.lock().await;
                    if let Err(e) = write_lock.send(msg).await {
                        exit_and_eprintln(e, event_tx);
                    }
                }
            }
        });
    }
}


fn exit_and_eprintln<E>(e: E, event_tx: Sender<InternalEvent>)
where
    E: std::fmt::Display,
{
    error!("{e}\nBot connection failed, please check the configuration and restart KoviBot");
    if let Err(e) = event_tx.send(InternalEvent::KoviEvent(handler::KoviEvent::Drop)) {
        debug!("通道关闭,{e}")
    };
}

#[macro_export]
macro_rules! build_bot {
    ( $( $sync_plugin:ident ),* $(,)* ) => {
        {
            let mut bot = kovi::bot::Bot::build();

            $(
                let (crate_name, crate_version) = $sync_plugin::__kovi__get_plugin_info();
                println!("Mounting plugin: {}", crate_name);
                bot.mount_main(crate_name, crate_version, std::sync::Arc::new($sync_plugin::main));
            )*

            bot
        }
    };

    ( async ( $( $async_plugin:ident ),* ) $(,)* ) => {
        {
            let mut bot = kovi::bot::Bot::build();

            $(
                let (crate_name, crate_version) = $async_plugin::__kovi__get_plugin_info();
                println!("Mounting async plugin: {}", crate_name);
                bot.mount_async_main(crate_name, crate_version, std::sync::Arc::new($async_plugin::__kovi__run_async_plugin));
            )*

            bot
        }
    };

    ( async ( $( $async_plugin:ident ),* ) & $( $sync_plugin:ident ),* $(,)* ) => {
        {
            let mut bot = kovi::bot::Bot::build();

            $(
                let (crate_name, crate_version) = $async_plugin::__kovi__get_plugin_info();
                println!("Mounting async plugin: {}", crate_name);
                bot.mount_async_main(crate_name, crate_version, std::sync::Arc::new($async_plugin::__kovi__run_async_plugin));
            )*

            $(
                let (crate_name, crate_version) = $sync_plugin::__kovi__get_plugin_info();
                println!("Mounting plugin: {}", crate_name);
                bot.mount_main(crate_name, crate_version, std::sync::Arc::new($sync_plugin::main));
            )*

            bot
        }
    };

    ( $( $sync_plugin:ident ),* & async ( $( $async_plugin:ident ),* ) $(,)* ) => {
        {
            let mut bot = kovi::bot::Bot::build();

            $(
                let (crate_name, crate_version) = $sync_plugin::__kovi__get_plugin_info();
                println!("Mounting plugin: {}", crate_name);
                bot.mount_main(crate_name, crate_version, std::sync::Arc::new($sync_plugin::main));
            )*

            $(
                let (crate_name, crate_version) = $async_plugin::__kovi__get_plugin_info();
                println!("Mounting async plugin: {}", crate_name);
                bot.mount_async_main(crate_name, crate_version, std::sync::Arc::new($async_plugin::__kovi__run_async_plugin));
            )*

            bot
        }
    };
}
