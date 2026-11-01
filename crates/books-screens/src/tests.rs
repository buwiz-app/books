use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex, mpsc},
    time::{Duration, Instant},
};

use buwiz_app::{Bridge, BuwizError, Done, Host, Listener};
use nsh_core::{
    ApproxTextEngine, EventKind, EventMask, EventPayload, MountBatch, MountItem, NodeId,
};
use nsh_engine::{App, Engine, EngineConfig, MountSink};
use serde_json::{Map, Value, json};

use super::*;

struct FakeHost {
    props: Value,
    answers: Mutex<VecDeque<Result<Value, BuwizError>>>,
    sent: Mutex<Vec<(String, Value)>>,
}

impl Host for FakeHost {
    fn params(&self) -> Map<String, Value> {
        json!({
            "props": self.props,
            "app": {"plugin": "app.buwiz.books", "app": "quiet"},
            "granted": ["buwiz.call", "buwiz.fit", "buwiz.open"],
            "viewport": {"width": 360, "height": 160, "placement": "inline"}
        })
        .as_object()
        .unwrap()
        .clone()
    }

    fn post(&self, method: &str, data: Value) {
        self.sent.lock().unwrap().push((method.to_owned(), data));
    }

    fn call(&self, method: &str, data: Value, done: Done) {
        self.sent.lock().unwrap().push((method.to_owned(), data));
        let answer = if method == "buwiz.call" {
            self.answers.lock().unwrap().pop_front().unwrap_or_else(|| {
                Err(BuwizError::new(
                    "missing_answer",
                    false,
                    "Missing synthetic answer",
                ))
            })
        } else {
            Ok(Value::Null)
        };
        done(answer);
    }

    fn listen(&self, _: Listener) {}

    fn fail(&self, message: &str) {
        panic!("screen failed: {message}");
    }
}

struct Sink(mpsc::Sender<MountBatch>);

impl MountSink for Sink {
    fn submit(&self, batch: MountBatch) {
        let _ = self.0.send(batch);
    }
}

fn drive(
    card: &str,
    answer: Result<Value, BuwizError>,
    expected: &str,
    args: Value,
    press: Option<(&str, &str, Value)>,
) {
    let host = Arc::new(FakeHost {
        props: json!({"card":card}),
        answers: Mutex::new(VecDeque::from([answer])),
        sent: Mutex::new(Vec::new()),
    });
    let def = screens()
        .iter()
        .find(|screen| screen.id == "books.quiet")
        .expect("quiet screen is registered");
    let (tx, rx) = mpsc::channel();
    let engine = Engine::start(
        EngineConfig::new(App::Rust((def.build)(Bridge::new(host.clone())))),
        || ApproxTextEngine,
        Sink(tx),
    )
    .expect("engine starts");
    let control = engine.control();
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut shown = Vec::new();
    let mut parents = HashMap::<NodeId, NodeId>::new();
    let mut events = HashMap::<NodeId, EventMask>::new();
    let mut labels = HashMap::<NodeId, String>::new();
    let mut pressed = false;
    while Instant::now() < deadline {
        if let Ok(batch) = rx.recv_timeout(Duration::from_millis(20)) {
            for item in batch.items {
                match item {
                    MountItem::Insert { parent, child, .. } => {
                        parents.insert(child, parent);
                    }
                    MountItem::Events { id, events: mask } => {
                        events.insert(id, mask);
                    }
                    MountItem::Text { id, text, .. } => {
                        let value = text.text.to_string();
                        labels.insert(id, value.clone());
                        shown.push(value);
                    }
                    _ => {}
                }
            }
        }
        if let Some((label, _, _)) = &press
            && !pressed
            && let Some(id) = press_node(label, &labels, &parents, &events)
        {
            control.dispatch_event(id, EventKind::Press, EventPayload::None);
            pressed = true;
        }
        let sent = host.sent.lock().unwrap();
        let called = sent.iter().any(|(method, data)| {
            method == "buwiz.call"
                && data.get("tool").and_then(Value::as_str) == Some("books_nudge")
                && data.get("args") == Some(&args)
        });
        let ready = sent.iter().any(|(method, _)| method == "buwiz.ready");
        let action_done = press.as_ref().is_none_or(|(_, method, data)| {
            sent.iter()
                .any(|(sent_method, sent_data)| sent_method == *method && sent_data == data)
        });
        if called && ready && shown.iter().any(|text| text.contains(expected)) && action_done {
            break;
        }
    }
    drop(engine);
    assert!(
        shown.iter().any(|text| text.contains(expected)),
        "{shown:?}"
    );
    let sent = host.sent.lock().unwrap();
    assert!(
        sent.iter().any(|(method, data)| {
            method == "buwiz.call"
                && data.get("tool").and_then(Value::as_str) == Some("books_nudge")
                && data.get("args") == Some(&args)
        }),
        "{sent:?}"
    );
    assert!(
        sent.iter().any(|(method, _)| method == "buwiz.ready"),
        "{sent:?}"
    );
    if let Some((_, method, data)) = press {
        assert!(
            sent.iter()
                .any(|(sent_method, sent_data)| sent_method == method && sent_data == &data),
            "{method}: {data:?}; {sent:?}"
        );
    }
}

fn press_node(
    label: &str,
    labels: &HashMap<NodeId, String>,
    parents: &HashMap<NodeId, NodeId>,
    events: &HashMap<NodeId, EventMask>,
) -> Option<NodeId> {
    for (id, text) in labels {
        if text != label {
            continue;
        }
        let mut current = *id;
        while let Some(parent) = parents.get(&current) {
            if events
                .get(&current)
                .is_some_and(|mask| mask.contains(EventKind::Press))
            {
                return Some(current);
            }
            current = *parent;
        }
        if events
            .get(&current)
            .is_some_and(|mask| mask.contains(EventKind::Press))
        {
            return Some(current);
        }
    }
    None
}

#[test]
fn tidy_uses_the_tsx_args_and_pluralization() {
    drive(
        "tidy",
        Ok(json!({"counts":{"lines":1,"drafts":2,"posted":3}})),
        "1 new bank line · 2 drafted · 3 posted",
        json!({"kind":"tidy","any_day":true}),
        None,
    );
}

#[test]
fn month_end_renders_the_settled_state() {
    drive(
        "month_end",
        Ok(json!({"month_label":"April","closed":true})),
        "April is closed",
        json!({"kind":"month_end"}),
        None,
    );
}

#[test]
fn handoff_renders_a_refresh_notice() {
    drive(
        "handoff",
        Ok(
            json!({"count":1,"forms":[{"title":"BIR hand-off · Synthetic Form","why":"A line changed"}]}),
        ),
        "A line changed after it was handed to BIR. 1 form to refresh.",
        json!({"kind":"bir_pair"}),
        None,
    );
}

#[test]
fn failed_reads_show_the_host_error() {
    drive(
        "tidy",
        Err(BuwizError::new(
            "read_failed",
            false,
            "Synthetic read failure",
        )),
        "Synthetic read failure",
        json!({"kind":"tidy","any_day":true}),
        None,
    );
}

#[test]
fn tidy_action_opens_the_ledger_page() {
    drive(
        "tidy",
        Ok(json!({"counts":{"lines":1,"drafts":0,"posted":0}})),
        "1 new bank line · 0 drafted · 0 posted",
        json!({"kind":"tidy","any_day":true}),
        Some((
            "Review 1",
            "buwiz.open",
            json!({"app":"books","props":{"page":"ledger"}}),
        )),
    );
}

#[test]
fn tidy_screen_matches_its_recorded_tsx_transcript() {
    let fixture: Value =
        serde_json::from_str(include_str!("../../../tests/fixtures/screens/tidy-1.json")).unwrap();
    let expected = fixture["transcript"]["text"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .find(|text| text.contains("new bank line"))
        .unwrap();
    let args = fixture["transcript"]["calls"]
        .as_array()
        .unwrap()
        .iter()
        .find(|call| call["method"] == "call")
        .unwrap()["args"]
        .clone();
    let open = fixture["transcript"]["calls"]
        .as_array()
        .unwrap()
        .iter()
        .find(|call| call["method"] == "open")
        .unwrap();
    drive(
        "tidy",
        Ok(fixture["answers"][0]["data"].clone()),
        expected,
        args,
        Some((
            "Review 1",
            "buwiz.open",
            json!({"app":open["app"]["app"],"props":open["app"]["props"]}),
        )),
    );
}

#[test]
fn quiet_manifest_permission_grants_only_its_read_tool_and_open() {
    let manifest = books_mini_app::manifest::unsigned("books-test-01");
    let quiet = manifest["apps"]
        .as_array()
        .unwrap()
        .iter()
        .find(|app| app["id"] == "quiet")
        .unwrap();
    assert_eq!(quiet["permissions"]["tools"], json!(["books_nudge"]));
    assert_eq!(quiet["permissions"]["open"], json!({"apps":["books"]}));
}
