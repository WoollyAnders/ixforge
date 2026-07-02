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

// Positive modulo into [0,1). JS `%` returns negative for negative inputs, which
// freezes a moving animation once its phase crosses zero.
function wrap01(v: number): number {
  return ((v % 1) + 1) % 1;
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
// Motion tuned to the owner's per-effect description (f108-animation-description).
// Reactive effects (single on/off, glittering, explode, launch) show an idle
// approximation since there's no key input in the preview.
const EFFECTS: Record<string, EffectFn> = {
  // Static — solid active color.
  static: ({ color }) => color,

  // Single On — board dark; keys softly glow in then fade back to off.
  single_on: ({ idx, t, sp, color }) => {
    const ph = (t * sp * 0.5 + rand(idx) * 3) % 3; // sparse, staggered per key
    const v = ph < 1 ? Math.sin(ph * Math.PI) : 0; // glow in→out over a ~1s window
    return v > 0.02 ? dim(color, v) : OFF;
  },

  // Single Off — board lit; keys softly dip off then fade back in.
  single_off: ({ idx, t, sp, color }) => {
    const ph = (t * sp * 0.5 + rand(idx) * 3) % 3;
    const dipv = ph < 1 ? Math.sin(ph * Math.PI) : 0;
    return dim(color, 1 - 0.92 * dipv);
  },

  // Glittering — static base; random keys abruptly flash brighter (no transition).
  glittering: ({ idx, t, sp, color }) => {
    const step = Math.floor(t * sp * 1.6 + rand(idx) * 13) % 9;
    return step === 0 ? color : dim(color, 0.4);
  },

  // Falling — continuous rain of drops per column (loops forever).
  falling: ({ nx, ny, t, sp, color }) => {
    const col = Math.floor(nx * 14);
    const ph = wrap01(ny + rand(col) - t * sp * 0.4);
    const v = ph < 0.3 ? 1 - ph / 0.3 : 0;
    return dim(color, 0.08 + 0.92 * v);
  },

  // Colorful — each key runs its own hue cycle at a different phase.
  colorful: ({ idx, t, sp }) => hsl(t * sp * 35 + rand(idx) * 360, 90, 58),

  // Breathe — whole board fades in/out (slowed down per feedback).
  breathe: ({ color, t, sp }) => dim(color, 0.12 + 0.88 * (0.5 + 0.5 * Math.sin(t * sp * 1.1))),

  // Spectrum — global hue cycle (slowed down per feedback).
  spectrum: ({ t, sp }) => hsl(t * sp * 30, 90, 58),

  // Outward — rainbow radiating from the center.
  outward: ({ nx, ny, t, sp }) => {
    const d = Math.hypot(nx - 0.5, ny - 0.5);
    return hsl(d * 520 - t * sp * 55, 90, 57);
  },

  // Scrolling — rainbow cycling top→down.
  scrolling: ({ ny, t, sp }) => hsl(ny * 360 + t * sp * 45, 90, 57),

  // Rolling — rainbow cycling left→right.
  rolling: ({ nx, t, sp }) => hsl(nx * 360 + t * sp * 45, 90, 57),

  // Rotating — rotating rainbow spiral ("perfect").
  rotating: ({ nx, ny, t, sp }) => {
    const ang = Math.atan2(ny - 0.5, nx - 0.5);
    return hsl((ang / Math.PI) * 180 + t * sp * 50, 88, 56);
  },

  // Explode — a triggered row fills left→right with a random color per key.
  explode: ({ nx, ny, idx, t, sp }) => {
    const row = Math.round(ny * 5);
    const active = Math.floor(t * sp * 1.1) % 6;
    if (row !== active) return OFF;
    const pos = wrap01(t * sp * 1.1); // ripple front sweeps the row
    if (nx > pos) return OFF;
    return hsl(rand(idx * 7 + active * 31) * 360, 90, 57); // random per-key color
  },

  // Launch — a fast radial ripple expanding from the center.
  launch: ({ nx, ny, t, sp }) => {
    const d = Math.hypot(nx - 0.5, ny - 0.5);
    const pos = wrap01(t * sp * 0.6) * 0.85; // expanding radius (resets each cycle)
    const v = clamp01(1 - Math.abs(d - pos) * 6);
    return v > 0.02 ? hsl(190 + pos * 260, 88, 57) : OFF;
  },

  // Ripples — like Launch but smoother, with ripples popping up at several spots
  // (as if random keys were being pressed).
  ripples: ({ nx, ny, t, sp }) => {
    const sources: [number, number][] = [
      [0.25, 0.35],
      [0.62, 0.62],
      [0.82, 0.3],
      [0.45, 0.7],
    ];
    let best = 0;
    for (let i = 0; i < sources.length; i++) {
      const phase = wrap01(t * sp * 0.3 + i / sources.length);
      const radius = phase * 0.9;
      const d = Math.hypot(nx - sources[i][0], ny - sources[i][1]);
      const v = clamp01(1 - Math.abs(d - radius) * 7) * (1 - phase);
      if (v > best) best = v;
    }
    return best > 0.02 ? dim(hsl(200 + best * 90, 85, 57), 0.12 + 0.88 * best) : OFF;
  },

  // Flowing — lights one row at a time, top→bottom, filling left→right.
  flowing: ({ nx, ny, t, sp }) => {
    const row = Math.round(ny * 5);
    const active = Math.floor(t * sp * 1.0) % 6;
    if (row !== active) return OFF;
    const pos = wrap01(t * sp * 1.0);
    return nx <= pos ? hsl((active / 6) * 300 + t * sp * 20, 88, 56) : OFF;
  },

  // Pulsating — a brightness wave expanding from the center out to the sides.
  pulsating: ({ nx, t, sp }) => {
    const d = Math.abs(nx - 0.5) * 2; // 0 center → 1 edge
    const pos = wrap01(t * sp * 0.5);
    const v = clamp01(1 - Math.abs(d - pos) * 4);
    return dim(hsl(t * sp * 22, 85, 57), 0.12 + 0.88 * v);
  },

  // Tilt — "\" diagonal bands sweeping across.
  tilt: ({ nx, ny, t, sp }) => hsl((nx - ny) * 300 + t * sp * 60, 88, 56),
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
