#![forbid(unsafe_code)]

mod commands;
mod domain;
mod error;
mod menu;
mod storage;

use tauri::{Emitter, Manager, RunEvent, Runtime, WindowEvent};
use time::UtcOffset;

use commands::{income, project, AppState};

/// Event the frontend listens for to ask "quit with unsaved changes?".
const CLOSE_REQUESTED_EVENT: &str = "close-requested";

fn has_unsaved_changes(state: &AppState) -> bool {
    // If the lock is poisoned, err on the side of asking.
    state
        .project()
        .map_or(true, |project| project.status().unsaved_changes)
}

/// Registers state, commands and the close confirmation on a builder. Shared by
/// the real app and by the IPC tests, which run it on the mock runtime.
fn configure<R: Runtime>(builder: tauri::Builder<R>, utc_offset: UtcOffset) -> tauri::Builder<R> {
    builder
        .manage(AppState::new(utc_offset))
        .on_menu_event(|app, event| {
            // CSD-002 REQ-9: the frontend runs the action behind the item.
            let id = event.id().as_ref();
            if menu::is_project_item(id) {
                let _ = app.emit(menu::MENU_EVENT, id);
            }
        })
        .invoke_handler(tauri::generate_handler![
            income::list_jobs,
            income::create_job,
            income::update_job,
            income::delete_job,
            income::list_paychecks,
            income::add_paycheck,
            income::update_paycheck,
            income::delete_paycheck,
            income::copy_previous_paycheck,
            income::yearly_totals,
            project::project_status,
            project::new_project,
            project::open_project,
            project::save_project,
            project::save_project_as,
            project::quit_app,
        ])
        .on_window_event(|window, event| {
            // CSD-002 REQ-4: closing with unsaved changes needs a confirmation.
            if let WindowEvent::CloseRequested { api, .. } = event {
                if has_unsaved_changes(&window.state::<AppState>()) {
                    api.prevent_close();
                    let _ = window.emit(CLOSE_REQUESTED_EVENT, ());
                }
            }
        })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Must happen before any thread starts: the local offset cannot be read
    // safely afterwards.
    let utc_offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);

    let built = configure(
        tauri::Builder::default()
            .plugin(tauri_plugin_dialog::init())
            // Native menus need the main thread, so the IPC tests skip this.
            .menu(menu::build),
        utc_offset,
    )
    .build(tauri::generate_context!());

    let Ok(app) = built else {
        eprintln!("error while building the application");
        return;
    };

    app.run(|handle, event| {
        // Quit requests that do not come from closing a window (such as the
        // macOS Quit menu item) get the same confirmation. An explicit exit
        // code means the frontend already confirmed.
        if let RunEvent::ExitRequested {
            api, code: None, ..
        } = event
        {
            if has_unsaved_changes(&handle.state::<AppState>()) {
                api.prevent_exit();
                let _ = handle.emit(CLOSE_REQUESTED_EVENT, ());
            }
        }
    });
}

#[cfg(test)]
mod ipc_tests;
