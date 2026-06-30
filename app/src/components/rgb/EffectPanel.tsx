import { Badge, Button, Group, Slider, Stack, Text } from "@mantine/core";
import { useStore } from "../../store/useStore";
import type { EffectDescriptor } from "../../types/forge";

const LEVEL_MARKS = [1, 2, 3, 4, 5].map((value) => ({ value }));

function hasParam(e: EffectDescriptor, type: string): boolean {
  return e.params.some((p) => p.type === type);
}

export function EffectPanel() {
  const rgb = useStore((s) => s.rgbCapability());
  const selectedEffectId = useStore((s) => s.selectedEffectId);
  const selectEffect = useStore((s) => s.selectEffect);
  const speed = useStore((s) => s.effectSpeed);
  const brightness = useStore((s) => s.effectBrightness);
  const setSpeed = useStore((s) => s.setEffectSpeed);
  const setBrightness = useStore((s) => s.setEffectBrightness);
  const apply = useStore((s) => s.applyEffect);
  const activeColor = useStore((s) => s.activeColor);

  if (!rgb) return null;
  const selected = rgb.effects.find((e) => e.id === selectedEffectId);

  return (
    <Stack gap="md" maw={640}>
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
        <Stack gap="sm">
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
                marks={LEVEL_MARKS}
              />
            </div>
          )}

          {hasParam(selected, "color_list") && (
            <Group gap="xs">
              <Text size="xs" c="dimmed">
                Color
              </Text>
              <span
                style={{
                  width: 18,
                  height: 18,
                  borderRadius: 4,
                  background: activeColor,
                  border: "1px solid #444",
                }}
              />
              <Text size="xs" c="dimmed">
                (set in the Custom tab)
              </Text>
            </Group>
          )}

          <Button mt="xs" w={200} onClick={() => void apply()}>
            Apply effect
          </Button>
        </Stack>
      ) : (
        <Text c="dimmed" size="sm">
          Select an animation above to configure and apply it.
        </Text>
      )}
    </Stack>
  );
}
