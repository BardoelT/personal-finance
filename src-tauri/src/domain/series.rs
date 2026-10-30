//! What the named series share: investment accounts, bank accounts, debts,
//! houses and expenses all have a name of at most 100 characters that is unique
//! ignoring case and is listed alphabetically; all but investments and expenses
//! also have a colour slot for the Insights chart.

use std::fmt::Display;

/// The longest name, in characters.
pub(crate) const MAX_NAME_CHARS: usize = 100;

/// How many colours the Insights charts have.
pub(crate) const COLOUR_SLOTS: u8 = 8;

/// The lowest colour slot used by the fewest series.
pub(crate) fn next_colour_slot(used: impl IntoIterator<Item = u8>) -> u8 {
    let used: Vec<u8> = used.into_iter().collect();
    (0..COLOUR_SLOTS)
        .min_by_key(|slot| {
            let count: usize = used.iter().map(|u| usize::from(u == slot)).sum();
            (count, *slot)
        })
        .unwrap_or_default()
}

/// Whether a series other than `except` already has `name`, ignoring case.
/// `existing` pairs each series' id with its name.
pub(crate) fn name_taken<'a, Id: Copy + PartialEq>(
    existing: impl IntoIterator<Item = (Id, &'a str)>,
    name: &str,
    except: Option<Id>,
) -> bool {
    let wanted = name.to_lowercase();
    existing
        .into_iter()
        .any(|(id, other)| except.is_none_or(|e| e != id) && other.to_lowercase() == wanted)
}

/// Alphabetical order ignoring case, with the id as a stable tie-break.
pub(crate) fn name_order(name: &str, id: &impl Display) -> (String, String) {
    (name.to_lowercase(), id.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_colour_slot_takes_the_lowest_slot_used_least() {
        assert_eq!(next_colour_slot([]), 0);
        assert_eq!(next_colour_slot([0, 1]), 2);
        assert_eq!(next_colour_slot(0..COLOUR_SLOTS), 0);
        assert_eq!(next_colour_slot((0..COLOUR_SLOTS).chain([0, 2])), 1);
    }

    #[test]
    fn name_taken_ignores_case_and_the_excepted_series() {
        let existing = [(1, "Savings"), (2, "Joint")];
        assert!(name_taken(existing, "savings", None));
        assert!(!name_taken(existing, "savings", Some(1)));
        assert!(name_taken(existing, "SAVINGS", Some(2)));
        assert!(!name_taken(existing, "Other", None));
    }

    #[test]
    fn name_order_sorts_ignoring_case_then_by_id() {
        assert!(name_order("apple", &"b") < name_order("Banana", &"a"));
        assert!(name_order("same", &"a") < name_order("Same", &"b"));
    }
}
