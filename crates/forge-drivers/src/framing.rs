//! Shared placeholder framing used by controller families until each protocol is
//! decoded from real captures.
//!
//! ⚠️ **PLACEHOLDER.** The paged full-frame layout here is a *plausible structure*
//! (report id, opcode, paged offset, channel-ordered payload), **not** a decoded
//! protocol. Real report IDs / opcodes / offsets / checksums / init-commit
//! sequences come from per-device captures (`docs/protocols/<device>.md`).
//! Families share this scaffold now; when a real protocol diverges, that family
//! grows its own session instead of using [`PlaceholderSession`].

use serde::de::DeserializeOwned;

use forge_core::{
    Capability, Color, ColorOrder, DeviceProfile, DeviceSession, EffectSelection, ForgeError,
    HidTransport, KeyId, LedLayout, RgbCommand, RgbMode, ZoneId,
};

/// Wire framing parameters for a paged full-frame LED write.
#[derive(Clone, Copy, Debug)]
pub struct FrameParams {
    /// HID report ID prefixed on each report (`0` if the device is unnumbered).
    pub report_id: u8,
    /// Total report length in bytes, including the report ID.
    pub packet_size: usize,
    /// Opcode byte for a full-frame write.
    pub opcode: u8,
    /// Opcode byte for selecting a built-in effect.
    pub effect_opcode: u8,
    /// Bytes reserved before the color payload: `[report_id, opcode, off_hi, off_lo]`.
    pub header_len: usize,
    /// Physical channel order of the LEDs.
    pub color_order: ColorOrder,
}

/// Parse a family-specific variant from `profile.driver.variant`, or use defaults
/// when the profile omits it.
pub fn parse_variant<T: Default + DeserializeOwned>(
    profile: &DeviceProfile,
) -> Result<T, ForgeError> {
    if profile.driver.variant.is_null() {
        Ok(T::default())
    } else {
        serde_json::from_value(profile.driver.variant.clone()).map_err(|e| {
            ForgeError::InvalidProfile(format!("{} variant: {e}", profile.driver.family))
        })
    }
}

/// The LED layout from the profile's first RGB capability, if any.
pub fn rgb_layout(profile: &DeviceProfile) -> Option<LedLayout> {
    profile.capabilities.iter().find_map(|c| match c {
        Capability::Rgb(rgb) => Some(rgb.layout.clone()),
        _ => None,
    })
}

/// Number of addressable LEDs in a layout = highest `led_index` + 1.
pub fn led_count(layout: &LedLayout) -> usize {
    layout
        .keys
        .iter()
        .filter_map(|k| k.led_index)
        .map(|i| i as usize + 1)
        .max()
        .unwrap_or(0)
}

/// Resolve an [`RgbCommand`] against a layout into a full LED buffer.
///
/// `SetKeys` produces a full frame (named keys set, the rest black) because the
/// placeholder protocol writes the whole buffer; a device with true partial
/// per-key updates overrides this once its protocol is decoded.
pub fn buffer_from_rgb(layout: &LedLayout, cmd: &RgbCommand) -> Result<Vec<Color>, ForgeError> {
    let count = led_count(layout);
    match cmd {
        RgbCommand::SetAll(color) => Ok(vec![*color; count]),
        RgbCommand::SetFrame(colors) => Ok(colors.clone()),
        RgbCommand::SetKeys(pairs) => {
            let mut buffer = vec![Color::BLACK; count];
            for (key, color) in pairs {
                match layout.led_index_of(key) {
                    Some(idx) if (idx as usize) < buffer.len() => buffer[idx as usize] = *color,
                    Some(idx) => {
                        return Err(ForgeError::InvalidArgument(format!(
                            "led index {idx} out of range (count {count})"
                        )))
                    }
                    None => {
                        return Err(ForgeError::InvalidArgument(format!(
                            "key {key:?} has no LED in this layout"
                        )))
                    }
                }
            }
            Ok(buffer)
        }
        RgbCommand::SetZone { .. } => Err(ForgeError::NotSupported),
    }
}

/// Encode an LED buffer into one or more reports (PLACEHOLDER framing).
pub fn encode_paged_frame(p: FrameParams, buffer: &[Color]) -> Result<Vec<Vec<u8>>, ForgeError> {
    if p.packet_size <= p.header_len || p.header_len < 2 {
        return Err(ForgeError::InvalidProfile(
            "packet_size too small for header".into(),
        ));
    }
    let mut payload = Vec::with_capacity(buffer.len() * 3);
    for c in buffer {
        payload.extend_from_slice(&c.to_order(p.color_order));
    }

    let body = p.packet_size - p.header_len;
    // An empty buffer still sends one report so the device is addressed.
    let chunks: Vec<&[u8]> = if payload.is_empty() {
        vec![&[]]
    } else {
        payload.chunks(body).collect()
    };

    let mut reports = Vec::with_capacity(chunks.len());
    for (i, chunk) in chunks.into_iter().enumerate() {
        let offset = (i * body) as u16;
        let mut report = vec![0u8; p.packet_size];
        report[0] = p.report_id;
        report[1] = p.opcode;
        if p.header_len >= 4 {
            report[2] = (offset >> 8) as u8;
            report[3] = (offset & 0xff) as u8;
        }
        report[p.header_len..p.header_len + chunk.len()].copy_from_slice(chunk);
        reports.push(report);
    }
    Ok(reports)
}

/// A device session that uses the shared placeholder framing.
///
/// Two families currently share it because neither protocol is decoded yet; the
/// only difference between them is the [`FrameParams`] their profile supplies.
pub struct PlaceholderSession {
    pub capabilities: Vec<Capability>,
    pub transport: Box<dyn HidTransport>,
    pub layout: LedLayout,
    pub frame: FrameParams,
}

impl PlaceholderSession {
    /// Onboard index of an effect = its position in the profile's effects list
    /// (profiles list effects in the device's cycle order).
    fn effect_index(&self, effect_id: &str) -> Result<u8, ForgeError> {
        let effects = self
            .capabilities
            .iter()
            .find_map(|c| match c {
                Capability::Rgb(rgb) => Some(&rgb.effects),
                _ => None,
            })
            .ok_or(ForgeError::NotSupported)?;
        effects
            .iter()
            .position(|e| e.id == effect_id)
            .map(|p| p as u8)
            .ok_or_else(|| ForgeError::InvalidArgument(format!("unknown effect {effect_id:?}")))
    }
}

/// The keys belonging to a lighting zone (for `RgbMode::Zoned` boards). Shared by
/// the placeholder session and the real drivers.
pub(crate) fn resolve_zone_keys(
    capabilities: &[Capability],
    zone: &ZoneId,
) -> Result<Vec<KeyId>, ForgeError> {
    let zones = capabilities.iter().find_map(|c| match c {
        Capability::Rgb(rgb) => match &rgb.mode {
            RgbMode::Zoned { zones } => Some(zones),
            _ => None,
        },
        _ => None,
    });
    zones
        .ok_or(ForgeError::NotSupported)?
        .iter()
        .find(|z| &z.id == zone)
        .map(|z| z.keys.clone())
        .ok_or_else(|| ForgeError::InvalidArgument(format!("unknown zone {zone:?}")))
}

impl DeviceSession for PlaceholderSession {
    fn capabilities(&self) -> &[Capability] {
        &self.capabilities
    }

    fn apply_rgb(&mut self, cmd: &RgbCommand) -> Result<(), ForgeError> {
        // Resolve a zone to its keys, then reuse the per-key path.
        let resolved;
        let cmd = match cmd {
            RgbCommand::SetZone { zone, color } => {
                let pairs = resolve_zone_keys(&self.capabilities, zone)?
                    .into_iter()
                    .map(|k| (k, *color))
                    .collect();
                resolved = RgbCommand::SetKeys(pairs);
                &resolved
            }
            other => other,
        };
        let buffer = buffer_from_rgb(&self.layout, cmd)?;
        let reports = encode_paged_frame(self.frame, &buffer)?;
        for report in &reports {
            self.transport.send_feature_report(report)?;
        }
        Ok(())
    }

    /// Select a built-in effect (PLACEHOLDER framing — replace with the captured
    /// protocol): `[report_id, effect_opcode, index, speed, brightness, r, g, b]`.
    fn set_effect(&mut self, sel: &EffectSelection) -> Result<(), ForgeError> {
        let index = self.effect_index(&sel.effect_id)?;
        let p = self.frame;
        if p.packet_size < 4 {
            return Err(ForgeError::InvalidProfile("packet_size too small".into()));
        }
        let mut report = vec![0u8; p.packet_size];
        report[0] = p.report_id;
        report[1] = p.effect_opcode;
        report[2] = index;
        report[3] = sel.speed.unwrap_or(3);
        if p.packet_size > 4 {
            report[4] = sel.brightness.unwrap_or(4);
        }
        if p.packet_size >= 8 {
            let color = sel.colors.first().copied().unwrap_or(Color::WHITE);
            report[5..8].copy_from_slice(&color.to_order(p.color_order));
        }
        self.transport.send_feature_report(&report)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(packet_size: usize) -> FrameParams {
        FrameParams {
            report_id: 0x06,
            packet_size,
            opcode: 0x01,
            effect_opcode: 0x02,
            header_len: 4,
            color_order: ColorOrder::Rgb,
        }
    }

    #[test]
    fn paged_frame_places_color_after_header() {
        let reports = encode_paged_frame(params(65), &[Color::RED]).unwrap();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].len(), 65);
        assert_eq!(reports[0][0], 0x06);
        assert_eq!(reports[0][1], 0x01);
        assert_eq!(&reports[0][4..7], &[0xff, 0x00, 0x00]);
    }

    #[test]
    fn frame_pages_large_buffers() {
        // packet_size 10 → body = 6 bytes = 2 leds per report
        let reports = encode_paged_frame(params(10), &[Color::RED; 5]).unwrap();
        assert_eq!(
            reports.len(),
            3,
            "5 leds * 3 bytes / 6 per report = 3 reports"
        );
        assert_eq!(reports[1][3], 6, "second page offset = 6");
    }

    #[test]
    fn set_effect_encodes_index_and_params() {
        use forge_core::{
            EffectDescriptor, EffectSelection, KeyDef, KeyId, LedLayout, RgbCapability, RgbMode,
        };
        use forge_transport::MockTransport;

        let effects = vec![
            EffectDescriptor {
                id: "static".into(),
                name: "Static".into(),
                params: vec![],
            },
            EffectDescriptor {
                id: "breathing".into(),
                name: "Breathing".into(),
                params: vec![],
            },
            EffectDescriptor {
                id: "wave".into(),
                name: "Wave".into(),
                params: vec![],
            },
        ];
        let rgb = RgbCapability {
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
            effects,
            max_brightness: 255,
            color_order: ColorOrder::Rgb,
        };
        let mock = MockTransport::new();
        let mut session = PlaceholderSession {
            capabilities: vec![Capability::Rgb(rgb.clone())],
            transport: Box::new(mock.clone()),
            layout: rgb.layout.clone(),
            frame: params(64),
        };

        session
            .set_effect(&EffectSelection {
                effect_id: "wave".into(), // index 2
                speed: Some(5),
                brightness: Some(4),
                colors: vec![Color::GREEN],
                direction: None,
                randomize: false,
            })
            .unwrap();

        let reports = mock.feature_writes();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0][1], 0x02, "effect opcode");
        assert_eq!(reports[0][2], 2, "effect index (position of 'wave')");
        assert_eq!(reports[0][3], 5, "speed");
        assert_eq!(reports[0][4], 4, "brightness");
        assert_eq!(&reports[0][5..8], &[0x00, 0xff, 0x00], "green");
    }

    #[test]
    fn set_effect_rejects_unknown_id() {
        use forge_core::EffectSelection;
        use forge_transport::MockTransport;

        let mut session = PlaceholderSession {
            capabilities: vec![],
            transport: Box::new(MockTransport::new()),
            layout: super::LedLayout {
                keys: vec![],
                matrix_size: (0, 0),
            },
            frame: params(64),
        };
        let err = session
            .set_effect(&EffectSelection {
                effect_id: "nope".into(),
                speed: None,
                brightness: None,
                colors: vec![],
                direction: None,
                randomize: false,
            })
            .unwrap_err();
        // No RGB capability at all → NotSupported.
        assert!(matches!(err, ForgeError::NotSupported));
    }

    #[test]
    fn set_zone_lights_only_that_zone() {
        use forge_core::{KeyDef, KeyId, LedLayout, RgbCapability, RgbMode, ZoneDef, ZoneId};
        use forge_transport::MockTransport;

        let key = |id: &str, led: u16| KeyDef {
            id: KeyId::from(id),
            label: id.into(),
            x: led as f32,
            y: 0.0,
            w: 1.0,
            h: 1.0,
            led_index: Some(led),
        };
        let layout = LedLayout {
            keys: vec![key("A", 0), key("B", 1), key("C", 2)],
            matrix_size: (1, 3),
        };
        let rgb = RgbCapability {
            mode: RgbMode::Zoned {
                zones: vec![ZoneDef {
                    id: ZoneId::from("left"),
                    label: "Left".into(),
                    keys: vec![KeyId::from("A"), KeyId::from("B")],
                }],
            },
            layout: layout.clone(),
            effects: vec![],
            max_brightness: 255,
            color_order: ColorOrder::Rgb,
        };
        let mock = MockTransport::new();
        let mut session = PlaceholderSession {
            capabilities: vec![Capability::Rgb(rgb)],
            transport: Box::new(mock.clone()),
            layout,
            frame: params(64),
        };

        session
            .apply_rgb(&RgbCommand::SetZone {
                zone: ZoneId::from("left"),
                color: Color::RED,
            })
            .unwrap();

        let r = &mock.feature_writes()[0];
        assert_eq!(&r[4..7], &[0xff, 0x00, 0x00], "A lit (zone)");
        assert_eq!(&r[7..10], &[0xff, 0x00, 0x00], "B lit (zone)");
        assert_eq!(&r[10..13], &[0x00, 0x00, 0x00], "C off (not in zone)");

        // Unknown zone is rejected.
        assert!(session
            .apply_rgb(&RgbCommand::SetZone {
                zone: ZoneId::from("nope"),
                color: Color::RED,
            })
            .is_err());
    }
}
