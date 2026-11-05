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

/// The longest the tile's text gets when it names the layout (`ABNT2`).
const LAYOUT_LETTERS: usize = 5;

/// The tile's text for a layout named `own` (as Windows lists it:
/// `Português (Brasil ABNT2)`) when other layouts of its language are
/// installed, named `others`: the first word of its name that none of
/// theirs has, upper case, at most five letters, so two Portuguese
/// keyboards read `ABNT` and `ABNT2`, whatever order they were added in
/// (achado do autor, 2026-11-05). The language's `short` name when it is
/// alone, or when no word sets it apart.
pub fn distinguished(short: &str, own: &str, others: &[String]) -> String {
    if others.is_empty() {
        return short.to_owned();
    }
    let words = |name: &str| -> Vec<String> {
        name.split(|c: char| !c.is_alphanumeric())
            .filter(|word| !word.is_empty())
            .map(str::to_lowercase)
            .collect()
    };
    let theirs: Vec<Vec<String>> = others.iter().map(|other| words(other)).collect();
    words(own)
        .into_iter()
        .find(|word| theirs.iter().all(|their| !their.contains(word)))
        .map(|word| {
            word.chars()
                .take(LAYOUT_LETTERS)
                .collect::<String>()
                .to_uppercase()
        })
        .unwrap_or_else(|| short.to_owned())
}

/// The key of a layout under `Keyboard Layouts` in the registry (its KLID)
/// from the high word of its handle: the layout's own language
/// (`0x0416` gives `00000416`), or `None` for a variant (`0xF0nn`), whose
/// key is found by its `Layout Id`.
pub fn klid_of(device: u16) -> Option<String> {
    (device & 0xF000 != 0xF000).then(|| format!("0000{device:04X}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_layouts_of_one_language_are_told_apart_by_their_names() {
        let abnt = "Português (Brasil ABNT)".to_owned();
        let abnt2 = "Português (Brasil ABNT2)".to_owned();
        assert_eq!(
            distinguished("PTB", &abnt, std::slice::from_ref(&abnt2)),
            "ABNT"
        );
        assert_eq!(
            distinguished("PTB", &abnt2, std::slice::from_ref(&abnt)),
            "ABNT2"
        );
        // Alone, the language's short name.
        assert_eq!(distinguished("PTB", &abnt, &[]), "PTB");
        // No word of its own: the short name; a long word, cut.
        let us = "Estados Unidos".to_owned();
        let intl = "Estados Unidos-Internacional".to_owned();
        assert_eq!(
            distinguished("ENU", &us, std::slice::from_ref(&intl)),
            "ENU"
        );
        assert_eq!(distinguished("ENU", &intl, &[us]), "INTER");
    }

    #[test]
    fn the_registry_key_of_a_layout_comes_from_its_handle() {
        assert_eq!(klid_of(0x0416).as_deref(), Some("00000416"));
        assert_eq!(klid_of(0x0409).as_deref(), Some("00000409"));
        assert_eq!(klid_of(0xF0B0), None);
    }

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
