use serde::Serialize;
use serde_json::{json, Value};
use std::fmt::Display;

use super::{Message, Segment};

#[cfg(feature = "cqstring")]
use super::CQMessage;

impl Message {
    /// 在消息加上文字
    pub fn add_text<T>(mut self, text: T) -> Self
    where
        String: From<T>,
        T: Serialize + Display,
    {
        self.push(Segment {
            type_: "text".to_string(),
            data: json!({ "text": text }),
        });
        self
    }

    /// 消息加上at
    pub fn add_at(mut self, id: &str) -> Self {
        self.0.push(Segment {
            type_: "at".to_string(),
            data: json!({ "qq": id }),
        });
        self
    }

    /// 消息加上引用
    pub fn add_reply(mut self, message_id: i32) -> Self {
        self.0.insert(0, Segment {
            type_: "reply".to_string(),
            data: json!({ "id": message_id.to_string() }),
        });
        self
    }

    /// 消息加上表情, 具体 id 请看服务端文档, 本框架不提供
    pub fn add_face(mut self, id: i64) -> Self {
        self.0.push(Segment {
            type_: "face".to_string(),
            data: json!({ "id": id.to_string() }),
        });
        self
    }

    /// 消息加上图片
    pub fn add_image(mut self, file: &str) -> Self {
        self.0.push(Segment {
            type_: "image".to_string(),
            data: json!({ "file": file }),
        });
        self
    }

    /// 消息加上 segment
    pub fn add_segment<T>(mut self, segment: T) -> Self
    where
        Value: From<T>,
        T: Serialize,
    {
        let value = Value::from(segment);
        if let Ok(segment) = serde_json::from_value(value) {
            self.0.push(segment);
        }
        self
    }
}

#[cfg(feature = "cqstring")]
impl CQMessage {
    /// 在消息加上文字
    pub fn add_text<T>(mut self, text: T) -> Self
    where
        String: From<T>,
        T: Serialize + Display,
    {
        self.0.push_str(&format!("[CQ:text,text={}]", text));
        self
    }

    /// 消息加上at
    pub fn add_at(mut self, id: &str) -> Self {
        self.0.push_str(&format!("[CQ:at,qq={}]", id));
        self
    }

    /// 消息加上引用
    pub fn add_reply(mut self, message_id: i32) -> Self {
        self.0
            .insert_str(0, &format!("[CQ:reply,id={}]", message_id));
        self
    }

    /// 消息加上表情
    pub fn add_face(mut self, id: i64) -> Self {
        self.0.push_str(&format!("[CQ:face,id={}]", id));
        self
    }

    /// 消息加上图片
    pub fn add_image(mut self, file: &str) -> Self {
        self.0.push_str(&format!("[CQ:image,file={}]", file));
        self
    }

    /// 消息加上 segment
    pub fn add_segment<T>(mut self, segment: T) -> Self
    where
        Value: From<T>,
        T: Serialize,
    {
        let value = Value::from(segment);
        if let Ok(segment) = serde_json::from_value::<Segment>(value) {
            self.0.push_str(&super::parse_cq_code(&segment));
        }
        self
    }
}
