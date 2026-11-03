//! What happens after a panic in the dock's window: made again, or closed
//! after too many.

use super::*;

/// When the docks were last made again after a panic.
pub(super) static RECOVERIES: Mutex<Vec<Instant>> = Mutex::new(Vec::new());
/// After a panic in a message of the window `hwnd`: the state may be half
/// changed, so its dock is thrown away and made again from scratch, as for
/// a monitor plugged in. Every dock, when `hwnd` is the window that runs the
/// work of other threads, which may have touched any. Queued, outside the
/// message.
pub(super) fn recover(hwnd: HWND) {
    let next = after_panic(
        &mut RECOVERIES.lock().unwrap_or_else(PoisonError::into_inner),
        Instant::now(),
        &GAVE_UP,
    );
    match next {
        AfterPanic::MadeAgain => {}
        AfterPanic::Closed => return give_up(),
        AfterPanic::AlreadyClosing => return,
    }
    log::warn!("dock: made again after a panic");
    let gone = hwnd.0 as isize;
    queue(Box::new(move |docks| {
        let one = docks.iter().any(|dock| dock.hwnd.0 as isize == gone);
        docks.retain_mut(|dock| {
            let keep = one && dock.hwnd.0 as isize != gone;
            if !keep {
                dock.destroy();
            }
            keep
        });
        sync(docks);
    }));
}

/// Whether the app already gave up on the dock: once only.
pub(super) static GAVE_UP: AtomicBool = AtomicBool::new(false);

/// What a panic in a dock's message leads to.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum AfterPanic {
    /// The dock is made again from scratch.
    MadeAgain,
    /// It panicked too often: the app closes ([`give_up`]).
    Closed,
    /// The app is closing already: nothing more.
    AlreadyClosing,
}

/// What the panic at `now` leads to, given when the docks were last made
/// again (`times`) and whether the app gave up already (`gave_up`, set
/// here when it does).
pub(super) fn after_panic(
    times: &mut Vec<Instant>,
    now: Instant,
    gave_up: &AtomicBool,
) -> AfterPanic {
    if gave_up.load(Ordering::SeqCst) {
        AfterPanic::AlreadyClosing
    } else if super::super::panics::may_retry(times, now) {
        AfterPanic::MadeAgain
    } else if gave_up.swap(true, Ordering::SeqCst) {
        AfterPanic::AlreadyClosing
    } else {
        AfterPanic::Closed
    }
}

/// The steps of closing after the dock kept panicking, apart so the tests
/// can check their order.
pub(super) trait Closing {
    fn destroy_docks(&mut self);
    /// The tray, the edge of the screen and the Windows taskbar, as on exit.
    fn release_taskbar(&mut self);
    /// The Windows notice; `false` when Windows has the notices off.
    fn notify(&mut self) -> bool;
    fn message_box(&mut self);
    fn exit(&mut self);
}

/// In this order: the docks go, the taskbar comes back, a notice says what
/// happened (a message box when the notices are off) and the app exits. A
/// dock that panics again while it goes does not stop the rest.
pub(super) fn close_after_failures(steps: &mut impl Closing) {
    let _ = super::super::panics::window_message(|| steps.destroy_docks());
    steps.release_taskbar();
    if !steps.notify() {
        steps.message_box();
    }
    steps.exit();
}

/// The real steps, on the docks of the main thread.
pub(super) struct ClosingForReal<'a> {
    docks: &'a mut Vec<Dock>,
}

impl Closing for ClosingForReal<'_> {
    fn destroy_docks(&mut self) {
        for dock in self.docks.iter_mut() {
            dock.destroy();
        }
        self.docks.clear();
    }

    fn release_taskbar(&mut self) {
        super::super::tray_host::stop();
        super::super::appbar::remove();
        super::super::taskbar::release();
        super::super::window_spaces::release_all();
    }

    fn notify(&mut self) -> bool {
        let text = &crate::i18n::messages().dock;
        super::super::toast::show(text.crash_notice_title, text.crash_notice_line).unwrap_or(false)
    }

    fn message_box(&mut self) {
        let text = &crate::i18n::messages().dock;
        // The test build only logs it: a box would wait for a click that
        // never comes, and the suite checks that the app closes.
        #[cfg(feature = "e2e")]
        log::warn!("e2e: {}", text.crash_notice_line);
        #[cfg(not(feature = "e2e"))]
        {
            use windows::Win32::UI::WindowsAndMessaging::{MB_ICONWARNING, MB_OK, MessageBoxW};
            let title = windows::core::HSTRING::from(text.crash_notice_title);
            let line = windows::core::HSTRING::from(text.crash_notice_line);
            unsafe { MessageBoxW(None, &line, &title, MB_OK | MB_ICONWARNING) };
        }
    }

    fn exit(&mut self) {
        if let Some(app) = APP.get() {
            app.exit(1);
        }
    }
}

/// The dock kept panicking: making it again does not help, and an app left
/// open with no dock would keep the taskbar hidden. So the app closes, the
/// way [`close_after_failures`] says. Queued, outside the message that
/// panicked.
pub(super) fn give_up() {
    log::error!(
        "dock: it panicked {} times within a minute; closing",
        super::super::panics::MAX_RETRIES
    );
    queue(Box::new(|docks| {
        close_after_failures(&mut ClosingForReal { docks });
    }));
}
