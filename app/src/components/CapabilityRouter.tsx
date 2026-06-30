import { Tabs, Text } from "@mantine/core";
import { useStore } from "../store/useStore";
import { RgbEditor } from "./rgb/RgbEditor";
import { MacroPanel } from "./macro/MacroPanel";
import { LcdPanel } from "./lcd/LcdPanel";

// The keystone of the capability-driven UI: it renders panels purely from what
// the selected device advertises — no per-model branching anywhere.
export function CapabilityRouter() {
  const capabilities = useStore((s) => s.capabilities);
  const selectedId = useStore((s) => s.selectedId);

  if (!selectedId) {
    return <Text c="dimmed">Select a device to configure.</Text>;
  }

  const has = (kind: string) => capabilities.some((c) => c.kind === kind);
  const firstTab = has("rgb")
    ? "rgb"
    : has("macro")
      ? "macro"
      : has("lcd")
        ? "lcd"
        : undefined;

  if (!firstTab) {
    return <Text c="dimmed">This device exposes no configurable capabilities yet.</Text>;
  }

  return (
    <Tabs defaultValue={firstTab} keepMounted={false}>
      <Tabs.List>
        {has("rgb") && <Tabs.Tab value="rgb">Lighting</Tabs.Tab>}
        {has("macro") && <Tabs.Tab value="macro">Macros</Tabs.Tab>}
        {has("lcd") && <Tabs.Tab value="lcd">Screen</Tabs.Tab>}
      </Tabs.List>

      {has("rgb") && (
        <Tabs.Panel value="rgb" pt="md">
          <RgbEditor />
        </Tabs.Panel>
      )}
      {has("macro") && (
        <Tabs.Panel value="macro" pt="md">
          <MacroPanel />
        </Tabs.Panel>
      )}
      {has("lcd") && (
        <Tabs.Panel value="lcd" pt="md">
          <LcdPanel />
        </Tabs.Panel>
      )}
    </Tabs>
  );
}
