mod error;
mod options;

use crate::connector::Error;
use crate::model::{Component, Discovery};
use bytes::Bytes;
pub use error::*;
pub use options::*;
use rand::{distributions::Alphanumeric, Rng};
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS, TlsConfiguration, Transport};
use std::borrow::Cow;
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

#[derive(Clone, Debug)]
pub struct DeviceId {
    pub id: Cow<'static, str>,
    pub component: Component,
    pub node_id: Option<Cow<'static, str>>,
}

impl DeviceId {
    pub fn new(id: impl Into<Cow<'static, str>>, component: Component) -> Self {
        Self {
            id: id.into(),
            component,
            node_id: None,
        }
    }
    pub fn with_node_id<I, C>(
        id: impl Into<Cow<'static, str>>,
        component: Component,
        node_id: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            id: id.into(),
            component,
            node_id: Some(node_id.into()),
        }
    }

    /// render the config topic
    pub fn config_topic(&self) -> String {
        format!(
            "{component}/{node_id}{node_id_slash}{object_id}/config",
            component = self.component,
            object_id = self.id,
            node_id_slash = if self.node_id.is_some() { "/" } else { "" },
            node_id = self.node_id.as_deref().unwrap_or(""),
        )
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
        topic: impl Into<String>,
        payload: impl Into<Vec<u8>>,
    ) -> Result<(), ClientError> {
        let topic = topic.into();
        log::info!("Update state on {topic}");

        self.client
            .publish(topic, QoS::AtLeastOnce, false, payload.into())
            .await?;

        Ok(())
    }

    pub async fn announce(&self, id: &DeviceId, discovery: &Discovery) -> Result<(), ClientError> {
        let topic = format!("{}/{}", self.base_topic, id.config_topic());
        log::info!("announce {id} on {topic}: {discovery:?}", id = id.id);

        self.client
            .publish(
                topic,
                QoS::AtLeastOnce,
                false,
                serde_json::to_vec(discovery)?,
            )
            .await?;

        Ok(())
    }

    pub async fn subscribe(&self, topic: impl Into<String>, qos: QoS) -> Result<(), ClientError> {
        let topic = topic.into();
        log::info!("Subscribing to: {topic}");
        self.client.subscribe(topic, qos).await?;

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
