//! Pure business logic: no Tauri, no IO.

pub mod error;
pub mod income;
pub mod money;

pub use error::DomainError;
pub use income::{Income, Job, JobId, JobInput, Paycheck, PaycheckPrefill, YearMonth, YearlyTotal};
pub use money::{Amounts, Currency};
