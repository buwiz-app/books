use std::{
    collections::{HashMap, VecDeque},
    fs,
    path::PathBuf,
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{Duration, Instant},
};

use buwiz_app::{Bridge, BuwizError, Done, Host, Listener};
use nsh_core::{
    ApproxTextEngine, EventKind, EventMask, EventPayload, MountBatch, MountItem, NodeId,
};
use nsh_engine::{App, Engine, EngineConfig, MountSink};
use serde_json::{Map, Value, json};

use super::screens;

struct FixtureHost {
    props: Value,
    answers: Mutex<HashMap<String, VecDeque<Value>>>,
    sent: Mutex<Vec<(String, Value)>>,
    served: Mutex<Vec<Value>>,
}

impl FixtureHost {
    fn new(props: Value, answers: &Value) -> Self {
        Self {
            props,
            answers: Mutex::new(
                answers
                    .as_object()
                    .unwrap()
                    .iter()
                    .map(|(tool, answers)| {
                        (
                            tool.clone(),
                            answers.as_array().unwrap().iter().cloned().collect(),
                        )
                    })
                    .collect(),
            ),
            sent: Mutex::new(Vec::new()),
            served: Mutex::new(Vec::new()),
        }
    }

    fn served(&self) -> Vec<Value> {
        self.served.lock().unwrap().clone()
    }

    fn sent(&self) -> Vec<(String, Value)> {
        self.sent.lock().unwrap().clone()
    }
}

impl Host for FixtureHost {
    fn params(&self) -> Map<String, Value> {
        json!({
            "props": self.props,
            "app": {"plugin":"app.buwiz.books","app":"ledger"},
            "granted": ["buwiz.call"],
            "viewport": {"width":720,"height":900,"placement":"full"}
        })
        .as_object()
        .unwrap()
        .clone()
    }

    fn post(&self, method: &str, data: Value) {
        self.sent.lock().unwrap().push((method.to_owned(), data));
    }

    fn call(&self, method: &str, data: Value, done: Done) {
        self.sent
            .lock()
            .unwrap()
            .push((method.to_owned(), data.clone()));
        let result = if method == "buwiz.call" {
            let tool = data["tool"].as_str().unwrap_or_default().to_owned();
            let answer = self
                .answers
                .lock()
                .unwrap()
                .get_mut(&tool)
                .and_then(|answers| {
                    if answers.len() > 1 {
                        answers.pop_front()
                    } else {
                        answers.front().cloned()
                    }
                })
                .unwrap_or_else(|| json!({"error":format!("missing fixture answer: {tool}")}));
            self.served
                .lock()
                .unwrap()
                .push(json!({"tool":tool,"answer":answer}));
            match answer.get("error").and_then(Value::as_str) {
                Some(message) => Err(BuwizError::new("fixture_error", false, message)),
                None => Ok(answer["data"].clone()),
            }
        } else {
            Ok(Value::Null)
        };
        done(result);
    }

    fn listen(&self, _: Listener) {}

    fn fail(&self, message: &str) {
        panic!("ledger screen failed: {message}");
    }
}

struct Sink(mpsc::Sender<MountBatch>);

impl MountSink for Sink {
    fn submit(&self, batch: MountBatch) {
        let _ = self.0.send(batch);
    }
}

#[derive(Default)]
struct ScreenState {
    parents: HashMap<NodeId, NodeId>,
    events: HashMap<NodeId, EventMask>,
    labels: HashMap<NodeId, String>,
    label_order: Vec<NodeId>,
    inputs: HashMap<NodeId, String>,
    shown: Vec<String>,
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/screens/ledger/ledger-cases.json")
}

fn fixture_answers(defaults: &Value, case: &Value) -> Value {
    let mut merged = defaults.clone();
    let output = merged.as_object_mut().unwrap();
    if let Some(overrides) = case.get("answers").and_then(Value::as_object) {
        for (tool, answers) in overrides {
            output.insert(tool.clone(), answers.clone());
        }
    }
    Value::Object(output.clone())
}

fn apply_batch(state: &mut ScreenState, batch: MountBatch) {
    for item in batch.items {
        match item {
            MountItem::Insert { parent, child, .. } => {
                state.parents.insert(child, parent);
            }
            MountItem::Events { id, events } => {
                state.events.insert(id, events);
            }
            MountItem::Remove { id } | MountItem::Delete { id } => {
                state.parents.remove(&id);
                state.events.remove(&id);
                state.labels.remove(&id);
                state.inputs.remove(&id);
            }
            MountItem::Text { id, text, .. } => {
                let value = text.text.to_string();
                if !state.labels.contains_key(&id) {
                    state.label_order.push(id);
                }
                state.labels.insert(id, value.clone());
                state.shown.push(value);
            }
            MountItem::Input { id, config } => {
                state.inputs.insert(id, config.placeholder.clone());
            }
            _ => {}
        }
    }
}

fn press_id(label: &str, occurrence: usize, state: &ScreenState, contains: bool) -> Option<NodeId> {
    let normalized = label.trim_end_matches('⌄');
    let text_ids = |matches: &dyn Fn(&str) -> bool| {
        state
            .label_order
            .iter()
            .filter(|id| state.labels.get(id).is_some_and(|text| matches(text)))
            .copied()
            .collect::<Vec<_>>()
    };
    let mut matches = if contains {
        text_ids(&|text| text.contains(normalized))
    } else {
        text_ids(&|text| text.eq_ignore_ascii_case(normalized))
    };
    if matches.is_empty() && !contains {
        matches = text_ids(&|text| {
            !text.is_empty() && (normalized.starts_with(text) || text.starts_with(normalized))
        });
    }

    let mut candidates = Vec::new();
    for id in matches {
        let mut current = id;
        loop {
            if state
                .events
                .get(&current)
                .is_some_and(|mask| mask.contains(EventKind::Press))
            {
                if !candidates.contains(&current) {
                    candidates.push(current);
                }
                break;
            }
            let Some(parent) = state.parents.get(&current) else {
                break;
            };
            current = *parent;
        }
    }
    candidates.get(occurrence).copied()
}

fn expected_call_count(case: &Value) -> usize {
    case["transcript"]["calls"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|call| call["method"] != "fit")
        .count()
}

fn drive_case(defaults: &Value, case: &Value) {
    let answers = fixture_answers(&defaults["answers"], case);
    let props = case.get("props").unwrap_or(&defaults["props"]).clone();
    let host = Arc::new(FixtureHost::new(props.clone(), &answers));
    let screen = screens()
        .iter()
        .find(|screen| screen.id == "books.ledger")
        .expect("ledger screen is registered");
    let (sender, receiver) = mpsc::channel();
    let engine = Engine::start(
        EngineConfig::new(App::Rust((screen.build)(Bridge::new(host.clone())))),
        || ApproxTextEngine,
        Sink(sender),
    )
    .expect("ledger engine starts");
    let control = engine.control();
    let deadline = Instant::now() + Duration::from_secs(8);
    let mut state = ScreenState::default();
    let steps = case["steps"].as_array().unwrap();

    for step in steps {
        let occurrence = step["occurrence"].as_u64().unwrap_or(0) as usize;
        let contains = step.get("contains").and_then(Value::as_str);
        let label = contains.or_else(|| step.get("press").and_then(Value::as_str));
        if let Some(label) = label {
            let mut found = None;
            while Instant::now() < deadline && found.is_none() {
                while let Ok(batch) = receiver.try_recv() {
                    apply_batch(&mut state, batch);
                }
                found = press_id(label, occurrence, &state, contains.is_some());
                if found.is_none() {
                    thread::sleep(Duration::from_millis(5));
                }
            }
            let id = found.unwrap_or_else(|| {
                panic!(
                    "{} cannot press {label:?}; text {:?}",
                    case["name"], state.shown
                )
            });
            control.dispatch_event(id, EventKind::Press, EventPayload::None);
        }
        if let Some(placeholder) = step.get("type").and_then(Value::as_str) {
            let value = step["value"].as_str().unwrap().to_owned();
            let mut id = None;
            let input_deadline = Instant::now() + Duration::from_secs(2);
            while Instant::now() < input_deadline && id.is_none() {
                while let Ok(batch) = receiver.try_recv() {
                    apply_batch(&mut state, batch);
                }
                id = state
                    .inputs
                    .iter()
                    .find(|(_, actual)| actual == &placeholder)
                    .map(|(id, _)| *id);
                if id.is_none() {
                    thread::sleep(Duration::from_millis(5));
                }
            }
            control.dispatch_event(
                id.unwrap_or_else(|| panic!("{} missing input {placeholder:?}", case["name"])),
                EventKind::ChangeText,
                EventPayload::Text {
                    text: value,
                    count: 1,
                },
            );
        }

        let settle_deadline = Instant::now() + Duration::from_millis(250);
        while Instant::now() < settle_deadline {
            match receiver.recv_timeout(Duration::from_millis(10)) {
                Ok(batch) => apply_batch(&mut state, batch),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        let load_deadline = Instant::now() + Duration::from_secs(2);
        while state.labels.values().any(|text| text == "Loading…") && Instant::now() < load_deadline
        {
            match receiver.recv_timeout(Duration::from_millis(10)) {
                Ok(batch) => apply_batch(&mut state, batch),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        assert!(
            !state.labels.values().any(|text| text == "Loading…"),
            "{} remained in a loading state; calls {:?}",
            case["name"],
            normalize_calls(&host.sent())
        );
        if let Some(expected) = step.get("expect").and_then(Value::as_str) {
            assert!(
                state.shown.iter().any(|text| text.contains(expected)),
                "{} missing validation {expected:?}: {:?}",
                case["name"],
                state.shown
            );
        }
    }

    let expected_count = expected_call_count(case);
    while Instant::now() < deadline
        && host
            .sent()
            .iter()
            .filter(|(method, _)| method == "buwiz.call")
            .count()
            < expected_count
    {
        match receiver.recv_timeout(Duration::from_millis(10)) {
            Ok(batch) => apply_batch(&mut state, batch),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    let expected_texts = case["transcript"]["text"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|text| text.as_str().map(str::to_owned))
        .collect::<Vec<_>>();
    while Instant::now() < deadline
        && !expected_texts.iter().all(|expected| {
            state
                .shown
                .iter()
                .any(|actual| actual.eq_ignore_ascii_case(expected))
        })
    {
        match receiver.recv_timeout(Duration::from_millis(10)) {
            Ok(batch) => apply_batch(&mut state, batch),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    while let Ok(batch) = receiver.try_recv() {
        apply_batch(&mut state, batch);
    }

    let actual_calls = normalize_calls(&host.sent());
    let expected_calls = case["transcript"]["calls"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|call| call["method"] != "fit")
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(
        actual_calls, expected_calls,
        "{} call transcript differs; shown {:?}",
        case["name"], state.shown
    );
    for expected in case["transcript"]["text"].as_array().unwrap() {
        let expected = expected.as_str().unwrap();
        assert!(
            state
                .shown
                .iter()
                .any(|actual| actual.eq_ignore_ascii_case(expected)),
            "{} missing text {expected:?}; shown {:?}",
            case["name"],
            state.shown
        );
    }
    let expected_served = expected_served(&expected_calls, &answers);
    assert_eq!(
        host.served(),
        expected_served,
        "{} answers were not served in transcript order",
        case["name"]
    );
    drop(engine);
}

fn normalize_calls(sent: &[(String, Value)]) -> Vec<Value> {
    let mut request_counter = 0;
    sent.iter()
        .filter_map(|(method, data)| {
            if method != "buwiz.call" {
                return None;
            }
            let mut args = data["args"].clone();
            if let Some(id) = args.get("client_request_id").and_then(Value::as_str) {
                let parts = id.split('-').collect::<Vec<_>>();
                assert_eq!(parts.len(), 3, "invalid client_request_id");
                assert_eq!(parts[0], "bk");
                assert_eq!(parts[1].len(), 8);
                assert!(parts[1].bytes().all(|byte| byte.is_ascii_digit()));
                request_counter += 1;
                assert_eq!(parts[2].parse::<usize>().unwrap(), request_counter);
                args["client_request_id"] = json!(format!("bk-<open>-{request_counter}"));
            }
            Some(json!({
                "method":"call",
                "tool":data["tool"],
                "args":args
            }))
        })
        .collect()
}

fn expected_served(calls: &[Value], answers: &Value) -> Vec<Value> {
    let mut indexes = HashMap::<String, usize>::new();
    calls
        .iter()
        .map(|call| {
            let tool = call["tool"].as_str().unwrap().to_owned();
            let values = answers[&tool].as_array().unwrap();
            let index = indexes.entry(tool.clone()).or_default();
            let answer = values[(*index).min(values.len() - 1)].clone();
            *index += 1;
            json!({"tool":tool,"answer":answer})
        })
        .collect()
}

fn first_row_ms(defaults: &Value, case: &Value) -> f64 {
    let answers = fixture_answers(&defaults["answers"], case);
    let host = Arc::new(FixtureHost::new(
        case.get("props").unwrap_or(&defaults["props"]).clone(),
        &answers,
    ));
    let screen = screens()
        .iter()
        .find(|screen| screen.id == "books.ledger")
        .expect("ledger screen is registered");
    let (sender, receiver) = mpsc::channel();
    let started = Instant::now();
    let engine = Engine::start(
        EngineConfig::new(App::Rust((screen.build)(Bridge::new(host)))),
        || ApproxTextEngine,
        Sink(sender),
    )
    .expect("ledger engine starts");
    let target = defaults["timing_text"].as_str().unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut state = ScreenState::default();
    while Instant::now() < deadline {
        while let Ok(batch) = receiver.try_recv() {
            apply_batch(&mut state, batch);
        }
        if state
            .shown
            .iter()
            .any(|text| text.eq_ignore_ascii_case(target))
        {
            drop(engine);
            return started.elapsed().as_secs_f64() * 1000.0;
        }
        thread::sleep(Duration::from_millis(1));
    }
    panic!("{} did not render the first ledger row", case["name"]);
}

#[test]
fn ledger_fixtures_match_native_transcripts() {
    let fixture: Value = serde_json::from_slice(&fs::read(fixture_path()).unwrap()).unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    let expected_names = [
        "default-list",
        "list-controls",
        "row-post",
        "row-void",
        "row-discard",
        "row-duplicate",
        "row-recategorize",
        "new-no-amount",
        "new-no-account",
        "new-no-category",
        "new-save-draft",
        "new-save-post",
        "transfer-validation",
        "journal-validation",
        "review-keep-both",
        "review-open-duplicate",
        "row-amend",
        "amend-links",
        "running-balance",
        "list-retry",
    ];
    assert_eq!(
        cases
            .iter()
            .map(|case| case["name"].as_str().unwrap())
            .collect::<Vec<_>>(),
        expected_names
    );
    for case in cases {
        drive_case(&fixture["defaults"], case);
    }
}

#[test]
#[ignore = "native release timing"]
fn native_ledger_timing_50() {
    let fixture: Value = serde_json::from_slice(&fs::read(fixture_path()).unwrap()).unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        if case["name"] == "list-retry" {
            continue;
        }
        let mut values = (0..50)
            .map(|_| first_row_ms(&fixture["defaults"], case))
            .collect::<Vec<_>>();
        values.sort_by(f64::total_cmp);
        println!(
            "TIMING native ledger {}: median {:.2} ms, p95 {:.2} ms (50 runs)",
            case["name"], values[24], values[47]
        );
    }
}

#[test]
fn ledger_screen_tool_set_covers_fixture_calls() {
    let fixture: Value = serde_json::from_slice(&fs::read(fixture_path()).unwrap()).unwrap();
    let mut called = Vec::new();
    for case in fixture["cases"].as_array().unwrap() {
        let answers = fixture_answers(&fixture["defaults"]["answers"], case);
        for call in case["transcript"]["calls"].as_array().unwrap() {
            let tool = call["tool"].as_str().unwrap();
            if !called.contains(&tool) {
                called.push(tool);
            }
            assert!(
                answers.get(tool).is_some(),
                "{} has no answer for {tool}",
                case["name"]
            );
        }
    }
    called.sort();
    assert_eq!(
        called,
        [
            "books_app_discard_entry",
            "books_app_post_entry",
            "books_app_recategorize",
            "books_app_void_entry",
            "books_entry_draft",
            "books_entry_get",
            "books_entry_options",
            "books_ledger_list",
        ]
    );
}
