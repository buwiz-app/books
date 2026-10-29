import { describe, expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { CASES, HAND_GRADUATED, HAND_PT_VECTORS, HAND_THIRTEENTH } from "../src/cases.ts";
import { canonical, prettyJson, sha256Of } from "../src/canonical.ts";
import { SYNTHETIC_DIR, VECTORS_DIR } from "../src/paths.ts";
import { buildCaseVector, buildMoneyVectors, runMoneyCase, VECTOR_FORMAT } from "../src/vectors.ts";
import { buildFixtures } from "../tools/gen-fixtures.ts";
import { data, fixture } from "./support.ts";

/**
 * Vectors are the language-neutral contract for a future Rust implementation: input in, expected proposal out.
 * These tests recompute every vector and compare it with the file on disk. If a test here fails after a change to
 * the engine, regenerate with `bun run vectors` and review the diff. Do not edit a vector by hand.
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
    }
  });

  test("the vector directory holds exactly the case vectors and the money vector", () => {
    const names = readdirSync(VECTORS_DIR).filter((n) => n.endsWith(".json")).sort();
    const expected = [...CASES.map((c) => `${c.id}.json`), "money-and-rules.json"].sort();
    expect(names).toEqual(expected);
  });
});

describe("money-and-rules vectors", () => {
  test("the file on disk equals a fresh build", () => {
    const onDisk = readJson(join(VECTORS_DIR, "money-and-rules.json"));
    expect(canonical(onDisk)).toBe(canonical(buildMoneyVectors(data)));
  });

  test("the engine agrees with every hand-written expectation", () => {
    const money = buildMoneyVectors(data) as { cases: Array<Record<string, any>> };
    for (const c of money.cases) {
      const result = runMoneyCase(data, c);
      if (c.op === "percent_of" || c.op === "graduated_tax_2023") {
        expect(result, c.id).toBe(String(c.expected_centavos));
      } else if (c.op === "thirteenth_month_split") {
        expect(result, c.id).toEqual({ exempt_centavos: String(c.exempt_centavos), taxable_centavos: String(c.taxable_centavos) });
      } else if (c.op === "vat_threshold_breached") {
        expect(result, c.id).toBe(c.expected);
      } else if (c.op === "pt_rate_for") {
        expect(result, c.id).toBe(c.expected_bps);
      }
    }
    expect(HAND_PT_VECTORS.length).toBeGreaterThan(10);
    expect(HAND_GRADUATED.length).toBeGreaterThan(5);
    expect(HAND_THIRTEENTH.length).toBeGreaterThan(1);
  });
});

describe("synthetic fixtures on disk equal the generator's output", () => {
  const generated = buildFixtures();
  for (const [name, body] of generated) {
    test(`${name}`, () => {
      const onDisk = readJson(join(SYNTHETIC_DIR, `${name}.json`));
      expect(canonical(onDisk)).toBe(canonical(body));
      expect(prettyJson(onDisk)).toBe(prettyJson(body));
    });
  }
});
