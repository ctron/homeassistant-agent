use crate::model::Device;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Discovery {
    /// The name of the application that is the origin the discovered MQTT item. This option is required.
    #[serde(default)]
    pub name: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unique_id: Option<String>,

    pub device: Device,

    pub device_class: Option<String>,

    pub state_topic: String,
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_serde() {
        assert_eq!(
            serde_json::to_value(Discovery {
                name: None,
                unique_id: None,
                device: Device {
                    identifiers: vec!["test-id1".into()],
                    name: "Test Device 1".to_string(),
                    base_topic: None,
                    sw_version: None,
                    support_url: None,
                },
                device_class: Some("motion".to_string()),
                state_topic: "some/topic".to_string(),
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
