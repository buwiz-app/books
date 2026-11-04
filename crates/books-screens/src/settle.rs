//! Waiting for a screen under test to finish what it was asked to do.

use std::{
    collections::{HashMap, HashSet},
    sync::mpsc,
    time::Duration,
};

use nsh_core::{EventMask, MountBatch, MountItem, NodeId};
use nsh_engine::EngineControl;

/// How long a screen may take to settle before its test fails. It bounds a
/// hang only: a passing run never waits for it.
const HANG: Duration = Duration::from_secs(60);

/// What a screen's mount batches have shown so far: its views' parents, event
/// masks and labels, and every text it has shown.
#[derive(Default)]
pub(crate) struct Mounted {
    pub(crate) parents: HashMap<NodeId, NodeId>,
    pub(crate) events: HashMap<NodeId, EventMask>,
    pub(crate) labels: HashMap<NodeId, String>,
    pub(crate) label_order: Vec<NodeId>,
    pub(crate) shown: Vec<String>,
    detached: HashSet<NodeId>,
}

impl Mounted {
    /// Folds in one item. `Remove` detaches a view and keeps it alive, so it
    /// keeps its entries and can come back; `Delete` destroys it.
    pub(crate) fn apply(&mut self, item: &MountItem) {
        match item {
            MountItem::Insert { parent, child, .. } => {
                self.parents.insert(*child, *parent);
                self.detached.remove(child);
            }
            MountItem::Events { id, events } => {
                self.events.insert(*id, *events);
            }
            MountItem::Remove { id } => {
                self.parents.remove(id);
                self.detached.insert(*id);
            }
            MountItem::Delete { id } => {
                self.parents.remove(id);
                self.events.remove(id);
                self.labels.remove(id);
                self.label_order.retain(|node| node != id);
                self.detached.remove(id);
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

    /// Whether `id` is on screen: neither it nor any view above it is detached.
    /// `Remove` names only the detached view, not the views under it.
    pub(crate) fn attached(&self, id: NodeId) -> bool {
        let mut current = id;
        loop {
            if self.detached.contains(&current) {
                return false;
            }
            match self.parents.get(&current) {
                Some(parent) => current = *parent,
                None => return true,
            }
        }
    }

    /// The labels on screen.
    pub(crate) fn live_labels(&self) -> impl Iterator<Item = &str> {
        self.labels
            .iter()
            .filter(|(id, _)| self.attached(**id))
            .map(|(_, text)| text.as_str())
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
/// tasks woken in it, up to nsh's `MAX_TASK_ROUNDS` (64), before it commits. This
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

#[cfg(test)]
mod tests {
    use super::*;

    fn insert(parent: NodeId, child: NodeId) -> MountItem {
        MountItem::Insert {
            parent,
            child,
            before: None,
        }
    }

    #[test]
    fn a_detached_view_takes_the_views_under_it_off_screen_until_it_returns() {
        let (root, row, label): (NodeId, NodeId, NodeId) = (1, 2, 3);
        let mut mounted = Mounted::default();
        mounted.apply(&insert(root, row));
        mounted.apply(&insert(row, label));
        assert!(mounted.attached(label));

        mounted.apply(&MountItem::Remove { id: row });
        assert!(!mounted.attached(row) && !mounted.attached(label));

        mounted.apply(&insert(root, row));
        assert!(mounted.attached(row) && mounted.attached(label));
    }

    #[test]
    fn a_deleted_view_is_forgotten() {
        let (root, row): (NodeId, NodeId) = (1, 2);
        let mut mounted = Mounted::default();
        mounted.apply(&insert(root, row));
        mounted.labels.insert(row, "Save".to_owned());
        mounted.label_order.push(row);

        mounted.apply(&MountItem::Delete { id: row });
        assert!(mounted.labels.is_empty() && mounted.label_order.is_empty());
        assert!(!mounted.parents.contains_key(&row));
    }
}
