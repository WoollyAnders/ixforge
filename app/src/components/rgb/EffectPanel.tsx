import { useRef } from "react";
import { HexColorPicker } from "react-colorful";
import {
  Badge,
  Button,
  ColorSwatch,
  Group,
  SegmentedControl,
  Slider,
  Stack,
  Switch,
  Text,
} from "@mantine/core";
import { useStore } from "../../store/useStore";
import type { ChassisSpec } from "../../rgb/deviceArt";
import type { EffectDescriptor, LedLayout } from "../../types/forge";
import { KeyboardPreview } from "./KeyboardPreview";

const LEVEL_MARKS = [1, 2, 3, 4, 5].map((value) => ({ value }));

const COLOR_PRESETS = [
  "#ff0000",
  "#00ff00",
  "#0000ff",
  "#ffff00",
  "#00ffff",
  "#ff00ff",
  "#ffffff",
  "#000000",
];

function hasParam(e: EffectDescriptor, type: string): boolean {
  return e.params.some((p) => p.type === type);
}

// Arrow labels per directional effect: [value 0, value 1], matching its axis.
const DIRECTION_ARROWS: Record<string, [string, string]> = {
  scrolling: ["↓", "↑"], // top-down / down-up
  rolling: ["→", "←"], // left-right / right-left
  rotating: ["↻", "↺"], // clockwise / counter-clockwise
};

export function EffectPanel({
  layout,
  chassis,
  screenAspect,
}: {
  layout: LedLayout;
  chassis: ChassisSpec;
  screenAspect?: number;
}) {
  const rgb = useStore((s) => s.rgbCapability());
  const selectedEffectId = useStore((s) => s.selectedEffectId);
  const selectEffect = useStore((s) => s.selectEffect);
  const speed = useStore((s) => s.effectSpeed);
  const brightness = useStore((s) => s.effectBrightness);
  const setSpeed = useStore((s) => s.setEffectSpeed);
  const setBrightness = useStore((s) => s.setEffectBrightness);
  const applyEffect = useStore((s) => s.applyEffect);
  const direction = useStore((s) => s.effectDirection);
  const setDirection = useStore((s) => s.setEffectDirection);
  const randomize = useStore((s) => s.effectRandomize);
  const setRandomize = useStore((s) => s.setEffectRandomize);
  const activeColor = useStore((s) => s.activeColor);
  const setActiveColor = useStore((s) => s.setActiveColor);
  const setEffectColor = useStore((s) => s.setEffectColor);

  // The picker fires onChange on every click and every drag tick. Update the
  // color live (preview + swatch) immediately, but debounce the device apply so
  // a drag doesn't flood the board — and, unlike the picker's onChangeEnd, this
  // still fires for a plain click (onChangeEnd can skip clicks). ~90 ms feels
  // instant while collapsing a whole drag into one apply.
  const colorApplyTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const pickColor = (hex: string) => {
    setActiveColor(hex);
    if (colorApplyTimer.current) clearTimeout(colorApplyTimer.current);
    // color-only: update the running effect's color without re-selecting it.
    colorApplyTimer.current = setTimeout(() => void applyEffect(true), 90);
  };

  if (!rgb) return null;
  const selected = rgb.effects.find((e) => e.id === selectedEffectId);

  return (
    <Stack gap="lg">
      {selected && (
        <div>
          <KeyboardPreview
            layout={layout}
            chassis={chassis}
            effectId={selected.id}
            speedLevel={speed}
            brightnessLevel={brightness}
            color={activeColor}
            screenAspect={screenAspect}
          />
          <Text size="xs" c="dimmed" mt={6}>
            On-screen preview · selecting an effect or moving a slider applies to the
            keyboard instantly.
          </Text>
        </div>
      )}

      <div>
        <Text size="sm" fw={600} mb={6}>
          Built-in animations
        </Text>
        <Group gap={8}>
          {rgb.effects.map((e) => (
            <Button
              key={e.id}
              size="compact-sm"
              variant={e.id === selectedEffectId ? "filled" : "light"}
              color={e.id === selectedEffectId ? "brand" : "gray"}
              onClick={() => selectEffect(e.id)}
            >
              {e.name}
            </Button>
          ))}
        </Group>
      </div>

      {selected ? (
        <Stack gap="sm" maw={420}>
          <Group gap="xs">
            <Text fw={600}>{selected.name}</Text>
            {hasParam(selected, "color_list") && (
              <Badge variant="light" color="gray">
                uses active color
              </Badge>
            )}
          </Group>

          {hasParam(selected, "speed") && (
            <div>
              <Text size="xs" c="dimmed">
                Speed
              </Text>
              <Slider
                min={1}
                max={5}
                step={1}
                value={speed}
                onChange={setSpeed}
                onChangeEnd={() => void applyEffect()}
                marks={LEVEL_MARKS}
              />
            </div>
          )}

          {hasParam(selected, "brightness") && (
            <div>
              <Text size="xs" c="dimmed">
                Brightness
              </Text>
              <Slider
                min={1}
                max={5}
                step={1}
                value={brightness}
                onChange={setBrightness}
                onChangeEnd={() => void applyEffect()}
                marks={LEVEL_MARKS}
              />
            </div>
          )}

          {hasParam(selected, "direction") && (
            <div>
              <Text size="xs" c="dimmed" mb={4}>
                Direction
              </Text>
              <SegmentedControl
                size="sm"
                value={String(direction)}
                onChange={(v) => setDirection(Number(v))}
                data={[
                  { label: (DIRECTION_ARROWS[selected.id] ?? ["▶", "◀"])[0], value: "0" },
                  { label: (DIRECTION_ARROWS[selected.id] ?? ["▶", "◀"])[1], value: "1" },
                ]}
              />
            </div>
          )}

          {(hasParam(selected, "randomize") || hasParam(selected, "colorful")) && (
            <Switch
              label={hasParam(selected, "colorful") ? "Colorful (rainbow)" : "Randomize color"}
              checked={randomize}
              onChange={(e) => setRandomize(e.currentTarget.checked)}
            />
          )}

          {hasParam(selected, "color_list") && !randomize && (
            <div>
              <Text size="xs" c="dimmed" mb={4}>
                Color
              </Text>
              <HexColorPicker color={activeColor} onChange={pickColor} />
              <Group gap={6} mt={6}>
                {COLOR_PRESETS.map((c) => (
                  <ColorSwatch
                    key={c}
                    color={c}
                    onClick={() => setEffectColor(c)}
                    style={{ cursor: "pointer" }}
                  />
                ))}
              </Group>
            </div>
          )}
        </Stack>
      ) : (
        <Text c="dimmed" size="sm">
          Select an animation above — it applies to the keyboard instantly.
        </Text>
      )}
    </Stack>
  );
}
