//! Loading, validating, and matching device profiles, plus user-config persistence.
//!
//! Profiles are *data* (TOML). This crate turns them into [`DeviceProfile`] values
//! and answers "which profile, if any, describes this attached device?".

use std::path::Path;

use forge_core::{DeviceProfile, ForgeError, MatchInput};

pub mod presets;
pub use presets::{Preset, PresetStore};

/// Parse a single [`DeviceProfile`] from TOML text.
pub fn parse_profile(toml_str: &str) -> Result<DeviceProfile, ForgeError> {
    toml::from_str(toml_str).map_err(|e| ForgeError::InvalidProfile(e.to_string()))
}

/// A catalog of known device profiles, queried by attached-device descriptor.
#[derive(Clone, Debug, Default)]
pub struct ProfileCatalog {
    profiles: Vec<DeviceProfile>,
}

impl ProfileCatalog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_profiles(profiles: Vec<DeviceProfile>) -> Self {
        Self { profiles }
    }

    /// Load every `*.toml` profile under a directory (non-recursive).
    pub fn from_dir(dir: impl AsRef<Path>) -> Result<Self, ForgeError> {
        let mut profiles = Vec::new();
        let entries = std::fs::read_dir(dir.as_ref()).map_err(|e| ForgeError::Io(e.to_string()))?;
        for entry in entries {
            let path = entry.map_err(|e| ForgeError::Io(e.to_string()))?.path();
            if path.extension().and_then(|e| e.to_str()) != Some("toml") {
                continue;
            }
            let text = std::fs::read_to_string(&path).map_err(|e| ForgeError::Io(e.to_string()))?;
            profiles.push(parse_profile(&text)?);
        }
        Ok(Self { profiles })
    }

    pub fn add(&mut self, profile: DeviceProfile) {
        self.profiles.push(profile);
    }

    pub fn profiles(&self) -> &[DeviceProfile] {
        &self.profiles
    }

    /// The first profile whose matcher selects this device.
    pub fn match_device(&self, candidate: &MatchInput) -> Option<&DeviceProfile> {
        self.profiles.iter().find(|p| p.matcher.matches(candidate))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
        id = "aula.example"
        display_name = "AULA Example"
        vendor = "AULA"

        [matcher]
        vid = 0x258a
        pid = 0x0049
        interface = 1

        [driver]
        family = "sinowealth"
        variant = { report_id = 6, packet_size = 65 }

        [[capabilities]]
        kind = "rgb"
        mode = "per_key"
        max_brightness = 255
        color_order = "RGB"

        [capabilities.layout]
        matrix_size = [6, 17]
        keys = [{ id = "KC_ESC", label = "Esc", x = 0.0, y = 0.0, led_index = 0 }]
    "#;

    #[test]
    fn parses_and_matches() {
        let profile = parse_profile(SAMPLE).expect("profile should parse");
        assert_eq!(profile.id, "aula.example");
        assert_eq!(profile.driver.family, "sinowealth");

        let catalog = ProfileCatalog::from_profiles(vec![profile]);
        let hit = catalog.match_device(&MatchInput {
            vid: 0x258a,
            pid: 0x0049,
            usage_page: None,
            usage: None,
            interface: Some(1),
        });
        assert!(hit.is_some());
        assert_eq!(hit.unwrap().display_name, "AULA Example");
    }

    /// Every profile shipped in the repo's `profiles/` tree must parse. This
    /// guards the templates (and future real profiles) against schema drift.
    #[test]
    fn shipped_profiles_parse() {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../profiles/aula");
        let catalog = ProfileCatalog::from_dir(root).expect("profiles/aula should load");
        assert!(
            catalog.profiles().iter().any(|p| p.id == "aula.f108-pro"),
            "expected the AULA F108 Pro template to be present and parseable"
        );
    }
}
