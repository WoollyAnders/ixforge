//! [`DeviceProfile`] — the data that makes adding a keyboard cheap.
//!
//! A profile describes *what* a device is and *where* its LEDs are; the driver
//! named by [`DriverRef::family`] knows *how* to encode bytes. Adding a new model
//! in a known family is ideally just a new profile file with no Rust changes.

use serde::{Deserialize, Serialize};

use crate::capability::Capability;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeviceProfile {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    /// Stable id, e.g. `"aula.f75.byk916"`.
    pub id: String,
    pub display_name: String,
    pub vendor: String,
    pub matcher: DeviceMatcher,
    pub driver: DriverRef,
    pub capabilities: Vec<Capability>,
    #[serde(default)]
    pub provenance: Provenance,
}

fn default_schema_version() -> u32 {
    1
}

/// How to recognize a device on the USB bus.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeviceMatcher {
    pub vid: u16,
    pub pid: u16,
    #[serde(default)]
    pub usage_page: Option<u16>,
    #[serde(default)]
    pub usage: Option<u16>,
    #[serde(default)]
    pub interface: Option<i32>,
}

/// The descriptor fields of a candidate device, fed to [`DeviceMatcher::matches`].
#[derive(Clone, Copy, Debug)]
pub struct MatchInput {
    pub vid: u16,
    pub pid: u16,
    pub usage_page: Option<u16>,
    pub usage: Option<u16>,
    pub interface: Option<i32>,
}

impl DeviceMatcher {
    /// True when this matcher selects the candidate. VID/PID must always match;
    /// optional fields only constrain when both the matcher and the candidate
    /// specify them (so a profile can ignore fields it doesn't care about).
    pub fn matches(&self, c: &MatchInput) -> bool {
        if self.vid != c.vid || self.pid != c.pid {
            return false;
        }
        let opt_ok = |a: Option<u16>, b: Option<u16>| match (a, b) {
            (Some(x), Some(y)) => x == y,
            _ => true,
        };
        opt_ok(self.usage_page, c.usage_page)
            && opt_ok(self.usage, c.usage)
            && match (self.interface, c.interface) {
                (Some(x), Some(y)) => x == y,
                _ => true,
            }
    }
}

/// Which driver handles this device, plus family-specific data knobs.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DriverRef {
    /// Selects the [`crate::driver::Driver`] impl (e.g. `"sinowealth"`).
    pub family: String,
    /// Family-specific parameters (report id, packet size, protocol revision…).
    /// Kept as free-form JSON so new knobs are *data*, not code changes.
    #[serde(default)]
    pub variant: serde_json::Value,
}

/// Clean-room provenance: where this device's protocol knowledge came from.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Provenance {
    #[serde(default)]
    pub captured_by: Option<String>,
    #[serde(default)]
    pub captured_on: Option<String>,
    #[serde(default)]
    pub firmware_revision: Option<String>,
    #[serde(default)]
    pub capture_files: Vec<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(vid: u16, pid: u16, iface: Option<i32>) -> MatchInput {
        MatchInput {
            vid,
            pid,
            usage_page: None,
            usage: None,
            interface: iface,
        }
    }

    #[test]
    fn matcher_vid_pid() {
        let m = DeviceMatcher {
            vid: 0x258a,
            pid: 0x0049,
            usage_page: None,
            usage: None,
            interface: Some(1),
        };
        assert!(m.matches(&input(0x258a, 0x0049, Some(1))));
        assert!(!m.matches(&input(0x258a, 0x0049, Some(0)))); // wrong interface
        assert!(!m.matches(&input(0x0001, 0x0049, Some(1)))); // wrong vid
                                                              // candidate omits interface → optional field doesn't disqualify
        assert!(m.matches(&input(0x258a, 0x0049, None)));
    }
}
