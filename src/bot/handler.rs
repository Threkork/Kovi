#[cfg(feature = "message_sent")]
use crate::types::MsgFn;
use crate::{
    bot::*,
    plugin::PLUGIN_NAME,
    types::{ApiAndOneshot, NoArgsFn, NoticeFn, RequestFn},
};
use log::{debug, error, info, warn};
use parking_lot::RwLock;
use plugin_builder::{
    ListenMsgFn,
    event::{MsgEvent, NoticeEvent, RequestEvent},
};
use serde_json::{Value, json};
use std::sync::Arc;
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
            let mut bot_write = bot.write();
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
        let msg_json: Value = match serde_json::from_str(&msg) {
            Ok(json) => json,
            Err(e) => {
                error!("Failed to parse JSON from message: {}", e);
                return;
            }
        };

        debug!("{msg_json}");

        if let Some(meta_event_type) = msg_json.get("meta_event_type") {
            match meta_event_type.as_str() {
                Some("lifecycle") => {
                    Self::handler_lifecycle(api_tx).await;
                    return;
                }
                Some("heartbeat") => {
                    return;
                }
                Some(_) | None => {
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

        let post_type = match msg_json.get("post_type") {
            Some(value) => match value.as_str() {
                Some(s) => s,
                None => {
                    error!("Invalid 'post_type' value in message JSON");
                    return;
                }
            },
            None => {
                error!("Missing 'post_type' in message JSON");
                return;
            }
        };

        let event = match post_type {
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

            "message_sent" => {
                #[cfg(not(feature = "message_sent"))]
                return;

                #[cfg(feature = "message_sent")]
                {
                    let e = match MsgEvent::new(api_tx, &msg) {
                        Ok(event) => event,
                        Err(e) => {
                            error!("{e}");
                            return;
                        }
                    };
                    OneBotEvent::MsgSent(e)
                }
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

        let bot_read = bot.read();

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
                        let enabled = plugin.enabled.subscribe();
                        RT.get().unwrap().spawn(async move {
                            tokio::select! {
                                _ = PLUGIN_NAME.scope(name, Self::handle_msg(listen, event_clone, bot_clone)) => {}
                                _ = monitor_enabled_state(enabled) => {}
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
                        let enabled = plugin.enabled.subscribe();

                        RT.get().unwrap().spawn(async move {
                            tokio::select! {
                                _ = PLUGIN_NAME.scope(name, Self::handler_msg_sent(listen,event_clone)) => {}
                                _ = monitor_enabled_state(enabled) => {}
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
                        let enabled = plugin.enabled.subscribe();

                        RT.get().unwrap().spawn(async move {
                            tokio::select! {
                                _ = PLUGIN_NAME.scope(name, Self::handler_notice(listen, event_clone)) => {}
                                _ = monitor_enabled_state(enabled) => {}
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
                        let enabled = plugin.enabled.subscribe();

                        RT.get().unwrap().spawn(async move {
                            tokio::select! {
                                _ = PLUGIN_NAME.scope(name, Self::handler_request(listen, event_clone)) => {}
                                _ = monitor_enabled_state(enabled) => {}
                            }
                        });
                    }
                }
            }
        }

        async fn monitor_enabled_state(mut enabled: watch::Receiver<bool>) {
            loop {
                enabled.changed().await.unwrap();
                if !*enabled.borrow_and_update() {
                    break;
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
                    let bot = bot.read();
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
    async fn handler_msg_sent(listen: MsgFn, e: Arc<MsgEvent>) {
        listen(e).await;
    }

    async fn handler_notice(listen: NoticeFn, e: Arc<NoticeEvent>) {
        listen(e).await;
    }

    async fn handler_request(listen: RequestFn, e: Arc<RequestEvent>) {
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

        let self_id = match self_info_value.data.get("user_id") {
            Some(user_id) => match user_id.as_i64() {
                Some(id) => id,
                None => {
                    error!("Expected 'user_id' to be an integer");
                    return;
                }
            },
            None => {
                error!("Missing 'user_id' in self_info_value data");
                return;
            }
        };
        let self_name = match self_info_value.data.get("nickname") {
            Some(nickname) => nickname.to_string(),
            None => {
                error!("Missing 'nickname' in self_info_value data");
                return;
            }
        };
        info!(
            "Bot connection successful，Nickname:{},ID:{}",
            self_name, self_id
        );
    }
}

#[cfg(feature = "plugin-access-control")]
fn is_access(plugin: &Plugin, event: &MsgEvent) -> bool {
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
