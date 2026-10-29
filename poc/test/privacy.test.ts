import { describe, expect, test } from "bun:test";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { CASES } from "../src/cases.ts";
import { sha256Hex } from "../src/canonical.ts";
import { normalizeFixture } from "../src/load.ts";
import { makeCanaries, scanValue } from "../src/privacy.ts";
import { SYNTHETIC_DIR, VECTORS_DIR } from "../src/paths.ts";
import { data, fixture, proposal, rawFixture, clone } from "./support.ts";


const MEMO_NEEDLES = ["MEMO-"];

describe("the scanner catches what it claims to catch (self-test with run-time canaries)", () => {
  const canary = makeCanaries();

  test("a planted nine-digit TIN-shaped run is found, dashed or not", () => {
    expect(scanValue({ note: `value ${canary.tin}` }).map((f) => f.kind)).toContain("tin_like");
    const dashed = `${canary.tin.slice(0, 3)}-${canary.tin.slice(3, 6)}-${canary.tin.slice(6)}`;
    expect(scanValue({ note: dashed }).map((f) => f.kind)).toContain("tin_like");
  });

  test("the centavos exemption is narrow: integer text under a *_centavos key only, tickets still caught", () => {
    expect(scanValue({ base_centavos: "300000000" })).toEqual([]);
    expect(scanValue({ base: canary.tin }).map((f) => f.kind)).toContain("tin_like");
    const dashed = `${canary.tin.slice(0, 3)}-${canary.tin.slice(3, 6)}-${canary.tin.slice(6)}`;
    expect(scanValue({ base_centavos: dashed }).map((f) => f.kind)).toContain("tin_like");
    expect(scanValue({ base_centavos: canary.ticket }).map((f) => f.kind)).toContain("ticket");
  });

  test("a planted ticket value is found", () => {
    expect(scanValue({ title: canary.ticket }).map((f) => f.kind)).toContain("ticket");
  });

  test("a planted memo marker is found when it is a needle", () => {
    expect(scanValue({ memo: canary.memo }, [canary.memo]).map((f) => f.kind)).toContain("needle");
  });

  test("a finding inside a key is found too", () => {
    expect(scanValue({ [`k${canary.tin}`]: "x" }).length).toBeGreaterThan(0);
  });

  test("peso amounts with up to seven integer digits are not flagged (3000000.00 is fine)", () => {
    expect(scanValue({ fill: { txtATCAmt1: "3000000.00", x: "290000.00", y: "-1000.00" } })).toEqual([]);
  });

  test("a whole 40-hex commit id or 64-hex digest is exempt, even when its digits form a run", () => {
    expect(scanValue({ schema_pin: "63267ca3d4d526a8ec490394470f026de82ac385" })).toEqual([]);
    expect(scanValue({ fixture_sha256: sha256Hex("anything") })).toEqual([]);
  });

  test("the digest exemption is narrow: only a string that is exactly one digest is exempt; the same digest with other text is scanned", () => {
    const digest = sha256Hex("narrow exemption");
    expect(scanValue({ input_sha256: digest })).toEqual([]);
    expect(scanValue({ note: `${digest} 123456789` }).map((f) => f.kind)).toContain("tin_like");
  });

  test("KNOWN LIMIT: a peso amount of 100000000.00 or more has a nine-digit run and is flagged (needs field context)", () => {
    expect(scanValue({ fill: { txtATCAmt1: "100000000.00" } }).map((f) => f.kind)).toContain("tin_like");
  });
});

describe("no proposal, vector or fixture carries a TIN-like run, a ticket or a memo", () => {
  test("every proposal in every case is clean", () => {
    for (const spec of CASES) {
      const memos = memoMarkers(spec.fixture);
      const p = proposal(fixture(spec.fixture), spec.form, spec.period, spec.form_context);
      expect(scanValue(p, [...MEMO_NEEDLES, ...memos]), spec.id).toEqual([]);
    }
  });

  test("every vector file on disk is clean", () => {
    for (const name of readdirSync(VECTORS_DIR).filter((n) => n.endsWith(".json"))) {
      const vector = JSON.parse(readFileSync(join(VECTORS_DIR, name), "utf8"));
      expect(scanValue(vector, MEMO_NEEDLES), name).toEqual([]);
    }
  });

  test("every synthetic fixture on disk is clean of TIN-like runs and tickets (memos are allowed in the fixture source)", () => {
    for (const name of readdirSync(SYNTHETIC_DIR).filter((n) => n.endsWith(".json"))) {
      const raw = JSON.parse(readFileSync(join(SYNTHETIC_DIR, name), "utf8"));
      const findings = scanValue(raw).filter((f) => f.kind !== "needle");
      expect(findings, name).toEqual([]);
    }
  });

  test("fixture profiles carry an opaque UUID and a masked display only", () => {
    for (const name of readdirSync(SYNTHETIC_DIR).filter((n) => n.endsWith(".json"))) {
      const raw = rawFixture(name.replace(/\.json$/, ""));
      for (const profile of raw.profiles) {
        expect(profile.display).toBe("***-***-***-00000");
        expect(profile.tax_profile_id).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/);
      }
    }
  });
});

describe("memos never reach a proposal, even when they carry a canary", () => {
  test("a canary memo planted in the source fixture is absent from every output", () => {
    const canary = makeCanaries();
    const raw = clone(rawFixture("a-sole-prop-2026"));
    raw.entries[0].memo = `${canary.memo} ${canary.ticket}`;
    const input = normalizeFixture(raw, data.vocab);
    for (const form of ["2551Q", "1601C", "1701MS"] as const) {
      const period = form === "2551Q" ? "2026-Q3" : form === "1601C" ? "2026-09" : "2026";
      const p = proposal(input, form, period);
      expect(scanValue(p, [canary.memo])).toEqual([]);
      expect(JSON.stringify(p)).not.toContain(canary.ticket);
    }
  });
});

function memoMarkers(fixtureName: string): string[] {
  const raw = rawFixture(fixtureName);
  const markers: string[] = [];
  for (const entry of raw.entries as Array<{ memo?: string }>) {
    if (entry.memo) markers.push(entry.memo);
  }
  return markers;
}
