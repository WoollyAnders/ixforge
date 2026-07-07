import { useState } from "react";
import {
  Alert,
  Badge,
  Button,
  FileButton,
  Group,
  Image,
  Loader,
  Stack,
  Text,
} from "@mantine/core";
import { useStore } from "../../store/useStore";
import { pushLcdImage } from "../../ipc/commands";

const previewBorder = "1px solid var(--mantine-color-dark-4)";

export function LcdPanel() {
  const cap = useStore((s) => s.capabilities.find((c) => c.kind === "lcd"));
  const lcd = cap && cap.kind === "lcd" ? cap : undefined;
  const selectedId = useStore((s) => s.selectedId);

  const [file, setFile] = useState<File | null>(null);
  const [preview, setPreview] = useState<string | null>(null);
  const [status, setStatus] = useState("");
  const [busy, setBusy] = useState(false);

  const onPick = (f: File | null) => {
    setStatus("");
    setFile(f);
    if (preview) URL.revokeObjectURL(preview);
    setPreview(f ? URL.createObjectURL(f) : null);
  };

  const send = async () => {
    if (!file || !selectedId) return;
    setBusy(true);
    setStatus("Uploading to screen…");
    try {
      const bytes = new Uint8Array(await file.arrayBuffer());
      const log = await pushLcdImage(selectedId, Array.from(bytes));
      setStatus(log || "Sent to screen.");
    } catch (e) {
      setStatus(`Error: ${String(e)}`);
    } finally {
      setBusy(false);
    }
  };

  const dims = lcd ? `${lcd.width}×${lcd.height}` : "the screen";

  return (
    <Stack gap="md" maw={420}>
      <Group gap="xs">
        <Text fw={600}>Screen</Text>
        {lcd && (
          <>
            <Badge variant="light" color="gray">
              {lcd.width}×{lcd.height}
            </Badge>
            <Badge variant="light" color="gray">
              {lcd.format}
            </Badge>
          </>
        )}
      </Group>

      <Text size="sm" c="dimmed">
        Upload a static image or an animated GIF. It's resized to {dims} and sent to the
        keyboard's screen; a GIF plays on-device at its own frame timing.
      </Text>

      <Group>
        <FileButton onChange={onPick} accept="image/png,image/jpeg,image/gif,image/bmp">
          {(props) => (
            <Button variant="light" {...props}>
              Choose image / GIF
            </Button>
          )}
        </FileButton>
        {file && (
          <Text size="sm" c="dimmed">
            {file.name}
          </Text>
        )}
      </Group>

      {preview && (
        <Image
          src={preview}
          w={240}
          h={135}
          fit="contain"
          radius="sm"
          style={{ border: previewBorder, background: "#000" }}
          alt="LCD preview"
        />
      )}

      <Button
        onClick={send}
        disabled={!file || busy || !selectedId}
        leftSection={busy ? <Loader size="xs" /> : undefined}
      >
        Send to screen
      </Button>

      {status && (
        <Text size="sm" c={status.startsWith("Error") ? "red" : "dimmed"}>
          {status}
        </Text>
      )}

      <Alert color="yellow" variant="light" title="Heads-up">
        Sending an image briefly pauses live RGB — the screen and lighting share the same USB
        interface — so lighting resumes on your next change.
      </Alert>
    </Stack>
  );
}
