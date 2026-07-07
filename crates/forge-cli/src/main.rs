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
        "probe" => {
            let dev = pick_device(&matched, opts.get("device"))?;
            let from = opts.get("from").and_then(|s| parse_int(s)).unwrap_or(1);
            let to = opts.get("to").and_then(|s| parse_int(s)).unwrap_or(0x7b);
            let dwell = opts.get("dwell").and_then(|s| s.parse().ok()).unwrap_or(3.0f64);
            let color = match opts.get("color") {
                Some(c) => parse_color(c)?,
                None => Color { r: 0xff, g: 0xff, b: 0xff },
            };
            probe(&backend, dev, from, to, dwell, color)
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
                direction: opts.get("direction").and_then(|s| s.parse().ok()),
                randomize: opts.get("randomize").map(|s| s == "1" || s == "true").unwrap_or(false),
                color_only: false,
            };
            let drivers = forge_drivers::all_drivers();
            let mut session = open_matched(&backend, dev, &drivers).map_err(|e| e.to_string())?;
            session.set_effect(&sel).map_err(|e| e.to_string())?;
            println!("ok: effect '{name}' on {}", dev.profile.display_name);
            Ok(())
        }
        "lcd" => {
            let dev = pick_device(&matched, opts.get("device"))?;
            let path = opts.get("image").ok_or("missing --image <file>")?;
            let (vid, pid) = (dev.profile.matcher.vid, dev.profile.matcher.pid);
            println!(
                "uploading {path} to the LCD of {} [{vid:04x}:{pid:04x}]",
                dev.profile.display_name
            );
            let log = forge_drivers::sonix::lcd::upload_image_file(vid, pid, path)?;
            println!("ok: {log}");
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
    // The driver streams the frame continuously on a background thread, so just
    // set it and keep the session alive; the board holds the color afterward
    // until a keypress. `--hold` extends how long we keep streaming.
    session.apply_rgb(&cmd).map_err(|e| e.to_string())?;
    let secs = hold.unwrap_or(2.0);
    std::thread::sleep(std::time::Duration::from_secs_f64(secs));
    println!(
        "ok: {} ({}) — streamed ~{secs:.0}s",
        dev.profile.display_name, dev.profile.driver.family
    );
    Ok(())
}

/// Parse an optional `--hold <secs>` (re-stream duration).
fn hold_secs(opts: &HashMap<String, String>) -> Option<f64> {
    opts.get("hold").and_then(|s| s.parse().ok())
}


/// Parse a decimal or `0x`-prefixed hex integer.
fn parse_int(s: &str) -> Option<u32> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u32::from_str_radix(hex, 16).ok()
    } else {
        s.parse().ok()
    }
}

/// Sweep LED indices `from..=to`, lighting one at a time and waiting for the
/// operator to type the physical key that lit, to build the key→led_index map.
/// Each answer is appended to `keymap-probe.txt` immediately so nothing is lost.
/// Indices with no LED stay dark — press Enter (blank) to record them as gaps.
fn probe(
    backend: &HidapiBackend,
    dev: &MatchedDevice<'_>,
    from: u32,
    to: u32,
    dwell: f64,
    color: Color,
) -> Result<(), String> {
    use std::io::{BufRead, Write};

    let drivers = forge_drivers::all_drivers();
    let mut session = open_matched(backend, dev, &drivers).map_err(|e| e.to_string())?;

    let out_path = "keymap-probe.txt";
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(out_path)
        .map_err(|e| e.to_string())?;

    println!(
        "Probing led_index {from}..={to} on {}.\n\
         For each index the key lights and STAYS lit (the driver streams it in the\n\
         background) while you type the key name and press Enter.\n\
         Just press Enter (blank) if nothing lit (a gap). Type 'quit' + Enter to stop early.\n\
         Answers are saved to {out_path}.\n",
        dev.profile.display_name,
    );

    // Brief settle time so the first frame reaches the board before we prompt.
    let settle = std::time::Duration::from_secs_f64(dwell.clamp(0.4, 2.0) * 0.5);
    let stdin = std::io::stdin();
    for idx in from..=to {
        let mut frame = vec![Color::BLACK; 128];
        if (idx as usize) < frame.len() {
            frame[idx as usize] = color;
        }
        // Swap in the single-index frame; the worker keeps it lit continuously.
        session
            .apply_rgb(&RgbCommand::SetFrame(frame))
            .map_err(|e| e.to_string())?;
        std::thread::sleep(settle);
        print!("led_index {idx} (0x{idx:02x}) — which key lit? ");
        std::io::stdout().flush().ok();
        let mut line = String::new();
        stdin.lock().read_line(&mut line).map_err(|e| e.to_string())?;
        let label = line.trim();
        if label.eq_ignore_ascii_case("quit") {
            println!("Stopped at 0x{idx:02x}.");
            break;
        }
        let record = if label.is_empty() { "(gap)" } else { label };
        writeln!(file, "{idx}\t0x{idx:02x}\t{record}").map_err(|e| e.to_string())?;
        file.flush().ok();
    }
    println!("\nSaved to {out_path}. Send me that file's contents and I'll build the profile.");
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
  forge-cli set-rgb --key <KEYID> --color <rrggbb> [--hold <secs>] [--device <id>]
  forge-cli fill --color <rrggbb> [--hold <secs>] [--device <id>]
  forge-cli probe [--from <n>] [--to <n>] [--dwell <secs>] [--color <rrggbb>] [--device <id>]
  forge-cli effect --name <id> [--speed 1-5] [--brightness 1-5] [--color <rrggbb>] [--device <id>]
  forge-cli lcd --image <file.gif|png|jpg> [--device <id>]

  lcd: upload an image to the 1.14\" screen (resized to 240x135, RGB565). Uses a
       raw USB endpoint via nusb — on Windows the LCD interface may need a WinUSB
       driver (Zadig) if the claim fails.

  probe: interactively map keys — lights one LED index at a time (from..=to, hex
         ok e.g. 0x1f), waits for you to type the key that lit, saves to
         keymap-probe.txt. Enter=gap, 'quit'=stop.

  --hold <secs>: keep re-streaming the frame for ~<secs> so the color locks and
                 holds steadily (recommended for the AULA F108 Pro; try --hold 10).";
