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
  Preset,
  RgbCommand,
} from "../types/forge";

// Full-size AULA F108 Pro layout, traced from the V3 product image: function
// row, main block, nav column, arrows, and the numpad. The volume knob and LCD
// screen at the top-right are chassis elements (see deviceArt), not keys.
// led_index is sequential here (browser preview); real indices come from capture.
function buildLayout(): LedLayout {
  const keys: KeyDef[] = [];
  let led = 0;
  const add = (id: string, label: string, x: number, y: number, w = 1, h = 1) =>
    keys.push({ id, label, x, y, w, h, led_index: led++ });

  const NAV = 15.25; // nav/edit column (Ins/Home/PgUp …)
  const NUM = 18.5; // numpad

  // Function row (y0)
  add("KC_ESC", "Esc", 0, 0);
  ["F1", "F2", "F3", "F4"].forEach((l, i) => add(`KC_${l}`, l, 2 + i, 0));
  ["F5", "F6", "F7", "F8"].forEach((l, i) => add(`KC_${l}`, l, 6.5 + i, 0));
  ["F9", "F10", "F11", "F12"].forEach((l, i) => add(`KC_${l}`, l, 11 + i, 0));
  add("KC_PSCR", "PrtSc", NAV, 0);
  add("KC_SLCK", "ScrLk", NAV + 1, 0);
  add("KC_PAUS", "Paus", NAV + 2, 0);

  // Number row (y1)
  ["`", "1", "2", "3", "4", "5", "6", "7", "8", "9", "0", "-", "="].forEach((l, i) =>
    add(`KC_R1_${i}`, l, i, 1),
  );
  add("KC_BSPC", "Bksp", 13, 1, 2);
  add("KC_INS", "Ins", NAV, 1);
  add("KC_HOME", "Home", NAV + 1, 1);
  add("KC_PGUP", "PgUp", NAV + 2, 1);
  add("KC_NUM", "Num", NUM, 1);
  add("KC_PSLS", "/", NUM + 1, 1);
  add("KC_PAST", "*", NUM + 2, 1);
  add("KC_PMNS", "-", NUM + 3, 1);

  // QWERTY row (y2)
  add("KC_TAB", "Tab", 0, 2, 1.5);
  ["Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P", "[", "]", "\\"].forEach((l, i) =>
    add(`KC_R2_${i}`, l, 1.5 + i, 2),
  );
  add("KC_DEL", "Del", NAV, 2);
  add("KC_END", "End", NAV + 1, 2);
  add("KC_PGDN", "PgDn", NAV + 2, 2);
  ["7", "8", "9"].forEach((l, i) => add(`KC_P${l}`, l, NUM + i, 2));
  add("KC_PPLS", "+", NUM + 3, 2, 1, 2); // tall

  // Home row (y3)
  add("KC_CAPS", "Caps", 0, 3, 1.75);
  ["A", "S", "D", "F", "G", "H", "J", "K", "L", ";", "'"].forEach((l, i) =>
    add(`KC_R3_${i}`, l, 1.75 + i, 3),
  );
  add("KC_ENT", "Enter", 12.75, 3, 2.25);
  ["4", "5", "6"].forEach((l, i) => add(`KC_P${l}`, l, NUM + i, 3));

  // Bottom letter row (y4)
  add("KC_LSFT", "Shift", 0, 4, 2.25);
  ["Z", "X", "C", "V", "B", "N", "M", ",", ".", "/"].forEach((l, i) =>
    add(`KC_R4_${i}`, l, 2.25 + i, 4),
  );
  add("KC_RSFT", "Shift", 12.25, 4, 2.75);
  add("KC_UP", "↑", NAV + 1, 4);
  ["1", "2", "3"].forEach((l, i) => add(`KC_P${l}`, l, NUM + i, 4));
  add("KC_PENT", "Ent", NUM + 3, 4, 1, 2); // tall

  // Bottom row (y5)
  add("KC_LCTL", "Ctrl", 0, 5, 1.25);
  add("KC_LGUI", "Win", 1.25, 5, 1.25);
  add("KC_LALT", "Alt", 2.5, 5, 1.25);
  add("KC_SPC", "", 3.75, 5, 6.25);
  add("KC_RALT", "Alt", 10, 5, 1.25);
  add("KC_FN", "Fn", 11.25, 5, 1.25);
  add("KC_MENU", "Menu", 12.5, 5, 1.25);
  add("KC_RCTL", "Ctrl", 13.75, 5, 1.25);
  add("KC_LEFT", "←", NAV, 5);
  add("KC_DOWN", "↓", NAV + 1, 5);
  add("KC_RGHT", "→", NAV + 2, 5);
  add("KC_P0", "0", NUM, 5, 2);
  add("KC_PDOT", ".", NUM + 2, 5);

  return { keys, matrix_size: [6, 22] };
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

// Presets persist to localStorage in browser mode (the app uses a real file).
const PRESET_KEY = "ixforge:presets";

function readPresets(): Preset[] {
  try {
    return JSON.parse(localStorage.getItem(PRESET_KEY) ?? "[]") as Preset[];
  } catch {
    return [];
  }
}

function writePresets(all: Preset[]): void {
  localStorage.setItem(PRESET_KEY, JSON.stringify(all));
}

export async function listPresets(device: string): Promise<Preset[]> {
  return readPresets().filter((p) => p.device === device);
}

export async function savePreset(preset: Preset): Promise<void> {
  const all = readPresets().filter(
    (p) => !(p.device === preset.device && p.name === preset.name),
  );
  all.push(preset);
  writePresets(all);
}

export async function deletePreset(device: string, name: string): Promise<void> {
  writePresets(
    readPresets().filter((p) => !(p.device === device && p.name === name)),
  );
}
