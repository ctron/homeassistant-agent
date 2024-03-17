use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[cfg_attr(feature = "clap", derive(clap::Parser))]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ConnectorOptions {
    /// The MQTT client id, defaults to a random ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "clap", arg(long, env))]
    pub client_id: Option<String>,

    /// Base topic, defaults to `homeassistant`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "clap", arg(long, env))]
    pub topic_base: Option<String>,

    /// The MQTT's servers/brokers hostname
    /// #[cfg_attr(feature = "clap", arg(long, env))]
    #[cfg_attr(feature = "clap", arg(long, env))]
    pub host: String,

    /// The MQTT's server/brokers port, defaults to 1883 without TLS and 8883 with TLS
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "clap", arg(long, env))]
    pub port: Option<u16>,

    /// TLS is used by default, you can disable it here.
    #[serde(default, skip_serializing_if = "is_default")]
    #[cfg_attr(feature = "clap", arg(long, env))]
    pub disable_tls: bool,

    #[serde(default = "default_keep_alive", skip_serializing_if = "is_default")]
    #[serde(with = "humantime_serde")]
    #[cfg_attr(feature = "clap", arg(long, env, value_parser = DurationValueParser, default_value = "5s"))]
    #[cfg_attr(feature = "schemars", schemars(schema_with = "humantime_duration"))]
    pub keep_alive: Duration,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "clap", arg(long, env))]
    pub username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "clap", arg(long, env))]
    pub password: Option<String>,
}

#[cfg(feature = "schemars")]
fn humantime_duration(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
    use schemars::schema::*;
    use schemars::JsonSchema;
    use serde_json::json;

    let mut schema: SchemaObject = <String>::json_schema(gen).into();
    schema.metadata = Some(Box::new(Metadata {
        id: None,
        title: None,
        description: Some(r#"A duration in the humantime format. For example: '30s' for 30 seconds. '5m' for 5 minutes."#.to_string()),
        default: None,
        deprecated: false,
        read_only: false,
        write_only: false,
        examples: vec![json!("30s"), json!("1m")],
    }));
    schema.into()
}

#[cfg(feature = "clap")]
#[derive(Clone)]
pub struct DurationValueParser;

#[cfg(feature = "clap")]
impl clap::builder::TypedValueParser for DurationValueParser {
    type Value = Duration;

    fn parse_ref(
        &self,
        cmd: &clap::Command,
        _arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        use std::str::FromStr;
        Ok(humantime::Duration::from_str(&value.to_string_lossy())
            .map_err(|_err| clap::Error::new(clap::error::ErrorKind::Format).with_cmd(cmd))?
            .into())
    }
}

fn default_keep_alive() -> Duration {
    Duration::from_secs(5)
}

fn is_default<D: Default + Eq>(value: &D) -> bool {
    value == &D::default()
}

pub struct ConnectionOptions {
    pub host: String,
    pub port: Option<u16>,
    pub client_id: Option<String>,

    pub username: Option<String>,
    pub password: Option<String>,

    pub topic_base: Option<String>,

    pub disable_tls: bool,

    pub keep_alive: Duration,
}
