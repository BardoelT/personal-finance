//! The fixed categories of an expense (CSD-013 REQ-3, CSD-014 section 3).

use serde::{Deserialize, Serialize};

use crate::domain::error::DomainError;

/// What an expense was for. The list is fixed and its order is the order of
/// the segments and the legend of the Insights chart (CSD-014 REQ-4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Category {
    Home,
    Vehicle,
    Holiday,
    Sport,
    Study,
    Appliances,
    Hobby,
    Other,
}

impl Category {
    /// Every category, in the fixed order.
    pub const ALL: [Self; 8] = [
        Self::Home,
        Self::Vehicle,
        Self::Holiday,
        Self::Sport,
        Self::Study,
        Self::Appliances,
        Self::Hobby,
        Self::Other,
    ];

    /// Parses the exact name of a category, so "home" is not accepted.
    ///
    /// # Errors
    /// Returns `ExpenseCategoryUnsupported`.
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        Self::ALL
            .into_iter()
            .find(|category| category.name() == text.trim())
            .ok_or(DomainError::ExpenseCategoryUnsupported)
    }

    /// The name shown to the user.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Home => "Home",
            Self::Vehicle => "Vehicle",
            Self::Holiday => "Holiday",
            Self::Sport => "Sport",
            Self::Study => "Study",
            Self::Appliances => "Appliances",
            Self::Hobby => "Hobby",
            Self::Other => "Other",
        }
    }

    /// The colour slot 0 to 7 of the chart, fixed by the place in the list
    /// (CSD-014 REQ-6).
    #[must_use]
    pub fn colour_slot(self) -> u8 {
        match self {
            Self::Home => 0,
            Self::Vehicle => 1,
            Self::Holiday => 2,
            Self::Sport => 3,
            Self::Study => 4,
            Self::Appliances => 5,
            Self::Hobby => 6,
            Self::Other => 7,
        }
    }
}
