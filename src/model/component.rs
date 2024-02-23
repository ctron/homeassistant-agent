use crate::model::*;
use std::fmt::Formatter;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Component {
    Button(Option<ButtonClass>),
    Switch(Option<SwitchClass>),
    BinarySensor(Option<BinarySensorClass>),
    Sensor,
}

impl AsRef<str> for Component {
    fn as_ref(&self) -> &str {
        match self {
            Self::BinarySensor(_) => "binary_sensor",
            Self::Button(_) => "button",
            Self::Sensor => "sensor",
            Self::Switch(_) => "switch",
        }
    }
}

impl std::fmt::Display for Component {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
