use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use buwiz_app::Bridge;
use nsh_core::element::{
    Element, Owner, ParentElement, Root, Scrollable, dynamic, scroll, set_timeout, signal, view,
};
use serde_json::{Value, json};

use crate::kit;

#[derive(Clone, PartialEq)]
enum ReadState {
    Loading,
    Data(Value),
    Error(String),
}

pub fn build(bridge: Bridge) -> Root {
    buwiz_app::root(bridge, |b| {
        let screen_owner = Owner::current();
        let props = b.props();
        let card = props
            .get("card")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let read_args = match card.as_str() {
            "tidy" => Some(json!({"kind":"tidy","any_day":true})),
            "month_end" => Some(json!({"kind":"month_end"})),
            "handoff" => Some(json!({"kind":"bir_pair"})),
            _ => None,
        };
        let state = signal(ReadState::Loading);
        let fit_sent = signal(0_i32);
        let toast = signal(String::new());
        let toast_epoch = signal(0_u64);
        let ready_sent = Arc::new(AtomicBool::new(false));
        if let Some(args) = read_args {
            read(b.clone(), state, args, ready_sent.clone());
        } else {
            b.ready();
            ready_sent.store(true, Ordering::SeqCst);
        }
        let render_bridge = b.clone();
        let content = dynamic(move || {
            let notice = toast.get();
            match state.get() {
                ReadState::Loading => {
                    kit::wait(None, None::<fn()>, render_bridge.clone(), fit_sent)
                }
                ReadState::Error(message) => {
                    let retry_state = state;
                    let retry_bridge = render_bridge.clone();
                    let retry_ready = ready_sent.clone();
                    let args = args_for(&card);
                    let retry = move || {
                        retry_state.set(ReadState::Loading);
                        screen_owner.run(|| {
                            read(
                                retry_bridge.clone(),
                                retry_state,
                                args.clone(),
                                retry_ready.clone(),
                            )
                        });
                    };
                    kit::wait(Some(message), Some(retry), render_bridge.clone(), fit_sent)
                }
                ReadState::Data(data) => match card.as_str() {
                    "tidy" => tidy(
                        data,
                        render_bridge.clone(),
                        fit_sent,
                        notice,
                        toast,
                        toast_epoch,
                    ),
                    "month_end" => month_end(
                        data,
                        render_bridge.clone(),
                        fit_sent,
                        notice,
                        toast,
                        toast_epoch,
                    ),
                    "handoff" => handoff(
                        data,
                        render_bridge.clone(),
                        fit_sent,
                        notice,
                        toast,
                        toast_epoch,
                    ),
                    _ => view(),
                },
            }
        });
        scroll()
            .scrollable(false)
            .bounces(false)
            .app_class(Some(format!("flex-1 {}", kit::K_SURFACE).into()))
            .child(content)
    })
}

fn args_for(card: &str) -> Value {
    match card {
        "tidy" => json!({"kind":"tidy","any_day":true}),
        "month_end" => json!({"kind":"month_end"}),
        _ => json!({"kind":"bir_pair"}),
    }
}

fn read(
    bridge: Bridge,
    state: nsh_core::element::Signal<ReadState>,
    args: Value,
    ready_sent: Arc<AtomicBool>,
) {
    let read_bridge = bridge.clone();
    bridge.spawn(async move {
        let result = read_bridge.call("books_nudge", args).await;
        state.set(match result {
            Ok(value) => ReadState::Data(value),
            Err(error) => ReadState::Error(error.message),
        });
        if !ready_sent.swap(true, Ordering::SeqCst) {
            read_bridge.ready();
        }
    });
}

fn open_page(
    bridge: Bridge,
    page: &'static str,
    toast: nsh_core::element::Signal<String>,
    epoch: nsh_core::element::Signal<u64>,
) -> impl Fn() + 'static {
    move || {
        let spawn_bridge = bridge.clone();
        bridge.spawn(async move {
            if let Err(error) = spawn_bridge.open("books", json!({"page":page})).await {
                toast.set(error.message);
                let next = epoch.get() + 1;
                epoch.set(next);
                let clear_toast = toast;
                set_timeout(Duration::from_millis(2600), move || {
                    if epoch.get() == next {
                        clear_toast.set(String::new());
                    }
                });
            }
        });
    }
}

fn tidy(
    data: Value,
    bridge: Bridge,
    fit: nsh_core::element::Signal<i32>,
    notice: String,
    toast: nsh_core::element::Signal<String>,
    epoch: nsh_core::element::Signal<u64>,
) -> nsh_core::element::View {
    let lines = data
        .pointer("/counts/lines")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let drafts = data
        .pointer("/counts/drafts")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let posted = data
        .pointer("/counts/posted")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let line_word = if lines == 1 { "line" } else { "lines" };
    let status = format!("{lines} new bank {line_word} · {drafts} drafted · {posted} posted");
    let action = if lines > 0 {
        format!("Review {lines}")
    } else {
        "Open Ledger".to_owned()
    };
    let primary = lines > 0;
    let content = kit::content()
        .child(kit::meta("This week", kit::pill("Mondays", "mute")))
        .child(kit::say(status));
    let foot = kit::foot(
        "The routine drafts only; you post",
        notice,
        kit::act(
            action,
            primary,
            open_page(bridge.clone(), "ledger", toast, epoch),
        ),
    );
    kit::body(content, Some(foot), false, bridge, fit)
}

fn month_end(
    data: Value,
    bridge: Bridge,
    fit: nsh_core::element::Signal<i32>,
    notice: String,
    toast: nsh_core::element::Signal<String>,
    epoch: nsh_core::element::Signal<u64>,
) -> nsh_core::element::View {
    let month = data
        .get("month_label")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if data.get("closed").and_then(Value::as_bool).unwrap_or(false) {
        return kit::settled(format!("{month} is closed"), fit, bridge);
    }
    let left = data.get("left").and_then(Value::as_u64).unwrap_or(0);
    let step_word = if left == 1 { "step" } else { "steps" };
    let content = kit::content()
        .child(kit::meta(month, kit::pill("Open", "draft")))
        .child(kit::say(format!(
            "{month} is still open: {left} {step_word} left before you can close it."
        )));
    let foot = kit::foot(
        "",
        notice,
        kit::act(
            "Start closing",
            true,
            open_page(bridge.clone(), "settings", toast, epoch),
        ),
    );
    kit::body(content, Some(foot), false, bridge, fit)
}

fn handoff(
    data: Value,
    bridge: Bridge,
    fit: nsh_core::element::Signal<i32>,
    notice: String,
    toast: nsh_core::element::Signal<String>,
    epoch: nsh_core::element::Signal<u64>,
) -> nsh_core::element::View {
    let count = data.get("count").and_then(Value::as_u64).unwrap_or(0);
    if count == 0 {
        return kit::settled("Every hand-off still matches the books", fit, bridge);
    }
    let form = data
        .pointer("/forms/0/title")
        .and_then(Value::as_str)
        .unwrap_or("a hand-off");
    let form = form.replace("BIR hand-off · ", "");
    let why = data
        .pointer("/forms/0/why")
        .and_then(Value::as_str)
        .unwrap_or("A line changed");
    let form_word = if count == 1 { "form" } else { "forms" };
    let content = kit::content()
        .child(kit::meta(form, kit::pill("Out of date", "rev")))
        .child(kit::say(format!(
            "{why} after it was handed to BIR. {count} {form_word} to refresh."
        )));
    let foot = kit::foot(
        "",
        notice,
        kit::act(
            "Refresh for BIR",
            true,
            open_page(bridge.clone(), "forms", toast, epoch),
        ),
    );
    kit::body(content, Some(foot), false, bridge, fit)
}
