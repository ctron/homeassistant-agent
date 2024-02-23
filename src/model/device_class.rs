use std::fmt::Debug;

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
