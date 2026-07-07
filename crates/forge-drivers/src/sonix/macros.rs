//! AULA F108 Pro on-device macros.
//!
//! **Decoded from captures `20`/`21`** (see `docs/protocols/aula-f108-pro.md`).
//! Setting a macro is two writes over the same Sonix framing as RGB/LCD:
//!
//! 1. **Program** — `04 19` / `04 15 [8]=09` / `90 01` marker / data / `04 f0`.
//!    The data packet holds a **count at byte 16** (= events × 2) and the event
//!    stream at **byte 26**; each event is 8 bytes:
//!    `[HID keycode][flag: b0=press / 30=release][delay16 LE][00 50 00 00]`.
//! 2. **Keymap binding** — one `04 27 [8]=09` bracket **per layer** (top, then
//!    function), each a 512-byte table (128 entries × 4 bytes) indexed by
//!    `led_index × 4`; `00 00 00 00` = default, `06 00 00 00` = run the macro.
//!
//! The pure encoders here are unit-tested; the `usb`-gated writer drives hardware.

pub const REPORT_LEN: usize = 64;
/// Keymap table size: 128 entries × 4 bytes (indexed by `led_index × 4`).
pub const KEYMAP_LEN: usize = 512;
/// Keymap entry byte0 meaning "run the bound macro".
pub const KEYMAP_MACRO: u8 = 0x06;

/// Which keymap layer a binding applies to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layer {
    /// Base layer (a plain key press).
    Top,
    /// Fn layer (Fn + key).
    Function,
}

/// One macro key transition: press or release of `code`, `delay_ms` after the
/// previous event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyEvent {
    pub code: u8,
    pub press: bool,
    pub delay_ms: u16,
}

impl KeyEvent {
    pub fn down(code: u8, delay_ms: u16) -> Self {
        Self { code, press: true, delay_ms }
    }
    pub fn up(code: u8, delay_ms: u16) -> Self {
        Self { code, press: false, delay_ms }
    }
    /// The 8-byte on-device event record.
    fn bytes(&self) -> [u8; 8] {
        let flag = if self.press { 0xb0 } else { 0x30 };
        let [lo, hi] = self.delay_ms.to_le_bytes();
        [self.code, flag, lo, hi, 0x00, 0x50, 0x00, 0x00]
    }
}

/// Default size of the macro-program buffer (9 × 64 = 576, as the app sends).
pub const PROGRAM_LEN: usize = 576;
const COUNT_OFF: usize = 400; // event count ×2
const EVENTS_OFF: usize = 410; // first 8-byte event

/// Build the macro **program** buffer sent after `04 15` (as 9× 64-byte reports).
/// Decoded from captures 20/21: `90 01` at byte0, **event count ×2 at byte400**,
/// **events from byte410**, `aa 55` at the last two bytes. Grows past 576 (in
/// 64-byte steps) only if the events don't fit.
pub fn encode_program(events: &[KeyEvent]) -> Vec<u8> {
    let needed = EVENTS_OFF + events.len() * 8 + 2; // + trailing aa55
    let len = needed.max(PROGRAM_LEN).div_ceil(REPORT_LEN) * REPORT_LEN;
    let mut buf = vec![0u8; len];
    buf[0] = 0x90;
    buf[1] = 0x01;
    buf[COUNT_OFF] = (events.len() * 2) as u8; // e.g. 6 events -> 0x0c
    let mut off = EVENTS_OFF;
    for e in events {
        buf[off..off + 8].copy_from_slice(&e.bytes());
        off += 8;
    }
    buf[len - 2] = 0xaa;
    buf[len - 1] = 0x55;
    buf
}

/// A generic 576-byte command buffer: all zero except `aa 55` at the end. Used
/// for the `04 11` commit (its data region is empty apart from the marker).
fn commit_buffer() -> [u8; PROGRAM_LEN] {
    let mut b = [0u8; PROGRAM_LEN];
    b[PROGRAM_LEN - 2] = 0xaa;
    b[PROGRAM_LEN - 1] = 0x55;
    b
}

/// Build one layer's 512-byte keymap table from `(led_index, macro?)` entries:
/// a key with `macro = true` gets `06 00 00 00`, else stays default `00 00 00 00`.
pub fn encode_keymap(macro_keys: &[u8]) -> [u8; KEYMAP_LEN] {
    let mut km = [0u8; KEYMAP_LEN];
    for &led in macro_keys {
        let off = led as usize * 4;
        if off + 4 <= KEYMAP_LEN {
            km[off] = KEYMAP_MACRO; // 06 00 00 00
        }
    }
    km
}

/// Write a single macro and bind it to one key (on `layer`) over HID. Replicates
/// the official app's sequence: connect handshake, program write (`04 19` then
/// `04 15` then the `90 01` marker then data), `04 11`, then a per-layer keymap
/// (`04 27`). Every command report is ACK-read (GET_REPORT) and ~33 ms paced,
/// matching the Sonix lock-step.
///
/// Only the given key gets the macro; every other key is written default, so
/// IX Forge is the source of truth for the keymap (matches how AULA writes it).
#[cfg(feature = "usb")]
pub fn write_macro(
    vid: u16,
    pid: u16,
    events: &[KeyEvent],
    led_index: u8,
    layer: Layer,
) -> Result<String, String> {
    use hidapi::HidApi;
    use std::{thread::sleep, time::Duration};

    let api = HidApi::new().map_err(|e| e.to_string())?;
    let dev = api
        .device_list()
        .find(|d| d.vendor_id() == vid && d.product_id() == pid && d.interface_number() == 3)
        .ok_or_else(|| format!("HID interface 3 of {vid:04x}:{pid:04x} not found"))?
        .open_device(&api)
        .map_err(|e| format!("open interface 3: {e}"))?;

    // Send one 64-byte payload as a report-id-0 Feature report; drain the ACK.
    let send = |payload: &[u8]| -> Result<(), String> {
        let mut buf = [0u8; REPORT_LEN + 1];
        buf[1..1 + payload.len().min(REPORT_LEN)]
            .copy_from_slice(&payload[..payload.len().min(REPORT_LEN)]);
        dev.send_feature_report(&buf).map_err(|e| format!("SET_REPORT: {e}"))?;
        let mut ack = [0u8; REPORT_LEN + 1];
        let _ = dev.get_feature_report(&mut ack);
        sleep(Duration::from_millis(33));
        Ok(())
    };
    let mk = |pairs: &[(usize, u8)]| -> [u8; REPORT_LEN] {
        let mut p = [0u8; REPORT_LEN];
        for &(i, v) in pairs {
            p[i] = v;
        }
        p
    };

    // Connect handshake. NB byte10 = 0x02 for the macro path (0x03 is the RGB one).
    send(&mk(&[(0, 0x04), (1, 0x18)]))?;
    send(&mk(&[(0, 0x04), (1, 0x28), (8, 0x01)]))?;
    send(&mk(&[
        (1, 0x01), (2, 0x5a), (3, 0x1a), (4, 0x07), (5, 0x01),
        (6, 0x08), (7, 0x26), (8, 0x09), (10, 0x02), (62, 0xaa), (63, 0x55),
    ]))?;
    send(&mk(&[(0, 0x04), (1, 0x02)]))?;

    // Program write: `04 19` / `04 15` then the 576-byte program buffer as 9
    // reports (90 01 @0, count @400, events @410, aa55 at the end), NO 04 f0.
    // The app writes it twice — replicate.
    let program = encode_program(events);
    for _ in 0..2 {
        send(&mk(&[(0, 0x04), (1, 0x19)]))?;
        send(&mk(&[(0, 0x04), (1, 0x15), (8, 0x09)]))?;
        for chunk in program.chunks(REPORT_LEN) {
            send(chunk)?;
        }
    }

    // 04 11 commit: 04 18 / 04 11 / empty 576-byte buffer (aa55 at end) / 04 f0.
    send(&mk(&[(0, 0x04), (1, 0x18)]))?;
    send(&mk(&[(0, 0x04), (1, 0x11), (8, 0x09)]))?;
    for chunk in commit_buffer().chunks(REPORT_LEN) {
        send(chunk)?;
    }
    send(&mk(&[(0, 0x04), (1, 0xf0)]))?;

    // Keymap write, one 04 27 bracket per layer (top first, then function): the
    // 512-byte table as 8 reports + a trailing aa55 commit report.
    for lyr in [Layer::Top, Layer::Function] {
        let km = if lyr == layer { encode_keymap(&[led_index]) } else { [0u8; KEYMAP_LEN] };
        send(&mk(&[(0, 0x04), (1, 0x18)]))?;
        send(&mk(&[(0, 0x04), (1, 0x27), (8, 0x09)]))?;
        for chunk in km.chunks(REPORT_LEN) {
            send(chunk)?;
        }
        send(&mk(&[(62, 0xaa), (63, 0x55)]))?; // commit marker
        send(&mk(&[(0, 0x04), (1, 0xf0)]))?;
    }
    Ok(format!(
        "wrote {}-event macro, bound to led_index {led_index} on {layer:?} layer",
        events.len()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_bytes_match_capture() {
        // a down, delay 0x0057 -> 04 b0 57 00 00 50 00 00
        assert_eq!(KeyEvent::down(0x04, 0x0057).bytes(), [0x04, 0xb0, 0x57, 0x00, 0x00, 0x50, 0x00, 0x00]);
        // a up, delay 0x0194 -> 04 30 94 01 00 50 00 00
        assert_eq!(KeyEvent::up(0x04, 0x0194).bytes(), [0x04, 0x30, 0x94, 0x01, 0x00, 0x50, 0x00, 0x00]);
    }

    #[test]
    fn program_abc_matches_capture() {
        // a b c pressed+released, delays from capture 20/21.
        let evs = [
            KeyEvent::down(0x04, 0x0057), KeyEvent::up(0x04, 0x0194),
            KeyEvent::down(0x05, 0x0050), KeyEvent::up(0x05, 0x016b),
            KeyEvent::down(0x06, 0x0040), KeyEvent::up(0x06, 0x000a),
        ];
        let p = encode_program(&evs);
        assert_eq!(p.len(), PROGRAM_LEN, "9×64 buffer");
        assert_eq!((p[0], p[1]), (0x90, 0x01), "90 01 marker at byte0");
        assert_eq!(p[COUNT_OFF], 0x0c, "count = 6 events × 2 at byte400");
        assert_eq!(&p[EVENTS_OFF..EVENTS_OFF + 4], &[0x04, 0xb0, 0x57, 0x00], "first event = a down");
        assert_eq!(&p[EVENTS_OFF + 40..EVENTS_OFF + 44], &[0x06, 0x30, 0x0a, 0x00], "last event = c up");
        assert_eq!((p[PROGRAM_LEN - 2], p[PROGRAM_LEN - 1]), (0xaa, 0x55), "aa55 at end");
    }

    #[test]
    fn keymap_binds_by_led_index() {
        let km = encode_keymap(&[12, 13]); // F11, F12
        assert_eq!(km[48], KEYMAP_MACRO, "F11 (led 12) at offset 48");
        assert_eq!(km[52], KEYMAP_MACRO, "F12 (led 13) at offset 52");
        assert_eq!(km[0], 0, "unbound keys default");
    }
}
