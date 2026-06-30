# Contributing to IX Forge

Thanks for helping build IX Forge. Two rules matter more than anything else here:
the **clean-room policy** and the **no-firmware policy**. Please read both.

## Clean-room policy (non-negotiable)

IX Forge is permissively licensed (MIT OR Apache-2.0). To keep it that way, all
device-protocol knowledge must come from **your own USB captures of hardware you
own**, or from manufacturer documentation you are permitted to use.

- **Do not** read, copy, port, paraphrase, or "reference while typing" code from
  GPL/copyleft projects such as OpenRGB, ckb-next, or similar. Their *existence*
  can confirm a device is supportable, but their **source must not inform ours**.
- Record provenance for every device in `docs/protocols/<device>.md` (who
  captured, when, firmware revision, capture files).
- The `cargo deny check` CI job rejects any copyleft dependency. Don't add one.

If you've previously studied a GPL implementation of a protocol, please don't be
the one to contribute that protocol's driver.

## No-firmware policy

IX Forge writes **only reversible configuration reports** (RGB, macros, LCD). It
must never write to a device bootloader or flash firmware — that risks bricking
hardware. PRs touching ISP/bootloader flows will not be accepted.

## Adding a device

Most devices need **no Rust** if they're in a supported controller family:

1. Capture and decode the protocol (see `docs/protocols/aula-f108-pro.md` for the
   template and the workflow).
2. Add `profiles/<vendor>/<model>.toml` — matcher, `driver.variant` knobs, and
   the full LED `layout` (key positions). Optionally add a UI `ChassisSpec` in
   `app/src/rgb/deviceArt.ts` for the case/knob/screen rendition.
3. Add a golden-byte test asserting the driver encodes a known command to the
   exact captured bytes (use `forge_transport::MockTransport`).

**Reference images & the rendition.** The keyboard rendition is drawn from
*coordinates*, not images. To trace a new board's layout you may use a reference
photo, but **don't commit third-party product images** (they're copyrighted — see
the clean-room policy). Keep references in `assets/refs/` (gitignored) and commit
only the derived data (the profile layout + chassis spec). Use your own photo, or
one you have rights to, as the reference.

A brand-new controller family is a new module in `crates/forge-drivers/`
implementing the `Driver`/`DeviceSession` traits from `forge-core`.

## Development setup

Most logic is hardware-free and runs anywhere:

```sh
cargo test -p forge-core -p forge-profiles -p forge-drivers -p forge-registry -p forge-macro
cargo test -p forge-transport --no-default-features   # MockTransport only
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo deny check
```

On-hardware work (USB capture with USBPcap + Wireshark, and testing real
devices) is done on **native Windows** — the primary target. Inside WSL2, use the
`MockTransport` paths above; USB HID is not visible without `usbipd-win`.

## Code style

- `forge-core` stays pure: no I/O, no platform deps. Backends live at the edges.
- Keep drivers thin; push device differences into profile `variant` data.
- Match the surrounding code's naming and comment density.
