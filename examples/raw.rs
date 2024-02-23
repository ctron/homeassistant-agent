//! A raw connector example

use clap::Parser;
use homeassistant_agent::connector::{
    Client, ClientError, Connector, ConnectorHandler, ConnectorOptions, DeviceId,
};
use homeassistant_agent::model::{BinarySensorClass, Component, Device};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::select;
use tokio::time::MissedTickBehavior;

#[derive(Debug, clap::Parser)]
struct Cli {
    #[command(flatten)]
    connector: ConnectorOptions,
}

struct CustomDevice {
    client: Client,
    motion: DeviceId,
}

impl CustomDevice {
    pub fn new(client: Client) -> Self {
        let motion = DeviceId::new(
            "test-id10-motion",
            Component::BinarySensor(Some(BinarySensorClass::Motion)),
            None,
        );
        let switch = DeviceId::new("test-id10-switch", Component::Button(None), None);

        let state = Arc::new(AtomicBool::new(true));

        {
            let id = switch.clone();
            let client = client.clone();
            let state = state.clone();

            tokio::spawn(async move {
                let command = client.subscribe(&id);

                let _ = client
                    .update_state(
                        &id,
                        if state.load(Ordering::SeqCst) {
                            "ON"
                        } else {
                            "OFF"
                        },
                    )
                    .await;

                loop {
                    select! {
                        cmd = command.recv() => {
                            let state = !state.fetch_xor(true, Ordering::SeqCst);
                            let _ = client.update_state(&id, if state { "ON"} else {"OFF"}).await;
                        }
                    }
                }
            });
        }

        {
            let id = motion.clone();
            let client = client.clone();
            tokio::spawn(async move {
                let _ = client.update_state(&id, "ON").await;

                let mut interval = tokio::time::interval(Duration::from_secs(5));
                interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
                let mut state = false;
                loop {
                    interval.tick().await;
                    let _ = client
                        .update_state(&id, if state { "ON" } else { "OFF" })
                        .await;
                    state = !state;
                }
            });
        }

        Self { motion, client }
    }

    async fn announce(&self) -> Result<(), ClientError> {
        self.client
            .announce(
                &self.motion,
                BinarySensorClass::Motion.into(),
                Device {
                    name: "Test Device 1".to_string(),
                    base_topic: None,
                    identifiers: vec!["test-id1".to_string()],
                    support_url: None,
                    sw_version: None,
                },
            )
            .await?;
        Ok(())
    }
}

impl ConnectorHandler for CustomDevice {
    type Error = ClientError;

    async fn connected(&mut self, state: bool) -> Result<(), Self::Error> {
        log::info!("Connected: {state}");
        if state {
            self.announce().await?;
        }
        Ok(())
    }

    async fn restarted(&mut self) -> Result<(), Self::Error> {
        log::info!("Restarted");
        self.announce().await?;
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
