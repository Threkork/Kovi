use super::{Anonymous, Sender};
use crate::bot::message::cq_to_arr_inner;
use crate::bot::runtimebot::send_api_request_with_forget;
use crate::{
    bot::{plugin_builder::event::Sex, ApiAndOneshot, SendApi},
    Message,
};
use log::{debug, info};
use serde::Serialize;
use serde_json::{self, json, Value};
use tokio::sync::mpsc;

#[cfg(feature = "cqstring")]
use crate::bot::message::{cq_to_arr, CQMessage};

#[deprecated(since = "0.11.0", note = "请使用 `MsgEvent` 代替")]
pub type AllMsgEvent = MsgEvent;

#[derive(Debug, Clone)]
pub struct MsgEvent {
    /// 事件发生的时间戳
    pub time: i64,
    /// 收到事件的机器人 登陆号
    pub self_id: i64,
    /// 上报类型
    pub post_type: String,
    /// 消息类型
    pub message_type: String,
    /// 消息子类型，如果是好友则是 friend，如果是群临时会话则是 group
    pub sub_type: String,
    /// 消息内容
    pub message: Message,
    /// 消息 ID
    pub message_id: i32,
    /// 群号
    pub group_id: Option<i64>,
    /// 发送者号
    pub user_id: i64,
    /// 匿名信息，如果不是匿名消息则为 null
    pub anonymous: Option<Anonymous>,
    /// 原始消息内容
    pub raw_message: String,
    /// 字体
    pub font: i32,
    /// 发送人信息
    pub sender: Sender,

    /// 处理过的纯文本，如果是纯图片或无文本，此处为None
    pub text: Option<String>,
    /// 处理过的文本，会解析成人类易读形式，里面会包含\[image\]\[face\]等解析后字符串
    pub human_text: String,
    /// 原始的onebot消息，已处理成json格式
    pub original_json: Value,

    api_tx: mpsc::Sender<ApiAndOneshot>,
}

impl MsgEvent {
    pub(crate) fn new(
        api_tx: mpsc::Sender<ApiAndOneshot>,
        msg: &str,
    ) -> Result<MsgEvent, Box<dyn std::error::Error>> {
        let temp: Value = serde_json::from_str(msg)?;

        let temp_object = temp.as_object().unwrap();

        let temp_sender = temp_object["sender"].as_object().unwrap();

        let sender = {
            Sender {
                user_id: temp_sender["user_id"].as_i64().unwrap(),
                nickname: temp_sender
                    .get("nickname")
                    .map(|v| v.as_str().unwrap().to_string()),
                card: temp_sender
                    .get("card")
                    .map(|v| v.as_str().unwrap().to_string()),
                sex: if let Some(v) = temp_sender.get("sex") {
                    match v.as_str().unwrap() {
                        "male" => Some(Sex::Male),
                        "female" => Some(Sex::Female),
                        _ => None,
                    }
                } else {
                    None
                },
                age: temp_sender.get("age").map(|v| v.as_i64().unwrap() as i32),
                area: temp_sender
                    .get("area")
                    .map(|v| v.as_str().unwrap().to_string()),
                level: temp_sender
                    .get("level")
                    .map(|v| v.as_str().unwrap().to_string()),
                role: temp_sender
                    .get("role")
                    .map(|v| v.as_str().unwrap().to_string()),
                title: temp_sender
                    .get("title")
                    .map(|v| v.as_str().unwrap().to_string()),
            }
        };

        let group_id = if let Some(v) = temp_object.get("group_id") {
            v.as_i64()
        } else {
            None
        };
        let message = if temp_object["message"].is_array() {
            let v = temp_object["message"].as_array().unwrap().to_vec();
            Message::from_vec_segment_value(v).unwrap()
        } else {
            let str_v = temp_object["message"].as_str().ok_or(format!("message is not string:{:?}",temp_object["message"]))?;
            let arr_v = cq_to_arr_inner(str_v);
            Message::from_vec_segment_value(arr_v).unwrap()
        };
        let anonymous: Option<Anonymous> =
            if temp_object.get("anonymous").is_none() || temp_object["anonymous"].is_null() {
                None
            } else {
                let anonymous = temp_object["anonymous"].clone();
                Some(serde_json::from_value(anonymous).unwrap())
            };

        let text = {
            let mut text_vec = Vec::new();
            for msg in message.iter() {
                if msg.type_ == "text" {
                    text_vec.push(msg.data.get("text").unwrap().as_str().unwrap());
                };
            }
            if !text_vec.is_empty() {
                Some(text_vec.join("\n").trim().to_string())
            } else {
                None
            }
        };

        let event = MsgEvent {
            human_text: message.to_human_string(),
            time: temp_object["time"].as_i64().unwrap(),
            self_id: temp_object["self_id"].as_i64().unwrap(),
            post_type: temp_object["post_type"].as_str().unwrap().to_string(),
            message_type: temp_object["message_type"].as_str().unwrap().to_string(),
            sub_type: temp_object["sub_type"].as_str().unwrap().to_string(),
            message,
            message_id: temp_object["message_id"].as_i64().unwrap() as i32,
            group_id,
            user_id: temp_object["user_id"].as_i64().unwrap(),
            anonymous,
            raw_message: temp_object["raw_message"].as_str().unwrap().to_string(),
            font: temp_object["font"].as_i64().unwrap() as i32,
            sender,
            api_tx,
            text,
            original_json: temp,
        };
        debug!("{:?}", event);
        Ok(event)
    }
}

impl MsgEvent {
    fn reply_builder<T>(&self, msg: T, auto_escape: bool) -> SendApi
    where
        T: Serialize,
    {
        if self.message_type == "private" {
            SendApi::new(
                "send_msg",
                json!({
                    "message_type":"private",
                "user_id":self.user_id,
                "message":msg,
                "auto_escape":auto_escape,
                }),
                "None",
            )
        } else {
            SendApi::new(
                "send_msg",
                json!({
                    "message_type":"group",
                    "group_id":self.group_id.unwrap(),
                    "message":msg,
                    "auto_escape":auto_escape,
                }),
                "None",
            )
        }
    }

    #[cfg(not(feature = "cqstring"))]
    /// 快速回复消息
    pub fn reply<T>(&self, msg: T)
    where
        Message: From<T>,
        T: Serialize,
    {
        let msg = Message::from(msg);
        let send_msg = self.reply_builder(&msg, false);
        let mut nickname = self.get_sender_nickname();
        nickname.insert(0, ' ');
        let id = &self.sender.user_id;
        let message_type = &self.message_type;
        let group_id = match &self.group_id {
            Some(v) => format!(" {v}"),
            None => "".to_string(),
        };
        let human_msg = msg.to_human_string();
        info!("[reply] [to {message_type}{group_id}{nickname} {id}]: {human_msg}");

        send_api_request_with_forget(&self.api_tx, send_msg)
    }

    #[cfg(feature = "cqstring")]
    /// 快速回复消息
    pub fn reply<T>(&self, msg: T)
    where
        CQMessage: From<T>,
        T: Serialize,
    {
        let msg = CQMessage::from(msg);
        let send_msg = self.reply_builder(&msg, false);
        let mut nickname = self.get_sender_nickname();
        nickname.insert(0, ' ');
        let id = &self.sender.user_id;
        let message_type = &self.message_type;
        let group_id = match &self.group_id {
            Some(v) => format!(" {v}"),
            None => "".to_string(),
        };
        let human_msg = Message::from(msg).to_human_string();
        info!("[reply] [to {message_type}{group_id}{nickname} {id}]: {human_msg}");
        send_api_request_with_forget(&self.api_tx, send_msg);
    }

    #[cfg(not(feature = "cqstring"))]
    /// 快速回复消息并且**引用**
    pub fn reply_and_quote<T>(&self, msg: T)
    where
        Message: From<T>,
        T: Serialize,
    {
        let msg = Message::from(msg).add_reply(self.message_id);
        let send_msg = self.reply_builder(&msg, false);

        let mut nickname = self.get_sender_nickname();
        nickname.insert(0, ' ');
        let id = &self.sender.user_id;
        let message_type = &self.message_type;
        let group_id = match &self.group_id {
            Some(v) => format!(" {v}"),
            None => "".to_string(),
        };
        let human_msg = msg.to_human_string();
        info!("[reply] [to {message_type}{group_id}{nickname} {id}]: {human_msg}");

        send_api_request_with_forget(&self.api_tx, send_msg);
    }

    #[cfg(feature = "cqstring")]
    /// 快速回复消息并且**引用**
    pub fn reply_and_quote<T>(&self, msg: T)
    where
        CQMessage: From<T>,
        T: Serialize,
    {
        let msg = CQMessage::from(msg).add_reply(self.message_id);
        let send_msg = self.reply_builder(&msg, false);

        let mut nickname = self.get_sender_nickname();
        nickname.insert(0, ' ');
        let id = &self.sender.user_id;
        let message_type = &self.message_type;
        let group_id = match &self.group_id {
            Some(v) => format!(" {v}"),
            None => "".to_string(),
        };
        let human_msg = Message::from(msg).to_human_string();
        info!("[reply] [to {message_type}{group_id}{nickname} {id}]: {human_msg}");
        send_api_request_with_forget(&self.api_tx, send_msg);
    }

    #[cfg(feature = "cqstring")]
    /// 快速回复消息，并且**kovi不进行解析，直接发送此字符串**
    pub fn reply_text<T>(&self, msg: T)
    where
        String: From<T>,
        T: Serialize,
    {
        let send_msg = self.reply_builder(&msg, true);
        let mut nickname = self.get_sender_nickname();
        nickname.insert(0, ' ');
        let id = &self.sender.user_id;
        let message_type = &self.message_type;
        let group_id = match &self.group_id {
            Some(v) => format!(" {v}"),
            None => "".to_string(),
        };
        let msg = String::from(msg);
        info!("[reply] [to {message_type}{group_id} {nickname} {id}]: {msg}");
        send_api_request_with_forget(&self.api_tx, send_msg);
    }

    /// 便捷获取文本，如果没有文本则会返回空字符串，如果只需要借用，请使用 `borrow_text()`
    pub fn get_text(&self) -> String {
        match self.text.clone() {
            Some(v) => v,
            None => "".to_string(),
        }
    }

    /// 便捷获取发送者昵称，如果无名字，此处为空字符串
    pub fn get_sender_nickname(&self) -> String {
        if let Some(v) = &self.sender.nickname {
            v.clone()
        } else {
            "".to_string()
        }
    }

    /// 借用 event 的 text，只是做了一下self.text.as_deref()的包装
    pub fn borrow_text(&self) -> Option<&str> {
        self.text.as_deref()
    }

    pub fn is_group(&self) -> bool {
        self.group_id.is_some()
    }

    pub fn is_private(&self) -> bool {
        self.group_id.is_none()
    }
}
