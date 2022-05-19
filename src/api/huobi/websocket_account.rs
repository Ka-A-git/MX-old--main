use super::client::{build_query_string, get_timestamp, sign_hmac_sha256_base64};
use super::models::*;
use lazy_static::lazy_static;
use serde::Deserialize;
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tracing::{debug, info};
use tungstenite::client::AutoStream;
use tungstenite::handshake::client::Response;
use tungstenite::protocol::WebSocket;
use tungstenite::{connect, Message};
use url::Url;

static WEBSOCKET_URL: &'static str = "wss://api.huobi.pro/ws/v2";

static WS_HOST: &'static str = "api.huobi.pro";

lazy_static! {
    static ref SYMBOLS: Mutex<Vec<String>> = Mutex::new(vec![]);
    static ref CHANNELS: Mutex<Vec<String>> = Mutex::new(vec![]);
}

#[derive(Debug, Deserialize)]
struct PingMessage {
    action: String,
    data: PingData,
}

#[derive(Debug, Deserialize)]
struct PingData {
    ts: u64,
}

pub enum WebsocketEvent {
    OrderUpdate(OrderSubs),
}

pub struct WebSockets<'a> {
    pub socket: Option<(WebSocket<AutoStream>, Response)>,
    handler: Box<dyn FnMut(WebsocketEvent) -> APIResult<()> + 'a>,
}

impl<'a> WebSockets<'a> {
    pub fn new<Callback>(handler: Callback) -> WebSockets<'a>
    where
        Callback: FnMut(WebsocketEvent) -> APIResult<()> + 'a,
    {
        WebSockets {
            socket: None,
            handler: Box::new(handler),
        }
    }

    pub fn connect_auth(
        &mut self,
        endpoint: &str,
        symbols: Vec<&str>,
        _channels: Vec<&str>,
        access_key: &str,
        secret_key: &str,
    ) -> APIResult<()> {
        let url = Url::parse(WEBSOCKET_URL)?;

        for symbol in symbols {
            SYMBOLS.lock().unwrap().push(symbol.to_string());
        }

        // for channel in channels {
        //     CHANNELS.lock().unwrap().push(channel.to_string());
        // }

        match connect(url) {
            Ok(answer) => {
                self.socket = Some(answer);

                let mut params: BTreeMap<String, String> = BTreeMap::new();

                params.insert("accessKey".to_string(), access_key.to_string());
                params.insert("signatureMethod".to_string(), "HmacSHA256".to_string());
                params.insert("signatureVersion".to_string(), "2.1".to_string());
                let utctime = get_timestamp();
                params.insert("timestamp".to_string(), utctime.clone());
                // println!("params: {:?}", params.clone());

                let build_params = build_query_string(params.clone());

                let format_str = format!("{}\n{}\n{}\n{}", "GET", WS_HOST, endpoint, build_params,);

                let signature = sign_hmac_sha256_base64(&secret_key, &format_str).to_string();

                let auth_message = json!( {
                    "action": "req",
                    "ch": "auth",
                    "params": {
                        "authType":"api",
                        "accessKey": params.get(&"accessKey".to_string()),
                        "signatureMethod": params.get(&"signatureMethod".to_string()),

                        "signatureVersion": params.get(&"signatureVersion".to_string()),

                        "timestamp": params.get(&"timestamp".to_string()),
                        "signature": signature,
                    }
                });

                // auth
                if let Some(ref mut socket) = self.socket {
                    socket
                        .0
                        .write_message(tungstenite::Message::Text(auth_message.to_string()))?;
                    debug!("Write auth message {}", auth_message.to_string());
                };

                Ok(())
            }
            Err(e) => {
                info!("Error during handshake {}", e);
                Err(Box::new(e))
            }
        }
    }

    pub fn disconnect(&mut self) -> APIResult<()> {
        if let Some(ref mut socket) = self.socket {
            socket.0.close(None)?;
            Ok(())
        } else {
            info!("Not able to close the connection");
            Ok(())
        }
    }

    pub fn event_loop(&mut self, running: &AtomicBool) -> APIResult<()> {
        while running.load(Ordering::Relaxed) {
            if let Some(ref mut socket) = self.socket {
                let message = socket.0.read_message()?;

                match message {
                    Message::Text(text) => {
                        let msg: serde_json::Value = serde_json::from_str(&text)?;

                        if let Some(action) = msg.get("action") {
                            match action {
                                serde_json::Value::String(action_type) => {
                                    match action_type.as_str() {
                                        "ping" => {
                                            debug!("ping {}", text);

                                            let ping_number =
                                                serde_json::from_str::<PingMessage>(&text)
                                                    .unwrap()
                                                    .data
                                                    .ts;

                                            if let Some(ref mut socket) = self.socket {
                                                let pong_message = json!({
                                                "action": "pong",
                                                "data": {
                                                      "ts": ping_number
                                                }
                                                });

                                                socket.0.write_message(
                                                    tungstenite::Message::Text(
                                                        pong_message.to_string(),
                                                    ),
                                                )?;

                                                debug!(
                                                    "Write message {}",
                                                    pong_message.to_string()
                                                );
                                            };
                                        }

                                        "req" => {
                                            debug!("req {}", text);

                                            for symbol in &*SYMBOLS.lock().unwrap() {
                                                let subscribe_message = json!({
                                                        "action": "sub",
                                                        "ch": format!("orders#{}", symbol.to_lowercase())
                                                });

                                                // subscribe
                                                if let Some(ref mut socket) = self.socket {
                                                    socket.0.write_message(
                                                        tungstenite::Message::Text(
                                                            subscribe_message.to_string(),
                                                        ),
                                                    )?;
                                                    debug!(
                                                        "[Huobi] WebSockets write message {}",
                                                        subscribe_message.to_string()
                                                    );
                                                };
                                            }
                                        }

                                        "sub" => {
                                            debug!("sub {}", text);
                                        }

                                        "push" => {
                                            debug!("push {:?}", text);

                                            let order_sub: OrderSubs =
                                                serde_json::from_str(&text).unwrap();

                                            (self.handler)(WebsocketEvent::OrderUpdate(order_sub))
                                                .unwrap();
                                        }

                                        _ => {}
                                    }
                                }

                                _ => {}
                            }
                        }
                    }

                    Message::Ping(_bin) | Message::Pong(_bin) | Message::Binary(_bin) => {
                        debug!("[Huobi] WebSockets binary message");
                    }

                    Message::Close(e) => {
                        info!("Disconnected {:?}", e);
                    }
                }
            }
        }
        Ok(())
    }
}
