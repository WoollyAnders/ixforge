//! `forge-cli` — a tiny developer tool for driving real hardware during protocol
//! bring-up, without going through the GUI.
//!
//!   forge-cli list
//!   forge-cli set-rgb --device <id> --key KC_ESC --color ff0000
//!   forge-cli fill --color 0000ff [--device <id>]
//!
//! `--device` is optional; it defaults to the first matched device. Profiles are
//! loaded from the repo `profiles/` tree. Requires a real, wired, supported device.

use std::collections::HashMap;

use forge_core::{Color, EffectSelection, KeyId, RgbCommand};
use forge_profiles::ProfileCatalog;
use forge_registry::{match_devices, open_matched, MatchedDevice};
use forge_transport::hidapi_backend::HidapiBackend;
use forge_transport::HidBackend;

const PROFILES_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../profiles/aula");

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let command = args.get(1).map(String::as_str).unwrap_or("help");
    let opts = parse_opts(&args);

    let catalog = ProfileCatalog::from_dir(PROFILES_DIR).map_err(|e| e.to_string())?;
    let backend = HidapiBackend::new().map_err(|e| e.to_string())?;
    let infos = backend.enumerate().map_err(|e| e.to_string())?;
    let matched = match_devices(infos, &catalog);

    match command {
        "list" => {
            if matched.is_empty() {
                println!("No matching devices. Connect a supported keyboard (wired).");
            }
            for m in &matched {
                println!(
                    "{}\t{}\t[{}]",
                    forge_registry::device_id(&m.info).0,
                    m.profile.display_name,
                    m.info.product.clone().unwrap_or_else(|| "?".to_string())
                );
            }
            Ok(())
        }
        "set-rgb" => {
            let dev = pick_device(&matched, opts.get("device"))?;
            let key = opts.get("key").ok_or("missing --key")?;
            let color = parse_color(opts.get("color").ok_or("missing --color")?)?;
            apply(
                &backend,
                dev,
                RgbCommand::SetKeys(vec![(KeyId(key.clone()), color)]),
            )
        }
        "fill" => {
            let dev = pick_device(&matched, opts.get("device"))?;
            let color = parse_color(opts.get("color").ok_or("missing --color")?)?;
            apply(&backend, dev, RgbCommand::SetAll(color))
        }
        "effect" => {
            let dev = pick_device(&matched, opts.get("device"))?;
            let name = opts.get("name").ok_or("missing --name")?;
            let sel = EffectSelection {
                effect_id: name.clone(),
                speed: opts.get("speed").and_then(|s| s.parse().ok()),
                brightness: opts.get("brightness").and_then(|s| s.parse().ok()),
                colors: opts
                    .get("color")
                    .and_then(|c| Color::from_hex(c).ok())
                    .into_iter()
                    .collect(),
            };
            let drivers = forge_drivers::all_drivers();
            let mut session = open_matched(&backend, dev, &drivers).map_err(|e| e.to_string())?;
            session.set_effect(&sel).map_err(|e| e.to_string())?;
            println!("ok: effect '{name}' on {}", dev.profile.display_name);
            Ok(())
        }
        _ => {
            println!("{USAGE}");
            Ok(())
        }
    }
}

fn apply(backend: &HidapiBackend, dev: &MatchedDevice<'_>, cmd: RgbCommand) -> Result<(), String> {
    let drivers = forge_drivers::all_drivers();
    let mut session = open_matched(backend, dev, &drivers).map_err(|e| e.to_string())?;
    session.apply_rgb(&cmd).map_err(|e| e.to_string())?;
    println!(
        "ok: {} ({})",
        dev.profile.display_name, dev.profile.driver.family
    );
    Ok(())
}

fn pick_device<'a, 'p>(
    matched: &'a [MatchedDevice<'p>],
    wanted: Option<&String>,
) -> Result<&'a MatchedDevice<'p>, String> {
    match wanted {
        Some(id) => matched
            .iter()
            .find(|m| &forge_registry::device_id(&m.info).0 == id || m.profile.id == *id)
            .ok_or_else(|| format!("no matched device with id/profile {id:?}")),
        None => matched
            .first()
            .ok_or_else(|| "no matching devices found".to_string()),
    }
}

fn parse_color(s: &str) -> Result<Color, String> {
    Color::from_hex(s).map_err(|e| e.to_string())
}

/// Parse `--flag value` pairs from the argument list.
fn parse_opts(args: &[String]) -> HashMap<String, String> {
    let mut opts = HashMap::new();
    let mut i = 2; // skip program name + subcommand
    while i < args.len() {
        if let Some(flag) = args[i].strip_prefix("--") {
            if let Some(value) = args.get(i + 1) {
                opts.insert(flag.to_string(), value.clone());
                i += 2;
                continue;
            }
        }
        i += 1;
    }
    opts
}

const USAGE: &str = "\
forge-cli — IX Forge developer hardware tool

USAGE:
  forge-cli list
  forge-cli set-rgb --key <KEYID> --color <rrggbb> [--device <id>]
  forge-cli fill --color <rrggbb> [--device <id>]
  forge-cli effect --name <id> [--speed 1-5] [--brightness 1-5] [--color <rrggbb>] [--device <id>]";
