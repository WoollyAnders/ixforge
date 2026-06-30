import { Divider, Group, Stack, Tabs, Text } from "@mantine/core";
import { useStore } from "../../store/useStore";
import { chassisFor } from "../../rgb/deviceArt";
import { KeyboardCanvas } from "./KeyboardCanvas";
import { ColorTools } from "./ColorTools";
import { EffectPanel } from "./EffectPanel";
import { PresetBar } from "./PresetBar";

export function RgbEditor() {
  const rgb = useStore((s) => s.rgbCapability());
  const capabilities = useStore((s) => s.capabilities);
  const devices = useStore((s) => s.devices);
  const selectedId = useStore((s) => s.selectedId);

  if (!rgb) {
    return <Text c="dimmed">No RGB capability.</Text>;
  }

  const deviceName = devices.find((d) => d.id === selectedId)?.name ?? "";
  const chassis = chassisFor(deviceName, rgb.layout);
  const hasEffects = rgb.effects.length > 0;

  // Screen aspect comes from the device's LCD capability, so the rendition's
  // screen matches the real panel (the F108 Pro's 1.14" panel is 240×135).
  const lcd = capabilities.find((c) => c.kind === "lcd");
  const screenAspect =
    lcd && lcd.kind === "lcd" && lcd.height > 0 ? lcd.width / lcd.height : 16 / 9;

  return (
    <Tabs defaultValue={hasEffects ? "effects" : "custom"} keepMounted={false}>
      <Tabs.List mb="md">
        {hasEffects && <Tabs.Tab value="effects">Effects</Tabs.Tab>}
        <Tabs.Tab value="custom">Custom (per-key)</Tabs.Tab>
      </Tabs.List>

      {hasEffects && (
        <Tabs.Panel value="effects">
          <EffectPanel layout={rgb.layout} chassis={chassis} screenAspect={screenAspect} />
        </Tabs.Panel>
      )}

      <Tabs.Panel value="custom">
        <Stack gap="md">
          <Group align="flex-start" gap="lg" wrap="nowrap">
            <div style={{ flex: 1, minWidth: 0 }}>
              <KeyboardCanvas layout={rgb.layout} chassis={chassis} screenAspect={screenAspect} />
            </div>
            <ColorTools />
          </Group>
          <Divider />
          <PresetBar />
        </Stack>
      </Tabs.Panel>
    </Tabs>
  );
}
