import { create } from "zustand";
import * as ipc from "../ipc/commands";
import {
  type Capability,
  type DeviceSummary,
  type EffectSelection,
  type RgbCapability,
  type RgbCommand,
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

  // Effects working state
  selectedEffectId: string | null;
  effectSpeed: number;
  effectBrightness: number;

  refreshDevices: () => Promise<void>;
  selectDevice: (id: string) => Promise<void>;
  rgbCapability: () => RgbCapability | undefined;

  setActiveColor: (hex: string) => void;
  paintKey: (keyId: string) => void;
  fillAll: () => Promise<void>;
  clearAll: () => Promise<void>;
  applyToKeyboard: () => Promise<void>;

  selectEffect: (id: string) => void;
  setEffectSpeed: (n: number) => void;
  setEffectBrightness: (n: number) => void;
  applyEffect: () => Promise<void>;
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

    activeColor: "#ff5a00",
    keyColors: {},

    selectedEffectId: null,
    effectSpeed: 3,
    effectBrightness: 4,

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
      set({ selectedId: id, keyColors: {} });
      try {
        const capabilities = await ipc.getCapabilities(id);
        set({ capabilities, status: `Selected ${id}` });
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
      const { selectedId, activeColor } = get();
      const rgb = get().rgbCapability();
      if (!selectedId || !rgb) return;
      const keyColors: Record<string, string> = {};
      for (const k of rgb.layout.keys) keyColors[k.id] = activeColor;
      set({ keyColors });
      await send(selectedId, { set_all: hexToColor(activeColor) }, "Filled all keys");
    },

    async clearAll() {
      const { selectedId } = get();
      set({ keyColors: {} });
      if (!selectedId) return;
      await send(selectedId, { set_all: { r: 0, g: 0, b: 0 } }, "Cleared all keys");
    },

    async applyToKeyboard() {
      const { selectedId, keyColors } = get();
      if (!selectedId) return;
      const set_keys = Object.entries(keyColors).map(
        ([k, hex]) => [k, hexToColor(hex)] as [string, ReturnType<typeof hexToColor>],
      );
      await send(selectedId, { set_keys }, `Applied ${set_keys.length} key(s)`);
    },

    selectEffect(id) {
      set({ selectedEffectId: id });
    },

    setEffectSpeed(n) {
      set({ effectSpeed: n });
    },

    setEffectBrightness(n) {
      set({ effectBrightness: n });
    },

    async applyEffect() {
      const { selectedId, selectedEffectId, effectSpeed, effectBrightness, activeColor } =
        get();
      if (!selectedId || !selectedEffectId) return;
      const sel: EffectSelection = {
        effect_id: selectedEffectId,
        speed: effectSpeed,
        brightness: effectBrightness,
        colors: [hexToColor(activeColor)],
      };
      try {
        await ipc.setEffect(selectedId, sel);
        set({ status: `Applied effect "${selectedEffectId}"` });
      } catch (e) {
        set({ status: `Error: ${String(e)}` });
      }
    },
  };
});
