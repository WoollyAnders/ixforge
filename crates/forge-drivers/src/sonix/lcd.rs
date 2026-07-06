//! AULA F108 Pro 1.14" LCD — image framing.
//!
//! **Decoded from captures `13`–`17`** (uploading solid + split test images via the
//! official app) and recorded in `docs/protocols/aula-f108-pro.md`.
//!
//! The panel is **240×135, RGB565 little-endian**, row-major (top→bottom,
//! left→right). One image is a fixed **65,536-byte buffer**:
//!
//! ```text
//!   [0..256)      header: 01 05, then 254 × 0xFF
//!   [256..65056)  240×135 pixels, RGB565-LE (64,800 bytes)
//!   [65056..)     trailer: 480 × 0xFF (pad to 65,536)
//! ```
//!
//! sent as **16 × 4096-byte chunks on interrupt OUT endpoint 0x03** (a raw
//! endpoint — the transport for that is separate; this module only builds the
//! bytes). The chunks are bracketed by HID Feature-report commands (see
//! [`begin_upload_payload`]).
//!
//! This module is pure (no I/O, no transport) so it is fully unit-tested offline
//! against the captured bytes.

use forge_core::Color;

use futures_lite::future::block_on;
use nusb::transfer::{ControlOut, ControlType, Direction, Recipient};

pub const LCD_WIDTH: usize = 240;
pub const LCD_HEIGHT: usize = 135;
/// Total device buffer size for one image.
pub const FRAME_LEN: usize = 65_536;
/// Pixel data starts after the header.
pub const HEADER_LEN: usize = 256;
pub const CHUNK_LEN: usize = 4096;
pub const NUM_CHUNKS: usize = FRAME_LEN / CHUNK_LEN; // 16

/// Pack an 8-bit-per-channel color into RGB565, little-endian (low byte first),
/// matching the panel: red→`00 f8`, green→`e0 07`, blue→`1f 00`.
pub fn rgb565_le(c: Color) -> [u8; 2] {
    let r = (c.r as u16 >> 3) & 0x1f;
    let g = (c.g as u16 >> 2) & 0x3f;
    let b = (c.b as u16 >> 3) & 0x1f;
    let word = (r << 11) | (g << 5) | b;
    [(word & 0xff) as u8, (word >> 8) as u8]
}

/// Nearest-neighbour sample of a `src_w×src_h` RGB image at panel pixel (x, y).
/// Keeps the encoder dependency-free; higher-quality resampling can be done by
/// the caller (e.g. the `image` crate) before handing pixels here.
fn sample(src: &[Color], src_w: usize, src_h: usize, x: usize, y: usize) -> Color {
    if src_w == 0 || src_h == 0 {
        return Color::BLACK;
    }
    // Map panel coords → source coords (nearest).
    let sx = (x * src_w) / LCD_WIDTH;
    let sy = (y * src_h) / LCD_HEIGHT;
    let sx = sx.min(src_w - 1);
    let sy = sy.min(src_h - 1);
    src.get(sy * src_w + sx).copied().unwrap_or(Color::BLACK)
}

/// Build the full 65,536-byte device buffer for a source image (`src_w×src_h`,
/// row-major RGB). The image is nearest-neighbour resampled to 240×135.
pub fn encode_frame(src: &[Color], src_w: usize, src_h: usize) -> Vec<u8> {
    let mut buf = vec![0xffu8; FRAME_LEN]; // header/trailer padding is 0xFF
    buf[0] = 0x01;
    buf[1] = 0x05;
    // header bytes [2..256) stay 0xFF (already filled)
    let mut off = HEADER_LEN;
    for y in 0..LCD_HEIGHT {
        for x in 0..LCD_WIDTH {
            let [lo, hi] = rgb565_le(sample(src, src_w, src_h, x, y));
            buf[off] = lo;
            buf[off + 1] = hi;
            off += 2;
        }
    }
    // trailer [65056..65536) stays 0xFF
    debug_assert_eq!(off, HEADER_LEN + LCD_WIDTH * LCD_HEIGHT * 2);
    buf
}

/// Split a full frame into the 16 fixed-size chunks streamed on endpoint 0x03.
pub fn chunks(frame: &[u8]) -> Vec<[u8; CHUNK_LEN]> {
    assert_eq!(frame.len(), FRAME_LEN, "LCD frame must be exactly {FRAME_LEN} bytes");
    frame
        .chunks_exact(CHUNK_LEN)
        .map(|c| {
            let mut a = [0u8; CHUNK_LEN];
            a.copy_from_slice(c);
            a
        })
        .collect()
}

/// The 64-byte "begin image upload" command payload (`04 72 02 …[8]=chunk count`),
/// sent as a Feature report inside a `04 18` / … bracket before the chunks.
pub fn begin_upload_payload() -> [u8; 64] {
    let mut p = [0u8; 64];
    p[0] = 0x04;
    p[1] = 0x72;
    p[2] = 0x02;
    p[8] = NUM_CHUNKS as u8; // 0x10 = 16
    p
}

/// Upload an image to the LCD over raw USB (`nusb`). Discovers the interface that
/// owns OUT endpoint `0x03`, claims it, sends the `04 18` / `04 72` begin-upload
/// Feature reports (SET_REPORT to interface 3), then streams the 16 pixel chunks
/// to ep `0x03`. Returns a diagnostic log on success. Verbose at each step so a
/// first hardware run pinpoints where it fails — especially the interface claim,
/// which on Windows needs a WinUSB driver bound to that interface.
pub fn upload_image(
    vid: u16,
    pid: u16,
    src: &[Color],
    src_w: usize,
    src_h: usize,
) -> Result<String, String> {
    let mut log = String::new();
    let di = nusb::list_devices()
        .map_err(|e| format!("list_devices: {e}"))?
        .find(|d| d.vendor_id() == vid && d.product_id() == pid)
        .ok_or_else(|| format!("device {vid:04x}:{pid:04x} not found (is it plugged in wired?)"))?;
    let device = di.open().map_err(|e| format!("open device: {e}"))?;

    // Discover the interface that owns OUT endpoint 0x03.
    let mut lcd_iface = None;
    for cfg in device.configurations() {
        for alt in cfg.interface_alt_settings() {
            for ep in alt.endpoints() {
                log.push_str(&format!(
                    "iface {} ep 0x{:02x} {:?} {:?}\n",
                    alt.interface_number(),
                    ep.address(),
                    ep.direction(),
                    ep.transfer_type()
                ));
                if ep.address() == 0x03 && ep.direction() == Direction::Out {
                    lcd_iface = Some(alt.interface_number());
                }
            }
        }
    }
    let lcd_iface =
        lcd_iface.ok_or_else(|| format!("no OUT endpoint 0x03 found.\ntopology:\n{log}"))?;
    let interface = device
        .claim_interface(lcd_iface)
        .map_err(|e| format!("claim interface {lcd_iface}: {e}\n(needs a WinUSB driver on Windows)\ntopology:\n{log}"))?;
    log.push_str(&format!("claimed LCD interface {lcd_iface}\n"));

    let frame = encode_frame(src, src_w, src_h);

    // Begin-upload commands: SET_REPORT (class, interface 3) Feature id 0.
    let set_report = |data: &[u8]| -> Result<(), String> {
        block_on(interface.control_out(ControlOut {
            control_type: ControlType::Class,
            recipient: Recipient::Interface,
            request: 0x09, // SET_REPORT
            value: 0x0300, // Feature report, id 0
            index: 3,      // interface 3 (same as the RGB command path)
            data,
        }))
        .into_result()
        .map(|_| ())
        .map_err(|e| format!("SET_REPORT: {e:?}"))
    };
    let mut open = [0u8; 64];
    open[0] = 0x04;
    open[1] = 0x18;
    set_report(&open)?;
    set_report(&begin_upload_payload())?;
    log.push_str("sent 04 18 + 04 72 begin-upload\n");

    // Stream the 16 pixel chunks to endpoint 0x03.
    for (i, chunk) in chunks(&frame).into_iter().enumerate() {
        block_on(interface.interrupt_out(0x03, chunk.to_vec()))
            .into_result()
            .map_err(|e| format!("interrupt_out chunk {i}: {e:?}"))?;
    }
    log.push_str(&format!("streamed {NUM_CHUNKS} chunks to ep 0x03 — done\n"));
    Ok(log)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb565_primaries_match_captures() {
        assert_eq!(rgb565_le(Color::RED), [0x00, 0xf8]); // 0xF800
        assert_eq!(rgb565_le(Color { r: 0, g: 255, b: 0 }), [0xe0, 0x07]); // 0x07E0
        assert_eq!(rgb565_le(Color { r: 0, g: 0, b: 255 }), [0x1f, 0x00]); // 0x001F
    }

    #[test]
    fn solid_red_frame_matches_capture_13() {
        // Reproduce the exact buffer seen in 13-lcd-red.pcapng.
        let src = vec![Color::RED; LCD_WIDTH * LCD_HEIGHT];
        let f = encode_frame(&src, LCD_WIDTH, LCD_HEIGHT);
        assert_eq!(f.len(), FRAME_LEN);
        assert_eq!(&f[0..2], &[0x01, 0x05], "header magic");
        assert!(f[2..HEADER_LEN].iter().all(|&b| b == 0xff), "header padding is 0xFF");
        assert_eq!(&f[HEADER_LEN..HEADER_LEN + 2], &[0x00, 0xf8], "first pixel = red LE");
        // exactly 240*135 red words in the pixel region
        let px = &f[HEADER_LEN..HEADER_LEN + LCD_WIDTH * LCD_HEIGHT * 2];
        let reds = px.chunks_exact(2).filter(|w| w == &[0x00, 0xf8]).count();
        assert_eq!(reds, LCD_WIDTH * LCD_HEIGHT, "32400 red pixels");
        assert!(f[65056..].iter().all(|&b| b == 0xff), "480-byte 0xFF trailer");
    }

    #[test]
    fn chunks_are_16x4096_and_roundtrip() {
        let f = encode_frame(&[Color::RED; LCD_WIDTH * LCD_HEIGHT], LCD_WIDTH, LCD_HEIGHT);
        let cs = chunks(&f);
        assert_eq!(cs.len(), NUM_CHUNKS);
        let joined: Vec<u8> = cs.iter().flatten().copied().collect();
        assert_eq!(joined, f, "chunks concatenate back to the frame");
    }

    #[test]
    fn begin_upload_has_chunk_count() {
        let p = begin_upload_payload();
        assert_eq!((p[0], p[1], p[2]), (0x04, 0x72, 0x02));
        assert_eq!(p[8], 0x10, "16 chunks");
    }

    #[test]
    fn resamples_arbitrary_size() {
        // A 2x2 source should map cleanly onto the panel without panicking, and
        // the top-left quadrant should take the top-left source pixel.
        let src = vec![
            Color::RED, Color { r: 0, g: 255, b: 0 },
            Color { r: 0, g: 0, b: 255 }, Color::WHITE,
        ];
        let f = encode_frame(&src, 2, 2);
        assert_eq!(f.len(), FRAME_LEN);
        assert_eq!(&f[HEADER_LEN..HEADER_LEN + 2], &rgb565_le(Color::RED), "top-left = red");
    }
}
