//! The workspaces in the `workspaces` and `workspace_items` tables (spec
//! 0013 C1), and the one each monitor shows, in `meta`.
//!
//! Positions are 0-based and contiguous after every write, which runs in one
//! transaction. An item key is kept only if it is one a workspace can hold
//! (`is_item_key`); a key whose program or widget is gone is skipped by the
//! dock, not an error.
//!
//! The spacers (spec 0022) are rows of `spacers`, named in `workspace_items`
//! by `spacer:<id>`; one leaves the table when it leaves its workspace.
//!
//! The stacks of widgets (spec 0023) are rows of `stacks`, named by
//! `stack:<id>`, with their widgets in `stack_members`. A stack is never
//! left with fewer than two: the one left takes its place (`tidy`).

use rusqlite::{Connection, OptionalExtension, Transaction, params};

use super::database::{begin, now_ms, read_failed, write_failed};
use crate::domain::widgets::WidgetId;
use crate::domain::workspaces::{
    FIRST_NAME, MAX_ITEMS, MAX_WORKSPACES, PALETTE, SPACER_PREFIX, STACK_PREFIX, Spacer,
    SpacerSize, Stack, Workspace, WorkspaceError, initial_order, is_item_key, is_palette_color,
    is_valid_name, joined, next_color, next_name, spacer_place,
};

/// Prefix in `meta` of the workspace a monitor shows: `workspace:` and the
/// monitor's device name (`\\.\DISPLAY1`).
const ACTIVE_PREFIX: &str = "workspace:";

/// What follows `ACTIVE_PREFIX` for the workspace in use (no monitor has
/// this name).
const IN_USE_KEY: &str = "in-use";

/// Every workspace, in order, with its items.
pub fn list(conn: &Connection) -> Result<Vec<Workspace>, WorkspaceError> {
    let mut statement = conn
        .prepare("SELECT id, name, color FROM workspaces ORDER BY position")
        .map_err(read_failed)?;
    let rows = statement
        .query_map([], |row| {
            Ok(Workspace {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                items: Vec::new(),
                spacers: Vec::new(),
                stacks: Vec::new(),
            })
        })
        .map_err(read_failed)?;
    let mut workspaces = Vec::new();
    for row in rows {
        let mut workspace = row.map_err(read_failed)?;
        workspace.items = items(conn, &workspace.id)?;
        workspace.spacers = spacers(conn, &workspace.id)?;
        workspace.stacks = stacks(conn, &workspace.id)?;
        workspaces.push(workspace);
    }
    Ok(workspaces)
}

/// The spacers of the workspace `id`, in dock order; a row whose size is
/// not one known is skipped.
fn spacers(conn: &Connection, id: &str) -> Result<Vec<Spacer>, WorkspaceError> {
    let mut statement = conn
        .prepare(
            "SELECT spacers.id, spacers.size FROM workspace_items              JOIN spacers ON workspace_items.item_key = ?2 || spacers.id              WHERE workspace_items.workspace_id = ?1 ORDER BY workspace_items.position",
        )
        .map_err(read_failed)?;
    let found = statement
        .query_map(params![id, SPACER_PREFIX], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(read_failed)?
        .filter_map(Result::ok)
        .filter_map(|(id, size)| {
            Some(Spacer {
                id,
                size: SpacerSize::parse(&size)?,
            })
        })
        .collect();
    Ok(found)
}

/// The keys of the workspace `id`, in dock order.
fn items(conn: &Connection, id: &str) -> Result<Vec<String>, WorkspaceError> {
    let mut statement = conn
        .prepare("SELECT item_key FROM workspace_items WHERE workspace_id = ?1 ORDER BY position")
        .map_err(read_failed)?;
    let keys = statement
        .query_map(params![id], |row| row.get::<_, String>(0))
        .map_err(read_failed)?
        .filter_map(Result::ok)
        .filter(|key| is_item_key(key))
        .collect();
    Ok(keys)
}

/// Makes the first workspace when there is none: "Principal", with the
/// programs and the widgets there are, in the order the dock showed them
/// (`dock_order`, the order of before the workspaces).
pub fn ensure_first(conn: &mut Connection, dock_order: &[String]) -> Result<(), WorkspaceError> {
    let any: bool = conn
        .query_row("SELECT EXISTS(SELECT 1 FROM workspaces)", [], |row| {
            row.get(0)
        })
        .map_err(read_failed)?;
    if any {
        return Ok(());
    }
    let tx = begin(conn)?;
    let keys = |sql: &str, kind: &str| -> Result<Vec<String>, WorkspaceError> {
        let mut statement = tx.prepare(sql).map_err(read_failed)?;
        let ids = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(read_failed)?
            .filter_map(Result::ok)
            .map(|id| format!("{kind}:{id}"))
            .collect();
        Ok(ids)
    };
    let apps = keys("SELECT id FROM pinned_apps ORDER BY position", "app")?;
    let widgets = keys(
        "SELECT id FROM widget_instances ORDER BY position",
        "widget",
    )?;
    let id = WidgetId::new();
    tx.execute(
        "INSERT INTO workspaces (id, name, color, position, created_at) VALUES (?1, ?2, ?3, 0, ?4)",
        params![id.as_str(), FIRST_NAME, PALETTE[0], now_ms()],
    )
    .map_err(write_failed)?;
    write_items(
        &tx,
        id.as_str(),
        &initial_order(dock_order, &apps, &widgets),
    )?;
    tx.commit().map_err(write_failed)?;
    Ok(())
}

/// Replaces the items of the workspace `id` with `keys`, in that order.
fn write_items(tx: &Transaction<'_>, id: &str, keys: &[String]) -> Result<(), WorkspaceError> {
    tx.execute(
        "DELETE FROM workspace_items WHERE workspace_id = ?1",
        params![id],
    )
    .map_err(write_failed)?;
    let mut seen = std::collections::HashSet::new();
    let kept = keys
        .iter()
        .filter(|key| is_item_key(key) && seen.insert(key.as_str()))
        .take(MAX_ITEMS);
    for (position, key) in kept.enumerate() {
        tx.execute(
            "INSERT INTO workspace_items (workspace_id, item_key, position) VALUES (?1, ?2, ?3)",
            params![id, key, position as i64],
        )
        .map_err(write_failed)?;
    }
    Ok(())
}

/// The ids of the workspaces, in order.
fn ids(conn: &Connection) -> Result<Vec<String>, WorkspaceError> {
    let mut statement = conn
        .prepare("SELECT id FROM workspaces ORDER BY position")
        .map_err(read_failed)?;
    let ids = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(read_failed)?
        .filter_map(Result::ok)
        .collect();
    Ok(ids)
}

fn renumber(tx: &Transaction<'_>, ids: &[String]) -> Result<(), WorkspaceError> {
    for (position, id) in ids.iter().enumerate() {
        tx.execute(
            "UPDATE workspaces SET position = ?1 WHERE id = ?2",
            params![position as i64, id],
        )
        .map_err(write_failed)?;
    }
    Ok(())
}

fn exists(conn: &Connection, id: &str) -> Result<bool, WorkspaceError> {
    let found = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM workspaces WHERE id = ?1)",
            params![id],
            |row| row.get(0),
        )
        .map_err(read_failed)?;
    Ok(found)
}

/// Makes a workspace at the end, empty: named `name`, or the next free
/// "Espaço N"; with the next free color of the palette.
pub fn create(conn: &mut Connection, name: Option<&str>) -> Result<Workspace, WorkspaceError> {
    if name.is_some_and(|name| !is_valid_name(name)) {
        return Err(WorkspaceError::InvalidName);
    }
    let tx = begin(conn)?;
    let existing = list(&tx)?;
    if existing.len() >= MAX_WORKSPACES {
        return Err(WorkspaceError::LimitReached);
    }
    let names: Vec<&str> = existing.iter().map(|w| w.name.as_str()).collect();
    let colors: Vec<&str> = existing.iter().map(|w| w.color.as_str()).collect();
    let workspace = Workspace {
        id: WidgetId::new().as_str().to_owned(),
        name: name.map_or_else(|| next_name(&names), str::to_owned),
        color: next_color(&colors).to_owned(),
        items: Vec::new(),
        spacers: Vec::new(),
        stacks: Vec::new(),
    };
    tx.execute(
        "INSERT INTO workspaces (id, name, color, position, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            workspace.id,
            workspace.name,
            workspace.color,
            existing.len() as i64,
            now_ms()
        ],
    )
    .map_err(write_failed)?;
    tx.commit().map_err(write_failed)?;
    Ok(workspace)
}

/// Copies the workspace `id` into a new one right after it (F7 of estudo
/// 0002): named `name`, or the next "Espaço N", with the next free color and
/// the same items in the same order. The programs and widgets are the same
/// ones, shown in both, as a widget shared with another workspace is; the
/// spacers and the stacks are new rows, so changing one in a workspace
/// leaves the other as it was.
pub fn duplicate(
    conn: &mut Connection,
    id: &str,
    name: Option<&str>,
) -> Result<Workspace, WorkspaceError> {
    if name.is_some_and(|name| !is_valid_name(name)) {
        return Err(WorkspaceError::InvalidName);
    }
    let tx = begin(conn)?;
    let existing = list(&tx)?;
    let at = existing
        .iter()
        .position(|workspace| workspace.id == id)
        .ok_or(WorkspaceError::NotFound)?;
    if existing.len() >= MAX_WORKSPACES {
        return Err(WorkspaceError::LimitReached);
    }
    let source = &existing[at];
    let names: Vec<&str> = existing.iter().map(|w| w.name.as_str()).collect();
    let colors: Vec<&str> = existing.iter().map(|w| w.color.as_str()).collect();
    let copy = WidgetId::new().as_str().to_owned();
    tx.execute(
        "INSERT INTO workspaces (id, name, color, position, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            copy,
            name.map_or_else(|| next_name(&names), str::to_owned),
            next_color(&colors),
            existing.len() as i64,
            now_ms()
        ],
    )
    .map_err(write_failed)?;
    let mut keys = Vec::with_capacity(source.items.len());
    for key in &source.items {
        if let Some(spacer) = source.spacers.iter().find(|spacer| spacer.key() == *key) {
            let new = Spacer {
                id: WidgetId::new().as_str().to_owned(),
                size: spacer.size,
            };
            tx.execute(
                "INSERT INTO spacers (id, size, created_at) VALUES (?1, ?2, ?3)",
                params![new.id, new.size.as_str(), now_ms()],
            )
            .map_err(write_failed)?;
            keys.push(new.key());
        } else if let Some(stack) = source.stacks.iter().find(|stack| stack.key() == *key) {
            let new = Stack {
                id: WidgetId::new().as_str().to_owned(),
                widgets: stack.widgets.clone(),
                top: stack.top,
            };
            tx.execute(
                "INSERT INTO stacks (id, top, created_at) VALUES (?1, ?2, ?3)",
                params![new.id, new.top as i64, now_ms()],
            )
            .map_err(write_failed)?;
            write_stack(&tx, &new)?;
            keys.push(new.key());
        } else if key.starts_with(SPACER_PREFIX) || key.starts_with(STACK_PREFIX) {
            // A spacer or a stack whose row is gone: the dock skips it, and
            // so does the copy.
        } else {
            keys.push(key.clone());
        }
    }
    write_items(&tx, &copy, &keys)?;
    let mut order = ids(&tx)?;
    order.retain(|known| *known != copy);
    order.insert(at + 1, copy.clone());
    renumber(&tx, &order)?;
    let made = list(&tx)?
        .into_iter()
        .find(|workspace| workspace.id == copy)
        .ok_or(WorkspaceError::NotFound)?;
    tx.commit().map_err(write_failed)?;
    Ok(made)
}

pub fn rename(conn: &mut Connection, id: &str, name: &str) -> Result<(), WorkspaceError> {
    if !is_valid_name(name) {
        return Err(WorkspaceError::InvalidName);
    }
    let changed = conn
        .execute(
            "UPDATE workspaces SET name = ?1 WHERE id = ?2",
            params![name, id],
        )
        .map_err(write_failed)?;
    if changed == 0 {
        return Err(WorkspaceError::NotFound);
    }
    Ok(())
}

pub fn recolor(conn: &mut Connection, id: &str, color: &str) -> Result<(), WorkspaceError> {
    if !is_palette_color(color) {
        return Err(WorkspaceError::InvalidColor);
    }
    let color = PALETTE
        .iter()
        .find(|known| known.eq_ignore_ascii_case(color))
        .copied()
        .unwrap_or(PALETTE[0]);
    let changed = conn
        .execute(
            "UPDATE workspaces SET color = ?1 WHERE id = ?2",
            params![color, id],
        )
        .map_err(write_failed)?;
    if changed == 0 {
        return Err(WorkspaceError::NotFound);
    }
    Ok(())
}

/// Moves the workspace one place up or down; at either end nothing changes.
pub fn shift(conn: &mut Connection, id: &str, up: bool) -> Result<(), WorkspaceError> {
    let tx = begin(conn)?;
    let mut order = ids(&tx)?;
    let index = order
        .iter()
        .position(|known| known == id)
        .ok_or(WorkspaceError::NotFound)?;
    let other = if up {
        index.checked_sub(1)
    } else {
        (index + 1 < order.len()).then_some(index + 1)
    };
    let Some(other) = other else {
        return Ok(());
    };
    order.swap(index, other);
    renumber(&tx, &order)?;
    tx.commit().map_err(write_failed)?;
    Ok(())
}

/// Deletes the workspace `id` (never the last one) and returns the keys of
/// its items that no other workspace has: those programs and widgets are to
/// be removed for good. The monitors that showed it go to the first one.
pub fn delete(conn: &mut Connection, id: &str) -> Result<Vec<String>, WorkspaceError> {
    let tx = begin(conn)?;
    let mut order = ids(&tx)?;
    let index = order
        .iter()
        .position(|known| known == id)
        .ok_or(WorkspaceError::NotFound)?;
    if order.len() == 1 {
        return Err(WorkspaceError::LastWorkspace);
    }
    // A stack goes with its workspace; its widgets are what may be left
    // behind.
    let mut gone = Vec::new();
    for key in items(&tx, id)? {
        match key.strip_prefix(STACK_PREFIX) {
            Some(stack) => {
                gone.extend(
                    members(&tx, stack)?
                        .into_iter()
                        .map(|w| format!("widget:{w}")),
                );
                tx.execute("DELETE FROM stacks WHERE id = ?1", params![stack])
                    .map_err(write_failed)?;
            }
            None => gone.push(key),
        }
    }
    tx.execute(
        "DELETE FROM workspace_items WHERE workspace_id = ?1",
        params![id],
    )
    .map_err(write_failed)?;
    tx.execute("DELETE FROM workspaces WHERE id = ?1", params![id])
        .map_err(write_failed)?;
    order.remove(index);
    renumber(&tx, &order)?;
    tx.execute(
        "DELETE FROM meta WHERE key LIKE ?1 AND value = ?2",
        params![format!("{ACTIVE_PREFIX}%"), id],
    )
    .map_err(write_failed)?;
    let mut orphans = Vec::new();
    for key in gone {
        if !held(&tx, &key)? {
            orphans.push(key);
        }
    }
    tx.commit().map_err(write_failed)?;
    Ok(orphans)
}

/// Whether any workspace has the item `key`: in its list, or (a widget)
/// in one of its stacks.
pub fn held(conn: &Connection, key: &str) -> Result<bool, WorkspaceError> {
    let found = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM workspace_items WHERE item_key = ?1)
                 OR EXISTS(SELECT 1 FROM stack_members
                     JOIN workspace_items ON workspace_items.item_key = ?2 || stack_members.stack_id
                     WHERE 'widget:' || stack_members.widget_id = ?1)",
            params![key, STACK_PREFIX],
            |row| row.get(0),
        )
        .map_err(read_failed)?;
    Ok(found)
}

/// Puts the item `key` at the end of the workspace `id`, unless it is there.
pub fn add_item(conn: &mut Connection, id: &str, key: &str) -> Result<(), WorkspaceError> {
    let tx = begin(conn)?;
    if !exists(&tx, id)? {
        return Err(WorkspaceError::NotFound);
    }
    let mut keys = items(&tx, id)?;
    if keys.iter().any(|known| known == key) {
        return Ok(());
    }
    if keys.len() >= MAX_ITEMS {
        return Err(WorkspaceError::LimitReached);
    }
    keys.push(key.to_owned());
    write_items(&tx, id, &keys)?;
    tx.commit().map_err(write_failed)?;
    Ok(())
}

/// Takes the item `key` out of the workspace `id`; returns whether another
/// workspace still has it.
pub fn remove_item(conn: &mut Connection, id: &str, key: &str) -> Result<bool, WorkspaceError> {
    let tx = begin(conn)?;
    tx.execute(
        "DELETE FROM workspace_items WHERE workspace_id = ?1 AND item_key = ?2",
        params![id, key],
    )
    .map_err(write_failed)?;
    let keys = items(&tx, id)?;
    write_items(&tx, id, &keys)?;
    let elsewhere = held(&tx, key)?;
    tx.commit().map_err(write_failed)?;
    Ok(elsewhere)
}

/// Takes the item `key` out of every workspace (its program or widget was
/// removed for good).
pub fn forget_item(conn: &mut Connection, key: &str) -> Result<(), WorkspaceError> {
    let tx = begin(conn)?;
    forget_in(&tx, key)?;
    tx.commit().map_err(write_failed)?;
    Ok(())
}

/// `forget_item` within the transaction `tx`.
fn forget_in(tx: &Transaction<'_>, key: &str) -> Result<(), WorkspaceError> {
    let holders: Vec<String> = {
        let mut statement = tx
            .prepare("SELECT workspace_id FROM workspace_items WHERE item_key = ?1")
            .map_err(read_failed)?;
        statement
            .query_map(params![key], |row| row.get::<_, String>(0))
            .map_err(read_failed)?
            .filter_map(Result::ok)
            .collect()
    };
    for id in holders {
        let keys: Vec<String> = items(tx, &id)?
            .into_iter()
            .filter(|known| known != key)
            .collect();
        write_items(tx, &id, &keys)?;
    }
    // A widget gone for good left its stacks (the cascade).
    tidy(tx)
}

/// Makes a spacer of `size` in the workspace `id`, right after its item
/// `after` (the one the dock's menu opened on), or at its end
/// (`spacer_place`).
pub fn add_spacer(
    conn: &mut Connection,
    id: &str,
    size: SpacerSize,
    after: Option<&str>,
) -> Result<Spacer, WorkspaceError> {
    let tx = begin(conn)?;
    if !exists(&tx, id)? {
        return Err(WorkspaceError::NotFound);
    }
    let mut keys = items(&tx, id)?;
    if keys.len() >= MAX_ITEMS {
        return Err(WorkspaceError::LimitReached);
    }
    let spacer = Spacer {
        id: WidgetId::new().as_str().to_owned(),
        size,
    };
    tx.execute(
        "INSERT INTO spacers (id, size, created_at) VALUES (?1, ?2, ?3)",
        params![spacer.id, size.as_str(), now_ms()],
    )
    .map_err(write_failed)?;
    keys.insert(spacer_place(&keys, after), spacer.key());
    write_items(&tx, id, &keys)?;
    tx.commit().map_err(write_failed)?;
    Ok(spacer)
}

/// Removes the spacer `id` for good: out of the workspace that has it, and
/// its row. `NotFound` when there is no such spacer.
pub fn remove_spacer(conn: &mut Connection, id: &str) -> Result<(), WorkspaceError> {
    let tx = begin(conn)?;
    let removed = tx
        .execute("DELETE FROM spacers WHERE id = ?1", params![id])
        .map_err(write_failed)?;
    if removed == 0 {
        return Err(WorkspaceError::NotFound);
    }
    forget_in(&tx, &format!("{SPACER_PREFIX}{id}"))?;
    tx.commit().map_err(write_failed)?;
    Ok(())
}

/// The stacks of the workspace `id`, in dock order; one with no widget left
/// is skipped.
fn stacks(conn: &Connection, id: &str) -> Result<Vec<Stack>, WorkspaceError> {
    let found: Vec<(String, i64)> = {
        let mut statement = conn
            .prepare(
                "SELECT stacks.id, stacks.top FROM workspace_items \
                 JOIN stacks ON workspace_items.item_key = ?2 || stacks.id \
                 WHERE workspace_items.workspace_id = ?1 ORDER BY workspace_items.position",
            )
            .map_err(read_failed)?;
        statement
            .query_map(params![id, STACK_PREFIX], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .map_err(read_failed)?
            .filter_map(Result::ok)
            .collect()
    };
    let mut stacks = Vec::new();
    for (id, top) in found {
        let widgets = members(conn, &id)?;
        if widgets.is_empty() {
            continue;
        }
        let top = usize::try_from(top).unwrap_or(0).min(widgets.len() - 1);
        stacks.push(Stack { id, widgets, top });
    }
    Ok(stacks)
}

/// The ids of the widgets of the stack `id`, in order.
fn members(conn: &Connection, id: &str) -> Result<Vec<String>, WorkspaceError> {
    let mut statement = conn
        .prepare("SELECT widget_id FROM stack_members WHERE stack_id = ?1 ORDER BY position")
        .map_err(read_failed)?;
    let found = statement
        .query_map(params![id], |row| row.get(0))
        .map_err(read_failed)?
        .filter_map(Result::ok)
        .collect();
    Ok(found)
}

/// The workspace that has the stack `id`, if one does.
fn holder_of(conn: &Connection, id: &str) -> Result<Option<String>, WorkspaceError> {
    let found = conn
        .query_row(
            "SELECT workspace_id FROM workspace_items WHERE item_key = ?1",
            params![format!("{STACK_PREFIX}{id}")],
            |row| row.get(0),
        )
        .optional()
        .map_err(read_failed)?;
    Ok(found)
}

/// The stack `id` and the workspace that has it; `NotFound` when no
/// workspace has such a stack.
fn stack_and_workspace(conn: &Connection, id: &str) -> Result<(Stack, String), WorkspaceError> {
    let workspace = holder_of(conn, id)?.ok_or(WorkspaceError::NotFound)?;
    let stack = stacks(conn, &workspace)?
        .into_iter()
        .find(|stack| stack.id == id)
        .ok_or(WorkspaceError::NotFound)?;
    Ok((stack, workspace))
}

/// Writes the widgets and the top of `stack`, whose row exists.
fn write_stack(tx: &Transaction<'_>, stack: &Stack) -> Result<(), WorkspaceError> {
    tx.execute(
        "UPDATE stacks SET top = ?2 WHERE id = ?1",
        params![stack.id, stack.top as i64],
    )
    .map_err(write_failed)?;
    tx.execute(
        "DELETE FROM stack_members WHERE stack_id = ?1",
        params![stack.id],
    )
    .map_err(write_failed)?;
    for (position, widget) in stack.widgets.iter().enumerate() {
        tx.execute(
            "INSERT INTO stack_members (stack_id, widget_id, position) VALUES (?1, ?2, ?3)",
            params![stack.id, widget, position as i64],
        )
        .map_err(write_failed)?;
    }
    Ok(())
}

/// The widgets an item key of a workspace stands for, to be stacked: a
/// widget's own id, or a stack's widgets. `None` for any other item.
fn stackable(conn: &Connection, key: &str) -> Result<Option<Vec<String>>, WorkspaceError> {
    if let Some(widget) = key.strip_prefix("widget:") {
        return Ok(Some(vec![widget.to_owned()]));
    }
    match key.strip_prefix(STACK_PREFIX) {
        Some(stack) => Ok(Some(members(conn, stack)?)),
        None => Ok(None),
    }
}

/// Puts the widget or the stack `moving` of the workspace `id` on the widget
/// or the stack `target` ("Empilhar com", spec 0023 §3): one stack in
/// `target`'s place, the first of `moving` on top. `NotFound` when either is
/// not a widget or a stack of it; `LimitReached` past `MAX_STACK`.
pub fn stack_with(
    conn: &mut Connection,
    id: &str,
    moving: &str,
    target: &str,
) -> Result<Stack, WorkspaceError> {
    let tx = begin(conn)?;
    let mut keys = items(&tx, id)?;
    if moving == target || !keys.iter().any(|key| key == moving) {
        return Err(WorkspaceError::NotFound);
    }
    let at = keys
        .iter()
        .position(|key| key == target)
        .ok_or(WorkspaceError::NotFound)?;
    let (Some(on), Some(put)) = (stackable(&tx, target)?, stackable(&tx, moving)?) else {
        return Err(WorkspaceError::NotFound);
    };
    let (widgets, top) = joined(&on, &put).ok_or(WorkspaceError::LimitReached)?;
    let stack = match target.strip_prefix(STACK_PREFIX) {
        Some(existing) => Stack {
            id: existing.to_owned(),
            widgets,
            top,
        },
        None => {
            let stack = Stack {
                id: WidgetId::new().as_str().to_owned(),
                widgets,
                top,
            };
            tx.execute(
                "INSERT INTO stacks (id, top, created_at) VALUES (?1, ?2, ?3)",
                params![stack.id, top as i64, now_ms()],
            )
            .map_err(write_failed)?;
            keys[at] = stack.key();
            stack
        }
    };
    write_stack(&tx, &stack)?;
    if let Some(gone) = moving.strip_prefix(STACK_PREFIX) {
        tx.execute("DELETE FROM stacks WHERE id = ?1", params![gone])
            .map_err(write_failed)?;
    }
    keys.retain(|key| key != moving);
    write_items(&tx, id, &keys)?;
    tx.commit().map_err(write_failed)?;
    Ok(stack)
}

/// Turns the stack `id` by `step` (the wheel, spec 0023 §2): another widget
/// on top, going round.
pub fn turn_stack(conn: &mut Connection, id: &str, step: i32) -> Result<(), WorkspaceError> {
    let (stack, _) = stack_and_workspace(conn, id)?;
    conn.execute(
        "UPDATE stacks SET top = ?2 WHERE id = ?1",
        params![stack.id, stack.turned(step) as i64],
    )
    .map_err(write_failed)?;
    Ok(())
}

/// Takes the widget on top of the stack `id` out of it ("Tirar da pilha"):
/// loose, right after the stack.
pub fn take_top(conn: &mut Connection, id: &str) -> Result<(), WorkspaceError> {
    let (stack, workspace) = stack_and_workspace(conn, id)?;
    let widget = stack.widgets[stack.top].clone();
    let tx = begin(conn)?;
    leave(&tx, &workspace, &stack, &widget, true)?;
    tx.commit().map_err(write_failed)?;
    Ok(())
}

/// Takes the widget `widget` out of whichever stack of the workspace `id`
/// has it, not loose anywhere (it is being removed); returns whether one
/// had it.
pub fn leave_stacks(conn: &mut Connection, id: &str, widget: &str) -> Result<bool, WorkspaceError> {
    let holder = stacks(conn, id)?
        .into_iter()
        .find(|stack| stack.widgets.iter().any(|known| known == widget));
    let Some(stack) = holder else {
        return Ok(false);
    };
    let tx = begin(conn)?;
    leave(&tx, id, &stack, widget, false)?;
    tx.commit().map_err(write_failed)?;
    Ok(true)
}

/// `widget` out of `stack` of the workspace `id`, and loose right after it
/// when `loose`; a stack left with one widget gives its place to it.
fn leave(
    tx: &Transaction<'_>,
    id: &str,
    stack: &Stack,
    widget: &str,
    loose: bool,
) -> Result<(), WorkspaceError> {
    let mut keys = items(tx, id)?;
    let at = keys
        .iter()
        .position(|key| *key == stack.key())
        .ok_or(WorkspaceError::NotFound)?;
    let key = format!("widget:{widget}");
    if loose && !keys.contains(&key) {
        keys.insert(at + 1, key);
    }
    let left: Vec<String> = stack
        .widgets
        .iter()
        .filter(|known| *known != widget)
        .cloned()
        .collect();
    if left.len() < 2 {
        dissolve_into(tx, &mut keys, &stack.id, &left)?;
    } else {
        let removed = stack
            .widgets
            .iter()
            .position(|known| known == widget)
            .unwrap_or(0);
        let top = if removed < stack.top {
            stack.top - 1
        } else {
            stack.top
        };
        write_stack(
            tx,
            &Stack {
                id: stack.id.clone(),
                top: top.min(left.len() - 1),
                widgets: left,
            },
        )?;
    }
    write_items(tx, id, &keys)
}

/// Undoes the stack `id` ("Desfazer pilha" in Configurações): its widgets
/// loose in its place, in order.
pub fn dissolve_stack(conn: &mut Connection, id: &str) -> Result<(), WorkspaceError> {
    let (stack, workspace) = stack_and_workspace(conn, id)?;
    let tx = begin(conn)?;
    let mut keys = items(&tx, &workspace)?;
    dissolve_into(&tx, &mut keys, &stack.id, &stack.widgets)?;
    write_items(&tx, &workspace, &keys)?;
    tx.commit().map_err(write_failed)?;
    Ok(())
}

/// Deletes the stack `id` and puts `widgets` loose in its place among
/// `keys` (those not there already).
fn dissolve_into(
    tx: &Transaction<'_>,
    keys: &mut Vec<String>,
    id: &str,
    widgets: &[String],
) -> Result<(), WorkspaceError> {
    let stack_key = format!("{STACK_PREFIX}{id}");
    let at = keys
        .iter()
        .position(|key| *key == stack_key)
        .unwrap_or(keys.len());
    keys.retain(|key| *key != stack_key);
    let loose: Vec<String> = widgets
        .iter()
        .map(|widget| format!("widget:{widget}"))
        .filter(|key| !keys.contains(key))
        .collect();
    let at = at.min(keys.len());
    keys.splice(at..at, loose);
    tx.execute("DELETE FROM stacks WHERE id = ?1", params![id])
        .map_err(write_failed)?;
    Ok(())
}

/// Undoes every stack left with fewer than two widgets (a widget of it was
/// removed for good): the one left, if any, takes its place.
fn tidy(tx: &Transaction<'_>) -> Result<(), WorkspaceError> {
    let thin: Vec<String> = {
        let mut statement = tx
            .prepare(
                "SELECT stacks.id FROM stacks \
                 LEFT JOIN stack_members ON stack_members.stack_id = stacks.id \
                 GROUP BY stacks.id HAVING COUNT(stack_members.widget_id) < 2",
            )
            .map_err(read_failed)?;
        statement
            .query_map([], |row| row.get(0))
            .map_err(read_failed)?
            .filter_map(Result::ok)
            .collect()
    };
    for id in thin {
        let widgets = members(tx, &id)?;
        match holder_of(tx, &id)? {
            Some(workspace) => {
                let mut keys = items(tx, &workspace)?;
                dissolve_into(tx, &mut keys, &id, &widgets)?;
                write_items(tx, &workspace, &keys)?;
            }
            None => {
                tx.execute("DELETE FROM stacks WHERE id = ?1", params![id])
                    .map_err(write_failed)?;
            }
        }
    }
    Ok(())
}

/// Puts the items of the workspace `id` in the order of `keys` (the dock's
/// drag): its items `keys` does not name keep their order, at the end, and
/// keys of items it does not have are left out.
pub fn reorder(conn: &mut Connection, id: &str, keys: &[String]) -> Result<(), WorkspaceError> {
    let tx = begin(conn)?;
    if !exists(&tx, id)? {
        return Err(WorkspaceError::NotFound);
    }
    let current = items(&tx, id)?;
    let mut ordered: Vec<String> = keys
        .iter()
        .filter(|key| current.contains(key))
        .cloned()
        .collect();
    ordered.extend(current.into_iter().filter(|key| !keys.contains(key)));
    write_items(&tx, id, &ordered)?;
    tx.commit().map_err(write_failed)?;
    Ok(())
}

/// The workspace the monitor `monitor` shows: the one kept for it, or the
/// first one.
pub fn active(conn: &Connection, monitor: &str) -> Result<Option<String>, WorkspaceError> {
    let kept: Option<String> = conn
        .query_row(
            "SELECT value FROM meta WHERE key = ?1",
            params![format!("{ACTIVE_PREFIX}{monitor}")],
            |row| row.get(0),
        )
        .optional()
        .map_err(read_failed)?;
    if let Some(id) = kept
        && exists(conn, &id)?
    {
        return Ok(Some(id));
    }
    Ok(ids(conn)?.into_iter().next())
}

/// The workspace in use: the one of the dock used last, whose items
/// Configurações lists and where a program pinned or a widget added goes;
/// the first one when none was kept or it is gone.
pub fn in_use(conn: &Connection) -> Result<Option<String>, WorkspaceError> {
    active(conn, IN_USE_KEY)
}

/// Keeps `id` as the workspace in use; returns whether it changed.
pub fn set_in_use(conn: &mut Connection, id: &str) -> Result<bool, WorkspaceError> {
    if in_use(conn)?.as_deref() == Some(id) {
        return Ok(false);
    }
    set_active(conn, IN_USE_KEY, id)?;
    Ok(true)
}

/// Keeps `id` as the workspace the monitor `monitor` shows.
pub fn set_active(conn: &mut Connection, monitor: &str, id: &str) -> Result<(), WorkspaceError> {
    if !exists(conn, id)? {
        return Err(WorkspaceError::NotFound);
    }
    conn.execute(
        "INSERT INTO meta (key, value) VALUES (?1, ?2) \
         ON CONFLICT (key) DO UPDATE SET value = excluded.value",
        params![format!("{ACTIVE_PREFIX}{monitor}"), id],
    )
    .map_err(write_failed)?;
    Ok(())
}

/// Prefix in `meta` of a monitor held on the workspace it shows (spec
/// 0030 C): `workspace-fixed:` and the monitor's key.
const FIXED_PREFIX: &str = "workspace-fixed:";

/// Whether the monitor `monitor` is held on the workspace it shows: it does
/// not switch.
pub fn fixed(conn: &Connection, monitor: &str) -> Result<bool, WorkspaceError> {
    let kept: Option<String> = conn
        .query_row(
            "SELECT value FROM meta WHERE key = ?1",
            params![format!("{FIXED_PREFIX}{monitor}")],
            |row| row.get(0),
        )
        .optional()
        .map_err(read_failed)?;
    Ok(kept.as_deref() == Some("1"))
}

/// Holds the monitor `monitor` on the workspace it shows, or lets it go.
pub fn set_fixed(conn: &mut Connection, monitor: &str, fixed: bool) -> Result<(), WorkspaceError> {
    let key = format!("{FIXED_PREFIX}{monitor}");
    if fixed {
        conn.execute(
            "INSERT INTO meta (key, value) VALUES (?1, '1') \
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            params![key],
        )
    } else {
        conn.execute("DELETE FROM meta WHERE key = ?1", params![key])
    }
    .map_err(write_failed)?;
    Ok(())
}

/// A monitor known by a new key (spec 0030 A: its device's path instead of
/// its name, `\\.\DISPLAY1`): what was kept under `old` goes under `new`,
/// unless `new` has its own already.
pub fn adopt_monitor_key(
    conn: &mut Connection,
    old: &str,
    new: &str,
) -> Result<(), WorkspaceError> {
    if old == new {
        return Ok(());
    }
    let tx = conn.transaction().map_err(write_failed)?;
    for prefix in [ACTIVE_PREFIX, FIXED_PREFIX] {
        let (from, to) = (format!("{prefix}{old}"), format!("{prefix}{new}"));
        tx.execute(
            "INSERT OR IGNORE INTO meta (key, value) \
             SELECT ?2, value FROM meta WHERE key = ?1",
            params![from, to],
        )
        .map_err(write_failed)?;
        tx.execute("DELETE FROM meta WHERE key = ?1", params![from])
            .map_err(write_failed)?;
    }
    tx.commit().map_err(write_failed)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::database::MIGRATIONS;

    fn database() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        for migration in MIGRATIONS {
            conn.execute_batch(migration).unwrap();
        }
        let tx = conn.transaction().unwrap();
        tx.execute_batch(
            "INSERT INTO pinned_apps VALUES ('a1', 'App1', 'App 1', 0, 0);
             INSERT INTO pinned_apps VALUES ('a2', 'App2', 'App 2', 1, 0);
             INSERT INTO widget_instances VALUES ('w1', 'clock', NULL, 0, '{}', 1, 0);",
        )
        .unwrap();
        tx.commit().unwrap();
        conn
    }

    fn keys(texts: &[&str]) -> Vec<String> {
        texts.iter().map(|text| (*text).to_owned()).collect()
    }

    /// A first workspace with the program a1 and the widgets w1, w2, w3.
    fn with_three_widgets() -> (Connection, String) {
        let mut conn = database();
        conn.execute_batch(
            "INSERT INTO widget_instances VALUES ('w2', 'note', NULL, 1, '{}', 1, 0);
             INSERT INTO widget_instances VALUES ('w3', 'clock', NULL, 2, '{}', 1, 0);",
        )
        .unwrap();
        ensure_first(
            &mut conn,
            &keys(&["app:a1", "widget:w1", "widget:w2", "widget:w3"]),
        )
        .unwrap();
        let first = list(&conn).unwrap()[0].id.clone();
        (conn, first)
    }

    #[test]
    fn a_widget_stacked_on_another_takes_its_place_on_top() {
        let (mut conn, first) = with_three_widgets();

        let stack = stack_with(&mut conn, &first, "widget:w3", "widget:w1").unwrap();

        assert_eq!(stack.widgets, keys(&["w1", "w3"]));
        assert_eq!(stack.top, 1);
        let all = list(&conn).unwrap();
        assert_eq!(
            all[0].items[..3],
            keys(&["app:a1", &stack.key(), "widget:w2"])
        );
        assert!(!all[0].items.contains(&"widget:w3".to_owned()));
        assert_eq!(all[0].stacks, vec![stack.clone()]);
        // Still held: in a stack of a workspace.
        assert!(held(&conn, "widget:w3").unwrap());

        // A widget on the stack: it joins, on top.
        let more = stack_with(&mut conn, &first, "widget:w2", &stack.key()).unwrap();
        assert_eq!(more.id, stack.id);
        assert_eq!(more.widgets, keys(&["w1", "w3", "w2"]));
        assert_eq!(more.top, 2);
        assert_eq!(
            list(&conn).unwrap()[0].items,
            keys(&["app:a1", &stack.key(), "app:a2"])
        );

        // Neither a program nor the stack on itself.
        assert_eq!(
            stack_with(&mut conn, &first, "app:a1", &stack.key()),
            Err(WorkspaceError::NotFound)
        );
        assert_eq!(
            stack_with(&mut conn, &first, &stack.key(), &stack.key()),
            Err(WorkspaceError::NotFound)
        );
    }

    #[test]
    fn a_copy_comes_right_after_with_the_same_items_and_its_own_spacers_and_stacks() {
        let (mut conn, first) = with_three_widgets();
        let other = create(&mut conn, None).unwrap();
        let stack = stack_with(&mut conn, &first, "widget:w3", "widget:w2").unwrap();
        let spacer = add_spacer(&mut conn, &first, SpacerSize::Large, Some("app:a1")).unwrap();

        let copy = duplicate(&mut conn, &first, Some("Principal (cópia)")).unwrap();
        let all = list(&conn).unwrap();
        assert_eq!(
            all.iter().map(|w| w.id.as_str()).collect::<Vec<_>>(),
            vec![first.as_str(), copy.id.as_str(), other.id.as_str()]
        );
        assert_eq!(copy.name, "Principal (cópia)");
        assert_ne!(copy.color, all[0].color);
        assert_ne!(copy.color, other.color);

        // The same programs and widgets, in the same places; a new spacer
        // of the same size and a new stack with the same widgets.
        assert_eq!(copy.items.len(), all[0].items.len());
        assert_eq!(copy.items[0], "app:a1");
        assert_eq!(copy.items[2], "widget:w1");
        assert_eq!(copy.spacers.len(), 1);
        assert_ne!(copy.spacers[0].id, spacer.id);
        assert_eq!(copy.spacers[0].size, SpacerSize::Large);
        assert_eq!(copy.items[1], copy.spacers[0].key());
        assert_eq!(copy.stacks.len(), 1);
        assert_ne!(copy.stacks[0].id, stack.id);
        assert_eq!(copy.stacks[0].widgets, stack.widgets);
        assert_eq!(copy.stacks[0].top, stack.top);
        assert_eq!(copy.items[3], copy.stacks[0].key());

        // Changing the copy leaves the first as it was.
        turn_stack(&mut conn, &copy.stacks[0].id, 1).unwrap();
        remove_spacer(&mut conn, &copy.spacers[0].id).unwrap();
        let all = list(&conn).unwrap();
        assert_eq!(all[0].stacks[0].top, stack.top);
        assert_eq!(all[0].spacers, vec![spacer]);
        assert!(all[1].spacers.is_empty());

        // Deleting the copy takes nothing the first still shows.
        assert!(delete(&mut conn, &copy.id).unwrap().is_empty());
        let all = list(&conn).unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].stacks[0].widgets, stack.widgets);
        assert!(held(&conn, "widget:w3").unwrap());
    }

    #[test]
    fn a_copy_without_a_name_takes_the_next_and_stops_at_the_limit() {
        let (mut conn, first) = with_three_widgets();
        let copy = duplicate(&mut conn, &first, None).unwrap();
        assert_eq!(copy.name, "Espaço 2");
        assert_eq!(
            duplicate(&mut conn, &first, Some(" bad")),
            Err(WorkspaceError::InvalidName)
        );
        assert_eq!(
            duplicate(&mut conn, "nope", None),
            Err(WorkspaceError::NotFound)
        );
        while list(&conn).unwrap().len() < MAX_WORKSPACES {
            duplicate(&mut conn, &first, None).unwrap();
        }
        assert_eq!(
            duplicate(&mut conn, &first, None),
            Err(WorkspaceError::LimitReached)
        );
    }

    #[test]
    fn a_stack_turns_round_and_gives_its_top_back() {
        let (mut conn, first) = with_three_widgets();
        let stack = stack_with(&mut conn, &first, "widget:w2", "widget:w1").unwrap();
        stack_with(&mut conn, &first, "widget:w3", &stack.key()).unwrap();

        turn_stack(&mut conn, &stack.id, 1).unwrap();
        assert_eq!(list(&conn).unwrap()[0].stacks[0].top, 0);
        turn_stack(&mut conn, &stack.id, -1).unwrap();
        assert_eq!(list(&conn).unwrap()[0].stacks[0].top, 2);

        // The one on top (w3) goes loose right after the stack.
        take_top(&mut conn, &stack.id).unwrap();
        let all = list(&conn).unwrap();
        assert_eq!(
            all[0].items,
            keys(&["app:a1", &stack.key(), "widget:w3", "app:a2"])
        );
        assert_eq!(all[0].stacks[0].widgets, keys(&["w1", "w2"]));
        assert_eq!(all[0].stacks[0].top, 1);

        // With one left, the stack gives its place to it.
        take_top(&mut conn, &stack.id).unwrap();
        let all = list(&conn).unwrap();
        assert_eq!(
            all[0].items,
            keys(&["app:a1", "widget:w1", "widget:w2", "widget:w3", "app:a2"])
        );
        assert!(all[0].stacks.is_empty());
        assert_eq!(
            take_top(&mut conn, &stack.id),
            Err(WorkspaceError::NotFound)
        );
    }

    #[test]
    fn a_stack_undone_or_emptied_leaves_its_widgets_in_its_place() {
        let (mut conn, first) = with_three_widgets();
        let stack = stack_with(&mut conn, &first, "widget:w3", "widget:w1").unwrap();
        stack_with(&mut conn, &first, "widget:w2", &stack.key()).unwrap();

        // Removed from the workspace (not loose): the others stay stacked.
        assert!(leave_stacks(&mut conn, &first, "w3").unwrap());
        assert!(!held(&conn, "widget:w3").unwrap());
        assert_eq!(
            list(&conn).unwrap()[0].stacks[0].widgets,
            keys(&["w1", "w2"])
        );
        assert!(!leave_stacks(&mut conn, &first, "w3").unwrap());

        dissolve_stack(&mut conn, &stack.id).unwrap();
        let all = list(&conn).unwrap();
        assert_eq!(
            all[0].items,
            keys(&["app:a1", "widget:w1", "widget:w2", "app:a2"])
        );
        assert!(all[0].stacks.is_empty());

        // A widget of a stack removed for good: the stack of two undoes.
        let again = stack_with(&mut conn, &first, "widget:w2", "widget:w1").unwrap();
        conn.execute("DELETE FROM widget_instances WHERE id = 'w2'", [])
            .unwrap();
        forget_item(&mut conn, "widget:w2").unwrap();
        let all = list(&conn).unwrap();
        assert_eq!(all[0].items, keys(&["app:a1", "widget:w1", "app:a2"]));
        assert!(all[0].stacks.is_empty());
        assert_eq!(
            dissolve_stack(&mut conn, &again.id),
            Err(WorkspaceError::NotFound)
        );
    }

    #[test]
    fn deleting_a_workspace_leaves_the_widgets_of_its_stacks_behind() {
        let (mut conn, first) = with_three_widgets();
        let second = create(&mut conn, None).unwrap().id;
        add_item(&mut conn, &second, "widget:w1").unwrap();
        add_item(&mut conn, &second, "widget:w2").unwrap();
        stack_with(&mut conn, &second, "widget:w2", "widget:w1").unwrap();
        // w2 only in the second workspace's stack.
        remove_item(&mut conn, &first, "widget:w2").unwrap();

        let orphans = delete(&mut conn, &second).unwrap();

        assert_eq!(orphans, keys(&["widget:w2"]));
        let left: i64 = conn
            .query_row("SELECT COUNT(*) FROM stacks", [], |row| row.get(0))
            .unwrap();
        assert_eq!(left, 0);
    }

    #[test]
    fn the_first_workspace_holds_what_the_dock_showed() {
        let mut conn = database();
        ensure_first(&mut conn, &keys(&["widget:w1", "app:a1"])).unwrap();
        // Once only.
        ensure_first(&mut conn, &[]).unwrap();

        let all = list(&conn).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].name, "Principal");
        assert_eq!(all[0].color, PALETTE[0]);
        assert_eq!(all[0].items, keys(&["widget:w1", "app:a1", "app:a2"]));
    }

    #[test]
    fn workspaces_are_made_renamed_recolored_and_moved() {
        let mut conn = database();
        ensure_first(&mut conn, &[]).unwrap();
        let second = create(&mut conn, None).unwrap();
        assert_eq!(second.name, "Espaço 2");
        assert_eq!(second.color, PALETTE[1]);
        let third = create(&mut conn, Some("Foco")).unwrap();

        rename(&mut conn, &second.id, "Trabalho").unwrap();
        recolor(&mut conn, &third.id, "#30d158").unwrap();
        shift(&mut conn, &third.id, true).unwrap();

        let all = list(&conn).unwrap();
        let names: Vec<&str> = all.iter().map(|w| w.name.as_str()).collect();
        assert_eq!(names, ["Principal", "Foco", "Trabalho"]);
        assert_eq!(all[1].color, "#30D158");
        assert_eq!(
            rename(&mut conn, "nope", "X"),
            Err(WorkspaceError::NotFound)
        );
        assert_eq!(
            rename(&mut conn, &third.id, " X"),
            Err(WorkspaceError::InvalidName)
        );
        assert_eq!(
            recolor(&mut conn, &third.id, "#123456"),
            Err(WorkspaceError::InvalidColor)
        );
    }

    #[test]
    fn no_more_than_nine() {
        let mut conn = database();
        ensure_first(&mut conn, &[]).unwrap();
        for _ in 1..MAX_WORKSPACES {
            create(&mut conn, None).unwrap();
        }
        assert_eq!(create(&mut conn, None), Err(WorkspaceError::LimitReached));
    }

    #[test]
    fn an_item_in_two_workspaces_stays_until_the_last_lets_it_go() {
        let mut conn = database();
        ensure_first(&mut conn, &[]).unwrap();
        let first = list(&conn).unwrap()[0].id.clone();
        let second = create(&mut conn, None).unwrap().id;
        add_item(&mut conn, &second, "widget:w1").unwrap();
        add_item(&mut conn, &second, "widget:w1").unwrap();
        assert_eq!(list(&conn).unwrap()[1].items, keys(&["widget:w1"]));

        assert!(remove_item(&mut conn, &first, "widget:w1").unwrap());
        assert!(!remove_item(&mut conn, &second, "widget:w1").unwrap());
        assert!(!held(&conn, "widget:w1").unwrap());
    }

    #[test]
    fn deleting_a_workspace_tells_what_no_other_has() {
        let mut conn = database();
        ensure_first(&mut conn, &[]).unwrap();
        let first = list(&conn).unwrap()[0].id.clone();
        let second = create(&mut conn, None).unwrap().id;
        add_item(&mut conn, &second, "app:a1").unwrap();
        set_active(&mut conn, r"\\.\DISPLAY1", &second).unwrap();

        let orphans = delete(&mut conn, &first).unwrap();
        assert_eq!(orphans, keys(&["app:a2", "widget:w1"]));
        assert_eq!(
            delete(&mut conn, &second),
            Err(WorkspaceError::LastWorkspace)
        );
        // The monitor that showed the second one still does; one that showed
        // the deleted one goes to the first.
        assert_eq!(
            active(&conn, r"\\.\DISPLAY1").unwrap(),
            Some(second.clone())
        );
        assert_eq!(active(&conn, r"\\.\DISPLAY2").unwrap(), Some(second));
    }

    #[test]
    fn the_drag_reorders_only_what_the_workspace_has() {
        let mut conn = database();
        ensure_first(&mut conn, &[]).unwrap();
        let first = list(&conn).unwrap()[0].id.clone();
        reorder(&mut conn, &first, &keys(&["widget:w1", "app:zz", "app:a2"])).unwrap();
        assert_eq!(
            list(&conn).unwrap()[0].items,
            keys(&["widget:w1", "app:a2", "app:a1"])
        );
    }

    #[test]
    fn a_removed_item_leaves_every_workspace() {
        let mut conn = database();
        ensure_first(&mut conn, &[]).unwrap();
        let second = create(&mut conn, None).unwrap().id;
        add_item(&mut conn, &second, "app:a1").unwrap();
        forget_item(&mut conn, "app:a1").unwrap();
        assert!(!held(&conn, "app:a1").unwrap());
    }

    #[test]
    fn a_spacer_goes_after_the_item_or_at_the_end_and_keeps_its_size() {
        let mut conn = database();
        ensure_first(&mut conn, &[]).unwrap();
        let first = list(&conn).unwrap()[0].id.clone();

        let small = add_spacer(&mut conn, &first, SpacerSize::Small, Some("app:a1")).unwrap();
        let large = add_spacer(&mut conn, &first, SpacerSize::Large, None).unwrap();

        let all = list(&conn).unwrap();
        assert_eq!(
            all[0].items,
            vec![
                "app:a1".to_owned(),
                small.key(),
                "app:a2".to_owned(),
                "widget:w1".to_owned(),
                large.key(),
            ]
        );
        assert_eq!(all[0].spacers, vec![small.clone(), large.clone()]);
        assert_eq!(
            add_spacer(&mut conn, "nope", SpacerSize::Small, None),
            Err(WorkspaceError::NotFound)
        );

        // Dragged before the first program, it stays a spacer of its size.
        let order = vec![large.key(), "app:a1".to_owned()];
        reorder(&mut conn, &first, &order).unwrap();
        let all = list(&conn).unwrap();
        assert_eq!(all[0].items[0], large.key());
        assert_eq!(all[0].spacers, vec![large.clone(), small.clone()]);

        remove_spacer(&mut conn, &small.id).unwrap();
        let all = list(&conn).unwrap();
        assert_eq!(all[0].spacers, vec![large]);
        assert!(!held(&conn, &small.key()).unwrap());
        assert_eq!(
            remove_spacer(&mut conn, &small.id),
            Err(WorkspaceError::NotFound)
        );
    }

    #[test]
    fn spacers_are_per_workspace() {
        let mut conn = database();
        ensure_first(&mut conn, &[]).unwrap();
        let second = create(&mut conn, None).unwrap().id;
        let spacer = add_spacer(&mut conn, &second, SpacerSize::Large, None).unwrap();

        let all = list(&conn).unwrap();
        assert!(all[0].spacers.is_empty());
        assert_eq!(all[1].items, vec![spacer.key()]);
        assert_eq!(all[1].spacers, vec![spacer]);
    }

    #[test]
    fn each_monitor_keeps_its_workspace() {
        let mut conn = database();
        ensure_first(&mut conn, &[]).unwrap();
        let first = list(&conn).unwrap()[0].id.clone();
        let second = create(&mut conn, None).unwrap().id;
        assert_eq!(active(&conn, "A").unwrap(), Some(first.clone()));
        set_active(&mut conn, "A", &second).unwrap();
        assert_eq!(active(&conn, "A").unwrap(), Some(second));
        assert_eq!(active(&conn, "B").unwrap(), Some(first));
        assert_eq!(
            set_active(&mut conn, "A", "nope"),
            Err(WorkspaceError::NotFound)
        );
    }

    #[test]
    fn the_workspace_in_use_starts_as_the_first() {
        let mut conn = database();
        ensure_first(&mut conn, &[]).unwrap();
        let first = list(&conn).unwrap()[0].id.clone();
        let second = create(&mut conn, None).unwrap().id;
        assert_eq!(in_use(&conn).unwrap(), Some(first));
        assert!(set_in_use(&mut conn, &second).unwrap());
        assert!(!set_in_use(&mut conn, &second).unwrap());
        assert_eq!(in_use(&conn).unwrap(), Some(second));
    }

    #[test]
    fn a_monitor_can_be_held_on_its_workspace() {
        let mut conn = database();
        assert!(!fixed(&conn, "M1").unwrap());

        set_fixed(&mut conn, "M1", true).unwrap();
        assert!(fixed(&conn, "M1").unwrap());
        assert!(!fixed(&conn, "M2").unwrap());

        set_fixed(&mut conn, "M1", false).unwrap();
        assert!(!fixed(&conn, "M1").unwrap());
    }

    #[test]
    fn a_monitor_known_by_a_new_key_keeps_its_workspace_and_hold() {
        let mut conn = database();
        let second = create(&mut conn, Some("Pessoal")).unwrap();
        set_active(&mut conn, r"\\.\DISPLAY2", &second.id).unwrap();
        set_fixed(&mut conn, r"\\.\DISPLAY2", true).unwrap();

        adopt_monitor_key(&mut conn, r"\\.\DISPLAY2", "path-of-dell").unwrap();

        assert_eq!(
            active(&conn, "path-of-dell").unwrap(),
            Some(second.id.clone())
        );
        assert!(fixed(&conn, "path-of-dell").unwrap());
        assert!(!fixed(&conn, r"\\.\DISPLAY2").unwrap());
        // Done again (the next start), nothing changes.
        adopt_monitor_key(&mut conn, r"\\.\DISPLAY2", "path-of-dell").unwrap();
        assert_eq!(active(&conn, "path-of-dell").unwrap(), Some(second.id));
    }
}
