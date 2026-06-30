import { useEffect, useRef, useState } from "react";
import type { LedLayout } from "../../types/forge";
import type { ChassisSpec } from "../../rgb/deviceArt";
import { simulateFrame } from "../../rgb/effects";
import { KeyboardView } from "./KeyboardView";

interface KeyboardPreviewProps {
  layout: LedLayout;
  chassis: ChassisSpec;
  effectId: string;
  speedLevel: number;
  brightnessLevel: number;
  color: string;
  screenAspect?: number;
}

// Drives KeyboardView with an animated effect simulation. ~30fps, respects
// reduced-motion (renders a single representative frame instead of looping).
export function KeyboardPreview({
  layout,
  chassis,
  effectId,
  speedLevel,
  brightnessLevel,
  color,
  screenAspect,
}: KeyboardPreviewProps) {
  const [colors, setColors] = useState<Record<string, string>>({});
  const raf = useRef(0);

  useEffect(() => {
    const opts = { speedLevel, brightnessLevel, color };
    const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    if (reduce) {
      setColors(simulateFrame(effectId, layout, 700, opts));
      return;
    }
    let start = 0;
    let last = 0;
    const tick = (now: number) => {
      if (!start) start = now;
      if (now - last >= 33) {
        // throttle ~30fps
        last = now;
        setColors(simulateFrame(effectId, layout, now - start, opts));
      }
      raf.current = requestAnimationFrame(tick);
    };
    raf.current = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf.current);
  }, [effectId, layout, speedLevel, brightnessLevel, color]);

  return (
    <KeyboardView
      layout={layout}
      chassis={chassis}
      colors={colors}
      accent={color}
      screenText={effectId.toUpperCase()}
      screenAspect={screenAspect}
    />
  );
}
