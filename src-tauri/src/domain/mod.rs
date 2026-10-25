//! Pure business logic: no Tauri, no IO.

pub mod error;
pub mod income;
pub mod money;
pub mod project_data;
pub mod settings;

pub use error::DomainError;
pub use income::{Income, Job, JobId, JobInput, Paycheck, PaycheckPrefill, YearMonth, YearlyTotal};
pub use money::{Amounts, Currency};
pub use project_data::ProjectData;
pub use settings::Settings;
