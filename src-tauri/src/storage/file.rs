use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::StorageError;
use crate::domain::{BankAccounts, Income, Investments, ProjectData, Settings};

/// Version of the project file format this app writes and understands.
pub const FORMAT_VERSION: u32 = 1;

/// Largest project file the app will read, a bound on untrusted input.
pub const MAX_FILE_BYTES: u64 = 50 * 1024 * 1024;

#[derive(Serialize)]
struct FileRef<'a> {
    format_version: u32,
    income: &'a Income,
    investments: &'a Investments,
    bank_accounts: &'a BankAccounts,
    settings: &'a Settings,
}

/// A file without a `settings` section gets the default settings (CSD-003 NG-5),
/// and one without an `investments` section has no investment accounts
/// (CSD-005 REQ-19), or without a `bank_accounts` section no bank accounts
/// (CSD-009 REQ-23).
#[derive(Deserialize)]
struct FileOwned {
    income: Income,
    #[serde(default)]
    investments: Investments,
    #[serde(default)]
    bank_accounts: BankAccounts,
    #[serde(default)]
    settings: Settings,
}

/// Renders the project file: JSON with a `format_version` and decimal strings.
pub(super) fn to_json(data: &ProjectData) -> Result<String, StorageError> {
    serde_json::to_string_pretty(&FileRef {
        format_version: FORMAT_VERSION,
        income: &data.income,
        investments: &data.investments,
        bank_accounts: &data.bank_accounts,
        settings: &data.settings,
    })
    .map_err(|error| StorageError::SaveFailed(io::Error::other(error)))
}

/// Parses and validates project file contents.
///
/// Files with a missing, zero or newer `format_version` are rejected.
pub(super) fn from_json(text: &str) -> Result<ProjectData, StorageError> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|_| StorageError::InvalidFile)?;
    let version = value
        .get("format_version")
        .and_then(serde_json::Value::as_u64)
        .ok_or(StorageError::InvalidFile)?;
    if version == 0 || version > u64::from(FORMAT_VERSION) {
        return Err(StorageError::InvalidFile);
    }
    let file: FileOwned = serde_json::from_value(value).map_err(|_| StorageError::InvalidFile)?;
    file.income
        .validate()
        .map_err(|_| StorageError::InvalidFile)?;
    file.investments
        .validate()
        .map_err(|_| StorageError::InvalidFile)?;
    file.bank_accounts
        .validate()
        .map_err(|_| StorageError::InvalidFile)?;
    Ok(ProjectData {
        income: file.income,
        investments: file.investments,
        bank_accounts: file.bank_accounts,
        settings: file.settings,
    })
}

/// Reads a project file, refusing anything that is not a regular file or is
/// larger than [`MAX_FILE_BYTES`].
pub(super) fn read(path: &Path) -> Result<ProjectData, StorageError> {
    let file = File::open(path).map_err(StorageError::OpenFailed)?;
    let metadata = file.metadata().map_err(StorageError::OpenFailed)?;
    if !metadata.is_file() || metadata.len() > MAX_FILE_BYTES {
        return Err(StorageError::InvalidFile);
    }
    let mut text = String::new();
    file.take(MAX_FILE_BYTES)
        .read_to_string(&mut text)
        .map_err(|error| match error.kind() {
            io::ErrorKind::InvalidData => StorageError::InvalidFile,
            _ => StorageError::OpenFailed(error),
        })?;
    from_json(&text)
}

/// Writes the project file atomically: contents go to a temporary file in the
/// same directory which is renamed over the target, so a failed write never
/// damages an existing file (CSD-002 REQ-5).
pub(super) fn write(path: &Path, data: &ProjectData) -> Result<(), StorageError> {
    let json = to_json(data)?;
    write_atomic_with(path, |file| file.write_all(json.as_bytes()))
        .map_err(StorageError::SaveFailed)
}

/// Atomic write with an injectable writer, so failures can be tested.
pub(super) fn write_atomic_with(
    path: &Path,
    write: impl FnOnce(&mut File) -> io::Result<()>,
) -> io::Result<()> {
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
    let directory = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let temporary = directory.join(format!(
        ".{}.{}.tmp",
        name.to_string_lossy(),
        Uuid::new_v4().simple()
    ));

    let result = (|| {
        let mut file = create_private(&temporary)?;
        write(&mut file)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        // Best effort: the original file is untouched either way.
        let _ = fs::remove_file(&temporary);
    }
    result
}

/// Creates a new file readable only by the owner: it holds financial data.
fn create_private(path: &Path) -> io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
}
