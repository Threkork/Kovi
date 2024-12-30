pub use msg_event::MsgEvent;
use serde::{Deserialize, Serialize};
use serde_json::{self, Value};

pub mod msg_event;

#[deprecated(since = "0.11.0", note = "请使用 `NoticeEvent` 代替")]
pub type AllNoticeEvent = NoticeEvent;

#[deprecated(since = "0.11.0", note = "请使用 `RequestEvent` 代替")]
pub type AllRequestEvent = RequestEvent;

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
pub struct NoticeEvent {
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
}
impl NoticeEvent {
    pub(crate) fn new(msg: &str) -> Result<NoticeEvent, Box<dyn std::error::Error>> {
        let temp: Value = serde_json::from_str(msg)?;
        let time = temp.get("time").unwrap().as_i64().unwrap();
        let self_id = temp.get("self_id").unwrap().as_i64().unwrap();
        let post_type = temp.get("post_type").unwrap().to_string();
        let notice_type = temp.get("notice_type").unwrap().to_string();
        Ok(NoticeEvent {
            time,
            self_id,
            post_type,
            notice_type,
            original_json: temp,
        })
    }
}

#[derive(Debug, Clone)]
pub struct RequestEvent {
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
}
impl RequestEvent {
    pub(crate) fn new(msg: &str) -> Result<RequestEvent, Box<dyn std::error::Error>> {
        let temp: Value = serde_json::from_str(msg)?;
        let time = temp.get("time").unwrap().as_i64().unwrap();
        let self_id = temp.get("self_id").unwrap().as_i64().unwrap();
        let post_type = temp.get("post_type").unwrap().to_string();
        let request_type = temp.get("request_type").unwrap().to_string();
        Ok(RequestEvent {
            time,
            self_id,
            post_type,
            request_type,
            original_json: temp,
        })
    }
}
