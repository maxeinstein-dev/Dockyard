//! A panic must not take the dock down (M4). The release build unwinds:
//! - the window procedures of the dock and of the launcher run each message
//!   through [`window_message`], which catches the panic and lets the
//!   procedure answer the message as Windows would;
//! - the threads that keep reading for the dock (media, badges, weather...)
//!   run through [`lasting`], which starts the work again after a panic, so
//!   a part of the dock does not stop updating in silence.
//!
//! The panic hook (`lib.rs`) logs every panic; for one caught here it leaves
//! the taskbar hidden: the dock goes on.

use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::{Duration, Instant};

/// The most times a work is started again after a panic within
/// [`RETRY_SPAN`]: a panic that comes back each time is left alone, not
/// looped on.
pub const MAX_RETRIES: usize = 3;
const RETRY_SPAN: Duration = Duration::from_secs(60);
/// How long a thread waits before it starts its work again.
const RETRY_PAUSE: Duration = Duration::from_secs(2);

/// Whether a work may start again `now`, given when it last did (`times`,
/// kept up to date).
pub fn may_retry(times: &mut Vec<Instant>, now: Instant) -> bool {
    times.retain(|then| now.saturating_duration_since(*then) < RETRY_SPAN);
    let may = times.len() < MAX_RETRIES;
    if may {
        times.push(now);
    }
    may
}

/// Starts a thread named `name` that runs `work`, and runs it again after a
/// panic (at most [`MAX_RETRIES`] times a minute; then the thread ends and
/// the log says so). The work ending on its own ends the thread.
pub fn lasting(
    name: &str,
    work: impl Fn() + Send + 'static,
) -> std::io::Result<std::thread::JoinHandle<()>> {
    let owned = name.to_owned();
    std::thread::Builder::new()
        .name(owned.clone())
        .spawn(move || {
            let mut times = Vec::new();
            while window_message(&work).is_none() {
                if !may_retry(&mut times, Instant::now()) {
                    log::error!("{owned}: panicked {MAX_RETRIES} times within a minute; stopped");
                    return;
                }
                log::warn!("{owned}: started again after a panic");
                std::thread::sleep(RETRY_PAUSE);
            }
        })
}

thread_local! {
    /// How many [`window_message`] calls are running on this thread: a
    /// window procedure can be entered again from inside one.
    static CATCHING: Cell<u32> = const { Cell::new(0) };
}

/// Whether a panic on this thread now would be caught by a window
/// procedure.
pub fn caught() -> bool {
    CATCHING.with(Cell::get) > 0
}

/// Runs `body`, the work of a window procedure for one message. `None`
/// when it panicked: the hook has logged the panic, and the procedure
/// answers the message without it.
pub fn window_message<T>(body: impl FnOnce() -> T) -> Option<T> {
    CATCHING.with(|count| count.set(count.get() + 1));
    let result = catch_unwind(AssertUnwindSafe(body));
    CATCHING.with(|count| count.set(count.get() - 1));
    result.ok()
}

/// The main thread's answer to work another thread sent it, waited for at
/// most 5 s. Longer means the main thread is stuck: the log says so, with
/// `whose` work it was.
pub fn answer<T>(received: &std::sync::mpsc::Receiver<T>, whose: &str) -> Option<T> {
    use std::sync::mpsc::RecvTimeoutError;
    match received.recv_timeout(std::time::Duration::from_secs(5)) {
        Ok(answer) => Some(answer),
        Err(RecvTimeoutError::Timeout) => {
            log::warn!("{whose}: the main thread did not answer in 5 s");
            None
        }
        Err(RecvTimeoutError::Disconnected) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_work_that_keeps_panicking_is_started_again_three_times_a_minute() {
        let start = Instant::now();
        let mut times = Vec::new();
        for second in 0..3 {
            assert!(may_retry(&mut times, start + Duration::from_secs(second)));
        }
        // The fourth within the minute is left alone, not looped on.
        assert!(!may_retry(&mut times, start + Duration::from_secs(10)));
        // A minute after the first, one more may come again.
        assert!(may_retry(&mut times, start + Duration::from_secs(61)));
    }

    #[test]
    fn a_lasting_thread_runs_its_work_again_after_a_panic() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static RUNS: AtomicUsize = AtomicUsize::new(0);
        let thread = lasting("test-lasting", || {
            // Panics the first time, ends the second.
            if RUNS.fetch_add(1, Ordering::SeqCst) == 0 {
                panic!("on purpose");
            }
        })
        .unwrap();
        thread.join().unwrap();
        assert_eq!(RUNS.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn a_panic_in_a_window_message_is_caught_and_the_thread_goes_on() {
        assert!(!caught());
        assert_eq!(window_message(|| 7), Some(7));
        let inner = window_message(|| {
            assert!(caught());
            // One message inside another: the outer one still catches.
            let nested = window_message(|| -> u8 { panic!("on purpose") });
            assert!(caught());
            nested
        });
        assert_eq!(inner, Some(None));
        assert!(!caught());
    }
}
