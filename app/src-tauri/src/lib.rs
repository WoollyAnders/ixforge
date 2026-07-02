//! IX Forge Tauri backend: the IPC seam between the React UI and the device crates.
//!
//! Device I/O is blocking (hidapi), so each command runs its work on a blocking
//! thread. The real Sonix driver drives the LEDs from a background streaming
//! thread that must stay alive to hold a color (a keypress makes the board redraw
//! its onboard profile otherwise), so we keep the open [`DeviceSession`] in app
//! state (`AppState::active`) and reuse it across commands — `apply_rgb` just
//! swaps the streamed frame in. The session (and its worker) is `Send`, so this is
//! sound; it's replaced when the target device changes and dropped on exit.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::{Emitter, Manager};

use forge_core::{Capability, DeviceSession, EffectSelection, ForgeError, RgbCommand};
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
    /// The currently-open device session, kept alive so its background streaming
    /// worker holds the color. Reopened when the target device id changes.
    active: Arc<Mutex<Option<ActiveDevice>>>,
}

/// An open session bound to a specific device id.
struct ActiveDevice {
    id: String,
    session: Box<dyn DeviceSession>,
}

/// Ensure `active` holds a live session for `id`, then run `f` on it. Opening a
/// session enumerates + matches (blocking) and spawns the streaming worker; once
/// open it is reused so subsequent calls just swap the streamed frame.
fn run_on_session(
    catalog: &ProfileCatalog,
    active: &Arc<Mutex<Option<ActiveDevice>>>,
    id: String,
    f: impl FnOnce(&mut dyn DeviceSession) -> Result<(), ForgeError>,
) -> Result<(), String> {
    let mut guard = active.lock().map_err(|_| "session lock poisoned".to_string())?;
    let needs_open = guard.as_ref().map(|a| a.id != id).unwrap_or(true);
    if needs_open {
        let backend = HidapiBackend::new().map_err(|e| e.to_string())?;
        let infos = backend.enumerate().map_err(|e| e.to_string())?;
        let matched = match_devices(infos, catalog);
        let dev = matched
            .iter()
            .find(|m| device_id(&m.info).0 == id)
            .ok_or_else(|| format!("device not found: {id}"))?;
        let drivers = forge_drivers::all_drivers();
        let session = open_matched(&backend, dev, &drivers).map_err(|e| e.to_string())?;
        *guard = Some(ActiveDevice { id, session }); // drops any previous session/worker
    }
    let active = guard.as_mut().expect("session present after open");
    f(active.session.as_mut()).map_err(|e| e.to_string())
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
    let active = state.active.clone();
    tauri::async_runtime::spawn_blocking(move || {
        run_on_session(&catalog, &active, device_id, move |s| s.apply_rgb(&cmd))
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
    let active = state.active.clone();
    tauri::async_runtime::spawn_blocking(move || {
        run_on_session(&catalog, &active, device_id, move |s| s.set_effect(&sel))
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
fn hotplug_loop(app: tauri::AppHandle, active: Arc<Mutex<Option<ActiveDevice>>>) {
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
                    // If the detached device held the live session, drop it so a
                    // reattach reopens a fresh streaming worker instead of reusing
                    // one whose thread has already died on the lost handle.
                    if let Ok(mut guard) = active.lock() {
                        if guard.as_ref().map(|a| a.id == id.0).unwrap_or(false) {
                            *guard = None;
                        }
                    }
                    let _ = app.emit("device://detached", Detached { id: id.0 });
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(1500));
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let active: Arc<Mutex<Option<ActiveDevice>>> = Arc::new(Mutex::new(None));
    tauri::Builder::default()
        .manage(AppState {
            catalog: load_catalog(),
            active: active.clone(),
        })
        .setup(move |app| {
            let handle = app.handle().clone();
            std::thread::spawn(move || hotplug_loop(handle, active));
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
