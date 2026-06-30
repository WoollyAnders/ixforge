//! Macro program model helpers and the (feature-gated) host-side replay engine.
//!
//! The [`forge_core::MacroProgram`] AST itself lives in `forge-core` so it can be
//! serialized over IPC and encoded by on-device drivers without pulling input
//! libraries. This crate adds validation and, behind `host-playback`, a player
//! that injects events on the host for devices without on-device macro storage.

use forge_core::{ForgeError, MacroEvent, MacroProgram};

/// Reject obviously malformed programs before sending them anywhere.
pub fn validate(prog: &MacroProgram) -> Result<(), ForgeError> {
    if prog.events.is_empty() {
        return Err(ForgeError::InvalidArgument("macro has no events".into()));
    }
    Ok(())
}

/// Sum of all explicit delay events, in milliseconds.
pub fn nominal_duration_ms(prog: &MacroProgram) -> u32 {
    prog.events
        .iter()
        .map(|e| match e {
            MacroEvent::Delay { ms } => *ms,
            _ => 0,
        })
        .sum()
}

#[cfg(feature = "host-playback")]
pub mod host;

#[cfg(test)]
mod tests {
    use super::*;
    use forge_core::MacroEvent;

    #[test]
    fn empty_macro_is_invalid() {
        assert!(validate(&MacroProgram::default()).is_err());
    }

    #[test]
    fn duration_sums_delays() {
        let prog = MacroProgram {
            events: vec![
                MacroEvent::KeyDown { code: 4 },
                MacroEvent::Delay { ms: 50 },
                MacroEvent::KeyUp { code: 4 },
                MacroEvent::Delay { ms: 25 },
            ],
            ..Default::default()
        };
        assert!(validate(&prog).is_ok());
        assert_eq!(nominal_duration_ms(&prog), 75);
    }
}
