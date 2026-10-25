use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use thiserror::Error;

use crate::domain::DomainError;
use crate::storage::StorageError;

/// The one error type of the app. It reaches the frontend as a safe message
/// and a code, never as paths, SQL or stack traces.
#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error("Something went wrong. Please try again.")]
    Internal,
}

impl AppError {
    /// Stable machine-readable code for the frontend.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::Domain(error) => error.code(),
            Self::Storage(error) => error.code(),
            Self::Internal => "INTERNAL",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("AppError", 2)?;
        state.serialize_field("code", self.code())?;
        state.serialize_field("message", &self.to_string())?;
        state.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_to_code_and_safe_message() {
        let error = AppError::from(DomainError::JobCurrencyLocked);
        let json = serde_json::to_value(&error).unwrap();
        assert_eq!(json["code"], "JOB_CURRENCY_LOCKED");
        assert_eq!(json["message"], DomainError::JobCurrencyLocked.to_string());
    }

    #[test]
    fn storage_errors_do_not_leak_the_cause() {
        let cause = std::io::Error::other("/Users/someone/secret/path.wealth");
        let error = AppError::from(StorageError::SaveFailed(cause));
        let json = serde_json::to_string(&error).unwrap();
        assert!(!json.contains("/Users"));
        assert!(json.contains("PROJECT_SAVE_FAILED"));
    }
}
