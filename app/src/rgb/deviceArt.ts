// Per-device chassis art for the keyboard rendition.
//
// All coordinates are in KEY UNITS (1 == one key), the same space as LedLayout,
// so KeyboardView scales everything by one factor. This is pure UI metadata; a
// device with no entry falls back to a plain bezel sized to its layout.

import type { LedLayout } from "../types/forge";

export interface ChassisSpec {
  /** Bezel padding around the key area, in key units. */
  padding: { top: number; right: number; bottom: number; left: number };
  /** Case corner radius, in px. */
  cornerRadius: number;
  caseTop: string;
  caseBottom: string;
  /** Volume knob: center + radius, key units. */
  knob?: { x: number; y: number; r: number };
  /** LCD screen anchor (top-left) + height, key units. The WIDTH is derived from
   *  the device's display aspect ratio (see KeyboardView `screenAspect`) so the
   *  rendition matches the real panel's proportions. */
  screen?: { x: number; y: number; h: number };
  /** Status indicator LEDs (top-left): start position + count. */
  indicators?: { x: number; y: number; count: number };
  /** Draw an RGB side-light glow along the case edges. */
  sidelight: boolean;
}

// AULA F108 Pro V3 — full-size; knob + LCD at the top-right above the numpad
// (which spans x≈18.5–22.5 on the function-row band), status LEDs at top-left.
const F108_PRO: ChassisSpec = {
  padding: { top: 0.6, right: 0.55, bottom: 0.65, left: 0.5 },
  cornerRadius: 16,
  caseTop: "#46434f",
  caseBottom: "#211f29",
  screen: { x: 18.55, y: 0.08, h: 0.8 }, // width derived from display aspect
  // Sits in the top bezel; bottom (y+r ≈ 0.83) stays clear of the numpad (y≥1).
  knob: { x: 21.95, y: 0.25, r: 0.58 },
  indicators: { x: 2.2, y: -0.36, count: 3 },
  sidelight: true,
};

function genericChassis(): ChassisSpec {
  return {
    padding: { top: 0.5, right: 0.5, bottom: 0.5, left: 0.5 },
    cornerRadius: 12,
    caseTop: "#43414b",
    caseBottom: "#23212a",
    sidelight: false,
  };
}

/**
 * Pick the chassis for a device. Matched by the device's display name / id so it
 * works for both the real device (`AULA F108 Pro`) and the mock. Unknown devices
 * get the generic bezel — still correct keys, just no model-specific flourishes.
 */
export function chassisFor(deviceKey: string, _layout: LedLayout): ChassisSpec {
  if (/f108\s*pro/i.test(deviceKey)) return F108_PRO;
  return genericChassis();
}
