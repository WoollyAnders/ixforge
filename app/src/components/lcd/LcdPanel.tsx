import { Alert, Badge, Group, Stack, Text } from "@mantine/core";
import { useStore } from "../../store/useStore";

export function LcdPanel() {
  const cap = useStore((s) => s.capabilities.find((c) => c.kind === "lcd"));
  const lcd = cap && cap.kind === "lcd" ? cap : undefined;

  return (
    <Stack gap="sm">
      <Text fw={600}>Screen</Text>
      {lcd && (
        <Group gap="xs">
          <Badge variant="light" color="gray">
            {lcd.width}×{lcd.height}
          </Badge>
          <Badge variant="light" color="gray">
            {lcd.format}
          </Badge>
        </Group>
      )}
      <Alert color="blue" variant="light" title="Coming in M3">
        The LCD designer (image / text / system-monitor widgets) lands in
        milestone M3, sized to the screen this device reports.
      </Alert>
    </Stack>
  );
}
