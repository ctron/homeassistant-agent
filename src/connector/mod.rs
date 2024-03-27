mod client;
mod error;
mod options;

pub use client::*;
pub use error::*;
pub use options::*;

use crate::connector::Error;
use bytes::Bytes;
use rand::{distributions::Alphanumeric, Rng};
use rumqttc::{
    AsyncClient, Event, Incoming, LastWill, MqttOptions, QoS, TlsConfiguration, Transport,
};
use std::{future::Future, time::Duration};

fn random_client_id() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(23)
        .map(char::from)
        .collect()
}

pub trait ConnectorHandler {
    type Error: std::error::Error + Send + Sync;

    /// called when the connection state changes.
    ///
    /// NOTE: it may be that this method gets called with the same state multiple times.
    fn connected(&mut self, state: bool) -> impl Future<Output = Result<(), Self::Error>>;

    /// Called then a restart of Home Assistant has been detected
    ///
    /// When Home Assistant is restarted, it is necessary to re-announce devices.
    fn restarted(&mut self) -> impl Future<Output = Result<(), Self::Error>>;

    /// A message received on a topic.
    ///
    /// You will only receive messages if you first subscribed to one or more topics.
    fn message(
        &mut self,
        topic: String,
        payload: Bytes,
    ) -> impl Future<Output = Result<(), Self::Error>>;
}

#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct AvailabilityOptions {
    pub topic: String,
}

impl AvailabilityOptions {
    pub fn new(topic: impl Into<String>) -> Self {
        Self {
            topic: topic.into(),
        }
    }
}

pub struct Connector<F, H>
where
    F: FnOnce(Client) -> H,
    H: ConnectorHandler,
{
    options: ConnectorOptions,
    handler: F,
    availability: Option<AvailabilityOptions>,
}

impl<F, H> Connector<F, H>
where
    F: FnOnce(Client) -> H,
    H: ConnectorHandler,
{
    pub fn new(options: ConnectorOptions, handler: F) -> Self {
        Self {
            options,
            handler,
            availability: None,
        }
    }

    pub fn availability(mut self, availability: impl Into<AvailabilityOptions>) -> Self {
        self.availability = Some(availability.into());
        self
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

        if let Some(availability) = &self.availability {
            log::info!("Using availability topic on: {}", availability.topic);
            mqttoptions.set_last_will(LastWill {
                topic: availability.topic.clone(),
                message: Bytes::from_static(b"offline"),
                qos: QoS::AtLeastOnce,
                retain: true,
            });
        }

        let (client, mut eventloop) = AsyncClient::new(mqttoptions, 128);

        let mut handler = (self.handler)(Client {
            base_topic: base.clone(),
            mqtt: client.clone(),
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

                    if let Some(availability) = &self.availability {
                        if let Err(err) = client.try_publish(
                            &availability.topic,
                            QoS::AtLeastOnce,
                            true,
                            b"online",
                        ) {
                            log::warn!("Failed to announce availability: {err}");
                            if let Err(err) = client.try_disconnect() {
                                panic!("Failed to disconnect after error: {err}");
                            }
                        }
                    }
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
