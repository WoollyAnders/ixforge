import { HexColorPicker } from "react-colorful";
import { Button, Stack, Text } from "@mantine/core";
import { useStore } from "../../store/useStore";

// Color editor for zoned (non-per-key) boards: pick a color, click a zone.
export function ZoneEditor() {
  const rgb = useStore((s) => s.rgbCapability());
  const activeColor = useStore((s) => s.activeColor);
  const setActiveColor = useStore((s) => s.setActiveColor);
  const zoneColors = useStore((s) => s.zoneColors);
  const setZoneColor = useStore((s) => s.setZoneColor);

  if (!rgb || typeof rgb.mode !== "object") return null;
  const zones = rgb.mode.zoned.zones;

  return (
    <Stack gap="sm" w={216} style={{ flexShrink: 0 }}>
      <Text size="sm" fw={600}>
        Zone color
      </Text>
      <HexColorPicker color={activeColor} onChange={setActiveColor} />
      <Text size="xs" c="dimmed">
        Pick a color, then click a zone to apply it.
      </Text>
      <Stack gap={6}>
        {zones.map((z) => (
          <Button
            key={z.id}
            variant="light"
            color="gray"
            justify="flex-start"
            leftSection={
              <span
                style={{
                  width: 14,
                  height: 14,
                  borderRadius: 3,
                  background: zoneColors[z.id] ?? "#2a2733",
                  border: "1px solid #555",
                }}
              />
            }
            onClick={() => void setZoneColor(z.id)}
          >
            {z.label}
          </Button>
        ))}
      </Stack>
    </Stack>
  );
}
