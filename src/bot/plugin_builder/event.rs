use serde::{Deserialize, Serialize};
use serde_json::{self, json, Value};
use std::sync::mpsc;

#[derive(Clone)]
pub enum Event {
    OnMsg(OnMsgEvent),
    OnNoticeAll(OnNoticeAllEvent),
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Sender {
    pub user_id: i64,
    pub nickname: String,
    pub card: Option<String>,
    pub sex: String,
    pub age: Option<i32>,
    pub area: Option<String>,
    pub level: Option<String>,
    pub role: Option<String>,
    pub title: Option<String>,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Anonymous {
    pub id: i64,
    pub name: String,
    pub flag: String,
}

#[derive(Debug, Deserialize)]
struct TempOnMsgEvent {
    time: i64,
    self_id: i64,
    post_type: String,
    message_type: String,
    sub_type: String,
    message: Vec<Value>,
    message_id: i32,
    group_id: Option<i64>,
    user_id: i64,
    anonymous: Option<Anonymous>,
    raw_message: String,
    font: i32,
    sender: Sender,
}
#[derive(Debug, Clone)]
pub struct OnMsgEvent {
    pub time: i64,
    pub self_id: i64,
    pub post_type: String,
    pub message_type: String,
    pub sub_type: String,
    pub message: Vec<Value>,
    pub message_id: i32,
    pub group_id: Option<i64>,
    pub user_id: i64,
    pub anonymous: Option<Anonymous>,
    pub raw_message: String,
    pub font: i32,
    pub sender: Sender,

    api_tx: mpsc::Sender<Value>,
    /// 处理过的纯文本，如果是纯图片或无文本，此初为None
    pub text: Option<String>,
    /// 原始未处理的onebot消息，为json格式，使用需处理
    pub original_msg: String,
}

impl OnMsgEvent {
    pub fn new(
        api_tx: mpsc::Sender<Value>,
        msg: &String,
    ) -> Result<OnMsgEvent, Box<dyn std::error::Error>> {
        let temp: TempOnMsgEvent = serde_json::from_str(msg)?;
        let text = {
            let mut text_vec = Vec::new();
            for msg in &temp.message {
                let type_ = msg.get("type").unwrap().as_str().unwrap();
                if type_ == "text" {
                    text_vec.push(
                        msg.get("data")
                            .unwrap()
                            .get("text")
                            .unwrap()
                            .as_str()
                            .unwrap(),
                    );
                };
            }
            if text_vec.len() != 0 {
                Some(text_vec.join("\n"))
            } else {
                None
            }
        };

        Ok(OnMsgEvent {
            time: temp.time,
            self_id: temp.self_id,
            post_type: temp.post_type,
            message_type: temp.message_type,
            sub_type: temp.sub_type,
            message: temp.message,
            message_id: temp.message_id,
            group_id: temp.group_id,
            user_id: temp.user_id,
            anonymous: temp.anonymous,
            raw_message: temp.raw_message,
            font: temp.font,
            sender: temp.sender,
            api_tx,
            text,
            original_msg: msg.clone(),
        })
    }

    /// 快速回复文本
    pub fn reply(&self, msg: &str) {
        let send_msg = if self.message_type == "private" {
            json!({
            "action": "send_msg",
            "params": {
                "message_type":"private",
                "user_id":self.user_id,
                "message":msg,
                "auto_escape":true
            },
            "echo": "123" })
        } else {
            json!({
            "action": "send_msg",
            "params": {
                "message_type":"group",
                "group_id":self.group_id.unwrap(),
                "message":msg,
                "auto_escape":true,
            },
            "echo": "123" })
        };

        self.api_tx.send(send_msg).unwrap();
    }
}

#[derive(Clone)]
pub struct OnNoticeAllEvent {
    pub time: i64,
    pub self_id: i64,
    pub post_type: String,
    pub notice_type: String,

    pub original_json: Value,
    pub original_msg: String,
}
impl OnNoticeAllEvent {
    pub fn new(msg: &String) -> Result<OnNoticeAllEvent, Box<dyn std::error::Error>> {
        let temp: Value = serde_json::from_str(msg)?;
        let time = temp.get("time").unwrap().as_i64().unwrap();
        let self_id = temp.get("self_id").unwrap().as_i64().unwrap();
        let post_type = temp.get("post_type").unwrap().to_string();
        let notice_type = temp.get("notice_type").unwrap().to_string();
        Ok(OnNoticeAllEvent {
            time,
            self_id,
            post_type,
            notice_type,
            original_json: temp,
            original_msg: msg.clone(),
        })
    }
}
