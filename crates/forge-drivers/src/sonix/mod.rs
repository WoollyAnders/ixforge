//! Sonix protocol family (VID `0x0c45`) — used by the AULA F108 Pro and many
//! other Sonix-MCU keyboards (SN32F2xx-class).
//!
//! ⚠️ Framing is the shared [`crate::framing`] PLACEHOLDER until decoded from a
//! real capture of the F108 Pro's official (Windows, wired-only) software. The
//! F108 Pro additionally has a 1.14" TFT and a knob; those are separate
//! capabilities decoded later (M3 / future). [`SonixVariant`] holds the RGB knobs.

use serde::Deserialize;

use forge_core::{ColorOrder, DeviceProfile, DeviceSession, Driver, ForgeError, HidTransport};

use crate::framing::{parse_variant, rgb_layout, FrameParams, PlaceholderSession};

/// Stateless driver for the Sonix family.
pub struct SonixDriver;

impl Driver for SonixDriver {
    fn family(&self) -> &'static str {
        "sonix"
    }

    fn open(
        &self,
        profile: &DeviceProfile,
        transport: Box<dyn HidTransport>,
    ) -> Result<Box<dyn DeviceSession>, ForgeError> {
        let variant: SonixVariant = parse_variant(profile)?;
        let layout = rgb_layout(profile).ok_or_else(|| {
            ForgeError::InvalidProfile(
                "sonix driver requires an rgb capability with a layout".into(),
            )
        })?;
        Ok(Box::new(PlaceholderSession {
            capabilities: profile.capabilities.clone(),
            transport,
            layout,
            frame: variant.frame(),
        }))
    }
}

/// Family-specific data knobs, supplied by the profile's `driver.variant`.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct SonixVariant {
    pub report_id: u8,
    pub packet_size: usize,
    pub set_frame_opcode: u8,
    pub color_order: ColorOrder,
}

impl SonixVariant {
    fn frame(&self) -> FrameParams {
        FrameParams {
            report_id: self.report_id,
            packet_size: self.packet_size,
            opcode: self.set_frame_opcode,
            header_len: 4,
            color_order: self.color_order,
        }
    }
}

impl Default for SonixVariant {
    fn default() -> Self {
        // PLACEHOLDER defaults — Sonix HID config reports are commonly 64 bytes.
        // Overwrite every value from a real F108 Pro capture.
        Self {
            report_id: 0x00,
            packet_size: 64,
            set_frame_opcode: 0x00,
            color_order: ColorOrder::Rgb,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_core::{
        Capability, Color, DeviceMatcher, DriverRef, KeyDef, KeyId, LedLayout, Provenance,
        RgbCapability, RgbCommand, RgbMode,
    };
    use forge_transport::MockTransport;

    fn one_key_profile() -> DeviceProfile {
        DeviceProfile {
            schema_version: 1,
            id: "aula.f108-pro.test".into(),
            display_name: "AULA F108 Pro (test)".into(),
            vendor: "AULA".into(),
            matcher: DeviceMatcher {
                vid: 0x0c45,
                pid: 0x800a,
                usage_page: None,
                usage: None,
                interface: None,
            },
            driver: DriverRef {
                family: "sonix".into(),
                variant: serde_json::Value::Null,
            },
            capabilities: vec![Capability::Rgb(RgbCapability {
                mode: RgbMode::PerKey,
                layout: LedLayout {
                    keys: vec![KeyDef {
                        id: KeyId::from("KC_ESC"),
                        label: "Esc".into(),
                        x: 0.0,
                        y: 0.0,
                        w: 1.0,
                        h: 1.0,
                        led_index: Some(0),
                    }],
                    matrix_size: (1, 1),
                },
                effects: vec![],
                max_brightness: 255,
                color_order: ColorOrder::Rgb,
            })],
            provenance: Provenance::default(),
        }
    }

    #[test]
    fn opens_and_emits_a_report() {
        let mock = MockTransport::new();
        let mut session = SonixDriver
            .open(&one_key_profile(), Box::new(mock.clone()))
            .expect("open");
        session
            .apply_rgb(&RgbCommand::SetKeys(vec![(
                KeyId::from("KC_ESC"),
                Color::GREEN,
            )]))
            .expect("apply");

        let reports = mock.feature_writes();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].len(), 64, "Sonix default packet size");
        assert_eq!(&reports[0][4..7], &[0x00, 0xff, 0x00], "Esc = green");
    }
}
