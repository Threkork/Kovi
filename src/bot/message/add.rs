use serde::Serialize;
use serde_json::{json, Value};
use std::fmt::Display;

use super::Message;


impl Message {
    /// 在消息加上文字
    pub fn add_text<T>(mut self, text: T) -> Self
    where
        String: From<T>,
        T: Serialize + Display,
    {
        match &mut self {
            Message::Array(ref mut array) => {
                let reply = json!({
                    "type": "text",
                    "data": {
                        "text": text
                    }
                });
                array.push(reply);
            }
            Message::CQString(ref mut string) => {
                let reply = format!("[CQ:text,text={text}]");
                string.push_str(&reply);
            }
        }

        self
    }
    /// 消息加上at
    pub fn add_at(mut self, id: &str) -> Self {
        match &mut self {
            Message::Array(ref mut array) => {
                let reply = json!({
                    "type": "at",
                    "data": {
                        "qq": id
                    }
                });
                array.push(reply);
            }
            Message::CQString(ref mut string) => {
                let reply = format!("[CQ:at,qq={id}]");
                string.push_str(&reply);
            }
        }

        self
    }
    /// 消息加上引用
    pub fn add_reply(mut self, message_id: i32) -> Self {
        match &mut self {
            Message::Array(ref mut array) => {
                let reply = json!({
                    "type": "reply",
                    "data": {
                        "id": message_id.to_string()
                    }
                });
                array.insert(0, reply);
            }
            Message::CQString(ref mut string) => {
                let reply = format!("[CQ:reply,id={message_id}]");
                string.insert_str(0, &reply);
            }
        }

        self
    }
    /// 消息加上表情, 具体 id 请看服务端文档, 本框架不提供
    pub fn add_face(mut self, id: i64) -> Self {
        match &mut self {
            Message::Array(ref mut array) => {
                let reply = json!({
                    "type": "face",
                    "data": {
                        "id": id.to_string()
                    }
                });
                array.push(reply);
            }
            Message::CQString(ref mut string) => {
                let reply = format!("[CQ:face,id={id}]");
                string.push_str(&reply);
            }
        }

        self
    }
    /// 消息加上图片
    ///
    /// 绝对路径，例如 <file:///C:\\Users\Richard\Pictures\1.png>，格式使用 file URI, 注意windows与Linux文件格式会不同，具体看OneBot服务端实现。
    ///
    /// 网络 URL，例如 <http://i1.piimg.com/567571/fdd6e7b6d93f1ef0.jpg>
    ///
    /// Base64 编码，例如 base64://iVBORw0KGgoAAAANSUhEUgAAABQAAAAVCAIAAADJt1n/AAAAKElEQVQ4EWPk5+RmIBcwkasRpG9UM4mhNxpgowFGMARGEwnBIEJVAAAdBgBNAZf+QAAAAABJRU5ErkJggg==
    pub fn add_image(mut self, file: &str) -> Self {
        match &mut self {
            Message::Array(ref mut array) => {
                let reply = json!({
                    "type": "image",
                    "data": {
                        "file": file
                    }
                });
                array.push(reply);
            }
            Message::CQString(ref mut string) => {
                let reply = format!("[CQ:image,file={file}]");
                string.push_str(&reply);
            }
        }

        self
    }
    /// 消息加上 segment
    pub fn add_segment(mut self, segment: Value) -> Self {
        match &mut self {
            Message::Array(ref mut array) => {
                array.push(segment);
            }
            Message::CQString(ref mut string) => {
                let segment = Self::parse_cq_code(&segment);
                string.push_str(&segment);
            }
        }
        self
    }
}
