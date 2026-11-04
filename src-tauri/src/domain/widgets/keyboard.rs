//! The Layout do teclado (F9 of estudo 0002): an item of the dock that shows
//! the keyboard layout of the window in front, as the taskbar's language
//! button does, and goes to the next one on a click. It has nothing to keep,
//! so the config is empty and stays `{}`.

use serde::{Deserialize, Serialize};

use super::WidgetError;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct KeyboardConfig {}

impl KeyboardConfig {
    /// Always valid: there is nothing in it.
    pub fn validate(&self) -> Result<(), WidgetError> {
        Ok(())
    }
}

/// The layout after `current` among the installed `layouts`, going round;
/// `None` with fewer than two (nothing to go to). A `current` that is not
/// in the list (a layout the window has that was removed) goes to the
/// first.
pub fn next<T: PartialEq + Copy>(layouts: &[T], current: T) -> Option<T> {
    if layouts.len() < 2 {
        return None;
    }
    let at = layouts.iter().position(|layout| *layout == current);
    Some(match at {
        Some(at) => layouts[(at + 1) % layouts.len()],
        None => layouts[0],
    })
}

/// What the tile shows for a layout: the language's short name as Windows
/// gives it (`PTB`, `ENU`), upper case, at most three letters; `?` without
/// one.
pub fn short_name(abbreviation: &str) -> String {
    let letters: String = abbreviation
        .trim()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .take(3)
        .collect::<String>()
        .to_uppercase();
    if letters.is_empty() {
        "?".to_owned()
    } else {
        letters
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_config_is_empty_and_reads_from_anything_older() {
        assert_eq!(
            serde_json::to_string(&KeyboardConfig::default()).unwrap(),
            "{}"
        );
        let read: KeyboardConfig = serde_json::from_str(r#"{"width": 3}"#).unwrap();
        assert_eq!(read, KeyboardConfig::default());
        assert_eq!(read.validate(), Ok(()));
    }

    #[test]
    fn the_next_layout_goes_round() {
        assert_eq!(next(&[1, 2, 3], 1), Some(2));
        assert_eq!(next(&[1, 2, 3], 3), Some(1));
        assert_eq!(next(&[1, 2, 3], 9), Some(1));
        assert_eq!(next(&[1], 1), None);
        assert_eq!(next::<u32>(&[], 1), None);
    }

    #[test]
    fn the_short_name_is_three_letters_in_upper_case() {
        assert_eq!(short_name("ptb"), "PTB");
        assert_eq!(short_name(" ENU "), "ENU");
        assert_eq!(short_name("ENGB"), "ENG");
        assert_eq!(short_name(""), "?");
    }
}
