# 06 Testing plan

Status: plan, dated 2026-10-29. Writer: W3.
Read first: [04 first slice](04-first-slice.md) (acceptance IDs A and B), [01 architecture](01-architecture.md).

Labels follow [01, How to read this file](01-architecture.md#how-to-read-this-file). Test IDs use the form `T-<AREA>-<nn>`. "Exists" means the test is in a repo today. "PROPOSED" means it is planned and not written. No test in this plan was run for this write-up.

---

## 0. What exists today

| Test group | Where | Exists today? | Evidence |
|---|---|---|---|
| BK-TS unit and integration tests | `/Volumes/goldcoders/OSS/buwiz-books/tests/` | Yes. Not run for this plan. Reference only. | `[books §9]` |
| BK-TS tax wiring test | `tests/unit/tax-reference-wiring.test.ts:590-596` | Yes. String presence only. Does not prove behaviour. | `[verify §2 item 30]` |
| Rox v1 tests (broker and host) | Rox repo `server/test/v1-*.test.mjs` (plugins, sessions, host, sdk, manifest, package, agent, routines, gws, instances) | Yes. Not run for this plan. | `[rox §9]` |
| Rox v1 Rust tests | Rox repo `app/src/mini_v1/` (118 `#[test]` functions) | Yes. Not run. | `[rox §9]` |
| Rox conformance runner | Rox repo `server/sdk/mini-app/bin/rox-plugin-test.mjs` | Yes [V]. Mechanical checks P3, P4, P7. | `[rox §9]`; header of the file [V] |
| JCS cross-language vectors | Rox repo `app/src/mini_v1/jcs-vectors.json`, `server/test/fixtures/jcs-vectors.json` | Yes. | `[rox §9]` |
| BIR mini-app Rust tests | Live BIR `crates/bir-shell/src/mini_app/` (106 `#[test]` attributes by grep) | Yes. Not run. | `[bir-shell §5]` |
| BIR app static test | Live BIR `crates/bir-shell/v1-tests/apps.test.mjs` | Yes. Its permission test skips unless `BIR_MINI_APP_BIN` is set. Not in CI. | `[bir-shell §5; §8 item 4]` |
| BIR broker test | Live BIR `crates/bir-shell/v1-tests/broker.test.mjs` | Yes. Not in CI. Uses a per-run key and a scratch DB. | `[bir-shell §5]` |
| Books (this repo) `poc/` | `/Volumes/goldcoders/buwiz-books/poc/` | Did not exist when this file was written (checked 2026-10-29). | Directory listing [V] |
| Books (this repo) `fixtures/` | `/Volumes/goldcoders/buwiz-books/fixtures/` | Did not exist when this file was written. | Directory listing [V] |
| Every Books test in this document | This repo | PROPOSED. None written. | This file |

Rule. A skipped test in CI is a failure. BIR's permission test is skipped without an env var and no CI workflow runs it `[bir-shell §8 item 4]`. Books must not inherit that.

---

## 1. Principles

1. **Synthetic data only.** Every value in a test is invented. Canary TINs are generated at run time from a seed and never committed.
2. **No live calls.** No test calls Rox production, BIR, or buwiz-cloud. Host-level tests run a local broker on a free port with a temp data folder.
3. **The live BIR database is never opened.** Slice B uses a scratch profile (B0).
4. **Golden first.** A golden tax test fails the build. A BK-TS defect gets a regression test that fails on the BK-TS behaviour.
5. **Determinism.** Same inputs, same outputs. Clocks are injected. Time zone is Asia/Manila for all calendar dates.
6. **Refusals are one class.** Tests treat `permission_denied` and `tool_error` the same way (A12).

---

## 2. Synthetic data sets

| Set | Purpose | Content |
|---|---|---|
| DS-A | Slice A | E1 (120,000.00, 2026-07-01), E2 (100,000.00, 2026-07-15), E3 (80,000.00, 2026-08-20), E4 (rent 25,000.00, 2026-08-05), E5 (void of E2), N1 (unbalanced 100.00 and 99.99). See [04 section 4](04-first-slice.md#4-synthetic-data-set). |
| DS-B | VAT gate | Posted sales year-to-date at exactly 300000000 centavos (3,000,000.00), 300000001 (3,000,000.01), and 299999999 (2,999,999.99). |
| DS-C | Rounding | Bases in centavos at 300 bps and 100 bps: 150, 149, −150, 1, 50, 49, −50, 300000000. |
| DS-D | Periods and rates | Dates on both sides of 2020-07-01, 2023-06-30, 2023-07-01, 2025-12-31, 2026-09-30, 2026-10-01. Timestamps 2026-09-30T23:30+08:00 and 2026-10-01T00:30+08:00. |
| DS-E | 1601-C netting (P6) | Basic 40,000.00, de minimis 2,000.00, employee SSS 2,000.00. Expected taxable 38,000.00. |
| DS-F | Certificates (P4) | Received WB082; pending, lost and disputed WB082; received WB084 (final). |
| DS-G | Privacy canaries | Generated at run time. Fake nine-digit strings, a memo marker `MEMO-CANARY-<run-id>`, and a fake ticket. Never in the repo. |
| DS-H | Worked example | Gross 300,000.00 at 3% gives PT 9,000.00. Fixture credit 3,000.00. Payable 6,000.00. Overpayment case: credit 7,000.00 against PT 6,000.00. |
| DS-I | Paging | 1,000 synthetic sale lines in Q3 2026 for the entry list. |
| DS-J | Negative base | Refunds larger than sales in one quarter. Expect a blocking reason (PROPOSAL). |

Each set is a fixture file under `fixtures/` (PROPOSED). The expected results sit beside the input. A reviewer can read both.

---

## 3. Calculation, period and blocking tests (golden)

### 3.1 Rates, dates, due dates

| ID | Input | Expected | Source of expectation |
|---|---|---|---|
| T-CALC-01 | Rate for 2020-06-30 | 300 bps | No 1% row before 2020-07-01 `[bir PT-2]` |
| T-CALC-02 | Rate for 2020-07-01 | 100 bps | RA 11534 proviso `[bir PT-2]` |
| T-CALC-03 | Rate for 2023-06-30 | 100 bps | Same |
| T-CALC-04 | Rate for 2023-07-01 | 300 bps | RMC 69-2023 `[bir PT-2]` |
| T-CALC-05 | Rate for 2026-09-30 | 300 bps | `[verify §2 item 62]` |
| T-CALC-06 | Rate for 2024-06-30 with the disputed 2023-25 row off | 300 bps. Enabling the row gives a warning. | D6; `[verify §2 item 63]` UNCONFIRMED |
| T-CALC-07 | Due dates for quarters of 2026 | 2026-04-25, 2026-07-25, 2026-10-25, 2027-01-25 | Quarter end plus 25 days `[bir PT-3]` |
| T-CALC-08 | Slice A Q3 2026 | Base 200,000.00. PT 6,000.00. | `[04 section 4]` |
| T-CALC-09 | Time zone. Entry at 2026-09-30T23:30+08:00 | Q3 2026 | Asia/Manila calendar day (PROPOSAL) |
| T-CALC-10 | Time zone. Entry at 2026-10-01T00:30+08:00 (2026-09-30T16:30Z) | Q4 2026. A UTC-based implementation would wrongly put it in Q3. | Same |
| T-CALC-11 | Overdue as of 2026-10-29 for Q3 | 4 days. Surcharge and interest not computed (fields absent or zero). | D14 |
| T-CALC-12 | Period override `tax_period_key` moves an entry to another period | The proposal for each period changes. Both inputs hashes change. | PROPOSAL |

### 3.2 Blocking matrix

| ID | Input | Expected | Source of expectation |
|---|---|---|---|
| T-BLK-01 | Posted sales YTD = 300000000 centavos (DS-B) | Not breached | Strict greater-than `[BK-TS percentage-tax.ts:106]`; `[bir PT-7]` |
| T-BLK-02 | Posted sales YTD = 300000001 | Breached. Blocks with the VAT reason. No PT values. | Same |
| T-BLK-03 | Election 8% recorded in the store; request body says `electedEightPercent: false` | Blocked with the 8% reason. No values. The request value is ignored. | `[verify §2 item 32]`; `[books §1 item 4]` |
| T-BLK-04 | Returns line in Q3 | Blocked until acknowledged. Acknowledgement regenerates the proposal. | A6; D20 |
| T-BLK-05 | Negative base (DS-J) | Blocked with "negative base" reason (PROPOSAL) | D20 |
| T-BLK-06 | Blocking and warnings order | Blocking reasons listed before warnings | `[01 section 2 step 5]` |

### 3.3 Rounding and sign

| ID | Input | Expected | Source |
|---|---|---|---|
| T-RND-01 | 150 centavos at 300 bps | 5 centavos (0.05) | A10; half away from zero, D7 |
| T-RND-02 | 149 centavos at 300 bps | 4 centavos (0.04) | A10 |
| T-RND-03 | −150 centavos at 300 bps | −5 centavos (−0.05) | A10 |
| T-RND-04 | 50 centavos at 100 bps (exactly 0.5) | 1 centavo | Half away from zero |
| T-RND-05 | 49 centavos at 100 bps | 0 | |
| T-RND-06 | −50 centavos at 100 bps | −1 | Symmetry |
| T-RND-07 | Property: for random x, round(−x) equals −round(x) | Holds for 10,000 random values | PROPOSAL |
| T-RND-08 | Property: PT is never negative for a non-negative base | Holds | PROPOSAL |
| T-RND-09 | Golden tie through the live Forms cents helper. 1.50 at 3%. | Gives 0.05. The test fails if it does not. | R-11; `crates/bir-core/src/forms/form_2551q.rs:1467` [V]. Runs in P3 with the BIR checkout. |
| T-RND-10 | Overpayment (DS-H): credit 7,000.00 against PT 6,000.00 | Signed −1,000.00 line. No clamp. | `[sheets §6 item 15]` |
| T-RND-11 | Worked example (DS-H): PT 9,000.00; credit 3,000.00; payable 6,000.00 | Exact | `[01 section 7.7]` |

### 3.4 Periods, close and reopen

| ID | Input | Expected | Source |
|---|---|---|---|
| T-PER-01 | Edit an amount on a closed entry | Refused. Entry unchanged. | A7 |
| T-PER-02 | Close Q3 | Snapshot payload and SHA-256 stored | A7; `[books §1 item 7]` (BK-TS stores only a hash) |
| T-PER-03 | Recompute the snapshot | Same SHA-256 | Reproducibility, section 7.4 |
| T-PER-04 | Reopen with a reason | Period reopened. `period_event` written with reason. | A7 |
| T-PER-05 | Reopen without a reason | Refused | PROPOSAL |
| T-PER-06 | Reopen | Proposal for the period marked `stale` | A7; B4 |
| T-PER-07 | Regenerate after reopen | New row. Old row `superseded`. | A7 |
| T-PER-08 | Lock is per tax period, not the org-wide date | Closing Q3 does not lock Q4 entries. A later date in an open period is accepted. | `[books §1 item 9]`; `[auth.ts:92]` [V] |

### 3.5 1601-C and certificates (P4 and P6)

| ID | Input | Expected | Source |
|---|---|---|---|
| T-P6-01 | DS-E | Taxable 38,000.00. Not 40,000.00. | `[books §1 item 1]` (probe A to C); `[verify §2 item 28]` |
| T-P6-02 | 1601-C due date for September 2026 | 2026-10-10 | `[books §4a]` |
| T-P6-03 | 1601-C due date for December 2026 | 2027-01-15 | `[bir §3.6 D-3]` |
| T-P6-04 | Semi-monthly runs for one month (DS-E variants) | Two runs do not overwrite each other. Key includes the run. | `[books §1 item 8]` (BK-TS collides) |
| T-P4-01 | Certificate `received` WB082 | Credits line 15 | `[books §4g]` |
| T-P4-02 | `pending`, `lost`, `disputed` WB082 | Do not credit | `[books §4g]` (BK-TS posts pending) |
| T-P4-03 | WB084 received | Never credited | `[bir W-4]` |
| T-P4-04 | Period fields on a certificate | DATE, not text | `[books §2]` (BK-TS text) |

### 3.6 Later rules (P7, graduated and mixed)

| ID | Input | Expected | Source |
|---|---|---|---|
| T-P7-01 | Mixed earner, 8%, receipts plus other non-operating income | Base includes other non-operating income. No 250,000 reduction. | `[bir 8-8]`; `[verify §2 item 36]` |
| T-P7-02 | 8% base and VAT threshold | Same dated figure. Not hard-coded. | `[bir PT-7]` |

---

## 4. Traceability and contract tests

These prove the chain transaction → rule → proposal → form key (A14).

| ID | Check | Expected |
|---|---|---|
| T-TRACE-01 | Slice A `fill_args` keys | Exactly `schedule_1.0.atc`, `txtATCAmt1`, `frm2551Qv2018:txt15` |
| T-TRACE-02 | Each key | Maps to exactly one `rule_mapping` row |
| T-TRACE-03 | Lineage sum for `txtATCAmt1` | 200000.00 = E1 + E3. Under Reading 1 of the void rule, E2 and E5 are absent. Under Reading 2, they are present and net to zero. (01 section 7.6, Q-1) |
| T-TRACE-04 | Expense E4 | Not in the base lineage |
| T-TRACE-05 | Back-trace | From proposal row, through `inputs_hash` and `proposal_lineage`, to each line revision. Forward-trace from a line revision to every proposal it makes stale. |
| T-TRACE-06 | Contract: each key | In the pinned bindings of `data/forms/2551q-v2018.json`, or in the verified typed-alias list (`[verify §4]`) |
| T-TRACE-07 | Pinned copy digest | SHA-256 of `data/forms/2551q-v2018.json` equals `a206416cabb09c5e1e0ebedf9f7ac635a46856a139ffbbde5f7be88f89005788` (computed from `63267ca`, [01 section 9.1](01-architecture.md#91-observed-constraints)) |
| T-TRACE-08 | Values grammar | Every value matches `^-?[0-9]+\.[0-9]{2}$`. More decimals refused on input. |
| T-TRACE-09 | Read-back (Slice B) | `bir_form_fields` returns the same strings as `fill_args` (B7) |
| T-TRACE-10 | Lineage access | Lineage is app-only. A model-origin call is refused. |

---

## 5. Store and restart tests

| ID | Check | Expected | Source |
|---|---|---|---|
| T-STORE-01 | Open with a wrong key | Refused. File checksum before and after is equal. No partial state. | A8 |
| T-STORE-02 | `cipher_version` mismatch | Refused. No write. | Store design, [01 section 6.2](01-architecture.md#62-store-design-proposal) |
| T-STORE-03 | `strings` on the database file | No `MEMO-CANARY` value. No plaintext account names or amounts. | A8 |
| T-STORE-04 | Kill the process after a committed call | Data present on restart | Durability |
| T-STORE-05 | Kill the process in the middle of a call (fault injection before commit) | No partial entry. Lines and header both absent. | A8 |
| T-STORE-06 | Copy the database file without the key file | Cannot open. | Backup design |
| T-STORE-07 | Restart the Books process and the broker | Data unchanged. Sessions reconnect. | A8; `[rox §5 stage 8]` |
| T-STORE-08 | Key file mode | Must be 0600. Books refuses to open a key file readable by others. | Rox's `sign-manifest.mjs` refuses such key files (`readKeyFile`, Rox repo `server/src/v1/sign-manifest.mjs`) |

Scope note. The `strings` scan on the store uses canaries and memo markers only. It does not use TIN-shaped regular expressions. Stored money is integer centavos, and a 9-digit centavo value would match a TIN pattern.

---

## 6. Idempotency and retry tests

| ID | Check | Expected | Source |
|---|---|---|---|
| T-IDEM-01 | Post the same entry twice with one `client_request_id` | One entry. Same `entry_id` returned. | A9; R-18 |
| T-IDEM-02 | `books_proposal_generate` three times with one id | One proposal row. Same result each time. | A9 |
| T-IDEM-03 | Two identical generate calls in flight at once | One row | Concurrency |
| T-IDEM-04 | Two `books_app_handoff_confirm` calls with one id | One handoff record | Concurrency |
| T-IDEM-05 | Same `client_request_id`, different request body | Refused. The stored request hash differs. | PROPOSAL |
| T-IDEM-06 | Retry after a 30 s timeout | Same result, no second write | `[rox §5 stage 6]` |

---

## 7. Staleness, reruns, user edits, snapshots and amendments

### 7.1 Staleness

| ID | Input class changed | Expected |
|---|---|---|
| T-STALE-01 | A line revision (void and repost in an open period) | Proposal `stale` |
| T-STALE-02 | A tax fact | Proposal `stale` |
| T-STALE-03 | A rule set | Proposal `stale` |
| T-STALE-04 | A rate row | Proposal `stale` |
| T-STALE-05 | The pinned schema hash | Proposal `stale`. Start-up hash check fails and blocks generation. |
| T-STALE-06 | A period reopen | Proposal `stale` |
| T-STALE-07 | A stale proposal asked for `fill_args` | No `fill_args` returned. The reason is given. (B4) |
| T-STALE-08 | Regenerate after a change | New row. Old row `superseded`. Output hash differs. |

### 7.2 Determinism

| ID | Check | Expected |
|---|---|---|
| T-DET-01 | Same inputs, 100 runs | Same `output_sha256` each time |
| T-DET-02 | Same inputs in a different process start order | Same `output_sha256` |

### 7.3 Preserved user edits (Slice B and a known limit)

| ID | Check | Expected |
|---|---|---|
| T-EDIT-01 | The user edited a box in the BIR draft (source `user`), then the assistant refills | BIR returns `kept_user_boxes` `[verify §3; bir-shell §4]`. The assistant sees a mismatch and stops. It does not overwrite. |
| T-EDIT-02 | A BIR restart after a user edit | Known limit. Provenance is session-only (RK-12). The mismatch rule (B2, B7) still stops the flow if values differ. The test records the limit. |

### 7.4 Snapshot reproducibility

| ID | Check | Expected |
|---|---|---|
| T-SNAP-01 | Recompute the proposal from the stored snapshot payload | Same `output_sha256` |
| T-SNAP-02 | After a rule change, recompute the old snapshot under its recorded `rule_set_id` | Same `output_sha256`. The rule set is immutable. |
| T-SNAP-03 | Snapshot payload is stored, not just a hash | Present in the store. BK-TS stores only a hash (`[books §1 item 7]`). |

### 7.5 Amendment (MVP has none)

| ID | Check | Expected |
|---|---|---|
| T-AMEND-01 | Books never writes `is_amended` (2551Q item 4) in MVP | Not present in any `fill_args` |
| T-AMEND-02 | A handed-off period changes after the user submitted outside Books | Proposal `stale`. The message says amendment is not supported in MVP. |
| T-AMEND-03 | A superseded proposal that was handed off | Its handoff record stays for audit. |

---

## 8. Privacy scanner

### 8.1 Patterns (PROPOSAL)

| Pattern | Applies to | Rule |
|---|---|---|
| Dashed TIN shapes: three groups of three digits, optionally a fourth group of five | Outputs, props, titles, logs, transcripts | Must not appear |
| Undashed digit runs of exactly 9, 12 or 14 digits, bounded by non-digits | Text in outputs, props, titles, logs and transcripts (string values and keys) | Must not appear. Money in outputs is decimal text, so a match is a real finding. Integer centavo values are integer strings under keys ending in `_centavos`; they skip the TIN check by field context only (AGENTS.md invariant 14). A string that is exactly one 40-hex commit id or 64-hex digest is exempt (`poc/README.md`). Triage any hit. |
| `tkt_` followed by 26 Crockford characters | Titles, logs | Must not appear. Allowed in tool args and props (up to 1024 bytes). |
| `MEMO-CANARY-<run-id>` | Outputs, logs, the store (`strings`) | Must not appear |
| Canary TINs (DS-G) | Outputs, logs, store | Must not appear. Books refuses TIN-shaped text on input, if Q-4 is decided that way. |
| Key material | Env dump, logs | Env holds paths only. No base64 run of 32 or more bytes. No key file content. |

### 8.2 Scope

Tool results, app props, attachment and pin titles (Rox `mini-v1.json`), logs, transcripts, the environment of the Books process, and the store file (canaries only, as in section 5).

### 8.3 Tests

| ID | Check | Expected |
|---|---|---|
| T-PRIV-01 | Scan every tool result in the A and B runs | No pattern hit |
| T-PRIV-02 | Scan titles that Rox stores | Neutral label only. No amount, no `tkt_`, no legal name. |
| T-PRIV-03 | Env of the Books process | Paths only |
| T-PRIV-04 | Links (`rox://app/...`) | Props at most 1024 bytes. No TIN. No `tkt_` in the title. |
| T-PRIV-05 | Books logs | No ticket value. No TIN. No memo. |
| T-PRIV-06 | Input of a canary TIN in a label or memo | Refused (if Q-4 is decided that way). Otherwise stored only, and never output. |

---

## 9. Limits and paging

| ID | Check | Expected | Source |
|---|---|---|---|
| T-LIM-01 | DS-I, 1,000 lines | Five pages of 200 rows | A13 |
| T-LIM-02 | Each page | At most 64 KiB | A13 |
| T-LIM-03 | Each call | Completes inside 30 s | A13; `[rox §5 stage 6]` |
| T-LIM-04 | Arguments over 256 KiB | Refused by the host | `[rox §5 stage 6]` (`bridge.rs:466-468`) |
| T-LIM-05 | Result over 1 MiB | Refused | `[rox §5 stage 6]` |
| T-LIM-06 | Over 8 calls in flight, or over 20 per second | `rate_limited`. Books backs off and does not branch on the code. | `[rox §5 stage 6]` |

---

## 10. Access, denial, consent and user review

### 10.1 Access and denial

| ID | Caller and action | Expected | Source |
|---|---|---|---|
| T-ACC-01 | App calls a tool outside its `permissions.tools` | `permission_denied` from the host (`app/src/mini_v1/bridge.rs:457`). Treated as refusal. | A12 |
| T-ACC-02 | Plugin refuses a call (plugin-side) | Arrives as `tool_error` (`server/src/v1/sessions.mjs:300`). Treated as refusal. | A12 |
| T-ACC-03 | Agent origin calls an app-only tool (`books_app_post_entry`, `books_app_handoff_confirm`, `books_app_proposal_lineage`) | Refused. The tool is not in the model's list. | `[rox §5 stage 1; agent.mjs:49-52]` |
| T-ACC-04 | Resolve origin calls a write tool | Refused. Only `books_resolve` and reads. | `[bir-shell §3]` pattern |
| T-ACC-05 | Routine origin calls any tool | Refused (no routines in MVP) | D8 |
| T-ACC-06 | App origin without a certified open | Refused | `[bir-shell §3]` pattern |
| T-ACC-07 | Ticket for book A used to read book B | Refused. Scoped by `business_id`. | Isolation |
| T-ACC-08 | Cloud link to a profile id not in the approved list | Refused (P5) | `[claims.rs:418]` [V] |
| T-ACC-09 | Approval withdrawn: id leaves the list | Link inactive. No fill. (P5) | `[claims.rs:418]` [V] |
| T-ACC-10 | Ticket revoked in Books | Ticket no longer resolves | Books revocation (BIR has none, `[identity §2.7]`) |
| T-ACC-11 | Books data location | Books store is not under the Rox app storage folder. Only UI preferences use `rox.storage`. | `[rox §7]` storage isolation (`storage.rs:39-46`) |
| T-ACC-12 | BIR tickets | Never found in the Books store or outputs | `[bir-shell §3]` |

### 10.2 Consent

| ID | Action | Expected | Source |
|---|---|---|---|
| T-CON-01 | Attach `proposal` token | Dialog | `[rox §5 stage 9]` |
| T-CON-02 | `rox.open` a same-plugin app | No dialog | `[rox §5 stage 9]` |
| T-CON-03 | `rox.open` an external URL | Dialog | `[rox §5 stage 9]` |
| T-CON-04 | A `rox://app/books` link from outside | Dialog every time | `[bir-shell §6]` |

### 10.3 User review flows

| ID | Check | Expected |
|---|---|---|
| T-UR-01 | `card-proposal` and `card-cash` (inline) | Counts and status only. No amounts. No TIN. (RK-21) |
| T-UR-02 | `bookform` (full) before hand-off | Shows every `fill_args` value and the due date. Shows a "BIR shows these values" control. |
| T-UR-03 | Clicking "BIR shows these values" | Calls `books_app_handoff_confirm` with origin `app`. Records `fill_args_sha256`. |
| T-UR-04 | Static check on every app's permissions | `bir_form_send` and `bir_form_submit` are never listed. No app can send. |
| T-UR-05 | Nothing in Books changes the BIR draft after the user's click | No BIR call after confirm |

---

## 11. Test tiers

Commands are PROPOSED. This repo is Rust, so `bun check` from BK-TS does not apply.

| Tier | Scope | Proposed command | Runs in CI | Needs |
|---|---|---|---|---|
| T0 | Static: no float in money code, no path dependency on BK-TS or BIR, no `bir_form_send` in any permission list, format and lint | `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, a static-check test | Yes | None |
| T1 | `books-core`: money, periods, rules, PT, proposal, golden tests | `cargo test -p books-core` | Yes | None |
| T2 | `books-store`: encryption, key file, restart, idempotency | `cargo test -p books-store` | Yes | Temp folders |
| T3 | `books-mini-app`: JSON-RPC over stdio, origins, reserved tools, limits | `cargo test -p books-mini-app` | Yes | None |
| T4 | Conformance: P3, P4, P7 | `node <rox>/server/sdk/mini-app/bin/rox-plugin-test.mjs --command <books-binary> --sign-with <test-key-file>` | Yes, when the Rox checkout is present | Rox checkout. Node with `node:sqlite` (version UNKNOWN; check at P0). |
| T5 | Host-level e2e through a local broker with a per-run trust file | Node test under `tests/host/` (PROPOSED) | Yes, from P0 on | Rox checkout, a free port, a temp folder (section 13) |
| T6 | Slice B replay against a scratch BIR install | Scripted replay of the tool calls, with an audit extract | Manual gate, owner approves | Scratch BIR (B2). Not the live database. |
| T7 | Release gate: A0 to A15, B0 to B8, privacy scan, conformance | All of the above, combined | Release only | All of the above |

Unit versus host-level. T1 to T3 test code without a broker. T4 and T5 test the protocol through Rox's own code. T6 tests the real BIR tools on a scratch profile.

Skips. A skipped test fails the run. Any `#[ignore]` needs a tracked reason in the test file and a ticket in [07](07-decisions-and-blockers.md).

---

## 12. Slice B replay and audit

| ID | Check | Expected |
|---|---|---|
| T-REP-01 | Replay the recorded calls (tickets and tokens redacted) | Sequence: `books_form_proposal_get`, `bir_form_open`, `bir_form_fill` (source `document`), `bir_form_fields`, `bir_form_validate`, `bir_form_save_draft` |
| T-REP-02 | Forbidden calls in the replay | None of: `bir_form_send`, any `source: "user"`, any `from_card: true`, any `form.file`, `form.queue` |
| T-REP-03 | Calls outside the sequence | None before the user's attach (B1) |
| T-REP-04 | Books' own audit table | One `audit_event` per call, with origin, tool and outcome, and no memo or TIN |
| T-REP-05 | BIR's audit trail | UNKNOWN. Not traced in this write-up. If BIR keeps no per-call log, the replay transcript and Books' audit table are the evidence. |

---

## 13. Host-level e2e: a broker with a per-run trust file

Nothing in this section has been run. The steps follow BIR's `broker.test.mjs` pattern `[bir-shell §5]` and Rox's conformance runner `[rox §9]`.

### 13.1 Facts used

- The trust file is read from `ROX_TRUST_FILE` if set. Otherwise it is the shipped file next to the module (`server/src/v1/trust.mjs:14` [V]). The running broker does not set it today `[rox §5 stage 2]`.
- The running broker is the Rust `rox-broker` with `--port 8787 --data <dir>` flags, which spawns the Node `app-server.mjs` `[rox §6, observed process listing]`. The Node config reads `PORT` (default 8787) and `GROK_DATA_DIR` (`server/src/config.mjs:16, 21` [V]). A second broker with its own `--port` and `--data` looks possible. It has not been run (RK-2). Whether the Rust launcher forwards `ROX_TRUST_FILE` to the Node side is UNKNOWN.
- The host token is 32 random bytes in `<data>/host-token`, mode 0600, sent as `X-Rox-Host`. It authorises `/v1/host/*` only, so it can open sessions but cannot call tools `[rox §7 item 1]`.
- The conformance runner spawns the plugin the way the broker does, uses an in-memory database, a file keystore in a temp folder, and a trust file written for the run. It opens no port and does not touch the Keychain `[rox-plugin-test.mjs header]`.
- `sign-manifest.mjs` takes `--key-file` (a 0600 file holding a base64url seed) or `--keychain`. It refuses a key file readable by others `[sign-manifest.mjs header]`.

### 13.2 Steps (PROPOSED)

1. Build the Books binary: `cargo build --release -p books-mini-app`.
2. Make a temp folder with mode 0700: `<tmp>`.
3. Make a test key: `node <rox>/server/sdk/mini-app/bin/rox-plugin-test.mjs --keygen <tmp>/test.key`. Never use the real `gc-2026-01` key in a test.
4. Write `<tmp>/trust.json`, with the test key's public part under a test key id (not `gc-2026-01`), and `approved_plugins` naming `dev.goldcoders.books` with that key id. The shape is the one in `server/src/v1/trust.json` `[V]`.
5. Sign the manifest: `node <rox>/server/src/v1/sign-manifest.mjs <tmp>/manifest.json --key-file <tmp>/test.key --app home=<built app> --out <tmp>/signed.json` (flags per the file header).
6. Start a second broker on a free port: `rox-broker --port <free> --data <tmp>/data` with `ROX_TRUST_FILE=<tmp>/trust.json` in its environment. The flags follow the observed process listing `[rox §6]`. Whether the launcher forwards the trust variable is UNKNOWN. Read the broker's entry file before writing the harness.
7. Read the host token from `<tmp>/data/host-token`.
8. Inspect and install through `/v1/host/plugins/inspect` and `/v1/host/plugins/install`, pinning `manifest_sha256` to the hash the test recorded `[rox §5 stage 2]`.
9. Open an app session, then call tools through `POST /v1/host/call` with a Ks-signed envelope. The harness signs Ks the way an app does. Model it on BIR's `broker.test.mjs` `[bir-shell §5]`.
10. Assert: refusals (T-ACC-01, T-ACC-02), origin in `_meta["rox/caller"]`, timestamp window (60 s), nonce replay refused `[rox §5 stage 6]`, plugin restart shows "reconnecting" and reopens.
11. Stop the broker. Delete `<tmp>` unless a test failed. If it failed, keep `<tmp>` outside the repo and never copy a key or token into the repo.

### 13.3 Rules

- Never point a test at `~/Library/Application Support/Grok Shell` or any live data folder.
- Never write the test key, the trust file or the host token into the repo.
- The harness is Node. That is allowed for Rox tooling only (D17). It must not become a second Books runtime.

---

## 14. Unknowns that block parts of this plan

| Item | What is unknown | Blocks | Source |
|---|---|---|---|
| Node version for `node:sqlite` | Which Node version the conformance runner needs | T4 | `rox-plugin-test.mjs` imports `node:sqlite` `[V]` |
| Broker start for a second instance | Exact entry file, flags, and whether `ROX_TRUST_FILE` reaches the Node side | T5, Slice B | `[rox §6]` process flags; `server/src/config.mjs:16, 21` [V]; `[rox §5 stage 2]` |
| Scratch BIR install through the account and licence gates | Whether the e2e seed bypasses sign-in and the licence check | T6 | `[bir-shell §5]`; `[identity §1 item 3]`; `[forms §5]` |
| BIR audit trail | Whether BIR keeps a per-call log | T-REP-05 | Not traced |
| Background BIR writes | Whether BIR writes while idle (B0 checksum) | T6 (B0) | RK-19 |
| Computed item 14 echo | Whether `bir_form_fields` returns computed boxes | B2 | Canonical design, open discrepancies |
