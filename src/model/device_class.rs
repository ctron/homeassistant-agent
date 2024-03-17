use std::fmt::Debug;

// for values see: https://github.com/home-assistant/core/blob/dev/homeassistant/components/number/const.py

#[derive(Copy, Clone, Eq, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub enum ButtonClass {
    Identify,
    Restart,
    Update,
}

impl AsRef<str> for ButtonClass {
    fn as_ref(&self) -> &str {
        match self {
            Self::Identify => "identify",
            Self::Restart => "restart",
            Self::Update => "update",
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub enum BinarySensorClass {
    Motion,
}

impl AsRef<str> for BinarySensorClass {
    fn as_ref(&self) -> &str {
        match self {
            Self::Motion => "motion",
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub enum SwitchClass {
    Outlet,
    Switch,
}

impl AsRef<str> for SwitchClass {
    fn as_ref(&self) -> &str {
        match self {
            Self::Outlet => "outlet",
            Self::Switch => "switch",
        }
    }
}

#[derive(
    Copy, Clone, Eq, PartialEq, Debug, strum::AsRefStr, strum::EnumString, strum::VariantNames,
)]
#[strum(serialize_all = "snake_case")]
pub enum SensorClass {
    ApparentPower,
    Aqi,
    AtmosphericPressure,
    Battery,
    #[strum(to_string = "carbon_dioxide")]
    Co2,
    #[strum(to_string = "carbon_monoxide")]
    Co,
    Current,
    DataRate,
    DataSize,
    Date,
    Distance,
    Duration,
    Energy,
    EnergyStorage,
    Enum,
    Frequency,
    Gas,
    Humidity,
    Illuminance,
    Irradiance,
    Moisture,
    Monetary,
    NitrogenDioxide,
    NitrogenMonoxide,
    NitrousOxide,
    Ozone,
    Ph,
    Pm1,
    Pm25,
    Pm10,
    Power,
    PowerFactor,
    Precipitation,
    PrecipitationDensity,
    Pressure,
    ReactivePower,
    SignalStrength,
    SoundPressure,
    Speed,
    SulphurDioxide,
    Temperature,
    Timestamp,
    VolatileOrganicCompounds,
    VolatileOrganicCompoundsParst,
    Voltage,
    Volume,
    VolumeFlowRate,
    VolumeStorage,
    Water,
    Weight,
    WindSpeed,
}
