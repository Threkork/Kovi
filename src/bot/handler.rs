use super::plugin_builder::event::{Event, OnMsgEvent, OnNoticeAllEvent};
use serde_json::Value;
use std::sync::{mpsc, Arc};

pub fn handler_on_msg(
    api_tx: mpsc::Sender<Value>,
    msg: &String,
    handler: Arc<dyn Fn(&Event) -> Result<(), ()> + Send + Sync + 'static>,
) {
    let event = match OnMsgEvent::new(api_tx, msg) {
        Ok(event) => event,
        Err(_e) => {
            return;
        }
    };
    handler(&Event::OnMsg(event)).unwrap();
}

pub fn handler_on_notice_all(
    msg: &String,
    handler: Arc<dyn Fn(&Event) -> Result<(), ()> + Send + Sync + 'static>,
) {
    let event = match OnNoticeAllEvent::new(msg) {
        Ok(event) => event,
        Err(_e) => {
            return;
        }
    };
    handler(&Event::OnNoticeAll(event)).unwrap();
}
