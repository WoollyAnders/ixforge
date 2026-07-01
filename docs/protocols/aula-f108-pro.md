# Protocol notes — AULA F108 Pro (Sonix)

> **Clean-room artifact.** Everything here must be derived from *your own* USB
> captures of the official software for an AULA F108 Pro *you own*. Do not
> transcribe bytes or tables from GPL projects (e.g. OpenRGB issue #5253 exists
> for this board — use it only to confirm the device is supportable, never as a
> source). Record provenance below so the derivation is auditable.

## Device

| Field | Value |
|---|---|
| Model | AULA F108 Pro (full-size, 104 keys) |
| Controller | **Sonix** (VID `0x0C45`) — confirmed (device addr 11 in capture) |
| VID:PID | `0x0C45` : `0x800A` — **confirmed** |
| Config interface | **interface 3**, HID **Feature** reports (`SET_REPORT`), report ID 0, 64 bytes — **confirmed** |
| Extras | 1.14" TFT screen, multifunction knob, per-key RGB (104 LEDs) |
| Connectivity | Tri-mode (BT / 2.4GHz / USB-C) |
| Captured firmware revision | *TODO* |

## ⚠️ Capture in WIRED mode only

The official software is **Windows + USB-C wired only**. The 2.4GHz dongle and
Bluetooth present different USB identities and the config app won't drive them.
Plug in by cable, confirm the app sees the keyboard, then capture.

## Provenance

- Captured by: WoollyAnders (device owner)
- Date: 2026-07-01
- Official software version: *TODO*
- Capture files: `captures/aula-f108-pro/02-…` (idle/handshake), `03-…` (Esc→red, Esc→green,
  W→red, all→red) — local only, git-ignored.

## Capture log

One variable per capture:

| File | What changed in the official app |
|---|---|
| `01-init.pcapng` | (baseline) plug in wired, app launch handshake |
| `02-esc-red.pcapng` | Esc → `#ff0000`, everything else off |
| `03-esc-green.pcapng` | Esc → `#00ff00` |
| `04-key1-red.pcapng` | the *next* key → `#ff0000` (find the index field) |
| `05-all-blue.pcapng` | all keys → `#0000ff` |
| `06-brightness.pcapng` | brightness only |
| `07-effect.pcapng` | select a built-in effect via the app or knob |
| `08-lcd-image.pcapng` | upload an image to the TFT screen |
| `09-macro.pcapng` | record/assign one macro |

## Findings

### RGB — per-key color write — **DECODED** (from `03-…`)
- **Transport:** `SET_REPORT` (`bmRequestType 0x21`, `bRequest 0x09`), `wValue 0x0300`
  (**Feature** report, **report ID 0**), `wIndex 3` (**interface 3**), `wLength 64`.
  → 64-byte **Feature reports on interface 3**.
- **LED frame:** the full per-key buffer is **8 consecutive Feature reports**, each = **16
  records × 4 bytes** = `[led_index, R, G, B]` (color order **RGB**, one byte each). Slots with
  no LED are `00 00 00 00`. Real LED indices run `0x01 .. 0x7b`; **index `0` = none**.
- **Proof** (same slot, `led_index = 1` = Esc):
  - Esc→red → record `01 ff 00 00`
  - Esc→green → record `01 00 ff 00`  ⇒ byte1 = R, byte2 = G, byte3 = B.
  - `W`→red → record at index `0x4b` (report 5). *(captured W value was `ff 00 fc` — the
    app's picker wasn't pure red; the `[idx,R,G,B]` layout still holds.)*
- **Upload sequence per "apply"** (all 64-byte Feature writes on iface 3, in order):
  1. `04 18 …`
  2. `04 13 …[byte8]=01`
  3. `80 … [byte9]=05 … [byte14..15]=aa 55`  (handshake/marker)
  4. `04 f0 …`
  5. `04 18 …`
  6. `04 23 …[byte8]=09`  (begin LED data)
  7. **8× LED-data reports** (the `[idx,R,G,B]×16` frame above; indices `0x00..0x7b`)
  8. `00 … [byte62..63]=aa 55`  (commit)
  9. `04 f0 …`
- **Checksum:** none evident (color bytes change with nothing else moving).
- **Idle heartbeat (ignore):** app alternates `04 02 …` / `04 f5 …[byte8]=09` while idle.

**Still needed:** the full **key → led_index** map (only `Esc=0x01`, `W=0x4b` known so far).
Fastest way to get it: single-key captures walking across the board, or one capture per row.
For now the encoder can be built + golden-tested against these captures; per-key painting is
correct for known keys and lands fully once the map is complete.

### LCD (1.14" TFT)
- Resolution / orientation / pixel format (RGB565?): *TODO*
- Upload framing: header, chunking, addressing: *TODO*
- Text vs image vs system-monitor modes: *TODO*

### Macros (on-device)
- Slot count, slot-write framing, event encoding: *TODO*

### Knob
- Does the knob send config reports or just HID consumer events? *TODO*

## Encoding → driver mapping

1. `profiles/aula/f108-pro.toml` — fill `matcher` (add usage_page!), the
   `driver.variant` knobs, the full 104-key `layout`, macro slot count, and LCD
   dimensions.
2. `crates/forge-drivers/src/sonix/` — adjust framing if the device differs from
   the placeholder (header length, checksum, commit report); add LCD/macro paths.

## Verification

- Golden test: assert the Sonix driver encodes `SetKeys([(KC_ESC, RED)])` to the
  exact payload bytes from `02-esc-red.pcapng`, using `MockTransport`.
- On-hardware: `forge-cli set-rgb --device aula.f108-pro --key KC_ESC --color ff0000`
  → the Esc key turns red (wired).
