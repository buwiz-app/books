import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));

export const POC_DIR = resolve(HERE, "..");
export const REPO_DIR = resolve(POC_DIR, "..");
export const DATA_DIR = resolve(REPO_DIR, "data");
export const FIXTURES_DIR = resolve(REPO_DIR, "fixtures");
export const SYNTHETIC_DIR = resolve(FIXTURES_DIR, "synthetic");
export const VECTORS_DIR = resolve(FIXTURES_DIR, "vectors");

/** Real agent-forms schemas (buwiz-oss copy of the 63267ca pin). Override with BUWIZ_FORMS_SCHEMAS. */
export const FORMS_SCHEMA_DIR: string =
  process.env.BUWIZ_FORMS_SCHEMAS ?? "/Volumes/goldcoders/buwiz-oss/crates/bir-core/data/agent-forms";
