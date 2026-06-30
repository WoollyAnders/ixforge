//! Color types and the per-device channel-order quirk.

use serde::{Deserialize, Serialize};

use crate::error::ForgeError;

/// An 8-bit-per-channel RGB color.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const BLACK: Color = Color::new(0, 0, 0);
    pub const WHITE: Color = Color::new(255, 255, 255);
    pub const RED: Color = Color::new(255, 0, 0);
    pub const GREEN: Color = Color::new(0, 255, 0);
    pub const BLUE: Color = Color::new(0, 0, 255);

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Color { r, g, b }
    }

    /// Parse `"#rrggbb"` or `"rrggbb"`.
    pub fn from_hex(s: &str) -> Result<Color, ForgeError> {
        let h = s.strip_prefix('#').unwrap_or(s);
        if h.len() != 6 || !h.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(ForgeError::InvalidArgument(format!(
                "expected 6 hex digits, got {s:?}"
            )));
        }
        let byte = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).expect("validated above");
        Ok(Color::new(byte(0), byte(2), byte(4)))
    }

    /// Render as `"#rrggbb"`.
    pub fn to_hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    /// Emit the three channels in the device's wire order.
    ///
    /// Many controllers physically wire their LEDs as GRB or BGR; the profile
    /// records the order and the driver uses this to lay bytes down correctly.
    pub fn to_order(&self, order: ColorOrder) -> [u8; 3] {
        let (r, g, b) = (self.r, self.g, self.b);
        match order {
            ColorOrder::Rgb => [r, g, b],
            ColorOrder::Grb => [g, r, b],
            ColorOrder::Bgr => [b, g, r],
            ColorOrder::Brg => [b, r, g],
            ColorOrder::Rbg => [r, b, g],
            ColorOrder::Gbr => [g, b, r],
        }
    }
}

/// The byte order in which a device expects color channels on the wire.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ColorOrder {
    #[default]
    Rgb,
    Grb,
    Bgr,
    Brg,
    Rbg,
    Gbr,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trip() {
        let c = Color::from_hex("#ff8000").unwrap();
        assert_eq!(c, Color::new(255, 128, 0));
        assert_eq!(c.to_hex(), "#ff8000");
        assert_eq!(Color::from_hex("00ff00").unwrap(), Color::GREEN);
    }

    #[test]
    fn hex_rejects_bad_input() {
        assert!(Color::from_hex("#fff").is_err());
        assert!(Color::from_hex("zzzzzz").is_err());
    }

    #[test]
    fn channel_order() {
        let c = Color::new(0x11, 0x22, 0x33);
        assert_eq!(c.to_order(ColorOrder::Rgb), [0x11, 0x22, 0x33]);
        assert_eq!(c.to_order(ColorOrder::Grb), [0x22, 0x11, 0x33]);
        assert_eq!(c.to_order(ColorOrder::Bgr), [0x33, 0x22, 0x11]);
    }
}
