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
  W→red, all→red), `04-connect-esc-red…` (**fresh app connect** + Esc red/green — contains the
  one-time init + the command/ACK handshake missing from 02/03) — local only, git-ignored.

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

### Command/ACK handshake — **DECODED** (from `04-connect-esc-red.pcapng`)
This is the piece that was missing when writes were "accepted" but nothing lit.

- **Every control command is followed by a device READ.** After each `04 xx` / `80`
  control write the app issues a **`GET_REPORT`** (`bmRequestType 0xa1`, `bRequest 0x01`,
  `wValue 0x0300` Feature/ID 0, `wIndex 3`, `wLength 64`) to drain the device's ACK. The
  **8 data reports stream with no reads**; there is **one read after the commit**. `04 f0`
  gets no read. (Capture 04: 139 reads interleaved among 565 writes.)
  → If you only write and never read, every `SET_REPORT` still succeeds at the USB layer
  ("16/16 accepted") but the firmware's command parser desyncs and the frame never latches
  → **nothing lights.** Reading the ACK after each control command is the fix.
- **One-time connect handshake** (app startup, before any frame; capture frames 1095–1151):
  1. `04 18 …`                                   → read ACK
  2. `04 28 …[byte8]=01`                          → read ACK
  3. `00 01 5a 1a 07 01 08 26 09 00 03 … [62..63]=aa 55`  (config packet, committed) → read ACK
  4. `04 02 …` (heartbeat)                        → read ACK
- **Captures 02/03 lacked this** because they started mid-session (after the app had already
  connected), which is why the replayed per-apply sequence alone didn't light the board.

### LED-matrix warmup — **DECODED / PROVEN ON HARDWARE** (from `04-…`)
The connect handshake + a byte-perfect static apply (matching commands, data, timing, and even
the device's ACK bytes) still left the board **dark**. The one remaining difference: right after
connect the app streams its default **effect** for ~1.5 s — repeated `04 20 …[byte8]=08` frames,
each a **densely-packed** `[led_index,R,G,B]` list (all real LEDs, 16/report, ~7 reports, no
INDEX_MAP gaps) — *before* the first static apply. Replaying the exact capture opening
(connect → effect stream → static apply) **lit the board** (rainbow, then solid red). ⇒ streaming
frames performs an implicit **LED-matrix power-on** that persists; static applies only display on
an already-powered matrix. After warmup, static applies work with only heartbeats between them
(no further effect frames — capture has 6 static latches spanning frames 3297–7245 with none).
- **Effect frame framing:** `04 20 …[byte8]=08`, then dense `[idx,R,G,B]×16` data reports, then an
  all-zero report; same command/ACK read cadence and ~33 ms pacing as the static path.
- **Open cadence detail:** the app paces **every** report (writes, reads, and each data report)
  ~33–36 ms apart; streaming faster than the device drains → frame never renders.
- **TODO (minimize):** find the smallest warmup that powers the matrix (1 effect frame? bare
  `04 20`?) so the driver need not flash a full rainbow on connect.

### TWO LIGHTING PATHS — **DECODED / PROVEN ON HARDWARE**
The board has an **onboard saved profile** (survives unplug/replug with no software running —
confirmed: replug shows the last app-saved color). There are two distinct write paths:

1. **Effect / live-display path (`04 20`)** — **this is how software drives the LEDs live.**
   `04 20 …[byte8]=08` + a **dense** `[led_index,R,G,B]×16` stream (all real LEDs, ~7 reports,
   then an all-zero terminator), each report ~33 ms apart with the command/ACK read cadence.
   **Streaming these continuously holds a stable, clean color** (proven: `tools/f108_effect.py`
   streamed solid red indefinitely). Records are self-describing by index, so packing order is
   free. The last streamed frame **persists after streaming stops until a keypress**, which makes
   the onboard controller redraw its saved profile → so robust display requires **continuous
   re-streaming** (a background loop), exactly as the official app does while open.
2. **Static / onboard-write path (`04 13`/`04 23`)** — the sequence decoded first. On a board
   already in software-display mode (e.g. after effect streaming) it displays; but on a fresh
   board sitting on its saved profile it is **ignored** (writes ACK fine, panel never changes,
   reverts to the saved color). ⇒ this path writes the onboard profile and needs a **save/commit**
   step (not yet captured) to take effect + persist. This is a **config write, not firmware** —
   safe and in scope; it's how the app makes a color survive replug.

**Driver model:** connect handshake → **stream frames via the `04 20` path in a background loop**
(~33 ms) for live color/effects while the app is open. This is implemented and proven.

### Persistence / "save to keyboard" — **PARTLY DECODED, OPEN** (from `05-…`)
`05-persistence-capture` = the app on a board sitting on its saved (blue) profile, changed to
solid red; red **survived unplug/replug**, so the app writes onboard memory. Findings:
- The save uses **only the static `04 13`/`04 23` path** — no `04 20` effect frame at all — plus a
  one-time setup packet `01 ff … 01 05 03 … aa 55` and two full static applies (current color, then
  the new one). There is **no separate save/commit command**: the last write is just the apply's
  `04 f0` trailer.
- But the static apply is **ignored when replayed in isolation** (bare static apply → no display,
  no persist; the board stays on its onboard color). The `01 05 03` setup packet, replayed alone,
  puts the board into an **effect (radial rainbow)**, not a static color — so `05` there is likely
  an effect index, not "static mode".
- The session is **read-heavy**: 89 writes vs **263 `GET_REPORT` reads**. ⇒ the static save only
  "takes" inside the app's live, continuously-polled session.
- **Tried and ruled out** (none persisted after replug): bare static apply in isolation; static
  apply + the `01 05 03` setup (→ rainbow); a verbatim slice of `05`'s opening (→ rainbow); and
  the static apply sent **inside an active `04 20` software session** (shows the color live but does
  **not** write onboard). ⇒ the commit is almost certainly **gated on the specific `GET_REPORT`
  responses** the app reads during the save (drain-and-proceed), which none of the above reproduce.
- **OPEN — next experiment:** replay capture `05` in its **entirety** (all 89 writes AND 263 reads,
  in order, ~33 ms), recoloring only the data reports, and confirm it persists. If it does, bisect
  to the minimal save; if not, the reads' *return values* matter and we decode those.
- Live display via the `04 20` effect stream is unaffected and already works.

**Key → led_index map: DONE** — all 104 mapped empirically with `forge-cli probe` (see
`profiles/aula/f108-pro.toml`). Note the earlier capture guess `W=0x4b` was wrong: **W=0x27**,
`0x4b`=X.

### On-device effects — **DECODED / PROVEN ON HARDWARE** (from `07-capture-effects`)
Built-in animations are **onboard** and selected by a **single command** (no streaming; the
board animates on its own MCU and keeps running after the host disconnects — confirmed):
`04 18` → `04 13 [8]=01` → **effect packet** → `04 f0`, each ACK-read, ~33 ms paced. The effect
packet is: `[b0=effect_id] ff 00 00 00 00 00 00 [b8=01] [b9=speed] [b10=brightness] … [62..63]=aa 55`.
- Unlike the static *color save*, this **works standalone** (proven: `tools/f108_effect_select.py 3`
  selected a reactive keypress effect and it kept running). So `set_effect` is implementable.
- speed/brightness are device **levels** (seen 0x05 / 0x03; brightness went 03→05 when raised);
  byte8 and byte2 (direction/variant) toggle for some effects.
- **Effect id → animation mapping is empirical** — the profile's guessed order is WRONG (id 3 is a
  reactive "press-to-light", not "spectrum"). Sweep with `f108_effect_select.py --sweep` to map ids
  → names, then rewrite the profile's `effects` list.
- **Effect COLOR — DECODED (capture `08`):** a color-based effect's color is set by a *second*
  bracketed command with the same `[id]` but **byte1 = `00`** (vs `ff` for select) and **RGB at
  bytes 2/3/4**: `[id, 00, R, G, B, …, speed@9, brightness@10, aa55@14-15]`. So configuring e.g.
  Breathe-in-green = select packet + this color packet. Rainbow effects (colorful/spectrum/outward/
  scrolling/rolling/rotating) don't take a color.
- **Direction = byte 11**, **randomize = byte 8** of the select packet (capture `08`: Rolling
  toggled byte11 0↔1, Single On toggled byte8 0↔1). Wired as EffectSelection options.
- **NB:** the `aa 55` commit is at **bytes 14-15** for effect packets (not 62-63 like the RGB
  frame commit).
- **CORRECTED effect packet layout — DECODED / PROVEN ON HARDWARE (captures `10`,`11`):**
  the effect config is a **single** bracketed command (no separate select vs color packet):
  `[id, R@1, G@2, B@3, .., randomize@8, brightness@9, speed@10, direction@11, .., aa55@14-15]`.
  - **Color is plain RGB at bytes 1/2/3** — capture `11` (Breathe): red=`07 ff 00 00`,
    green=`07 00 ff 00`, cyan=`07 00 ff ff`. NOT a hue, NOT bytes 2/3/4. (Earlier wrong guesses:
    a "select" packet with byte1=`0xff` actually meant R=255 → forced red on every re-select;
    a hue-in-byte1 attempt with byte3 pinned to `0xff` made everything read blue↔purple.)
  - **byte9 = brightness, byte10 = speed** (level 1..5). These were initially swapped — proven on
    hardware (Speed slider changed brightness). The earlier capture-08 note had them reversed.
  - **byte8 = 1** → rainbow (flow effects) / per-key random (reactive effects); board ignores RGB.
  - Re-sending this one packet on any change keeps the color, so there is no "color-only" path.

### LCD (1.14" TFT) — image upload **DECODED** (from captures `13`–`17`)
Confirmed by uploading solid red/blue/green + split test GIFs and diffing:
- **Resolution `240 × 135`, pixel format RGB565 little-endian.** Red=`f800`→bytes `00 f8`,
  blue=`001f`→`1f 00`, green=`07e0`→`e0 07`. Each solid image = exactly 32,400 matching words.
- **Scan order: row-major, top→bottom, left→right** (top-red/bottom-blue → red rows first;
  left-red/right-green → red/green alternating every 120 px within each row).
- **Buffer (65,536 bytes)** = **256-byte header** (`01 05` then 254×`0xFF`) + **240×135×2 = 64,800
  bytes** of RGB565-LE pixels (offset 256) + **480-byte `0xFF` trailer** (pad to 65,536). Sent as
  **16 × 4096-byte chunks on interrupt OUT endpoint `0x03`** — NOT a HID Feature report; a raw
  endpoint, so the driver needs raw-USB/bulk for the chunks while the commands below stay HID
  Feature reports.
- **Upload sequence** (Feature-report commands ACK-read, same lock-step as RGB): connect
  handshake → `04 18` (open) → **`04 72 02 …[byte8]=0x10`** (begin image upload; `0x10`=16 = the
  chunk count) → the 16 pixel chunks on ep `0x03`.
- **TODO:** the exact display/commit after the chunks (a `04 02` heartbeat follows; no `04 f0`
  close seen — confirm whether the image latches on the last chunk or needs a commit); GIF
  animation (multi-frame) and text/system-monitor "cards"; whether the 736-byte header is fixed
  or content-dependent (verify with a photo).

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
