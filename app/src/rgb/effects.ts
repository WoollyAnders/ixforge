// Client-side animation simulator for the keyboard preview.
//
// Each effect is a pure function of (key position, time, options) -> hex. This is
// a UI *approximation* of how the device renders an effect — enough to preview
// the look without hardware. Input/audio-driven effects (reactive, raindrop,
// music) use plausible idle animations since there's no key input here.

import type { LedLayout } from "../types/forge";

export interface SimOpts {
  speedLevel: number; // 1..5
  brightnessLevel: number; // 1..5
  color: string; // active color for color-based effects
}

const OFF = "#1b1922";

interface Ctx {
  nx: number; // normalized center x (0..1)
  ny: number; // normalized center y (0..1)
  t: number; // seconds
  idx: number; // key index
  n: number; // key count
  sp: number; // speed factor
  color: string; // active color
}

type EffectFn = (c: Ctx) => string;

function clamp01(v: number): number {
  return v < 0 ? 0 : v > 1 ? 1 : v;
}

function hsl(h: number, s: number, l: number): string {
  h = ((h % 360) + 360) % 360;
  s /= 100;
  l /= 100;
  const c = (1 - Math.abs(2 * l - 1)) * s;
  const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
  const m = l - c / 2;
  let r = 0,
    g = 0,
    b = 0;
  if (h < 60) [r, g, b] = [c, x, 0];
  else if (h < 120) [r, g, b] = [x, c, 0];
  else if (h < 180) [r, g, b] = [0, c, x];
  else if (h < 240) [r, g, b] = [0, x, c];
  else if (h < 300) [r, g, b] = [x, 0, c];
  else [r, g, b] = [c, 0, x];
  const to = (v: number) => Math.round((v + m) * 255).toString(16).padStart(2, "0");
  return `#${to(r)}${to(g)}${to(b)}`;
}

function dim(hex: string, f: number): string {
  const n = parseInt(hex.slice(1), 16);
  const r = Math.round(((n >> 16) & 255) * f);
  const g = Math.round(((n >> 8) & 255) * f);
  const b = Math.round((n & 255) * f);
  return `#${[r, g, b].map((v) => v.toString(16).padStart(2, "0")).join("")}`;
}

// Deterministic per-index pseudo-random in [0,1).
function rand(i: number): number {
  const v = Math.sin(i * 12.9898) * 43758.5453;
  return v - Math.floor(v);
}

// Keyed by the device's real onboard effect ids (see profiles/aula/f108-pro.toml).
// Each is a UI approximation of the animation's motion — enough to tell them apart.
const EFFECTS: Record<string, EffectFn> = {
  // 1 — solid active color
  static: ({ color }) => color,
  // 2 — keys mostly lit, occasional flicker-on (reactive "single on")
  single_on: ({ idx, t, sp, color }) => {
    const ph = (t * sp * 0.8 + rand(idx) * 1.4) % 1.6;
    return ph < 0.4 ? dim(color, 0.3 + 0.7 * (ph / 0.4)) : color;
  },
  // 3 — keys lit, blink off on "press" (reactive "single off")
  single_off: ({ idx, t, sp, color }) => {
    const ph = (t * sp * 0.8 + rand(idx) * 1.4) % 1.6;
    return ph < 0.3 ? dim(color, 0.12) : color;
  },
  // 4 — random twinkle
  glittering: ({ idx, t, sp, color }) => {
    const ph = (t * sp * 0.6 + rand(idx)) % 1;
    return ph < 0.18 ? dim(color, 1 - ph / 0.18) : dim(color, 0.14);
  },
  // 5 — drops falling per column
  falling: ({ nx, ny, t, sp, color }) => {
    const col = Math.floor(nx * 14);
    const ph = (ny + rand(col) - t * sp * 0.5) % 1;
    const v = ph > 0 && ph < 0.3 ? 1 - ph / 0.3 : 0;
    return dim(color, 0.12 + 0.88 * v);
  },
  // 6 — per-key rainbow, slowly cycling
  colorful: ({ idx, n, t, sp }) => hsl((idx / Math.max(n, 1)) * 360 + t * sp * 20, 90, 58),
  // 7 — whole-board breathe in the active color
  breathe: ({ color, t, sp }) => dim(color, 0.15 + 0.85 * (0.5 + 0.5 * Math.sin(t * sp * 2))),
  // 8 — global hue cycle
  spectrum: ({ t, sp }) => hsl(t * sp * 60, 90, 58),
  // 9 — rings radiating outward from center
  outward: ({ nx, ny, t, sp }) => {
    const d = Math.hypot(nx - 0.5, ny - 0.5);
    const v = 0.5 + 0.5 * Math.sin(d * 14 - t * sp * 3);
    return dim(hsl(t * sp * 40 + d * 120, 88, 56), 0.2 + 0.8 * v);
  },
  // 10 — hue wave scrolling left→right
  scrolling: ({ nx, t, sp }) => hsl(nx * 320 + t * sp * 90, 90, 57),
  // 11 — solid hue band rolling across
  rolling: ({ nx, t, sp }) => hsl(((((nx - t * sp * 0.25) % 1) + 1) % 1) * 360, 90, 56),
  // 12 — rotating spiral
  rotating: ({ nx, ny, t, sp }) => {
    const ang = Math.atan2(ny - 0.5, nx - 0.5);
    return hsl((ang / Math.PI) * 180 + t * sp * 70, 88, 56);
  },
  // 13 — expanding ring burst
  explode: ({ nx, ny, t, sp }) => {
    const d = Math.hypot(nx - 0.5, ny - 0.5);
    const pos = (t * sp * 0.3) % 1;
    return dim(hsl(20 + pos * 300, 90, 56), clamp01(1 - Math.abs(d - pos) * 6));
  },
  // 14 — comet sweeping left→right
  launch: ({ nx, t, sp }) => {
    const pos = (t * sp * 0.2) % 1;
    return dim(hsl(180 + pos * 120, 88, 56), clamp01(1 - Math.abs(nx - pos) * 6));
  },
  // 15 — concentric ripples
  ripples: ({ nx, ny, t, sp }) => {
    const d = Math.hypot(nx - 0.5, ny - 0.5);
    const v = 0.5 + 0.5 * Math.sin(d * 10 - t * sp * 3);
    return dim(hsl(200 + d * 120, 85, 55), 0.2 + 0.8 * v);
  },
  // 16 — smooth neon flow
  flowing: ({ nx, t, sp }) => hsl(200 + 80 * Math.sin(nx * 6 - t * sp * 2), 90, 56),
  // 17 — rainbow with a brightness pulse
  pulsating: ({ t, sp }) =>
    dim(hsl(t * sp * 45, 90, 58), 0.2 + 0.8 * (0.5 + 0.5 * Math.sin(t * sp * 3))),
  // 18 — diagonal gradient sweep
  tilt: ({ nx, ny, t, sp }) => hsl((nx + ny) * 160 + t * sp * 70, 88, 56),
};

/** Compute the per-key colors for an effect at time `tMs`. */
export function simulateFrame(
  effectId: string,
  layout: LedLayout,
  tMs: number,
  opts: SimOpts,
): Record<string, string> {
  const fn = EFFECTS[effectId] ?? EFFECTS.spectrum;
  const maxX = Math.max(...layout.keys.map((k) => k.x + k.w), 1);
  const maxY = Math.max(...layout.keys.map((k) => k.y + k.h), 1);
  const n = layout.keys.length;
  const t = tMs / 1000;
  const sp = 0.4 + opts.speedLevel * 0.45;
  const bf = 0.2 + 0.8 * ((opts.brightnessLevel - 1) / 4);

  const out: Record<string, string> = {};
  layout.keys.forEach((k, idx) => {
    const c = fn({
      nx: (k.x + k.w / 2) / maxX,
      ny: (k.y + k.h / 2) / maxY,
      t,
      idx,
      n,
      sp,
      color: opts.color,
    });
    out[k.id] = c === OFF ? OFF : dim(c, bf);
  });
  return out;
}
