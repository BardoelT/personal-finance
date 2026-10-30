#![forbid(unsafe_code)]

mod commands;
mod domain;
mod error;
mod menu;
mod storage;

use tauri::{Emitter, Manager, RunEvent, Runtime, WindowEvent};
use time::UtcOffset;

use commands::{
    bank_accounts, dashboard, debts, expenses, houses, income, investments, net_worth, preferences,
    project, ratios, settings, AppState,
};

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
            if menu::is_forwarded_item(id) {
                let _ = app.emit(menu::MENU_EVENT, id);
            }
        })
        .invoke_handler(tauri::generate_handler![
            income::list_jobs,
            income::create_job,
            income::update_job,
            income::delete_job,
            income::list_entries,
            income::add_paycheck,
            income::add_statement,
            income::update_paycheck,
            income::update_statement,
            income::delete_paycheck,
            income::delete_statement,
            income::copy_previous_entry,
            income::yearly_totals,
            income::get_income_insights,
            investments::list_investment_accounts,
            investments::create_investment_account,
            investments::update_investment_account,
            investments::delete_investment_account,
            investments::list_investment_statements,
            investments::add_investment_statement,
            investments::update_investment_statement,
            investments::delete_investment_statement,
            investments::copy_previous_investment_statement,
            investments::calculate_investment_statement,
            investments::yearly_investment_totals,
            investments::get_investment_insights,
            bank_accounts::list_bank_accounts,
            bank_accounts::create_bank_account,
            bank_accounts::update_bank_account,
            bank_accounts::delete_bank_account,
            bank_accounts::list_bank_entries,
            bank_accounts::add_bank_entry,
            bank_accounts::update_bank_entry,
            bank_accounts::delete_bank_entry,
            bank_accounts::copy_previous_bank_entry,
            bank_accounts::yearly_bank_totals,
            bank_accounts::get_bank_insights,
            debts::list_debts,
            debts::debt_open_balance,
            debts::create_debt,
            debts::update_debt,
            debts::delete_debt,
            debts::list_debt_statements,
            debts::add_debt_statement,
            debts::update_debt_statement,
            debts::delete_debt_statement,
            debts::copy_previous_debt_statement,
            debts::yearly_debt_totals,
            debts::get_debt_insights,
            houses::list_houses,
            houses::list_open_debts,
            houses::create_house,
            houses::update_house,
            houses::delete_house,
            houses::list_house_valuations,
            houses::add_house_valuation,
            houses::update_house_valuation,
            houses::delete_house_valuation,
            houses::copy_previous_house_valuation,
            houses::yearly_house_values,
            houses::get_house_insights,
            expenses::list_expenses,
            expenses::add_expense,
            expenses::update_expense,
            expenses::delete_expense,
            expenses::copy_previous_expense,
            expenses::yearly_expense_totals,
            expenses::get_expense_insights,
            net_worth::get_net_worth_insights,
            ratios::get_ratios,
            dashboard::get_dashboard,
            settings::get_settings,
            settings::set_currency,
            preferences::get_preferences,
            preferences::set_theme,
            project::project_status,
            project::take_startup_notice,
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
            .setup(|app| {
                // CSD-002 REQ-8: reopen the last used project. Without a config
                // folder the app simply starts empty.
                let config_dir = app.path().app_config_dir().ok();
                if let Some(dir) = &config_dir {
                    if let Ok(mut project) = app.state::<AppState>().project() {
                        project.restore(storage::LastProject::in_dir(dir));
                    }
                }

                // CSD-008: the saved theme also goes to the native window, so
                // it is right before the first page is shown (REQ-6, REQ-7).
                let saved = config_dir.as_deref().map_or_else(
                    storage::Preferences::unavailable,
                    storage::Preferences::in_dir,
                );
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.set_theme(preferences::window_theme(saved.load_theme()));
                }
                app.manage(saved);
                Ok(())
            })
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
