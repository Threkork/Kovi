use crate::bot::*;
use log::{debug, error, info};
use plugin_builder::{
    event::{AllMsgEvent, AllNoticeEvent, AllRequestEvent},
    ListenFn,
};
use serde_json::{json, Value};
use std::sync::{mpsc, Arc, RwLock};


/// Kovi内部事件
pub enum InternalEvent {
    KoviEvent(KoviEvent),
    OneBotEvent(String),
}

pub enum KoviEvent {
    Drop,
}

impl Bot {
    async fn handle_kovi_event(bot: Arc<RwLock<Self>>, event: KoviEvent) {
        let plugins = bot.read().unwrap().plugins.clone();
        // let event = Arc::new(event);
        #[allow(clippy::needless_late_init)]
        let drop_task;
        match event {
            KoviEvent::Drop => {
                let mut task_vec = Vec::new();
                for plugin in plugins.into_values() {
                    for listen in plugin {
                        // let event_clone = Arc::clone(&event);
                        task_vec.push(tokio::spawn(async move { handler_kovi_drop(listen).await }));
                    }
                }
                drop_task = Some(task_vec)
            }
        }
        if let Some(drop_task) = drop_task {
            for task in drop_task {
                task.await.unwrap()
            }
        }
    }
    pub async fn handler_event(
        bot: Arc<RwLock<Self>>,
        event: InternalEvent,
        api_tx: mpsc::Sender<ApiMpsc>,
    ) {
        match event {
            InternalEvent::KoviEvent(event) => Self::handle_kovi_event(bot, event).await,
            InternalEvent::OneBotEvent(msg) => Self::handler_msg(bot, msg, api_tx),
        }
    }


    pub fn handler_msg(bot: Arc<RwLock<Self>>, msg: String, api_tx: mpsc::Sender<ApiMpsc>) {
        let msg_json: Value = serde_json::from_str(&msg).unwrap();

        debug!("{msg_json}");

        if let Some(meta_event_type) = msg_json.get("meta_event_type") {
            match meta_event_type.as_str().unwrap() {
                // 生命周期一开始请求bot的信息
                "lifecycle" => {
                    handler_lifecycle(bot, api_tx);
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

        enum OneBotEvent {
            Msg(AllMsgEvent),
            AllNotice(AllNoticeEvent),
            AllRequest(AllRequestEvent),
        }

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
                OneBotEvent::Msg(e)
            }
            "notice" => {
                let e = match AllNoticeEvent::new(&msg) {
                    Ok(event) => event,
                    Err(e) => {
                        error!("{e}");
                        return;
                    }
                };
                OneBotEvent::AllNotice(e)
            }
            "request" => {
                let e = match AllRequestEvent::new(&msg) {
                    Ok(event) => event,
                    Err(e) => {
                        error!("{e}");
                        return;
                    }
                };
                OneBotEvent::AllRequest(e)
            }

            _ => {
                panic!()
            }
        };

        let plugins = bot.read().unwrap().plugins.clone();

        match event {
            OneBotEvent::Msg(e) => {
                let e = Arc::new(e);
                for plugin in plugins.into_values() {
                    for listen in plugin {
                        let event_clone = Arc::clone(&e);
                        let bot_clone = bot.clone();
                        tokio::spawn(handle_msg(listen, event_clone, bot_clone));
                    }
                }
            }
            OneBotEvent::AllNotice(e) => {
                let e = Arc::new(e);
                for plugin in plugins.into_values() {
                    for listen in plugin {
                        let event_clone = Arc::clone(&e);
                        tokio::spawn(handler_notice(listen, event_clone));
                    }
                }
            }
            OneBotEvent::AllRequest(e) => {
                let e = Arc::new(e);
                for plugin in plugins.into_values() {
                    for listen in plugin {
                        let event_clone = Arc::clone(&e);
                        tokio::spawn(handler_request(listen, event_clone));
                    }
                }
            }
        }
    }
}

async fn handle_msg(listen: ListenFn, e: Arc<AllMsgEvent>, bot: Arc<RwLock<Bot>>) {
    match listen {
        ListenFn::MsgFn(handler) => {
            handler(&e);
        }
        ListenFn::MsgAsyncFn(handler) => {
            handler(e).await;
        }

        ListenFn::AdminMsgFn(handler) => {
            let user_id = e.user_id;
            let admin_vec = {
                let bot = bot.read().unwrap();
                let mut admin_vec = bot.information.admin.clone();
                admin_vec.push(bot.information.main_admin);
                admin_vec
            };
            if admin_vec.contains(&user_id) {
                handler(&e);
            }
        }
        ListenFn::AdminMsgAsyncFn(handler) => {
            let user_id = e.user_id;
            let admin_vec = {
                let bot = bot.read().unwrap();
                let mut admin_vec = bot.information.admin.clone();
                admin_vec.push(bot.information.main_admin);
                admin_vec
            };
            if admin_vec.contains(&user_id) {
                handler(e).await;
            }
        }
        _ => {}
    }
}

async fn handler_notice(listen: ListenFn, e: Arc<AllNoticeEvent>) {
    match listen {
        ListenFn::AllNoticeFn(handler) => {
            handler(&e);
        }
        ListenFn::AllNoticeAsyncFn(handler) => {
            handler(e).await;
        }
        _ => {}
    }
}

async fn handler_request(listen: ListenFn, e: Arc<AllRequestEvent>) {
    match listen {
        ListenFn::AllRequestFn(handler) => {
            handler(&e);
        }
        ListenFn::AllRequestAsyncFn(handler) => {
            handler(e).await;
        }
        _ => {}
    }
}

async fn handler_kovi_drop(listen: ListenFn) {
    match listen {
        ListenFn::KoviEventDropFn(handler) => {
            info!("A plugin is performing its shutdown tasks, please wait. 有插件正在进行结束工作，请稍候。");
            handler();
        }
        ListenFn::KoviEventDropAsyncFn(handler) => {
            info!("A plugin is performing its shutdown tasks, please wait. 有插件正在进行结束工作，请稍候。");
            handler().await;
        }
        _ => {}
    }
}


pub fn handler_lifecycle(bot: Arc<RwLock<Bot>>, api_tx_: mpsc::Sender<ApiMpsc>) {
    let api_msg = SendApi::new("get_login_info", json!({}), "kovi");

    #[allow(clippy::type_complexity)]
    let (api_tx, api_rx): (
        mpsc::Sender<Result<ApiReturn, ApiReturn>>,
        mpsc::Receiver<Result<ApiReturn, ApiReturn>>,
    ) = mpsc::channel();

    api_tx_.send((api_msg, Some(api_tx))).unwrap();

    let receive = api_rx.recv().unwrap();

    let self_info_value = match receive {
        Ok(msg_result) => msg_result,
        Err(e) => {
            error!("Lifecycle Error, get bot info failed: {}", e);
            return;
        }
    };

    let self_id = self_info_value
        .data
        .get("user_id")
        .unwrap()
        .as_i64()
        .unwrap();
    let self_name = self_info_value.data.get("nickname").unwrap().to_string();
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
