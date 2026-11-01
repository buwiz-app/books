#!/usr/bin/env bash
# tools/release/prepare.sh — everything the release needs except the
# signature (M9-04, M9-05 are the owners'). It builds the release binary
# and writes, to dist/release/ (or $1):
#   manifest.json     unsigned, publisher.key_id gc-2026-01
#   app-<id>.tsx      each TSX app's source, exactly as the binary serves it
# then prints the three owner steps: the trust entry (Bot owner), the
# signature with gc-2026-01 from the login Keychain (the owner), and the
# install in the owner's Buwiz Bot. This script never reads a key or the Keychain.
set -euo pipefail
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT="${1:-$REPO/dist/release}"
BOT="${BUWIZ_BOT_DIR:-/Volumes/goldcoders/Projects/rox}"
# the signer the owner runs: rox-broker (buwiz-bot main) when BUWIZ_BROKER_BIN names
# one, else the Bot's server/src/v1/sign-manifest.mjs; the same arguments either way
if [ -n "${BUWIZ_BROKER_BIN:-}" ]; then SIGNER="$BUWIZ_BROKER_BIN sign-manifest"; else SIGNER="node $BOT/server/src/v1/sign-manifest.mjs"; fi
cd "$REPO"
cargo build --release -p books-mini-app >/dev/null
BIN="$REPO/target/release/books-mini-app"
mkdir -p "$OUT"
"$BIN" manifest >"$OUT/manifest.json"
APPS=()
for id in $("$BIN" manifest | python3 -c 'import json,sys; print(" ".join(a["id"] for a in json.load(sys.stdin)["apps"] if "native" not in a))'); do
  "$BIN" app-source "$id" >"$OUT/app-$id.tsx"
  APPS+=("--app" "$id=$OUT/app-$id.tsx")
done
KEY_ID="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["publisher"]["key_id"])' "$OUT/manifest.json")"
VERSION="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["version"])' "$OUT/manifest.json")"
cat <<STEPS
Prepared app.buwiz.books $VERSION in $OUT (publisher.key_id $KEY_ID).

1. Bot owner (M9-04): add to approved_plugins in $BOT/server/src/v1/trust.json
     { "id": "app.buwiz.books", "publisher_key_id": "$KEY_ID" }

2. You (M9-05): sign with the Goldcoders key from the login Keychain
     $SIGNER $OUT/manifest.json \\
       --keychain $KEY_ID ${APPS[*]} \\
       --out $OUT/manifest.signed.json

3. You (M9-05): install in your Buwiz Bot (Connect apps), as a local plugin
     command  $BIN
     args     mcp
     env      BOOKS_V1_MANIFEST=$OUT/manifest.signed.json
   Books keeps its store in ~/Library/Application Support/app.buwiz.books
   (BOOKS_DATA_DIR to move it). Sign-in goes to https://api.buwiz.com
   (BOOKS_CLOUD_URL=https://… for another cloud; empty turns it off).
STEPS
