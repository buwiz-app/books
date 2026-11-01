// The repo privacy scan (docs/06 §8, AGENTS invariants 3 and 14): every JSON file under data/, fixtures/,
// docs/plan/seed/ and screen fixtures through the PoC scanner, and TSX/Rust screen sources as text. A finding
// is a TIN-like run, a ticket value (tkt_ plus 26 Crockford characters) or a memo marker outside a fixture's
// memo field. Run from the repo root: bun run poc/tools/scan-repo.ts
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { scanValue, TIN_LIKE } from "../src/privacy.ts";

const ROOT = join(import.meta.dir, "..", "..");
const TICKET_VALUE = /tkt_[0-9A-HJKMNP-TV-Z]{26}/;

function files(dir: string, ext: string): string[] {
  const out: string[] = [];
  const walk = (d: string): void => {
    for (const name of readdirSync(d)) {
      const path = join(d, name);
      if (statSync(path).isDirectory()) walk(path);
      else if (path.endsWith(ext)) out.push(path);
    }
  };
  walk(join(ROOT, dir));
  return out.sort();
}

let findings = 0;
let scanned = 0;
const report = (path: string, what: string): void => {
  findings += 1;
  console.log(`FINDING  ${relative(ROOT, path)}  ${what}`);
};

for (const dir of ["data", "fixtures", "docs/plan/seed", "tests/fixtures/screens"]) {
  for (const path of files(dir, ".json")) {
    scanned += 1;
    const value = JSON.parse(readFileSync(path, "utf8"));
    // synthetic fixtures carry MEMO-SYNTH- markers on purpose, in their memo fields only
    const found = scanValue(value).filter((f) => f.kind !== "ticket" || TICKET_VALUE.test(JSON.stringify(value)));
    for (const f of found) report(path, `${f.kind} at ${f.path}`);
  }
}
for (const [dir, ext] of [["apps", ".tsx"], ["crates/books-screens/src", ".rs"]]) {
  for (const path of files(dir, ext)) {
    scanned += 1;
    readFileSync(path, "utf8")
      .split("\n")
      .forEach((line, i) => {
        if (TIN_LIKE.test(line.replace(/0123456789/g, ""))) report(path, `tin_like on line ${i + 1}`);
        if (TICKET_VALUE.test(line)) report(path, `ticket on line ${i + 1}`);
        if (/MEMO-/.test(line)) report(path, `memo marker on line ${i + 1}`);
      });
  }
}
console.log(`privacy scan: ${scanned} files, ${findings} findings`);
process.exit(findings === 0 ? 0 : 1);
