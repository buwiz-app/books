# 08 Mini app pages, cards, guided chat, and the feature log

Status: drawn for owner review, 2026-10-29. Nothing here is built yet. The owner's rule: no build work until every page is drawn and reviewed.

Source: every route of web Books (BK-TS `src/routes/`, 52 page files at `1662ca3`), viewed live at `localhost:3000` on 2026-10-29, plus the user's Category Mappings screenshots.

## Drawings (interactive, private artifacts)

| What | Link |
|---|---|
| Every page of the mini app | https://claude.ai/artifact/GW3HkSAJw9wQcECqGXTTyp |
| Inline cards in the chat | https://claude.ai/artifact/PmQm8iZicWFHDW9qjfqyVY |
| Guided chat: `/books`, `@books:`, templates, routines | https://claude.ai/artifact/LxFB3a2FiSsQ3XDJemx8QW |

## 1. Pages (12)

The mini app is not a one-to-one port. Rox already provides AI agents, routines, the document viewer and the chat, and the BIR app owns compliance (ADR 0007), so web Books' 52 pages collapse to 12.

| Page | Replaces in web Books | What changes |
|---|---|---|
| Today | `/`, `/inbox` | Cash now, the month so far, unpaid totals, and a Needs-you list (review, drafts, reconcile, BIR totals ready). |
| Ledger | `/transactions`, `/transactions/:id`, `/transactions/new`, `/inbox`, `/ai-proposals` | Account list, transactions with filters (Needs review, Drafts, Posted), and a detail drawer showing the double entry and the receipt. New-entry sheet with Money in, Money out, Transfer and Journal. The assistant drafts; only the user posts. |
| Unpaid | `/bills*`, `/invoices*` | Two lists, To collect and To pay. On a cash book (ADR 0008) these are reminders, and "Mark paid" records the money on the day it moves. |
| Reconcile | `/reconciliations`, `/reconciliations/:id` | Grid of accounts by month. The match view puts the statement beside Books, with suggestions, missing lines and the difference, then Finish. |
| Reports | `/financials` | P&L, balance sheet, cash flow and trial balance. Aging only on accrual books. Clicking a line drills into the Ledger. |
| Forms | new (the bridge) | Pick a form and period. Each Books total sits beside its BIR box key and value, with lineage on click. Held boxes say why. 1701-MS reads the tax option from the BIR draft. The page counts who fills the other boxes. "Hand to BIR draft" is the user's click. |
| People | `/entities/*` (7 pages and the detail page) | One page with a kind switch. Totals paid or received, a sparkline, recent transactions. No TIN. |
| Accounts | `/accounts`, `/accounts/category/:id` | Chart of accounts with balances. Receivable and payable accounts show as accrual-only on a cash book. |
| Tags | `/departments`, `/locations` and their detail pages | One page, two segments. |
| Payroll | `/payroll`, `/payroll/:runId` (amounts only) | Pay runs and per-employee lines feed 1601-C. Employee TINs, 2316s and the filing are BIR's. |
| Certificates | `/tax/certificates` (the received list) | 2307s received, with status. Only received ones count. A 2306 never counts. Building SAWT is BIR's. |
| Settings | `/organization/:id/settings`, `/mappings`, `/connections` (bank part) | Book (name, basis, fiscal year), Months (close and reopen with a reason), Defaults (Category Mappings, shortened; cash books hide the receivable and payable rows), Link & data (tax profile by id, import from web Books, export). |

The Forms page uses the real r2 mapping rules (`data/rules/*.rules.json`) and the expected values in `fixtures/vectors/`:
- **2551Q Q3:** `txtATCAmt1` 290,000.00, and `frm2551Qv2018:txt15` 3,000.00 (fixture g).
- **1601-C September:**

  | Key | Value |
  |---|---|
  | `TaxWithheld` | Yes |
  | `txtTax14` | 42,000.00 |
  | `txtTax15` | 0.00 |
  | `txtTax17` | 0.00 |
  | `txtTax18` | 2,000.00 |
  | `txtTax19` | 2,000.00 |
  | `txtTax25` | 2,808.40 |

  Item 16 is held.
- **1701-MS 2026, graduated:** IV.5 1,200,000.00 and IV.6 10,000.00.
- **1701-MS 2026, 8%:** IV.19 1,190,000.00.
- **1701-MS references:** V.3 8,900.00 and V.4 3,000.00.

Box ownership counts come from each rule set's `not_mapped` owners. Identity is folded into the profile count.

| Form | Books | BIR | Profile | Past returns | User |
|---|---|---|---|---|---|
| 2551Q | 2 | 26 | 9 | 1 | 33 |
| 1601-C | 8 | 16 | 8 | 19 | 27 |
| 1701-MS | 8 | 36 | 6 | 4 | 40 |

## 2. Inline cards

Four kinds:

| Kind | Cards |
|---|---|
| Answers (the user asked) | Cash now, Month so far, Person, P&L. |
| One-click actions | Draft from a receipt, Categorize bank lines, Mark paid, Reconcile, Close the month. |
| The BIR pair | Return to confirm, then 2551Q ready, Hand to BIR, and the BIR card with BIR's tax due. Also 1701-MS with the tax option read from BIR, and a 2307 received. |
| Quiet (routine or app start) | Book status, weekly tidy-up, month-end, BIR hand-off out of date. |

Rox limits used:
- inline width 240 to 960 px (Books uses 360 and 480);
- height 96 to 640 by default, set by `rox.fit` (SDK `docs/manifest.md:83`, `bridge.md:87-118`).

## 3. Guided chat

Three ways in: `/books` (open a screen), `@books:` scopes, and a plain question.

**Templates** (read as "TMP"; confirm with the owner):
- Record a receipt
- Tidy up the bank lines
- Close last month
- Get the quarter ready for BIR
- How did we do?

**Scopes:**

| Scope | Points at |
|---|---|
| `person` | a vendor, customer, employee or other party |
| `account` | an account in the chart |
| `period` | a month, quarter or year |
| `entry` | one transaction |
| `form` | a BIR form and period |
| `tag` | a department or location |

**Routines:**

| Routine | Can write |
|---|---|
| Weekly tidy-up | `books_entry_draft` only |
| Month-end nudge | nothing |
| BIR pair check | nothing; reads the BIR app's read-only tools |

Rox limits: 16 scopes and 8 routines per app (`docs/manifest.md:116-129`). Books tool names in the drawings are proposals.

## 4. Feature log: what does not come along

### Rox already does it
| Web feature | Route | Where it goes |
|---|---|---|
| Review agents | `/review-agents` | Rox AI agents and routines run Books' checks. Results show as Needs review in the Ledger. |
| AI proposals | `/ai-proposals` | The assistant drafts in chat. Drafts wait in the Ledger. |
| Documents | `/documents`, `/documents/:id` | Rox document viewer. Books keeps a link per receipt. |
| Inbox triage | `/inbox` | Rox chat and agents. Books shows Needs review and Drafts. |
| Sign-in, profile, theme, onboarding, create or join organization | `/login`, `/profile`, `/onboarding`, `/create-organization`, `/organizations/join` | Rox owns the window and the account. Books shows a first-run card. |
| AI credentials | Settings | Rox. |
| Documentation link | sidebar | Rox help and the `/books` menu. |

### The BIR app does it (ADR 0007)
| Web feature | Route |
|---|---|
| Tax compute (EWT, VAT split, 8% threshold) | `/tax/compute` |
| Expanded withholding, QAP, 0619-E, 1601-EQ | `/tax/ewt` |
| Tax parties (payee TINs, VAT registration) | `/tax/parties` |
| Tax settings and the yearly 8% or graduated election | `/tax/settings` |
| Filing deadlines | `/tax/deadlines` |
| Payroll filing parts (employer TIN, employee TINs, 2316) | `/payroll` |
| SAWT building | `/tax/certificates` |

### Later (needs sync or many users)
| Web feature | Why later |
|---|---|
| Business groups (`/business-groups`, Enterprise) | Many companies needs cloud sync. |
| Members and roles | One person per book on this Mac. Accountant access comes with sync (D15). |
| Live bank, Stripe and PayPal connections | The first version imports statements through Rox. |
| Sending invoices and payment links (`/invoices/pay/:id`, public view) | Books records invoices; sending comes later. |
| Inbound email address | Forward to Rox for now. |

### Dropped
| Web feature | Why |
|---|---|
| Bill approval board (In review, Pending approval, Scheduled) | One person approves by marking paid. |
| Invoice draft review step | Save, then mark paid. |
| Seven separate entity pages | One People page. |
| Separate departments and locations pages | One Tags page. |
| Per-category party read/create matrix (Mappings › Parties) | Defaults per category instead. |
| Org URL slug, logo, email settings | A local book has a name. |

## 5. Decisions this raises (owner)

1. **Amounts on inline cards.** DD-21 and DD-22 said cards carry counts and status only, because first-party inline cards mount without a click (RK-21). The drawings show amounts on cards the user asked for, and counts only on cards that appear on their own. Accept, or keep counts only.
2. **Build order.** Screens first (Ledger and Accounts, then People and Tags, Reports, Reconcile, cards and chat), with the BIR hand-off last. This replaces P2's position in `docs/05`. Not yet applied to `docs/05`.
3. **Templates.** Confirm that "TMP" means templates, and which ones ship first.
