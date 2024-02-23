//! A raw connector example

use clap::Parser;
use homeassistant_agent::connector::{Client, Connector, ConnectorHandler, ConnectorOptions};
use std::convert::Infallible;
use std::future::Future;
use tokio::sync::mpsc;

#[derive(Debug, clap::Parser)]
struct Cli {
    #[command(flatten)]
    connector: ConnectorOptions,
}

#[derive(Clone, Debug)]
pub enum Event {
    Connection { state: bool },
    Restarted,
    Message { topic: String, payload: Vec<u8> },
}

fn event_based<F, Fut, E>(connection: Client, handler: F) -> impl ConnectorHandler
where
    F: FnOnce(mpsc::Receiver<Event>, Client) -> Fut,
    Fut: Future<Output = Result<(), E>> + Send + 'static,
    E: Send + 'static,
{
    let (tx, rx) = mpsc::channel(8);

    tokio::spawn(handler(rx, connection));

    EventBasedHandler { tx }
}

struct EventBasedHandler {
    tx: mpsc::Sender<Event>,
}

impl ConnectorHandler for EventBasedHandler {
    type Error = mpsc::error::SendError<Event>;

    async fn connected(&mut self, state: bool) -> Result<(), Self::Error> {
        self.tx.send(Event::Connection { state }).await
    }

    async fn restarted(&mut self) -> Result<(), Self::Error> {
        self.tx.send(Event::Restarted).await
    }

    fn message(
        &mut self,
        topic: String,
        payload: Vec<u8>,
    ) -> impl Future<Output = Result<(), Self::Error>> {
        self.tx.send(Event::Message { topic, payload })
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    log::info!("Starting up example");

    let connector = Connector::new(cli.connector, |connection| {
        event_based(connection, |mut events, _connection| async move {
            while let Some(event) = events.recv().await {
                log::info!("Event: {event:?}");
            }
            log::info!("Exiting connection loop");
            Ok::<_, Infallible>(())
        })
    });
    connector.run().await?;

    log::info!("Exiting");

    Ok(())
}
