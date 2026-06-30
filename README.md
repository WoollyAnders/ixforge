# IX Forge

Open-source configurator for third-party gaming keyboards — **RGB lighting, macros, and on-keyboard LCD screens** — that ship with poor or no first-party software. Built with Rust + Tauri, Windows-first, cross-platform by design.

IX Forge is a peripheral-configuration platform. The first target is keyboards (starting with AULA models, which use the widely reverse-engineered SinoWealth controller family); future modules cover mice and screen/wallpaper content.

## Principles

- **Data-driven devices** — adding a keyboard model is mostly a profile file plus a thin protocol adapter.
- **Capabilities, not device types** — the UI renders controls from what a device advertises (`Rgb`, `Macro`, `Lcd`), never from a hard-coded model list.
- **Reversible only** — IX Forge only writes reversible configuration reports over HID. It **never** touches device firmware or bootloaders (no flashing, no brick risk).
- **Clean-room** — protocol support is derived solely from the maintainers' own USB captures of hardware they own. No GPL/copyleft code is used or referenced.

## Workspace layout

```
crates/
  forge-core/       Pure domain: capability model, DeviceProfile, Driver/DeviceSession + HidTransport traits. No I/O.
  forge-transport/  HID I/O: HidapiTransport + MockTransport implementing forge-core's HidTransport.
  forge-profiles/   Device profile loading/validation + user config persistence.
  forge-drivers/    Per-controller-family protocol encoders (sonix, sinowealth, ...).
  forge-registry/   Enumeration, profile/driver matching, hotplug, per-device actors.
  forge-macro/      Macro AST + host-side replay engine (feature-gated).
  forge-cli/        Developer CLI: fire commands at real hardware during bring-up.
app/                React + TypeScript + Vite front end (capability-driven UI).
app/src-tauri/      Tauri app (the `forge-app` crate): IPC commands + the binary.
profiles/           Device profiles (TOML), embedded + shippable.
docs/protocols/     Clean-room protocol notes per device.
```

## Status

Early foundation. Roadmap: **M0** RGB vertical slice → **M1** RGB breadth → **M2** macros → **M3** LCD.

## Development

Most logic is hardware-free and testable anywhere (`MockTransport`):

```sh
cargo test -p forge-core -p forge-profiles -p forge-drivers -p forge-registry -p forge-macro
cargo deny check      # enforce permissive-only dependency policy
```

Run the desktop app (Tauri + React):

```sh
cd app && pnpm install && pnpm tauri dev
```

The front end also runs in a plain browser with a **mock device** (no hardware/Tauri):

```sh
cd app && pnpm dev      # open the printed localhost URL
```

Fire a command at real hardware during protocol bring-up:

```sh
cargo run -p forge-cli -- set-rgb --key KC_ESC --color ff0000
```

On-hardware testing and USB protocol capture (USBPcap + Wireshark) happen on **native Windows**.
The Tauri app and `forge-cli` need system libraries: webview (`webkit2gtk` on Linux; bundled on
Windows/macOS) and `libudev` on Linux for HID. The committed `Cargo.lock` pins `time` to a release
compatible with Tauri's `cookie` dependency.

## License

Dual-licensed under either of [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT) at your option.

---

*Not affiliated with, endorsed by, or sponsored by AULA or any keyboard manufacturer. Third-party trademarks are used nominatively only, to indicate supported hardware.*
