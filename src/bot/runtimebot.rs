use super::{ApiAndOneshot, Bot};
use rand::Rng;
use std::{
    net::IpAddr,
    sync::{Arc, RwLock},
};
use tokio::sync::mpsc;

pub mod kovi_api;

pub mod onebot_api;


/// 运行时的Bot，可以用来发送api，需要从PluginBuilder的.build_runtime_bot()构建。
/// # Examples
/// ```
/// let bot = PluginBuilder::get_runtime_bot();
/// let user_id = bot.main_admin;
///
/// bot.send_private_msg(user_id, "bot online")
/// ```
#[derive(Clone)]
pub struct RuntimeBot {
    /// 主管理员
    pub main_admin: i64,
    /// 副管理员，不包含主管理员
    pub admin: Vec<i64>,

    pub host: IpAddr,
    pub port: u16,

    pub(crate) bot: Arc<RwLock<Bot>>,
    pub(crate) plugin_name: String,
    pub api_tx: mpsc::Sender<ApiAndOneshot>,
}

pub fn rand_echo() -> String {
    let mut rng = rand::thread_rng();
    let mut s = String::new();
    s.push_str(&chrono::Utc::now().timestamp().to_string());
    for _ in 0..10 {
        s.push(rng.gen_range('a'..='z'));
    }
    s
}
