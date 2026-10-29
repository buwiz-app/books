# Rox Cards API v1: the plan

Status: proposal, 2026-10-30. Nothing here is built yet. It turns the research in the "Rox Card Model" workbook
(claude.ai/artifact/1L19rWsaehPV44z7bm6jad, private to the owner) into one stable API that Rox, Books, BIR and any
later plugin can build on. The decisions in section 11 are the owner's; the phases in section 10 start once they are
made.

Already decided (owner, through the rox session, 2026-10-30): **D17, protocol 1.1** adds `rox.actions([...])`
(Rox draws a card's footer buttons and sends each press back to the app), `rox.settle(text)` (the card collapses to
one outcome line) and `rox.meta({text, pill?})` (a status in the header), for inline cards, at most 3 actions; it is
being built on rox branch `protocol-1.1`. `inline.class` is **not** adopted: the manifest's `min_height` and
`max_height` size cards. **D18**: a command's resolve may answer `{result: "prompt", text}`, which Rox puts in the
composer for the person to send. Rox #17 (Books in `trust.json`) and #18 (one header per card) are merged. This plan
names its own open decisions C1 to C9 so they do not clash with the owner's D numbers.

Sources read (all public): hexuria/nativechat at `72efae52`, hexuria/opengrok-server at `d6f30d2c`, hexuria/rox at
`b6a1258` plus PR #18 (`569c41b`), Rox's spec `drafts/rox-mini-app-protocol-v1.md` §9.6 to §9.8, and buwiz-books at
`6881175`. Short references below use `nc:` for nativechat, `og:` for opengrok-server and `rox:` for Rox.

## 1. Why

Cards are built four times today, each with its own rules:

- **nativechat + opengrok-server**: eight fixed kinds (approval, user form/login, computer handoff, choice, bar chart,
  plugin needs, save login, screenshot), a closed list (`nc:src/opengrok/gen_ui.rs:22-45`).
- **Rox**: a port of five of them (`rox:app/src/cards/mod.rs`), plus mini-app answer cards drawn by each plugin's own
  signed TSX (§9.8: "no host templates in v1").
- **Books**: 15 TSX cards and a 217-line card kit (`apps/cards/shell.tsx`).
- **BIR**: 8 TSX cards and its own 277-line card kit (`crates/bir-shell/mini-apps/card-kit.tsx.in`).

So the same card is drawn three ways, two plugins copy Rox's look by hand, and the owner's scenarios have no home:
cards that need no buttons by design, a receipt after Rox acted alone, a decision so important the chat waits for it,
expiry for any card, "remind me" and "do it on a date", and cards made on demand.

## 2. Goals and non-goals

Goals:

1. One card model, the same for Rox's own cards, plugin cards and cards the assistant makes.
2. Rust authors write no JavaScript for a card; TSX authors use Rox's kit, not their own.
3. Reuse what already works: opengrok-server's parking, answer routes, form expiry and steer; nativechat's states and
   look; Rox's consent rules and PR #18's one-card header.
4. Stable: versioned, schema-checked, conformance-tested, additive-only within v1.
5. The owner's invariants hold everywhere (section 5): only the person presses buttons that change things.

Non-goals for v1: plugins compiled into Rox; full pages drawn from descriptions (pages stay TSX); any change to how
Books computes money or what it may do on its own.

## 3. What we keep

| Today | Where | In the new model |
|---|---|---|
| `run-awaiting-approval` with `reason` `exec-consent` / `policy-approval` / `auto-review` / `user-form` | `og:crates/opengrok-core/src/run.rs:126-163` | an `ask` card that parks; the frame gains an optional `card` field |
| `POST /ag-ui/runs/{id}/answer {call_id, approved}`, yes or no resumes | `og:crates/opengrok-server/src/agui/routes.rs:4700-4909` | answer `run` |
| `request_user_form` / `formRequest`, held 10 min then dismissed, run resumes | `og:.../agui/user_form.rs:51-54, 710-743` | kinds `form` / `login`, `life.expires_in_s: 600`, `on_expire: dismiss` |
| Steer: a new message stops the parked run, cards read "Moved on" | `og:.../agui/resume.rs:27-55` | `hold: park` with `until: new_message` |
| `form` tool args `{title, prompt, fields[{id,label,options}], submit}` | `og:.../agui/chat_ui.rs:10-31` | kind `choice` body, unchanged |
| `bar_chart {title, bars[{label,value}]}` | `nc:src/opengrok/gen_ui.rs:1818-1908` | kind `chart` |
| Standing rules (Mac command rules, per-Bot tool mode, egress, auto-review texts) | `og:crates/opengrok-store/src/schema.sql:525-563` | answer `rule`, written to the same stores |
| States: sending, answered, dismissed, superseded, error | `rox:app/src/cards/mod.rs:212-223` | the state machine (4.5), plus expired, snoozed, scheduled |
| "Waiting for approval" / "Waiting for you" | `rox:app/src/cards/mod.rs:254-262` | composer status for `park` |
| `_meta["rox/open"]`, consent §9.7, first-party auto-mount, fold after 5 turns, `rox/suggest` | Rox spec §9.6 to 9.8 | door 1, unchanged |
| One header with collapse and maximize | rox PR #18, merged | the host header for every card |
| `rox.actions`, `rox.settle`, `rox.meta` | D17, protocol 1.1, in progress | door 1's footer, outcome and status |

## 4. The model

### 4.1 Six answers

Every card answers six questions. They are independent, with three rules: `park` and `gate` need `need: ask`;
`later` needs an action that calls a tool; a `show` card has no button that changes anything except Undo.

| Field | Values | Default |
|---|---|---|
| `kind` | `glance` `list` `chart` `decision` `choice` `form` `login` `notice` | required |
| `need` | `show` (no buttons) · `offer` (buttons, nobody waits) · `ask` (the run waits) | required |
| `hold` | `free` · `park` (the run pauses; a new message moves on) · `gate` (the chat waits too) | `free`; `park` for `ask` |
| `life` | until answered · expires · stays · later | see 4.7 |
| `answer` | per action: Rox carries on · a plugin tool · a message as you · a rule · open an app · undo | per action |
| `draw` | Rox draws it · the plugin's app · made by the assistant | Rox |

Rox's existing cards fit without change: an approval is `decision · ask · park · until answered · run (+rule)`; a
choice card is `choice · ask · park · until new_message · message`; a login is `login · ask · park · expires 600 s
on_expire dismiss · submit`.

### 4.2 The card object

```ts
type Card = {
  v: 1;
  id: string;            // the plugin's key for this card, unique per plugin per chat ("owed:inv-0143");
                         // Rox prefixes the plugin id. The same id again replaces the card (4.9)
  kind: "glance" | "list" | "chart" | "decision" | "choice" | "form" | "login" | "notice";
  title: string;         // ≤ 60: the header ("Books · Mark paid")
  need: "show" | "offer" | "ask";
  hold?: "free" | "park" | "gate";
  life?: Life;
  meta?: { text?: string; pill?: Pill };      // the line under the header; never the title again
  body: Body;                                  // by kind (4.3)
  actions?: Action[];                          // ≤ 3, in the footer; Rox adds Later and Decide later (4.6, 4.7)
  refresh?: { tool: string; args?: object };   // what Refresh, and a kept card, read again
  as_of?: number;                              // unix ms the data was read; Rox shows "as of"
  draw?: "host" | { app: string; props?: object };  // default "host" (door 2); an app is door 1
};
type Life = {
  until?: "answered" | "new_message" | "removed";
  expires_in_s?: number;                       // 30 .. 604800
  on_expire?: "dismiss" | "no";
  later?: ("remind" | "schedule")[];
};
type Pill = { tone: "ok" | "warn" | "rev" | "info" | "mute"; text: string };
type Row = { label: string; sub?: string; amount?: string; prev?: string; text?: string;
             tone?: "pos" | "neg" | "mute"; strong?: boolean };
```

Amounts are display strings made by the plugin (Books: from integer centavos). Rox never does arithmetic on a card.

### 4.3 Bodies

| Kind | Body | Notes |
|---|---|---|
| `glance` | `{ hero: {text, tone?}, rows?: Row[] }` | one figure, at most 6 rows |
| `list` | `{ columns?: [string, string?, string?], rows: Row[], total?: Row }` | ≤ 50 rows; scrolls inside 240 points |
| `chart` | `{ bars: [{label, value, ratio_pm}], caption? }` | `ratio_pm` 0..1000, an integer: no floats |
| `decision` | `{ hero?, rows?, pick?: {id, label, options: [{id, label, best?}], required} }` | a choice the action depends on, picked in the body |
| `choice` | `{ prompt?, fields: [{id, label, options: [string]}], submit? }` | the `form` tool's arguments, unchanged |
| `form` | `{ instruction?, fields: Field[], submit? }` | `formRequest`'s fields; `secret` fields never reach the chat or a log |
| `login` | a `form` plus `{ domain, challenge_kind, passkey_mode?, saved_logins? }` | as `request_user_form` |
| `notice` | `{ text }` | one sentence |

### 4.4 Actions and answers

```ts
type Action = {
  id: string; label: string;
  role?: "primary" | "secondary" | "danger";     // one primary at most; primary first, right-aligned
  requires?: string[];                           // body picks that must be chosen first ("pick:account")
  answer: Answer;
};
type Answer =
  | { run: { approved: boolean } }               // resume the parked run (an ask)
  | { submit: true }                             // send the form or choice values to the run
  | { message: string }                          // send as the person (a suggestion, a choice)
  | { tool: string; args?: object }              // the plugin's own tool, origin app, the person's press
  | { open: { app: string; props?: object } }    // open the plugin's app over the chat
  | { undo: { tool: string; args?: object; until: number } }   // a receipt's undo window
  | { rule: { mode: "always" | "never"; scope: "bot" | "all_bots";
              subject: { tool: string; pattern?: string } }; then?: Answer }
  | { dismiss: true };
```

Rox labels the outcome line when a card settles ("Posted ₱ 1,350.00"): the plugin's tool returns it as
`{outcome, sub?}`, or Rox uses the action's label.

### 4.5 States

```
open ──press──▶ sending ──ok──▶ answered(outcome)
  │                  └──fail──▶ error(text) ──▶ open
  ├──✕ or Dismiss──▶ dismissed
  ├──new message (park, choice)──▶ superseded
  ├──timer──▶ expired (on_expire: dismiss → dismissed; no → answered "No", the run resumes)
  ├──Later ▸ Remind──▶ snoozed(until) ──time──▶ open (posted again as a reminder)
  └──Later ▸ Do it on──▶ scheduled(at) ──time──▶ sending ──▶ answered | error
                              └──Cancel──▶ open
```

Folding (after 5 turns, or the chevron) is a view, not a state. State is stored by card id and survives a restart:
for asks parked on the server, the server owns it (as `GET /ag-ui/approvals` does today); for plugin cards, Rox's
store does.

### 4.6 Hold

- `free`: nothing waits.
- `park`: the run that raised the card pauses. The composer stays open and shows "Waiting for you" (or "Waiting for
  approval"); a new message is a steer: the run stops and the card is superseded, as opengrok-server does now.
- `gate`: a `park` that also locks the composer ("Decide on the card above to keep chatting"). It always has an
  exit: Rox adds **Decide later**, which turns it into a park, and Stop stays available. One gate per chat at a
  time; a second one queues. Only Rox itself and first-party plugins may gate; a gate from anyone else is drawn as a
  park and logged.
- A plugin's `ask` card parks the agent's tool call that returned it: the broker treats the result as awaiting (as
  opengrok's `ToolResult::awaiting`) and gives the model `{card, answer: {action, values?}}` or `{card, state}` when
  it settles.

### 4.7 Life

- `until`: `answered` (the default for `ask`), `new_message` (the default for `park` and `choice`), `removed` (the
  default for `show` and `offer`: it stays, folds after 5 turns, and goes when the plugin replaces it).
- `expires_in_s` with `on_expire`: forms and logins default to 600 s and `dismiss`, matching `FORM_HOLD_TIMEOUT`.
  Approvals keep no default expiry (they never expire today) unless the owner decides otherwise (C3). A card that
  expires while Rox is closed is marked expired at the next launch.
- `later`: Rox adds **Later ▾** to the footer.
  - **Remind**: In 1 hour · Tomorrow 9:00 · Pick a time. The card is snoozed and posted again at that time.
  - **Do it on**: only for a primary action that calls a tool marked schedulable (`_meta["rox/schedulable"]: true`
    in the tool listing). Rox records the press and its time, shows "Scheduled for Nov 15 · Cancel", and calls the
    tool then with `rox/caller {origin: "app", scheduled: {pressed_at, by}}`. If Rox is closed at that time it runs
    at the next launch within 24 hours; after that it shows "Missed: Nov 15" with **Run now**.

### 4.8 Who draws: three doors

- **Door 1, the plugin's app (TSX)**: the manifest app as today. Rox draws the header; the app draws the body with
  Rox's TSX card kit (`rox/cards`). Protocol 1.1 (D17, decided) adds `rox.actions([...])` for Rox's footer buttons
  with each press sent back as an event, `rox.settle(text)` and `rox.meta({text, pill?})`. With `inline.class`
  rejected, an app's `need`, `hold` and `life` cannot live in the manifest; C1 decides whether they come from the
  `_meta["rox/open"]` that shows the card (recommended) or a bridge call.
- **Door 2, a description**: a tool result carries `_meta["rox/card"]: Card`. Rox draws it with its own Rust
  components, the same ones as its approval card. No JavaScript.
- **Door 3, made on demand**:
  - **Found**: plugins list their cards in the manifest (`cards[]`: name, kind, title, what it shows, its props
    schema). The model's `card_show {plugin, name, props}` asks the plugin to open one (Books' `books_show`,
    made general).
  - **Made**: the model's `card` tool sends a Card with `draw: "host"`, filled with data it read through tools. It
    is data, not code. In v1 its actions may only be `message`, `open` or `dismiss`: no tool calls (C6).
  - **Built**: the model's `app_draft` makes a small TSX app that runs in the QuickJS sandbox with no tools. It is
    unsigned, shows "Sandboxed", and is kept only when the person presses Keep (C7). Behind a flag.

### 4.9 Updating a card

A card with an id Rox already shows replaces it in place (door 2: a later tool result; door 1: `rox.settle`,
`rox.meta`). Its state is kept unless the update carries `reset: true`. A plugin removes a card by sending it with
`life.until: "removed"` and `body: null`.

## 5. Invariants

1. Only the person presses a button that changes something. The model, a routine and a made-on-demand card can show
   a button and never press it (§9.8).
2. A tool that changes something refuses `origin: agent` unless its grant allows it (P5); a press arrives as
   `origin: app` with the card and action named in `rox/caller`.
3. A third party's card is collapsed until opened (§9.7, PR #18) and cannot gate.
4. A card the model makes carries no tool action in v1.
5. A scheduled press is recorded with who and when, shown with Cancel until it runs, and runs once.
6. Secret form fields never reach the chat, a log or a stored card.
7. A plugin's card passes its own privacy rules before it leaves the plugin (Books: no TIN, ever).
8. Rox never computes on a card's numbers; amounts are the plugin's strings.

## 6. Wire and routes

| From → to | Message |
|---|---|
| plugin → Rox | tool result `_meta["rox/card"]` (door 2), or `_meta["rox/open"]` + the manifest `card` block (door 1) |
| model → Rox | the `card`, `card_show` and `app_draft` tools (door 3) |
| opengrok-server → client | `run-awaiting-approval` gains an optional `card: Card`; old clients ignore it, and Rox maps old frames to Cards |
| client → Rox broker | `POST /v1/host/cards/{id}/press {action, values?, picks?}`, `/later {remind_at \| do_at}`, `/cancel`, `/dismiss` |
| Rox → plugin | the action's tool, `origin: app`, `rox/caller {card, action, scheduled?}`, a fresh `client_request_id` |
| Rox → model | for a plugin's ask: the awaited tool result becomes `{card, answer}` or `{card, state}` |

Server-parked asks keep their routes (`/ag-ui/runs/{id}/answer`, `/ag-ui/user-form/submit` and `/dismiss`).

## 7. Stability

- Every card carries `v: 1`. Within v1 changes are additive only: an unknown field is ignored, an unknown pill tone
  is drawn `mute`, an unknown kind is drawn as a notice reading "Update Rox to see this card".
- Limits: title 60 characters, a row's text 200, 50 rows, 3 actions, 8 KiB per card (the props limit).
- One JSON Schema, `card.v1.schema.json`, in Rox's SDK, with `docs/cards.md`.
- Conformance cases in `rox-plugin-test` (CARD-01 onwards): the schema and limits, a tool action from a model card
  refused, a third party's gate drawn as a park, expiry, supersede on a new message, replace by id, secret fields
  absent from the stored card.
- Golden fixtures: Rox's renderer is tested against opengrok-server's recorded wire corpus
  (`og:tests/fixtures/wire/agui/custom/`) so the old frames keep drawing the same.
- `form` and `bar_chart` stay as aliases of `card` with `kind: choice` and `kind: chart`.

## 8. Packages

- **`rox-card`** (Rust crate in the Rox workspace, usable by plugins as a dependency): the types, builders
  (`Card::decision("Books · Mark paid").offer().meta(...).action(...)`), and `validate()`. Books and BIR depend on it.
- **`rox/cards`** (TSX kit, given by Rox to every TSX app): Body, Meta, Hero, Row, List, Chips, Wait, Empty, Settled,
  drawn with Rox's tokens. Books' `shell.tsx` and BIR's `card-kit.tsx.in` are deleted.
- **Adapters** in Rox: every old frame (approval, user form, choice, bar chart, plugin needs, save login) to a Card,
  so one renderer draws them all.

## 9. Books

| Card | kind | need | hold | life | answer | draw after |
|---|---|---|---|---|---|---|
| cash, month | glance | show | free | stays | open | door 2 |
| person | glance (picker: list) | show | free | stays | open | door 2 |
| unpaid, P&L | list | show | free | stays | open | door 2 |
| draft | decision | offer | free | stays | tool `books_app_post_entry` | door 2 |
| bank lines | decision (pick: category) | offer | free | stays | tool `books_app_recategorize` | door 2 |
| mark paid | decision (pick: account) | offer | free | stays, later | tool `books_app_owed_settle` (schedulable) | door 2 |
| match the bank, close the month | decision | offer | free | stays | tool | door 2 |
| 2307 | decision | offer | free | stays | tool `books_app_certificate_set` | door 2 |
| for the BIR app | decision | offer | free | stays | tool `books_app_handoff_create`, then attach | door 1 (it attaches a ticket) |
| tidy, month end, hand-off | notice | offer | free | stays, later (remind) | open | door 2 |

Books' rules hold: posting, voiding, closing, reopening, the return OK and the hand-off stay the person's press. A
receipt after Rox posts on its own (scenario 2 of the workbook) would change a Books owner decision and is not
planned for Books unless the owner changes it (C5).

## 10. Phases

Each phase ends with the gates of the repos it touches green. Owners: **Rox** (the rox session), **Books** (this
repo's session), **BIR** (the BIR owner), **OG** (opengrok-server).

| # | Phase | Owner | Exit |
|---|---|---|---|
| 0 | Decide C1 to C9; amend Rox spec §9.8 and add §9.9 Cards | owner, Rox | the decisions recorded in the spec |
| 1 | Schema, `rox-card` crate, TS types, adapters, conformance scaffolding | Rox | every built-in card and opengrok's wire fixtures round-trip through `Card` |
| 2 | One host renderer for all Cards; built-ins drawn through it; one state store | Rox | no visible change; render and state tests |
| 3 | Door 2: `rox/card` in tool results, the press route, replace by id. Books moves cash, month, person, unpaid, P&L, tidy, month end, hand-off and the decision cards to descriptions | Rox, Books | those Books cards render with no TSX in a test Rox; CARD conformance |
| 4 | Door 1 kit: `rox/cards` and the protocol 1.1 calls (`rox.actions`, `settle`, `meta`: D17, in progress on `protocol-1.1`). Books' cards and BIR's move to them; both kits deleted | Rox, Books, BIR | no plugin ships its own card kit |
| 5 | Hold and life: a plugin's ask parks the run, gate, expiry timers, Later (remind, schedule) with a one-shot scheduler | Rox, OG | workbook scenarios 3 to 7 pass as end-to-end tests |
| 6 | Door 3: manifest `cards[]` and `card_show`; the `card` tool (no tool actions); `app_draft` behind a flag | Rox, OG | workbook scenarios 8 to 10 |
| 7 | Rules and receipts for plugin tools: rule answers through the per-Bot tool mode, receipts with an undo the plugin declares | Rox, OG | scenarios 2 and 7 with a real plugin tool (not Books unless C5) |

Order: 1 → 2 → 3. Phase 4 needs only 1. Phase 5 needs 2. Phase 6 needs 3. Phase 7 needs 5.

Tests at every phase: schema and builder unit tests; Rox render snapshot tests; end-to-end in a test Rox driven by
gpui-agent (on workspace 3 for Books' runs); Books' `cargo test` asserts each card description passes the privacy
scan and the schema.

## 11. Decisions for the owner

| # | Decision | Recommendation |
|---|---|---|
| C1 | Reverse §9.8 "no host templates in v1" so Rox draws cards from descriptions; and where a door-1 card's `need`, `hold` and `life` come from now that `inline.class` is not adopted | yes; from `_meta["rox/open"]` |
| C2 | Allow `gate`, and who may raise one | yes; Rox and first-party plugins only, always with Decide later |
| C3 | Expiry defaults | forms and logins 10 min (as now); approvals none (as now) |
| C4 | A one-shot Later (remind, schedule) as a Rox feature, apart from routines | yes |
| C5 | Rox acting alone under a rule, with receipts and undo, for plugin writes | for plugins that opt in; not Books without changing its owner decision |
| C6 | Cards the model makes: no tool actions in v1 | yes |
| C7 | Apps the model builds (`app_draft`): allowed, and the default sandbox | behind a flag, no tools by default |
| C8 | Names: `card`, `card_show`, `app_draft`, `rox/card`, `rox/schedulable`; `form` and `bar_chart` kept as aliases | yes |
| C9 | Home of the canonical spec | Rox's `drafts/rox-mini-app-protocol-v1.md`, a new §9.9; this file points there once agreed |

## 12. Risks

| Risk | Mitigation |
|---|---|
| The protocol churns again | `v: 1`, additive only, adapters for old frames, conformance tests |
| Rox's renderer and the TSX kit drift apart | the kit uses the same tokens; snapshot tests of the same card both ways |
| A made-on-demand card misleads ("View" that posts) | no tool actions on model cards (C6) |
| A scheduled press runs at the wrong time or twice | recorded once with a `client_request_id`, run once, shown until it runs, Missed after 24 hours |
| A gate traps the person | always Decide later and Stop; one gate per chat |
| Server and client disagree on a card's state | server-parked asks are the server's; plugin cards are Rox's; never both |

## 13. Open questions

- Cards across devices once the Hub syncs chats: where plugin-card state lives then.
- Reminder delivery when Rox is closed: an OS notification, or only on the next launch.
- Whether `chart` needs lines and stacked bars in v1, or bars only.
