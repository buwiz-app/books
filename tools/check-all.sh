#!/usr/bin/env bash
# tools/check-all.sh — every gate a commit must pass, in order: no detached
# signal setters in apps/ (`onChange={x.set}` loses `this` when kit.tsx calls
# p.onChange(v); write `onChange={(v) => x.set(v)}`), rustfmt, clippy with
# warnings denied, the Rust tests (the vectors byte for byte), the SDK doc links
# against the pinned revision, the SDK's conformance runner, the Bot host test
# and the privacy scan. One line per gate, each gate's output kept in a temp
# folder; exits non-zero when any gate fails.
#
# Env: BUWIZ_SDK_DIR    a buwiz-sdk checkout at the pinned revision, with
#                       `bun install` run (required): the conformance runner,
#                       and cargo test compares the SDK's JCS vectors
#      BUWIZ_BROKER_BIN  a buwiz-broker binary (an executable file, built from
#                       buwiz-bot main): the Bot host test starts it, and the
#                       conformance run signs with it; required unless
#                       SKIP_BOT=1
#      BOOKS_HOST_PORT  the host test's broker port (8811 or above; 8787 is the
#                       owner's broker)
#      SKIP_BOT=1       skip the Bot host test (no broker binary at hand); the
#                       summary says so, and the run cannot pass
set -uo pipefail
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$(mktemp -d "${TMPDIR:-/tmp}/books-gates.XXXXXX")"
cd "$REPO"

# every prerequisite up front, not after minutes of lint and tests
if [ -z "${BUWIZ_SDK_DIR:-}" ] || [ ! -d "$BUWIZ_SDK_DIR" ]; then
  echo "check-all: set BUWIZ_SDK_DIR to a buwiz-sdk checkout (git clone https://github.com/buwiz-app/buwiz-sdk; cd buwiz-sdk; bun install)" >&2
  exit 2
fi
if [ "${SKIP_BOT:-0}" != "1" ] && { [ -z "${BUWIZ_BROKER_BIN:-}" ] || [ ! -f "$BUWIZ_BROKER_BIN" ] || [ ! -x "$BUWIZ_BROKER_BIN" ]; }; then
  echo "check-all: set BUWIZ_BROKER_BIN to a buwiz-broker binary (an executable file; the host test starts it), or SKIP_BOT=1" >&2
  exit 2
fi

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

sdk_links_follow_the_pin() {
  local pin stale
  pin="$(tools/sdk-pin.sh)" || return 1
  stale="$(grep -rhoE 'buwiz-sdk/blob/[^/)]+|checkout [0-9a-f]{40}' README.md AGENTS.md docs | grep -vE "blob/$pin\$|checkout $pin\$" || true)"
  # any other full commit written after buwiz-sdk (a link path, a #fragment, a git URL) must be the pin too
  stale="$stale$(grep -rhoE 'buwiz-sdk[^ )]*[0-9a-f]{40}' README.md AGENTS.md docs | grep -oE '[0-9a-f]{40}' | grep -vx "$pin" || true)"
  [ -z "$stale" ] || { printf 'SDK links name a revision other than the pinned %s:\n%s\n' "$pin" "$stale"; return 1; }
  echo "0 findings: SDK links name $pin"
}

gate "SDK pins agree" sdk-pins tools/sdk-pin.sh
gate "no detached setters" setters no_detached_setters
gate "cargo fmt --check" fmt cargo fmt --check
gate "cargo clippy -D warnings" clippy cargo clippy --all-targets -- -D warnings
gate "cargo test" cargo-test tests_total
gate "SDK links follow the pin" sdk-links sdk_links_follow_the_pin
# needs BUWIZ_SDK_DIR (a buwiz-sdk checkout)
gate "SDK conformance" conformance bash tools/conformance.sh
# needs BUWIZ_BROKER_BIN (a buwiz-broker binary) and BUWIZ_SDK_DIR
if [ "${SKIP_BOT:-0}" = "1" ]; then
  printf 'SKIP  %-28s SKIP_BOT=1 (needs BUWIZ_BROKER_BIN)\n' "Bot host test"
  skipped=1
else
  gate "Bot host test" host bash tools/bot/host-test.sh
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
