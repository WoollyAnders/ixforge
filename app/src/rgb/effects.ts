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

const EFFECTS: Record<string, EffectFn> = {
  static: ({ color }) => color,
  breathing: ({ color, t, sp }) => dim(color, 0.15 + 0.85 * (0.5 + 0.5 * Math.sin(t * sp * 2))),
  spectrum: ({ t, sp }) => hsl(t * sp * 60, 90, 58),
  wave: ({ nx, t, sp }) => hsl(nx * 320 + t * sp * 90, 90, 58),
  aurora: ({ ny, t, sp }) => hsl(150 + ny * 120 + t * sp * 35, 70, 55),
  neon_stream: ({ nx, t, sp }) => hsl(200 + 60 * Math.sin(nx * 6 - t * sp * 2), 90, 56),
  ripple: ({ nx, ny, t, sp }) => {
    const d = Math.hypot(nx - 0.5, ny - 0.5);
    const v = 0.5 + 0.5 * Math.sin(d * 10 - t * sp * 3);
    return dim(hsl(200 + d * 120, 85, 55), 0.2 + 0.8 * v);
  },
  snake: ({ idx, n, t, sp, color }) => {
    const head = Math.floor(t * sp * 14) % n;
    const dist = (idx - head + n) % n;
    return dist < 6 ? dim(color, 1 - dist / 6) : OFF;
  },
  scan: ({ nx, t, sp }) => {
    const pos = (t * sp * 0.18) % 1;
    const d = Math.abs(nx - pos);
    return dim(hsl(190, 85, 56), clamp01(1 - d * 6));
  },
  radar: ({ nx, ny, t, sp }) => {
    const ang = (Math.atan2(ny - 0.5, nx - 0.5) + Math.PI) / (2 * Math.PI);
    const pos = (t * sp * 0.12) % 1;
    let d = Math.abs(ang - pos);
    d = Math.min(d, 1 - d);
    return dim(hsl(160, 85, 55), clamp01(1 - d / 0.12));
  },
  spiral: ({ nx, ny, t, sp }) => {
    const ang = Math.atan2(ny - 0.5, nx - 0.5);
    return hsl((ang / Math.PI) * 180 + t * sp * 60, 85, 55);
  },
  stars: ({ idx, t, sp, color }) => {
    const ph = (t * sp * 0.5 + rand(idx)) % 1;
    return ph < 0.15 ? dim(color, 1 - ph / 0.15) : OFF;
  },
  reactive: ({ idx, t, sp, color }) => {
    const ph = (t * sp * 0.7 + rand(idx) * 1.3) % 1.3;
    return ph < 0.5 ? dim(color, 1 - ph * 2) : OFF;
  },
  raindrop: ({ nx, ny, t, sp }) => {
    const col = Math.floor(nx * 14);
    const ph = (ny + rand(col) - t * sp * 0.4) % 1;
    const v = ph > 0 && ph < 0.25 ? 1 - ph / 0.25 : 0;
    return dim(hsl(210, 80, 56), v);
  },
  flash_away: ({ t, sp, color }) => {
    const ph = (t * sp * 0.4) % 1;
    return dim(color, ph < 0.5 ? 1 - ph * 2 : 0);
  },
  music: ({ nx, t, sp }) => {
    const col = Math.floor(nx * 14);
    const v = 0.3 + 0.7 * Math.abs(Math.sin(t * sp * 3 + rand(col) * 6));
    return dim(hsl(290 - nx * 130, 85, 56), v);
  },
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
