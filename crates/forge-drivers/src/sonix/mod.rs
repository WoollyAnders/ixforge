//! Sonix protocol family (VID `0x0c45`) — the AULA F108 Pro.
//!
//! **Decoded from the owner's USB capture** (see `docs/protocols/aula-f108-pro.md`).
//! Config is HID `SET_REPORT` **Feature**, report ID 0, **64-byte** reports, on
//! **interface 3**. A full per-key frame is **8 Feature reports**, each carrying
//! 16 records of `[led_index, R, G, B]` (color order **RGB**); slots with no LED
//! are zeroed. The upload is bracketed by a fixed command preamble and an `aa55`
//! commit, all captured verbatim below.
//!
//! Built-in effects, macros, and the LCD are not yet decoded → `set_effect`/etc.
//! return `NotSupported` for now.

use forge_core::{
    Capability, Color, DeviceProfile, DeviceSession, Driver, ForgeError, HidTransport, LedLayout,
    RgbCommand,
};

use crate::framing::{resolve_zone_keys, rgb_layout};

const REPORT_ID: u8 = 0x00;
const REPORT_LEN: usize = 64;
const SLOTS: usize = 128; // 16 records/report × 8 reports

/// Which LED index occupies each of the 128 frame slots (`0` = no LED). Traced
/// from the capture; for real LEDs the stored byte equals the slot position.
#[rustfmt::skip]
const INDEX_MAP: [u8; SLOTS] = [
    0x00,0x01,0x02,0x03,0x04,0x05,0x06,0x07,0x08,0x09,0x0a,0x0b,0x0c,0x0d,0x00,0x00,
    0x00,0x00,0x00,0x13,0x14,0x15,0x16,0x17,0x18,0x19,0x1a,0x1b,0x1c,0x1d,0x1e,0x1f,
    0x20,0x21,0x22,0x00,0x00,0x25,0x26,0x27,0x28,0x29,0x2a,0x2b,0x2c,0x2d,0x2e,0x2f,
    0x30,0x31,0x32,0x33,0x34,0x00,0x00,0x37,0x38,0x39,0x3a,0x3b,0x3c,0x3d,0x3e,0x3f,
    0x40,0x41,0x42,0x43,0x44,0x45,0x46,0x00,0x00,0x49,0x4a,0x4b,0x4c,0x4d,0x4e,0x4f,
    0x50,0x51,0x52,0x53,0x54,0x55,0x56,0x57,0x58,0x00,0x00,0x5b,0x5c,0x5d,0x5e,0x5f,
    0x60,0x61,0x62,0x63,0x64,0x65,0x66,0x67,0x68,0x69,0x6a,0x00,0x00,0x00,0x00,0x00,
    0x70,0x71,0x00,0x73,0x74,0x75,0x76,0x77,0x78,0x79,0x7a,0x7b,0x00,0x00,0x00,0x00,
];

/// Build a 64-byte report with the given (offset, value) bytes set, rest zero.
fn report(bytes: &[(usize, u8)]) -> [u8; REPORT_LEN] {
    let mut r = [0u8; REPORT_LEN];
    for &(i, v) in bytes {
        r[i] = v;
    }
    r
}

/// Fixed command reports the official app sends before the LED data (captured).
fn preamble() -> [[u8; REPORT_LEN]; 6] {
    [
        report(&[(0, 0x04), (1, 0x18)]),
        report(&[(0, 0x04), (1, 0x13), (8, 0x01)]),
        report(&[(0, 0x80), (9, 0x05), (14, 0xaa), (15, 0x55)]),
        report(&[(0, 0x04), (1, 0xf0)]),
        report(&[(0, 0x04), (1, 0x18)]),
        report(&[(0, 0x04), (1, 0x23), (8, 0x09)]),
    ]
}

/// Commit report that latches the frame (captured: `aa 55` at the tail).
fn commit() -> [u8; REPORT_LEN] {
    report(&[(62, 0xaa), (63, 0x55)])
}

/// Trailing command after commit (captured).
fn trailer() -> [u8; REPORT_LEN] {
    report(&[(0, 0x04), (1, 0xf0)])
}

/// Encode a 128-entry color buffer (indexed by led_index) into the 8 data reports.
fn encode_frame(buffer: &[Color; SLOTS]) -> [[u8; REPORT_LEN]; 8] {
    let mut reports = [[0u8; REPORT_LEN]; 8];
    for (r, rep) in reports.iter_mut().enumerate() {
        for s in 0..16 {
            let pos = r * 16 + s;
            let idx = INDEX_MAP[pos];
            let off = s * 4;
            rep[off] = idx;
            if idx != 0 {
                let c = buffer[pos];
                rep[off + 1] = c.r;
                rep[off + 2] = c.g;
                rep[off + 3] = c.b;
            }
        }
    }
    reports
}

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
        let layout = rgb_layout(profile).ok_or_else(|| {
            ForgeError::InvalidProfile(
                "sonix driver requires an rgb capability with a layout".into(),
            )
        })?;
        Ok(Box::new(SonixSession {
            capabilities: profile.capabilities.clone(),
            transport,
            layout,
        }))
    }
}

struct SonixSession {
    capabilities: Vec<Capability>,
    transport: Box<dyn HidTransport>,
    layout: LedLayout,
}

impl SonixSession {
    /// Send one 64-byte payload as a Feature report (report ID 0 prefixed).
    fn send(&mut self, payload: &[u8; REPORT_LEN]) -> Result<(), ForgeError> {
        let mut buf = [0u8; REPORT_LEN + 1];
        buf[0] = REPORT_ID;
        buf[1..].copy_from_slice(payload);
        self.transport.send_feature_report(&buf)
    }

    /// Turn a command into the 128-entry LED buffer (indexed by led_index).
    fn buffer_from(&self, cmd: &RgbCommand) -> Result<[Color; SLOTS], ForgeError> {
        let mut buf = [Color::BLACK; SLOTS];
        let mut set = |idx: usize, c: Color| {
            if idx < SLOTS {
                buf[idx] = c;
            }
        };
        match cmd {
            RgbCommand::SetAll(c) => {
                for (pos, &idx) in INDEX_MAP.iter().enumerate() {
                    if idx != 0 {
                        buf[pos] = *c;
                    }
                }
            }
            RgbCommand::SetFrame(colors) => {
                for (pos, c) in colors.iter().enumerate().take(SLOTS) {
                    buf[pos] = *c;
                }
            }
            RgbCommand::SetKeys(pairs) => {
                for (key, color) in pairs {
                    let idx = self.layout.led_index_of(key).ok_or_else(|| {
                        ForgeError::InvalidArgument(format!(
                            "key {key:?} has no LED in this layout"
                        ))
                    })? as usize;
                    if idx >= SLOTS {
                        return Err(ForgeError::InvalidArgument(format!(
                            "led index {idx} out of range (max {SLOTS})"
                        )));
                    }
                    set(idx, *color);
                }
            }
            RgbCommand::SetZone { zone, color } => {
                for key in resolve_zone_keys(&self.capabilities, zone)? {
                    if let Some(idx) = self.layout.led_index_of(&key) {
                        set(idx as usize, *color);
                    }
                }
            }
        }
        Ok(buf)
    }
}

impl DeviceSession for SonixSession {
    fn capabilities(&self) -> &[Capability] {
        &self.capabilities
    }

    fn apply_rgb(&mut self, cmd: &RgbCommand) -> Result<(), ForgeError> {
        let buffer = self.buffer_from(cmd)?;
        let data = encode_frame(&buffer);
        for p in preamble() {
            self.send(&p)?;
        }
        for rep in &data {
            self.send(rep)?;
        }
        self.send(&commit())?;
        self.send(&trailer())?;
        Ok(())
    }

    // set_effect / write_macro / push_lcd default to NotSupported until decoded.
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_core::{DeviceMatcher, DriverRef, KeyDef, KeyId, Provenance, RgbCapability, RgbMode};
    use forge_transport::MockTransport;

    fn hx(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    // Captured data report 0 (the one containing Esc = led_index 1).
    const ESC_RED_R0: &str = "0000000001ff000002000000030000000400000005000000060000000700000008000000090000000a0000000b0000000c0000000d0000000000000000000000";
    const ESC_GREEN_R0: &str = "000000000100ff0002000000030000000400000005000000060000000700000008000000090000000a0000000b0000000c0000000d0000000000000000000000";

    #[test]
    fn encodes_esc_red_and_green_exactly_as_captured() {
        let mut buf = [Color::BLACK; SLOTS];
        buf[1] = Color::RED;
        assert_eq!(encode_frame(&buf)[0].to_vec(), hx(ESC_RED_R0), "Esc red");

        buf[1] = Color::GREEN;
        assert_eq!(
            encode_frame(&buf)[0].to_vec(),
            hx(ESC_GREEN_R0),
            "Esc green"
        );
    }

    fn esc_profile() -> DeviceProfile {
        DeviceProfile {
            schema_version: 1,
            id: "aula.f108-pro".into(),
            display_name: "AULA F108 Pro".into(),
            vendor: "AULA".into(),
            matcher: DeviceMatcher {
                vid: 0x0c45,
                pid: 0x800a,
                usage_page: None,
                usage: None,
                interface: Some(3),
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
                        led_index: Some(1), // confirmed from capture
                    }],
                    matrix_size: (6, 22),
                },
                effects: vec![],
                max_brightness: 255,
                color_order: forge_core::ColorOrder::Rgb,
            })],
            provenance: Provenance::default(),
        }
    }

    #[test]
    fn apply_rgb_sends_full_sequence() {
        let mock = MockTransport::new();
        let mut session = SonixDriver
            .open(&esc_profile(), Box::new(mock.clone()))
            .unwrap();
        session
            .apply_rgb(&RgbCommand::SetKeys(vec![(
                KeyId::from("KC_ESC"),
                Color::RED,
            )]))
            .unwrap();

        let w = mock.feature_writes();
        assert_eq!(
            w.len(),
            6 + 8 + 1 + 1,
            "preamble + 8 data + commit + trailer"
        );
        assert!(w
            .iter()
            .all(|r| r.len() == REPORT_LEN + 1 && r[0] == REPORT_ID));
        // writes[6] is the first data report; strip the report-id byte and compare.
        assert_eq!(
            w[6][1..].to_vec(),
            hx(ESC_RED_R0),
            "Esc-red data report matches capture"
        );
        // Preamble first report is the 04 18 command.
        assert_eq!(w[0][1], 0x04);
        assert_eq!(w[0][2], 0x18);
    }
}
