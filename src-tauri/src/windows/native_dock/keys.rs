//! The dock by the keyboard (M14): what each key does to the item chosen.
//! Pure, so the rules are tested without a window.
//!
//! The items are the ones assistive technology sees, in their order on the
//! screen: the workspace dots, the dock's items, then the strip's icons.

/// The virtual-key codes the dock answers.
pub const VK_RETURN: u16 = 0x0D;
pub const VK_ESCAPE: u16 = 0x1B;
pub const VK_SPACE: u16 = 0x20;
pub const VK_END: u16 = 0x23;
pub const VK_HOME: u16 = 0x24;
pub const VK_LEFT: u16 = 0x25;
pub const VK_UP: u16 = 0x26;
pub const VK_RIGHT: u16 = 0x27;
pub const VK_DOWN: u16 = 0x28;
/// The menu key, beside the right Ctrl.
pub const VK_APPS: u16 = 0x5D;
pub const VK_F10: u16 = 0x79;

/// What a key asks of the dock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    /// The item at this index is chosen now.
    Select(usize),
    /// The item at this index is opened, as a click does.
    Press(usize),
    /// Its menu opens, as the right button's.
    Menu(usize),
    /// The keyboard goes back to the program from before.
    Leave,
    /// Nothing: the key is not the dock's, or there is nowhere to go.
    Nothing,
}

/// `key` as the dock's list reads it on `edge`: the right's column reads
/// from the bottom up (spec 0028 Q2), so there ↑ goes on along the list and
/// ↓ back, as they look; every other key as it is.
pub fn along(key: u16, edge: crate::domain::settings::DockEdge) -> u16 {
    use crate::domain::settings::DockEdge;
    match (edge, key) {
        (DockEdge::Right, VK_UP) => VK_DOWN,
        (DockEdge::Right, VK_DOWN) => VK_UP,
        _ => key,
    }
}

/// What `key` (a virtual-key code), with Shift down or not, does among `len`
/// items with `current` chosen. The arrows go one item along (left and up
/// back, right and down on), without wrapping round at the ends; Home and
/// End go to the ends; with nothing chosen, the arrows start at an end.
pub fn key_action(len: usize, current: Option<usize>, key: u16, shift: bool) -> KeyAction {
    if key == VK_ESCAPE {
        return KeyAction::Leave;
    }
    let Some(last) = len.checked_sub(1) else {
        return KeyAction::Nothing;
    };
    // An index from a list since shortened counts as the last item.
    let current = current.map(|index| index.min(last));
    let select = |index: usize| {
        if current == Some(index) {
            KeyAction::Nothing
        } else {
            KeyAction::Select(index)
        }
    };
    match key {
        VK_LEFT | VK_UP => match current {
            Some(0) => KeyAction::Nothing,
            Some(index) => select(index - 1),
            None => select(last),
        },
        VK_RIGHT | VK_DOWN => match current {
            Some(index) if index == last => KeyAction::Nothing,
            Some(index) => select(index + 1),
            None => select(0),
        },
        VK_HOME => select(0),
        VK_END => select(last),
        VK_RETURN | VK_SPACE => current.map_or(KeyAction::Nothing, KeyAction::Press),
        VK_APPS => current.map_or(KeyAction::Nothing, KeyAction::Menu),
        VK_F10 if shift => current.map_or(KeyAction::Nothing, KeyAction::Menu),
        _ => KeyAction::Nothing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use KeyAction::{Leave, Menu, Nothing, Press, Select};

    #[test]
    fn the_arrows_go_one_item_along_and_stop_at_the_ends() {
        for back in [VK_LEFT, VK_UP] {
            assert_eq!(key_action(4, Some(2), back, false), Select(1));
            assert_eq!(key_action(4, Some(0), back, false), Nothing);
        }
        for on in [VK_RIGHT, VK_DOWN] {
            assert_eq!(key_action(4, Some(2), on, false), Select(3));
            assert_eq!(key_action(4, Some(3), on, false), Nothing);
        }
    }

    #[test]
    fn with_nothing_chosen_the_arrows_start_at_an_end() {
        assert_eq!(key_action(4, None, VK_RIGHT, false), Select(0));
        assert_eq!(key_action(4, None, VK_DOWN, false), Select(0));
        assert_eq!(key_action(4, None, VK_LEFT, false), Select(3));
        assert_eq!(key_action(4, None, VK_UP, false), Select(3));
    }

    #[test]
    fn home_and_end_go_to_the_ends() {
        assert_eq!(key_action(4, Some(2), VK_HOME, false), Select(0));
        assert_eq!(key_action(4, Some(1), VK_END, false), Select(3));
        assert_eq!(key_action(4, None, VK_HOME, false), Select(0));
        assert_eq!(key_action(4, None, VK_END, false), Select(3));
        // Already there.
        assert_eq!(key_action(4, Some(0), VK_HOME, false), Nothing);
        assert_eq!(key_action(4, Some(3), VK_END, false), Nothing);
    }

    #[test]
    fn enter_and_space_open_the_item_chosen() {
        for key in [VK_RETURN, VK_SPACE] {
            assert_eq!(key_action(4, Some(2), key, false), Press(2));
            assert_eq!(key_action(4, None, key, false), Nothing);
        }
    }

    #[test]
    fn shift_f10_and_the_menu_key_open_its_menu() {
        assert_eq!(key_action(4, Some(1), VK_F10, true), Menu(1));
        assert_eq!(key_action(4, Some(1), VK_APPS, false), Menu(1));
        assert_eq!(key_action(4, Some(1), VK_APPS, true), Menu(1));
        // F10 alone is Windows' (the menu bar), not the item's menu.
        assert_eq!(key_action(4, Some(1), VK_F10, false), Nothing);
        assert_eq!(key_action(4, None, VK_F10, true), Nothing);
        assert_eq!(key_action(4, None, VK_APPS, false), Nothing);
    }

    #[test]
    fn escape_leaves_with_or_without_items() {
        assert_eq!(key_action(4, Some(1), VK_ESCAPE, false), Leave);
        assert_eq!(key_action(4, None, VK_ESCAPE, false), Leave);
        assert_eq!(key_action(0, None, VK_ESCAPE, false), Leave);
    }

    #[test]
    fn an_empty_dock_does_nothing_else() {
        for key in [
            VK_LEFT, VK_RIGHT, VK_UP, VK_DOWN, VK_HOME, VK_END, VK_RETURN, VK_SPACE, VK_APPS,
        ] {
            assert_eq!(key_action(0, None, key, false), Nothing, "{key:#x}");
            assert_eq!(key_action(0, Some(0), key, false), Nothing, "{key:#x}");
        }
        assert_eq!(key_action(0, None, VK_F10, true), Nothing);
    }

    #[test]
    fn a_choice_past_a_shortened_list_counts_as_its_last_item() {
        assert_eq!(key_action(3, Some(7), VK_LEFT, false), Select(1));
        assert_eq!(key_action(3, Some(7), VK_RETURN, false), Press(2));
        assert_eq!(key_action(3, Some(7), VK_RIGHT, false), Nothing);
    }

    #[test]
    fn other_keys_are_not_the_docks() {
        for key in [0x41, 0x09, 0x70, 0x2E] {
            assert_eq!(key_action(4, Some(1), key, false), Nothing, "{key:#x}");
        }
    }

    #[test]
    fn on_the_right_up_goes_on_along_the_column_read_from_the_bottom() {
        use crate::domain::settings::DockEdge;
        assert_eq!(along(VK_UP, DockEdge::Right), VK_DOWN);
        assert_eq!(along(VK_DOWN, DockEdge::Right), VK_UP);
        assert_eq!(along(VK_LEFT, DockEdge::Right), VK_LEFT);
        for edge in [DockEdge::Bottom, DockEdge::Top, DockEdge::Left] {
            for key in [VK_UP, VK_DOWN, VK_LEFT, VK_RIGHT, VK_HOME] {
                assert_eq!(along(key, edge), key);
            }
        }
    }
}
