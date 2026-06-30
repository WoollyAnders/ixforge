import { useEffect } from "react";
import { Badge, Box, Group, Text, Title } from "@mantine/core";
import { useStore } from "./store/useStore";
import { DeviceDashboard } from "./components/DeviceDashboard";
import { CapabilityRouter } from "./components/CapabilityRouter";
import { IS_TAURI } from "./ipc/backend";

const border = "1px solid var(--mantine-color-dark-4)";

export default function App() {
  const refreshDevices = useStore((s) => s.refreshDevices);
  const status = useStore((s) => s.status);

  useEffect(() => {
    void refreshDevices();
  }, [refreshDevices]);

  return (
    <Box style={{ display: "grid", gridTemplateRows: "auto 1fr auto", height: "100vh" }}>
      <Group justify="space-between" px="md" py="sm" style={{ borderBottom: border }}>
        <Group gap="xs" align="baseline">
          <Title order={3} c="orange.4">
            IX Forge
          </Title>
          <Text size="sm" c="dimmed">
            peripheral configurator
          </Text>
        </Group>
        {!IS_TAURI && (
          <Badge color="yellow" variant="light">
            Browser preview · mock device
          </Badge>
        )}
      </Group>

      <Box style={{ display: "grid", gridTemplateColumns: "260px 1fr", minHeight: 0 }}>
        <Box p="md" style={{ borderRight: border, overflowY: "auto" }}>
          <DeviceDashboard />
        </Box>
        <Box p="md" style={{ overflowY: "auto" }}>
          <CapabilityRouter />
        </Box>
      </Box>

      <Group px="md" py="xs" style={{ borderTop: border }}>
        <Text size="xs" c="dimmed">
          {status}
        </Text>
      </Group>
    </Box>
  );
}
