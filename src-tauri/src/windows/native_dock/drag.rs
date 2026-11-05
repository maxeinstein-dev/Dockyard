//! Dragging an item along the dock and dropping files on it.

use super::*;

impl Dock {
    /// The drag drawn: the one going on, or the one let go while its item
    /// waits for its new place; with where the pointer is along the bar.
    pub(super) fn shown_drag(&self) -> Option<(Press, f64)> {
        self.press
            .filter(|press| press.dragging)
            .map(|press| (press, self.drag_x))
            .or(self.landing.map(|landing| (landing.press, landing.x)))
    }

    /// How far the dragged item is from its place at rest: with the pointer,
    /// inside its group.
    pub(super) fn drag_shift(press: Press, x: f64, rest: &dock_layout::Placement) -> f64 {
        let along = |index: usize| rest.side.along_area(rest.items[index]);
        let first = along(press.group.0);
        let last = along(press.group.0 + press.group.1 - 1);
        let item = along(press.index);
        drag_offset(
            x,
            press.start_x,
            item.x,
            item.width,
            first.x,
            last.x + last.width,
        )
    }

    /// Where each item is headed, from its place at rest: the dragged item
    /// with the pointer, the others making room where it would land.
    pub(super) fn slide_targets(&self, rest: &dock_layout::Placement) -> Vec<f64> {
        let mut targets = vec![0.0; self.items.len()];
        let Some((press, x)) = self
            .shown_drag()
            .filter(|(press, _)| press.range().end <= rest.items.len())
        else {
            return targets;
        };
        // Along the bar: on a side, the column read in its order.
        let group: Vec<Area> = rest.items[press.range()]
            .iter()
            .map(|&area| rest.side.along_area(area))
            .collect();
        let centers: Vec<f64> = group.iter().map(Area::center_x).collect();
        let target = drop_index(&centers, press.local(), x);
        let room = make_room(
            press.group.1,
            press.local(),
            target,
            drag_pitch(&group, press.local()),
        );
        for (local, offset) in room.into_iter().enumerate() {
            targets[press.group.0 + local] = offset;
        }
        targets[press.index] = Self::drag_shift(press, x, rest);
        targets
    }

    /// How far each item is drawn from its place at rest now: the dragged
    /// one right with the pointer, the others where their slide got to.
    pub(super) fn shown_offsets(&self, rest: &dock_layout::Placement) -> Vec<f64> {
        let mut offsets: Vec<f64> = (0..self.items.len())
            .map(|index| self.slide.get(index).copied().unwrap_or(0.0))
            .collect();
        if let Some((press, x)) = self
            .shown_drag()
            .filter(|(press, _)| press.range().end <= rest.items.len())
        {
            offsets[press.index] = Self::drag_shift(press, x, rest);
        }
        offsets
    }
}

impl Dock {
    /// The pointer shows the icon of the program at `index` while its tile
    /// is off the dock, and the arrow again with `None`.
    pub(super) fn show_drag_cursor(&mut self, index: Option<usize>) {
        match index {
            Some(index) => {
                if self.drag_cursor.is_none() {
                    let side = (48.0 * self.scale).round() as i32;
                    self.drag_cursor = self
                        .program_id(index)
                        .and_then(|id| super::super::apps::drag_cursor(&id, side));
                }
                let cursor = self
                    .drag_cursor
                    .or_else(|| unsafe { LoadCursorW(None, IDC_ARROW) }.ok());
                unsafe { SetCursor(cursor) };
            }
            None => {
                if let Some(cursor) = self.drag_cursor.take() {
                    unsafe {
                        SetCursor(LoadCursorW(None, IDC_ARROW).ok());
                        let _ = DestroyCursor(cursor);
                    }
                }
            }
        }
    }

    /// Files dragged over the dock at the screen point `at` (physical
    /// pixels), or gone with `None`: the program under them is shown as
    /// under the pointer. Its id, when there is one.
    pub(super) fn drop_over(&mut self, at: Option<(i32, i32)>) -> Option<String> {
        let index = at.and_then(|(x, y)| {
            let mut point = POINT { x, y };
            unsafe { windows::Win32::Graphics::Gdi::ScreenToClient(self.hwnd, &mut point) }
                .as_bool()
                .then(|| {
                    self.hit(
                        f64::from(point.x) / self.scale,
                        f64::from(point.y) / self.scale,
                    )
                })
                .flatten()
                .filter(|&index| self.program_id(index).is_some())
        });
        if index != self.drop_hover {
            self.drop_hover = index;
            self.hovered = index;
            self.animate();
            self.paint();
        }
        index.and_then(|index| self.program_id(index))
    }

    /// Files dropped at the screen point (`x`, `y`) (physical pixels) that
    /// get pinned: the key of the pinned program or widget they go before,
    /// `None` past the last one (or with no such item). Along the bar: y
    /// down a column, and up the right's, read from the bottom up.
    pub(super) fn dropped_before(&self, x: i32, y: i32) -> Option<String> {
        let count = self.groups.first().copied().unwrap_or(0);
        let group = self.items.get(..count)?;
        if !group.iter().all(|item| {
            matches!(
                item,
                Item::App(_) | Item::Widget(_) | Item::Spacer(_) | Item::Stack(_)
            )
        }) {
            return None;
        }
        let mut origin = POINT::default();
        unsafe { windows::Win32::Graphics::Gdi::ClientToScreen(self.hwnd, &mut origin) }
            .as_bool()
            .then_some(())?;
        let rest = self.rest();
        let side = rest.side;
        let centers: Vec<f64> = rest
            .items
            .get(..count)?
            .iter()
            .map(|&area| side.along_area(area).center_x())
            .collect();
        // The screen point and the client's origin along the bar: negated
        // on the right, as `Side::along` reads it.
        let (at, from) = match side {
            Side::Bottom | Side::Top => (x, origin.x),
            Side::Left => (y, origin.y),
            Side::Right => (-y, -origin.y),
        };
        dock_items::dropped_before(group, &centers, at, from, self.scale)
    }

    /// The group an item can be dragged in, as its first index and its size:
    /// the pinned programs, the widgets and the spacers, mixed, or the open
    /// programs. The system items stay where they are. A spacer goes
    /// anywhere among the first group, as a widget does (spec 0022), and is
    /// kept in the workspace's order with them.
    pub(super) fn group_of(&self, index: usize) -> Option<(usize, usize)> {
        if matches!(self.items.get(index)?, Item::System(_)) {
            return None;
        }
        let mut start = 0;
        for &size in &self.groups {
            if index < start + size {
                // A group of one has nowhere to go.
                return (size > 1).then_some((start, size));
            }
            start += size;
        }
        None
    }

    /// A drag let go: the item goes where it was let go, in its group.
    /// Returns whether it moves (the dock then reads the new order).
    pub(super) fn drop_at(&mut self, press: Press, x: f64) -> bool {
        let rest = self.rest();
        let centers: Vec<f64> = rest.items[press.range()]
            .iter()
            .map(|&area| rest.side.along_area(area).center_x())
            .collect();
        let target = drop_index(&centers, press.local(), x);
        if target == press.local() {
            return false;
        }
        let Some(item) = self.items.get(press.index).cloned() else {
            return false;
        };
        if let Item::Other(other) = item {
            self.effects.push(Box::new(move |app| {
                super::super::open_apps::place_other(app, &other.key, target);
            }));
            return true;
        }
        // A program, a widget or a spacer, among them all: the order of the
        // workspace this dock shows (spec 0013).
        let group = &self.items[press.range()];
        let Some((order, _)) = dock_items::moved(group, press.local(), target) else {
            return false;
        };
        let Some(workspace) = self.data.workspace.as_ref().map(|w| w.id.clone()) else {
            return false;
        };
        self.effects.push(Box::new(move |app| {
            {
                let state = app.state::<Mutex<Storage>>();
                let mut storage = state.lock().unwrap_or_else(PoisonError::into_inner);
                if let Err(error) = storage.reorder_workspace(&workspace, &order) {
                    log::warn!("dock: the order could not be kept: {error}");
                }
            }
            crate::commands::workspaces::notify(app);
            super::super::dock::refresh();
        }));
        true
    }
}
