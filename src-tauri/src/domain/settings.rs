use serde::{Deserialize, Serialize};

use super::error::DomainError;
use super::money::Currency;

/// Settings of the whole project (CSD-003). They are saved in the project file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    /// The one currency of every amount in the project; a new project uses EUR.
    #[serde(default)]
    pub currency: Currency,
}

impl Settings {
    /// Sets the currency from text such as `"USD"` without converting any
    /// amount (REQ-4, CON-1).
    ///
    /// # Errors
    /// Returns `ProjectCurrencyUnsupported` for anything but EUR, USD or GBP,
    /// and leaves the currency as it was.
    pub fn set_currency(&mut self, text: &str) -> Result<(), DomainError> {
        self.currency = Currency::parse(text).ok_or(DomainError::ProjectCurrencyUnsupported)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csd003_ac_3_1_a_new_project_uses_eur() {
        assert_eq!(Settings::default().currency, Currency::Eur);
    }

    #[test]
    fn test_csd003_ac_4_1_setting_the_currency_changes_only_the_currency() {
        let mut settings = Settings::default();
        settings.set_currency("USD").unwrap();
        assert_eq!(settings.currency, Currency::Usd);
        settings.set_currency(" gbp ").unwrap();
        assert_eq!(settings.currency, Currency::Gbp);
    }

    #[test]
    fn test_csd003_ac_6_1_unsupported_currencies_are_rejected_and_keep_the_old_one() {
        for text in ["CHF", "euro", "", "  ", "EU R", "US$"] {
            let mut settings = Settings::default();
            settings.set_currency("USD").unwrap();
            let error = settings.set_currency(text).unwrap_err();
            assert_eq!(error.code(), "PROJECT_CURRENCY_UNSUPPORTED", "{text:?}");
            assert_eq!(settings.currency, Currency::Usd, "{text:?}");
        }
    }
}
