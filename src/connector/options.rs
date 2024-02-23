use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[cfg_attr(feature = "clap", derive(clap::Parser))]
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
    pub keep_alive: Duration,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "clap", arg(long, env))]
    pub username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "clap", arg(long, env))]
    pub password: Option<String>,
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
