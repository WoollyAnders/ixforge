import { Alert, Badge, Stack, Text } from "@mantine/core";
import { useStore } from "../../store/useStore";

export function MacroPanel() {
  const cap = useStore((s) => s.capabilities.find((c) => c.kind === "macro"));
  const storage = cap && cap.kind === "macro" ? cap.storage : undefined;

  return (
    <Stack gap="sm">
      <Text fw={600}>Macros</Text>
      {storage?.mode === "on_device" && (
        <Badge variant="light" color="gray">
          On-device · {storage.slots} slots
        </Badge>
      )}
      {storage?.mode === "host_replay" && (
        <Badge variant="light" color="gray">
          Host replay
        </Badge>
      )}
      <Alert color="blue" variant="light" title="Coming in M2">
        The macro recorder and editor land in milestone M2. The capability is
        detected and the UI is ready to wire to the on-device / host-replay paths.
      </Alert>
    </Stack>
  );
}
