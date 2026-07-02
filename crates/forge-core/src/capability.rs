//! The capability model — the contract between a device and the UI.
//!
//! The front end never branches on device model. It receives a `Vec<Capability>`
//! and renders the matching controls. A device that advertises
//! `Rgb { mode: PerKey, layout }` gets the per-key canvas editor seeded from
//! `layout`; one that advertises `Lcd { 128x40, Mono1bpp }` gets an LCD designer
//! constrained to that surface.

use serde::{Deserialize, Serialize};

use crate::color::ColorOrder;
use crate::ids::{KeyId, ZoneId};

/// A single, self-describing device feature.
///
/// New feature kinds are added as new variants; older builds that meet an
/// unknown kind deserialize it to [`Capability::Unknown`] and simply render a
/// "not supported in this version" placeholder instead of failing.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Capability {
    Rgb(RgbCapability),
    Macro(MacroCapability),
    Lcd(LcdCapability),
    /// Forward-compatible fallback for capability kinds this build doesn't know.
    #[serde(other)]
    Unknown,
}

// ---------------------------------------------------------------------------
// RGB
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RgbCapability {
    pub mode: RgbMode,
    pub layout: LedLayout,
    #[serde(default)]
    pub effects: Vec<EffectDescriptor>,
    #[serde(default = "default_brightness")]
    pub max_brightness: u8,
    #[serde(default)]
    pub color_order: ColorOrder,
}

fn default_brightness() -> u8 {
    255
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RgbMode {
    /// Every key is independently addressable.
    PerKey,
    /// LEDs are grouped into a fixed set of zones.
    Zoned { zones: Vec<ZoneDef> },
    /// A single color for the whole device.
    SingleColor,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ZoneDef {
    pub id: ZoneId,
    pub label: String,
    pub keys: Vec<KeyId>,
}

/// Physical arrangement of the addressable LEDs, used both for rendering the
/// editor and for mapping a [`KeyId`] to its position in the device LED buffer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LedLayout {
    pub keys: Vec<KeyDef>,
    /// (rows, cols) of the electrical matrix.
    pub matrix_size: (u8, u8),
}

impl LedLayout {
    /// LED buffer index for a key, if it has an LED.
    pub fn led_index_of(&self, key: &KeyId) -> Option<u16> {
        self.keys
            .iter()
            .find(|k| &k.id == key)
            .and_then(|k| k.led_index)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyDef {
    pub id: KeyId,
    pub label: String,
    /// Position in layout grid units (1.0 == one standard key).
    pub x: f32,
    pub y: f32,
    #[serde(default = "default_unit")]
    pub w: f32,
    #[serde(default = "default_unit")]
    pub h: f32,
    /// Index into the device's LED buffer; `None` for keys without an LED.
    #[serde(default)]
    pub led_index: Option<u16>,
}

fn default_unit() -> f32 {
    1.0
}

/// A built-in, on-device lighting effect and its tunable parameters.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EffectDescriptor {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub params: Vec<EffectParam>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EffectParam {
    Speed { min: u8, max: u8, default: u8 },
    Brightness { min: u8, max: u8, default: u8 },
    Direction,
    /// The effect can randomize its color(s) instead of using a fixed one.
    Randomize,
    ColorList { max: u8 },
}

// ---------------------------------------------------------------------------
// Macro
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MacroCapability {
    pub storage: MacroStorage,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum MacroStorage {
    /// The keyboard stores and replays the macro itself.
    OnDevice { slots: u8 },
    /// IX Forge replays the macro on the host (only while running).
    HostReplay,
}

// ---------------------------------------------------------------------------
// LCD
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LcdCapability {
    pub width: u16,
    pub height: u16,
    pub format: LcdFormat,
    #[serde(default)]
    pub features: LcdFeatures,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LcdFormat {
    Mono1bpp,
    Gray4bpp,
    Rgb565,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LcdFeatures {
    #[serde(default)]
    pub image: bool,
    #[serde(default)]
    pub text: bool,
    #[serde(default)]
    pub gif: bool,
    #[serde(default)]
    pub system_monitor: bool,
}
