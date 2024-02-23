mod error;
mod options;

use crate::connector::Error;
use crate::model::{Component, Discovery};
use bytes::Bytes;
pub use error::*;
pub use options::*;
use rand::{distributions::Alphanumeric, Rng};
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS, TlsConfiguration, Transport};
use std::future::Future;
use std::time::Duration;

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
    fn message(
        &mut self,
        topic: String,
        payload: Bytes,
    ) -> impl Future<Output = Result<(), Self::Error>>;
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

    pub state_topic: Option<String>,
    pub command_topic: Option<String>,
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

        // FIXME: depends on the support of the devices
        let state_topic = Some(format!("{topic}/state"));
        let command_topic = Some(format!("{topic}/set"));

        Self {
            id,
            component,
            node_id,
            config_topic,
            state_topic,
            command_topic,
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
        if let Some(topic) = &id.state_topic {
            self.client
                .publish(
                    format!("{}/{topic}", self.base_topic),
                    QoS::AtLeastOnce,
                    false,
                    payload.into(),
                )
                .await?;
        }
        Ok(())
    }

    pub async fn announce(
        &self,
        id: &DeviceId,
        device_class: Option<impl AsRef<str>>,
        device: &crate::model::Device,
    ) -> Result<(), ClientError> {
        let state_topic = id
            .state_topic
            .as_ref()
            .map(|topic| format!("{}/{topic}", self.base_topic));
        let command_topic = id
            .command_topic
            .as_ref()
            .map(|topic| format!("{}/{topic}", self.base_topic));

        self.client
            .publish(
                format!("{}/{}", self.base_topic, id.config_topic),
                QoS::AtLeastOnce,
                false,
                serde_json::to_vec(&Discovery {
                    name: None,
                    state_topic,
                    command_topic,
                    unique_id: Some(id.id.to_string()),
                    device,
                    device_class: device_class.map(|s| s.as_ref().to_string()),
                })?,
            )
            .await?;

        Ok(())
    }

    pub async fn subscribe(&self, id: &DeviceId, qos: QoS) -> Result<(), ClientError> {
        if let Some(topic) = &id.command_topic {
            self.client
                .subscribe(format!("{}/{topic}", self.base_topic), qos)
                .await?;
        }

        // FIXME: we should not allow subscribe on a non-command type device

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

        let status_topic = format!("{base}/status");

        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                    log::info!("Connected");
                    if let Err(err) = client.try_subscribe(&status_topic, QoS::AtLeastOnce) {
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
                    if publish.topic == status_topic {
                        let payload = String::from_utf8_lossy(&publish.payload);
                        log::info!("Payload: {}", payload);
                        if payload == "online" {
                            handler.restarted().await.map_err(Error::Handler)?;
                        }
                    } else {
                        let topic = publish.topic;
                        let payload = publish.payload;
                        log::info!("Message published: {topic} (len: {})", payload.len());

                        // FIXME: there's actually no requirement to have the device topics aligned with the base prefix
                        if let Some(topic) = topic.strip_prefix(&format!("{base}/")) {
                            if let Err(err) = handler.message(topic.to_string(), payload).await {
                                if publish.qos != QoS::AtMostOnce {
                                    // we can't ignore this
                                    log::warn!("Failed to process message: {err}");
                                    if let Err(err) = client.try_disconnect() {
                                        panic!("Failed to disconnect after error: {err}");
                                    }
                                } else {
                                    log::info!(
                                        "Failed to process message: {err} … ignoring due to QoS"
                                    );
                                }
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
    }
}
