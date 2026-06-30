import { useState } from "react";
import { ActionIcon, Button, Group, Stack, Text, TextInput } from "@mantine/core";
import { useStore } from "../../store/useStore";

// Save / apply / delete named per-key lighting presets for the selected device.
export function PresetBar() {
  const presets = useStore((s) => s.presets);
  const keyColors = useStore((s) => s.keyColors);
  const saveCurrent = useStore((s) => s.saveCurrentPreset);
  const applyPreset = useStore((s) => s.applyPreset);
  const deletePreset = useStore((s) => s.deletePreset);
  const [name, setName] = useState("");

  const hasColors = Object.keys(keyColors).length > 0;
  const trimmed = name.trim();

  const onSave = () => {
    if (!trimmed || !hasColors) return;
    void saveCurrent(trimmed);
    setName("");
  };

  return (
    <Stack gap={8}>
      <Text size="sm" fw={600}>
        Presets
      </Text>

      <Group gap={8}>
        {presets.length === 0 && (
          <Text size="xs" c="dimmed">
            No saved presets yet — paint some keys, name it, and save.
          </Text>
        )}
        {presets.map((p) => (
          <Group key={p.name} gap={2} wrap="nowrap">
            <Button
              size="compact-sm"
              variant="light"
              color="gray"
              onClick={() => void applyPreset(p)}
            >
              {p.name}
            </Button>
            <ActionIcon
              size="sm"
              variant="subtle"
              color="gray"
              aria-label={`Delete preset ${p.name}`}
              onClick={() => void deletePreset(p.name)}
            >
              ×
            </ActionIcon>
          </Group>
        ))}
      </Group>

      <Group gap={8}>
        <TextInput
          size="xs"
          placeholder="Preset name"
          value={name}
          onChange={(e) => setName(e.currentTarget.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") onSave();
          }}
        />
        <Button
          size="xs"
          variant="default"
          onClick={onSave}
          disabled={!trimmed || !hasColors}
        >
          Save current
        </Button>
      </Group>
    </Stack>
  );
}
