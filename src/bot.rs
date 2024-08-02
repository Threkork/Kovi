use dialoguer::theme::ColorfulTheme;
use dialoguer::Input;
use plugin_builder::{OnType, Plugin, PluginBuilder};
use serde::{Deserialize, Serialize};
use serde_json::{self, json, Value};
use std::net::Ipv4Addr;
use std::sync::mpsc::{self};
use std::sync::RwLock;
use std::{
    fs,
    net::IpAddr,
    process::exit,
    sync::{Arc, Mutex},
    thread,
};
use websocket_lite::{ClientBuilder, Message};

mod handler;
pub mod plugin_builder;
pub mod runtimebot;


/// 将配置文件写入磁盘
fn config_file_write_and_return() -> Result<ConfigJson, std::io::Error> {
    let host: IpAddr = Input::with_theme(&ColorfulTheme::default())
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

/// bot信息结构体
#[derive(Clone)]
pub struct BotInformation {
    id: i64,
    nickname: String,
    main_admin: i64,
    admin: Vec<i64>,
    server: Server,
    debug: bool,
}

/// bot结构体
#[derive(Clone)]
pub struct Bot {
    information: BotInformation,
    main: Vec<Arc<dyn Fn(PluginBuilder) + Send + Sync + 'static>>,
    plugins: Vec<Plugin>,
    life: BotLife,
}

#[derive(Deserialize, Serialize)]
struct ConfigJson {
    main_admin: i64,
    admins: Vec<i64>,
    plugins: Vec<String>,
    server: Server,
    debug: bool,
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
                    eprintln!("{}", err);
                    exit(1)
                }
            },
            Err(_) => match config_file_write_and_return() {
                Ok(v) => v,
                Err(err) => {
                    eprintln!("{}", err);
                    exit(1)
                }
            },
        };


        Bot {
            information: BotInformation {
                id: 0,
                nickname: "".to_string(),
                main_admin: config_json.main_admin,
                admin: config_json.admins,
                server: config_json.server,
                debug: config_json.debug,
            },
            main: Vec::new(),
            plugins: Vec::new(),
            life: BotLife {
                status: LifeStatus::Initial,
            },
        }
    }

    /// 向bot挂载插件，须传入Arc\<Fn\>
    ///
    /// # Examples
    /// ```
    /// let bot = bot
    ///     .mount_main(Arc::new(online::main))
    ///     .mount_main(Arc::new(hello::main));
    /// bot.run()
    /// ```
    pub fn mount_main(mut self, main: Arc<dyn Fn(PluginBuilder) + Send + Sync + 'static>) -> Self {
        self.main.push(main);
        self
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
        let (host, port, access_token, debug) = (
            self.information.server.host,
            self.information.server.port,
            self.information.server.access_token.clone(),
            self.information.debug,
        );

        let bot = Arc::new(RwLock::new(self));

        //处理连接，从msg_tx返回消息
        let (msg_tx, msg_rx): (mpsc::Sender<String>, mpsc::Receiver<String>) = mpsc::channel();

        let msg_tx_clone = msg_tx.clone();
        let access_token_clone = access_token.clone();
        let connect_job = thread::spawn(move || {
            Self::ws_connect(host, port, access_token_clone, msg_tx_clone);
        });

        let (api_tx, api_rx): (mpsc::Sender<Value>, mpsc::Receiver<Value>) = mpsc::channel();

        let access_token_clone = access_token.clone();
        let send_api_job = thread::spawn(move || {
            Self::ws_send_api(host, port, access_token_clone, api_rx, debug);
        });


        // 运行所有main()
        let bot_main_job_clone = bot.clone();
        let api_tx_main_job_clone = api_tx.clone();
        let handler_main_job = thread::spawn(move || {
            let mut main_job_vec;
            {
                let bot = bot_main_job_clone.read().unwrap();
                main_job_vec = bot.main.clone();
            }

            //储存所有main()
            let mut handler_main_job = Vec::new();

            for _i in 0..main_job_vec.len() {
                let main_job = main_job_vec.pop().unwrap();
                let bot_main_job_clone = bot_main_job_clone.clone();
                let api_tx = api_tx_main_job_clone.clone();
                handler_main_job.push(thread::spawn(move || {
                    //plugin创建
                    let plugin_builder = PluginBuilder::new(bot_main_job_clone.clone(), api_tx);
                    //多线程运行main()
                    main_job(plugin_builder);
                }));
            }
            //等待所有main()结束
            for handler in handler_main_job {
                handler.join().unwrap();
            }
        });

        //处理消息，每个消息事件都会来到这里
        for msg in msg_rx {
            let api_tx_clone = api_tx.clone();
            let bot = bot.clone();
            thread::spawn(move || {
                Self::handler_msg(bot, msg, api_tx_clone, debug);
            });
        }

        handler_main_job.join().unwrap();
        connect_job.join().unwrap();
        send_api_job.join().unwrap();
    }

    fn handler_msg(bot: Arc<RwLock<Self>>, msg: String, api_tx: mpsc::Sender<Value>, debug: bool) {
        let msg_json: Value = serde_json::from_str(&msg).unwrap();
        if debug {
            println!("{}", msg_json);
        }
        if let Some(meta_event_type) = msg_json.get("meta_event_type") {
            match meta_event_type.as_str().unwrap() {
                // 生命周期一开始请求bot的信息
                "lifecycle" => {
                    handler::handle_lifecycle(bot.clone(), debug);
                    return;
                }
                "heartbeat" => {
                    return;
                }
                _ => {
                    return;
                }
            }
        }

        let plugins = bot.read().unwrap().plugins.clone();


        //线程储存
        let mut handles = vec![];
        for plugin in plugins {
            for listen in plugin.all_listen {
                match listen.on_type {
                    OnType::OnMsg => {
                        let api_tx = api_tx.clone();
                        let msg = msg.clone();
                        handles.push(thread::spawn(move || {
                            handler::handler_on_msg(api_tx, &msg, listen.handler)
                        }));
                    }
                    OnType::OnAdminMsg => {
                        let api_tx = api_tx.clone();
                        let msg = msg.clone();
                        let bot = bot.clone();

                        if let None = msg_json.get("message_type") {
                            continue;
                        };

                        let user_id = msg_json
                            .get("sender")
                            .unwrap()
                            .get("user_id")
                            .unwrap()
                            .as_i64()
                            .unwrap();

                        handles.push(thread::spawn(move || {
                            let admin_vec = {
                                let bot = bot.read().unwrap();
                                let mut admin_vec = bot.information.admin.clone();
                                admin_vec.push(bot.information.main_admin);
                                admin_vec
                            };

                            if admin_vec.contains(&user_id) {
                                handler::handler_on_msg(api_tx, &msg, listen.handler)
                            }
                        }));
                    }
                    OnType::OnNoticeAll => {
                        let msg = msg.clone();
                        handles.push(thread::spawn(move || {
                            handler::handler_on_notice_all(&msg, listen.handler)
                        }));
                    }
                }
            }
        }


        for handle in handles {
            handle.join().unwrap();
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
    fn ws_send_api(
        host: IpAddr,
        port: u16,
        access_token: String,
        rx: mpsc::Receiver<Value>,
        debug: bool,
    ) {
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
        let arc_ws = Arc::new(Mutex::new(ws));

        for api_msg in rx {
            if debug {
                println!("{}", api_msg);
            }

            let arc_ws_send = arc_ws.clone();
            thread::spawn(move || {
                let mut ws = arc_ws_send.lock().unwrap();
                let msg = Message::text(api_msg.to_string());
                ws.send(msg).unwrap();
                let receive = ws.receive();
                match receive {
                    Ok(msg_result) => match msg_result {
                        Some(msg) => {
                            if !msg.opcode().is_text() {
                                return;
                            }
                            let text = msg.as_text().unwrap();
                            if debug {
                                println!("{}", text);
                            }
                        }
                        None => {
                            return;
                        }
                    },
                    Err(e) => exit_and_eprintln(e),
                }
            });
        }
    }
}

fn exit_and_eprintln<E>(e: E) -> !
where
    E: std::fmt::Display,
{
    eprintln!(
        "Error: {}\nBot connection failed, please check the configuration and restart Kovi",
        e
    );
    exit(1);
}
