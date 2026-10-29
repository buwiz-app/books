//! M5 and M6 through the tools, on synthetic books only.
//!
//! - M5-06: the three sample charts (web Books, a QuickBooks CSV, a
//!   spreadsheet CSV) give 2551Q Q3 2026 `txtATCAmt1` = 290,000.00 once
//!   the owner confirms the matches, OKs the return and closes Q3; before
//!   the matches are confirmed that box is held (M5-04).
//! - M5-05: a mid-year start counts its opening totals, with proof.
//! - M6-06..08: the Forms page reads payroll and received 2307s; a 2306
//!   and a pending 2307 never count; lineage is the app's only.

use books_mini_app::service::{Books, Origin};
use books_mini_app::tools;
use books_store::{Paths, TempDir};
use serde_json::{Map, Value, json};

fn sample(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/../../fixtures/import/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

fn run_as(books: &mut Books, origin: Origin, name: &str, args: Value) -> Value {
    let args: Map<String, Value> = args.as_object().unwrap().clone();
    tools::call(name, &args, origin, books).unwrap()
}

fn ok(books: &mut Books, name: &str, args: Value) -> Value {
    let out = run_as(books, Origin::App, name, args);
    assert_eq!(out["isError"], false, "{name}: {out}");
    out["structuredContent"].clone()
}

fn refused(books: &mut Books, name: &str, args: Value) -> String {
    let out = run_as(books, Origin::App, name, args);
    assert_eq!(out["isError"], true, "{name}: {out}");
    out["structuredContent"]["code"]
        .as_str()
        .unwrap()
        .to_string()
}

fn fresh(tag: &str) -> (TempDir, Books) {
    books_core::date::pin_today(books_core::date::Date::new(2026, 10, 29));
    let dir = TempDir::new(tag);
    let mut books = Books::new(Paths::in_dir(dir.0.join("store")));
    ok(
        &mut books,
        "books_app_book_create",
        json!({ "client_request_id": format!("{tag}-book-0001"), "label": "Isla Demo Trading" }),
    );
    (dir, books)
}

fn row<'a>(view: &'a Value, key: &str) -> &'a Value {
    view["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["key"] == key)
        .unwrap_or_else(|| panic!("no row {key}: {view}"))
}

fn close(books: &mut Books, months: &[&str]) {
    for m in months {
        // an old draft that came in with the import is the owner's to discard
        let ledger = ok(
            books,
            "books_ledger_list",
            json!({ "month": m, "filter": "all" }),
        );
        for (i, r) in ledger["rows"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["status"] == "draft")
            .enumerate()
        {
            ok(
                books,
                "books_app_discard_entry",
                json!({ "client_request_id": format!("discard-{m}-{i:04}"), "entry": r["ticket"] }),
            );
        }
        ok(
            books,
            "books_app_month_close",
            json!({ "client_request_id": format!("close-{m}-0001"), "month": m }),
        );
    }
}

fn import(books: &mut Books, which: &str) -> Value {
    let args = match which {
        "web" => {
            json!({ "client_request_id": "imp-web-00001", "source": "web", "web_text": sample("web-books-sample.json"), "file_name": "web-books-sample.json" })
        }
        "quickbooks" => {
            json!({ "client_request_id": "imp-qb-000001", "source": "csv", "chart_text": sample("quickbooks-chart.csv"), "journal_text": sample("quickbooks-journal.csv"), "file_name": "quickbooks-journal.csv" })
        }
        _ => {
            json!({ "client_request_id": "imp-ss-000001", "source": "csv", "chart_text": sample("spreadsheet-chart.csv"), "journal_text": sample("spreadsheet-journal.csv"), "file_name": "spreadsheet-journal.csv" })
        }
    };
    ok(books, "books_app_import", args)
}

#[test]
fn m5_06_three_sample_charts_give_the_same_2551q_box() {
    for which in ["web", "quickbooks", "spreadsheet"] {
        let (_dir, mut books) = fresh(&format!("m5-{which}"));
        let b = &mut books;
        let summary = import(b, which);
        assert!(
            summary["posted"].as_i64().unwrap() >= 5,
            "{which}: {summary}"
        );

        // matches waiting: the sales box is held until the owner confirms
        let status = ok(b, "books_import_get", json!({}));
        assert!(
            status["counts"]["to_confirm"].as_i64().unwrap() > 0,
            "{which}: {status}"
        );
        assert_eq!(status["held"], true);
        close(b, &["2026-07", "2026-08", "2026-09"]);
        let before = ok(
            b,
            "books_form_proposal",
            json!({ "form": "2551Q", "period": "2026-Q3" }),
        );
        let sales = row(&before, "txtATCAmt1");
        assert_eq!(
            (sales["held"].clone(), sales["value"].clone()),
            (json!(true), Value::Null),
            "{which}: {before}"
        );

        // the owner confirms every suggestion, then OKs the September return
        ok(
            b,
            "books_app_accounts_confirm",
            json!({ "client_request_id": format!("conf-{which}-0001"), "min": 50 }),
        );
        let status = ok(b, "books_import_get", json!({}));
        assert_eq!(status["counts"]["to_confirm"], 0, "{which}: {status}");
        let mid = ok(
            b,
            "books_form_proposal",
            json!({ "form": "2551Q", "period": "2026-Q3" }),
        );
        let waiting: Vec<&Value> = mid["blocking"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|i| i["code"] == "GATE.RETURNS_ACK")
            .collect();
        assert_eq!(
            waiting.len(),
            1,
            "{which}: one return waits for the owner's OK: {mid}"
        );
        for entry in mid["returns"].as_array().unwrap() {
            ok(
                b,
                "books_app_return_ok",
                json!({ "client_request_id": format!("ack-{which}-0001"), "entry": entry["ticket"] }),
            );
        }
        let after = ok(
            b,
            "books_form_proposal",
            json!({ "form": "2551Q", "period": "2026-Q3" }),
        );
        assert_eq!(after["status"], "ready", "{which}: {after}");
        assert_eq!(
            after["fill_args"]["txtATCAmt1"], "290000.00",
            "{which}: {after}"
        );
        assert_eq!(row(&after, "txtATCAmt1")["value_shown"], "290,000.00");
        assert_eq!(
            row(&after, "txtATCAmt1")["sub"],
            "Sales 300,000.00 − returns 10,000.00"
        );

        // lineage is the app's: four lines, the return negative
        let lin = ok(
            b,
            "books_app_form_lineage",
            json!({ "form": "2551Q", "period": "2026-Q3", "key": "txtATCAmt1" }),
        );
        assert_eq!(lin["rows"].as_array().unwrap().len(), 4, "{which}: {lin}");
        assert_eq!(lin["total_shown"], "290,000.00");
        // the assistant never sees lines (D11): the broker hides the tool from it
        assert_eq!(
            tools::find("books_app_form_lineage").unwrap().reach,
            tools::Reach::App
        );

        // a second import is refused: the book has posted entries now
        let again = match which {
            "web" => {
                json!({ "client_request_id": "imp-web-00002", "source": "web", "web_text": sample("web-books-sample.json") })
            }
            _ => {
                json!({ "client_request_id": "imp-csv-00002", "source": "csv", "chart_text": sample("spreadsheet-chart.csv"), "journal_text": sample("spreadsheet-journal.csv") })
            }
        };
        assert_eq!(refused(b, "books_app_import", again), "book_not_empty");
    }
}

#[test]
fn m5_files_arrive_in_parts() {
    let (_dir, mut books) = fresh("m5-parts");
    let b = &mut books;
    let web = sample("web-books-sample.json").into_bytes();
    let size = web.len().div_ceil(3);
    let parts: Vec<&[u8]> = web.chunks(size).collect();
    for (i, p) in parts.iter().enumerate() {
        let r = ok(
            b,
            "books_app_upload_part",
            json!({ "upload": "web-upload-01", "part": i, "parts": parts.len(), "data_base64": books_mini_app::b64::encode(p) }),
        );
        assert_eq!(r["received"], i + 1);
    }
    let s = ok(
        b,
        "books_app_import",
        json!({ "client_request_id": "imp-parts-0001", "source": "web", "web_upload": "web-upload-01" }),
    );
    assert!(s["posted"].as_i64().unwrap() > 0, "{s}");
    // the parts were taken; a missing upload is plain about it
    assert_eq!(
        refused(
            b,
            "books_app_import",
            json!({ "client_request_id": "imp-parts-0002", "source": "web", "web_upload": "web-upload-01" })
        ),
        "not_found"
    );
}

#[test]
fn m5_05_a_mid_year_start_counts_its_opening_totals() {
    let (_dir, mut books) = fresh("m5-open");
    let b = &mut books;
    import(b, "spreadsheet");
    ok(
        b,
        "books_app_accounts_confirm",
        json!({ "client_request_id": "open-conf-0001", "min": 50 }),
    );
    // no proof, no opening total
    let no_proof = refused(
        b,
        "books_app_opening_set",
        json!({ "client_request_id": "open-np-0001", "measure": "gross_sales", "from": "2026-04-01", "to": "2026-06-30", "amount": "250000.00" }),
    );
    assert_eq!(no_proof, "proof_needed");
    // an opening total may not overlap the book's own months
    let overlap = refused(
        b,
        "books_app_opening_set",
        json!({ "client_request_id": "open-ov-0001", "measure": "gross_sales", "from": "2026-04-01", "to": "2026-07-31", "amount": "1.00", "proof": "x" }),
    );
    assert_eq!(overlap, "opening_overlaps");
    ok(
        b,
        "books_app_opening_set",
        json!({ "client_request_id": "open-q2-0001", "measure": "gross_sales", "from": "2026-04-01", "to": "2026-06-30", "amount": "250000.00", "proof": "Filed 2551Q for Q2 and the bank statements" }),
    );
    let q2 = ok(
        b,
        "books_form_proposal",
        json!({ "form": "2551Q", "period": "2026-Q2" }),
    );
    assert_eq!(
        q2["status"], "ready",
        "the months an opening covers are fixed: {q2}"
    );
    assert_eq!(q2["fill_args"]["txtATCAmt1"], "250000.00", "{q2}");
    let lin = ok(
        b,
        "books_app_form_lineage",
        json!({ "form": "2551Q", "period": "2026-Q2", "key": "txtATCAmt1" }),
    );
    assert_eq!(lin["rows"][0]["date"], "Opening", "{lin}");
    // the assistant sees the total, not the proof text
    let agent = run_as(b, Origin::Agent, "books_import_get", json!({}));
    let opening = &agent["structuredContent"]["openings"][0];
    assert_eq!(
        (opening["amount_shown"].as_str(), opening.get("proof")),
        (Some("250,000.00"), None)
    );
}

fn person(books: &mut Books, kind: &str, name: &str, n: u32) -> String {
    let r = ok(
        books,
        "books_app_person_create",
        json!({ "client_request_id": format!("p-{kind}-{n:04}"), "kind": kind, "name": name }),
    );
    r["person"]
        .as_str()
        .unwrap_or_else(|| panic!("{r}"))
        .to_string()
}

#[test]
fn m6_payroll_and_certificates_feed_the_forms_page() {
    let (_dir, mut books) = fresh("m6-pay");
    let b = &mut books;
    let ana = person(b, "employees", "Ana Cruz", 1);
    let santos = person(b, "customers", "Santos Hardware", 2);
    let bayani = person(b, "customers", "Bayani Foods", 3);
    let isla = person(b, "customers", "Isla Café", 4);
    // a September sale so the quarter has business
    ok(
        b,
        "books_app_post_entry",
        json!({ "client_request_id": "sale-0001", "form": "money_in", "date": "2026-09-05", "amount": "100000.00", "account": "11000", "category": "41000", "payee": "Santos Hardware" }),
    );

    // a pay run: amounts as payroll recorded them; Books computes nothing
    let run = ok(
        b,
        "books_app_wage_run_save",
        json!({
            "client_request_id": "pay-sep-0001", "period_from": "2026-09-01", "period_to": "2026-09-30", "account": "11000",
            "employees": [{ "person": ana, "basic": "40000.00", "de_minimis": "2000.00", "ee_sss": "2000.00", "er_sss": "1500.00", "tax_withheld": "2808.40" }]
        }),
    );
    assert_eq!(
        (run["status"].as_str(), run["net_shown"].as_str()),
        (Some("draft"), Some("37,191.60"))
    );
    let list = ok(b, "books_wages_list", json!({}));
    assert_eq!(list["runs"][0]["bir"], "Waiting for this run", "{list}");
    let entry = list["runs"][0]["ticket"].as_str().unwrap().to_string();
    ok(
        b,
        "books_app_post_entry",
        json!({ "client_request_id": "pay-post-0001", "entry": entry }),
    );

    // certificates: a received 2307 counts; a pending one and a 2306 never do
    for (n, who, form, status, amount, atc) in [
        (1, &santos, "2307", "received", "3000.00", "WB082"),
        (2, &bayani, "2307", "pending", "2000.00", "WB082"),
        (3, &isla, "2306", "received", "1500.00", "WB084"),
    ] {
        ok(
            b,
            "books_app_certificate_add",
            json!({ "client_request_id": format!("cert-{n:04}"), "form": form, "credit_kind": "pt", "atc": atc, "quarter": "2026-Q3", "amount": amount, "status": status, "person": who, "date": "2026-09-30" }),
        );
    }
    let certs = ok(b, "books_certificates_list", json!({}));
    let counts: Vec<&str> = certs["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["counts"].as_str().unwrap())
        .collect();
    assert!(
        counts.contains(&"2551Q item 15")
            && counts.contains(&"Not counted until received")
            && counts.contains(&"Never counted: final"),
        "{certs}"
    );
    assert_eq!(
        refused(
            b,
            "books_app_certificate_add",
            json!({ "client_request_id": "cert-bad-0001", "form": "2307", "credit_kind": "pt", "atc": "XX999", "quarter": "2026-Q3", "amount": "1.00", "person": santos })
        ),
        "invalid_args"
    );

    close(b, &["2026-07", "2026-08", "2026-09"]);
    let pay = ok(b, "books_wages_list", json!({}));
    assert_eq!(pay["runs"][0]["bir"], "1601-C Sep ready", "{pay}");

    let c = ok(
        b,
        "books_form_proposal",
        json!({ "form": "1601C", "period": "2026-09" }),
    );
    // item 16 fails closed until its tax review: the drawing's "Ready · 1 held"
    assert_eq!(
        (c["status"].as_str(), c["status_label"].as_str()),
        (Some("ready_held"), Some("Ready · 1 held")),
        "{c}"
    );
    for (key, value) in [
        ("frm1601c:TaxWithheld", "1"),
        ("frm1601c:txtTax14", "42000.00"),
        ("frm1601c:txtTax18", "2000.00"),
        ("frm1601c:txtTax19", "2000.00"),
        ("frm1601c:txtTax25", "2808.40"),
    ] {
        assert_eq!(c["fill_args"][key], value, "{key}: {c}");
    }
    assert_eq!(row(&c, "frm1601c:TaxWithheld")["value_shown"], "Yes");

    let q = ok(
        b,
        "books_form_proposal",
        json!({ "form": "2551Q", "period": "2026-Q3" }),
    );
    assert_eq!(
        q["fill_args"]["frm2551Qv2018:txt15"], "3000.00",
        "only the received 2307: {q}"
    );
    // the paper for the pending one arrives: now it counts
    let pending = certs["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["status"] == "pending")
        .unwrap()["ticket"]
        .clone();
    ok(
        b,
        "books_app_certificate_set",
        json!({ "client_request_id": "cert-set-0001", "certificate": pending, "status": "received" }),
    );
    let q = ok(
        b,
        "books_form_proposal",
        json!({ "form": "2551Q", "period": "2026-Q3" }),
    );
    assert_eq!(q["fill_args"]["frm2551Qv2018:txt15"], "5000.00", "{q}");

    // the three cards, and who fills the boxes
    let forms = ok(b, "books_forms", json!({}));
    let codes: Vec<&str> = forms["cards"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["code"].as_str().unwrap())
        .collect();
    assert_eq!(codes, ["2551Q", "1601-C", "1701-MS"]);
    assert_eq!(
        q["owners"],
        json!({ "books": 2, "bir": 26, "profile": 9, "past": 1, "you": 33 })
    );
    // 1701-MS reads the tax option from BIR; Books never sets it
    let ms = ok(
        b,
        "books_form_proposal",
        json!({ "form": "1701MS", "period": "2026" }),
    );
    assert!(
        ms["blocking"].as_array().unwrap().iter().any(|i| i["code"]
            .as_str()
            .unwrap()
            .contains("FORM_CONTEXT")
            || i["detail"].as_str().unwrap().contains("tax_option")),
        "{ms}"
    );
    let eight = ok(
        b,
        "books_form_proposal",
        json!({ "form": "1701MS", "period": "2026", "form_context": { "/taxpayer/tax_option": "eight_percent" } }),
    );
    let keys: Vec<&str> = eight["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["key"].as_str().unwrap())
        .collect();
    assert!(
        keys.contains(&"frm1701MS:txtSalesRevenuesFeesANo19a")
            && !keys.contains(&"frm1701MS:txtSalesRevenueNo5a"),
        "{keys:?}"
    );
}
