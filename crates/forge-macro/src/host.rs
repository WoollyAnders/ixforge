//! Host-side macro replay via `enigo`.
//!
//! Compiled only with the `host-playback` feature (off by default, so the rest of
//! the workspace tests with no platform input deps). This is a scaffold: the
//! event-to-`enigo` translation is filled in when the macro feature (M2) lands.
//! API targets `enigo` 0.2; adjust if the dependency is bumped.

use enigo::Enigo;

use forge_core::{ForgeError, MacroEvent, MacroProgram};

/// Injects macro events into the host OS input stream.
pub struct HostPlayer {
    _enigo: Enigo,
}

impl HostPlayer {
    pub fn new() -> Self {
        Self {
            _enigo: Enigo::new(),
        }
    }

    /// Replay a program once. (TODO: map each [`MacroEvent`] to an `enigo` call.)
    pub fn play(&mut self, prog: &MacroProgram) -> Result<(), ForgeError> {
        for event in &prog.events {
            match event {
                MacroEvent::Delay { ms } => {
                    std::thread::sleep(std::time::Duration::from_millis(*ms as u64))
                }
                // TODO(M2): KeyDown/KeyUp/Text/MouseButton/MouseMove → enigo.
                _ => {}
            }
        }
        Ok(())
    }
}

impl Default for HostPlayer {
    fn default() -> Self {
        Self::new()
    }
}
