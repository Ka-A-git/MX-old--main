use chrono::{DateTime, Utc};
use crossbeam::channel::Receiver;
use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
    net::SocketAddr,
};
use std::{sync::RwLock, thread, vec::Vec};
use tokio::net::UdpSocket;
use tracing::{debug, error, info};

use super::SensorMsg;

pub struct SensorManager {
    pub receiver: Receiver<SensorMsg>,
    host_address: SocketAddr,
    current_state: RwLock<SensorManagerState>,
}

impl SensorManager {
    pub fn new(receiver: Receiver<SensorMsg>, host_address: SocketAddr) -> Self {
        Self {
            receiver,
            host_address,
            current_state: RwLock::new(SensorManagerState::Stopped),
        }
    }

    pub fn start(&'static self) -> Result<(), &'static str> {
        info!("Sensor Manager is starting");

        let remote_addr = self.host_address;

        let local_addr: SocketAddr = if remote_addr.is_ipv4() {
            "0.0.0.0:0"
        } else {
            "[::]:0"
        }
        .parse()
        .unwrap();
        info!(
            "Sensor sent to remote {:?} and local {:?}",
            &remote_addr, &local_addr
        );

        match self.current_state.write() {
            Ok(mut state) => match *state {
                SensorManagerState::Started => {
                    let err_msg = "Sensor Manager is already started";
                    error!("{}", err_msg);
                    Err(err_msg)
                }
                SensorManagerState::Stopped => {
                    thread::spawn(move || {
                        info!("[Sensor Manager] Starts to receive sensors");

                        loop {
                            let message = self.receiver.recv();
                            if let Err(_) = message {
                                continue;
                            }
                            let message = message.unwrap();
                            match message {
                                SensorMsg::Terminate => break,
                                SensorMsg::InfluxPoint(point) => {
                                    debug!("Send point {:?}", &point);
                                    tokio::runtime::Runtime::new()
                                        .unwrap()
                                        .block_on(async move {
                                            let socket = UdpSocket::bind(local_addr).await.unwrap();
                                            socket.connect(&remote_addr).await.unwrap();
                                            socket
                                                .send(&point.to_string().into_bytes())
                                                .await
                                                .unwrap();
                                        });
                                } // create udp client and send to it
                            }
                        }
                    });

                    *state = SensorManagerState::Started;

                    info!("Sensor Manager has started");
                    Ok(())
                }
            },
            Err(_e) => {
                debug!("Sensor Manager, Lock error");
                Err("Lock Error")
            }
        }
    }
}

enum SensorManagerState {
    Started,
    Stopped,
}

#[derive(Debug)]
pub struct InfluxPoint {
    measurement: String,
    tags: HashMap<String, String>,
    time: DateTime<Utc>,
    fields: HashMap<String, f64>,
}

impl Display for InfluxPoint {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", String::from(self))
    }
}

impl From<&InfluxPoint> for String {
    fn from(p: &InfluxPoint) -> Self {
        format!(
            "measurement={},{} {} {}",
            p.measurement,
            InfluxPoint::format_params(&p.tags),
            InfluxPoint::format_params(&p.fields),
            p.time.timestamp_nanos()
        )
    }
}

impl InfluxPoint {
    pub fn new(measurement: String) -> Self {
        let tags = HashMap::new();
        let fields = HashMap::new();
        let time = Utc::now();
        Self {
            measurement,
            tags,
            time,
            fields,
        }
    }

    pub fn add_tag(&mut self, name: String, value: String) -> &mut Self {
        self.tags.insert(name, value);
        self
    }

    pub fn add_field(&mut self, name: String, value: f64) -> &mut Self {
        self.fields.insert(name, value);
        self
    }

    pub fn set_time(&mut self, time: chrono::DateTime<Utc>) -> &mut Self {
        self.time = time;
        self
    }

    fn format_params<T: Display, K: Display>(params: &HashMap<T, K>) -> String {
        let params: Vec<String> = params
            .iter()
            .map(|(key, value)| format!("{}={}", key.to_string(), value.to_string()))
            .collect();
        params.join(",")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_formatting_should_work() {
        let now = Utc::now();
        let mut p = InfluxPoint::new("test".into());
        p.set_time(now);
        p.add_tag("some_tag".into(), "some_name".into());
        p.add_field("some_field".into(), 8237482.0);

        assert_eq!(
            p.to_string(),
            format!(
                "measurement=test,some_tag=some_name some_field=8237482 {}",
                now.timestamp_nanos()
            )
        );
    }
}
