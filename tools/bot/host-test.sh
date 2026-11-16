#!/usr/bin/env bash
# tools/bot/host-test.sh — the second-instance test: install Books in a real
# Buwiz Bot broker (the Rust buwiz-broker) and call it through the host routes,
# headless.
#
# The throwaway broker:
#   - the buwiz-broker binary named by BUWIZ_BROKER_BIN, started with
#     BUWIZ_BROKER_HUB=1 and --no-node, so it runs no Node;
#   - its own port (8811 or above; 8787 is the owner's), its own --data dir, a
#     per-run BUWIZ_TRUST_FILE with a throwaway test key (never gc-2026-01), and
#     BUWIZ_KEYSTORE=file:<dir> with BUWIZ_NO_KEYCHAIN=1, so no plugin key
#     reaches any Keychain;
#   - an empty environment of its own: a temporary HOME and a PATH of /usr/bin
#     and /bin that holds no `secret` command, so it cannot reach the owner's
#     stores or the real data folder.
# Then tests/host/books-host-test.mjs: inspect, install, resolve /books, open
# home, a signed books_status call, the refusals, and the Books window's source
# run on tests/host/nsh-standin.mjs: type into a TextBox, change a picker.
# Everything is removed on exit (KEEP_TMP=1 keeps the temp dir; it holds test
# keys).
#
# Env: BUWIZ_BROKER_BIN (required): a buwiz-broker binary (an executable file)
#      BUWIZ_SDK_DIR (required): a buwiz-sdk checkout at the pinned revision with
#        its dependencies installed (bun install); its runner makes the test key,
#        the test client imports its src/v1/ modules (JCS, ed25519, permissions
#        digest), and its esbuild compiles the Books window's source
#      BOOKS_HOST_PORT (default 8811, 8811 or above)
#      BOOKS_TEST_KEY_ID (default books-test-01)
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
BROKER="${BUWIZ_BROKER_BIN:-}"
SDK="${BUWIZ_SDK_DIR:-}"
[ -z "$SDK" ] || SDK="$(cd "$SDK" 2>/dev/null && pwd -P)" || { echo "host-test: BUWIZ_SDK_DIR ${BUWIZ_SDK_DIR} is not a directory" >&2; exit 2; }
PORT="${BOOKS_HOST_PORT:-8811}"
KEY_ID="${BOOKS_TEST_KEY_ID:-books-test-01}"
PLUGIN_ID="app.buwiz.books"
RUNNER="$SDK/packages/conformance/bin/buwiz-plugin-test.mjs"
BROKER_PATH="/usr/bin:/bin"

die() { echo "host-test: $*" >&2; exit 2; }
[ -n "$BROKER" ] || die "set BUWIZ_BROKER_BIN to a buwiz-broker binary (built from buwiz-bot main)"
[ -f "$BROKER" ] && [ -x "$BROKER" ] || die "BUWIZ_BROKER_BIN $BROKER is not an executable file"
[ -n "$SDK" ] || die "set BUWIZ_SDK_DIR to a buwiz-sdk checkout (git clone https://github.com/buwiz-app/buwiz-sdk; cd buwiz-sdk; bun install)"
[ -f "$RUNNER" ] || die "$RUNNER not found: is BUWIZ_SDK_DIR a buwiz-sdk checkout?"
[ "$KEY_ID" != "gc-2026-01" ] || die "the test key id must not be gc-2026-01"
[ "$PORT" -ge 8811 ] 2>/dev/null || die "use a port of 8811 or above (8787 is the owner's broker)"
! lsof -nP -iTCP:"$PORT" -sTCP:LISTEN >/dev/null 2>&1 || die "port $PORT is in use"
! PATH="$BROKER_PATH" command -v secret >/dev/null 2>&1 || die "the broker's PATH ($BROKER_PATH) holds a command named secret"
PIN="$("$REPO/tools/sdk-pin.sh")" || die "the buwiz-sdk pins are not one revision (tools/sdk-pin.sh says why)"
# the checkout must hold exactly the pinned files (a merge of the pin has another commit id)
SDK_TREE="$(git -C "$SDK" rev-parse 'HEAD^{tree}' 2>/dev/null)" || die "$SDK is not a git checkout of buwiz-sdk"
[ "$(git -C "$SDK" rev-parse "$PIN^{tree}" 2>/dev/null)" = "$SDK_TREE" ] ||
  die "the SDK checkout is not the pinned revision $PIN: git -C \"\$BUWIZ_SDK_DIR\" fetch && git -C \"\$BUWIZ_SDK_DIR\" checkout $PIN"
git -C "$SDK" diff --quiet HEAD -- packages/conformance || die "the SDK checkout has uncommitted changes under packages/conformance"
[ -z "$(git -C "$SDK" status --porcelain --untracked-files=all -- packages/conformance)" ] || die "the SDK checkout has untracked files under packages/conformance"
command -v node >/dev/null || die "node is needed to run the test client and the SDK's runner"
(cd "$SDK/packages/conformance" && node --input-type=module -e 'await import("esbuild")') >/dev/null 2>&1 ||
  die "the SDK's esbuild cannot be loaded: run bun install in $SDK"

TMP_BASE="${TMPDIR:-/tmp}"
TMP_BASE="$(cd "$TMP_BASE" && pwd -P)" || die "TMPDIR must name an existing directory"
case "$TMP_BASE" in ""|"$REPO"|"$REPO"/*) die "TMPDIR is empty or inside the repo" ;; esac
if ! TMP="$(mktemp -d "${TMP_BASE%/}/books-host.XXXXXX")"; then
  die "could not create a temp dir under $TMP_BASE"
fi
TMP="$(cd "$TMP" && pwd -P)" || die "could not resolve the temp dir"
case "$TMP" in ""|"$REPO"|"$REPO"/*) die "temp dir is empty or inside the repo" ;; esac
chmod 700 "$TMP"
SERVER_PID=""
cleanup() {
  if [ -n "$SERVER_PID" ]; then kill "$SERVER_PID" 2>/dev/null || true; wait "$SERVER_PID" 2>/dev/null || true; fi
  if [ -n "$TMP" ]; then
    case "$TMP" in ""|"$REPO"|"$REPO"/*) echo "host-test: refusing unsafe temp cleanup path" >&2 ;;
      *) if [ "${KEEP_TMP:-0}" = 1 ]; then echo "kept $TMP (holds test keys; delete it after)" >&2; else rm -rf "$TMP"; fi ;;
    esac
  fi
}
trap cleanup EXIT

# the release binary
cargo build --release --quiet -p books-mini-app --manifest-path "$REPO/Cargo.toml"
BIN="$REPO/target/release/books-mini-app"

mkdir -m 700 "$TMP/home" "$TMP/data" "$TMP/keys" "$TMP/books-data"

# throwaway test key, per-run trust file, manifest signed with the test key
node "$RUNNER" --keygen "$TMP/test.key" >"$TMP/keygen.txt"
PUB="$(sed -n 's/.*Public key: \([A-Za-z0-9_-]*\).*/\1/p' "$TMP/keygen.txt")"
[ -n "$PUB" ] || die "keygen printed no public key"
cat >"$TMP/trust.json" <<JSON
{
  "v": 1,
  "about": "Per-run trust store for the Books host test (a throwaway test key; never gc-2026-01).",
  "publishers": { "$KEY_ID": { "publisher": "app.buwiz", "alg": "ed25519", "public_key": "$PUB", "first_party": true } },
  "approved_plugins": [ { "id": "$PLUGIN_ID", "publisher_key_id": "$KEY_ID" } ]
}
JSON
"$BIN" manifest --key-id "$KEY_ID" >"$TMP/manifest.json"
# every TSX app the manifest lists, as the binary serves it
APP_ARGS=()
for id in $(node -e 'for (const a of JSON.parse(require("fs").readFileSync(process.argv[1], "utf8")).apps) if (!a.native) console.log(a.id)' "$TMP/manifest.json"); do
  "$BIN" app-source "$id" >"$TMP/app-$id.tsx"
  APP_ARGS+=(--app "$id=$TMP/app-$id.tsx")
done
"$BROKER" sign-manifest "$TMP/manifest.json" --key-file "$TMP/test.key" \
  ${APP_ARGS[@]+"${APP_ARGS[@]}"} --out "$TMP/signed.json"

# the throwaway broker, in an environment of its own
env -i HOME="$TMP/home" PATH="$BROKER_PATH" TMPDIR="$TMP" \
  BUWIZ_BROKER_HUB=1 BUWIZ_BROKER_NO_NODE=1 BUWIZ_NO_KEYCHAIN=1 \
  BUWIZ_TRUST_FILE="$TMP/trust.json" BUWIZ_KEYSTORE="file:$TMP/keys" \
  "$BROKER" --port "$PORT" --data "$TMP/data" --no-node >"$TMP/broker.log" 2>&1 &
SERVER_PID=$!
for _ in $(seq 1 150); do
  if [ -s "$TMP/data/host-token" ] && lsof -nP -iTCP:"$PORT" -sTCP:LISTEN >/dev/null 2>&1; then break; fi
  kill -0 "$SERVER_PID" 2>/dev/null || break
  sleep 0.2
done
[ -s "$TMP/data/host-token" ] || { tail -20 "$TMP/broker.log" >&2; die "the broker did not start"; }

# name the broker this run used: what it reports about itself, and the binary's hash (it has no version command)
META="$(curl -fsS --max-time 5 "http://127.0.0.1:$PORT/meta" 2>/dev/null | tr -d '\n' | cut -c1-200)" || META="(no /meta answer)"
echo "broker: $META; sha256 $(shasum -a 256 "$BROKER" | cut -d' ' -f1)"

set +e
node --disable-warning=ExperimentalWarning "$REPO/tests/host/books-host-test.mjs" "$TMP" "$BIN" "$PORT" "$SDK/packages/conformance"
STATUS=$?
set -e
# BUWIZ_KEYSTORE=file:<dir> selects the broker's file keystore; the Keychain is the other choice of that
# setting (helper/keystore.rs), never used with it. So a plugin key in <dir> proves the run kept its keys there.
if [ ! -s "$TMP/keys/$PLUGIN_ID.key" ]; then
  echo "FAIL  the file keystore holds no $PLUGIN_ID plugin key" >&2
  STATUS=1
else
  echo "PASS  the plugin key is in the file keystore of the run's temp dir"
fi
exit "$STATUS"
