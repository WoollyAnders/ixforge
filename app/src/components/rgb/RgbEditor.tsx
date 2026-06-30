import { Group, Tabs, Text } from "@mantine/core";
import { useStore } from "../../store/useStore";
import { KeyboardCanvas } from "./KeyboardCanvas";
import { ColorTools } from "./ColorTools";
import { EffectPanel } from "./EffectPanel";

export function RgbEditor() {
  const rgb = useStore((s) => s.rgbCapability());

  if (!rgb) {
    return <Text c="dimmed">No RGB capability.</Text>;
  }

  const hasEffects = rgb.effects.length > 0;

  return (
    <Tabs defaultValue={hasEffects ? "effects" : "custom"} keepMounted={false}>
      <Tabs.List mb="md">
        {hasEffects && <Tabs.Tab value="effects">Effects</Tabs.Tab>}
        <Tabs.Tab value="custom">Custom (per-key)</Tabs.Tab>
      </Tabs.List>

      {hasEffects && (
        <Tabs.Panel value="effects">
          <EffectPanel />
        </Tabs.Panel>
      )}

      <Tabs.Panel value="custom">
        <Group align="flex-start" gap="lg" wrap="nowrap">
          <div style={{ flex: 1, minWidth: 0 }}>
            <KeyboardCanvas layout={rgb.layout} />
          </div>
          <ColorTools />
        </Group>
      </Tabs.Panel>
    </Tabs>
  );
}
