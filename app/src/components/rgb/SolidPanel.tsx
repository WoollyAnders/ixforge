import { HexColorPicker } from "react-colorful";
import { Button, ColorSwatch, Group, Slider, Stack, Text } from "@mantine/core";
import { useStore } from "../../store/useStore";
import type { ChassisSpec } from "../../rgb/deviceArt";
import type { LedLayout } from "../../types/forge";
import { KeyboardView } from "./KeyboardView";

const PRESETS = [
  "#22d3ee", // brand cyan
  "#3b82f6", // blue
  "#ffffff", // white
  "#a855f7", // purple
  "#ff0040", // red
  "#39ff14", // green
  "#ffd700", // gold
  "#000000", // off
];

/** One solid color across the whole board — no per-key painting needed. */
export function SolidPanel({
  layout,
  chassis,
  screenAspect,
}: {
  layout: LedLayout;
  chassis: ChassisSpec;
  screenAspect?: number;
}) {
  const activeColor = useStore((s) => s.activeColor);
  const setActiveColor = useStore((s) => s.setActiveColor);
  const brightness = useStore((s) => s.customBrightness);
  const setBrightness = useStore((s) => s.setCustomBrightness);
  const fillAll = useStore((s) => s.fillAll);

  // Preview: every key shows the chosen color.
  const colors: Record<string, string> = {};
  for (const k of layout.keys) colors[k.id] = activeColor;

  return (
    <Group align="flex-start" gap="lg" wrap="nowrap">
      <div style={{ flex: 1, minWidth: 0 }}>
        <KeyboardView
          layout={layout}
          chassis={chassis}
          colors={colors}
          accent={activeColor}
          screenAspect={screenAspect}
          brightness={brightness / 100}
        />
      </div>
      <Stack gap="sm" w={216} style={{ flexShrink: 0 }}>
        <Text size="sm" fw={600}>
          Solid color
        </Text>
        <HexColorPicker color={activeColor} onChange={setActiveColor} />
        <Group gap={6}>
          {PRESETS.map((c) => (
            <ColorSwatch
              key={c}
              color={c}
              onClick={() => setActiveColor(c)}
              style={{ cursor: "pointer" }}
            />
          ))}
        </Group>
        <div>
          <Text size="xs" c="dimmed">
            Brightness
          </Text>
          <Slider min={0} max={100} step={5} value={brightness} onChange={setBrightness} />
        </div>
        <Button onClick={() => void fillAll()}>Apply to keyboard</Button>
      </Stack>
    </Group>
  );
}
