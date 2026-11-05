//! Android event-thread stall watchdog. The event thread names the step it
//! is in; a watcher thread logs that name to the diagnostic log when the
//! step lasts 2 s or more, so a freeze report says where the game stopped.
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

struct Mark {
    stage: &'static str,
    at: Instant,
    reported: bool,
}
static MARK: Mutex<Option<Mark>> = Mutex::new(None);
static STARTED: AtomicBool = AtomicBool::new(false);
const LIMIT: Duration = Duration::from_secs(2);

/// The event thread is now in `name`.
pub(crate) fn stage(name: &'static str) {
    let mut mark = MARK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(previous) = mark.as_ref()
        && previous.reported
    {
        log::warn!(
            "Horizon stall: '{}' finished after {} ms",
            previous.stage,
            previous.at.elapsed().as_millis()
        );
    }
    *mark = Some(Mark {
        stage: name,
        at: Instant::now(),
        reported: false,
    });
}

/// The event thread is waiting for the OS: not a stall.
pub(crate) fn idle() {
    let mut mark = MARK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(previous) = mark.as_ref()
        && previous.reported
    {
        log::warn!(
            "Horizon stall: '{}' finished after {} ms",
            previous.stage,
            previous.at.elapsed().as_millis()
        );
    }
    *mark = None;
}

/// Start the watcher once per process.
pub(crate) fn start() {
    if STARTED.swap(true, Ordering::AcqRel) {
        return;
    }
    let spawned = std::thread::Builder::new()
        .name("Stall watchdog".into())
        .spawn(|| {
            loop {
                std::thread::sleep(Duration::from_millis(500));
                let mut mark = MARK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                if let Some(m) = mark.as_mut()
                    && !m.reported
                    && m.at.elapsed() >= LIMIT
                {
                    m.reported = true;
                    log::warn!(
                        "Horizon stall: event thread stuck in '{}' for {} ms",
                        m.stage,
                        m.at.elapsed().as_millis()
                    );
                    drop(mark);
                    log::logger().flush();
                }
            }
        });
    if let Err(error) = spawned {
        log::warn!("Stall watchdog: {error}");
    }
}

/// Marks `name` until dropped, whichever way the scope ends.
pub(crate) struct Scope;
impl Drop for Scope {
    fn drop(&mut self) {
        idle();
    }
}
pub(crate) fn scope(name: &'static str) -> Scope {
    stage(name);
    Scope
}
