//! What the rest of the app calls: each one posts work to the dock's thread
//! (or reads what it shares) and returns.

use super::*;

/// The data changed (programs, widgets, open programs, settings): the docks
/// read it again.
/// Lets go of the docks and what draws them, on the main thread, as the app
/// ends (`RunEvent::Exit`). Left in the thread's storage, they would be
/// dropped inside `ExitProcess`, after Windows has ended the other threads:
/// Direct2D then calls DXGI, which refuses (`DXGI_ERROR_INVALID_CALL`) and
/// ends the process with an exception, so every exit was a crash. The
/// windows go with the process.
pub fn shutdown() {
    let docks = DOCKS.with(|slot| {
        slot.try_borrow_mut()
            .map(|mut docks| std::mem::take(&mut *docks))
            .unwrap_or_default()
    });
    drop(docks);
}

pub fn refresh() {
    post(Dock::reload);
}

/// The Calendário's events were read again: the docks repaint.
pub fn calendar_changed() {
    post(Dock::paint);
}

/// A click on the Calendário's card: its panel opens on the dock `owner`,
/// on `event`'s details when given.
pub fn open_calendar(
    owner: isize,
    widget: crate::domain::widgets::WidgetId,
    event: Option<String>,
) {
    post_to(owner, move |dock| dock.open_calendar(&widget, event));
}

/// The widgets `widgets` have a notice not seen yet: a Ciclo's (F2b-3) or
/// a Pomodoro's (F2c-2).
pub fn nudged(widgets: Vec<String>) {
    post(move |dock| dock.nudged(&widgets));
}

/// Files dragged over the dock `hwnd` at the screen point `at`, or gone
/// with `None` (spec 0018): the id of the program under them.
pub fn drop_over(hwnd: isize, at: Option<(i32, i32)>) -> Option<String> {
    let mut found = None;
    with_dock(HWND(hwnd as _), |dock| found = dock.drop_over(at));
    found
}

/// Whether the pointer is off the dock's window.
pub(super) fn pointer_off(hwnd: HWND) -> bool {
    let mut at = POINT::default();
    let mut window = windows::Win32::Foundation::RECT::default();
    unsafe {
        if windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut at).is_err()
            || GetWindowRect(hwnd, &mut window).is_err()
        {
            return false;
        }
    }
    at.x < window.left || at.x >= window.right || at.y < window.top || at.y >= window.bottom
}

/// What plays now changed (spec 0017 F4).
pub fn media_changed() {
    post(Dock::media_changed);
}

/// The notification badges of the taskbar changed (spec 0016).
pub fn badges_changed() {
    post(Dock::notices_changed);
}

/// The icon pack or its folder changed: reads the icons again.
pub fn forget_icons() {
    post(|dock| {
        dock.icons.clear();
        dock.reload();
    });
}

/// Shows the docks, up if they hide on their own, but not one whose monitor
/// is in full screen.
pub fn show() {
    SHOWN.store(true, Ordering::SeqCst);
    post(|dock| {
        if !dock.hidden_for_fullscreen {
            dock.show();
            dock.untuck();
        }
    });
}

/// The monitor `key` is in full screen (`full`) or not any more (spec 0010
/// D4: each dock looks only at its own monitor): its dock steps aside, and
/// comes back after unless the user hid the docks meanwhile.
pub fn set_fullscreen(key: String, full: bool) {
    post(move |dock| {
        if dock.monitor.key != key {
            return;
        }
        if full && dock.visible {
            log::info!("dock: a program is in full screen on its monitor, stepping aside");
            dock.hidden_for_fullscreen = true;
            dock.hide();
        } else if !full && dock.hidden_for_fullscreen {
            log::info!("dock: full screen is over on its monitor, back");
            dock.hidden_for_fullscreen = false;
            if SHOWN.load(Ordering::SeqCst) {
                dock.show();
            }
        }
    });
}

/// The monitors the docks are on: their key, handle and rectangle, for the
/// full screen check.
pub fn monitors() -> Vec<(String, isize, Rect)> {
    shared()
        .docks
        .iter()
        .map(|placed| (placed.monitor.clone(), placed.handle, placed.rect))
        .collect()
}

/// What the E2E suite reads of the dock (test build only): each item's name
/// and what its face draws, and whether it is painted dark.
#[cfg(feature = "e2e")]
pub struct View {
    pub items: Vec<(String, &'static str)>,
    pub dark: bool,
}

/// Asks the primary monitor's dock for its `View`, from a thread other than
/// the main one.
#[cfg(feature = "e2e")]
pub fn view() -> Option<View> {
    let (sent, received) = std::sync::mpsc::channel();
    post_primary(move |dock| {
        let items = dock
            .items
            .iter()
            .zip(&dock.names)
            .map(|(item, name)| {
                let face = match item {
                    Item::App(_) | Item::Other(_) => "app",
                    Item::System(_) => "glyph",
                    Item::Widget(widget) => match &widget.config {
                        crate::domain::widgets::WidgetConfig::Clock(clock) => {
                            match clock.icon_style {
                                crate::domain::widgets::clock::ClockIconStyle::Analog => "dial",
                                crate::domain::widgets::clock::ClockIconStyle::Digital => "time",
                            }
                        }
                        crate::domain::widgets::WidgetConfig::TimeProgress(progress) => {
                            if progress.width > 1 { "meters" } else { "ring" }
                        }
                        crate::domain::widgets::WidgetConfig::Activity(_) => "meters",
                        crate::domain::widgets::WidgetConfig::Network(_) => "rates",
                        crate::domain::widgets::WidgetConfig::Calendar(_) => "agenda",
                        crate::domain::widgets::WidgetConfig::Cycle(_) => "ring",
                        crate::domain::widgets::WidgetConfig::Countdown(_) => "agenda",
                        crate::domain::widgets::WidgetConfig::Pomodoro(_) => "ring",
                        crate::domain::widgets::WidgetConfig::Todo(_) => "agenda",
                        crate::domain::widgets::WidgetConfig::Water(_) => "ring",
                        crate::domain::widgets::WidgetConfig::Media(_) => "media",
                        crate::domain::widgets::WidgetConfig::Weather(_) => "weather",
                        crate::domain::widgets::WidgetConfig::Claude(config) => {
                            if config.width > 1 { "meters" } else { "ring" }
                        }
                        _ => "glyph",
                    },
                };
                (name.clone(), face)
            })
            .collect();
        let _ = sent.send(View {
            items,
            dark: dock.palette().dark,
        });
    });
    super::super::panics::answer(&received, "dock")
}

/// Presses the item named `name` on the primary monitor's dock, as a click
/// would (test build only, from a thread other than the main one). `false`
/// when the dock has no such item.
#[cfg(feature = "e2e")]
pub fn press_named(name: String) -> bool {
    let (sent, received) = std::sync::mpsc::channel();
    post_primary(move |dock| {
        let key = dock
            .names
            .iter()
            .position(|shown| *shown == name)
            .and_then(|index| dock.items.get(index))
            .map(Item::key);
        if let Some(key) = &key {
            shared().clicked = Some(Instant::now());
            dock.press(key);
        }
        // Configurações is in the dock's menu, not an item.
        let menu = key.is_none() && name == crate::i18n::messages().tray.open_settings;
        if menu {
            let app = dock.app.clone();
            dock.effects.push(Box::new(move |_| {
                if let Err(error) = super::super::settings_window::open(&app) {
                    log::warn!("e2e: the settings could not be opened: {error}");
                }
            }));
        }
        let _ = sent.send(key.is_some() || menu);
    });
    super::super::panics::answer(&received, "dock").unwrap_or(false)
}

/// Where a tray icon dragged out of the tray panel and let go at `(x, y)`
/// (physical pixels on the screen) goes among those the dock shows: `None`
/// when it was not let go on a dock; else before the tray icon kept in the
/// dock that follows the cell under the pointer (its lasting id), or at the
/// end (`Some(None)`). From a thread other than the main one.
pub fn tray_drop_index(x: i32, y: i32) -> Option<Option<String>> {
    let (sent, received) = std::sync::mpsc::channel();
    queue(Box::new(move |docks| {
        let found = docks.iter().find_map(|dock| dock.tray_drop_index(x, y));
        let _ = sent.send(found);
    }));
    super::super::panics::answer(&received, "dock").flatten()
}

/// Whether the next paint panics (test build only, M4).
#[cfg(feature = "e2e")]
pub(super) static PANIC_ON_PAINT: AtomicBool = AtomicBool::new(false);

/// Makes the primary monitor's dock panic in its next paint (test build
/// only): it must be made again, and the app go on.
#[cfg(feature = "e2e")]
pub fn panic_once() {
    PANIC_ON_PAINT.store(true, Ordering::SeqCst);
    post_primary(Dock::paint);
}

/// The monitors or the bands changed under the docks: they are placed again
/// (and made or destroyed for a monitor plugged in or unplugged).
pub fn reposition() {
    queue(Box::new(sync));
}

/// The open panel changed: the dock it anchors to marks its item and stops
/// magnifying; the others have none open.
pub fn set_open(panel: Option<PanelId>) {
    queue(Box::new(move |docks| {
        let active = shared().active;
        for dock in docks.iter_mut() {
            let mine = dock.hwnd.0 as isize == active;
            dock.set_open(if mine { panel.clone() } else { None });
        }
    }));
}

/// The settings moved the docks (or put them on every monitor or on one):
/// lays them out again now and, once placed, reopens the panel the move
/// closed at the dock it was anchored to.
pub fn settings_moved(app: &AppHandle) {
    let app = app.clone();
    queue(Box::new(move |docks| {
        for dock in docks.iter_mut() {
            dock.data = read_data(&dock.app, &dock.monitor.key);
        }
        sync(docks);
        let reopen = super::super::panel::lock(&app).dock_settled(false, Instant::now());
        if let Some(panel) = reopen {
            let active = shared().active;
            let index = docks
                .iter()
                .position(|dock| dock.hwnd.0 as isize == active)
                .unwrap_or(0);
            if let Some(dock) = docks.get_mut(index) {
                dock.request(panel);
            }
        }
    }));
}

/// Whether Windows shows apps dark (the launcher follows it too).
pub fn system_dark() -> bool {
    theme::system_dark()
}

/// The monitor (its device name) of the dock whose window is `hwnd`.
pub fn monitor_of(hwnd: isize) -> Option<String> {
    shared()
        .docks
        .iter()
        .find(|placed| placed.hwnd == hwnd)
        .map(|placed| placed.monitor.clone())
}

/// Whether a dock is on screen.
pub fn is_visible() -> bool {
    shared().docks.iter().any(|placed| placed.visible)
}

/// The dock the panel anchors to: the one whose item opened it, or the
/// primary monitor's.
pub(super) fn anchor_dock(shared: &Shared) -> Option<&Placed> {
    shared
        .docks
        .iter()
        .find(|placed| placed.hwnd == shared.active)
        .or_else(|| shared.docks.first())
}

/// The anchoring dock's window origin on the screen (physical pixels), its
/// scale and its bar on the screen: where the panels anchor.
pub fn geometry() -> Option<((i32, i32), f64, Rect)> {
    let shared = shared();
    let placed = anchor_dock(&shared)?;
    Some(((placed.window.x, placed.window.y), placed.scale, placed.bar))
}

/// The work area of the anchoring dock's monitor, and its scale.
pub fn work_area() -> Option<(Rect, f64)> {
    let shared = shared();
    let placed = anchor_dock(&shared)?;
    Some((placed.work, placed.scale))
}

/// Whether the user clicked a dock a moment ago: the panel that click opens
/// may take the focus.
pub fn clicked_recently() -> bool {
    shared()
        .clicked
        .is_some_and(|at| at.elapsed() < CLICK_GRANTS_FOCUS)
}

/// Keeps in `map` only the entries whose key is in `keys`: a cache of
/// pictures holds what is shown now, not all that ever was.
pub(super) fn keep_only<K, V>(map: &mut HashMap<K, V>, keys: &HashSet<K>)
where
    K: std::hash::Hash + Eq,
{
    map.retain(|key, _| keys.contains(key));
}
