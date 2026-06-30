//! `forge-core` — the pure domain contract for IX Forge.
//!
//! This crate has **no I/O and no platform dependencies**. Every other crate in
//! the workspace depends on `forge-core`; `forge-core` depends on nothing in the
//! workspace. That keeps the contract fast to compile, trivial to unit-test
//! without hardware, and impossible to pollute with backend concerns.
//!
//! The key pieces:
//! - [`capability`] — what a device can do, described richly enough for the UI to
//!   render controls without knowing the device model.
//! - [`profile`] — [`DeviceProfile`], the data that makes adding a keyboard cheap.
//! - [`driver`] — the [`Driver`]/[`DeviceSession`] traits a protocol family implements.
//! - [`transport`] — the [`HidTransport`] seam that decouples drivers from the USB backend.

pub mod capability;
pub mod color;
pub mod command;
pub mod driver;
pub mod error;
pub mod ids;
pub mod profile;
pub mod transport;

pub use capability::{
    Capability, EffectDescriptor, EffectParam, KeyDef, LcdCapability, LcdFeatures, LcdFormat,
    LedLayout, MacroCapability, MacroStorage, RgbCapability, RgbMode, ZoneDef,
};
pub use color::{Color, ColorOrder};
pub use command::{
    DeviceState, EffectSelection, LcdFrame, MacroEvent, MacroProgram, MacroRepeat, RgbCommand,
};
pub use driver::{DeviceSession, Driver};
pub use error::{ForgeError, Result};
pub use ids::{DeviceId, KeyId, MacroSlot, ZoneId};
pub use profile::{DeviceMatcher, DeviceProfile, DriverRef, MatchInput, Provenance};
pub use transport::HidTransport;
