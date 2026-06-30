//! SinoWealth 8051 protocol family (VID `0x258a`) — many rebadged AULA boards
//! (e.g. F75, F99) and countless other budget keyboards.
//!
//! ⚠️ Framing is the shared [`crate::framing`] PLACEHOLDER until decoded from a
//! real capture. [`SinoVariant`] holds the per-device data knobs so that, once
//! decoded, tuning a sibling model is mostly profile data, not code.

use serde::Deserialize;

use forge_core::{ColorOrder, DeviceProfile, DeviceSession, Driver, ForgeError, HidTransport};

use crate::framing::{parse_variant, rgb_layout, FrameParams, PlaceholderSession};

/// Stateless driver for the SinoWealth family.
pub struct SinoWealthDriver;

impl Driver for SinoWealthDriver {
    fn family(&self) -> &'static str {
        "sinowealth"
    }

    fn open(
        &self,
        profile: &DeviceProfile,
        transport: Box<dyn HidTransport>,
    ) -> Result<Box<dyn DeviceSession>, ForgeError> {
        let variant: SinoVariant = parse_variant(profile)?;
        let layout = rgb_layout(profile).ok_or_else(|| {
            ForgeError::InvalidProfile(
                "sinowealth driver requires an rgb capability with a layout".into(),
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
pub struct SinoVariant {
    pub report_id: u8,
    pub packet_size: usize,
    pub set_frame_opcode: u8,
    pub set_effect_opcode: u8,
    pub color_order: ColorOrder,
}

impl SinoVariant {
    fn frame(&self) -> FrameParams {
        FrameParams {
            report_id: self.report_id,
            packet_size: self.packet_size,
            opcode: self.set_frame_opcode,
            effect_opcode: self.set_effect_opcode,
            header_len: 4,
            color_order: self.color_order,
        }
    }
}

impl Default for SinoVariant {
    fn default() -> Self {
        // PLACEHOLDER defaults — overwrite from a real capture.
        Self {
            report_id: 0x06,
            packet_size: 65,
            set_frame_opcode: 0x01,
            set_effect_opcode: 0x02,
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

    fn esc_only_profile() -> DeviceProfile {
        DeviceProfile {
            schema_version: 1,
            id: "aula.test".into(),
            display_name: "Test".into(),
            vendor: "AULA".into(),
            matcher: DeviceMatcher {
                vid: 0x258a,
                pid: 0x0049,
                usage_page: None,
                usage: None,
                interface: None,
            },
            driver: DriverRef {
                family: "sinowealth".into(),
                variant: serde_json::Value::Null, // use SinoVariant::default()
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
    fn set_keys_places_color_at_led_offset() {
        let mock = MockTransport::new();
        let mut session = SinoWealthDriver
            .open(&esc_only_profile(), Box::new(mock.clone()))
            .expect("open");

        session
            .apply_rgb(&RgbCommand::SetKeys(vec![(
                KeyId::from("KC_ESC"),
                Color::RED,
            )]))
            .expect("apply");

        let reports = mock.feature_writes();
        assert_eq!(reports.len(), 1, "one report for a single-LED buffer");
        assert_eq!(reports[0].len(), 65, "report padded to packet_size");
        assert_eq!(reports[0][0], 0x06, "report id");
        assert_eq!(reports[0][1], 0x01, "set-frame opcode");
        assert_eq!(&reports[0][4..7], &[0xff, 0x00, 0x00], "Esc = red");
    }

    #[test]
    fn unknown_key_is_rejected() {
        let mock = MockTransport::new();
        let mut session = SinoWealthDriver
            .open(&esc_only_profile(), Box::new(mock.clone()))
            .unwrap();
        let err = session
            .apply_rgb(&RgbCommand::SetKeys(vec![(
                KeyId::from("KC_NOPE"),
                Color::RED,
            )]))
            .unwrap_err();
        assert!(matches!(err, ForgeError::InvalidArgument(_)));
    }
}
