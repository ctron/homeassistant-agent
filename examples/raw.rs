//! A more elaborate device example.
//!
//! The idea of this example is to research what common code exists, so that it can be added
//! to the main crate.

use bytes::Bytes;
use clap::Parser;
use homeassistant_agent::{
    connector::{Client, ClientError, Connector, ConnectorHandler, ConnectorOptions},
    model::{BinarySensorClass, Component, Device, DeviceId, Discovery},
};
use rumqttc::QoS;
use std::time::Duration;
use tokio::{
    select,
    sync::{mpsc, oneshot},
    time::MissedTickBehavior,
};

#[derive(Debug, clap::Parser)]
struct Cli {
    #[command(flatten)]
    connector: ConnectorOptions,
}

#[derive(Clone, Debug)]
pub enum Event {
    Switch(bool),
}

struct CustomDevice {
    client: Client,

    events: mpsc::Sender<Event>,

    motion: MotionDevice,
    switch: SwitchDevice,
}

#[derive(Clone, Debug)]
struct MotionDevice {
    id: DeviceId,
    discovery: Discovery,

    state_topic: String,
}

impl MotionDevice {
    pub fn new(
        base: &str,
        device: Device,
        id: DeviceId,
        device_class: Option<BinarySensorClass>,
    ) -> Self {
        let state_topic = format!("{base}/{id}/state", id = id.id);
        let discovery = Discovery {
            unique_id: Some(id.id.to_string()),
            device: Some(device),
            device_class: device_class.map(|c| c.as_ref().to_string()),
            state_topic: Some(state_topic.clone()),
            ..Default::default()
        };
        Self {
            id,
            state_topic,
            discovery,
        }
    }
}

#[derive(Clone, Debug)]
struct SwitchDevice {
    id: DeviceId,
    discovery: Discovery,

    command_topic: String,
    state_topic: String,
}

impl SwitchDevice {
    pub fn new(base: &str, device: Device, id: DeviceId) -> Self {
        let command_topic = format!("{base}/{id}/command", id = id.id);
        let state_topic = format!("{base}/{id}/state", id = id.id);
        let discovery = Discovery {
            unique_id: Some(id.id.to_string()),
            device: Some(device),
            command_topic: Some(command_topic.clone()),
            state_topic: Some(state_topic.clone()),
            ..Default::default()
        };
        Self {
            id,
            command_topic,
            state_topic,
            discovery,
        }
    }
}

impl CustomDevice {
    pub fn new(client: Client) -> Self {
        let device = Device {
            name: Some("Test Device 1".to_string()),
            base_topic: None,
            identifiers: vec!["test-id1".to_string()],
            support_url: None,
            sw_version: None,
        };

        let motion = MotionDevice::new(
            "my-base",
            device.clone(),
            DeviceId::new("test-id10-motion", Component::BinarySensor),
            Some(BinarySensorClass::Motion),
        );
        let switch = SwitchDevice::new(
            "my-base",
            device,
            DeviceId::new("test-id10-switch", Component::Switch),
        );

        let (tx, mut rx) = mpsc::channel(8);

        {
            let switch = switch.clone();
            let motion = motion.clone();
            let client = client.clone();

            tokio::spawn(async move {
                let mut runner = None::<oneshot::Sender<()>>;

                let _ = client.update_state(&switch.state_topic, "OFF").await;

                while let Some(event) = rx.recv().await {
                    match event {
                        Event::Switch(command) => {
                            log::info!("Switch toggled ({command})");

                            let _ = client
                                .update_state(
                                    &switch.state_topic,
                                    if command { "ON" } else { "OFF" },
                                )
                                .await;

                            runner = match (command, runner) {
                                (true, Some(runner)) => Some(runner),
                                (false, Some(runner)) => {
                                    // terminate
                                    let _ = runner.send(());
                                    None
                                }
                                (true, None) => Some({
                                    let id = motion.clone();
                                    let client = client.clone();
                                    let (tx, mut rx) = oneshot::channel::<()>();
                                    tokio::spawn(async move {
                                        let _ = client.update_state(&id.state_topic, "ON").await;

                                        let mut interval =
                                            tokio::time::interval(Duration::from_secs(5));
                                        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
                                        let mut state = false;
                                        loop {
                                            select! {
                                                _ = interval.tick() => {
                                                        let _ = client.update_state(&id.state_topic, if state { "ON" } else { "OFF" }).await;
                                                        state = !state;
                                                    }
                                                _ = &mut rx => {
                                                    break;
                                                }
                                            }
                                        }
                                    });

                                    tx
                                }),
                                (false, None) => None,
                            };
                        }
                    }
                }
            });
        }

        Self {
            motion,
            switch,
            client,
            events: tx,
        }
    }

    async fn subscribe(&self) -> Result<(), ClientError> {
        self.client
            .subscribe(&self.switch.command_topic, QoS::AtLeastOnce)
            .await
    }

    async fn announce(&self) -> Result<(), ClientError> {
        self.client
            .announce(&self.motion.id, &self.motion.discovery)
            .await?;
        self.client
            .announce(&self.switch.id, &self.switch.discovery)
            .await?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CustomDeviceError {
    #[error(transparent)]
    Client(#[from] ClientError),
    #[error("unable to handle message")]
    Message,
}

impl ConnectorHandler for CustomDevice {
    type Error = CustomDeviceError;

    async fn connected(&mut self, state: bool) -> Result<(), Self::Error> {
        log::info!("Connected: {state}");
        if state {
            self.subscribe().await?;
            self.announce().await?;
        }
        Ok(())
    }

    async fn restarted(&mut self) -> Result<(), Self::Error> {
        log::info!("Restarted");
        self.announce().await?;
        Ok(())
    }

    async fn message(&mut self, topic: String, payload: Bytes) -> Result<(), Self::Error> {
        log::info!(
            "message - topic: {topic}, payload: {:?}",
            String::from_utf8_lossy(&payload)
        );

        log::debug!("Switch command: {:?}", self.switch.discovery.command_topic);

        if self.switch.discovery.command_topic == Some(topic) {
            let state = payload == b"ON".as_slice();
            let command = Event::Switch(state);
            log::info!("Dispatching command: {command:?}");
            self.events
                .send(command)
                .await
                .map_err(|_| CustomDeviceError::Message)?;
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    log::info!("Starting up example");

    let connector = Connector::new(cli.connector, CustomDevice::new);
    connector.run().await?;

    log::info!("Exiting");

    Ok(())
}
