use super::{
    plugin_builder::{
        event::{Event, OnAllNoticeEvent, OnMsgEvent},
        ListenFn,
    },
    ApiMpsc, Bot, LifeStatus,
};
use crate::bot::exit_and_eprintln;
use serde_json::{json, Value};
use std::sync::{mpsc, Arc, RwLock};
use websocket_lite::{ClientBuilder, Message};

pub fn handle_lifecycle(bot: Arc<RwLock<Bot>>, debug: bool) {
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
                if debug {
                    println!("{}", text);
                }
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
    println!(
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

pub fn handler_on_msg(api_tx: mpsc::Sender<ApiMpsc>, msg: &str, handler: ListenFn) {
    let event = match OnMsgEvent::new(api_tx, msg) {
        Ok(event) => event,
        Err(_e) => {
            return;
        }
    };
    if let Err(err) = handler(&Event::OnMsg(event)) {
        eprintln!("{}", err);
    }
}


pub fn handler_on_all_notice(msg: &str, handler: ListenFn) {
    let event = match OnAllNoticeEvent::new(msg) {
        Ok(event) => event,
        Err(_e) => {
            return;
        }
    };

    if let Err(err) = handler(&Event::OnAllNotice(event)) {
        eprintln!("{}", err);
    }
}
