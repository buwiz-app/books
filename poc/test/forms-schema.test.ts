import { describe, expect, test } from "bun:test";
import { existsSync } from "node:fs";
import { CASES } from "../src/cases.ts";
import { FORMS_SCHEMA_DIR } from "../src/paths.ts";
import { checkSchemas } from "../src/schema-check.ts";
import { data, fixture, proposal } from "./support.ts";
import { readFileSync } from "node:fs";
import { join } from "node:path";

/**
 * Checks against the real agent-forms JSON (the buwiz-oss copy of the 63267ca pin). The whole group is skipped,
 * with a message, when the directory is absent. Set BUWIZ_FORMS_SCHEMAS to point elsewhere.
 */
const present = existsSync(FORMS_SCHEMA_DIR);
if (!present) {
  console.warn(`SKIPPED forms-schema tests: ${FORMS_SCHEMA_DIR} does not exist. Set BUWIZ_FORMS_SCHEMAS to run them.`);
}

const reports = present ? checkSchemas(data, FORMS_SCHEMA_DIR) : [];

describe.skipIf(!present)(`real agent-forms schemas at ${FORMS_SCHEMA_DIR}`, () => {
  for (const form of ["2551Q", "1601C", "1701MS"] as const) {
    test(`${form}: the schema file matches the pinned SHA-256 and every crosswalk key, mapping and Forms-context path checks out`, () => {
      const report = reports.find((r) => r.form === form)!;
      expect(report.present).toBe(true);
      expect(report.problems).toEqual([]);
      expect(report.fieldsChecked).toBe(data.crosswalk[form].fields.length);
      expect(report.rulesChecked).toBe(data.rules[form].mappings.length);
    });
  }

  test("the crosswalk and rules pin the same schema digest for each form", () => {
    for (const form of ["2551Q", "1601C", "1701MS"] as const) {
      const bytes = readFileSync(join(FORMS_SCHEMA_DIR, data.crosswalk[form].schema_file.split("/").pop()!));
      const digest = new Bun.CryptoHasher("sha256").update(bytes).digest("hex");
      expect(digest).toBe(data.crosswalk[form].schema_sha256);
      expect(data.rules[form].schema_sha256).toBe(data.crosswalk[form].schema_sha256);
    }
  });

  test("every sendable key in every case is a bound, unlocked official key or a verified typed alias", () => {
    const schemas = new Map<string, any>();
    for (const form of ["2551Q", "1601C", "1701MS"] as const) {
      schemas.set(form, JSON.parse(readFileSync(join(FORMS_SCHEMA_DIR, data.crosswalk[form].schema_file.split("/").pop()!), "utf8")));
    }
    for (const spec of CASES) {
      const p = proposal(fixture(spec.fixture), spec.form, spec.period, spec.form_context);
      const schema = schemas.get(spec.form);
      const cw = data.crosswalk[spec.form];
      for (const key of Object.keys(p.fill_args)) {
        const field = cw.byFillKey.get(key)!;
        if (field.fill_key === field.key) {
          const binding = schema.bindings[key];
          expect(binding, `${spec.id}: ${key} must be bound`).toBeDefined();
          expect(binding.lock ?? null, `${spec.id}: ${key} must not be locked`).toBeNull();
        } else {
          expect(field.typed_alias_status, `${spec.id}: ${key} alias`).toMatch(/^CONFIRMED/);
          expect(schema.boxes.some((b: { key: string }) => b.key === field.key), `${spec.id}: ${key} official box`).toBe(true);
        }
      }
    }
  });
});
