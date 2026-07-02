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
//! order **RGB**) + an all-zero terminator. The board holds the last streamed
//! frame only until a keypress (then it redraws its saved onboard profile), so
//! the session runs a **background thread that continuously re-streams** the
//! current frame. [`SonixSession::apply_rgb`] just swaps that frame in and
//! returns immediately; the worker keeps the color asserted.
//!
//! The `04 13`/`04 23` "static" path writes the onboard profile and needs a
//! save/commit that isn't captured yet — that's the future "save color to
//! keyboard" (persist-after-close) feature. Effects, macros, and the LCD are
//! also not yet decoded → those methods return `NotSupported`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use forge_core::{
    Capability, Color, DeviceProfile, DeviceSession, Driver, EffectSelection, ForgeError,
    HidTransport, LedLayout, RgbCommand,
};

use crate::framing::{resolve_zone_keys, rgb_layout};

const REPORT_ID: u8 = 0x00;
const REPORT_LEN: usize = 64;
const SLOTS: usize = 128; // color buffer is indexed by led_index (== frame slot)

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
/// the capture); streaming faster desyncs the frame. No-op under `cfg(test)`.
fn pace() {
    #[cfg(not(test))]
    thread::sleep(Duration::from_millis(33));
}

/// Gap between full re-streamed frames. Small: after a keypress makes the board
/// redraw its onboard profile, the next frame re-asserts our colors within one
/// cycle. Also bounds the worker loop under `cfg(test)`.
const FRAME_GAP: Duration = Duration::from_millis(8);

/// Send one 64-byte payload as a Feature report (report ID 0 prefixed), + pace.
fn send(t: &mut dyn HidTransport, payload: &[u8; REPORT_LEN]) -> Result<(), ForgeError> {
    let mut buf = [0u8; REPORT_LEN + 1];
    buf[0] = REPORT_ID;
    buf[1..].copy_from_slice(payload);
    t.send_feature_report(&buf)?;
    pace();
    Ok(())
}

/// Drain the device's ACK after a control report (the lock-step handshake).
/// Best-effort: the bytes aren't needed and a read hiccup must not abort, but
/// skipping it entirely would stall the control pipe on hardware.
fn ack(t: &mut dyn HidTransport) {
    let mut buf = [0u8; REPORT_LEN + 1];
    buf[0] = REPORT_ID;
    let _ = t.get_feature_report(&mut buf);
    pace();
}

/// One-time connect handshake.
fn connect(t: &mut dyn HidTransport) -> Result<(), ForgeError> {
    for rep in connect_reports() {
        send(t, &rep)?;
        ack(t);
    }
    Ok(())
}

/// Stream one effect frame: preamble (+ACK), 7 data reports, terminator, and
/// heartbeats, matching the captured cadence.
fn stream_once(t: &mut dyn HidTransport, frame: &[[u8; REPORT_LEN]; 7]) -> Result<(), ForgeError> {
    send(t, &effect_preamble())?;
    ack(t);
    for rep in frame {
        send(t, rep)?;
    }
    send(t, &report(&[]))?; // all-zero terminator
    send(t, &heartbeat())?;
    ack(t);
    send(t, &heartbeat())?;
    Ok(())
}

/// One captured `04 18` / `04 13[8]=01` / <payload> / `04 f0` bracketed command,
/// each control report ACK-read.
fn cmd_bracket(t: &mut dyn HidTransport, payload: &[u8; REPORT_LEN]) -> Result<(), ForgeError> {
    send(t, &report(&[(0, 0x04), (1, 0x18)]))?;
    ack(t);
    send(t, &report(&[(0, 0x04), (1, 0x13), (8, 0x01)]))?;
    ack(t);
    send(t, payload)?;
    ack(t);
    send(t, &report(&[(0, 0x04), (1, 0xf0)]))?;
    Ok(())
}

/// Select an onboard animation and (for color-capable effects) set its base
/// color. Packets decoded from captures `07`/`08`:
///   select: `[id, ff, .., speed@9, brightness@10, dir@11, .., aa 55 @14-15]`
///   color:  `[id, 00, R@2, G@3, B@4, .., speed@9, brightness@10, aa 55 @14-15]`
/// The board then animates on its own MCU. `dir`/randomize are future options.
fn send_effect(
    t: &mut dyn HidTransport,
    id: u8,
    speed: u8,
    brightness: u8,
    color: Option<Color>,
) -> Result<(), ForgeError> {
    cmd_bracket(
        t,
        &report(&[(0, id), (1, 0xff), (9, speed), (10, brightness), (14, 0xaa), (15, 0x55)]),
    )?;
    if let Some(c) = color {
        cmd_bracket(
            t,
            &report(&[
                (0, id), (2, c.r), (3, c.g), (4, c.b), (9, speed), (10, brightness),
                (14, 0xaa), (15, 0x55),
            ]),
        )?;
    }
    Ok(())
}

/// What the worker does each tick; the session handle swaps this behind a mutex.
enum Mode {
    /// Continuously stream this per-key frame (live RGB display).
    Streaming(Box<[Color; SLOTS]>),
    /// Select an onboard animation once, then go idle (board animates itself).
    SelectEffect { id: u8, speed: u8, brightness: u8, color: Option<Color> },
    /// Nothing to drive (fresh session, or an onboard effect is running).
    Idle,
}

/// State shared between the session handle and its worker thread.
struct Shared {
    mode: Mutex<Mode>,
    running: AtomicBool,
}

/// Idle poll interval — how quickly the worker notices a new command.
const IDLE_GAP: Duration = Duration::from_millis(20);

/// The worker: connect once, then act on the current mode until stopped.
fn worker_loop(mut transport: Box<dyn HidTransport>, shared: Arc<Shared>) {
    if let Err(e) = connect(transport.as_mut()) {
        eprintln!("[forge] sonix worker: connect failed ({e:?}); exiting");
        return;
    }
    eprintln!("[forge] sonix worker: connected, entering loop");
    enum Act {
        Stream(Box<[[u8; REPORT_LEN]; 7]>),
        Effect { id: u8, speed: u8, brightness: u8, color: Option<Color> },
        Idle,
    }
    while shared.running.load(Ordering::Relaxed) {
        // Decide without holding the lock across I/O.
        let act = {
            let mut mode = shared.mode.lock().unwrap_or_else(|e| e.into_inner());
            match &*mode {
                Mode::Streaming(frame) => Act::Stream(Box::new(encode_effect_frame(frame))),
                Mode::SelectEffect { id, speed, brightness, color } => {
                    let a = Act::Effect {
                        id: *id,
                        speed: *speed,
                        brightness: *brightness,
                        color: *color,
                    };
                    *mode = Mode::Idle; // one-shot; the board keeps animating on its own
                    a
                }
                Mode::Idle => Act::Idle,
            }
        };
        match act {
            Act::Stream(frame) => {
                if let Err(e) = stream_once(transport.as_mut(), &frame) {
                    eprintln!("[forge] sonix worker: stream write failed ({e:?}); exiting");
                    break;
                }
                thread::sleep(FRAME_GAP);
            }
            Act::Effect { id, speed, brightness, color } => {
                if let Err(e) = send_effect(transport.as_mut(), id, speed, brightness, color) {
                    eprintln!("[forge] sonix worker: effect write failed ({e:?}); exiting");
                    break;
                }
                thread::sleep(FRAME_GAP);
            }
            Act::Idle => thread::sleep(IDLE_GAP),
        }
    }
    eprintln!("[forge] sonix worker: loop exited");
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
        let shared = Arc::new(Shared {
            mode: Mutex::new(Mode::Idle),
            running: AtomicBool::new(true),
        });
        let worker = shared.clone();
        let handle = thread::spawn(move || worker_loop(transport, worker));
        Ok(Box::new(SonixSession {
            capabilities: profile.capabilities.clone(),
            layout,
            shared,
            handle: Some(handle),
        }))
    }
}

struct SonixSession {
    capabilities: Vec<Capability>,
    layout: LedLayout,
    shared: Arc<Shared>,
    handle: Option<JoinHandle<()>>,
}

impl SonixSession {
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
        *self.shared.mode.lock().unwrap_or_else(|e| e.into_inner()) =
            Mode::Streaming(Box::new(buffer));
        Ok(())
    }

    fn set_effect(&mut self, sel: &EffectSelection) -> Result<(), ForgeError> {
        // The device selects an onboard effect by a 1-based id; the profile lists
        // effects in device order, so id = position + 1.
        let effects = self
            .capabilities
            .iter()
            .find_map(|c| match c {
                Capability::Rgb(r) => Some(&r.effects),
                _ => None,
            })
            .ok_or(ForgeError::NotSupported)?;
        let pos = effects
            .iter()
            .position(|e| e.id == sel.effect_id)
            .ok_or_else(|| ForgeError::InvalidArgument(format!("unknown effect {:?}", sel.effect_id)))?;
        let id = (pos + 1) as u8;
        let speed = sel.speed.unwrap_or(3).clamp(1, 5);
        let brightness = sel.brightness.unwrap_or(5).clamp(1, 5);
        // Send a base color only for effects that declare a color_list param
        // (color-capable); rainbow/multi effects ignore it.
        let takes_color = effects[pos]
            .params
            .iter()
            .any(|p| matches!(p, forge_core::EffectParam::ColorList { .. }));
        let color = if takes_color { sel.colors.first().copied() } else { None };
        *self.shared.mode.lock().unwrap_or_else(|e| e.into_inner()) =
            Mode::SelectEffect { id, speed, brightness, color };
        Ok(())
    }

    // write_macro / push_lcd default to NotSupported until decoded.
}

impl Drop for SonixSession {
    fn drop(&mut self) {
        self.shared.running.store(false, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_core::{
        DeviceMatcher, DriverRef, EffectDescriptor, EffectParam, EffectSelection, KeyDef, KeyId,
        Provenance, RgbCapability, RgbMode,
    };
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

    #[test]
    fn connect_emits_the_captured_handshake() {
        let mut t = MockTransport::new();
        connect(&mut t).unwrap();
        let w = t.feature_writes();
        assert_eq!(w.len(), 4);
        assert!(w.iter().all(|r| r.len() == REPORT_LEN + 1 && r[0] == REPORT_ID));
        assert_eq!((w[0][1], w[0][2]), (0x04, 0x18));
        assert_eq!((w[1][1], w[1][2], w[1][9]), (0x04, 0x28, 0x01));
        // config packet: payload starts 00 01 5a 1a … and ends …aa 55 (write is
        // report-id-prefixed, so payload byte N is at write index N+1).
        assert_eq!((w[2][2], w[2][3]), (0x01, 0x5a));
        assert_eq!((w[2][63], w[2][64]), (0xaa, 0x55));
        assert_eq!((w[3][1], w[3][2]), (0x04, 0x02));
    }

    #[test]
    fn stream_once_emits_preamble_data_terminator() {
        let mut t = MockTransport::new();
        let mut buf = [Color::BLACK; SLOTS];
        buf[1] = Color::RED;
        stream_once(&mut t, &encode_effect_frame(&buf)).unwrap();
        let w = t.feature_writes();
        assert_eq!(w.len(), 1 + 7 + 1 + 2, "preamble + 7 data + terminator + 2 hb");
        assert_eq!((w[0][1], w[0][2]), (0x04, 0x20), "effect preamble");
        assert_eq!(&w[1][1..5], &[0x01, 0xff, 0x00, 0x00], "first data report = Esc red");
        assert!(w[8][1..].iter().all(|&b| b == 0), "terminator is all-zero");
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
                effects: ["static", "single_on", "single_off"]
                    .iter()
                    .map(|id| EffectDescriptor {
                        id: (*id).into(),
                        name: (*id).into(),
                        params: vec![EffectParam::ColorList { max: 1 }],
                    })
                    .collect(),
                max_brightness: 255,
                color_order: forge_core::ColorOrder::Rgb,
            })],
            provenance: Provenance::default(),
        }
    }

    #[test]
    fn worker_connects_then_streams_the_applied_frame() {
        let mock = MockTransport::new();
        let mut session = SonixDriver
            .open(&esc_profile(), Box::new(mock.clone()))
            .unwrap();
        session
            .apply_rgb(&RgbCommand::SetKeys(vec![(KeyId::from("KC_ESC"), Color::RED)]))
            .unwrap();
        // Let the worker run a few frames, then stop it (Drop joins the thread).
        thread::sleep(Duration::from_millis(150));
        drop(session);

        let w = mock.feature_writes();
        // Connect handshake happened first.
        assert_eq!((w[0][1], w[0][2]), (0x04, 0x18), "starts with connect 04 18");
        // At least one effect preamble was streamed.
        assert!(w.iter().any(|r| r[1] == 0x04 && r[2] == 0x20), "streamed a frame");
        // At least one frame carried Esc red (idx 1 = ff0000).
        assert!(
            w.iter().any(|r| r[1] == 0x01 && r[2] == 0xff && r[3] == 0x00 && r[4] == 0x00),
            "the applied Esc-red frame was streamed"
        );
    }

    #[test]
    fn set_effect_selects_onboard_by_position_plus_one() {
        let mock = MockTransport::new();
        let mut session = SonixDriver
            .open(&esc_profile(), Box::new(mock.clone()))
            .unwrap();
        // "single_off" is at position 2 → device effect id 3; give it a red color.
        session
            .set_effect(&EffectSelection {
                effect_id: "single_off".into(),
                speed: Some(4),
                brightness: Some(2),
                colors: vec![Color::RED],
            })
            .unwrap();
        thread::sleep(Duration::from_millis(150));
        drop(session);

        let w = mock.feature_writes();
        assert_eq!((w[0][1], w[0][2]), (0x04, 0x18), "connect first");
        // Payload byte N is at write index N+1 (report-id-prefixed).
        // Select packet: byte0=id 3, byte1=0xff, speed@9, brightness@10, aa55@14-15.
        let sel = w
            .iter()
            .find(|r| r[1] == 0x03 && r[2] == 0xff)
            .expect("effect-select packet present");
        assert_eq!(sel[10], 4, "speed at payload byte9");
        assert_eq!(sel[11], 2, "brightness at payload byte10");
        assert_eq!((sel[15], sel[16]), (0xaa, 0x55), "aa55 at payload bytes 14-15");
        // Color packet: byte0=id 3, byte1=0x00, R@2/G@3/B@4 = ff/00/00.
        let col = w
            .iter()
            .find(|r| r[1] == 0x03 && r[2] == 0x00)
            .expect("effect-color packet present");
        assert_eq!((col[3], col[4], col[5]), (0xff, 0x00, 0x00), "red at payload bytes 2-4");
    }
}
