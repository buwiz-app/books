use std::{
    collections::{HashMap, VecDeque},
    fs,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

use buwiz_app::{Bridge, BuwizError, Done, Host, Listener};
use nsh_core::{
    ApproxTextEngine, EventKind, EventMask, EventPayload, MountBatch, MountItem, NodeId,
    geometry::{Bounds, Point, Points, Size},
};
use nsh_engine::{App, Engine, EngineConfig, EngineControl, MountSink};
use serde_json::{Map, Value, json};

use super::*;

struct FakeHost {
    props: Value,
    answers: Mutex<VecDeque<Result<Value, BuwizError>>>,
    sent: Mutex<Vec<(String, Value)>>,
    served: Mutex<Vec<Value>>,
    open_error: Option<String>,
    started: Instant,
    ready_at: Mutex<Option<Duration>>,
    timeline: Arc<Timeline>,
}

#[derive(Default)]
struct Timeline {
    next: AtomicUsize,
    fit_events: Mutex<Vec<usize>>,
    last_read_settled: Mutex<Option<usize>>,
    open_error_rendered: Mutex<Option<usize>>,
}

impl Timeline {
    fn mark(&self) -> usize {
        self.next.fetch_add(1, Ordering::SeqCst) + 1
    }
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
        if method == "buwiz.ready" {
            *self.ready_at.lock().unwrap() = Some(self.started.elapsed());
        }
        if method == "buwiz.fit" {
            self.timeline
                .fit_events
                .lock()
                .unwrap()
                .push(self.timeline.mark());
        }
        self.sent.lock().unwrap().push((method.to_owned(), data));
    }

    fn call(&self, method: &str, data: Value, done: Done) {
        self.sent.lock().unwrap().push((method.to_owned(), data));
        let answer = if method == "buwiz.call" {
            let answer = self.answers.lock().unwrap().pop_front().unwrap_or_else(|| {
                Err(BuwizError::new(
                    "missing_answer",
                    false,
                    "Missing synthetic answer",
                ))
            });
            self.served.lock().unwrap().push(match &answer {
                Ok(value) => json!({"data":value}),
                Err(error) => json!({"error":error.message}),
            });
            *self.timeline.last_read_settled.lock().unwrap() = Some(self.timeline.mark());
            answer
        } else if method == "buwiz.open" {
            self.open_error.as_ref().map_or(Ok(Value::Null), |message| {
                Err(BuwizError::new("open_failed", false, message.clone()))
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

struct Sink {
    sender: mpsc::Sender<MountBatch>,
    open_error: Option<String>,
    timeline: Arc<Timeline>,
}

impl MountSink for Sink {
    fn submit(&self, batch: MountBatch) {
        if self.open_error.as_ref().is_some_and(|message| {
            batch.items.iter().any(|item| {
                matches!(
                    item,
                    MountItem::Text { text, .. } if text.text.to_string().contains(message)
                )
            })
        }) {
            *self.timeline.open_error_rendered.lock().unwrap() = Some(self.timeline.mark());
        }
        let _ = self.sender.send(batch);
    }
}

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/screens/quiet")
}

fn fixture_names() -> Vec<String> {
    let mut names = fs::read_dir(fixture_dir())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".json"))
        .collect::<Vec<_>>();
    names.sort();
    names
}

fn fixture_answer(value: &Value) -> Result<Value, BuwizError> {
    if let Some(message) = value.get("error").and_then(Value::as_str) {
        Err(BuwizError::new("read_failed", false, message))
    } else {
        Ok(value["data"].clone())
    }
}

fn normalize_calls(sent: &[(String, Value)]) -> Vec<Value> {
    sent.iter()
        .filter_map(|(method, data)| match method.as_str() {
            "buwiz.call" => Some(json!({
                "method":"call",
                "tool":data["tool"],
                "args":data["args"]
            })),
            "buwiz.open" => Some(json!({
                "method":"open",
                "app":data["app"],
                "props":data["props"]
            })),
            "buwiz.ready" => Some(json!({"method":"ready"})),
            _ => None,
        })
        .collect()
}

fn trigger_final_layout(control: &EngineControl, node: NodeId, height: f64) {
    let frame = Bounds::new(
        Point::default(),
        Size::new(Points(0.0), Points((height + 1.0) as f32)),
    );
    control.dispatch_event(node, EventKind::Layout, EventPayload::Layout(frame));
}

fn drive_fixture(fixture: &Value) -> f64 {
    let answers = fixture["answers"]
        .as_array()
        .unwrap()
        .iter()
        .map(fixture_answer)
        .collect::<VecDeque<_>>();
    let timeline = Arc::new(Timeline::default());
    let open_error = fixture["open_error"].as_str().map(str::to_owned);
    let host = Arc::new(FakeHost {
        props: fixture["props"].clone(),
        answers: Mutex::new(answers),
        sent: Mutex::new(Vec::new()),
        served: Mutex::new(Vec::new()),
        open_error: open_error.clone(),
        started: Instant::now(),
        ready_at: Mutex::new(None),
        timeline: timeline.clone(),
    });
    let def = screens()
        .iter()
        .find(|screen| screen.id == "books.quiet")
        .expect("quiet screen is registered");
    let (tx, rx) = mpsc::channel();
    let engine = Engine::start(
        EngineConfig::new(App::Rust((def.build)(Bridge::new(host.clone())))),
        || ApproxTextEngine,
        Sink {
            sender: tx,
            open_error,
            timeline: timeline.clone(),
        },
    )
    .expect("engine starts");
    let control = engine.control();
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut shown = Vec::new();
    let mut parents = HashMap::<NodeId, NodeId>::new();
    let mut events = HashMap::<NodeId, EventMask>::new();
    let mut labels = HashMap::<NodeId, String>::new();
    let mut layout_node = None;
    let presses = fixture["press"].as_array().unwrap();
    let mut press_index = 0;
    let expected_calls = fixture["transcript"]["calls"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|call| call["method"] != "fit")
        .count();
    let expected_fit = fixture["transcript"]["calls"]
        .as_array()
        .unwrap()
        .iter()
        .any(|call| call["method"] == "fit");
    assert!(
        expected_fit,
        "{} fixture has no fit golden",
        fixture["props"]
    );
    while Instant::now() < deadline {
        if let Ok(batch) = rx.recv_timeout(Duration::from_millis(20)) {
            for item in batch.items {
                match item {
                    MountItem::Insert { parent, child, .. } => {
                        parents.insert(child, parent);
                    }
                    MountItem::Events { id, events: mask } => {
                        events.insert(id, mask);
                        if mask.contains(EventKind::Layout) {
                            layout_node = Some(id);
                        } else if layout_node == Some(id) {
                            layout_node = None;
                        }
                    }
                    MountItem::Remove { id } | MountItem::Delete { id }
                        if layout_node == Some(id) =>
                    {
                        layout_node = None;
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
        if let Some(label) = presses.get(press_index).and_then(Value::as_str)
            && let Some(id) = press_node(label, &labels, &parents, &events)
        {
            control.dispatch_event(id, EventKind::Press, EventPayload::None);
            press_index += 1;
        }
        let sent = host.sent.lock().unwrap();
        let calls = normalize_calls(&sent);
        let has_fit = sent.iter().any(|(method, _)| method == "buwiz.fit");
        let expected_text = fixture["transcript"]["text"].as_array().unwrap();
        let all_text = expected_text.iter().all(|expected| {
            expected.as_str().is_some_and(|expected| {
                shown
                    .iter()
                    .any(|actual| actual.eq_ignore_ascii_case(expected))
            })
        });
        if press_index == presses.len() && calls.len() == expected_calls && has_fit && all_text {
            break;
        }
    }
    let layout_node = layout_node.expect("quiet screen has a layout callback");
    let last_height = host
        .sent
        .lock()
        .unwrap()
        .iter()
        .rev()
        .find(|(method, _)| method == "buwiz.fit")
        .and_then(|(_, data)| data["height"].as_f64())
        .expect("quiet screen has a fit height");
    let previous_fits = timeline.fit_events.lock().unwrap().len();
    trigger_final_layout(&control, layout_node, last_height);
    let fit_deadline = Instant::now() + Duration::from_secs(1);
    while timeline.fit_events.lock().unwrap().len() <= previous_fits {
        assert!(
            Instant::now() < fit_deadline,
            "{} final layout did not emit fit",
            fixture["props"]
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    drop(engine);
    let sent = host.sent.lock().unwrap();
    let calls = normalize_calls(&sent);
    let expected_transcript = fixture["transcript"]["calls"].as_array().unwrap();
    let expected_calls = expected_transcript
        .iter()
        .filter(|call| call["method"] != "fit")
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(
        calls,
        expected_calls,
        "{} presses {press_index}/{}; visible {shown:?}; labels {labels:?}",
        fixture["props"],
        presses.len()
    );
    let fit_data = sent
        .iter()
        .filter(|(method, _)| method == "buwiz.fit")
        .map(|(_, data)| data)
        .collect::<Vec<_>>();
    assert!(!fit_data.is_empty(), "{} sent no fit", fixture["props"]);
    for data in &fit_data {
        let height = data["height"].as_f64().unwrap_or(f64::NAN);
        assert!(
            height.is_finite() && height > 0.0,
            "{} invalid fit height: {data}",
            fixture["props"]
        );
    }
    for expected in fixture["transcript"]["text"].as_array().unwrap() {
        let expected = expected.as_str().unwrap();
        assert!(
            shown
                .iter()
                .any(|actual| actual.eq_ignore_ascii_case(expected)),
            "{} missing {expected:?} from {shown:?}",
            fixture["props"]
        );
    }
    assert_eq!(
        host.served.lock().unwrap().as_slice(),
        fixture["answers"].as_array().unwrap().as_slice()
    );
    let last_fit = *timeline.fit_events.lock().unwrap().last().unwrap();
    if fixture["open_error"].is_string() {
        let toast_rendered = timeline.open_error_rendered.lock().unwrap().unwrap();
        assert!(
            last_fit > toast_rendered,
            "{} last fit preceded the open-error toast render: fit {last_fit}, toast {toast_rendered}",
            fixture["props"]
        );
    } else {
        let last_read_settled = timeline.last_read_settled.lock().unwrap().unwrap();
        assert!(
            last_fit > last_read_settled,
            "{} last fit preceded the last read settling: fit {last_fit}, read {last_read_settled}",
            fixture["props"]
        );
    }
    host.ready_at.lock().unwrap().unwrap().as_secs_f64() * 1000.0
}

fn press_node(
    label: &str,
    labels: &HashMap<NodeId, String>,
    parents: &HashMap<NodeId, NodeId>,
    events: &HashMap<NodeId, EventMask>,
) -> Option<NodeId> {
    for (id, text) in labels {
        if !text.eq_ignore_ascii_case(label) {
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
fn quiet_fixtures_match_native_transcripts() {
    let expected_names = [
        "handoff-0.json",
        "handoff-1.json",
        "handoff-2.json",
        "month-closed.json",
        "month-open-1.json",
        "month-open-2.json",
        "open-failure.json",
        "read-error.json",
        "retry-read-error.json",
        "tidy-0.json",
        "tidy-1.json",
        "tidy-3.json",
    ]
    .map(str::to_owned)
    .to_vec();
    assert_eq!(fixture_names(), expected_names, "quiet fixture set changed");
    for name in expected_names {
        let fixture: Value =
            serde_json::from_slice(&fs::read(fixture_dir().join(&name)).unwrap()).unwrap();
        assert!(fixture["props"]["card"].is_string(), "{name}");
        drive_fixture(&fixture);
    }
}

#[test]
#[ignore = "native release timing"]
fn native_quiet_timing_50() {
    for name in fixture_names() {
        let fixture: Value =
            serde_json::from_slice(&fs::read(fixture_dir().join(&name)).unwrap()).unwrap();
        let mut values = (0..50).map(|_| drive_fixture(&fixture)).collect::<Vec<_>>();
        values.sort_by(f64::total_cmp);
        let median = values[24];
        let p95 = values[47];
        println!("TIMING native quiet {name}: median {median:.2} ms, p95 {p95:.2} ms (50 runs)");
    }
}

#[test]
#[ignore = "W9 quiet screen timing sample"]
fn quiet_timing_samples() {
    let expected_names = fixture_names();
    for name in expected_names {
        let fixture: Value =
            serde_json::from_slice(&fs::read(fixture_dir().join(&name)).unwrap()).unwrap();
        let mut times = (0..50).map(|_| drive_fixture(&fixture)).collect::<Vec<_>>();
        times.sort_by(f64::total_cmp);
        println!(
            "TIMING native quiet {name}: median {:.2} ms, p95 {:.2} ms (50 runs)",
            times[24], times[47]
        );
    }
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
