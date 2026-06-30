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
| Controller | **Sonix** (SN32F2xx-class) — inferred from VID `0x0C45`; *confirm* |
| VID:PID | `0x0C45` : `0x800A` — *confirm in wired mode* |
| Config interface / usage page | *TODO* (several HID interfaces; Windows showed iface 3) |
| Extras | 1.14" TFT screen, multifunction knob, per-key RGB (104 LEDs) |
| Connectivity | Tri-mode (BT / 2.4GHz / USB-C) |
| Captured firmware revision | *TODO* |

## ⚠️ Capture in WIRED mode only

The official software is **Windows + USB-C wired only**. The 2.4GHz dongle and
Bluetooth present different USB identities and the config app won't drive them.
Plug in by cable, confirm the app sees the keyboard, then capture.

## Provenance

- Captured by: *TODO*
- Date: *TODO*
- Official software version: *TODO* (from aulakeyboard.com "F108 Pro Drive")
- Capture files: `captures/aula-f108-pro/*.pcapng` *(not committed; large)*

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

## Findings (fill in from packet diffs)

### RGB — per-key color write
- Report mechanism: *TODO — feature vs output report*
- Report ID / length: `0x__` / `__` bytes
- Opcode, key/LED index field, color payload offset, channel order: *TODO*
- Paging across multiple reports? offset field: *TODO*
- Checksum: *TODO*
- Init / commit sequence: *TODO*

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
