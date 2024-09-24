use super::{Bot, SendApi};
use onebot_api::ApiReturn;
use std::{
    net::IpAddr,
    sync::{Arc, RwLock},
};
use tokio::sync::{mpsc, oneshot};

pub mod kovi_api;
pub mod onebot_api;

pub type ApiOneshot = (
    SendApi,
    Option<oneshot::Sender<Result<ApiReturn, ApiReturn>>>,
);

/// 运行时的Bot，可以用来发送api，需要从PluginBuilder的.build_runtime_bot()构建。
/// # Examples
/// ```
/// pub fn main(mut plugin: PluginBuilder) {
///     plugin.set_info("online");
///     let bot = plugin.build_runtime_bot();
///     let user_id = bot.main_admin;
///
///     bot.send_private_msg(user_id, "bot online")
/// }
/// ```
#[allow(clippy::needless_doctest_main)]
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
    pub(crate) api_tx: mpsc::Sender<ApiOneshot>,
}
