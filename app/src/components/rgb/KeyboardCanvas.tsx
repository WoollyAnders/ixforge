import { useStore } from "../../store/useStore";
import type { LedLayout } from "../../types/forge";
import type { ChassisSpec } from "../../rgb/deviceArt";
import { KeyboardView } from "./KeyboardView";

// Editor surface: the device rendition wired to the store — click a key to paint
// it with the active color (clicking again with the same color clears it).
export function KeyboardCanvas({
  layout,
  chassis,
  screenAspect,
}: {
  layout: LedLayout;
  chassis: ChassisSpec;
  screenAspect?: number;
}) {
  const keyColors = useStore((s) => s.keyColors);
  const paintKey = useStore((s) => s.paintKey);
  const activeColor = useStore((s) => s.activeColor);
  const customBrightness = useStore((s) => s.customBrightness);

  return (
    <KeyboardView
      layout={layout}
      chassis={chassis}
      colors={keyColors}
      onKeyClick={paintKey}
      accent={activeColor}
      screenText="CUSTOM"
      screenAspect={screenAspect}
      brightness={customBrightness / 100}
    />
  );
}
