use super::{
    plugin_builder::{event::Event, ListenFn},
    runtimebot::ApiMpsc,
    Bot, LifeStatus,
};
use crate::bot::{
    exit_and_eprintln, handler,
    plugin_builder::{
        event::{AllMsgEvent, AllNoticeEvent, AllRequestEvent},
        Listen, OnType,
    },
};
use log::{debug, error, info};
use serde_json::{json, Value};
use std::{
    borrow::Borrow,
    sync::{mpsc, Arc, RwLock},
    thread,
};
use websocket_lite::{ClientBuilder, Message};

impl Bot {
    pub fn handler_msg(bot: Arc<RwLock<Self>>, msg: String, api_tx: mpsc::Sender<ApiMpsc>) {
        let msg_json: Value = serde_json::from_str(&msg).unwrap();

        debug!("{msg_json}");

        if let Some(meta_event_type) = msg_json.get("meta_event_type") {
            match meta_event_type.as_str().unwrap() {
                // 生命周期一开始请求bot的信息
                "lifecycle" => {
                    handler::handle_lifecycle(bot.clone());
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

        let event = match msg_json.get("post_type").unwrap().as_str().unwrap() {
            "message" => {
                let e = match AllMsgEvent::new(api_tx, &msg) {
                    Ok(event) => event,
                    Err(e) => {
                        error!("{e}");
                        return;
                    }
                };
                let text = &e.human_text;
                let mut nickname = e.get_sender_nickname();
                nickname.insert(0, ' ');
                let id = &e.sender.user_id;
                let message_type = &e.message_type;
                let group_id = match &e.group_id {
                    Some(v) => format!(" {v}"),
                    None => "".to_string(),
                };
                info!("[{message_type}{group_id}{nickname} {id}]: {text}");
                Event::OnMsg(e)
            }
            "notice" => {
                let e = match AllNoticeEvent::new(&msg) {
                    Ok(event) => event,
                    Err(e) => {
                        error!("{e}");
                        return;
                    }
                };
                Event::OnAllNotice(e)
            }
            "request" => {
                let e = match AllRequestEvent::new(&msg) {
                    Ok(event) => event,
                    Err(e) => {
                        error!("{e}");
                        return;
                    }
                };
                Event::OnAllRequest(e)
            }

            _ => {
                panic!()
            }
        };

        let event = Arc::new(event);

        for plugin in plugins {
            for listen in plugin.all_listen {
                handle_listen(listen, event.clone(), bot.clone());
            }
        }
    }
}

fn handle_listen(listen: Listen, event: Arc<Event>, bot: Arc<RwLock<Bot>>) {
    match listen.on_type {
        OnType::OnMsg => {
            if let Event::OnMsg(_) = event.borrow() {
            } else {
                return;
            };
            let event = Arc::clone(&event);
            thread::spawn(move || handler::handler_on(&event, listen.handler));
        }
        OnType::OnAdminMsg => {
            let bot = bot.clone();
            let event = Arc::clone(&event);

            let event_b = if let Event::OnMsg(e) = event.borrow() {
                e
            } else {
                return;
            };
            let user_id = event_b.user_id;

            let admin_vec = {
                let bot = bot.read().unwrap();
                let mut admin_vec = bot.information.admin.clone();
                admin_vec.push(bot.information.main_admin);
                admin_vec
            };
            if admin_vec.contains(&user_id) {
                handler::handler_on(&event, listen.handler)
            }
        }
        OnType::OnAllNotice => {
            if let Event::OnAllNotice(_) = event.borrow() {
            } else {
                return;
            };
            let event = Arc::clone(&event);
            thread::spawn(move || handler::handler_on(&event, listen.handler));
        }
        OnType::OnAllRequest => {
            if let Event::OnAllRequest(_) = event.borrow() {
            } else {
                return;
            };
            let event = Arc::clone(&event);
            thread::spawn(move || handler::handler_on(&event, listen.handler));
        }
    }
}

pub fn handler_on(event: &Event, handler: ListenFn) {
    handler(event)
}


pub fn handle_lifecycle(bot: Arc<RwLock<Bot>>) {
    let (host, port, access_token) = {
        let bot = bot.read().unwrap();
        (
            bot.information.server.host,
            bot.information.server.port,
            bot.information.server.access_token.clone(),
        )
    };
    let url = format!("ws://{}:{}/api", host, port);
    let mut client = ClientBuilder::new(&url).unwrap();
    client.add_header(
        "Authorization".to_string(),
        format!("Bearer {}", access_token),
    );
    let mut ws = match client.connect_insecure() {
        Ok(v) => v,
        Err(e) => exit_and_eprintln(e),
    };
    let api_msg = json!({
                        "action": "get_login_info","echo": "None"});
    let api_msg = Message::text(api_msg.to_string());
    ws.send(api_msg).unwrap();
    let receive = ws.receive();
    let self_info_value: Value;
    match receive {
        Ok(msg_result) => match msg_result {
            Some(msg) => {
                if !msg.opcode().is_text() {
                    return;
                }
                let text = msg.as_text().unwrap();

                debug!("{}", text);

                self_info_value = serde_json::from_str(text).unwrap();
            }
            None => exit_and_eprintln("Error, UnknownError"),
        },
        Err(e) => exit_and_eprintln(e),
    }

    let self_id = self_info_value
        .get("data")
        .unwrap()
        .get("user_id")
        .unwrap()
        .as_i64()
        .unwrap();
    let self_name = self_info_value
        .get("data")
        .unwrap()
        .get("nickname")
        .unwrap()
        .to_string();
    info!(
        "Bot connection successful，Nickname:{},ID:{}",
        self_name, self_id
    );

    {
        let mut bot = bot.write().unwrap();
        bot.information.id = self_id;
        bot.information.nickname = self_name;
        bot.life.status = LifeStatus::Running;
    }
}
