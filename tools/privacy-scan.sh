#!/usr/bin/env bash
# tools/privacy-scan.sh — the privacy gate (AGENTS invariant 3). All synthetic
# data, two Rust test targets:
#   1. tests/privacy_repo.rs: the scanner's self-test with run-time canaries,
#      and its scan of data/, fixtures/, tests/fixtures/, the app and screen
#      sources, every vector, every synthetic fixture and every proposal;
#   2. tests/privacy_outputs.rs: what the assistant, titles and the store
#      file carry.
set -euo pipefail
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
out="$(cargo test --quiet --manifest-path "$REPO/Cargo.toml" -p books-mini-app \
  --test privacy_repo --test privacy_outputs 2>&1)" || status=$?
grep -E 'test result|FAILED|panicked' <<<"$out" || true
exit "${status:-0}"
