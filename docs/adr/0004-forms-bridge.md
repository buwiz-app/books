# ADR 0004: Books reaches BIR's Forms through the model in MVP, and through a signed envelope later

Status: Proposed. MVP path needs the BIR owner, the Forms owner and the decision on the source label (D3). P8 path needs the BIR owner, the Forms owner and the signing-key decision (D2).
Date: 2026-10-29. Writer: W3.

## Context

- BIR's tools are `bir_form_open`, `bir_form_fill`, `bir_form_fields`, `bir_form_validate`, `bir_form_save_draft`, and `bir_form_send` `[bir-shell §3; forms §5]`.
- The fill tool takes exact official keys (for example `frm2551Qv2018:txt15`) or typed aliases for 2551Q and 1601-C. A bad key fails the whole 2551Q or 1601-C call `[verify §1 item 1, §3]`.
- Fill sources: `profile`, `past_return`, `document`, `ai`. `user` only with `from_card: true` `[tools.rs:309-311, 547 [V]]`.
- Provenance is session-only. It resets on every open and is never persisted `[session.rs:55, 526 [V]]`; `[verify §3]`.
- `bir_form_send` is declared in the live tool list `[tools.rs:174-175 [V]]`. The live send path needs a standing send grant. Without it the result is `approval_required` `[session.rs:1388-1400 [V]]`. Standing grants have no expiry `[bir-shell §8 item 1]`.
- Cross-plugin calls are refused. Books cannot call BIR tools or open BIR apps `[bir-shell §6]`.
- BIR's `rox.open` props have no channel for values `[bir-shell §6]`.

## Decision

**MVP (P3). Model-mediated.**

1. Books produces a `ready` proposal with `fill_args`.
2. The user attaches the `proposal` and BIR `taxprofile` tokens. Each attachment has a consent dialog.
3. The assistant reads the proposal, opens the BIR form, fills it once with `source: "document"`, reads it back, validates, and saves a draft.
4. Any mismatch stops the flow.
5. The user confirms in Books, which records `fill_args_sha256` (app-only).
6. Books never sends. It never calls `bir_form_send`.

**P8. Signed envelope.**

- Books signs an envelope with `form_code`, `schema_sha256`, `tax_period_key`, `bir_period`, `fill_args`, `rule_set_id`, `proposal_output_sha256`, `issued_at`, `nonce`, and a signature. It has no TIN and no BIR ticket.
- BIR verifies the signature against a pinned Books public key, checks the schema hash and key set, checks freshness, applies with source `books`, and persists per-box provenance.
- A dedicated host function is the alternative. It needs a scope rule, a tool entry, a manifest permission and a re-sign `[bir-shell §6]`.

## Alternatives considered

| Option | Why not (MVP) |
|---|---|
| Send values with source `user` | Books would be claiming a human typed them. The `user` source is for typed answers only `[bir-shell §8 item 2]`. |
| Use `from_card` | Only for keys on a form card that BIR shows first. No card is shown first today `[bir-shell §8 item 2]`. |
| Use `ai` | Labels Books output as the assistant's own derivation. Less accurate than `document`. |
| Write to the BIR database directly | Violates the data boundary. Breaks the tenancy rule and the BIR key custody. |
| Dedicated host function in MVP | Needs a BIR change and a re-sign. Slower than model-mediated, which needs no BIR code change. |

## Consequences

- MVP needs no BIR code change `[01 section 9.4 item 3]`. It does need the source label confirmed (D3) and a scratch BIR install (B2).
- The model sees fill values (amounts), not TINs or Books IDs. That is a privacy trade-off (RK-11).
- Provenance is lost on a BIR restart (RK-12). P8 fixes it.
- Until change 4 lands (gate or remove `bir_form_send`), any live use of BIR must keep the send grant off. Books never requests it.
- Books must re-read every value after a BIR restart before the user confirms.

## Evidence

- `[bir-shell §3, §4, §6, §8 items 1-3]`
- `[forms §5, §6]`
- `[verify §1 item 1, §3, §4]`
- `crates/bir-mcp/src/tools.rs:90, 174-175, 309-311, 547` [V]
- `crates/bir-shell/src/agent/session.rs:55, 526, 1214, 1388-1400` [V]
- `[canonical design, 2026-10-29]` (hand-off v1 and v2)

## Revisit when

- D3 is decided (label).
- P8 is accepted by the BIR and Forms owners.
- BIR gains a supported way to show a card before a `user` fill.
