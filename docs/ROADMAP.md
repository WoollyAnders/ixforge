# IX Forge — status & roadmap

Snapshot of where the project stands and what's next. (Companion to the per-device
protocol notes in [docs/protocols/](protocols/) and the contributor guide in
[CONTRIBUTING.md](../CONTRIBUTING.md).)

> **Resuming after a PC/session restart?** Do the capture below — it's the one gating
> step. Everything else is built (see "What's built"). When the capture is done, hand
> the files back and the placeholder protocol becomes real.

## ▶ Resume here after restart — capture the protocol (Wireshark + USBPcap)

The F108 Pro uses a **proprietary Sonix protocol** (VID `0x0C45`; it is **not** VIA, verified
via usevia.app), so we capture the official software's USB traffic and decode it. Do this on
**native Windows** with the keyboard connected **wired (USB-C)**.

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
| `01-init` | just launch / connect (baseline handshake) |
| `02-esc-red` | set only **Esc** → red |
| `03-esc-green` | set **Esc** → green |
| `04-key1-red` | set the **next** key (e.g. `1`) → red |
| `05-all-blue` | set **all** keys → blue |
| `06-brightness` | change brightness only |
| `07-effect` | pick one built-in effect |
| `08-lcd-image` *(later)* | upload an image to the screen |
| `09-macro` *(later)* | record/assign a macro |

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

## ⚠️ The one thing that's still placeholder: the real protocol

`set_rgb`/`set_effect` currently emit **structurally-correct placeholder bytes**, and the
F108 Pro profile's VID/PID, `led_index`, report IDs/opcodes, and color order are `TODO`.
Nothing actually lights up on hardware yet — that's **gated on a USB capture**.

- **The F108 Pro is NOT VIA-compatible** (verified: usevia.app does not see it) — it uses a
  **proprietary Sonix protocol** (VID `0x0C45`). So the path is capture + decode.
- **Next action:** the Wireshark + USBPcap capture playbook at the top of this file. Once the
  captures are in, the placeholder `sonix` encoder becomes the real one + byte-exact golden
  tests + real profile values.

## Roadmap

- **M1 — RGB breadth:** UI/persistence/hotplug/zones/brightness **done**; real RGB is the only
  piece left, blocked on the capture above.
- **M2 — Macros:** recorder/editor + on-device write (needs a macro capture).
- **M3 — LCD:** push image/text/system-monitor to the 1.14″ screen (needs a capture; hardest).

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
