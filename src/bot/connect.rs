use super::Server;
use super::{handler::InternalEvent, ApiAndOneshot, ApiReturn, Bot, Host};
use ahash::{HashMapExt as _, RandomState};
use futures_util::{SinkExt, StreamExt};
use log::{debug, error, warn};
use parking_lot::Mutex;
use reqwest::header::HeaderValue;
use std::error::Error;
use std::fmt::Display;
use std::sync::RwLock;
use std::{collections::HashMap, net::IpAddr, sync::Arc};
use tokio::sync::mpsc::Sender;
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::{connect_async, tungstenite::client::IntoClientRequest};

type ApiTxMap = Arc<
    Mutex<HashMap<String, tokio::sync::oneshot::Sender<Result<ApiReturn, ApiReturn>>, RandomState>>,
>;

impl Bot {
    pub(crate) async fn ws_connect(
        server: Server,
        api_rx: mpsc::Receiver<ApiAndOneshot>,
        event_tx: mpsc::Sender<InternalEvent>,
        bot: Arc<RwLock<Bot>>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        #[allow(clippy::type_complexity)]
        let (event_connected_tx, event_connected_rx): (
            oneshot::Sender<Result<(), Box<dyn std::error::Error + Send + Sync>>>,
            oneshot::Receiver<Result<(), Box<dyn std::error::Error + Send + Sync>>>,
        ) = oneshot::channel();

        #[allow(clippy::type_complexity)]
        let (api_connected_tx, api_connected_rx): (
            oneshot::Sender<Result<(), Box<dyn Error + Send + Sync>>>,
            oneshot::Receiver<Result<(), Box<dyn Error + Send + Sync>>>,
        ) = oneshot::channel();

        {
            let mut bot_write = bot.write().unwrap();
            bot_write.spawn(Self::ws_event_connect(
                server.clone(),
                event_tx.clone(),
                event_connected_tx,
                bot.clone(),
            ));
            bot_write.spawn(Self::ws_send_api(
                server,
                api_rx,
                event_tx,
                api_connected_tx,
                bot.clone(),
            ));
        }

        let (res1, res2) = tokio::join!(event_connected_rx, api_connected_rx);
        let (res1, res2) = (res1.unwrap(), res2.unwrap());
        match (res1, res2) {
            (Ok(_), Ok(_)) => Ok(()),
            (Err(e), _) | (_, Err(e)) => Err(e),
        }
    }

    pub(crate) async fn ws_event_connect(
        server: Server,
        event_tx: mpsc::Sender<InternalEvent>,
        connected_tx: oneshot::Sender<Result<(), Box<dyn Error + Send + Sync>>>,
        bot: Arc<RwLock<Bot>>,
    ) {
        let (host, port, access_token, secure) =
            (server.host, server.port, server.access_token, server.secure);

        let protocol = if secure { "wss" } else { "ws" };
        let mut request = match host {
            Host::IpAddr(ip) => match ip {
                IpAddr::V4(ip) => format!("{}://{}:{}/event", protocol, ip, port)
                    .into_client_request()
                    .unwrap(),
                IpAddr::V6(ip) => format!("{}://[{}]:{}/event", protocol, ip, port)
                    .into_client_request()
                    .unwrap(),
            },
            Host::Domain(domain) => format!("{}://{}:{}/event", protocol, domain, port)
                .into_client_request()
                .unwrap(),
        };

        //增加Authorization头
        if !access_token.is_empty() {
            request.headers_mut().insert(
                "Authorization",
                HeaderValue::from_str(&format!("Bearer {}", access_token)).unwrap(),
            );
        }

        let (ws_stream, _) = match connect_async(request).await {
            Ok(v) => v,
            Err(e) => {
                connected_tx.send(Err(e.into())).unwrap();
                return;
            }
        };

        connected_tx.send(Ok(())).unwrap();

        let (_, read) = ws_stream.split();

        let mut bot_write = bot.write().unwrap();
        bot_write.spawn(read.for_each(move |msg| {
            let event_tx = event_tx.clone();
            async {
                match msg {
                    Ok(msg) => {
                        if !msg.is_text() {
                            return;
                        }

                        let text = msg.to_text().unwrap();
                        if let Err(e) = event_tx
                            .send(InternalEvent::OneBotEvent(text.to_string()))
                            .await
                        {
                            debug!("通道关闭：{e}")
                        }
                    }
                    Err(e) => connection_failed_eprintln(e, event_tx).await,
                }
            }
        }));
    }

    pub(crate) async fn ws_send_api(
        server: Server,
        mut api_rx: mpsc::Receiver<ApiAndOneshot>,
        event_tx: mpsc::Sender<InternalEvent>,
        connected_tx: oneshot::Sender<Result<(), Box<dyn std::error::Error + Send + Sync>>>,
        bot: Arc<RwLock<Bot>>,
    ) {
        let (host, port, access_token, secure) =
            (server.host, server.port, server.access_token, server.secure);

        let protocol = if secure { "wss" } else { "ws" };
        let mut request = match host {
            Host::IpAddr(ip) => match ip {
                IpAddr::V4(ip) => format!("{}://{}:{}/api", protocol, ip, port)
                    .into_client_request()
                    .unwrap(),
                IpAddr::V6(ip) => format!("{}://[{}]:{}/api", protocol, ip, port)
                    .into_client_request()
                    .unwrap(),
            },
            Host::Domain(domain) => format!("{}://{}:{}/api", protocol, domain, port)
                .into_client_request()
                .unwrap(),
        };

        //增加Authorization头
        if !access_token.is_empty() {
            request.headers_mut().insert(
                "Authorization",
                HeaderValue::from_str(&format!("Bearer {}", access_token)).unwrap(),
            );
        }

        let (ws_stream, _) = match connect_async(request).await {
            Ok(v) => v,
            Err(e) => {
                connected_tx.send(Err(e.into())).unwrap();
                return;
            }
        };

        connected_tx.send(Ok(())).unwrap();

        let (mut write, read) = ws_stream.split();
        let api_tx_map: ApiTxMap = Arc::new(Mutex::new(HashMap::<_, _, RandomState>::new()));

        let mut bot_write = bot.write().unwrap();

        //读
        bot_write.spawn({
            let api_tx_map = Arc::clone(&api_tx_map);
            let event_tx = event_tx.clone();
            async move {
                read.for_each(|msg| {
                    let event_tx = event_tx.clone();
                    async {
                        match msg {
                            Ok(msg) => {
                                if msg.is_close() {
                                    connection_failed_eprintln(
                                        format!("{msg}\nBot api connection failed"),
                                        event_tx,
                                    )
                                    .await;
                                    return;
                                }
                                if !msg.is_text() {
                                    return;
                                }

                                let text = msg.to_text().unwrap();

                                debug!("{}", text);

                                let return_value: ApiReturn = match serde_json::from_str(text) {
                                    Ok(v) => v,
                                    Err(_) => {
                                        warn!("Unknow api return： {text}");
                                        return;
                                    }
                                };

                                if return_value.status != "ok" {
                                    warn!("Api return error: {text}")
                                }

                                if return_value.echo == "None" {
                                    return;
                                }

                                let mut api_tx_map = api_tx_map.lock();

                                let api_tx = api_tx_map.remove(&return_value.echo).unwrap();
                                let r = if return_value.status.to_lowercase() == "ok" {
                                    api_tx.send(Ok(return_value))
                                } else {
                                    api_tx.send(Err(return_value))
                                };

                                if r.is_err() {
                                    log::debug!(
                                        "Return Api to plugin failed, the receiver has been closed"
                                    )
                                };
                            }

                            Err(e) => connection_failed_eprintln(e, event_tx).await,
                        }
                    }
                })
                .await;
            }
        });

        //写
        bot_write.spawn({
            let api_tx_map = Arc::clone(&api_tx_map);
            async move {
                while let Some((api_msg, return_api_tx)) = api_rx.recv().await {
                    let event_tx = event_tx.clone();
                    debug!("{}", api_msg);

                    if &api_msg.echo != "None" {
                        api_tx_map
                            .lock()
                            .insert(api_msg.echo.clone(), return_api_tx.unwrap());
                    }

                    let msg = tokio_tungstenite::tungstenite::Message::text(api_msg.to_string());

                    if let Err(e) = write.send(msg).await {
                        connection_failed_eprintln(e, event_tx).await;
                    }
                }
            }
        });
    }
}

async fn connection_failed_eprintln<E>(e: E, event_tx: Sender<InternalEvent>)
where
    E: Display,
{
    log::error!("{e}\nBot connection failed, please check the configuration and restart KoviBot");
    if let Err(e) = event_tx
        .send(InternalEvent::KoviEvent(
            crate::bot::handler::KoviEvent::Drop,
        ))
        .await
    {
        error!("通道关闭,{e}")
    };
}
