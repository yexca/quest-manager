//! Private local device profiles and connection preferences.
//!
//! Profiles deliberately contain only the physical identity and user-facing
//! settings. Pairing codes, ADB keys, and transient network addresses are not
//! persisted here.

use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, sync::Mutex};

const MAX_NAME: usize = 80;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceProfile {
    pub id: String,
    pub display_name: String,
    pub model: String,
    pub connection_preference: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DevicePreferences {
    pub profiles: Vec<DeviceProfile>,
    pub auto_switch: bool,
}

impl Default for DevicePreferences {
    fn default() -> Self {
        Self {
            profiles: Vec::new(),
            auto_switch: true,
        }
    }
}

pub struct DeviceProfileStore {
    path: PathBuf,
    state: Mutex<DevicePreferences>,
}

impl DeviceProfileStore {
    pub fn new(path: PathBuf) -> Self {
        let state = fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        Self {
            path,
            state: Mutex::new(state),
        }
    }

    pub fn get(&self) -> DevicePreferences {
        self.state
            .lock()
            .expect("device profile mutex poisoned")
            .clone()
    }

    pub fn save_profile(&self, mut profile: DeviceProfile) -> Result<DevicePreferences, String> {
        profile.id = profile.id.trim().to_string();
        profile.model = profile.model.trim().to_string();
        profile.display_name = profile.display_name.trim().to_string();
        if profile.id.is_empty() || profile.id.len() > 240 {
            return Err("The device identity is invalid.".into());
        }
        if profile.display_name.is_empty() || profile.display_name.chars().count() > MAX_NAME {
            return Err(format!(
                "Choose a device name with 1–{MAX_NAME} characters."
            ));
        }
        if !matches!(
            profile.connection_preference.as_str(),
            "auto" | "usb" | "wifi"
        ) {
            return Err("Choose USB, Wi-Fi, or automatic connection preference.".into());
        }
        let mut state = self.state.lock().expect("device profile mutex poisoned");
        let mut next = state.clone();
        if next.profiles.iter().any(|item| {
            item.id != profile.id
                && item
                    .display_name
                    .trim()
                    .eq_ignore_ascii_case(&profile.display_name)
        }) {
            return Err("That device name is already in use. Choose a different name.".into());
        }
        if let Some(existing) = next.profiles.iter_mut().find(|item| item.id == profile.id) {
            *existing = profile;
        } else {
            next.profiles.push(profile);
        }
        self.persist(&next)?;
        *state = next.clone();
        Ok(next)
    }

    pub fn set_auto_switch(&self, enabled: bool) -> Result<DevicePreferences, String> {
        let mut state = self.state.lock().expect("device profile mutex poisoned");
        let mut next = state.clone();
        next.auto_switch = enabled;
        self.persist(&next)?;
        *state = next.clone();
        Ok(next)
    }

    fn persist(&self, state: &DevicePreferences) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Could not create device settings storage: {e}"))?;
        }
        let bytes = serde_json::to_vec_pretty(state)
            .map_err(|e| format!("Could not encode device settings: {e}"))?;
        let temporary = self.path.with_extension("json.tmp");
        fs::write(&temporary, bytes).map_err(|e| format!("Could not save device settings: {e}"))?;
        #[cfg(windows)]
        if self.path.exists() {
            fs::remove_file(&self.path)
                .map_err(|e| format!("Could not replace device settings: {e}"))?;
        }
        fs::rename(&temporary, &self.path)
            .map_err(|e| format!("Could not finish saving device settings: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(id: &str, name: &str) -> DeviceProfile {
        DeviceProfile {
            id: id.into(),
            display_name: name.into(),
            model: "Quest 3".into(),
            connection_preference: "auto".into(),
        }
    }

    #[test]
    fn profile_names_are_unique_case_insensitively() {
        let path = std::env::temp_dir().join(format!(
            "quest-manager-device-profile-{}.json",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);
        let store = DeviceProfileStore::new(path.clone());
        store
            .save_profile(profile("DEMO-A", "Living room"))
            .unwrap();
        assert!(
            store
                .save_profile(profile("DEMO-B", "LIVING ROOM"))
                .is_err()
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn repeated_saves_are_loaded_from_disk() {
        let path = std::env::temp_dir().join(format!(
            "quest-manager-device-profile-repeat-{}.json",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);
        let store = DeviceProfileStore::new(path.clone());
        store
            .save_profile(profile("DEMO-A", "Living room"))
            .unwrap();
        store.save_profile(profile("DEMO-A", "Office")).unwrap();
        assert!(store.get().auto_switch);
        store.set_auto_switch(false).unwrap();
        let loaded = DeviceProfileStore::new(path.clone()).get();
        assert!(!loaded.auto_switch);
        assert_eq!(loaded.profiles.len(), 1);
        assert_eq!(loaded.profiles[0].display_name, "Office");
        let _ = fs::remove_file(path);
    }
}
