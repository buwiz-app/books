# Buwiz Books

Bookkeeping for a small Philippine business, as a [Buwiz Bot](https://github.com/buwiz-bot/buwiz-bot) mini app: the
ledger, unpaid invoices and bills, reconcile, reports, payroll and certificates, kept encrypted on your computer.
Books hands accounting figures to Buwiz Forms. It never computes tax, stores a TIN, or sends, files or pays anything.

## Getting started

Needs Rust through rustup (`rust-toolchain.toml` pins it). Node is needed only for the conformance and host-test gates.
`cargo build --release -p books-mini-app` builds `target/release/books-mini-app`.

**In Buwiz Bot:** ask its agent to install `https://github.com/buwiz-app/books` (the latest release, after you approve).

**A local build:** the Bot installs only manifests signed by a key in its trust store. Run a separate Bot instance
(never your everyday one) with `BUWIZ_TRUST_FILE` naming a trust store that holds a throwaway test key and
`BUWIZ_KEYSTORE=file:<dir>`. Sign the build's manifest with that key and install it; `tools/bot/host-test.sh` is that recipe.

Settings: `BOOKS_DATA_DIR` (default `~/Library/Application Support/app.buwiz.books`), `BOOKS_DB_KEY_FILE` (default
`<dir>/books.key`, 0600) and `BOOKS_CLOUD_URL` (another cloud; empty turns the optional Buwiz sign-in off).

## Develop

| Path | What |
|---|---|
| `crates/books-core` | Domain rules, no I/O: centavos, dates, the ledger's post rules, measures and the Forms bridge. |
| `crates/books-store` | The SQLCipher store and its migrations. |
| `crates/books-mini-app` | The stdio MCP server, the reserved `buwiz.*` tools, the manifest and the `books_*` tools. |
| `crates/books-screens` | Native screens: `books.ledger` and `books.quiet`. |
| `apps/` | The TSX apps: the Books window (`apps/books/`), the inline cards and the home card. |
| `data/` | The category catalogue, measures, mapping rule sets and crosswalks the engine loads. |
| `fixtures/`, `tests/` | Synthetic fixtures and vectors, and the tests. Never real data. |

`tools/check-all.sh` runs every gate and must pass before a commit:

| Gate | Needs |
|---|---|
| fmt, clippy (`-D warnings`), `cargo test` | Nothing else. With `BUWIZ_FORMS_SCHEMAS` set to a folder of the pinned agent-forms schemas, `cargo test` also checks the crosswalks against them; without it, that check prints SKIPPED. |
| SDK doc links | Nothing else. Links to the SDK in these docs name the pinned revision. |
| SDK conformance (`tools/conformance.sh`) | `BUWIZ_SDK_DIR` (required): a [buwiz-sdk](https://github.com/buwiz-app/buwiz-sdk) checkout at the revision pinned in `crates/books-mini-app/Cargo.toml`, with `bun install` run, and `node`. With it set, `cargo test` also compares the SDK's JCS vectors with `tests/fixtures/jcs-vectors.json`. |
| Bot host test (`tools/bot/host-test.sh`) | `BUWIZ_BOT_DIR` (required): a checkout or worktree of buwiz-bot with `server/node_modules` installed, and `node`. It also needs: <ul><li>the commit it pins (`BUWIZ_BOT_COMMIT`) to exist in that checkout;</li><li>`lsof`;</li><li>a port of 8810 or above (`BOOKS_HOST_PORT`);</li><li>on macOS, `security`, which reads only the Keychain entry's attributes to prove the run left it alone.</li></ul> |
| Privacy scan (`tools/privacy-scan.sh`) | Nothing else. |

`SKIP_BOT=1` skips the Bot host test (the run then isn't green). Gates sign with a throwaway key (`books-test-01`) and a
file keystore, never the publisher key or the login Keychain. Read [AGENTS.md](AGENTS.md) before changing a rule; the
protocol is [buwiz-sdk `docs/protocol-1.0.md`](https://github.com/buwiz-app/buwiz-sdk/blob/317bae64f85947c94ce6d2efb7f2d9bdedc387a0/docs/protocol-1.0.md).

## Releases

1. Bump `VERSION` in `crates/books-mini-app/src/manifest.rs`. That is the only place: packaging copies it into
   `plugin.json`.
2. Dry run, checked as the Bot will check it (build, sign the manifest and `plugin.json`, verify both):
   `BUWIZ_BROKER_BIN=<buwiz-broker> tools/release/publish.sh --trust <the Bot's trust list>`.
3. Publish a pre-release: `BUWIZ_BROKER_BIN=<buwiz-broker> tools/release/publish.sh --publish --prerelease`. A Bot
   installs the latest release, which skips pre-releases.
4. Promote it once it's checked: `gh release edit v<version> -R buwiz-app/books --prerelease=false --latest`.

Signing is the owner's: `publish.sh` signs with Buwiz's release key from the login Keychain (macOS asks
twice). Build `buwiz-broker` from buwiz-bot's `broker/`. Running it on another platform adds that binary to the release.

## License

[Apache License 2.0](LICENSE); `NOTICE` holds the copyright line.
