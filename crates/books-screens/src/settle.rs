//! Waiting for a screen under test to finish what it was asked to do.

use std::{sync::mpsc, time::Duration};

use nsh_engine::EngineControl;

/// How long a screen may take to settle before its test fails. It bounds a
/// hang only: a passing run never waits for it.
const HANG: Duration = Duration::from_secs(60);

/// Returns once the engine has handled every message sent to it before this
/// call (a start, an event, a layout) and submitted the mount batches that
/// work made, so the test can drain its sink and see the whole result.
///
/// A zero `advance_clock` on the system clock is that barrier: its callback
/// runs after a turn that follows the earlier messages, and a turn polls
/// every task woken in it before it commits. The test hosts answer on the
/// engine thread, so a press, the calls it makes, their answers and the
/// re-render all land before the callback. Timers, such as a toast's 2.6 s
/// clear, are not waited for.
pub(crate) fn settle(control: &EngineControl) {
    let (done, settled) = mpsc::channel();
    control.advance_clock(Duration::ZERO, move || {
        let _ = done.send(());
    });
    match settled.recv_timeout(HANG) {
        Ok(()) => {}
        Err(mpsc::RecvTimeoutError::Timeout) => {
            panic!("the screen did not settle within {HANG:?}")
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            panic!("the engine stopped before the screen settled")
        }
    }
}
