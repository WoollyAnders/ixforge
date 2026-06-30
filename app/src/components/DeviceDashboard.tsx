import { Badge, Button, Group, Stack, Text, UnstyledButton } from "@mantine/core";
import { useStore } from "../store/useStore";

export function DeviceDashboard() {
  const devices = useStore((s) => s.devices);
  const selectedId = useStore((s) => s.selectedId);
  const selectDevice = useStore((s) => s.selectDevice);
  const refresh = useStore((s) => s.refreshDevices);
  const loading = useStore((s) => s.loading);

  return (
    <Stack gap="sm">
      <Group justify="space-between">
        <Text fw={600} size="sm">
          Devices
        </Text>
        <Button
          size="compact-xs"
          variant="subtle"
          loading={loading}
          onClick={() => void refresh()}
        >
          Refresh
        </Button>
      </Group>

      {devices.length === 0 && (
        <Text size="xs" c="dimmed">
          No devices detected. Connect a supported keyboard over USB (wired).
        </Text>
      )}

      {devices.map((d) => {
        const active = d.id === selectedId;
        return (
          <UnstyledButton
            key={d.id}
            onClick={() => void selectDevice(d.id)}
            p="sm"
            style={{
              borderRadius: 8,
              border: "1px solid var(--mantine-color-dark-4)",
              background: active ? "var(--mantine-color-dark-6)" : "transparent",
              outline: active ? "1px solid var(--mantine-color-orange-5)" : "none",
            }}
          >
            <Text size="sm" fw={500}>
              {d.name}
            </Text>
            <Group gap={4} mt={6}>
              {d.capability_kinds.map((k) => (
                <Badge key={k} size="xs" variant="light" color="gray">
                  {k}
                </Badge>
              ))}
            </Group>
          </UnstyledButton>
        );
      })}
    </Stack>
  );
}
