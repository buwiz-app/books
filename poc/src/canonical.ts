import { createHash } from "node:crypto";

/**
 * Deterministic JSON: object keys sorted, no whitespace, safe integers only.
 * Hashes and byte-identical comparisons are taken over this form.
 */
export function canonical(value: unknown): string {
  if (value === null) return "null";
  if (typeof value === "string") return JSON.stringify(value);
  if (typeof value === "boolean") return value ? "true" : "false";
  if (typeof value === "number") {
    if (!Number.isSafeInteger(value)) throw new TypeError(`canonical JSON takes safe integers only, got ${value}`);
    return String(value);
  }
  if (typeof value === "bigint") throw new TypeError("canonical JSON does not take bigint; convert to decimal text first");
  if (Array.isArray(value)) return `[${value.map((item) => canonical(item)).join(",")}]`;
  if (typeof value === "object") {
    const record = value as Record<string, unknown>;
    const keys = Object.keys(record)
      .filter((key) => record[key] !== undefined)
      .sort();
    return `{${keys.map((key) => `${JSON.stringify(key)}:${canonical(record[key])}`).join(",")}}`;
  }
  throw new TypeError(`unsupported value in canonical JSON: ${typeof value}`);
}

export function sha256Hex(text: string): string {
  return createHash("sha256").update(text, "utf8").digest("hex");
}

export function sha256Of(value: unknown): string {
  return sha256Hex(canonical(value));
}

function sortKeysDeep(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(sortKeysDeep);
  if (value !== null && typeof value === "object") {
    const record = value as Record<string, unknown>;
    const out: Record<string, unknown> = {};
    for (const key of Object.keys(record).sort()) {
      if (record[key] !== undefined) out[key] = sortKeysDeep(record[key]);
    }
    return out;
  }
  return value;
}

/** Human-readable file form of canonical JSON (2-space indent, sorted keys, trailing newline). */
export function prettyJson(value: unknown): string {
  return `${JSON.stringify(sortKeysDeep(value), null, 2)}\n`;
}
