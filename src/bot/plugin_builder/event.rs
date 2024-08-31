pub use msg_event::AllMsgEvent;
use serde::{ Deserialize, Serialize};
use serde_json::{self, Value};

mod msg_event;


#[derive(Debug, Copy, Clone)]
pub enum Sex {
    Male,
    Female,
}

#[derive(Debug, Clone)]
pub struct Sender {
    pub user_id: i64,
    pub nickname: Option<String>,
    pub card: Option<String>,
    pub sex: Option<Sex>,
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


#[derive(Debug, Clone)]
pub struct AllNoticeEvent {
    /// 事件发生的时间戳
    pub time: i64,
    /// 收到事件的机器人 登陆号
    pub self_id: i64,
    /// 上报类型
    pub post_type: String,
    /// 通知类型
    pub notice_type: String,

    /// 原始的onebot消息，已处理成json格式
    pub original_json: Value,
    /// 原始未处理的onebot消息，为json格式，使用需处理
    pub original_msg: String,
}
impl AllNoticeEvent {
    pub fn new(msg: &str) -> Result<AllNoticeEvent, Box<dyn std::error::Error>> {
        let temp: Value = serde_json::from_str(msg)?;
        let time = temp.get("time").unwrap().as_i64().unwrap();
        let self_id = temp.get("self_id").unwrap().as_i64().unwrap();
        let post_type = temp.get("post_type").unwrap().to_string();
        let notice_type = temp.get("notice_type").unwrap().to_string();
        Ok(AllNoticeEvent {
            time,
            self_id,
            post_type,
            notice_type,
            original_json: temp,
            original_msg: msg.to_string(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct AllRequestEvent {
    /// 事件发生的时间戳
    pub time: i64,
    /// 收到事件的机器人 登陆号
    pub self_id: i64,
    /// 上报类型
    pub post_type: String,
    /// 请求类型
    pub request_type: String,

    /// 原始的onebot消息，已处理成json格式
    pub original_json: Value,
    /// 原始未处理的onebot消息，为json格式，使用需处理
    pub original_msg: String,
}
impl AllRequestEvent {
    pub fn new(msg: &str) -> Result<AllRequestEvent, Box<dyn std::error::Error>> {
        let temp: Value = serde_json::from_str(msg)?;
        let time = temp.get("time").unwrap().as_i64().unwrap();
        let self_id = temp.get("self_id").unwrap().as_i64().unwrap();
        let post_type = temp.get("post_type").unwrap().to_string();
        let request_type = temp.get("request_type").unwrap().to_string();
        Ok(AllRequestEvent {
            time,
            self_id,
            post_type,
            request_type,
            original_json: temp,
            original_msg: msg.to_string(),
        })
    }
}
