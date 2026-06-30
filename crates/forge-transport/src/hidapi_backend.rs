//! The real HID backend, backed by the `hidapi` C library.
//!
//! Gated behind the `hidapi-backend` feature. Builds on Windows and macOS out of
//! the box; on Linux it needs `libudev` headers at build time and a udev rule at
//! runtime for unprivileged access.

use hidapi::{HidApi, HidDevice};

use forge_core::{ForgeError, HidTransport};

use crate::{DeviceInfo, HidBackend};

/// Enumerate/open devices via `hidapi`.
pub struct HidapiBackend {
    api: HidApi,
}

impl HidapiBackend {
    pub fn new() -> Result<Self, ForgeError> {
        let api = HidApi::new().map_err(|e| ForgeError::Transport(e.to_string()))?;
        Ok(Self { api })
    }

    /// Re-scan the USB bus (call before [`HidBackend::enumerate`] to pick up
    /// freshly attached devices).
    pub fn refresh(&mut self) -> Result<(), ForgeError> {
        self.api
            .refresh_devices()
            .map_err(|e| ForgeError::Transport(e.to_string()))
    }
}

impl HidBackend for HidapiBackend {
    fn enumerate(&self) -> Result<Vec<DeviceInfo>, ForgeError> {
        let devices = self
            .api
            .device_list()
            .map(|d| DeviceInfo {
                path: d.path().to_string_lossy().into_owned(),
                vid: d.vendor_id(),
                pid: d.product_id(),
                usage_page: Some(d.usage_page()),
                usage: Some(d.usage()),
                interface: Some(d.interface_number()),
                serial: d.serial_number().map(|s| s.to_owned()),
                product: d.product_string().map(|s| s.to_owned()),
            })
            .collect();
        Ok(devices)
    }

    fn open(&self, info: &DeviceInfo) -> Result<Box<dyn HidTransport>, ForgeError> {
        let path = std::ffi::CString::new(info.path.clone())
            .map_err(|e| ForgeError::Transport(e.to_string()))?;
        let dev = self
            .api
            .open_path(&path)
            .map_err(|e| ForgeError::Transport(e.to_string()))?;
        Ok(Box::new(HidapiTransport { dev }))
    }
}

/// An open device handle wrapping a `hidapi::HidDevice`.
pub struct HidapiTransport {
    dev: HidDevice,
}

impl HidTransport for HidapiTransport {
    fn write_report(&mut self, data: &[u8]) -> Result<usize, ForgeError> {
        self.dev
            .write(data)
            .map_err(|e| ForgeError::Transport(e.to_string()))
    }

    fn send_feature_report(&mut self, data: &[u8]) -> Result<(), ForgeError> {
        self.dev
            .send_feature_report(data)
            .map_err(|e| ForgeError::Transport(e.to_string()))
    }

    fn get_feature_report(&mut self, buf: &mut [u8]) -> Result<usize, ForgeError> {
        self.dev
            .get_feature_report(buf)
            .map_err(|e| ForgeError::Transport(e.to_string()))
    }

    fn read(&mut self, buf: &mut [u8], timeout_ms: i32) -> Result<usize, ForgeError> {
        self.dev
            .read_timeout(buf, timeout_ms)
            .map_err(|e| ForgeError::Transport(e.to_string()))
    }
}
