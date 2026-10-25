//! The native macOS menu bar. The project actions live in the File menu
//! (CSD-002 REQ-9) and Settings… in the app menu (CSD-003 REQ-2); everything
//! else is Tauri's standard menu, which keeps copy, paste and undo working in
//! text fields.

use tauri::menu::{IsMenuItem, Menu, MenuItemBuilder, MenuItemKind, PredefinedMenuItem};
use tauri::{AppHandle, Runtime};

/// Event the frontend listens for; the payload is one of the item ids below.
pub const MENU_EVENT: &str = "menu-action";

/// The project items in menu order: id, label and shortcut.
pub const ITEMS: [(&str, &str, &str); 4] = [
    ("project-new", "New", "CmdOrCtrl+N"),
    ("project-open", "Open…", "CmdOrCtrl+O"),
    ("project-save", "Save", "CmdOrCtrl+S"),
    ("project-save-as", "Save As…", "CmdOrCtrl+Shift+S"),
];

/// The Settings… item of the app menu: id, label and shortcut (⌘,).
pub const SETTINGS: (&str, &str, &str) = ("open-settings", "Settings…", "CmdOrCtrl+,");

/// Whether `id` is an item whose action the frontend runs.
#[must_use]
pub fn is_forwarded_item(id: &str) -> bool {
    id == SETTINGS.0 || ITEMS.iter().any(|(item, _, _)| *item == id)
}

const FILE_MENU: &str = "File";

/// In the default macOS app menu: About, separator, then the rest. Settings…
/// goes right below About and that separator, as in other macOS apps.
const SETTINGS_POSITION: usize = 2;

/// Builds Tauri's default menu with the project items at the top of File and
/// Settings… in the app menu.
///
/// # Errors
/// Returns an error if the menu or one of its items cannot be created, or if
/// the default menu has no File or app menu.
pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let menu = Menu::default(app)?;
    let mut items = Vec::new();
    for (id, label, accelerator) in ITEMS {
        items.push(
            MenuItemBuilder::with_id(id, label)
                .accelerator(accelerator)
                .build(app)?,
        );
    }
    let separator = PredefinedMenuItem::separator(app)?;

    let file = menu
        .items()?
        .into_iter()
        .find_map(|item| match item {
            MenuItemKind::Submenu(submenu) if submenu.text().is_ok_and(|t| t == FILE_MENU) => {
                Some(submenu)
            }
            _ => None,
        })
        .ok_or(tauri::Error::InvalidWindowHandle)?;
    let mut entries: Vec<&dyn IsMenuItem<R>> = items.iter().map(|i| i as _).collect();
    entries.push(&separator);
    file.insert_items(&entries, 0)?;

    // The app menu is the first one, named after the program.
    let app_menu = menu
        .items()?
        .into_iter()
        .find_map(|item| match item {
            MenuItemKind::Submenu(submenu) => Some(submenu),
            _ => None,
        })
        .ok_or(tauri::Error::InvalidWindowHandle)?;
    let (id, label, accelerator) = SETTINGS;
    let settings = MenuItemBuilder::with_id(id, label)
        .accelerator(accelerator)
        .build(app)?;
    let after_settings = PredefinedMenuItem::separator(app)?;
    app_menu.insert_items(&[&settings, &after_settings], SETTINGS_POSITION)?;
    Ok(menu)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Native menus can only be created on the main thread, which unit tests do
    // not run on. The table the menu is built from is tested here; the built
    // menu is part of the manual checklist (CSD-002 section 7).
    #[test]
    fn test_csd002_ac_9_1_menu_table_matches_the_spec() {
        let spec = [
            ("project-new", "New", "CmdOrCtrl+N"),
            ("project-open", "Open…", "CmdOrCtrl+O"),
            ("project-save", "Save", "CmdOrCtrl+S"),
            ("project-save-as", "Save As…", "CmdOrCtrl+Shift+S"),
        ];
        assert_eq!(ITEMS, spec);
    }

    #[test]
    fn test_csd003_ac_2_1_settings_item_is_in_the_app_menu_with_the_shortcut() {
        assert_eq!(SETTINGS, ("open-settings", "Settings…", "CmdOrCtrl+,"));
        // Right below About and its separator.
        assert_eq!(SETTINGS_POSITION, 2);
    }

    #[test]
    fn only_known_items_are_forwarded_to_the_frontend() {
        for (id, _, _) in ITEMS {
            assert!(is_forwarded_item(id));
        }
        assert!(is_forwarded_item("open-settings"));
        assert!(!is_forwarded_item("quit"));
        assert!(!is_forwarded_item(""));
    }
}
