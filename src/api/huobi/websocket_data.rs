use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use serde_json;
use std::io::prelude::*;
use tracing::debug;
use tungstenite::{client::AutoStream, connect, Message, WebSocket};
use url::Url;

const WS_HUOBI_URL: &str = "wss://api.huobi.pro/ws";

const HUOBI_READ_MESSAGE_ERROR: &str = "Error reading message";

#[derive(Debug)]
pub struct HuobiWS {
    socket: WebSocket<AutoStream>,
}

impl HuobiWS {
    pub fn connect(instrument: &str) -> Self {
        debug!("Connecting to Huobi WebSocket");

        // Connect to Huobi WebSockets
        let (mut socket, response) =
            connect(Url::parse(WS_HUOBI_URL).unwrap()).expect("Can't connect");

        debug!("Huobi WebSocket headers");

        for (header, _value) in response.headers() {
            debug!("* {}", header);
        }

        // Read Huobi ping message
        let msg = socket.read_message().expect(HUOBI_READ_MESSAGE_ERROR);
        let data = msg.into_data();
        let ping_number = HuobiWS::ping_number(&data);

        // Send back pong message to Huobi
        HuobiWS::send_pong_message(ping_number, &mut socket);

        let sub_msg = SubscriptionMessage {
            sub: format!("market.{}.mbp.refresh.20", instrument.to_lowercase()),
            id: "id1".to_string(),
        };

        let sub_msg_str = serde_json::to_string(&sub_msg).unwrap();

        // Send subscription message
        socket
            .write_message(Message::Text(sub_msg_str.into()))
            .unwrap();

        // Receive responce status
        let _status_msg = socket.read_message().expect(HUOBI_READ_MESSAGE_ERROR);

        HuobiWS { socket }
    }

    fn decode_message(data: &Vec<u8>) -> String {
        let mut gz_decoder = GzDecoder::new(&data[..]);
        let mut buffer = String::new();
        match gz_decoder.read_to_string(&mut buffer) {
            Ok(_v) => {}
            Err(e) => println!("error decode message {}", e),
        };

        debug!("[Huobi WS] Decoded message: {}", buffer);

        buffer
    }

    fn ping_number(data: &Vec<u8>) -> i64 {
        let ping_string = HuobiWS::decode_message(data);
        let ping_msg: PingMessage = serde_json::from_str(&ping_string).unwrap();

        debug!("[Huobi WS] Ping number: {}", ping_msg.ping);

        ping_msg.ping
    }

    fn send_pong_message(pong: i64, socket: &mut WebSocket<AutoStream>) {
        debug!("[Huobi WS] Send pong message, pong = {}", pong);

        let pong_msg = PongMessage { pong: pong };

        let pong_str = serde_json::to_string(&pong_msg).unwrap();

        match socket.write_message(Message::Text(pong_str.into())) {
            Ok(_) => {}
            Err(e) => println!("error send pong message {}", e),
        };
    }

    fn read_message(&mut self) -> String {
        debug!("[Huobi WS] Read message");

        let msg = self.socket.read_message().expect(HUOBI_READ_MESSAGE_ERROR);
        let data = msg.into_data();

        HuobiWS::decode_message(&data)
    }

    pub fn get_depth(&mut self) -> Depth {
        // Loop until get depth data
        loop {
            let msg = self.socket.read_message().expect(HUOBI_READ_MESSAGE_ERROR);

            let data = msg.into_data();
            let decoded_msg = HuobiWS::decode_message(&data);

            if decoded_msg.contains("ping") {
                debug!("[Huobi WS] Ping");

                let ping_number = HuobiWS::ping_number(&data);

                HuobiWS::send_pong_message(ping_number, &mut self.socket);
            } else {
                match serde_json::from_str::<DepthMessage>(&decoded_msg) {
                    Ok(depth_msg) => {
                        return HuobiWS::tick_to_depth(depth_msg.tick);
                    }
                    Err(e) => println!("err get depth {}", e),
                }
            }
        }
    }

    fn tick_to_depth(tick: Tick) -> Depth {
        Depth {
            exchange: "Huobi".to_string(),
            bids: tick
                .bids
                .iter()
                .map(|b| Ticker {
                    price: b[0],
                    qty: b[1],
                })
                .collect(),
            asks: tick
                .asks
                .iter()
                .map(|a| Ticker {
                    price: a[0],
                    qty: a[1],
                })
                .collect(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct PingMessage {
    ping: i64,
}

#[derive(Debug, Serialize)]
struct PongMessage {
    pong: i64,
}

#[derive(Debug, Serialize)]
struct SubscriptionMessage {
    sub: String,
    id: String,
}

#[derive(Debug, Deserialize)]
struct DepthMessage {
    ch: String,
    ts: i64, //system update time
    tick: Tick,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Tick {
    seq_num: i64,
    bids: Vec<[f64; 2]>, // [price, size]
    asks: Vec<[f64; 2]>,
}

#[derive(Clone, Debug)]
pub struct Ticker {
    pub price: f64,
    pub qty: f64,
}

impl Default for Ticker {
    fn default() -> Self {
        Ticker {
            price: 1.1,
            qty: 10.01,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Depth {
    pub exchange: String,
    pub bids: Vec<Ticker>,
    pub asks: Vec<Ticker>,
}

impl Default for Depth {
    fn default() -> Self {
        Depth {
            exchange: "StubExchange".to_string(),
            bids: vec![Ticker::default()],
            asks: vec![Ticker::default()],
        }
    }
}

#[cfg(test)]
mod tests {

    use super::HuobiWS;

    #[test]
    #[ignore]
    // For local testing
    fn get_depth() {
        let instrument = "btcusdt";

        let mut huobi_ws = HuobiWS::connect(instrument);

        loop {
            println!("{:?}", huobi_ws.get_depth());
        }
    }
}
