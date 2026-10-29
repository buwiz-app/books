# ADR 0002: The ledger is local-first, encrypted, inside the Books process

Status: Accepted (owner decision O2, 2026-10-29).
Date: 2026-10-29. Writer: W3.

## Context

- The owner decided the ledger is local-first: encrypted storage inside the Books mini app's own MCP process, the same pattern as BIR's SQLCipher database. Cloud sync is a later, separate step `[decisions.md, owner 2]`.
- BIR's pattern: SQLCipher AES-256, key from `BIR_DB_KEY_FILE` with `BIR_NO_KEYCHAIN=1` `[bir-shell §3 Local database]`. BIR's installed env uses a 0600 key file `[rox §6]`.
- Rox rule: avoid the Keychain in rebuilt binaries. Use an env key file and `*_NO_KEYCHAIN=1` `[decisions.md, Rox constraints]`.
- The TypeScript Books repo has no offline or local store `[books §8 item 27]`. Its ledger lives in Postgres behind Better Auth `[identity §2.10]`.
- buwiz-cloud v1 has no ledger sync and no organizations `[decisions.md, cloud]`.

## Decision

- The store is an encrypted SQLite file, opened only by the Books process.
- The key is a file, mode 0600, outside the repo. Its path is passed by env. No Keychain in MVP.
- No server database and no sync in MVP.
- Each tool call commits once. A restart cannot leave a half-written entry.
- The store refuses a wrong key and refuses a `cipher_version` mismatch. Neither leaves partial state.

## Alternatives considered

| Option | Why not |
|---|---|
| Keychain for the key | Rox rule against Keychain in rebuilt binaries `[decisions.md]`. Adds a dependency on the login session. |
| Server database from day one | Owner decision 2. Also adds network and tenancy to every write. |
| Reuse the TypeScript Postgres schema | Coupling. The TypeScript repo is a reference only (ADR 0005). |

## Consequences

- Key custody becomes the main risk. Losing the key file means losing the ledger. Backup is the user's job, with a warning at setup. A passphrase-wrapped key with a backup prompt is required before release (D2).
- Multi-device use waits for P9.
- Restore and backup procedures must be written and tested before release (RK-8).
- Encryption depends on an SQLCipher-compatible build that is not yet proven (RK-7). The engine and crate are UNKNOWN until P1.
- Tests use temp folders and fixed keys that are never reused (see [06 section 5](../06-testing-plan.md#5-store-and-restart-tests)).

## Evidence

- `[decisions.md, owner 2; Rox constraints]`
- `[bir-shell §3 Local database]`; `[rox §6]` (BIR env keys and 0600 key file)
- `[books §8 item 27]` (no offline store in the TypeScript repo)
- `[identity §2.10]` (Books uses Better Auth and Postgres RLS)
- `[bir-shell §8 item 10]` (spawn env is persisted: pass a key file path, never a key value)

## Revisit when

- P9 sync is designed.
- The passphrase-wrapped key (D2) lands.
- A build of SQLCipher-compatible SQLite is proven on macOS.
