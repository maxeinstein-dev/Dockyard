//! What the native windows of the app (the dock and the launcher) share:
//! their state lives on the main thread, and other threads hand them work
//! through a [`WorkQueue`], which wakes the window to run it there (M12).

use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::{Mutex, PoisonError};

#[cfg(windows)]
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;

/// Work for the state `T` of a window, from any thread.
pub type Work<T> = Box<dyn FnOnce(&mut T) + Send>;

/// The work other threads queued for a window's state, run in order on the
/// window's thread once it gets `message` (`drain`).
pub struct WorkQueue<T: 'static> {
    work: Mutex<Vec<Work<T>>>,
    /// The window woken with `message`; 0 before it is made (the work waits
    /// for the first `drain`).
    window: &'static AtomicIsize,
    message: u32,
}

impl<T> WorkQueue<T> {
    pub const fn new(window: &'static AtomicIsize, message: u32) -> Self {
        Self {
            work: Mutex::new(Vec::new()),
            window,
            message,
        }
    }

    /// Queues `work`, from any thread, and wakes the window.
    pub fn push(&self, work: Work<T>) {
        self.work
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(work);
        let hwnd = self.window.load(Ordering::SeqCst);
        #[cfg(windows)]
        if hwnd != 0 {
            let _ =
                unsafe { PostMessageW(Some(HWND(hwnd as _)), self.message, WPARAM(0), LPARAM(0)) };
        }
        #[cfg(not(windows))]
        let _ = (hwnd, self.message);
    }

    /// Hands each work queued to `with`, in order, until none is left: also
    /// what a work queues while it runs.
    pub fn drain(&self, mut with: impl FnMut(Work<T>)) {
        loop {
            let work =
                std::mem::take(&mut *self.work.lock().unwrap_or_else(PoisonError::into_inner));
            if work.is_empty() {
                return;
            }
            for item in work {
                with(item);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static NO_WINDOW: AtomicIsize = AtomicIsize::new(0);

    #[test]
    fn the_work_runs_in_order_with_what_it_queues() {
        static QUEUE: WorkQueue<Vec<u32>> = WorkQueue::new(&NO_WINDOW, 0);
        QUEUE.push(Box::new(|seen| seen.push(1)));
        QUEUE.push(Box::new(|seen| {
            seen.push(2);
            QUEUE.push(Box::new(|seen| seen.push(3)));
        }));
        let mut seen = Vec::new();
        QUEUE.drain(|work| work(&mut seen));
        assert_eq!(seen, [1, 2, 3]);
        // Nothing is left for the next message.
        QUEUE.drain(|_| panic!("ran twice"));
    }
}
