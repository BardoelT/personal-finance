#![allow(
    clippy::needless_pass_by_value,
    reason = "Tauri commands receive their arguments by value"
)]

use rust_decimal::Decimal;
use serde::Serialize;
use tauri::State;

use super::AppState;
use crate::domain::assets::{asset_note, parse_period, AssetInput, AssetValue};
use crate::domain::{
    Asset, AssetId, AssetInsights, AssetValuation, AssetValuationPrefill, AssetYearlyValue, Assets,
    AssetsOverview,
};
use crate::error::AppError;
use crate::storage::Project;

/// A valuation with the name of its asset and its change, as the valuations
/// table shows it (CSD-015 REQ-21).
#[derive(Serialize)]
pub struct ValuationView {
    asset_id: AssetId,
    asset_name: String,
    /// `YYYY-MM` for a monthly asset, `YYYY` for a yearly one.
    period: String,
    value: Decimal,
    /// The previous valuation's value minus this one; `None` for a first
    /// valuation (REQ-19).
    change: Option<Decimal>,
    note: String,
}

impl ValuationView {
    fn new(assets: &Assets, asset: &Asset, valuation: &AssetValuation) -> Self {
        Self {
            asset_id: asset.id,
            asset_name: asset.name.clone(),
            period: valuation.period.to_string(),
            value: valuation.value.value(),
            change: assets.change(valuation),
            note: valuation.note.as_str().to_owned(),
        }
    }
}

/// The assets with their latest values and the total asset (REQ-17).
#[tauri::command]
pub fn list_assets(state: State<'_, AppState>) -> Result<AssetsOverview, AppError> {
    let project = state.project()?;
    Ok(project.assets().overview())
}

#[tauri::command]
pub fn create_asset(state: State<'_, AppState>, input: AssetInput) -> Result<Asset, AppError> {
    let mut project = state.project()?;
    Ok(project.assets_mut().create_asset(&input)?)
}

#[tauri::command]
pub fn update_asset(
    state: State<'_, AppState>,
    asset_id: String,
    input: AssetInput,
) -> Result<Asset, AppError> {
    let mut project = state.project()?;
    Ok(project
        .assets_mut()
        .update_asset(AssetId::parse(&asset_id)?, &input)?)
}

#[tauri::command]
pub fn delete_asset(state: State<'_, AppState>, asset_id: String) -> Result<(), AppError> {
    let mut project = state.project()?;
    project
        .assets_mut()
        .delete_asset(AssetId::parse(&asset_id)?)?;
    Ok(())
}

/// The valuations of all assets, newest period first (REQ-18).
#[tauri::command]
pub fn list_asset_valuations(state: State<'_, AppState>) -> Result<Vec<ValuationView>, AppError> {
    let project = state.project()?;
    let assets = project.assets();
    Ok(assets
        .all_valuations()
        .into_iter()
        .map(|(asset, valuation)| ValuationView::new(assets, asset, valuation))
        .collect())
}

#[tauri::command]
pub fn add_asset_valuation(
    state: State<'_, AppState>,
    asset_id: String,
    period: String,
    value: String,
    note: Option<String>,
) -> Result<ValuationView, AppError> {
    let today = state.today();
    let asset_id = AssetId::parse(&asset_id)?;
    let period = parse_period(&period)?;
    let value = AssetValue::parse(&value)?;
    let note = asset_note(note.as_deref().unwrap_or_default())?;
    let mut project = state.project()?;
    let valuation = project
        .assets_mut()
        .add_valuation(asset_id, period, value, note, today)?;
    valuation_view(&project, &valuation)
}

#[tauri::command]
pub fn update_asset_valuation(
    state: State<'_, AppState>,
    asset_id: String,
    period: String,
    new_period: String,
    value: String,
    note: Option<String>,
) -> Result<ValuationView, AppError> {
    let today = state.today();
    let asset_id = AssetId::parse(&asset_id)?;
    let period = parse_period(&period)?;
    let new_period = parse_period(&new_period)?;
    let value = AssetValue::parse(&value)?;
    let note = asset_note(note.as_deref().unwrap_or_default())?;
    let mut project = state.project()?;
    let valuation = project
        .assets_mut()
        .update_valuation(asset_id, period, new_period, value, note, today)?;
    valuation_view(&project, &valuation)
}

#[tauri::command]
pub fn delete_asset_valuation(
    state: State<'_, AppState>,
    asset_id: String,
    period: String,
) -> Result<(), AppError> {
    let asset_id = AssetId::parse(&asset_id)?;
    let period = parse_period(&period)?;
    let mut project = state.project()?;
    project.assets_mut().delete_valuation(asset_id, period)?;
    Ok(())
}

/// Form values for "copy previous month" or "copy previous year" (REQ-15).
#[tauri::command]
pub fn copy_previous_asset_valuation(
    state: State<'_, AppState>,
    asset_id: String,
) -> Result<AssetValuationPrefill, AppError> {
    let project = state.project()?;
    Ok(project
        .assets()
        .prefill_from_latest(AssetId::parse(&asset_id)?)?)
}

/// Yearly values over all assets, newest year first (REQ-20).
#[tauri::command]
pub fn yearly_asset_values(state: State<'_, AppState>) -> Result<Vec<AssetYearlyValue>, AppError> {
    let project = state.project()?;
    Ok(project.assets().yearly_values())
}

/// The assets and the yearly series of the Insights chart, computed from the
/// project as it is now, saved or not (CSD-016 REQ-15).
#[tauri::command]
pub fn get_asset_insights(state: State<'_, AppState>) -> Result<AssetInsights, AppError> {
    let today = state.today();
    let project = state.project()?;
    Ok(project.assets().insights(today))
}

fn valuation_view(
    project: &Project,
    valuation: &AssetValuation,
) -> Result<ValuationView, AppError> {
    let assets = project.assets();
    let asset = assets
        .assets()
        .into_iter()
        .find(|asset| asset.id == valuation.asset_id)
        .ok_or(AppError::Internal)?;
    Ok(ValuationView::new(assets, asset, valuation))
}
