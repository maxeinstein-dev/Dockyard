//! Moves each Ciclo into the Calendário once (spec 0031 §3.7), as
//! `domain::cycle_into_calendar` plans, in one transaction, and marks it
//! done in `meta`.

use rusqlite::{Connection, OptionalExtension, params};

use super::database::{begin, read_failed, write_failed};
use super::{widgets_repo, workspaces_repo};
use crate::domain::cycle_into_calendar::{Move, plan};
use crate::domain::widgets::calendar::CalendarConfig;
use crate::domain::widgets::{
    WidgetConfig, WidgetError, WidgetId, WidgetInstance, WidgetKind, decode_config, encode_config,
};

/// The `meta` key that says the Ciclos were moved.
const DONE_KEY: &str = "cycle_into_calendar";

/// The Ciclos stored, in dock order; one that can't be read is left alone.
fn stored_cycles(conn: &Connection) -> Result<Vec<WidgetInstance>, WidgetError> {
    let mut statement = conn
        .prepare(
            "SELECT id, config, config_version FROM widget_instances              WHERE kind = ?1 ORDER BY position",
        )
        .map_err(read_failed)?;
    let rows = statement
        .query_map(params![WidgetKind::Cycle.as_str()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, u32>(2)?,
            ))
        })
        .map_err(read_failed)?;
    Ok(rows
        .filter_map(Result::ok)
        .filter_map(|(id, json, version)| {
            Some(WidgetInstance {
                id: WidgetId::parse(&id).ok()?,
                label: None,
                config: decode_config(WidgetKind::Cycle, version, &json).ok()?,
            })
        })
        .collect())
}

/// Moves the Ciclos into the Calendário, the first time only. Whether a
/// Ciclo moved. On an error nothing changes, and the next start tries again.
pub fn run(conn: &mut Connection) -> Result<bool, WidgetError> {
    let done: Option<String> = conn
        .query_row(
            "SELECT value FROM meta WHERE key = ?1",
            params![DONE_KEY],
            |row| row.get(0),
        )
        .optional()
        .map_err(read_failed)?;
    if done.is_some() {
        return Ok(false);
    }
    // The interface no longer sees the Ciclos (not creatable): read them
    // here, with the rest.
    let mut widgets = widgets_repo::list(conn)?;
    widgets.extend(stored_cycles(conn)?);
    let workspaces = workspaces_repo::list(conn).map_err(|error| {
        log::warn!("cycle: the workspaces could not be read: {error}");
        WidgetError::WriteFailed
    })?;
    let moves = plan(&widgets, &workspaces);
    let cycle_of = |id: &str| {
        widgets
            .iter()
            .find(|widget| widget.id.as_str() == id)
            .and_then(|widget| widget.config.cycle().cloned())
    };
    let tx = begin(conn)?;
    for each in &moves {
        match each {
            Move::Into { cycle, calendar } => {
                let Some(WidgetConfig::Calendar(config)) = widgets
                    .iter()
                    .find(|widget| widget.id.as_str() == calendar)
                    .map(|widget| &widget.config)
                else {
                    continue;
                };
                let config = WidgetConfig::Calendar(CalendarConfig {
                    cycle: cycle_of(cycle),
                    ..config.clone()
                });
                tx.execute(
                    "UPDATE widget_instances SET config = ?1 WHERE id = ?2",
                    params![encode_config(&config), calendar],
                )
                .map_err(write_failed)?;
                tx.execute("DELETE FROM widget_instances WHERE id = ?1", params![cycle])
                    .map_err(write_failed)?;
                workspaces_repo::forget_in(&tx, &format!("widget:{cycle}")).map_err(|error| {
                    log::warn!("cycle: the Ciclo could not leave its workspace: {error}");
                    WidgetError::WriteFailed
                })?;
            }
            Move::Become { cycle } => {
                let config = WidgetConfig::Calendar(CalendarConfig {
                    cycle: cycle_of(cycle),
                    ..CalendarConfig::default()
                });
                tx.execute(
                    "UPDATE widget_instances SET kind = ?1, config = ?2, config_version = ?3 \
                     WHERE id = ?4",
                    params![
                        WidgetKind::Calendar.as_str(),
                        encode_config(&config),
                        WidgetKind::Calendar.config_version(),
                        cycle
                    ],
                )
                .map_err(write_failed)?;
            }
        }
    }
    // The dock's order stays contiguous after the Ciclos that went.
    tx.execute(
        "UPDATE widget_instances SET position = (SELECT COUNT(*) FROM widget_instances AS before \
         WHERE before.position < widget_instances.position)",
        [],
    )
    .map_err(write_failed)?;
    tx.execute(
        "INSERT INTO meta (key, value) VALUES (?1, '1')",
        params![DONE_KEY],
    )
    .map_err(write_failed)?;
    tx.commit().map_err(write_failed)?;
    if !moves.is_empty() {
        log::info!("cycle: {} Ciclo(s) moved into the Calendário", moves.len());
    }
    Ok(!moves.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::widgets::calendar::CalendarConfig;
    use crate::domain::widgets::{CycleConfig, WidgetConfig, WidgetKind, encode_config};
    use crate::persistence::database::MIGRATIONS;
    use crate::persistence::{widgets_repo, workspaces_repo};

    const CYCLE: &str = "019a1b2c-3d4e-7f60-8a1b-2c3d4e5f6001";
    const CALENDAR: &str = "019a1b2c-3d4e-7f60-8a1b-2c3d4e5f6002";
    const CLOCK: &str = "019a1b2c-3d4e-7f60-8a1b-2c3d4e5f6003";

    fn mine() -> CycleConfig {
        CycleConfig {
            sensitive: 7,
            ..CycleConfig::default()
        }
    }

    /// A database with the widgets `rows` (id and config, in dock order)
    /// and a first workspace with `items`.
    fn database(rows: &[(&str, WidgetConfig)], items: &[&str]) -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        for migration in MIGRATIONS {
            conn.execute_batch(migration).unwrap();
        }
        for (position, (id, config)) in rows.iter().enumerate() {
            conn.execute(
                "INSERT INTO widget_instances VALUES (?1, ?2, NULL, ?3, ?4, 1, 0)",
                rusqlite::params![
                    id,
                    config.kind().as_str(),
                    position as i64,
                    encode_config(config)
                ],
            )
            .unwrap();
        }
        let keys: Vec<String> = items.iter().map(|item| (*item).to_owned()).collect();
        workspaces_repo::ensure_first(&mut conn, &keys).unwrap();
        conn
    }

    fn key(id: &str) -> String {
        format!("widget:{id}")
    }

    fn calendar_of(conn: &Connection, id: &str) -> CalendarConfig {
        let widgets = widgets_repo::list(conn).unwrap();
        let widget = widgets
            .iter()
            .find(|widget| widget.id.as_str() == id)
            .expect("the widget is there");
        match &widget.config {
            WidgetConfig::Calendar(config) => config.clone(),
            other => panic!("not a Calendário: {other:?}"),
        }
    }

    #[test]
    fn a_ciclo_goes_into_the_calendario_of_its_workspace_once() {
        let mut conn = database(
            &[
                (CYCLE, WidgetConfig::Cycle(mine())),
                (CALENDAR, WidgetConfig::default_for(WidgetKind::Calendar)),
                (CLOCK, WidgetConfig::default_for(WidgetKind::Clock)),
            ],
            &[&key(CYCLE), &key(CALENDAR), &key(CLOCK)],
        );
        assert!(run(&mut conn).unwrap());
        assert_eq!(calendar_of(&conn, CALENDAR).cycle, Some(mine()));
        let widgets = widgets_repo::list(&conn).unwrap();
        assert_eq!(widgets.len(), 2);
        let workspace = &workspaces_repo::list(&conn).unwrap()[0];
        assert_eq!(workspace.items, [key(CALENDAR), key(CLOCK)]);
        // Once: a later run moves nothing.
        assert!(!run(&mut conn).unwrap());
    }

    #[test]
    fn a_stack_left_with_the_calendario_alone_comes_apart() {
        let mut conn = database(
            &[
                (CALENDAR, WidgetConfig::default_for(WidgetKind::Calendar)),
                (CYCLE, WidgetConfig::Cycle(mine())),
                (CLOCK, WidgetConfig::default_for(WidgetKind::Clock)),
            ],
            &[&key(CALENDAR), &key(CYCLE), &key(CLOCK)],
        );
        let workspace = workspaces_repo::list(&conn).unwrap()[0].id.clone();
        workspaces_repo::stack_with(&mut conn, &workspace, &key(CYCLE), &key(CALENDAR)).unwrap();
        assert!(run(&mut conn).unwrap());
        let workspace = &workspaces_repo::list(&conn).unwrap()[0];
        assert!(workspace.stacks.is_empty());
        assert_eq!(workspace.items, [key(CALENDAR), key(CLOCK)]);
        assert_eq!(calendar_of(&conn, CALENDAR).cycle, Some(mine()));
    }

    #[test]
    fn a_ciclo_alone_becomes_a_calendario_in_its_place() {
        let mut conn = database(
            &[
                (CLOCK, WidgetConfig::default_for(WidgetKind::Clock)),
                (CYCLE, WidgetConfig::Cycle(mine())),
            ],
            &[&key(CLOCK), &key(CYCLE)],
        );
        assert!(run(&mut conn).unwrap());
        let calendar = calendar_of(&conn, CYCLE);
        assert_eq!(
            calendar,
            CalendarConfig {
                cycle: Some(mine()),
                ..CalendarConfig::default()
            }
        );
        let workspace = &workspaces_repo::list(&conn).unwrap()[0];
        assert_eq!(workspace.items, [key(CLOCK), key(CYCLE)]);
    }

    #[test]
    fn without_ciclos_it_is_marked_done_and_nothing_changes() {
        let mut conn = database(
            &[(CLOCK, WidgetConfig::default_for(WidgetKind::Clock))],
            &[&key(CLOCK)],
        );
        assert!(!run(&mut conn).unwrap());
        let done: String = conn
            .query_row("SELECT value FROM meta WHERE key = ?1", [DONE_KEY], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(done, "1");
    }
}
