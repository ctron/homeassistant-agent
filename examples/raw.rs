//! A raw connector example

use clap::Parser;
use homeassistant_agent::connector::{
    Client, ClientError, Component, Connector, ConnectorHandler, ConnectorOptions,
};
use homeassistant_agent::model::{Device, DeviceClass};
use std::time::Duration;
use tokio::time::MissedTickBehavior;

#[derive(Debug, clap::Parser)]
struct Cli {
    #[command(flatten)]
    connector: ConnectorOptions,
}

struct CustomDevice {
    connection: Client,
}

impl CustomDevice {
    pub fn new(connection: Client) -> Self {
        Self { connection }
    }

    async fn announce(&self) -> Result<(), ClientError> {
        let sensor = self
            .connection
            .announce(
                Component::BinarySensor,
                DeviceClass::Motion,
                "test-id1",
                None,
                Device {
                    name: "Test Device 1".to_string(),
                    base_topic: None,
                    identifiers: vec!["test-id1".to_string()],
                    support_url: None,
                    sw_version: None,
                },
            )
            .await?;

        sensor.update_state("ON").await?;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
            let mut state = false;
            loop {
                interval.tick().await;
                let _ = sensor.update_state(if state { "ON" } else { "OFF" }).await;
                state = !state;
            }
        });

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
