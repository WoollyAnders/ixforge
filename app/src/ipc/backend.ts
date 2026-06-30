// Detect whether we're running inside the Tauri shell (vs a plain browser).
// Tauri v2 injects `__TAURI_INTERNALS__` on window.
export const IS_TAURI =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
