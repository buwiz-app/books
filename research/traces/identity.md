# Track A: identity, accounts, tax profiles, access passes

Scope: buwiz-server, the buwiz-oss plan, Buwiz Books auth and org/tax schema, the BIR desktop and Rox plugin identity code, and the Rox v1 broker. Labels: **CONFIRMED** (seen in code or docs), **PARTIAL**, **MISSING** (searched, not found), **UNKNOWN**. INFERENCE marks reasoning. No tests were run. The only live check was an `lsof` listener check. No TIN or ticket value is printed.

## 0. Repos read (git HEAD)

| Repo | Path | HEAD | Branch and state |
|---|---|---|---|
| buwiz-server | /Volumes/goldcoders/buwiz-server | 417ec3a | main, clean (matches handover) |
| buwiz-oss | /Volumes/goldcoders/buwiz-oss | 0f34106 | main, clean. `internal/` is gitignored and was read |
| Buwiz Books | /Volumes/goldcoders/OSS/buwiz-books | 1662ca3 | main. Four pre-existing uncommitted entries, not mine, untouched |
| BIR desktop and Rox plugin source | /Volumes/goldcoders/reverse-engineer-ebir-forms/bir | 29ca915b | rox-mini-app-round2 (handover says mini-app-v1; same commit as tag oss-cut-base) |
| Buwiz Forms | /Volumes/goldcoders/OSS/buwiz-forms | 301f9a0f | main (handover's 63267ca not checked out). Read docs/tax-profile/CONTEXT.md and the tax_profile diff only |
| Rox | /Volumes/goldcoders/Projects/rox | 8453d53 | perf-parity, clean. Handover head e3cdc18 is an ancestor |

## 1. Bottom line

1. **"buwiz-cloud" does not exist (MISSING).** Searched: directory names under /Volumes/goldcoders to depth 4; `for-each-ref` and `worktree list` for "cloud" in five repos; content grep and `git log --all --grep` for "buwiz-cloud" in five repos. No hits. The nearest real artifact is buwiz-server, a Spin/WASM app.
2. **Two designs share the name "buwiz-server", and they conflict.**
   - Implemented (buwiz-server repo): accounts, tax profiles, passes, agents, device-filing queue. It forbids SFTP in this service (/Volumes/goldcoders/buwiz-server/docs/extensions.md:20-30).
   - Planned (buwiz-oss): Gatekeeper and Sender, signed 5-minute tickets, HPKE, SFTP to BIR (/Volumes/goldcoders/buwiz-oss/internal/PLAN.md:146-176). It assigns SFTP to buwiz-server (PLAN.md:69). P5 is `todo` (/Volumes/goldcoders/buwiz-oss/internal/PROGRESS.md:70-76). /Volumes/goldcoders/buwiz-oss/crates/bir-submit/src/lib.rs is 8 lines.
   - Cloud host conflicts: buwiz-oss D15 says Google Cloud (DECISIONS.md:21). buwiz-server says Fermyon Spin (README.md:9; DEPLOY.md).
3. **Identity today is a human Buwiz account on the BIR desktop, not in Rox.** The plugin signs in by device flow. Every BIR data verb is refused while signed out (/Volumes/goldcoders/reverse-engineer-ebir-forms/bir/crates/bir-shell/src/account_gate.rs:19, 63). Rox has no account. `_meta["rox/caller"]` carries plugin, app, open_id and ts only (/Volumes/goldcoders/Projects/rox/server/src/v1/sessions.mjs:293).
4. **A server tax profile is one registration unit** (TIN root plus 5-digit branch), keyed by UUID. Exclusivity is HMAC-SHA256(pepper, root|branch) (/Volumes/goldcoders/buwiz-server/src/domain/tin_identity.rs:19). The pepper derives from AUTH_ROOT_KEY_BASE64 (/Volumes/goldcoders/buwiz-server/src/auth_product/runtime.rs:109, 142). Every API DTO carries `tin_last4` only (/Volumes/goldcoders/buwiz-server/src/contracts/tax_profile.rs:1-5).
5. **Plaintext TIN persists in two server places.** `device_filings.tin` (/Volumes/goldcoders/buwiz-server/migrations/postgres/0014_device_filings.sql:15) has no purge path. Legacy `tax_profiles.tin_root` (0002:16) is never cleared by a migration. It is cleared only by reprojection of event-backed rows.
6. **Access model is application code only.** Owner always wins. Firms reach a profile only through a pass, with 10 permissions and `filings.publish` as a separate switch. Also: firm drafts, owner invites via verified email, per-connection agents. buwiz-server has no Postgres RLS (grep of migrations and src: none).
7. **Tickets (`tkt_`) are plugin-owned.** BIR mints 130-bit random tickets. The ticket-to-subject map sits in one settings row of BIR's own database (/Volumes/goldcoders/reverse-engineer-ebir-forms/bir/crates/bir-shell/src/mini_app/v1/tickets.rs:28). Rox validates format and stores tokens and titles. Tickets never expire and have no per-ticket revoke. Spec §9.5 requires plugin revocation, so BIR's missing revoke is MISSING.
8. **Books:** better-auth, organization as tenant, RLS by org. The org's TIN is plaintext in `org_tax_profiles.tin` (/Volumes/goldcoders/OSS/buwiz-books/src/db/schema/tax-reference.ts:168) and returned unmasked by `getTaxSettings` (/Volumes/goldcoders/OSS/buwiz-books/src/routes/api/-tax-settings.ts:13-17). Firm and accountant delegation tables exist (src/db/schema/inbox.ts:77-126) but no app code uses them and no permission check reads the `firm` resource. Schema only.
9. **No identifier is shared** across Books, wasi-auth, the desktop and Rox. Books-to-server link: MISSING. Desktop-to-server `profile_id`: MISSING (the desktop never stores one). Desktop tax-profile sync (years, drafts, filings): MISSING on the client. The server API exists (/Volumes/goldcoders/buwiz-server/docs/sync-api.md; HANDOVER.md "Done" item 4).
10. **Local dev is live.** `spin` (pid 32259) listens on 127.0.0.1:3008 (lsof). Rox's log reports the BIR plugin signing in through that address (/Volumes/goldcoders/Projects/rox/drafts/mini-app-goal-status.md:218). Production `api.buwiz.com`: UNKNOWN. Not probed. DEPLOY.md says DNS and deploy are not applied.

## 2. Findings by question

### 2.1 Account model

| Component | Mechanism | Status and evidence |
|---|---|---|
| buwiz-server accounts | wasi-auth `=0.1.0-rc.5` (/Volumes/goldcoders/buwiz-server/README.md "Version pins", ~L455); auth_users, auth_organizations | CONFIRMED |
| Email gate for tax-profile writes | `require_verified_actor` refuses `pending_verification` and `disabled` (/Volumes/goldcoders/buwiz-server/src/application/tax_profile.rs:312; src/store/tax_profile.rs:1056 `user_status`) | CONFIRMED |
| Workspace membership | `organization_for_session` per request (src/auth_product/organization.rs:182), via `org_permissions` (src/application/tax_profile.rs:1014). Nothing cached on the profile | CONFIRMED |
| Desktop sign-in | device flow: `start_device` (/Volumes/goldcoders/reverse-engineer-ebir-forms/bir/crates/bir-core/src/cloud_account.rs:506), poll (550), `refresh_tokens` (587, `/oauth/token`), `load_profile` (617, `/me`), `revoke_session` (710) | CONFIRMED |
| Desktop refresh token | keychain service `com.buwiz.bir`, account `desktop-refresh` (cloud_account.rs:33-35). Fallback file `buwiz-cloud-account.json` beside the DB, chmod 0600 (cloud_account.rs:38, 855, 867) | CONFIRMED |
| Desktop DB key | `BIR_DB_KEY` or `BIR_DB_KEY_FILE`, else keychain (/Volumes/goldcoders/reverse-engineer-ebir-forms/bir/crates/bir-core/src/db/mod.rs:249, 663) | CONFIRMED |
| Desktop data gate | `ensure_unlocked` (bir-shell/src/account_gate.rs:63). Non-account BIR functions refused while signed out (bir-shell/src/mini_app/bridge.rs:63) | CONFIRMED |
| Desktop license for mini-app tools | every model-facing tool refreshes the session, then requires `GET /api/tax-profiles` to show a profile with permissions (/Volumes/goldcoders/reverse-engineer-ebir-forms/bir/crates/bir-mcp/src/license.rs:291, 306, 327; bir-shell/src/mini_app/server.rs:780-790) | CONFIRMED |
| Agent identity | OAuth DCR; own `bwa_`/`bwr_` tokens, hashed; refused on user routes (README "AI agents"; migration 0007 header) | CONFIRMED (docs and migration; token code not traced) |
| Books | better-auth (/Volumes/goldcoders/OSS/buwiz-books/src/lib/auth.ts:62); emailAndPassword disabled (:73); Google (:78); emailOTP (:146); INVITE_ONLY (:26, :48, :84). Plugins: organization and emailOTP (:3) | CONFIRMED |
| Books role | role read from live `auth_members` on each request (src/lib/auth-middleware.ts:54; src/lib/session-organization-policy.ts:29). Cookie role not trusted | CONFIRMED |
| Rox user identity | none. Plugin Ed25519 key signs a session certificate per app open. Session key signs each call (/Volumes/goldcoders/Projects/rox/drafts/rox-mini-app-protocol-v1.md §4.3–4.4, lines 120–156). Broker checks at server/src/v1/sessions.mjs:146–224 | CONFIRMED |
| Rox broker listener | `X-Rox-Host` token, 32 random bytes, 0600 file (/Volumes/goldcoders/Projects/rox/server/src/v1/routes.mjs:5-10, 23-40). Origin and JSON guard (/Volumes/goldcoders/Projects/rox/server/src/guard.mjs:1-12) | CONFIRMED |
| Rox hosted account | "Rox to Hub: account sign-in" marked Planned (/Volumes/goldcoders/Projects/rox/drafts/rox-server-architecture.md:59) | CONFIRMED (doc) |

### 2.2 Business entities and the identity crosswalk

| Concept | buwiz-server | Books | BIR desktop | Rox |
|---|---|---|---|---|
| Human | wasi `user_id` (UUID) | `auth_users.id` (text) | local OS user plus Buwiz account | none |
| Firm or workspace | wasi organization UUID. Pass grantee `PassGrantee::Organization` (src/domain/pass.rs:265). Publishers in `firm_publishers` (migration 0008:39) | `auth_organizations` (text). `firms`, `firm_members`, `firm_clients`, `firm_member_client_access` (src/db/schema/inbox.ts:77, 86, 106, 126). Unused | none | none |
| Client business | `AccountHolder::Organization` for company-managed profiles (src/domain/tax_profile.rs:25) | org as tenant; `org_tax_profiles` 1:1 (tax-reference.ts:163) | `Taxpayer` (bir-core/src/tax_profile/mod.rs:158) | none |
| Taxpayer (9-digit root) | no aggregate. One TaxProfile per (root, branch) | TIN in `org_tax_profiles.tin` | `tin_root` | ticket subject `tp:<root>` |
| Branch or registration unit | TaxProfile. `RegistrationKey` (src/domain/tin.rs:68) | `org_tax_branches` (tax-reference.ts:208-215; unique org+branch) | `BranchFacts` (mod.rs:218) | `br:<14 digits>` |
| Return or draft | key = (profile, form_code, tax_year, period_key) (docs/sync-api.md) | none | forms set and drafts | `rt:<canonical return>` |

INFERENCE: no system records another system's ID. The only recorded link is Books' organization to its own TIN.

### 2.3 Tax profile: who owns each value

| Value | Authoritative owner | Other copies |
|---|---|---|
| TIN root, name, taxpayer type, birth date | desktop `Taxpayer` (bir-core mod.rs:158) | server `CloudTaxProfileFacts` (src/domain/tax_profile.rs:173-190), written only through the API; Books `org_tax_profiles.tin` and `registeredName` (tax-reference.ts:168) |
| Branch facts (RDO, address, phone, line of business, VAT) | desktop `BranchFacts` (mod.rs:218) | server per-profile facts; Books `org_tax_branches.rdoCode` and `name` |
| Classification and EOPT tier | none. Three owners | desktop `setup("eopt_tier")` (bir-core/src/tax_profile/field_kinds.rs:253); server `eopt_tier` and `tax_classification` (domain/tax_profile.rs:173-190); Books `taxpayerClassification` (tax-reference.ts:163-200; drives EOPT penalties) |
| Elections (regime, item 13) | desktop templates (bir-core/src/tax_profile/templates.rs:300-401) | Books `orgTaxYearElections` (read in -tax-settings.ts); server none |
| Forms set per year | desktop `forms_set.rs` (`FormSetSource`: Manual, CorAi, ReviewedCor, InferredTaxType, MigrationBackfill) | server `per_year_forms` (migration 0004) through the sync API |
| Ownership, passes, claims | server only | none |
| PIN and TOTP | desktop reverse-engineer copy has `TaxpayerCredentials` (mod.rs:82). buwiz-oss removed it (DECISIONS D18, D20) | server refuses secret keys in drafts (src/domain/records.rs:104-147) |

### 2.4 TIN: where it lives and where it leaves

| Location | Stored form | Egress | Status |
|---|---|---|---|
| server `tax_profiles` | `tin_hash` BYTEA (0004:34), `tin_last4` | DTOs show `tin_last4` only (src/contracts/tax_profile.rs:10, 149, 211) | CONFIRMED |
| server `tax_profiles.tin_root` | plaintext CHAR(9) (0002:16) | not in any API view. Migration 0003 only reads it (0003:19). Cleared on write (src/store/tax_profile.rs:455). Admin reproject clears event-backed rows only (src/store/tax_profile.rs:308, 343; src/application/admin.rs:68) | CONFIRMED. Presence in any real DB: UNKNOWN |
| server `device_filings.tin` | plaintext `text NOT NULL` (0014:15) | owner's desktop lists it (src/store/agent.rs:725). Written when the agent supplies the full TIN. Server checks last four digits only (src/application/agent.rs:72-95). No DELETE or purge in src | CONFIRMED |
| desktop local DB | encrypted (SQLCipher key) | apps get the TIN from `bir_ticket_subject` (app-only; tickets.rs:144 `describe`). Model cannot call it (server.rs:1191 `is_app_only`) | CONFIRMED |
| model-facing bir-mcp tools | `tin` argument takes a full TIN (bir-mcp/src/tools.rs:287, 491-517) | whether results echo the full TIN: UNKNOWN |
| Rox | tokens, tickets, titles (records.rs:25, 39). Titles are taxpayer names (tickets.rs:116) | CONFIRMED |
| Books | plaintext org TIN (tax-reference.ts:168) and party TINs (src/db/schema/party-tax.ts:71) | unmasked to members (-tax-settings.ts:17). AI redaction on main has no TIN rule (src/lib/ai/redact.ts:1-20). PH TIN redaction exists only on branch `fix/ai-privacy-ph-ids` (commit 5341f63), not in main history |
| Form 2303 upload | AES-256-GCM at rest (src/store/ownership_claim.rs:76-83). Ciphertext cleared on close (:250) | sent to a vision model during AI review. Reader shape defaults to anthropic when unset (src/claim_ai/config.rs:73); classifier defaults to jev (:104). Egress needs a resolved key. Production keys: UNKNOWN. Local .env has `AI_GATEWAY_*` names; values not read |
| URLs | invite secret in the fragment (`#t=`). TIN only in the request body (README "Tax profiles") | CONFIRMED |

### 2.5 TIN claim and verification

- **Create** (CONFIRMED). TIN attested in the body. `TinRoot` is 9 digits (src/domain/tin.rs:8). `BranchCode` is 5 digits, with 3-digit legacy padding (tin.rs:38). HMAC (tin_identity.rs:19). Existence pre-check (src/store/tax_profile.rs:99). Then `RegisterTaxProfile` (src/domain/tax_profile.rs:211). Global `UNIQUE(tin_hash)` (0004:77-79) and `UNIQUE(account_id, tin_hash)` (0004:81-83).
- **ORUS claim** (MISSING in production). `attested_existing` calls `verify_tin_ownership` (src/application/tax_profile.rs:1101). `FakeOrusVerifier` only when `ORUS_FAKE_VERIFIER` is set (spin.toml:106, default "false"; local .env value "false", read). Otherwise `UnconfiguredOrusVerifier` returns `NotConfigured` (src/domain/orus.rs:81). The production adapter is stubbed (README "What is stubbed").
- **Form 2303 claim** (CONFIRMED). Start, then 4 KiB parts, then a provenance screen that refuses AI-marked files. Staff approve with `system.user.manage` and step-up (README). Approval calls `ProofMethod::Form2303Review` (src/application/ownership_claim.rs:356, 361). The claim runs and every pass and invite ends.
- **Device filing** (PARTIAL). Last-four match only (src/application/agent.rs:72-95).
- **Desktop** (CONFIRMED by absence). No TIN proof. Trusts the local profile.
- **Books** (CONFIRMED by absence in the tax-settings path, not exhaustive). Org TIN is self-declared.

### 2.6 Access passes, delegation, agents

- **Owner** (CONFIRMED). `ProfileAccess::Owner` with all permissions (src/domain/tax_profile.rs:604-616).
- **Firm draft with no taxpayer owner** (CONFIRMED). `HoldingOrg` gets the Full preset, without publish or share (domain/tax_profile.rs:604-616; `allows` at :643).
- **Pass** (CONFIRMED). `Pass` (src/domain/pass.rs:321). Statuses: requested, offered, active, declined, revoked, expired (pass.rs:284). One open pass per grantee per profile (migration 0006:30). Usable = active and not past `ends_on` (pass.rs:336). Permissions are unioned across member orgs in `access_for` (src/domain/tax_profile.rs:1550).
- **Permissions** (CONFIRMED). Ten: `profile.view`, `profile.edit`, `years.view`, `years.manage`, `drafts.view`, `drafts.save`, `drafts.update`, `drafts.review`, `filings.view`, `filings.publish` (pass.rs:17). Presets: read_only, safe (default), full (pass.rs:207-262). `from_choice` refuses `filings.publish` in a list (pass.rs:166).
- **Publishing** (CONFIRMED). Needs the owner's switch AND the firm's publisher pick (`effective_access`, domain/tax_profile.rs:1819; `publisher_orgs`, store/tax_profile.rs:632; `firm_publishers` 0008:39).
- **Creation** (CONFIRMED). Owner offers (`offer_pass`), or the workspace asks with TIN and branch (`ask_for_pass`, application/tax_profile.rs:405), then the owner approves (`approve_pass`, :477). Accepting needs `organization.update`.
- **Expiry** (CONFIRMED). `ends_on` is a Philippine-time date. `PassExpired` is recorded on the next pass action (domain/tax_profile.rs `expiry_events`, ~787; migration 0011).
- **Invites** (CONFIRMED). `invite_owner` (application/tax_profile.rs:683). Token stored as SHA-256. Accept requires a matching verified email (:818). Lifetime 1–14 days (src/domain/invite.rs:13).
- **Agents** (PARTIAL). Per connection: permissions, `may_queue` (default false; migration 0014:7-8), profile list, hashed tokens. A password change or security change ends the connection (README; migration 0007). Effective rights are the connection intersected with live access (README "AI agents"). The narrowing function was not traced in full.
- **Agent writes** (CONFIRMED). `queue_on_device` is the only write (/Volumes/goldcoders/buwiz-server/src/mcp/protocol.rs:25-30, 257; src/mcp/mod.rs:172). It requires the full `tin` (protocol.rs required list). The owner's app validates and uploads only with a standing OK.
- **Multi-client accountant** (INFERENCE). One workspace can hold many passes, one per profile. Each branch is its own profile, so a multi-branch taxpayer needs one pass per branch.
- **Books firm model** (MISSING enforcement; PARTIAL schema). No app references to the firm tables (src grep). The `firm` statement appears only in permissions and roles (/Volumes/goldcoders/OSS/buwiz-books/src/lib/permissions.ts:44, 72, 97, 124, 152, 183). No route checks it. RLS uses the tenant predicate (drizzle/rls_policies.sql:575) plus `firm_member_access` and `own_firm_membership` (src/lib/migrations/verifiers/0019.ts:859, 869). INFERENCE: the tenant predicate on `firm_clients` blocks a firm-wide client list under one org context.

### 2.7 Consent and revocation

| Event | Effect | Status |
|---|---|---|
| Owner ends or changes a pass | access stops on the next call (access evaluated per request) | CONFIRMED |
| Workspace declines, withdraws, or gives back | `handle_end_pass` (domain/tax_profile.rs:2006) | CONFIRMED |
| Claim or reclaim | ends every open pass and invite (`end_open_passes`, domain/tax_profile.rs:1284; README) | CONFIRMED |
| Archive | grantees get nothing (`access_for`, domain/tax_profile.rs:1550) | CONFIRMED |
| Owner ends an agent connection | `POST /api/agents/{id}/revoke` → `revoke_agent_connection` (application/agent.rs:130) | CONFIRMED |
| Password or security change; account disabled | ends agent tokens (`security_revision`; README; migration 0007) | CONFIRMED (docs and migration) |
| Desktop device filing | requires a standing send OK on the computer. Otherwise `approval_required` (bir-shell/src/bir_module.rs:2314-2396) | CONFIRMED (gate referenced; accept function body not fully read) |
| Rox app open | consent table and per-tool switch (spec §8.3, §9.7) | PARTIAL (spec and consent.rs header only) |
| Ticket revocation | spec §9.5 permits it. BIR has only uninstall (`forget_all`, tickets.rs:302), TIN move (`move_root`, :277), and a deleted taxpayer (title None, so resolve returns `valid: false`, tickets.rs:116 and v1/mod.rs:277-289) | MISSING |
| Books firm consent | none | MISSING |

### 2.8 Tickets (Rox v1): issuance and validation

- **Grammar** (CONFIRMED). `^tkt_[0-9A-HJKMNP-TV-Z]{26}$` (spec §3; Rox app/src/mini_v1/perms.rs:254-257; tickets.rs:179).
- **Mint** (CONFIRMED). `tickets::issue` reuses one ticket per subject (tickets.rs:215). `issue_all` (:229) serves lists. `bir_ticket_issue` for a same-taxpayer branch checks scope (app_tools.rs:455-476). App-only.
- **Entropy** (CONFIRMED). 17 bytes from `getrandom`, rendered as 26 Crockford characters (tickets.rs:186-200).
- **Storage** (CONFIRMED). One settings row `rox.v1.tickets` (tickets.rs:28). Maps ticket to `tp:<root>`, `br:<14 digits>` or `rt:<canonical return>`. Lives in BIR's database and survives restart.
- **Validation** (CONFIRMED). `subject()` (tickets.rs:265) returns None for unknown tickets. App calls resolve through `subject_of` (app_tools.rs:325). A root check runs before the tool (app_tools.rs:355-372). `rox.ticket.resolve` returns `{valid, title}` only (v1/mod.rs:277).
- **Rox side** (CONFIRMED). Rox never mints tickets. app/ and server/ contain only validators and tests. Rox stores `Attachment` and `Pin` records (records.rs:25, 39). The installed `mini-v1.json` has one attachment and four pins. I checked structure and lengths only.
- **Lifetime** (CONFIRMED). No expiry field.
- **Scope** (CONFIRMED for app calls, UNKNOWN for model calls). Root scope comes from `scope.rs:192` `root_of`. `MiniServer::authorize` (server.rs:554-585) gates origin, manifest and app-only rules. Root scoping of model-origin calls: UNKNOWN.

### 2.9 Permission scopes by system

| System | Scope unit | Rules | Status |
|---|---|---|---|
| buwiz-server REST | profile x pass permissions | `ProfileAction` View=`profile.view`, Edit=`profile.edit`, Archive=Owner or HoldingOrg, Share=Owner only (domain/tax_profile.rs:618-660) | CONFIRMED |
| buwiz-server agents | connection presets. Never publish. Queue only | protocol.rs:25-30 | CONFIRMED |
| BIR mini-app host calls | TIN root from the ticket | `Rule` enum with 8 kinds (scope.rs:16-35). Table at :37. Closed by default (app_tools.rs:360). App-wide `Refuse` list (scope.rs ~150-180) | CONFIRMED |
| BIR origin rules | resolve and routine calls read-only, routine writes must be listed. App opens must be certified. App tools must be in the manifest | server.rs:554-585 | CONFIRMED |
| Rox | per-app manifest permissions (tools, open, attach, pick_file, storage, preview, copy). Per-open digest. Per-tool switch | spec §5, §7, §8.2–8.3; sessions.mjs:224 | CONFIRMED (spec and broker) |
| Books | ABAC statements (permissions.ts) and five roles: superuser (alias owner), admin, member, clientApprover, reportViewer (roles export, :187) | CONFIRMED |

### 2.10 Books auth vs buwiz-server auth

| Dimension | Books | buwiz-server |
|---|---|---|
| Identity | better-auth; Google OAuth; email OTP; no passwords | wasi-auth; argon2id default (spin.toml variables); passkeys, TOTP, OAuth providers (names in .env) |
| Tenancy | Postgres RLS: `current_organization_id() IS NULL OR organization_id = …` (drizzle/rls_policies.sql:1-25). Null-bypass documented in CLAUDE.md | application gatekeeper only. No RLS in migrations or src |
| Role source | live `auth_members` read per request | live wasi org permissions per request (`org_permissions`) |
| Secrets | `organization_secrets` table | env and Spin variables. Keys derived from AUTH_ROOT_KEY_BASE64 |
| Agents | none | OAuth DCR agents with `bwa_` tokens |

### 2.11 Spin/WASM deployment model (buwiz-server)

- **Target** (CONFIRMED). One Rust crate built to `wasm32-wasip2` (README:9; spin.toml:153-156).
- **Components** (CONFIRMED). All share one wasm binary. `auth-api` (spin.toml:153) handles `/api/auth/…` and `/api/ui/…` (:125-133). `authorization-api` (:295) handles `/api/authorization/…` (:135). `auth-ui` (:432) is the catch-all `/…` (:148). Static file components serve `/pkg` (:569) and `/assets` (:573).
- **Outbound and KV** (CONFIRMED). The dev manifest allows `https://*:*`, `http://*:*`, `postgres://*:*`, `redis://*:*` (:156-172). The production example enumerates hosts. KV store `default` (:172).
- **Database** (CONFIRMED). Postgres through the vendored spin-sdk, blocking `spin:postgres@4.2`. Zero-row queries hang on a mismatched host (README troubleshooting; docs/integrations.md). Migrations run through the native `wasi-auth-migrate` (DEPLOY.md; src/bin/wasi-auth-migrate.rs).
- **Redis** (CONFIRMED). Wake-only pub/sub, fail-open (README "Redis wake").
- **Pins** (CONFIRMED). wasi-auth `=0.1.0-rc.5`; leptos-wasi-runtime `=0.4.2-rc.1`; ddd_cqrs_es git `71ac721`; spin-sdk git `a02d330`; Rust 1.93.0 (README "Version pins").
- **Transport limits** (CONFIRMED in docs; production behavior UNKNOWN). Request bodies do not stream on current Spin. The browser shell uses a 6000-character header shim. 2303 uploads use 4 KiB parts. Desktop draft payloads may exceed the limit (docs/sync-api.md "Transport limits").
- **Hosting** (UNKNOWN for production). DEPLOY.md and README point to Fermyon Cloud. `scripts/deploy.sh` is a dry run unless `BUWIZ_DEPLOY_CONFIRM` is set. buwiz-oss D15 says Google Cloud.
- **Desktop URLs** (CONFIRMED). Release `https://api.buwiz.com` (bir-core cloud_account.rs:22), account `https://account.buwiz.com` (:25). Debug `http://localhost:3008` (:27).
- **Local flags** (names and booleans only). `ORUS_FAKE_VERIFIER=false` and `AUTH_PRODUCTION_MODE=false` in local .env.

## 3. Where a Books business profile links to a PH tax profile

**Today: nothing links them.** The natural anchor is the Books org (`organization_id`) with its `org_tax_profiles` row, which already holds the TIN. The server's registration unit is a separate entity, and Books holds no reference to it.

The only existing path that gets a server `profile_id` without repeating the TIN later is `RequestPass`. It takes `tin_root` and `branch_code` once, in the body, then needs owner approval (src/application/tax_profile.rs:405). INFERENCE: store the returned `profile_id` on the Books side (new column, MISSING). Never join on TIN equality. The desktop cannot compute the server HMAC without the pepper.

**Cross-boundary IDs (proposal, labeled):**

| Boundary | Safe key | Must not cross | Status today |
|---|---|---|---|
| Rox to BIR plugin | `tkt_` ticket, canonical token | TIN, branch TIN, return value containing TIN | CONFIRMED |
| Rox to Books (future) | Books-issued ticket, from Books' own subject map | TIN in links or props (spec §9.4) | MISSING |
| Books to server | `profile_id` (UUID) + `branch_code` | TIN after the one-time request | MISSING |
| Server to desktop | `profile_id` + `form_code` + `tax_year` + `period_key` | `device_filings.tin` (today's exception) | PARTIAL |
| Forms to Books | `profile_id`, form, year, period, field key | TIN root | MISSING |
| User | none shared. wasi UUID is not Books' text id | equality assumption | CONFIRMED mismatch |

Period encodings also differ. Drafts use `period_key` strings such as "2026", "2026-Q1", "2026-03" (docs/sync-api.md). `queue_on_device` takes an integer 1–12 (protocol.rs schema). Pick one before cross-system drafts.

## 4. Designed vs implemented

| Capability | Status | Evidence |
|---|---|---|
| Desktop account sign-in (device flow) | Implemented (dev server verified in logs) | bir-core cloud_account.rs:506-710 |
| Email verification gate for tax-profile writes | Implemented | application/tax_profile.rs:312 |
| Per-registration-unit tax profile, hashed exclusivity | Implemented | domain/tin_identity.rs:19; store/tax_profile.rs:99; 0004:77 |
| Masked TIN in REST DTOs | Implemented for DTOs. Plaintext remnants exist | contracts/tax_profile.rs; 0014:15; 0002:16 |
| ORUS TIN verification | MISSING in production (fake verifier only) | domain/orus.rs:81; README "stubbed" |
| Form 2303 ownership claim with staff review | Implemented | application/ownership_claim.rs; store/ownership_claim.rs |
| AI review of 2303 | Implemented, config-gated | src/claim_ai/* |
| Access passes (organization grantee) | Implemented | domain/pass.rs; application/tax_profile.rs |
| Agent connections and MCP read tools | Implemented | src/mcp/*; application/agent.rs |
| Agent `queue_on_device` | Implemented (a write despite README) | protocol.rs:25-30; application/agent.rs:72 |
| Device-filing pickup on the desktop | Implemented | bir_module.rs:2314 |
| Server tax years, drafts, filings API | Implemented server-side. No desktop client | docs/sync-api.md; grep |
| Desktop license for pass-only users | Implemented, pass-blind (see §5 #4) | bir-mcp license.rs:291; store/tax_profile.rs:833-890 |
| Rox v1 tickets | Implemented in BIR plugin. Rox validates format only | tickets.rs; perms.rs:254 |
| Ticket revocation | MISSING in BIR (spec §9.5 permits it) | tickets.rs |
| buwiz-oss Gatekeeper, Ticket, HPKE, Sender | Designed only. P5 `todo`. bir-submit is a stub | internal/PLAN.md §5.3; PROGRESS.md:70 |
| Rox hosted account (Rox Hub) | Planned | drafts/rox-server-architecture.md:59 |
| Books firm and accountant multi-client | Schema and RLS only | src/db/schema/inbox.ts:77-126 |
| Books org TIN storage | Implemented, plaintext | tax-reference.ts:168 |
| Books PH TIN AI redaction | Branch only (5341f63) | fix/ai-privacy-ph-ids |
| Books to server link | MISSING | none |
| Desktop to server profile link (`profile_id`) | MISSING | desktop calls only license, device filings, /me, refresh, logout |

## 5. Risks and inconsistencies (priority order)

1. **P1 privacy.** Plaintext TIN in `device_filings.tin` with no purge (0014:15; store/agent.rs:677-725). The model must also supply the full TIN to queue (protocol.rs required list).
2. **P1 privacy decision.** Form 2303 (TIN, name, address) goes to a vision model when a key resolves (claim_ai/config.rs:73, 104). Production key state: UNKNOWN.
3. **P2 data exposure.** `TaxProfileView` carries `owner_user_id`, `account_id`, `org_id`, `verified_owner_user_id`. Built from the row without caller redaction (contracts/tax_profile.rs:10; store/tax_profile.rs:814, 992-1040). Any holder with standing sees them. README says the MCP tool omits internal IDs. The MCP path: UNKNOWN.
4. **P2 functional** (CONFIRMED by code reading, not executed). The desktop license probe calls `GET /api/tax-profiles` with no `organization_id` (bir-mcp license.rs:291). Without an org, the server returns only owned profiles (store/tax_profile.rs:833-890, `else` branch). A firm member whose access is only passes gets "Buwiz access required", while the README says a working pass is the license.
5. **P2 key lifecycle** (INFERENCE). `tin_hash` is the only exclusivity key and derives from AUTH_ROOT_KEY_BASE64 via HKDF (runtime.rs:109, 142; store/tax_profile.rs:99). Rotating the root key would orphan every hash. No rehash or rotation path found (grep for rotate and rehash in store, application, auth_product).
6. **P2 rule divergence.** Server: "2303 never used to choose forms" (README "Ownership claims"). Desktop: forms set from reviewed COR evidence (bir-core/src/forms/forms_set.rs:1-12; `FormSetSource`), and "2303" excluded as a return code (integration/validation.rs:22, 1114). Both can hold, but the rule is not shared.
7. **P2 architecture.** buwiz-oss D2 (no account UI, no mini apps) and D3 (TIN ownership plus submission only) conflict with buwiz-server (accounts, passes, agents). The installed mini-app is `bir-shell`: `dev.goldcoders.bir` appears in its Cargo.toml. Decide which desktop hosts the Books mini-app.
8. **P2 naming.** Forms CONTEXT.md:10-45 defines Taxpayer, Taxpayer Profile ("avoid: tax profile") and Registration Unit. buwiz-server's `TaxProfile` is a Registration Unit (domain/tax_profile.rs:1-8). Books' `org_tax_profiles` is a third meaning.
9. **P3 duplication.** Classification and EOPT tier have three owners (§2.3).
10. **P3 Books.** Org TIN plaintext and unmasked (tax-reference.ts:168; -tax-settings.ts:17). PH TIN redaction is not on main. `org_tax_profiles.tin` has no cross-org uniqueness (tax-reference.ts:163-170), so two orgs can record one TIN (INFERENCE from schema).
11. **P3 docs drift.** README says there is no write tool (README:275). The code has `queue_on_device` (protocol.rs:25, 257).
12. **P3 legacy rows.** Plaintext `tin_root` survives migrations. Reprojection skips projection-only rows (store/tax_profile.rs:343). Presence: UNKNOWN.
13. **P3 tickets.** No expiry or revoke. Model-origin ticket scope: UNKNOWN.

## 6. Not verified and limits

- No test suites run (cargo, bun). No sign-in, ticket resolution or API call executed. One `lsof` listener check on 127.0.0.1:3008.
- Not read: .env values (names and two booleans only), `.auth-root-key`, `.auth-outbox-key`, Rox `logins.db`, plugin cache contents (structure only), buwiz-oss `internal/server-handoff` secrets (README and file list only).
- No ticket values or TINs printed. The `mini-v1.json` and cache files were profiled for keys, types and lengths only.
- Not traced: whether bir-mcp results echo full TINs (UNKNOWN). The MCP `get_tax_profile` internal-ID claim (UNKNOWN in code). `accept_device_filing` body. Agent narrowing in full. Rox consent flow beyond the header. Books firm usage (grep only).
- buwiz-cloud: MISSING, by the search method in §1 item 1.