use crate::bot::*;
use log::{debug, error, info, warn};
#[cfg(feature = "message_sent")]
use plugin_builder::AllMsgFn;
use plugin_builder::{
    event::{MsgEvent, NoticeEvent, RequestEvent},
    AllNoticeFn, AllRequestFn, ListenMsgFn, NoArgsFn,
};
use serde_json::{json, Value};
use std::sync::{Arc, RwLock};
use tokio::sync::oneshot;

/// Kovi内部事件
pub enum InternalEvent {
    KoviEvent(KoviEvent),
    OneBotEvent(String),
}

pub enum KoviEvent {
    Drop,
}

impl Bot {
    pub(crate) async fn handler_event(
        bot: Arc<RwLock<Self>>,
        event: InternalEvent,
        api_tx: mpsc::Sender<ApiAndOneshot>,
    ) {
        match event {
            InternalEvent::KoviEvent(event) => Self::handle_kovi_event(bot, event).await,
            InternalEvent::OneBotEvent(msg) => Self::handler_msg(bot, msg, api_tx).await,
        }
    }

    pub(crate) async fn handle_kovi_event(bot: Arc<RwLock<Self>>, event: KoviEvent) {
        let drop_task = {
            let mut bot_write = bot.write().unwrap();
            match event {
                KoviEvent::Drop => {
                    #[cfg(any(feature = "save_plugin_status", feature = "save_bot_admin"))]
                    bot_write.save_bot_status();
                    let mut task_vec = Vec::new();
                    for plugin in bot_write.plugins.values_mut() {
                        task_vec.push(plugin.shutdown());
                    }
                    Some(task_vec)
                }
            }
        };
        if let Some(drop_task) = drop_task {
            for task in drop_task {
                let _ = task.await;
            }
        }
    }

    async fn handler_msg(bot: Arc<RwLock<Self>>, msg: String, api_tx: mpsc::Sender<ApiAndOneshot>) {
        let msg_json: Value = serde_json::from_str(&msg).unwrap();

        debug!("{msg_json}");

        if let Some(meta_event_type) = msg_json.get("meta_event_type") {
            match meta_event_type.as_str().unwrap() {
                // 生命周期一开始请求bot的信息
                "lifecycle" => {
                    Self::handler_lifecycle(api_tx).await;
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
            Msg(MsgEvent),
            #[cfg(feature = "message_sent")]
            MsgSent(MsgEvent),
            AllNotice(NoticeEvent),
            AllRequest(RequestEvent),
        }

        let event = match msg_json.get("post_type").unwrap().as_str().unwrap() {
            "message" => {
                let e = match MsgEvent::new(api_tx, &msg) {
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
            #[cfg(feature = "message_sent")]
            "message_sent" => {
                let e = match MsgEvent::new(api_tx, &msg) {
                    Ok(event) => event,
                    Err(e) => {
                        error!("{e}");
                        return;
                    }
                };
                OneBotEvent::MsgSent(e)
            }
            "notice" => {
                let e = match NoticeEvent::new(&msg) {
                    Ok(event) => event,
                    Err(e) => {
                        error!("{e}");
                        return;
                    }
                };
                OneBotEvent::AllNotice(e)
            }
            "request" => {
                let e = match RequestEvent::new(&msg) {
                    Ok(event) => event,
                    Err(e) => {
                        error!("{e}");
                        return;
                    }
                };
                OneBotEvent::AllRequest(e)
            }

            _ => {
                warn!("Unknown event: {msg}");
                return;
            }
        };

        let bot_read = bot.read().unwrap();

        match event {
            OneBotEvent::Msg(e) => {
                let e = Arc::new(e);
                for (name, plugin) in bot_read.plugins.iter() {
                    // 判断是否黑白名单
                    #[cfg(feature = "plugin-access-control")]
                    if !is_access(plugin, &e) {
                        continue;
                    }

                    let name_ = Arc::new(name.clone());

                    for listen in &plugin.listen.msg {
                        let name = name_.clone();
                        let event_clone = Arc::clone(&e);
                        let bot_clone = bot.clone();
                        let listen = listen.clone();
                        let mut enabled = plugin.enabled.subscribe();
                        tokio::spawn(async move {
                            tokio::select! {
                                _ = PLUGIN_NAME.scope(name, Self::handle_msg(listen, event_clone, bot_clone)) => {}
                                _ = async {
                                        loop {
                                            enabled.changed().await.unwrap();
                                            if !*enabled.borrow_and_update() {
                                                break;
                                            }
                                        }
                                } => {}
                            }
                        });
                    }
                }
            }
            #[cfg(feature = "message_sent")]
            OneBotEvent::MsgSent(e) => {
                let e = Arc::new(e);
                for (name, plugin) in bot_read.plugins.iter() {
                    let name_ = Arc::new(name.clone());

                    for listen in &plugin.listen.msg_sent {
                        let name = name_.clone();
                        let event_clone = Arc::clone(&e);
                        let listen = listen.clone();
                        let mut enabled = plugin.enabled.subscribe();

                        tokio::spawn(async move {
                            tokio::select! {
                                _ = PLUGIN_NAME.scope(name, Self::handler_msg_sent(listen,event_clone)) => {}
                                _ = async {
                                        loop {
                                            enabled.changed().await.unwrap();
                                            if !*enabled.borrow_and_update() {
                                                break;
                                            }
                                        }
                                } => {}
                            }
                        });
                    }
                }
            }
            OneBotEvent::AllNotice(e) => {
                let e = Arc::new(e);
                for (name, plugin) in bot_read.plugins.iter() {
                    let name_ = Arc::new(name.clone());

                    for listen in &plugin.listen.notice {
                        let name = name_.clone();
                        let event_clone = Arc::clone(&e);
                        let listen = listen.clone();
                        let mut enabled = plugin.enabled.subscribe();

                        tokio::spawn(async move {
                            tokio::select! {
                                _ = PLUGIN_NAME.scope(name, Self::handler_notice(listen, event_clone)) => {}
                                _ = async {
                                        loop {
                                            enabled.changed().await.unwrap();
                                            if !*enabled.borrow_and_update() {
                                                break;
                                            }
                                        }
                                } => {}
                            }
                        });
                    }
                }
            }
            OneBotEvent::AllRequest(e) => {
                let e = Arc::new(e);
                for (name, plugin) in bot_read.plugins.iter() {
                    let name_ = Arc::new(name.clone());

                    for listen in &plugin.listen.request {
                        let name = name_.clone();
                        let event_clone = Arc::clone(&e);
                        let listen = listen.clone();
                        let mut enabled = plugin.enabled.subscribe();

                        tokio::spawn(async move {
                            tokio::select! {
                                _ = PLUGIN_NAME.scope(name, Self::handler_request(listen, event_clone)) => {}
                                _ = async {
                                        loop {
                                            enabled.changed().await.unwrap();
                                            if !*enabled.borrow_and_update() {
                                                break;
                                        }}} => {}
                            }
                        });
                    }
                }
            }
        }
    }

    async fn handle_msg(listen: Arc<ListenMsgFn>, e: Arc<MsgEvent>, bot: Arc<RwLock<Bot>>) {
        match &*listen {
            ListenMsgFn::Msg(handler) => {
                handler(e).await;
            }

            ListenMsgFn::AdminMsg(handler) => {
                let user_id = e.user_id;
                let admin_vec = {
                    let bot = bot.read().unwrap();
                    let mut admin_vec = bot
                        .information
                        .deputy_admins
                        .iter()
                        .cloned()
                        .collect::<Vec<_>>();
                    admin_vec.push(bot.information.main_admin);
                    admin_vec
                };
                if admin_vec.contains(&user_id) {
                    handler(e).await;
                }
            }
            ListenMsgFn::PrivateMsg(handler) => {
                if !e.is_group() {
                    handler(e).await;
                }
            }
            ListenMsgFn::GroupMsg(handler) => {
                if e.is_group() {
                    handler(e).await;
                }
            }
        }
    }

    #[cfg(feature = "message_sent")]
    async fn handler_msg_sent(listen: AllMsgFn, e: Arc<MsgEvent>) {
        listen(e).await;
    }

    async fn handler_notice(listen: AllNoticeFn, e: Arc<NoticeEvent>) {
        listen(e).await;
    }

    async fn handler_request(listen: AllRequestFn, e: Arc<RequestEvent>) {
        listen(e).await;
    }

    pub(crate) async fn handler_drop(listen: NoArgsFn) {
        listen().await;
    }

    pub(crate) async fn handler_lifecycle(api_tx_: mpsc::Sender<ApiAndOneshot>) {
        let api_msg = SendApi::new("get_login_info", json!({}), "kovi");

        #[allow(clippy::type_complexity)]
        let (api_tx, api_rx): (
            oneshot::Sender<Result<ApiReturn, ApiReturn>>,
            oneshot::Receiver<Result<ApiReturn, ApiReturn>>,
        ) = oneshot::channel();

        api_tx_.send((api_msg, Some(api_tx))).await.unwrap();

        let receive = match api_rx.await {
            Ok(v) => v,
            Err(e) => {
                error!("Lifecycle Error, get bot info failed: {}", e);
                return;
            }
        };

        let self_info_value = match receive {
            Ok(v) => v,
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
    }
}

#[cfg(feature = "plugin-access-control")]
fn is_access(plugin: &BotPlugin, event: &MsgEvent) -> bool {
    if !plugin.access_control {
        return true;
    }

    let access_list = &plugin.access_list;
    let in_group = event.is_group();

    match (plugin.list_mode, in_group) {
        (AccessControlMode::WhiteList, true) => access_list
            .groups
            .contains(event.group_id.as_ref().unwrap()),
        (AccessControlMode::WhiteList, false) => {
            access_list.friends.contains(&event.sender.user_id)
        }
        (AccessControlMode::BlackList, true) => !access_list
            .groups
            .contains(event.group_id.as_ref().unwrap()),
        (AccessControlMode::BlackList, false) => {
            !access_list.friends.contains(&event.sender.user_id)
        }
    }
}
