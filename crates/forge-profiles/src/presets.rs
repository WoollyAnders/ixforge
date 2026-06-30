//! User-saved lighting presets — named per-key configurations the user creates,
//! persisted to a JSON file. Distinct from [`crate::DeviceProfile`] (the device
//! definition): a `Preset` is *user* state, a profile is *device* data.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use forge_core::{Color, ForgeError, KeyId};

/// A named per-key lighting setup the user saved for a device.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Preset {
    pub name: String,
    /// The device this preset belongs to (the selected device's id).
    pub device: String,
    /// Per-key colors. Keys not listed are off.
    pub keys: Vec<(KeyId, Color)>,
}

/// Reads/writes all presets as a single JSON array file.
pub struct PresetStore {
    path: PathBuf,
}

impl PresetStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// All saved presets (across every device). Returns `[]` if the file is absent.
    pub fn load_all(&self) -> Result<Vec<Preset>, ForgeError> {
        match std::fs::read_to_string(&self.path) {
            Ok(text) => serde_json::from_str(&text).map_err(|e| ForgeError::Io(e.to_string())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(e) => Err(ForgeError::Io(e.to_string())),
        }
    }

    /// Presets for one device.
    pub fn list_for(&self, device: &str) -> Result<Vec<Preset>, ForgeError> {
        Ok(self
            .load_all()?
            .into_iter()
            .filter(|p| p.device == device)
            .collect())
    }

    /// Insert or replace a preset (matched by device + name), then persist.
    pub fn save(&self, preset: Preset) -> Result<(), ForgeError> {
        let mut all = self.load_all()?;
        all.retain(|p| !(p.device == preset.device && p.name == preset.name));
        all.push(preset);
        self.write(&all)
    }

    /// Remove a preset by device + name, then persist.
    pub fn delete(&self, device: &str, name: &str) -> Result<(), ForgeError> {
        let mut all = self.load_all()?;
        all.retain(|p| !(p.device == device && p.name == name));
        self.write(&all)
    }

    fn write(&self, all: &[Preset]) -> Result<(), ForgeError> {
        if let Some(parent) = self.path.parent() {
            create_dir(parent)?;
        }
        let json = serde_json::to_string_pretty(all).map_err(|e| ForgeError::Io(e.to_string()))?;
        std::fs::write(&self.path, json).map_err(|e| ForgeError::Io(e.to_string()))
    }
}

fn create_dir(dir: &Path) -> Result<(), ForgeError> {
    std::fs::create_dir_all(dir).map_err(|e| ForgeError::Io(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preset(name: &str, device: &str) -> Preset {
        Preset {
            name: name.into(),
            device: device.into(),
            keys: vec![(KeyId::from("KC_ESC"), Color::RED)],
        }
    }

    #[test]
    fn save_load_delete_roundtrip() {
        let dir = std::env::temp_dir().join(format!("ixforge-presets-{}", std::process::id()));
        let path = dir.join("presets.json");
        let _ = std::fs::remove_file(&path);
        let store = PresetStore::new(&path);

        assert!(
            store.load_all().unwrap().is_empty(),
            "missing file -> empty"
        );

        store.save(preset("Night", "aula.f108-pro")).unwrap();
        store.save(preset("Day", "aula.f108-pro")).unwrap();
        store.save(preset("Night", "other.kbd")).unwrap();
        assert_eq!(store.load_all().unwrap().len(), 3);
        assert_eq!(store.list_for("aula.f108-pro").unwrap().len(), 2);

        // Re-saving the same (device, name) replaces, not duplicates.
        store.save(preset("Night", "aula.f108-pro")).unwrap();
        assert_eq!(store.list_for("aula.f108-pro").unwrap().len(), 2);

        store.delete("aula.f108-pro", "Night").unwrap();
        let remaining = store.list_for("aula.f108-pro").unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].name, "Day");

        std::fs::remove_file(&path).ok();
    }
}
