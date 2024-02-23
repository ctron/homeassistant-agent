use crate::model::Device;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Discovery<'a> {
    /// The name of the application that is the origin the discovered MQTT item. This option is required.
    // Don't skip serde if it's empty, as it has to be null then
    #[serde(default)]
    pub name: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unique_id: Option<String>,

    pub device: &'a Device,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_class: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_topic: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_topic: Option<String>,
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
                device: &Device {
                    identifiers: vec!["test-id1".into()],
                    name: "Test Device 1".to_string(),
                    base_topic: None,
                    sw_version: None,
                    support_url: None,
                },
                device_class: Some("motion".to_string()),
                state_topic: Some("some/topic".to_string()),
                command_topic: None,
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
