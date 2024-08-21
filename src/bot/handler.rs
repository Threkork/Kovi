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
    SendApi,
};
use log::{debug, error, info};
use serde_json::{json, Value};
use std::{
    borrow::Borrow,
    sync::{mpsc, Arc, RwLock},
    thread,
};

impl Bot {
    pub fn handler_msg(bot: Arc<RwLock<Self>>, msg: String, api_tx: mpsc::Sender<ApiMpsc>) {
        let msg_json: Value = serde_json::from_str(&msg).unwrap();

        debug!("{msg_json}");

        if let Some(meta_event_type) = msg_json.get("meta_event_type") {
            match meta_event_type.as_str().unwrap() {
                // 生命周期一开始请求bot的信息
                "lifecycle" => {
                    handler::handle_lifecycle(bot.clone(), api_tx);
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


pub fn handle_lifecycle(bot: Arc<RwLock<Bot>>, api_tx_: mpsc::Sender<ApiMpsc>) {
    let api_msg = SendApi::new("get_login_info", json!({}), "kovi");

    #[allow(clippy::type_complexity)]
    let (api_tx, api_rx): (
        mpsc::Sender<Result<Value, crate::error::Error>>,
        mpsc::Receiver<Result<Value, crate::error::Error>>,
    ) = mpsc::channel();

    api_tx_.send((api_msg, Some(api_tx))).unwrap();

    let receive = api_rx.recv().unwrap();

    let self_info_value = match receive {
        Ok(msg_result) => msg_result,
        Err(e) => exit_and_eprintln(e),
    };

    let self_id = self_info_value.get("user_id").unwrap().as_i64().unwrap();
    let self_name = self_info_value.get("nickname").unwrap().to_string();
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
