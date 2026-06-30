import { HexColorPicker } from "react-colorful";
import { Button, ColorSwatch, Group, Stack, Text } from "@mantine/core";
import { useStore } from "../../store/useStore";

const PRESETS = [
  "#ff5a00",
  "#ff0040",
  "#00e0ff",
  "#39ff14",
  "#ffffff",
  "#8a2be2",
  "#ffd700",
  "#000000",
];

export function ColorTools() {
  const activeColor = useStore((s) => s.activeColor);
  const setActiveColor = useStore((s) => s.setActiveColor);
  const fillAll = useStore((s) => s.fillAll);
  const clearAll = useStore((s) => s.clearAll);
  const apply = useStore((s) => s.applyToKeyboard);

  return (
    <Stack gap="sm" w={216} style={{ flexShrink: 0 }}>
      <Text size="sm" fw={600}>
        Color
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
      <Text size="xs" c="dimmed">
        Click keys to paint them with the active color, then apply to the device.
      </Text>
      <Button onClick={() => void apply()}>Apply to keyboard</Button>
      <Button variant="light" onClick={() => void fillAll()}>
        Fill all
      </Button>
      <Button variant="subtle" color="gray" onClick={() => void clearAll()}>
        Clear
      </Button>
    </Stack>
  );
}
