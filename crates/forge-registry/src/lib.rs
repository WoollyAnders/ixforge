//! Pairing enumerated devices with profiles and drivers, then opening sessions.
//!
//! This crate is intentionally hardware-agnostic: it works against the
//! [`HidBackend`] trait, so the whole matching+opening path is testable with a
//! fake backend and `MockTransport` — no real USB and no `hidapi` feature.

use std::collections::HashSet;

use forge_core::{DeviceId, DeviceProfile, DeviceSession, Driver, ForgeError};
use forge_profiles::ProfileCatalog;
use forge_transport::{DeviceInfo, HidBackend};

/// An enumerated device paired with the profile that describes it.
pub struct MatchedDevice<'a> {
    pub info: DeviceInfo,
    pub profile: &'a DeviceProfile,
}

impl MatchedDevice<'_> {
    pub fn id(&self) -> DeviceId {
        device_id(&self.info)
    }
}

/// Stable per-device id, preferring serial, falling back to the OS path.
pub fn device_id(info: &DeviceInfo) -> DeviceId {
    let suffix = info.serial.clone().unwrap_or_else(|| info.path.clone());
    DeviceId(format!("{:04x}:{:04x}:{}", info.vid, info.pid, suffix))
}

/// Pair each enumerated device with a known profile; unknown devices are dropped.
pub fn match_devices<'a>(
    infos: Vec<DeviceInfo>,
    catalog: &'a ProfileCatalog,
) -> Vec<MatchedDevice<'a>> {
    infos
        .into_iter()
        .filter_map(|info| {
            catalog
                .match_device(&info.match_input())
                .map(|profile| MatchedDevice { info, profile })
        })
        .collect()
}

/// Open a live session for a matched device using the appropriate driver.
pub fn open_matched(
    backend: &dyn HidBackend,
    matched: &MatchedDevice<'_>,
    drivers: &[Box<dyn Driver>],
) -> Result<Box<dyn DeviceSession>, ForgeError> {
    let family = &matched.profile.driver.family;
    let driver = drivers
        .iter()
        .find(|d| d.family() == family)
        .ok_or_else(|| ForgeError::InvalidProfile(format!("no driver for family {family:?}")))?;
    let transport = backend.open(&matched.info)?;
    driver.open(matched.profile, transport)
}

/// What changed between two enumeration snapshots.
pub struct Delta {
    /// Devices present now but not in the previous snapshot.
    pub attached: Vec<DeviceInfo>,
    /// Ids present in the previous snapshot but gone now.
    pub detached: Vec<DeviceId>,
}

/// Tracks the set of seen devices across polls so the app can emit hotplug
/// events. Pure and hardware-agnostic — feed it each enumeration result.
#[derive(Default)]
pub struct DeviceWatcher {
    seen: HashSet<DeviceId>,
}

impl DeviceWatcher {
    pub fn new() -> Self {
        Self::default()
    }

    /// Diff `current` against the last snapshot, update internal state, and
    /// return what was attached/detached since.
    pub fn diff(&mut self, current: Vec<DeviceInfo>) -> Delta {
        let current_set: HashSet<DeviceId> = current.iter().map(device_id).collect();
        let attached = current
            .into_iter()
            .filter(|i| !self.seen.contains(&device_id(i)))
            .collect();
        let detached = self
            .seen
            .iter()
            .filter(|id| !current_set.contains(id))
            .cloned()
            .collect();
        self.seen = current_set;
        Delta { attached, detached }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_core::{Color, HidTransport, KeyId, RgbCommand};
    use forge_profiles::parse_profile;
    use forge_transport::MockTransport;

    const PROFILE: &str = r#"
        id = "aula.test"
        display_name = "AULA Test"
        vendor = "AULA"
        [matcher]
        vid = 0x258a
        pid = 0x0049
        [driver]
        family = "sinowealth"
        [[capabilities]]
        kind = "rgb"
        mode = "per_key"
        [capabilities.layout]
        matrix_size = [1, 1]
        keys = [{ id = "KC_ESC", label = "Esc", x = 0.0, y = 0.0, led_index = 0 }]
    "#;

    fn info() -> DeviceInfo {
        info_sn("SN123")
    }

    fn info_sn(sn: &str) -> DeviceInfo {
        DeviceInfo {
            path: format!("dev/{sn}"),
            vid: 0x258a,
            pid: 0x0049,
            usage_page: None,
            usage: None,
            interface: None,
            serial: Some(sn.into()),
            product: Some("AULA Test".into()),
        }
    }

    #[test]
    fn watcher_reports_attach_and_detach() {
        let mut w = DeviceWatcher::new();

        let d = w.diff(vec![info_sn("A"), info_sn("B")]);
        assert_eq!(d.attached.len(), 2, "both new on first poll");
        assert!(d.detached.is_empty());

        // B stays, C arrives, A leaves.
        let d = w.diff(vec![info_sn("B"), info_sn("C")]);
        assert_eq!(d.attached.len(), 1);
        assert_eq!(d.attached[0].serial.as_deref(), Some("C"));
        assert_eq!(d.detached, vec![device_id(&info_sn("A"))]);

        // No change.
        let d = w.diff(vec![info_sn("B"), info_sn("C")]);
        assert!(d.attached.is_empty() && d.detached.is_empty());
    }

    /// A fake backend that hands out MockTransports and records the last one.
    struct FakeBackend {
        last: std::sync::Mutex<Option<MockTransport>>,
    }

    impl HidBackend for FakeBackend {
        fn enumerate(&self) -> Result<Vec<DeviceInfo>, ForgeError> {
            Ok(vec![info()])
        }
        fn open(&self, _info: &DeviceInfo) -> Result<Box<dyn HidTransport>, ForgeError> {
            let mock = MockTransport::new();
            *self.last.lock().unwrap() = Some(mock.clone());
            Ok(Box::new(mock))
        }
    }

    #[test]
    fn match_and_open_session_end_to_end() {
        let catalog = ProfileCatalog::from_profiles(vec![parse_profile(PROFILE).unwrap()]);
        let backend = FakeBackend {
            last: std::sync::Mutex::new(None),
        };
        let drivers = forge_drivers::all_drivers();

        let matched = match_devices(backend.enumerate().unwrap(), &catalog);
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].id(), DeviceId("258a:0049:SN123".into()));

        let mut session = open_matched(&backend, &matched[0], &drivers).expect("open");
        session
            .apply_rgb(&RgbCommand::SetKeys(vec![(
                KeyId::from("KC_ESC"),
                Color::BLUE,
            )]))
            .expect("apply");

        // The driver wrote to the transport the fake backend handed out.
        let mock = backend.last.lock().unwrap().clone().unwrap();
        let reports = mock.feature_writes();
        assert_eq!(reports.len(), 1);
        // Blue, default RGB order → 00 00 ff at the payload offset.
        assert_eq!(&reports[0][4..7], &[0x00, 0x00, 0xff]);
    }

    #[test]
    fn unknown_device_is_dropped() {
        let catalog = ProfileCatalog::from_profiles(vec![parse_profile(PROFILE).unwrap()]);
        let mut other = info();
        other.pid = 0xffff;
        assert!(match_devices(vec![other], &catalog).is_empty());
    }
}
