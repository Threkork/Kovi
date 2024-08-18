use crate::error::Error;
use crate::log::set_log;
use dialoguer::theme::ColorfulTheme;
use dialoguer::Input;
use log::{debug, error};
use plugin_builder::{Plugin, PluginBuilder};
use runtimebot::ApiMpsc;
use serde::{Deserialize, Serialize};
use serde_json::{self, json, Value};
use std::cell::RefCell;
use std::env;
use std::net::Ipv4Addr;
use std::rc::Rc;
use std::sync::mpsc::{self};
use std::sync::RwLock;
use std::{fs, net::IpAddr, process::exit, sync::Arc, thread};
use websocket_lite::{ClientBuilder, Message};

mod handler;
pub mod message;
pub mod plugin_builder;
pub mod runtimebot;

/// 将配置文件写入磁盘
fn config_file_write_and_return() -> Result<ConfigJson, std::io::Error> {
    let ip_addr = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("What is the IP of the OneBot server?")
        .default(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)))
        .interact_text()
        .unwrap();
    let host: IpAddr = ip_addr;

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

/// bot结构体
#[derive(Clone)]
pub struct Bot {
    information: BotInformation,
    main: Vec<BotMain>,
    /* main: Vec<Arc<dyn Fn(PluginBuilder) + Send + Sync + 'static>>, */
    plugins: Vec<Plugin>,
    life: BotLife,
}

#[derive(Clone)]
pub struct BotMain {
    pub name: String,
    pub version: String,
    pub main: Arc<dyn Fn(PluginBuilder) + Send + Sync + 'static>,
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
        set_log();

        Bot {
            information: BotInformation {
                id: 0,
                nickname: "".to_string(),
                main_admin: config_json.main_admin,
                admin: config_json.admins,
                server: config_json.server,
            },
            main: Vec::new(),
            plugins: Vec::new(),
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
        let bot_main = BotMain {
            name,
            version,
            main,
        };
        self.main.push(bot_main)
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

impl Bot {
    /// 运行bot
    /// **注意此函数会阻塞**
    pub fn run(self) {
        let (host, port, access_token) = (
            self.information.server.host,
            self.information.server.port,
            self.information.server.access_token.clone(),
        );

        let bot = Arc::new(RwLock::new(self));

        //处理连接，从msg_tx返回消息
        let (msg_tx, msg_rx): (mpsc::Sender<String>, mpsc::Receiver<String>) = mpsc::channel();

        let msg_tx_clone = msg_tx.clone();
        let access_token_clone = access_token.clone();
        let connect_task = thread::spawn(move || {
            Self::ws_connect(host, port, access_token_clone, msg_tx_clone);
        });

        // 接收插件的api
        let (api_tx, api_rx): (mpsc::Sender<ApiMpsc>, mpsc::Receiver<ApiMpsc>) = mpsc::channel();

        let access_token_clone = access_token.clone();
        let send_api_task = thread::spawn(move || {
            Self::ws_send_api(host, port, access_token_clone, api_rx);
        });


        let main_task_bot = bot.clone();
        let api_tx_clone = api_tx.clone();
        let handler_main_task =
            thread::spawn(move || Self::plugin_main(main_task_bot, api_tx_clone));

        //处理消息，每个消息事件都会来到这里
        for msg in msg_rx {
            let api_tx_clone = api_tx.clone();
            let bot = bot.clone();
            thread::spawn(move || {
                Self::handler_msg(bot, msg, api_tx_clone);
            });
        }

        handler_main_task.join().unwrap();
        connect_task.join().unwrap();
        send_api_task.join().unwrap();
    }

    fn plugin_main(bot: Arc<RwLock<Self>>, api_tx: mpsc::Sender<ApiMpsc>) {
        // 运行所有main()
        let bot_main_job_clone = bot.clone();
        let api_tx_main_job_clone = api_tx.clone();

        let mut main_job_vec;
        {
            let bot = bot_main_job_clone.read().unwrap();
            main_job_vec = bot.main.clone();
        }

        //储存所有main()
        let mut handler_main_job = Vec::new();

        for _ in 0..main_job_vec.len() {
            let main_job = main_job_vec.pop().unwrap();
            let bot_main_job_clone = bot_main_job_clone.clone();
            let api_tx = api_tx_main_job_clone.clone();
            handler_main_job.push(thread::spawn(move || {
                //plugin创建
                let plugin_builder =
                    PluginBuilder::new(main_job.name.clone(), bot_main_job_clone.clone(), api_tx);
                //多线程运行main()
                (main_job.main)(plugin_builder);
            }));
        }
        //等待所有main()结束
        for handler in handler_main_job {
            handler.join().unwrap();
        }
    }


    fn ws_connect(host: IpAddr, port: u16, access_token: String, tx: mpsc::Sender<String>) {
        let url = format!("ws://{}:{}/event", host, port);
        let mut client = ClientBuilder::new(&url).unwrap();
        client.add_header(
            "Authorization".to_string(),
            format!("Bearer {}", access_token),
        );
        let mut ws = match client.connect_insecure() {
            Ok(v) => v,
            Err(e) => exit_and_eprintln(e),
        };
        loop {
            match ws.receive() {
                Ok(msg_result) => match msg_result {
                    Some(msg) => {
                        if !msg.opcode().is_text() {
                            continue;
                        }

                        let text = msg.as_text().unwrap();
                        tx.send(text.to_string()).unwrap();
                    }
                    None => {
                        continue;
                    }
                },
                Err(e) => exit_and_eprintln(e),
            }
        }
    }
    fn ws_send_api(host: IpAddr, port: u16, access_token: String, rx: mpsc::Receiver<ApiMpsc>) {
        let url = format!("ws://{}:{}/api", host, port);
        let mut client = ClientBuilder::new(&url).unwrap();
        client.add_header(
            "Authorization".to_string(),
            format!("Bearer {}", access_token),
        );
        let ws = match client.connect_insecure() {
            Ok(v) => v,
            Err(e) => exit_and_eprintln(e),
        };
        let arc_ws = Rc::new(RefCell::new(ws));

        for (api_msg, return_api_tx) in rx {
            debug!("{}", api_msg);

            let rc_ws_send = arc_ws.clone();

            let msg = Message::text(api_msg.to_string());

            {
                let mut ws = rc_ws_send.borrow_mut();
                ws.send(msg).unwrap();
            }
            loop {
                let mut ws = rc_ws_send.borrow_mut();
                let receive = ws.receive();

                match receive {
                    Ok(msg_result) => {
                        if let Some(msg) = msg_result {
                            if !msg.opcode().is_text() {
                                continue;
                            }
                            drop(ws);
                            let text = msg.as_text().unwrap();
                            let return_value: Value = serde_json::from_str(text).unwrap();

                            if return_value.get("status").unwrap().as_str().unwrap() != "ok" {
                                error!("Api return error: {text}")
                            }

                            debug!("{}", text);

                            let api_tx = match return_api_tx {
                                Some(v) => v,
                                None => break,
                            };

                            let status = return_value.get("status").unwrap().as_str().unwrap();
                            if status == "ok" {
                                api_tx
                                    .send(Ok(return_value.get("data").unwrap().clone()))
                                    .unwrap();
                            } else {
                                api_tx.send(Err(Error::UnknownError())).unwrap();
                            }
                        }
                    }
                    Err(e) => exit_and_eprintln(e),
                }
                break;
            }
        }
    }
}

fn exit_and_eprintln<E>(e: E) -> !
where
    E: std::fmt::Display,
{
    error!("{e}\nBot connection failed, please check the configuration and restart Kovi");
    exit(1);
}

#[macro_export]
macro_rules! build_bot {
    ($($plugin:ident),*) => {{
        let mut bot = kovi::bot::Bot::build();
        $(
            let (crate_name, crate_version) = $plugin::__kovi__get_crate_name();
            println!("Mounting plugin: {}", crate_name);
            bot.mount_main(crate_name, crate_version, std::sync::Arc::new($plugin::main));
        )*
        bot
    }};
}
