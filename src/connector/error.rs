#[derive(Debug, thiserror::Error)]
pub enum Error<H> {
    #[error(transparent)]
    Handler(H),
    #[error("MQTT client error: {0}")]
    Client(#[from] rumqttc::v5::ClientError),
}
