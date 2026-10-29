import { createHash } from "node:crypto";
import { existsSync, readFileSync } from "node:fs";
import { basename, join } from "node:path";
import type { LoadedData } from "./load.ts";
import type { FormCode } from "./types.ts";
import { FORM_CODES } from "./types.ts";

/**
 * Checks the crosswalk and the rules against the real agent-forms JSON (the pinned schema copy).
 * Problems are returned as text; an empty list means the check passed.
 *
 * Rules applied:
 *  1. The file's SHA-256 equals the pinned digest in the crosswalk.
 *  2. Every crosswalk key exists as a binding, a box or an unmapped key.
 *  3. A sendable official key has a binding with no lock and is not unmapped.
 *  4. A sendable typed alias (for example schedule_1.0.atc) has an official box and a CONFIRMED alias status.
 *  5. A locked crosswalk class matches the schema lock; a computed key is not a binding.
 *  6. Each rule's target keys (by rule_ids) exist, and a fill rule's targets are sendable.
 */

export interface SchemaReport {
  form: FormCode;
  file: string;
  present: boolean;
  problems: string[];
  fieldsChecked: number;
  rulesChecked: number;
}

export function checkSchemas(data: LoadedData, dir: string): SchemaReport[] {
  return FORM_CODES.map((form) => checkOne(data, form, dir));
}

function checkOne(data: LoadedData, form: FormCode, dir: string): SchemaReport {
  const cw = data.crosswalk[form];
  const file = join(dir, basename(cw.schema_file));
  if (!existsSync(file)) {
    return { form, file, present: false, problems: [], fieldsChecked: 0, rulesChecked: 0 };
  }
  const bytes = readFileSync(file);
  const problems: string[] = [];
  const digest = createHash("sha256").update(bytes).digest("hex");
  if (digest !== cw.schema_sha256) {
    problems.push(`schema file sha256 ${digest} differs from the pinned ${cw.schema_sha256}`);
  }
  const schema = JSON.parse(bytes.toString("utf8"));
  const bindings: Record<string, { lock?: string | null }> = schema.bindings ?? {};
  const boxKeys = new Set<string>(schema.boxes.map((box: { key: string }) => box.key));
  const unmapped: Record<string, unknown> = schema.unmapped ?? {};
  const has = (record: object, key: string): boolean => Object.prototype.hasOwnProperty.call(record, key);

  for (const field of cw.fields) {
    if (field.key === null) {
      // Runtime-indexed row family (key_pattern). Never addressable as a key, so it must not be sendable.
      if (field.fill_key !== null) problems.push(`${field.key_pattern}: row family marked sendable`);
      continue;
    }
    const key = field.key;
    if (!has(bindings, key) && !boxKeys.has(key) && !has(unmapped, key)) {
      problems.push(`${key}: absent from the schema`);
    }
    if (field.fill_key !== null) {
      if (field.fill_key === key) {
        if (!has(bindings, key)) problems.push(`${key}: sendable but has no binding in the schema`);
        else if (bindings[key].lock) problems.push(`${key}: sendable but locked in the schema (${bindings[key].lock})`);
        if (has(unmapped, key)) problems.push(`${key}: sendable but unmapped in the schema`);
      } else {
        if (!boxKeys.has(key)) problems.push(`${key}: typed alias ${field.fill_key} needs an official box`);
        if (!String(field.typed_alias_status ?? "").startsWith("CONFIRMED")) {
          problems.push(`${key}: typed alias ${field.fill_key} is not verified`);
        }
      }
    }
    if (field.fill_class.startsWith("locked_")) {
      const wanted = field.fill_class.slice("locked_".length);
      const lock = has(bindings, key) ? bindings[key].lock ?? "none" : "none";
      if (lock !== wanted) problems.push(`${key}: crosswalk ${field.fill_class} but schema lock is ${lock}`);
    }
    if (field.fill_class === "computed" && has(bindings, key)) {
      problems.push(`${key}: crosswalk says computed but the schema binds it`);
    }
  }

  let rulesChecked = 0;
  for (const rule of data.rules[form].rules) {
    const targets = cw.fields.filter((f) => f.rule_ids.includes(rule.id));
    if (targets.length === 0) continue;
    rulesChecked += 1;
    // A fill rule must have at least one sendable target. Its spouse (B-side) twins and runtime row families
    // are allowed to be non-sendable, since they are refused or not addressable.
    if (rule.books_policy?.startsWith("fill_")) {
      const sendable = targets.filter((t) => t.fill_key !== null && t.books_policy.startsWith("fill_"));
      if (sendable.length === 0) problems.push(`${rule.id}: fill rule with no sendable target`);
    }
  }
  return { form, file, present: true, problems, fieldsChecked: cw.fields.length, rulesChecked };
}
