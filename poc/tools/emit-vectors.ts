#!/usr/bin/env bun
/**
 * Writes the language-neutral vectors to fixtures/vectors/. Run from poc/: bun run vectors
 * Each case vector holds the normalized, profile-scoped input, the Forms context and the expected proposal. A Rust
 * implementation can load the input, compute the proposal, and compare canonical JSON with the expected block.
 * Vector files that no case writes any more are removed, so the directory always matches the cases.
 */
import { readdirSync, rmSync } from "node:fs";
import { basename, join } from "node:path";
import { loadData, loadFixture } from "../src/load.ts";
import { VECTORS_DIR } from "../src/paths.ts";
import { writeVectors } from "../src/vectors.ts";

const data = loadData();
const written = writeVectors(data, (name) => loadFixture(name, data.vocab));
for (const item of written) console.log(`wrote ${item.file} (${item.bytes} chars)`);
const keep = new Set(written.map((w) => basename(w.file)));
for (const name of readdirSync(VECTORS_DIR).filter((n) => n.endsWith(".json") && !keep.has(n))) {
  rmSync(join(VECTORS_DIR, name));
  console.log(`removed ${join(VECTORS_DIR, name)} (no case writes it)`);
}
