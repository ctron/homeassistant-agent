use crate::{model::Device, utils::is_default};

// also see: https://developers.home-assistant.io/docs/core/entity/

/// Discovery message
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct Discovery {
    /// The name of the application that is the origin the discovered MQTT item. This option is required.
    // Don't skip serde if it's empty, as it has to be null then
    #[serde(default)]
    pub name: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unique_id: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device: Option<Device>,

    /// The device class. Should be `null` if omitted, so don't skip.
    #[serde(default)]
    pub device_class: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_class: Option<StateClass>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_topic: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_template: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_topic: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit_of_measurement: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_template: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled_by_default: Option<bool>,

    #[serde(default, skip_serializing_if = "is_default")]
    pub availability_mode: AvailabilityMode,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub availability: Vec<Availability>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct Availability {
    pub topic: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload_available: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload_not_available: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_template: Option<String>,
}

impl Availability {
    pub fn new(topic: impl Into<String>) -> Self {
        Self {
            topic: topic.into(),
            payload_available: None,
            payload_not_available: None,
            value_template: None,
        }
    }

    pub fn payload_available(mut self, payload: impl Into<String>) -> Self {
        self.payload_available = Some(payload.into());
        self
    }

    pub fn payload_not_available(mut self, payload: impl Into<String>) -> Self {
        self.payload_not_available = Some(payload.into());
        self
    }

    pub fn value_template(mut self, value_template: impl Into<String>) -> Self {
        self.value_template = Some(value_template.into());
        self
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum AvailabilityMode {
    All,
    Any,
    #[default]
    Latest,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum StateClass {
    Measurement,
    Total,
    TotalIncreasing,
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_serde() {
        assert_eq!(
            serde_json::to_value(Discovery {
                device: Some(Device {
                    identifiers: vec!["test-id1".into()],
                    name: Some("Test Device 1".to_string()),
                    base_topic: None,
                    sw_version: None,
                    support_url: None,
                }),
                device_class: Some("motion".to_string()),
                state_topic: Some("some/topic".to_string()),
                ..Default::default()
            })
            .unwrap(),
            json!({
                "name": null,
                "device_class": "motion",
                "device": {
                    "identifiers" : [
                        "test-id1"
                    ],
                    "name": "Test Device 1",
                },
                "state_topic": "some/topic"
            })
        )
    }
}
