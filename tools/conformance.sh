#!/usr/bin/env bash
# tools/conformance.sh: the SDK's plugin conformance runner (buwiz-plugin-test)
# against the release build of books-mini-app, under a throwaway key and a
# per-run trust file. Nothing here touches the Keychain, a Bot's data or the
# release key.
#
# Steps 1-5, then the runner:
#   1. build the release binary;
#   2. a fresh 0700 temp dir outside the repo;
#   3. a throwaway ed25519 test key (buwiz-plugin-test --keygen);
#   4. a per-run trust file mapping a TEST key id to that key, marking it
#      first-party and approving app.buwiz.books under it (never gc-2026-01);
#   5. the manifest generated under the test key id and signed with the test
#      key file by `buwiz-broker sign-manifest` when BUWIZ_BROKER_BIN names a
#      broker build, else by the SDK's sign-manifest.mjs (same arguments,
#      byte-identical output);
#   then buwiz-plugin-test --manifest <signed> --key-file <test key>, whose
#   own in-process broker uses a temp trust store of the same shape.
#
# The release manifest keeps publisher.key_id gc-2026-01; only the owner
# signs that one, with tools/release/publish.sh.
#
# Usage: tools/conformance.sh [runner args, e.g. --json]
# Env:   BUWIZ_SDK_DIR (required): a buwiz-sdk checkout
#          (https://github.com/buwiz-app/buwiz-sdk) at the revision this repo
#          pins (the root Cargo.toml), with its dependencies
#          installed (bun install)
#        BUWIZ_BROKER_BIN (a buwiz-broker binary; signs in place of the SDK's
#          sign-manifest.mjs)
#        BOOKS_TEST_KEY_ID (default books-test-01)
#        KEEP_TMP=1 keeps the temp dir (it holds a test key; delete it after)
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
SDK="${BUWIZ_SDK_DIR:-}"
KEY_ID="${BOOKS_TEST_KEY_ID:-books-test-01}"
PLUGIN_ID="app.buwiz.books"
RUNNER="$SDK/packages/conformance/bin/buwiz-plugin-test.mjs"
SIGNER="$SDK/packages/conformance/src/v1/sign-manifest.mjs"

die() { echo "conformance: $*" >&2; exit 2; }
[ -n "$SDK" ] || die "set BUWIZ_SDK_DIR to a buwiz-sdk checkout (git clone https://github.com/buwiz-app/buwiz-sdk; cd buwiz-sdk; bun install)"
if [ -n "${BUWIZ_BROKER_BIN:-}" ]; then
  [ -x "$BUWIZ_BROKER_BIN" ] || die "BUWIZ_BROKER_BIN $BUWIZ_BROKER_BIN is not executable"
  SIGN=("$BUWIZ_BROKER_BIN" sign-manifest)
else
  SIGN=(node "$SIGNER")
fi
[ "$KEY_ID" != "gc-2026-01" ] || die "the test key id must not be gc-2026-01"
for f in "$RUNNER" "$SIGNER"; do [ -f "$f" ] || die "$f not found: is BUWIZ_SDK_DIR a buwiz-sdk checkout?"; done
PIN="$("$REPO/tools/sdk-pin.sh")" || die "the buwiz-sdk pins are not one revision (tools/sdk-pin.sh says why)"
# the checkout must hold exactly the pinned files (a merge of the pin has another commit id)
SDK_TREE="$(git -C "$SDK" rev-parse 'HEAD^{tree}' 2>/dev/null)" || die "$SDK is not a git checkout of buwiz-sdk"
[ "$(git -C "$SDK" rev-parse "$PIN^{tree}" 2>/dev/null)" = "$SDK_TREE" ] ||
  die "the SDK checkout is not the pinned revision $PIN: git -C \"\$BUWIZ_SDK_DIR\" fetch && git -C \"\$BUWIZ_SDK_DIR\" checkout $PIN"
git -C "$SDK" diff --quiet HEAD -- packages/conformance || die "the SDK checkout has uncommitted changes under packages/conformance"
[ -z "$(git -C "$SDK" status --porcelain --untracked-files=all -- packages/conformance)" ] || die "the SDK checkout has untracked files under packages/conformance"
command -v node >/dev/null || die "node is needed to run the SDK's tools"
(cd "$SDK/packages/conformance" && node --input-type=module -e 'await import("esbuild"); await import("@modelcontextprotocol/sdk/client/index.js"); await import("@modelcontextprotocol/sdk/client/stdio.js"); await import("node:sqlite")') >/dev/null 2>&1 ||
  die "the SDK's runner cannot load its dependencies or node:sqlite: it needs Node 22.13 or newer (node --version), and bun install run in $SDK"

# 2. a fresh temp dir outside the repo, removed on exit
TMP_BASE="${TMPDIR:-/tmp}"
TMP_BASE="$(cd "$TMP_BASE" && pwd -P)" || { echo "conformance: TMPDIR must name an existing directory" >&2; exit 2; }
case "$TMP_BASE" in ""|"$REPO"|"$REPO"/*) echo "conformance: TMPDIR is empty or inside the repo" >&2; exit 2 ;; esac
if ! TMP="$(mktemp -d "${TMP_BASE%/}/books-conformance.XXXXXX")"; then
  echo "conformance: could not create a temp dir under $TMP_BASE" >&2
  exit 2
fi
TMP="$(cd "$TMP" && pwd -P)" || { echo "conformance: could not resolve the temp dir" >&2; exit 2; }
case "$TMP" in ""|"$REPO"|"$REPO"/*) echo "conformance: temp dir is empty or inside the repo" >&2; exit 2 ;; esac
chmod 700 "$TMP"
cleanup() {
  case "$TMP" in ""|"$REPO"|"$REPO"/*) echo "conformance: refusing unsafe temp cleanup path" >&2 ;;
    *) if [ "${KEEP_TMP:-0}" = 1 ]; then echo "kept $TMP (holds a test key)" >&2; else rm -rf "$TMP"; fi ;;
  esac
}
trap cleanup EXIT

# 1. the release binary
cargo build --release --quiet -p books-mini-app --manifest-path "$REPO/Cargo.toml"
BIN="$REPO/target/release/books-mini-app"

# 3. a throwaway test key (0600); the runner prints only its public key
node "$RUNNER" --keygen "$TMP/test.key" >"$TMP/keygen.txt"
PUB="$(sed -n 's/.*Public key: \([A-Za-z0-9_-]*\).*/\1/p' "$TMP/keygen.txt")"
[ -n "$PUB" ] || { echo "conformance: keygen printed no public key" >&2; exit 1; }

# 4. the per-run trust file: the TEST key id only
cat >"$TMP/trust.json" <<JSON
{
  "v": 1,
  "about": "Per-run trust store for Books conformance (a throwaway test key; never a release key).",
  "publishers": { "$KEY_ID": { "publisher": "app.buwiz", "alg": "ed25519", "public_key": "$PUB", "first_party": true } },
  "approved_plugins": [ { "id": "$PLUGIN_ID", "publisher_key_id": "$KEY_ID" } ]
}
JSON
export BUWIZ_TRUST_FILE="$TMP/trust.json"

# 5. generate under the test key id, sign with the test key file
"$BIN" manifest --key-id "$KEY_ID" >"$TMP/manifest.json"
# every TSX app the manifest lists, as the binary serves it
APP_ARGS=()
for id in $(node -e 'for (const a of JSON.parse(require("fs").readFileSync(process.argv[1], "utf8")).apps) if (!a.native) console.log(a.id)' "$TMP/manifest.json"); do
  "$BIN" app-source "$id" >"$TMP/app-$id.tsx"
  APP_ARGS+=(--app "$id=$TMP/app-$id.tsx")
done
"${SIGN[@]}" "$TMP/manifest.json" --key-file "$TMP/test.key" \
  ${APP_ARGS[@]+"${APP_ARGS[@]}"} --out "$TMP/signed.json"

# the runner: spawn the binary the way the broker does, serving the signed
# manifest through BOOKS_V1_MANIFEST; print its result verbatim
set +e
node --disable-warning=ExperimentalWarning "$RUNNER" \
  --command "$BIN" --args mcp \
  --manifest "$TMP/signed.json" --key-file "$TMP/test.key" \
  --manifest-env BOOKS_V1_MANIFEST \
  --env "BOOKS_DATA_DIR=$TMP/books-data" "$@"
STATUS=$?
set -e
exit "$STATUS"
