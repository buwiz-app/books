# ADR 0005: Stack for the Books repo and mini app

Status: Proposed. The Rust choice follows the BIR pattern. The crates are not yet chosen or build-tested.
Date: 2026-10-29. Writer: W3.

## Context

- The Books runtime must be a stdio MCP server that Rox spawns `[rox §7]`.
- BIR's mini app is a Rust stdio MCP server `[bir-shell §3]`.
- Rox's SDK helper is Node, and the conformance and signing tools are Node. They run from the Rox checkout `[rox §9; sdk docs/testing.md]`.
- The TypeScript Books repo is Bun-based `[BK-TS CLAUDE.md]`. Its pure tax modules are reusable as reference only `[books §7]`. Its defects must not be ported.
- Mini-app apps are TSX modules that import only `nsh` SDK-1 names and `rox`, and run in QuickJS with no network `[rox §5 stage 5]`.
- Money must be integer centavos. Floats are not allowed in Books `[decisions.md, Rox constraints]`.
- JCS canonical JSON is used for manifests and hashes. Rox ships cross-language vectors `[rox §4]`.
- Ticket randomness uses `getrandom` in BIR `[tickets.rs:186-188 [V]]`.

## Decision

| Area | Choice | Note |
|---|---|---|
| Runtime | Rust stable, one Cargo workspace | `crates/books-core` (pure rules, money), `crates/books-store` (encrypted store), `crates/books-mini-app` (stdio MCP, reserved `rox.*` tools) |
| Store | SQLite with SQLCipher-compatible encryption | Engine and crate UNKNOWN until P1 proves the build |
| Ticket randomness | `getrandom` | Same as BIR |
| Canonical JSON | A JCS implementation, checked against Rox's `jcs-vectors.json` | `app/src/mini_v1/jcs-vectors.json`, `server/test/fixtures/jcs-vectors.json` |
| Money | `i64` centavos, `i128` for intermediates, checked overflow | No floats in Books. BIR's Forms path is f64 at its input (see [01 section 9.1](../01-architecture.md#91-observed-constraints)). |
| Apps | TSX, `nsh` SDK-1 names and `rox` only | No `<If>` or `<Each>` until their source is found `[bir-shell §2; §8 item 16]` |
| Node | Rox signing and conformance tools only, run from the Rox checkout | D17. No Node runtime in Books. |
| TypeScript Books | Reference only. Bun stays there. | DD-26 |
| Tests | `cargo test` for Books. Node test under `tests/host/` only for the broker harness. | [06 section 11](../06-testing-plan.md#11-test-tiers) |

## Alternatives considered

| Option | Why not |
|---|---|
| TypeScript (Bun) for the mini app | Would make the Books runtime depend on the TypeScript repo's code and toolchain. Rox runs any command, so it is possible, but the BIR pattern is Rust and the owner wants no hard coupling. DD-26. |
| Node for the mini app | Rox's SDK helper can run under Node, but the helper is not the runtime. Adds a second toolchain to the plugin. |
| Postgres in the mini app | Owner decision 2 (local-first). |

## Consequences

- Rust toolchain and `cargo` in CI. The repo's own `CLAUDE.md` (W1) sets the commands.
- Node is needed only to run Rox's tools on a machine that has the Rox checkout (tier T4 in [06](../06-testing-plan.md#11-test-tiers)).
- The `node:sqlite` requirement of the conformance runner sets a minimum Node version (UNKNOWN; check at P0).
- Crate choices are open and need a spike (RK-7).

## Evidence

- `[rox §4, §7, §9]`; `[bir-shell §3]`; `[bir-shell §5]`
- `[books §7]` (reuse versus coupling)
- `[decisions.md, Rox constraints]`
- `crates/bir-shell/src/mini_app/v1/tickets.rs:186-188` [V] (`getrandom`)
- `server/sdk/mini-app/bin/rox-plugin-test.mjs` imports `node:sqlite` [V]

## Revisit when

- A Rust SQLCipher build fails on macOS and no fallback exists.
- Rox ships a Rust SDK.
