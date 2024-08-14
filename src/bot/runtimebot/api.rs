use super::RuntimeBot;
use crate::{
    bot::message::Message,
    error::{ApiError, Error},
};
use log::info;
use serde::Serialize;
use serde_json::{json, Value};
use std::sync::mpsc;

pub enum HonorType {
    All,
    Talkative,
    Performer,
    Legend,
    StrongNewbie,
    Emotion,
}

/// Kovi提供解析过的返回值的api
impl RuntimeBot {
    ///发送群组消息, 并返回消息ID
    pub fn send_group_msg_return<T>(&self, group_id: i64, msg: T) -> Result<i32, ApiError>
    where
        Message: From<T>,
        T: Serialize,
    {
        let send_api = json!({
            "action": "send_msg",
            "params": {
                "message_type":"group",
                "group_id":group_id,
                "message":msg,
                "auto_escape":true,
            },
            "echo": "Some" });
        let msg = Message::from(msg);
        let group_id = &group_id;
        info!("[send] [to group {group_id}]: {}", msg.to_human_string());
        let api_rx = self.mpsc_and_send(send_api);
        match api_rx.recv().unwrap() {
            Ok(v) => Ok(v.get("message_id").unwrap().as_i64().unwrap() as i32),
            // 参数错误
            Err(_) => Err(ApiError::UnknownError()),
        }
    }

    ///发送私聊消息, 并返回消息ID
    pub fn send_private_msg_return<T>(&self, user_id: i64, msg: T) -> Result<i32, ApiError>
    where
        Message: From<T>,
        T: Serialize,
    {
        let send_api = json!({
            "action": "send_msg",
            "params": {
                "message_type":"private",
                "user_id":user_id,
                "message":msg,
                "auto_escape":true,
            },
            "echo": "Some" });
        let msg = Message::from(msg);
        let user_id = &user_id;
        info!("[send] [to private {user_id}]: {}", msg.to_human_string());
        let api_rx = self.mpsc_and_send(send_api);
        match api_rx.recv().unwrap() {
            Ok(v) => Ok(v.get("message_id").unwrap().as_i64().unwrap() as i32),
            // 参数错误
            Err(_) => Err(ApiError::UnknownError()),
        }
    }
    /// 是否能发送图片
    pub fn can_send_image(&self) -> Result<bool, ApiError> {
        let send_api = json!({
            "action": "can_send_image",
            "params": {},
            "echo": "Some" });
        let api_rx = self.mpsc_and_send(send_api);
        match api_rx.recv().unwrap() {
            Ok(v) => Ok(v.get("yes").unwrap().as_bool().unwrap()),
            // 参数错误
            Err(_) => Err(ApiError::UnknownError()),
        }
    }
    /// 是否能发生语音
    pub fn can_send_record(&self) -> Result<bool, ApiError> {
        let send_api = json!({
            "action": "can_send_record",
            "params": {},
            "echo": "Some" });
        let api_rx = self.mpsc_and_send(send_api);
        match api_rx.recv().unwrap() {
            Ok(v) => Ok(v.get("yes").unwrap().as_bool().unwrap()),
            // 参数错误
            Err(_) => Err(ApiError::UnknownError()),
        }
    }
    /// 获取 Cookies
    ///
    /// # Arguments
    ///
    /// `domain`: 需要获取 cookies 的域名
    pub fn get_cookies(&self, domain: &str) -> Result<String, ApiError> {
        let send_api = json!({
            "action": "get_cookies",
            "params": {
                "domain":domain,
            },
            "echo": "Some" });
        let api_rx = self.mpsc_and_send(send_api);
        match api_rx.recv().unwrap() {
            Ok(v) => Ok(v.get("cookies").unwrap().to_string()),
            // 参数错误
            Err(_) => Err(ApiError::ParamsError(format!(
                "Check the incoming parameter: domain: '{}'",
                domain
            ))),
        }
    }
    /// 获取 CSRF Token
    pub fn get_csrf_token(&self) -> Result<i32, ApiError> {
        let send_api = json!({
            "action": "get_csrf_token",
            "params": {},
            "echo": "Some" });
        let api_rx = self.mpsc_and_send(send_api);
        match api_rx.recv().unwrap() {
            Ok(v) => Ok(v.get("token").unwrap().as_i64().unwrap() as i32),
            // 参数错误
            Err(_) => Err(ApiError::UnknownError()),
        }
    }
    /// 获取语音
    ///
    /// # Arguments
    ///
    /// `file`: 收到的语音文件名（消息段的 file 参数），如 `0B38145AA44505000B38145AA4450500.silk`
    ///
    /// `out_format`: 要转换到的格式，目前支持 `mp3`、`amr`、`wma`、`m4a`、`spx`、`ogg`、`wav`、`flac`
    pub fn get_record(&self, file: &str, out_format: &str) -> Result<String, ApiError> {
        let send_api = json!({
            "action": "get_record",
            "params": {
                "file":file,
                "out_format":out_format
            },
            "echo": "Some" });
        let api_rx = self.mpsc_and_send(send_api);
        match api_rx.recv().unwrap() {
            Ok(v) => match v.get("file") {
                Some(v) => Ok(v.to_string()),
                None => Err(ApiError::ParamsError(format!(
                    "Check the incoming parameter: file: '{}', out_format: '{}'",
                    file, out_format
                ))),
            },
            // 参数错误
            Err(_) => Err(ApiError::ParamsError(format!(
                "Check the incoming parameter: file: '{}', out_format: '{}'",
                file, out_format
            ))),
        }
    }
    /// 获取图片
    ///
    /// # Arguments
    ///
    /// `file`: 收到的图片文件名（消息段的 file 参数），如 `6B4DE3DFD1BD271E3297859D41C530F5.jpg`
    pub fn get_image(&self, file: &str) -> Result<String, ApiError> {
        let send_api = json!({
            "action": "get_image",
            "params": {
                "file":file,
            },
            "echo": "Some" });
        let api_rx = self.mpsc_and_send(send_api);
        match api_rx.recv().unwrap() {
            Ok(v) => match v.get("file") {
                Some(v) => Ok(v.to_string()),
                None => Err(ApiError::ParamsError(format!(
                    "Check the incoming parameter: file: '{}'",
                    file
                ))),
            },
            // 参数错误
            Err(_) => Err(ApiError::ParamsError(format!(
                "Check the incoming parameter: file: '{}'",
                file
            ))),
        }
    }
}

// 这些都是无需处理返回值的api
impl RuntimeBot {
    ///发送群组消息，如果需要返回消息id，请使用send_group_msg_return()
    pub fn send_group_msg<T>(&self, group_id: i64, msg: T)
    where
        Message: From<T>,
        T: Serialize,
    {
        let send_api = json!({
            "action": "send_msg",
            "params": {
                "message_type":"group",
                "group_id":group_id,
                "message":msg,
                "auto_escape":true,
            },
            "echo": "None" });
        let msg = Message::from(msg);
        let group_id = &group_id;
        info!("[send] [to group {group_id}]: {}", msg.to_human_string());
        self.api_tx.send((send_api, None)).unwrap();
    }

    ///发送私聊消息，如果需要返回消息id，请使用send_private_msg_return()
    pub fn send_private_msg<T>(&self, user_id: i64, msg: T)
    where
        Message: From<T>,
        T: Serialize,
    {
        let send_api = json!({
            "action": "send_msg",
            "params": {
                "message_type":"private",
                "user_id":user_id,
                "message":msg,
                "auto_escape":true,
            },
            "echo": "None" });
        let msg = Message::from(msg);
        let user_id = &user_id;
        info!("[send] [to private {user_id}]: {}", msg.to_human_string());
        self.api_tx.send((send_api, None)).unwrap();
    }

    /// 撤回消息
    ///
    /// # Arguments
    ///
    /// `message_id`: 消息 ID
    pub fn delete_msg(&self, message_id: i32) {
        let send_api = json!({
            "action": "delete_msg",
            "params": {
                "message_id":message_id,
            },
            "echo": "None" });

        self.api_tx.send((send_api, None)).unwrap();
    }

    /// 点赞
    /// # Arguments
    ///
    /// `user_id`
    ///
    /// `times`: 次数
    pub fn send_like(&self, user_id: i64, times: usize) {
        let send_api = json!({
            "action": "send_like",
            "params": {
                "user_id":user_id,
                "times":times,
            },
            "echo": "None" });

        self.api_tx.send((send_api, None)).unwrap();
    }

    /// 群组踢人
    /// # Arguments
    ///
    /// `group_id`
    ///
    /// `user_id`
    ///
    /// `reject_add_request`: 是否拒绝此人的加群请求，传入true则拒绝
    pub fn set_group_kick(&self, group_id: i64, user_id: i64, reject_add_request: bool) {
        let send_api = json!({
            "action": "set_group_kick",
            "params": {
                "group_id":group_id,
                "user_id":user_id,
                "reject_add_request":reject_add_request,
            },
            "echo": "None" });

        self.api_tx.send((send_api, None)).unwrap();
    }

    /// 群组单人禁言
    ///
    /// # Arguments
    ///
    /// `group_id`
    ///
    /// `user_id`
    ///
    /// `duration`: 禁言时长，单位秒，0 表示取消禁言
    pub fn set_group_ban(&self, group_id: i64, user_id: i64, duration: usize) {
        let send_api = json!({
            "action": "set_group_ban",
            "params": {
                "group_id":group_id,
                "user_id":user_id,
                "duration":duration,
            },
            "echo": "None" });

        self.api_tx.send((send_api, None)).unwrap();
    }
    /// 群组匿名用户禁言
    ///
    /// # Arguments
    ///
    /// `group_id`
    ///
    /// `anonymous`: 要禁言的匿名用户对象（群消息上报的 anonymous 字段）
    ///
    /// `enable`: 是否禁言
    pub fn set_group_anonymous_ban_use_anonymous(
        &self,
        group_id: i64,
        anonymous: Value,
        duration: usize,
    ) {
        let send_api = json!({
            "action": "set_group_anonymous_ban",
            "params": {
                "group_id":group_id,
                "anonymous":anonymous,
                "duration":duration,
            },
            "echo": "None" });

        self.api_tx.send((send_api, None)).unwrap();
    }
    /// 群组匿名用户禁言
    ///
    /// # Arguments
    ///
    /// `group_id`
    ///
    /// `flag`: 要禁言的匿名用户的 flag（需从群消息上报的数据中获得）
    ///
    /// `enable`: 是否禁言
    pub fn set_group_anonymous_ban_use_flag(&self, group_id: i64, flag: &str, duration: usize) {
        let send_api = json!({
            "action": "set_group_anonymous_ban",
            "params": {
                "group_id":group_id,
                "flag":flag,
                "duration":duration,
            },
            "echo": "None" });

        self.api_tx.send((send_api, None)).unwrap();
    }

    /// 群组全员禁言
    ///
    /// # Arguments
    ///
    /// `group_id`
    ///
    /// `enable`: 是否禁言
    pub fn set_group_whole_ban(&self, group_id: i64, enable: bool) {
        let send_api = json!({
            "action": "set_group_whole_ban",
            "params": {
                "group_id":group_id,
                "enable":enable,
            },
            "echo": "None" });

        self.api_tx.send((send_api, None)).unwrap();
    }

    /// 群组设置管理员
    ///
    /// # Arguments
    ///
    /// `group_id`
    ///
    /// `user_id`
    ///
    /// `enable`: true 为设置，false 为取消
    pub fn set_group_admin(&self, group_id: i64, user_id: i64, enable: bool) {
        let send_api = json!({
            "action": "set_group_admin",
            "params": {
                "group_id":group_id,
                "user_id":user_id,
                "enable":enable,
            },
            "echo": "None" });

        self.api_tx.send((send_api, None)).unwrap();
    }
    /// 群组匿名
    ///
    /// # Arguments
    ///
    /// `group_id`
    ///
    /// `enable`: true 为设置，false 为取消
    pub fn set_group_anonymous(&self, group_id: i64, enable: bool) {
        let send_api = json!({
            "action": "set_group_anonymous",
            "params": {
                "group_id":group_id,
                "enable":enable,
            },
            "echo": "None" });

        self.api_tx.send((send_api, None)).unwrap();
    }

    /// 设置群名片（群备注）
    ///
    /// # Arguments
    ///
    /// `group_id`
    ///
    /// `user_id`
    ///
    /// `card`: 群名片内容，不填或空字符串表示删除群名片
    pub fn set_group_card(&self, group_id: i64, user_id: i64, card: &str) {
        let send_api = json!({
            "action": "set_group_card",
            "params": {
                "group_id":group_id,
                "user_id":user_id,
                "card":card,
            },
            "echo": "None" });

        self.api_tx.send((send_api, None)).unwrap();
    }

    /// 设置群名
    ///
    /// # Arguments
    ///
    /// `group_id`
    ///
    /// `group_name`: 新群名
    pub fn set_group_name(&self, group_id: i64, group_name: &str) {
        let send_api = json!({
            "action": "set_group_name",
            "params": {
                "group_id":group_id,
                "group_name":group_name,
            },
            "echo": "None" });

        self.api_tx.send((send_api, None)).unwrap();
    }

    /// 退出群组
    ///
    /// # Arguments
    ///
    /// `group_id`
    ///
    /// `is_dismiss`: 是否解散，如果登录号是群主，则仅在此项为 true 时能够解散
    pub fn set_group_leave(&self, group_id: i64, is_dismiss: bool) {
        let send_api = json!({
            "action": "set_group_leave",
            "params": {
                "group_id":group_id,
                "is_dismiss":is_dismiss,
            },
            "echo": "None" });

        self.api_tx.send((send_api, None)).unwrap();
    }

    /// 设置群组专属头衔
    ///
    /// # Arguments
    ///
    /// `group_id`
    ///
    /// `user_id`
    ///
    /// `special_title`: 专属头衔，空字符串表示删除专属头衔
    pub fn set_group_special_title(&self, group_id: i64, user_id: i64, special_title: &str) {
        let send_api = json!({
            "action": "set_group_special_title",
            "params": {
                "group_id":group_id,
                "user_id":user_id,
                "special_title":special_title,
            },
            "echo": "None" });

        self.api_tx.send((send_api, None)).unwrap();
    }
    /// 处理加好友请求
    ///
    /// # Arguments
    ///
    /// `flag`: 加好友请求的 flag（需从上报的数据中获得）
    ///
    /// `approve`: 是否同意请求
    ///
    /// `remark`: 添加后的好友备注（仅在同意时有效）
    pub fn set_friend_add_request(&self, flag: &str, approve: bool, remark: &str) {
        let send_api = json!({
            "action": "set_friend_add_request",
            "params": {
                "flag":flag,
                "approve":approve,
                "remark":remark,
            },
            "echo": "None" });

        self.api_tx.send((send_api, None)).unwrap();
    }
    /// 处理加群请求／邀请
    ///
    /// # Arguments
    ///
    /// `flag`: 加群请求的 flag（需从上报的数据中获得）
    ///
    /// `sub_type`: add 或 invite，请求类型（需要和上报消息中的 sub_type 字段相符）
    ///
    /// `approve`: 是否同意请求／邀请
    ///
    /// `remark`: 可为空, 拒绝理由（仅在拒绝时有效）
    pub fn set_group_add_request(&self, flag: &str, sub_type: &str, approve: bool, reason: &str) {
        let send_api = json!({
            "action": "set_friend_add_request",
            "params": {
                "flag":flag,
                "sub_type":sub_type,
                "approve":approve,
                "reason":reason,
            },
            "echo": "None" });

        self.api_tx.send((send_api, None)).unwrap();
    }

    /// 清理缓存
    ///
    /// 用于清理积攒了太多的**OneBot服务端**缓存文件。**并非是对于本框架清除**。
    pub fn clean_cache(&self) {
        let send_api = json!({
            "action": "clean_cache",
            "params": {},
            "echo": "None" });

        self.api_tx.send((send_api, None)).unwrap();
    }
}

// 这些是需要处理返回值的api
impl RuntimeBot {
    /// 获取消息
    /// # Arguments
    ///
    /// `message_id`: 消息ID
    pub fn get_msg(&self, message_id: i32) -> Result<Value, ApiError> {
        let send_api = json!({
            "action": "get_msg",
            "params": {
                "message_id":message_id
            },
            "echo": "Some" });
        let api_rx = self.mpsc_and_send(send_api);
        match api_rx.recv().unwrap() {
            Ok(v) => Ok(v),
            // 参数错误
            Err(_) => Err(ApiError::ParamsError(format!(
                "Check the incoming parameter: message_id: '{}'",
                message_id
            ))),
        }
    }
    /// 获取合并转发消息
    /// # Arguments
    ///
    /// `id`: 合并转发 ID
    pub fn get_forward_msg(&self, id: &str) -> Result<Value, ApiError> {
        let send_api = json!({
            "action": "get_forward_msg",
            "params": {
                "id":id
            },
            "echo": "Some" });
        let api_rx = self.mpsc_and_send(send_api);
        match api_rx.recv().unwrap() {
            Ok(v) => Ok(v),
            // 参数错误
            Err(_) => Err(ApiError::ParamsError(format!(
                "Check the incoming parameter: id: '{}'",
                id
            ))),
        }
    }
    /// 获取获取登录号信息
    pub fn get_login_info(&self) -> Result<Value, ApiError> {
        let send_api = json!({
            "action": "get_login_info",
            "params": {},
            "echo": "Some" });
        let api_rx = self.mpsc_and_send(send_api);
        match api_rx.recv().unwrap() {
            Ok(v) => Ok(v),
            // 参数错误
            Err(_) => Err(ApiError::UnknownError()),
        }
    }
    /// 获取获取陌生人信息
    /// # Arguments
    ///
    /// `user_id`
    ///
    /// `no_cache`: 是否不使用缓存（使用缓存可能更新不及时，但响应更快）
    pub fn get_stranger_info(&self, user_id: i64, no_cache: bool) -> Result<Value, ApiError> {
        let send_api = json!({
            "action": "get_stranger_info",
            "params": {
                "user_id":user_id,
                "no_cache":no_cache
            },
            "echo": "Some" });
        let api_rx = self.mpsc_and_send(send_api);
        match api_rx.recv().unwrap() {
            Ok(v) => Ok(v),
            // 参数错误
            Err(_) => Err(ApiError::ParamsError(format!(
                "Check the incoming parameter: user_id: '{}', no_cache: '{}'",
                user_id, no_cache
            ))),
        }
    }
    /// 获取好友列表
    pub fn get_friend_list(&self) -> Result<Value, ApiError> {
        let send_api = json!({
            "action": "get_friend_list",
            "params": {},
            "echo": "Some" });
        let api_rx = self.mpsc_and_send(send_api);
        match api_rx.recv().unwrap() {
            Ok(v) => Ok(v),
            // 参数错误
            Err(_) => Err(ApiError::UnknownError()),
        }
    }
    /// 获取群信息
    /// # Arguments
    ///
    /// `group_id`
    ///
    /// `no_cache`: 是否不使用缓存（使用缓存可能更新不及时，但响应更快）
    pub fn get_group_info(&self, group_id: i64, no_cache: bool) -> Result<Value, ApiError> {
        let send_api = json!({
            "action": "get_group_info",
            "params": {
                "group_id":group_id,
                "no_cache":no_cache
            },
            "echo": "Some" });
        let api_rx = self.mpsc_and_send(send_api);
        match api_rx.recv().unwrap() {
            Ok(v) => Ok(v),
            // 参数错误
            Err(_) => Err(ApiError::ParamsError(format!(
                "Check the incoming parameter: group_id: '{}', no_cache: '{}'",
                group_id, no_cache
            ))),
        }
    }
    /// 获取群列表
    pub fn get_group_list(&self) -> Result<Value, ApiError> {
        let send_api = json!({
            "action": "get_group_list",
            "params": {},
            "echo": "Some" });
        let api_rx = self.mpsc_and_send(send_api);
        match api_rx.recv().unwrap() {
            Ok(v) => Ok(v),
            // 参数错误
            Err(_) => Err(ApiError::UnknownError()),
        }
    }
    ///获取群成员信息
    /// # Arguments
    ///
    /// `group_id`
    ///
    /// `user_id`
    ///
    /// `no_cache`: 是否不使用缓存（使用缓存可能更新不及时，但响应更快）
    pub fn get_group_member_info(
        &self,
        group_id: i64,
        user_id: i64,
        no_cache: bool,
    ) -> Result<Value, ApiError> {
        let send_api = json!({
            "action": "get_group_member_info",
            "params": {
                "group_id":group_id,
                "user_id":user_id,
                "no_cache":no_cache
            },
            "echo": "Some" });
        let api_rx = self.mpsc_and_send(send_api);
        match api_rx.recv().unwrap() {
            Ok(v) => Ok(v),
            // 参数错误
            Err(_) => Err(ApiError::ParamsError(format!(
                "Check the incoming parameter: group_id: '{}',user_id: '{}', no_cache: '{}'",
                group_id, user_id, no_cache
            ))),
        }
    }
    /// 获取群成员列表
    ///
    /// # Arguments
    ///
    /// `group_id`
    pub fn get_group_member_list(&self, group_id: i64) -> Result<Value, ApiError> {
        let send_api = json!({
            "action": "get_group_member_list",
            "params": {
                "group_id":group_id,
            },
            "echo": "Some" });
        let api_rx = self.mpsc_and_send(send_api);
        match api_rx.recv().unwrap() {
            Ok(v) => Ok(v),
            // 参数错误
            Err(_) => Err(ApiError::ParamsError(format!(
                "Check the incoming parameter: group_id: '{}'",
                group_id
            ))),
        }
    }

    /// 获取群荣誉信息
    /// # Arguments
    ///
    /// `group_id`
    ///
    /// `honor_type`: 要获取的群荣誉类型，可传入 talkative performer legend strong_newbie emotion 以分别获取单个类型的群荣誉数据，或传入 all 获取所有数据。**本框架已包装好了HonorType枚举**
    pub fn get_group_honor_info(
        &self,
        group_id: i64,
        honor_type: HonorType,
    ) -> Result<Value, ApiError> {
        let honor_type = match honor_type {
            HonorType::All => "all",
            HonorType::Talkative => "talkative",
            HonorType::Performer => "performer",
            HonorType::Legend => "legend",
            HonorType::StrongNewbie => "strong_newbie",
            HonorType::Emotion => "emotion",
        };

        let send_api = json!({
            "action": "get_group_honor_info",
            "params": {
                "group_id":group_id,
                "type":honor_type
            },
            "echo": "Some" });
        let api_rx = self.mpsc_and_send(send_api);
        match api_rx.recv().unwrap() {
            Ok(v) => Ok(v),
            // 参数错误
            Err(_) => Err(ApiError::ParamsError(format!(
                "Check the incoming parameter: group_id: '{}', honor_type: '{}'",
                group_id, honor_type
            ))),
        }
    }

    /// 获取相关接口凭证, 即 Cookies 和 CSRF Token 的合并。
    ///
    /// # Arguments
    ///
    /// `domain`: 需要获取 cookies 的域名
    pub fn get_credentials(&self, domain: &str) -> Result<Value, ApiError> {
        let send_api = json!({
            "action": "get_credentials",
            "params": {
                "domain":domain,
            },
            "echo": "Some" });
        let api_rx = self.mpsc_and_send(send_api);
        match api_rx.recv().unwrap() {
            Ok(v) => Ok(v),
            // 参数错误
            Err(_) => Err(ApiError::ParamsError(format!(
                "Check the incoming parameter: domain: '{}'",
                domain
            ))),
        }
    }

    /// 获取运行状态
    pub fn get_status(&self) -> Result<Value, ApiError> {
        let send_api = json!({
            "action": "get_status",
            "params": {},
            "echo": "Some" });
        let api_rx = self.mpsc_and_send(send_api);
        match api_rx.recv().unwrap() {
            Ok(v) => Ok(v),
            // 参数错误
            Err(_) => Err(ApiError::UnknownError()),
        }
    }
    /// 获取版本信息
    pub fn get_version_info(&self) -> Result<Value, ApiError> {
        let send_api = json!({
            "action": "get_version_info",
            "params": {},
            "echo": "Some" });
        let api_rx = self.mpsc_and_send(send_api);
        match api_rx.recv().unwrap() {
            Ok(v) => Ok(v),
            // 参数错误
            Err(_) => Err(ApiError::UnknownError()),
        }
    }
}

impl RuntimeBot {
    fn mpsc_and_send(&self, send_api: Value) -> mpsc::Receiver<Result<Value, Error>> {
        #[allow(clippy::type_complexity)]
        let (api_tx, api_rx): (
            mpsc::Sender<Result<Value, Error>>,
            mpsc::Receiver<Result<Value, Error>>,
        ) = mpsc::channel();
        self.api_tx.send((send_api, Some(api_tx))).unwrap();
        api_rx
    }
}
