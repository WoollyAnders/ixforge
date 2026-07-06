import { create } from "zustand";
import * as ipc from "../ipc/commands";
import {
  type Capability,
  type DeviceSummary,
  type EffectSelection,
  type Preset,
  type RgbCapability,
  type RgbCommand,
  colorToHex,
  hexToColor,
} from "../types/forge";

interface ForgeState {
  devices: DeviceSummary[];
  selectedId: string | null;
  capabilities: Capability[];
  loading: boolean;
  status: string;

  // RGB editor working state
  activeColor: string;
  keyColors: Record<string, string>; // keyId -> hex
  customBrightness: number; // 0..100, global dim applied to custom colors
  zoneColors: Record<string, string>; // zoneId -> hex (zoned devices)

  // Effects working state
  selectedEffectId: string | null;
  effectSpeed: number;
  effectBrightness: number;
  effectDirection: number; // 0 = default, 1 = reverse
  effectRandomize: boolean;

  refreshDevices: () => Promise<void>;
  selectDevice: (id: string) => Promise<void>;
  rgbCapability: () => RgbCapability | undefined;

  setActiveColor: (hex: string) => void;
  setCustomBrightness: (pct: number) => void;
  paintKey: (keyId: string) => void;
  fillAll: () => Promise<void>;
  clearAll: () => Promise<void>;
  applyToKeyboard: () => Promise<void>;
  setZoneColor: (zoneId: string) => Promise<void>;

  selectEffect: (id: string) => void;
  setEffectSpeed: (n: number) => void;
  setEffectBrightness: (n: number) => void;
  setEffectDirection: (n: number) => void;
  setEffectRandomize: (b: boolean) => void;
  setEffectColor: (hex: string) => void;
  applyEffect: (colorOnly?: boolean) => Promise<void>;

  // Saved per-key presets for the selected device
  presets: Preset[];
  loadPresets: () => Promise<void>;
  saveCurrentPreset: (name: string) => Promise<void>;
  applyPreset: (preset: Preset) => Promise<void>;
  deletePreset: (name: string) => Promise<void>;

  // Hotplug
  deviceAttached: (device: DeviceSummary) => void;
  deviceDetached: (id: string) => void;
}

/** Scale a hex color toward black by a 0..100 percentage. */
function scaleHex(hex: string, pct: number): string {
  const c = hexToColor(hex);
  const f = Math.max(0, Math.min(100, pct)) / 100;
  return colorToHex({
    r: Math.round(c.r * f),
    g: Math.round(c.g * f),
    b: Math.round(c.b * f),
  });
}

export const useStore = create<ForgeState>((set, get) => {
  const send = async (deviceId: string, cmd: RgbCommand, okMessage: string) => {
    try {
      await ipc.setRgb(deviceId, cmd);
      set({ status: okMessage });
    } catch (e) {
      set({ status: `Error: ${String(e)}` });
    }
  };

  return {
    devices: [],
    selectedId: null,
    capabilities: [],
    loading: false,
    status: "Ready",

    activeColor: "#22d3ee",
    keyColors: {},
    customBrightness: 100,
    zoneColors: {},

    selectedEffectId: null,
    effectSpeed: 3,
    effectBrightness: 4,
    effectDirection: 0,
    effectRandomize: false,

    presets: [],

    async refreshDevices() {
      set({ loading: true });
      try {
        const devices = await ipc.listDevices();
        set({ devices, status: `${devices.length} device(s) found` });
        if (!get().selectedId && devices.length > 0) {
          await get().selectDevice(devices[0].id);
        }
      } catch (e) {
        set({ status: `Error listing devices: ${String(e)}` });
      } finally {
        set({ loading: false });
      }
    },

    async selectDevice(id) {
      set({ selectedId: id, keyColors: {}, zoneColors: {} });
      try {
        const capabilities = await ipc.getCapabilities(id);
        const rgb = capabilities.find((c): c is RgbCapability => c.kind === "rgb");
        set({
          capabilities,
          selectedEffectId: rgb?.effects[0]?.id ?? null,
          status: `Selected ${id}`,
        });
        await get().loadPresets();
      } catch (e) {
        set({ status: `Error loading capabilities: ${String(e)}` });
      }
    },

    rgbCapability() {
      return get().capabilities.find((c): c is RgbCapability => c.kind === "rgb");
    },

    setActiveColor(hex) {
      set({ activeColor: hex });
    },

    setCustomBrightness(pct) {
      set({ customBrightness: pct });
    },

    paintKey(keyId) {
      set((s) => {
        const next = { ...s.keyColors };
        if (next[keyId] === s.activeColor) {
          delete next[keyId]; // click again with the same color clears it
        } else {
          next[keyId] = s.activeColor;
        }
        return { keyColors: next };
      });
    },

    async fillAll() {
      const { selectedId, activeColor, customBrightness } = get();
      const rgb = get().rgbCapability();
      if (!selectedId || !rgb) return;
      const keyColors: Record<string, string> = {};
      for (const k of rgb.layout.keys) keyColors[k.id] = activeColor;
      set({ keyColors });
      const color = hexToColor(scaleHex(activeColor, customBrightness));
      await send(selectedId, { set_all: color }, "Filled all keys");
    },

    async clearAll() {
      const { selectedId } = get();
      set({ keyColors: {} });
      if (!selectedId) return;
      await send(selectedId, { set_all: { r: 0, g: 0, b: 0 } }, "Cleared all keys");
    },

    async applyToKeyboard() {
      const { selectedId, keyColors, customBrightness } = get();
      if (!selectedId) return;
      const set_keys = Object.entries(keyColors).map(
        ([k, hex]) =>
          [k, hexToColor(scaleHex(hex, customBrightness))] as [
            string,
            ReturnType<typeof hexToColor>,
          ],
      );
      await send(selectedId, { set_keys }, `Applied ${set_keys.length} key(s)`);
    },

    async setZoneColor(zoneId) {
      const { selectedId, activeColor, customBrightness } = get();
      const rgb = get().rgbCapability();
      if (!selectedId || !rgb || typeof rgb.mode !== "object") return;
      const zone = rgb.mode.zoned.zones.find((z) => z.id === zoneId);
      if (!zone) return;
      // Light the zone's keys in the rendition + remember the zone's color.
      set((s) => {
        const keyColors = { ...s.keyColors };
        for (const k of zone.keys) keyColors[k] = activeColor;
        return { keyColors, zoneColors: { ...s.zoneColors, [zoneId]: activeColor } };
      });
      const color = hexToColor(scaleHex(activeColor, customBrightness));
      await send(selectedId, { set_zone: { zone: zoneId, color } }, `Set zone "${zoneId}"`);
    },

    // Effects apply live: selecting an effect or moving a slider pushes it to
    // the device immediately (no Apply button on the Effects tab).
    selectEffect(id) {
      // Reset per-effect options so a new effect doesn't inherit the previous
      // one's. Rainbow-capable effects (a "colorful" param) default to rainbow
      // on (byte8=1), matching how the board ships them; "randomize" effects
      // default off (use the custom color).
      const eff = get()
        .rgbCapability()
        ?.effects.find((e) => e.id === id);
      const defaultMultiColor = eff?.params.some((p) => p.type === "colorful") ?? false;
      set({ selectedEffectId: id, effectDirection: 0, effectRandomize: defaultMultiColor });
      void get().applyEffect();
    },

    // Speed/brightness are 1..5 sliders, so dragging crosses at most ~4 steps —
    // apply on each step for live feedback (the worker coalesces to the latest,
    // so there's no flood). The continuous color picker still debounces.
    setEffectSpeed(n) {
      set({ effectSpeed: n });
      void get().applyEffect();
    },

    setEffectBrightness(n) {
      set({ effectBrightness: n });
      void get().applyEffect();
    },

    setEffectDirection(n) {
      set({ effectDirection: n });
      void get().applyEffect();
    },

    setEffectRandomize(b) {
      set({ effectRandomize: b });
      void get().applyEffect();
    },

    // Set the active color AND push it to the running effect as a color-only
    // change (no re-select — the board resets a color effect on re-select).
    // Used by the Effects tab's swatches; plain setActiveColor (Custom tab) only
    // updates state.
    setEffectColor(hex) {
      set({ activeColor: hex });
      void get().applyEffect(true);
    },

    async applyEffect(colorOnly = false) {
      const {
        selectedId,
        selectedEffectId,
        effectSpeed,
        effectBrightness,
        effectDirection,
        effectRandomize,
        activeColor,
      } = get();
      if (!selectedId || !selectedEffectId) return;
      const sel: EffectSelection = {
        effect_id: selectedEffectId,
        speed: effectSpeed,
        brightness: effectBrightness,
        colors: [hexToColor(activeColor)],
        direction: effectDirection,
        randomize: effectRandomize,
        color_only: colorOnly,
      };
      try {
        await ipc.setEffect(selectedId, sel);
        set({ status: `Applied effect "${selectedEffectId}"` });
      } catch (e) {
        set({ status: `Error: ${String(e)}` });
      }
    },

    async loadPresets() {
      const { selectedId } = get();
      if (!selectedId) {
        set({ presets: [] });
        return;
      }
      try {
        set({ presets: await ipc.listPresets(selectedId) });
      } catch (e) {
        set({ status: `Error loading presets: ${String(e)}` });
      }
    },

    async saveCurrentPreset(name) {
      const { selectedId, keyColors } = get();
      const trimmed = name.trim();
      if (!selectedId || !trimmed) return;
      const keys = Object.entries(keyColors).map(
        ([k, hex]) => [k, hexToColor(hex)] as [string, ReturnType<typeof hexToColor>],
      );
      try {
        await ipc.savePreset({ name: trimmed, device: selectedId, keys });
        await get().loadPresets();
        set({ status: `Saved preset "${trimmed}"` });
      } catch (e) {
        set({ status: `Error saving preset: ${String(e)}` });
      }
    },

    async applyPreset(preset) {
      const { selectedId } = get();
      if (!selectedId) return;
      const keyColors: Record<string, string> = {};
      for (const [k, c] of preset.keys) keyColors[k] = colorToHex(c);
      set({ keyColors });
      await send(selectedId, { set_keys: preset.keys }, `Applied preset "${preset.name}"`);
    },

    async deletePreset(name) {
      const { selectedId } = get();
      if (!selectedId) return;
      try {
        await ipc.deletePreset(selectedId, name);
        await get().loadPresets();
        set({ status: `Deleted preset "${name}"` });
      } catch (e) {
        set({ status: `Error deleting preset: ${String(e)}` });
      }
    },

    deviceAttached(device) {
      if (!get().devices.some((d) => d.id === device.id)) {
        set((s) => ({
          devices: [...s.devices, device],
          status: `${device.name} connected`,
        }));
      }
      if (!get().selectedId) void get().selectDevice(device.id);
    },

    deviceDetached(id) {
      set((s) => ({ devices: s.devices.filter((d) => d.id !== id) }));
      if (get().selectedId === id) {
        const next = get().devices[0]?.id ?? null;
        if (next) {
          void get().selectDevice(next);
        } else {
          set({ selectedId: null, capabilities: [], presets: [], keyColors: {} });
        }
      }
      set({ status: "Device disconnected" });
    },
  };
});
