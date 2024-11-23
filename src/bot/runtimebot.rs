use super::{ApiAndOneshot, ApiReturn, Bot, Host, SendApi};
use log::error;
use rand::Rng;
use std::sync::{RwLock, Weak};
use tokio::sync::{mpsc, oneshot};

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

type ApiOneshotSender = oneshot::Sender<Result<ApiReturn, ApiReturn>>;
type ApiOneshotReceiver = oneshot::Receiver<Result<ApiReturn, ApiReturn>>;

pub fn send_api_request_with_response(
    api_tx: &mpsc::Sender<ApiAndOneshot>,
    send_api: SendApi,
) -> impl std::future::Future<Output = Result<ApiReturn, ApiReturn>> {
    let api_rx = send_api_request(api_tx, send_api);
    send_api_await_response(api_rx)
}

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

pub async fn send_api_await_response(api_rx: ApiOneshotReceiver) -> Result<ApiReturn, ApiReturn> {
    match api_rx.await {
        Ok(v) => v,
        Err(e) => {
            error!("{e}");
            panic!()
        }
    }
}
