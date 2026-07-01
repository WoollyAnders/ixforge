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

/// The F108 Pro profile is embedded so a cross-compiled `.exe` is self-contained
/// (copy one file to the target machine). On a dev checkout the on-disk `profiles/`
/// tree is used instead; the embedded copy is the fallback.
const F108_PROFILE_TOML: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../profiles/aula/f108-pro.toml"));

/// Load the profile catalog: the on-disk tree (dev, or `--profiles <dir>`) if it
/// has profiles, else the embedded F108 profile (portable standalone binary).
fn load_catalog(profiles_dir: Option<&String>) -> Result<ProfileCatalog, String> {
    let dir = profiles_dir.map(String::as_str).unwrap_or(PROFILES_DIR);
    if let Ok(catalog) = ProfileCatalog::from_dir(dir) {
        if !catalog.profiles().is_empty() {
            return Ok(catalog);
        }
    }
    let profile = forge_profiles::parse_profile(F108_PROFILE_TOML).map_err(|e| e.to_string())?;
    Ok(ProfileCatalog::from_profiles(vec![profile]))
}

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

    let catalog = load_catalog(opts.get("profiles"))?;
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
                hold_secs(&opts),
            )
        }
        "fill" => {
            let dev = pick_device(&matched, opts.get("device"))?;
            let color = parse_color(opts.get("color").ok_or("missing --color")?)?;
            apply(&backend, dev, RgbCommand::SetAll(color), hold_secs(&opts))
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

fn apply(
    backend: &HidapiBackend,
    dev: &MatchedDevice<'_>,
    cmd: RgbCommand,
    hold: Option<f64>,
) -> Result<(), String> {
    let drivers = forge_drivers::all_drivers();
    let mut session = open_matched(backend, dev, &drivers).map_err(|e| e.to_string())?;
    match hold {
        // Keep re-streaming for ~`secs` so the color locks and holds steadily
        // (the board redraws its onboard profile otherwise). Watch the board.
        Some(secs) => {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs_f64(secs);
            let mut n = 0u32;
            while std::time::Instant::now() < deadline {
                session.apply_rgb(&cmd).map_err(|e| e.to_string())?;
                n += 1;
            }
            println!(
                "ok: {} ({}) — streamed {n}× over ~{secs:.0}s",
                dev.profile.display_name, dev.profile.driver.family
            );
        }
        None => {
            session.apply_rgb(&cmd).map_err(|e| e.to_string())?;
            println!(
                "ok: {} ({})",
                dev.profile.display_name, dev.profile.driver.family
            );
        }
    }
    Ok(())
}

/// Parse an optional `--hold <secs>` (re-stream duration).
fn hold_secs(opts: &HashMap<String, String>) -> Option<f64> {
    opts.get("hold").and_then(|s| s.parse().ok())
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
  forge-cli set-rgb --key <KEYID> --color <rrggbb> [--hold <secs>] [--device <id>]
  forge-cli fill --color <rrggbb> [--hold <secs>] [--device <id>]
  forge-cli effect --name <id> [--speed 1-5] [--brightness 1-5] [--color <rrggbb>] [--device <id>]

  --hold <secs>: keep re-streaming the frame for ~<secs> so the color locks and
                 holds steadily (recommended for the AULA F108 Pro; try --hold 10).";
