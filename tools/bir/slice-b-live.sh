#!/usr/bin/env bash
# tools/bir/slice-b-live.sh — M7 Slice B, live: Books' Q3 2551Q into a real BIR draft on a scratch BIR profile, and
# a later Books change marks the hand-off out of date (tests/host/slice-b-live.mjs). Headless, no port.
#
# The recipe is BIR's own broker harness (crates/bir-shell/v1-tests/broker.test.mjs), from the rox session
# (2026-10-30):
#   - the Bot's broker modules from a git worktree of the Bot repo at a pinned commit (never a branch switch in the
#     Bot checkout itself), with server/node_modules symlinked;
#   - a bir-mini-app debug build (crates/bir-shell, its own workspace, --features agent) that you build yourself in
#     a worktree of the BIR repo, with your own CARGO_TARGET_DIR, under the Bot's scripts/wait-build lock;
#   - BIR's HOME, BIR_DATABASE_PATH and a throwaway BIR_DB_KEY in this run's temp dir, EBIR_TEST_ENV=1,
#     BIR_AUTH_BYPASS=1, BIR_SFTP_DRY_RUN=1, so BIR never opens the owner's database or reaches the Keychain;
#   - one throwaway test key (books-test-01, never gc-2026-01) and a trust file of this run only.
# Everything is removed on exit (KEEP_TMP=1 keeps the temp dir; it holds test keys and the scratch BIR database).
#
# Env: BIR_MINI_APP_BIN  the bir-mini-app debug binary (required)
#      BIR_DIR           the BIR source tree the binary was built from (mini apps, the made-up test taxpayer)
#      BUWIZ_BOT_DIR     default /Volumes/goldcoders/Projects/rox
#      BUWIZ_BOT_COMMIT  default origin/main
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BOT_DIR="${BUWIZ_BOT_DIR:-/Volumes/goldcoders/Projects/rox}"
BOT_COMMIT="${BUWIZ_BOT_COMMIT:-origin/main}"
BIR_BIN="${BIR_MINI_APP_BIN:-}"
BIR_DIR="${BIR_DIR:-}"

die() { echo "slice-b-live: $*" >&2; exit 2; }
[ -x "$BIR_BIN" ] || die "set BIR_MINI_APP_BIN to a bir-mini-app debug build"
[ -f "$BIR_DIR/crates/bir-shell/v1-tests/broker.test.mjs" ] || die "set BIR_DIR to the BIR source tree the binary was built from"
[ -d "$BOT_DIR/server/node_modules/@modelcontextprotocol/sdk" ] || die "$BOT_DIR/server/node_modules is missing; install it in the Bot checkout yourself"
command -v node >/dev/null || die "node is needed to run the Bot's broker modules"

cargo build --release --quiet -p books-mini-app --manifest-path "$REPO/Cargo.toml"
BIN="$REPO/target/release/books-mini-app"

# the physical path: the Bot's modules compare resolved paths
TMP="$(cd "$(mktemp -d "${TMPDIR%/}/books-slice-b.XXXXXX")" && pwd -P)"
chmod 700 "$TMP"
case "$TMP" in "$REPO"/*) die "temp dir is inside the repo" ;; esac
WT="$TMP/bot"
cleanup() {
  git -C "$BOT_DIR" worktree remove --force "$WT" >/dev/null 2>&1 || true
  git -C "$BOT_DIR" worktree prune
  if [ "${KEEP_TMP:-0}" = 1 ]; then echo "kept $TMP (test keys and the scratch BIR database; delete it after)" >&2; else rm -rf "$TMP"; fi
}
trap cleanup EXIT

git -C "$BOT_DIR" worktree add --detach --quiet "$WT" "$BOT_COMMIT"
ln -s "$BOT_DIR/server/node_modules" "$WT/server/node_modules"
mkdir -m 700 "$TMP/run"

node --disable-warning=ExperimentalWarning "$REPO/tests/host/slice-b-live.mjs" "$WT" "$TMP/run" "$BIN" "$BIR_BIN" "$BIR_DIR"
