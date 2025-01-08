pub use msg_event::MsgEvent;
pub use notice_event::NoticeEvent;
pub use request_event::RequestEvent;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod msg_event;
pub mod notice_event;
pub mod request_event;

#[deprecated(since = "0.11.0", note = "请使用 `MsgEvent` 代替")]
pub type AllMsgEvent = MsgEvent;

#[deprecated(since = "0.11.0", note = "请使用 `NoticeEvent` 代替")]
pub type AllNoticeEvent = NoticeEvent;

#[deprecated(since = "0.11.0", note = "请使用 `RequestEvent` 代替")]
pub type AllRequestEvent = RequestEvent;

#[derive(Error, Debug)]
pub(crate) enum EventBuildError {
    /// 解析出错
    #[error("Parse error: {0}")]
    ParseError(String),
}

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
