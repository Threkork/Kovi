use std::ops::Add;

#[cfg(feature = "cqstring")]
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::MessageError;

pub mod add;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    #[serde(rename = "type")]
    pub type_: String,
    pub data: Value,
}

impl Segment {
    pub fn new(type_: &str, data: Value) -> Self {
        Segment {
            type_: type_.to_string(),
            data,
        }
    }
}

impl PartialEq for Segment {
    fn eq(&self, other: &Self) -> bool {
        self.type_ == other.type_ && self.data == other.data
    }
}


/// 消息
///
/// **不保证 data 里的 Value 格式是否正确，需要自行检查**
///
/// # Examples
/// ```
/// use kovi::bot::message::Message;
/// use serde_json::json;
///
/// let msg: Message = Message::from("Hi");
/// let msg: Message = Message::from_value(json!(
///     [
///         {
///             "type":"text",
///             "data":{
///                 "text":"Some msg"
///             }
///         }
///     ]
/// )).unwrap();
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message(Vec<Segment>);

impl From<Vec<Segment>> for Message {
    fn from(v: Vec<Segment>) -> Self {
        Message(v)
    }
}

impl From<Message> for Vec<Segment> {
    fn from(v: Message) -> Self {
        v.0
    }
}

impl From<&str> for Message {
    fn from(v: &str) -> Self {
        Message(vec![Segment {
            type_: "text".to_string(),
            data: json!({
                "text":v,
            }),
        }])
    }
}

impl From<String> for Message {
    fn from(v: String) -> Self {
        Message(vec![Segment {
            type_: "text".to_string(),
            data: json!({
                "text":v,
            }),
        }])
    }
}

impl From<&String> for Message {
    fn from(v: &String) -> Self {
        Message(vec![Segment {
            type_: "text".to_string(),
            data: json!({
                "text":v,
            }),
        }])
    }
}

#[cfg(feature = "cqstring")]
impl From<CQMessage> for Message {
    fn from(v: CQMessage) -> Self {
        cq_to_arr(v)
    }
}


impl PartialEq for Message {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Iterator for Message {
    type Item = Segment;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.pop()
    }
}

impl Add for Message {
    type Output = Message;

    fn add(mut self, rhs: Self) -> Self::Output {
        for seg in rhs {
            self.push(seg);
        }
        self
    }
}

impl<'a> IntoIterator for &'a Message {
    type Item = &'a Segment;
    type IntoIter = std::slice::Iter<'a, Segment>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl Message {
    pub fn iter(&self) -> std::slice::Iter<Segment> {
        self.0.iter()
    }
}

impl Message {
    pub fn from_value(v: Value) -> Result<Message, MessageError> {
        if let Some(v) = v.as_array() {
            match Message::from_vec_segment_value(v.clone()) {
                Ok(msg) => return Ok(msg),
                Err(err) => return Err(MessageError::ParseError(err.to_string())),
            };
        }
        if let Some(v) = v.as_str() {
            return Ok(Message::from(v));
        }

        Err(MessageError::ParseError(
            "Message::from_value only accept array".to_string(),
        ))
    }

    pub fn from_vec_segment_value(v: Vec<Value>) -> Result<Message, serde_json::Error> {
        let segments: Result<Vec<Segment>, serde_json::Error> = v
            .into_iter()
            .map(|value| {
                let segment: Segment = serde_json::from_value(value)?;
                Ok(segment)
            })
            .collect();

        match segments {
            Ok(segments) => Ok(Message(segments)),
            Err(err) => Err(err),
        }
    }


    /// Message 解析成人类可读字符串, 会将里面的 segment 转换成 `[type]` 字符串，如： image segment 会转换成 `[image]` 字符串。不要靠此函数做判断，可能不同版本会改变内容。
    pub fn to_human_string(&self) -> String {
        let mut result = String::new();

        for item in self.iter() {
            match item.type_.as_str() {
                "text" => {
                    if let Some(text_data) = item.data.get("text") {
                        if let Some(text_str) = text_data.as_str() {
                            result.push_str(text_str);
                        }
                    }
                }
                _ => {
                    result.push_str(&format!("[{}]", item.type_));
                }
            }
        }
        result
    }
}

impl Default for Message {
    fn default() -> Self {
        Self::new()
    }
}

impl Message {
    /// 返回空的 Message
    pub fn new() -> Message {
        Message(Vec::new())
    }

    pub fn from<T>(v: T) -> Message
    where
        Message: From<T>,
    {
        v.into()
    }

    /// 检查 Message 是否包含任意一项 segment 。返回 bool。
    ///
    /// # Examples
    /// ```
    /// use kovi::bot::message::Message;
    /// use serde_json::json;
    ///
    /// let msg1: Message = Message::from("Hi");
    /// let msg2: Message = Message::from_value(json!(
    ///     [
    ///         {
    ///             "type":"text",
    ///             "data":{
    ///                 "text":"Some msg"
    ///             }
    ///         }
    ///     ]
    /// )).unwrap();
    ///
    /// assert!(msg1.contains("text"));
    /// assert!(msg2.contains("text"));
    pub fn contains(&self, s: &str) -> bool {
        self.iter().any(|item| item.type_ == s)
    }

    /// 获取 Message 任意一种 segment 。返回 `Vec<Value>`，有多少项，就会返回多少项。
    ///
    /// # Examples
    /// ```
    /// use kovi::bot::message::Message;
    /// use serde_json::{json, Value};
    ///
    /// let msg: Message = Message::from_value(json!(
    ///     [
    ///         {
    ///             "type":"text",
    ///             "data":{
    ///                 "text":"Some msg"
    ///             }
    ///         },
    ///         {
    ///             "type":"face",
    ///             "data":{
    ///                 "id":"0"
    ///             }
    ///         },
    ///     ]
    /// )).unwrap();
    ///
    /// let text_value:Value = json!({
    ///             "type":"text",
    ///             "data":{
    ///                 "text":"Some msg"
    ///             }
    ///         });
    /// let face_value:Value = json!({
    ///             "type":"face",
    ///             "data":{
    ///                 "id":"0"
    ///             }
    ///         });
    /// assert_eq!(msg.get("text")[0], text_value);
    /// assert_eq!(msg.get("face")[0], face_value);
    pub fn get(&self, s: &str) -> Vec<Segment> {
        self.iter()
            .filter(|item| item.type_ == s)
            .cloned()
            .collect()
    }
}


#[cfg(feature = "cqstring")]
#[derive(Debug, Clone, Serialize)]
pub struct CQMessage(String);

#[cfg(feature = "cqstring")]
impl From<String> for CQMessage {
    fn from(str: String) -> Self {
        CQMessage(str)
    }
}

#[cfg(feature = "cqstring")]
impl From<&String> for CQMessage {
    fn from(str: &String) -> Self {
        CQMessage(str.clone())
    }
}

#[cfg(feature = "cqstring")]
impl From<&str> for CQMessage {
    fn from(str: &str) -> Self {
        CQMessage(str.to_string())
    }
}

#[cfg(feature = "cqstring")]
impl From<CQMessage> for String {
    fn from(cq: CQMessage) -> Self {
        cq.0
    }
}

#[cfg(feature = "cqstring")]
impl From<Message> for CQMessage {
    fn from(v: Message) -> Self {
        arr_to_cq(v)
    }
}

#[cfg(feature = "cqstring")]
pub fn cq_to_arr(message: CQMessage) -> Message {
    let cq_regex = Regex::new(r"\[CQ:([a-zA-Z]+)(,[^\]]+)?\]").unwrap();
    let mut result = Vec::new();

    let mut last_end = 0;

    for cap in cq_regex.captures_iter(&message.0) {
        let start = cap.get(0).unwrap().start();

        // 如果前面有纯文本，添加纯文本部分
        if start > last_end {
            let text_segment = &message.0[last_end..start];
            if !text_segment.is_empty() {
                result.push(json!({
                    "type": "text",
                    "data": {
                        "text": text_segment
                    }
                }));
            }
        }

        // 解析 CQ 码
        let function_name = &cap[1];
        let mut data = serde_json::Map::new();

        if let Some(params) = cap.get(2) {
            let params_str = params.as_str().trim_start_matches(',');
            for param in params_str.split(',') {
                let mut parts = param.splitn(2, '=');
                let key = parts.next().unwrap().to_string();
                let value = parts.next().unwrap_or("").to_string();
                data.insert(key, Value::String(value));
            }
        }

        result.push(json!({
            "type": function_name,
            "data": data
        }));

        last_end = cap.get(0).unwrap().end();
    }

    // 添加最后一段纯文本
    if last_end < message.0.len() {
        let text_segment = &message.0[last_end..];
        if !text_segment.is_empty() {
            result.push(json!({
                "type": "text",
                "data": {
                    "text": text_segment
                }
            }));
        }
    }

    Message::from_vec_segment_value(result).unwrap()
}

#[cfg(feature = "cqstring")]
fn parse_cq_code(item: &Segment) -> String {
    let mut result = String::new();

    match item.type_.as_str() {
        "text" => {
            if let Some(text_data) = item.data.get("text") {
                if let Some(text_str) = text_data.as_str() {
                    result.push_str(text_str);
                }
            }
        }
        _ => {
            result.push_str(&format!("[CQ:{}]", item.type_));

            let mut params = Vec::new();
            for (key, value) in item.data.as_object().unwrap().iter() {
                if let Some(value_str) = value.as_str() {
                    params.push(format!("{}={}", key, value_str));
                }
            }
            if !params.is_empty() {
                let params_str = params.join(",");
                result.push_str(&format!("[CQ:{},{}]", item.type_, params_str));
            } else {
                result.push_str(&format!("[CQ:{}]", item.type_));
            }
        }
    }
    result
}

#[cfg(feature = "cqstring")]
pub fn arr_to_cq(message: Message) -> CQMessage {
    let mut result = String::new();

    for item in message.iter() {
        result.push_str(&parse_cq_code(item));
    }

    result.into()
}

#[cfg(feature = "cqstring")]
#[test]
fn __cq_to_arr() {
    let cq = "左边的消息[CQ:face,id=178]看看我刚拍的照片[CQ:image,file=123.jpg]右边的消息";
    let msg = cq_to_arr(cq.into());
    println!("{:?}", msg)
}

#[test]
fn check_msg() {
    let msg: Message = Message::from_value(json!(
        [
            {
                "type":"text",
                "data":{
                    "text":"Some msg"
                }
            },
            {
                "type":"face",
                "data":{
                    "id":"0"
                }
            },
        ]
    ))
    .unwrap();
    let text_value: Segment = serde_json::from_value(json!({
        "type":"text",
        "data":{
            "text":"Some msg"
        }
    }))
    .unwrap();
    let face_value: Segment = serde_json::from_value(json!({
        "type":"face",
        "data":{
            "id":"0"
        }
    }))
    .unwrap();
    assert_eq!(msg.get("text")[0], text_value);
    assert_eq!(msg.get("face")[0], face_value);

    let msg1: Message = Message::from("Hi");
    let msg2: Message = Message::from_value(json!(
        [
            {
                "type":"text",
                "data":{
                    "text":"Some msg"
                }
            }
        ]
    ))
    .unwrap();
    assert!(msg1.contains("text"));
    assert!(msg2.contains("text"));
}
