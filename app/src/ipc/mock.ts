// Browser-mode mock backend.
//
// When the app runs in a plain browser (no Tauri), the IPC layer falls back to
// these mocks so the UI is fully usable for design/iteration without hardware.
// The mock device mirrors the AULA F108 Pro's capabilities.

import type {
  Capability,
  DeviceSummary,
  EffectDescriptor,
  EffectParam,
  EffectSelection,
  KeyDef,
  LedLayout,
  RgbCommand,
} from "../types/forge";

// Build a representative full-size-ish layout programmatically. Real layouts and
// led_index values come from the device profile once captured.
function buildLayout(): LedLayout {
  const keys: KeyDef[] = [];
  let led = 0;
  const add = (id: string, label: string, x: number, y: number, w = 1, h = 1) => {
    keys.push({ id, label, x, y, w, h, led_index: led++ });
  };

  // Function row
  add("KC_ESC", "Esc", 0, 0);
  ["F1", "F2", "F3", "F4"].forEach((l, i) => add(`KC_${l}`, l, 2 + i, 0));
  ["F5", "F6", "F7", "F8"].forEach((l, i) => add(`KC_${l}`, l, 6.5 + i, 0));
  ["F9", "F10", "F11", "F12"].forEach((l, i) => add(`KC_${l}`, l, 11 + i, 0));
  add("KC_KNOB", "◉", 15.25, 0); // the F108 Pro knob, shown as a key for now

  // Number row
  const r1 = ["`", "1", "2", "3", "4", "5", "6", "7", "8", "9", "0", "-", "="];
  r1.forEach((l, i) => add(`KC_R1_${i}`, l, i, 1.25));
  add("KC_BSPC", "Bksp", r1.length, 1.25, 2);

  // QWERTY row
  add("KC_TAB", "Tab", 0, 2.25, 1.5);
  const r2 = ["Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P", "[", "]", "\\"];
  r2.forEach((l, i) => add(`KC_R2_${i}`, l, 1.5 + i, 2.25));

  // Home row
  add("KC_CAPS", "Caps", 0, 3.25, 1.75);
  const r3 = ["A", "S", "D", "F", "G", "H", "J", "K", "L", ";", "'"];
  r3.forEach((l, i) => add(`KC_R3_${i}`, l, 1.75 + i, 3.25));
  add("KC_ENT", "Enter", 1.75 + r3.length, 3.25, 2.25);

  // Bottom letter row
  add("KC_LSFT", "Shift", 0, 4.25, 2.25);
  const r4 = ["Z", "X", "C", "V", "B", "N", "M", ",", ".", "/"];
  r4.forEach((l, i) => add(`KC_R4_${i}`, l, 2.25 + i, 4.25));
  add("KC_RSFT", "Shift", 2.25 + r4.length, 4.25, 2.75);

  // Space row
  add("KC_LCTL", "Ctrl", 0, 5.25, 1.25);
  add("KC_LGUI", "Win", 1.25, 5.25, 1.25);
  add("KC_LALT", "Alt", 2.5, 5.25, 1.25);
  add("KC_SPC", "", 3.75, 5.25, 6.25);
  add("KC_RALT", "Alt", 10, 5.25, 1.25);
  add("KC_FN", "Fn", 11.25, 5.25, 1.25);
  add("KC_RCTL", "Ctrl", 12.5, 5.25, 1.25);

  return { keys, matrix_size: [6, 21] };
}

const LAYOUT = buildLayout();

const DEVICE: DeviceSummary = {
  id: "mock:aula.f108-pro",
  name: "AULA F108 Pro (mock)",
  connected: true,
  capability_kinds: ["rgb", "macro", "lcd"],
};

// Mirrors profiles/aula/f108-pro.toml — the F108 Pro's seeded stock animations.
function buildEffects(): EffectDescriptor[] {
  const speed: EffectParam = { type: "speed", min: 1, max: 5, default: 3 };
  const bright: EffectParam = { type: "brightness", min: 1, max: 5, default: 4 };
  const dir: EffectParam = { type: "direction" };
  const color: EffectParam = { type: "color_list", max: 1 };
  return [
    { id: "static", name: "Static", params: [bright, color] },
    { id: "breathing", name: "Breathing", params: [speed, bright, color] },
    { id: "spectrum", name: "Spectrum Cycle", params: [speed, bright] },
    { id: "wave", name: "Rainbow Wave", params: [speed, bright, dir] },
    { id: "ripple", name: "Ripple", params: [speed, bright, color] },
    { id: "reactive", name: "Reactive", params: [speed, bright, color] },
    { id: "raindrop", name: "Raindrop", params: [speed, bright] },
    { id: "snake", name: "Snake", params: [speed, bright, dir] },
    { id: "aurora", name: "Aurora", params: [speed, bright] },
    { id: "neon_stream", name: "Neon Stream", params: [speed, bright, dir] },
    { id: "scan", name: "Scan", params: [speed, bright, dir] },
    { id: "stars", name: "Twinkling Stars", params: [speed, bright] },
    { id: "spiral", name: "Spiral", params: [speed, bright] },
    { id: "radar", name: "Radar Sweep", params: [speed, bright, dir] },
    { id: "flash_away", name: "Flash Away", params: [speed, bright, color] },
    { id: "music", name: "Music Rhythm", params: [bright] },
  ];
}

const CAPABILITIES: Capability[] = [
  {
    kind: "rgb",
    mode: "per_key",
    layout: LAYOUT,
    effects: buildEffects(),
    max_brightness: 255,
    color_order: "RGB",
  },
  { kind: "macro", storage: { mode: "on_device", slots: 5 } },
  {
    kind: "lcd",
    width: 240,
    height: 135,
    format: "rgb565",
    features: { image: true, text: true, gif: true, system_monitor: true },
  },
];

export async function listDevices(): Promise<DeviceSummary[]> {
  return [DEVICE];
}

export async function getCapabilities(_deviceId: string): Promise<Capability[]> {
  return CAPABILITIES;
}

export async function setRgb(deviceId: string, cmd: RgbCommand): Promise<void> {
  // No hardware in browser mode; log so the wiring is observable.
  // eslint-disable-next-line no-console
  console.info("[mock] set_rgb", deviceId, cmd);
}

export async function setEffect(
  deviceId: string,
  sel: EffectSelection,
): Promise<void> {
  // eslint-disable-next-line no-console
  console.info("[mock] set_effect", deviceId, sel);
}
