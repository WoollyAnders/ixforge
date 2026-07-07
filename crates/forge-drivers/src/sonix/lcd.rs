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

/// One source frame: `(width, height, row-major RGB pixels)`.
pub type SrcFrame = (usize, usize, Vec<Color>);

/// Bytes of RGB565 pixel data per frame (240×135×2).
pub const FRAME_PIXELS_LEN: usize = LCD_WIDTH * LCD_HEIGHT * 2; // 64,800

/// Default per-frame duration for a single still image (device units = ms/2;
/// `0x05` matches the official app's static upload, capture 13).
pub const STILL_DURATION: u8 = 0x05;

/// Build the device buffer for one or more frames. Decoded from captures 13
/// (still), 18 (3-frame GIF) and 19 (20-frame GIF): a **single 256-byte header**
/// then each frame's raw 64,800-byte RGB565 pixels back-to-back, padded with
/// `0xFF` up to the next **4096-byte chunk boundary** (NOT N×65536 — that only
/// coincided for small N).
///
/// Header: **byte0 = frame count**, **bytes[1..1+N] = per-frame duration** (ms/2),
/// rest `0xFF`. (A still is just N=1: `01 05 FF…`.) Frames after the first carry
/// no header — that was the bug that left animations stuck on frame 0.
///
/// `durations` is in device units (ms/2); missing entries default to
/// [`STILL_DURATION`]. The image is nearest-neighbour resampled to 240×135.
pub fn encode(frames: &[SrcFrame], durations: &[u8]) -> Vec<u8> {
    let n = frames.len().max(1);
    // Pad the header + N pixel blocks up to a whole number of 4096-byte chunks.
    let total = (HEADER_LEN + n * FRAME_PIXELS_LEN).div_ceil(CHUNK_LEN) * CHUNK_LEN;
    let mut buf = vec![0xffu8; total];
    buf[0] = n as u8; // frame count (byte0)
    for i in 0..n {
        buf[1 + i] = durations.get(i).copied().unwrap_or(STILL_DURATION);
    }
    // header bytes [1+n .. 256) stay 0xFF
    let mut off = HEADER_LEN;
    for (w, h, px) in frames {
        for y in 0..LCD_HEIGHT {
            for x in 0..LCD_WIDTH {
                let [lo, hi] = rgb565_le(sample(px, *w, *h, x, y));
                buf[off] = lo;
                buf[off + 1] = hi;
                off += 2;
            }
        }
    }
    // remaining bytes (trailer) stay 0xFF
    debug_assert_eq!(off, HEADER_LEN + n * FRAME_PIXELS_LEN);
    buf
}

/// Convenience for a single still image (N=1). See [`encode`].
pub fn encode_frame(src: &[Color], src_w: usize, src_h: usize) -> Vec<u8> {
    encode(&[(src_w, src_h, src.to_vec())], &[STILL_DURATION])
}

/// Max frames the protocol can address: the header's `byte0` is a u8 and its
/// per-frame duration bytes fill the 256-byte header (`byte0` + N durations ≤ 256),
/// so N ≤ 255. (The chunk count is 16-bit — see [`begin_upload_payload`] — so it's
/// not the limit.) The device may run out of memory well before this.
pub const MAX_FRAMES: usize = 255;

/// Split a buffer (one 65,536-byte frame, or N of them concatenated for an
/// animation) into the 4096-byte chunks streamed on endpoint 0x03.
pub fn chunks(buf: &[u8]) -> Vec<[u8; CHUNK_LEN]> {
    assert!(
        !buf.is_empty() && buf.len().is_multiple_of(CHUNK_LEN),
        "LCD buffer must be a whole number of {CHUNK_LEN}-byte chunks"
    );
    buf.chunks_exact(CHUNK_LEN)
        .map(|c| {
            let mut a = [0u8; CHUNK_LEN];
            a.copy_from_slice(c);
            a
        })
        .collect()
}

/// The 64-byte "begin upload" command (`04 72`). Decoded from captures 13/18/19:
/// **byte2 = `0x02` for a single image, `0x07` for an animation**; **bytes 8–9 =
/// total chunk count, 16-bit little-endian** (`10 00`=16 for a still, `30 00`=48
/// for 3 frames, `3d 01`=317 for 20). Sent as a Feature report inside a `04 18`
/// bracket. `chunk_count` is `buffer.len() / 4096` (see [`encode`]/[`chunks`]).
pub fn begin_upload_payload(frame_count: usize, chunk_count: usize) -> [u8; 64] {
    let mut p = [0u8; 64];
    p[0] = 0x04;
    p[1] = 0x72;
    p[2] = if frame_count > 1 { 0x07 } else { 0x02 };
    p[8] = (chunk_count & 0xff) as u8; // chunk count, low byte
    p[9] = ((chunk_count >> 8) & 0xff) as u8; // high byte (16-bit LE)
    p
}

/// The `04 18` "open" command payload that precedes [`begin_upload_payload`].
pub fn open_payload() -> [u8; 64] {
    let mut p = [0u8; 64];
    p[0] = 0x04;
    p[1] = 0x18;
    p
}

/// Decode image/GIF bytes into frames + per-frame durations (device units = ms/2).
/// Animated GIFs yield every frame with its delay; other formats yield one still.
#[cfg(feature = "imageload")]
pub fn load_frames_from_bytes(data: &[u8]) -> Result<(Vec<SrcFrame>, Vec<u8>), String> {
    let to_px = |w: u32, h: u32, raw: &[u8]| -> SrcFrame {
        let px = raw
            .chunks_exact(4) // RGBA8
            .map(|p| Color { r: p[0], g: p[1], b: p[2] })
            .collect();
        (w as usize, h as usize, px)
    };
    if data.starts_with(b"GIF") {
        use image::{codecs::gif::GifDecoder, AnimationDecoder};
        let dec = GifDecoder::new(std::io::Cursor::new(data))
            .map_err(|e| format!("decode gif: {e}"))?;
        let frames = dec
            .into_frames()
            .collect_frames()
            .map_err(|e| format!("read gif frames: {e}"))?;
        if !frames.is_empty() {
            let mut out = Vec::with_capacity(frames.len());
            let mut durs = Vec::with_capacity(frames.len());
            for f in &frames {
                let (num, den) = f.delay().numer_denom_ms();
                let ms = if den == 0 { 100 } else { num / den.max(1) };
                let ms = if ms == 0 { 100 } else { ms }; // some GIFs report 0
                durs.push((ms / 2).clamp(1, 255) as u8); // device unit = ms/2
                let img = f.buffer();
                out.push(to_px(img.width(), img.height(), img.as_raw()));
            }
            return Ok((out, durs));
        }
    }
    let img = image::load_from_memory(data)
        .map_err(|e| format!("decode image: {e}"))?
        .to_rgba8();
    Ok((vec![to_px(img.width(), img.height(), img.as_raw())], vec![STILL_DURATION]))
}

/// Read an image/GIF file and decode it (see [`load_frames_from_bytes`]).
#[cfg(feature = "imageload")]
pub fn load_frames(path: &str) -> Result<(Vec<SrcFrame>, Vec<u8>), String> {
    let data = std::fs::read(path).map_err(|e| format!("read {path:?}: {e}"))?;
    load_frames_from_bytes(&data)
}

/// Upload a prepared buffer (from [`encode`]) to the LCD over HID. The LCD is a
/// HID interface (HidUsb), so no raw-USB/WinUSB: commands are Feature reports on
/// interface 3, pixel chunks are output reports on interface 2 (its OUT endpoint
/// is `0x03`), each gated by the device's per-chunk `0x84` ACK plus a small floor.
/// Mirrors the official app's full sequence (connect handshake + trailing `04 02`)
/// so an animation actually loops. `frame_count` picks the begin command.
#[cfg(feature = "usb")]
pub fn upload(vid: u16, pid: u16, buffer: &[u8], frame_count: usize) -> Result<String, String> {
    use hidapi::HidApi;
    let api = HidApi::new().map_err(|e| e.to_string())?;
    let open_iface = |iface: i32| -> Result<hidapi::HidDevice, String> {
        api.device_list()
            .find(|d| {
                d.vendor_id() == vid && d.product_id() == pid && d.interface_number() == iface
            })
            .ok_or_else(|| format!("HID interface {iface} of {vid:04x}:{pid:04x} not found"))?
            .open_device(&api)
            .map_err(|e| format!("open interface {iface}: {e}"))
    };
    let cmd = open_iface(3)?; // command interface
    let data = open_iface(2)?; // pixel chunks (its OUT endpoint is 0x03)

    let feature = |payload: &[u8; 64]| -> Result<(), String> {
        let mut buf = [0u8; 65]; // [report id 0][64-byte payload]
        buf[1..].copy_from_slice(payload);
        cmd.send_feature_report(&buf)
            .map_err(|e| format!("command SET_REPORT: {e}"))?;
        let mut ack = [0u8; 65];
        let _ = cmd.get_feature_report(&mut ack); // best-effort lock-step drain
        Ok(())
    };
    let mk = |pairs: &[(usize, u8)]| -> [u8; 64] {
        let mut p = [0u8; 64];
        for &(i, v) in pairs {
            p[i] = v;
        }
        p
    };
    // Connect handshake (same as the RGB driver) — needed for onboard animation.
    feature(&mk(&[(0, 0x04), (1, 0x18)]))?;
    feature(&mk(&[(0, 0x04), (1, 0x28), (8, 0x01)]))?;
    feature(&mk(&[
        (1, 0x01), (2, 0x5a), (3, 0x1a), (4, 0x07), (5, 0x01),
        (6, 0x08), (7, 0x26), (8, 0x09), (10, 0x03), (62, 0xaa), (63, 0x55),
    ]))?;
    feature(&mk(&[(0, 0x04), (1, 0x02)]))?;

    feature(&open_payload())?;
    let chunk_count = buffer.len() / CHUNK_LEN;
    feature(&begin_upload_payload(frame_count, chunk_count))?;
    std::thread::sleep(std::time::Duration::from_millis(30)); // arm the upload

    let mut n = 0;
    let mut ack = [0u8; 64];
    for chunk in chunks(buffer) {
        let mut buf = Vec::with_capacity(1 + chunk.len());
        buf.push(0x00); // report id 0
        buf.extend_from_slice(&chunk);
        data.write(&buf)
            .map_err(|e| format!("write chunk {n}: {e}"))?;
        let _ = data.read_timeout(&mut ack, 500); // per-chunk ACK (ep 0x84)
        std::thread::sleep(std::time::Duration::from_millis(20)); // floor
        n += 1;
    }
    std::thread::sleep(std::time::Duration::from_millis(150)); // commit last chunk
    feature(&mk(&[(0, 0x04), (1, 0x02)]))?; // trailing heartbeat — starts playback
    Ok(format!("uploaded {frame_count} frame(s), {n} chunks"))
}

/// Decode image/GIF bytes and upload to the LCD (caps frames at [`MAX_FRAMES`]).
#[cfg(all(feature = "usb", feature = "imageload"))]
pub fn upload_image_bytes(vid: u16, pid: u16, data: &[u8]) -> Result<String, String> {
    let (mut frames, mut durations) = load_frames_from_bytes(data)?;
    if frames.len() > MAX_FRAMES {
        frames.truncate(MAX_FRAMES);
        durations.truncate(MAX_FRAMES);
    }
    let n = frames.len();
    let buffer = encode(&frames, &durations);
    upload(vid, pid, &buffer, n)
}

/// Load an image/GIF file and upload it to the LCD.
#[cfg(all(feature = "usb", feature = "imageload"))]
pub fn upload_image_file(vid: u16, pid: u16, path: &str) -> Result<String, String> {
    let data = std::fs::read(path).map_err(|e| format!("read {path:?}: {e}"))?;
    upload_image_bytes(vid, pid, &data)
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
    fn animation_layout_matches_capture_18() {
        // 3 frames R/G/B, durations 0xc8 each → one header + 3 raw pixel blocks +
        // 0xFF trailer, total 3×65536 (capture 18).
        let red = (LCD_WIDTH, LCD_HEIGHT, vec![Color::RED; LCD_WIDTH * LCD_HEIGHT]);
        let grn = (LCD_WIDTH, LCD_HEIGHT, vec![Color::GREEN; LCD_WIDTH * LCD_HEIGHT]);
        let blu = (LCD_WIDTH, LCD_HEIGHT, vec![Color::BLUE; LCD_WIDTH * LCD_HEIGHT]);
        let f = encode(&[red, grn, blu], &[0xc8, 0xc8, 0xc8]);
        assert_eq!(f.len(), 3 * FRAME_LEN, "3 frame-slots");
        assert_eq!(&f[0..4], &[0x03, 0xc8, 0xc8, 0xc8], "byte0=frames, then durations");
        assert!(f[4..HEADER_LEN].iter().all(|&b| b == 0xff), "header pad 0xFF");
        // pixel regions back-to-back with NO per-frame header
        let at = |off: usize| [f[off], f[off + 1]];
        assert_eq!(at(HEADER_LEN), [0x00, 0xf8], "frame0 red");
        assert_eq!(at(HEADER_LEN + FRAME_PIXELS_LEN), [0xe0, 0x07], "frame1 green (no header)");
        assert_eq!(at(HEADER_LEN + 2 * FRAME_PIXELS_LEN), [0x1f, 0x00], "frame2 blue (no header)");
        assert!(
            f[HEADER_LEN + 3 * FRAME_PIXELS_LEN..].iter().all(|&b| b == 0xff),
            "0xFF trailer"
        );
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
    fn begin_upload_encodes_16bit_chunk_count() {
        // Still (16 chunks, capture 13): 04 72 02 …[8..10]=10 00.
        let s = begin_upload_payload(1, 16);
        assert_eq!((s[0], s[1], s[2]), (0x04, 0x72, 0x02), "static type");
        assert_eq!((s[8], s[9]), (0x10, 0x00), "16 chunks LE");
        // 3 frames (48 chunks, capture 18): 04 72 07 …[8..10]=30 00.
        let a = begin_upload_payload(3, 48);
        assert_eq!((a[0], a[1], a[2]), (0x04, 0x72, 0x07), "animated type");
        assert_eq!((a[8], a[9]), (0x30, 0x00), "48 chunks LE");
        // 20 frames (317 chunks, capture 19): 04 72 …[8..10]=3d 01.
        let l = begin_upload_payload(20, 317);
        assert_eq!((l[8], l[9]), (0x3d, 0x01), "317 chunks LE (>255, needs both bytes)");
    }

    #[test]
    fn long_animation_pads_to_4096_not_frame_slots() {
        // 20 frames → 256 header + 20×64800 pixels → padded to 317×4096 (capture 19),
        // NOT 20×65536.
        let frames: Vec<SrcFrame> = (0..20)
            .map(|_| (LCD_WIDTH, LCD_HEIGHT, vec![Color::RED; LCD_WIDTH * LCD_HEIGHT]))
            .collect();
        let f = encode(&frames, &[0x4b; 20]);
        assert_eq!(f.len(), 317 * CHUNK_LEN, "padded to next 4096 boundary");
        assert_ne!(f.len(), 20 * FRAME_LEN, "not N×65536");
        assert_eq!(f[0], 20, "byte0 = frame count");
        assert!(f[1..21].iter().all(|&b| b == 0x4b), "20 duration bytes");
        assert!(f[21..HEADER_LEN].iter().all(|&b| b == 0xff), "header pad");
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
