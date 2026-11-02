#!/usr/bin/env bash
# tools/bir/slice-b-live.sh — Slice B, live: Books' Q3 2551Q into a real eBIRForms draft on a scratch eBIRForms
# data folder, and a later Books change marks the hand-off out of date (tests/host/slice-b-live.mjs). Headless, no
# port. Not a check-all gate.
#
#   - the Bot's v1 broker modules from a git worktree of the Bot repo (never a branch switch in the Bot checkout
#     itself), with server/node_modules symlinked;
#   - release builds of books-mini-app (here) and of ebirforms-mini-app and its host-seed example (in EBIRFORMS_DIR),
#     under the Bot's scripts/wait-build lock;
#   - eBIRForms' data folder in this run's temp dir, seeded with eBIRForms' own synthetic taxpayer (host-seed), with
#     BUWIZ_NO_KEYCHAIN=1 and its Buwiz account URL on a closed loopback port, so nothing reaches the owner's data,
#     the Keychain or the network;
#   - one throwaway test key (books-test-01, never gc-2026-01) and a trust file of this run only.
# Everything is removed on exit (KEEP_TMP=1 keeps the temp dir; it holds test keys and the scratch database).
#
# Env: EBIRFORMS_DIR     an eBIRForms mini app checkout (github.com/buwiz-app/ebirforms) (required)
#      BUWIZ_BOT_DIR     a Buwiz Bot checkout with server/node_modules installed (required)
#      BUWIZ_BOT_COMMIT  default origin/main
#      EBIRFORMS_TARGET_DIR  eBIRForms' cargo target dir (default EBIRFORMS_DIR/target)
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BOT_DIR="${BUWIZ_BOT_DIR:-}"
BOT_COMMIT="${BUWIZ_BOT_COMMIT:-origin/main}"
EBIR_DIR="${EBIRFORMS_DIR:-}"

die() { echo "slice-b-live: $*" >&2; exit 2; }
[ -f "$EBIR_DIR/crates/ebirforms-mini-app/Cargo.toml" ] || die "set EBIRFORMS_DIR to an eBIRForms mini app checkout"
[ -d "$BOT_DIR/server/node_modules/@modelcontextprotocol/sdk" ] || die "set BUWIZ_BOT_DIR to a Bot checkout with server/node_modules installed"
command -v node >/dev/null || die "node is needed to run the Bot's broker modules"
WAIT="$BOT_DIR/scripts/wait-build"
[ -x "$WAIT" ] || WAIT=""

${WAIT:+"$WAIT"} env CARGO_TARGET_DIR="$REPO/target" cargo build --release --quiet -p books-mini-app --manifest-path "$REPO/Cargo.toml"
BIN="$REPO/target/release/books-mini-app"
EBIR_TARGET="${EBIRFORMS_TARGET_DIR:-$EBIR_DIR/target}"
${WAIT:+"$WAIT"} env CARGO_TARGET_DIR="$EBIR_TARGET" cargo build --release --quiet -p ebirforms-mini-app --bin ebirforms-mini-app \
  --example host-seed --manifest-path "$EBIR_DIR/Cargo.toml"
EBIR_BIN="$EBIR_TARGET/release/ebirforms-mini-app"
[ -x "$EBIR_BIN" ] || die "no ebirforms-mini-app at $EBIR_BIN"

# the physical path: the Bot's modules compare resolved paths
TMP="$(cd "$(mktemp -d "${TMPDIR%/}/books-slice-b.XXXXXX")" && pwd -P)"
chmod 700 "$TMP"
case "$TMP" in "$REPO"/*) die "temp dir is inside the repo" ;; esac
WT="$TMP/bot"
cleanup() {
  git -C "$BOT_DIR" worktree remove --force "$WT" >/dev/null 2>&1 || true
  git -C "$BOT_DIR" worktree prune
  if [ "${KEEP_TMP:-0}" = 1 ]; then echo "kept $TMP (test keys and the scratch database; delete it after)" >&2; else rm -rf "$TMP"; fi
}
trap cleanup EXIT

git -C "$BOT_DIR" worktree add --detach --quiet "$WT" "$BOT_COMMIT"
ln -s "$BOT_DIR/server/node_modules" "$WT/server/node_modules"
mkdir -m 700 "$TMP/run" "$TMP/run/ebirforms"

# eBIRForms' synthetic taxpayer in the scratch data folder
touch "$TMP/run/ebirforms/host-test-scratch"
EBIRFORMS_DATA_DIR="$TMP/run/ebirforms" BUWIZ_NO_KEYCHAIN=1 "$EBIR_TARGET/release/examples/host-seed"

node --disable-warning=ExperimentalWarning "$REPO/tests/host/slice-b-live.mjs" "$WT" "$TMP/run" "$BIN" "$EBIR_BIN"
