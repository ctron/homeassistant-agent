use std::fmt::Formatter;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Component {
    Button,
    Switch,
    BinarySensor,
    Sensor,
}

impl AsRef<str> for Component {
    fn as_ref(&self) -> &str {
        match self {
            Self::BinarySensor => "binary_sensor",
            Self::Button => "button",
            Self::Sensor => "sensor",
            Self::Switch => "switch",
        }
    }
}

impl std::fmt::Display for Component {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
