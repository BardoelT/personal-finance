use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::file::write_atomic_with;

const FILE_NAME: &str = "last-project.json";

/// The settings file is tiny; anything bigger is not ours.
const MAX_BYTES: u64 = 64 * 1024;

#[derive(Serialize, Deserialize)]
struct Remembered {
    path: String,
}

/// The path of the last used project file, kept in the app's own config folder
/// so it can be read before any project is open (CSD-002 CON-7). It holds the
/// path only, never project data.
#[derive(Debug, Clone)]
pub struct LastProject {
    file: PathBuf,
}

impl LastProject {
    /// The settings file inside `config_dir`.
    #[must_use]
    pub fn in_dir(config_dir: &Path) -> Self {
        Self {
            file: config_dir.join(FILE_NAME),
        }
    }

    /// The remembered path, if there is one. Unreadable, oversized or malformed
    /// settings and relative paths count as "nothing remembered".
    #[must_use]
    pub fn load(&self) -> Option<PathBuf> {
        let file = fs::File::open(&self.file).ok()?;
        let mut text = String::new();
        file.take(MAX_BYTES).read_to_string(&mut text).ok()?;
        let remembered: Remembered = serde_json::from_str(&text).ok()?;
        let path = PathBuf::from(remembered.path);
        path.is_absolute().then_some(path)
    }

    /// Remembers `path`.
    ///
    /// # Errors
    /// Fails if the path is not valid UTF-8 or the settings cannot be written.
    pub fn store(&self, path: &Path) -> io::Result<()> {
        let text = path
            .to_str()
            .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
        let json = serde_json::to_string_pretty(&Remembered {
            path: text.to_owned(),
        })
        .map_err(io::Error::other)?;
        if let Some(parent) = self.file.parent() {
            fs::create_dir_all(parent)?;
        }
        write_atomic_with(&self.file, |file| {
            use std::io::Write;
            file.write_all(json.as_bytes())
        })
    }

    /// Forgets the remembered path. A missing settings file is fine.
    pub fn clear(&self) {
        let _ = fs::remove_file(&self.file);
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn stores_loads_and_clears_a_path() {
        let dir = TempDir::new().unwrap();
        let memory = LastProject::in_dir(&dir.path().join("config"));
        assert_eq!(memory.load(), None);

        let path = dir.path().join("example-a.wealth");
        memory.store(&path).unwrap();
        assert_eq!(memory.load(), Some(path));

        memory.clear();
        assert_eq!(memory.load(), None);
        memory.clear(); // clearing twice is fine
    }

    #[test]
    fn settings_hold_the_path_only_and_are_private() {
        use std::os::unix::fs::PermissionsExt;
        let dir = TempDir::new().unwrap();
        let memory = LastProject::in_dir(dir.path());
        memory.store(&dir.path().join("example-a.wealth")).unwrap();

        let file = dir.path().join(FILE_NAME);
        let value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
        assert_eq!(value.as_object().unwrap().len(), 1);
        assert!(value["path"].is_string());
        let mode = fs::metadata(&file).unwrap().permissions().mode();
        assert_eq!(mode & 0o077, 0, "mode {mode:o}");
    }

    #[test]
    fn malformed_or_relative_settings_mean_nothing_remembered() {
        let dir = TempDir::new().unwrap();
        let memory = LastProject::in_dir(dir.path());
        let file = dir.path().join(FILE_NAME);
        for content in [
            "not json",
            "{}",
            r#"{"path": 5}"#,
            r#"{"path": "relative/example-a.wealth"}"#,
            "",
        ] {
            fs::write(&file, content).unwrap();
            assert_eq!(memory.load(), None, "{content:?}");
        }
    }
}
