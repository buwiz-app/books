# ADR 0001: Books is its own Rox plugin, `dev.goldcoders.books`

Status: Accepted for the separate service (owner decision O1, 2026-10-29). The plugin split is a design decision (DD-1) that the owner has not separately accepted.
Date: 2026-10-29. Writer: W3.

## Context

- The owner decided Books is a separate service in its own repo. It owns profiles, the ledger, categories and the Books-to-Forms bridge. buwiz-cloud is used only for sign-in and tax-profile confirmation `[decisions.md, owner 1]`.
- BIR already ships a Rox plugin, `dev.goldcoders.bir`, with 13 apps and 113 tools `[bir-shell §1, §3]`.
- Rox lets an app call only its own plugin's tools and open only its own apps. Cross-plugin use goes through the model, routine reads, or user-clicked links `[rox §1 item 5]`.
- Two ways exist to ship Books in Rox:
  - (A) apps inside the BIR plugin;
  - (B) a new sibling plugin.
  The traces name these options differently. This ADR uses the names, not the letters `[canonical correction 6]`.

## Decision

Books is a new Rox plugin, `dev.goldcoders.books` (short `books`). It has its own signed manifest, its own stdio MCP process, its own apps, its own sign-in and its own store.

## Alternatives considered

| Option | Why not |
|---|---|
| Apps inside `dev.goldcoders.bir` | Books data would enter BIR's process and its ticket store. BIR has no Books client. Every Books change re-signs BIR and bumps its version `[bir-shell §6; manifest.rs:30]`. BIR's failure domain would include Books. Books would inherit BIR's Buwiz sign-in gate. |
| A sibling plugin, as chosen | Costs: one trust-store entry (a Rox release), and no direct calls to BIR. The hand-off must go through the model (ADR 0004). |

## Consequences

- Needs one approved-plugin entry in the Rox trust list, `server/src/v1/trust.json:11-23` `[V]`. That is a Rox-side change. In stage A a per-run trust file is used `[rox §5 stage 2]`.
- Books uses its own budget: 6 apps, 1 command, 2 scopes, 0 routines `[rox §7 limits]`.
- Books cannot open BIR apps or call BIR tools directly `[bir-shell §6]`. The model is the bridge in MVP.
- A separate failure domain: a Books crash does not stop BIR, and the reverse.
- The short name `books` must be unique among enabled plugins `[rox §7]`.

## Evidence

- `[decisions.md, owner 1]`
- `[rox §8]` (options and cross-plugin consequences) and `[bir-shell §6]` (recommendation (a) sibling plugin, with the same cross-plugin limits)
- `server/src/v1/trust.json:11-23` [V]
- `[rox §5 stage 1]` (discovery and the model's tool names, `agent.mjs` `v1Tools`)

## Revisit when

- Rox adds a supported cross-plugin call with consent (today none exists).
- The owner accepts a different packaging.
