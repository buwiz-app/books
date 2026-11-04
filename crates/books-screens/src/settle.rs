//! Waiting for a screen under test to finish what it was asked to do.

use std::{collections::HashMap, sync::mpsc, time::Duration};

use nsh_core::{EventMask, MountBatch, MountItem, NodeId};
use nsh_engine::EngineControl;

/// How long a screen may take to settle before its test fails. It bounds a
/// hang only: a passing run never waits for it.
const HANG: Duration = Duration::from_secs(60);

/// What a screen's mount batches have shown so far: its live nodes' parents,
/// event masks and labels, and every text it has shown.
#[derive(Default)]
pub(crate) struct Mounted {
    pub(crate) parents: HashMap<NodeId, NodeId>,
    pub(crate) events: HashMap<NodeId, EventMask>,
    pub(crate) labels: HashMap<NodeId, String>,
    pub(crate) label_order: Vec<NodeId>,
    pub(crate) shown: Vec<String>,
}

impl Mounted {
    /// Folds in one item. A removed node's entries go with it, so a press
    /// can't land on a node the screen has dropped.
    pub(crate) fn apply(&mut self, item: &MountItem) {
        match item {
            MountItem::Insert { parent, child, .. } => {
                self.parents.insert(*child, *parent);
            }
            MountItem::Events { id, events } => {
                self.events.insert(*id, *events);
            }
            MountItem::Remove { id } | MountItem::Delete { id } => {
                self.parents.remove(id);
                self.events.remove(id);
                self.labels.remove(id);
                self.label_order.retain(|node| node != id);
            }
            MountItem::Text { id, text, .. } => {
                let value = text.text.to_string();
                if !self.labels.contains_key(id) {
                    self.label_order.push(*id);
                }
                self.labels.insert(*id, value.clone());
                self.shown.push(value);
            }
            _ => {}
        }
    }
}

/// Waits for the screen to settle, then hands `each` every batch it sent.
pub(crate) fn catch_up(
    control: &EngineControl,
    batches: &mpsc::Receiver<MountBatch>,
    mut each: impl FnMut(MountBatch),
) {
    settle(control);
    while let Ok(batch) = batches.try_recv() {
        each(batch);
    }
}

/// Returns once the engine has handled every message sent to it before this
/// call (a start, an event, a layout) and submitted the mount batches that
/// work made, so the test can drain its sink and see the whole result.
///
/// A zero `advance_clock` on the system clock is that barrier: its callback
/// runs after a turn that follows the earlier messages, and a turn polls the
/// tasks woken in it, up to its limit of 64 rounds, before it commits. This
/// rests on nsh's system-clock path, so re-check it on every nsh bump.
///
/// Two things it doesn't cover:
/// - Answers from another thread. `Done` is `Send`, but both test hosts call
///   it inline from `Host::call`, so a press, the calls it makes, their
///   answers and the re-render all land before the callback. A host that
///   answers elsewhere needs its own bounded wait on its call count.
/// - Timers, such as a toast's 2.6 s clear. A screen that starts work behind
///   a timer still shows its old state here, so its test must advance the
///   clock past that timer first.
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
