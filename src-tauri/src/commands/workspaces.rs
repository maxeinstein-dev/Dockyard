//! The workspaces (spec 0013): what Configurações reads and changes, and the
//! dock's switch from one to another.
//!
//! The programs and the widgets of a workspace are changed with the apps and
//! widgets commands, which take the workspace (`workspace`); these commands
//! change the workspaces themselves, and their spacers (spec 0022), which
//! belong to no other list.

use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::commands::widgets::lock;
use crate::domain::widgets::MoveDirection;
use crate::domain::workspaces::{
    MAX_WORKSPACES, SPACER_PREFIX, SpacerSize, Workspace, WorkspaceError, WorkspaceErrorCode,
    neighbor,
};
use crate::persistence::database::Storage;

/// Event sent to every window with the full [`WorkspacesSnapshot`] after a
/// change.
pub const CHANGED_EVENT: &str = "workspaces-changed";

/// The workspaces in order, and the one in use (the dock used last).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspacesSnapshot {
    pub workspaces: Vec<Workspace>,
    pub in_use: Option<String>,
}

/// How a workspace command fails: `{ "code": "lastWorkspace" }`, matching
/// `WorkspaceCommandError` in `src/core/workspaces.ts`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceCommandError {
    code: WorkspaceErrorCode,
}

impl From<WorkspaceError> for WorkspaceCommandError {
    fn from(error: WorkspaceError) -> Self {
        WorkspaceCommandError { code: error.code() }
    }
}

fn read(storage: &mut Storage) -> Result<WorkspacesSnapshot, WorkspaceError> {
    let workspaces = storage.workspaces()?;
    let in_use = storage.in_use().ok().map(|workspace| workspace.id);
    Ok(WorkspacesSnapshot { workspaces, in_use })
}

/// Tells every window how the workspaces are now (after the dock changed
/// one: a drag, a switch).
pub fn notify(app: &AppHandle) {
    let state = app.state::<Mutex<Storage>>();
    let snapshot = read(&mut lock(&state));
    match snapshot {
        Ok(snapshot) => {
            if let Err(error) = app.emit(CHANGED_EVENT, &snapshot) {
                log::warn!("workspaces: failed to notify the windows: {error}");
            }
        }
        Err(error) => log::warn!("workspaces: could not be read: {error}"),
    }
}

/// Runs `change` and, if it was accepted, tells the windows and the docks.
/// Deleting a workspace may remove programs and widgets for good: their
/// lists are sent again too.
fn change<T>(
    app: &AppHandle,
    state: &State<'_, Mutex<Storage>>,
    what: &str,
    change: impl FnOnce(&mut Storage) -> Result<T, WorkspaceError>,
) -> Result<T, WorkspaceCommandError> {
    let value = change(&mut lock(state)).inspect_err(|error| {
        log::warn!("workspaces: {what} rejected: {error}");
    })?;
    notify(app);
    let (apps, widgets) = {
        let storage = lock(state);
        (storage.apps_snapshot().ok(), storage.snapshot().ok())
    };
    if let Some(apps) = apps {
        let _ = app.emit(crate::commands::apps::CHANGED_EVENT, &apps);
    }
    if let Some(widgets) = widgets {
        let _ = app.emit(crate::commands::widgets::CHANGED_EVENT, &widgets);
    }
    crate::windows::dock::refresh();
    Ok(value)
}

#[tauri::command]
pub fn list_workspaces(
    state: State<'_, Mutex<Storage>>,
) -> Result<WorkspacesSnapshot, WorkspaceCommandError> {
    read(&mut lock(&state)).map_err(|error| {
        log::warn!("workspaces: list failed: {error}");
        WorkspaceCommandError::from(error)
    })
}

/// Makes a workspace at the end: named `name`, or the next "Espaço N".
#[tauri::command]
pub fn create_workspace(
    app: AppHandle,
    state: State<'_, Mutex<Storage>>,
    name: Option<String>,
) -> Result<Workspace, WorkspaceCommandError> {
    change(&app, &state, "create", |storage| {
        storage.create_workspace(name.as_deref())
    })
}

/// Makes a copy of the workspace `id` right after it (F7 of estudo 0002),
/// named `name` or the next "Espaço N": the same programs and widgets, its
/// own spacers and stacks.
#[tauri::command]
pub fn duplicate_workspace(
    app: AppHandle,
    state: State<'_, Mutex<Storage>>,
    id: String,
    name: Option<String>,
) -> Result<Workspace, WorkspaceCommandError> {
    change(&app, &state, "duplicate", |storage| {
        storage.duplicate_workspace(&id, name.as_deref())
    })
}

#[tauri::command]
pub fn rename_workspace(
    app: AppHandle,
    state: State<'_, Mutex<Storage>>,
    id: String,
    name: String,
) -> Result<(), WorkspaceCommandError> {
    change(&app, &state, "rename", |storage| {
        storage.rename_workspace(&id, &name)
    })
}

#[tauri::command]
pub fn recolor_workspace(
    app: AppHandle,
    state: State<'_, Mutex<Storage>>,
    id: String,
    color: String,
) -> Result<(), WorkspaceCommandError> {
    change(&app, &state, "recolor", |storage| {
        storage.recolor_workspace(&id, &color)
    })
}

#[tauri::command]
pub fn move_workspace(
    app: AppHandle,
    state: State<'_, Mutex<Storage>>,
    id: String,
    direction: MoveDirection,
) -> Result<(), WorkspaceCommandError> {
    change(&app, &state, "move", |storage| {
        storage.shift_workspace(&id, direction == MoveDirection::Up)
    })
}

/// Deletes a workspace (never the last one); the programs and widgets no
/// other workspace has go with it.
#[tauri::command]
pub fn delete_workspace(
    app: AppHandle,
    state: State<'_, Mutex<Storage>>,
    id: String,
) -> Result<(), WorkspaceCommandError> {
    change(&app, &state, "delete", |storage| {
        storage.delete_workspace(&id)
    })?;
    // Its hidden windows come back, and join what their monitor shows.
    #[cfg(windows)]
    {
        crate::windows::window_spaces::forget_workspace(&id);
        crate::windows::open_apps::wake(&app);
    }
    Ok(())
}

/// The monitors a switch on `monitor` reaches: every one with "Trocar em
/// todos os monitores" (spec 0013 C3), `monitor` first; else `monitor`.
fn reached(app: &AppHandle, monitor: &str) -> Vec<String> {
    let mut monitors = vec![monitor.to_owned()];
    #[cfg(windows)]
    if crate::commands::settings::current(app)
        .dock
        .switch_every_monitor
    {
        monitors.extend(
            crate::windows::monitors::all()
                .into_iter()
                .map(|other| other.key)
                .filter(|key| key != monitor),
        );
    }
    #[cfg(not(windows))]
    let _ = app;
    monitors
}

/// The dock on the monitor `monitor` (its device name) shows the workspace
/// `id`, which is now in use; with "Trocar em todos os monitores", every
/// monitor does.
pub fn switch(app: &AppHandle, monitor: &str, id: &str) {
    let state = app.state::<Mutex<Storage>>();
    let monitors = reached(app, monitor);
    let from: Vec<Option<String>> = {
        let mut storage = lock(&state);
        monitors
            .iter()
            .map(|monitor| storage.shown_on(monitor).ok().map(|w| w.id))
            .collect()
    };
    let switched = change(app, &state, "switch", |storage| {
        // The monitor of the dock last: it is the one in use.
        for other in monitors.iter().skip(1) {
            storage.switch_workspace(other, id)?;
        }
        storage.switch_workspace(monitor, id)
    });
    // The windows follow (spec 0013 C2): those of the workspace left are
    // parked, those of the one shown come back, the top one of the monitor
    // that switched in front.
    #[cfg(windows)]
    if switched.is_ok() {
        let mut front = None;
        for (at, (monitor, from)) in monitors.iter().zip(from).enumerate() {
            let Some(from) = from.filter(|from| from != id) else {
                continue;
            };
            let top = crate::windows::window_spaces::switch(monitor, &from, id);
            if at == 0 {
                front = top;
            }
        }
        if let Some(top) = front {
            crate::windows::openwin::focus(top);
        }
        crate::windows::open_apps::wake(app);
    }
    #[cfg(not(windows))]
    let _ = (switched, from);
}

/// Every monitor parks the windows that are not of the workspace it shows
/// and brings back the ones of it (after windows changed workspace).
#[cfg(windows)]
fn settle(app: &AppHandle) {
    let state = app.state::<Mutex<Storage>>();
    let shown: Vec<(String, String)> = {
        let mut storage = lock(&state);
        crate::windows::monitors::all()
            .into_iter()
            .filter_map(|monitor| {
                let workspace = storage.shown_on(&monitor.key).ok()?;
                Some((monitor.key, workspace.id))
            })
            .collect()
    };
    for (monitor, workspace) in &shown {
        crate::windows::window_spaces::switch(monitor, workspace, workspace);
    }
    crate::windows::open_apps::wake(app);
}

/// The windows `hwnds` go to the workspace `target`, which must exist: they
/// leave the screen unless their monitor shows it.
#[cfg(windows)]
fn give_windows(
    app: &AppHandle,
    hwnds: &[isize],
    target: &str,
) -> Result<usize, WorkspaceCommandError> {
    let state = app.state::<Mutex<Storage>>();
    let exists = lock(&state)
        .workspaces()?
        .iter()
        .any(|workspace| workspace.id == target);
    if !exists {
        return Err(WorkspaceError::NotFound.into());
    }
    if !hwnds.is_empty() {
        crate::windows::window_spaces::give(hwnds, target);
        settle(app);
    }
    Ok(hwnds.len())
}

/// "Mover para <espaço>" in the menu of a pinned program (spec 0013 C3): its
/// windows in the workspace in use go to `workspace`. How many moved.
#[tauri::command]
pub fn move_app_windows(
    app: AppHandle,
    id: String,
    workspace: String,
) -> Result<usize, WorkspaceCommandError> {
    #[cfg(windows)]
    {
        let from = in_use_id(&app);
        let hwnds = crate::windows::open_apps::windows_of_pinned(&id, from.as_deref());
        give_windows(&app, &hwnds, &workspace)
    }
    #[cfg(not(windows))]
    {
        let _ = (app, id, workspace);
        Ok(0)
    }
}

/// The same for an open program that is not pinned, by its `key`.
#[tauri::command]
pub fn move_open_windows(
    app: AppHandle,
    key: String,
    workspace: String,
) -> Result<usize, WorkspaceCommandError> {
    #[cfg(windows)]
    {
        let from = in_use_id(&app);
        let hwnds = crate::windows::open_apps::windows_of_other(&key, from.as_deref());
        give_windows(&app, &hwnds, &workspace)
    }
    #[cfg(not(windows))]
    {
        let _ = (app, key, workspace);
        Ok(0)
    }
}

/// The id of the workspace in use.
#[cfg(windows)]
fn in_use_id(app: &AppHandle) -> Option<String> {
    let state = app.state::<Mutex<Storage>>();
    let mut storage = lock(&state);
    storage.in_use().ok().map(|workspace| workspace.id)
}

/// The dock on `monitor` goes to the workspace at `index` (the shortcut
/// `Ctrl+Alt+<index + 1>`); nothing when there is none there.
pub fn switch_to_index(app: &AppHandle, monitor: &str, index: usize) {
    let state = app.state::<Mutex<Storage>>();
    let id = lock(&state)
        .workspaces()
        .ok()
        .and_then(|all| all.into_iter().nth(index))
        .map(|workspace| workspace.id);
    if let Some(id) = id {
        switch(app, monitor, &id);
    }
}

/// The dock on `monitor` goes to the next workspace (`forward`) or the one
/// before, without going round (the mouse wheel over the dock).
pub fn switch_by(app: &AppHandle, monitor: &str, forward: bool) {
    let state = app.state::<Mutex<Storage>>();
    let target = {
        let mut storage = lock(&state);
        let all = storage.workspaces().unwrap_or_default();
        let current = storage.shown_on(monitor).ok().map(|w| w.id);
        current
            .and_then(|id| all.iter().position(|w| w.id == id))
            .and_then(|at| neighbor(all.len(), at, forward))
            .and_then(|at| all.get(at).map(|w| w.id.clone()))
    };
    if let Some(id) = target {
        switch(app, monitor, &id);
    }
}

/// Makes a workspace and shows it on `monitor` ("Novo espaço" in the dock's
/// menu); nothing when there are `MAX_WORKSPACES` already.
pub fn create_and_switch(app: &AppHandle, monitor: &str) {
    let state = app.state::<Mutex<Storage>>();
    let created = change(app, &state, "create", |storage| {
        storage.create_workspace(None)
    });
    if let Ok(workspace) = created {
        switch(app, monitor, &workspace.id);
    }
}

/// The workspace of the dock used last is the one in use: Configurações and
/// what is pinned from the dock go to it.
pub fn use_from_dock(app: &AppHandle, monitor: &str) {
    let state = app.state::<Mutex<Storage>>();
    let changed = {
        let mut storage = lock(&state);
        storage
            .shown_on(monitor)
            .and_then(|workspace| storage.use_workspace(&workspace.id))
            .unwrap_or(false)
    };
    if changed {
        notify(app);
    }
}

/// The workspace `id`, which must exist, or the one in use.
fn workspace_or_in_use(
    storage: &mut Storage,
    id: Option<&str>,
) -> Result<Workspace, WorkspaceError> {
    match id {
        Some(id) => storage
            .workspaces()?
            .into_iter()
            .find(|known| known.id == id)
            .ok_or(WorkspaceError::NotFound),
        None => storage.in_use(),
    }
}

/// The key of the spacer `id` of `workspace`; `NotFound` when it has none
/// such (an id from the interface is never trusted).
fn spacer_key(workspace: &Workspace, id: &str) -> Result<String, WorkspaceError> {
    workspace
        .spacers
        .iter()
        .find(|spacer| spacer.id == id)
        .map(|spacer| format!("{SPACER_PREFIX}{}", spacer.id))
        .ok_or(WorkspaceError::NotFound)
}

/// Makes a spacer of `size` in the workspace `workspace` (the one the dock
/// shows; the one in use without it), right after its item `after` or at
/// its end ("Adicionar espaçador" in the dock's menu, spec 0022). A refusal
/// is only logged.
pub fn add_spacer(app: &AppHandle, workspace: Option<&str>, size: SpacerSize, after: Option<&str>) {
    let state = app.state::<Mutex<Storage>>();
    // `change` logs a refusal.
    let _ = change(app, &state, "add spacer", |storage| {
        let id = workspace_or_in_use(storage, workspace)?.id;
        storage.add_spacer(&id, size, after)
    });
}

/// Removes the spacer `id` for good ("Remover espaçador" in its menu).
pub fn remove_spacer_from_dock(app: &AppHandle, id: &str) {
    let state = app.state::<Mutex<Storage>>();
    // `change` logs a refusal.
    let _ = change(app, &state, "remove spacer", |storage| {
        storage.remove_spacer(id)
    });
}

/// Removes the spacer `id` of the workspace `workspace` (or the one in
/// use) for good: Configurações › Itens do dock (spec 0022).
#[tauri::command]
pub fn remove_spacer(
    app: AppHandle,
    state: State<'_, Mutex<Storage>>,
    id: String,
    workspace: Option<String>,
) -> Result<(), WorkspaceCommandError> {
    change(&app, &state, "remove spacer", |storage| {
        let found = workspace_or_in_use(storage, workspace.as_deref())?;
        spacer_key(&found, &id)?;
        storage.remove_spacer(&id)
    })
}

/// Moves the spacer `id` one place up or down among the widgets and the
/// spacers of the workspace `workspace` (or the one in use), as Itens do
/// dock lists them; at either end nothing changes.
#[tauri::command]
pub fn move_spacer(
    app: AppHandle,
    state: State<'_, Mutex<Storage>>,
    id: String,
    direction: MoveDirection,
    workspace: Option<String>,
) -> Result<(), WorkspaceCommandError> {
    change(&app, &state, "move spacer", |storage| {
        let found = workspace_or_in_use(storage, workspace.as_deref())?;
        let key = spacer_key(&found, &id)?;
        storage.shift_in_workspace(&found.id, &key, direction == MoveDirection::Up)
    })
}

/// Puts the widget or the stack `moving` on the widget or the stack
/// `target` of the workspace `workspace` (the one the dock shows; the one
/// in use without it): "Empilhar com" in the dock's menu (spec 0023). A
/// refusal is only logged.
pub fn stack_from_dock(app: &AppHandle, workspace: Option<&str>, moving: &str, target: &str) {
    let state = app.state::<Mutex<Storage>>();
    // `change` logs a refusal.
    let _ = change(app, &state, "stack widgets", |storage| {
        let id = workspace_or_in_use(storage, workspace)?.id;
        storage.stack_with(&id, moving, target).map(|_| ())
    });
}

/// The widget on top of the stack `id` out of it, loose right after it
/// ("Tirar da pilha" in its menu).
pub fn take_from_stack(app: &AppHandle, id: &str) {
    let state = app.state::<Mutex<Storage>>();
    // `change` logs a refusal.
    let _ = change(app, &state, "take from stack", |storage| {
        storage.take_from_stack(id)
    });
}

/// Another widget on top of the stack `id`, `step` turns of the wheel away
/// (Ctrl and the wheel over it).
pub fn turn_stack(app: &AppHandle, id: &str, step: i32) {
    let state = app.state::<Mutex<Storage>>();
    // `change` logs a refusal.
    let _ = change(app, &state, "turn stack", |storage| {
        storage.turn_stack(id, step)
    });
}

/// The key of the stack `id` of `workspace`; `NotFound` when it has none
/// such.
fn stack_key(workspace: &Workspace, id: &str) -> Result<String, WorkspaceError> {
    workspace
        .stacks
        .iter()
        .find(|stack| stack.id == id)
        .map(|stack| stack.key())
        .ok_or(WorkspaceError::NotFound)
}

/// Moves the stack `id` one place up or down among the widgets, the
/// spacers and the stacks of the workspace `workspace` (or the one in use),
/// as Itens do dock lists them (spec 0023).
#[tauri::command]
pub fn move_stack(
    app: AppHandle,
    state: State<'_, Mutex<Storage>>,
    id: String,
    direction: MoveDirection,
    workspace: Option<String>,
) -> Result<(), WorkspaceCommandError> {
    change(&app, &state, "move stack", |storage| {
        let found = workspace_or_in_use(storage, workspace.as_deref())?;
        let key = stack_key(&found, &id)?;
        storage.shift_in_workspace(&found.id, &key, direction == MoveDirection::Up)
    })
}

/// Undoes the stack `id` of the workspace `workspace` (or the one in use):
/// its widgets loose in its place ("Desfazer pilha" in Itens do dock).
#[tauri::command]
pub fn dissolve_stack(
    app: AppHandle,
    state: State<'_, Mutex<Storage>>,
    id: String,
    workspace: Option<String>,
) -> Result<(), WorkspaceCommandError> {
    change(&app, &state, "dissolve stack", |storage| {
        let found = workspace_or_in_use(storage, workspace.as_deref())?;
        stack_key(&found, &id)?;
        storage.dissolve_stack(&id)
    })
}

/// Whether one more workspace can be made.
pub fn can_create(workspaces: &[Workspace]) -> bool {
    workspaces.len() < MAX_WORKSPACES
}
