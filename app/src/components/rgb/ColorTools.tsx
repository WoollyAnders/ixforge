import { HexColorPicker, HexColorInput } from "react-colorful";
import { Button, ColorSwatch, Group, Slider, Stack, Text } from "@mantine/core";
import { useStore } from "../../store/useStore";

// True RGB primaries/secondaries — these are the exact bytes sent to the board.
const PRESETS = [
  "#ff0000", // red
  "#00ff00", // green
  "#0000ff", // blue
  "#ffff00", // yellow
  "#00ffff", // cyan
  "#ff00ff", // magenta
  "#ffffff", // white
  "#000000", // off
];

const HEX_INPUT_STYLE: React.CSSProperties = {
  width: "100%",
  boxSizing: "border-box",
  background: "#141019",
  color: "#e9e6f0",
  border: "1px solid #3a3450",
  borderRadius: 4,
  padding: "4px 8px",
  fontFamily: "monospace",
  textTransform: "uppercase",
};

export function ColorTools() {
  const activeColor = useStore((s) => s.activeColor);
  const setActiveColor = useStore((s) => s.setActiveColor);
  const fillAll = useStore((s) => s.fillAll);
  const clearAll = useStore((s) => s.clearAll);
  const apply = useStore((s) => s.applyToKeyboard);
  const brightness = useStore((s) => s.customBrightness);
  const setBrightness = useStore((s) => s.setCustomBrightness);

  return (
    <Stack gap="sm" w={216} style={{ flexShrink: 0 }}>
      <Text size="sm" fw={600}>
        Color
      </Text>
      <HexColorPicker color={activeColor} onChange={setActiveColor} />
      <HexColorInput
        color={activeColor}
        onChange={setActiveColor}
        prefixed
        style={HEX_INPUT_STYLE}
        aria-label="Hex color"
      />
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
