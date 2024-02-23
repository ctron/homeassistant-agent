mod error;
mod options;

use crate::connector::Error;
use crate::model::{Component, Discovery};
pub use error::*;
pub use options::*;
use rand::{distributions::Alphanumeric, Rng};
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS, TlsConfiguration, Transport};
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

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("serialization failure")]
    Serialization(#[from] serde_json::Error),
    #[error("client error")]
    Client(#[from] rumqttc::ClientError),
}

#[derive(Clone)]
pub struct DeviceId {
    pub id: String,
    pub component: Component,
    pub node_id: Option<String>,

    pub config_topic: String,
    pub state_topic: String,
}

impl DeviceId {
    pub fn new(
        id: impl Into<String>,
        component: Component,
        node_id: impl Into<Option<String>>,
    ) -> Self {
        let id = id.into();
        let node_id = node_id.into();

        let topic = format!(
            "{component}/{node_id}{node_id_slash}{object_id}",
            object_id = id,
            node_id_slash = if node_id.is_some() { "/" } else { "" },
            node_id = node_id.as_deref().unwrap_or(""),
        );

        let config_topic = format!("{topic}/config");
        let state_topic = format!("{topic}/state");

        Self {
            id,
            component,
            node_id,
            config_topic,
            state_topic,
        }
    }
}

#[derive(Clone)]
pub struct Client {
    base_topic: String,
    client: AsyncClient,
}

impl Client {
    pub async fn update_state(
        &self,
        id: &DeviceId,
        payload: impl Into<Vec<u8>>,
    ) -> Result<(), ClientError> {
        Ok(self
            .client
            .publish(
                format!("{}/{}", self.base_topic, id.state_topic),
                QoS::AtLeastOnce,
                false,
                payload,
            )
            .await?)
    }

    pub async fn announce(
        &self,
        id: &DeviceId,
        device_class: Option<impl AsRef<str>>,
        device: crate::model::Device,
    ) -> Result<(), ClientError> {
        let state_topic = format!("{}/{}", self.base_topic, id.state_topic);

        self.client
            .publish(
                format!("{}/{}", self.base_topic, id.config_topic),
                QoS::AtLeastOnce,
                false,
                serde_json::to_vec(&Discovery {
                    name: None,
                    state_topic: state_topic.clone(),
                    unique_id: Some(id.id.to_string()),
                    device,
                    device_class: device_class.map(|s| s.as_ref().to_string()),
                })?,
            )
            .await?;

        Ok(())
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
        mqttoptions.set_keep_alive(self.options.keep_alive);

        if !self.options.disable_tls {
            mqttoptions.set_transport(Transport::Tls(TlsConfiguration::Native));
        }

        log::debug!("Options: {mqttoptions:#?}");

        if let Some(username) = self.options.username {
            mqttoptions.set_credentials(username, self.options.password.unwrap_or_default());
        }

        let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

        let mut handler = (self.handler)(Client {
            base_topic: base.clone(),
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
