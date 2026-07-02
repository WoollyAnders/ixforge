import { HexColorPicker } from "react-colorful";
import { Badge, Button, ColorSwatch, Group, Slider, Stack, Text } from "@mantine/core";
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
  const activeColor = useStore((s) => s.activeColor);
  const setActiveColor = useStore((s) => s.setActiveColor);

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
              <Slider min={1} max={5} step={1} value={speed} onChange={setSpeed} marks={LEVEL_MARKS} />
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
                marks={LEVEL_MARKS}
              />
            </div>
          )}

          {hasParam(selected, "color_list") && (
            <div>
              <Text size="xs" c="dimmed" mb={4}>
                Color
              </Text>
              <HexColorPicker color={activeColor} onChange={setActiveColor} />
              <Group gap={6} mt={6}>
                {COLOR_PRESETS.map((c) => (
                  <ColorSwatch
                    key={c}
                    color={c}
                    onClick={() => setActiveColor(c)}
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
