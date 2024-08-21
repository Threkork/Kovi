use regex::Regex;
use serde::{ser::Serializer, Serialize};
use serde_json::{json, Value};


pub mod add;

/// 消息枚举，含有两种消息， CQString 和 Array 。
///
/// 两者可互相转换。
///
/// **Array 不保证 Value 格式是否正确，需要自行检查**
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
#[derive(Debug, Clone)]
pub enum Message {
    CQString(String),
    Array(Vec<Value>),
}

impl Serialize for Message {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Message::CQString(s) => serializer.serialize_str(s),
            Message::Array(arr) => arr.serialize(serializer),
        }
    }
}

impl From<String> for Message {
    fn from(str: String) -> Self {
        Message::CQString(str)
    }
}
impl From<&String> for Message {
    fn from(str: &String) -> Self {
        Message::CQString(str.clone())
    }
}
impl From<&str> for Message {
    fn from(str: &str) -> Self {
        Message::CQString(str.to_string())
    }
}
impl From<Vec<Value>> for Message {
    fn from(v: Vec<Value>) -> Self {
        Message::Array(v)
    }
}


impl Message {
    /// Message::CQString 转换成 Array ，如果本来就是 CQString 则不变。
    pub fn into_array(self) -> Message {
        match self {
            Message::Array(_) => self,
            Message::CQString(v) => Self::cq_to_arr(Message::CQString(v)),
        }
    }

    /// Message::Array 转换成 CQString ，如果本来就是 Array 则不变。
    pub fn into_cqstring(self) -> Message {
        match self {
            Message::CQString(_) => self,
            Message::Array(v) => Self::arr_to_cq(Message::Array(v)),
        }
    }

    fn cq_to_arr(message: Message) -> Message {
        let message = match message {
            Message::Array(_) => {
                return message;
            }
            Message::CQString(v) => v,
        };
        let cq_regex = Regex::new(r"\[CQ:([a-zA-Z]+)(,[^\]]+)?\]").unwrap();
        let mut result = Vec::new();

        let mut last_end = 0;

        for cap in cq_regex.captures_iter(&message) {
            let start = cap.get(0).unwrap().start();

            // 如果前面有纯文本，添加纯文本部分
            if start > last_end {
                let text_segment = &message[last_end..start];
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
        if last_end < message.len() {
            let text_segment = &message[last_end..];
            if !text_segment.is_empty() {
                result.push(json!({
                    "type": "text",
                    "data": {
                        "text": text_segment
                    }
                }));
            }
        }

        Message::Array(result)
    }

    fn parse_cq_code(item: &Value) -> String {
        let mut result = String::new();
        if let Some(type_value) = item.get("type") {
            if let Some(type_str) = type_value.as_str() {
                match type_str {
                    "text" => {
                        if let Some(text_data) = item.get("data").and_then(|d| d.get("text")) {
                            if let Some(text_str) = text_data.as_str() {
                                result.push_str(text_str);
                            }
                        }
                    }
                    _ => {
                        result.push_str(&format!("[CQ:{}]", type_str));
                        if let Some(data) = item.get("data") {
                            let mut params = Vec::new();
                            for (key, value) in data.as_object().unwrap().iter() {
                                if let Some(value_str) = value.as_str() {
                                    params.push(format!("{}={}", key, value_str));
                                }
                            }
                            if !params.is_empty() {
                                let params_str = params.join(",");
                                result.push_str(&format!("[CQ:{},{}]", type_str, params_str));
                            } else {
                                result.push_str(&format!("[CQ:{}]", type_str));
                            }
                        }
                    }
                }
            }
        }
        result
    }
    fn arr_to_cq(message: Message) -> Message {
        let array = match message {
            Message::CQString(_) => return message,
            Message::Array(array) => array,
        };
        let mut result = String::new();

        for item in array {
            result.push_str(&Self::parse_cq_code(&item));
        }

        Message::CQString(result)
    }

    /// Message 解析成人类可读字符串, 会将里面的 segment 转换成 `[type]` 字符串，如： image segment 会转换成 `[image]` 字符串
    pub fn to_human_string(&self) -> String {
        match self {
            Message::Array(array) => {
                let mut result = String::new();

                for item in array {
                    if let Some(type_value) = item.get("type") {
                        if let Some(type_str) = type_value.as_str() {
                            match type_str {
                                "text" => {
                                    if let Some(text_data) =
                                        item.get("data").and_then(|d| d.get("text"))
                                    {
                                        if let Some(text_str) = text_data.as_str() {
                                            result.push_str(text_str);
                                        }
                                    }
                                }
                                _ => {
                                    result.push_str(&format!("[{}]", type_str));
                                }
                            }
                        }
                    }
                }

                result
            }
            Message::CQString(_) => {
                let msg = Self::cq_to_arr(self.clone());
                msg.to_human_string()
            }
        }
    }
}


impl Message {
    /// 返回空的 Message::CQString ，里面是空字符串
    pub fn new_string() -> Message {
        Message::CQString("".to_string())
    }

    /// 返回空的 Message::Array ，里面是空 Vec
    pub fn new_array() -> Message {
        Message::Array(Vec::new())
    }

    /// 根据传入什么返回对应的类型，字符串返回CQString。`Vec<Value>` 会返回 Array ，**Array 不保证 Value 格式是否正确，需要自行检查**
    pub fn from<T>(v: T) -> Message
    where
        Message: From<T>,
    {
        v.into()
    }

    /// 根据传入什么返回对应的类型， Value 的 String 返回 CQString。Value的 `Array` 会返回 Array。
    ///
    /// **Array 不保证 Value 格式是否正确，需要自行检查**
    pub fn from_value(v: Value) -> Option<Message> {
        match v {
            Value::String(s) => Some(Message::CQString(s)),
            Value::Array(arr) => Some(Message::Array(arr)),

            _ => None,
        }
    }

    /// 传入字符串，返回 CQString
    pub fn from_string(s: String) -> Message {
        Message::CQString(s)
    }

    /// 传入字符串，返回 CQString
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Message {
        Message::CQString(s.to_string())
    }

    /// 会返回 Array ，但是不保证 Value 格式是否正确，需要自行检查
    pub fn from_array(arr: Vec<Value>) -> Message {
        Message::Array(arr)
    }

    /// 检查是否是 Message::CQSting ,返回 bool
    pub fn is_cqstring(&self) -> bool {
        match self {
            Message::CQString(_) => true,
            Message::Array(_) => false,
        }
    }
    /// 检查是否是 Message::Array ,返回 bool
    pub fn is_array(&self) -> bool {
        match self {
            Message::Array(_) => true,
            Message::CQString(_) => false,
        }
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
        match self {
            Message::Array(array) => array.iter().any(|item| {
                item.get("type")
                    .and_then(Value::as_str)
                    .map_or(false, |type_str| type_str == s)
            }),
            Message::CQString(_) => {
                let msg = Self::cq_to_arr(self.clone());
                msg.contains(s)
            }
        }
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
    pub fn get(&self, s: &str) -> Vec<Value> {
        match self {
            Message::Array(array) => {
                array
                    .iter()
                    .filter_map(|item| {
                        item.get("type")
                            .and_then(Value::as_str)
                            .filter(|&type_str| type_str == s)
                            .map(|_| item.clone()) // 如果匹配，则克隆当前项
                    })
                    .collect()
            }
            Message::CQString(_) => {
                let msg = Self::cq_to_arr(self.clone());
                msg.get(s)
            }
        }
    }
}

#[test]
fn __cq_to_arr() {
    let cq = "左边的消息[CQ:face,id=178]看看我刚拍的照片[CQ:image,file=123.jpg]右边的消息";
    let msg = Message::cq_to_arr(cq.into());
    println!("{:?}", msg)
}
