//! Stable identifier newtypes.

use serde::{Deserialize, Serialize};

/// Runtime identifier for an attached device (derived from its HID path/serial).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeviceId(pub String);

/// Logical key identifier, stable across physical layouts (e.g. `"KC_ESC"`).
///
/// Drivers map this to a device-specific LED index via the profile's
/// [`crate::capability::LedLayout`].
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyId(pub String);

/// Identifier for a lighting zone (used by zoned, non-per-key boards).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ZoneId(pub String);

/// On-device macro storage slot index.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MacroSlot(pub u8);

impl From<&str> for KeyId {
    fn from(s: &str) -> Self {
        KeyId(s.to_owned())
    }
}

impl From<String> for KeyId {
    fn from(s: String) -> Self {
        KeyId(s)
    }
}

impl From<&str> for DeviceId {
    fn from(s: &str) -> Self {
        DeviceId(s.to_owned())
    }
}

impl From<&str> for ZoneId {
    fn from(s: &str) -> Self {
        ZoneId(s.to_owned())
    }
}
