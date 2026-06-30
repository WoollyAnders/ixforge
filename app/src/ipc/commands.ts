// Typed wrappers over the Rust IPC commands, with a browser-mode fallback.
//
// Every call routes to a `#[tauri::command]` in forge-app when running under
// Tauri, or to the in-memory mock when running in a browser.

import { invoke } from "@tauri-apps/api/core";
import type {
  Capability,
  DeviceSummary,
  EffectSelection,
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
