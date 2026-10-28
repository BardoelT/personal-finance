use std::path::{Path, PathBuf};

use serde::Serialize;

use super::{file, LastProject, StorageError};
use crate::domain::{Income, Investments, ProjectData, Settings};

/// The extension of project files.
const EXTENSION: &str = "wealth";

/// Whether the project differs from disk, and whether it has a file yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct ProjectStatus {
    pub unsaved_changes: bool,
    pub has_path: bool,
}

/// Result of a save that may ask for a path first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveOutcome {
    Saved,
    /// The user cancelled the path dialog; nothing was written.
    Cancelled,
}

/// Result of an open that asks for a file first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenOutcome {
    Opened,
    /// The user cancelled the file dialog; nothing changed.
    Cancelled,
}

/// The project in memory: its data, the file it belongs to and the state last
/// written to or read from that file (CSD-002).
#[derive(Debug, Default)]
pub struct Project {
    data: ProjectData,
    saved: ProjectData,
    path: Option<PathBuf>,
    /// Where the last used path is remembered, if anywhere (REQ-10).
    memory: Option<LastProject>,
    /// Why the remembered file could not be reopened at start (REQ-11).
    startup_notice: Option<StorageError>,
}

impl Project {
    /// An empty project with no path (REQ-6, REQ-8).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The income data.
    #[must_use]
    pub fn income(&self) -> &Income {
        &self.data.income
    }

    /// Mutable income data. Edits show up in [`Project::status`] by comparison
    /// with the last saved state, so no separate flag can go stale (REQ-3).
    pub fn income_mut(&mut self) -> &mut Income {
        &mut self.data.income
    }

    /// The investment accounts and statements (CSD-005).
    #[must_use]
    pub fn investments(&self) -> &Investments {
        &self.data.investments
    }

    /// Mutable investments. Like the income data, changes show up in
    /// [`Project::status`] by comparison with the last saved state.
    pub fn investments_mut(&mut self) -> &mut Investments {
        &mut self.data.investments
    }

    /// The project settings (CSD-003).
    #[must_use]
    pub fn settings(&self) -> &Settings {
        &self.data.settings
    }

    /// Mutable project settings. Like the income data, changes show up in
    /// [`Project::status`] by comparison (CSD-003 AC-4.2).
    pub fn settings_mut(&mut self) -> &mut Settings {
        &mut self.data.settings
    }

    /// Unsaved-changes indicator and whether a path is set.
    #[must_use]
    pub fn status(&self) -> ProjectStatus {
        ProjectStatus {
            unsaved_changes: self.data != self.saved,
            has_path: self.path.is_some(),
        }
    }

    /// Replaces the project with an empty one that has no path (REQ-6).
    ///
    /// The remembered path is left alone: starting a new project is not using
    /// another file (REQ-10).
    pub fn reset(&mut self) {
        let memory = self.memory.take();
        *self = Self::new();
        self.memory = memory;
    }

    /// Starts using `memory` and reopens the file it remembers, if any (REQ-8).
    ///
    /// If that file cannot be opened the project stays empty, the path is
    /// forgotten so the error does not repeat, and the reason is kept for
    /// [`Project::take_startup_notice`] (REQ-11).
    pub fn restore(&mut self, memory: LastProject) {
        let remembered = memory.load();
        self.memory = Some(memory);
        let Some(path) = remembered else {
            return;
        };
        if let Err(error) = self.open(&path) {
            if let Some(memory) = &self.memory {
                memory.clear();
            }
            self.startup_notice = Some(error);
        }
    }

    /// The reason the last project was not reopened, once.
    pub fn take_startup_notice(&mut self) -> Option<StorageError> {
        self.startup_notice.take()
    }

    /// Remembers the current path. Best effort: a project that was saved or
    /// opened fine must not fail because the settings could not be written.
    fn remember(&self) {
        if let (Some(memory), Some(path)) = (&self.memory, &self.path) {
            let _ = memory.store(path);
        }
    }

    /// Writes to the current path.
    ///
    /// # Errors
    /// Returns `NoPath` when the project has no file yet, or `SaveFailed`; in
    /// both cases the unsaved indicator stays and any existing file is intact.
    pub fn save(&mut self) -> Result<(), StorageError> {
        let path = self.path.clone().ok_or(StorageError::NoPath)?;
        self.save_as(path)
    }

    /// Writes to `path`, which becomes the project's path (REQ-1).
    ///
    /// # Errors
    /// Returns `SaveFailed`; the project keeps its previous path and stays unsaved.
    pub fn save_as(&mut self, path: PathBuf) -> Result<(), StorageError> {
        let path = with_extension(path);
        file::write(&path, &self.data)?;
        self.saved = self.data.clone();
        self.path = Some(path);
        self.remember();
        Ok(())
    }

    /// Saves to the current path, or asks for one if there is none yet. A
    /// `None` from `ask_path` means the user cancelled (AC-1.3, AC-1.5).
    ///
    /// # Errors
    /// Returns `SaveFailed`.
    pub fn save_asking(
        &mut self,
        ask_path: impl FnOnce() -> Option<PathBuf>,
    ) -> Result<SaveOutcome, StorageError> {
        if self.path.is_some() {
            self.save()?;
        } else {
            let Some(path) = ask_path() else {
                return Ok(SaveOutcome::Cancelled);
            };
            self.save_as(path)?;
        }
        Ok(SaveOutcome::Saved)
    }

    /// Always asks for a path, even if there is one (Save As, AC-1.4).
    ///
    /// # Errors
    /// Returns `SaveFailed`.
    pub fn save_as_asking(
        &mut self,
        ask_path: impl FnOnce() -> Option<PathBuf>,
    ) -> Result<SaveOutcome, StorageError> {
        let Some(path) = ask_path() else {
            return Ok(SaveOutcome::Cancelled);
        };
        self.save_as(path)?;
        Ok(SaveOutcome::Saved)
    }

    /// Replaces the project with the contents of `path` (REQ-2).
    ///
    /// # Errors
    /// Returns `OpenFailed` or `InvalidFile`; the project is left unchanged.
    pub fn open(&mut self, path: &Path) -> Result<(), StorageError> {
        let data = file::read(path)?;
        self.saved = data.clone();
        self.data = data;
        self.path = Some(path.to_path_buf());
        self.remember();
        Ok(())
    }

    /// Opens the file `ask_path` returns; `None` means the user cancelled.
    ///
    /// # Errors
    /// Returns `OpenFailed` or `InvalidFile`; the project is left unchanged.
    pub fn open_asking(
        &mut self,
        ask_path: impl FnOnce() -> Option<PathBuf>,
    ) -> Result<OpenOutcome, StorageError> {
        let Some(path) = ask_path() else {
            return Ok(OpenOutcome::Cancelled);
        };
        self.open(&path)?;
        Ok(OpenOutcome::Opened)
    }
}

fn with_extension(path: PathBuf) -> PathBuf {
    if path.extension().is_some() {
        path
    } else {
        path.with_extension(EXTENSION)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::fs;

    use tempfile::TempDir;

    use super::*;
    use crate::domain::{Amounts, JobId, JobInput, YearMonth};
    use crate::storage::file::{write_atomic_with, FORMAT_VERSION};

    fn add_job(project: &mut Project, employer: &str) -> JobId {
        project
            .income_mut()
            .create_job(&JobInput {
                employer: employer.to_owned(),
                start_date: Some("01/03/2024".to_owned()),
                ..JobInput::default()
            })
            .unwrap()
            .id
    }

    fn add_paycheck(project: &mut Project, job: JobId, month: &str, gross: &str) {
        project
            .income_mut()
            .add_paycheck(
                job,
                YearMonth::parse(month).unwrap(),
                Amounts::parse(gross, "", "").unwrap(),
                YearMonth::parse("2026-10").unwrap(),
            )
            .unwrap();
    }

    fn saved_project(dir: &TempDir, name: &str) -> (Project, PathBuf) {
        let mut project = Project::new();
        add_job(&mut project, "Example Employer");
        let path = dir.path().join(name);
        project.save_as(path.clone()).unwrap();
        (project, path)
    }

    fn job_count(project: &Project) -> usize {
        project.income().jobs().len()
    }

    fn file_job_count(path: &Path) -> usize {
        file::read(path).unwrap().income.jobs().len()
    }

    // ---- REQ-1: save ----

    #[test]
    fn test_csd002_ac_1_1_save_and_reload_round_trips() {
        let dir = TempDir::new().unwrap();
        let mut project = Project::new();
        let first = add_job(&mut project, "Example Employer");
        let second = add_job(&mut project, "Other Example Employer");
        for month in ["2026-07", "2026-08", "2026-09"] {
            add_paycheck(&mut project, first, month, "3000.00");
        }
        add_paycheck(&mut project, second, "2026-08", "1000.50");
        add_paycheck(&mut project, second, "2026-09", "1000.50");
        let path = dir.path().join("example-a.wealth");
        project.save_as(path.clone()).unwrap();

        let mut fresh = Project::new();
        fresh.open(&path).unwrap();
        assert_eq!(fresh.income(), project.income());
        assert_eq!(job_count(&fresh), 2);
        let paychecks = fresh.income().paychecks(first).unwrap();
        assert_eq!(paychecks.len(), 3);
        assert_eq!(paychecks[0].amounts.gross_salary().to_string(), "3000.00");
    }

    #[test]
    fn test_csd002_ac_1_2_write_failure_keeps_changes_unsaved() {
        let dir = TempDir::new().unwrap();
        let mut project = Project::new();
        add_job(&mut project, "Example Employer");
        let missing = dir.path().join("no-such-dir").join("example-a.wealth");

        let error = project.save_as(missing.clone()).unwrap_err();
        assert_eq!(error.code(), "PROJECT_SAVE_FAILED");
        let message = error.to_string();
        assert!(!message.contains(dir.path().to_str().unwrap()));
        assert!(!message.contains("No such file"));
        assert_eq!(
            project.status(),
            ProjectStatus {
                unsaved_changes: true,
                has_path: false
            }
        );
        assert!(!missing.exists());
    }

    #[test]
    fn test_csd002_ac_1_3_path_is_asked_once_then_reused() {
        let dir = TempDir::new().unwrap();
        let mut project = Project::new();
        add_job(&mut project, "Example Employer");
        let path = dir.path().join("example-a.wealth");
        let asked = Cell::new(0);
        let ask = || {
            asked.set(asked.get() + 1);
            Some(path.clone())
        };

        assert_eq!(project.save_asking(ask).unwrap(), SaveOutcome::Saved);
        assert_eq!(asked.get(), 1);
        assert_eq!(file_job_count(&path), 1);

        add_job(&mut project, "Second Example Employer");
        assert_eq!(project.save_asking(ask).unwrap(), SaveOutcome::Saved);
        assert_eq!(asked.get(), 1, "second save must not ask again");
        assert_eq!(file_job_count(&path), 2);
    }

    #[test]
    fn test_csd002_ac_1_4_save_as_asks_again() {
        let dir = TempDir::new().unwrap();
        let (mut project, path_a) = saved_project(&dir, "example-a.wealth");
        let path_b = dir.path().join("example-b.wealth");

        let outcome = project.save_as_asking(|| Some(path_b.clone())).unwrap();
        assert_eq!(outcome, SaveOutcome::Saved);
        assert_eq!(file_job_count(&path_b), 1);

        add_job(&mut project, "Second Example Employer");
        project.save().unwrap();
        assert_eq!(file_job_count(&path_b), 2, "later saves go to example-b");
        assert_eq!(file_job_count(&path_a), 1, "example-a is left as it was");
    }

    #[test]
    fn test_csd002_ac_1_5_cancelling_the_dialog_saves_nothing() {
        let dir = TempDir::new().unwrap();
        let mut project = Project::new();
        add_job(&mut project, "Example Employer");

        assert_eq!(
            project.save_asking(|| None).unwrap(),
            SaveOutcome::Cancelled
        );
        assert_eq!(
            project.save_as_asking(|| None).unwrap(),
            SaveOutcome::Cancelled
        );
        assert!(project.status().unsaved_changes);
        assert!(!project.status().has_path);
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    #[test]
    fn saving_without_a_path_is_an_error_not_a_write() {
        let mut project = Project::new();
        assert_eq!(project.save().unwrap_err().code(), "PROJECT_NO_PATH");
    }

    #[test]
    fn a_path_without_extension_gets_the_project_extension() {
        let dir = TempDir::new().unwrap();
        let mut project = Project::new();
        add_job(&mut project, "Example Employer");
        project.save_as(dir.path().join("example")).unwrap();
        assert!(dir.path().join("example.wealth").exists());
        assert!(!dir.path().join("example").exists());
    }

    // ---- REQ-2: open ----

    #[test]
    fn test_csd002_ac_2_1_open_valid_file() {
        let dir = TempDir::new().unwrap();
        let mut project = Project::new();
        let job = add_job(&mut project, "Example Employer");
        add_paycheck(&mut project, job, "2026-08", "3000.00");
        add_paycheck(&mut project, job, "2026-09", "3000.00");
        let path = dir.path().join("example-a.wealth");
        project.save_as(path.clone()).unwrap();

        let mut opened = Project::new();
        let outcome = opened.open_asking(|| Some(path.clone())).unwrap();
        assert_eq!(outcome, OpenOutcome::Opened);
        assert_eq!(job_count(&opened), 1);
        assert_eq!(opened.income().entry_count(job), 2);
        assert!(opened.status().has_path);
    }

    #[test]
    fn test_csd002_ac_2_2_invalid_file_is_rejected() {
        let dir = TempDir::new().unwrap();
        let mut project = Project::new();
        add_job(&mut project, "Example Employer");
        let before = project.income().clone();

        let garbage = [
            ("not-json.wealth", "this is not a project file"),
            (
                "no-version.wealth",
                r#"{"income":{"jobs":[],"paychecks":[]}}"#,
            ),
            ("wrong-shape.wealth", r#"{"format_version":1,"income":42}"#),
            (
                "orphan-paycheck.wealth",
                r#"{"format_version":1,"income":{"jobs":[],"paychecks":[{
                    "job_id":"00000000-0000-4000-8000-000000000000","month":"2026-09",
                    "gross_salary":"3000.00","holiday_pay":"0.00","bonus":"0.00"}]}}"#,
            ),
        ];
        for (name, content) in garbage {
            let path = dir.path().join(name);
            fs::write(&path, content).unwrap();
            let error = project.open(&path).unwrap_err();
            assert_eq!(error.code(), "PROJECT_INVALID_FILE", "{name}");
            assert_eq!(
                project.income(),
                &before,
                "{name} must not change the project"
            );
            assert!(!project.status().has_path, "{name} must not set a path");
        }
    }

    #[test]
    fn test_csd002_ac_2_3_file_from_newer_version_is_rejected() {
        let dir = TempDir::new().unwrap();
        let (mut project, path) = saved_project(&dir, "example-a.wealth");
        let newer = fs::read_to_string(&path)
            .unwrap()
            .replace("\"format_version\": 1", "\"format_version\": 2");
        assert!(newer.contains("\"format_version\": 2"));
        let newer_path = dir.path().join("newer.wealth");
        fs::write(&newer_path, newer).unwrap();
        let before = project.income().clone();

        let error = project.open(&newer_path).unwrap_err();
        assert_eq!(error.code(), "PROJECT_INVALID_FILE");
        assert_eq!(project.income(), &before);
    }

    #[test]
    fn opening_a_missing_file_or_a_directory_fails_without_changes() {
        let dir = TempDir::new().unwrap();
        let mut project = Project::new();
        add_job(&mut project, "Example Employer");
        let before = project.income().clone();

        let missing = project
            .open(&dir.path().join("missing.wealth"))
            .unwrap_err();
        assert_eq!(missing.code(), "PROJECT_OPEN_FAILED");
        assert!(!missing.to_string().contains(dir.path().to_str().unwrap()));
        let directory = project.open(dir.path()).unwrap_err();
        assert_eq!(directory.code(), "PROJECT_INVALID_FILE");
        assert_eq!(project.income(), &before);
    }

    #[test]
    fn cancelling_the_open_dialog_changes_nothing() {
        let mut project = Project::new();
        add_job(&mut project, "Example Employer");
        let before = project.income().clone();
        assert_eq!(
            project.open_asking(|| None).unwrap(),
            OpenOutcome::Cancelled
        );
        assert_eq!(project.income(), &before);
    }

    // ---- REQ-3: unsaved indicator ----

    #[test]
    fn test_csd002_ac_3_1_indicator_follows_edits() {
        let dir = TempDir::new().unwrap();
        let (_, path) = saved_project(&dir, "example-a.wealth");
        let mut project = Project::new();
        project.open(&path).unwrap();
        assert!(!project.status().unsaved_changes);

        add_job(&mut project, "Second Example Employer");
        assert!(project.status().unsaved_changes);

        project.save().unwrap();
        assert!(!project.status().unsaved_changes);
    }

    #[test]
    fn indicator_clears_when_an_edit_is_undone() {
        let dir = TempDir::new().unwrap();
        let (mut project, _) = saved_project(&dir, "example-a.wealth");
        let job = add_job(&mut project, "Second Example Employer");
        assert!(project.status().unsaved_changes);
        project.income_mut().delete_job(job).unwrap();
        assert!(!project.status().unsaved_changes);
    }

    // ---- CSD-001 REQ-29, REQ-30 and CSD-005 REQ-28, REQ-29: moving an entry ----

    fn move_paycheck(project: &mut Project, job: JobId, from: &str, to: &str) -> bool {
        let month = |text: &str| YearMonth::parse(text).unwrap();
        project
            .income_mut()
            .update_paycheck(
                job,
                month(from),
                month(to),
                Amounts::parse("3000.00", "", "").unwrap(),
                month("2026-10"),
            )
            .is_ok()
    }

    #[test]
    fn test_csd001_ac_29_7_a_move_is_an_unsaved_change() {
        let dir = TempDir::new().unwrap();
        let (mut project, _) = saved_project(&dir, "example-a.wealth");
        let job = project.income().jobs()[0].id;
        add_paycheck(&mut project, job, "2026-05", "3000.00");
        project.save().unwrap();
        assert!(!project.status().unsaved_changes);

        assert!(move_paycheck(&mut project, job, "2026-05", "2026-04"));
        assert!(project.status().unsaved_changes);
        // Moving it back restores the saved state, so the indicator clears.
        assert!(move_paycheck(&mut project, job, "2026-04", "2026-05"));
        assert!(!project.status().unsaved_changes);
    }

    #[test]
    fn test_csd001_ac_30_4_a_rejected_move_is_not_an_unsaved_change() {
        let dir = TempDir::new().unwrap();
        let (mut project, _) = saved_project(&dir, "example-a.wealth");
        let job = project.income().jobs()[0].id;
        add_paycheck(&mut project, job, "2026-04", "2000.00");
        add_paycheck(&mut project, job, "2026-05", "3000.00");
        project.save().unwrap();

        // In the future, and already taken.
        assert!(!move_paycheck(&mut project, job, "2026-05", "2026-11"));
        assert!(!move_paycheck(&mut project, job, "2026-05", "2026-04"));
        assert!(!project.status().unsaved_changes);
    }

    #[test]
    fn test_csd005_ac_28_8_a_move_is_an_unsaved_change() {
        use crate::domain::investments::{AccountInput, Balance, NetDeposits, Note, Period};
        let dir = TempDir::new().unwrap();
        let (mut project, _) = saved_project(&dir, "example-a.wealth");
        let account = project
            .investments_mut()
            .create_account(&AccountInput {
                name: "Example Brokerage".to_owned(),
                statement_period: Some("Monthly".to_owned()),
            })
            .unwrap()
            .id;
        let period = |text: &str| Period::parse(text).unwrap();
        let today = YearMonth::parse("2026-10").unwrap();
        project
            .investments_mut()
            .add_statement(
                account,
                period("2026-09"),
                Balance::parse("12500.00").unwrap(),
                NetDeposits::default(),
                Note::default(),
                today,
            )
            .unwrap();
        project.save().unwrap();
        assert!(!project.status().unsaved_changes);

        project
            .investments_mut()
            .update_statement(
                account,
                period("2026-09"),
                period("2026-08"),
                Balance::parse("12500.00").unwrap(),
                NetDeposits::default(),
                Note::default(),
                today,
            )
            .unwrap();
        assert!(project.status().unsaved_changes);
    }

    // REQ-4 (confirm close) and REQ-7 (confirm before replacing) are dialogs in
    // the UI: see the manual checklist in CSD-002 section 7.

    // ---- REQ-5: failed save leaves the file intact ----

    #[test]
    fn test_csd002_ac_5_1_interrupted_write_does_not_corrupt_the_file() {
        let dir = TempDir::new().unwrap();
        let (_, path) = saved_project(&dir, "example-a.wealth");
        let original = fs::read(&path).unwrap();

        let result = write_atomic_with(&path, |file| {
            use std::io::Write;
            file.write_all(b"{\"format_version\": 1, \"inco")?;
            Err(std::io::Error::other("disk full"))
        });

        assert!(result.is_err());
        assert_eq!(fs::read(&path).unwrap(), original);
        assert_eq!(file_job_count(&path), 1);
        let leftovers: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(
            leftovers.len(),
            1,
            "temporary file must be removed: {leftovers:?}"
        );
    }

    #[test]
    fn test_csd002_ac_5_1_failed_save_reports_the_spec_error_code() {
        let dir = TempDir::new().unwrap();
        let (mut project, path) = saved_project(&dir, "example-a.wealth");
        add_job(&mut project, "Second Example Employer");
        // Make the target a directory so the final rename fails.
        fs::remove_file(&path).unwrap();
        fs::create_dir(&path).unwrap();

        let error = project.save().unwrap_err();
        assert_eq!(error.code(), "PROJECT_SAVE_FAILED");
        assert!(project.status().unsaved_changes);
        let leftovers = fs::read_dir(dir.path()).unwrap().count();
        assert_eq!(leftovers, 1, "temporary file must be removed");
    }

    // ---- REQ-6: new project ----

    #[test]
    fn test_csd002_ac_6_1_new_project_is_empty() {
        let dir = TempDir::new().unwrap();
        let (mut project, _) = saved_project(&dir, "example-a.wealth");
        project.reset();
        assert_eq!(job_count(&project), 0);
        assert_eq!(
            project.status(),
            ProjectStatus {
                unsaved_changes: false,
                has_path: false
            }
        );
        assert_eq!(project.save().unwrap_err().code(), "PROJECT_NO_PATH");
    }

    #[test]
    fn test_csd002_ac_6_2_saved_file_is_untouched_by_new() {
        let dir = TempDir::new().unwrap();
        let (mut project, path) = saved_project(&dir, "example-a.wealth");
        project.reset();
        drop(project);
        assert_eq!(file_job_count(&path), 1);
    }

    // ---- REQ-8, REQ-10, REQ-11: reopen the last project ----

    fn memory_in(dir: &TempDir) -> LastProject {
        LastProject::in_dir(&dir.path().join("config"))
    }

    /// A fresh "process": an empty project that then restores the last file.
    fn restart(dir: &TempDir) -> Project {
        let mut project = Project::new();
        project.restore(memory_in(dir));
        project
    }

    #[test]
    fn test_csd002_ac_8_1_first_launch_has_nothing_to_reopen() {
        let dir = TempDir::new().unwrap();
        let mut project = restart(&dir);
        assert_eq!(job_count(&project), 0);
        assert!(!project.status().has_path);
        assert!(!project.status().unsaved_changes);
        assert!(project.take_startup_notice().is_none());
    }

    #[test]
    fn test_csd002_ac_8_2_the_last_project_is_reopened() {
        let dir = TempDir::new().unwrap();
        let mut project = restart(&dir);
        add_job(&mut project, "Example Employer");
        let path = dir.path().join("example-a.wealth");
        project.save_as(path.clone()).unwrap();
        drop(project);

        let mut next = restart(&dir);
        assert_eq!(job_count(&next), 1);
        assert_eq!(
            next.status(),
            ProjectStatus {
                unsaved_changes: false,
                has_path: true
            }
        );
        assert!(next.take_startup_notice().is_none());
        // Saving goes back to the reopened file without asking for a path.
        add_job(&mut next, "Second Example Employer");
        next.save().unwrap();
        assert_eq!(file_job_count(&path), 2);
    }

    #[test]
    fn test_csd002_ac_10_1_save_remembers_the_path() {
        let dir = TempDir::new().unwrap();
        let mut project = restart(&dir);
        add_job(&mut project, "Example Employer");
        project
            .save_as(dir.path().join("example-a.wealth"))
            .unwrap();
        assert_eq!(
            memory_in(&dir).load(),
            Some(dir.path().join("example-a.wealth"))
        );
    }

    #[test]
    fn test_csd002_ac_10_2_open_and_save_as_move_the_remembered_path() {
        let dir = TempDir::new().unwrap();
        let (_, path_a) = saved_project(&dir, "example-a.wealth");
        let mut project = restart(&dir);
        project.open(&path_a).unwrap();
        assert_eq!(memory_in(&dir).load(), Some(path_a));

        let path_c = dir.path().join("example-c.wealth");
        project.save_as(path_c.clone()).unwrap();
        assert_eq!(memory_in(&dir).load(), Some(path_c.clone()));
        let next = restart(&dir);
        assert!(next.status().has_path);
        assert_eq!(job_count(&next), 1);
    }

    #[test]
    fn test_csd002_ac_10_3_new_project_keeps_the_remembered_path() {
        let dir = TempDir::new().unwrap();
        let mut project = restart(&dir);
        add_job(&mut project, "Example Employer");
        let path = dir.path().join("example-a.wealth");
        project.save_as(path.clone()).unwrap();

        project.reset();
        add_job(&mut project, "Unsaved Example Employer");
        drop(project); // quit without saving

        assert_eq!(memory_in(&dir).load(), Some(path));
        let next = restart(&dir);
        assert_eq!(job_count(&next), 1);
    }

    #[test]
    fn test_csd002_ac_10_4_a_failed_save_keeps_the_remembered_path() {
        let dir = TempDir::new().unwrap();
        let mut project = restart(&dir);
        add_job(&mut project, "Example Employer");
        let path = dir.path().join("example-a.wealth");
        project.save_as(path.clone()).unwrap();

        let unwritable = dir.path().join("no-such-dir").join("example-b.wealth");
        assert!(project.save_as(unwritable).is_err());
        assert_eq!(memory_in(&dir).load(), Some(path));
    }

    #[test]
    fn test_csd002_ac_11_1_missing_or_invalid_file_starts_empty_with_a_notice() {
        let cases = [
            ("missing", None, "PROJECT_OPEN_FAILED"),
            (
                "invalid",
                Some("this is not a project file"),
                "PROJECT_INVALID_FILE",
            ),
        ];
        for (name, content, code) in cases {
            let dir = TempDir::new().unwrap();
            let path = dir.path().join("example-a.wealth");
            if let Some(content) = content {
                fs::write(&path, content).unwrap();
            }
            memory_in(&dir).store(&path).unwrap();

            let mut project = restart(&dir);
            assert_eq!(job_count(&project), 0, "{name}");
            assert!(!project.status().has_path, "{name}");
            let notice = project.take_startup_notice().expect(name);
            assert_eq!(notice.code(), code, "{name}");
            let text = notice.to_string();
            assert!(
                !text.contains(dir.path().to_str().unwrap()),
                "{name}: {text}"
            );
            assert!(!text.contains("example-a"), "{name}: {text}");
            assert!(project.take_startup_notice().is_none(), "{name}: only once");
            assert_eq!(memory_in(&dir).load(), None, "{name}: path is forgotten");
        }
    }

    #[test]
    fn test_csd002_ac_11_2_the_error_is_not_repeated_at_the_next_start() {
        let dir = TempDir::new().unwrap();
        memory_in(&dir)
            .store(&dir.path().join("example-a.wealth"))
            .unwrap();
        let mut first = restart(&dir);
        assert!(first.take_startup_notice().is_some());

        let mut second = restart(&dir);
        assert!(second.take_startup_notice().is_none());
        assert_eq!(job_count(&second), 0);
        assert!(!second.status().has_path);
    }

    #[test]
    fn a_project_without_memory_still_saves_and_opens() {
        let dir = TempDir::new().unwrap();
        let (project, path) = saved_project(&dir, "example-a.wealth");
        assert_eq!(job_count(&project), 1);
        assert_eq!(file_job_count(&path), 1);
        assert_eq!(memory_in(&dir).load(), None);
    }

    // ---- CSD-003: project settings ----

    use crate::domain::Currency;

    #[test]
    fn test_csd003_ac_7_1_currency_survives_save_and_open() {
        let dir = TempDir::new().unwrap();
        let mut project = Project::new();
        add_job(&mut project, "Example Employer");
        project.settings_mut().set_currency("GBP").unwrap();
        let path = dir.path().join("example-a.wealth");
        project.save_as(path.clone()).unwrap();

        let mut fresh = Project::new();
        fresh.open(&path).unwrap();
        assert_eq!(fresh.settings().currency, Currency::Gbp);
        assert!(!fresh.status().unsaved_changes);
    }

    #[test]
    fn test_csd003_ac_7_2_a_file_without_settings_opens_with_the_defaults() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("example-a.wealth");
        // Also has the old per-job currency, which is ignored (NG-5).
        fs::write(
            &path,
            r#"{"format_version":1,"income":{"jobs":[{"id":"671166f4-5827-4794-bcdc-54d7824fb650",
                "employer":"Example Employer","title":null,"start_date":"2024-03-01",
                "end_date":null,"currency":"USD"}],"paychecks":[]}}"#,
        )
        .unwrap();
        let mut project = Project::new();
        project.open(&path).unwrap();
        assert_eq!(project.settings().currency, Currency::Eur);
        assert_eq!(job_count(&project), 1);
    }

    #[test]
    fn test_csd003_ac_7_3_the_settings_are_a_section_of_the_file() {
        let dir = TempDir::new().unwrap();
        let mut project = Project::new();
        project.settings_mut().set_currency("USD").unwrap();
        let path = dir.path().join("example-a.wealth");
        project.save_as(path.clone()).unwrap();
        let value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(value["settings"], serde_json::json!({ "currency": "USD" }));
        assert!(value["income"].get("currency").is_none());
    }

    #[test]
    fn test_csd003_ac_4_2_a_currency_change_is_an_unsaved_change() {
        let dir = TempDir::new().unwrap();
        let (mut project, _) = saved_project(&dir, "example-a.wealth");
        assert!(!project.status().unsaved_changes);
        project.settings_mut().set_currency("GBP").unwrap();
        assert!(project.status().unsaved_changes);
        project.save().unwrap();
        assert!(!project.status().unsaved_changes);
    }

    #[test]
    fn test_csd003_ac_4_3_choosing_the_current_currency_changes_nothing() {
        let dir = TempDir::new().unwrap();
        let (mut project, _) = saved_project(&dir, "example-a.wealth");
        project.settings_mut().set_currency("EUR").unwrap();
        assert!(!project.status().unsaved_changes);
        // Changing and changing back is no change either.
        project.settings_mut().set_currency("USD").unwrap();
        assert!(project.status().unsaved_changes);
        project.settings_mut().set_currency("EUR").unwrap();
        assert!(!project.status().unsaved_changes);
    }

    #[test]
    fn test_csd003_ac_3_1_a_new_project_starts_in_eur() {
        let dir = TempDir::new().unwrap();
        let (mut project, _) = saved_project(&dir, "example-a.wealth");
        project.settings_mut().set_currency("USD").unwrap();
        project.reset();
        assert_eq!(project.settings().currency, Currency::Eur);
    }

    // ---- annual statements in the project file (CSD-001 0.9) ----

    fn add_yearly_job(project: &mut Project, employer: &str) -> JobId {
        project
            .income_mut()
            .create_job(&JobInput {
                employer: employer.to_owned(),
                start_date: Some("01/03/2024".to_owned()),
                pay_period: Some("Yearly".to_owned()),
                ..JobInput::default()
            })
            .unwrap()
            .id
    }

    #[test]
    fn test_csd002_ac_1_1_annual_statements_round_trip() {
        use crate::domain::Year;
        let dir = TempDir::new().unwrap();
        let mut project = Project::new();
        let monthly = add_job(&mut project, "Example Employer");
        add_paycheck(&mut project, monthly, "2026-09", "3000.00");
        let yearly = add_yearly_job(&mut project, "Older Example Employer");
        for (y, gross) in [("2024", "40000.00"), ("2025", "42000.00")] {
            project
                .income_mut()
                .add_statement(
                    yearly,
                    Year::parse(y).unwrap(),
                    Amounts::parse(gross, "", "3000").unwrap(),
                    YearMonth::parse("2026-10").unwrap(),
                )
                .unwrap();
        }
        let path = dir.path().join("example-a.wealth");
        project.save_as(path.clone()).unwrap();

        let mut fresh = Project::new();
        fresh.open(&path).unwrap();
        assert_eq!(fresh.income(), project.income());
        assert_eq!(fresh.income().entry_count(yearly), 2);
        assert_eq!(fresh.income().entry_count(monthly), 1);
        let totals = fresh.income().yearly_totals();
        assert_eq!(totals[0].year, 2026);
        assert_eq!(totals[1].total_gross.to_string(), "45000.00");
        assert!(!fresh.status().unsaved_changes);
    }

    #[test]
    fn test_csd001_the_file_keeps_pay_periods_and_statements_in_their_own_list() {
        use crate::domain::Year;
        let dir = TempDir::new().unwrap();
        let mut project = Project::new();
        add_job(&mut project, "Example Employer");
        let yearly = add_yearly_job(&mut project, "Older Example Employer");
        project
            .income_mut()
            .add_statement(
                yearly,
                Year::parse("2025").unwrap(),
                Amounts::parse("40000.00", "", "").unwrap(),
                YearMonth::parse("2026-10").unwrap(),
            )
            .unwrap();
        let path = dir.path().join("example-a.wealth");
        project.save_as(path.clone()).unwrap();

        let value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(value["format_version"], 1);
        let periods: Vec<&str> = value["income"]["jobs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|j| j["pay_period"].as_str().unwrap())
            .collect();
        assert_eq!(periods.len(), 2);
        assert!(periods.contains(&"Monthly") && periods.contains(&"Yearly"));
        let statement = &value["income"]["statements"][0];
        assert_eq!(statement["year"], "2025");
        assert_eq!(statement["gross_salary"], "40000.00");
        assert_eq!(value["income"]["paychecks"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_csd001_an_older_file_without_pay_periods_or_statements_opens_as_monthly() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("example-a.wealth");
        fs::write(
            &path,
            r#"{"format_version":1,"settings":{"currency":"EUR"},"income":{
                "jobs":[{"id":"671166f4-5827-4794-bcdc-54d7824fb650","employer":"Example Employer",
                    "title":null,"start_date":"2024-03-01","end_date":null}],
                "paychecks":[{"job_id":"671166f4-5827-4794-bcdc-54d7824fb650","month":"2026-09",
                    "gross_salary":"3000.00","holiday_pay":"0.00","bonus":"0.00"}]}}"#,
        )
        .unwrap();
        let mut project = Project::new();
        project.open(&path).unwrap();
        let jobs = project.income().jobs();
        assert_eq!(
            jobs[0].pay_period,
            crate::domain::income::PayPeriod::Monthly
        );
        assert_eq!(project.income().entry_count(jobs[0].id), 1);
    }

    #[test]
    fn a_file_with_an_entry_of_the_wrong_kind_is_invalid() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("example-a.wealth");
        // An annual statement for a job that has no pay period, so it is monthly.
        fs::write(
            &path,
            r#"{"format_version":1,"income":{
                "jobs":[{"id":"671166f4-5827-4794-bcdc-54d7824fb650","employer":"Example Employer",
                    "title":null,"start_date":"2024-03-01","end_date":null}],
                "paychecks":[],
                "statements":[{"job_id":"671166f4-5827-4794-bcdc-54d7824fb650","year":"2025",
                    "gross_salary":"40000.00","holiday_pay":"0.00","bonus":"0.00"}]}}"#,
        )
        .unwrap();
        let mut project = Project::new();
        assert_eq!(
            project.open(&path).unwrap_err().code(),
            "PROJECT_INVALID_FILE"
        );
    }

    // ---- constraints ----

    #[test]
    fn test_csd002_con_3_file_has_version_and_decimal_strings() {
        let dir = TempDir::new().unwrap();
        let mut project = Project::new();
        let job = add_job(&mut project, "Example Employer");
        add_paycheck(&mut project, job, "2026-09", "3000.00");
        let path = dir.path().join("example-a.wealth");
        project.save_as(path.clone()).unwrap();

        let value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(value["format_version"], u64::from(FORMAT_VERSION));
        let paycheck = &value["income"]["paychecks"][0];
        assert_eq!(paycheck["gross_salary"], "3000.00");
        assert_eq!(paycheck["holiday_pay"], "0.00");
        assert_eq!(paycheck["month"], "2026-09");
        assert_eq!(value["income"]["jobs"][0]["start_date"], "2024-03-01");
        assert_eq!(value["income"]["jobs"][0].get("currency"), None);
        assert_eq!(value["settings"]["currency"], "EUR");
    }

    #[test]
    fn test_csd002_con_1_project_file_is_private_to_the_owner() {
        use std::os::unix::fs::PermissionsExt;
        let dir = TempDir::new().unwrap();
        let (_, path) = saved_project(&dir, "example-a.wealth");
        let mode = fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o077, 0, "mode {mode:o}");
    }
}

#[cfg(test)]
mod investments_tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;
    use crate::domain::{AccountInput, Balance, NetDeposits, Note, Period, YearMonth};

    fn account(
        project: &mut Project,
        name: &str,
        period: &str,
    ) -> crate::domain::InvestmentAccountId {
        project
            .investments_mut()
            .create_account(&AccountInput {
                name: name.to_owned(),
                statement_period: Some(period.to_owned()),
            })
            .unwrap()
            .id
    }

    #[test]
    fn test_csd005_ac_19_1_investments_survive_save_and_open_including_notes() {
        let dir = TempDir::new().unwrap();
        let mut project = Project::new();
        let id = account(&mut project, "Example Pension", "Yearly");
        project
            .investments_mut()
            .add_statement(
                id,
                Period::parse("2025").unwrap(),
                Balance::parse("80000.00").unwrap(),
                NetDeposits::parse("-250.00").unwrap(),
                Note::new("Line one\nLine two").unwrap(),
                YearMonth::parse("2026-10").unwrap(),
            )
            .unwrap();
        let path = dir.path().join("example-a.wealth");
        project.save_as(path.clone()).unwrap();

        let mut fresh = Project::new();
        fresh.open(&path).unwrap();
        assert_eq!(fresh.investments(), project.investments());
        let rows = fresh.investments().all_statements();
        assert_eq!(rows[0].1.note.as_str(), "Line one\nLine two");
        // AC-19.3: net deposits survive the round trip.
        assert_eq!(rows[0].1.net_deposits.value().to_string(), "-250.00");
        assert!(!fresh.status().unsaved_changes);

        // The section sits next to income and settings; the version stays 1.
        let value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(value["format_version"], 1);
        assert_eq!(
            value["investments"]["accounts"][0]["name"],
            "Example Pension"
        );
        assert_eq!(value["investments"]["statements"][0]["balance"], "80000.00");
    }

    #[test]
    fn test_csd005_ac_19_2_a_file_without_the_section_opens_without_accounts() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("example-a.wealth");
        fs::write(
            &path,
            r#"{"format_version":1,"income":{"jobs":[],"paychecks":[]}}"#,
        )
        .unwrap();
        let mut project = Project::new();
        project.open(&path).unwrap();
        assert_eq!(project.investments().accounts().len(), 0);
        assert!(!project.status().unsaved_changes);
    }

    #[test]
    fn test_csd005_an_inconsistent_investments_section_is_an_invalid_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("example-a.wealth");
        // A statement of an account that does not exist.
        fs::write(
            &path,
            r#"{"format_version":1,"income":{"jobs":[],"paychecks":[]},
                "investments":{"accounts":[],"statements":[
                {"account_id":"671166f4-5827-4794-bcdc-54d7824fb650","period":"2026-09","balance":"1.00"}]}}"#,
        )
        .unwrap();
        let mut project = Project::new();
        let error = project.open(&path).unwrap_err();
        assert_eq!(error.code(), "PROJECT_INVALID_FILE");
    }

    #[test]
    fn test_csd005_an_investment_change_is_an_unsaved_change() {
        let dir = TempDir::new().unwrap();
        let mut project = Project::new();
        let path = dir.path().join("example-a.wealth");
        project.save_as(path).unwrap();
        assert!(!project.status().unsaved_changes);
        account(&mut project, "Example Brokerage", "Monthly");
        assert!(project.status().unsaved_changes);
    }
}
