import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { DeviceSummary } from "../types/forge";
import { IS_TAURI } from "./backend";

// Subscribe to device hotplug events. No-op in the browser mock (devices are
// static there); returns an unlisten function to call on cleanup.
export async function subscribeHotplug(
  onAttached: (device: DeviceSummary) => void,
  onDetached: (id: string) => void,
): Promise<UnlistenFn> {
  if (!IS_TAURI) return () => {};
  const a = await listen<DeviceSummary>("device://attached", (e) =>
    onAttached(e.payload),
  );
  const d = await listen<{ id: string }>("device://detached", (e) =>
    onDetached(e.payload.id),
  );
  return () => {
    a();
    d();
  };
}
