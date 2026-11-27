//! Where each Ciclo goes when the Ciclo becomes a part of the Calendário
//! (spec 0031 §3.7). Pure rules: `persistence/cycle_into_calendar.rs` does
//! the moves.

use crate::domain::widgets::{WidgetConfig, WidgetInstance};
use crate::domain::workspaces::Workspace;

/// What happens to one Ciclo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Move {
    /// Its settings go into the Calendário `calendar`, and it goes.
    Into { cycle: String, calendar: String },
    /// It becomes a Calendário in its place, with the cycle on.
    Become { cycle: String },
}

/// The widgets of `workspace` in dock order, a stack's in its order.
fn widgets_of(workspace: &Workspace) -> Vec<&str> {
    workspace
        .items
        .iter()
        .flat_map(|key| {
            if let Some(id) = key.strip_prefix("widget:") {
                vec![id]
            } else {
                workspace
                    .stacks
                    .iter()
                    .find(|stack| stack.key() == *key)
                    .map(|stack| stack.widgets.iter().map(String::as_str).collect())
                    .unwrap_or_default()
            }
        })
        .collect()
}

/// Where each Ciclo of `widgets` goes, in their order:
/// 1. into the Calendário stacked with it;
/// 2. else into the first Calendário of its workspace;
/// 3. else, or when it is in more than one workspace, it becomes one.
///
/// A Calendário takes one cycle only, and none when it has one already.
pub fn plan(widgets: &[WidgetInstance], workspaces: &[Workspace]) -> Vec<Move> {
    let mut taken: Vec<&str> = widgets
        .iter()
        .filter(|widget| matches!(&widget.config, WidgetConfig::Calendar(config) if config.cycle.is_some()))
        .map(|widget| widget.id.as_str())
        .collect();
    let free_calendar = |id: &str, taken: &[&str]| {
        !taken.contains(&id)
            && widgets.iter().any(|widget| {
                widget.id.as_str() == id && matches!(widget.config, WidgetConfig::Calendar(_))
            })
    };
    let mut moves = Vec::new();
    for widget in widgets {
        if !matches!(widget.config, WidgetConfig::Cycle(_)) {
            continue;
        }
        let cycle = widget.id.as_str();
        let holders: Vec<&Workspace> = workspaces
            .iter()
            .filter(|workspace| widgets_of(workspace).contains(&cycle))
            .collect();
        let calendar = match holders.as_slice() {
            [workspace] => {
                let stacked = workspace
                    .stacks
                    .iter()
                    .find(|stack| stack.widgets.iter().any(|id| id == cycle))
                    .and_then(|stack| {
                        stack
                            .widgets
                            .iter()
                            .map(String::as_str)
                            .find(|id| free_calendar(id, &taken))
                    });
                stacked.or_else(|| {
                    widgets_of(workspace)
                        .into_iter()
                        .find(|id| free_calendar(id, &taken))
                })
            }
            _ => None,
        };
        moves.push(match calendar {
            Some(calendar) => {
                taken.push(calendar);
                Move::Into {
                    cycle: cycle.to_owned(),
                    calendar: calendar.to_owned(),
                }
            }
            None => Move::Become {
                cycle: cycle.to_owned(),
            },
        });
    }
    moves
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::widgets::calendar::CalendarConfig;
    use crate::domain::widgets::{CycleConfig, WidgetId};
    use crate::domain::workspaces::Stack;

    fn id(n: u8) -> String {
        format!("019a1b2c-3d4e-7f60-8a1b-2c3d4e5f60{n:02}")
    }

    fn widget(n: u8, config: WidgetConfig) -> WidgetInstance {
        WidgetInstance {
            id: WidgetId::parse(&id(n)).unwrap(),
            label: None,
            config,
        }
    }

    fn cycle(n: u8) -> WidgetInstance {
        widget(n, WidgetConfig::Cycle(CycleConfig::default()))
    }

    fn calendar(n: u8) -> WidgetInstance {
        widget(n, WidgetConfig::Calendar(CalendarConfig::default()))
    }

    fn key(n: u8) -> String {
        format!("widget:{}", id(n))
    }

    fn workspace(name: &str, items: Vec<String>, stacks: Vec<Stack>) -> Workspace {
        Workspace {
            id: name.into(),
            name: name.into(),
            color: "#0A84FF".into(),
            items,
            spacers: Vec::new(),
            stacks,
        }
    }

    fn into(cycle: u8, calendar: u8) -> Move {
        Move::Into {
            cycle: id(cycle),
            calendar: id(calendar),
        }
    }

    fn become_(cycle: u8) -> Move {
        Move::Become { cycle: id(cycle) }
    }

    #[test]
    fn a_ciclo_stacked_with_a_calendario_goes_into_it() {
        let stack = Stack {
            id: "s".into(),
            widgets: vec![id(2), id(1)],
            top: 1,
        };
        // A Calendário outside the stack comes first in the dock, but the
        // one stacked with the Ciclo takes it.
        let spaces = [workspace(
            "w",
            vec![key(3), stack.key()],
            vec![stack.clone()],
        )];
        let widgets = [cycle(1), calendar(2), calendar(3)];
        assert_eq!(plan(&widgets, &spaces), [into(1, 2)]);
    }

    #[test]
    fn a_ciclo_goes_into_the_first_calendario_of_its_workspace() {
        let stack = Stack {
            id: "s".into(),
            widgets: vec![id(4), id(2)],
            top: 0,
        };
        let spaces = [
            workspace("other", vec![key(5)], Vec::new()),
            workspace("w", vec![key(1), stack.key(), key(3)], vec![stack.clone()]),
        ];
        let widgets = [cycle(1), calendar(2), calendar(3), calendar(5)];
        // 2 is in a stack before 3 in the dock; 5 is in another workspace.
        assert_eq!(plan(&widgets, &spaces), [into(1, 2)]);
    }

    #[test]
    fn a_ciclo_without_a_calendario_by_it_becomes_one() {
        // The Calendário is only in another workspace: the Ciclo does not
        // jump to it.
        let spaces = [
            workspace("private", vec![key(1)], Vec::new()),
            workspace("shared", vec![key(2)], Vec::new()),
        ];
        assert_eq!(plan(&[cycle(1), calendar(2)], &spaces), [become_(1)]);
        // In no workspace at all.
        assert_eq!(plan(&[cycle(1)], &[]), [become_(1)]);
    }

    #[test]
    fn a_calendario_takes_one_ciclo_only() {
        let spaces = [workspace("w", vec![key(1), key(2), key(3)], Vec::new())];
        let widgets = [cycle(1), calendar(2), cycle(3)];
        assert_eq!(plan(&widgets, &spaces), [into(1, 2), become_(3)]);
        // One that has a cycle already takes none.
        let taken = widget(
            2,
            WidgetConfig::Calendar(CalendarConfig {
                cycle: Some(CycleConfig::default()),
                ..CalendarConfig::default()
            }),
        );
        assert_eq!(plan(&[cycle(1), taken], &spaces), [become_(1)]);
    }

    #[test]
    fn a_ciclo_in_several_workspaces_becomes_a_calendario() {
        let spaces = [
            workspace("a", vec![key(1), key(2)], Vec::new()),
            workspace("b", vec![key(1)], Vec::new()),
        ];
        assert_eq!(plan(&[cycle(1), calendar(2)], &spaces), [become_(1)]);
    }

    #[test]
    fn without_ciclos_nothing_moves() {
        let spaces = [workspace("w", vec![key(2)], Vec::new())];
        assert!(plan(&[calendar(2)], &spaces).is_empty());
    }
}
