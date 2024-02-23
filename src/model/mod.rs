mod discovery;

pub use discovery::*;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Device {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub identifiers: Vec<String>,

    pub name: String,

    #[serde(rename = "~")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_topic: Option<String>,

    /// Software version of the application that supplies the discovered MQTT item.
    #[serde(alias = "sw")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sw_version: Option<String>,

    /// Support URL of the application that supplies the discovered MQTT item.
    #[serde(alias = "url")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub support_url: Option<String>,
}
