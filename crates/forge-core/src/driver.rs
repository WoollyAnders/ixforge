//! The driver / session traits a protocol family implements.

use crate::capability::Capability;
use crate::command::{DeviceState, EffectSelection, LcdFrame, MacroProgram, RgbCommand};
use crate::error::ForgeError;
use crate::ids::MacroSlot;
use crate::profile::DeviceProfile;
use crate::transport::HidTransport;

/// A protocol family (e.g. `"sinowealth"`). Stateless and shared; it produces a
/// [`DeviceSession`] when handed a matched profile and an open transport.
pub trait Driver: Send + Sync {
    /// The family key referenced by [`crate::profile::DriverRef::family`].
    fn family(&self) -> &'static str;

    /// Bind a matched profile and an open transport into a live session.
    fn open(
        &self,
        profile: &DeviceProfile,
        transport: Box<dyn HidTransport>,
    ) -> Result<Box<dyn DeviceSession>, ForgeError>;
}

/// A live, opened device.
///
/// Every capability method defaults to [`ForgeError::NotSupported`]; a driver
/// overrides only the ones its device's profile advertises. The app should not
/// call a method whose capability is absent, but the default is defense in depth.
pub trait DeviceSession: Send {
    /// The capabilities this session exposes (mirrors the profile).
    fn capabilities(&self) -> &[Capability];

    fn apply_rgb(&mut self, _cmd: &RgbCommand) -> Result<(), ForgeError> {
        Err(ForgeError::NotSupported)
    }

    fn set_effect(&mut self, _effect: &EffectSelection) -> Result<(), ForgeError> {
        Err(ForgeError::NotSupported)
    }

    fn write_macro(&mut self, _slot: MacroSlot, _prog: &MacroProgram) -> Result<(), ForgeError> {
        Err(ForgeError::NotSupported)
    }

    fn push_lcd(&mut self, _frame: &LcdFrame) -> Result<(), ForgeError> {
        Err(ForgeError::NotSupported)
    }

    /// Read current device state. Many controllers are write-only and return
    /// [`ForgeError::NotSupported`]; callers then treat the saved profile as truth.
    fn read_state(&mut self) -> Result<DeviceState, ForgeError> {
        Err(ForgeError::NotSupported)
    }
}
