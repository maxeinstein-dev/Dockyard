//! The dock, drawn natively (spec 0011 H3, ADR 0011): a layered window
//! painted with Direct2D, with no WebView2. The WebView2 is loaded only
//! when a panel or the Configurações app opens.
//!
//! One dock per monitor (spec 0010 D4b), each with its own window, its own
//! band on its monitor's edge and its own full screen rule; they show the
//! same items.
//!
//! Everything about the docks lives on the main thread (the one of the event
//! loop, which also runs their windows' procedure). Other threads reach them
//! through `post`, which queues the work and wakes them; what other threads
//! need to read (whether they are on screen, where they are) is kept in
//! `SHARED`.

pub(crate) mod access;
mod api;
mod autohide;
mod card;
mod drag;
mod faces;
mod failures;
mod hotkeys;
mod keyboard;
mod keys;
mod look;
mod menus;
mod paint;
mod strip;
mod svg;
mod theme;

pub use api::*;
use autohide::*;
use failures::*;
pub use hotkeys::capture_hotkey;
use hotkeys::*;
pub use menus::tray_menu;
use menus::*;

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::Graphics::Direct2D::ID2D1Bitmap;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    ReleaseCapture, SetCapture, TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent,
};
use windows::Win32::UI::WindowsAndMessaging::PBT_APMPOWERSTATUSCHANGE;
use windows::Win32::UI::WindowsAndMessaging::{
    CS_DBLCLKS, CreateWindowExW, DefWindowProcW, DestroyCursor, DestroyWindow, GetWindowRect,
    HCURSOR, HWND_MESSAGE, HWND_TOPMOST, IDC_ARROW, KillTimer, LoadCursorW, RegisterClassW,
    SPI_GETCLIENTAREAANIMATION, SW_HIDE, SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SetCursor, SetTimer, SetWindowPos, ShowWindow,
    SystemParametersInfoW, WM_ACTIVATE, WM_APP, WM_CAPTURECHANGED, WM_CLOSE, WM_DISPLAYCHANGE,
    WM_DPICHANGED, WM_GETOBJECT, WM_HOTKEY, WM_KEYDOWN, WM_KILLFOCUS, WM_LBUTTONDBLCLK,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL,
    WM_POWERBROADCAST, WM_RBUTTONUP, WM_SETTINGCHANGE, WM_SYSKEYDOWN, WM_TIMER, WNDCLASSW,
    WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};
use windows::core::{PCWSTR, w};

use self::paint::{Dot, Face, FocusRing, Painter, Scene, Status, StripCell, Tile, Tip};
use self::strip::Cell;
use self::theme::{Color, Palette};
use super::dock::Rect;
use super::menu;
use super::monitors::Monitor;
use super::panel::{LogicalRect, PanelId};
use crate::domain::apps::AppsSnapshot;
use crate::domain::badges::{self, Badge};
use crate::domain::dock_items::{self, Item, Names, SystemItem};
use crate::domain::dock_layout::{
    self, Area, DRAG_GROW, DRAG_LIFT, DRAG_THRESHOLD, DockSize, Hover, Magnification, Shape,
    drag_offset, drag_pitch, drop_index, make_room, slide_step,
};
use crate::domain::open_apps::OpenSnapshot;
use crate::domain::power::Poll;
use crate::domain::settings::{DockEdge, MiddleClick, OpenAnimation, Settings, TaskbarMode, Theme};
use crate::domain::widgets::{WidgetConfig, WidgetsSnapshot};
use crate::domain::workspaces::{SpacerSize, Workspace};
use crate::persistence::database::Storage;

/// The class of the docks' windows (and of the hidden one that wakes them).
const CLASS: PCWSTR = w!("DockMaxDock");

/// `WM_MOUSELEAVE`, which the features in use do not name.
const WM_MOUSELEAVE: u32 = 0x02A3;
/// Runs the work queued by `post`.
const WM_RUN: u32 = WM_APP + 0x31;

const ANIMATION_TIMER: usize = 1;
const CLOCK_TIMER: usize = 2;
const WARM_TIMER: usize = 3;
/// The pointer rested on an open program for the preview's delay (spec
/// 0015).
const PREVIEW_TIMER: usize = 6;
const STATUS_TIMER: usize = 4;
/// The pointer rested on the Calendário: its card shows (spec 0017 F2a-4).
const CARD_TIMER: usize = 8;
/// How long the pointer rests before the Calendário's card shows.
const CARD_DELAY: u32 = 450;
/// Reads the processor, the memory and the network again while a system
/// widget is on the dock (spec 0017 F1).
const ACTIVITY_TIMER: usize = 7;
/// The next frame of a long text that rolls (the Tocando agora, F4).
const MARQUEE_TIMER: usize = 9;
/// Looks at the pointer while the dock hides on its own.
const AUTO_HIDE_TIMER: usize = 5;
/// The key of a workspace dot for assistive technology, before its id.
const WORKSPACE_KEY: &str = "workspace:";
const AUTO_HIDE_EVERY: u32 = 100;

/// How often the battery, the network and the volume are read again, and
/// the system widgets: `domain::power::every`, longer while the battery is
/// saved (spec 0021), in the milliseconds of `SetTimer`.
fn timer_every(poll: Poll) -> u32 {
    u32::try_from(super::power::every(poll).as_millis()).unwrap_or(u32::MAX)
}

/// A frame of the animations.
const FRAME: Duration = Duration::from_millis(16);
/// How long a program's icon animates after a click.
const OPENING: Duration = Duration::from_millis(1500);
/// How long the pointer rests on an item before its panel is warmed
/// (spec 0011 H2).
const WARM_DELAY: u32 = 150;
/// For how long after a click on the dock the panel it opens may take the
/// focus: the click is the last input, so Windows lets Dockyard have it.
const CLICK_GRANTS_FOCUS: Duration = Duration::from_secs(2);

/// What other threads read about one dock.
#[derive(Debug, Clone)]
struct Placed {
    hwnd: isize,
    /// Its monitor: its key, its handle (an `HMONITOR` as a number), its
    /// rectangle and its work area.
    monitor: String,
    handle: isize,
    rect: Rect,
    work: Rect,
    visible: bool,
    /// The window on the screen, in physical pixels.
    window: Rect,
    scale: f64,
    /// The bar on the screen, in physical pixels.
    bar: Rect,
}

/// What other threads read about the docks (one per monitor).
#[derive(Debug, Default)]
struct Shared {
    /// The primary monitor's first.
    docks: Vec<Placed>,
    /// The window of the dock whose item opened the panel last: the panel
    /// anchors to it.
    active: isize,
    /// When a dock was last clicked.
    clicked: Option<Instant>,
}

static SHARED: Mutex<Shared> = Mutex::new(Shared {
    docks: Vec::new(),
    active: 0,
    clicked: None,
});

fn shared() -> std::sync::MutexGuard<'static, Shared> {
    SHARED.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The hidden window that wakes the docks for the work of other threads
/// (0 before it is made).
static WAKE: AtomicIsize = AtomicIsize::new(0);

/// Whether the docks are on screen, as the user (or the app) last said: a
/// dock made for a monitor plugged in meanwhile shows only then.
static SHOWN: AtomicBool = AtomicBool::new(false);

static APP: std::sync::OnceLock<AppHandle> = std::sync::OnceLock::new();

type Work = super::native_window::Work<Vec<Dock>>;

static QUEUE: super::native_window::WorkQueue<Vec<Dock>> =
    super::native_window::WorkQueue::new(&WAKE, WM_RUN);

thread_local! {
    /// One per monitor, the primary monitor's first.
    static DOCKS: RefCell<Vec<Dock>> = const { RefCell::new(Vec::new()) };
}

/// Something the dock asks of the rest of the app, done once the dock's own
/// state is no longer borrowed (it may call back into the dock).
type Effect = Box<dyn FnOnce(&AppHandle)>;

/// A press on an item that may become a drag.
#[derive(Debug, Clone, Copy)]
struct Press {
    index: usize,
    /// The item's group, where it can go: its first index and its size.
    group: (usize, usize),
    start_x: f64,
    dragging: bool,
    /// Off the dock: a program's tile on its way to a folder (spec 0018).
    out: bool,
}

/// A drag let go that moved its item: the item stays where it was let go,
/// and the others keep the room they made, until the dock reads the new
/// order (or for `LANDING`, when it never comes).
#[derive(Debug, Clone, Copy)]
struct Landing {
    press: Press,
    /// The pointer's x when it was let go.
    x: f64,
    since: Instant,
}

/// The longest a drag let go waits for the new order.
const LANDING: Duration = Duration::from_secs(1);

/// How far the items of a workspace just shown start along the bar, in
/// logical pixels, before they glide to their places (spec 0013 C3).
const SWITCH_SLIDE: f64 = 36.0;

impl Press {
    /// The item's place in its group.
    fn local(&self) -> usize {
        self.index - self.group.0
    }

    /// The indices of its group.
    fn range(&self) -> std::ops::Range<usize> {
        self.group.0..self.group.0 + self.group.1
    }
}

/// The cover of a track, and the icon of its program, as bitmaps (F4).
type MediaArt = (
    (String, String, String),
    Option<ID2D1Bitmap>,
    Option<ID2D1Bitmap>,
);

/// What the dock reads from the rest of the app.
struct Data {
    settings: Settings,
    widgets: Option<WidgetsSnapshot>,
    apps: Option<AppsSnapshot>,
    open: OpenSnapshot,
    /// The workspace the dock's monitor shows (spec 0013), and every one,
    /// for the switcher; `None` and empty when they can't be read.
    workspace: Option<Workspace>,
    workspaces: Vec<Workspace>,
    /// The icons of the notification area (while the dock holds it).
    tray: Vec<crate::commands::tray::TrayIcon>,
    /// The folder skin of the Custom style (spec 0020, Q14), read here so
    /// the paint never touches a file; `None` for any other style, or while
    /// it can't be read (the bar is then Nativo's).
    custom_skin: Option<Arc<crate::domain::skin::CachedSkin>>,
}

/// A program's icon: being read, or read (`None` when it has none).
enum Loaded {
    Loading,
    Ready(Option<ID2D1Bitmap>),
}

pub struct Dock {
    app: AppHandle,
    hwnd: HWND,
    /// The monitor it is on.
    monitor: Monitor,
    painter: Painter,
    data: Data,
    items: Vec<Item>,
    names: Vec<String>,
    groups: Vec<usize>,
    label_widths: Vec<f64>,
    dark: bool,
    /// Windows' contrast colors while its contrast mode is on (M6): the
    /// dock draws with them alone.
    contrast: Option<theme::Contrast>,
    motion: bool,
    /// The window in physical pixels and the monitor's scale.
    window: Rect,
    scale: f64,
    /// Each item's growth as drawn now, moving toward its target.
    growth: Vec<Magnification>,
    hovered: Option<usize>,
    status_hovered: bool,
    /// The workspace dot under the pointer (spec 0013).
    switcher_hovered: Option<usize>,
    /// The small icons at the end, and the one under the pointer.
    strip: Vec<Cell>,
    strip_hovered: Option<usize>,
    /// The battery, the network and the volume, as last read.
    system: Option<super::system_status::Status>,
    /// The last reading for the system widgets; `None` while none is on
    /// the dock, when nothing is read.
    activity: Option<crate::domain::system_activity::Reading>,
    /// How far each item has risen above the bar, 0 to 1 (a Calendário
    /// under the pointer).
    widen: Vec<f64>,
    /// The tray icons' images, by icon key: the image they came from and
    /// the bitmap.
    tray_images: HashMap<String, (String, Option<ID2D1Bitmap>)>,
    /// The item held down by the left button.
    pressed: Option<usize>,
    press: Option<Press>,
    /// While dragging: the pointer's x.
    drag_x: f64,
    /// The icon the pointer shows while a program's tile is off the dock.
    drag_cursor: Option<HCURSOR>,
    /// The tile under files dragged over the dock (spec 0018).
    drop_hover: Option<usize>,
    /// The cover of what plays now, as a bitmap, with its track, and the
    /// icon of the program that plays it (F4).
    media_art: Option<MediaArt>,
    /// When the track now playing showed: where its long title rolls to.
    media_since: Instant,
    landing: Option<Landing>,
    /// How far each item is drawn from its place at rest, sideways: the
    /// room made for a dragged item, or the way to a new place, eased.
    slide: Vec<f64>,
    /// The panel open, anchored to one of the items.
    open: Option<PanelId>,
    /// The programs whose icon is animating after a click, by item key.
    opening: HashMap<String, Instant>,
    /// The battery saver is on (spec 0021): nothing moves on its own (the
    /// rolling title, the jump of a program opening) and the system is read
    /// less often.
    saving: bool,
    icons: HashMap<(Option<String>, String, bool), Loaded>,
    badges: HashMap<String, Loaded>,
    /// The notification badge of each item (spec 0016), as the taskbar
    /// last had it.
    notices: Vec<Option<Badge>>,
    tracking: bool,
    animating: bool,
    visible: bool,
    /// Hidden because a program is in full screen on its monitor (spec 0010
    /// D4): it comes back when that ends.
    hidden_for_fullscreen: bool,
    /// Hiding on its own: how far off the screen the dock is drawn (0 in
    /// place, 1 all out), whether it is headed out, and since when the
    /// pointer is on the edge or away.
    tucked: f64,
    tucking: bool,
    on_edge_since: Option<Instant>,
    away_since: Option<Instant>,
    /// The dock's menu is open (it runs a loop of its own).
    menu_open: bool,
    /// The item the keyboard is on (M14), by key: one of `nav_keys`. Only
    /// while the dock has the keyboard, which only its hotkey gives it.
    keyboard: Option<String>,
    /// The window that had the keyboard before the hotkey: Esc gives it
    /// back.
    before: Option<HWND>,
    /// The next menu is the keyboard's: by the item, not at the pointer.
    menu_by_keyboard: bool,
    effects: Vec<Effect>,
}

/// Queues `work` for the docks, from any thread, and wakes them.
fn queue(work: Work) {
    QUEUE.push(work);
}

/// Queues `work` for every dock.
pub fn post(work: impl Fn(&mut Dock) + Send + 'static) {
    queue(Box::new(move |docks| {
        for dock in docks.iter_mut() {
            work(dock);
        }
    }));
}

/// Queues `work` for the dock of the window `hwnd`, if it is still there.
fn post_to(hwnd: isize, work: impl FnOnce(&mut Dock) + Send + 'static) {
    queue(Box::new(move |docks| {
        if let Some(dock) = docks.iter_mut().find(|dock| dock.hwnd.0 as isize == hwnd) {
            work(dock);
        }
    }));
}

/// Queues `work` for the dock of the primary monitor.
#[cfg_attr(not(feature = "e2e"), allow(dead_code))]
fn post_primary(work: impl FnOnce(&mut Dock) + Send + 'static) {
    queue(Box::new(move |docks| {
        if let Some(dock) = docks.first_mut() {
            work(dock);
        }
    }));
}

/// Makes the docks' windows, one per monitor, hidden. On the main thread,
/// once.
pub fn start(app: &AppHandle) -> windows::core::Result<()> {
    // COM and OLE on the main thread, for the images (WIC) and the drop
    // target: WebView2 used to do it, and none runs at startup any more.
    if let Err(error) = unsafe { windows::Win32::System::Ole::OleInitialize(None) } {
        log::warn!("dock: OLE could not be initialized: {error}");
    }
    let _ = APP.set(app.clone());
    let instance = unsafe { GetModuleHandleW(None) }?;
    let class_data = WNDCLASSW {
        style: CS_DBLCLKS,
        lpfnWndProc: Some(procedure),
        hInstance: instance.into(),
        lpszClassName: CLASS,
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW) }.unwrap_or_default(),
        ..Default::default()
    };
    unsafe { RegisterClassW(&class_data) };
    // A window only for messages: what other threads ask of the docks.
    let wake = unsafe {
        CreateWindowExW(
            Default::default(),
            CLASS,
            w!(""),
            Default::default(),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            Some(instance.into()),
            None,
        )
    }?;
    WAKE.store(wake.0 as isize, Ordering::SeqCst);
    with_docks(sync);
    Ok(())
}

/// Makes a dock on `monitor`, hidden.
fn make_dock(app: &AppHandle, monitor: Monitor) -> Option<Dock> {
    let instance = unsafe { GetModuleHandleW(None) }.ok()?;
    // Never activated by a click: the program the user is in stays in
    // front, and a click on a program knows which one that is.
    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            CLASS,
            w!("Dockyard"),
            WS_POPUP,
            0,
            0,
            1,
            1,
            None,
            None,
            Some(instance.into()),
            None,
        )
    }
    .inspect_err(|error| log::warn!("dock: the window could not be made: {error}"))
    .ok()?;
    let painter = match Painter::new() {
        Ok(painter) => painter,
        Err(error) => {
            log::warn!("dock: nothing to paint with: {error}");
            let _ = unsafe { DestroyWindow(hwnd) };
            return None;
        }
    };
    // Files dropped on the dock are pinned (spec 0006 B2).
    super::drop_target::install(app, hwnd);
    let data = read_data(app, &monitor.key);
    let saving = super::power::decide(data.settings.power_saver);
    let dock = Dock {
        app: app.clone(),
        hwnd,
        monitor,
        painter,
        data,
        items: Vec::new(),
        names: Vec::new(),
        groups: Vec::new(),
        label_widths: Vec::new(),
        dark: theme::system_dark(),
        contrast: theme::system_contrast(),
        motion: system_motion(),
        window: Rect {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        },
        scale: 1.0,
        growth: Vec::new(),
        hovered: None,
        status_hovered: false,
        switcher_hovered: None,
        strip: Vec::new(),
        strip_hovered: None,
        system: Some(super::system_status::read_shared()),
        activity: None,
        widen: Vec::new(),
        tray_images: HashMap::new(),
        pressed: None,
        press: None,
        drag_x: 0.0,
        drag_cursor: None,
        drop_hover: None,
        media_art: None,
        media_since: Instant::now(),
        landing: None,
        slide: Vec::new(),
        open: None,
        opening: HashMap::new(),
        saving,
        icons: HashMap::new(),
        badges: HashMap::new(),
        notices: Vec::new(),
        tracking: false,
        animating: false,
        visible: false,
        hidden_for_fullscreen: false,
        tucked: 0.0,
        tucking: false,
        on_edge_since: None,
        away_since: None,
        menu_open: false,
        keyboard: None,
        before: None,
        menu_by_keyboard: false,
        effects: Vec::new(),
    };
    dock.arm_clock();
    unsafe {
        SetTimer(
            Some(dock.hwnd),
            STATUS_TIMER,
            timer_every(Poll::Status),
            None,
        )
    };
    Some(dock)
}

/// One dock per monitor (spec 0010 D4b), or only the primary monitor's when
/// the user asks for that: a dock is made for a monitor plugged in and
/// destroyed for one unplugged; the others learn where their monitor is now.
fn sync(docks: &mut Vec<Dock>) {
    let Some(app) = APP.get() else {
        return;
    };
    let every = crate::commands::settings::current(app).dock.every_monitor;
    // The primary monitor's first: alone, it is the one kept.
    let mut monitors = super::monitors::all();
    if !every {
        monitors.truncate(1);
    }
    docks.retain_mut(|dock| {
        let keep = monitors
            .iter()
            .any(|monitor| monitor.key == dock.monitor.key);
        if !keep {
            dock.destroy();
        }
        keep
    });
    for monitor in &monitors {
        match docks
            .iter_mut()
            .find(|dock| dock.monitor.key == monitor.key)
        {
            Some(dock) => dock.monitor = monitor.clone(),
            None => {
                if let Some(dock) = make_dock(app, monitor.clone()) {
                    docks.push(dock);
                }
            }
        }
    }
    // The primary monitor's first.
    docks.sort_by_key(|dock| {
        monitors
            .iter()
            .position(|monitor| monitor.key == dock.monitor.key)
    });
    let keys: Vec<String> = docks.iter().map(|dock| dock.monitor.key.clone()).collect();
    super::appbar::retain(&keys);
    let shown = SHOWN.load(Ordering::SeqCst);
    for dock in docks.iter_mut() {
        dock.rebuild();
        if shown && !dock.visible && !dock.hidden_for_fullscreen {
            dock.show();
        }
    }
}

/// Runs `change` on the docks, then what they asked of the app.
fn with_docks(change: impl FnOnce(&mut Vec<Dock>)) {
    let effects = DOCKS.with(|slot| {
        let Ok(mut docks) = slot.try_borrow_mut() else {
            // A message inside another (a modal menu): this one is lost.
            log::debug!("dock: busy; a message was dropped");
            return Vec::new();
        };
        change(&mut docks);
        docks
            .iter_mut()
            .flat_map(|dock| std::mem::take(&mut dock.effects))
            .collect::<Vec<Effect>>()
    });
    if let Some(app) = APP.get() {
        for effect in effects {
            effect(app);
        }
    }
}

/// Runs `change` on the dock of the window `hwnd`.
fn with_dock(hwnd: HWND, change: impl FnOnce(&mut Dock)) {
    with_docks(|docks| {
        if let Some(dock) = docks.iter_mut().find(|dock| dock.hwnd == hwnd) {
            change(dock);
        }
    });
}

/// The work queued by other threads, in order.
fn run_queue() {
    QUEUE.drain(with_docks);
}

/// What the dock on the monitor `monitor` (its device name) shows.
fn read_data(app: &AppHandle, monitor: &str) -> Data {
    let settings = crate::commands::settings::current(app);
    #[cfg(windows)]
    let tray = crate::commands::tray::icons();
    let (widgets, apps, workspace, workspaces): (_, _, Option<Workspace>, _) =
        match app.try_state::<Mutex<Storage>>() {
            Some(state) => {
                let mut storage = state.lock().unwrap_or_else(PoisonError::into_inner);
                (
                    crate::commands::widgets::snapshot(&storage).ok(),
                    crate::commands::apps::snapshot(&storage).ok(),
                    storage
                        .shown_on(monitor)
                        .inspect_err(|error| log::warn!("dock: no workspace to show: {error}"))
                        .ok(),
                    storage.workspaces().unwrap_or_default(),
                )
            }
            None => (None, None, None, Vec::new()),
        };
    let custom_skin = crate::commands::skins::chosen(app, &settings.dock);
    Data {
        custom_skin,
        settings,
        widgets,
        apps,
        open: super::open_apps::snapshot_for(workspace.as_ref().map(|w| w.id.as_str())),
        workspace,
        workspaces,
        tray,
    }
}

/// Whether Windows animates (its "animation effects"); on when it can't be
/// told.
fn system_motion() -> bool {
    let mut on = windows::core::BOOL(1);
    let read = unsafe {
        SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            Some(std::ptr::from_mut(&mut on).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    };
    read.is_err() || on.as_bool()
}

fn names() -> Names {
    let dock = &crate::i18n::messages().dock;
    Names {
        settings: dock.settings,
        clock: dock.clock,
        time_progress: dock.time_progress,
        note: dock.note,
        shortcut: dock.shortcut,
        files: dock.files,
        activity: dock.activity,
        network: dock.network,
        calendar: dock.calendar,
        cycle: dock.cycle,
        countdown: dock.countdown,
        pomodoro: dock.pomodoro,
        todo: dock.todo,
        water: dock.water,
        media: dock.media,
        claude: dock.claude,
        weather: dock.weather,
        launcher: dock.launcher,
    }
}

/// Whether `item` is a Lançador (spec 0012): a click opens the launcher,
/// not a panel.
fn is_launcher(item: &Item) -> bool {
    matches!(item, Item::Widget(widget) if matches!(widget.config, WidgetConfig::Launcher(_)))
}

/// The panel an item opens: a program's is its menu. A spacer has none.
fn panel_of(item: &Item) -> Option<PanelId> {
    match item {
        Item::App(app) => Some(PanelId::App(app.id.clone())),
        Item::Widget(widget) => Some(PanelId::Widget(widget.id.clone())),
        Item::Other(other) => Some(PanelId::Open(other.key.clone())),
        Item::System(SystemItem::Settings) => Some(PanelId::Settings),
        Item::Spacer(_) => None,
    }
}

/// What the dock keeps a program's picture by: where it comes from (the
/// icon chosen for it, spec 0024, or the dock's pack; `None` for Windows'
/// own), its id, and whether it is a pack's dark image (only a pack has
/// one, while the bar is `dark`).
fn icon_key(
    dock: &crate::domain::settings::DockSettings,
    app_id: &str,
    dark: bool,
) -> (Option<String>, String, bool) {
    let source = match dock
        .app_icons
        .get(&crate::domain::app_icon::choice_key(app_id))
    {
        Some(chosen) => Some(chosen.source()),
        None => dock.icon_pack.as_ref().map(|pack| format!("pack:{pack}")),
    };
    let dark = dark
        && source
            .as_deref()
            .is_some_and(|source| source.starts_with("pack:"));
    (source, app_id.to_owned(), dark)
}

fn logical(area: Area) -> LogicalRect {
    LogicalRect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: area.height,
    }
}

impl Dock {
    fn edge(&self) -> DockEdge {
        self.data.settings.dock.edge
    }

    fn shape(&self) -> Shape {
        let dock = &self.data.settings.dock;
        Shape {
            size: DockSize::of(dock.icon_size),
            groups: self.groups.clone(),
            hover: Hover::of(dock.hover, dock.hover_scale),
            status: dock.taskbar != TaskbarMode::Visible,
            tray: self.strip.len(),
            switcher: self.data.workspaces.len(),
            spans: self.items.iter().map(Item::span).collect(),
            spacers: self.items.iter().map(Item::spacer_size).collect(),
        }
    }

    /// Whether the panel of `item` is the one open.
    fn is_open(&self, item: &Item) -> bool {
        self.open.is_some() && self.open == panel_of(item)
    }

    /// The window's client area, in logical pixels.
    fn viewport(&self) -> (f64, f64) {
        (
            f64::from(self.window.width) / self.scale,
            f64::from(self.window.height) / self.scale,
        )
    }

    /// The Calendário two icons wide under the pointer, which rises: not
    /// while a panel is open or an item is dragged.
    fn opened_calendar(&self) -> Option<usize> {
        let dragging = self.press.is_some_and(|press| press.dragging);
        if self.open.is_some() || dragging {
            return None;
        }
        self.hovered.filter(|&index| {
            matches!(
                self.items.get(index),
                Some(Item::Widget(widget))
                    if matches!(&widget.config, WidgetConfig::Calendar(config) if config.width == 2)
            )
        })
    }

    /// The item magnified now: the one under the pointer, unless a panel is
    /// open, an item is being dragged or Windows asks for less motion.
    fn magnified(&self) -> Option<usize> {
        let dragging = self.press.is_some_and(|press| press.dragging);
        if !self.motion || self.open.is_some() || dragging {
            return None;
        }
        self.hovered.filter(|&index| index < self.items.len())
    }

    /// Reads the data again and lays the dock out for it. After a switch of
    /// workspace, the items of the new one glide in from the side it lies on
    /// (spec 0013 C3).
    fn reload(&mut self) {
        let position = |data: &Data| {
            let id = &data.workspace.as_ref()?.id;
            data.workspaces.iter().position(|w| &w.id == id)
        };
        let before = position(&self.data);
        self.data = read_data(&self.app, &self.monitor.key);
        // The battery saver may have been turned on or off.
        self.check_power();
        let enter = match (before, position(&self.data)) {
            (Some(before), Some(after)) if after > before => SWITCH_SLIDE,
            (Some(before), Some(after)) if after < before => -SWITCH_SLIDE,
            _ => 0.0,
        };
        self.rebuild_entering(enter);
    }

    /// Lays the dock out for the data it has: the items, their names, the
    /// window, and paints.
    fn rebuild(&mut self) {
        self.rebuild_entering(0.0);
    }

    /// `rebuild`, with the items that were not on the dock coming from
    /// `enter` logical pixels along the bar.
    fn rebuild_entering(&mut self, enter: f64) {
        // Where each item is drawn now, on the screen, by key: from there it
        // slides to its new place.
        let drawn: HashMap<String, f64> = self.drawn_x().collect();
        let chosen_at = self.keyboard_index();
        self.rebuild_layout();
        self.keep_keyboard(chosen_at);
        let rest = self.rest();
        let left = f64::from(self.window.x) / self.scale;
        let (width, _) = self.viewport();
        self.slide = self
            .items
            .iter()
            .zip(&rest.items)
            .map(|(item, area)| {
                let Some(before) = drawn.get(&item.key()) else {
                    return if self.motion { enter } else { 0.0 };
                };
                let offset = before - (left + area.x);
                // Another monitor, or no motion: straight there.
                if !self.motion || offset.abs() > width {
                    0.0
                } else {
                    offset
                }
            })
            .collect();
        self.landing = None;
        self.arm_auto_hide();
        arm_hotkeys(self.data.workspaces.len());
        arm_focus_hotkey();
        self.animate();
        self.paint();
    }

    /// Each item's x as drawn now, on the screen in logical pixels, by key.
    fn drawn_x(&self) -> impl Iterator<Item = (String, f64)> + '_ {
        let rest = self.rest();
        let left = f64::from(self.window.x) / self.scale;
        let shown = self.shown_offsets(&rest);
        self.items
            .iter()
            .zip(rest.items)
            .zip(shown)
            .map(move |((item, area), offset)| (item.key(), left + area.x + offset))
    }

    /// The items, their names and the window for the data the dock has.
    fn rebuild_layout(&mut self) {
        let settings = &self.data.settings;
        let found = dock_items::items(dock_items::Sources {
            widgets: self.data.widgets.as_ref(),
            apps: self.data.apps.as_ref(),
            open: &self.data.open,
            show_open: settings.dock.show_open_apps,
            workspace: self.data.workspace.as_ref(),
        });
        self.strip = strip::cells(
            settings.dock.taskbar,
            self.system.as_ref(),
            &self.data.tray,
            &settings.dock.tray_in_dock,
        );
        if self
            .strip_hovered
            .is_some_and(|index| index >= self.strip.len())
        {
            self.strip_hovered = None;
        }
        self.decode_tray_images();
        // The growth of each item follows it by key across changes.
        let before: HashMap<String, Magnification> = self
            .items
            .iter()
            .map(Item::key)
            .zip(self.growth.iter().copied())
            .collect();
        let hovered_key = self
            .hovered
            .and_then(|index| self.items.get(index))
            .map(Item::key);
        let names = names();
        // The Lançador's name carries the launcher's hotkey, while it has
        // one (spec 0012): it is its tip and what the Narrator reads.
        let settings = &self.data.settings;
        let hotkey = settings
            .launcher
            .then_some(settings.launcher_hotkey.as_str());
        let space = crate::i18n::messages().dock.space_key;
        self.names = found
            .items
            .iter()
            .map(|item| {
                let name = names.of(item);
                if is_launcher(item) {
                    crate::domain::widgets::launcher::tip(&name, hotkey, space)
                } else {
                    name
                }
            })
            .collect();
        self.items = found.items;
        self.arm_activity();
        self.groups = found.groups;
        self.growth = self
            .items
            .iter()
            .map(|item| {
                before.get(&item.key()).copied().unwrap_or(Magnification {
                    scale: 1.0,
                    lift: 0.0,
                })
            })
            .collect();
        self.hovered =
            hovered_key.and_then(|key| self.items.iter().position(|item| item.key() == key));
        let keys: HashSet<String> = self.items.iter().map(Item::key).collect();
        self.opening.retain(|key, _| keys.contains(key));
        if self
            .press
            .is_some_and(|press| press.index >= self.items.len())
        {
            self.press = None;
        }
        // Every label the dock can show: the items', the strip cells' and
        // the workspace dots'.
        let labels: Vec<String> = self
            .names
            .iter()
            .cloned()
            .chain(self.strip.iter().map(Cell::tip))
            .chain(self.data.workspaces.iter().map(|w| w.name.clone()))
            .collect();
        self.label_widths = labels
            .iter()
            .map(|label| self.painter.label_width(label))
            .collect();
        self.notices = self.read_notices();
        self.request_pictures();
        self.place();
    }

    /// The notification badge of each item: a program's, from its button of
    /// the taskbar (spec 0016). None while the setting is off.
    fn read_notices(&self) -> Vec<Option<Badge>> {
        let buttons = if self.data.settings.dock.show_badges {
            super::badges::buttons()
        } else {
            Vec::new()
        };
        self.items
            .iter()
            .map(|item| {
                // A Ciclo's notice not seen yet (F2b-3): whatever the
                // setting, it is the dock's own.
                if let Item::Widget(widget) = item {
                    match widget.config {
                        WidgetConfig::Cycle(_) => {
                            return super::calendar::pending(widget.id.as_str())
                                .then_some(Badge::Dot);
                        }
                        // A stage of the Pomodoro ended (F2c-2).
                        WidgetConfig::Pomodoro(_) => {
                            return super::pomodoro::pending(widget.id.as_str())
                                .then_some(Badge::Dot);
                        }
                        // Time for a cup (F3-2).
                        WidgetConfig::Water(_) => {
                            return super::water::pending(widget.id.as_str()).then_some(Badge::Dot);
                        }
                        _ => {}
                    }
                }
                if buttons.is_empty() {
                    return None;
                }
                match item {
                    Item::App(app) => badges::for_pinned(
                        &buttons,
                        super::open_apps::known_identity(&app.app_id).as_ref(),
                        &app.name,
                    ),
                    Item::Other(other) => badges::for_other(
                        &buttons,
                        super::open_apps::group_key_of_other(&other.key).as_deref(),
                        &other.name,
                    ),
                    _ => None,
                }
            })
            .collect()
    }

    /// The taskbar's badges changed: drawn again if this dock's did.
    fn notices_changed(&mut self) {
        let notices = self.read_notices();
        if notices != self.notices {
            self.notices = notices;
            self.paint();
        }
    }

    /// The images of the tray icons kept in the dock, read again when one
    /// changed.
    fn decode_tray_images(&mut self) {
        use base64::Engine as _;
        // An icon that left the tray takes its image with it.
        let in_tray: HashSet<String> = self
            .strip
            .iter()
            .filter_map(|cell| match cell {
                Cell::Tray(icon) => Some(icon.key.clone()),
                _ => None,
            })
            .collect();
        keep_only(&mut self.tray_images, &in_tray);
        for cell in &self.strip {
            let Cell::Tray(icon) = cell else {
                continue;
            };
            let Some(image) = &icon.image else {
                self.tray_images.remove(&icon.key);
                continue;
            };
            if self
                .tray_images
                .get(&icon.key)
                .is_some_and(|(from, _)| from == image)
            {
                continue;
            }
            let bitmap = image
                .strip_prefix("data:image/png;base64,")
                .and_then(|data| base64::engine::general_purpose::STANDARD.decode(data).ok())
                .and_then(|png| self.painter.bitmap(&png));
            self.tray_images
                .insert(icon.key.clone(), (image.clone(), bitmap));
        }
    }

    /// Reads the system for its widgets every `Poll::Activity` while one is
    /// on the dock, and stops when none is: then nothing is read.
    fn arm_activity(&mut self) {
        let calendar = self.items.iter().any(|item| {
            matches!(item, Item::Widget(widget) if matches!(widget.config, WidgetConfig::Calendar(_)))
        });
        let cycles: Vec<_> = self
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Widget(widget) => match &widget.config {
                    WidgetConfig::Cycle(config) => {
                        Some((widget.id.as_str().to_owned(), config.clone()))
                    }
                    _ => None,
                },
                _ => None,
            })
            .collect();
        super::calendar::want(self.hwnd.0 as isize, calendar, cycles);
        let media = self.items.iter().any(|item| {
            matches!(item, Item::Widget(widget) if matches!(widget.config, WidgetConfig::Media(_)))
        });
        super::media::want(self.hwnd.0 as isize, media);
        let claude = self.items.iter().any(|item| {
            matches!(item, Item::Widget(widget) if matches!(widget.config, WidgetConfig::Claude(_)))
        });
        super::claude_usage::want(self.hwnd.0 as isize, claude);
        let cities = self
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Widget(widget) => match &widget.config {
                    WidgetConfig::Weather(config) => config.city.as_ref(),
                    _ => None,
                },
                _ => None,
            })
            .map(|city| super::weather::key(city.latitude, city.longitude))
            .collect();
        super::weather::want(self.hwnd.0 as isize, cities);
        let wanted = self.items.iter().any(|item| {
            matches!(
                item,
                Item::Widget(widget) if matches!(
                    widget.config,
                    WidgetConfig::Activity(_) | WidgetConfig::Network(_)
                )
            )
        });
        match (wanted, self.activity.is_some()) {
            (true, false) => {
                self.activity = Some(super::system_activity::sample());
                unsafe {
                    SetTimer(
                        Some(self.hwnd),
                        ACTIVITY_TIMER,
                        timer_every(Poll::Activity),
                        None,
                    )
                };
            }
            (false, true) => {
                self.activity = None;
                let _ = unsafe { KillTimer(Some(self.hwnd), ACTIVITY_TIMER) };
            }
            _ => {}
        }
    }

    fn read_activity(&mut self) {
        if self.activity.is_none() {
            return;
        }
        let reading = super::system_activity::sample();
        if self.activity.as_ref() != Some(&reading) {
            self.activity = Some(reading);
            self.paint();
        }
    }

    /// The battery, the network and the volume, read again; laid out again
    /// when they changed (an icon may come or go). The power line too: the
    /// battery saver follows it within a tick, should `WM_POWERBROADCAST`
    /// not come.
    fn read_system(&mut self) {
        self.check_power();
        let now = Some(super::system_status::read_shared());
        if now != self.system {
            self.system = now;
            self.rebuild();
        }
    }

    /// Decides the battery saver again (spec 0021), from the setting and the
    /// power line; when it turned on or off, the readings take their new
    /// pace and what moved on its own stops, or rolls again.
    fn check_power(&mut self) {
        let saving = super::power::decide(self.data.settings.power_saver);
        if saving == self.saving {
            return;
        }
        self.saving = saving;
        log::info!("dock: battery saver {}", if saving { "on" } else { "off" });
        // The same timer set again: its new interval, from now.
        unsafe {
            SetTimer(
                Some(self.hwnd),
                STATUS_TIMER,
                timer_every(Poll::Status),
                None,
            )
        };
        if self.activity.is_some() {
            unsafe {
                SetTimer(
                    Some(self.hwnd),
                    ACTIVITY_TIMER,
                    timer_every(Poll::Activity),
                    None,
                )
            };
        }
        if saving {
            self.opening.clear();
        }
        // A rolling title stops at its start, or starts over.
        self.media_since = Instant::now();
        self.paint();
    }

    /// Animates `key`'s icon as a program opening, unless the battery is
    /// saved: then it stays still (spec 0021). Whether it animates.
    fn start_opening(&mut self, key: String, now: Instant) -> bool {
        if self.saving {
            return false;
        }
        self.opening.insert(key, now);
        true
    }

    /// The cells of the strip, where they are drawn now.
    fn strip_cells(&self) -> Vec<Area> {
        let icon = self.shape().size.geometry().icon;
        self.placement_now()
            .tray
            .map(|area| dock_layout::tray_cells(area, self.strip.len(), icon))
            .unwrap_or_default()
    }

    /// `tray_drop_index`, on the dock's thread.
    fn tray_drop_index(&self, x: i32, y: i32) -> Option<Option<String>> {
        if !self.visible {
            return None;
        }
        let x = f64::from(x - self.window.x) / self.scale;
        let y = f64::from(y - self.window.y) / self.scale;
        let placement = self.placement_now();
        // The bar, with a little room around it: a drop is not aimed to the
        // pixel.
        let bar = placement.bar;
        let near = x >= bar.x - 8.0
            && x < bar.x + bar.width + 8.0
            && y >= bar.y - 8.0
            && y < bar.y + bar.height + 8.0;
        if !near {
            return None;
        }
        // From the cell under the pointer on (before network, volume and
        // battery, after the arrow), the first tray icon kept in the dock.
        let from = self.strip_hit(x, y).unwrap_or(self.strip.len());
        Some(self.strip[from..].iter().find_map(|cell| match cell {
            Cell::Tray(icon) => icon.id.clone(),
            _ => None,
        }))
    }

    /// The cell of the strip under the point.
    fn strip_hit(&self, x: f64, y: f64) -> Option<usize> {
        self.strip_cells()
            .iter()
            .position(|cell| cell.contains(x, y))
    }

    /// A click on a cell of the strip: the arrow opens the tray panel, the
    /// battery, the network and the volume the quick settings of Windows, a
    /// tray icon its program's own action.
    fn strip_click(&mut self, index: usize, right: bool) {
        let Some(cell) = self.strip.get(index).cloned() else {
            return;
        };
        match cell {
            Cell::Arrow if !right => {
                let Some(area) = self.strip_cells().get(index).copied() else {
                    return;
                };
                self.strip_hovered = None;
                self.toggle(PanelId::Tray, area);
            }
            Cell::Tray(icon) => {
                let button = if right {
                    super::tray_host::Button::Right
                } else {
                    super::tray_host::Button::Left
                };
                // Its cell on the screen: the program opens its menu there.
                let place = self.strip_cells().get(index).map(|area| {
                    (
                        self.window.x + (area.x * self.scale).round() as i32,
                        self.window.y + (area.y * self.scale).round() as i32,
                        (area.width * self.scale).round() as i32,
                        (area.height * self.scale).round() as i32,
                    )
                });
                self.effects.push(Box::new(move |_| {
                    if let Err(error) = super::tray_host::click_at(&icon.key, button, place) {
                        log::warn!("dock: a tray icon refused the click: {error}");
                    }
                }));
            }
            Cell::Network(_) | Cell::Volume(_) | Cell::Battery(_) if !right => {
                self.effects.push(Box::new(|_| {
                    if let Err(error) = super::flyout::open(super::flyout::Flyout::QuickSettings) {
                        log::warn!("dock: the quick settings could not be opened: {error}");
                    }
                }));
            }
            _ => self.dock_menu(None),
        }
    }

    /// Asks for the icons and badges not read yet, off the main thread, and
    /// lets go of the ones no item shows any more (a program closed, another
    /// pack or theme): the dock is open all day, so they must not pile up.
    fn request_pictures(&mut self) {
        let settings = &self.data.settings.dock;
        let dark = self.bar_dark();
        let hwnd = self.hwnd.0 as isize;
        let shown: HashSet<_> = self
            .items
            .iter()
            .filter_map(|item| match item {
                Item::App(app) => Some(icon_key(settings, &app.app_id, dark)),
                Item::Other(other) => Some(icon_key(settings, &other.app_id, dark)),
                _ => None,
            })
            .collect();
        keep_only(&mut self.icons, &shown);
        let badged: HashSet<String> = self
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Other(other) if other.badge => Some(other.key.clone()),
                _ => None,
            })
            .collect();
        keep_only(&mut self.badges, &badged);
        for item in &self.items {
            let app_id = match item {
                Item::App(app) => app.app_id.clone(),
                Item::Other(other) => other.app_id.clone(),
                _ => continue,
            };
            let key = icon_key(&self.data.settings.dock, &app_id, dark);
            if !self.icons.contains_key(&key) {
                self.icons.insert(key.clone(), Loaded::Loading);
                let app = self.app.clone();
                // A pack's dark images while the dock is dark.
                let dark = key.2;
                std::thread::spawn(move || {
                    let png = crate::commands::apps::dock_icon_png(&app, &app_id, dark);
                    post_to(hwnd, move |dock| {
                        let bitmap = png.and_then(|png| dock.painter.bitmap(&png));
                        dock.icons.insert(key, Loaded::Ready(bitmap));
                        dock.paint();
                    });
                });
            }
            if let Item::Other(other) = item
                && other.badge
                && !self.badges.contains_key(&other.key)
            {
                self.badges.insert(other.key.clone(), Loaded::Loading);
                let key = other.key.clone();
                std::thread::spawn(move || {
                    let png = super::open_apps::aumid_of_other(&key)
                        .and_then(|aumid| super::profiles::badge_png(&aumid));
                    post_to(hwnd, move |dock| {
                        let bitmap = png.and_then(|png| dock.painter.bitmap(&png));
                        dock.badges.insert(key, Loaded::Ready(bitmap));
                        dock.paint();
                    });
                });
            }
        }
    }

    /// Puts the window where the settings say, the size its content needs.
    fn place(&mut self) {
        let shape = self.shape();
        let size = dock_layout::window_size(&shape, self.motion, &self.label_widths);
        let bar_height = {
            let geometry = shape.size.geometry();
            geometry.icon + 2.0 * geometry.pad
        };
        let Some((window, scale)) = super::dock::place_native(
            &self.app,
            &self.data.settings.dock,
            &self.monitor,
            size,
            bar_height,
        ) else {
            return;
        };
        if (scale - self.scale).abs() > f64::EPSILON {
            self.painter.set_scale(scale);
        }
        self.window = window;
        self.scale = scale;
        self.publish();
    }

    /// Tells the other threads where the dock is.
    fn publish(&self) {
        let rest = dock_layout::place(&self.shape(), self.edge(), self.viewport(), None);
        let to_screen = |area: Area| Rect {
            x: self.window.x + (area.x * self.scale).round() as i32,
            y: self.window.y + (area.y * self.scale).round() as i32,
            width: (area.width * self.scale).round().max(0.0) as u32,
            height: (area.height * self.scale).round().max(0.0) as u32,
        };
        let hwnd = self.hwnd.0 as isize;
        let placed = Placed {
            hwnd,
            monitor: self.monitor.key.clone(),
            handle: self.monitor.handle,
            rect: self.monitor.rect,
            work: self.monitor.work,
            visible: self.visible,
            window: self.window,
            scale: self.scale,
            bar: to_screen(rest.bar),
        };
        {
            let mut shared = shared();
            match shared.docks.iter_mut().find(|known| known.hwnd == hwnd) {
                Some(known) => *known = placed,
                None => shared.docks.push(placed),
            }
            // The primary monitor's first, as the docks are.
            let primary = shared
                .docks
                .iter()
                .position(|known| known.monitor == self.monitor.key && self.monitor.primary);
            if let Some(primary) = primary {
                let first = shared.docks.remove(primary);
                shared.docks.insert(0, first);
            }
        }
        // For assistive technology: each item's name (with the notice on
        // Configurações) and its box on the screen.
        let attention = dock_items::needs_attention(self.data.widgets.as_ref());
        let notice = crate::i18n::messages().dock.with_notice;
        // The workspace dots first, as on the screen (spec 0013 C3).
        let text = &crate::i18n::messages().dock;
        let shown = self.data.workspace.as_ref().map(|w| w.id.as_str());
        let side = 2.0 * dock_layout::ACTIVE_DOT_RADIUS;
        let dots: Vec<access::Entry> = rest
            .switcher
            .map(|area| dock_layout::switcher_dots(area, self.data.workspaces.len()))
            .unwrap_or_default()
            .into_iter()
            .zip(&self.data.workspaces)
            .map(|((x, y), workspace)| {
                let on_screen = to_screen(Area {
                    x: x - side / 2.0,
                    y: y - side / 2.0,
                    width: side,
                    height: side,
                });
                let name = if Some(workspace.id.as_str()) == shown {
                    text.workspace_shown
                } else {
                    text.workspace
                };
                access::Entry {
                    key: format!("{WORKSPACE_KEY}{}", workspace.id),
                    name: name.replace("{name}", &workspace.name),
                    rect: (
                        f64::from(on_screen.x),
                        f64::from(on_screen.y),
                        f64::from(on_screen.width),
                        f64::from(on_screen.height),
                    ),
                }
            })
            .collect();
        access::publish(
            hwnd,
            dots.into_iter()
                .chain(
                    self.items
                        .iter()
                        .zip(&self.names)
                        .zip(&rest.items)
                        // A spacer is room, not a button: the Narrator
                        // goes from the item before it to the one after.
                        .filter(|((item, _), _)| !matches!(item, Item::Spacer(_)))
                        .map(|((item, name), area)| {
                            let on_screen = to_screen(*area);
                            access::Entry {
                                key: item.key(),
                                name: if attention
                                    && matches!(item, Item::System(SystemItem::Settings))
                                {
                                    notice.replace("{name}", name)
                                } else {
                                    name.clone()
                                },
                                rect: (
                                    f64::from(on_screen.x),
                                    f64::from(on_screen.y),
                                    f64::from(on_screen.width),
                                    f64::from(on_screen.height),
                                ),
                            }
                        })
                        .chain(
                            self.strip
                                .iter()
                                .zip(self.strip_cells())
                                .map(|(cell, area)| {
                                    let on_screen = to_screen(area);
                                    access::Entry {
                                        key: cell.key(),
                                        name: cell.tip(),
                                        rect: (
                                            f64::from(on_screen.x),
                                            f64::from(on_screen.y),
                                            f64::from(on_screen.width),
                                            f64::from(on_screen.height),
                                        ),
                                    }
                                }),
                        ),
                )
                .collect(),
        );
    }

    /// Presses the item `key` (assistive technology), as a click would.
    fn press(&mut self, key: &str) {
        if let Some(id) = key.strip_prefix(WORKSPACE_KEY) {
            if let Some(index) = self.data.workspaces.iter().position(|w| w.id == id) {
                self.switch_to(index);
            }
        } else if let Some(index) = self.items.iter().position(|item| item.key() == key) {
            self.click(index);
            self.paint();
        } else if let Some(index) = self.strip.iter().position(|cell| cell.key() == key) {
            self.strip_click(index, false);
            self.paint();
        }
    }

    /// The dock's monitor is gone (or the user keeps only one dock).
    fn destroy(&mut self) {
        self.end_preview();
        self.hide();
        let hwnd = self.hwnd.0 as isize;
        shared().docks.retain(|known| known.hwnd != hwnd);
        access::forget(hwnd);
        let _ = unsafe { DestroyWindow(self.hwnd) };
    }

    fn show(&mut self) {
        self.place();
        self.visible = true;
        self.paint();
        self.show_window();
        self.publish();
    }

    /// Puts the window on the screen, above the others, without the focus.
    fn show_window(&self) {
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
            let _ = SetWindowPos(
                self.hwnd,
                Some(HWND_TOPMOST),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
    }

    /// The item under the point (logical, in the window), as drawn now: the
    /// one whose column holds it, inside the bar or on a grown tile. Never a
    /// spacer (spec 0022): see `spacer_hit`.
    fn hit(&self, x: f64, y: f64) -> Option<usize> {
        dock_layout::hit(&self.shape(), &self.placement_now(), x, y)
    }

    /// The spacer under the point, as drawn now.
    fn spacer_hit(&self, x: f64, y: f64) -> Option<usize> {
        dock_layout::spacer_hit(&self.shape(), &self.placement_now(), x, y)
    }

    fn placement_now(&self) -> dock_layout::Placement {
        let growth = self.growth.clone();
        dock_layout::place_with(&self.shape(), self.edge(), self.viewport(), &move |index| {
            growth.get(index).copied().unwrap_or(Magnification {
                scale: 1.0,
                lift: 0.0,
            })
        })
    }

    fn rest(&self) -> dock_layout::Placement {
        dock_layout::place(&self.shape(), self.edge(), self.viewport(), None)
    }

    /// Starts the frames, if anything moves.
    fn animate(&mut self) {
        if !self.animating {
            self.animating = true;
            unsafe {
                SetTimer(
                    Some(self.hwnd),
                    ANIMATION_TIMER,
                    FRAME.as_millis() as u32,
                    None,
                )
            };
        }
    }

    /// One frame: each item moves a part of the way to its growth. Stops the
    /// frames once nothing moves.
    fn step(&mut self) {
        let shape = self.shape();
        let magnified = self.magnified();
        let mut moving = false;
        // A Calendário two icons wide rises above the bar under the
        // pointer, as wide as three icons, to show the meeting after the
        // next one (spec 0017 F2a); its neighbours stay where they are.
        let opened = self.opened_calendar();
        self.widen.resize(self.items.len(), 0.0);
        for (index, now) in self.widen.iter_mut().enumerate() {
            let target = if opened == Some(index) { 1.0 } else { 0.0 };
            if !self.motion || (*now - target).abs() < 0.01 {
                *now = target;
            } else {
                *now += (target - *now) * 0.3;
                moving = true;
            }
        }
        for (index, now) in self.growth.iter_mut().enumerate() {
            let target = shape.hover.of_item(index, magnified);
            if !self.motion {
                *now = target;
                continue;
            }
            let next = |from: f64, to: f64| from + (to - from) * 0.35;
            let scale = next(now.scale, target.scale);
            let lift = next(now.lift, target.lift);
            if (scale - target.scale).abs() < 0.002 && (lift - target.lift).abs() < 0.05 {
                *now = target;
            } else {
                *now = Magnification { scale, lift };
                moving = true;
            }
        }
        if self
            .landing
            .is_some_and(|landing| landing.since.elapsed() >= LANDING)
        {
            // The new order never came: back to where it was.
            self.landing = None;
        }
        let rest = self.rest();
        let targets = self.slide_targets(&rest);
        let held = self.shown_drag().map(|(press, _)| press.index);
        self.slide.resize(self.items.len(), 0.0);
        for (index, (now, target)) in self.slide.iter_mut().zip(targets).enumerate() {
            // The dragged item is with the pointer, not behind it.
            *now = if !self.motion || held == Some(index) {
                target
            } else {
                slide_step(*now, target)
            };
            if *now != target {
                moving = true;
            }
        }
        let out = if self.tucking { 1.0 } else { 0.0 };
        if self.tucked != out {
            self.tucked = if self.motion {
                tuck_step(self.tucked, out)
            } else {
                out
            };
            if self.tucked == 1.0 {
                // All out: off the screen, and off any monitor beyond it.
                let _ = unsafe { ShowWindow(self.hwnd, SW_HIDE) };
            } else if self.tucked != out {
                moving = true;
            }
        }
        let now = Instant::now();
        self.opening
            .retain(|_, started| now.saturating_duration_since(*started) < OPENING);
        let dragging = self.press.is_some_and(|press| press.dragging) || self.landing.is_some();
        if !moving && self.opening.is_empty() && !dragging {
            self.animating = false;
            let _ = unsafe { KillTimer(Some(self.hwnd), ANIMATION_TIMER) };
        }
        self.paint();
    }

    /// Repaints at the next minute, for the clock and the live tiles.
    fn arm_clock(&self) {
        let now = local_now();
        let left = 60_000 - (u32::from(now.wSecond) * 1000 + u32::from(now.wMilliseconds));
        unsafe { SetTimer(Some(self.hwnd), CLOCK_TIMER, left + 50, None) };
    }

    fn paint(&mut self) {
        if !self.visible {
            return;
        }
        #[cfg(feature = "e2e")]
        if PANIC_ON_PAINT.swap(false, Ordering::SeqCst) {
            panic!("e2e: a panic in the dock's paint, on purpose");
        }
        // Tucked all the way out, the window is hidden: nothing to draw, and
        // a rolling title does not wake it. Coming back up paints again.
        if self.tucked >= 1.0 {
            let _ = unsafe { KillTimer(Some(self.hwnd), MARQUEE_TIMER) };
            return;
        }
        let scene = self.scene();
        // Hiding on its own: drawn that far past the screen edge.
        let away = (self.tucked * f64::from(self.window.height)).round() as i32;
        let origin = POINT {
            x: self.window.x,
            y: match self.edge() {
                DockEdge::Bottom => self.window.y + away,
                DockEdge::Top => self.window.y - away,
            },
        };
        if let Err(error) = self.painter.present(
            self.hwnd,
            origin,
            self.window.width as i32,
            self.window.height as i32,
            &scene,
        ) {
            log::warn!("dock: could not paint: {error}");
        }
        // A long title rolls: painted again when it moves next. Not while
        // the battery is saved (spec 0021): it stays at its start.
        match self.painter.marquee.filter(|_| !self.saving) {
            Some(next) => {
                let ms = (next * 1000.0).ceil().clamp(16.0, 60_000.0) as u32;
                unsafe { SetTimer(Some(self.hwnd), MARQUEE_TIMER, ms, None) };
            }
            None => {
                let _ = unsafe { KillTimer(Some(self.hwnd), MARQUEE_TIMER) };
            }
        }
    }

    /// Whether the dock is drawn dark: the setting, or Windows' own.
    fn dark_now(&self) -> bool {
        match self.data.settings.theme {
            Theme::System => self.dark,
            Theme::Light => false,
            Theme::Dark => true,
        }
    }

    /// Whether the bar is dark, for an icon pack's dark images: see
    /// [`DockStyle::bar_dark`](crate::domain::settings::DockStyle::bar_dark).
    fn bar_dark(&self) -> bool {
        let custom = self.data.custom_skin.as_ref();
        self.data
            .settings
            .dock
            .style
            .bar_dark(self.dark_now(), custom.map(|custom| custom.skin.meta.dark))
    }

    fn palette(&self) -> Palette {
        match self.contrast {
            Some(colors) => Palette::contrast(colors),
            None => Palette::of(self.dark_now(), self.data.settings.dock.opacity),
        }
    }

    fn scene(&mut self) -> Scene {
        let settings = self.data.settings.dock.clone();
        let palette = self.palette();
        let shape = self.shape();
        let geometry = shape.size.geometry();
        let edge = self.edge();
        let now_placement = self.placement_now();
        let target = dock_layout::place(&shape, edge, self.viewport(), self.magnified());
        let rest = self.rest();
        let look = if self.contrast.is_some() {
            look::contrast()
        } else {
            look::Look::of(
                &settings,
                self.data
                    .custom_skin
                    .as_ref()
                    .map(|custom| &custom.skin.meta),
            )
        };
        let attention = dock_items::needs_attention(self.data.widgets.as_ref());
        let clock = local_now();
        let drag = self.press.filter(|press| press.dragging);
        let held = self.shown_drag().map(|(press, _)| press.index);
        let offsets = self.shown_offsets(&rest);
        let mut tiles = Vec::with_capacity(self.items.len());
        for (index, item) in self.items.iter().enumerate() {
            let dragging = held == Some(index);
            // A spacer draws nothing, but while it is dragged: an empty
            // plate its size, so what moves shows (spec 0022).
            if matches!(item, Item::Spacer(_)) && !dragging {
                continue;
            }
            let mut area = now_placement.tiles[index];
            let mut box_area = now_placement.items[index];
            area.x += offsets[index];
            box_area.x += offsets[index];
            if dragging {
                // Picked up: off the bar, a little bigger.
                let lift = match edge {
                    DockEdge::Bottom => -DRAG_LIFT,
                    DockEdge::Top => DRAG_LIFT,
                };
                let (width, height) = (area.width * DRAG_GROW, area.height * DRAG_GROW);
                area = Area {
                    x: area.x - (width - area.width) / 2.0,
                    y: area.y + lift - (height - area.height) / 2.0,
                    width,
                    height,
                };
            }
            let risen = self.widen.get(index).copied().unwrap_or(0.0);
            if risen > 0.0 {
                // A floor above the others: wider by an icon, and up.
                let wide = area.width * (1.0 + 0.5 * risen);
                let up = geometry.icon * 0.38 * risen;
                area = Area {
                    x: area.x - (wide - area.width) / 2.0,
                    y: match edge {
                        DockEdge::Bottom => area.y - up,
                        DockEdge::Top => area.y + up,
                    },
                    width: wide,
                    ..area
                };
            }
            if let Some(started) = self.opening.get(&item.key()) {
                let (dx, dy, grow) = opening_offset(
                    settings.open_animation,
                    started.elapsed(),
                    geometry.icon,
                    edge,
                );
                let side = area.width * grow;
                area = Area {
                    x: area.x + dx - (side - area.width) / 2.0,
                    y: area.y + dy - (side - area.height) / 2.0,
                    width: side,
                    height: side,
                };
            }
            let open_here = self.is_open(item);
            let running = dock_items::running(&self.data.open, item);
            let dot = match (open_here, running) {
                (true, _) | (_, Some(true)) => 1.0,
                (false, Some(false)) => 0.55,
                (false, None) => 0.0,
            };
            let face = self.face(item, index, &settings, shape.size, &clock);
            tiles.push(Tile {
                area,
                face,
                pressed: self.pressed == Some(index) && !dragging,
                attention: attention && matches!(item, Item::System(SystemItem::Settings)),
                dragging,
                raised: risen > 0.0,
                dot: Some((box_area.center_x(), dot)),
                item: box_area,
                notice: self.notices.get(index).copied().flatten(),
            });
        }
        let tip = self
            .magnified()
            .or(self.hovered.filter(|_| self.open.is_none()))
            .or_else(|| self.keyboard_position(|keys| &keys.items))
            .filter(|_| drag.is_none())
            // The risen Calendário says it all itself.
            .filter(|&index| self.widen.get(index).copied().unwrap_or(0.0) == 0.0)
            .filter(|&index| {
                self.items
                    .get(index)
                    .is_some_and(|item| !self.is_open(item))
            })
            .and_then(|index| {
                let tile = target.tiles.get(index)?;
                Some(Tip {
                    anchor: dock_layout::tip_anchor(*tile, edge),
                    text: self.names.get(index)?.clone(),
                })
            });
        let cells = self.strip_cells();
        let strip: Vec<StripCell> = self
            .strip
            .iter()
            .zip(&cells)
            .enumerate()
            .map(|(index, (cell, area))| StripCell {
                area: *area,
                glyph: cell.glyph(edge),
                image: match cell {
                    Cell::Tray(icon) => self
                        .tray_images
                        .get(&icon.key)
                        .and_then(|(_, bitmap)| bitmap.clone()),
                    _ => None,
                },
                hovered: self.strip_hovered == Some(index),
            })
            .collect();
        // Over a cell of the strip, its label.
        let tip = tip.or_else(|| {
            let index = self
                .strip_hovered
                .filter(|_| self.open.is_none())
                .or_else(|| self.keyboard_position(|keys| &keys.strip))?;
            Some(Tip {
                anchor: dock_layout::tip_anchor(*cells.get(index)?, edge),
                text: self.strip.get(index)?.tip(),
            })
        });
        // The workspace dots, and the name of the one under the pointer.
        let dot_centers = now_placement
            .switcher
            .map(|area| dock_layout::switcher_dots(area, self.data.workspaces.len()))
            .unwrap_or_default();
        let shown = self.data.workspace.as_ref().map(|w| w.id.as_str());
        let dots: Vec<Dot> = dot_centers
            .iter()
            .zip(&self.data.workspaces)
            .enumerate()
            .map(|(index, (&center, workspace))| Dot {
                center,
                color: Color::hex(&workspace.color).unwrap_or(palette.accent),
                active: Some(workspace.id.as_str()) == shown,
                hovered: self.switcher_hovered == Some(index),
            })
            .collect();
        let tip = tip.or_else(|| {
            let index = self
                .switcher_hovered
                .filter(|_| self.open.is_none())
                .or_else(|| self.keyboard_position(|keys| &keys.dots))?;
            let (x, y) = *dot_centers.get(index)?;
            let side = 2.0 * dock_layout::ACTIVE_DOT_RADIUS;
            Some(Tip {
                anchor: dock_layout::tip_anchor(
                    Area {
                        x: x - side / 2.0,
                        y: y - side / 2.0,
                        width: side,
                        height: side,
                    },
                    edge,
                ),
                text: self.data.workspaces.get(index)?.name.clone(),
            })
        });
        let status = now_placement.status.map(|area| Status {
            area,
            time: format!("{:02}:{:02}", clock.wHour, clock.wMinute),
            date: format!("{:02}/{:02}/{:04}", clock.wDay, clock.wMonth, clock.wYear),
            hovered: self.status_hovered,
        });
        let workspace_color = settings
            .indicator_follows_workspace
            .then_some(self.data.workspace.as_ref())
            .flatten()
            .and_then(|workspace| Color::hex(&workspace.color));
        let indicator_color = workspace_color
            .or_else(|| settings.indicator_color.as_deref().and_then(Color::hex))
            .or(look.indicator)
            .unwrap_or(palette.accent);
        Scene {
            palette,
            look,
            edge,
            pad: geometry.pad,
            icon: geometry.icon,
            bar: now_placement.bar,
            separators: now_placement.separators.clone(),
            dots,
            tiles,
            status,
            strip,
            tip,
            indicator: settings.indicator,
            indicator_color,
            focus: self.focus_ring(&palette),
            custom_skin: self.data.custom_skin.clone(),
        }
    }

    // ---- The keyboard (M14) ----

    fn face(
        &self,
        item: &Item,
        index: usize,
        settings: &crate::domain::settings::DockSettings,
        size: DockSize,
        clock: &windows::Win32::Foundation::SYSTEMTIME,
    ) -> Face {
        let dark = self.bar_dark();
        let program = |app_id: &str, badge: Option<&str>| {
            let icon = match self.icons.get(&icon_key(settings, app_id, dark)) {
                Some(Loaded::Ready(Some(icon))) => Some(icon.clone()),
                _ => None,
            };
            let badge = badge.and_then(|key| match self.badges.get(key) {
                Some(Loaded::Ready(Some(badge))) => Some(badge.clone()),
                _ => None,
            });
            let letter = self
                .names
                .get(index)
                .and_then(|name| name.trim().chars().next())
                .map(|c| c.to_uppercase().collect::<String>())
                .unwrap_or_else(|| "?".into());
            Face::App {
                icon,
                letter,
                shape: settings.icon_shape,
                badge,
            }
        };
        match item {
            Item::App(app) => program(&app.app_id, None),
            Item::Other(other) => program(&other.app_id, other.badge.then_some(other.key.as_str())),
            Item::Widget(widget) if matches!(widget.config, WidgetConfig::Media(_)) => {
                self.media_face(widget.config.width(), size)
            }
            Item::Widget(widget) => {
                let calendar = matches!(widget.config, WidgetConfig::Calendar(_))
                    .then(super::calendar::snapshot)
                    .flatten();
                faces::widget(
                    widget,
                    size,
                    clock,
                    self.activity.as_ref(),
                    calendar.as_ref(),
                    self.widen.get(index).copied().unwrap_or(0.0) > 0.5,
                )
            }
            Item::System(SystemItem::Settings) => Face::Glyph {
                glaze: theme::Glaze::SETTINGS,
                paths: paint::SETTINGS_GLYPH,
            },
            // Drawn only while dragged: an empty plate.
            Item::Spacer(_) => Face::Glyph {
                glaze: theme::Glaze::APP,
                paths: &[],
            },
        }
    }

    // ---- The mouse ----

    fn point(&self, lparam: LPARAM) -> (f64, f64) {
        let x = f64::from((lparam.0 & 0xffff) as u16 as i16);
        let y = f64::from(((lparam.0 >> 16) & 0xffff) as u16 as i16);
        (x / self.scale, y / self.scale)
    }

    fn track(&mut self) {
        if self.tracking {
            return;
        }
        let mut track = TRACKMOUSEEVENT {
            cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
            dwFlags: TME_LEAVE,
            hwndTrack: self.hwnd,
            dwHoverTime: 0,
        };
        self.tracking = unsafe { TrackMouseEvent(&mut track) }.is_ok();
    }

    fn mouse_move(&mut self, x: f64, y: f64) {
        self.track();
        let program = self
            .press
            .is_some_and(|press| self.program_id(press.index).is_some());
        // A program pulled straight off the dock is dragged too.
        let off = program && pointer_off(self.hwnd);
        if let Some(press) = self.press.as_mut() {
            if !press.dragging && ((x - press.start_x).abs() >= DRAG_THRESHOLD || off) {
                press.dragging = true;
                self.hovered = None;
                self.pressed = None;
            }
            if press.dragging {
                let index = press.index;
                let out = off;
                press.out = out;
                self.drag_x = if out { press.start_x } else { x };
                self.show_drag_cursor(out.then_some(index));
                self.animate();
                return;
            }
        }
        // Frozen while a panel is open.
        if self.open.is_some() {
            return;
        }
        let status = self
            .placement_now()
            .status
            .is_some_and(|area| area.contains(x, y));
        let cell = self.strip_hit(x, y);
        let dot = self.switcher_hit(x, y);
        let hovered = if status || cell.is_some() || dot.is_some() {
            None
        } else {
            self.hit(x, y)
        };
        if hovered != self.hovered
            || status != self.status_hovered
            || cell != self.strip_hovered
            || dot != self.switcher_hovered
        {
            self.hovered = hovered;
            self.status_hovered = status;
            self.strip_hovered = cell;
            self.switcher_hovered = dot;
            let _ = unsafe { KillTimer(Some(self.hwnd), WARM_TIMER) };
            if self.warmable() {
                unsafe { SetTimer(Some(self.hwnd), WARM_TIMER, WARM_DELAY, None) };
            }
            self.arm_preview();
            self.animate();
            self.paint();
        }
    }

    fn mouse_leave(&mut self) {
        self.tracking = false;
        // A card already shown hides itself once the pointer is off it too.
        let _ = unsafe { KillTimer(Some(self.hwnd), PREVIEW_TIMER) };
        if self.press.is_some() {
            return;
        }
        self.hovered = None;
        self.switcher_hovered = None;
        self.status_hovered = false;
        self.strip_hovered = None;
        self.pressed = None;
        let _ = unsafe { KillTimer(Some(self.hwnd), WARM_TIMER) };
        self.animate();
        self.paint();
    }

    /// The workspace this dock shows (spec 0013); `None` when they can't be
    /// read.
    fn workspace_id(&self) -> Option<String> {
        self.data.workspace.as_ref().map(|w| w.id.clone())
    }

    /// The workspace dot under the point, when the bar starts with them.
    fn switcher_hit(&self, x: f64, y: f64) -> Option<usize> {
        let area = self.placement_now().switcher?;
        dock_layout::switcher_hit(area, self.data.workspaces.len(), x, y)
    }

    /// Shows the workspace at `index` on this dock's monitor.
    fn switch_to(&mut self, index: usize) {
        let Some(id) = self.data.workspaces.get(index).map(|w| w.id.clone()) else {
            return;
        };
        if self.data.workspace.as_ref().is_some_and(|w| w.id == id) {
            return;
        }
        let monitor = self.monitor.key.clone();
        self.effects.push(Box::new(move |app| {
            crate::commands::workspaces::switch(app, &monitor, &id);
        }));
    }

    /// The mouse wheel over the dock: the next workspace down, the one
    /// before up (spec 0013).
    fn wheel(&mut self, delta: i16) {
        if self.data.workspaces.len() < 2 || delta == 0 {
            return;
        }
        self.end_preview();
        let monitor = self.monitor.key.clone();
        let forward = delta < 0;
        self.effects.push(Box::new(move |app| {
            crate::commands::workspaces::switch_by(app, &monitor, forward);
        }));
    }

    /// The user is on this dock: its workspace is the one in use (where a
    /// program pinned from a menu goes).
    fn use_here(&mut self) {
        let monitor = self.monitor.key.clone();
        self.effects.push(Box::new(move |app| {
            crate::commands::workspaces::use_from_dock(app, &monitor);
        }));
    }

    fn left_down(&mut self, x: f64, y: f64) {
        self.end_preview();
        self.use_here();
        shared().clicked = Some(Instant::now());
        // A spacer is pressed only to be dragged: it never looks pressed.
        let index = self.hit(x, y).or_else(|| self.spacer_hit(x, y));
        self.pressed =
            index.filter(|&index| !matches!(self.items.get(index), Some(Item::Spacer(_))));
        // A program alone in its group still goes out to a folder.
        let group = |index: usize| {
            self.group_of(index)
                .or_else(|| self.program_id(index).map(|_| (index, 1)))
        };
        if let Some((index, group)) = index.and_then(|index| Some((index, group(index)?)))
            && self.open.is_none()
        {
            self.press = Some(Press {
                index,
                group,
                start_x: x,
                dragging: false,
                out: false,
            });
            self.drag_x = x;
            unsafe { SetCapture(self.hwnd) };
        }
        self.paint();
    }

    fn left_up(&mut self, x: f64, y: f64) {
        let press = self.press.take();
        let pressed = self.pressed.take();
        if press.is_some() {
            let _ = unsafe { ReleaseCapture() };
        }
        self.show_drag_cursor(None);
        if let Some(press) = press.filter(|press| press.dragging && press.out) {
            // Let go off the dock: the program opens with what is there.
            if let Some(id) = self.program_id(press.index) {
                let mut at = POINT::default();
                let _ = unsafe { windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut at) };
                self.effects.push(Box::new(move |_| {
                    super::open_with::open_at(id, at.x, at.y);
                }));
            }
            self.animate();
            self.paint();
            return;
        }
        if let Some(press) = press.filter(|press| press.dragging) {
            if self.drop_at(press, x) {
                self.landing = Some(Landing {
                    press,
                    x,
                    since: Instant::now(),
                });
            }
            self.animate();
            self.paint();
            return;
        }
        if let Some(index) = self.switcher_hit(x, y) {
            self.switch_to(index);
            self.paint();
            return;
        }
        let status = self
            .placement_now()
            .status
            .is_some_and(|area| area.contains(x, y));
        if status {
            self.effects.push(Box::new(|_| {
                if let Err(error) = super::flyout::open(super::flyout::Flyout::Calendar) {
                    log::warn!("dock: the calendar could not be opened: {error}");
                }
            }));
        } else if let Some(cell) = self.strip_hit(x, y) {
            self.strip_click(cell, false);
        } else if let Some(index) = self.hit(x, y).filter(|&index| Some(index) == pressed) {
            match self.media_button_at(index, x, y) {
                Some(action) => super::media::control(action),
                None => self.click(index),
            }
        }
        self.paint();
    }

    /// The Tocando agora's tile (spec 0017 F4): the cover of what plays now
    /// and, wider, its title and artist.
    fn media_face(&self, width: u8, size: DockSize) -> Face {
        let now = super::media::snapshot();
        let (art, badge) = match (&now, &self.media_art) {
            (Some(now), Some((track, art, icon))) if *track == now.track() => {
                (art.clone(), icon.clone())
            }
            _ => (None, None),
        };
        let (title, artist) = match &now {
            // A small dock has room for the title alone.
            Some(now) if width > 1 && size == DockSize::Small => (Some(now.title.clone()), None),
            Some(now) if width > 1 => (Some(now.title.clone()), Some(now.artist.clone())),
            _ => (None, None),
        };
        Face::Media {
            // The cover's color, as a phone's media notification has it.
            glaze: now
                .as_ref()
                .and_then(|now| now.tint)
                .map_or(theme::Glaze::MUSIC, theme::Glaze::ground_of),
            art,
            badge,
            playing: now.as_ref().is_some_and(|now| now.playing),
            title,
            artist,
            idle: now.is_none(),
            controls: width == 3 && now.is_some(),
            // Saving the battery, a long title stays at its start.
            since: if self.saving {
                0.0
            } else {
                self.media_since.elapsed().as_secs_f64()
            },
        }
    }

    /// The button of the Tocando agora at `index` under `(x, y)`, three
    /// icons wide while something plays (spec 0017 F4).
    fn media_button_at(&self, index: usize, x: f64, y: f64) -> Option<super::media::Action> {
        let Some(Item::Widget(widget)) = self.items.get(index) else {
            return None;
        };
        if !matches!(widget.config, WidgetConfig::Media(_))
            || widget.config.width() != 3
            || super::media::snapshot().is_none()
        {
            return None;
        }
        let tile = *self.placement_now().tiles.get(index)?;
        let actions = [
            super::media::Action::Previous,
            super::media::Action::Toggle,
            super::media::Action::Next,
        ];
        dock_layout::media_buttons(tile)
            .into_iter()
            .zip(actions)
            .find(|(button, _)| {
                // A little more than the glyph, easier to hit.
                let pad = button.width * 0.2;
                x >= button.x - pad
                    && x <= button.x + button.width + pad
                    && y >= button.y - pad
                    && y <= button.y + button.height + pad
            })
            .map(|(_, action)| action)
    }

    /// What plays changed: a new cover is read into a bitmap, and the tile
    /// painted again.
    fn media_changed(&mut self) {
        let now = super::media::snapshot();
        let track = now.as_ref().map(super::media::Snapshot::track);
        let kept = self.media_art.as_ref().map(|(track, _, _)| track.clone());
        if track != kept {
            self.media_since = Instant::now();
            self.media_art = now.as_ref().map(|now| {
                let art = now.art.as_ref().and_then(|art| self.painter.bitmap(&art.1));
                let icon = now.icon.as_ref().and_then(|icon| self.painter.bitmap(icon));
                (now.track(), art, icon)
            });
        }
        self.paint();
    }

    /// The Beber água's reminder (spec 0017 F3-2), looked at every minute.
    fn check_water(&self) {
        let waters: Vec<_> = self
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Widget(widget) => match &widget.config {
                    WidgetConfig::Water(config) => {
                        Some((widget.id.as_str().to_owned(), config.clone()))
                    }
                    _ => None,
                },
                _ => None,
            })
            .collect();
        if !waters.is_empty() {
            super::water::check(&waters, super::calendar::now(), super::calendar::offset());
        }
    }

    /// The id in the Apps folder of the program at `index`: a pinned one or
    /// an open one. What a folder or files open with (spec 0018).
    fn program_id(&self, index: usize) -> Option<String> {
        match self.items.get(index)? {
            Item::App(app) => Some(app.app_id.clone()),
            Item::Other(other) => Some(other.app_id.clone()).filter(|id| !id.is_empty()),
            _ => None,
        }
    }

    fn click(&mut self, index: usize) {
        let Some(item) = self.items.get(index).cloned() else {
            return;
        };
        match &item {
            Item::App(app) => {
                let id = app.id.as_str().to_string();
                let key = item.key();
                let running = dock_items::running(&self.data.open, &item).is_some();
                let hwnd = self.hwnd.0 as isize;
                let workspace = self.workspace_id();
                if running {
                    self.effects.push(Box::new(move |app| {
                        if !super::open_apps::activate(app, &id, workspace.as_deref()) {
                            post_to(hwnd, move |dock| dock.launch_pinned(key, id));
                        }
                    }));
                } else {
                    self.launch_pinned(key, id);
                }
            }
            Item::Other(other) => {
                let key = other.key.clone();
                let workspace = self.workspace_id();
                self.effects.push(Box::new(move |app| {
                    super::open_apps::activate_other(app, &key, workspace.as_deref());
                }));
            }
            // The Lançador opens or closes the launcher, as the menus' "Buscar
            // apps…", where it opens today (spec 0012): with the hotkey off
            // in Configurações too, since that only lets the hotkey go.
            _ if is_launcher(&item) => super::launcher::toggle(),
            Item::Spacer(_) => {}
            _ => {
                if let Item::Widget(widget) = &item
                    && (super::calendar::seen(widget.id.as_str())
                        | super::pomodoro::seen(widget.id.as_str())
                        | super::water::seen(widget.id.as_str()))
                {
                    post(Dock::notices_changed);
                }
                let anchor = self.rest().items[index];
                if let Some(panel) = panel_of(&item) {
                    self.toggle(panel, anchor);
                }
            }
        }
    }

    /// The widgets `widgets` have a notice (a Ciclo's, F2b-3; a Pomodoro's
    /// stage ended, F2c-2): their badge, and their tile bounces as a
    /// program's does when it opens (the badge alone while the battery is
    /// saved, spec 0021).
    fn nudged(&mut self, widgets: &[String]) {
        let now = Instant::now();
        let keys: Vec<String> = self
            .items
            .iter()
            .filter(|item| {
                matches!(item, Item::Widget(widget)
                    if widgets.iter().any(|each| each == widget.id.as_str()))
            })
            .map(Item::key)
            .collect();
        for key in keys {
            self.start_opening(key, now);
        }
        self.notices = self.read_notices();
        self.animate();
    }

    fn toggle(&mut self, panel: PanelId, anchor: Area) {
        // The panel anchors to this dock.
        shared().active = self.hwnd.0 as isize;
        // The panel opens where the item rests: nothing magnified.
        self.hovered = None;
        self.animate();
        self.effects.push(Box::new(move |app| {
            if let Err(error) = crate::commands::panel::toggle(app, panel, logical(anchor)) {
                log::warn!("dock: toggling a panel failed: {error}");
            }
        }));
    }

    /// Opens the pinned program and animates its icon a moment; one that
    /// could not be opened stops at once.
    fn launch_pinned(&mut self, key: String, id: String) {
        if self.start_opening(key.clone(), Instant::now()) {
            self.animate();
        }
        let hwnd = self.hwnd.0 as isize;
        self.effects.push(Box::new(move |app| {
            crate::commands::apps::launch_pinned(app, &id, move || {
                post_to(hwnd, move |dock| {
                    dock.opening.remove(&key);
                    dock.paint();
                });
            });
        }));
    }

    fn middle_click(&mut self, index: usize) {
        // The Calendário joins the meeting that starts within 10 minutes
        // (spec 0017 F2a), whatever the programs' middle click does.
        if let Some(Item::Widget(widget)) = self.items.get(index)
            && let WidgetConfig::Calendar(config) = &widget.config
        {
            let config = config.clone();
            self.effects.push(Box::new(move |_| {
                let Some(snapshot) = super::calendar::snapshot() else {
                    return;
                };
                let shown: Vec<_> = snapshot
                    .events
                    .into_iter()
                    .filter(|event| config.shows(&event.calendar))
                    .collect();
                let now = super::calendar::now();
                if let Some(join) = crate::domain::calendar::joinable(&shown, now)
                    .and_then(|event| event.join.clone())
                    && let Err(error) = super::google::open_meeting(&join)
                {
                    log::warn!("calendar: the meeting could not be opened: {error}");
                }
            }));
            return;
        }
        // The Tocando agora plays or pauses (spec 0017 F4).
        if let Some(Item::Widget(widget)) = self.items.get(index)
            && matches!(widget.config, WidgetConfig::Media(_))
        {
            super::media::control(super::media::Action::Toggle);
            return;
        }
        // The Pomodoro starts or pauses (spec 0017 F2c-2).
        if let Some(Item::Widget(widget)) = self.items.get(index)
            && let WidgetConfig::Pomodoro(config) = &widget.config
        {
            let (id, config) = (widget.id.as_str().to_owned(), config.clone());
            self.effects.push(Box::new(move |_| {
                use super::pomodoro::{Action, act, view};
                let action = if view(&id, &config).running {
                    Action::Pause
                } else {
                    Action::Start
                };
                act(&id, &config, action);
            }));
            return;
        }
        if self.data.settings.dock.middle_click != MiddleClick::NewInstance {
            return;
        }
        let Some(item) = self.items.get(index).cloned() else {
            return;
        };
        let key = item.key();
        match item {
            Item::App(app) => self.launch_pinned(key, app.id.as_str().to_string()),
            Item::Other(other) => {
                if self.start_opening(key.clone(), Instant::now()) {
                    self.animate();
                }
                let hwnd = self.hwnd.0 as isize;
                self.effects.push(Box::new(move |_| {
                    crate::commands::apps::launch_other(&other.key, move || {
                        post_to(hwnd, move |dock| {
                            dock.opening.remove(&key);
                            dock.paint();
                        });
                    });
                }));
            }
            _ => {}
        }
    }

    fn right_click(&mut self, index: usize) {
        let Some(item) = self.items.get(index).cloned() else {
            return;
        };
        let Some(panel) = panel_of(&item).filter(|_| item.is_program()) else {
            return;
        };
        let anchor = self.rest().items[index];
        self.toggle(panel, anchor);
    }

    /// The dock's own menu, on the right button away from a program: what
    /// the tray icon's menu has (author's decision, 2026-10-26: Configurações
    /// left the dock's items; the Task Manager as in the taskbar's menu).
    /// Shown once the dock's state is let go: the menu runs a loop of its own.
    /// The dock's menu; on a widget that takes more than one width
    /// (`index`), the widths come first; on a spacer, "Remover espaçador"
    /// (spec 0022). A spacer added from it goes right after the item it
    /// opened on, or at the end of the workspace.
    fn dock_menu(&mut self, index: Option<usize>) {
        let item = index.and_then(|index| self.items.get(index));
        let widget = match item {
            Some(Item::Widget(widget)) => Some(widget.clone()),
            _ => None,
        };
        let spacer = match item {
            Some(Item::Spacer(spacer)) => Some(spacer.id.clone()),
            _ => None,
        };
        // Only the workspace's own items have a place in it.
        let after = item
            .filter(|item| matches!(item, Item::App(_) | Item::Widget(_) | Item::Spacer(_)))
            .map(Item::key);
        self.hovered = None;
        self.animate();
        self.paint();
        let hwnd = self.hwnd.0 as isize;
        let edge = self.edge();
        // By the keyboard: by the item, the dock its owner (it takes the
        // keyboard back when the menu closes).
        let from_keys = std::mem::take(&mut self.menu_by_keyboard)
            .then(|| self.keyboard_menu_point())
            .flatten()
            .map(|at| (self.hwnd, at));
        let shown = self.data.workspace.as_ref().map(|w| w.id.clone());
        let workspaces: Vec<MenuWorkspace> = self
            .data
            .workspaces
            .iter()
            .map(|w| MenuWorkspace {
                name: w.name.clone(),
                color: w.color.clone(),
                shown: Some(&w.id) == shown.as_ref(),
            })
            .collect();
        let mut items = widget
            .as_ref()
            .map(|widget| widget_items(&widget.config))
            .unwrap_or_default();
        if spacer.is_some() {
            items.extend(spacer_items());
        }
        items.extend(dock_menu_items(
            self.data.settings.dock.auto_hide,
            &workspaces,
            crate::commands::workspaces::can_create(&self.data.workspaces),
            shown.is_some(),
        ));
        let widget = widget.map(|widget| widget.id);
        let monitor = self.monitor.key.clone();
        let ids: Vec<String> = self.data.workspaces.iter().map(|w| w.id.clone()).collect();
        // Up, under its menu, until it closes.
        self.menu_open = true;
        self.effects.push(Box::new(move |app| {
            let chosen = show_dock_menu(app, edge, items, from_keys);
            post_to(hwnd, Dock::menu_closed);
            if let Some(choice) = chosen {
                let on = MenuOn {
                    widget: widget.as_ref(),
                    spacer: spacer.as_deref(),
                    after: after.as_deref(),
                };
                act(app, choice, &monitor, &ids, on, shown.as_deref());
            }
        }));
    }

    /// The program's own menu, native, when its panel can't open (its
    /// window never came up, `panel::fallback`): at the pointer, where the
    /// right click was.
    fn program_menu(&mut self, panel: &PanelId) {
        let Some(item) = self
            .items
            .iter()
            .find(|item| panel_of(item).as_ref() == Some(panel))
        else {
            return;
        };
        let (program, windows) = match item {
            Item::App(app) => (
                Program::Pinned(app.id.as_str().to_string()),
                self.data
                    .open
                    .apps
                    .iter()
                    .find(|open| open.id == app.id.as_str())
                    .map_or(0, |open| open.count),
            ),
            Item::Other(other) => (Program::Open(other.key.clone()), other.count),
            _ => return,
        };
        let items = program_items(&program, windows);
        self.hovered = None;
        self.animate();
        self.paint();
        let hwnd = self.hwnd.0 as isize;
        let edge = self.edge();
        let workspace = self.workspace_id();
        self.menu_open = true;
        self.effects.push(Box::new(move |app| {
            let chosen = show_dock_menu(app, edge, items, None);
            post_to(hwnd, Dock::menu_closed);
            if let Some(choice) = chosen {
                act_on_program(app, choice, program, workspace);
            }
        }));
    }

    // ---- What the app tells the dock ----

    fn set_open(&mut self, panel: Option<PanelId>) {
        if panel.is_some() {
            self.hovered = None;
            self.status_hovered = false;
            // A panel anchors to the dock: up while it is open.
            self.untuck();
        }
        self.open = panel;
        self.animate();
        self.paint();
    }

    /// Opens the panel of the item that has it, as a click would, unless it
    /// is open already.
    fn request(&mut self, panel: PanelId) {
        if self.open.as_ref() == Some(&panel) {
            return;
        }
        let rest = self.rest();
        let Some(index) = self
            .items
            .iter()
            .position(|item| panel_of(item).as_ref() == Some(&panel))
        else {
            return;
        };
        let anchor = rest.items[index];
        self.toggle(panel, anchor);
    }
}

/// How the icon of a program that is opening is moved and grown,
/// `elapsed` into its animation: away from the screen edge for a bounce
/// (twice, 0.7 s each), a swell (three times, 0.5 s) or a shake (twice,
/// 0.5 s).
fn opening_offset(
    animation: OpenAnimation,
    elapsed: Duration,
    icon: f64,
    edge: DockEdge,
) -> (f64, f64, f64) {
    let away = match edge {
        DockEdge::Bottom => -1.0,
        DockEdge::Top => 1.0,
    };
    let seconds = elapsed.as_secs_f64();
    // Linear between the keyframes of a cycle of `period`.
    let keyframes = |period: f64, cycles: f64, frames: &[(f64, f64)]| {
        if seconds >= period * cycles {
            return 0.0;
        }
        let t = (seconds % period) / period;
        frames
            .windows(2)
            .find(|pair| t >= pair[0].0 && t <= pair[1].0)
            .map(|pair| {
                let (t0, v0) = pair[0];
                let (t1, v1) = pair[1];
                if t1 <= t0 {
                    v1
                } else {
                    v0 + (v1 - v0) * (t - t0) / (t1 - t0)
                }
            })
            .unwrap_or(0.0)
    };
    match animation {
        OpenAnimation::Bounce => {
            let lift = keyframes(
                0.7,
                2.0,
                &[(0.0, 0.0), (0.3, 0.5), (0.6, 0.0), (0.8, 0.14), (1.0, 0.0)],
            );
            (0.0, away * icon * lift, 1.0)
        }
        OpenAnimation::Pulse => {
            let grow = keyframes(0.5, 3.0, &[(0.0, 1.0), (0.5, 1.22), (1.0, 1.0)]);
            (0.0, 0.0, if grow == 0.0 { 1.0 } else { grow })
        }
        OpenAnimation::Shake => {
            let x = keyframes(
                0.5,
                2.0,
                &[
                    (0.0, 0.0),
                    (0.2, -4.0),
                    (0.4, 4.0),
                    (0.6, -4.0),
                    (0.8, 4.0),
                    (1.0, 0.0),
                ],
            );
            (x, 0.0, 1.0)
        }
    }
}

fn local_now() -> windows::Win32::Foundation::SYSTEMTIME {
    unsafe { windows::Win32::System::SystemInformation::GetLocalTime() }
}

extern "system" fn procedure(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    super::panics::window_message(|| handle(hwnd, message, wparam, lparam)).unwrap_or_else(|| {
        recover(hwnd);
        match message {
            // Never closed for a panic: Alt+F4 only hides the dock.
            WM_CLOSE => LRESULT(0),
            _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
        }
    })
}

/// The work of the window procedure for one message.
fn handle(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let mut handled = true;
    match message {
        WM_RUN => run_queue(),
        WM_MOUSEWHEEL => with_dock(hwnd, |dock| {
            // The high word of `wparam`: notches of 120, negative toward the
            // user.
            dock.wheel((wparam.0 >> 16) as u16 as i16);
        }),
        WM_HOTKEY if wparam.0 == FOCUS_HOTKEY as usize => take_focus(),
        WM_HOTKEY => hotkey(wparam.0),
        // The keyboard, only ever given by the dock's hotkey (M14).
        WM_KEYDOWN | WM_SYSKEYDOWN => {
            let shift = unsafe {
                windows::Win32::UI::Input::KeyboardAndMouse::GetKeyState(i32::from(
                    windows::Win32::UI::Input::KeyboardAndMouse::VK_SHIFT.0,
                ))
            } < 0;
            let mut ours = false;
            // Alt and a key (WM_SYSKEYDOWN) stays Windows' but for Shift+F10.
            if message == WM_KEYDOWN || wparam.0 == usize::from(keys::VK_F10) {
                with_dock(hwnd, |dock| ours = dock.key_down(wparam.0 as u16, shift));
            }
            handled = ours;
        }
        // Losing the keyboard: nothing is chosen; the focus stays where it
        // went. Windows still does its own part.
        WM_ACTIVATE | WM_KILLFOCUS => {
            let inactive = message == WM_KILLFOCUS || (wparam.0 & 0xFFFF) == 0;
            if inactive {
                with_dock(hwnd, Dock::lose_keyboard);
            }
            handled = false;
        }
        WM_MOUSEMOVE => with_dock(hwnd, |dock| {
            let (x, y) = dock.point(lparam);
            dock.mouse_move(x, y);
        }),
        WM_MOUSELEAVE => with_dock(hwnd, Dock::mouse_leave),
        WM_LBUTTONDOWN | WM_LBUTTONDBLCLK => with_dock(hwnd, |dock| {
            let (x, y) = dock.point(lparam);
            dock.left_down(x, y);
        }),
        WM_LBUTTONUP => with_dock(hwnd, |dock| {
            let (x, y) = dock.point(lparam);
            dock.left_up(x, y);
        }),
        WM_MBUTTONDOWN => {
            shared().clicked = Some(Instant::now());
        }
        WM_MBUTTONUP => with_dock(hwnd, |dock| {
            let (x, y) = dock.point(lparam);
            if let Some(index) = dock.hit(x, y) {
                dock.middle_click(index);
            }
        }),
        WM_RBUTTONUP => with_dock(hwnd, |dock| {
            dock.end_preview();
            shared().clicked = Some(Instant::now());
            dock.use_here();
            let (x, y) = dock.point(lparam);
            if let Some(cell) = dock.strip_hit(x, y) {
                dock.strip_click(cell, true);
                return;
            }
            match dock.hit(x, y) {
                Some(index) if dock.items.get(index).is_some_and(Item::is_program) => {
                    dock.right_click(index);
                }
                // Anywhere else on the dock, a widget or a spacer too: its
                // own menu, with the widget's widths or the spacer's
                // "Remover espaçador" first.
                hit => {
                    let on = hit.or_else(|| dock.spacer_hit(x, y));
                    dock.dock_menu(on);
                }
            }
        }),
        WM_CAPTURECHANGED => with_dock(hwnd, |dock| {
            // The capture went elsewhere: the drag is over.
            dock.show_drag_cursor(None);
            if dock.press.take().is_some() {
                dock.pressed = None;
                dock.animate();
                dock.paint();
            }
        }),
        WM_TIMER => match wparam.0 {
            FOCUS_CAPTURE_TIMER => {
                let _ = unsafe { KillTimer(Some(hwnd), FOCUS_CAPTURE_TIMER) };
                FOCUS_CAPTURING.store(false, Ordering::SeqCst);
                arm_focus_hotkey();
            }
            ANIMATION_TIMER => with_dock(hwnd, Dock::step),
            CLOCK_TIMER => with_dock(hwnd, |dock| {
                dock.paint();
                dock.arm_clock();
                dock.check_water();
            }),
            STATUS_TIMER => with_dock(hwnd, Dock::read_system),
            CARD_TIMER => with_dock(hwnd, |dock| {
                let _ = unsafe { KillTimer(Some(dock.hwnd), CARD_TIMER) };
                dock.show_card();
            }),
            ACTIVITY_TIMER => with_dock(hwnd, Dock::read_activity),
            MARQUEE_TIMER => with_dock(hwnd, Dock::paint),
            AUTO_HIDE_TIMER => with_dock(hwnd, Dock::watch_pointer),
            PREVIEW_TIMER => with_dock(hwnd, |dock| {
                let _ = unsafe { KillTimer(Some(dock.hwnd), PREVIEW_TIMER) };
                dock.show_preview();
            }),
            WARM_TIMER => with_dock(hwnd, |dock| {
                let _ = unsafe { KillTimer(Some(dock.hwnd), WARM_TIMER) };
                if dock.warmable() && dock.open.is_none() {
                    dock.effects.push(Box::new(super::panel_window::warm));
                }
            }),
            _ => handled = false,
        },
        WM_SETTINGCHANGE => {
            // "ImmersiveColorSet" among them: the taskbar's theme, which the
            // tray icon's ink follows.
            check_tray_icon();
            with_dock(hwnd, |dock| {
                dock.dark = theme::system_dark();
                // The contrast mode turned on or off, or another theme of it.
                dock.contrast = theme::system_contrast();
                dock.motion = system_motion();
                // A pack's icons for the theme Windows switched to.
                dock.request_pictures();
                dock.place();
                dock.paint();
            });
        }
        // A monitor came, went or changed: a dock each, placed again.
        WM_DISPLAYCHANGE => {
            check_tray_icon();
            with_docks(sync);
        }
        WM_DPICHANGED => {
            // The tray icon's size follows the primary monitor's scale.
            check_tray_icon();
            with_dock(hwnd, |dock| {
                dock.place();
                dock.paint();
            });
        }
        // Alt+F4 hides the dock instead of closing it.
        // On the battery or back on the charger (or the battery's level,
        // which changes nothing): the battery saver follows at once.
        WM_POWERBROADCAST if wparam.0 == PBT_APMPOWERSTATUSCHANGE as usize => {
            with_dock(hwnd, Dock::check_power);
        }
        WM_CLOSE => with_dock(hwnd, Dock::hide),
        WM_GETOBJECT => match access::get_object(hwnd, wparam, lparam) {
            Some(result) => return result,
            None => handled = false,
        },
        _ => handled = false,
    }
    if handled {
        LRESULT(0)
    } else {
        unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_picture_cache_keeps_only_what_is_shown_now() {
        // Fifty programs opened and closed: the cache ends with the two the
        // dock still shows, whatever passed through it.
        let mut cache: HashMap<String, u32> = (0..50).map(|n| (format!("app{n}"), n)).collect();
        let shown: HashSet<String> = ["app3".to_owned(), "app7".to_owned()].into();
        keep_only(&mut cache, &shown);
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.get("app3"), Some(&3));
        assert_eq!(cache.get("app7"), Some(&7));
        keep_only(&mut cache, &HashSet::new());
        assert!(cache.is_empty());
    }

    #[test]
    fn a_dock_that_keeps_panicking_closes_the_app_once() {
        let start = Instant::now();
        let gave_up = AtomicBool::new(false);
        let mut times = Vec::new();
        let at = |second| start + Duration::from_secs(second);
        for second in 0..3 {
            assert_eq!(
                after_panic(&mut times, at(second), &gave_up),
                AfterPanic::MadeAgain
            );
        }
        // The fourth within the minute closes the app, and only once.
        assert_eq!(after_panic(&mut times, at(4), &gave_up), AfterPanic::Closed);
        assert_eq!(
            after_panic(&mut times, at(5), &gave_up),
            AfterPanic::AlreadyClosing
        );
        // Even a minute later: the app is on its way out.
        assert_eq!(
            after_panic(&mut times, at(90), &gave_up),
            AfterPanic::AlreadyClosing
        );
    }

    /// Closing steps that write down what was done.
    struct Steps {
        done: Vec<&'static str>,
        notices: bool,
        dock_panics: bool,
    }

    impl Closing for Steps {
        fn destroy_docks(&mut self) {
            self.done.push("docks");
            if self.dock_panics {
                panic!("on purpose");
            }
        }
        fn release_taskbar(&mut self) {
            self.done.push("taskbar");
        }
        fn notify(&mut self) -> bool {
            self.done.push("notice");
            self.notices
        }
        fn message_box(&mut self) {
            self.done.push("box");
        }
        fn exit(&mut self) {
            self.done.push("exit");
        }
    }

    #[test]
    fn closing_gives_the_taskbar_back_then_says_so_then_exits() {
        let mut steps = Steps {
            done: Vec::new(),
            notices: true,
            dock_panics: false,
        };
        close_after_failures(&mut steps);
        assert_eq!(steps.done, ["docks", "taskbar", "notice", "exit"]);
    }

    #[test]
    fn closing_shows_a_box_when_the_notices_are_off() {
        let mut steps = Steps {
            done: Vec::new(),
            notices: false,
            dock_panics: false,
        };
        close_after_failures(&mut steps);
        assert_eq!(steps.done, ["docks", "taskbar", "notice", "box", "exit"]);
    }

    #[test]
    fn closing_goes_on_when_a_dock_panics_again_on_its_way_out() {
        let mut steps = Steps {
            done: Vec::new(),
            notices: true,
            dock_panics: true,
        };
        close_after_failures(&mut steps);
        assert_eq!(steps.done, ["docks", "taskbar", "notice", "exit"]);
    }

    const MONITOR: Rect = Rect {
        x: -1920,
        y: 6,
        width: 1920,
        height: 1200,
    };

    #[test]
    fn the_pointer_on_the_screen_edge_brings_the_dock_up() {
        assert!(at_screen_edge((-900, 1205), MONITOR, DockEdge::Bottom));
        assert!(at_screen_edge((-1920, 1204), MONITOR, DockEdge::Bottom));
        assert!(!at_screen_edge((-900, 1200), MONITOR, DockEdge::Bottom));
        // The other monitor's edge is not this one's.
        assert!(!at_screen_edge((100, 1205), MONITOR, DockEdge::Bottom));
        assert!(at_screen_edge((-900, 6), MONITOR, DockEdge::Top));
        assert!(!at_screen_edge((-900, 1205), MONITOR, DockEdge::Top));
    }

    #[test]
    fn the_dock_stays_up_with_the_pointer_on_it_or_below_it() {
        let window = Rect {
            x: -1387,
            y: 1065,
            width: 853,
            height: 138,
        };

        assert!(near_dock((-1000, 1100), window, MONITOR, DockEdge::Bottom));
        // Between the dock and the screen edge.
        assert!(near_dock((-1000, 1205), window, MONITOR, DockEdge::Bottom));
        assert!(!near_dock((-1000, 1000), window, MONITOR, DockEdge::Bottom));
        assert!(!near_dock((-1500, 1100), window, MONITOR, DockEdge::Bottom));
    }

    #[test]
    fn the_dock_slides_out_and_in() {
        let mut now = 0.0;
        let mut frames = 0;
        while now != 1.0 {
            let next = tuck_step(now, 1.0);
            assert!(next > now && next <= 1.0);
            now = next;
            frames += 1;
        }
        // About a quarter of a second at 60 frames a second.
        assert!((8..=20).contains(&frames), "{frames} frames");
        assert_eq!(tuck_step(1.0, 0.0), 0.75);
    }

    /// The menu as text: an item's label (with ✓ when checked), "—" for a
    /// separator and "# title" for a title.
    fn labels(items: &[MenuEntry]) -> Vec<String> {
        items
            .iter()
            .map(|entry| match &entry.line {
                menu::Line::Item(item) => {
                    let check = if item.checked { " ✓" } else { "" };
                    format!("{}{check}", item.label)
                }
                menu::Line::Separator => "—".to_owned(),
                menu::Line::Title(title) => format!("# {title}"),
            })
            .collect()
    }

    fn workspace(name: &str, color: &str, shown: bool) -> MenuWorkspace {
        MenuWorkspace {
            name: name.to_owned(),
            color: color.to_owned(),
            shown,
        }
    }

    #[test]
    fn the_dock_menu_has_what_the_tray_menu_has() {
        // One workspace: no list to switch, "Novo espaço" to make the second;
        // the spacers, for the workspace shown.
        let one = [workspace("Principal", "#0A84FF", true)];
        assert_eq!(
            labels(&dock_menu_items(true, &one, true, true)),
            [
                "Buscar apps…",
                "Gerenciador de Tarefas",
                "—",
                "Adicionar espaçador pequeno",
                "Adicionar espaçador grande",
                "—",
                "Novo espaço",
                "—",
                "Configurações",
                "Ocultar automaticamente ✓",
                "—",
                "Sair",
            ]
        );
        // The tray icon's: the same, without the workspaces and the
        // spacers.
        assert_eq!(
            labels(&dock_menu_items(false, &[], false, false)),
            [
                "Buscar apps…",
                "Gerenciador de Tarefas",
                "—",
                "Configurações",
                "Ocultar automaticamente",
                "—",
                "Sair",
            ]
        );
    }

    #[test]
    fn every_item_has_a_glyph_and_auto_hide_says_it_turns() {
        for entry in dock_menu_items(true, &[], true, true) {
            if let menu::Line::Item(item) = entry.line {
                assert!(item.glyph.is_some(), "{}", item.label);
                let turns = entry.choice == Some(MenuChoice::AutoHide);
                assert_eq!(item.toggle, turns, "{}", item.label);
            }
        }
    }

    #[test]
    fn the_dock_menu_switches_workspaces_with_their_color_and_shortcut() {
        let two = [
            workspace("Principal", "#0A84FF", false),
            workspace("Foco", "#30D158", true),
        ];
        let items = dock_menu_items(false, &two, false, false);
        assert_eq!(
            labels(&items)[3..7],
            ["# Espaços de trabalho", "Principal", "Foco ✓", "—"]
        );
        assert_eq!(items[5].choice, Some(MenuChoice::Workspace(1)));
        let menu::Line::Item(focus) = &items[5].line else {
            panic!("not an item");
        };
        assert_eq!(focus.swatch, Some((0x30, 0xD1, 0x58)));
        assert_eq!(focus.glyph, None);
        assert_eq!(focus.shortcut.as_deref(), Some("Ctrl+Alt+2"));
        // A title chooses nothing.
        assert_eq!(items[3].choice, None);
    }

    #[test]
    fn the_spacer_entries_add_each_size_and_a_spacer_removes_itself() {
        let one = [workspace("Principal", "#0A84FF", true)];
        let items = dock_menu_items(false, &one, true, true);
        assert_eq!(
            items[3].choice,
            Some(MenuChoice::AddSpacer(SpacerSize::Small))
        );
        assert_eq!(
            items[4].choice,
            Some(MenuChoice::AddSpacer(SpacerSize::Large))
        );
        // A spacer's own menu: "Remover espaçador" before the dock's.
        let own = spacer_items();
        assert_eq!(labels(&own), ["Remover espaçador", "—"]);
        assert_eq!(own[0].choice, Some(MenuChoice::RemoveSpacer));
    }

    #[test]
    fn a_program_s_picture_is_kept_by_where_it_comes_from() {
        use crate::domain::app_icon::AppIcon;
        let mut dock = crate::domain::settings::DockSettings::default();
        assert_eq!(icon_key(&dock, "Calc", true), (None, "Calc".into(), false));
        dock.icon_pack = Some("pixel-system".into());
        assert_eq!(
            icon_key(&dock, "Calc", true),
            (Some("pack:pixel-system".into()), "Calc".into(), true)
        );
        // A choice wins over the dock's pack; only a pack has dark images.
        dock.app_icons.insert("calc".into(), AppIcon::Original);
        assert_eq!(
            icon_key(&dock, "Calc", true),
            (Some("original".into()), "Calc".into(), false)
        );
        dock.app_icons.insert(
            "calc".into(),
            AppIcon::Pack {
                pack: "retro".into(),
            },
        );
        assert_eq!(
            icon_key(&dock, "Calc", true).0.as_deref(),
            Some("pack:retro")
        );
        assert!(icon_key(&dock, "Calc", true).2);
    }

    #[test]
    fn a_widget_that_takes_widths_lists_them_first() {
        let items = width_items(1..=3, 2);
        assert_eq!(
            labels(&items),
            ["# Tamanho", "1 ícone", "2 ícones ✓", "3 ícones", "—"]
        );
        assert_eq!(items[3].choice, Some(MenuChoice::Width(3)));
        // One width: nothing to choose.
        assert!(width_items(1..=1, 1).is_empty());
    }

    #[test]
    fn a_widget_menu_can_remove_it_with_or_without_widths() {
        let activity = crate::domain::widgets::ActivityConfig {
            width: 2,
            ..Default::default()
        };
        let items = widget_items(&WidgetConfig::Activity(activity));
        assert_eq!(
            labels(&items),
            [
                "# Tamanho",
                "1 ícone",
                "2 ícones ✓",
                "3 ícones",
                "—",
                "Remover do dock",
                "—"
            ]
        );
        assert_eq!(items[5].choice, Some(MenuChoice::RemoveWidget));
        // A note takes one width: only the removal.
        let note = WidgetConfig::default_for(crate::domain::widgets::WidgetKind::Note);
        let items = widget_items(&note);
        assert_eq!(labels(&items), ["Remover do dock", "—"]);
    }

    #[test]
    fn a_pinned_program_menu_opens_closes_and_unpins() {
        let pinned = Program::Pinned("01a1a2d0-5d5d-70a1-be42-91ab4f4ea892".into());
        let closed = program_items(&pinned, 0);
        assert_eq!(labels(&closed), ["Abrir", "Remover do dock"]);
        assert_eq!(closed[1].choice, Some(MenuChoice::Unpin));
        let open = program_items(&pinned, 1);
        assert_eq!(labels(&open), ["Abrir", "Fechar janela", "Remover do dock"]);
        assert_eq!(open[0].choice, Some(MenuChoice::OpenProgram));
        assert_eq!(open[1].choice, Some(MenuChoice::CloseWindows));
        assert_eq!(labels(&program_items(&pinned, 3))[1], "Fechar janelas");
    }

    #[test]
    fn an_open_program_menu_is_not_unpinned() {
        let other = Program::Open("open-key".into());
        assert_eq!(
            labels(&program_items(&other, 2)),
            ["Abrir", "Fechar janelas"]
        );
    }

    #[test]
    fn the_launcher_menu_sets_the_hotkey_and_removes_it() {
        let launcher = WidgetConfig::default_for(crate::domain::widgets::WidgetKind::Launcher);
        let items = widget_items(&launcher);
        assert_eq!(
            labels(&items),
            ["Configurar atalho…", "Remover do dock", "—"]
        );
        assert_eq!(items[0].choice, Some(MenuChoice::SetHotkey));
        assert_eq!(items[1].choice, Some(MenuChoice::RemoveWidget));
    }

    #[test]
    fn only_the_launcher_is_one() {
        use crate::domain::widgets::{WidgetId, WidgetInstance, WidgetKind};
        let widget = |kind| {
            Item::Widget(WidgetInstance {
                id: WidgetId::new(),
                label: None,
                config: WidgetConfig::default_for(kind),
            })
        };
        assert!(is_launcher(&widget(WidgetKind::Launcher)));
        assert!(!is_launcher(&widget(WidgetKind::Note)));
        assert!(!is_launcher(&Item::System(SystemItem::Settings)));
    }

    #[test]
    fn a_bounce_rises_away_from_the_edge_and_lands() {
        let at = |ms: u64, edge| {
            opening_offset(OpenAnimation::Bounce, Duration::from_millis(ms), 48.0, edge)
        };
        assert_eq!(at(0, DockEdge::Bottom), (0.0, 0.0, 1.0));
        assert!((at(210, DockEdge::Bottom).1 + 24.0).abs() < 1e-6);
        assert!((at(210, DockEdge::Top).1 - 24.0).abs() < 1e-6);
        assert_eq!(at(1400, DockEdge::Bottom), (0.0, 0.0, 1.0));
    }

    #[test]
    fn a_pulse_swells_and_a_shake_moves_sideways() {
        let pulse = opening_offset(
            OpenAnimation::Pulse,
            Duration::from_millis(250),
            48.0,
            DockEdge::Bottom,
        );
        assert!((pulse.2 - 1.22).abs() < 1e-6);
        let shake = opening_offset(
            OpenAnimation::Shake,
            Duration::from_millis(100),
            48.0,
            DockEdge::Bottom,
        );
        assert!((shake.0 + 4.0).abs() < 1e-6);
        assert_eq!(
            opening_offset(
                OpenAnimation::Pulse,
                Duration::from_millis(1600),
                48.0,
                DockEdge::Bottom
            ),
            (0.0, 0.0, 1.0)
        );
    }
}
