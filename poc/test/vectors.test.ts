import { describe, expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { CASES, HAND_MONEY, HAND_THIRTEENTH } from "../src/cases.ts";
import { canonical, prettyJson, sha256Of } from "../src/canonical.ts";
import { SYNTHETIC_DIR, VECTORS_DIR } from "../src/paths.ts";
import { MONEY_VECTOR_FILE, VECTOR_FORMAT, buildCaseVector, buildMoneyVectors, runMoneyCase } from "../src/vectors.ts";
import { buildFixtures } from "../tools/gen-fixtures.ts";
import { data, fixture } from "./support.ts";

/**
 * Vectors are the language-neutral contract for a future Rust implementation: input and Forms context in, expected
 * proposal (measures and fill arguments) out. These tests recompute every vector and compare it with the file on disk.
 * If a test here fails after a change to the engine, regenerate with `bun run vectors` and review the diff.
 */

const readJson = (path: string): any => JSON.parse(readFileSync(path, "utf8"));

describe("case vectors match a fresh computation", () => {
  for (const spec of CASES) {
    test(`${spec.id}`, () => {
      const onDisk = readJson(join(VECTORS_DIR, `${spec.id}.json`));
      const { vector } = buildCaseVector(data, spec, fixture(spec.fixture));
      expect(onDisk.vector_format).toBe(VECTOR_FORMAT);
      expect(canonical(onDisk)).toBe(canonical(vector));
    });
  }

  test("each vector's input is memo-free and its hash matches the input it carries", () => {
    for (const spec of CASES) {
      const vector = readJson(join(VECTORS_DIR, `${spec.id}.json`));
      expect(canonical(vector.input)).not.toContain("memo");
      expect(canonical(vector.input)).not.toContain("MEMO-");
      expect(vector.fixture_sha256).toBe(sha256Of(vector.input));
      expect(vector.expected.fixture_sha256).toBe(vector.fixture_sha256);
      expect(vector.expected.form_context).toEqual(vector.form_context);
    }
  });

  test("expected_forms_computed is documentation only: flagged as not asserted, and never a Books output key", () => {
    for (const spec of CASES.filter((c) => c.expected_forms_computed)) {
      const vector = readJson(join(VECTORS_DIR, `${spec.id}.json`));
      expect(vector.expected_forms_computed.asserted_against_books).toBe(false);
      for (const key of Object.keys(vector.expected_forms_computed.values)) {
        expect(key in vector.expected.fill_args, `${spec.id}: ${key}`).toBe(false);
        expect(key in vector.expected.reference_values, `${spec.id}: ${key}`).toBe(false);
      }
    }
  });

  test("the vector directory holds exactly the case vectors and the money vector", () => {
    const names = readdirSync(VECTORS_DIR).filter((n) => n.endsWith(".json")).sort();
    expect(names).toEqual([...CASES.map((c) => `${c.id}.json`), MONEY_VECTOR_FILE].sort());
  });
});

describe("money-and-mapping vectors", () => {
  test("the file on disk equals a fresh build", () => {
    expect(canonical(readJson(join(VECTORS_DIR, MONEY_VECTOR_FILE)))).toBe(canonical(buildMoneyVectors(data)));
  });

  test("the engine agrees with every hand-written expectation", () => {
    const money = buildMoneyVectors(data) as { cases: Array<Record<string, any>> };
    for (const c of money.cases) {
      const result = runMoneyCase(data, c);
      if (c.op === "money_text_round_trip") expect(result, c.id).toEqual({ value_centavos: c.value_centavos, text: c.text });
      else expect(result, c.id).toBe(c.nontaxable_centavos);
    }
    expect(money.cases.length).toBe(HAND_MONEY.length + HAND_THIRTEENTH.length);
  });
});

describe("synthetic fixtures on disk equal the generator's output", () => {
  for (const [name, body] of buildFixtures()) {
    test(`${name}`, () => {
      const onDisk = readJson(join(SYNTHETIC_DIR, `${name}.json`));
      expect(canonical(onDisk)).toBe(canonical(body));
      expect(prettyJson(onDisk)).toBe(prettyJson(body));
    });
  }
});
