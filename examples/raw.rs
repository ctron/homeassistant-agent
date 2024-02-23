//! A raw connector example

use bytes::Bytes;
use clap::Parser;
use homeassistant_agent::connector::{
    Client, ClientError, Connector, ConnectorHandler, ConnectorOptions, DeviceId,
};
use homeassistant_agent::model::{BinarySensorClass, Component, Device};
use rumqttc::QoS;
use std::time::Duration;
use tokio::select;
use tokio::sync::{mpsc, oneshot};
use tokio::time::MissedTickBehavior;

#[derive(Debug, clap::Parser)]
struct Cli {
    #[command(flatten)]
    connector: ConnectorOptions,
}

pub enum Event {
    Switch(bool),
}

struct CustomDevice {
    client: Client,

    events: mpsc::Sender<Event>,

    motion: DeviceId,
    switch: DeviceId,
}

impl CustomDevice {
    pub fn new(client: Client) -> Self {
        let motion = DeviceId::new(
            "test-id10-motion",
            Component::BinarySensor(Some(BinarySensorClass::Motion)),
            None,
        );
        let switch = DeviceId::new("test-id10-switch", Component::Switch(None), None);

        let (tx, mut rx) = mpsc::channel(8);

        {
            let switch = switch.clone();
            let motion = motion.clone();
            let client = client.clone();

            tokio::spawn(async move {
                let mut runner = None::<oneshot::Sender<()>>;

                let _ = client.update_state(&switch, "OFF").await;

                while let Some(event) = rx.recv().await {
                    match event {
                        Event::Switch(command) => {
                            log::info!("Switch toggled ({command})");

                            let _ = client
                                .update_state(&switch, if command { "ON" } else { "OFF" })
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
                                        let _ = client.update_state(&id, "ON").await;

                                        let mut interval =
                                            tokio::time::interval(Duration::from_secs(5));
                                        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
                                        let mut state = false;
                                        loop {
                                            select! {
                                                _ = interval.tick() => {
                                                        let _ = client.update_state(&id, if state { "ON" } else { "OFF" }).await;
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
        self.client.subscribe(&self.switch, QoS::AtLeastOnce).await
    }

    async fn announce(&self) -> Result<(), ClientError> {
        let device = Device {
            name: "Test Device 1".to_string(),
            base_topic: None,
            identifiers: vec!["test-id1".to_string()],
            support_url: None,
            sw_version: None,
        };

        self.client
            .announce(&self.motion, Some(BinarySensorClass::Motion), &device)
            .await?;
        self.client
            .announce(&self.switch, None::<String>, &device)
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

        if self.switch.command_topic == Some(topic) {
            let state = payload == b"ON".as_slice();
            self.events
                .send(Event::Switch(state))
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
