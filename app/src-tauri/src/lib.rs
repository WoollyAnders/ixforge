//! IX Forge Tauri backend: the IPC seam between the React UI and the device crates.
//!
//! Device I/O is blocking (hidapi), so each command runs its work on a blocking
//! thread. To stay clear of `!Send` HID handles in shared state, commands create a
//! fresh backend per call and don't hold sessions across calls. That is fine for
//! the current placeholder protocol; a persistent per-device actor (the planned
//! refinement) lands when the real protocol does.

use serde::Serialize;

use forge_core::{Capability, EffectSelection, RgbCommand};
use forge_profiles::{parse_profile, ProfileCatalog};
use forge_registry::{match_devices, open_matched};
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
#[derive(Serialize)]
struct DeviceSummary {
    id: String,
    name: String,
    connected: bool,
    capability_kinds: Vec<String>,
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
        let summaries = match_devices(infos, &catalog)
            .iter()
            .map(|m| DeviceSummary {
                id: forge_registry::device_id(&m.info).0,
                name: m.profile.display_name.clone(),
                connected: true,
                capability_kinds: m.profile.capabilities.iter().map(capability_kind).collect(),
            })
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState {
            catalog: load_catalog(),
        })
        .invoke_handler(tauri::generate_handler![
            list_devices,
            get_capabilities,
            set_rgb,
            set_effect
        ])
        .run(tauri::generate_context!())
        .expect("error while running IX Forge");
}
