//! IX Forge Tauri backend: the IPC seam between the React UI and the device crates.
//!
//! Device I/O is blocking (hidapi), so each command runs its work on a blocking
//! thread. To stay clear of `!Send` HID handles in shared state, commands create a
//! fresh backend per call and don't hold sessions across calls. That is fine for
//! the current placeholder protocol; a persistent per-device actor (the planned
//! refinement) lands when the real protocol does.

use std::collections::HashSet;

use serde::Serialize;
use tauri::{Emitter, Manager};

use forge_core::{Capability, EffectSelection, RgbCommand};
use forge_profiles::{parse_profile, Preset, PresetStore, ProfileCatalog};
use forge_registry::{device_id, match_devices, open_matched, DeviceWatcher};
use forge_transport::hidapi_backend::HidapiBackend;
use forge_transport::HidBackend;

/// Device profiles embedded at build time. Add more `include_str!`s as decoded.
const EMBEDDED_PROFILES: &[&str] = &[include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../profiles/aula/f108-pro.toml"
))];

struct AppState {
    catalog: ProfileCatalog,
}

fn load_catalog() -> ProfileCatalog {
    let mut catalog = ProfileCatalog::new();
    for src in EMBEDDED_PROFILES {
        match parse_profile(src) {
            Ok(profile) => catalog.add(profile),
            Err(e) => eprintln!("IX Forge: skipping invalid embedded profile: {e}"),
        }
    }
    catalog
}

/// What the UI lists in the device sidebar.
#[derive(Clone, Serialize)]
struct DeviceSummary {
    id: String,
    name: String,
    connected: bool,
    capability_kinds: Vec<String>,
}

/// Payload for the `device://detached` hotplug event.
#[derive(Clone, Serialize)]
struct Detached {
    id: String,
}

/// Build a summary for a matched device.
fn summarize(
    info: &forge_transport::DeviceInfo,
    profile: &forge_core::DeviceProfile,
) -> DeviceSummary {
    DeviceSummary {
        id: device_id(info).0,
        name: profile.display_name.clone(),
        connected: true,
        capability_kinds: profile.capabilities.iter().map(capability_kind).collect(),
    }
}

fn capability_kind(c: &Capability) -> String {
    match c {
        Capability::Rgb(_) => "rgb",
        Capability::Macro(_) => "macro",
        Capability::Lcd(_) => "lcd",
        Capability::Unknown => "unknown",
    }
    .to_string()
}

#[tauri::command]
async fn list_devices(state: tauri::State<'_, AppState>) -> Result<Vec<DeviceSummary>, String> {
    let catalog = state.catalog.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<Vec<DeviceSummary>, String> {
        let backend = HidapiBackend::new().map_err(|e| e.to_string())?;
        let infos = backend.enumerate().map_err(|e| e.to_string())?;
        // Dedup by device id — a device exposes several HID interfaces that can
        // all match a vid/pid-only profile, but it's one device to the user.
        let mut seen = HashSet::new();
        let summaries = match_devices(infos, &catalog)
            .into_iter()
            .filter(|m| seen.insert(device_id(&m.info)))
            .map(|m| summarize(&m.info, m.profile))
            .collect();
        Ok(summaries)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn get_capabilities(
    state: tauri::State<'_, AppState>,
    device_id: String,
) -> Result<Vec<Capability>, String> {
    let catalog = state.catalog.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<Vec<Capability>, String> {
        let backend = HidapiBackend::new().map_err(|e| e.to_string())?;
        let infos = backend.enumerate().map_err(|e| e.to_string())?;
        let matched = match_devices(infos, &catalog);
        let dev = matched
            .iter()
            .find(|m| forge_registry::device_id(&m.info).0 == device_id)
            .ok_or_else(|| format!("device not found: {device_id}"))?;
        Ok(dev.profile.capabilities.clone())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn set_rgb(
    state: tauri::State<'_, AppState>,
    device_id: String,
    cmd: RgbCommand,
) -> Result<(), String> {
    let catalog = state.catalog.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let backend = HidapiBackend::new().map_err(|e| e.to_string())?;
        let infos = backend.enumerate().map_err(|e| e.to_string())?;
        let matched = match_devices(infos, &catalog);
        let dev = matched
            .iter()
            .find(|m| forge_registry::device_id(&m.info).0 == device_id)
            .ok_or_else(|| format!("device not found: {device_id}"))?;
        let drivers = forge_drivers::all_drivers();
        let mut session = open_matched(&backend, dev, &drivers).map_err(|e| e.to_string())?;
        session.apply_rgb(&cmd).map_err(|e| e.to_string())?;
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn set_effect(
    state: tauri::State<'_, AppState>,
    device_id: String,
    sel: EffectSelection,
) -> Result<(), String> {
    let catalog = state.catalog.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let backend = HidapiBackend::new().map_err(|e| e.to_string())?;
        let infos = backend.enumerate().map_err(|e| e.to_string())?;
        let matched = match_devices(infos, &catalog);
        let dev = matched
            .iter()
            .find(|m| forge_registry::device_id(&m.info).0 == device_id)
            .ok_or_else(|| format!("device not found: {device_id}"))?;
        let drivers = forge_drivers::all_drivers();
        let mut session = open_matched(&backend, dev, &drivers).map_err(|e| e.to_string())?;
        session.set_effect(&sel).map_err(|e| e.to_string())?;
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

// --- User lighting presets (persisted to the app config dir) ----------------

fn preset_store(app: &tauri::AppHandle) -> Result<PresetStore, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    Ok(PresetStore::new(dir.join("presets.json")))
}

#[tauri::command]
async fn list_presets(app: tauri::AppHandle, device: String) -> Result<Vec<Preset>, String> {
    preset_store(&app)?
        .list_for(&device)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn save_preset(app: tauri::AppHandle, preset: Preset) -> Result<(), String> {
    preset_store(&app)?.save(preset).map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_preset(app: tauri::AppHandle, device: String, name: String) -> Result<(), String> {
    preset_store(&app)?
        .delete(&device, &name)
        .map_err(|e| e.to_string())
}

/// Background hotplug poller: emits `device://attached` / `device://detached`
/// as supported devices connect/disconnect. Runs on its own thread because the
/// `hidapi` handle is blocking and not `Send`, so the backend lives entirely here.
fn hotplug_loop(app: tauri::AppHandle) {
    let catalog = load_catalog();
    let mut backend = match HidapiBackend::new() {
        Ok(b) => b,
        Err(e) => {
            eprintln!("IX Forge: hotplug disabled ({e})");
            return;
        }
    };
    let mut watcher = DeviceWatcher::new();
    loop {
        if backend.refresh().is_ok() {
            if let Ok(infos) = backend.enumerate() {
                // Track only supported devices, deduped by id.
                let mut seen = HashSet::new();
                let matched: Vec<_> = infos
                    .into_iter()
                    .filter(|i| catalog.match_device(&i.match_input()).is_some())
                    .filter(|i| seen.insert(device_id(i)))
                    .collect();
                let delta = watcher.diff(matched);
                for info in &delta.attached {
                    if let Some(profile) = catalog.match_device(&info.match_input()) {
                        let _ = app.emit("device://attached", summarize(info, profile));
                    }
                }
                for id in delta.detached {
                    let _ = app.emit("device://detached", Detached { id: id.0 });
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(1500));
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState {
            catalog: load_catalog(),
        })
        .setup(|app| {
            let handle = app.handle().clone();
            std::thread::spawn(move || hotplug_loop(handle));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_devices,
            get_capabilities,
            set_rgb,
            set_effect,
            list_presets,
            save_preset,
            delete_preset
        ])
        .run(tauri::generate_context!())
        .expect("error while running IX Forge");
}
