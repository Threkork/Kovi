use super::{exit_and_eprintln, handler::InternalEvent, ApiAndOneshot, ApiReturn, Bot};
use ahash::{HashMapExt as _, RandomState};
use futures_util::{SinkExt, StreamExt};
use log::{debug, warn};
use reqwest::header::HeaderValue;
use std::{collections::HashMap, net::IpAddr, sync::Arc};
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::{connect_async, tungstenite::client::IntoClientRequest};

type ApiTxMap = Arc<
    Mutex<HashMap<String, tokio::sync::oneshot::Sender<Result<ApiReturn, ApiReturn>>, RandomState>>,
>;

impl Bot {
    pub(crate) async fn ws_connect(
        host: IpAddr,
        port: u16,
        access_token: String,
        event_tx: mpsc::Sender<InternalEvent>,
    ) {
        //增加Authorization头
        let mut request = format!("ws://{}:{}/event", host, port)
            .into_client_request()
            .unwrap();

        if !access_token.is_empty() {
            request.headers_mut().insert(
                "Authorization",
                HeaderValue::from_str(&format!("Bearer {}", access_token)).unwrap(),
            );
        }

        let (ws_stream, _) = match connect_async(request).await {
            Ok(v) => v,
            Err(e) => {
                exit_and_eprintln(e, event_tx).await;
                return;
            }
        };

        let (_, read) = ws_stream.split();
        read.for_each(|msg| {
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
                    Err(e) => exit_and_eprintln(e, event_tx).await,
                }
            }
        })
        .await;
    }

    pub(crate) async fn ws_send_api(
        host: IpAddr,
        port: u16,
        access_token: String,
        mut api_rx: mpsc::Receiver<ApiAndOneshot>,
        event_tx: mpsc::Sender<InternalEvent>,
    ) {
        //增加Authorization头
        let mut request = format!("ws://{}:{}/api", host, port)
            .into_client_request()
            .unwrap();

        if !access_token.is_empty() {
            request.headers_mut().insert(
                "Authorization",
                HeaderValue::from_str(&format!("Bearer {}", access_token)).unwrap(),
            );
        }


        let (ws_stream, _) = match connect_async(request).await {
            Ok(v) => v,
            Err(e) => {
                exit_and_eprintln(e, event_tx).await;
                return;
            }
        };

        let (write, read) = ws_stream.split();
        let write = Arc::new(Mutex::new(write));

        let api_tx_map: ApiTxMap = Arc::new(Mutex::new(HashMap::<_, _, RandomState>::new()));

        //读
        tokio::spawn({
            let api_tx_map = Arc::clone(&api_tx_map);
            let event_tx = event_tx.clone();
            async move {
                read.for_each(|msg| {
                    let event_tx = event_tx.clone();
                    async {
                        match msg {
                            Ok(msg) => {
                                if msg.is_close() {
                                    exit_and_eprintln(
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


                                let mut api_tx_map = api_tx_map.lock().await;

                                let api_tx = api_tx_map.remove(&return_value.echo).unwrap();
                                let r = if return_value.status.to_lowercase() == "ok" {
                                    api_tx.send(Ok(return_value))
                                } else {
                                    api_tx.send(Err(return_value))
                                };

                                if r.is_err() {
                                    log::error!("Return Api failed, the receiver has been closed")
                                };
                            }

                            Err(e) => exit_and_eprintln(e, event_tx).await,
                        }
                    }
                })
                .await;
            }
        });


        //写
        tokio::spawn({
            let write = Arc::clone(&write);
            let api_tx_map = Arc::clone(&api_tx_map);
            async move {
                while let Some((api_msg, return_api_tx)) = api_rx.recv().await {
                    let event_tx = event_tx.clone();
                    debug!("{}", api_msg);

                    if &api_msg.echo != "None" {
                        api_tx_map
                            .lock()
                            .await
                            .insert(api_msg.echo.clone(), return_api_tx.unwrap());
                    }

                    let msg = tokio_tungstenite::tungstenite::Message::text(api_msg.to_string());
                    let mut write_lock = write.lock().await;
                    if let Err(e) = write_lock.send(msg).await {
                        exit_and_eprintln(e, event_tx).await;
                    }
                }
            }
        });
    }
}
