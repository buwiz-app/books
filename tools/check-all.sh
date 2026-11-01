#!/usr/bin/env bash
# tools/check-all.sh — every gate a commit must pass, in order (docs/06,
# build plan 5.1): rustfmt, clippy with warnings denied, the Rust tests
# (with the 17 PoC vectors byte for byte), the PoC's bun tests, the Buwiz
# Bot's conformance runner, the Bot host test (which also types into the
# Books window and changes its pickers), native screen parity, the privacy scan, and no detached
# signal setters in apps/ (`onChange={x.set}` loses `this` when kit.tsx calls
# p.onChange(v): "not a live signal"; write `onChange={(v) => x.set(v)}`).
#
# It prints one line per gate and a summary, keeps each gate's output in a
# temp folder, and exits non-zero when any gate fails.
#
# Env: BUWIZ_BOT_DIR    a Buwiz Bot checkout at protocol 2.0 for the conformance
#                       runner and the host test (each script's own default
#                       otherwise); when set, cargo test also compares the Bot's
#                       own JCS vector files
#      BOOKS_HOST_PORT  the host test's broker port (8810 or above; 8787 is the
#                       owner's, 8797-8803 the rox session's)
#      SKIP_BOT=1       skip the Bot and native screen gates (no Bot checkout at hand); the
#                       summary says so, and the run cannot pass
set -uo pipefail
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$(mktemp -d "${TMPDIR:-/tmp}/books-gates.XXXXXX")"
cd "$REPO"

failed=0
skipped=0
gate() {
  local name="$1" log="$OUT/$2.log"
  shift 2
  if "$@" >"$log" 2>&1; then
    printf 'PASS  %-28s %s\n' "$name" "$(summary "$log")"
  else
    printf 'FAIL  %-28s see %s\n' "$name" "$log"
    failed=$((failed + 1))
  fi
}
summary() {
  # the last line a gate prints that carries a count
  grep -E 'test result|pass|conforms|findings' "$1" | tail -1 | sed 's/^ *//' | cut -c1-90
}
tests_total() {
  cargo test --quiet 2>&1 | tee "$OUT/cargo-test-full.log" | grep -E '^test result' | awk '{p+=$4; f+=$6} END {printf "test result: %d passed; %d failed\n", p, f; exit (f > 0)}'
}

no_detached_setters() {
  # `\w` is a literal backslash and w inside an ERE bracket, so the word class is spelled out
  if grep -rnE '=\{[A-Za-z_$][A-Za-z0-9_$.]*\.set\}' apps/; then return 1; fi
  echo "0 findings: no detached setters in apps/"
}

gate "no detached setters" setters no_detached_setters
gate "cargo fmt --check" fmt cargo fmt --check
gate "cargo clippy -D warnings" clippy cargo clippy --all-targets -- -D warnings
gate "cargo test (vectors 17/17)" cargo-test tests_total
gate "PoC bun test" poc bash -c 'cd poc && bun test'
if [ "${SKIP_BOT:-0}" = "1" ]; then
  printf 'SKIP  %-28s SKIP_BOT=1\n' "Bot conformance"
  printf 'SKIP  %-28s SKIP_BOT=1\n' "Bot host test"
  printf 'SKIP  %-28s SKIP_BOT=1\n' "native screen parity"
  skipped=3
else
  gate "Bot conformance" conformance bash tools/bot/conformance.sh
  gate "Bot host test" host bash tools/bot/host-test.sh
  gate "native screen parity" screens-parity bash tools/bot/screens-parity.sh
fi
gate "privacy scan" privacy bash tools/privacy-scan.sh

echo "logs: $OUT"
if [ "$failed" -gt 0 ]; then
  echo "$failed gate(s) failed"
  exit 1
fi
if [ "$skipped" -gt 0 ]; then
  echo "all run gates passed; $skipped skipped, so this run does not count as green"
  exit 2
fi
echo "all gates passed"
