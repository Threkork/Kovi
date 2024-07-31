use super::RuntimeBot;
use serde_json::json;


impl RuntimeBot {
    pub fn send_group_msg(&self, group_id: i64, msg: &str) {
        let send_msg = json!({
            "action": "send_msg",
            "params": {
                "message_type":"group",
                "group_id":group_id,
                "message":msg,
                "auto_escape":true,
            },
            "echo": "123" });

        self.api_tx.send(send_msg).unwrap();
    }

    pub fn send_private_msg(&self, user_id: i64, msg: &str) {
        let send_msg = json!({
            "action": "send_msg",
            "params": {
                "message_type":"private",
                "user_id":user_id,
                "message":msg,
                "auto_escape":true,
            },
            "echo": "123" });

        self.api_tx.send(send_msg).unwrap();
    }
}
