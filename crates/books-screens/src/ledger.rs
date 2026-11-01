use std::{
    future::Future,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use buwiz_app::Bridge;
use nsh_core::element::{
    Element, Interactive, Owner, ParentElement, Root, Styled, dynamic, input, scroll, set_timeout,
    signal, text, view,
};
use serde_json::{Value, json};

use crate::kit;

const FILTERS: [(&str, &str); 4] = [
    ("all", "All"),
    ("review", "Needs review"),
    ("draft", "Drafts"),
    ("posted", "Posted"),
];

#[derive(Clone, PartialEq)]
enum ReadState {
    Loading,
    Data(Value),
    Error(String),
}

#[derive(Clone)]
struct RequestIds {
    tag: String,
    counter: Arc<AtomicU64>,
}

impl RequestIds {
    fn new() -> Self {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .to_string();
        let tag = millis
            .chars()
            .rev()
            .take(8)
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        Self {
            tag,
            counter: Arc::new(AtomicU64::new(0)),
        }
    }

    fn next(&self) -> String {
        format!(
            "bk-{}-{}",
            self.tag,
            self.counter.fetch_add(1, Ordering::SeqCst) + 1
        )
    }
}

#[derive(Clone, Copy)]
struct SheetFields {
    amount: nsh_core::element::Signal<String>,
    date: nsh_core::element::Signal<String>,
    money: nsh_core::element::Signal<String>,
    to_account: nsh_core::element::Signal<String>,
    category: nsh_core::element::Signal<String>,
    payee: nsh_core::element::Signal<String>,
    memo: nsh_core::element::Signal<String>,
    reference: nsh_core::element::Signal<String>,
    lines: nsh_core::element::Signal<Vec<JournalLine>>,
    error: nsh_core::element::Signal<String>,
    busy: nsh_core::element::Signal<bool>,
    money_open: nsh_core::element::Signal<bool>,
    to_account_open: nsh_core::element::Signal<bool>,
    category_open: nsh_core::element::Signal<bool>,
    line_account_open: nsh_core::element::Signal<bool>,
}

/// The drawer's amend form: open for the entry `open_for` (empty when
/// closed, so another entry never shows it), its reason, optional date and
/// the last refusal shown under it.
#[derive(Clone, Copy)]
struct AmendFields {
    open_for: nsh_core::element::Signal<String>,
    reason: nsh_core::element::Signal<String>,
    date: nsh_core::element::Signal<String>,
    error: nsh_core::element::Signal<String>,
}

impl AmendFields {
    fn new() -> Self {
        Self {
            open_for: signal(String::new()),
            reason: signal(String::new()),
            date: signal(String::new()),
            error: signal(String::new()),
        }
    }

    fn open(self, ticket: String) {
        self.reason.set(String::new());
        self.date.set(String::new());
        self.error.set(String::new());
        self.open_for.set(ticket);
    }
}

#[derive(Clone, PartialEq)]
struct JournalLine {
    account: String,
    debit: String,
    credit: String,
}

impl SheetFields {
    fn new() -> Self {
        Self {
            amount: signal(String::new()),
            date: signal(String::new()),
            money: signal(String::new()),
            to_account: signal(String::new()),
            category: signal(String::new()),
            payee: signal(String::new()),
            memo: signal(String::new()),
            reference: signal(String::new()),
            lines: signal(vec![
                JournalLine {
                    account: String::new(),
                    debit: String::new(),
                    credit: String::new(),
                },
                JournalLine {
                    account: String::new(),
                    debit: String::new(),
                    credit: String::new(),
                },
            ]),
            error: signal(String::new()),
            busy: signal(false),
            money_open: signal(false),
            to_account_open: signal(false),
            category_open: signal(false),
            line_account_open: signal(false),
        }
    }

    fn reset(self) {
        self.amount.set(String::new());
        self.date.set(String::new());
        self.money.set(String::new());
        self.to_account.set(String::new());
        self.category.set(String::new());
        self.payee.set(String::new());
        self.memo.set(String::new());
        self.reference.set(String::new());
        self.lines.set(vec![
            JournalLine {
                account: String::new(),
                debit: String::new(),
                credit: String::new(),
            },
            JournalLine {
                account: String::new(),
                debit: String::new(),
                credit: String::new(),
            },
        ]);
        self.error.set(String::new());
        self.busy.set(false);
        self.money_open.set(false);
        self.to_account_open.set(false);
        self.category_open.set(false);
        self.line_account_open.set(false);
    }
}

pub fn build(bridge: Bridge) -> Root {
    buwiz_app::root(bridge, |b| {
        let screen_owner = Owner::current();
        let props = b.props();
        let account = signal(
            props
                .get("account")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
        );
        let month = signal(
            props
                .get("month")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
        );
        let filter = signal(
            props
                .get("filter")
                .and_then(Value::as_str)
                .unwrap_or("all")
                .to_owned(),
        );
        let list = signal(ReadState::Loading);
        let entry = signal(ReadState::Loading);
        let options = signal(ReadState::Loading);
        let drawer_entry = signal(String::new());
        let sheet_mode = signal(String::new());
        let fields = SheetFields::new();
        let toast = signal(String::new());
        let toast_epoch = signal(0_u64);
        let version = signal(0_u64);
        let drawer_busy = signal(false);
        let amend = AmendFields::new();
        let ids = RequestIds::new();

        load_list(
            screen_owner,
            b.clone(),
            list,
            account.get(),
            month.get(),
            filter.get(),
        );

        let render_bridge = b.clone();
        let content = dynamic(move || {
            render(
                screen_owner,
                render_bridge.clone(),
                list,
                entry,
                options,
                account,
                month,
                filter,
                drawer_entry,
                sheet_mode,
                fields,
                toast,
                toast_epoch,
                version,
                drawer_busy,
                amend,
                ids.clone(),
            )
        });

        view()
            .app_class(Some(format!("flex-1 {}", kit::K_SURFACE).into()))
            .child(content)
    })
}

fn load_list(
    screen_owner: Owner,
    bridge: Bridge,
    state: nsh_core::element::Signal<ReadState>,
    account: String,
    month: String,
    filter: String,
) {
    let mut args = serde_json::Map::new();
    args.insert("filter".into(), json!(filter));
    if !account.is_empty() {
        args.insert("account".into(), json!(account));
    }
    if !month.is_empty() {
        args.insert("month".into(), json!(month));
    }
    state.set(ReadState::Loading);
    let call_bridge = bridge.clone();
    spawn_task(screen_owner, bridge, async move {
        state.set(
            match call_bridge
                .call("books_ledger_list", Value::Object(args))
                .await
            {
                Ok(value) => ReadState::Data(value),
                Err(error) => ReadState::Error(error.message),
            },
        );
    });
}

fn spawn_task<T: 'static>(
    screen_owner: Owner,
    bridge: Bridge,
    future: impl Future<Output = T> + 'static,
) {
    screen_owner.run(move || {
        bridge.spawn(future);
    });
}

fn load_entry(
    screen_owner: Owner,
    bridge: Bridge,
    state: nsh_core::element::Signal<ReadState>,
    ticket: String,
) {
    state.set(ReadState::Loading);
    let call_bridge = bridge.clone();
    spawn_task(screen_owner, bridge, async move {
        state.set(
            match call_bridge
                .call("books_entry_get", json!({"entry":ticket}))
                .await
            {
                Ok(value) => ReadState::Data(value),
                Err(error) => ReadState::Error(error.message),
            },
        );
    });
}

fn load_options(screen_owner: Owner, bridge: Bridge, state: nsh_core::element::Signal<ReadState>) {
    state.set(ReadState::Loading);
    let call_bridge = bridge.clone();
    spawn_task(screen_owner, bridge, async move {
        state.set(
            match call_bridge.call("books_entry_options", json!({})).await {
                Ok(value) => ReadState::Data(value),
                Err(error) => ReadState::Error(error.message),
            },
        );
    });
}

#[allow(clippy::too_many_arguments)]
fn render(
    screen_owner: Owner,
    bridge: Bridge,
    list: nsh_core::element::Signal<ReadState>,
    entry: nsh_core::element::Signal<ReadState>,
    options: nsh_core::element::Signal<ReadState>,
    account: nsh_core::element::Signal<String>,
    month: nsh_core::element::Signal<String>,
    filter: nsh_core::element::Signal<String>,
    drawer_entry: nsh_core::element::Signal<String>,
    sheet_mode: nsh_core::element::Signal<String>,
    fields: SheetFields,
    toast: nsh_core::element::Signal<String>,
    toast_epoch: nsh_core::element::Signal<u64>,
    version: nsh_core::element::Signal<u64>,
    busy: nsh_core::element::Signal<bool>,
    amend: AmendFields,
    ids: RequestIds,
) -> nsh_core::element::View {
    let data = match list.get() {
        ReadState::Data(data) => Some(data),
        ReadState::Loading | ReadState::Error(_) => None,
    };
    let mut page = view().app_class(Some("flex-1 bg-[#f6f6f4] dark:bg-[#151515]".into()));
    let mut body = view().app_class(Some("gap-3.5 p-4".into())).child(head(
        "Ledger",
        data.as_ref()
            .map(|value| text_value(value.pointer("/month/label"), "").to_owned())
            .unwrap_or_default(),
        button("+ New", "go", {
            let bridge = bridge.clone();
            move || {
                fields.reset();
                sheet_mode.set("money_out".into());
                load_options(screen_owner, bridge.clone(), options);
            }
        }),
    ));

    match list.get() {
        ReadState::Loading => {
            body = body.child(
                text("Loading…").app_class(Some(format!("text-[13px] {}", kit::C_DIM).into())),
            );
        }
        ReadState::Error(message) => {
            let retry_bridge = bridge.clone();
            let retry_list = list;
            let retry_account = account.get();
            let retry_month = month.get();
            let retry_filter = filter.get();
            body = body.child(
                view()
                    .app_class(Some("gap-2".into()))
                    .child(
                        text(message).app_class(Some(format!("text-[13px] {}", kit::C_RED).into())),
                    )
                    .child(button("Try again", "plain", move || {
                        load_list(
                            screen_owner,
                            retry_bridge.clone(),
                            retry_list,
                            retry_account.clone(),
                            retry_month.clone(),
                            retry_filter.clone(),
                        );
                    })),
            );
        }
        ReadState::Data(data) => {
            body = body.child(ledger_body(
                screen_owner,
                bridge.clone(),
                list,
                entry,
                data,
                account,
                month,
                filter,
                drawer_entry,
            ));
        }
    }
    page = page.child(scroll().app_class(Some("flex-1".into())).child(body));

    if !drawer_entry.get().is_empty() {
        page = page.child(drawer(
            screen_owner,
            bridge.clone(),
            list,
            entry,
            account,
            month,
            filter,
            drawer_entry,
            toast,
            toast_epoch,
            version,
            busy,
            amend,
            ids.clone(),
        ));
    }
    if !sheet_mode.get().is_empty() {
        page = page.child(sheet(
            bridge,
            list,
            entry,
            options,
            account,
            month,
            filter,
            drawer_entry,
            sheet_mode,
            fields,
            toast,
            toast_epoch,
            version,
            ids,
            screen_owner,
        ));
    }
    let notice = toast.get();
    if !notice.is_empty() {
        page = page.child(
            view()
                .app_class(Some("absolute bottom-4 left-0 right-0 items-center".into()))
                .child(
                    view()
                        .app_class(Some(
                            "rounded-[10px] px-3.5 py-2 bg-[#1c1c1c] dark:bg-[#f0f0f0]".into(),
                        ))
                        .child(text(notice).app_class(Some(
                            "text-[12.5px] font-semibold text-white dark:text-[#111111]".into(),
                        ))),
                ),
        );
    }
    let _ = version.get();
    page
}

#[allow(clippy::too_many_arguments)]
fn ledger_body(
    screen_owner: Owner,
    bridge: Bridge,
    list: nsh_core::element::Signal<ReadState>,
    entry: nsh_core::element::Signal<ReadState>,
    data: Value,
    account: nsh_core::element::Signal<String>,
    month: nsh_core::element::Signal<String>,
    filter: nsh_core::element::Signal<String>,
    drawer_entry: nsh_core::element::Signal<String>,
) -> nsh_core::element::View {
    let mut accounts = view().app_class(Some("w-[196px] p-1.5 gap-px rounded-xl border bg-white dark:bg-[#1d1d1d] border-black/10 dark:border-[#2b2b2b]".into()));
    accounts = accounts.child(account_row("All accounts", "", "", account, {
        let bridge = bridge.clone();
        let month = month.get();
        let filter = filter.get();
        move || {
            if account.get().is_empty() {
                return;
            }
            account.set(String::new());
            load_list(
                screen_owner,
                bridge.clone(),
                list,
                String::new(),
                month.clone(),
                filter.clone(),
            );
        }
    }));
    if let Some(groups) = data.get("accounts").and_then(Value::as_array) {
        for group in groups {
            accounts = accounts.child(
                view().app_class(Some("gap-px".into())).child(
                    text(text_value(group.get("group"), "").to_owned()).app_class(Some(
                        format!(
                            "text-[10.5px] uppercase tracking-wider px-2 pt-2 pb-0.5 {}",
                            kit::C_DIM
                        )
                        .into(),
                    )),
                ),
            );
            if let Some(rows) = group.get("rows").and_then(Value::as_array) {
                for row in rows {
                    let ticket = text_value(row.get("ticket"), "").to_owned();
                    let name = text_value(row.get("name"), "").to_owned();
                    let number = text_value(row.get("number"), "").to_owned();
                    accounts = accounts.child(account_row(&name, &number, &ticket, account, {
                        let bridge = bridge.clone();
                        let month = month.get();
                        let filter = filter.get();
                        let ticket = ticket.clone();
                        move || {
                            if account.get() == ticket {
                                return;
                            }
                            account.set(ticket.clone());
                            load_list(
                                screen_owner,
                                bridge.clone(),
                                list,
                                ticket.clone(),
                                month.clone(),
                                filter.clone(),
                            );
                        }
                    }));
                }
            }
        }
    }

    let account_name = text_value(data.pointer("/account/name"), "").to_owned();
    let account_number = text_value(data.pointer("/account/number"), "").to_owned();
    let balance_label = text_value(data.pointer("/account/balance_label"), "Balance").to_owned();
    let balance = text_value(data.pointer("/account/balance_shown"), "").to_owned();
    let month_range = text_value(data.pointer("/month/range"), "").to_owned();
    let prev_month = text_value(data.pointer("/month/prev"), "").to_owned();
    let next_month = text_value(data.pointer("/month/next"), "").to_owned();

    let mut header = view()
        .app_class(Some(
            "flex-row items-center gap-2.5 flex-wrap rounded-t-xl px-3.5 py-3 bg-[#157a48]".into(),
        ))
        .child(
            text(account_name.clone()).app_class(Some("text-[15px] font-bold text-white".into())),
        );
    if !account_number.is_empty() {
        header = header.child(
            view()
                .app_class(Some("rounded-[7px] px-2 py-1 bg-white/15".into()))
                .child(text(account_number).app_class(Some("text-[12px] text-white".into()))),
        );
    }
    header = header.child(
        view()
            .app_class(Some(
                "flex-row items-center rounded-[7px] bg-white/15".into(),
            ))
            .child(month_button(
                screen_owner,
                "‹",
                bridge.clone(),
                list,
                account,
                month,
                filter,
                prev_month,
            ))
            .child(text(month_range).app_class(Some("text-[12px] text-white py-1".into())))
            .child(month_button(
                screen_owner,
                "›",
                bridge.clone(),
                list,
                account,
                month,
                filter,
                next_month,
            )),
    );
    header = header.child(view().app_class(Some("flex-1".into()))).child(
        view()
            .app_class(Some("rounded-[7px] px-2 py-1 bg-white/15".into()))
            .child(
                text(format!("{balance_label} ₱ {balance}"))
                    .app_class(Some("text-[12px] text-white font-mono".into())),
            ),
    );

    let mut chips = view().app_class(Some("flex-row gap-1.5 flex-wrap px-3 py-2.5".into()));
    for (key, label) in FILTERS {
        let key_owned = key.to_owned();
        let selected = filter.get() == key;
        let count = data
            .pointer(&format!("/counts/{key}"))
            .map(value_string)
            .unwrap_or_default();
        chips = chips.child(chip(label, count, selected, {
            let bridge = bridge.clone();
            let account = account.get();
            let month = month.get();
            move || {
                if filter.get() == key_owned {
                    return;
                }
                filter.set(key_owned.clone());
                load_list(
                    screen_owner,
                    bridge.clone(),
                    list,
                    account.clone(),
                    month.clone(),
                    key_owned.clone(),
                );
            }
        }));
    }
    let columns = view()
        .app_class(Some(
            "flex-row gap-3 px-3 py-2 border-y border-black/10 dark:border-[#2b2b2b]".into(),
        ))
        .child(
            text("Date").app_class(Some(
                format!(
                    "w-[52px] text-[10.5px] uppercase tracking-wider {}",
                    kit::C_DIM
                )
                .into(),
            )),
        )
        .child(
            text("What").app_class(Some(
                format!(
                    "flex-1 text-[10.5px] uppercase tracking-wider {}",
                    kit::C_DIM
                )
                .into(),
            )),
        )
        .child(
            text("Category").app_class(Some(
                format!(
                    "w-[170px] text-[10.5px] uppercase tracking-wider {}",
                    kit::C_DIM
                )
                .into(),
            )),
        )
        .child(
            text("Status").app_class(Some(
                format!(
                    "w-[96px] text-[10.5px] uppercase tracking-wider {}",
                    kit::C_DIM
                )
                .into(),
            )),
        )
        .child(
            text("Amount").app_class(Some(
                format!(
                    "w-[104px] text-right text-[10.5px] uppercase tracking-wider {}",
                    kit::C_DIM
                )
                .into(),
            )),
        );
    // one account picked: books_ledger_list carries the balance after each
    // posted or voided entry, the same walk as account activity
    let running = data.get("running_balance").and_then(Value::as_bool) == Some(true);
    let columns = if running {
        columns.child(
            text("Balance").app_class(Some(
                format!(
                    "w-[104px] text-right text-[10.5px] uppercase tracking-wider {}",
                    kit::C_DIM
                )
                .into(),
            )),
        )
    } else {
        columns
    };
    let mut rows = view().app_class(Some("rounded-b-xl border border-t-0 bg-white dark:bg-[#1d1d1d] border-black/10 dark:border-[#2b2b2b]".into()))
        .child(chips)
        .child(columns);
    let list_rows = data.get("rows").and_then(Value::as_array);
    if let Some(entries) = list_rows {
        if entries.is_empty() {
            let matching = if filter.get() == "all" {
                ""
            } else {
                "matching "
            };
            rows = rows.child(
                text(format!(
                    "No {matching}entries in {account_name} this month."
                ))
                .app_class(Some(
                    format!("text-[12.5px] px-3 py-4 {}", kit::C_DIM).into(),
                )),
            );
        } else {
            for row in entries {
                rows = rows.child(tx_row(
                    screen_owner,
                    row.clone(),
                    running,
                    bridge.clone(),
                    entry,
                    drawer_entry,
                ));
            }
        }
    }

    view()
        .app_class(Some("flex-row gap-3 items-start".into()))
        .child(accounts)
        .child(
            view()
                .app_class(Some("flex-1".into()))
                .child(header)
                .child(rows),
        )
}

fn account_row(
    name: &str,
    number: &str,
    key: &str,
    selected: nsh_core::element::Signal<String>,
    on_press: impl Fn() + 'static,
) -> nsh_core::element::View {
    let active = selected.get() == key;
    let background = if active {
        "bg-[#f0f0ee] dark:bg-[#262626]"
    } else {
        ""
    };
    view()
        .app_class(Some(
            format!("flex-row justify-between gap-1.5 px-2 py-1.5 rounded-[7px] {background}")
                .into(),
        ))
        .on_press(on_press)
        .child(
            text(name.to_owned())
                .app_class(Some(
                    format!(
                        "flex-1 text-[12.5px] {}",
                        if active {
                            "font-semibold text-[#1c1c1c] dark:text-[#f0f0f0]"
                        } else {
                            "text-[#5c5c5c] dark:text-[#a0a0a0]"
                        }
                    )
                    .into(),
                ))
                .number_of_lines(1),
        )
        .child(
            text(number.to_owned()).app_class(Some(format!("text-[11px] {}", kit::C_DIM).into())),
        )
}

#[allow(clippy::too_many_arguments)]
fn month_button(
    screen_owner: Owner,
    label: &'static str,
    bridge: Bridge,
    list: nsh_core::element::Signal<ReadState>,
    account: nsh_core::element::Signal<String>,
    month: nsh_core::element::Signal<String>,
    filter: nsh_core::element::Signal<String>,
    target: String,
) -> nsh_core::element::View {
    view()
        .app_class(Some("px-2 py-1".into()))
        .on_press(move || {
            if target.is_empty() {
                return;
            }
            month.set(target.clone());
            load_list(
                screen_owner,
                bridge.clone(),
                list,
                account.get(),
                target.clone(),
                filter.get(),
            );
        })
        .child(text(label).app_class(Some("text-[12px] text-white".into())))
}

fn tx_row(
    screen_owner: Owner,
    row: Value,
    running: bool,
    bridge: Bridge,
    entry: nsh_core::element::Signal<ReadState>,
    drawer_entry: nsh_core::element::Signal<String>,
) -> nsh_core::element::View {
    let ticket = text_value(row.get("ticket"), "").to_owned();
    let selected = drawer_entry.get() == ticket;
    let bg = if selected {
        "bg-[#f0f0ee] dark:bg-[#262626]"
    } else {
        ""
    };
    let tone = match text_value(row.get("tone"), "") {
        "pos" => "text-[#157a48] dark:text-[#3fcf89]",
        _ => kit::C_INK,
    };
    let mut what = view()
        .app_class(Some("flex-1 gap-0.5".into()))
        .child(
            text(text_value(row.get("title"), "").to_owned())
                .app_class(Some(
                    format!("text-[13px] font-semibold {}", kit::C_INK).into(),
                ))
                .number_of_lines(1),
        )
        .child(
            text(text_value(row.get("sub"), "").to_owned())
                .app_class(Some(format!("text-[11.5px] {}", kit::C_DIM).into()))
                .number_of_lines(1),
        );
    if let Some(badges) = review_badges(&row) {
        what = what.child(badges);
    }
    let result = view()
        .app_class(Some(format!("flex-row items-center gap-3 px-3 py-2.5 border-b border-black/10 dark:border-[#2b2b2b] {bg}").into()))
        .active(|style| style.opacity(0.75))
        .on_press(move || {
            drawer_entry.set(ticket.clone());
            load_entry(screen_owner, bridge.clone(), entry, ticket.clone());
        })
        .child(text(text_value(row.get("date_short"), "").to_owned()).app_class(Some(format!("w-[52px] text-[12.5px] font-mono {}", kit::C_INK).into())))
        .child(what)
        .child(
            view()
                .app_class(Some("w-[170px] flex-row items-center gap-1.5".into()))
                .child(view().app_class(Some("w-2 h-2 rounded-[2px]".into())))
                .child(text(text_value(row.get("category"), "").to_owned()).app_class(Some(format!("text-[12.5px] {}", kit::C_INK).into())).number_of_lines(1)),
        )
        .child(
            view()
                .app_class(Some("w-[96px]".into()))
                .child(pill(
                    text_value(row.get("status_label"), ""),
                    text_value(row.get("pill"), ""),
                )),
        )
        .child(text(text_value(row.get("amount_shown"), "").to_owned()).app_class(Some(format!("w-[104px] text-right text-[12.5px] font-mono {tone}").into())));
    if running {
        result.child(
            text(text_value(row.get("balance_shown"), "").to_owned()).app_class(Some(
                format!(
                    "w-[104px] text-right text-[12.5px] font-mono {}",
                    kit::C_MUTE
                )
                .into(),
            )),
        )
    } else {
        result
    }
}

/// The row's review pills (M10 ledger-1): proposed by the assistant or a
/// routine, a field below the confidence line, a likely duplicate still to
/// decide. `None` when there is nothing to flag.
fn review_badges(row: &Value) -> Option<nsh_core::element::View> {
    let flags = [
        (
            row.get("proposed_by").is_some_and(|value| !value.is_null()),
            "Assistant",
            "in",
        ),
        (
            row.get("low_confidence").and_then(Value::as_bool) == Some(true),
            "Check fields",
            "rev",
        ),
        (
            row.get("likely_duplicate").and_then(Value::as_bool) == Some(true),
            "Possible duplicate",
            "draft",
        ),
    ];
    if !flags.iter().any(|(on, _, _)| *on) {
        return None;
    }
    let mut badges = view().app_class(Some("flex-row gap-1 flex-wrap".into()));
    for (_, label, tone) in flags.iter().filter(|(on, _, _)| *on) {
        badges = badges.child(pill(label, tone));
    }
    Some(badges)
}

#[allow(clippy::too_many_arguments)]
fn drawer(
    screen_owner: Owner,
    bridge: Bridge,
    list: nsh_core::element::Signal<ReadState>,
    entry: nsh_core::element::Signal<ReadState>,
    account: nsh_core::element::Signal<String>,
    month: nsh_core::element::Signal<String>,
    filter: nsh_core::element::Signal<String>,
    drawer_entry: nsh_core::element::Signal<String>,
    toast: nsh_core::element::Signal<String>,
    toast_epoch: nsh_core::element::Signal<u64>,
    version: nsh_core::element::Signal<u64>,
    busy: nsh_core::element::Signal<bool>,
    amend: AmendFields,
    ids: RequestIds,
) -> nsh_core::element::View {
    let data = match entry.get() {
        ReadState::Data(data) => Some(data),
        ReadState::Error(_) | ReadState::Loading => None,
    };
    let title = data
        .as_ref()
        .and_then(|value| value.get("title"))
        .map(value_string)
        .unwrap_or_else(|| "Entry".to_owned());
    let mut top = view()
        .app_class(Some("flex-row items-center gap-2 px-3.5 py-3 border-b border-black/10 dark:border-[#2b2b2b]".into()))
        .child(text(title).app_class(Some(format!("flex-1 text-[14px] font-bold {}", kit::C_INK).into())).number_of_lines(1));
    if let Some(data) = data.as_ref() {
        top = top.child(pill(
            text_value(data.get("status_label"), ""),
            text_value(data.get("pill"), ""),
        ));
    }
    top = top.child(button("✕", "plain", {
        move || drawer_entry.set(String::new())
    }));

    let mut content = view().app_class(Some("p-3.5 gap-3".into()));
    match entry.get() {
        ReadState::Loading => {
            content = content.child(
                text("Loading…").app_class(Some(format!("text-[13px] {}", kit::C_DIM).into())),
            );
        }
        ReadState::Error(message) => {
            let retry_bridge = bridge.clone();
            let retry_list = list;
            let retry_entry = entry;
            let ticket = drawer_entry.get();
            let retry_account = account.get();
            let retry_month = month.get();
            let retry_filter = filter.get();
            content = content
                .child(text(message).app_class(Some(format!("text-[13px] {}", kit::C_RED).into())))
                .child(button("Try again", "plain", move || {
                    load_list(
                        screen_owner,
                        retry_bridge.clone(),
                        retry_list,
                        retry_account.clone(),
                        retry_month.clone(),
                        retry_filter.clone(),
                    );
                    load_entry(
                        screen_owner,
                        retry_bridge.clone(),
                        retry_entry,
                        ticket.clone(),
                    );
                }));
        }
        ReadState::Data(data) => {
            content = content.child(drawer_data(
                screen_owner,
                bridge.clone(),
                list,
                entry,
                account,
                month,
                filter,
                drawer_entry,
                toast,
                toast_epoch,
                version,
                busy,
                ids.clone(),
                data,
            ));
        }
    }
    let mut actions = view().app_class(Some(
        "flex-row gap-2 flex-wrap px-3.5 py-3 border-t border-black/10 dark:border-[#2b2b2b]"
            .into(),
    ));
    let can_amend = data
        .as_ref()
        .is_some_and(|value| has_action(value, "amend"));
    let amend_form = if can_amend && amend.open_for.get() == drawer_entry.get() {
        Some(amend_form(
            screen_owner,
            bridge.clone(),
            ids.clone(),
            list,
            entry,
            account.get(),
            month.get(),
            filter.get(),
            drawer_entry,
            toast,
            toast_epoch,
            version,
            busy,
            amend,
        ))
    } else {
        None
    };
    if can_amend && amend_form.is_none() {
        actions = actions.child(button("Amend", "plain", move || {
            amend.open(drawer_entry.get());
        }));
    }
    if data.as_ref().is_some_and(|value| has_action(value, "void")) {
        actions = actions.child(button("Void (adds a reversal)", "plain", {
            let ids = ids.clone();
            let args = json!({"entry":drawer_entry.get()});
            let bridge = bridge.clone();
            let account = account.get();
            let month = month.get();
            let filter = filter.get();
            move || {
                write_and_refresh(
                    screen_owner,
                    bridge.clone(),
                    ids.clone(),
                    "books_app_void_entry",
                    args.clone(),
                    list,
                    entry,
                    account.clone(),
                    month.clone(),
                    filter.clone(),
                    drawer_entry,
                    toast,
                    toast_epoch,
                    version,
                    busy,
                    WriteAction::Void,
                )
            }
        }));
    }
    if data.as_ref().is_some_and(|value| has_action(value, "copy")) {
        actions = actions.child(button("Duplicate", "plain", {
            let ids = ids.clone();
            let bridge = bridge.clone();
            let args = json!({"copy_of":drawer_entry.get()});
            let account = account.get();
            let month = month.get();
            let filter = filter.get();
            move || {
                write_and_refresh(
                    screen_owner,
                    bridge.clone(),
                    ids.clone(),
                    "books_entry_draft",
                    args.clone(),
                    list,
                    entry,
                    account.clone(),
                    month.clone(),
                    filter.clone(),
                    drawer_entry,
                    toast,
                    toast_epoch,
                    version,
                    busy,
                    WriteAction::Duplicate,
                )
            }
        }));
    }
    if data
        .as_ref()
        .is_some_and(|value| has_action(value, "discard"))
    {
        actions = actions.child(button("Discard", "plain", {
            let ids = ids.clone();
            let args = json!({"entry":drawer_entry.get()});
            let bridge = bridge.clone();
            let account = account.get();
            let month = month.get();
            let filter = filter.get();
            move || {
                write_and_refresh(
                    screen_owner,
                    bridge.clone(),
                    ids.clone(),
                    "books_app_discard_entry",
                    args.clone(),
                    list,
                    entry,
                    account.clone(),
                    month.clone(),
                    filter.clone(),
                    drawer_entry,
                    toast,
                    toast_epoch,
                    version,
                    busy,
                    WriteAction::Discard,
                )
            }
        }));
    }
    if data.as_ref().is_some_and(|value| has_action(value, "post")) {
        actions = actions.child(button("Post", "go", {
            let ids = ids.clone();
            let blocked = data
                .as_ref()
                .and_then(|value| value.get("post_blocker"))
                .is_some_and(|value| {
                    !value.is_null() && !value.as_str().unwrap_or_default().is_empty()
                });
            let args = json!({"entry":drawer_entry.get()});
            let bridge = bridge.clone();
            let account = account.get();
            let month = month.get();
            let filter = filter.get();
            move || {
                if !blocked {
                    write_and_refresh(
                        screen_owner,
                        bridge.clone(),
                        ids.clone(),
                        "books_app_post_entry",
                        args.clone(),
                        list,
                        entry,
                        account.clone(),
                        month.clone(),
                        filter.clone(),
                        drawer_entry,
                        toast,
                        toast_epoch,
                        version,
                        busy,
                        WriteAction::Post,
                    );
                }
            }
        }));
    }
    let mut panel = view()
        .app_class(Some("absolute top-0 right-0 bottom-0 w-[340px] border-l bg-white dark:bg-[#1d1d1d] border-black/10 dark:border-[#2b2b2b]".into()))
        .child(top)
        .child(scroll().app_class(Some("flex-1".into())).child(content));
    if let Some(form) = amend_form {
        panel = panel.child(form);
    }
    panel.child(actions)
}

/// Amend by reversal (M10 ledger-3): a reason and an optional date. A void
/// with `amend: true` (no new tool): the original shows Voided, a reversal
/// posts, and a replacement draft with the same lines opens. Only a press
/// here amends (DD-20). The reason goes to Books and is never shown back.
#[allow(clippy::too_many_arguments)]
fn amend_form(
    screen_owner: Owner,
    bridge: Bridge,
    ids: RequestIds,
    list: nsh_core::element::Signal<ReadState>,
    entry: nsh_core::element::Signal<ReadState>,
    account: String,
    month: String,
    filter: String,
    drawer_entry: nsh_core::element::Signal<String>,
    toast: nsh_core::element::Signal<String>,
    toast_epoch: nsh_core::element::Signal<u64>,
    version: nsh_core::element::Signal<u64>,
    busy: nsh_core::element::Signal<bool>,
    amend: AmendFields,
) -> nsh_core::element::View {
    let mut form = view()
        .app_class(Some(
            "gap-2.5 px-3.5 py-3 border-t border-black/10 dark:border-[#2b2b2b]".into(),
        ))
        .child(
            text("Amend this entry")
                .app_class(Some(format!("text-[13px] font-bold {}", kit::C_INK).into())),
        )
        .child(
            text("It is voided with a reversal, and a draft with the same lines opens for you to correct.")
                .app_class(Some(format!("text-[12px] {}", kit::C_DIM).into())),
        )
        .child(field(
            "Reason",
            text_box(
                amend.reason.get(),
                "Why it changes (3 to 200 characters)",
                false,
                move |value| amend.reason.set(value),
            ),
        ))
        .child(field(
            "Date (optional)",
            text_box(amend.date.get(), "YYYY-MM-DD", false, move |value| {
                amend.date.set(value)
            }),
        ));
    let error = amend.error.get();
    if !error.is_empty() {
        form =
            form.child(text(error).app_class(Some(format!("text-[12px] {}", kit::C_RED).into())));
    }
    form.child(
        view()
            .app_class(Some("flex-row gap-2".into()))
            .child(button("Cancel", "plain", move || {
                amend.open_for.set(String::new())
            }))
            .child(button("Amend entry", "go", move || {
                let reason = amend.reason.get().trim().to_owned();
                if reason.chars().count() < 3 {
                    amend
                        .error
                        .set("Give a reason of at least 3 characters.".into());
                    return;
                }
                amend.error.set(String::new());
                let mut args = json!({"entry":drawer_entry.get(),"amend":true,"reason":reason});
                let date = amend.date.get().trim().to_owned();
                if !date.is_empty() {
                    args["date"] = json!(date);
                }
                write_and_refresh(
                    screen_owner,
                    bridge.clone(),
                    ids.clone(),
                    "books_app_void_entry",
                    args,
                    list,
                    entry,
                    account.clone(),
                    month.clone(),
                    filter.clone(),
                    drawer_entry,
                    toast,
                    toast_epoch,
                    version,
                    busy,
                    WriteAction::Amend,
                );
            })),
    )
}

#[allow(clippy::too_many_arguments)]
fn drawer_data(
    screen_owner: Owner,
    bridge: Bridge,
    list: nsh_core::element::Signal<ReadState>,
    entry: nsh_core::element::Signal<ReadState>,
    account: nsh_core::element::Signal<String>,
    month: nsh_core::element::Signal<String>,
    filter: nsh_core::element::Signal<String>,
    drawer_entry: nsh_core::element::Signal<String>,
    toast: nsh_core::element::Signal<String>,
    toast_epoch: nsh_core::element::Signal<u64>,
    version: nsh_core::element::Signal<u64>,
    busy: nsh_core::element::Signal<bool>,
    ids: RequestIds,
    data: Value,
) -> nsh_core::element::View {
    let mut detail = view()
        .app_class(Some("gap-3".into()))
        .child(
            text(text_value(data.get("big"), "").to_owned()).app_class(Some(
                format!("text-[24px] font-semibold font-mono {}", kit::C_INK).into(),
            )),
        )
        .child(
            view()
                .app_class(Some("gap-1.5".into()))
                .child(kv("Date", text_value(data.get("date_label"), "")))
                .child(kv(
                    text_value_or(data.get("who_label"), "Who"),
                    text_value_or(data.get("payee"), "—"),
                ))
                .child(kv("Category", text_value_or(data.get("category"), "—")))
                .child(kv("Reference", text_value_or(data.get("reference"), "—")))
                .child(kv("Entry", text_value(data.get("no"), "—"))),
        );
    if let Some(related) = data.get("related").and_then(Value::as_str)
        && !related.is_empty()
    {
        detail = detail.child(kv("Linked", related));
    }
    let open_entry = {
        let bridge = bridge.clone();
        move |ticket: String| {
            drawer_entry.set(ticket.clone());
            load_entry(screen_owner, bridge.clone(), entry, ticket);
        }
    };
    if let Some(links) = amend_links(&data, open_entry.clone()) {
        detail = detail.child(links);
    }
    let keep_both = {
        let ids = ids.clone();
        let bridge = bridge.clone();
        let account = account.get();
        let month = month.get();
        let filter = filter.get();
        move || {
            write_and_refresh(
                screen_owner,
                bridge.clone(),
                ids.clone(),
                "books_app_post_entry",
                json!({"entry":drawer_entry.get(),"keep_both":true}),
                list,
                entry,
                account.clone(),
                month.clone(),
                filter.clone(),
                drawer_entry,
                toast,
                toast_epoch,
                version,
                busy,
                WriteAction::Post,
            )
        }
    };
    if let Some(review) = review_panel(&data, open_entry, keep_both) {
        detail = detail.child(review);
    }
    if data.get("reviewable").and_then(Value::as_bool) == Some(true) {
        let mut suggestions = view().app_class(Some("flex-row gap-1.5 flex-wrap".into()));
        if let Some(items) = data.get("suggest").and_then(Value::as_array) {
            for item in items {
                let category = text_value(item.get("name"), "").to_owned();
                let ticket = text_value(item.get("ticket"), "").to_owned();
                let best = item.get("best").and_then(Value::as_bool) == Some(true);
                let label = if best {
                    format!("{category} · likely")
                } else {
                    category.clone()
                };
                let bridge_for_action = bridge.clone();
                let list_for_action = list;
                let entry_for_action = entry;
                let account_for_action = account.get();
                let month_for_action = month.get();
                let filter_for_action = filter.get();
                let drawer_for_action = drawer_entry;
                let toast_for_action = toast;
                let toast_epoch_for_action = toast_epoch;
                let version_for_action = version;
                let busy_for_action = busy;
                let ids_for_action = ids.clone();
                let category_for_action = category.clone();
                let ticket_for_action = ticket.clone();
                suggestions = suggestions.child(
                    view()
                        .app_class(Some(
                            format!(
                                "rounded-full border px-2.5 py-1 bg-[#f0f0ee] dark:bg-[#262626] {}",
                                kit::K_LINE
                            )
                            .into(),
                        ))
                        .on_press(move || {
                            let category = category_for_action.clone();
                            let call_bridge = bridge_for_action.clone();
                            let list = list_for_action;
                            let entry_state = entry_for_action;
                            let account_state = account_for_action.clone();
                            let month_state = month_for_action.clone();
                            let filter_state = filter_for_action.clone();
                            let drawer_state = drawer_for_action;
                            let version = version_for_action;
                            let busy = busy_for_action;
                            let toast = toast_for_action;
                            let toast_epoch = toast_epoch_for_action;
                            if busy.get() {
                                return;
                            }
                            busy.set(true);
                            let mut args = json!({
                                "entry":drawer_state.get(),
                                "category":ticket_for_action.clone()
                            });
                            args["client_request_id"] = json!(ids_for_action.next());
                            spawn_task(screen_owner, call_bridge.clone(), async move {
                                match call_bridge.call("books_app_recategorize", args).await {
                                    Ok(_) => {
                                        version.set(version.get() + 1);
                                        show_toast(format!("Now {category}"), toast, toast_epoch);
                                        refresh_after_write(
                                            call_bridge,
                                            list,
                                            entry_state,
                                            account_state,
                                            month_state,
                                            filter_state,
                                            drawer_state,
                                        )
                                        .await;
                                        busy.set(false);
                                    }
                                    Err(error) => {
                                        busy.set(false);
                                        show_toast(error.message, toast, toast_epoch);
                                    }
                                }
                            });
                        })
                        .child(
                            text(label).app_class(Some(
                                format!(
                                    "text-[12px] {}",
                                    if best { kit::C_GREEN_BG } else { kit::C_INK }
                                )
                                .into(),
                            )),
                        ),
                );
            }
        }
        detail = detail.child(
            view()
                .app_class(Some("gap-1.5".into()))
                .child(
                    text("Pick a category")
                        .app_class(Some(format!("text-[12px] {}", kit::C_DIM).into())),
                )
                .child(suggestions),
        );
    }
    let mut table = view()
        .app_class(Some(
            format!("rounded-[10px] border overflow-hidden {}", kit::K_LINE).into(),
        ))
        .child(
            view()
                .app_class(Some("flex-row gap-1.5 px-2.5 py-1.5".into()))
                .child(
                    text("Account").app_class(Some(
                        format!(
                            "flex-1 text-[10.5px] uppercase tracking-wider {}",
                            kit::C_DIM
                        )
                        .into(),
                    )),
                )
                .child(
                    text("Debit").app_class(Some(
                        format!(
                            "w-[84px] text-right text-[10.5px] uppercase tracking-wider {}",
                            kit::C_DIM
                        )
                        .into(),
                    )),
                )
                .child(
                    text("Credit").app_class(Some(
                        format!(
                            "w-[84px] text-right text-[10.5px] uppercase tracking-wider {}",
                            kit::C_DIM
                        )
                        .into(),
                    )),
                ),
        );
    if let Some(lines) = data.get("lines").and_then(Value::as_array) {
        for line in lines {
            table = table.child(
                view()
                    .app_class(Some(
                        format!("flex-row gap-1.5 px-2.5 py-1.5 border-t {}", kit::K_LINE).into(),
                    ))
                    .child(
                        text(text_value(line.get("account"), "").to_owned())
                            .app_class(Some(format!("flex-1 text-[12px] {}", kit::C_INK).into()))
                            .number_of_lines(1),
                    )
                    .child(
                        text(text_value(line.get("debit_shown"), "").to_owned()).app_class(Some(
                            format!("w-[84px] text-right text-[12px] font-mono {}", kit::C_INK)
                                .into(),
                        )),
                    )
                    .child(
                        text(text_value(line.get("credit_shown"), "").to_owned()).app_class(Some(
                            format!("w-[84px] text-right text-[12px] font-mono {}", kit::C_INK)
                                .into(),
                        )),
                    ),
            );
        }
    }
    detail = detail.child(
        view()
            .app_class(Some("gap-1.5".into()))
            .child(
                text("Behind the scenes")
                    .app_class(Some(format!("text-[12px] {}", kit::C_DIM).into())),
            )
            .child(table),
    );
    if let Some(memo) = data.get("memo").and_then(Value::as_str)
        && !memo.is_empty()
    {
        detail = detail.child(
            view()
                .app_class(Some("gap-1".into()))
                .child(text("Memo").app_class(Some(format!("text-[12px] {}", kit::C_DIM).into())))
                .child(
                    text(memo.to_owned())
                        .app_class(Some(format!("text-[12.5px] {}", kit::C_INK).into())),
                ),
        );
    }
    if let Some(blocker) = data.get("post_blocker").and_then(Value::as_str)
        && !blocker.is_empty()
    {
        detail = detail.child(
            text(blocker.to_owned()).app_class(Some(format!("text-[12px] {}", kit::C_RED).into())),
        );
    }
    detail
}

/// A field's name in the review panel.
fn field_name(field: &str) -> &str {
    match field {
        "date" => "Date",
        "amount" => "Amount",
        "account" => "Account",
        "to_account" => "To account",
        "category" => "Category",
        "payee" => "Payee",
        "memo" => "Memo",
        "reference" => "Reference",
        "lines" => "Lines",
        other => other,
    }
}

/// A whole-percent confidence as a word: High 90+, Check 70-89, Low below
/// 70. The assistant reports it; Books computes none.
fn confidence_word(confidence: Option<&Value>) -> &'static str {
    match confidence.and_then(Value::as_i64) {
        Some(n) if n >= 90 => "High",
        Some(n) if n >= 70 => "Check",
        Some(_) => "Low",
        None => "—",
    }
}

/// The drawer's review notes (M10 ledger-1): the fields the assistant or a
/// routine set, with the confidence it reported, and the likely duplicates
/// to open or keep. `None` when there is nothing to show.
fn review_panel(
    data: &Value,
    open_entry: impl Fn(String) + Clone + 'static,
    keep_both: impl Fn() + 'static,
) -> Option<nsh_core::element::View> {
    let review = data.get("review")?;
    let changed: Vec<&Value> = review
        .get("fields")
        .and_then(Value::as_array)
        .map(|fields| {
            fields
                .iter()
                .filter(|f| {
                    f.get("changed_by")
                        .and_then(Value::as_str)
                        .is_some_and(|by| !by.is_empty() && by != "app")
                })
                .collect()
        })
        .unwrap_or_default();
    let duplicates: &[Value] = review
        .get("likely_duplicates")
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice);
    if changed.is_empty() && duplicates.is_empty() {
        return None;
    }
    let label = |value: &str| {
        text(value.to_owned()).app_class(Some(format!("text-[12px] {}", kit::C_DIM).into()))
    };
    let mut panel = view().app_class(Some("gap-1.5".into()));
    if !changed.is_empty() {
        panel = panel.child(label(
            if text_value(review.get("proposed_by"), "") == "routine" {
                "Set by a routine"
            } else {
                "Set by the assistant"
            },
        ));
        for f in changed {
            panel = panel.child(kv(
                field_name(text_value(f.get("field"), "")),
                confidence_word(f.get("confidence")),
            ));
        }
    }
    if !duplicates.is_empty() {
        panel = panel.child(label("Possible duplicates"));
        for d in duplicates {
            let ticket = text_value(d.get("ticket"), "").to_owned();
            let open_entry = open_entry.clone();
            panel = panel.child(
                view()
                    .app_class(Some(
                        format!(
                            "flex-row gap-2 rounded-[8px] border px-2.5 py-1.5 {}",
                            kit::K_LINE
                        )
                        .into(),
                    ))
                    .on_press(move || open_entry(ticket.clone()))
                    .child(
                        text(format!(
                            "Entry {} · {}",
                            text_value(d.get("no"), ""),
                            text_value(d.get("date"), "")
                        ))
                        .app_class(Some(format!("flex-1 text-[12px] {}", kit::C_INK).into())),
                    )
                    .child(
                        text(
                            text_value_or(d.get("amount_shown"), text_value(d.get("status"), ""))
                                .to_owned(),
                        )
                        .app_class(Some(format!("text-[12px] {}", kit::C_DIM).into())),
                    ),
            );
        }
        if text_value(data.get("status"), "") == "draft" {
            panel = panel.child(button("Keep both and post", "go", keep_both));
        }
    }
    Some(panel)
}

/// "Amends entry N" / "Amended by entry N" chips; a press opens that entry.
fn amend_links(
    data: &Value,
    open_entry: impl Fn(String) + Clone + 'static,
) -> Option<nsh_core::element::View> {
    let mut chips = view().app_class(Some("flex-row gap-2 flex-wrap".into()));
    let mut any = false;
    for key in ["amends", "amended_by"] {
        let ticket = text_value(data.pointer(&format!("/links/{key}")), "").to_owned();
        let label = text_value(data.pointer(&format!("/link_labels/{key}")), "").to_owned();
        if ticket.is_empty() || label.is_empty() {
            continue;
        }
        any = true;
        let open_entry = open_entry.clone();
        chips = chips.child(button(&label, "plain", move || open_entry(ticket.clone())));
    }
    any.then_some(chips)
}

#[allow(clippy::too_many_arguments)]
fn write_and_refresh(
    screen_owner: Owner,
    bridge: Bridge,
    ids: RequestIds,
    tool: &'static str,
    mut args: Value,
    list: nsh_core::element::Signal<ReadState>,
    entry: nsh_core::element::Signal<ReadState>,
    account: String,
    month: String,
    filter: String,
    drawer_entry: nsh_core::element::Signal<String>,
    toast: nsh_core::element::Signal<String>,
    toast_epoch: nsh_core::element::Signal<u64>,
    version: nsh_core::element::Signal<u64>,
    busy: nsh_core::element::Signal<bool>,
    action: WriteAction,
) {
    if busy.get() {
        return;
    }
    busy.set(true);
    args["client_request_id"] = json!(ids.next());
    let call_bridge = bridge.clone();
    spawn_task(screen_owner, call_bridge.clone(), async move {
        match call_bridge.call(tool, args).await {
            Ok(result) => {
                version.set(version.get() + 1);
                let close_drawer = matches!(action, WriteAction::Discard);
                let mut duplicate_entry = None;
                match action {
                    WriteAction::Post => show_toast(
                        format!("Entry {} posted", text_value(result.get("no"), "")),
                        toast,
                        toast_epoch,
                    ),
                    WriteAction::Void => show_toast(
                        format!(
                            "Entry {} voided; a reversal is posted",
                            text_value(result.get("no"), "")
                        ),
                        toast,
                        toast_epoch,
                    ),
                    WriteAction::Discard => {}
                    WriteAction::Duplicate => {
                        show_toast(
                            format!("Draft {} saved", text_value(result.get("no"), "")),
                            toast,
                            toast_epoch,
                        );
                        duplicate_entry = Some(text_value(result.get("entry"), "").to_owned());
                    }
                    WriteAction::Amend => {
                        show_toast(
                            format!(
                                "Draft {} saved; the original is voided",
                                text_value(result.get("replacement_no"), "")
                            ),
                            toast,
                            toast_epoch,
                        );
                        duplicate_entry =
                            Some(text_value(result.get("replacement"), "").to_owned());
                    }
                }
                refresh_after_write(
                    bridge.clone(),
                    list,
                    entry,
                    account,
                    month,
                    filter,
                    drawer_entry,
                )
                .await;
                if close_drawer {
                    drawer_entry.set(String::new());
                }
                if let Some(ticket) = duplicate_entry {
                    drawer_entry.set(ticket.clone());
                    load_entry(screen_owner, bridge, entry, ticket);
                }
                busy.set(false);
            }
            Err(error) => {
                busy.set(false);
                show_toast(error.message, toast, toast_epoch);
            }
        }
    });
}

#[derive(Clone, Copy)]
enum WriteAction {
    Post,
    Void,
    Discard,
    Duplicate,
    Amend,
}

async fn refresh_after_write(
    bridge: Bridge,
    list: nsh_core::element::Signal<ReadState>,
    entry: nsh_core::element::Signal<ReadState>,
    account: String,
    month: String,
    filter: String,
    drawer_entry: nsh_core::element::Signal<String>,
) {
    let mut args = serde_json::Map::new();
    args.insert("filter".into(), json!(filter));
    if !account.is_empty() {
        args.insert("account".into(), json!(account));
    }
    if !month.is_empty() {
        args.insert("month".into(), json!(month));
    }
    list.set(ReadState::Loading);
    let list_result = bridge.call("books_ledger_list", Value::Object(args)).await;
    list.set(match list_result {
        Ok(value) => ReadState::Data(value),
        Err(error) => ReadState::Error(error.message),
    });
    let ticket = drawer_entry.get();
    if !ticket.is_empty() {
        entry.set(ReadState::Loading);
        let entry_result = bridge
            .call("books_entry_get", json!({"entry":ticket}))
            .await;
        entry.set(match entry_result {
            Ok(value) => ReadState::Data(value),
            Err(error) => ReadState::Error(error.message),
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn sheet(
    bridge: Bridge,
    list: nsh_core::element::Signal<ReadState>,
    entry: nsh_core::element::Signal<ReadState>,
    options: nsh_core::element::Signal<ReadState>,
    account: nsh_core::element::Signal<String>,
    month: nsh_core::element::Signal<String>,
    filter: nsh_core::element::Signal<String>,
    drawer_entry: nsh_core::element::Signal<String>,
    mode: nsh_core::element::Signal<String>,
    fields: SheetFields,
    toast: nsh_core::element::Signal<String>,
    toast_epoch: nsh_core::element::Signal<u64>,
    version: nsh_core::element::Signal<u64>,
    ids: RequestIds,
    screen_owner: Owner,
) -> nsh_core::element::View {
    let options_data = match options.get() {
        ReadState::Data(data) => Some(data),
        ReadState::Loading | ReadState::Error(_) => None,
    };
    let mut header = view()
        .app_class(Some(
            format!(
                "flex-row items-center gap-2.5 px-3.5 py-3 border-b {}",
                kit::K_LINE
            )
            .into(),
        ))
        .child(text("New entry").app_class(Some(
            format!("flex-1 text-[14px] font-bold {}", kit::C_INK).into(),
        )));
    for (key, label) in [
        ("money_in", "Money in"),
        ("money_out", "Money out"),
        ("transfer", "Transfer"),
        ("journal", "Journal"),
    ] {
        let active = mode.get() == key;
        header = header.child(button(label, if active { "selected" } else { "plain" }, {
            let key = key.to_owned();
            move || mode.set(key.clone())
        }));
    }
    header = header.child(button("✕", "plain", move || mode.set(String::new())));

    let mut form = view().app_class(Some("p-3.5 gap-3".into()));
    if let ReadState::Error(message) = options.get() {
        let retry_bridge = bridge.clone();
        let retry_options = options;
        form = form
            .child(text(message).app_class(Some(format!("text-[13px] {}", kit::C_RED).into())))
            .child(button("Try again", "plain", move || {
                load_options(screen_owner, retry_bridge.clone(), retry_options);
            }));
    }
    if mode.get() != "journal" {
        form = form
            .child(field(
                "Amount",
                text_box(fields.amount.get(), "₱ 0.00", true, move |value| {
                    fields.amount.set(value)
                }),
            ))
            .child(
                view()
                    .app_class(Some("flex-row gap-3".into()))
                    .child(field(
                        "Date",
                        text_box(
                            fields.date.get(),
                            text_value(
                                options_data.as_ref().and_then(|data| data.get("today")),
                                "YYYY-MM-DD",
                            ),
                            false,
                            move |value| fields.date.set(value),
                        ),
                    ))
                    .child(field(
                        match mode.get().as_str() {
                            "money_in" => "Into",
                            "transfer" => "From",
                            _ => "Paid from",
                        },
                        picker(
                            fields.money.get(),
                            options_data
                                .as_ref()
                                .and_then(|data| data.get("money"))
                                .and_then(Value::as_array)
                                .cloned()
                                .unwrap_or_default(),
                            "Bank, cash or card",
                            fields.money_open,
                            move |ticket| fields.money.set(ticket),
                        ),
                    )),
            );
        if mode.get() == "transfer" {
            form = form.child(field(
                "To",
                picker(
                    fields.to_account.get(),
                    options_data
                        .as_ref()
                        .and_then(|data| data.get("money"))
                        .and_then(Value::as_array)
                        .cloned()
                        .unwrap_or_default(),
                    "Bank, cash or card",
                    fields.to_account_open,
                    move |ticket| fields.to_account.set(ticket),
                ),
            ));
        } else {
            let categories_key = if mode.get() == "money_in" {
                "income"
            } else {
                "expense"
            };
            form = form.child(
                view()
                    .app_class(Some("flex-row gap-3".into()))
                    .child(field(
                        if mode.get() == "money_in" {
                            "From"
                        } else {
                            "To"
                        },
                        text_box(
                            fields.payee.get(),
                            if mode.get() == "money_in" {
                                "Customer"
                            } else {
                                "Vendor"
                            },
                            false,
                            move |value| fields.payee.set(value),
                        ),
                    ))
                    .child(field(
                        "Category",
                        picker(
                            fields.category.get(),
                            options_data
                                .as_ref()
                                .and_then(|data| data.get(categories_key))
                                .and_then(Value::as_array)
                                .cloned()
                                .unwrap_or_default(),
                            "Pick a category",
                            fields.category_open,
                            move |ticket| fields.category.set(ticket),
                        ),
                    )),
            );
        }
        form = form.child(
            view()
                .app_class(Some("flex-row gap-3".into()))
                .child(field(
                    "What it was for",
                    text_box(fields.memo.get(), "Office supplies", false, move |value| {
                        fields.memo.set(value)
                    }),
                ))
                .child(field(
                    "Reference",
                    text_box(
                        fields.reference.get(),
                        "Invoice 0143",
                        false,
                        move |value| fields.reference.set(value),
                    ),
                )),
        );
        let behind = behind_text(mode.get().as_str(), fields, options_data.as_ref());
        form = form.child(
            view()
                .app_class(Some(
                    format!(
                        "rounded-[10px] border border-dashed p-2.5 gap-1.5 {}",
                        kit::K_LINE
                    )
                    .into(),
                ))
                .child(text("Behind the scenes").app_class(Some(
                    format!("text-[12px] font-bold {}", kit::C_MUTE).into(),
                )))
                .child(
                    text(behind)
                        .app_class(Some(format!("text-[12px] font-mono {}", kit::C_INK).into())),
                ),
        );
        if mode.get() == "transfer" {
            form = form.child(
                text("Not income, not an expense. Reports leave it out of the P&L.")
                    .app_class(Some(format!("text-[12px] {}", kit::C_DIM).into())),
            );
        }
    } else {
        form = form.child(
            text("For accountants: any balanced set of lines.")
                .app_class(Some(format!("text-[12px] {}", kit::C_DIM).into())),
        );
        let lines = fields.lines.get();
        for (index, line) in lines.iter().enumerate() {
            let line_picker = picker(
                line.account.clone(),
                options_data
                    .as_ref()
                    .and_then(|data| data.get("all"))
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default(),
                "Account",
                fields.line_account_open,
                move |ticket| set_line(fields.lines, index, "account", ticket),
            );
            form = form.child(
                view()
                    .app_class(Some("flex-row gap-2 items-start".into()))
                    .child(line_picker)
                    .child(text_box(line.debit.clone(), "Debit", false, {
                        move |value| set_line(fields.lines, index, "debit", value)
                    }))
                    .child(text_box(line.credit.clone(), "Credit", false, {
                        move |value| set_line(fields.lines, index, "credit", value)
                    })),
            );
        }
        form = form.child(button("+ Line", "plain", {
            let lines = fields.lines;
            move || {
                let mut values = lines.get();
                values.push(JournalLine {
                    account: String::new(),
                    debit: String::new(),
                    credit: String::new(),
                });
                lines.set(values);
            }
        }));
        form = form.child(field(
            "What it was for",
            text_box(fields.memo.get(), "Prepaid software", false, move |value| {
                fields.memo.set(value)
            }),
        ));
    }
    let error = fields.error.get();
    if !error.is_empty() {
        form =
            form.child(text(error).app_class(Some(format!("text-[12.5px] {}", kit::C_RED).into())));
    }
    let basis_label = options_data
        .as_ref()
        .and_then(|data| data.pointer("/book/basis_label"))
        .map(value_string)
        .unwrap_or_else(|| "Cash basis".to_owned());
    let footer = view()
        .app_class(Some(
            format!(
                "flex-row items-center gap-2 px-3.5 py-3 border-t {}",
                kit::K_LINE
            )
            .into(),
        ))
        .child(
            text(format!("{basis_label} · counts on its date"))
                .app_class(Some(format!("flex-1 text-[12px] {}", kit::C_DIM).into())),
        )
        .child(button("Cancel", "plain", {
            move || mode.set(String::new())
        }))
        .child(button("Save draft", "plain", {
            let ids = ids.clone();
            let bridge = bridge.clone();
            move || {
                save_sheet(
                    screen_owner,
                    bridge.clone(),
                    list,
                    entry,
                    options,
                    account,
                    month,
                    filter,
                    drawer_entry,
                    mode,
                    fields,
                    toast,
                    toast_epoch,
                    version,
                    ids.clone(),
                    false,
                )
            }
        }))
        .child(button("Save & post", "go", {
            let ids = ids.clone();
            let bridge = bridge.clone();
            move || {
                save_sheet(
                    screen_owner,
                    bridge.clone(),
                    list,
                    entry,
                    options,
                    account,
                    month,
                    filter,
                    drawer_entry,
                    mode,
                    fields,
                    toast,
                    toast_epoch,
                    version,
                    ids.clone(),
                    true,
                )
            }
        }));
    view()
        .app_class(Some(
            "absolute inset-0 bg-black/50 items-center justify-center".into(),
        ))
        .child(
            view()
                .app_class(Some(
                    format!(
                        "w-[560px] max-h-[600px] rounded-[14px] border {}",
                        kit::K_LINE
                    )
                    .into(),
                ))
                .child(header)
                .child(scroll().app_class(Some("max-h-[460px]".into())).child(form))
                .child(footer),
        )
}

#[allow(clippy::too_many_arguments)]
fn save_sheet(
    screen_owner: Owner,
    bridge: Bridge,
    list: nsh_core::element::Signal<ReadState>,
    entry: nsh_core::element::Signal<ReadState>,
    options: nsh_core::element::Signal<ReadState>,
    account: nsh_core::element::Signal<String>,
    month: nsh_core::element::Signal<String>,
    filter: nsh_core::element::Signal<String>,
    drawer_entry: nsh_core::element::Signal<String>,
    mode: nsh_core::element::Signal<String>,
    fields: SheetFields,
    toast: nsh_core::element::Signal<String>,
    toast_epoch: nsh_core::element::Signal<u64>,
    version: nsh_core::element::Signal<u64>,
    ids: RequestIds,
    post: bool,
) {
    if fields.busy.get() {
        return;
    }
    let Some(args) = sheet_args(mode.get().as_str(), fields) else {
        return;
    };
    fields.busy.set(true);
    let mut args = args;
    args["client_request_id"] = json!(ids.next());
    let tool = if post {
        "books_app_post_entry"
    } else {
        "books_entry_draft"
    };
    let call_bridge = bridge.clone();
    spawn_task(screen_owner, call_bridge.clone(), async move {
        match call_bridge.call(tool, args).await {
            Ok(result) => {
                version.set(version.get() + 1);
                show_toast(
                    if post {
                        format!("Entry {} posted", text_value(result.get("no"), ""))
                    } else {
                        format!("Draft {} saved", text_value(result.get("no"), ""))
                    },
                    toast,
                    toast_epoch,
                );
                if let (ReadState::Data(_), _) = (options.get(), mode.get()) {
                    load_options(screen_owner, call_bridge.clone(), options);
                }
                fields.amount.set(String::new());
                fields.memo.set(String::new());
                fields.reference.set(String::new());
                fields.payee.set(String::new());
                mode.set(String::new());
                fields.busy.set(false);
                let mut list_args = serde_json::Map::new();
                list_args.insert("filter".into(), json!(filter.get()));
                if !account.get().is_empty() {
                    list_args.insert("account".into(), json!(account.get()));
                }
                if !month.get().is_empty() {
                    list_args.insert("month".into(), json!(month.get()));
                }
                list.set(ReadState::Loading);
                let result = bridge
                    .call("books_ledger_list", Value::Object(list_args))
                    .await;
                list.set(match result {
                    Ok(value) => ReadState::Data(value),
                    Err(error) => ReadState::Error(error.message),
                });
                let ticket = drawer_entry.get();
                if !ticket.is_empty() {
                    load_entry(screen_owner, bridge.clone(), entry, ticket);
                }
            }
            Err(error) => {
                fields.error.set(error.message.clone());
                fields.busy.set(false);
                show_toast(error.message, toast, toast_epoch);
            }
        }
    });
}

fn sheet_args(mode: &str, fields: SheetFields) -> Option<Value> {
    fields.error.set(String::new());
    let mut base = json!({"form":mode});
    let date = fields.date.get();
    if !date.is_empty() {
        base["date"] = json!(date);
    }
    let payee = fields.payee.get();
    if !payee.is_empty() {
        base["payee"] = json!(payee);
    }
    let memo = fields.memo.get();
    if !memo.is_empty() {
        base["memo"] = json!(memo);
    }
    let reference = fields.reference.get();
    if !reference.is_empty() {
        base["reference"] = json!(reference);
    }
    if mode == "journal" {
        let lines = fields
            .lines
            .get()
            .into_iter()
            .filter(|line| !line.account.is_empty())
            .map(|line| {
                let mut item = json!({"account":line.account});
                if let Some(amount) = money_text(&line.debit) {
                    item["debit"] = json!(amount);
                }
                if let Some(amount) = money_text(&line.credit) {
                    item["credit"] = json!(amount);
                }
                item
            })
            .collect::<Vec<_>>();
        if lines.len() < 2 {
            fields
                .error
                .set("A journal needs at least two lines with an account.".into());
            return None;
        }
        base["lines"] = json!(lines);
        return Some(base);
    }
    let Some(amount) = money_text(&fields.amount.get()) else {
        fields.error.set("Type an amount, like 1,350.00.".into());
        return None;
    };
    base["amount"] = json!(amount);
    let money = fields.money.get();
    if money.is_empty() {
        fields.error.set(
            if mode == "transfer" {
                "Pick where the money leaves."
            } else {
                "Pick the bank, cash or card account."
            }
            .into(),
        );
        return None;
    }
    base["account"] = json!(money);
    if mode == "transfer" {
        let to_account = fields.to_account.get();
        if to_account.is_empty() {
            fields.error.set("Pick where the money goes.".into());
            return None;
        }
        base["to_account"] = json!(to_account);
    } else {
        let category = fields.category.get();
        if category.is_empty() {
            fields.error.set("Pick a category.".into());
            return None;
        }
        base["category"] = json!(category);
    }
    Some(base)
}

fn set_line(
    lines: nsh_core::element::Signal<Vec<JournalLine>>,
    index: usize,
    key: &'static str,
    value: String,
) {
    let mut current = lines.get();
    if let Some(line) = current.get_mut(index) {
        match key {
            "account" => line.account = value,
            "debit" => line.debit = value,
            "credit" => line.credit = value,
            _ => {}
        }
    }
    lines.set(current);
}

fn money_text(typed: &str) -> Option<String> {
    let mut normalized = typed
        .chars()
        .filter(|character| !matches!(character, '₱' | ',') && !character.is_whitespace())
        .collect::<String>();
    if normalized
        .get(..3)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("PHP"))
    {
        normalized = normalized[3..].to_owned();
    }
    if normalized.ends_with('.') {
        return None;
    }
    let mut pieces = normalized.split('.');
    let whole = pieces.next().unwrap_or_default();
    let fraction = pieces.next().unwrap_or_default();
    if pieces.next().is_some()
        || whole.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
        || fraction.len() > 2
    {
        return None;
    }
    let trimmed = whole.trim_start_matches('0');
    let whole = if trimmed.is_empty() { "0" } else { trimmed };
    let fraction = match fraction.len() {
        0 => "00",
        1 => {
            if fraction == "0" {
                "00"
            } else {
                return Some(format!("{whole}.{}0", fraction));
            }
        }
        _ => fraction,
    };
    let result = format!("{whole}.{fraction}");
    (result != "0.00").then_some(result)
}

fn behind_text(mode: &str, fields: SheetFields, options: Option<&Value>) -> String {
    let amount = money_text(&fields.amount.get()).unwrap_or_else(|| "0.00".into());
    let amount = grouped(&amount);
    let money_label = options
        .and_then(|data| data.get("money"))
        .and_then(Value::as_array)
        .and_then(|items| {
            items
                .iter()
                .find(|item| text_value(item.get("ticket"), "") == fields.money.get())
        })
        .map(|item| text_value(item.get("label"), ""))
        .unwrap_or("the bank");
    let category_label = options
        .and_then(|data| {
            data.get(if mode == "money_in" {
                "income"
            } else {
                "expense"
            })
        })
        .and_then(Value::as_array)
        .and_then(|items| {
            items
                .iter()
                .find(|item| text_value(item.get("ticket"), "") == fields.category.get())
        })
        .map(|item| text_value(item.get("label"), ""))
        .unwrap_or("the category");
    match mode {
        "money_in" => format!("Dr {money_label} {amount} · Cr {category_label} {amount}"),
        "transfer" => {
            let to = options
                .and_then(|data| data.get("money"))
                .and_then(Value::as_array)
                .and_then(|items| {
                    items
                        .iter()
                        .find(|item| text_value(item.get("ticket"), "") == fields.to_account.get())
                })
                .map(|item| text_value(item.get("label"), ""))
                .unwrap_or("the other account");
            format!("Dr {to} {amount} · Cr {money_label} {amount}")
        }
        _ => format!("Dr {category_label} {amount} · Cr {money_label} {amount}"),
    }
}

fn grouped(amount: &str) -> String {
    let (whole, fraction) = amount.split_once('.').unwrap_or((amount, "00"));
    let mut grouped = String::new();
    for (index, digit) in whole.chars().enumerate() {
        if index > 0 && (whole.len() - index) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    format!("{grouped}.{fraction}")
}

fn show_toast(
    message: String,
    toast: nsh_core::element::Signal<String>,
    epoch: nsh_core::element::Signal<u64>,
) {
    toast.set(message);
    let next = epoch.get() + 1;
    epoch.set(next);
    set_timeout(Duration::from_millis(2600), move || {
        if epoch.get() == next {
            toast.set(String::new());
        }
    });
}

fn head(title: &str, sub: String, action: nsh_core::element::View) -> nsh_core::element::View {
    let mut result = view()
        .app_class(Some("flex-row items-center gap-2.5 flex-wrap".into()))
        .child(
            text(title.to_owned())
                .app_class(Some(format!("text-[18px] font-bold {}", kit::C_INK).into())),
        );
    if !sub.is_empty() {
        result =
            result.child(text(sub).app_class(Some(format!("text-[13px] {}", kit::C_MUTE).into())));
    }
    result
        .child(view().app_class(Some("flex-1".into())))
        .child(action)
}

fn field(label: &str, control: nsh_core::element::View) -> nsh_core::element::View {
    view()
        .app_class(Some("gap-1 flex-1".into()))
        .child(text(label.to_owned()).app_class(Some(
            format!("text-[11px] uppercase tracking-wider {}", kit::C_DIM).into(),
        )))
        .child(control)
}

fn text_box(
    value: String,
    placeholder: &str,
    big: bool,
    on_change: impl Fn(String) + 'static,
) -> nsh_core::element::View {
    let classes = if big {
        "h-10 text-[18px] font-mono"
    } else {
        "h-8 text-[13px]"
    };
    view().child(
        input()
            .value(value)
            .placeholder(placeholder.to_owned())
            .app_class(Some(format!("rounded-lg border px-2.5 {classes} bg-[#f6f6f4] dark:bg-[#151515] border-black/10 dark:border-[#2b2b2b] {}", kit::C_INK).into()))
            .on_change(move |value: &str| on_change(value.to_owned())),
    )
}

fn picker(
    value: String,
    options: Vec<Value>,
    placeholder: &str,
    open: nsh_core::element::Signal<bool>,
    on_pick: impl Fn(String) + Clone + 'static,
) -> nsh_core::element::View {
    let current = options
        .iter()
        .find(|item| text_value(item.get("ticket"), "") == value)
        .map(|item| text_value(item.get("label"), text_value(item.get("name"), "")))
        .unwrap_or(placeholder)
        .to_owned();
    let mut result = view()
        .app_class(Some("gap-1".into()))
        .child(
            view()
                .app_class(Some(format!("flex-row items-center rounded-lg border px-2.5 h-8 bg-[#f6f6f4] dark:bg-[#151515] {}", kit::K_LINE).into()))
                .on_press(move || open.set(!open.get()))
                .child(text(current).app_class(Some(format!("flex-1 text-[13px] {}", kit::C_DIM).into())).number_of_lines(1))
                .child(text("⌄").app_class(Some(kit::C_DIM.into()))),
        );
    if open.get() {
        let mut options_view = view().app_class(Some(
            format!(
                "rounded-lg border max-h-[180px] bg-white dark:bg-[#1d1d1d] {}",
                kit::K_LINE
            )
            .into(),
        ));
        for option in options {
            let label =
                text_value(option.get("label"), text_value(option.get("name"), "")).to_owned();
            let ticket = text_value(option.get("ticket"), "").to_owned();
            let on_pick = on_pick.clone();
            options_view = options_view.child(
                view()
                    .app_class(Some("px-2.5 py-1.5".into()))
                    .on_press(move || {
                        on_pick(ticket.clone());
                        open.set(false);
                    })
                    .child(
                        text(label)
                            .app_class(Some(format!("text-[12.5px] {}", kit::C_INK).into()))
                            .number_of_lines(1),
                    ),
            );
        }
        result = result.child(
            scroll()
                .app_class(Some("max-h-[178px]".into()))
                .child(options_view),
        );
    }
    result
}

fn button(label: &str, tone: &str, on_press: impl Fn() + 'static) -> nsh_core::element::View {
    let (background, border, ink) = match tone {
        "go" => (
            "bg-[#157a48] dark:bg-[#3fcf89]",
            "border-[#157a48] dark:border-[#3fcf89]",
            "text-white dark:text-[#06281a]",
        ),
        "selected" => (
            "bg-[#f0f0ee] dark:bg-[#262626]",
            "border-black/10 dark:border-[#2b2b2b]",
            "text-[#1c1c1c] dark:text-[#f0f0f0]",
        ),
        _ => (
            "bg-[#f0f0ee] dark:bg-[#262626]",
            "border-black/10 dark:border-[#2b2b2b]",
            "text-[#1c1c1c] dark:text-[#f0f0f0]",
        ),
    };
    view()
        .app_class(Some(
            format!(
                "rounded-lg border px-3 py-1.5 items-center justify-center {background} {border}"
            )
            .into(),
        ))
        .active(|style| style.opacity(0.7))
        .on_press(on_press)
        .child(
            text(label.to_owned())
                .app_class(Some(format!("text-[12.5px] font-semibold {ink}").into()))
                .number_of_lines(1),
        )
}

fn chip(
    label: &str,
    count: String,
    selected: bool,
    on_press: impl Fn() + 'static,
) -> nsh_core::element::View {
    let (background, border, ink) = if selected {
        (
            "bg-[#1c1c1c] dark:bg-[#f0f0f0]",
            "border-transparent",
            "text-white dark:text-[#111111]",
        )
    } else {
        (
            "bg-white dark:bg-[#1d1d1d]",
            "border-black/10 dark:border-[#2b2b2b]",
            "text-[#5c5c5c] dark:text-[#a0a0a0]",
        )
    };
    view()
        .app_class(Some(
            format!("rounded-full px-2.5 py-1 border flex-row gap-1 {background} {border}").into(),
        ))
        .on_press(on_press)
        .child(text(label.to_owned()).app_class(Some(format!("text-[12px] {ink}").into())))
        .child(text(count).app_class(Some(format!("text-[12px] opacity-70 {ink}").into())))
}

fn pill(label: &str, tone: &str) -> nsh_core::element::View {
    let colors = match tone {
        "draft" => kit::PILL_DRAFT,
        "rev" => kit::PILL_REV,
        "ok" => "bg-[#dff3e8] dark:bg-[#123423] text-[#157a48] dark:text-[#3fcf89]",
        "in" => "bg-[#e1ebfb] dark:bg-[#15253f] text-[#2f6fd6] dark:text-[#7fb2ff]",
        _ => kit::PILL_MUTE,
    };
    view()
        .app_class(Some(
            format!("rounded-full px-2 py-0.5 self-start {colors}").into(),
        ))
        .child(
            text(label.to_owned())
                .app_class(Some(format!("text-[11px] {colors}").into()))
                .number_of_lines(1),
        )
}

fn kv(key: &str, value: &str) -> nsh_core::element::View {
    view()
        .app_class(Some("flex-row gap-2.5".into()))
        .child(text(key.to_owned()).app_class(Some(
            format!("w-[96px] text-[12.5px] {}", kit::C_DIM).into(),
        )))
        .child(
            text(value.to_owned())
                .app_class(Some(format!("flex-1 text-[12.5px] {}", kit::C_INK).into()))
                .number_of_lines(2),
        )
}

fn has_action(data: &Value, action: &str) -> bool {
    data.get("actions")
        .and_then(Value::as_array)
        .is_some_and(|actions| actions.iter().any(|value| value.as_str() == Some(action)))
}

fn value_string(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Null => String::new(),
        _ => value.to_string(),
    }
}

fn text_value<'a>(value: Option<&'a Value>, fallback: &'a str) -> &'a str {
    value.and_then(Value::as_str).unwrap_or(fallback)
}

fn text_value_or<'a>(value: Option<&'a Value>, fallback: &'a str) -> &'a str {
    let value = text_value(value, fallback);
    if value.is_empty() { fallback } else { value }
}
