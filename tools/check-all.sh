#!/usr/bin/env bash
# tools/check-all.sh — every gate a commit must pass, in order (docs/06,
# build plan 5.1): rustfmt, clippy with warnings denied, the Rust tests
# (with the 17 PoC vectors byte for byte), the PoC's bun tests, Rox
# conformance, the Rox host test, and the privacy scan.
#
# It prints one line per gate and a summary, keeps each gate's output in a
# temp folder, and exits non-zero when any gate fails.
#
# Env: ROX_DIR          a Rox checkout or worktree for the conformance runner
#                       and the host test (each script's own default otherwise)
#      BOOKS_HOST_PORT  the host test's broker port (8810 or above; 8787 is the
#                       owner's, 8797-8803 the rox session's)
#      SKIP_ROX=1       skip the two Rox gates (no Rox checkout at hand); the
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

gate "cargo fmt --check" fmt cargo fmt --check
gate "cargo clippy -D warnings" clippy cargo clippy --all-targets -- -D warnings
gate "cargo test (vectors 17/17)" cargo-test tests_total
gate "PoC bun test" poc bash -c 'cd poc && bun test'
if [ "${SKIP_ROX:-0}" = "1" ]; then
  printf 'SKIP  %-28s SKIP_ROX=1\n' "Rox conformance"
  printf 'SKIP  %-28s SKIP_ROX=1\n' "Rox host test"
  skipped=2
else
  gate "Rox conformance" conformance bash tools/rox/conformance.sh
  gate "Rox host test" host bash tools/rox/host-test.sh
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
