use super::income::Income;
use super::settings::Settings;

/// Everything a project file holds: the income data and the project settings.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProjectData {
    pub income: Income,
    pub settings: Settings,
}
