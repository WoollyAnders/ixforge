//! Sonix protocol family (VID `0x0c45`) — the AULA F108 Pro.
//!
//! **Decoded from the owner's USB capture and proven on hardware** (see
//! `docs/protocols/aula-f108-pro.md`). Config is HID `SET_REPORT` **Feature**,
//! report ID 0, **64-byte** reports, on **interface 3**. The protocol is a
//! **command/ACK lock-step**: every control report (`04 xx` / `80`) must be
//! followed by a `GET_REPORT` that drains the device's ACK, or the control pipe
//! stalls; the LED-data reports stream without reads. Every report is paced
//! ~33 ms (the device drains slowly).
//!
//! Lighting uses the **effect / live-display path** (`04 20`): a full frame is
//! the effect preamble + 7 dense `[led_index, R, G, B]` data reports (color
//! order **RGB**) + an all-zero terminator. **Streaming this frame repeatedly
//! holds a stable color** — a single frame doesn't reliably win the panel from
//! the board's onboard profile, so [`SonixSession::apply_rgb`] streams the frame
//! [`STREAM_REPEATS`] times to lock software-display mode. (A running app should
//! keep re-streaming on a background loop; a keypress makes the board redraw its
//! saved onboard profile.)
//!
//! The `04 13`/`04 23` "static" path writes the onboard profile and needs a
//! save/commit that isn't captured yet — that's the future "save color to
//! keyboard" (persist-after-close) feature. Effects, macros, and the LCD are
//! also not yet decoded → those methods return `NotSupported`.

use forge_core::{
    Capability, Color, DeviceProfile, DeviceSession, Driver, ForgeError, HidTransport, LedLayout,
    RgbCommand,
};

use crate::framing::{resolve_zone_keys, rgb_layout};

const REPORT_ID: u8 = 0x00;
const REPORT_LEN: usize = 64;
const SLOTS: usize = 128; // color buffer is indexed by led_index (== frame slot)

/// How many times `apply_rgb` streams the frame to lock software-display mode.
/// One frame doesn't reliably override the onboard profile; a handful does.
const STREAM_REPEATS: usize = 6;

/// Which LED index occupies each frame slot (`0` = no LED). For real LEDs the
/// stored byte equals the slot position, so the color buffer can be indexed by
/// led_index directly. Traced from the capture.
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

/// Which led_index sits at each record slot of the 7 effect-frame data reports
/// (`0` = padding). Traced from the captured effect stream; the records are
/// self-describing by index, so this exact packing is what the device sent.
#[rustfmt::skip]
const EFFECT_LAYOUT: [[u8; 16]; 7] = [
    [0x01,0x02,0x03,0x04,0x05,0x06,0x07,0x08,0x09,0x0a,0x0b,0x0c,0x0d,0x70,0x71,0x73],
    [0x13,0x14,0x15,0x16,0x17,0x18,0x19,0x1a,0x1b,0x1c,0x1d,0x1e,0x1f,0x67,0x74,0x75],
    [0x76,0x20,0x21,0x22,0x7a,0x25,0x26,0x27,0x28,0x29,0x2a,0x2b,0x2c,0x2d,0x2e,0x2f],
    [0x30,0x31,0x43,0x77,0x78,0x79,0x32,0x33,0x34,0x7b,0x37,0x38,0x39,0x3a,0x3b,0x3c],
    [0x3d,0x3e,0x3f,0x40,0x41,0x42,0x55,0x44,0x45,0x46,0x49,0x4a,0x4b,0x4c,0x4d,0x4e],
    [0x4f,0x50,0x51,0x52,0x53,0x54,0x65,0x56,0x57,0x58,0x6a,0x5b,0x5c,0x5d,0x5e,0x5f],
    [0x60,0x61,0x62,0x63,0x64,0x66,0x68,0x69,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00],
];

/// Build a 64-byte report with the given (offset, value) bytes set, rest zero.
fn report(bytes: &[(usize, u8)]) -> [u8; REPORT_LEN] {
    let mut r = [0u8; REPORT_LEN];
    for &(i, v) in bytes {
        r[i] = v;
    }
    r
}

/// The one-time connect handshake reports (captured), in order. Each is followed
/// by an ACK read by the caller.
fn connect_reports() -> [[u8; REPORT_LEN]; 4] {
    [
        report(&[(0, 0x04), (1, 0x18)]),
        report(&[(0, 0x04), (1, 0x28), (8, 0x01)]),
        // config packet: 00 01 5a 1a 07 01 08 26 09 00 03 … aa 55
        report(&[
            (1, 0x01), (2, 0x5a), (3, 0x1a), (4, 0x07), (5, 0x01),
            (6, 0x08), (7, 0x26), (8, 0x09), (10, 0x03), (62, 0xaa), (63, 0x55),
        ]),
        report(&[(0, 0x04), (1, 0x02)]), // heartbeat
    ]
}

/// Effect/live-display frame preamble (captured `04 20 …[8]=08`).
fn effect_preamble() -> [u8; REPORT_LEN] {
    report(&[(0, 0x04), (1, 0x20), (8, 0x08)])
}

/// Heartbeat report the app sends to keep the session alive.
fn heartbeat() -> [u8; REPORT_LEN] {
    report(&[(0, 0x04), (1, 0x02)])
}

/// Encode a color buffer (indexed by led_index) into the 7 effect data reports.
fn encode_effect_frame(buffer: &[Color; SLOTS]) -> [[u8; REPORT_LEN]; 7] {
    let mut reports = [[0u8; REPORT_LEN]; 7];
    for (r, rep) in reports.iter_mut().enumerate() {
        for (s, &idx) in EFFECT_LAYOUT[r].iter().enumerate() {
            if idx == 0 {
                continue; // padding slot stays all-zero
            }
            let off = s * 4;
            let c = buffer[idx as usize];
            rep[off] = idx;
            rep[off + 1] = c.r;
            rep[off + 2] = c.g;
            rep[off + 3] = c.b;
        }
    }
    reports
}

/// Pace between reports. The device drains control transfers slowly (~33 ms in
/// the capture); streaming faster desyncs the frame. No-op under `cfg(test)` so
/// the mock-backed tests don't sleep.
fn pace() {
    #[cfg(not(test))]
    std::thread::sleep(std::time::Duration::from_millis(33));
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
            connected: false,
        }))
    }
}

struct SonixSession {
    capabilities: Vec<Capability>,
    transport: Box<dyn HidTransport>,
    layout: LedLayout,
    connected: bool,
}

impl SonixSession {
    /// Send one 64-byte payload as a Feature report (report ID 0 prefixed), then
    /// pace.
    fn send(&mut self, payload: &[u8; REPORT_LEN]) -> Result<(), ForgeError> {
        let mut buf = [0u8; REPORT_LEN + 1];
        buf[0] = REPORT_ID;
        buf[1..].copy_from_slice(payload);
        self.transport.send_feature_report(&buf)?;
        pace();
        Ok(())
    }

    /// Drain the device's ACK after a control report (the lock-step handshake).
    /// Best-effort: the bytes aren't needed, and a read hiccup must not abort an
    /// apply, but skipping it entirely would stall the control pipe on hardware.
    fn ack(&mut self) {
        let mut buf = [0u8; REPORT_LEN + 1];
        buf[0] = REPORT_ID;
        let _ = self.transport.get_feature_report(&mut buf);
        pace();
    }

    /// One-time connect handshake (safe to skip once done).
    fn connect(&mut self) -> Result<(), ForgeError> {
        for rep in connect_reports() {
            self.send(&rep)?;
            self.ack();
        }
        self.connected = true;
        Ok(())
    }

    /// Stream one effect frame: preamble (+ACK), the 7 data reports, terminator,
    /// and heartbeats matching the captured cadence.
    fn stream_once(&mut self, frame: &[[u8; REPORT_LEN]; 7]) -> Result<(), ForgeError> {
        self.send(&effect_preamble())?;
        self.ack();
        for rep in frame {
            self.send(rep)?;
        }
        self.send(&report(&[]))?; // all-zero terminator
        self.send(&heartbeat())?;
        self.ack();
        self.send(&heartbeat())?;
        Ok(())
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
                for &idx in INDEX_MAP.iter() {
                    if idx != 0 {
                        buf[idx as usize] = *c;
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
        let frame = encode_effect_frame(&buffer);
        if !self.connected {
            self.connect()?;
        }
        // Stream repeatedly to win the panel from the onboard profile and lock
        // software-display mode.
        for _ in 0..STREAM_REPEATS {
            self.stream_once(&frame)?;
        }
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

    #[test]
    fn effect_layout_covers_every_index_map_led_exactly_once() {
        let mut in_layout = std::collections::BTreeSet::new();
        for row in EFFECT_LAYOUT {
            for idx in row {
                if idx != 0 {
                    assert!(in_layout.insert(idx), "index {idx:#x} appears twice");
                }
            }
        }
        let in_map: std::collections::BTreeSet<u8> =
            INDEX_MAP.iter().copied().filter(|&i| i != 0).collect();
        assert_eq!(in_layout, in_map, "effect frame must cover exactly the panel LEDs");
        assert_eq!(in_layout.len(), 104);
    }

    // First effect data report for an all-red board = each layout[0] index at
    // full red (matches the recolored captured frame).
    const ALL_RED_R0: &str = "01ff000002ff000003ff000004ff000005ff000006ff000007ff000008ff000009ff00000aff00000bff00000cff00000dff000070ff000071ff000073ff0000";

    #[test]
    fn encodes_all_red_first_report_as_captured() {
        let buf = [Color::RED; SLOTS];
        assert_eq!(encode_effect_frame(&buf)[0].to_vec(), hx(ALL_RED_R0));
    }

    #[test]
    fn encodes_only_the_requested_key() {
        // Esc = led_index 1, at slot 0 of report 0.
        let mut buf = [Color::BLACK; SLOTS];
        buf[1] = Color::RED;
        let r0 = encode_effect_frame(&buf)[0];
        assert_eq!(&r0[0..4], &[0x01, 0xff, 0x00, 0x00], "Esc red");
        assert_eq!(&r0[4..8], &[0x02, 0x00, 0x00, 0x00], "F1 stays off");
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
    fn apply_rgb_connects_then_streams() {
        let mock = MockTransport::new();
        let mut session = SonixDriver
            .open(&esc_profile(), Box::new(mock.clone()))
            .unwrap();
        session
            .apply_rgb(&RgbCommand::SetKeys(vec![(KeyId::from("KC_ESC"), Color::RED)]))
            .unwrap();

        let w = mock.feature_writes();
        // Every write is report-id 0, 65 bytes.
        assert!(w.iter().all(|r| r.len() == REPORT_LEN + 1 && r[0] == REPORT_ID));
        // Connect handshake first: 04 18, then 04 28.
        assert_eq!((w[0][1], w[0][2]), (0x04, 0x18), "connect starts with 04 18");
        assert_eq!((w[1][1], w[1][2]), (0x04, 0x28), "then 04 28");
        // 4 connect reports + STREAM_REPEATS × (preamble + 7 data + terminator + 2 hb).
        let per_stream = 1 + 7 + 1 + 2;
        assert_eq!(w.len(), 4 + STREAM_REPEATS * per_stream);
        // The 5th write is the first stream's effect preamble (04 20).
        assert_eq!((w[4][1], w[4][2]), (0x04, 0x20), "stream begins with 04 20");
    }
}
