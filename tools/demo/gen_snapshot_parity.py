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
- reconciliations + statement_lines -> one statement read per
  (bank account, month) through the draft path (books_statement_draft_lines
  + books_app_statement_accept), with each line's whole-row box on its page.
- documents -> evidence: statement PDFs are generated here at known
  coordinates and attach through the read's document_upload (each
  reconciliation's PDF is the same bytes for every month it spans, so
  evidence dedupe keeps one document); receipts/bills attach unlinked (their
  link targets are the old app's bills table, which has no Books
  counterpart). The one CSV document cannot be evidence (PDF/PNG/JPEG/WebP
  only) and is skipped.

Every name, amount and memo is synthetic, copied verbatim from the snapshot.
Amounts stay two-place decimal text; no floats are used.

Run from the repo root: python3 tools/demo/gen_snapshot_parity.py
"""
import base64
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

# --- statement documents: generated bank statements ------------------------
# Every statement PDF is drawn at known coordinates on 612x792 letter pages,
# so each row's whole-row box and the title/period areas are exact. Boxes are
# [x0, y0, x1, y1] in thousandths (0-1000) of the page, top-left origin; PDF
# points are bottom-left, 1 pt = 1000/612 (x) or 1000/792 (y) thousandths.
# 16 rows per page, as the snapshot's own ocr_bbox rows imply (page 0 holds
# sort_order 0-15, page 1 the rest).

PAGE_W, PAGE_H = 612.0, 792.0
MARG_L, MARG_R = 54.0, 558.0
ROW_PITCH, ROW_TOP_0, ROW_TOP_N, ROWS_PER_PAGE = 18.0, 112.0, 64.0, 16

def kx(x):
    return round(x / PAGE_W * 1000)

def ky(y):
    return round(y / PAGE_H * 1000)

def bbox(x0, y0, x1, y1):
    return [kx(x0), ky(y0), kx(x1), ky(y1)]

# PDF literal strings here are ASCII: hayro's base-14 fonts render no WinAnsi
# punctuation, so dashes are drawn as hyphens (the area labels keep the real
# characters) and anything else non-ASCII becomes '?'.
def pdf_escape(s):
    out = []
    for ch in s:
        if ch in "\\()":
            out.append("\\" + ch)
        elif ord(ch) < 128:
            out.append(ch)
        elif ch in "–—":
            out.append("-")
        else:
            out.append("?")
    return "".join(out)

def pdf_text(x, y_top, size, text, font="F1"):
    return f"BT /{font} {size} Tf {x:.1f} {PAGE_H - y_top:.1f} Td ({pdf_escape(text)}) Tj ET\n"

def pdf_rule(y_top, weight=0.8, gray=0.85):
    return f"{gray} g {MARG_L:.1f} {PAGE_H - y_top:.1f} {MARG_R - MARG_L:.1f} {weight} re f 0 g\n"

# Helvetica's own widths, in ems, for right-aligning the amount column.
HELV = {ch: .556 for ch in "0123456789"}
HELV.update({".": .278, ",": .278, "-": .333, " ": .278})

def text_w(s, size):
    return sum(HELV.get(ch, .5) for ch in s) * size

MON = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]

def period_text(r):
    a, b = r["period_start"][:10].split("-"), r["period_end"][:10].split("-")
    if a[0] == b[0]:
        return f"Statement Period: {MON[int(a[1]) - 1]} {int(a[2])} – {MON[int(b[1]) - 1]} {int(b[2])}, {b[0]}"
    return f"Statement Period: {MON[int(a[1]) - 1]} {int(a[2])}, {a[0]} – {MON[int(b[1]) - 1]} {int(b[2])}, {b[0]}"

def statement_pdf(title, period, rows, closing):
    """One bank statement. rows = [(date, description, amount)] in page
    order; returns (pdf bytes, row_boxes, areas) where row_boxes[i] is
    (page, bbox) for rows[i] and areas are the title/period boxes."""
    if len(rows) > 96:
        raise SystemExit("statement_pdf: more than 96 rows")
    chunks = [rows[i:i + ROWS_PER_PAGE] for i in range(0, len(rows), ROWS_PER_PAGE)] or [[]]
    page_ops = []
    row_boxes = []
    for pi, chunk in enumerate(chunks):
        ops = []
        if pi == 0:
            ops.append(pdf_text(MARG_L, 42, 14, title, "F2"))
            ops.append(pdf_text(MARG_L, 60, 9, period))
            ops.append(pdf_rule(78))
            header_y, rows_y = 94, ROW_TOP_0
        else:
            header_y, rows_y = 44, ROW_TOP_N
        ops.append(pdf_text(MARG_L, header_y, 9, "Date", "F2"))
        ops.append(pdf_text(150, header_y, 9, "Description", "F2"))
        ops.append(pdf_text(MARG_R - text_w("Amount", 9), header_y, 9, "Amount", "F2"))
        ops.append(pdf_rule(header_y + 10, 0.5, 0.9))
        top = rows_y
        for date, desc, amount in chunk:
            ops.append(pdf_text(MARG_L, top + 11, 9, date))
            ops.append(pdf_text(150, top + 11, 9, desc[:52]))
            ops.append(pdf_text(MARG_R - text_w(amount, 9), top + 11, 9, amount))
            row_boxes.append((pi + 1, bbox(MARG_L, top, MARG_R, top + 16)))
            top += ROW_PITCH
        if pi == len(chunks) - 1:
            ops.append(pdf_rule(top + 4))
            ops.append(pdf_text(MARG_L, top + 18, 10, "Closing Balance", "F2"))
            ops.append(pdf_text(MARG_R - text_w(closing, 10), top + 18, 10, closing, "F2"))
        page_ops.append("".join(ops).encode())
    objs = [b"<</Type/Catalog/Pages 2 0 R>>"]
    kids = " ".join(f"{3 + 2 * i} 0 R" for i in range(len(page_ops)))
    objs.append(f"<</Type/Pages/Kids[{kids}]/Count {len(page_ops)}>>".encode())
    for i, body in enumerate(page_ops):
        page_no, cont_no = 3 + 2 * i, 4 + 2 * i
        objs.append(f"<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]/Contents {cont_no} 0 R/Resources<</Font<</F1 {3 + 2 * len(page_ops)} 0 R/F2 {4 + 2 * len(page_ops)} 0 R>>>>>>".encode())
        objs.append(b"<</Length " + str(len(body)).encode() + b">>\nstream\n" + body + b"\nendstream")
    objs.append(b"<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>")
    objs.append(b"<</Type/Font/Subtype/Type1/BaseFont/Helvetica-Bold>>")
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
    areas = [
        {"kind": "title", "label": title, "page": 1,
         "bbox": bbox(MARG_L, 26, MARG_R, 50)},
        {"kind": "date", "label": period, "page": 1,
         "bbox": bbox(MARG_L, 52, MARG_L + text_w(period, 9) + 8, 68)},
    ]
    return out, row_boxes, areas

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

# --- statements: one read per (account, month) -----------------------------
# Each read goes through the statement-read path (draft lines + the person's
# accept) so every line keeps its whole-row box and the statement document
# keeps its title/period areas. The document is the reconciliation's whole
# statement, so a reconciliation spanning two months uploads the same PDF
# twice under different upload ids; evidence dedupe keeps one document, and
# each month's statement points at it with its own rows boxed.
slines = sorted(data["statement_lines"], key=lambda l: (l["reconciliation_id"], l["sort_order"]))
months = {}
for l in slines:
    r = recs[l["reconciliation_id"]]
    acct_no = accounts[fins[r["bank_account_id"]]["ledger_account_id"]]["account_number"]
    months.setdefault((acct_no, l["transaction_date"][:7]), []).append(l)

# one generated statement document per reconciliation that has lines
pdf_by_rec = {}
for rid_, r in sorted(recs.items(), key=lambda kv: kv[1]["period_start"]):
    rec_lines = [l for l in slines if l["reconciliation_id"] == rid_]
    fin = fins[r["bank_account_id"]]
    if rec_lines:
        run = Decimal(r["statement_beginning_balance"])
        drawn = []
        for l in rec_lines:
            run += Decimal(l["amount"])
            drawn.append((l, l["transaction_date"][:10], l["description"] or "", money(l["amount"])))
        closing = run
        if accounts[fin["ledger_account_id"]]["account_type"] == "liability":
            closing = -closing
        pdf_by_rec[rid_] = statement_pdf(
            f"{fin['account_name']} — Monthly Statement",
            period_text(r),
            [(d, t, a) for _, d, t, a in drawn],
            str(closing.quantize(Decimal("0.01"))))
    elif doc_by_rec.get(rid_):
        # a statement document with no lines: evidence only, no read
        acct_no = accounts[fin["ledger_account_id"]]["account_number"]
        up = f"stmt-{rid_[:8]}"
        upload(up, doc_blob(doc_by_rec[rid_]))
        step("books_app_statement_import", {"account": acct_no, "document_upload": up,
                                          "document_mime": "application/pdf"})

for (acct_no, month), ls in sorted(months.items()):
    # closing = this account's first opening plus every line through the month
    acct_lines = sorted(
        (l for l in slines
         if accounts[fins[recs[l["reconciliation_id"]]["bank_account_id"]]
                    ["ledger_account_id"]]["account_number"] == acct_no),
        key=lambda l: l["transaction_date"])
    first_rec = recs[acct_lines[0]["reconciliation_id"]]
    account = accounts[fins[first_rec["bank_account_id"]]["ledger_account_id"]]
    raw_closing = Decimal(first_rec["statement_beginning_balance"]) + sum(
        Decimal(l["amount"]) for l in acct_lines
        if l["transaction_date"][:7] <= month)
    closing = -raw_closing if account["account_type"] == "liability" else raw_closing
    rid_ = ls[0]["reconciliation_id"]
    pdf, row_boxes, areas = pdf_by_rec[rid_]
    rec_lines = [l for l in slines if l["reconciliation_id"] == rid_]
    order = {id(l): i for i, l in enumerate(rec_lines)}
    # each line's balance is its running raw sum, so balance_breaks is 0
    run = raw_closing - sum(Decimal(l["amount"]) for l in ls)
    lines_arg = []
    for l in ls:
        run += Decimal(l["amount"])
        page, box = row_boxes[order[id(l)]]
        lines_arg.append({"date": l["transaction_date"][:10],
                          "description": l["description"] or "",
                          "amount": money(l["amount"]),
                          "balance": str(run.quantize(Decimal("0.01"))),
                          "page": page, "bbox": box, "confidence": 96})
    args = {"account": acct_no, "month": month, "lines": lines_arg,
            "closing": str(closing.quantize(Decimal("0.01")))}
    if doc_by_rec.get(rid_):
        up = f"stmt-{rid_[:8]}-{month}"
        upload(up, pdf)
        args["document_upload"] = up
        args["document_mime"] = "application/pdf"
        args["areas"] = areas
    step("books_statement_draft_lines", args)
    step("books_app_statement_accept", {"account": acct_no, "month": month,
                                        "draft": "pending"})

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
