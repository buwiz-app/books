#!/usr/bin/env bash
# tools/bir/slice-b-live.sh — Slice B, live: Books' Q3 2551Q into a real eBIRForms draft on a scratch eBIRForms
# data folder, and a later Books change marks the hand-off out of date (tests/host/slice-b-live.mjs). Not a
# check-all gate.
#
#   - the Rust buwiz-broker named by BUWIZ_BROKER_BIN, on loopback, in an environment of its own (`env -i`): a
#     temporary HOME, a PATH of system tools that holds no `secret` command, BUWIZ_KEYSTORE=file:<dir> and
#     BUWIZ_NO_KEYCHAIN=1, so no plugin key reaches a Keychain, and a model that is a scripted stand-in on loopback;
#   - release builds of books-mini-app (here) and of ebirforms-mini-app and its host-seed example (in EBIRFORMS_DIR);
#   - eBIRForms' data folder in this run's temp dir, seeded with a synthetic taxpayer (eBIRForms' host-seed), with
#     BUWIZ_NO_KEYCHAIN=1 and its Buwiz account URL on a closed loopback port, so nothing reaches the owner's data,
#     the Keychain or the network;
#   - one throwaway test key (books-test-01, never gc-2026-01), a trust file of this run only, and both manifests
#     signed with `buwiz-broker sign-manifest`.
# Everything is removed on exit (KEEP_TMP=1 keeps the temp dir; it holds test keys and the scratch database).
#
# Env: EBIRFORMS_DIR     an eBIRForms mini app checkout (github.com/buwiz-app/ebirforms) (required)
#      BUWIZ_BROKER_BIN  a buwiz-broker binary, an executable file (required)
#      BUWIZ_SDK_DIR     a buwiz-sdk checkout at the pinned revision with `bun install` run (required): its runner
#                        makes the test key, and the test client imports its src/v1/ modules
#      SLICE_B_PORT      the broker's port (default 8812, 8811 or above; the scripted model takes the next one)
#      EBIRFORMS_TARGET_DIR  eBIRForms' cargo target dir (default EBIRFORMS_DIR/target)
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
EBIR_DIR="${EBIRFORMS_DIR:-}"
BROKER="${BUWIZ_BROKER_BIN:-}"
SDK="${BUWIZ_SDK_DIR:-}"
[ -z "$SDK" ] || SDK="$(cd "$SDK" 2>/dev/null && pwd -P)" || { echo "slice-b-live: BUWIZ_SDK_DIR ${BUWIZ_SDK_DIR} is not a directory" >&2; exit 2; }
PORT="${SLICE_B_PORT:-8812}"
MODEL_PORT=$((PORT + 1))
KEY_ID="books-test-01"
BOOKS_ID="app.buwiz.books"
EBIR_ID="app.buwiz.ebirforms"
RUNNER="$SDK/packages/conformance/bin/buwiz-plugin-test.mjs"
BROKER_PATH="/usr/bin:/bin:/usr/sbin:/sbin"

die() { echo "slice-b-live: $*" >&2; exit 2; }
[ -f "$EBIR_DIR/crates/ebirforms-mini-app/Cargo.toml" ] || die "set EBIRFORMS_DIR to an eBIRForms mini app checkout"
[ -n "$BROKER" ] || die "set BUWIZ_BROKER_BIN to a buwiz-broker binary (built from buwiz-bot main)"
[ -f "$BROKER" ] && [ -x "$BROKER" ] || die "BUWIZ_BROKER_BIN $BROKER is not an executable file"
[ -n "$SDK" ] || die "set BUWIZ_SDK_DIR to a buwiz-sdk checkout (git clone https://github.com/buwiz-app/buwiz-sdk; cd buwiz-sdk; bun install)"
[ -f "$RUNNER" ] || die "$RUNNER not found: is BUWIZ_SDK_DIR a buwiz-sdk checkout?"
[ "$KEY_ID" != "gc-2026-01" ] || die "the test key id must not be gc-2026-01"
[[ "$PORT" =~ ^[0-9]+$ ]] && [ "$PORT" -ge 8811 ] && [ "$PORT" -le 65534 ] || die "use a port from 8811 through 65534 (the scripted model takes the next one)"
command -v node >/dev/null || die "node is needed to run the test client and the SDK's runner"
command -v lsof >/dev/null || die "lsof is needed"
for p in "$PORT" "$MODEL_PORT"; do
  ! lsof -nP -iTCP:"$p" -sTCP:LISTEN >/dev/null 2>&1 || die "port $p is in use (the broker takes $PORT, the scripted model $MODEL_PORT)"
done
! PATH="$BROKER_PATH" command -v secret >/dev/null 2>&1 || die "the broker's PATH ($BROKER_PATH) holds a command named secret"
PIN="$("$REPO/tools/sdk-pin.sh")" || die "the buwiz-sdk pins are not one revision (tools/sdk-pin.sh says why)"
# the checkout must hold exactly the pinned files (a merge of the pin has another commit id)
SDK_TREE="$(git -C "$SDK" rev-parse 'HEAD^{tree}' 2>/dev/null)" || die "$SDK is not a git checkout of buwiz-sdk"
[ "$(git -C "$SDK" rev-parse "$PIN^{tree}" 2>/dev/null)" = "$SDK_TREE" ] ||
  die "the SDK checkout is not the pinned revision $PIN: git -C \"\$BUWIZ_SDK_DIR\" fetch && git -C \"\$BUWIZ_SDK_DIR\" checkout $PIN"
[ -z "$(git -C "$SDK" status --porcelain --untracked-files=all -- packages/conformance)" ] || die "the SDK checkout has changed or untracked files under packages/conformance"

# (No build lock: the Bot checkout's scripts/wait-build served builds that shared a machine with the Bot's own, and
# this script no longer needs that checkout.)
env CARGO_TARGET_DIR="$REPO/target" cargo build --release --quiet -p books-mini-app --manifest-path "$REPO/Cargo.toml"
BIN="$REPO/target/release/books-mini-app"
EBIR_TARGET="${EBIRFORMS_TARGET_DIR:-$EBIR_DIR/target}"
env CARGO_TARGET_DIR="$EBIR_TARGET" cargo build --release --quiet -p ebirforms-mini-app --bin ebirforms-mini-app \
  --example host-seed --manifest-path "$EBIR_DIR/Cargo.toml"
EBIR_BIN="$EBIR_TARGET/release/ebirforms-mini-app"
[ -x "$EBIR_BIN" ] || die "no ebirforms-mini-app at $EBIR_BIN"

# the physical path
TMP="$(cd "$(mktemp -d "${TMPDIR:-/tmp}/books-slice-b.XXXXXX")" && pwd -P)"
chmod 700 "$TMP"
case "$TMP" in "$REPO"/*) die "temp dir is inside the repo" ;; esac
BROKER_PID=""
cleanup() {
  if [ -n "$BROKER_PID" ] && kill -0 "$BROKER_PID" 2>/dev/null; then
    kill "$BROKER_PID" 2>/dev/null || true
    for _ in $(seq 1 50); do kill -0 "$BROKER_PID" 2>/dev/null || break; sleep 0.1; done
    kill -9 "$BROKER_PID" 2>/dev/null || true
  fi
  if [ "${KEEP_TMP:-0}" = 1 ]; then echo "kept $TMP (test keys and the scratch database; delete it after)" >&2; else rm -rf "$TMP"; fi
}
trap cleanup EXIT
mkdir -m 700 "$TMP/home" "$TMP/data" "$TMP/keys" "$TMP/assets" "$TMP/ebirforms" "$TMP/books-data"

# eBIRForms' synthetic taxpayer in the scratch data folder, with a root made here so the test can check it never
# leaks (never printed; a random synthetic number, not a real TIN)
touch "$TMP/ebirforms/host-test-scratch"
ROOT="$(od -An -N4 -tu4 /dev/urandom | tr -d ' ')"
ROOT="$(printf '%09d' $(( ROOT % 999999999 + 1 )))"
# the plugins' environment, written once: the manifests and the seed here, the plugins' specs in the test client
# (which reads these from its environment and stops if one is missing)
BOOKS_ENV=(BOOKS_DATA_DIR="$TMP/books-data" BOOKS_TODAY="2026-10-29")
EBIR_ENV=(EBIRFORMS_DATA_DIR="$TMP/ebirforms" BUWIZ_NO_KEYCHAIN=1 EBIRFORMS_ACCOUNT_URL="http://127.0.0.1:1")
env -i PATH="$BROKER_PATH" HOME="$TMP/home" TMPDIR="$TMP" "${EBIR_ENV[@]}" EBIRFORMS_SEED_TIN_ROOT="$ROOT" \
  "$EBIR_TARGET/release/examples/host-seed"

# a throwaway test key (0600) from the SDK; only its public key is printed
node "$RUNNER" --keygen "$TMP/test.key" >"$TMP/keygen.txt"
PUB="$(sed -n 's/.*Public key: \([A-Za-z0-9_-]*\).*/\1/p' "$TMP/keygen.txt")"
[ -n "$PUB" ] || die "keygen printed no public key"
cat >"$TMP/trust.json" <<JSON
{
  "v": 1,
  "about": "Per-run trust store for Books' live Slice B (a throwaway test key; never gc-2026-01).",
  "publishers": { "$KEY_ID": { "publisher": "app.buwiz", "alg": "ed25519", "public_key": "$PUB", "first_party": true } },
  "approved_plugins": [ { "id": "$BOOKS_ID", "publisher_key_id": "$KEY_ID" }, { "id": "$EBIR_ID", "publisher_key_id": "$KEY_ID" } ]
}
JSON

# the broker's isolation, for the signer and the broker alike
ISOLATED=(env -i PATH="$BROKER_PATH" HOME="$TMP/home" TMPDIR="$TMP"
  BUWIZ_KEYSTORE="file:$TMP/keys" BUWIZ_NO_KEYCHAIN=1 BUWIZ_TRUST_FILE="$TMP/trust.json")

# each manifest under the test key id, with every TSX app as the binary serves it, signed by the broker
sign_plugin() { # <name> <binary> <env assignments...>
  local name="$1" bin="$2" id; shift 2
  local app_args=()
  env -i PATH="$BROKER_PATH" HOME="$TMP/home" TMPDIR="$TMP" "$@" "$bin" manifest --key-id "$KEY_ID" >"$TMP/$name-manifest.json"
  for id in $(node -e 'for (const a of JSON.parse(require("fs").readFileSync(process.argv[1], "utf8")).apps) if (!a.native) console.log(a.id)' "$TMP/$name-manifest.json"); do
    env -i PATH="$BROKER_PATH" HOME="$TMP/home" TMPDIR="$TMP" "$@" "$bin" app-source "$id" >"$TMP/$name-app-$id.tsx"
    app_args+=(--app "$id=$TMP/$name-app-$id.tsx")
  done
  "${ISOLATED[@]}" "$BROKER" sign-manifest "$TMP/$name-manifest.json" --key-file "$TMP/test.key" \
    ${app_args[@]+"${app_args[@]}"} --out "$TMP/$name.json"
}
sign_plugin books "$BIN" "${BOOKS_ENV[@]}"
sign_plugin ebirforms "$EBIR_BIN" "${EBIR_ENV[@]}"

# the broker: its own home, data folder, keystore and trust file; no Keychain, no update checks; its model is the
# test's scripted one on loopback
"${ISOLATED[@]}" \
  BUWIZ_BROKER_HUB=1 BUWIZ_BROKER_V1_HOST=1 BUWIZ_UPDATE_CHECK=0 \
  ANTHROPIC_BASE_URL="http://127.0.0.1:$MODEL_PORT" OPENCODEX_URL="http://127.0.0.1:$MODEL_PORT" ANTHROPIC_API_KEY=test-key \
  "$BROKER" --port "$PORT" --no-node --assets "$TMP/assets" --data "$TMP/data" >"$TMP/broker.log" 2>&1 &
BROKER_PID=$!

set +e
env "${BOOKS_ENV[@]}" "${EBIR_ENV[@]}" SLICE_B_TIN_ROOT="$ROOT" node --disable-warning=ExperimentalWarning "$REPO/tests/host/slice-b-live.mjs" \
  "$TMP" "$BIN" "$EBIR_BIN" "$PORT" "$MODEL_PORT" "$BROKER_PID" "$SDK/packages/conformance"
STATUS=$?
set -e
# BUWIZ_KEYSTORE=file:<dir> selects the broker's file keystore, never the Keychain: a key for each plugin in <dir>
# proves the run kept its keys there.
for id in "$BOOKS_ID" "$EBIR_ID"; do
  if [ ! -s "$TMP/keys/$id.key" ]; then
    echo "FAIL  the file keystore holds no $id plugin key" >&2
    STATUS=1
  else
    echo "PASS  the $id plugin key is in the file keystore of the run's temp dir"
  fi
done
exit "$STATUS"
