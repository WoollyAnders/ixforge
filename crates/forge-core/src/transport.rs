//! The HID transport seam.

use crate::error::ForgeError;

/// An open, exclusive handle to a single HID device.
///
/// This trait is the seam that decouples protocol drivers from the concrete USB
/// backend. `forge-transport` provides implementations (`hidapi` today; `nusb` or
/// `async-hid` later) and a `MockTransport` for tests. A driver only ever sees a
/// `Box<dyn HidTransport>`, so swapping the backend touches no driver code.
///
/// Byte conventions follow the HID report model: for output and feature reports,
/// `data[0]` is the report ID (use `0` when the device has no numbered reports).
pub trait HidTransport: Send {
    /// Send an output report (interrupt OUT transfer).
    fn write_report(&mut self, data: &[u8]) -> Result<usize, ForgeError>;

    /// Send a feature report (control transfer, `SET_REPORT`).
    fn send_feature_report(&mut self, data: &[u8]) -> Result<(), ForgeError>;

    /// Fetch a feature report (`GET_REPORT`); set `buf[0]` to the report ID first.
    /// Returns the number of bytes read.
    fn get_feature_report(&mut self, buf: &mut [u8]) -> Result<usize, ForgeError>;

    /// Read an input report (interrupt IN) with a timeout in milliseconds
    /// (`-1` blocks, `0` is non-blocking). Returns the number of bytes read.
    fn read(&mut self, buf: &mut [u8], timeout_ms: i32) -> Result<usize, ForgeError>;
}
