use crate::model::Device;

/// Discovery message
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
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

    #[serde(default)]
    pub state_class: Option<StateClass>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_topic: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_topic: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit_of_measurement: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_template: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
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
