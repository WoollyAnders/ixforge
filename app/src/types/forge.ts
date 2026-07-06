// TypeScript mirror of the `forge-core` IPC contract.
//
// TODO: generate this from Rust via `ts-rs` so it can't drift. Hand-written for
// now; shapes match serde's output (see forge-core/src/*.rs).

export interface Color {
  r: number;
  g: number;
  b: number;
}

export type ColorOrder = "RGB" | "GRB" | "BGR" | "BRG" | "RBG" | "GBR";

// RgbMode is an externally-tagged enum: unit variants are bare strings, the
// struct variant is an object keyed by the variant name.
export type RgbMode =
  | "per_key"
  | "single_color"
  | { zoned: { zones: ZoneDef[] } };

export interface ZoneDef {
  id: string;
  label: string;
  keys: string[];
}

export interface KeyDef {
  id: string;
  label: string;
  x: number;
  y: number;
  w: number;
  h: number;
  led_index: number | null;
}

export interface LedLayout {
  keys: KeyDef[];
  matrix_size: [number, number];
}

export type EffectParam =
  | { type: "speed"; min: number; max: number; default: number }
  | { type: "brightness"; min: number; max: number; default: number }
  | { type: "direction" }
  | { type: "randomize" }
  | { type: "colorful" }
  | { type: "color_list"; max: number };

export interface EffectDescriptor {
  id: string;
  name: string;
  params: EffectParam[];
}

export interface RgbCapability {
  kind: "rgb";
  mode: RgbMode;
  layout: LedLayout;
  effects: EffectDescriptor[];
  max_brightness: number;
  color_order: ColorOrder;
}

export type MacroStorage =
  | { mode: "on_device"; slots: number }
  | { mode: "host_replay" };

export interface MacroCapability {
  kind: "macro";
  storage: MacroStorage;
}

export type LcdFormat = "mono1bpp" | "gray4bpp" | "rgb565";

export interface LcdFeatures {
  image: boolean;
  text: boolean;
  gif: boolean;
  system_monitor: boolean;
}

export interface LcdCapability {
  kind: "lcd";
  width: number;
  height: number;
  format: LcdFormat;
  features: LcdFeatures;
}

export type Capability =
  | RgbCapability
  | MacroCapability
  | LcdCapability
  | { kind: "unknown" };

// RgbCommand is externally tagged (see forge-core/src/command.rs).
export type RgbCommand =
  | { set_all: Color }
  | { set_keys: [string, Color][] }
  | { set_zone: { zone: string; color: Color } }
  | { set_frame: Color[] };

// Select + configure a built-in on-device effect (matches forge-core EffectSelection).
export interface EffectSelection {
  effect_id: string;
  speed?: number;
  brightness?: number;
  colors: Color[];
  direction?: number; // 0 = default, 1 = reverse (effects with a "direction" param)
  randomize?: boolean; // randomize color instead of `colors` ("randomize" param)
  color_only?: boolean; // change color without re-selecting (live color tweak)
}

export interface DeviceSummary {
  id: string;
  name: string;
  connected: boolean;
  capability_kinds: string[];
}

// A user-saved per-key lighting preset (matches forge-profiles Preset).
export interface Preset {
  name: string;
  device: string;
  keys: [string, Color][];
}

// --- small helpers --------------------------------------------------------

export function hexToColor(hex: string): Color {
  const h = hex.replace(/^#/, "");
  return {
    r: parseInt(h.slice(0, 2), 16),
    g: parseInt(h.slice(2, 4), 16),
    b: parseInt(h.slice(4, 6), 16),
  };
}

export function colorToHex(c: Color): string {
  const h = (n: number) => n.toString(16).padStart(2, "0");
  return `#${h(c.r)}${h(c.g)}${h(c.b)}`;
}
