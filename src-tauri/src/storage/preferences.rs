use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::file::write_atomic_with;
use super::StorageError;
use crate::domain::Theme;

const FILE_NAME: &str = "preferences.json";

/// The preference file is tiny; anything bigger is not ours.
const MAX_BYTES: u64 = 64 * 1024;

#[derive(Serialize, Deserialize)]
struct Stored {
    theme: Theme,
}

/// The application preferences, kept in the app's own config folder next to
/// the last used project (CSD-008 CON-1). They hold preferences only, never
/// project data.
#[derive(Debug, Clone)]
pub struct Preferences {
    file: Option<PathBuf>,
}

impl Preferences {
    /// The preference file inside `config_dir`.
    #[must_use]
    pub fn in_dir(config_dir: &Path) -> Self {
        Self {
            file: Some(config_dir.join(FILE_NAME)),
        }
    }

    /// Used when the app has no config folder: the default theme is used and
    /// nothing can be saved.
    #[must_use]
    pub fn unavailable() -> Self {
        Self { file: None }
    }

    /// The saved theme. A missing, unreadable, oversized or malformed file, or
    /// an unknown theme, counts as System (REQ-8).
    #[must_use]
    pub fn load_theme(&self) -> Theme {
        self.read()
            .map_or_else(Theme::default, |stored| stored.theme)
    }

    fn read(&self) -> Option<Stored> {
        let file = fs::File::open(self.file.as_ref()?).ok()?;
        let metadata = file.metadata().ok()?;
        if !metadata.is_file() || metadata.len() > MAX_BYTES {
            return None;
        }
        let mut text = String::new();
        file.take(MAX_BYTES).read_to_string(&mut text).ok()?;
        serde_json::from_str(&text).ok()
    }

    /// Saves `theme`.
    ///
    /// # Errors
    /// Returns `PreferencesSaveFailed` if the preference file cannot be written.
    pub fn store_theme(&self, theme: Theme) -> Result<(), StorageError> {
        self.write(theme)
            .map_err(StorageError::PreferencesSaveFailed)
    }

    fn write(&self, theme: Theme) -> io::Result<()> {
        let path = self
            .file
            .as_ref()
            .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
        let json = serde_json::to_string_pretty(&Stored { theme }).map_err(io::Error::other)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        write_atomic_with(path, |file| file.write_all(json.as_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_csd008_ac_2_1_without_a_file_the_theme_is_system() {
        let dir = TempDir::new().unwrap();
        assert_eq!(Preferences::in_dir(dir.path()).load_theme(), Theme::System);
        assert_eq!(Preferences::unavailable().load_theme(), Theme::System);
    }

    #[test]
    fn test_csd008_ac_5_1_the_theme_survives_a_new_session() {
        let dir = TempDir::new().unwrap();
        // The config folder does not exist yet: it is created on the first save.
        let config = dir.path().join("config");
        Preferences::in_dir(&config)
            .store_theme(Theme::Dark)
            .unwrap();
        assert_eq!(Preferences::in_dir(&config).load_theme(), Theme::Dark);
        Preferences::in_dir(&config)
            .store_theme(Theme::Light)
            .unwrap();
        assert_eq!(Preferences::in_dir(&config).load_theme(), Theme::Light);
    }

    #[test]
    fn test_csd008_ac_5_2_the_file_holds_the_theme_only_and_is_private() {
        use std::os::unix::fs::PermissionsExt;
        let dir = TempDir::new().unwrap();
        Preferences::in_dir(dir.path())
            .store_theme(Theme::Dark)
            .unwrap();

        let file = dir.path().join(FILE_NAME);
        let value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
        assert_eq!(value, serde_json::json!({ "theme": "dark" }));
        let mode = fs::metadata(&file).unwrap().permissions().mode();
        assert_eq!(mode & 0o077, 0, "mode {mode:o}");
    }

    #[test]
    fn test_csd008_ac_5_3_the_project_file_holds_no_theme() {
        let json = super::super::file::to_json(&crate::domain::ProjectData::default()).unwrap();
        assert!(!json.contains("theme"), "{json}");
    }

    #[test]
    fn test_csd008_ac_8_1_a_missing_or_invalid_file_means_system() {
        let dir = TempDir::new().unwrap();
        let preferences = Preferences::in_dir(dir.path());
        let file = dir.path().join(FILE_NAME);
        let oversized = format!(r#"{{"theme": "dark"}}{}"#, " ".repeat(70 * 1024));
        for content in [
            "",
            r#""dark""#,
            "dark",
            "not json",
            r#"{"theme": "sepia"}"#,
            r#"{"theme": "Dark"}"#,
            r#"{"theme": 7}"#,
            r#"{"theme": null}"#,
            "{}",
            oversized.as_str(),
        ] {
            fs::write(&file, content).unwrap();
            assert_eq!(preferences.load_theme(), Theme::System, "{content:.40?}");
        }
    }

    #[test]
    fn test_csd008_ac_8_1_a_directory_or_unreadable_file_means_system() {
        use std::os::unix::fs::PermissionsExt;
        let dir = TempDir::new().unwrap();
        let preferences = Preferences::in_dir(dir.path());
        let file = dir.path().join(FILE_NAME);

        fs::create_dir(&file).unwrap();
        assert_eq!(preferences.load_theme(), Theme::System);
        fs::remove_dir(&file).unwrap();

        fs::write(&file, r#"{"theme": "dark"}"#).unwrap();
        fs::set_permissions(&file, fs::Permissions::from_mode(0o000)).unwrap();
        assert_eq!(preferences.load_theme(), Theme::System);
    }

    #[test]
    fn test_csd008_ac_8_1_unknown_fields_are_ignored() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join(FILE_NAME),
            r#"{"theme": "light", "future": true}"#,
        )
        .unwrap();
        assert_eq!(Preferences::in_dir(dir.path()).load_theme(), Theme::Light);
    }

    #[test]
    fn test_csd008_ac_10_1_a_failed_save_reports_a_code_and_no_path() {
        use std::os::unix::fs::PermissionsExt;
        let dir = TempDir::new().unwrap();
        let preferences = Preferences::in_dir(dir.path());
        preferences.store_theme(Theme::Light).unwrap();

        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o500)).unwrap();
        let error = preferences.store_theme(Theme::Dark).unwrap_err();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();

        assert_eq!(error.code(), "PREFERENCES_SAVE_FAILED");
        assert!(!error.to_string().contains(dir.path().to_str().unwrap()));
        assert_eq!(preferences.load_theme(), Theme::Light);
    }

    #[test]
    fn test_csd008_ac_10_1_without_a_config_folder_saving_fails() {
        let error = Preferences::unavailable()
            .store_theme(Theme::Dark)
            .unwrap_err();
        assert_eq!(error.code(), "PREFERENCES_SAVE_FAILED");
    }
}
