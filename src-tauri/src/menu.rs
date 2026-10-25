//! The native macOS menu bar. The project actions live in the File menu
//! (CSD-002 REQ-9); everything else is Tauri's standard menu, which keeps
//! copy, paste and undo working in text fields.

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

/// Whether `id` is one of the project items.
#[must_use]
pub fn is_project_item(id: &str) -> bool {
    ITEMS.iter().any(|(item, _, _)| *item == id)
}

const FILE_MENU: &str = "File";

/// Builds Tauri's default menu with the project items at the top of File.
///
/// # Errors
/// Returns an error if the menu or one of its items cannot be created, or if
/// the default menu has no File menu.
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
    fn only_project_items_are_forwarded_to_the_frontend() {
        for (id, _, _) in ITEMS {
            assert!(is_project_item(id));
        }
        assert!(!is_project_item("quit"));
        assert!(!is_project_item(""));
    }
}
