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
/// installed, named `others` (achado do autor, 2026-11-05): the acronym its
/// name already carries (`ABNT2`), or the initials of its words
/// (`Estados Unidos-Internacional` reads `EUI`), whatever order the layouts
/// were added in. Should that be another layout's too, the first word of its
/// name none of theirs has; the language's `short` name when it is alone or
/// nothing sets it apart.
pub fn distinguished(short: &str, own: &str, others: &[String]) -> String {
    if others.is_empty() {
        return short.to_owned();
    }
    let label = |name: &str| acronym(name).unwrap_or_else(|| initials(name));
    let mine = label(own);
    if !mine.is_empty() && others.iter().all(|other| label(other) != mine) {
        return mine;
    }
    let theirs: Vec<Vec<String>> = others
        .iter()
        .map(|other| words(other).map(str::to_lowercase).collect())
        .collect();
    words(own)
        .map(str::to_lowercase)
        .find(|word| theirs.iter().all(|their| !their.contains(word)))
        .map(|word| {
            word.chars()
                .take(LAYOUT_LETTERS)
                .collect::<String>()
                .to_uppercase()
        })
        .unwrap_or_else(|| short.to_owned())
}

/// The words of a layout's name.
fn words(name: &str) -> impl Iterator<Item = &str> {
    name.split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
}

/// The last acronym of a name: a word of capitals (and digits), two to six
/// long, with two capitals at least (`ABNT2`, `US`).
fn acronym(name: &str) -> Option<String> {
    words(name)
        .filter(|word| {
            (2..=6).contains(&word.chars().count())
                && word
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
                && word.chars().filter(char::is_ascii_uppercase).count() >= 2
        })
        .last()
        .map(str::to_owned)
}

/// The initials of the words of a name that are not short joining words
/// (`de`, `do`), upper case, at most five.
fn initials(name: &str) -> String {
    words(name)
        .filter(|word| word.chars().count() > 2)
        .filter_map(|word| word.chars().next())
        .take(LAYOUT_LETTERS)
        .collect::<String>()
        .to_uppercase()
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
        let intl = "Estados Unidos-Internacional".to_owned();
        let us = "Estados Unidos".to_owned();
        // The acronym the name carries, whatever the other one is.
        assert_eq!(
            distinguished("PTB", &abnt, std::slice::from_ref(&abnt2)),
            "ABNT"
        );
        assert_eq!(
            distinguished("PTB", &abnt2, std::slice::from_ref(&abnt)),
            "ABNT2"
        );
        assert_eq!(
            distinguished("PTB", &abnt2, std::slice::from_ref(&intl)),
            "ABNT2"
        );
        // No acronym: the initials.
        assert_eq!(
            distinguished("PTB", &intl, std::slice::from_ref(&abnt2)),
            "EUI"
        );
        assert_eq!(distinguished("ENU", &us, std::slice::from_ref(&intl)), "EU");
        // Alone, the language's short name.
        assert_eq!(distinguished("PTB", &abnt, &[]), "PTB");
        // Initials that differ by a word.
        let dvorak = "Estados Unidos-Dvorak".to_owned();
        let ext = "Estados Unidos-Dvorak Extra".to_owned();
        assert_eq!(
            distinguished("ENU", &ext, std::slice::from_ref(&dvorak)),
            "EUDE"
        );
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
