# IX Forge — status & roadmap

Snapshot of where the project stands and what's next. (Companion to the per-device
protocol notes in [docs/protocols/](protocols/) and the contributor guide in
[CONTRIBUTING.md](../CONTRIBUTING.md).)

> **Status: the entire RGB chapter is DECODED, PROVEN ON HARDWARE, and shipped** — per-key
> color, all 18 onboard effects, and per-effect color (RGB) / speed / brightness / direction /
> rainbow, all driven live from the GUI. Protocol in [docs/protocols/aula-f108-pro.md](protocols/aula-f108-pro.md).
> **The next chapter is the LCD screen** — same capture→decode method, playbook below.

## ▶ Next capture: the LCD screen (Wireshark + USBPcap)

The F108 Pro uses a **proprietary Sonix protocol** (VID `0x0C45`; it is **not** VIA, verified
via usevia.app), so we capture the official software's USB traffic and decode it. Do this on
**native Windows** with the keyboard connected **wired (USB-C)**. This same method decoded all of
RGB; the LCD is expected to be the largest protocol (image framing, chunking, addressing).

**1. Install (once).**
- Install **Wireshark** (Windows 64-bit) from wireshark.org — keep the defaults (Wireshark, TShark, tools).
- When prompted, install **Npcap** (defaults; it's the network driver, required generally, harmless here).
- **USBPcap:** recent Wireshark installers include it as a checkbox on the *Choose Components* screen — **check it**. If it isn't offered, install **USBPcap** separately from usbpcap.com.
- **Reboot** — USBPcap is a kernel driver and won't appear until you restart.

**2. Verify + identify the device.**
- Plug the F108 Pro in (wired). Launch **Wireshark as Administrator**.
- The interface list should now show **`USBPcap1`, `USBPcap2`, …**. (Missing? reboot / run as admin.)
- Note the VID:PID: Device Manager → the AULA HID device → *Details → Hardware IDs* (expect `VID_0C45&PID_xxxx`).
- USBPcap captures a whole **root hub**; pick the `USBPcapN` whose device tree includes the AULA keyboard.

**3. Capture — one change per file.** Open AULA's official F108 Pro software. For each item: start
the capture on the right `USBPcapN`, make **exactly one** change in the app, stop, then
**File → Save As** with the given name (keep each capture short — a few seconds around the change):

| File | Change to make in the official app |
|---|---|
| `13-lcd-image` | upload one **image** to the LCD screen |
| `14-lcd-image2` | upload a **different** image (diff reveals header vs pixel data) |
| `15-lcd-text` *(if supported)* | set the screen to a **text** / clock mode |
| `16-lcd-monitor` *(if supported)* | enable the **system-monitor** display |

Captures `01`–`11` (RGB init/keys/effects/effect-color) are already decoded and done. LCD is next;
two different images help separate the fixed header/addressing from the pixel payload.

**4. Hand it back.** Save the `.pcapng` files under **`captures/aula-f108-pro/`** in the repo
(that folder is git-ignored, so they stay local — perfect for clean-room). To make decoding fast,
also copy the raw write bytes: in Wireshark, select an **outgoing** packet to the keyboard
(`URB_CONTROL out` / `SET_REPORT`, or an interrupt OUT), find the **HID data / "Leftover Capture
Data"** field, right-click → **Copy → … as a Hex Stream**, and paste those into
[docs/protocols/aula-f108-pro.md](protocols/aula-f108-pro.md). Then tell the next session: the
confirmed **VID:PID**, the **software version**, and where the files are — it will diff the
captures, replace the placeholder `sonix` encoder, add byte-exact tests, and fill the real profile.

## What's built and working

**Backend (Rust workspace, all tested, clippy-clean):**
- `forge-core` — capability model, `DeviceProfile`, `Driver`/`DeviceSession` + `HidTransport` traits.
- `forge-transport` — `HidapiBackend` + `MockTransport`.
- `forge-profiles` — TOML profile loading/matching + **user preset persistence** (`PresetStore`).
- `forge-drivers` — `sonix` + `sinowealth` families over shared framing; RGB per-key, effects, **zones**.
- `forge-registry` — enumerate → match → open session, + `DeviceWatcher` (hotplug diff).
- `forge-cli` — fire `set-rgb` / `fill` / `effect` at hardware during bring-up.
- `forge-app` (Tauri) — IPC: `list_devices`, `get_capabilities`, `set_rgb`, `set_effect`,
  `list/save/delete_preset`; background **hotplug** poller emitting `device://attached|detached`.

**Front end (React + Tauri, dark-purple/cyan theme):**
- Capability-driven UI; a **live vector keyboard rendition** (case + knob + LCD + side-light,
  drawn from data — no images) that animates effects and serves as the per-key editor.
- Per-key paint (toggle-clear), built-in effect picker + animated preview, **zone editor**,
  **global brightness**, **named presets** (save/apply/delete), **live hotplug** device list.
- Runs in a browser with a **mock** F108 Pro + a mock zoned strip (no hardware needed).

## RGB: real and proven (no longer placeholder)

The `sonix` driver drives real hardware. Decoded + working: connect handshake, the `04 20`
live-display stream (continuous re-stream loop holds color), the full 104-key `led_index` map,
and the onboard effect command — one bracketed packet
`[id, R@1, G@2, B@3, randomize@8, brightness@9, speed@10, direction@11, .., aa55@14-15]`
(color is plain RGB at bytes 1/2/3; speed higher = faster; byte8=1 = rainbow/random). Byte-exact
golden tests cover it. VID `0x0C45` / PID `0x800A`, interface 3, 64-byte Feature reports, report id 0.

## Roadmap

- **M1 — RGB breadth:** ✅ **DONE** — UI/persistence/hotplug/zones/brightness + real per-key color,
  18 effects, and effect color/speed/brightness/direction/rainbow, all proven on hardware.
- **M2 — Macros:** recorder/editor + on-device write (needs a macro capture: record/assign one macro).
- **M3 — LCD:** ✅ **image + animated GIF DONE, with in-app UI** (240×135 RGB565, HID output
  reports on iface 2, per-chunk `0x84` ACK; animation = one header + raw frames + per-frame
  durations). Proven on hardware via the **Screen tab** (pick file → preview → send) and
  `forge-cli lcd --image <gif|png|jpg>`. Transport is shared, feature-gated in forge-drivers
  (`usb`/`imageload`). Left: text / system-monitor "cards" (own captures).

## Deferred (non-blocking)

- **`ts-rs` type generation** — replace the hand-written `app/src/types/forge.ts` with types
  generated from the Rust core to prevent drift. Deferred deliberately: it's a cross-crate
  refactor that must reproduce serde's exact JSON shapes at runtime; best done in a live
  session where the result can be verified interactively. The hand-written types are correct.
- **Windows release CI** (signed installer) and macOS/Linux packaging.

## Dev workflow

```sh
# hardware-free (any OS):
cargo test -p forge-core -p forge-profiles -p forge-drivers -p forge-registry -p forge-macro
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
cargo deny check

cd app && pnpm install && pnpm dev        # browser preview (mock devices)
cd app && pnpm tauri dev                  # full desktop app (needs webview libs)
cargo build -p forge-app -p forge-cli     # needs webkit2gtk (Linux) / libudev for hidapi
```

Protocol capture + on-hardware testing happen on **native Windows**. `Cargo.lock` pins `time`
to a release compatible with Tauri's `cookie` dependency — keep it.
