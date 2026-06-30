//! HID transport implementations for IX Forge.
//!
//! [`forge_core::HidTransport`] is the seam; this crate provides the backends.
//! [`MockTransport`] is always available and underpins the golden-byte driver
//! tests. The real [`hidapi_backend::HidapiBackend`] is gated behind the
//! `hidapi-backend` feature (on by default) so the rest of the workspace can be
//! tested with no C library and no hardware.

pub mod mock;

#[cfg(feature = "hidapi-backend")]
pub mod hidapi_backend;

pub use mock::{MockTransport, RecordingTransport};

use forge_core::{ForgeError, HidTransport, MatchInput};

/// A device discovered during enumeration.
#[derive(Clone, Debug)]
pub struct DeviceInfo {
    /// Opaque OS path used to re-open the exact interface.
    pub path: String,
    pub vid: u16,
    pub pid: u16,
    pub usage_page: Option<u16>,
    pub usage: Option<u16>,
    pub interface: Option<i32>,
    pub serial: Option<String>,
    pub product: Option<String>,
}

impl DeviceInfo {
    /// View the descriptor fields used for profile matching.
    pub fn match_input(&self) -> MatchInput {
        MatchInput {
            vid: self.vid,
            pid: self.pid,
            usage_page: self.usage_page,
            usage: self.usage,
            interface: self.interface,
        }
    }
}

/// Enumerate and open HID devices. Implemented once per backend.
pub trait HidBackend: Send + Sync {
    fn enumerate(&self) -> Result<Vec<DeviceInfo>, ForgeError>;
    fn open(&self, info: &DeviceInfo) -> Result<Box<dyn HidTransport>, ForgeError>;
}
