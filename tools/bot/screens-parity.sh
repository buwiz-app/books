#!/usr/bin/env bash
set -euo pipefail
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BOT_DIR="${BUWIZ_BOT_DIR:-}"
[ -n "$BOT_DIR" ] || { echo "screens-parity: set BUWIZ_BOT_DIR" >&2; exit 2; }
node "$REPO/tests/host/screens-parity.mjs" "$@"
