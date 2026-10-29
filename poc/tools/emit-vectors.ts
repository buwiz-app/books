#!/usr/bin/env bun
/**
 * Writes the language-neutral vectors to fixtures/vectors/. Run from poc/: bun run vectors
 * Each case vector holds the normalized, profile-scoped input and the expected proposal. A Rust implementation
 * can load the input, compute the proposal, and compare canonical JSON with the expected block.
 */
import { loadData, loadFixture } from "../src/load.ts";
import { writeVectors } from "../src/vectors.ts";

const data = loadData();
const written = writeVectors(data, (name) => loadFixture(name, data.vocab));
for (const item of written) console.log(`wrote ${item.file} (${item.bytes} chars)`);
