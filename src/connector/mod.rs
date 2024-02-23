mod error;
mod options;

use crate::connector::Error;
use crate::model::{DeviceClass, Discovery};
pub use error::*;
pub use options::*;
use rand::{distributions::Alphanumeric, Rng};
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS, TlsConfiguration, Transport};
use serde::Serialize;
use std::fmt::Formatter;
use std::future::Future;
use std::time::Duration;
use tokio::select;

fn random_client_id() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(23)
        .map(char::from)
        .collect()
}

pub struct Connector<F, H>
where
    F: FnOnce(Client) -> H,
    H: ConnectorHandler,
{
    options: ConnectorOptions,
    handler: F,
}

pub trait ConnectorHandler {
    type Error: std::error::Error + Send + Sync;

    fn connected(&mut self, state: bool) -> impl Future<Output = Result<(), Self::Error>>;
    fn restarted(&mut self) -> impl Future<Output = Result<(), Self::Error>>;
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Component {
    BinarySensor,
    Sensor,
}

impl std::fmt::Display for Component {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BinarySensor => f.write_str("binary_sensor"),
            Self::Sensor => f.write_str("sensor"),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("serialization failure")]
    Serialization(#[from] serde_json::Error),
    #[error("client error")]
    Client(#[from] rumqttc::ClientError),
}

pub struct Device {
    state_topic: String,
    client: AsyncClient,
}

impl Device {
    pub async fn update_state<P>(&self, payload: P) -> Result<(), ClientError>
    where
        P: Into<Vec<u8>>,
    {
        self.client
            .publish(&self.state_topic, QoS::AtLeastOnce, false, payload)
            .await?;

        Ok(())
    }
}

pub struct Client {
    discovery_topic: String,
    client: AsyncClient,
}

impl Client {
    pub async fn announce(
        &self,
        component: Component,
        device_class: DeviceClass,
        object_id: &str,
        node_id: Option<&str>,
        device: crate::model::Device,
    ) -> Result<Device, ClientError> {
        let base_topic = format!(
            "{base}/{component}/{node_id}{node_id_slash}{object_id}",
            base = self.discovery_topic,
            node_id = node_id.unwrap_or(""),
            node_id_slash = if node_id.is_some() { "/" } else { "" }
        );

        let state_topic = format!("{base_topic}/state");

        self.client
            .publish(
                format!("{base_topic}/config"),
                QoS::AtLeastOnce,
                false,
                serde_json::to_vec(&Discovery {
                    name: None,
                    state_topic: state_topic.clone(),
                    unique_id: Some(object_id.to_string()),
                    device,
                    device_class,
                })?,
            )
            .await?;

        Ok(Device {
            client: self.client.clone(),
            state_topic,
        })
    }
}

impl<F, H> Connector<F, H>
where
    F: FnOnce(Client) -> H,
    H: ConnectorHandler,
{
    pub fn new(options: ConnectorOptions, handler: F) -> Self {
        Self { options, handler }
    }

    pub async fn run(self) -> Result<(), Error<H::Error>> {
        let base = self
            .options
            .topic_base
            .unwrap_or_else(|| "homeassistant".to_string());

        let client_id = self.options.client_id.unwrap_or_else(random_client_id);

        let port = self
            .options
            .port
            .unwrap_or(if self.options.disable_tls { 1883 } else { 8883 });

        let mut mqttoptions = MqttOptions::new(client_id, self.options.host, port);
        mqttoptions.set_keep_alive(self.options.keep_alive.into());

        if !self.options.disable_tls {
            mqttoptions.set_transport(Transport::Tls(TlsConfiguration::Native));
        }

        log::debug!("Options: {mqttoptions:#?}");

        if let Some(username) = self.options.username {
            mqttoptions.set_credentials(username, self.options.password.unwrap_or_default());
        }

        let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

        let mut handler = (self.handler)(Client {
            discovery_topic: base.clone(),
            client: client.clone(),
        });

        let runner = async {
            loop {
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        };

        let connection = async {
            loop {
                match eventloop.poll().await {
                    Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                        log::info!("Connected");
                        if let Err(err) =
                            client.try_subscribe(format!("{base}/status"), QoS::AtLeastOnce)
                        {
                            log::warn!("Failed to subscribe to status the topic: {err}");
                            if let Err(err) = client.try_disconnect() {
                                panic!("Failed to disconnect after error: {err}");
                            }
                        }
                        handler.connected(true).await.map_err(Error::Handler)?;
                    }
                    Ok(Event::Incoming(Incoming::Disconnect)) => {
                        log::info!("Disconnected");
                        handler.connected(false).await.map_err(Error::Handler)?;
                    }
                    Ok(Event::Incoming(Incoming::Publish(publish))) => {
                        log::info!("Received: {publish:?}");
                        if let Some(topic) = publish.topic.strip_prefix(&base) {
                            match topic {
                                "/status" => {
                                    let payload = String::from_utf8_lossy(&publish.payload);
                                    log::info!("Payload: {}", payload);
                                    if payload == "online" {
                                        handler.restarted().await.map_err(Error::Handler)?;
                                    }
                                }
                                _ => {
                                    log::info!("Skipping unknown topic: {base}{topic}");
                                }
                            }
                        }
                    }
                    Ok(_) => {}
                    Err(err) => {
                        log::warn!("Connection failed: {err}");
                        handler.connected(false).await.map_err(Error::Handler)?;
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
            #[allow(unreachable_code)]
            Ok(())
        };

        select! {
            _ = runner => {},
            ret = connection => { ret? },
        }

        log::info!("MQTT runner exited");

        Ok(())
    }
}
