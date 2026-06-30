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
    Capability, Color, ColorOrder, DeviceProfile, DeviceSession, ForgeError, HidTransport,
    LedLayout, RgbCommand,
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

impl DeviceSession for PlaceholderSession {
    fn capabilities(&self) -> &[Capability] {
        &self.capabilities
    }

    fn apply_rgb(&mut self, cmd: &RgbCommand) -> Result<(), ForgeError> {
        let buffer = buffer_from_rgb(&self.layout, cmd)?;
        let reports = encode_paged_frame(self.frame, &buffer)?;
        for report in &reports {
            self.transport.send_feature_report(report)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paged_frame_places_color_after_header() {
        let p = FrameParams {
            report_id: 0x06,
            packet_size: 65,
            opcode: 0x01,
            header_len: 4,
            color_order: ColorOrder::Rgb,
        };
        let reports = encode_paged_frame(p, &[Color::RED]).unwrap();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].len(), 65);
        assert_eq!(reports[0][0], 0x06);
        assert_eq!(reports[0][1], 0x01);
        assert_eq!(&reports[0][4..7], &[0xff, 0x00, 0x00]);
    }

    #[test]
    fn frame_pages_large_buffers() {
        let p = FrameParams {
            report_id: 0,
            packet_size: 10, // body = 6 bytes = 2 leds per report
            opcode: 0,
            header_len: 4,
            color_order: ColorOrder::Rgb,
        };
        let reports = encode_paged_frame(p, &[Color::RED; 5]).unwrap();
        assert_eq!(
            reports.len(),
            3,
            "5 leds * 3 bytes / 6 per report = 3 reports"
        );
        assert_eq!(reports[1][3], 6, "second page offset = 6");
    }
}
