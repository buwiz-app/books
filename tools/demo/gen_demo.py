#!/usr/bin/env python3
"""Write fixtures/demo/buwiz-trading-2026.json: the synthetic demo book the
drawings show (Books Mini App), as a list of Books tool calls.

Every name, amount and memo is synthetic. Amounts are integer centavos here and
two-place decimal text in the output; no float is used. The script checks the
golden numbers itself before it writes: the October P&L nets 251,151.00, the
trial balance at Oct 31 totals 1,224,548.40 on each side, and Q3 sales less
returns are 290,000.00 (the 2551Q row 1 the Forms page shows).

Run from the repo root: python3 tools/demo/gen_demo.py
"""
import json
import os
from collections import defaultdict

OUT = "fixtures/demo/buwiz-trading-2026.json"

def text(c):
    sign = "-" if c < 0 else ""
    c = abs(c)
    return f"{sign}{c // 100}.{c % 100:02d}"

def php(whole, cents=0):
    return whole * 100 + cents

steps = []
ledger = []   # (date, [(account, debit, credit)]) for the checks
n = 0

def rid(tag):
    global n
    n += 1
    return f"demo-{n:04d}-{tag}"[:64]

def step(tool, args):
    steps.append({"tool": tool, "args": dict(args, client_request_id=rid(tool.split("_")[-1]))})

def post(date, form, amount, account, category, payee=None, memo=None, reference=None, facts=None, dept=None, loc=None, person_kind=None, draft=False, review=False):
    args = {"form": form, "date": date, "amount": text(amount), "account": account, "category": category}
    for k, v in [("payee", payee), ("memo", memo), ("reference", reference), ("facts", facts), ("department", dept), ("location", loc), ("person_kind", person_kind)]:
        if v is not None:
            args[k] = v
    step("books_entry_draft" if draft else "books_app_post_entry", args)
    if draft:
        return
    if form == "money_in":
        ledger.append((date, [(account, amount, 0), (category, 0, amount)]))
    elif form == "money_out":
        ledger.append((date, [(category, amount, 0), (account, 0, amount)]))
    elif form == "transfer":
        ledger.append((date, [(args["to_account"] if "to_account" in args else category, amount, 0), (account, 0, amount)]))

def journal(date, lines, memo=None, payee=None, reference=None, person_kind=None, dept=None):
    args = {"form": "journal", "date": date, "lines": [
        {k: v for k, v in [("account", a), ("debit", text(d) if d else None), ("credit", text(c) if c else None), ("facts", f)] if v is not None}
        for (a, d, c, f) in lines]}
    for k, v in [("memo", memo), ("payee", payee), ("reference", reference), ("person_kind", person_kind), ("department", dept)]:
        if v is not None:
            args[k] = v
    step("books_app_post_entry", args)
    ledger.append((date, [(a, d, c) for (a, d, c, _) in lines]))

BANK, CASHBOX, CARD = "11010", "11100", "22000"
SALES, REFUNDS, CWT = "41000", "49000", "12600"
SALARIES, EMPLOYER, WTC, SSS = "61100", "61600", "25110", "25120"
RENT, UTIL, SOFT, UNCAT, SUPPLIES, DRAW = "66100", "66200", "65000", "69999", "67200", "32100"

# the book, its bank account, people and tags
step("books_app_book_create", {"label": "Buwiz Trading", "basis": "cash"})
step("books_app_account_create", {"number": BANK, "name": "Bank •••• 4021", "kind": "bank_accounts"})
for kind, names in [("customers", ["Isla Café", "Santos Hardware", "Bayani Foods"]),
                    ("vendors", ["Isla Office Rentals", "Lumen Power Co.", "Pixel Print Shop", "Ledgerline", "Reyes & Co. CPAs"]),
                    ("employees", ["Ana Cruz", "Ben Dizon"]), ("owners", ["Owner"]), ("lenders", ["Rural Bank loan"]),
                    ("government", ["BIR", "SSS", "PhilHealth", "Pag-IBIG"])]:
    for name in names:
        step("books_app_person_create", {"kind": kind, "name": name})
for kind, names in [("department", ["Operations", "Sales", "Admin"]), ("location", ["Makati office", "Warehouse"])]:
    for name in names:
        step("books_app_tag_create", {"kind": kind, "name": name})

SALE = {"document_kind": "sale_invoice"}
WITHHELD = {"document_kind": "certificate_2307", "certificate_atc": "WI010", "certificate_status": "received"}

def sale(date, who, inv, gross, withheld=0, loc="Makati office", period=None):
    lines = [(BANK, gross - withheld, 0, None)]
    if withheld:
        f = dict(WITHHELD)
        if period:
            f["period_start"], f["period_end"] = period
        lines.append((CWT, withheld, 0, f))
    lines.append((SALES, 0, gross, SALE))
    journal(date, lines, memo="Sale", payee=who, reference=f"Invoice {inv}", person_kind="customers", dept="Sales")

# January to June: one sale a month; Santos withholds in March and June (2307s received)
inv = 101
for m in range(1, 7):
    if m in (3, 6):
        q = (m - 1) // 3 + 1
        sale(f"2026-{m:02d}-20", "Santos Hardware", f"0{inv}", php(100000), php(3000), "Warehouse",
             (f"2026-{3 * q - 2:02d}-01", f"2026-{3 * q:02d}-{31 if m == 3 else 30}"))
    else:
        sale(f"2026-{m:02d}-20", "Isla Café", f"0{inv}", php(100000))
    inv += 1
# Q3: 120,000 + 100,000 (Santos withholds 2,900) + 80,000, a 10,000 return in September
sale("2026-07-01", "Isla Café", "0124", php(120000))
sale("2026-07-15", "Santos Hardware", "0127", php(100000), php(2900), "Warehouse", ("2026-07-01", "2026-09-30"))
sale("2026-08-20", "Bayani Foods", "0131", php(80000))
journal("2026-09-10", [(REFUNDS, php(10000), 0, {"document_kind": "credit_note", "contra_kind": "return"}), (BANK, 0, php(10000), None)],
        memo="Return", payee="Bayani Foods", reference="Credit note 0009", person_kind="customers")
# October: 120,000 + 100,000 (Santos withholds 3,000) + 80,000
sale("2026-10-05", "Isla Café", "0136", php(120000))
sale("2026-10-20", "Santos Hardware", "0139", php(100000), php(3000), "Warehouse", ("2026-10-01", "2026-12-31"))
sale("2026-10-28", "Bayani Foods", "0142", php(80000))

# the cash box, filled once
step("books_app_post_entry", {"form": "transfer", "date": "2026-01-05", "amount": text(php(5000)), "account": BANK, "to_account": CASHBOX, "memo": "Fill the cash box"})
ledger.append(("2026-01-05", [(CASHBOX, php(5000), 0), (BANK, 0, php(5000))]))

for m in range(1, 11):
    d = lambda day: f"2026-{m:02d}-{day:02d}"
    post(d(25), "money_out", php(25000), BANK, RENT, "Isla Office Rentals", f"Rent · {['January','February','March','April','May','June','July','August','September','October'][m-1]}", "Lease", dept="Operations", loc="Makati office", person_kind="vendors")
    # software by card until September, then from the bank
    post(d(12), "money_out", php(1200), CARD if m < 10 else BANK, SOFT, "Ledgerline", "Accounting software", "Monthly plan", dept="Operations", person_kind="vendors")
    if m <= 8:
        util = php(2811) if m < 8 else php(2814)
        card_part = php(2180) if m <= 7 else php(2180)
        post(d(15), "money_out", card_part, CARD, UTIL, "Lumen Power Co.", "Electricity", f"{['January','February','March','April','May','June','July','August'][m-1]} bill", dept="Operations", person_kind="vendors")
        post(d(16), "money_out", util - card_part, BANK, UTIL, "Lumen Power Co.", "Water and internet", None, dept="Operations", person_kind="vendors")
        pay = php(44500) if m < 8 else php(45000)
        post(d(10), "money_out", pay - php(1500), BANK, SALARIES, "Payroll", f"Payroll · {['January','February','March','April','May','June','July','August'][m-1]}", "1 employee", dept="Operations")
        post(d(10), "money_out", php(1500), BANK, EMPLOYER, "SSS", "Employer contributions", None, person_kind="government")
    if m <= 9:
        post(d(2), "money_out", php(7500), BANK, DRAW, "Owner", "Owner took out", None, person_kind="owners")
post("2026-09-15", "money_out", php(2499), BANK, UTIL, "Lumen Power Co.", "Electricity · September", "September bill", dept="Operations", person_kind="vendors")
# September payroll, with what payroll recorded withheld (a fact; Books computes no tax)
journal("2026-09-30", [
    (SALARIES, php(40000), 0, {"comp_component": "basic_pay", "party_ref": "emp-ana"}),
    (SALARIES, php(2000), 0, {"comp_component": "de_minimis", "party_ref": "emp-ana"}),
    (EMPLOYER, php(1500), 0, {"party_ref": "emp-ana"}),
    (WTC, 0, php(2808, 40), {"party_ref": "emp-ana"}),
    (SSS, 0, php(2000), {"party_ref": "emp-ana"}),
    (SSS, 0, php(1500), {"contribution_side": "employer", "party_ref": "emp-ana"}),
    (BANK, 0, php(37191, 60), None),
], memo="Payroll · September", payee="Ana Cruz", reference="1 employee", person_kind="employees", dept="Operations")
# October
post("2026-10-02", "money_out", php(8159, 40), BANK, DRAW, "Owner", "Owner took out", None, person_kind="owners")
step("books_app_post_entry", {"form": "transfer", "date": "2026-10-03", "amount": text(php(10000)), "account": BANK, "to_account": CARD, "memo": "Transfer · pay the card"})
ledger.append(("2026-10-03", [(CARD, php(10000), 0), (BANK, 0, php(10000))]))
post("2026-10-10", "money_out", php(20000), BANK, SALARIES, "Payroll", "Payroll · Oct 1–15", "2 employees", dept="Operations")
# two bank lines not sorted yet: they hold every box they could feed
post("2026-10-08", "money_out", php(150), BANK, UNCAT, "Bank", "Bank service fee", "From bank statement")
post("2026-10-15", "money_out", php(2499), BANK, UNCAT, "Lumen Power Co.", "Electricity · October", "From bank statement", person_kind="vendors")
# a draft from a receipt photo: in no total until posted
post("2026-10-18", "money_out", php(1350), BANK, SUPPLIES, "Pixel Print Shop", "Office supplies", "Photo of receipt", dept="Operations", loc="Makati office", person_kind="vendors", draft=True)

# unpaid documents: reminders on a cash book
for side, who, ref, due, amount, cat in [("collect", "Isla Café", "Invoice 0143", "2026-11-15", php(60000), SALES),
                                         ("collect", "Bayani Foods", "Invoice 0144", "2026-11-20", php(45000), SALES),
                                         ("pay", "Pixel Print Shop", "Bill PP-0381", "2026-11-05", php(3200), SUPPLIES),
                                         ("pay", "Lumen Power Co.", "November bill", "2026-11-12", php(2600), UTIL)]:
    step("books_app_owed_create", {"side": side, "person_name": who, "reference": ref, "issued": "2026-10-29", "due": due, "amount": text(amount), "category": cat})

# --- the golden checks -----------------------------------------------------
TYPES = {BANK: "a", CASHBOX: "a", CWT: "a", CARD: "l", WTC: "l", SSS: "l", DRAW: "e", SALES: "r", REFUNDS: "r",
         SALARIES: "x", EMPLOYER: "x", RENT: "x", UTIL: "x", SOFT: "x", UNCAT: "x"}
bal = defaultdict(int)
oct_pl = 0
q3_sales = 0
for date, lines in ledger:
    dr = sum(d for _, d, _ in lines)
    cr = sum(c for _, _, c in lines)
    assert dr == cr, (date, lines)
    for a, d, c in lines:
        bal[a] += d - c
        if date.startswith("2026-10") and TYPES[a] in "rx":
            oct_pl += c - d
        if "2026-07-01" <= date <= "2026-09-30" and a in (SALES, REFUNDS):
            q3_sales += c - d
tb_dr = sum(v for v in bal.values() if v > 0)
tb_cr = -sum(v for v in bal.values() if v < 0)
assert oct_pl == php(251151), text(oct_pl)
assert tb_dr == tb_cr == php(1224548, 40), (text(tb_dr), text(tb_cr))
assert q3_sales == php(290000), text(q3_sales)
assert bal[BANK] == php(412350), text(bal[BANK])
assert bal[CASHBOX] == php(5000) and -bal[CARD] == php(18240) and bal[CWT] == php(11900)
assert -(bal[WTC] + bal[SSS]) == php(6308, 40)

os.makedirs(os.path.dirname(OUT), exist_ok=True)
doc = {
    "demo_format": "buwiz-books-demo/1",
    "about": "Synthetic demo book for the drawings (Books Mini App). Every name, amount and memo is synthetic. Steps are Books tool calls made as the app.",
    "today": "2026-10-29",
    "golden": {"pl_2026_10_net": "251151.00", "tb_2026_10_31_each_side": "1224548.40", "q3_net_sales": "290000.00", "bank_2026_10_31": "412350.00"},
    "steps": steps,
}
open(OUT, "w").write(json.dumps(doc, indent=1, ensure_ascii=False) + "\n")
print(f"{OUT}: {len(steps)} steps; Oct net {text(oct_pl)}; TB {text(tb_dr)}; bank {text(bal[BANK])}")
