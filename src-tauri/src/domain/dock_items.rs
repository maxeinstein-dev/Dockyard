//! What the dock shows, in order (spec 0011 H3; the page's `dockEntries`):
//! the pinned programs and the widgets, mixed as the user dragged them, the
//! open programs that are not pinned, then the system items. A separator
//! stands between the groups that have items.
//!
//! Each kind keeps its own order (its table, what Configurações reorders);
//! the dock's order (`Storage::dock_order`) only says where each kind goes,
//! so moving a program up in Configurações moves it on the dock too.
//!
//! Configurações and Sobre left the dock (author's decision, 2026-10-26):
//! the app opens from the dock's menu (right button) and has Sobre inside.
//! Configurações comes back only while it carries the attention badge, so
//! the notice is not lost. The tray's arrow is not an item either: it is in
//! the strip of small icons at the end (`dock_layout::Shape::tray`).
//!
//! The spacers of the workspace shown (spec 0022) are items of the first
//! group too, where it puts them: room with nothing in it.

use super::apps::{AppsSnapshot, PinnedApp};
use super::open_apps::{OpenOther, OpenSnapshot};
use super::widgets::{StorageStatus, WidgetInstance, WidgetKind, WidgetsSnapshot};
use super::workspaces::{Spacer, SpacerSize, Workspace};

/// The items Dockyard itself puts at the end of the dock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemItem {
    /// Only while the storage needs attention (`needs_attention`).
    Settings,
}

/// What an item of the dock stands for.
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    /// A pinned program: a click opens it, the right button its menu.
    App(PinnedApp),
    /// A widget: a click opens its panel.
    Widget(WidgetInstance),
    /// An open program that is not pinned.
    Other(OpenOther),
    System(SystemItem),
    /// Room between two items (spec 0022): nothing under the pointer, no
    /// label, no mark; it is dragged and has a menu of its own.
    Spacer(Spacer),
}

impl Item {
    /// A stable name for the item across changes of the list: the key of its
    /// panel (`app:<id>`, `widget:<id>`, `open:<key>`, or the system item's).
    pub fn key(&self) -> String {
        match self {
            Item::App(app) => format!("app:{}", app.id.as_str()),
            Item::Widget(widget) => format!("widget:{}", widget.id.as_str()),
            Item::Other(other) => format!("open:{}", other.key),
            Item::System(SystemItem::Settings) => "settings".into(),
            Item::Spacer(spacer) => spacer.key(),
        }
    }

    /// How many icons wide the item is in the dock: a widget says; the rest
    /// are one.
    pub fn span(&self) -> usize {
        match self {
            Item::Widget(widget) => widget.config.span(),
            _ => 1,
        }
    }

    /// Whether the item stands for a program (pinned or open).
    pub fn is_program(&self) -> bool {
        matches!(self, Item::App(_) | Item::Other(_))
    }

    /// The size of a spacer; `None` for any other item.
    pub fn spacer_size(&self) -> Option<SpacerSize> {
        match self {
            Item::Spacer(spacer) => Some(spacer.size),
            _ => None,
        }
    }
}

/// The texts the names come from.
#[derive(Debug, Clone, Copy)]
pub struct Names {
    pub settings: &'static str,
    pub clock: &'static str,
    pub time_progress: &'static str,
    pub note: &'static str,
    pub shortcut: &'static str,
    pub files: &'static str,
    pub activity: &'static str,
    pub network: &'static str,
    pub calendar: &'static str,
    pub cycle: &'static str,
    pub countdown: &'static str,
    pub pomodoro: &'static str,
    pub todo: &'static str,
    pub water: &'static str,
    pub media: &'static str,
    pub claude: &'static str,
    pub weather: &'static str,
    pub launcher: &'static str,
}

impl Names {
    /// The item's name: its label, never the time (spec 0003 §2.3 item 4).
    pub fn of(&self, item: &Item) -> String {
        match item {
            Item::App(app) => app.name.clone(),
            Item::Other(other) => other.name.clone(),
            Item::Widget(widget) => widget.label.clone().unwrap_or_else(|| {
                match widget.config.kind() {
                    WidgetKind::Clock => self.clock,
                    WidgetKind::TimeProgress => self.time_progress,
                    WidgetKind::Note => self.note,
                    WidgetKind::Shortcut => self.shortcut,
                    WidgetKind::Files => self.files,
                    WidgetKind::Activity => self.activity,
                    WidgetKind::Network => self.network,
                    WidgetKind::Calendar => self.calendar,
                    WidgetKind::Cycle => self.cycle,
                    WidgetKind::Countdown => self.countdown,
                    WidgetKind::Pomodoro => self.pomodoro,
                    WidgetKind::Todo => self.todo,
                    WidgetKind::Water => self.water,
                    WidgetKind::Media => self.media,
                    WidgetKind::Claude => self.claude,
                    WidgetKind::Weather => self.weather,
                    WidgetKind::Launcher => self.launcher,
                }
                .to_string()
            }),
            Item::System(SystemItem::Settings) => self.settings.into(),
            // Never shown nor read: a spacer has no label.
            Item::Spacer(_) => String::new(),
        }
    }
}

/// What the items come from.
#[derive(Debug, Clone, Copy)]
pub struct Sources<'a> {
    pub widgets: Option<&'a WidgetsSnapshot>,
    pub apps: Option<&'a AppsSnapshot>,
    pub open: &'a OpenSnapshot,
    /// Whether the open programs that are not pinned show (a setting).
    pub show_open: bool,
    /// The workspace the dock shows (spec 0013): only the programs and
    /// widgets its keys name show, in its order, with its spacers. `None`
    /// when the workspaces can't be read: then every program, then every
    /// widget, and no spacer.
    pub workspace: Option<&'a Workspace>,
}

impl Sources<'_> {
    fn attention(&self) -> bool {
        needs_attention(self.widgets)
    }
}

/// The items in order and the size of each group that has items.
#[derive(Debug, Clone, PartialEq)]
pub struct Items {
    pub items: Vec<Item>,
    pub groups: Vec<usize>,
}

fn usable(storage: StorageStatus) -> bool {
    !matches!(storage, StorageStatus::Unavailable { .. })
}

/// The dock's items: without a readable list, or with the storage
/// unavailable, a group is empty.
pub fn items(sources: Sources<'_>) -> Items {
    let apps: Vec<Item> = sources
        .apps
        .filter(|snapshot| usable(snapshot.storage))
        .map(|snapshot| snapshot.apps.iter().cloned().map(Item::App).collect())
        .unwrap_or_default();
    let widgets: Vec<Item> = sources
        .widgets
        .filter(|snapshot| usable(snapshot.storage))
        .map(|snapshot| {
            snapshot
                .widgets
                .iter()
                .filter(|widget| widget.config.kind().creatable())
                .cloned()
                .map(Item::Widget)
                .collect()
        })
        .unwrap_or_default();
    let others: Vec<Item> = if sources.show_open {
        sources
            .open
            .others
            .iter()
            .cloned()
            .map(Item::Other)
            .collect()
    } else {
        Vec::new()
    };
    let mut systems = Vec::new();
    if sources.attention() {
        systems.push(Item::System(SystemItem::Settings));
    }
    let mixed = match sources.workspace {
        Some(workspace) => {
            let spacers = workspace.spacers.iter().cloned().map(Item::Spacer);
            let all = apps.into_iter().chain(widgets).chain(spacers).collect();
            in_workspace(&workspace.items, all)
        }
        None => apps.into_iter().chain(widgets).collect(),
    };
    let groups = [mixed.len(), others.len(), systems.len()]
        .into_iter()
        .filter(|&size| size > 0)
        .collect();
    let items = [mixed, others, systems].into_iter().flatten().collect();
    Items { items, groups }
}

/// The programs, the widgets and the spacers of a workspace, in its order,
/// of `all` there are: `keys` names them; a key whose item is gone is
/// skipped.
pub fn in_workspace(keys: &[String], mut all: Vec<Item>) -> Vec<Item> {
    keys.iter()
        .filter_map(|key| {
            let index = all.iter().position(|item| item.key() == *key)?;
            Some(all.remove(index))
        })
        .collect()
}

/// The dock's order after the item `from` (an index of `items`) is let go at
/// `to` among them: the keys, and where the item now is among those of its
/// kind (what its own table keeps).
pub fn moved(items: &[Item], from: usize, to: usize) -> Option<(Vec<String>, usize)> {
    let mut keys: Vec<String> = items.iter().map(Item::key).collect();
    if from >= keys.len() {
        return None;
    }
    let key = keys.remove(from);
    let to = to.min(keys.len());
    keys.insert(to, key);
    let kind = |key: &str| key.split(':').next().unwrap_or_default().to_string();
    let own = kind(&keys[to]);
    let among = keys[..to].iter().filter(|key| kind(key) == own).count();
    Some((keys, among))
}

/// Where files dropped on the dock at the screen x `screen_x` (physical
/// pixels) are pinned, among `group`, the pinned programs and the widgets
/// with the centers `centers` (logical pixels from the window's client
/// left `left`, at `scale`): the key of the item they go before, `None`
/// past the last one.
pub fn dropped_before(
    group: &[Item],
    centers: &[f64],
    screen_x: i32,
    left: i32,
    scale: f64,
) -> Option<String> {
    let x = f64::from(screen_x - left) / scale;
    let index = centers.iter().filter(|&&center| center < x).count();
    group.get(index).map(Item::key)
}

/// Whether Configurações wears the attention badge: while the database was
/// recovered (until the notice is dismissed) or is unavailable (spec 0003
/// §2.11).
pub fn needs_attention(widgets: Option<&WidgetsSnapshot>) -> bool {
    widgets.is_some_and(|snapshot| snapshot.storage != StorageStatus::Ok)
}

/// Whether the pinned program `app` has windows open, and whether the user is
/// in one of them.
pub fn running(open: &OpenSnapshot, item: &Item) -> Option<bool> {
    match item {
        Item::App(app) => open
            .apps
            .iter()
            .find(|found| found.id == app.id.as_str())
            .map(|found| found.active),
        Item::Other(other) => Some(other.active),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::open_apps::OpenApp;
    use crate::domain::widgets::{UnavailableReason, WidgetConfig, WidgetId};

    const NAMES: Names = Names {
        settings: "Configurações",
        clock: "Relógio",
        time_progress: "Progresso do tempo",
        note: "Nota",
        shortcut: "Atalho",
        files: "Arquivos",
        activity: "Atividade",
        network: "Atividade de rede",
        calendar: "Calendário",
        cycle: "Ciclo",
        countdown: "Contagem regressiva",
        pomodoro: "Pomodoro",
        todo: "Lista de tarefas",
        water: "Beber água",
        media: "Tocando agora",
        claude: "Uso do Claude",
        weather: "Clima",
        launcher: "Lançador",
    };

    fn id(n: u8) -> WidgetId {
        WidgetId::parse(&format!("0190a1b2-c3d4-7e5f-8a6b-{:012x}", n)).unwrap()
    }

    fn app(n: u8, name: &str) -> PinnedApp {
        PinnedApp {
            id: id(n),
            app_id: format!("App{n}"),
            name: name.into(),
        }
    }

    fn widget(n: u8, kind: WidgetKind, label: Option<&str>) -> WidgetInstance {
        WidgetInstance {
            id: id(n),
            label: label.map(String::from),
            config: WidgetConfig::default_for(kind),
        }
    }

    fn other(key: &str, active: bool) -> OpenOther {
        OpenOther {
            key: key.into(),
            app_id: "Chrome".into(),
            name: "Google Chrome".into(),
            count: 1,
            active,
            badge: false,
        }
    }

    fn sources<'a>(
        widgets: Option<&'a WidgetsSnapshot>,
        apps: Option<&'a AppsSnapshot>,
        open: &'a OpenSnapshot,
    ) -> Sources<'a> {
        Sources {
            widgets,
            apps,
            open,
            show_open: true,
            workspace: None,
        }
    }

    #[test]
    fn programs_widgets_open_programs_then_the_system_items() {
        let widgets = WidgetsSnapshot {
            storage: StorageStatus::Ok,
            widgets: vec![widget(10, WidgetKind::Clock, None)],
        };
        let apps = AppsSnapshot {
            storage: StorageStatus::Ok,
            apps: vec![app(1, "Bloco"), app(2, "Paint")],
        };
        let open = OpenSnapshot {
            apps: vec![],
            others: vec![other("00000000000000aa", false)],
        };

        let Items { items, groups } = items(sources(Some(&widgets), Some(&apps), &open));

        let keys: Vec<String> = items.iter().map(Item::key).collect();
        assert_eq!(
            keys,
            vec![
                format!("app:{}", id(1).as_str()),
                format!("app:{}", id(2).as_str()),
                format!("widget:{}", id(10).as_str()),
                "open:00000000000000aa".to_string(),
            ]
        );
        // The programs and the widgets are one group.
        assert_eq!(groups, vec![3, 1]);
    }

    #[test]
    fn a_workspace_shows_its_items_in_its_order() {
        let keys = |items: &[Item]| -> Vec<String> { items.iter().map(Item::key).collect() };
        let apps = [Item::App(app(1, "A")), Item::App(app(2, "B"))];
        let widgets = [
            Item::Widget(widget(10, WidgetKind::Clock, None)),
            Item::Widget(widget(11, WidgetKind::Note, None)),
        ];
        let (a1, a2) = (apps[0].key(), apps[1].key());
        let (w1, w2) = (widgets[0].key(), widgets[1].key());

        // Widget, program, widget; the other program is not in it.
        let all: Vec<Item> = apps.iter().chain(&widgets).cloned().collect();
        let order = vec![w2.clone(), a2.clone(), w1.clone()];
        let shown = in_workspace(&order, all.clone());
        assert_eq!(keys(&shown), [w2.clone(), a2.clone(), w1.clone()]);

        // A key whose item is gone is skipped.
        let order = vec!["app:gone".to_string(), a1.clone()];
        assert_eq!(keys(&in_workspace(&order, all)), [a1]);
    }

    fn spacer(n: u8, size: SpacerSize) -> Spacer {
        Spacer {
            id: id(n).as_str().to_owned(),
            size,
        }
    }

    fn workspace_of(items: Vec<String>, spacers: Vec<Spacer>) -> Workspace {
        Workspace {
            id: "w".into(),
            name: "Principal".into(),
            color: "#0A84FF".into(),
            items,
            spacers,
        }
    }

    #[test]
    fn the_spacers_of_the_workspace_show_where_it_puts_them() {
        let apps = AppsSnapshot {
            storage: StorageStatus::Ok,
            apps: vec![app(1, "Bloco"), app(2, "Paint")],
        };
        let open = OpenSnapshot {
            apps: vec![],
            others: vec![other("00000000000000aa", false)],
        };
        let (small, large) = (spacer(20, SpacerSize::Small), spacer(21, SpacerSize::Large));
        let a1 = format!("app:{}", id(1).as_str());
        let a2 = format!("app:{}", id(2).as_str());
        let workspace = workspace_of(
            vec![large.key(), a1.clone(), small.key(), a2.clone()],
            vec![large.clone(), small.clone()],
        );

        let found = items(Sources {
            workspace: Some(&workspace),
            ..sources(None, Some(&apps), &open)
        });

        let keys: Vec<String> = found.items.iter().map(Item::key).collect();
        assert_eq!(
            keys,
            vec![
                large.key(),
                a1,
                small.key(),
                a2,
                "open:00000000000000aa".to_string()
            ]
        );
        // In the group of the programs and the widgets.
        assert_eq!(found.groups, vec![4, 1]);
        assert_eq!(found.items[0].spacer_size(), Some(SpacerSize::Large));
        assert_eq!(found.items[1].spacer_size(), None);
        assert_eq!(NAMES.of(&found.items[2]), "");
        assert!(!found.items[0].is_program());
        assert_eq!(running(&open, &found.items[0]), None);
    }

    #[test]
    fn a_spacer_is_dragged_among_the_programs_and_the_widgets() {
        let items = vec![
            Item::App(app(1, "A")),
            Item::Spacer(spacer(20, SpacerSize::Small)),
            Item::App(app(2, "B")),
            Item::Widget(widget(10, WidgetKind::Clock, None)),
        ];
        // The spacer past the second program.
        let (order, _) = moved(&items, 1, 2).unwrap();
        assert_eq!(
            order,
            vec![
                items[0].key(),
                items[2].key(),
                items[1].key(),
                items[3].key()
            ]
        );
        // To the end, after the widget.
        let (order, _) = moved(&items, 1, 3).unwrap();
        assert_eq!(order[3], items[1].key());
        // A program across it.
        let (order, _) = moved(&items, 0, 1).unwrap();
        assert_eq!(order[..2], [items[1].key(), items[0].key()]);
    }

    #[test]
    fn a_drag_gives_the_new_order_and_the_place_among_its_kind() {
        let items = vec![
            Item::App(app(1, "A")),
            Item::App(app(2, "B")),
            Item::Widget(widget(10, WidgetKind::Clock, None)),
            Item::Widget(widget(11, WidgetKind::Note, None)),
        ];
        // The first widget to the front: first of the widgets still.
        let (order, among) = moved(&items, 2, 0).unwrap();
        assert_eq!(order[0], items[2].key());
        assert_eq!(among, 0);
        // The first program after the widgets: last of the programs.
        let (order, among) = moved(&items, 0, 3).unwrap();
        assert_eq!(order[3], items[0].key());
        assert_eq!(among, 1);
        assert_eq!(moved(&items, 9, 0), None);
    }

    #[test]
    fn without_lists_the_dock_is_empty() {
        let open = OpenSnapshot::default();

        let found = items(sources(None, None, &open));

        assert!(found.items.is_empty());
        assert!(found.groups.is_empty());
    }

    #[test]
    fn configuracoes_shows_only_while_the_storage_needs_attention() {
        let recovered = WidgetsSnapshot {
            storage: StorageStatus::Recovered,
            widgets: vec![widget(10, WidgetKind::Note, None)],
        };
        let open = OpenSnapshot::default();

        let found = items(sources(Some(&recovered), None, &open));

        assert_eq!(found.groups, vec![1, 1]);
        assert_eq!(found.items[1], Item::System(SystemItem::Settings));
        assert_eq!(NAMES.of(&found.items[1]), "Configurações");
    }

    #[test]
    fn an_unavailable_storage_hides_the_programs_and_the_widgets() {
        let unavailable = StorageStatus::Unavailable {
            reason: UnavailableReason::OpenFailed,
        };
        let widgets = WidgetsSnapshot {
            storage: unavailable,
            widgets: vec![widget(10, WidgetKind::Note, None)],
        };
        let apps = AppsSnapshot {
            storage: unavailable,
            apps: vec![app(1, "Bloco")],
        };
        let open = OpenSnapshot::default();

        let found = items(sources(Some(&widgets), Some(&apps), &open));

        // Only Configurações, with the badge.
        assert_eq!(found.groups, vec![1]);
        assert_eq!(found.items, vec![Item::System(SystemItem::Settings)]);
        assert!(needs_attention(Some(&widgets)));
    }

    #[test]
    fn the_open_programs_hide_with_the_setting() {
        let open = OpenSnapshot {
            apps: vec![],
            others: vec![other("00000000000000aa", true)],
        };
        let found = items(Sources {
            show_open: false,
            ..sources(None, None, &open)
        });

        assert!(found.items.is_empty());
    }

    #[test]
    fn widgets_of_kinds_not_usable_yet_are_left_out() {
        let widgets = WidgetsSnapshot {
            storage: StorageStatus::Ok,
            widgets: vec![
                widget(10, WidgetKind::Files, None),
                widget(11, WidgetKind::Note, None),
            ],
        };
        let open = OpenSnapshot::default();

        let found = items(sources(Some(&widgets), None, &open));

        assert_eq!(found.groups, vec![1]);
    }

    #[test]
    fn names_are_the_label_or_the_kind_never_the_time() {
        assert_eq!(
            NAMES.of(&Item::Widget(widget(1, WidgetKind::Clock, None))),
            "Relógio"
        );
        assert_eq!(
            NAMES.of(&Item::Widget(widget(1, WidgetKind::Clock, Some("Lisboa")))),
            "Lisboa"
        );
        assert_eq!(NAMES.of(&Item::App(app(1, "Paint"))), "Paint");
    }

    #[test]
    fn running_programs_are_told_with_whether_the_user_is_in_them() {
        let open = OpenSnapshot {
            apps: vec![OpenApp {
                id: id(1).as_str().to_string(),
                count: 2,
                active: true,
            }],
            others: vec![],
        };

        assert_eq!(running(&open, &Item::App(app(1, "Bloco"))), Some(true));
        assert_eq!(running(&open, &Item::App(app(2, "Paint"))), None);
        assert_eq!(
            running(&open, &Item::Other(other("00000000000000aa", false))),
            Some(false)
        );
        assert_eq!(running(&open, &Item::System(SystemItem::Settings)), None);
        assert!(!needs_attention(None));
    }

    /// The pinned programs and the widgets: a program, a clock two icons
    /// wide, a program; their centers at rest, in logical pixels.
    fn dropped_on() -> (Vec<Item>, Vec<f64>) {
        let group = vec![
            Item::App(app(1, "Bloco")),
            Item::Widget(widget(10, WidgetKind::Clock, None)),
            Item::App(app(2, "Paint")),
        ];
        (group, vec![24.0, 96.0, 168.0])
    }

    #[test]
    fn files_dropped_beside_a_spacer_go_before_it() {
        let group = vec![
            Item::App(app(1, "Bloco")),
            Item::Spacer(spacer(20, SpacerSize::Large)),
            Item::App(app(2, "Paint")),
        ];
        let centers = [24.0, 67.0, 110.0];
        assert_eq!(
            dropped_before(&group, &centers, 50, 0, 1.0),
            Some(group[1].key())
        );
        assert_eq!(
            dropped_before(&group, &centers, 80, 0, 1.0),
            Some(group[2].key())
        );
    }

    #[test]
    fn files_dropped_go_where_the_pointer_lets_them_go() {
        let (group, centers) = dropped_on();
        let key = |n: usize| Some(group[n].key());

        // Before the first, between two, past the last.
        assert_eq!(dropped_before(&group, &centers, 10, 0, 1.0), key(0));
        assert_eq!(dropped_before(&group, &centers, 60, 0, 1.0), key(1));
        assert_eq!(dropped_before(&group, &centers, 200, 0, 1.0), None);
        // On a widget: its half says which side.
        assert_eq!(dropped_before(&group, &centers, 90, 0, 1.0), key(1));
        assert_eq!(dropped_before(&group, &centers, 110, 0, 1.0), key(2));
    }

    #[test]
    fn files_dropped_on_a_second_monitor_go_where_its_dock_has_the_pointer() {
        let (group, centers) = dropped_on();
        let key = |n: usize| Some(group[n].key());

        // Right of the main one, at 150%: 130 logical pixels in (195
        // physical, past the last center).
        assert_eq!(dropped_before(&group, &centers, 2115, 1920, 1.5), key(2));
        // Left of it, at 100%.
        assert_eq!(dropped_before(&group, &centers, -2500, -2560, 1.0), key(1));
        assert_eq!(dropped_before(&group, &centers, -2340, -2560, 1.0), None);
    }
}
