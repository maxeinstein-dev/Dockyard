//! The dock's menu: what it offers and what each choice does.

use super::*;

/// What the dock's menu offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MenuChoice {
    TaskManager,
    Launcher,
    Settings,
    AutoHide,
    Quit,
    /// Show the workspace at this index (spec 0013).
    Workspace(usize),
    NewWorkspace,
    /// The widget clicked becomes this many icons wide (spec 0017 F1b).
    Width(u8),
    /// The widget clicked leaves the dock (of the workspace shown).
    RemoveWidget,
    /// Configurações › Geral, where the launcher's hotkey is (the
    /// Lançador's menu, spec 0012).
    SetHotkey,
}

/// One line of the dock's menu: what it does and how it shows (spec 0014
/// R3).
#[derive(Debug, Clone, PartialEq)]
pub(super) struct MenuEntry {
    pub(super) choice: Option<MenuChoice>,
    pub(super) line: menu::Line,
}

/// Glyphs of Windows' icon font for the dock's menu.
pub(super) const SEARCH_GLYPH: char = '\u{E721}';
pub(super) const TASKS_GLYPH: char = '\u{E9D9}';
pub(super) const ADD_GLYPH: char = '\u{E710}';
pub(super) const SETTINGS_GLYPH: char = '\u{E713}';
pub(super) const HIDE_GLYPH: char = '\u{E70D}';
pub(super) const QUIT_GLYPH: char = '\u{E7E8}';
pub(super) const REMOVE_GLYPH: char = '\u{E711}';
pub(super) const KEYBOARD_GLYPH: char = '\u{E765}';

impl MenuEntry {
    fn item(choice: MenuChoice, item: menu::Item) -> MenuEntry {
        MenuEntry {
            choice: Some(choice),
            line: menu::Line::Item(item),
        }
    }

    fn separator() -> MenuEntry {
        MenuEntry {
            choice: None,
            line: menu::Line::Separator,
        }
    }

    fn title(text: &str) -> MenuEntry {
        MenuEntry {
            choice: None,
            line: menu::Line::Title(text.to_owned()),
        }
    }
}

/// A workspace as the dock's menu lists it: its name, its color (`#RRGGBB`)
/// and whether it is the one shown.
pub(super) struct MenuWorkspace {
    pub(super) name: String,
    pub(super) color: String,
    pub(super) shown: bool,
}

/// The dock's menu, in order. `workspaces`: with two or more, they come
/// under a title, with their color and their shortcut, the shown one
/// checked; "Novo espaço" comes while one more can be made. The tray icon's
/// menu is this one with no workspaces.
pub(super) fn dock_menu_items(
    auto_hide: bool,
    workspaces: &[MenuWorkspace],
    can_create: bool,
) -> Vec<MenuEntry> {
    let text = &crate::i18n::messages().tray;
    let mut items = vec![
        MenuEntry::item(
            MenuChoice::Launcher,
            menu::Item::new(text.search_apps, SEARCH_GLYPH),
        ),
        MenuEntry::item(
            MenuChoice::TaskManager,
            menu::Item::new(text.task_manager, TASKS_GLYPH),
        ),
        MenuEntry::separator(),
    ];
    if workspaces.len() >= 2 {
        items.push(MenuEntry::title(text.workspaces));
        for (index, workspace) in workspaces.iter().enumerate() {
            let color = Color::hex(&workspace.color).unwrap_or(Color::WHITE);
            let rgb = |part: f32| (part * 255.0).round() as u8;
            items.push(MenuEntry::item(
                MenuChoice::Workspace(index),
                menu::Item::new(&workspace.name, SEARCH_GLYPH)
                    .swatch((rgb(color.r), rgb(color.g), rgb(color.b)))
                    .shortcut(format!("Ctrl+Alt+{}", index + 1))
                    .checked(workspace.shown),
            ));
        }
    }
    if can_create {
        items.push(MenuEntry::item(
            MenuChoice::NewWorkspace,
            menu::Item::new(text.new_workspace, ADD_GLYPH),
        ));
    }
    if workspaces.len() >= 2 || can_create {
        items.push(MenuEntry::separator());
    }
    items.extend([
        MenuEntry::item(
            MenuChoice::Settings,
            menu::Item::new(text.open_settings, SETTINGS_GLYPH),
        ),
        MenuEntry::item(
            MenuChoice::AutoHide,
            menu::Item::new(text.auto_hide, HIDE_GLYPH).checked(auto_hide),
        ),
        MenuEntry::separator(),
        MenuEntry::item(MenuChoice::Quit, menu::Item::new(text.quit, QUIT_GLYPH)),
    ]);
    items
}

/// Glyph of the widths in a widget's menu.
pub(super) const SIZE_GLYPH: char = '\u{E8A9}';

/// What the dock's menu starts with when it opens on a widget that takes
/// more than one width (spec 0017 F1b): the widths under a title, the one
/// in use checked, then a separator. Nothing for the other widgets.
pub(super) fn width_items(widths: std::ops::RangeInclusive<u8>, current: u8) -> Vec<MenuEntry> {
    if widths.start() == widths.end() {
        return Vec::new();
    }
    let text = &crate::i18n::messages().tray;
    let mut items = vec![MenuEntry::title(text.size)];
    for width in widths {
        let label = if width == 1 {
            text.one_icon.to_owned()
        } else {
            text.icons.replace("{n}", &width.to_string())
        };
        items.push(MenuEntry::item(
            MenuChoice::Width(width),
            menu::Item::new(&label, SIZE_GLYPH).checked(width == current),
        ));
    }
    items.push(MenuEntry::separator());
    items
}

/// What a widget's menu has before the dock's: its widths, when it takes
/// more than one; the Lançador's "Configurar atalho…" (spec 0012); then
/// "Remover do dock".
pub(super) fn widget_items(config: &WidgetConfig) -> Vec<MenuEntry> {
    let text = &crate::i18n::messages().tray;
    let mut items = width_items(config.widths(), config.width());
    if matches!(config, WidgetConfig::Launcher(_)) {
        items.push(MenuEntry::item(
            MenuChoice::SetHotkey,
            menu::Item::new(text.set_hotkey, KEYBOARD_GLYPH),
        ));
    }
    items.push(MenuEntry::item(
        MenuChoice::RemoveWidget,
        menu::Item::new(text.remove_widget, REMOVE_GLYPH),
    ));
    items.push(MenuEntry::separator());
    items
}

/// Shows the menu `items` at the pointer, opening away from the screen edge,
/// and returns what was chosen. The pointer resting on Configurações warms
/// WebView2 for it (`settings_window::warm`), as an item that opens a panel
/// does (H2).
/// From the keyboard (`from_keys`), it opens by the item, its first line
/// lit, for the dock's window.
pub(super) fn show_dock_menu(
    app: &AppHandle,
    edge: DockEdge,
    items: Vec<MenuEntry>,
    from_keys: Option<(HWND, POINT)>,
) -> Option<MenuChoice> {
    let (choices, lines): (Vec<_>, Vec<_>) = items
        .into_iter()
        .map(|entry| (entry.choice, entry.line))
        .unzip();
    let settings = choices
        .iter()
        .position(|choice| *choice == Some(MenuChoice::Settings));
    let app = app.clone();
    let (owner, at) = from_keys.map_or((None, None), |(owner, at)| (Some(owner), Some(at)));
    let chosen = menu::show_hovering(
        owner,
        at,
        edge == DockEdge::Bottom,
        lines,
        from_keys.is_some(),
        move |index| {
            if Some(index) == settings {
                super::super::settings_window::warm(&app);
            }
        },
    )?;
    choices.get(chosen).copied().flatten()
}

/// Has the tray icon checked against the taskbar's theme and the primary
/// monitor's scale, on the main thread: a dock gets the system's broadcasts
/// (every dock does, so the check sets the icon only when it changed).
pub(super) fn check_tray_icon() {
    if let Some(app) = APP.get() {
        crate::tray::refresh_soon(app);
    }
}

/// The tray icon's menu (spec 0014 R3: the Dockyard's own, as the dock's),
/// opened at the pointer: what the dock's has, without the workspaces.
pub fn tray_menu(app: &AppHandle) {
    let auto_hide = crate::commands::settings::current(app).dock.auto_hide;
    // The taskbar is at the bottom of the screen, as a dock most often is.
    let chosen = show_dock_menu(
        app,
        DockEdge::Bottom,
        dock_menu_items(auto_hide, &[], false),
        None,
    );
    if chosen == Some(MenuChoice::Settings) {
        // From the tray, the dock comes up too if it is hiding.
        show();
    }
    if let Some(choice) = chosen {
        act(app, choice, "", &[], None, None);
    }
}

/// Does what was chosen in the dock's menu (or the tray icon's): `monitor`
/// and `ids` are where the workspaces switch and which they are; `widget`
/// is the widget the menu opened on, if any.
pub(super) fn act(
    app: &AppHandle,
    choice: MenuChoice,
    monitor: &str,
    ids: &[String],
    widget: Option<&crate::domain::widgets::WidgetId>,
    workspace: Option<&str>,
) {
    match choice {
        MenuChoice::TaskManager => {
            if let Err(error) = super::super::apps::open_task_manager() {
                log::warn!("dock: the Task Manager could not be opened: {error}");
            }
        }
        MenuChoice::Settings => {
            if let Err(error) = super::super::settings_window::open(app) {
                log::warn!("dock: the settings could not be opened: {error}");
            }
        }
        MenuChoice::Launcher => super::super::launcher::toggle(),
        MenuChoice::AutoHide => {
            let changed = crate::commands::settings::change(app, |settings| {
                settings.dock.auto_hide = !settings.dock.auto_hide;
            });
            if let Err(error) = changed {
                log::warn!("dock: hiding on its own could not be changed: {error}");
            }
        }
        MenuChoice::Quit => app.exit(0),
        MenuChoice::Workspace(index) => {
            if let Some(id) = ids.get(index) {
                crate::commands::workspaces::switch(app, monitor, id);
            }
        }
        MenuChoice::NewWorkspace => {
            crate::commands::workspaces::create_and_switch(app, monitor);
        }
        MenuChoice::Width(width) => {
            if let Some(id) = widget {
                crate::commands::widgets::resize(app, id, width);
            }
        }
        MenuChoice::RemoveWidget => {
            if let Some(id) = widget {
                crate::commands::widgets::remove_from_dock(app, id, workspace);
            }
        }
        MenuChoice::SetHotkey => {
            let general = crate::domain::launcher_pages::Section::General;
            if let Err(error) = super::super::settings_window::open_section(app, general) {
                log::warn!("dock: the settings could not be opened: {error}");
            }
        }
    }
}
