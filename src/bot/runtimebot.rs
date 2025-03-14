use crate::types::{ApiAndOneshot, ApiOneshotReceiver, ApiOneshotSender};

use super::{ApiReturn, Bot, Host, SendApi};
use log::error;
use parking_lot::RwLock;
use rand::Rng;
use std::sync::Weak;
use tokio::sync::{mpsc, oneshot};

pub mod kovi_api;
pub mod onebot_api;

pub use kovi_api::SetAdmin;

/// 运行时的Bot，可以用来发送api，需要从PluginBuilder的.get_runtime_bot()获取。
/// # Examples
/// ```
/// let bot = PluginBuilder::get_runtime_bot();
/// let user_id = bot.main_admin;
///
/// bot.send_private_msg(user_id, "bot online")
/// ```
#[derive(Clone)]
pub struct RuntimeBot {
    pub host: Host,
    pub port: u16,

    pub(crate) bot: Weak<RwLock<Bot>>,
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

/// 提供给拓展API插件开发者的异步 API 请求发送函数，返回一个 Future ，用于等待在 Kovi 中已经缓存好的API响应。
pub fn send_api_request_with_response(
    api_tx: &mpsc::Sender<ApiAndOneshot>,
    send_api: SendApi,
) -> impl std::future::Future<Output = Result<ApiReturn, ApiReturn>> {
    let api_rx = send_api_request(api_tx, send_api);
    send_api_await_response(api_rx)
}

/// 提供给拓展 API 插件开发者的 API 请求发送函数，返回一个 API 通道，可以用于等待 API 响应。
pub fn send_api_request(
    api_tx: &mpsc::Sender<ApiAndOneshot>,
    send_api: SendApi,
) -> ApiOneshotReceiver {
    let (api_tx_, api_rx): (ApiOneshotSender, ApiOneshotReceiver) = oneshot::channel();

    if let Err(e) = api_tx.try_send((send_api, Some(api_tx_))) {
        match e {
            mpsc::error::TrySendError::Full(v) => {
                log::trace!("RuntimeBot Api Queue Full, spawn new task to send");

                let api_tx = api_tx.clone();

                tokio::task::spawn(async move {
                    api_tx.send(v).await.unwrap();
                });
            }
            mpsc::error::TrySendError::Closed(_) => {
                log::error!("RuntimeBot Api Queue Closed");
            }
        }
    };

    api_rx
}

/// 提供给拓展 API 插件开发者的 API 请求发送函数，忽略返回值。
pub fn send_api_request_with_forget(api_tx: &mpsc::Sender<ApiAndOneshot>, send_api: SendApi) {
    if let Err(e) = api_tx.try_send((send_api, None)) {
        match e {
            mpsc::error::TrySendError::Full(v) => {
                log::trace!("RuntimeBot Api Queue Full, spawn new task to send");

                let api_tx = api_tx.clone();

                tokio::task::spawn(async move {
                    api_tx.send(v).await.unwrap();
                });
            }
            mpsc::error::TrySendError::Closed(_) => {
                log::error!("RuntimeBot Api Queue Closed");
            }
        }
    };
}

/// 一个异步 Future ，传入一个 API 通道，可以用于等待在 Kovi 中缓存好的 API 响应。
pub async fn send_api_await_response(api_rx: ApiOneshotReceiver) -> Result<ApiReturn, ApiReturn> {
    match api_rx.await {
        Ok(v) => v,
        Err(e) => {
            error!("{e}");
            panic!()
        }
    }
}
