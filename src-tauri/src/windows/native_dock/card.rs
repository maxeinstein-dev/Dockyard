//! The card over an item and the preview of its windows.

use super::*;

impl Dock {
    /// Whether this dock's preview card shows.
    pub(super) fn previewing(&self) -> bool {
        super::super::previews::shown_by() == Some(self.hwnd.0 as isize)
    }

    /// The pointer is on the Calendário: its card shows after a moment, or
    /// at once when it already shows (spec 0017 F2a-4).
    pub(super) fn arm_card(&mut self) {
        let _ = unsafe { KillTimer(Some(self.hwnd), CARD_TIMER) };
        let calendar = self.hovered.and_then(|index| self.items.get(index)).is_some_and(
            |item| matches!(item, Item::Widget(widget) if matches!(widget.config, WidgetConfig::Calendar(_))),
        );
        if !calendar {
            return;
        }
        if super::super::agenda::shown_by() == Some(self.hwnd.0 as isize) {
            self.show_card();
        } else {
            unsafe { SetTimer(Some(self.hwnd), CARD_TIMER, CARD_DELAY, None) };
        }
    }

    /// Shows the card of the Calendário under the pointer.
    pub(super) fn show_card(&mut self) {
        let busy = self.press.is_some() || self.open.is_some() || self.menu_open || self.tucking;
        let Some(index) = self.hovered.filter(|_| !busy) else {
            return;
        };
        let Some(Item::Widget(widget)) = self.items.get(index) else {
            return;
        };
        let WidgetConfig::Calendar(config) = &widget.config else {
            return;
        };
        let Some(snapshot) = super::super::calendar::snapshot() else {
            return;
        };
        let Some(area) = self.placement_now().tiles.get(index).copied() else {
            return;
        };
        let now = super::super::calendar::now();
        let shown: Vec<_> = snapshot
            .events
            .iter()
            .filter(|event| config.shows(&event.calendar))
            .collect();
        let tasks = crate::domain::agenda::ordered(&snapshot.tasks, &config.task_order);
        let words = &crate::i18n::messages().dock;
        let rows = crate::domain::agenda::rows(
            &shown,
            &tasks,
            config.card_tasks && config.tasks,
            crate::domain::agenda::range(config.card_period, now, snapshot.offset),
            now,
            snapshot.offset,
            &crate::domain::agenda::Words {
                today: words.calendar_today,
                tomorrow: words.calendar_tomorrow_title,
                weekdays: words.weekdays,
                all_day: words.calendar_all_day,
                now: words.calendar_now,
                busy: words.calendar_busy,
                untitled: words.calendar_untitled,
                tasks: words.calendar_tasks,
                free: words.calendar_nothing,
            },
        );
        let rect = self.monitor.rect;
        super::super::agenda::show(super::super::agenda::Request {
            owner: self.hwnd.0 as isize,
            widget: widget.id.clone(),
            tile: Area {
                x: f64::from(self.window.x) + area.x * self.scale,
                y: f64::from(self.window.y) + area.y * self.scale,
                width: area.width * self.scale,
                height: area.height * self.scale,
            },
            screen: Area {
                x: f64::from(rect.x),
                y: f64::from(rect.y),
                width: f64::from(rect.width),
                height: f64::from(rect.height),
            },
            edge: self.edge(),
            scale: self.scale,
            dark: self.dark_now(),
            rows,
        });
    }

    /// Opens the Calendário `widget`'s panel, on `event`'s details when
    /// given (a click on the card).
    pub(super) fn open_calendar(
        &mut self,
        widget: &crate::domain::widgets::WidgetId,
        event: Option<String>,
    ) {
        super::super::calendar::open_event(event);
        let Some(index) = self
            .items
            .iter()
            .position(|item| matches!(item, Item::Widget(each) if &each.id == widget))
        else {
            return;
        };
        let Some(item) = self.items.get(index).cloned() else {
            return;
        };
        if self.open.as_ref() == Some(&panel_of(&item)) {
            // Already open: it reads the event it should show.
            self.effects.push(Box::new(|app| {
                let _ = tauri::Emitter::emit(app, super::super::calendar::OPEN_EVENT, ());
            }));
            return;
        }
        let anchor = self.rest().items[index];
        self.toggle(panel_of(&item), anchor);
    }

    /// The pointer is on another item: with a card already shown it changes
    /// at once (spec 0015, proposal 1), else it shows after the delay. The
    /// card hides itself once the pointer has left the icon and the card.
    pub(super) fn arm_preview(&mut self) {
        self.arm_card();
        let _ = unsafe { KillTimer(Some(self.hwnd), PREVIEW_TIMER) };
        let settings = &self.data.settings.dock;
        let program = self
            .hovered
            .and_then(|index| self.items.get(index))
            .is_some_and(Item::is_program);
        if !settings.previews || !program {
            return;
        }
        if self.previewing() || settings.preview_delay == 0 {
            self.show_preview();
        } else {
            let delay = u32::from(settings.preview_delay);
            unsafe { SetTimer(Some(self.hwnd), PREVIEW_TIMER, delay, None) };
        }
    }

    /// Shows the card of the program under the pointer: its windows in this
    /// dock's workspace.
    pub(super) fn show_preview(&mut self) {
        let busy = self.press.is_some() || self.open.is_some() || self.menu_open || self.tucking;
        let Some(index) = self.hovered.filter(|_| !busy) else {
            return;
        };
        let workspace = self.workspace_id();
        let windows = match self.items.get(index) {
            Some(Item::App(app)) => {
                super::super::open_apps::windows_of_pinned(app.id.as_str(), workspace.as_deref())
            }
            Some(Item::Other(other)) => {
                super::super::open_apps::windows_of_other(&other.key, workspace.as_deref())
            }
            _ => return,
        };
        let settings = &self.data.settings.dock;
        if !crate::domain::previews::shows(windows.len(), settings.previews_only_several) {
            if self.previewing() {
                super::super::previews::hide();
            }
            return;
        }
        let Some(area) = self.placement_now().items.get(index).copied() else {
            return;
        };
        let rect = self.monitor.rect;
        super::super::previews::show(super::super::previews::Request {
            owner: self.hwnd.0 as isize,
            icon: Area {
                x: f64::from(self.window.x) + area.x * self.scale,
                y: f64::from(self.window.y) + area.y * self.scale,
                width: area.width * self.scale,
                height: area.height * self.scale,
            },
            screen: Area {
                x: f64::from(rect.x),
                y: f64::from(rect.y),
                width: f64::from(rect.width),
                height: f64::from(rect.height),
            },
            edge: self.edge(),
            scale: self.scale,
            dark: self.dark_now(),
            windows,
            name: self.names.get(index).cloned().unwrap_or_default(),
            motion: self.motion,
            media: None,
        });
    }

    /// Hides this dock's card, if it shows (a click, a menu, a drag, the dock
    /// going down).
    pub(super) fn end_preview(&mut self) {
        let _ = unsafe { KillTimer(Some(self.hwnd), PREVIEW_TIMER) };
        if self.previewing() {
            super::super::previews::hide();
        }
        // The Calendário's card goes the same way.
        let _ = unsafe { KillTimer(Some(self.hwnd), CARD_TIMER) };
        if super::super::agenda::shown_by() == Some(self.hwnd.0 as isize) {
            super::super::agenda::hide();
        }
    }

    /// Whether the item under the pointer opens a panel that warming helps:
    /// not a program (a click opens it) nor Configurações (an app of its own).
    pub(super) fn warmable(&self) -> bool {
        // The tray's arrow opens the tray panel.
        if self
            .strip_hovered
            .and_then(|index| self.strip.get(index))
            .is_some_and(|cell| *cell == Cell::Arrow)
        {
            return true;
        }
        self.hovered
            .and_then(|index| self.items.get(index))
            .is_some_and(|item| {
                // The Lançador opens the launcher, not a panel.
                !item.is_program()
                    && !matches!(item, Item::System(SystemItem::Settings))
                    && !is_launcher(item)
            })
    }
}
