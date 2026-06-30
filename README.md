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
  forge-app/        Tauri app: IPC commands, events, the only binary.
app/                React + TypeScript + Vite front end.
profiles/           Device profiles (TOML), embedded + shippable.
docs/protocols/     Clean-room protocol notes per device.
```

## Status

Early foundation. Roadmap: **M0** RGB vertical slice → **M1** RGB breadth → **M2** macros → **M3** LCD.

## Development

Most logic is hardware-free and testable anywhere (`MockTransport`):

```sh
cargo test            # core, transport, drivers (golden-byte fixtures)
cargo deny check      # enforce permissive-only dependency policy
```

On-hardware testing and USB protocol capture (USBPcap + Wireshark) happen on **native Windows**.

## License

Dual-licensed under either of [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT) at your option.

---

*Not affiliated with, endorsed by, or sponsored by AULA or any keyboard manufacturer. Third-party trademarks are used nominatively only, to indicate supported hardware.*
