#!/usr/bin/env bash
# tools/release/prepare.sh — everything the release needs except the
# signature (M9-04, M9-05 are the owners'). It builds the release binary
# and writes, to dist/release/ (or $1):
#   manifest.json     unsigned, publisher.key_id gc-2026-01
#   app-<id>.tsx      each app's source, exactly as the binary serves it
# then prints the three owner steps: the trust entry (Rox owner), the
# signature with gc-2026-01 from the login Keychain (the owner), and the
# install in the owner's Rox. This script never reads a key or the Keychain.
set -euo pipefail
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT="${1:-$REPO/dist/release}"
ROX="${ROX_DIR:-/Volumes/goldcoders/Projects/rox}"
cd "$REPO"
cargo build --release -p books-mini-app >/dev/null
BIN="$REPO/target/release/books-mini-app"
mkdir -p "$OUT"
"$BIN" manifest >"$OUT/manifest.json"
APPS=()
for id in $("$BIN" manifest | python3 -c 'import json,sys; print(" ".join(a["id"] for a in json.load(sys.stdin)["apps"]))'); do
  "$BIN" app-source "$id" >"$OUT/app-$id.tsx"
  APPS+=("--app" "$id=$OUT/app-$id.tsx")
done
KEY_ID="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["publisher"]["key_id"])' "$OUT/manifest.json")"
VERSION="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["version"])' "$OUT/manifest.json")"
cat <<STEPS
Prepared dev.goldcoders.books $VERSION in $OUT (publisher.key_id $KEY_ID).

1. Rox owner (M9-04): add to approved_plugins in $ROX/server/src/v1/trust.json
     { "id": "dev.goldcoders.books", "publisher_key_id": "$KEY_ID" }

2. You (M9-05): sign with the Goldcoders key from the login Keychain
     node $ROX/server/src/v1/sign-manifest.mjs $OUT/manifest.json \\
       --keychain $KEY_ID ${APPS[*]} \\
       --out $OUT/manifest.signed.json

3. You (M9-05): install in your Rox (Connect apps), as a local plugin
     command  $BIN
     args     mcp
     env      BOOKS_V1_MANIFEST=$OUT/manifest.signed.json
   Books keeps its store in ~/Library/Application Support/dev.goldcoders.books
   (BOOKS_DATA_DIR to move it). Optional sign-in: BOOKS_CLOUD_URL=https://…
STEPS
