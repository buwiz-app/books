#!/usr/bin/env bash
# tools/privacy-scan.sh — the privacy gate (docs/06 §8; AGENTS invariants 3,
# 13 and 14). Three parts, all synthetic data:
#   1. the PoC scanner's self-test (run-time canaries) and its scan of every
#      proposal, vector and fixture (poc/test/privacy.test.ts);
#   2. the repo scan: data/, fixtures/, docs/plan/seed/ and the app sources
#      (poc/tools/scan-repo.ts);
#   3. the Rust output scan: what the assistant, titles and the store file
#      carry (tests/privacy_outputs.rs).
set -euo pipefail
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
(cd "$REPO/poc" && bun test test/privacy.test.ts 2>&1 | tail -3)
(cd "$REPO" && bun run poc/tools/scan-repo.ts)
cargo test --quiet --manifest-path "$REPO/Cargo.toml" -p books-mini-app --test privacy_outputs 2>&1 | grep -E 'test result|FAILED|panicked'
