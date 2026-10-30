#![allow(
    clippy::needless_pass_by_value,
    reason = "Tauri commands receive their arguments by value"
)]

use rust_decimal::Decimal;
use serde::Serialize;
use tauri::State;

use super::AppState;
use crate::domain::houses::{house_note, parse_period, HouseInput, HouseValue};
use crate::domain::{
    House, HouseId, HouseInsights, HouseValuation, HouseValuationPrefill, HouseYearlyValue, Houses,
    HousesOverview,
};
use crate::error::AppError;
use crate::storage::Project;

/// A valuation with the name of its house and its change, as the valuations
/// table shows it (CSD-015 REQ-21).
#[derive(Serialize)]
pub struct ValuationView {
    house_id: HouseId,
    house_name: String,
    /// `YYYY-MM` for a monthly house, `YYYY` for a yearly one.
    period: String,
    value: Decimal,
    /// The previous valuation's value minus this one; `None` for a first
    /// valuation (REQ-19).
    change: Option<Decimal>,
    note: String,
}

impl ValuationView {
    fn new(houses: &Houses, house: &House, valuation: &HouseValuation) -> Self {
        Self {
            house_id: house.id,
            house_name: house.name.clone(),
            period: valuation.period.to_string(),
            value: valuation.value.value(),
            change: houses.change(valuation),
            note: valuation.note.as_str().to_owned(),
        }
    }
}

/// The houses with their latest values and the total house (REQ-17).
#[tauri::command]
pub fn list_houses(state: State<'_, AppState>) -> Result<HousesOverview, AppError> {
    let project = state.project()?;
    Ok(project.houses().overview())
}

#[tauri::command]
pub fn create_house(state: State<'_, AppState>, input: HouseInput) -> Result<House, AppError> {
    let today = state.today_date();
    let mut project = state.project()?;
    Ok(project.houses_mut().create_house(&input, today)?)
}

#[tauri::command]
pub fn update_house(
    state: State<'_, AppState>,
    house_id: String,
    input: HouseInput,
) -> Result<House, AppError> {
    let today = state.today_date();
    let mut project = state.project()?;
    Ok(project
        .houses_mut()
        .update_house(HouseId::parse(&house_id)?, &input, today)?)
}

#[tauri::command]
pub fn delete_house(state: State<'_, AppState>, house_id: String) -> Result<(), AppError> {
    let mut project = state.project()?;
    project
        .houses_mut()
        .delete_house(HouseId::parse(&house_id)?)?;
    Ok(())
}

/// The valuations of all houses, newest period first (REQ-18).
#[tauri::command]
pub fn list_house_valuations(state: State<'_, AppState>) -> Result<Vec<ValuationView>, AppError> {
    let project = state.project()?;
    let houses = project.houses();
    Ok(houses
        .all_valuations()
        .into_iter()
        .map(|(house, valuation)| ValuationView::new(houses, house, valuation))
        .collect())
}

#[tauri::command]
pub fn add_house_valuation(
    state: State<'_, AppState>,
    house_id: String,
    period: String,
    value: String,
    note: Option<String>,
) -> Result<ValuationView, AppError> {
    let today = state.today();
    let house_id = HouseId::parse(&house_id)?;
    let period = parse_period(&period)?;
    let value = HouseValue::parse(&value)?;
    let note = house_note(note.as_deref().unwrap_or_default())?;
    let mut project = state.project()?;
    let valuation = project
        .houses_mut()
        .add_valuation(house_id, period, value, note, today)?;
    valuation_view(&project, &valuation)
}

#[tauri::command]
pub fn update_house_valuation(
    state: State<'_, AppState>,
    house_id: String,
    period: String,
    new_period: String,
    value: String,
    note: Option<String>,
) -> Result<ValuationView, AppError> {
    let today = state.today();
    let house_id = HouseId::parse(&house_id)?;
    let period = parse_period(&period)?;
    let new_period = parse_period(&new_period)?;
    let value = HouseValue::parse(&value)?;
    let note = house_note(note.as_deref().unwrap_or_default())?;
    let mut project = state.project()?;
    let valuation = project
        .houses_mut()
        .update_valuation(house_id, period, new_period, value, note, today)?;
    valuation_view(&project, &valuation)
}

#[tauri::command]
pub fn delete_house_valuation(
    state: State<'_, AppState>,
    house_id: String,
    period: String,
) -> Result<(), AppError> {
    let house_id = HouseId::parse(&house_id)?;
    let period = parse_period(&period)?;
    let mut project = state.project()?;
    project.houses_mut().delete_valuation(house_id, period)?;
    Ok(())
}

/// Form values for "copy previous month" or "copy previous year" (REQ-15).
#[tauri::command]
pub fn copy_previous_house_valuation(
    state: State<'_, AppState>,
    house_id: String,
) -> Result<HouseValuationPrefill, AppError> {
    let project = state.project()?;
    Ok(project
        .houses()
        .prefill_from_latest(HouseId::parse(&house_id)?)?)
}

/// Yearly values over all houses, newest year first (REQ-20).
#[tauri::command]
pub fn yearly_house_values(state: State<'_, AppState>) -> Result<Vec<HouseYearlyValue>, AppError> {
    let project = state.project()?;
    Ok(project.houses().yearly_values())
}

/// The houses and the yearly series of the Insights chart, computed from the
/// project as it is now, saved or not (CSD-016 REQ-15).
#[tauri::command]
pub fn get_house_insights(state: State<'_, AppState>) -> Result<HouseInsights, AppError> {
    let today = state.today();
    let project = state.project()?;
    Ok(project.houses().insights(today))
}

fn valuation_view(
    project: &Project,
    valuation: &HouseValuation,
) -> Result<ValuationView, AppError> {
    let houses = project.houses();
    let house = houses
        .houses()
        .into_iter()
        .find(|house| house.id == valuation.house_id)
        .ok_or(AppError::Internal)?;
    Ok(ValuationView::new(houses, house, valuation))
}
