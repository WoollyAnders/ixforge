// Typed wrappers over the Rust IPC commands, with a browser-mode fallback.
//
// Every call routes to a `#[tauri::command]` in forge-app when running under
// Tauri, or to the in-memory mock when running in a browser.

import { invoke } from "@tauri-apps/api/core";
import type {
  Capability,
  DeviceSummary,
  EffectSelection,
  Preset,
  RgbCommand,
} from "../types/forge";
import { IS_TAURI } from "./backend";
import * as mock from "./mock";

export async function listDevices(): Promise<DeviceSummary[]> {
  return IS_TAURI ? invoke<DeviceSummary[]>("list_devices") : mock.listDevices();
}

export async function getCapabilities(deviceId: string): Promise<Capability[]> {
  return IS_TAURI
    ? invoke<Capability[]>("get_capabilities", { deviceId })
    : mock.getCapabilities(deviceId);
}

export async function setRgb(deviceId: string, cmd: RgbCommand): Promise<void> {
  if (IS_TAURI) {
    await invoke("set_rgb", { deviceId, cmd });
  } else {
    await mock.setRgb(deviceId, cmd);
  }
}

export async function setEffect(
  deviceId: string,
  sel: EffectSelection,
): Promise<void> {
  if (IS_TAURI) {
    await invoke("set_effect", { deviceId, sel });
  } else {
    await mock.setEffect(deviceId, sel);
  }
}

/** Upload an image/GIF (raw file bytes) to the device's LCD. Returns a log line. */
export async function pushLcdImage(deviceId: string, image: number[]): Promise<string> {
  if (IS_TAURI) return invoke<string>("push_lcd_image", { deviceId, image });
  return "Browser preview — LCD upload runs on hardware only.";
}

export async function listPresets(device: string): Promise<Preset[]> {
  return IS_TAURI ? invoke<Preset[]>("list_presets", { device }) : mock.listPresets(device);
}

export async function savePreset(preset: Preset): Promise<void> {
  if (IS_TAURI) await invoke("save_preset", { preset });
  else await mock.savePreset(preset);
}

export async function deletePreset(device: string, name: string): Promise<void> {
  if (IS_TAURI) await invoke("delete_preset", { device, name });
  else await mock.deletePreset(device, name);
}
