//! Device-agnostic command intents.
//!
//! These are what the app sends to a [`crate::driver::DeviceSession`]; the driver
//! turns them into device-specific bytes. They are deliberately free of any
//! protocol detail.

use serde::{Deserialize, Serialize};

use crate::color::Color;
use crate::ids::{KeyId, ZoneId};

/// An RGB lighting intent.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RgbCommand {
    /// Set every LED to one color.
    SetAll(Color),
    /// Set specific keys (per-key boards).
    SetKeys(Vec<(KeyId, Color)>),
    /// Set a whole zone (zoned boards).
    SetZone { zone: ZoneId, color: Color },
    /// Set the entire LED buffer at once, in `led_index` order.
    SetFrame(Vec<Color>),
}

/// Select and configure a built-in on-device effect.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EffectSelection {
    pub effect_id: String,
    #[serde(default)]
    pub speed: Option<u8>,
    #[serde(default)]
    pub brightness: Option<u8>,
    #[serde(default)]
    pub colors: Vec<Color>,
    /// Direction/variant for effects that support it (device byte 11; 0 = default).
    #[serde(default)]
    pub direction: Option<u8>,
    /// Randomize the effect's color instead of using `colors` (device byte 8).
    #[serde(default)]
    pub randomize: bool,
    /// Change only the effect's color, without re-selecting it. The board resets
    /// a color effect to its default when re-selected, so once an effect is
    /// running, live color tweaks must send the color packet alone (no select) —
    /// otherwise every tweak resets to the default and the color never sticks.
    #[serde(default)]
    pub color_only: bool,
}

/// A macro program: an ordered list of input events plus a repeat policy.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MacroProgram {
    pub events: Vec<MacroEvent>,
    #[serde(default)]
    pub repeat: MacroRepeat,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum MacroEvent {
    /// HID usage code press.
    KeyDown {
        code: u16,
    },
    KeyUp {
        code: u16,
    },
    /// Pause, in milliseconds.
    Delay {
        ms: u32,
    },
    /// Typed text (expanded to key events by the engine/driver).
    Text {
        text: String,
    },
    MouseButton {
        button: u8,
        down: bool,
    },
    MouseMove {
        dx: i16,
        dy: i16,
    },
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MacroRepeat {
    #[default]
    Once,
    Count(u16),
    UntilKeyPress,
    WhileHeld,
}

/// One LCD frame already encoded in the device's native pixel format.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LcdFrame {
    pub width: u16,
    pub height: u16,
    pub data: Vec<u8>,
}

/// State read back from a device (only on devices that support readback).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DeviceState {
    #[serde(default)]
    pub firmware_version: Option<String>,
    #[serde(default)]
    pub current_profile: Option<String>,
}
