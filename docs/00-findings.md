# 00 Findings: what already exists

Evidence date: 2026-10-29. Read-only work. No tests were run. No live Rox, BIR or cloud call was made. The live BIR filing database was not opened.

- Citations like `[rox §5]` point to the investigation reports in `../research/traces/`: `rox`, `bir-shell`, `books`, `forms`, `identity`, `sheets`, `bir`, `verify`, `decisions`.
- `verify` governs where reports disagree. Conflicts and resolutions are in section 11.
- `[V]` means the fact was re-checked on disk on 2026-10-29 with read-only commands. Other facts come from the cited report.
- Labels: **CONFIRMED** (seen in code, data or a cited primary source), **PARTIAL** (seen in part), **MISSING** (searched, not found), **UNKNOWN** (cannot be determined from the evidence), **UNCONFIRMED** (claimed, no primary support), **INFERENCE** (reasoning from confirmed facts, not a finding).
- This file records what was observed. Recommendations are only in section 13, and are labelled.

## 0. Answer in brief

| Question | Answer |
|---|---|
| What already works? | Rox v1 host and broker are installed and running (CONFIRMED, [rox §6]). The live BIR mini app 2.6.3 is installed, with tickets, Forms tools and a send-grant gate (CONFIRMED, [bir-shell §3-4]). The TypeScript Books repo has pure PH tax engines (CONFIRMED, with defects, [books §7]). The Forms schema pins for 2551Q, 1601-C and 1701-MS match buwiz-oss byte for byte (CONFIRMED [V]). The cloud has device sign-in, a `buwiz-books` OAuth client and a tax-profile list (CONFIRMED [V], with contract drift). |
| What is partial? | BIR provenance exists only inside one session (CONFIRMED). The 8% election is handled differently in each source (conflict exists; see section 11). The cloud tax-profile response differs from the agreed contract (CONFIRMED [V]). Certificate status is not checked when TS Books posts withholding (CONFIRMED). |
| What is missing? | Any Books plugin or app (MISSING, [rox §2]). A channel from Books to BIR other than the model (MISSING by design). Ticket revocation (MISSING, [identity §2.7]). A Rox broker-side TIN scrubber (MISSING, [bir-shell §8 #7]). A token path for a Rox app in TS Books (MISSING, [books §5c]). A 1701 family in TS Books (MISSING, [books §1 #2]). Credit notes and sales returns (MISSING, [books §2]). |
| What could not be verified? | Section 12. The main gaps: live BIR state, the Rox build that runs, the RR 8-2018 text (8% and 2551Q), the claimed 1% PT extension, and production hosting and database facts. |

## 1. Repo keys, paths and HEADs

Keys are used in every citation below. State was read on 2026-10-29 [V].

| Key | Repo path | HEAD (branch) | State at check | Note |
|---|---|---|---|---|
| BK-TS | /Volumes/goldcoders/OSS/buwiz-books | 1662ca3 (main) | 4 entries outside `src`: `skills-lock.json` modified; three untracked | Reference only ([decisions] owner 4). |
| BK-NEW | /Volumes/goldcoders/buwiz-books | no commits (main unborn) | `research/` untracked | This repo. |
| ROX | /Volumes/goldcoders/Projects/rox | 8453d53 (perf-parity) | clean | Protocol v1 host and broker source. The brief's `e3cdc18` is an ancestor ([rox §0]). |
| GROK | /Volumes/goldcoders/Projects/grok-shell | 9a64a92 (rox-server) | clean | Holds the broker bundle that runs (`dist/.../server/`) ([rox §0, §6]). |
| BIR | /Volumes/goldcoders/reverse-engineer-ebir-forms/bir | 29ca915b (rox-mini-app-round2) | 2 entries at repo level; `crates/bir-shell` has no uncommitted changes ([bir-shell §0]) | Live BIR plugin source. Canonical tree for BIR changes ([decisions] and design D22). VERSION 2.6.3 at `crates/bir-shell/src/mini_app/v1/manifest.rs:30`. |
| FORMS | /Volumes/goldcoders/OSS/buwiz-forms | 301f9a0f (main), STALE | clean | Local checkout. Behind the pin by 30 commits ([forms §0]). |
| FORMS@63267ca | same repo, commit `63267ca` | object present (`cat-file -t`: commit) [V] | n/a | The pin named in the handover. Read from the git object store, with no fetch. |
| BOSS | /Volumes/goldcoders/buwiz-oss | 0f34106 (main) | 42 entries dirty [V] | An earlier run counted 40 ([verify §0]). Some cited lines are working-tree text. |
| SPIN | /Volumes/goldcoders/buwiz-server | 417ec3a (main) | clean [V] | Existing Spin/WASM app. Not the planned cloud (section 6). |
| CLOUD | /Volumes/goldcoders/buwiz-cloud | 9f5ee16 (main), checked after 14:34 +0800 | clean [V] | The repo moved during the investigation: 12d63e8 (recorded at 13:49 PST), then `085c36d` (MCP and agent OAuth, 14:34 +0800) and its merge `9f5ee16`. The reflog shows checkouts of `feat/accounts-tin-activation` and `feat/submission-relay`, then a fast-forward pull to `origin/main`. `git branch -a` no longer lists `feat/submission-relay` (merged as PR #2). Earlier notes recorded 6523fcf and 2887e50 with uncommitted work. Each was correct at its time. Cite a HEAD with its time. |

Pinned Forms schemas (`crates/bir-core/data/agent-forms/`). Full SHA-256 of the exact blob bytes [V]:

| File | FORMS@63267ca (= BOSS HEAD) | FORMS local 301f9a0f (stale) |
|---|---|---|
| `2551q-v2018.json` | `a206416cabb09c5e1e0ebedf9f7ac635a46856a139ffbbde5f7be88f89005788` | `67188ca6966901b97c82aca4d8b1d9717a58efabc4d05c991769ed1a25fd55af` |
| `1601c-v2018.json` | `07ca2f04da430efa12b9fa30ca4ab4fa37bb6a226c397783a3d1e4358e7bc251` | same as pin |
| `1701ms-v2024.json` | `6cf45329d89a09d6436de58df4b0e65acb7d68d91a91bf299c3b3b635dcaab84` | `85a4fd3d9f2d7f17a2bec47f3b316c09931902639faba6a2197e292e51e8fd27` |

These digests extend the 16-character prefixes in [verify #3]. They were read from git blobs. Hash exact bytes: a trailing newline changes the digest. Hex digests can contain long digit runs. They are not TINs, and the privacy scanner should not treat them as such.

## 2. Rox host (v1)

Source: ROX @ 8453d53. Installed state is read from `~/Library/Application Support/`. Values are not reproduced.

| Item | Status | Evidence | Note |
|---|---|---|---|
| v1 host modules: `app/src/mini_v1/*.rs` (for example `bridge.rs`, 924 lines; `session.rs`; `load.rs`; `perms.rs`; `shim.js`) | CONFIRMED | [rox §0, §4] | v0 is retired for BIR. |
| Broker v1 modules: `server/src/v1/*.mjs` (trust, manifest, install, sessions, routes, agent, keystore) | CONFIRMED | [rox §0, §4] | Running build versus source: PARTIAL. Feature strings and `trust.json` match. Full parity UNKNOWN ([rox §2]). |
| Trust list: `server/src/v1/trust.json:11-24` lists publisher `gc-2026-01` and approved plugins `dev.goldcoders.bir`, `dev.goldcoders.google-workspace`, `dev.goldcoders.fixture` | CONFIRMED [V] | [rox §5 stage 2] | A new plugin needs a new entry. That is a Rox-side change ([bir-shell §6]). |
| Install checks: `manifest_sha256` must equal the reviewed hash; ed25519 over JCS; protocol major must be 1; `short` unique among enabled plugins | CONFIRMED | [rox §5 stage 2] | |
| Update rules: changed manifest at the same version gives `manifest_changed`; added permissions give `consent_declined` unless `approve_permissions`; older version needs `allow_downgrade` | CONFIRMED | [rox §5 stage 2] | |
| Session: a fresh session key per app open; certificate lifetime at most 8 h (`session.rs` MAX_LIFETIME_MS, L27) | CONFIRMED | [rox §5 stage 3] | |
| Broker call checks, in order: certificate, envelope match, Ks signature, 60 s timestamp, 128-bit nonce replay, plugin generation, permissions digest and tool membership, user disable, plugin enabled and running (`sessions.mjs` L190-232, L262-318) | CONFIRMED | [rox §5 stage 6] | |
| App-side gate: a tool not in `permissions.tools` gives `permission_denied` (`bridge.rs` L457-459). Args at most 256 KiB; 20 calls per second; 8 in flight; 30 s timeout (L104, L412-424) | CONFIRMED | [rox §5 stage 6] | |
| Other limits: result at most 1 MiB; broker to plugin 120 s; resolve 10 s; provision, session open and uninstall 5 s; app source at most 512 KiB; per JS call 2 s; heap 256 MB | CONFIRMED | [rox §7] | |
| Denial codes: broker-side and app-side denial give `permission_denied`. Plugin-side denial reaches the app as `tool_error` (`sessions.mjs` L300) | CONFIRMED (mapping); INFERENCE (spec intends `permission_denied` for plugin-side) | [rox §5 stage 7]; [bir-shell §8] | Books must treat both as refusal and never branch on the code. |
| Cross-plugin reach: an app calls only its own plugin's tools and opens only its own apps (`bridge.rs` L513-539; `sessions.mjs` L220-224, L287) | CONFIRMED | [rox §1 #5]; [bir-shell §6] | Cross-plugin flow exists only through the model (agent origin), routine `reads` (read-only, user-approved), or user-clicked links and cards. |
| Consent: an OS link, `rox.open({url})`, an https URL in plugin output, and `rox.attach` need a dialog. A full-placement result or agent resolve opens a card on click. Inline results show a card on click. Same-plugin `rox.open({app})`, pins and typed commands need no prompt | CONFIRMED | [rox §5 stage 9] | |
| First-party auto-mount: publisher `dev.goldcoders` (`plugins.mjs:32`, `FIRST_PARTY`). Its inline answer cards mount without a click | CONFIRMED [V] | [rox §5 stage 9] | A Books plugin signed with the same key would inherit this. INFERENCE from the shared key. |
| Restart policy: backoff `[1,2,4,8,16,32,60]` s (`plugins.mjs:34`); `MAX_RESTARTS = 6` (`:36`); the seventh restart within 10 minutes marks the plugin failed (`:297`) | CONFIRMED [V] | [rox §5 stage 8] | The design said 1 to 32 s. The source says 1 to 60 s (section 11). |
| Restart effects: a generation change ends sessions as `Restarted`; the app shows "reconnecting" and reopens when the plugin is running (`session.rs` L309-330) | CONFIRMED | [rox §5 stage 8] | |
| Plugin process: a child of the broker. No sandbox found in code. Spawn passes PATH and the install env only (`plugins.mjs:157`) | INFERENCE | [rox §1 #4]; [rox §7] | |
| Spawn env is written to the broker database (`v1_plugins.env`, insert in `record()`, `plugins.mjs` ~L463-466) | CONFIRMED [V] | [bir-shell §8 #10] | Pass key file paths, never key values. |
| App network: none. The shim deletes `fetch`, XHR, WebSocket, EventSource, `localStorage` and `sessionStorage` (`shim.js` L112-121) | CONFIRMED | [rox §3] | |
| UI model: compiled TSX becomes an nsh element tree that runs in QuickJS in-process and is drawn by the AppKit host. No webview, no HTML card schema. HTML appears only in a static `rox.preview` document under `PREVIEW_CSP` | CONFIRMED | [rox §1 #3, §5 stage 5] | The web-view class used for `rox.preview`: UNKNOWN. |
| nsh imports: `nsh` SDK-1 names only; `nsh/base` any names; `rox` only `rox` and `RoxError` (`load.rs` L349) | CONFIRMED | [rox §5 stage 5] | |
| `<If>` and `<Each>` runtime provider | UNKNOWN | [bir-shell §2, §8 #16] | Books must avoid them until verified. |
| Storage per app: `<plugins>/<plugin>/apps/<app>/storage.json`; keys match `^[A-Za-z0-9._:-]+$` and are at most 128 characters; values at most 64 KiB; at most 1000 keys (`storage.rs` L22-25, L112-117) | CONFIRMED | [rox §7] | |
| Permission vocabulary and caps: `tools` at most 64; `open.apps` at most 16; `open.external` at most 8; `attach` at most 16; `pick_file` up to 50 MiB; `storage` quota up to 10 MiB; `preview` up to 8 MiB. A method not granted is absent (`perms.rs` L71-90) | CONFIRMED | [rox §7] | |
| Manifest budget: 16 apps, 32 commands, 16 scopes, 8 routines, 64 tools per app, 256 KiB | CONFIRMED | [bir-shell §1] | BIR uses 13 apps and 2 routines. |
| Routines: `reads` are read-only tools of another approved plugin (user-approved once); `writes` need user consent to enable (`routines.mjs:139-141`) | CONFIRMED | [bir-shell §6] | |
| Command name collisions are renamed to `/<short>:<cmd>` (`plugins.mjs:421-429`) | CONFIRMED | [bir-shell §6] | |
| Tickets: `rox.ticket.resolve` returns `{valid, title}` only | CONFIRMED | [identity §2.8]; [bir-shell §3] | |
| Attachments and pins: canonical token `@<short>:<scope>:tkt_...`. Records live in `<data>/mini-v1.json` with titles in clear text | CONFIRMED | [rox §5 stage 4, §6]; [bir-shell §10] | Privacy effect of clear titles: INFERENCE ([rox §10]). |
| Installed state (counts only): broker on port 8787; BIR v1 2.6.3 enabled; 13 apps; 118 tools (113 BIR and 5 reserved `rox.*`); 1 attachment; 4 pins | CONFIRMED | [rox §6]; [forms §7] | |
| Broker-side TIN scrubber in `server/src/v1` | MISSING | [bir-shell §8 #7] | Books must run its own output scrubber. |
| Tests: 118 `#[test]` in `app/src/mini_v1` and 9 in `app/src/mini.rs`. `rox-plugin-test` mechanically checks P3, P4 and P7. Other checks are review items | CONFIRMED (counts); UNKNOWN (not run) | [rox §9] | |
| Signing: publisher key `gc-2026-01` in Keychain; signing with `sign-manifest.mjs` (Node). Key existence checked, contents not read | CONFIRMED | [rox §5 stage 2] | |
| Protocol spec `drafts/rox-mini-app-protocol-v1.md` is git-ignored and untracked. It exists only on this machine | CONFIRMED | [rox §10] | |
| A Books plugin or app anywhere in Rox | MISSING | [rox §2] | Searched Rox `app`, `server`, `sdk`, `connectors`, `drafts`, BIR shell and grok-shell state. |

## 3. Live BIR mini app (`dev.goldcoders.bir`)

Source: BIR @ 29ca915b. Installed as a v1 plugin.

| Item | Status | Evidence | Note |
|---|---|---|---|
| Installed as v1 plugin 2.6.3, protocol 1.0.0, enabled. Launched from a dev build path with argument `mcp` | CONFIRMED | [rox §6] | |
| Manifest: 13 apps; commands `taxprofile`, `taxform`; scopes `taxprofile`, `branchcode`, `taxform` (all `attach: true`); 2 routines | CONFIRMED | [bir-shell §1]; [rox §6] | |
| Apps and tool counts: forms 36; tax-profile 31; calendar 5; deadlines 1; card-people 5; card-forms 9; card-dates 2; card-money 7; card-help 1; card-return 5; card-quick 11; card-calc 1; register 13 | CONFIRMED | [bir-shell §1] | |
| UI: apps import only `nsh` and `rox`. Named imports include `signal`, `computed`, `resource`, `effect`, `cleanup`, `untracked`, `Text`, `View`, `Scroll`, `Input`, `Overlay`. No `fetch`, XHR or storage. Data only through `rox.call` | CONFIRMED | [bir-shell §2] | `<If>` and `<Each>` provider: UNKNOWN. |
| Routines `bir-confirmations` and `bir-payments`: every 300 s; read Gmail through google-workspace; may write `bir_confirm_filing` and `bir_record_payment` without asking. Both enabled | CONFIRMED | [rox §6]; [bir-shell §1] | See section 10, F-31. |
| Ticket format: `tkt_` plus 26 Crockford base32 characters, 130 random bits (`tickets.rs:4`; `getrandom` at `:188`) | CONFIRMED [V] | [bir-shell §3]; [identity §2.8] | |
| Ticket subjects: `tp:<root9>` (taxpayer), `br:<tin14>` (branch), `rt:<tin>/<FORM>-<period>` (return) (`tickets.rs:130-147`). The subjects are stored only inside the encrypted database | CONFIRMED | [bir-shell §3] | |
| Ticket store: one settings row `rox.v1.tickets` (`tickets.rs:28` [V]). Issue reuses one ticket per subject (`:215` [V]). `move_root` on TIN change (`:277`). `forget_all` on uninstall (`:302`) | CONFIRMED | [bir-shell §3]; [identity §2.8] | |
| Ticket revocation and expiry: no per-ticket revoke; no expiry. BIR has uninstall, TIN move and deleted-taxpayer paths only | MISSING (revoke) | [identity §2.7-2.8]; [rox §10] | Spec §9.5 expects plugin revocation. |
| Inbound ticket expansion (`mod.rs` L304-378): app-origin calls must send tickets and raw TINs are refused. Agent-origin calls may send TINs | CONFIRMED | [bir-shell §3] | |
| Outbound scrubbing (`mod.rs` L420-533): TIN keys and dashed TINs in text become tickets or `[TIN]`. Undashed digit runs in other string fields are not rewritten | PARTIAL | [bir-shell §3, §8 #6] | |
| Authorization (`server.rs` L554-595): resolve and routine origins are read-only, plus routine writes that are listed. App origin needs a certified open and the tool in the app's permissions. App-only tools refuse other origins (`is_app_only`, L1191-1197) | CONFIRMED | [bir-shell §3]; [identity §2.9] | |
| App-only tools: `bir_ticket_subject`, `bir_ticket_issue`, `bir_app_upload`, `bir_app_print_preview`, and card-button tools. Constants at `app_tools.rs:39-41` [V] | CONFIRMED [V] | [bir-shell §3] | Resolves the UNKNOWN in [forms §7] for the live tree. |
| Model-facing Forms tools (live tree): `bir_form_open`, `bir_form_fields`, `bir_form_context`, `bir_form_fill`, `bir_form_needs_you`, `bir_form_validate`, `bir_form_save_draft`, `bir_form_dismiss`, `bir_form_send`. Not exposed: `bir_form_preview`, `bir_form_card`, `bir_form_review` | CONFIRMED | [bir-shell §3] | `bir_form_send` is registered (`crates/bir-mcp/src/server.rs:610` [V]; `http.rs:579` [V]). |
| `bir_form_fill` source enum: `profile`, `past_return`, `document`, `ai`, `user`. Default `ai` (`crates/bir-mcp/src/tools.rs:547` [V]). `FILL_SOURCES` lists four (`tools.rs:90` [V]). `user` only with `from_card: true` (`tools.rs:309-311` [V]) | CONFIRMED [V] | [bir-shell §3-4] | The tool text says "Call once per source" and "Never fill a tax election (such as the 8% option) unless bir_form_context shows it stored" (`tools.rs:155` [V]). Both are text guards only. |
| `user` provenance: the model can claim `user` for keys on a card. The card itself is not exposed through MCP, so no card is shown first | CONFIRMED | [bir-shell §8 #2] | See F-16. |
| Send path: `send_draft` (`session.rs:1388` [V]) checks `has_send_grant` (`:1396` [V]). Without a grant it returns `approval_required` (`:1401` [V]) and queues nothing | CONFIRMED [V] | [bir-shell §4 step 8] | |
| Send grants: `grant_send` (`bir-core db/mod.rs:1032`), `revoke_send` (`:1059`), `has_send_grant` (`:1071`). Standing grants have no expiry. "Allow once" writes a standing grant (`bir-shell server.rs:465` [V]) and keeps the pair in memory (`:167` [V]). It is reverted only after a successful send (`end_once_grants`, `:535` [V], called at `:388` [V]) | CONFIRMED | [bir-shell §8 #1] | See F-15. |
| GUI Submit (`screen_form_queue`, `SFM:6187`) calls `queue(QueueAuthSource::Gui)` and does not check send grants | CONFIRMED | [bir-shell §4 step 7] | This is the user's own click. Books never triggers it. |
| Per-box provenance: `OpenForm.sources` (`session.rs:55` [V]) is reset on every open (`:526` [V]), is never persisted, and protects a `user` box only within the session (`:597-600`) | CONFIRMED | [bir-shell §4, §8 #3] | Provenance does not survive a restart. |
| App edits and card imports record no source (`RV:631-656`; `SFM:4458`). `bir_card_import_fill` writes one total into 2551Q `txtATCAmt1` (`RV:378-384`) | CONFIRMED | [bir-shell §6, §8 #3] | This path bypasses session provenance. |
| License gate: every bir-mcp tool calls `License::allow()` first. Desktop mode calls `GET /api/tax-profiles` without an organization id (`license.rs:291`). A firm member with passes only is refused | CONFIRMED (code reading, not executed) | [identity §5 #4] | See F-32. |
| Identity refresh: each model-facing call runs `signin::ensure_fresh` (`server.rs` L780-800), which can refresh the Buwiz session over the network | PARTIAL | [bir-shell §8 #14] | Frequency UNKNOWN. |
| Local database: `bir_data.db`, SQLCipher AES-256. Key from the Keychain, or from `BIR_DB_KEY` or `BIR_DB_KEY_FILE` when `BIR_NO_KEYCHAIN=1`. A set but invalid key is an error with no Keychain fallback. Path override `BIR_DATABASE_PATH` (`bir-core db/mod.rs` L237-262, L356, L362-370; `lib.rs` L42-47) | CONFIRMED | [bir-shell §3] | |
| Scratch database seed: `BIR_E2E_SEED=1 EBIR_TEST_ENV=1 BIR_DATABASE_PATH=<scratch> cargo test --features agent --lib mini_app::e2e_seed -- --ignored` | CONFIRMED (command from file header) | [bir-shell §5] | Whether Rox can point the installed plugin at that path: UNKNOWN (section 12, U-05). |
| Tests: 23 `#[test]` in `src/mini_app/v1`; 106 across `src/mini_app`. The `apps.test.mjs` permission check skips unless `BIR_MINI_APP_BIN` is set. No CI workflow or justfile target runs it | CONFIRMED | [bir-shell §5, §8 #4] | The test cited at `manifest.rs:16` does not exist ([bir-shell §0]). |
| Card kit: `uploadFile` calls `rox.pickFile` and `bir_app_upload`, which five card apps do not grant. Unreachable today | CONFIRMED | [bir-shell §8 #5] | A trap for later edits. |
| `bir_resolve` is described as Rox-only, but `authorize()` allows agent calls. It is read-only | CONFIRMED | [bir-shell §8 #9] | |
| Core sync path (`process_sync_json`, `UniversalTaxPayload` with f64 money) has no callers and maps only 2551Q | CONFIRMED | [bir-shell §8 #13] | Not a drop-in for Books. |
| Dispatcher constants are plain `http://` (`bir-core transport.rs:34-35`). Wire encryption UNKNOWN | CONFIRMED (constants); UNKNOWN (wire) | [bir-shell §3, §8 #11] | |
| Doc drift: the bir-mcp module doc says "Draft-only by construction: there is no submit, queue, file or payment tool" (`tools.rs:5-6` [V]), yet `bir_form_send` is registered (`server.rs:610` [V]) | CONFIRMED [V] | [forms §6] (for 63267ca) | |
| Books cannot reach BIR apps, tools, scopes or tickets. The only cross-plugin link is `rox://app/dev.goldcoders.bir/<app>?ticket=...`, which needs a BIR-issued ticket and consent each time | CONFIRMED | [bir-shell §6]; [rox §1 #5] | |
| Props for Forms: `ticket, form, period, action, draft_id, form_code, year, key` only. No channel for Books values in `rox.open` props (`M:133-145`) | CONFIRMED | [bir-shell §6] | |

## 4. Existing TypeScript Buwiz Books (BK-TS)

Source: BK-TS @ 1662ca3. Reference only. Do not import or change it (owner decision 4; design decision 26).

| Item | Status | Evidence | Note |
|---|---|---|---|
| Journal headers and lines: amounts `decimal(20,8)`; statuses draft, posted, voided; `idempotencyKey` unique per org | CONFIRMED | [books §2] `src/db/schema/journals.ts:62-163, 178-215` | Journal lines have no tax fields. |
| Bill and invoice lines are `decimal(15,2)` | CONFIRMED | [books §2, §8 #13] `bills.ts:110-135`; `invoices.ts:97-118` | Precision differs from journal lines. |
| Accounts have no tax, ATC, currency or normal-balance attribute | CONFIRMED | [books §2] `accounts.ts:22-66` | The chart of accounts is not tax meaning. |
| Party records carry a TIN (`party-tax.ts:71`) and a default ATC (`:135`) | CONFIRMED | [books §2] | ATC is a default per party, not per line. |
| Period lock: one org-wide date (`closedThrough`, `auth.ts:92`). It can move backward. Tax filing does not check it | CONFIRMED | [books §1 #9, §2] `-_period-close.ts:46-110` | No per-tax-period lock (MISSING). |
| Mapping types: bank, bill and invoice only (`mapping-types.ts:11`) | CONFIRMED | [books §2] | PH accounts resolve by number, not by mapping. |
| PH chart: 21 entries (`ph-chart.ts:71-255`). The preset header says 20 (`philippines-smb.ts:2`) | CONFIRMED | [verify #38] | Doc comment is stale. |
| Ten PH accounts have no posting path: 12610, 12700, 13800, 13810, 21700, 21710, 21750, 21800, 68400, 94100 | CONFIRMED | [verify #37] (corrects "twelve" in [books §1 #11]) | Output VAT, percentage-tax payable and percentage-tax expense are never posted. |
| 1601-C builder takes no employee-contribution input (`form-1601c.ts:52-109`). Only caller `issue-1601c.ts:52` | CONFIRMED | [verify #28]; [books §1 #1] | Probe: taxable 40,000 against 38,000 from `segregate()`. See F-01. |
| 1601-C tests cover MWE and exempt pay only (`form-1601c.test.ts:173-185`). The wiring test checks a name (`tax-reference-wiring.test.ts:593`) | CONFIRMED | [verify #29, #30] | |
| Payroll reconciliation is not passed to the filing workspace (`assemble-payroll-filing-workspace.ts:218-235`) | CONFIRMED | [books §1 #5] | |
| Re-snapshot after filing is allowed (`-filing.ts:87-174`). The payload is not stored, only the hash. `export-ph.ts:6` overclaims | CONFIRMED (code path; not executed) | [books §1 #6, #7] | See F-06, F-07. |
| Semi-monthly 1601-C runs share one upsert key. A probe gave the same period with tax 500 and 600 | UNCONFIRMED | [verify #43] | [books §1 #8] said CONFIRMED. verify governs. |
| 2551Q gross receipts and the 8% flag come from the request (`-tax-returns.ts:86-87`). The election table is not read on this path | CONFIRMED | [verify #32] | See F-02. |
| UI defaults: 8% on, quarter 1, year 2026 (`tax.compute.tsx:37, :108`) | CONFIRMED | [verify #33] | |
| 8% block is text only. Figures are still computed (taxDue 9000.00 with the block) (`percentage-tax.ts:354-360`) | CONFIRMED | [verify #31] | See F-03. |
| 2551Q rate by date: 1% for 2020-07-01 to 2023-06-30, otherwise 3% (`percentage-tax.ts:65-68`) | CONFIRMED | [verify #35, #62] | The 2023-25 extension is UNCONFIRMED (section 9B). |
| VAT threshold ₱3,000,000 is defined twice: `percentage-tax.ts:50` and `vat.ts:70` (both `"3000000"`) [V] | CONFIRMED [V] | [books §4d] | Do not port the duplicate. |
| VAT gate is strictly greater than the threshold (`percentage-tax.ts:106`) [V] | CONFIRMED [V] | [books §4d] | |
| 8% base takes gross receipts and the compensation flag only (`computeEightPercent`, L167-171). Other non-operating income is omitted | CONFIRMED | [verify #36] | See F-04. |
| Gross receipts are typed by the user (`tax.compute.tsx:34`). No ledger aggregation. No sales-return input | CONFIRMED | [books §4d] | |
| 2551Q due 25 days after quarter end (`percentage-tax.ts:344-345`) | CONFIRMED | [books §4d]; [verify #64] | |
| Creditable withholding posts a receivable without checking certificate status (`post-cwt-receivable.ts:23-113`) | CONFIRMED (code path) | [books §4g] | See F-09. |
| 2307 period columns are text (`tax-certificates.ts:53-54`) | CONFIRMED | [books §2] | |
| Money: BigInt at scale 8, rounded half-up to centavos once (`money.ts:19-22, 63-97`). Two output formats coexist. Stale comment at `money.ts:4-9` | CONFIRMED | [books §4a, §4i] | |
| No 1701 family in code (`filing-period.ts:30-38`; `tax-stage-remainder.ts:21`) | CONFIRMED | [books §1 #2] | |
| No credit notes and no sales-return flow | MISSING | [books §2] | See F-11. |
| Payroll posting requires status computed or acknowledged, and checks the period lock on run end (`payroll-journal.ts:136-150`) | CONFIRMED | [books §4c] | |
| Auth: Better Auth cookie sessions; emailOTP and organization plugins only (`auth.ts:144-234`; `auth-middleware.ts:32-40`) | CONFIRMED | [books §5c, §1 #12] | No API key, bearer or OAuth provider. MISSING. |
| Roles: superuser, admin, member, clientApprover, reportViewer. The last two hold journal view only (`permissions.ts:131-176`) | CONFIRMED | [books §5b] | |
| `issuePayrollFilingArtifacts` has no permission gate. It returns base64 2316 PDFs (`-payroll-runs.ts:448-466`) | CONFIRMED (code path) | [books §1 #10] | See F-10. |
| RLS: the tenant predicate allows a NULL bypass (`drizzle/rls_policies.sql:1-25`). Hardening section B is commented out (`rls_hardening.sql:254-262`) | CONFIRMED | [books §5b] | Production DB role: UNKNOWN. |
| Org TIN stored plaintext (`tax-reference.ts:168`) and returned unmasked to members (`-tax-settings.ts:13-17`) | CONFIRMED | [identity §2.4]; [books §8 #9] | See F-25. |
| PH TIN AI redaction exists only on branch `fix/ai-privacy-ph-ids` (commit 5341f63), not on main | CONFIRMED | [identity §2.4, §4] | |
| Firm and accountant tables exist (`inbox.ts:77-141`). No app code uses them | CONFIRMED (schema); MISSING (use) | [identity §2.6]; [books §2] | |
| Pure tax modules (form-1601c, percentage-tax, compensation, engine, vat, ewt, certificate-2307, filing-snapshot, money) can be reused, subject to the defects above | CONFIRMED (reading) | [books §7] | Coupling is by direct imports only. |
| Coupled modules: `period-close.ts` (singleton import), `db/index.ts`, `server-context.ts` (TanStack), auth | CONFIRMED | [books §7] | |
| 2550Q: output VAT and input VAT are caller-supplied. No ledger aggregation | CONFIRMED | [books §4e] | |
| Test suites | UNKNOWN (not run) | [books §9] | |

## 5. Buwiz Forms (bir-core, agent-forms schemas, bir-mcp)

Sources: FORMS (301f9a0f, stale), the pin FORMS@63267ca, and BOSS (0f34106). The live BIR tree (section 3) is canonical for BIR changes.

| Item | Status | Evidence | Note |
|---|---|---|---|
| The local checkout is stale. 63267ca is 30 commits ahead of 301f9a0f | CONFIRMED | [forms §0]; [verify §0] | Do not use the local checkout as reference. |
| The pin 63267ca is a commit in the local object store | CONFIRMED [V] | [verify #1] | The claim "not in local object store" is refuted. |
| BOSS agent-forms JSON is clean and byte-identical to the pin (digests in section 1) | CONFIRMED [V] | [verify #2, #3] | The claim "BOSS is clean" is refuted. BOSS has 42 dirty entries. |
| Crates: bir-core is full at the pin and cut in BOSS. bir-mcp and bir-desktop exist only at the pin and local. bir-app-core and bir-submit exist only in BOSS | CONFIRMED | [forms §1] | |
| Canonical copy per crate: the pin for app, MCP, desktop and transport; BOSS for offline-filer crates. Neither repo is a superset | CONFIRMED | [forms §1] | |
| Pinned agent-form schemas for 2551Q, 1601-C and 1701-MS. Each has bindings, boxes, rows and unmapped entries | CONFIRMED | [forms §3] | |
| Lock rule: identity path gives an identity lock; a path containing "election" gives an election lock; a value from the profile gives a profile lock. `lock_for` at pin `probe.rs` L2466-2475 | CONFIRMED | [verify #8] | `forms/agent/probe.rs:1326-1343` in the local checkout. Same rule, different line numbers. |
| 2551Q item 13 is locked by election, so the fill tool refuses it | CONFIRMED | [verify #7] | |
| 1701-MS Item 12 (8%), Item 17 (OSD), and `taxCodeDropdown` II015-II017 have no lock. Nothing compares them to the profile's election | CONFIRMED | [verify #10]; [forms §5, §9 #5] | INFERENCE: nothing else enforces it. See F-18. |
| Stale checkout only: 1701-MS items 17 and 31 also set the regime (`tax_option`, `source`). The pin and BOSS fixed this | CONFIRMED (stale tree) | [verify #11]; [sheets §3] | See F-19. |
| Fill tool keys: the exact official key with the `frm<Form>:` prefix. Bare `txt15` and `txt22` are rejected. For 2551Q and 1601-C one bad key fails the whole call; for 1701-MS the call refuses per key | CONFIRMED | [verify §1 #1, §3] | |
| Bare keys accepted: 2551Q `txtATCAmt1..6` and `txtEmail#occurrence-1`; 1601-C `txtEmail`; 1701-MS `taxCodeDropdown` | CONFIRMED | [verify #4] | |
| Typed aliases exist for 2551Q (`schedule_1.N.taxable_amount`, `creditable_tax_withheld`, `compromise`) and 1601-C (`tax_14_total_compensation`, and others). 1701-MS has none | CONFIRMED | [verify §4]; [forms §10] | See the key table below. |
| 2551Q penalties: at the pin, items 20-22 bind `/penalty_overrides/{surcharge,interest,compromise}`. The stale checkout leaves 20 and 21 unbound | CONFIRMED | [verify #5] | |
| Typed 2551Q `compromise` writes the root field. An auto-penalty recompute can overwrite it (`auto_compute_penalties` default true, `form_2551q.rs:455` at the pin). `refresh_auto_penalties` not traced | PARTIAL | [verify #9] | See F-20. |
| Money: typed drafts use f64 throughout. `cents(v) = round(v*100) as i64` at `form_2551q.rs:1259-1261` (FORMS). The live BIR tree has the same helper at `form_2551q.rs:1467` [V] | CONFIRMED | [forms §4] | See F-44. |
| Payable rounding: `((total_tax_due - total_tax_credits)*100).round()/100` (`form_2551q.rs:802`, FORMS) | CONFIRMED | [forms §4] | |
| Mapper: 2551Q only. No 1601-C or 1701-MS mapper. The 8% gate rejects a positive PT010 row in an 8%-election year. A NIL PT010 row is allowed | CONFIRMED | [forms §4] | |
| Queue gate: 43 registry records. `can_queue` is true for 31 (including 2551Q, 1601C, 1701MS, 1701A) and false for 12. No real form is release-ready | CONFIRMED | [forms §4] | See F-48. |
| Queue path (FORMS): `filing.queue` and `form.queue` need `confirm` set to JSON `true`, which is a caller assertion. Only 1601C and 2551Q. `filing.submit` returns `queued: false`. `form.file`, `form.submit_external` and `filing.file` are refused | CONFIRMED | [forms §6] | |
| `bir_form_send` at the pin: listed as a tool, but no host handler in the checked files (`host.rs:3472` falls through to `unknown invoke`) | CONFIRMED for the files checked [verify #19] | [forms §6] | The live tree wires it behind a send grant (section 3). Both are true for different trees. |
| Source enum at FORMS: `profile`, `past_return`, `document`, `ai`, and `user` (only with `from_card`) (`tools.rs:89`). No ledger or bookkeeping source | CONFIRMED | [forms §5] | |
| Draft identity: MCP key `(code, year, period, tin)`; storage key `period_key`. One form open at a time (`gate_form_lock`) | CONFIRMED | [forms §5] | |
| Forms identifies a taxpayer only by the 14-digit full TIN. No ticket or scope concept | CONFIRMED (grep) | [forms §7] | Ticket-to-profile mapping would have to happen before the Forms call. |
| The installed mini app's `bir_ticket_*` and `bir_app_*` handlers are absent from FORMS and the pin, and present in the live tree | CONFIRMED (live) | [bir-shell §3]; [identity §2.8]; `app_tools.rs:39-41` [V] | Resolves [forms §7] UNKNOWN. |
| 1701-MS printed item numbers repeat across parts (item 5 is both sales and Form 2316 credit). The key suffix identifies a box | CONFIRMED | [forms §3c] | |
| 1601-C Schedule I official keys cover rows 0-2 only. Typed rows N>=3 are not traced. `MAX_SCHEDULE_1_ROWS = 3` (`form_1601c.rs:35`) | UNCONFIRMED | [verify #17] | |
| Forms doc drift: the README says 13 tools; FORMS has 16 and the pin 17. `UPDATE_AGENT_FORMS=1` (docs) versus env `n` (test) | CONFIRMED | [forms §8, §9 #9-10] | |

Exact keys verified for the first slice (official key the fill tool accepts; typed alias; lock):

| Use | Exact key | Typed alias | Lock | Source |
|---|---|---|---|---|
| 2551Q Schedule 1 row 1 taxable amount | `txtATCAmt1` | `schedule_1.0.taxable_amount` | none | [verify #23] |
| 2551Q Schedule 1 row 1 ATC | `drpATC1` is refused | `schedule_1.0.atc` (ATC table) | not fillable as official | [verify §4] |
| 2551Q item 15 creditable PT withheld | `frm2551Qv2018:txt15` | `creditable_tax_withheld` | none | [verify §4] |
| 2551Q item 13 rate | `frm2551Qv2018:taxRate` (1 graduated; 2 eight_percent; the pin's `taxRateNA` maps to `not_applicable`; the printed NA option is UNCONFIRMED, [verify #7], [bir §2]) | none | election | [verify §4] |
| 2551Q item 22 compromise | `frm2551Qv2018:txt22` | `compromise` (root; can be overwritten in auto mode) | none | [verify #9] |
| 1601-C item 14 total compensation | `frm1601c:txtTax14` | `tax_14_total_compensation` | conditional | [verify #24] |
| 1601-C item 19 SSS, GSIS, PHIC, HDMF and union dues | `frm1601c:txtTax19` | `tax_19_sss_gsis` | none (path narrower than the label) | [verify #25]; [forms §3b] |
| 1701-MS IV.5 sales | `frm1701MS:txtSalesRevenueNo5a` | none | none | [verify #26] |

## 6. buwiz-server (existing Spin app)

Source: SPIN @ 417ec3a. Clean. Not used by the Books MVP.

| Item | Status | Evidence | Note |
|---|---|---|---|
| One Rust crate built to `wasm32-wasip2`. Components `auth-api`, `authorization-api`, `auth-ui` | CONFIRMED | [identity §2.11] | |
| Accounts: wasi-auth `=0.1.0-rc.5`. Email gate for tax-profile writes (`application/tax_profile.rs:312`) | CONFIRMED | [identity §2.1] | |
| Tax profile is one registration unit (TIN root plus branch), keyed by UUID. Exclusivity is HMAC-SHA256 (`domain/tin_identity.rs:19`), with a pepper from `AUTH_ROOT_KEY_BASE64` (`auth_product/runtime.rs:109, 142`) | CONFIRMED | [identity §1 #4, §2.5] | |
| Rotating the root key would orphan every `tin_hash`. No rehash or rotation path found | INFERENCE | [identity §5 #5] | |
| Access passes: ten permissions (`domain/pass.rs:17`). Presets read_only, safe (default), full (`pass.rs:207-262`). `filings.publish` is a separate switch. One open pass per grantee per profile | CONFIRMED | [identity §2.6] | |
| Agents: OAuth dynamic client registration; `bwa_` and `bwr_` tokens, hashed. `queue_on_device` is a write (`mcp/protocol.rs:25-30, 257`) and needs the full TIN | CONFIRMED | [identity §2.6, §4] | |
| README says there is no write tool (`README.md:275`). The code has `queue_on_device` | CONFIRMED (doc drift) | [identity §5 #11] | |
| Device filing: pickup on the desktop (`bir_module.rs:2314-2396`). The server checks the last four digits only (`application/agent.rs:72-95`) | PARTIAL | [identity §2.5] | |
| Plaintext TIN in `device_filings.tin` (`migrations/postgres/0014_device_filings.sql:15`). No purge path in `src` | CONFIRMED | [identity §2.4, §5 #1] | See F-23. |
| Legacy plaintext `tax_profiles.tin_root` (migration 0002, line 16). Cleared on write. Presence in a real database: UNKNOWN | CONFIRMED (schema); UNKNOWN (data) | [identity §2.4] | See F-24. |
| API DTOs carry `tin_last4` only (`contracts/tax_profile.rs:1-5`) | CONFIRMED | [identity §1 #4, §2.4] | |
| `TaxProfileView` carries `owner_user_id`, `account_id`, `org_id` and `verified_owner_user_id` (`contracts/tax_profile.rs:10`; `store/tax_profile.rs:814, 992-1040`). MCP exposure: UNKNOWN | CONFIRMED; UNKNOWN (MCP) | [identity §5 #3] | See F-33. |
| No Postgres RLS. Tenant checks are application code only | CONFIRMED | [identity §2.10] | |
| ORUS TIN verification: production adapter stubbed. `UnconfiguredOrusVerifier` returns NotConfigured (`domain/orus.rs:81`) | MISSING (production) | [identity §2.5, §4] | |
| Form 2303 ownership claim with staff review is implemented. AI review of the form is config-gated (`claim_ai/config.rs:73, 104`) | CONFIRMED | [identity §2.5, §4] | See F-34. |
| Hosting: Fermyon per README and DEPLOY.md. buwiz-oss D15 says Google Cloud | CONFIRMED (conflict exists); production UNKNOWN | [identity §1 #2, §2.11] | Section 11, C-19. |
| Local dev: `spin` listens on 127.0.0.1:3008 (one `lsof` check) | CONFIRMED | [identity §1 #10] | |
| Books-to-server link and desktop-to-server profile link | MISSING | [identity §4] | |
| Period encodings differ: drafts use `"2026"`, `"2026-Q1"`, `"2026-03"`. `queue_on_device` takes an integer 1-12 | CONFIRMED | [identity §3] | |

## 7. buwiz-cloud (planned; present at CLOUD)

Source: CLOUD @ 9f5ee16 (main), checked after 14:34 +0800 (section 1). Line numbers below are at 9f5ee16.

| Item | Status | Evidence | Note |
|---|---|---|---|
| The repo exists. Its history includes "Accounts, TIN activation and device sign-in for the Buwiz apps" (2887e50), "Submission relay: file with BIR through the taxpayer's own connection" (6523fcf), and merges #1 and #2 | CONFIRMED [V] | `git log` in CLOUD | [identity §0-1] and [decisions] say the repo did not exist. It exists now (INFERENCE: it was created after those reports). |
| Stack: axum 0.8 (`Cargo.toml:22` [V]); Postgres per [decisions] | CONFIRMED [V] (axum) | `Cargo.toml:22` | |
| Clients `buwiz-desktop` and `buwiz-books` (`src/oauth.rs:21` [V]) | CONFIRMED [V] | `src/oauth.rs:21` | |
| Device-code and token endpoints: `POST /auth/device/start` (README:70), poll (:71), `POST /oauth/token` with refresh (:72), `GET /me` (:73), logout (:75) | CONFIRMED [V] | README:70-75 (9f5ee16) | |
| Tax-profile response in code: `{id, tin, tin_masked, registered_name, activated_at, permissions}`. `permissions` is the fixed list `["profile.view","filing.prepare","filing.submit"]` (`src/claims.rs:27` [V]). The handler is at `:410`; the query filters `status = 'approved'` at `:418` [V]; `tin` at `:429`, `tin_masked` at `:430`, `permissions` at `:433` [V] (9f5ee16). | CONFIRMED [V] | `src/claims.rs` | Contract drift (F-29). The approved-only filter is traced (`:418`). |
| README documents the same TIN-bearing shape at `README.md:74` [V] (9f5ee16; `:67` at 12d63e8) | CONFIRMED [V] | README:74 | The design cited README:59. That line is about the apps' URLs. |
| Agreed contract ([decisions], cloud; revised 2026-10-29): a `books` token gets `{id, tin_masked, registered_name, activated_at, permissions:["profile.view"]}` per profile, no `tin`. Approved means non-empty `permissions`. Books must not rely on `filing.submit` | CONFIRMED (contract text; buwiz-server PR #4 (head `42190a5`, open, CI running on 2026-10-29)) | [decisions] cloud; cloud session message 2026-10-29 | Earlier wording said `display`; the field is `tin_masked`. |
| Plaintext TIN root: `tin_claims.tin_root text NOT NULL CHECK (^[0-9]{9}$)` (`migrations/0001_init.sql:79` [V]). Unique owner index (`:89` [V]) | CONFIRMED [V] (schema) | `migrations/0001_init.sql:79, :89` | Presence in a deployed database: UNKNOWN. |
| Submission relay at HEAD: `src/bir/` (dispatcher, SFTP), `src/relay/`, `src/bin/buwiz-relay.rs`, `migrations/0002_submissions.sql`. README lists `POST /api/submissions` and the relay WebSocket (README:76-77) | CONFIRMED (files and README); route wiring not traced | `git log`; README:76-77 | [decisions] said "submission tickets not built". Books must not call these endpoints. See F-30. |
| Product rules (v1): email one-time-code sign-in; one account; no organizations or workspaces. Activation by Form 2303, ID and documents, with manual staff review | CONFIRMED (owner direction text) | [decisions] cloud | Not re-traced in code. |
| Books links a business profile to a tax profile by the opaque `id` UUID, never by TIN | CONFIRMED (owner decision text) | [decisions] owner 1 and cloud | The TIN is the registration key. |
| Accountant-for-client delegation is not in v1 | CONFIRMED (text) | [decisions] cloud | |
| Rox ticket IDs are not known to the server ("nothing on the server maps them") | CONFIRMED (text) | [decisions] cloud | |
| Two services answer `GET /api/tax-profiles` with different shapes: SPIN returns `tin_last4` only; CLOUD returns `tin`, `tin_masked`, and more | CONFIRMED | [identity §2.4]; README:74 [V] | Owner must pick the contract (section 11, C-20). |

## 8. Identity, tickets and permissions

| Item | Status | Evidence | Note |
|---|---|---|---|
| No identifier is shared across Books, SPIN, the BIR desktop and Rox. The only recorded link is Books' org to its own TIN | CONFIRMED | [identity §1 #9, §2.2] | |
| Rox has no user identity. The plugin key signs session certificates. The session key signs calls. `_meta["rox/caller"]` carries plugin, app, open id and timestamp (`sessions.mjs:293`) | CONFIRMED | [identity §1 #3, §2.1] | |
| BIR desktop: device flow. Refresh token in keychain service `com.buwiz.bir`, with a 0600 fallback file beside the database (`cloud_account.rs:33-38`) | CONFIRMED | [identity §2.1] | |
| Books identity: better-auth (`src/lib/auth.ts:62`). Password sign-in disabled (`:73`). Google (`:78`). Email OTP (`:146`). Invite-only (`:26, :48, :84`) | CONFIRMED | [identity §2.1] | |
| Ticket grammar: `^tkt_[0-9A-HJKMNP-TV-Z]{26}$` (`perms.rs:254-257`; `tickets.rs:179`) | CONFIRMED | [identity §2.8] | |
| Ticket scope: root scope from `scope.rs:192`. App calls are checked. Scope for model-origin calls: UNKNOWN | CONFIRMED (app); UNKNOWN (model) | [identity §2.8] | Section 12, U-06. |
| Tickets never expire. No per-ticket revoke. BIR has uninstall, TIN move and deleted-taxpayer paths (`tickets.rs:116, 277, 302`; `v1/mod.rs:277-289`) | CONFIRMED | [identity §2.7-2.8] | Spec §9.5 expects revocation. See F-39. |
| Books tickets: MISSING. The design gives Books its own subject map | MISSING | [identity §2.8] | |
| Permission scopes: BIR scope rules (eight kinds at `scope.rs:16-35`; closed by default at `app_tools.rs:360`). Rox per-app manifest permissions. Books ABAC statements (`permissions.ts`). SPIN: ten pass permissions | CONFIRMED | [identity §2.9] | |
| Consent and revocation: owner ends a pass or an agent connection; a claim ends open passes; BIR send grants have no expiry; Books firm consent is MISSING | CONFIRMED / MISSING | [identity §2.7] | |
| Taxpayer identity in Forms: the 14-digit full TIN only | CONFIRMED | [forms §7] | |
| TINs appear in ticket subjects only inside BIR's encrypted database. Titles are taxpayer names, stored clear in Rox's records | CONFIRMED | [bir-shell §3]; [rox §6] | See F-26. |
| Period encodings differ across systems (`"2026-Q1"` in drafts; integer months in `queue_on_device`) | CONFIRMED | [identity §3] | Pick one encoding before cross-system drafts. |
| Cross-boundary key table (proposal in [identity §3]) | PROPOSAL, not implemented | [identity §3] | |

## 9. Research spreadsheets and tax-rule status

### 9A. Workbooks

Two workbooks: `Buwiz_Tax_Profiles_Categories_Form_Fields.xlsx` (10 sheets) and `Philippine_Bookkeeping_and_BIR_Form_Map.xlsx` (5 sheets). Copies of the two workbooks and the handover are in `research/inputs/` (this repo). The CSV dumps and scratch copies were session files and are not in the repo ([sheets §0]).

| Item | Status | Evidence | Note |
|---|---|---|---|
| Worked example: 22 cached cells recomputed; all match their formulas | CONFIRMED | [sheets §2] | |
| Arithmetic errors: none. Errors are in labels, template formulas and one input | CONFIRMED | [sheets §2, §6] | |
| Mislabels: C12 is II.19, not II.24; C25 is II.31, not II.36; C11 should sum II.15-17; C17 should sum II.15-20 | CONFIRMED | [verify #45-48] | Values unchanged. |
| C37 labelled V.10 should sum V.1-V.9 | UNCONFIRMED (E99 formula not printed) | [verify #54] | Section 12, U-24. |
| Monthly 1601-C: 2,808.4 in the workbook against 2,808.33 by the stated method. The payroll method is UNCONFIRMED | CONFIRMED (arithmetic); UNCONFIRMED (method) | [verify #49] | |
| "PAY-Q3" 42,000 is one month (September), not a quarter | CONFIRMED | [verify #55] | |
| 8% sheet ties to 1701-MS (V.2 43,000; V.3 9,000; V.4 3,000; II.25 21,000) | CONFIRMED | [verify #56] | |
| 8% sheet `MAX(0, ...)` clamps an overpayment. The workbook's own rule says keep the sign | CONFIRMED | [verify #57] | See F-13. |
| Profile cells C11-C15 are referenced by no formula. C12 "filer earns compensation" is unused | CONFIRMED | [verify #58] | |
| 1701-MS scope label "Individual/Fiduciary" should stay. RMC 34-2025 Annex A item 3 includes some fiduciaries. The proposed change to "Individual" is refuted | CONFIRMED (current label); proposed change not supported | [verify #60] | |
| Account is not tax meaning: `salaries` (61100) feeds 1601-C II.14 and 1701-MS IV.10A. `refunds_and_discounts` (49000) feeds the 2551Q base, IV.6 and IV.19 | CONFIRMED (mapping reading) | [sheets §8 #1] | |
| Category catalog: 59 categories (33 base, 18 PH, 8 proposed). 51 reused account numbers match BK-TS exactly. Three PH accounts (12700, 12800, 61950) are missing from the catalog | CONFIRMED | [verify #39, #40]; [sheets §4] | |
| Category selection formulas share one template (59 cells) | CONFIRMED | [verify #61] | |
| Field tabs against Forms schemas: 2551Q 51 of 71 match; 1601-C 56 of 78; 1701-MS 67 of 95. 1701-MS II.35 has the wrong ID. II.34 representative TIN is absent from the schema. III.38 drawee is absent | CONFIRMED | [sheets §3]; [verify #13-14] | |
| Profile alternatives are not enforced: C9 (1701-MS) and G10 (1701/1701A) can both be "Yes". Annex C describes one annual return | CONFIRMED | [sheets §8 #5] | |
| Workbook 2551Q example nets a ₱10,000 credit note before the 3% | UNCONFIRMED (no printed deduction found) | [bir PT-8]; [sheets §6 #9] | See F-14. |

### 9B. Tax-rule status (as of 2026-10-29)

| Rule | Status | Evidence |
|---|---|---|
| PT010 at 3% from 2023-07-01; 1% window 2020-07-01 to 2023-06-30 | CONFIRMED | [bir PT-2]; [verify #62, #35] |
| 1% extended to 2025-12-31 by a later law | UNCONFIRMED | Only respicio.ph commentary, which names no law [verify #63] |
| 2551Q due 25 days after quarter end | CONFIRMED | NIRC §128(1) as amended by RA 11976 §29 [verify #64]; [bir PT-3] |
| VAT threshold ₱3,000,000, CPI-indexed every three years | Threshold CONFIRMED; current indexed figure UNKNOWN | [bir PT-7]; [verify #65] |
| 8%: 8% of receipts plus other non-operating income, less ₱250,000 for purely self-employed. Election each year | CONFIRMED (RMO 23-2018; RR 8-2018 digest) | [bir 8-1, 8-5]. RR 8-2018 full text UNCONFIRMED (HTTP 403) |
| 8% electors do not file the 2551Q (RMO 23-2018). BOSS requires NIL PT010 rows. BK-TS blocks | CONFIRMED (conflict exists); resolution UNCONFIRMED | [bir 8-6]; [verify #73, #84]; BOSS `form_2551q.rs:2214-2228` [V] |
| Mixed earners: no ₱250,000 reduction | CONFIRMED | [bir 8-8]; [verify #75] |
| Mid-year breach of ₱3M moves the year to graduated rates; PT applies from the start of the year until VAT | CONFIRMED | [bir 8-7]; [verify #74] |
| 1601-C item 19 is the employee's share only | CONFIRMED (RR 11-2018 §2.78.1(B)(12)); statute UNCONFIRMED | [verify #69] |
| 1601-C item 17 cap ₱90,000 | CONFIRMED | [verify #67] |
| Holiday pay exempt for MWEs. OT, NSD and hazard basis UNCONFIRMED | CONFIRMED (holiday pay); UNCONFIRMED (rest) | [verify #68] |
| 1601-C due 10th of the following month; December due 15 January | CONFIRMED in code (P6 rule; `research/handoff-to-forms/rates-reference.v1.json`, formerly `data/rules/rates.v1.json`, cites `form-1601c.ts:178-199`; due dates are Forms' under ADR 0007). Printed due-date text UNCONFIRMED | [verify #83]; [books §4a]. [bir D-3] says printed CONFIRMED; verify governs (section 11, C-24) |
| 1701-MS tiers: micro below ₱3M; small ₱3M to below ₱20M (RR 8-2024) | CONFIRMED (words); lawphil numerals are off by 1,000 | [bir M-2]; [verify #71] |
| 1701-MS: one business income-tax rate for individuals, including mixed-income earners. Annex C covers TY2025 | CONFIRMED for TY2025; TY2026 applicability UNKNOWN | [verify #77, #80] |
| 1701-MS manual filing allowed without a COR (Annex C A10, TY2025 FAQ) | CONFIRMED (manual route) | [verify #76] |
| 1701-MS whole-peso rounding ("49 centavos or less drop down") | UNCONFIRMED (image only) | [verify #83] |
| WB080 and WB082 credit against 2551Q item 15. WB084 is final and never credited | CONFIRMED (printed codes: [bir W-2], [bir W-4]; [verify #82] confirms the codes on the forms) | [bir W-2, W-4]; [verify #82] |
| WI and WC income-tax withholding goes to income-tax credits | INFERENCE | [bir W-3] |
| Only certificates with status `received` should credit | CONFIRMED (rule); BK-TS does not check (section 4) | [books §4g] |
| TIN branch code is five digits (RMC 36-2026 item 3) | UNCONFIRMED (image or OCR only) | [verify #83] |
| 2551M monthly route status | UNCONFIRMED | [verify #83] |
| 1600-PT due date | UNCONFIRMED | [verify #83] |
| Annex C thresholds, spouse limits and special-rate limits | MISSING | [verify #79] |

## 10. Confirmed bugs and risks

Ordered roughly by effect on Books. Status describes the evidence, not the severity.

| # | Finding | Status | Evidence | Effect on Books |
|---|---|---|---|---|
| F-01 | TS Books 1601-C drops employee mandatory contributions | CONFIRMED (probe A-C) | [verify #28]; [books §1 #1]; BK-TS `src/lib/tax/form-1601c.ts:52-109` | II.19 is absent and II.22 is overstated by the employee shares. Tax is unchanged ([verify #28]). |
| F-02 | TS Books 2551Q takes gross receipts and the 8% flag from the request | CONFIRMED | [verify #32]; BK-TS `src/routes/api/-tax-returns.ts:86-87` | A wrong regime can be filed. UI default is 8% on ([verify #33]). |
| F-03 | TS Books 8% block is text only; figures still computed | CONFIRMED | [verify #31]; BK-TS `percentage-tax.ts:354-360` | A caller can compute figures in an 8% year. |
| F-04 | TS Books 8% base omits other non-operating income | CONFIRMED | [verify #36]; BK-TS `percentage-tax.ts:167-171` | RMO 23-2018 includes it ([bir 8-1]). |
| F-05 | TS Books tax filing ignores the period lock; the lock can move backward | CONFIRMED | [books §1 #9]; BK-TS `auth.ts:92`; `-_period-close.ts:46-110` | Closed-period figures can change under a filed return. |
| F-06 | TS Books allows re-snapshot after filing | CONFIRMED (code path; not executed) | [books §1 #6]; BK-TS `-filing.ts:87-174` | Snapshot checksum and time can be overwritten after filing. |
| F-07 | TS Books stores only a hash of the snapshot, not the payload. Export header overclaims | CONFIRMED | [books §1 #7]; BK-TS `export-ph.ts:6` | Evidence is not reproducible. |
| F-08 | TS Books does not pass payroll reconciliation to the filing workspace | CONFIRMED | [books §1 #5]; BK-TS `assemble-payroll-filing-workspace.ts:218-235` | Blocking reconciliation issues do not block filing. |
| F-09 | TS Books posts withholding receivables for pending, lost and disputed certificates | CONFIRMED (code path) | [books §4g]; BK-TS `post-cwt-receivable.ts:23-113` | Books would claim credits not yet received. |
| F-10 | TS Books `issuePayrollFilingArtifacts` has no permission gate | CONFIRMED (code path) | [books §1 #10]; BK-TS `-payroll-runs.ts:448-466` | Read-only roles can obtain 2316 PDFs. |
| F-11 | TS Books has no credit notes or sales-return flow | MISSING | [books §2] | 2551Q base can be overstated (design decision 20 is open). |
| F-12 | Semi-monthly 1601-C runs share one key | UNCONFIRMED | [verify #43]; [books §1 #8] | Second run can overwrite the first. Live effect UNKNOWN (U-26). |
| F-13 | Workbook 8% sheet clamps overpayment to zero | CONFIRMED | [verify #57]; [sheets §6 #15] | Overpayments disappear in the example. |
| F-14 | Workbook nets a ₱10,000 return before 3% | UNCONFIRMED | [bir PT-8] | Base may be wrong if the deduction is not allowed. |
| F-15 | "Allow once" send grant can outlive the process; no startup cleanup | CONFIRMED (code path) | [bir-shell §8 #1]; BIR `crates/bir-shell/src/mini_app/server.rs:455-469, 535-546`; `crates/bir-core/src/db/mod.rs:1032-1058` | A standing grant can authorise a later send. Grants have no expiry. |
| F-16 | Model can label values `user` for keys on a card; no card shown first | CONFIRMED (code path) | [bir-shell §8 #2]; BIR `session.rs:1214-1249`; `crates/bir-mcp/src/tools.rs:309-311` [V] | Books must never send `user` or `from_card`. |
| F-17 | BIR provenance and user-box protection are session-only | CONFIRMED | [bir-shell §8 #3]; BIR `session.rs:55, 526, 597-600` | Provenance is lost on restart. App edits are never recorded. |
| F-18 | 1701-MS items 12, 17 and II015-II017 have no lock | CONFIRMED | [verify #10]; [forms §5] | Nothing compares them to the recorded election (INFERENCE). Tool text is the only guard. |
| F-19 | Stale FORMS checkout couples 1701-MS items 17 and 31 to the regime | CONFIRMED (stale tree) | [verify #11]; [sheets §3] | Fixed at the pin and in BOSS. A stale copy is dangerous. |
| F-20 | Typed 2551Q `compromise` can be overwritten by auto-penalty recompute | PARTIAL | [verify #9]; pin `form_2551q.rs:961-991` | Item 22 is only durable through `txt22`. |
| F-21 | `bir_form_send` listed with no handler at the pin; the module doc says no send tool | CONFIRMED for the pin's files checked | [verify #19]; [forms §6]; live `tools.rs:5-6` [V] | The live tree wires it behind a grant (`session.rs:1388-1401` [V]). |
| F-22 | The GUI Submit path queues without a grant check | CONFIRMED | [bir-shell §4 step 7] | This is the user's click. Books must never trigger it. |
| F-23 | Plaintext TIN in SPIN `device_filings.tin`, with no purge path | CONFIRMED | [identity §2.4, §5 #1]; `migrations/postgres/0014_device_filings.sql:15` | Privacy P1 in [identity]. |
| F-24 | Plaintext TIN root in SPIN legacy `tax_profiles.tin_root` and CLOUD `tin_claims.tin_root` | CONFIRMED (schema); UNKNOWN (deployed data) | [identity §2.4]; CLOUD `migrations/0001_init.sql:79` [V] | Presence in production UNKNOWN (U-20). |
| F-25 | Plaintext TIN in BK-TS: org TIN (`tax-reference.ts:168`), party TINs (`party-tax.ts:71`). Org TIN returned unmasked (`-tax-settings.ts:13-17`) | CONFIRMED | [identity §2.4]; [books §8 #9] | Any Books copy would add another plaintext location. |
| F-26 | Taxpayer names and titles stored in clear text in Rox `mini-v1.json` | CONFIRMED (location); privacy impact INFERENCE | [rox §6, §10]; [bir-shell §10] | Books titles must carry no names or amounts by default. |
| F-27 | BIR scrubber is partial: undashed digit runs in free text are not rewritten | PARTIAL | [bir-shell §8 #6] | Books must scrub its own output. |
| F-28 | BIR dispatcher constants use plain `http://` | CONFIRMED (constants); UNKNOWN (wire) | [bir-shell §8 #11] | Section 12, U-21. |
| F-29 | CLOUD tax-profile response carries `tin`, `tin_masked`, `registered_name`, `activated_at`, and `filing.submit` for every returned profile. README:74 documents the same | CONFIRMED [V] | CLOUD `src/claims.rs:27, 410-433` [V] (9f5ee16); README:74 [V] | Blocks the cloud link (design blocker B1). Fix pending merge: buwiz-server PR #4 (head `42190a5`, open, CI running on 2026-10-29). Books must refuse TIN-bearing responses and never rely on `filing.submit`. |
| F-30 | CLOUD has a submission relay at HEAD (`src/bir/`, `src/relay/`, `POST /api/submissions`) | CONFIRMED (files; README:76-77) | `git log` (6523fcf); README:76-77 | Contradicts [decisions] ("not built"). Books must never call it. Route wiring not traced (U-28). |
| F-31 | Two enabled BIR routines write BIR data every 300 s without a prompt | CONFIRMED | [rox §6]; [bir-shell §10] | Precedent only. Any Books routine must follow the same `writes` rule. |
| F-32 | Desktop license refuses firm members who hold passes only | CONFIRMED (code reading) | [identity §5 #4] | A firm workflow fails silently. |
| F-33 | SPIN `TaxProfileView` exposes internal IDs. MCP exposure UNKNOWN | CONFIRMED; UNKNOWN | [identity §5 #3] | Internal IDs may reach a model. |
| F-34 | Form 2303 (TIN, name, address) can go to a vision model when a key resolves | CONFIRMED (config); UNKNOWN (production key) | [identity §5 #2] | Possible third-party disclosure. |
| F-35 | A credentialed database URL appears in the local Spin process argument list | CONFIRMED (value not reproduced) | [rox §6] | INFERENCE: visible to other processes of the same user through `ps`. |
| F-36 | Spawn env is persisted in the broker database | CONFIRMED [V] | `plugins.mjs` insert (~L463-466); [bir-shell §8 #10] | Pass file paths, never key values. |
| F-37 | Rox broker has no TIN scrubber | MISSING | [bir-shell §8 #7] | The broker cannot catch a TIN in a plugin result. |
| F-38 | Plugin-side denial reaches the app as `tool_error`, not `permission_denied` | INFERENCE (spec intent) | [rox §5 stage 7] | Books must treat both codes as refusal. |
| F-39 | Tickets have no expiry and no per-ticket revoke | CONFIRMED / MISSING | [identity §2.7-2.8] | A leaked ticket stays valid until uninstall. |
| F-40 | BIR permission test is skipped without an env var and is not in CI | CONFIRMED | [bir-shell §8 #4] | Permission drift can ship unnoticed. Books must not inherit this. |
| F-41 | TS Books period locks are org-wide, with no per-tax-period lock | CONFIRMED | [books §2] | Design decision 16 needs a new mechanism. |
| F-42 | BOSS working tree has 42 uncommitted entries. Citations into `form_2551q.rs` are working-tree text | CONFIRMED [V] | [verify §0]; [sheets §7 X8] | Re-check any BOSS line before citing it. |
| F-43 | TS Books has ten PH accounts with no posting path, including output VAT and percentage-tax payable | CONFIRMED | [verify #37] | VAT and percentage-tax postings do not exist yet. |
| F-44 | Forms typed drafts use f64 for money | CONFIRMED | [forms §4] | Do not reuse these drafts as money. |
| F-45 | TS Books defines `VAT_THRESHOLD` twice | CONFIRMED [V] | `percentage-tax.ts:50`; `vat.ts:70` | Use one dated fact. |
| F-46 | No token path for a Rox app to call TS Books | CONFIRMED | [books §5c] | A new auth path is needed for any Rox-to-server design. |
| F-47 | SPIN and BOSS plans conflict: SFTP owner, cloud host, and D2/D3 against accounts and passes | CONFIRMED (conflict exists) | [identity §1 #2, §5 #7]; BOSS `PLAN.md:69`; `PROGRESS.md:70-76`; `DECISIONS.md:21` | Owner must pick one design of record. |
| F-48 | Forms README claims in-app queue for 43 forms. No real form is release-ready | CONFIRMED | [forms §4, §9 #8] | Do not trust the README's readiness claim. |
| F-49 | `rox.preview` renders HTML under a CSP. The web-view class is UNKNOWN | UNKNOWN | [rox §5 stage 5] | Print previews may need review before use. |
| F-50 | Cloud agent (`mcp`-scope) tokens read full TINs. `GET /api/tax-profiles` (`src/claims.rs:410`) and MCP `list_tax_profiles` (`src/mcp.rs:156-158`) both call `active_profiles` (`src/claims.rs:415-433`), which returns `tin`. Only the submission handlers call `require_app()` (`src/submissions.rs:95, :236`; `src/oauth.rs:384-390`) | CONFIRMED (code reading; not executed) | [decisions] cloud (contract: masked `display` only; "no extra scope in v1"); CLOUD `085c36d` | Books never uses an `mcp` token. Fix pending merge: buwiz-server PR #4 (head `42190a5`, open, CI running on 2026-10-29) (`mcp` and `books` tokens get `tin_masked` only; 403 on submissions). The cloud session also found `buwiz-books` had scope `app` and could create submissions; the same PR fixes it. |

## 11. Report conflicts and resolutions

| # | Topic | Source A says | Source B says | Resolution |
|---|---|---|---|---|
| C-01 | Does buwiz-cloud exist? | [identity §0, §1 #1]: MISSING | [decisions] cloud: "not yet created". Design: exists | Exists [V] (section 7). The identity search predates it. |
| C-02 | CLOUD HEAD and state | Design: 2887e50 with seven changes and untracked paths | [V] 9f5ee16 on main, clean, after 14:34 +0800. Earlier, 12d63e8 (13:49) and 6523fcf on `feat/submission-relay` (reflog) | Time-dependent. Cite the time with the HEAD. |
| C-03 | CLOUD status | [decisions] cloud: design only, nothing built | [V] commits show accounts, TIN activation, device sign-in and submission relay | The contract still binds. The status line is stale. |
| C-04 | Tax-profile response | [decisions] cloud: `{id, display, permissions}` | [V] `claims.rs:429-433` and README:74 return `tin`, `tin_masked`, and more | Drift (F-29). The design cited README:59. The row is at :74 at 9f5ee16 (:67 at 12d63e8). |
| C-05 | Rox restart backoff | Design: 1 to 32 s | [rox §5 stage 8]; [V] `plugins.mjs:34`: up to 60 s | 1 to 60 s. |
| C-06 | BIR ticket handlers | [forms §7]: UNKNOWN (absent from FORMS) | [bir-shell §3]; [identity §2.8]; [V] `app_tools.rs:39-41` | Present in the live tree. |
| C-07 | `bir_form_send` | [verify #19]: no handler at the pin | [bir-shell §4]; [V] `session.rs:1388-1401`; `server.rs:610` | Both true for different trees. The live tree gates it. |
| C-08 | Rox option letters for the Books placement | [rox §8]: (a) is an app inside BIR, recommended for a first slice | [bir-shell §6]: (a) is a sibling plugin, recommended | The letters are inverted. This design uses names. Owner decision: separate service ([decisions] owner 1). |
| C-09 | Unposted PH accounts | [books §1 #11]: twelve | [verify #37]: ten | Ten. |
| C-10 | Cents helper line | [forms §4]: FORMS `form_2551q.rs:1259` | [V] live BIR `form_2551q.rs:1467` | Cite the tree. Both exist. |
| C-11 | Semi-monthly 1601-C key | [books §1 #8]: CONFIRMED | [verify #43]: UNCONFIRMED | UNCONFIRMED. |
| C-12 | Annex C retrieval | [bir §6]: not retrieved (HTTP 403) | [verify #76]: retrieved as a scan. Manual 1701-MS filing allowed (TY2025 FAQ) | [verify] governs. TY2026 applicability UNKNOWN ([verify #80]). |
| C-13 | 1701-MS scope label | [sheets §6 #12]: change to "Individual" | [verify #60]: refuted. RMC 34-2025 Annex A item 3 includes some fiduciaries | Keep "Individual/Fiduciary" and annotate. |
| C-14 | Section 109 lettering | [bir PT-6]: "109(CC) under EOPT" | [verify #66]: refuted as stated. CC appears in RA 11534 §116 (CREATE) | Predates EOPT. Keep the lettering by concept. |
| C-15 | Provenance in FORMS | [forms §4]: CONFIRMED (stale checkout) | [verify §5]: not verified for the pin | Live tree: CONFIRMED ([bir-shell §8 #3]). |
| C-16 | Lock rule line references | [forms §3]: `probe.rs:1326-1343` (local) | [verify #8]: pin `lock_for` at L2466-2475 | Same rule. Cite the tree used. |
| C-17 | 2551Q 8% NIL row lines | [sheets §7]: BOSS `form_2551q.rs:2217-2226` | [verify #73]: about 2216-2226. Design: 2214-2228 [V] | Working-tree text. Ranges differ. |
| C-18 | 1% PT extension | [sheets §5]: UNCONFIRMED | [verify #63]: UNCONFIRMED. [bir PT-2]: reverted to 3% from 2023-07-01 | Stored as disputed and off (design decision 9). |
| C-19 | Repo naming and host | [decisions] cloud: remote `buwiz-server` (empty); local `buwiz-cloud` | SPIN is a populated Spin app at `/Volumes/goldcoders/buwiz-server` (417ec3a). Hosting: Fermyon ([identity]) against Google Cloud ([BOSS D15]) | Owner picks the repo of record (design D13). |
| C-20 | Tax-profile endpoint | SPIN: `tin_last4` only ([identity §2.4]) | CLOUD README:74: TIN-bearing shape | Two services share one path with different shapes. Owner picks the contract. |
| C-21 | Workbook 2551Q base | [sheets §6 #9]: nets credit note CN-Q3 | [bir PT-8]: no printed deduction found | UNCONFIRMED (design decision 20). |
| C-22 | 8% electors and 2551Q | [bir 8-6], RMO 23-2018: not required | BOSS `form_2551q.rs:2214-2228`: NIL PT010 required. BK-TS: blocks ([verify #73]) | CONFIRMED (conflict exists); resolution UNCONFIRMED. RR 8-2018 full text needed (U-09). |
| C-23 | Forms tool counts and "draft-only" wording | [forms §8]: README says 13 | FORMS 16; pin 17; live module doc says no send tool (`tools.rs:5-6` [V]) | Doc drift. Use the code's registration list. |
| C-24 | 1601-C printed due-date text | [bir D-3]: CONFIRMED (printed) | [verify #83]: UNCONFIRMED (image or OCR only) | UNCONFIRMED. The code rule is CONFIRMED (section 9B). Verify governs. |
| C-25 | 1701-MS printed rounding and 15 October installment | Canonical design and [bir D-4]: CONFIRMED | [verify #83]: UNCONFIRMED (image only) | UNCONFIRMED (docs/07 section 5; docs/03 D-14, D-15). |
| C-26 | Cloud grant scopes and agent tokens | [decisions] cloud: no extra scope in v1; any token for the user sees the approved TINs | [V] `085c36d` (9f5ee16) adds `app` and `mcp` grants. `mcp` tokens reach the same handler and get `tin` (F-50) | Settled in buwiz-server PR #4 (head `42190a5`, open, CI running on 2026-10-29): device clients get their own scopes (`buwiz-desktop` → `app`, `buwiz-books` → `books`). Books uses only the `buwiz-books` client. |

## 12. Could not verify

What is missing, why it matters, and what access is needed to close it.

| # | Item | Why it matters | Status | Access needed |
|---|---|---|---|---|
| U-01 | Live BIR state: signed in, database unlocked, healthy now | Gates Slice B | UNKNOWN | Owner runs a read-only status call with the owner present. Do not open the live database (owner decision 5). |
| U-02 | Whether the running Rox build equals perf-parity 8453d53 | Source findings may not describe what runs | UNKNOWN | Rox owner builds 8453d53 and compares, or confirms the running build hash. |
| U-03 | nsh `<If>` and `<Each>` provider | UI code paths | UNKNOWN | nsh SDK source. Location is not in the reports. |
| U-04 | Web view behind `rox.preview` | Print and receipt preview | UNKNOWN | Rox owner. |
| U-05 | Whether Rox can point an installed plugin at a scratch BIR database (design B2, D12) | Slice B isolation | UNKNOWN | Rox and BIR owners. Test with a scratch install and the seed command in section 3. |
| U-06 | Model-origin ticket scope | Whether a Books ticket can reach another taxpayer through the model | UNKNOWN | Trace `server.rs` authorize and `scope.rs` in the live tree, or run a test. |
| U-07 | Whether `bir_form_fields` echoes computed item 14 | Read-back check in Slice B | UNKNOWN | Scratch BIR run, or a trace. If not echoed, validate or preview instead. |
| U-08 | `refresh_auto_penalties` and the typed `compromise` write | Penalty values in drafts | PARTIAL | Trace in FORMS@63267ca (object store) or the live tree. |
| U-09 | RR 8-2018 full text (BIR CDN HTTP 403) | 8% base and 2551Q in an 8% year (design D5) | UNCONFIRMED | A BIR-issued copy, from the owner or a tax adviser. |
| U-10 | 1% PT extension to 2025-12-31 | PT010 rates for 2023-07 to 2025-12 (design D6) | UNCONFIRMED | Text of the enacting law, or a tax adviser. |
| U-11 | Annex C applicability to TY2026; RMC 30-2026 | 1701-MS for TY2026 | UNKNOWN | A BIR issuance for TY2026, or a tax adviser. |
| U-12 | CPI-adjusted VAT threshold | VAT gate (design D10) | UNKNOWN | BIR issuance. |
| U-13 | 2551M status; 1600-PT due date; RR 3-2024 | Monthly PT route; remittance dates | UNCONFIRMED / MISSING | BIR CDN copies (eFPS page gave a TLS error); tax adviser. |
| U-14 | NIRC §32(B)(7)(f) text; OT, NSD and hazard basis; de minimis items beyond (a)-(e); 1601-C item 23 basis | 1601-C payroll rules (phase P6) | UNCONFIRMED | Statute text; tax adviser. |
| U-15 | Image-only items: 1601-C due-date text; 2551Q item 13 instruction; 1701-MS rounding and October 15 installment; RMC 36-2026 item 3 (five-digit branch) | Printed rules | UNCONFIRMED | Readable copies or OCR; tax adviser. |
| U-16 | BIR issuances after May 2026 (RMC 42-2026 seen only in a snippet) | Currency of the rules | UNKNOWN | BIR issuance list. |
| U-17 | 1701-MS spouse limits and special-rate limits | 1701-MS scope (phase P7) | MISSING | Annex text. Not in the Annex C pages read. |
| U-18 | Local FORMS checkout still at 301f9a0f | Contract tests (phase P3) | PARTIAL | Owner decides to update the checkout. Read-only use of the pin is enough for hashes. |
| U-19 | Test results in each repo | Trust in current behaviour | UNKNOWN (not run) | Each owner's local run. |
| U-20 | Production facts: `api.buwiz.com`; hosting target; production database role (RLS); legacy plaintext TIN rows; Form 2303 AI key state | Privacy and tenancy posture | UNKNOWN | The owners. Read-only configuration review. |
| U-21 | BIR dispatcher wire encryption | TIN in transit | UNKNOWN | BIR owner. |
| U-22 | Whether the Books identity and the Buwiz account are linked | One person, one identity | UNKNOWN | Product owner. |
| U-23 | BOSS working-tree baseline (42 entries) | Citations into uncommitted text | UNKNOWN | BOSS owner commits or shares a baseline. |
| U-24 | Workbook C37 formula (E99) and printed 1701-MS line 11 wording | Labels only | UNCONFIRMED | Re-parse the workbook. The S3 PDF is needed for line 11. |
| U-25 | Sources S1-S17 except S3, and build-note links S6, S7 | Some rules | UNKNOWN | Fetch the sources. |
| U-26 | Semi-monthly 1601-C on live tenants | Whether F-12 matters in practice | UNKNOWN | BK-TS owner runs a read-only query on live data, or confirms no tenant uses semi-monthly payroll. |
| U-27 | Whether a `form.send` standing-OK store exists outside the files read at the pin | Send gate in the pin | UNKNOWN | Full-tree search of FORMS@63267ca. The live tree already answers for BIR (section 3). |
| U-28 | Cloud route wiring for submissions and relay | Whether the cloud can file today | UNKNOWN | CLOUD owner; route tests. |
| U-29 | Books use of the `firm` ABAC statement | Accountant model | MISSING (use) | Books owner. |

## 13. Recommended (not findings)

These are recommendations for the owners. They are not facts.

1. Use the full digests in section 1 as the pin for `data/` (phase P3). Keep the stale local FORMS checkout out of the reference set.
2. Treat the cloud contract as `id`, `tin_masked`, `registered_name` and `permissions`. Refuse TIN-bearing responses. The fix for F-29 and F-50 is buwiz-server PR #4 (head `42190a5`, open, CI running on 2026-10-29); confirm it is merged before phase P5 (design blocker B1).
3. Do not use the submission relay or any submission endpoint from Books (F-30).
4. Before Slice B, close U-05 (scratch BIR isolation) and the source label decision (design D3) with the Forms owner.
5. Keep the 8% and 2551Q block until RR 8-2018 is read (U-09; design D5).
6. Ask the design owner to add the corrections to the "Design corrections" section of `docs/07-decisions-and-blockers.md`. Three corrections are to the design itself: C-02 (cloud HEAD and state), C-04 (the tax-profile row is at README:74, and the code shape differs) and C-05 (Rox restart backoff is 1 to 60 s). C-19 is an owner decision.
