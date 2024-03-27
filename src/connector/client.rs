use crate::model::{DeviceId, Discovery};
use rumqttc::{AsyncClient, QoS};

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("serialization failure")]
    Serialization(#[from] serde_json::Error),
    #[error("client error")]
    Client(#[from] rumqttc::ClientError),
}

#[derive(Clone)]
pub struct Client {
    pub mqtt: AsyncClient,

    pub(crate) base_topic: String,
}

impl Client {
    pub async fn update_state(
        &self,
        topic: impl Into<String>,
        payload: impl Into<Vec<u8>>,
    ) -> Result<(), ClientError> {
        let topic = topic.into();
        log::info!("Update state on {topic}");

        self.mqtt
            .try_publish(topic, QoS::AtLeastOnce, false, payload.into())
            .inspect_err(|err| {
                log::warn!("failed to publish state: {err}");
            })?;

        Ok(())
    }

    pub async fn announce(&self, id: &DeviceId, discovery: &Discovery) -> Result<(), ClientError> {
        let topic = format!("{}/{}", self.base_topic, id.config_topic());
        log::info!("announce {id} on {topic}: {discovery:?}", id = id.id);

        self.mqtt
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
        self.mqtt.subscribe(topic, qos).await?;

        Ok(())
    }
}
