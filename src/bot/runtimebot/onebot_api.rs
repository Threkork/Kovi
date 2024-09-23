use super::RuntimeBot;
use crate::bot::{message::Message, SendApi};
use log::{error, info};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::oneshot;
use tokio_tungstenite::tungstenite::handshake::client::generate_key;

pub enum HonorType {
    All,
    Talkative,
    Performer,
    Legend,
    StrongNewbie,
    Emotion,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiReturn {
    pub status: String,
    pub retcode: i32,
    pub data: Value,
    pub echo: String,
}

impl std::fmt::Display for ApiReturn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "status: {}, retcode: {}, data: {}, echo: {}",
            self.status, self.retcode, self.data, self.echo
        )
    }
}


/// Kovi提供解析过的返回值的api
impl RuntimeBot {
    ///发送群组消息, 并返回消息ID
    pub async fn send_group_msg_return<T>(&self, group_id: i64, msg: T) -> Result<i32, ApiReturn>
    where
        Message: From<T>,
        T: Serialize,
    {
        let send_api = SendApi::new(
            "send_msg",
            json!({
                "message_type":"group",
                "group_id":group_id,
                "message":msg,
                "auto_escape":true,
            }),
            &generate_key(),
        );

        let msg = Message::from(msg);
        let group_id = &group_id;
        info!("[send] [to group {group_id}]: {}", msg.to_human_string());
        let r = self.send_and_return(send_api).await;
        match r {
            Ok(v) => Ok(v.data.get("message_id").unwrap().as_i64().unwrap() as i32),

            Err(v) => Err(v),
        }
    }

    ///发送私聊消息, 并返回消息ID
    pub async fn send_private_msg_return<T>(&self, user_id: i64, msg: T) -> Result<i32, ApiReturn>
    where
        Message: From<T>,
        T: Serialize,
    {
        let send_api = SendApi::new(
            "send_msg",
            json!({"message_type":"private",
                "user_id":user_id,
                "message":msg,
                "auto_escape":true,}),
            &generate_key(),
        );

        let msg = Message::from(msg);
        let user_id = &user_id;
        info!("[send] [to private {user_id}]: {}", msg.to_human_string());
        let r = self.send_and_return(send_api).await;
        match r {
            Ok(v) => Ok(v.data.get("message_id").unwrap().as_i64().unwrap() as i32),

            Err(v) => Err(v),
        }
    }
    /// 是否能发送图片
    pub async fn can_send_image(&self) -> Result<bool, ApiReturn> {
        let send_api = SendApi::new("can_send_image", json!({}), &generate_key());

        let r = self.send_and_return(send_api).await;
        match r {
            Ok(v) => Ok(v.data.get("yes").unwrap().as_bool().unwrap()),

            Err(v) => Err(v),
        }
    }
    /// 是否能发送语音
    pub async fn can_send_record(&self) -> Result<bool, ApiReturn> {
        let send_api = SendApi::new("can_send_record", json!({}), &generate_key());
        let r = self.send_and_return(send_api).await;
        match r {
            Ok(v) => Ok(v.data.get("yes").unwrap().as_bool().unwrap()),

            Err(v) => Err(v),
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
        let send_api = SendApi::new(
            "send_msg",
            json!({
                    "message_type":"group",
                    "group_id":group_id,
                    "message":msg,
                    "auto_escape":true,
            }),
            "None",
        );
        let msg = Message::from(msg);
        let group_id = &group_id;
        info!("[send] [to group {group_id}]: {}", msg.to_human_string());
        let api_tx = self.api_tx.clone();
        tokio::spawn(async move {
            api_tx.send((send_api, None)).await.unwrap();
        });
    }

    ///发送私聊消息，如果需要返回消息id，请使用send_private_msg_return()
    pub fn send_private_msg<T>(&self, user_id: i64, msg: T)
    where
        Message: From<T>,
        T: Serialize,
    {
        let send_api = SendApi::new(
            "send_msg",
            json!({
                "message_type":"private",
                    "user_id":user_id,
                    "message":msg,
                    "auto_escape":true,
            }),
            "None",
        );

        let msg = Message::from(msg);
        let user_id = &user_id;
        info!("[send] [to private {user_id}]: {}", msg.to_human_string());
        let api_tx = self.api_tx.clone();
        tokio::spawn(async move {
            api_tx.send((send_api, None)).await.unwrap();
        });
    }

    /// 撤回消息
    ///
    /// # Arguments
    ///
    /// `message_id`: 消息 ID
    pub fn delete_msg(&self, message_id: i32) {
        let send_api = SendApi::new(
            "delete_msg",
            json!({
                "message_id":message_id,
            }),
            "None",
        );

        let api_tx = self.api_tx.clone();
        tokio::spawn(async move {
            api_tx.send((send_api, None)).await.unwrap();
        });
    }

    /// 点赞，有些服务端会返回点赞失败，所以需要返回值的话请使用 send_like_return()
    /// # Arguments
    ///
    /// `user_id`
    ///
    /// `times`: 次数
    pub fn send_like(&self, user_id: i64, times: usize) {
        let send_api = SendApi::new(
            "send_like",
            json!({
                                "user_id":user_id,
                    "times":times,
            }),
            "None",
        );

        let api_tx = self.api_tx.clone();
        tokio::spawn(async move {
            api_tx.send((send_api, None)).await.unwrap();
        });
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
        let send_api = SendApi::new(
            "set_group_kick",
            json!({
                "group_id":group_id,
                    "user_id":user_id,
                    "reject_add_request":reject_add_request,
            }),
            "None",
        );

        let api_tx = self.api_tx.clone();
        tokio::spawn(async move {
            api_tx.send((send_api, None)).await.unwrap();
        });
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
        let send_api = SendApi::new(
            "set_group_ban",
            json!({
                "group_id":group_id,
                    "user_id":user_id,
                    "duration":duration,
            }),
            "None",
        );

        let api_tx = self.api_tx.clone();
        tokio::spawn(async move {
            api_tx.send((send_api, None)).await.unwrap();
        });
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
        let send_api = SendApi::new(
            "set_group_anonymous_ban",
            json!({
                "group_id":group_id,
                    "anonymous":anonymous,
                    "duration":duration,
            }),
            "None",
        );


        let api_tx = self.api_tx.clone();
        tokio::spawn(async move {
            api_tx.send((send_api, None)).await.unwrap();
        });
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
        let send_api = SendApi::new(
            "set_group_anonymous_ban",
            json!({
                "group_id":group_id,
                    "flag":flag,
                    "duration":duration,
            }),
            "None",
        );


        let api_tx = self.api_tx.clone();
        tokio::spawn(async move {
            api_tx.send((send_api, None)).await.unwrap();
        });
    }

    /// 群组全员禁言
    ///
    /// # Arguments
    ///
    /// `group_id`
    ///
    /// `enable`: 是否禁言
    pub fn set_group_whole_ban(&self, group_id: i64, enable: bool) {
        let send_api = SendApi::new(
            "set_group_whole_ban",
            json!({
                "group_id":group_id,
                    "enable":enable,
            }),
            "None",
        );


        let api_tx = self.api_tx.clone();
        tokio::spawn(async move {
            api_tx.send((send_api, None)).await.unwrap();
        });
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
        let send_api = SendApi::new(
            "set_group_admin",
            json!({
                "group_id":group_id,
                    "user_id":user_id,
                    "enable":enable,
            }),
            "None",
        );

        let api_tx = self.api_tx.clone();
        tokio::spawn(async move {
            api_tx.send((send_api, None)).await.unwrap();
        });
    }
    /// 群组匿名
    ///
    /// # Arguments
    ///
    /// `group_id`
    ///
    /// `enable`: true 为设置，false 为取消
    pub fn set_group_anonymous(&self, group_id: i64, enable: bool) {
        let send_api = SendApi::new(
            "set_group_anonymous",
            json!({
                "group_id":group_id,
                    "enable":enable,
            }),
            "None",
        );


        let api_tx = self.api_tx.clone();
        tokio::spawn(async move {
            api_tx.send((send_api, None)).await.unwrap();
        });
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
        let send_api = SendApi::new(
            "set_group_card",
            json!({
                "group_id":group_id,
                    "user_id":user_id,
                    "card":card,
            }),
            "None",
        );


        let api_tx = self.api_tx.clone();
        tokio::spawn(async move {
            api_tx.send((send_api, None)).await.unwrap();
        });
    }

    /// 设置群名
    ///
    /// # Arguments
    ///
    /// `group_id`
    ///
    /// `group_name`: 新群名
    pub fn set_group_name(&self, group_id: i64, group_name: &str) {
        let send_api = SendApi::new(
            "set_group_name",
            json!({
                "group_id":group_id,
                    "group_name":group_name,
            }),
            "None",
        );


        let api_tx = self.api_tx.clone();
        tokio::spawn(async move {
            api_tx.send((send_api, None)).await.unwrap();
        });
    }

    /// 退出群组
    ///
    /// # Arguments
    ///
    /// `group_id`
    ///
    /// `is_dismiss`: 是否解散，如果登录号是群主，则仅在此项为 true 时能够解散
    pub fn set_group_leave(&self, group_id: i64, is_dismiss: bool) {
        let send_api = SendApi::new(
            "set_group_leave",
            json!({
                "group_id":group_id,
                    "is_dismiss":is_dismiss,
            }),
            "None",
        );


        let api_tx = self.api_tx.clone();
        tokio::spawn(async move {
            api_tx.send((send_api, None)).await.unwrap();
        });
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
        let send_api = SendApi::new(
            "set_group_special_title",
            json!({
                "group_id":group_id,
                    "user_id":user_id,
                    "special_title":special_title,
            }),
            "None",
        );


        let api_tx = self.api_tx.clone();
        tokio::spawn(async move {
            api_tx.send((send_api, None)).await.unwrap();
        });
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
        let send_api = SendApi::new(
            "set_friend_add_request",
            json!({
                "flag":flag,
                    "approve":approve,
                    "remark":remark,
            }),
            "None",
        );


        let api_tx = self.api_tx.clone();
        tokio::spawn(async move {
            api_tx.send((send_api, None)).await.unwrap();
        });
    }
    /// 处理加群请求／邀请
    ///
    /// # Arguments
    ///
    /// `flag`: 加群请求的 flag（需从上报的数据中获得）
    ///
    /// `type_param`: type 或 sub_type，不同的服务端需要不同的字段名
    ///
    /// `sub_type`: add 或 invite，请求类型（需要和上报消息中的 sub_type 字段相符）
    ///
    /// `approve`: 是否同意请求／邀请
    ///
    /// `remark`: 可为空, 拒绝理由（仅在拒绝时有效）
    pub fn set_group_add_request(&self, flag: &str, type_param: &str, sub_type: &str, approve: bool, reason: &str) {
        let send_api = SendApi::new(
            "set_friend_add_request",
            json!({
                "flag":flag,
                    type_param: sub_type,
                    "approve":approve,
                    "reason":reason,
            }),
            "None",
        );


        let api_tx = self.api_tx.clone();
        tokio::spawn(async move {
            api_tx.send((send_api, None)).await.unwrap();
        });
    }

    /// 清理缓存
    ///
    /// 用于清理积攒了太多的**OneBot服务端**缓存文件。**并非是对于本框架清除**。
    pub fn clean_cache(&self) {
        let send_api = SendApi::new("clean_cache", json!({}), "None");


        let api_tx = self.api_tx.clone();
        tokio::spawn(async move {
            api_tx.send((send_api, None)).await.unwrap();
        });
    }
}

// 这些是需要处理返回值的api
impl RuntimeBot {
    /// 获取消息
    /// # Arguments
    ///
    /// `message_id`: 消息ID
    pub async fn get_msg(&self, message_id: i32) -> Result<ApiReturn, ApiReturn> {
        let send_api = SendApi::new(
            "get_msg",
            json!({
                "message_id":message_id
            }),
            &generate_key(),
        );

        self.send_and_return(send_api).await
    }
    /// 获取合并转发消息
    /// # Arguments
    ///
    /// `id`: 合并转发 ID
    pub async fn get_forward_msg(&self, id: &str) -> Result<ApiReturn, ApiReturn> {
        let send_api = SendApi::new(
            "get_forward_msg",
            json!({
                "id":id
            }),
            &generate_key(),
        );

        self.send_and_return(send_api).await
    }
    /// 获取获取登录号信息
    pub async fn get_login_info(&self) -> Result<ApiReturn, ApiReturn> {
        let send_api = SendApi::new("get_login_info", json!({}), &generate_key());

        self.send_and_return(send_api).await
    }
    /// 获取获取陌生人信息
    /// # Arguments
    ///
    /// `user_id`
    ///
    /// `no_cache`: 是否不使用缓存（使用缓存可能更新不及时，但响应更快）
    pub async fn get_stranger_info(
        &self,
        user_id: i64,
        no_cache: bool,
    ) -> Result<ApiReturn, ApiReturn> {
        let send_api = SendApi::new(
            "get_stranger_info",
            json!({
                    "user_id":user_id,
                    "no_cache":no_cache
            }),
            &generate_key(),
        );

        self.send_and_return(send_api).await
    }
    /// 获取好友列表
    pub async fn get_friend_list(&self) -> Result<ApiReturn, ApiReturn> {
        let send_api = SendApi::new("get_friend_list", json!({}), &generate_key());

        self.send_and_return(send_api).await
    }
    /// 获取群信息
    /// # Arguments
    ///
    /// `group_id`
    ///
    /// `no_cache`: 是否不使用缓存（使用缓存可能更新不及时，但响应更快）
    pub async fn get_group_info(
        &self,
        group_id: i64,
        no_cache: bool,
    ) -> Result<ApiReturn, ApiReturn> {
        let send_api = SendApi::new(
            "get_group_info",
            json!({
                    "group_id":group_id,
                    "no_cache":no_cache
            }),
            &generate_key(),
        );

        self.send_and_return(send_api).await
    }
    /// 获取群列表
    pub async fn get_group_list(&self) -> Result<ApiReturn, ApiReturn> {
        let send_api = SendApi::new("get_group_list", json!({}), &generate_key());

        self.send_and_return(send_api).await
    }
    ///获取群成员信息
    /// # Arguments
    ///
    /// `group_id`
    ///
    /// `user_id`
    ///
    /// `no_cache`: 是否不使用缓存（使用缓存可能更新不及时，但响应更快）
    pub async fn get_group_member_info(
        &self,
        group_id: i64,
        user_id: i64,
        no_cache: bool,
    ) -> Result<ApiReturn, ApiReturn> {
        let send_api = SendApi::new(
            "get_group_member_info",
            json!({
                "group_id":group_id,
                    "user_id":user_id,
                    "no_cache":no_cache
            }),
            &generate_key(),
        );

        self.send_and_return(send_api).await
    }
    /// 获取群成员列表
    ///
    /// # Arguments
    ///
    /// `group_id`
    pub async fn get_group_member_list(&self, group_id: i64) -> Result<ApiReturn, ApiReturn> {
        let send_api = SendApi::new(
            "get_group_member_list",
            json!({
                "group_id":group_id,
            }),
            &generate_key(),
        );

        self.send_and_return(send_api).await
    }

    /// 获取群荣誉信息
    /// # Arguments
    ///
    /// `group_id`
    ///
    /// `honor_type`: 要获取的群荣誉类型，可传入 talkative performer legend strong_newbie emotion 以分别获取单个类型的群荣誉数据，或传入 all 获取所有数据。**本框架已包装好了HonorType枚举**
    pub async fn get_group_honor_info(
        &self,
        group_id: i64,
        honor_type: HonorType,
    ) -> Result<ApiReturn, ApiReturn> {
        let honor_type = match honor_type {
            HonorType::All => "all",
            HonorType::Talkative => "talkative",
            HonorType::Performer => "performer",
            HonorType::Legend => "legend",
            HonorType::StrongNewbie => "strong_newbie",
            HonorType::Emotion => "emotion",
        };

        let send_api = SendApi::new(
            "get_group_honor_info",
            json!({
                "group_id":group_id,
                    "type":honor_type
            }),
            &generate_key(),
        );

        self.send_and_return(send_api).await
    }

    /// 获取相关接口凭证, 即 Cookies 和 CSRF Token 的合并。
    ///
    /// # Arguments
    ///
    /// `domain`: 需要获取 cookies 的域名
    pub async fn get_credentials(&self, domain: &str) -> Result<ApiReturn, ApiReturn> {
        let send_api = SendApi::new(
            "get_credentials",
            json!({
                "domain":domain,
            }),
            &generate_key(),
        );

        self.send_and_return(send_api).await
    }

    /// 获取运行状态
    pub async fn get_status(&self) -> Result<ApiReturn, ApiReturn> {
        let send_api = SendApi::new("get_status", json!({}), &generate_key());

        self.send_and_return(send_api).await
    }
    /// 获取版本信息
    pub async fn get_version_info(&self) -> Result<ApiReturn, ApiReturn> {
        let send_api = SendApi::new("get_version_info", json!({}), &generate_key());
        self.send_and_return(send_api).await
    }
    /// 获取 Cookies
    ///
    /// # Arguments
    ///
    /// `domain`: 需要获取 cookies 的域名
    pub async fn get_cookies(&self, domain: &str) -> Result<ApiReturn, ApiReturn> {
        let send_api = SendApi::new(
            "get_cookies",
            json!({
                "domain":domain,
            }),
            &generate_key(),
        );
        self.send_and_return(send_api).await
    }
    /// 获取 CSRF Token
    pub async fn get_csrf_token(&self) -> Result<ApiReturn, ApiReturn> {
        let send_api = SendApi::new("get_csrf_token", json!({}), &generate_key());

        self.send_and_return(send_api).await
    }
    /// 获取语音
    ///
    /// # Arguments
    ///
    /// `file`: 收到的语音文件名（消息段的 file 参数），如 `0B38145AA44505000B38145AA4450500.silk`
    ///
    /// `out_format`: 要转换到的格式，目前支持 `mp3`、`amr`、`wma`、`m4a`、`spx`、`ogg`、`wav`、`flac`
    pub async fn get_record(&self, file: &str, out_format: &str) -> Result<ApiReturn, ApiReturn> {
        let send_api = SendApi::new(
            "get_record",
            json!({
                "file":file,
                    "out_format":out_format
            }),
            &generate_key(),
        );
        self.send_and_return(send_api).await
    }
    /// 获取图片
    ///
    /// # Arguments
    ///
    /// `file`: 收到的图片文件名（消息段的 file 参数），如 `6B4DE3DFD1BD271E3297859D41C530F5.jpg`
    pub async fn get_image(&self, file: &str) -> Result<ApiReturn, ApiReturn> {
        let send_api = SendApi::new(
            "get_image",
            json!({
                "file":file,
            }),
            &generate_key(),
        );
        self.send_and_return(send_api).await
    }

    /// 点赞，有些服务端会返回点赞失败，不关注返回值的话请使用 send_like()
    /// # Arguments
    ///
    /// `user_id`
    ///
    /// `times`: 次数
    pub async fn send_like_return(
        &self,
        user_id: i64,
        times: usize,
    ) -> Result<ApiReturn, ApiReturn> {
        let send_api = SendApi::new(
            "send_like",
            json!({
                                "user_id":user_id,
                    "times":times,
            }),
            &generate_key(),
        );

        self.send_and_return(send_api).await
    }
}

impl RuntimeBot {
    /// 发送拓展 Api, 此方法无需关注返回值，返回值将丢失。
    ///
    /// 如需要返回值，请使用 `send_api_return()`
    ///
    /// # Arguments
    ///
    /// `action`: 拓展 Api 的方法名
    ///
    /// `params`: 参数
    pub fn send_api(&self, action: &str, params: Value) {
        let send_api = SendApi::new(action, params, "None");

        let api_tx = self.api_tx.clone();
        tokio::spawn(async move {
            api_tx.send((send_api, None)).await.unwrap();
        });
    }
    /// 发送拓展 Api, 此方法关注返回值。
    ///
    /// 如不需要返回值，推荐使用 `send_api()`
    ///
    /// # Arguments
    ///
    /// `action`: 拓展 Api 的方法名
    ///
    /// `params`: 参数
    pub async fn send_api_return(
        &self,
        action: &str,
        params: Value,
    ) -> Result<ApiReturn, ApiReturn> {
        let send_api = SendApi::new(action, params, &generate_key());

        self.send_and_return(send_api).await
    }
}


impl RuntimeBot {
    async fn send_and_return(&self, send_api: SendApi) -> Result<ApiReturn, ApiReturn> {
        #[allow(clippy::type_complexity)]
        let (api_tx, api_rx): (
            oneshot::Sender<Result<ApiReturn, ApiReturn>>,
            oneshot::Receiver<Result<ApiReturn, ApiReturn>>,
        ) = oneshot::channel();
        self.api_tx.send((send_api, Some(api_tx))).await.unwrap();
        match api_rx.await {
            Ok(v) => v,
            Err(e) => {
                error!("{e}");
                panic!()
            }
        }
    }
}
