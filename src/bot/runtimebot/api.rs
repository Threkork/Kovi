use super::RuntimeBot;
use serde_json::json;


impl RuntimeBot {
    ///发送群组消息
    pub fn send_group_msg(&self, group_id: i64, msg: &str) {
        let send_api = json!({
            "action": "send_msg",
            "params": {
                "message_type":"group",
                "group_id":group_id,
                "message":msg,
                "auto_escape":true,
            },
            "echo": "123" });

        self.api_tx.send(send_api).unwrap();
    }

    ///发送私聊消息
    pub fn send_private_msg(&self, user_id: i64, msg: &str) {
        let send_api = json!({
            "action": "send_msg",
            "params": {
                "message_type":"private",
                "user_id":user_id,
                "message":msg,
                "auto_escape":true,
            },
            "echo": "123" });

        self.api_tx.send(send_api).unwrap();
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
                "message_id	":message_id,
            },
            "echo": "123" });

        self.api_tx.send(send_api).unwrap();
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
            "echo": "123" });

        self.api_tx.send(send_api).unwrap();
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
            "echo": "123" });

        self.api_tx.send(send_api).unwrap();
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
            "echo": "123" });

        self.api_tx.send(send_api).unwrap();
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
            "echo": "123" });

        self.api_tx.send(send_api).unwrap();
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
            "echo": "123" });

        self.api_tx.send(send_api).unwrap();
    }

    /// 设置群名片（群备注）
    ///
    /// # Arguments
    ///
    /// `group_id`
    ///
    /// `user_id`: 要设置的 QQ 号
    ///
    /// `card`: 群名片内容，不填或空字符串表示删除群名片
    pub fn set_group_card(&self, group_id: i64, user_id: i64, card: String) {
        let send_api = json!({
            "action": "set_group_card",
            "params": {
                "group_id":group_id,
                "user_id":user_id,
                "card":card,
            },
            "echo": "123" });

        self.api_tx.send(send_api).unwrap();
    }

    /// 设置群名
    ///
    /// # Arguments
    ///
    /// `group_id`
    ///
    /// `group_name`: 新群名
    pub fn set_group_name(&self, group_id: i64, group_name: String) {
        let send_api = json!({
            "action": "set_group_name",
            "params": {
                "group_id":group_id,
                "group_name":group_name,
            },
            "echo": "123" });

        self.api_tx.send(send_api).unwrap();
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
            "echo": "123" });

        self.api_tx.send(send_api).unwrap();
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
    pub fn set_group_special_title(&self, group_id: i64, user_id: i64, special_title: String) {
        let send_api = json!({
            "action": "set_group_special_title",
            "params": {
                "group_id":group_id,
                "user_id":user_id,
                "special_title":special_title,
            },
            "echo": "123" });

        self.api_tx.send(send_api).unwrap();
    }

    /// 清理缓存
    ///
    /// 用于清理积攒了太多的OneBot服务端缓存文件。并非是对于本框架清除。
    pub fn clean_cache(&self) {
        let send_api = json!({
            "action": "clean_cache",
            "params": {},
            "echo": "123" });

        self.api_tx.send(send_api).unwrap();
    }
}
