use serde_json::Value;
use std::{net::IpAddr, sync::mpsc};

use crate::error::Error;

pub mod api;

pub type ApiMpsc = (Value, Option<mpsc::Sender<Result<Value, Error>>>);

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
pub struct RuntimeBot {
    /// 主管理员
    pub main_admin: i64,
    /// 副管理员，不包含主管理员
    pub admin: Vec<i64>,

    pub host: IpAddr,
    pub port: u16,
    /// 可以发送api，请按照OneBot v11发送api，不然会失败
    pub api_tx: mpsc::Sender<ApiMpsc>,
}
