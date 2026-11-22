#!/usr/bin/env python3
"""Write fixtures/demo/old-app-snapshot-seed.json: the old web app's E2E
snapshot (buwiz-books' tests/e2e/fixtures/snapshot-data.ts) expressed as a
list of Books tool calls, replayable with `books-mini-app seed`.

Source of truth: fixtures/demo/old-app-snapshot.json, a dump of the old app's
snapshot tables with UUIDs un-dashed (the privacy scan reads dash-separated
digit runs as TIN-shaped; undashed hex cannot trip it, and the ids are join
keys only). Regenerate from the web repo with:

    bun -e 'import {SNAPSHOT_DATA} from "./tests/e2e/fixtures/snapshot-data";
    console.log(JSON.stringify(SNAPSHOT_DATA("ORG","USER")))' > old-app-snapshot.json

then strip org/user columns if present).

Mapping:
- book: one accrual-basis book (the old app posts accrual journals and has
  AR/AP accounts).
- accounts: the PH preset already carries 34 of the 80 numbers; the other 46
  are created with their closest kind, or `unsure` + the old account_type for
  subtypes the catalogue does not name (group headers, clearing, insurance).
- parties -> persons (customer/vendor/employee/bank/shareholder/government
  -> customers/vendors/employees/lenders/owners/government).
- dimensions -> tags (department/location). The snapshot has a duplicate
  "Olongapo" location; Books keeps one tag per (kind, name).
- journal_headers -> posted entries: pay_in -> money_in, pay_out -> money_out
  (multi-line debits become `allocations`), transfer -> transfer,
  journal -> journal. Line party/department/location come through as
  payee/department/location names (Books resolves names to people/tags).
- reconciliations + statement_lines -> one statement import per
  (bank account, month), CSV built from the lines, then statement_accept.
- documents -> evidence: statement PDFs attach through the statement import's
  document_upload; receipts/bills attach unlinked (their link targets are the
  old app's bills table, which has no Books counterpart). The one CSV
  document cannot be evidence (PDF/PNG/JPEG/WebP only) and is skipped.

Every name, amount and memo is synthetic, copied verbatim from the snapshot.
Amounts stay two-place decimal text; no floats are used.

Run from the repo root: python3 tools/demo/gen_snapshot_parity.py
"""
import base64
import csv
import io
import json
from decimal import Decimal

SNAPSHOT = "fixtures/demo/old-app-snapshot.json"
OUT = "fixtures/demo/old-app-snapshot-seed.json"

steps = []
n = 0

def rid(tag):
    global n
    n += 1
    return f"snap-{n:04d}-{tag}"[:64]

def step(tool, args):
    steps.append({"tool": tool, "args": dict(args, client_request_id=rid(tool.rsplit("_", 1)[-1]))})

data = json.load(open(SNAPSHOT))
accounts = {a["id"]: a for a in data["accounts"]}
parties = {p["id"]: p for p in data["parties"]}
dims = {d["id"]: d for d in data["dimensions"]}
fins = {f["id"]: f for f in data["financial_accounts"]}
heads = {h["id"]: h for h in data["journal_headers"]}
docs = {d["id"]: d for d in data["documents"]}
recs = {r["id"]: r for r in data["reconciliations"]}
lines_by_head = {}
for l in data["journal_lines"]:
    lines_by_head.setdefault(l["journal_header_id"], []).append(l)
for ls in lines_by_head.values():
    ls.sort(key=lambda l: l["sort_order"])

# old web subtype -> Books catalogue kind (unset means `unsure` + type)
SUBTYPE_KIND = {
    "bank_accounts": "bank_accounts",
    "account_receivable": "accounts_receivable_gross",
    "accounts_payable": "accounts_payable",
    "prepaid_expenses": "prepaid_expenses",
    "credit_cards": "loan_payable",  # Books' card accounts; statements need it
    "owners_equity": "owners_equity",
    "sales_revenue": "sales_revenue",
    "subscription_revenue": "subscription_revenue",
    "transaction_revenue": "transaction_revenue",
    "cost_of_goods": "cost_of_goods_sold",
    "hosting_fees": "hosting_fees",
    "payment_processing_fees": "payment_processing_fees",
    "rent": "rent",
    "interests_dividends": "interest_income",
    "interest_expense": "interest_expense",
    "bank_fees": "bank_fees",
    "business_application_software": "business_applications_and_software",
    "credit_card_rewards": "other_non_operating_income",
}
# the PH preset's numbers: a book starts with these already
PRESET = {a["number"] for a in json.load(open("data/presets/ph-small-business.v1.json"))["accounts"]}
CASH_KINDS = {"bank_accounts"}
MONEY_SUBTYPES = {"bank_accounts", "credit_cards"}

PARTY_KIND = {
    "customer": "customers",
    "vendor": "vendors",
    "employee": "employees",
    "bank": "lenders",
    "shareholder": "owners",
    "government": "government",
}
DOC_KIND = {"statement": "statement", "bill": "bill", "invoice": "invoice",
            "receipt": "receipt", "payslip": "payslip", "contract": "contract",
            "tax_form": "tax_form"}

def money(v):
    return str(Decimal(v).quantize(Decimal("0.01"))) if v is not None else None

# --- book -----------------------------------------------------------------
step("books_app_book_create", {"label": "Buwiz Platform", "basis": "accrual"})

# --- accounts: create the numbers the preset does not carry ---------------
for a in sorted(data["accounts"], key=lambda a: a["account_number"]):
    if a["account_number"] in PRESET:
        continue
    kind = SUBTYPE_KIND.get(a["subtype"])
    args = {"number": a["account_number"], "name": a["name"]}
    if kind:
        args["kind"] = kind
    else:
        args["kind"] = "unsure"
        args["type"] = a["account_type"]
    step("books_app_account_create", args)

# --- persons ---------------------------------------------------------------
for p in data["parties"]:
    step("books_app_person_create", {"kind": PARTY_KIND[p["party_type"]], "name": p["name"]})

# --- tags ------------------------------------------------------------------
seen_tags = set()
for d in data["dimensions"]:
    key = (d["dimension_type"], d["name"])
    if key in seen_tags:
        continue  # Books keeps one tag per (kind, name)
    seen_tags.add(key)
    step("books_app_tag_create", {"kind": d["dimension_type"], "name": d["name"]})

# --- entries ---------------------------------------------------------------
def person_args(h):
    out = {}
    party = parties.get(h.get("party_id"))
    if party:
        out["payee"] = party["name"]
        out["person_kind"] = PARTY_KIND[party["party_type"]]
    return out

def dim_args(l):
    out = {}
    dep = dims.get(l.get("department_id"))
    loc = dims.get(l.get("location_id"))
    if dep:
        out["department"] = dep["name"]
    if loc:
        out["location"] = loc["name"]
    return out

def is_money(l):
    return accounts[l["account_id"]]["subtype"] in MONEY_SUBTYPES

for h in sorted(heads.values(), key=lambda h: h["transaction_number"]):
    lines = lines_by_head[h["id"]]
    t = h["transaction_type"]
    date = h["transaction_date"][:10]
    base = {"date": date}
    if h.get("memo"):
        base["memo"] = h["memo"]
    if h.get("reference_number"):
        base["reference"] = h["reference_number"]
    base.update(person_args(h))
    if t == "journal":
        jl = []
        for l in lines:
            row = {"account": accounts[l["account_id"]]["account_number"]}
            if l.get("debit"):
                row["debit"] = money(l["debit"])
            if l.get("credit"):
                row["credit"] = money(l["credit"])
            if l.get("line_description"):
                row["memo"] = l["line_description"]
            row.update(dim_args(l))
            jl.append(row)
        step("books_app_post_entry", {**base, "form": "journal", "lines": jl})
        continue
    if t == "transfer":
        src = next(l for l in lines if l.get("credit"))
        dst = next(l for l in lines if l.get("debit"))
        step("books_app_post_entry", {**base, "form": "transfer",
                                    "account": accounts[src["account_id"]]["account_number"],
                                    "to_account": accounts[dst["account_id"]]["account_number"],
                                    "amount": money(dst["debit"])})
        continue
    money_line = next(l for l in lines if is_money(l) and (l.get("credit") if t == "pay_out" else l.get("debit")))
    others = [l for l in lines if l is not money_line]
    args = {**base,
            "form": "money_in" if t == "pay_in" else "money_out",
            "account": accounts[money_line["account_id"]]["account_number"],
            "amount": money(h["total_amount"])}
    if len(others) == 1:
        args["category"] = accounts[others[0]["account_id"]]["account_number"]
        args.update(dim_args(others[0]))
    else:
        args["allocations"] = [
            {**{"category": accounts[l["account_id"]]["account_number"],
                "amount": money(l.get("debit") or l.get("credit"))},
             **({"memo": l["line_description"]} if l.get("line_description") else {}),
             **dim_args(l)}
            for l in others]
    step("books_app_post_entry", args)

# --- statement documents: upload, keep with the import --------------------
def tiny_pdf(label):
    body = f"BT /F1 10 Tf 40 720 Td ({label}) Tj ET".encode()
    # minimal valid single-page PDF
    objs = [b"<</Type/Catalog/Pages 2 0 R>>",
            b"<</Type/Pages/Kids[3 0 R]/Count 1>>",
            b"<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]/Contents 4 0 R/Resources<</Font<</F1 5 0 R>>>>>>",
            b"<</Length " + str(len(body)).encode() + b">>\nstream\n" + body + b"\nendstream",
            b"<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>"]
    out = b"%PDF-1.4\n"
    offs = []
    for i, o in enumerate(objs, 1):
        offs.append(len(out))
        out += f"{i} 0 obj\n".encode() + o + b"\nendobj\n"
    xref = len(out)
    out += f"xref\n0 {len(objs)+1}\n0000000000 65535 f \n".encode()
    for o in offs:
        out += f"{o:010d} 00000 n \n".encode()
    out += f"trailer<</Size {len(objs)+1}/Root 1 0 R>>\nstartxref\n{xref}\n%%EOF".encode()
    return out

PNG_1PX = base64.b64decode(
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==")

def doc_blob(d):
    """Bytes that pass the MIME check and hash differently per document,
    so evidence dedupe keeps every snapshot document."""
    if d.get("mime_type") == "application/pdf":
        return tiny_pdf(f"{d['original_filename']}#{d['id'][:8]}")
    return PNG_1PX + f" {d['id']}".encode()

def upload(upload_id, blob):
    step("books_app_upload_part", {"upload": upload_id, "part": 0, "parts": 1,
                                   "data_base64": base64.urlsafe_b64encode(blob).decode().rstrip("=")})

# statement docs -> their reconciliation's account, via document_upload
doc_by_rec = {}
for a in data["document_attachments"]:
    if a["linkable_type"] == "reconciliation":
        doc_by_rec[a["linkable_id"]] = docs[a["document_id"]]

for rid_, r in sorted(recs.items(), key=lambda kv: kv[1]["period_start"]):
    acct_no = accounts[fins[r["bank_account_id"]]["ledger_account_id"]]["account_number"]
    doc = doc_by_rec.get(rid_)
    if doc:
        up = f"stmt-{rid_[:8]}"
        upload(up, doc_blob(doc))
        step("books_app_statement_import", {"account": acct_no, "document_upload": up,
                                          "document_mime": "application/pdf"})

# --- statements: one import per (account, month) ---------------------------
slines = sorted(data["statement_lines"], key=lambda l: (l["reconciliation_id"], l["sort_order"]))
months = {}
for l in slines:
    r = recs[l["reconciliation_id"]]
    acct_no = accounts[fins[r["bank_account_id"]]["ledger_account_id"]]["account_number"]
    months.setdefault((acct_no, l["transaction_date"][:7]), []).append(l)

for (acct_no, month), ls in sorted(months.items()):
    # closing = this account's first opening plus every line through the month
    acct_lines = sorted(
        (l for l in slines
         if accounts[fins[recs[l["reconciliation_id"]]["bank_account_id"]]
                    ["ledger_account_id"]]["account_number"] == acct_no),
        key=lambda l: l["transaction_date"])
    first_rec = recs[acct_lines[0]["reconciliation_id"]]
    closing = Decimal(first_rec["statement_beginning_balance"]) + sum(
        Decimal(l["amount"]) for l in acct_lines
        if l["transaction_date"][:7] <= month)
    buf = io.StringIO()
    w = csv.writer(buf)
    w.writerow(["Date", "Description", "Amount"])
    for l in ls:
        w.writerow([l["transaction_date"][:10], l["description"] or "", money(l["amount"])])
    step("books_app_statement_import", {"account": acct_no, "csv_text": buf.getvalue(),
                                        "closing": str(closing.quantize(Decimal("0.01"))),
                                        "file_name": f"{acct_no}-{month}.csv"})
    step("books_app_statement_accept", {"account": acct_no, "month": month})

# --- other documents: unlinked evidence ------------------------------------
attached_ids = {doc["id"] for doc in doc_by_rec.values()}
for d in data["documents"]:
    if d["id"] in attached_ids:
        continue
    mime = d.get("mime_type") or ""
    if mime not in ("application/pdf", "image/png", "image/jpeg", "image/webp"):
        continue  # the CSV: Books evidence takes PDF/PNG/JPEG/WebP only
    blob = doc_blob(d)
    up = f"doc-{d['id'][:8]}"
    step("books_app_evidence_attach", {"op": "part", "upload": up, "part": 0, "parts": 1,
                                       "data_base64": base64.urlsafe_b64encode(blob).decode().rstrip("=")})
    args = {"op": "commit", "upload": up, "mime": mime,
            "kind": DOC_KIND.get(d["document_type"], "other"), "source": "user"}
    if d.get("page_count"):
        args["page_count"] = d["page_count"]
    step("books_app_evidence_attach", args)

json.dump({"steps": steps}, open(OUT, "w"), indent=1)
print(f"wrote {OUT}: {len(steps)} steps")
