# BIR mini app (`dev.goldcoders.bir`): reference for a Books sibling

Read-only. Nothing was built, run, edited, signed or installed. Labels: CONFIRMED (read in code or a static check I ran), PARTIAL, MISSING, UNKNOWN.

**Sources:** BIR branch `rox-mini-app-round2` at HEAD `29ca915b` (VERSION 2.6.3, manifest.rs:30). Rox SDK, spec and broker on branch `perf-parity` at `8453d53`. Books repo only for its dev port and auth.

**Summary**
- BIR serves 13 TSX apps, 57 `bir_app_*` host functions, 2 routines, 3 scopes, 2 commands and about 113 tool names (my count from the registration lists, not run).
- Tickets are `tkt_` plus 26 Crockford characters. The subject map lives in BIR's SQLCipher DB. Apps send tickets only. The model sees tickets, and TIN scrubbing is BIR-side.
- Books cannot reach BIR's apps, tools, scopes or tickets directly. Cross-plugin contact is user-mediated or read-only inside routines.
- Recommendation: option (a), a sibling plugin `dev.goldcoders.books`.
- Section 8 lists findings to fix before copying the pattern.

**Corrections to the brief**
- No top-level `permissions` key. Permissions are per app (manifest.rs:336).
- "Books on :3000" is CONFIRMED for Books (`package.json:6`, `vite dev --port 3000`). BIR has no reference to it. The only `:3000` in BIR is a test origin at http.rs:752.
- "Broker-side scrubber" is MISSING. Rox's `server/src/v1` contains no TIN scrubber. The scrubber is BIR-side (mod.rs:420-533).
- `the_permissions_name_only_listed_tools`, cited at manifest.rs:16, does not exist. The real check is apps.test.mjs:83, which skips unless `BIR_MINI_APP_BIN` is set. No `.github` workflow references `v1-tests`.

**File key (absolute paths; citations below use these short names)**
- `M` = `/Volumes/goldcoders/reverse-engineer-ebir-forms/bir/crates/bir-shell/src/mini_app/v1/manifest.rs`
- `AT` = `.../bir-shell/src/mini_app/v1/app_tools.rs`
- `MOD` = `.../bir-shell/src/mini_app/v1/mod.rs`
- `TK` = `.../bir-shell/src/mini_app/v1/tickets.rs`
- `KEYS` = `.../bir-shell/src/mini_app/v1/keys.rs`
- `SRV` = `.../bir-shell/src/mini_app/server.rs`
- `SCOPE` = `.../bir-shell/src/mini_app/scope.rs`
- `CARDS` = `.../bir-shell/src/mini_app/cards.rs`
- `RV` = `.../bir-shell/src/mini_app/return_view.rs`
- `BRIDGE` = `.../bir-shell/src/mini_app/bridge.rs`
- `APPS` = `.../bir-shell/src/mini_app/apps.rs`
- `MINIMOD` = `.../bir-shell/src/mini_app/mod.rs`
- `PAY` = `.../bir-shell/src/mini_app/payments.rs`
- `SESS` = `/Volumes/goldcoders/reverse-engineer-ebir-forms/bir/crates/bir-shell/src/agent/session.rs`
- `BIRMOD` = `.../bir-shell/src/bir_module.rs`
- `SFM` = `.../bir-shell/src/screen_form_module.rs`
- `TPM` = `.../bir-shell/src/tax_profile_module.rs`
- `APPDIR` = `.../bir-shell/mini-apps/` (`tax-profile.tsx`, `forms.tsx`, `card-*.tsx`, `deadlines.tsx`, `calendar.tsx`, `register.tsx`, `card-kit.tsx.in`, `apps.json`)
- `SCRIPTS` = `.../bir-shell/scripts/` (`sign-v1-manifest.sh`, `sync-card-kit.mjs`)
- `V1T` = `.../bir-shell/v1-tests/` (`apps.test.mjs`, `broker.test.mjs`)
- `CORE` = `/Volumes/goldcoders/reverse-engineer-ebir-forms/bir/crates/bir-core/src/` (`cloud_account.rs`, `db/mod.rs`, `lib.rs`, `platform/macos.rs`, `transport.rs`, `google_calendar.rs`, `email/auth_oauth.rs`, `news_fetcher.rs`, `integration/{service,models,mapper}.rs`, `integration/providers/generic.rs`)
- `MCP` = `/Volumes/goldcoders/reverse-engineer-ebir-forms/bir/crates/bir-mcp/src/` (`tools.rs`, `http.rs`, `main.rs`)
- `ROX` = `/Volumes/goldcoders/Projects/rox/`: `server/src/v1/{trust.json,plugins.mjs,manifest.mjs,routines.mjs}`, `server/src/config.mjs`, `drafts/rox-mini-app-protocol-v1.md` (spec), `server/sdk/mini-app/docs/{manifest,bridge,testing}.md`
- `BOOKS` = `/Volumes/goldcoders/OSS/buwiz-books/` (`package.json`, `.env.example`, `CLAUDE.md`)

---

## 1. Manifest (`M`)

- Constants: PROTOCOL `1.0.0` (27), PLUGIN_ID `dev.goldcoders.bir` (28), SHORT `bir` (29), VERSION `2.6.3` (30), `rox://manifest` (31), `BIR_V1_MANIFEST` (34), `ROUTINE_WRITES` (39-42), TICKET_PROP (53).
- `struct AppSpec { id, placement, inline, props, permissions }` (69-75). `card(id, permissions)` (88-106) builds inline apps whose props are `{card, ticket, snapshot, as_of, tool, args}`.
- `specs()` (108-254) lists the 13 apps. `routine()` (304-316). `unsigned(catalog)` (319-378).
- Each app entry: `id`, `title`, `description`, `entry: "rox-app://<id>"`, `mime: "text/x-nsh-tsx"`, `sdk: "1"`, `fingerprint` = sha256 of the served source (63-65), `props`, `placement`, `inline`, `permissions`.
- Top-level keys (343-377): `protocol`, `id`, `short`, `version`, `title: "BIR"`, `description`, `publisher {id: dev.goldcoders, key_id: gc-2026-01, name: Goldcoders}`, `commands`, `scopes`, `routines`, `apps`.
- Serving: `served_text` (382-388) serves the file named by `BIR_V1_MANIFEST`, else the unsigned JSON, which Rox refuses. `staleness` (424-441) flags a signed file that no longer matches the build.
- Signing (`SCRIPTS/sign-v1-manifest.sh`): runs `bir-mini-app print-v1-manifest`, then the broker's `sign-manifest.mjs` with key id `gc-2026-01`. Keys are never printed.

**Commands (2):** `taxprofile` `[tin or name]`; `taxform` `[form] [period] [view|file|submit]`. Both resolve through `bir_resolve`.

**Scopes (3, all `attach: true`, all resolve through `bir_resolve`):**
- `taxprofile`: hint = `TAXPROFILE_HINT` (267-274).
- `branchcode`: hint = `BRANCHCODE_HINT` (277-280).
- `taxform`: hint = `TAXFORM_HINT` (257-264). Its description, "Return: form and period", lowercases to the noun `return:` (minor).

**Routines (2, `every_s: 300`, both `reads` `gws_search_mail`/`gws_read_mail` from `dev.goldcoders.google-workspace`):**
- `bir-confirmations`: query `bir_confirmation_search.query`; writes `[bir_confirm_filing]`.
- `bir-payments`: query `bir_payment_search`; writes `[bir_record_payment]`.
- Both declare `params` = `watch::schema()` (`taxpayers` up to 50, `forms` up to 20).

**Apps:**

| id | placement | inline size | tools | other permissions | props |
|---|---|---|---|---|---|
| forms | full | - | 36 | open.apps [tax-profile]; open.external [https://mail.google.com]; attach [taxprofile, taxform]; pick_file (7 receipt types, 25 MiB); preview `{}` | ticket, form, period, action (view/file/submit), draft_id, form_code, year, key |
| tax-profile | full | - | 31 | open.apps [forms]; copy | ticket, year |
| calendar | full | - | 5 | open.apps [forms] | ticket |
| deadlines | inline + full | 480x320, min_w 320 | 1 (`bir_calendar_get`) | open.apps [forms, calendar] | ticket |
| card-people | inline | 320 | 5 | open [tax-profile, forms, register, deadlines]; attach [taxprofile] | card kit |
| card-forms | inline | 320 | 9 | open [forms, calendar, tax-profile] | card kit |
| card-dates | inline | 360 | 2 | open [forms, calendar] | card kit |
| card-money | inline | 320 | 7 | open [forms] + Gmail; preview; pick_file | card kit |
| card-help | inline | 240 | 1 | open [register, forms] | card kit |
| card-return | inline | 360 | 5 | open [forms]; pick_file (adds xlsx, csv) | card kit |
| card-quick | inline | 230 | 11 | open [forms, register, calendar]; storage; preview; pick_file | card kit |
| card-calc | inline | 215 | 1 (`bir_lookup`) | none | card kit |
| register | full | - | 13 | open [forms, tax-profile]; copy | `{}` |

Limits (CONFIRMED by the unit tests at M:451-521 and M:486-487, and against SDK `manifest.md:101-146`): apps 13/16, commands 2/32, scopes 3/16, routines 2/8, largest tool list 36/64, manifest under 256 KiB.

Drift: spec §5.10 (`ROX` drafts, lines 326-387) is an older example at version 2.0.0 with placeholder fingerprints. Use code, not that example. The SDK doc says routines take "No other keys" (`manifest.md:53`), but the broker validator allows `params` (`manifest.mjs:267, 279-287`) and the spec describes instances (drafts 319-324). The SDK doc is stale.

---

## 2. App UI model

**Runtime (CONFIRMED):** each app is one TSX module. Its default export is mounted by Rox. Allowed imports are `nsh`, `nsh/base` and `rox` only (spec §6.3; `apps.test.mjs`). All 13 apps import only `nsh` and `rox`.
- Named `nsh` imports used: `signal, computed, resource, effect, cleanup, untracked, Text, View, Scroll, Input, Overlay`, plus types `JSX, Signal, Accessor, YNode`.
- `<If>` and `<Each>` are used without import and are not defined in the repo or the spec. Which runtime module provides them is UNKNOWN.
- No `fetch`, XHR, WebSocket or web storage (spec §6.3; `testing.md` APP-04). Data comes only from `rox.call`.
- Colors need `dark:` variants (the `K` palette in `card-kit.tsx.in`).

**Card kit:** `card-kit.tsx.in` is copied verbatim between `// card-kit:begin/end` markers into each `card-*.tsx` by `sync-card-kit.mjs`. `apps.test.mjs:42` strips the kit before the permission check.

**`rox.*` surface actually used (grep counts across TSX, kit copies included):**

| member | count | where |
|---|---|---|
| `rox.props` | 18 | read once at module top (`tax-profile.tsx:31`) |
| `rox.call` | 56 | `bir_app_<fn>` wrapper (`tax-profile.tsx:35-40`, `forms.tsx:51`); card refresh (`card-money.tsx:72`) |
| `rox.open` | 20 | `{app, ticket, props}` same-plugin (`forms.tsx:547`, `deadlines.tsx:110`, card kit `card-money.tsx:96`); `{url}` Gmail (`forms.tsx:2966`, `card-money.tsx:413`) |
| `rox.fit` | 45 | card kit height reporting (`card-money.tsx:146-164`) in all 8 card apps |
| `rox.pickFile` | 10 | receipts and imports (`forms.tsx:82`, `card-money.tsx:265`) |
| `rox.preview` | 9 | print preview (`forms.tsx:1627`), receipt (`card-money.tsx:402`, `card-quick`) |
| `rox.attach` | 3 | `forms.tsx:97` `rox.attach("taxform", …)`; card-people (taxprofile) |
| `rox.copy` | 3 | tax-profile, register |
| `rox.storage` | 2 | `card-quick.tsx:368,391` (remembers "Not this return") |
| `rox.ticket.title` | 1 | `deadlines.tsx:74` (always available, spec §6.4) |
| `rox.viewport` | 2 | `deadlines.tsx:96`, `forms.tsx:2670` |

Not used: `rox.app`, and the host-only `rox.session.*` and `rox.ticket.resolve`.

**Static permission audit (CONFIRMED, my script over the TSX vs. the `M`/`AT` lists):** every literal `rox.call`, `rox.preview`, `tool()` and `bir("fn")` target in the five full apps is granted, and no granted tool is uncalled. Counts: forms 36, tax-profile 31, calendar 5, register 13, deadlines 1. Card apps are covered except the kit's upload helper (finding 5).

**Excerpts:**

```ts
// tax-profile.tsx:31-40
const PROPS: Props = (rox.props || {}) as Props;      // ticket from the open
const TICKET = typeof PROPS.ticket === "string" ? PROPS.ticket : "";
async function bir<T>(fn: string, ...args: unknown[]): Promise<T> {
  const r: any = await rox.call(`bir_app_${fn}`,       // -> app_tools.rs tool
    TICKET ? { args_json: JSON.stringify(args), ticket: TICKET } : { args_json: JSON.stringify(args) });
  return r?.result as T;
}
// tax-profile.tsx:593 export default function R() { ... rox.call("bir_ticket_subject", {ticket}) → Profile({tin}) }
```
Apps see raw TINs from `bir_ticket_subject` (first-party UI). The model never does (CONFIRMED).

```ts
// forms.tsx:100-133 H = { open, fill, validate, save, queue, … } — each is bir<any>("screen_form_*", …)
// forms.tsx:97 const attachReturn = (s) => rox.attach("taxform", `${BRANCH}/${s.code}-${tokenPeriod(s)}`);
// forms.tsx:1627 rox.preview({ tool: "bir_app_print_preview", args: { ticket: BRANCH, form: s.code, ...args } })
```

```ts
// card-money.tsx:72 (kit refresh): snap.set(answerOf(await rox.call(P.tool, P.args)))
//   P.tool is the tool that answered the chat, chosen by the server (cards.rs:46-81).
// card-money.tsx:297-306: rox.call("bir_record_payment", { tin: ret.tin, form, period, … })
//   tin here is a ticket; the server refuses a raw TIN from an app (MOD:318-322).
```

---

## 3. MCP server (`bir-mini-app`)

- **Entry (`MINIMOD`, usage text):** `mcp` (stdio, Rox's transport), `serve --http 127.0.0.1:PORT` (desktop side, not Rox), `print-v1-manifest`. The binary requires the `agent` feature (`bir-shell/Cargo.toml`).
- **Protocol:** `initialize` (SRV:1280); `tools/list` (SRV:1296); `resources/list` = `rox://manifest` plus `rox-app://<id>` (SRV:258-268); `resources/read` (SRV:269-283).
- **Apps as resources:** embedded at build time by `build.rs` (one `include_str!` per `*.tsx`), or read from `BIR_MINI_APPS_DIR` (`APPS:17-73`).
- **Reserved `rox.*` tools (not in `tools/list`; `MOD:129-201`):**
  - `rox.provision`: checks plugin id and protocol major 1, stores the ed25519 key in memory only (`KEYS` header), replies with the manifest sha256.
  - `rox.session.open` (MOD:231-259): checks the app exists and `perms_sha256` = sha256(JCS(permissions)) (`M:415-419`), signs an 8-hour certificate, records open_id to app.
  - `rox.session.close`, `rox.ticket.resolve` (title only, MOD:277-292), `rox.uninstall` (forgets all tickets, MOD:261-268).
- **Authorization (`SRV:554-595`, CONFIRMED):**
  - Resolve and routine origins may call read-only tools and `bir_resolve` only.
  - A routine turn may write only the tools its routine lists (`routine_writes`, `M:45-51`).
  - An app origin needs a certified open and a tool in that app's permissions.
  - Non-app origins are refused app-only tools (`is_app_only`, SRV:1191-1197).

**Tool inventory (≈113 names):**

- **Model-facing (agent or routine origin):**
  - `bir_status` (no args).
  - `bir_app_open {app ∈ forms|tax-profile|register|calendar|deadlines, tin?, form?, period?, action?, draft_id?, branch?}`: returns a `_meta["rox/open"]` card (`APPS:20` OPENABLE; SRV:863-990).
  - `bir_drafts_list {tin?, form?, status?}`, `bir_calendar_get {tin?, from?, to?}`, `bir_calendar_set {tin, forms[], enabled, reminder_days[]}` (write), `bir_calendar_changes {since?}`, `bir_calendar_link {key, event_id?}` (write).
  - `bir_confirmation_search {tins[], taxpayers[], forms[], since?}` (read; routine query), `bir_confirm_filing {tin, form, period, reference, received_at, message_id}` (write).
  - `bir_resolve {kind: command|scope, name, value?, complete?, context.attached[]}`: described as Rox-only, but not enforced (finding 9).
  - Payments: `bir_payment_search`, `bir_payment_parse {text}`, `bir_record_payment {tin, form, period, channel, amount: number|string, paid_at, …}` (write), `bir_mark_paid {tin, form, period, proof_path|proof_upload}` (write), `bir_payments_list`, `bir_filing_record {tin, form, period}` (returns a `rox://view?path=` link), `bir_payment_receipt {tin, form, period}` (read; preview document).
  - Deadlines: `bir_deadline {form, period?, tin?}`, `bir_deadline_overrides_list`, `bir_deadline_override_set` and `_remove` (write; rules apply to all taxpayers).
  - Reads: `bir_profile_forms`, `bir_returns_list` (`taxpayer.rs`), `bir_find_taxpayer`, `bir_taxpayer`, `bir_penalty`, `bir_unpaid`, `bir_board`, `bir_quick_add` (`answers.rs`), `bir_outbox`, `bir_digest`, `bir_form_info`, `bir_lookup`, `bir_return` (`lookup.rs`, `RV`).
  - bir-mcp host tools (`SRV:45-57`, descriptors from `MCP/tools.rs`): `bir_profiles_list`, `bir_dues_list`, `bir_form_open {code, period?, tin?}`, `bir_form_fields`, `bir_form_context`, `bir_form_fill {fields{box:value}, source, from_card}`, `bir_form_needs_you`, `bir_form_validate`, `bir_form_send` (send-grant gate), `bir_form_save_draft`, `bir_form_dismiss`. Left out on purpose: `bir_form_preview`, `bir_form_card`, `bir_form_review`, `bir_connect`, `bir_list_apps`, `bir_status`-as-connect.
- **App-only (`rox/audience: app`; `is_app_only`):**
  - `bir_ticket_subject {ticket}`: returns kind, tin, title. `bir_ticket_issue {ticket, tin}`: a sibling-branch ticket.
  - `bir_app_upload {upload?, data: base64}`: up to 25 MiB, 4 uploads in flight (`AT:47-49, 597-645`).
  - `bir_app_print_preview {ticket, form, draft_id | year+quarter | draft_json+edits+unlocked | period}`: returns an embedded `text/html` resource (`AT:479-594`).
  - `bir_card_allow_send {tin, form}` (write, "allow once").
  - Card buttons: `bir_card_add_taxpayer`, `bir_card_cancel_send`, `bir_card_remove_payment`, `bir_card_attach`, `bir_card_import {upload, name}` (read), `bir_card_import_fill {import}` (write).
  - `bir_app_<fn>` × 57 (`AT:168-173`: FORMS 27 + TAX_PROFILE 29 + CALENDAR 4 + REGISTER 13, unioned). Arguments: `args` array, or `args_json` (JSON array text, for amounts), and `ticket`. 21 are read-only (`AT:135-157`).
- **Reserved, handled by name:** the five `rox.*` tools above.

**Output envelopes:**
- `call_result`: `content[0].text` = JSON of the result, plus `structuredContent.result` (SRV:1164-1176).
- bir-mcp host tools use `tool_text`: pretty JSON text only, no `structuredContent` (SRV:770-771, 1251-1256).
- Errors: `tool_error`, `isError: true` (SRV:1258-1262).
- Agent answers may carry `_meta["rox/open"]` cards and `rox/suggest` chips, decorated for origin `agent` only (SRV:~398-402; table at `CARDS:46-81`).

**Tickets (CONFIRMED):**
- Format: `tkt_` plus 26 Crockford characters, 130 bits from `getrandom` (`TK:179-197`).
- Subjects: `tp:<root9>` (taxpayer), `br:<tin14>` (branch), `rt:<tin>/<FORM>-<period>` (return) (`TK:130-147`).
- Storage: one settings row holding `{ticket: subject}` in BIR's SQLCipher DB (`TK` header).
- Minting: `issue` reuses one ticket per subject (`TK:215-227`), `issue_all` (229), `subject` (265), `move_root` on a TIN change (277), `forget_all` on uninstall (302). Titles are read live (`TK:116`).
- Inbound (`MOD:304-378` `expand`): app origin must send tickets, and raw TINs are refused. Agent origin may send TINs. A return ticket given as `tin` fills in form and code.
- Outbound (`MOD:420-533` `Scrubber`): TIN keys (`tin`, `tin_root`, `tin_display`, `branch_tin`, `root`), `tins`, return `key` values, `@bir:` tokens, and dashed TINs in text are rewritten to tickets, or `[TIN]` if the taxpayer is unsaved.
- Resolve (`MOD:644-655`): choices, open, token and error results, per `bridge.md`.

**Local database (CONFIRMED):**
- Default file: `platform::data_dir()/bir_data.db` (`CORE/db/mod.rs:356`).
- macOS data dir: `~/Library/Group Containers/group.dev.goldcoders.bir` (`CORE/platform/macos.rs:12-22`).
- Override: `BIR_DATABASE_PATH` (`db/mod.rs:362-370`).
- Encryption: SQLCipher, AES-256 (`db/mod.rs:4`).
- Master key sources: the macOS keychain (service `com.ebir.rust`, account `sqlcipher_master_key`; `db/mod.rs:237-242`), or with `BIR_NO_KEYCHAIN=1` (`CORE/lib.rs:42-47`) the `BIR_DB_KEY` (64 hex) or `BIR_DB_KEY_FILE` (`db/mod.rs:245-262`). A set but invalid value is an error, never a keychain fallback.
- One owner at a time (`MINIMOD` usage text).

**Buwiz identity (CONFIRMED):**
- The desktop account is read on every model-facing call (`Gate::Desktop`, SRV:136-141; `allow()` at SRV:780-800, which calls `signin::ensure_fresh`).
- Saved account = the newer of the keychain record and the private file, by `(last_refresh_unix, access_expires_at_unix)`. A tie goes to the keychain (`CORE/cloud_account.rs:747-758`).
- Before sign-in only `cloud_account_*` functions answer (`BRIDGE:12-14`). Everything else gets the lock message (`SHELL/account_gate.rs`).
- `BUWIZ_ACCESS_TOKEN` (stdio only) licenses tools with an agent token instead (`MINIMOD` usage; SRV:173-180).
- Device-code endpoints `POST /auth/device/start`, `POST /auth/device/poll`, `POST /oauth/token` (`cloud_account.rs:1-10`).

**Outbound HTTP (CONFIRMED list):**
- Buwiz API `https://api.buwiz.com`, account `https://account.buwiz.com`, dev default `http://localhost:3008` (`cloud_account.rs:22-29`). Overrides: `BUWIZ_SERVER_URL`, `BUWIZ_ACCOUNT_URL`, `BUWIZ_MCP_URL` (`MCP/main.rs:49-52, 90`).
- Google OAuth and Calendar: `accounts.google.com`, `oauth2.googleapis.com`, `www.googleapis.com` (`CORE/google_calendar.rs:15-19`). Gmail scope `https://mail.google.com/` (`CORE/email/auth_oauth.rs:30-35`).
- BIR e-filing dispatcher: `http://birgovph.com/tinDispatcherSFTP.php` and `http://ws2.birgovph.com/…` (`CORE/transport.rs:34-35`). These are plain http constants. Whether the payload is encrypted on the wire is UNKNOWN.
- BIR CMS and Official Gazette fetches (`CORE/news_fetcher.rs:35, 129`).
- Dormant: `CORE/integration/providers/generic.rs:19-57` GETs a configured URL with headers. No callers.
- Books (`:3000`): none. BIR has no client for it.
- Rox broker: port 8787 by default (`ROX/server/src/config.mjs:21`).

---

## 4. Data flows

**A. Open tax-profile, pick a taxpayer, see registration and forms**
1. Entry: `/taxprofile` or an `@bir:taxprofile:` token. Rox calls `bir_resolve` (resolve origin), gets choices with tickets, and the user picks one. Rox then opens tax-profile with `props.ticket` (MOD:644-655). Or the model calls `bir_app_open {app: tax-profile, tin: <ticket>}`.
2. Start: `rox.call("bir_ticket_subject", {ticket})` returns `{tin, title…}` (`AT:436-452`). There is no taxpayer switcher. With no ticket the app shows "Open this from a taxpayer" (`tax-profile.tsx:616`).
3. Reads: `get_taxpayer`, `list_tax_years`, `yearly_forms(tin, year)`, `starter_packs`, `list_form_templates`, `deadlines_between`, `list_returns`. Each is held to the ticket's TIN root by `SCOPE` (`get_taxpayer` Tin(0) at 76, `yearly_forms` at 91, list filters at 69-74). Each runs through `bir_module` into bir-core.
4. Writes: `update_taxpayer`, `create_branch`, `update_branch`, `move_branch_code`, `set_yearly_forms`, `grant_send`/`revoke_send`, `delete_form_template`, `set_taxpayer_archived`.
5. COR: bir-core models effective-dated COR profile configuration (`CORE/profile.rs:302, 559-564`). The app shows registered name and address (`tax-profile.tsx:187, 1176`) and yearly forms. The word "COR" does not appear in the app.
6. Opening a return: `rox.open({app:"forms", ticket: <branch>, props})` (`forms.tsx:547`).

**B. Open Forms, create or fill a draft, validate, save, queue**
1. Open: props `{ticket, form?, period?, action?, draft_id?}` from `/taxform` or `bir_app_open`. `bir_ticket_subject` is at `forms.tsx:3631`.
2. Schema: `screen_forms_index`, `screen_form_schema(code)` (read-only; `SFM:1491`).
3. Open draft: `screen_form_open(tin, code, year, period, key, newKey)` (`SFM:3052`; `forms.tsx:621`). Samples use `screen_form_sample` and `screen_form_sample_period`. Amendments use `screen_form_amend`.
4. Fill: the page sends `edits{box: string}` to `screen_form_fill(code, draft, edits, unlocked)` (`SFM:4458`; `forms.tsx:955, 1036, 1411, 1460, 1167`). No provenance is recorded. `screen_form_use_profile` fills from the taxpayer profile (`forms.tsx:1351`).
5. Validate: `screen_form_validate` (read-only; `SFM:5369`; `forms.tsx:975`).
6. Save: `screen_form_save` (`SFM:5415`; `forms.tsx:1423, 1467`).
7. Queue (the user's Submit): `screen_form_queue` (`SFM:6187`). It calls `queueable.queue(QueueAuthSource::Gui)` and does not check send grants. Monthly and quarterly forms use `queue_form_draft` (`forms.tsx:1596-1597`). Cancel uses `screen_form_cancel_queue` (`forms.tsx:1609`).
8. Agent path: `bir_form_fill` goes to `Session::form_fill` (`SESS:573-655`), which calls bir-core `agent::fill`, `compute`, `field_map`, `save_draft`. `bir_form_send` goes to `send_draft` (`SESS:1388-1400`). Without `db.has_send_grant` it returns `approval_required` and queues nothing.
9. Send grants: standing rows (`CORE/db/mod.rs:1032` grant, `1059` revoke, `1071` has; no expiry) or "allow once" (`SRV:455-469`, reverted by `SRV:535-546`; finding 1).

**Provenance and source enums:**
- Agent `source` (`MCP/tools.rs:307-313`): enum `profile | past_return | document | ai`, default `ai`. Described as "profile (COR facts), past_return, document (uploaded OCR), or ai". `user` is accepted only with `from_card: true`, and only for keys on the form card (`SESS:1222-1249`; finding 2).
- Per-box provenance: `OpenForm.sources` (`SESS:55`). Set on fill (`SESS:627`). Reset to empty on every open (`SESS:526`). Echoed per box in `form.fields` (`SESS:557`). Not stored in the draft. Inside one session, a `user` box is protected from non-user overwrite (`SESS:597-600`, `kept_user_boxes`).
- Payments `source`: `email | receipt | manual` (`forms.tsx:168`; `PAY:49`).
- App-typed edits: no source is recorded (`SFM:4458` body).

**bir-core APIs in these flows:** `forms::agent::{identity, fill, compute, field_map, validate, card_boxes, save_draft, definition}`, `Database::{has_send_grant, grant_send, revoke_send, get_profile_by_tin, confirm_initial_profile_year, draft_data_json}`, `QueueableForm::queue(QueueAuthSource::Gui)`, `FilingStatus::Draft`, `taxform::{parse, canonical_value, form_code_of}`, `FORM_REGISTRY`.

---

## 5. Tests and how to run (nothing here was run)

**Rust (106 `#[test]` attributes under `src/mini_app`, by grep):** `server.rs` 23, `scope.rs` 10, `resolve.rs` 9, `payments.rs` 9, `v1/tests.rs` 7, `watch.rs` 6, `v1/manifest.rs` 4, `calendar.rs` 4, `bridge.rs` 4, `v1/tickets.rs` 3, `v1/keys.rs` 3, `v1/jcs.rs` 3, `v1/app_tools.rs` 3, `taxpayer.rs` 3, `confirm.rs` 3, `plain.rs` 2, `demo_tests.rs` 2, `cards.rs` 2, `apps.rs` 2, `return_view.rs` 1. `e2e_seed` is ignored. Agent session tests (`SESS:1535-1739`) cover open, fill, validate, save and send gating.

**Node (`V1T`):**
- `apps.test.mjs`: imports, default export, no network or storage, kit verbatim. The permission test (line 83) runs only with `BIR_MINI_APP_BIN`. It checks that each app's tools are granted, no more and no fewer, and that Refresh tools are granted.
- `broker.test.mjs`: imports Rox broker modules from `ROX_BROKER_DIR`, signs the manifest with a per-run key, installs, opens certified sessions, sends signed calls. Uses a scratch DB and dry-run filing.
- Neither is referenced by `.github` or the `justfile` (CONFIRMED). The justfile has no mini-app recipes (CONFIRMED). BIR does not use the SDK's `rox-plugin-test` (CONFIRMED).

**Build and run (from file headers):**
- Build: `cargo build -p bir-shell --features agent --bin bir-mini-app`.
- Static app test: `node --test crates/bir-shell/v1-tests/apps.test.mjs`. Add `BIR_MINI_APP_BIN=<target>/debug/bir-mini-app` for the permission test.
- Broker test: `ROX_BROKER_DIR=<rox>/server BIR_MINI_APP_BIN=<bin> node --test crates/bir-shell/v1-tests/broker.test.mjs`.
- Sign: `BIR_MINI_APP=<bin> ROX_BROKER_DIR=<rox>/server crates/bir-shell/scripts/sign-v1-manifest.sh --keychain gc-2026-01 --out <signed.json>` (or `--key-file` for tests). Run with `BIR_V1_MANIFEST=<signed.json>`.
- After editing `card-kit.tsx.in`: `node scripts/sync-card-kit.mjs`, then re-sign.
- Scratch DB seed: `BIR_E2E_SEED=1 EBIR_TEST_ENV=1 BIR_DATABASE_PATH=<scratch>/bir.db cargo test --features agent --lib mini_app::e2e_seed -- --ignored`.
- Headless key: `BIR_NO_KEYCHAIN=1` with `BIR_DB_KEY_FILE=<path>`.
- Books note: `BOOKS/CLAUDE.md` says Bun only. BIR's test scripts use node, so a Books port would need Bun equivalents.

---

## 6. Books: copy versus new, and a recommendation

**Hard constraints (CONFIRMED):**
- Trust: a plugin must be on the broker's `approved_plugins` (`ROX/server/src/v1/trust.json`, BIR entry at line 13; spec §4.1: a plugin not on the list "cannot be installed in v1"). Today: BIR, google-workspace, fixture. Adding Books is a Rox-side change.
- Same-plugin authority only:
  - `rox.call` reaches only the plugin's own tools (bridge.md; spec §6.4).
  - `rox.open({app})` reaches only the plugin's own apps (spec §7.5).
  - `rox.attach` reaches only its own scopes with `attach: true` (spec §7.6).
  - A resolve `open` naming another plugin's app is refused (bridge.md resolve table; spec INV-05, line 1192).
- Cross-plugin reads exist only for routines: `reads` lists read-only tools of another approved plugin, approved once by the user (spec §5.9, line 285). A routine turn gets its own tools plus the `reads` tools only (`ROX/server/src/v1/routines.mjs:20-24`). Writes are own-plugin only.
- Routines with `writes` need the user's enable consent (`routines.mjs:139-141`).
- Names: `short` is unique among enabled plugins, and tool names start with `<short>_` (SDK `manifest.md`). Colliding command names get renamed `/<short>:<cmd>` (`plugins.mjs:421-429`).
- Card auto-mount applies only to approved first-party plugins (`publisher` `dev.goldcoders`, spec §9.8, line 775). Others get a Show click.
- Spawn: any `command` with `args` and `env` is accepted (`plugins.mjs:58-72`). Bun can serve it. Whether the Node-based SDK helper `lib/rox-plugin.mjs` runs under Bun is UNKNOWN (not tested).
- Spawn env is persisted in the broker DB (`plugins.mjs:463-466`). Keep secrets out of env, and pass key file paths rather than key values.

| concern | (a) sibling plugin `dev.goldcoders.books` | (b) new apps inside `dev.goldcoders.bir` |
|---|---|---|
| Trust | new approved_plugins entry (Rox release) | none |
| Data boundary | Books data stays in Books; tickets are per plugin (spec §9.5) | Books data enters BIR's process and its SQLCipher ticket store; BIR has no Books client |
| Sign-in | Books' own auth (Better Auth; `BOOKS/.env.example:54`; tenant RLS per `BOOKS/CLAUDE.md`) | inherits BIR's Buwiz gate (`BRIDGE`, account_gate) |
| App-to-app | cross-plugin needs a consent link | `rox.open({app})` with no prompt |
| Budget | own 16 apps, 32 commands, 8 routines | BIR uses 13/16 apps, 2/8 routines, 3/16 scopes; forms uses 36/64 tools |
| Release | own signature and version | every change re-signs BIR (`M:424-441`) |
| Network | Books client in its own process | a new outbound client in BIR (none today) |

**Recommendation: (a).** The data boundary, the sign-in model, the manifest budget and the re-signing coupling all point the same way. The cost is one trust-store approval and user-mediated hand-offs. Evidence is above. Testing before Rox approves can use a per-run trust file, as `broker.test.mjs` and the SDK `testing.md` do.

**Copy from BIR (the pattern):**
- Manifest builder: `AppSpec`, `unsigned()`, `fingerprint()`, JCS, the sign script, and the staleness check.
- Reserved-tool handling, session certificates and plugin key (`MOD`, `KEYS`).
- Ticket store design (`TK`): random tickets, subject map, live titles, move and forget.
- App-tool pattern: `<short>_app_<fn>` with `args_json` and `ticket`, a per-function scope rule table (`SCOPE`), `prepare()`/`finish()`, and a scrubber.
- App code pattern: a `bir()` wrapper, `rox.props` ticket, `rox.call` only, the card kit, and the static permission test.
- Routine pattern: query tool, prompt, `reads`, `writes`, `params`.
- Card pattern: `_meta["rox/open"]` with snapshot, `as_of`, `tool` and `args`, plus `rox.fit`.

**New for Books:** its stdio MCP server; its own Books API client and auth; its own ticket store keyed on Books IDs; its own apps and card apps; its trust entry and signing; its own test harness.

**Cross-plugin linking (answers):**
- A Books app opening a BIR app with `rox.open({app:"tax-profile"})`: refused (spec §7.5, same plugin).
- The only cross-plugin open is a `rox://app/dev.goldcoders.bir/<app>?ticket=<BIR ticket>` link (spec §9.4). Consent is required every time (spec §9.7). Books cannot mint BIR tickets (random, issued only by BIR, `TK:186-227`), so Books can only show a link the user got from BIR.
- A Books app attaching a BIR `taxprofile` or `taxform` token: refused (spec §7.6). BIR tokens enter the chat only through BIR's own resolver, when the user picks in Rox's composer.
- A Books app calling BIR tools: refused.
- A Books routine reading BIR: possible via `reads` (read-only BIR tools, approved once by the user). BIR's `authorize` allows read-only tools (`SRV:554-569`). End-to-end behavior is UNKNOWN until tested.
- Practical hand-off: both plugins in one chat. The model reads Books output and calls BIR tools. Books output should carry Books IDs only.
- Identity: Books uses Better Auth; BIR uses Buwiz accounts. Whether the two identities are linked is UNKNOWN.

**How BIR's Forms app could receive Books-proposed values:**
- No channel in `rox.open` props. The forms props schema has only `ticket`, `form`, `period`, `action`, `draft_id`, `form_code`, `year` and `key` (`M:133-145`). Links cap props at 1024 bytes (spec §9.4).
- Via the model: `bir_form_fill {fields, source: "document"}`. The assistant hint already prescribes this for attached spreadsheets (`TAXFORM_HINT`, `M:257-264`). Do not send `from_card` or `user` (finding 2). The user reviews in the Forms app and presses Submit. Agent-side submission still needs a send grant (`SESS:1396`).
- Via sales-sheet import: `bir_card_import_fill` writes one total into 2551Q `txtATCAmt1`, 1701Q `frm1701q:txt36A`, or 2550Q `frm2550qv2024:vatableSales` (`RV:378-384, 631-656`). This bypasses session provenance.
- Via bir-core sync: `process_sync_json` (`CORE/integration/service.rs:83-86`) takes `UniversalTaxPayload` (`models.rs:82-115`, with `f64` money fields). It maps only 2551Q (`mapper.rs:308`) and has no callers. Not a drop-in (Books rule: money as strings).
- Via a new BIR host function: needs a scope rule, a tool entry, a manifest permission and a re-sign. It would give the cleanest provenance, by adding a new source value to `FILL_SOURCES` (`tools.rs:90`) and `fill_source` (`SESS:1222`).

---

## 7. Existing hooks where Books data could enter a draft

- Agent fill source (`MCP/tools.rs:307-313`): `"document"` is described as "uploaded OCR". `from_card` and `user` are described as the user's own answers to `bir_form_card`.
- Assistant hint (`M:257-264`, `TAXFORM_HINT`): "Offer to read an attached Excel or CSV and fill from it with bir_form_fill (`source: document`)."
- Sales-sheet import (`RV:1-12`): "`bir_card_import` (read a picked .xlsx or .csv; reads only) and `bir_card_import_fill` (put the import's total on the return's draft)." The box map is at `RV:378-384`. The sheet's dates and amounts are read at `RV:562-630`.
- Core sync (`CORE/integration/service.rs:83-86`): "Process a typed `UniversalTaxPayload` … (e.g., from a URL scheme with query parameters)". No MCP or app caller (grep across all crates finds only re-exports).
- card-money: receipts and payments only (`bir_record_payment` with `channel`, `amount`, `paid_at`). Not a draft source.
- card-calc: lookups only (`bir_lookup`), not a draft source.
- `bir_record_payment.amount` is `["number", "string"]` (`SRV` payment tools), so a Books payment record can pass its amount as a decimal string.

---

## 8. Findings (ranked)

1. **CONFIRMED (code path). "Allow once" send grant outlives the process.** `allow_send_once` writes a standing DB grant (`SRV:462`) and keeps the pair only in memory (`once_grants`, `SRV:167, 468`). `end_once_grants` revokes only after a successful `bir_form_send` (`SRV:386-389, 535-546`). `grant_send` has no expiry (`CORE/db/mod.rs:1032-1058`). After a restart, or after a failed send, the grant stays standing until the user revokes it. No startup cleanup exists (`revoke_send` callers: `TPM:452`, `SRV:542`, and a test at `SESS:1713`).
2. **CONFIRMED (code path). The model can claim `user` provenance.** `fill_source` with `from_card: true` and `source: "user"` checks only that the keys are on the form card (`SESS:1222-1249`). `bir_form_card` is not exposed through MCP (`SRV:45-57`), so no card is ever shown first. Keep Books-proposed values away from `user`.
3. **CONFIRMED. Provenance is session-only.** `sources` is reset on every open (`SESS:526`) and never persisted. Card imports and app edits record no source (`RV:631-656`; `SFM:4458`).
4. **CONFIRMED. The permission guard is weaker than documented.** `M:16` names a missing test. `apps.test.mjs:83` skips without `BIR_MINI_APP_BIN`. No CI runs it.
5. **CONFIRMED (dead code, drift). The kit's upload helper is copied into 8 card apps but granted in only some.** `card-kit.tsx.in` `uploadFile` calls `rox.pickFile` and `rox.call("bir_app_upload")`. Five card apps (card-calc, card-dates, card-forms, card-help, card-people) lack `pick_file` and `bir_app_upload` in their permissions. `apps.test.mjs:42` strips the kit from the check, so the test misses it. No call sites exist in those five, so it is unreachable, but it is a trap.
6. **PARTIAL. Free-text TIN scrubbing has a gap.** `Scrubber::text` rewrites `@bir` tokens and dashed TINs only (`MOD:516-533`). Undashed digit runs in other string fields are not rewritten. `assert_no_tin` (`SRV:2584-2600`) checks only card answers in tests.
7. **MISSING. No broker-side TIN scrub in Rox v1.** `ROX/server/src/v1` contains no scrubber.
8. **UNKNOWN. `rox://view?path=` links** (`PAY:1465`; `SRV:1670`) are not defined in the v1 spec. Spec §9.7 says `rox://` links other than `app` are never opened from plugin output.
9. **CONFIRMED (minor). `bir_resolve` is described as Rox-only** ("Do not call it yourself", `SRV:1429`), but `authorize()` allows agent calls (`SRV:554-595`). It is read-only, so there is no write risk.
10. **CONFIRMED. Spawn env is persisted in the broker DB** (`plugins.mjs:463-466`). Do not put `BIR_DB_KEY` or `BUWIZ_ACCESS_TOKEN` in env. Pass a key file path instead.
11. **CONFIRMED. BIR's dispatcher constants are plain `http://`** (`CORE/transport.rs:34-35`). Wire encryption is UNKNOWN.
12. **CONFIRMED. Documentation drift.** SDK `manifest.md:53` is stale on routine keys (`params` is valid per `manifest.mjs:267` and spec lines 319-324). Spec §5.10 is an old example (version 2.0.0).
13. **CONFIRMED. The core sync path is unwired and uses `f64` money** (`models.rs:82-115`), against Books' money-as-text rule (`BOOKS/CLAUDE.md`).
14. **PARTIAL.** Each model-facing call runs `signin::ensure_fresh` (`SRV:780-800`), which can refresh the Buwiz session over the network. Frequency is UNKNOWN.
15. **UNKNOWN.** Whether an open Forms mini-app shows agent fills live. `form_fill` queues `op: choose` commands for the desktop UI (`SESS` near 640-645), but the Rox path was not traced.
16. **UNKNOWN.** Which runtime module provides `<If>` and `<Each>`.
17. **MINOR.** The `taxform` scope description yields the noun `return:` with a colon (`M:358`).