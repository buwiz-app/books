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
 *  6. Each mapping's target exists, cites the mapping, and is sendable when the mapping fills it.
 *  7. Each Forms-context path the bridge reads is set by a schema binding, with every supported value.
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
  for (const mapping of data.rules[form].mappings) {
    rulesChecked += 1;
    const target = cw.byFillKey.get(mapping.target_key) ?? cw.byKey.get(mapping.target_key);
    if (!target) {
      problems.push(`${mapping.id}: target ${mapping.target_key} is not a crosswalk key`);
      continue;
    }
    if (!target.rule_ids.includes(mapping.id)) problems.push(`${mapping.id}: target ${mapping.target_key} does not cite the mapping`);
    if (target.fill_class !== "input") problems.push(`${mapping.id}: target ${mapping.target_key} is ${target.fill_class}`);
    if (mapping.mode === "fill" && (target.fill_key === null || !target.books_policy.startsWith("fill_"))) {
      problems.push(`${mapping.id}: fill mapping with a target that is not sendable`);
    }
  }

  const setsByPath = new Map<string, Set<string>>();
  for (const binding of Object.values<any>(bindings)) {
    for (const option of binding?.options ?? []) {
      for (const [path, value] of Object.entries<string>(option.set ?? {})) {
        if (!setsByPath.has(path)) setsByPath.set(path, new Set());
        setsByPath.get(path)!.add(value);
      }
    }
  }
  for (const input of data.rules[form].form_context_inputs) {
    const values = setsByPath.get(input.path);
    if (!values) {
      problems.push(`form context ${input.path}: no schema binding sets it`);
      continue;
    }
    for (const value of input.supported_values) {
      if (!values.has(value)) problems.push(`form context ${input.path}: no schema option sets ${value}`);
    }
  }
  return { form, file, present: true, problems, fieldsChecked: cw.fields.length, rulesChecked };
}
