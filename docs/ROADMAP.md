# IX Forge — status & roadmap

Snapshot of where the project stands and what's next. (Companion to the per-device
protocol notes in [docs/protocols/](protocols/) and the contributor guide in
[CONTRIBUTING.md](../CONTRIBUTING.md).)

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
- **Next action (on native Windows):** capture with Wireshark + USBPcap per
  [docs/protocols/aula-f108-pro.md](protocols/aula-f108-pro.md), then the placeholder
  `sonix` encoder becomes the real one + byte-exact golden tests + real profile values.

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
