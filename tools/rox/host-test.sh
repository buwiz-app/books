#!/usr/bin/env bash
# tools/rox/host-test.sh — the second-instance test: install Books in a real
# Rox v1 broker and call it through the host routes, headless
# (docs/06-testing-plan.md §13.2 steps 6-11).
#
# The throwaway broker follows the rox session's recipe (2026-10-29):
#   - a git worktree of the Rox repo at a pinned commit (never a branch switch
#     in the Rox checkout itself), with server/node_modules symlinked;
#   - plain Node (src/app-server.mjs), no rox-broker build;
#   - its own port (8810 or above; 8787 is the owner's, 8797-8803 the rox
#     session's), its own GROK_DATA_DIR, a per-run ROX_TRUST_FILE with a
#     throwaway test key (never gc-2026-01), and ROX_KEYSTORE=file:<dir>, so no
#     plugin key reaches the login Keychain.
# Then tests/host/books-host-test.mjs: inspect, install, resolve /books, open
# home, a signed books_status call, and the refusals. Everything is removed
# on exit (KEEP_TMP=1 keeps the temp dir; it holds test keys).
#
# Env: ROX_DIR (default /Volumes/goldcoders/Projects/rox)
#      ROX_COMMIT (default 1205f7a, the commit the rox session named)
#      BOOKS_HOST_PORT (default 8810)
#      BOOKS_TEST_KEY_ID (default books-test-01)
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ROX_DIR="${ROX_DIR:-/Volumes/goldcoders/Projects/rox}"
ROX_COMMIT="${ROX_COMMIT:-1205f7a}"
PORT="${BOOKS_HOST_PORT:-8810}"
KEY_ID="${BOOKS_TEST_KEY_ID:-books-test-01}"
PLUGIN_ID="dev.goldcoders.books"

die() { echo "host-test: $*" >&2; exit 2; }
[ "$KEY_ID" != "gc-2026-01" ] || die "the test key id must not be gc-2026-01"
[ "$PORT" -ge 8810 ] 2>/dev/null || die "use a port of 8810 or above (8787 is the owner's broker, 8797-8803 the rox session's tests)"
! lsof -nP -iTCP:"$PORT" -sTCP:LISTEN >/dev/null 2>&1 || die "port $PORT is in use"
[ -d "$ROX_DIR/server/node_modules/@modelcontextprotocol/sdk" ] || die "$ROX_DIR/server/node_modules is missing; install it in the Rox checkout yourself"
command -v node >/dev/null || die "node is needed to run Rox's broker and tools"

# the release binary
cargo build --release --quiet -p books-mini-app --manifest-path "$REPO/Cargo.toml"
BIN="$REPO/target/release/books-mini-app"

# The physical path, with no // and no /var -> /private/var symlink: Rox's
# scripts run only when their resolved URL equals the path node was given.
TMP="$(cd "$(mktemp -d "${TMPDIR%/}/books-host.XXXXXX")" && pwd -P)"
chmod 700 "$TMP"
case "$TMP" in "$REPO"/*) die "temp dir is inside the repo" ;; esac
WT="$TMP/rox"
SERVER_PID=""
cleanup() {
  if [ -n "$SERVER_PID" ]; then kill "$SERVER_PID" 2>/dev/null || true; wait "$SERVER_PID" 2>/dev/null || true; fi
  git -C "$ROX_DIR" worktree remove --force "$WT" >/dev/null 2>&1 || true
  if [ "${KEEP_TMP:-0}" = 1 ]; then echo "kept $TMP (holds test keys; delete it after)" >&2; else rm -rf "$TMP"; fi
}
trap cleanup EXIT

# a Rox worktree at the pinned commit; the Rox checkout's own branch is untouched
git -C "$ROX_DIR" worktree add --detach --quiet "$WT" "$ROX_COMMIT"
ln -s "$ROX_DIR/server/node_modules" "$WT/server/node_modules"
mkdir -m 700 "$TMP/data" "$TMP/keys" "$TMP/books-data"

# throwaway test key, per-run trust file, manifest signed with the test key
node "$WT/server/sdk/mini-app/bin/rox-plugin-test.mjs" --keygen "$TMP/test.key" >"$TMP/keygen.txt"
PUB="$(sed -n 's/.*Public key: \([A-Za-z0-9_-]*\).*/\1/p' "$TMP/keygen.txt")"
[ -n "$PUB" ] || die "keygen printed no public key"
cat >"$TMP/trust.json" <<JSON
{
  "v": 1,
  "about": "Per-run trust store for the Books host test (a throwaway test key; never gc-2026-01).",
  "publishers": { "$KEY_ID": { "publisher": "dev.goldcoders", "alg": "ed25519", "public_key": "$PUB" } },
  "approved_plugins": [ { "id": "$PLUGIN_ID", "publisher_key_id": "$KEY_ID" } ]
}
JSON
"$BIN" manifest --key-id "$KEY_ID" >"$TMP/manifest.json"
# every app the manifest lists, as the binary serves it
APP_ARGS=()
for id in $(node -e 'for (const a of JSON.parse(require("fs").readFileSync(process.argv[1], "utf8")).apps) console.log(a.id)' "$TMP/manifest.json"); do
  "$BIN" app-source "$id" >"$TMP/app-$id.tsx"
  APP_ARGS+=(--app "$id=$TMP/app-$id.tsx")
done
node "$WT/server/src/v1/sign-manifest.mjs" "$TMP/manifest.json" --key-file "$TMP/test.key" \
  "${APP_ARGS[@]}" --out "$TMP/signed.json"

# the throwaway broker
(
  cd "$WT/server"
  PORT="$PORT" GROK_DATA_DIR="$TMP/data" ROX_TRUST_FILE="$TMP/trust.json" ROX_KEYSTORE="file:$TMP/keys" \
    exec node --disable-warning=ExperimentalWarning src/app-server.mjs >"$TMP/broker.log" 2>&1
) &
SERVER_PID=$!
for _ in $(seq 1 150); do
  if [ -s "$TMP/data/host-token" ] && lsof -nP -iTCP:"$PORT" -sTCP:LISTEN >/dev/null 2>&1; then break; fi
  sleep 0.2
done
[ -s "$TMP/data/host-token" ] || { tail -20 "$TMP/broker.log" >&2; die "the broker did not start"; }

set +e
node --disable-warning=ExperimentalWarning "$REPO/tests/host/books-host-test.mjs" "$WT" "$TMP" "$BIN" "$PORT"
STATUS=$?
set -e
if security find-generic-password -s dev.goldcoders.rox.plugin-key -a "$PLUGIN_ID" >/dev/null 2>&1; then
  echo "FAIL  a $PLUGIN_ID plugin key is in the login Keychain" >&2
  STATUS=1
else
  echo "PASS  no $PLUGIN_ID plugin key in the login Keychain (file keystore used)"
fi
exit "$STATUS"
