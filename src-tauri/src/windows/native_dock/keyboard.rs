//! The keyboard in the dock (M14): the focus, the arrows and the ring.

use super::*;

impl Dock {
    /// What the keyboard goes through, in the order assistive technology
    /// reads it (`publish`): the workspace dots, the items, the strip.
    pub(super) fn nav_keys(&self) -> NavKeys {
        let dots = self.rest().switcher.map_or(0, |area| {
            dock_layout::switcher_dots(area, self.data.workspaces.len()).len()
        });
        NavKeys {
            dots: self
                .data
                .workspaces
                .iter()
                .take(dots)
                .map(|w| format!("{WORKSPACE_KEY}{}", w.id))
                .collect(),
            // A spacer is room, not a stop (spec 0022).
            items: self
                .items
                .iter()
                .filter(|item| !matches!(item, Item::Spacer(_)))
                .map(Item::key)
                .collect(),
            strip: self.strip.iter().map(Cell::key).collect(),
        }
    }

    /// Where the item the keyboard is on is in `nav_keys`.
    pub(super) fn keyboard_index(&self) -> Option<usize> {
        let key = self.keyboard.as_ref()?;
        self.nav_keys().all().position(|each| each == key)
    }

    /// Where the item the keyboard is on is in one part of `nav_keys`.
    pub(super) fn keyboard_position(
        &self,
        part: impl Fn(&NavKeys) -> &Vec<String>,
    ) -> Option<usize> {
        let key = self.keyboard.as_ref()?;
        part(&self.nav_keys()).iter().position(|each| each == key)
    }

    /// The list changed under the keyboard: an item gone leaves it on the
    /// one now in its place (or the last).
    pub(super) fn keep_keyboard(&mut self, chosen_at: Option<usize>) {
        let Some(at) = chosen_at else {
            return;
        };
        let keys: Vec<String> = self.nav_keys().all().cloned().collect();
        if self.keyboard.as_ref().is_some_and(|key| keys.contains(key)) {
            return;
        }
        let next = keys.get(at.min(keys.len().saturating_sub(1))).cloned();
        self.keyboard.clone_from(&next);
        access::set_focus(self.hwnd.0 as isize, next);
    }

    /// The box of the item the keyboard is on, as drawn now (logical, in the
    /// window).
    pub(super) fn keyboard_area(&self) -> Option<Area> {
        let key = self.keyboard.as_ref()?;
        let now = self.placement_now();
        if let Some(id) = key.strip_prefix(WORKSPACE_KEY) {
            let index = self.data.workspaces.iter().position(|w| w.id == id)?;
            let (x, y) = *now
                .switcher
                .map(|area| dock_layout::switcher_dots(area, self.data.workspaces.len()))
                .unwrap_or_default()
                .get(index)?;
            let side = 2.0 * dock_layout::ACTIVE_DOT_RADIUS + 6.0;
            return Some(Area {
                x: x - side / 2.0,
                y: y - side / 2.0,
                width: side,
                height: side,
            });
        }
        if let Some(index) = self.items.iter().position(|item| &item.key() == key) {
            let mut area = *now.items.get(index)?;
            area.x += self.shown_offsets(&self.rest()).get(index).copied()?;
            return Some(area);
        }
        let index = self.strip.iter().position(|cell| &cell.key() == key)?;
        self.strip_cells().get(index).copied()
    }

    /// The ring around the item the keyboard is on: the accent, or the
    /// system's highlight in contrast mode, with a line of the bar's color
    /// inside it so it shows on any icon.
    pub(super) fn focus_ring(&self, palette: &Palette) -> Option<FocusRing> {
        let area = self.keyboard_area()?;
        let (ring, inside) = if super::super::launcher::high_contrast() {
            use windows::Win32::Graphics::Gdi::{COLOR_HIGHLIGHT, COLOR_WINDOW};
            let color = |index| {
                let (r, g, b) = super::super::launcher::system_color(index);
                Color::rgb(r, g, b)
            };
            (color(COLOR_HIGHLIGHT), color(COLOR_WINDOW))
        } else {
            (palette.accent, palette.surface.with_alpha(1.0))
        };
        Some(FocusRing { area, ring, inside })
    }

    /// The point a menu opened by the keyboard comes from: the middle of the
    /// item's side away from the screen edge, on the screen.
    pub(super) fn keyboard_menu_point(&self) -> Option<POINT> {
        let area = self.keyboard_area()?;
        let (x, y) = match self.edge() {
            DockEdge::Bottom => (area.center_x(), area.y),
            DockEdge::Top => (area.center_x(), area.y + area.height),
            DockEdge::Left => (area.x + area.width, area.center_y()),
            DockEdge::Right => (area.x, area.center_y()),
        };
        Some(POINT {
            x: self.window.x + (x * self.scale).round() as i32,
            y: self.window.y + (y * self.scale).round() as i32,
        })
    }

    /// The keyboard is on `key` now (or on nothing), for the ring and for
    /// assistive technology.
    pub(super) fn select(&mut self, key: Option<String>) {
        self.keyboard.clone_from(&key);
        access::set_focus(self.hwnd.0 as isize, key);
        self.paint();
    }

    /// The hotkey brought the keyboard here, from the window `before`: the
    /// dock comes to the front with the first item chosen.
    pub(super) fn take_keyboard(&mut self, before: Option<HWND>) {
        self.end_preview();
        self.untuck();
        // WS_EX_NOACTIVATE only keeps a click from activating the dock (the
        // program in use stays in front); activating it on purpose still
        // works. Windows lets a process come to the front when the last
        // input was its own, and the hotkey just pressed was.
        if !unsafe { windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow(self.hwnd) }
            .as_bool()
        {
            log::warn!("dock: Windows did not let the dock take the keyboard");
            return;
        }
        let _ = unsafe { windows::Win32::UI::Input::KeyboardAndMouse::SetFocus(Some(self.hwnd)) };
        self.before = before;
        let first = self
            .keyboard
            .clone()
            .or_else(|| self.nav_keys().all().next().cloned());
        self.select(first);
    }

    /// The dock lost the keyboard: nothing is chosen, and the focus is left
    /// where it went. Not while its menu is up, which takes the keyboard and
    /// gives it back.
    pub(super) fn lose_keyboard(&mut self) {
        if self.menu_open || (self.keyboard.is_none() && self.before.is_none()) {
            return;
        }
        self.before = None;
        self.select(None);
    }

    /// Esc: nothing is chosen, and the window from before has the keyboard
    /// again.
    pub(super) fn leave(&mut self) {
        use windows::Win32::UI::WindowsAndMessaging::{IsWindow, SetForegroundWindow};
        let before = self.before.take();
        self.select(None);
        if let Some(window) = before.filter(|window| unsafe { IsWindow(Some(*window)) }.as_bool()) {
            let _ = unsafe { SetForegroundWindow(window) };
        }
    }

    /// A key pressed while the dock has the keyboard; whether it was the
    /// dock's.
    pub(super) fn key_down(&mut self, key: u16, shift: bool) -> bool {
        use keys::{KeyAction, key_action};
        // The card opened from the keyboard takes the keys while it shows
        // (F10 of estudo 0002); the item stays chosen when it goes.
        if self.previewing()
            && let Some(still) = super::super::previews::key(key)
        {
            if !still {
                self.paint();
            }
            return true;
        }
        let ours = matches!(
            key,
            keys::VK_LEFT
                | keys::VK_RIGHT
                | keys::VK_UP
                | keys::VK_DOWN
                | keys::VK_HOME
                | keys::VK_END
                | keys::VK_RETURN
                | keys::VK_SPACE
                | keys::VK_APPS
                | keys::VK_ESCAPE
        ) || (key == keys::VK_F10 && shift);
        if !ours {
            return false;
        }
        // The arrow toward the card opens the chosen program's windows.
        let (toward, _) = crate::domain::previews::card_keys(self.edge());
        if key == toward
            && let Some(index) = self
                .keyboard
                .as_ref()
                .and_then(|chosen| self.items.iter().position(|item| item.key() == *chosen))
            && self.items[index].is_program()
            && self.show_preview_of(index, Some(0))
        {
            return true;
        }
        let all: Vec<String> = self.nav_keys().all().cloned().collect();
        match key_action(all.len(), self.keyboard_index(), key, shift) {
            KeyAction::Select(index) => self.select(all.get(index).cloned()),
            KeyAction::Press(index) => {
                // As a click: the panel it opens may take the focus.
                shared().clicked = Some(Instant::now());
                self.use_here();
                self.press(&all[index]);
            }
            KeyAction::Menu(index) => self.item_menu(&all[index]),
            KeyAction::Leave => self.leave(),
            KeyAction::Nothing => {}
        }
        true
    }

    /// The menu of the item `key`, as the right button opens it, by the
    /// item.
    pub(super) fn item_menu(&mut self, key: &str) {
        shared().clicked = Some(Instant::now());
        self.use_here();
        self.menu_by_keyboard = true;
        if key.starts_with(WORKSPACE_KEY) {
            self.dock_menu(None);
        } else if let Some(index) = self.items.iter().position(|item| item.key() == key) {
            if self.items[index].is_program() {
                self.right_click(index);
            } else {
                self.dock_menu(Some(index));
            }
        } else if let Some(index) = self.strip.iter().position(|cell| cell.key() == key) {
            self.strip_click(index, true);
        }
        self.menu_by_keyboard = false;
    }

    /// The dock's menu closed: without the keyboard back, nothing is
    /// chosen any more.
    pub(super) fn menu_closed(&mut self) {
        self.menu_open = false;
        let foreground = unsafe { windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow() };
        if foreground != self.hwnd {
            self.lose_keyboard();
        }
    }
}

/// What the keyboard goes through, in parts (`Dock::nav_keys`).
pub(super) struct NavKeys {
    pub(super) dots: Vec<String>,
    pub(super) items: Vec<String>,
    pub(super) strip: Vec<String>,
}

impl NavKeys {
    fn all(&self) -> impl Iterator<Item = &String> {
        self.dots.iter().chain(&self.items).chain(&self.strip)
    }
}
