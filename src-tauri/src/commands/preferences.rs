#![allow(
    clippy::needless_pass_by_value,
    reason = "Tauri commands receive their arguments by value"
)]

use serde::Serialize;
use tauri::{Runtime, State, Window};

use crate::domain::Theme;
use crate::error::AppError;
use crate::storage::Preferences;

/// The application preferences as the frontend needs them (CSD-008).
#[derive(Serialize)]
pub struct PreferencesView {
    theme: Theme,
}

/// The native window theme for `theme`: `None` leaves the window to follow
/// macOS (REQ-7).
#[must_use]
pub fn window_theme(theme: Theme) -> Option<tauri::Theme> {
    match theme {
        Theme::System => None,
        Theme::Light => Some(tauri::Theme::Light),
        Theme::Dark => Some(tauri::Theme::Dark),
    }
}

/// The saved theme, System when there is none or it cannot be read (REQ-2, REQ-8).
#[tauri::command]
pub fn get_preferences(preferences: State<'_, Preferences>) -> PreferencesView {
    PreferencesView {
        theme: preferences.load_theme(),
    }
}

/// Saves the theme and gives the native window the same theme. Saving comes
/// first, so a failed save changes nothing (REQ-10).
#[tauri::command]
pub fn set_theme<R: Runtime>(
    window: Window<R>,
    preferences: State<'_, Preferences>,
    theme: String,
) -> Result<PreferencesView, AppError> {
    let theme = Theme::parse(&theme)?;
    preferences.store_theme(theme)?;
    // The saved theme is what counts; a window that refuses the theme keeps
    // following macOS until the next start.
    let _ = window.set_theme(window_theme(theme));
    Ok(PreferencesView { theme })
}
