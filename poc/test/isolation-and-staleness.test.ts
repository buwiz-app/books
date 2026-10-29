import { describe, expect, test } from "bun:test";
import { canonical, prettyJson } from "../src/canonical.ts";
import { freshness, assertFillable, computeInputsHash, buildProposal, ProposalError } from "../src/proposal.ts";
import { data, fixture, PROFILE_A, PROFILE_B, proposal, clone, rawFixture } from "./support.ts";
import { normalizeFixture } from "../src/load.ts";
import type { NormInput } from "../src/types.ts";

/** Profiles A and B merged into one input, the way a multi-profile store would hand them over. */
function merged(): NormInput {
  const a = fixture("a-sole-prop-2026");
  const e = fixture("e-second-profile-2026");
  return {
    input_format: a.input_format,
    as_of: a.as_of,
    profiles: [...a.profiles, ...e.profiles],
    acknowledged_entry_ids: [...a.acknowledged_entry_ids, ...e.acknowledged_entry_ids],
    entries: [...a.entries, ...e.entries],
  };
}

describe("profile isolation: proposals for A never include lines of B", () => {
  const both = merged();
  const aAlone = fixture("a-sole-prop-2026");
  const bAlone = fixture("e-second-profile-2026");

  for (const [form, period] of [
    ["2551Q", "2026-Q3"],
    ["1601C", "2026-09"],
    ["1701MS", "2026"],
  ] as const) {
    test(`${form} ${period} for A is byte-identical whether or not B is in the same input`, () => {
      const alone = proposal(aAlone, form, period);
      const withB = buildProposal(data, both, form, period, PROFILE_A);
      expect(withB.output_sha256).toBe(alone.output_sha256);
      expect(prettyJson(withB)).toBe(prettyJson(alone));
    });
  }

  test("B's 2551Q is computed from B's lines alone: base 999000.00, PT 29970.00", () => {
    const withA = buildProposal(data, both, "2551Q", "2026-Q3", PROFILE_B);
    const alone = proposal(bAlone, "2551Q", "2026-Q3");
    expect(withA.output_sha256).toBe(alone.output_sha256);
    expect(withA.fill_args["txtATCAmt1"]).toBe("999000.00");
    expect(withA.expected_readback["frm2551Qv2018:txt14"]).toBe("29970.00");
  });

  test("A's lineage names only A's lines, and A's output carries no B identifier", () => {
    const p = buildProposal(data, both, "2551Q", "2026-Q3", PROFILE_A);
    const bLines = bAlone.entries.flatMap((e) => e.lines.map((l) => l.line_id));
    const lineageLines = (p.lineage["txtATCAmt1"] as { lines: Array<{ line_id: string }> }).lines.map((l) => l.line_id);
    for (const id of lineageLines) expect(bLines).not.toContain(id);
    const text = JSON.stringify(p);
    expect(text).not.toContain(PROFILE_B);
    expect(text).not.toContain("E-001");
    expect(text).not.toContain("E-002");
  });

  test("a proposal for a profile that is not in the input is refused", () => {
    expect(() => buildProposal(data, aAlone, "2551Q", "2026-Q3", PROFILE_B)).toThrow(/no_profile/);
  });

  test("a reversal whose original belongs to another profile is an orphan, not a silent cross-profile reversal", () => {
    const mixed = clone(merged());
    const bEntry = mixed.entries.find((e) => e.entry_id === "E-001")!;
    const aReversal = {
      entry_id: "X-1",
      tax_profile_id: PROFILE_A,
      entry_date: bEntry.entry_date,
      reverses_entry_id: "E-001",
      lines: bEntry.lines.map((l) => ({ ...l, line_id: `X-1.${l.line_id.slice(-1)}`, debit: l.credit, credit: l.debit })),
    };
    mixed.entries.push(aReversal as NormInput["entries"][number]);
    expect(() => buildProposal(data, mixed, "2551Q", "2026-Q3", PROFILE_A)).toThrow(/orphan_reversal/);
  });
});

describe("same inputs give byte-identical proposals (idempotency)", () => {
  test("building the same proposal three times gives the same canonical JSON", () => {
    const input = fixture("a-sole-prop-2026");
    const first = canonical(proposal(input, "2551Q", "2026-Q3"));
    const second = canonical(proposal(input, "2551Q", "2026-Q3"));
    const third = canonical(proposal(input, "2551Q", "2026-Q3"));
    expect(second).toBe(first);
    expect(third).toBe(first);
  });

  test("the pretty file form is stable too", () => {
    const input = fixture("f-slice-a-2026");
    expect(prettyJson(proposal(input, "2551Q", "2026-Q3"))).toBe(prettyJson(proposal(input, "2551Q", "2026-Q3")));
  });

  test("a memo change does not change the input or the proposal (memos are dropped before any hash)", () => {
    const raw = rawFixture("f-slice-a-2026");
    const edited = clone(raw);
    edited.entries[0].memo = "MEMO-SYNTH-EDITED a different free-text memo";
    const before = normalizeFixture(raw, data.vocab);
    const after = normalizeFixture(edited, data.vocab);
    expect(canonical(after)).toBe(canonical(before));
    expect(proposal(after, "2551Q", "2026-Q3").output_sha256).toBe(proposal(before, "2551Q", "2026-Q3").output_sha256);
  });
});

describe("stale detection: a changed input marks the stored proposal stale and refuses fill", () => {
  const base = fixture("a-sole-prop-2026");
  const stored = proposal(base, "2551Q", "2026-Q3");

  test("the stored proposal is fresh against the same inputs and fill is allowed", () => {
    expect(freshness(stored, data, base)).toBe("fresh");
    expect(assertFillable(stored, data, base)).toEqual(stored.fill_args);
  });

  test("a changed Q3 line makes the proposal stale and fill is refused", () => {
    const changed = clone(base);
    changed.entries.find((e) => e.entry_id === "A-010")!.lines.find((l) => l.line_id === "A-010.2")!.credit = "100500.00";
    changed.entries.find((e) => e.entry_id === "A-010")!.lines.find((l) => l.line_id === "A-010.1")!.debit = "100500.00";
    expect(freshness(stored, data, changed)).toBe("stale");
    expect(() => assertFillable(stored, data, changed)).toThrow(ProposalError);
    try {
      assertFillable(stored, data, changed);
    } catch (error) {
      expect((error as ProposalError).code).toBe("stale");
    }
  });

  test("a Q4 change does not stale the Q3 proposal, but the whole-input fixture hash changes", () => {
    const changed = clone(base);
    changed.entries.find((e) => e.entry_id === "A-017")!.lines.find((l) => l.line_id === "A-017.2")!.credit = "100500.00";
    changed.entries.find((e) => e.entry_id === "A-017")!.lines.find((l) => l.line_id === "A-017.1")!.debit = "100500.00";
    expect(freshness(stored, data, changed)).toBe("fresh");
    const after = proposal(changed, "2551Q", "2026-Q3");
    expect(after.fixture_sha256).not.toBe(stored.fixture_sha256);
    expect(after.output_sha256).not.toBe(stored.output_sha256);
    expect(after.fill_args).toEqual(stored.fill_args);
  });

  test("withdrawing an acknowledgement makes the Q3 proposal stale and blocked", () => {
    const changed = clone(base);
    changed.acknowledged_entry_ids = [];
    expect(freshness(stored, data, changed)).toBe("stale");
    const blockedNow = proposal(changed, "2551Q", "2026-Q3");
    expect(blockedNow.blocking.map((b) => b.code)).toContain("GATE.RETURNS_ACK");
  });

  test("a tax fact change (regime to 8%) makes the proposal stale", () => {
    const changed = clone(base);
    changed.profiles[0].facts["2026"].regime = "eight_percent";
    expect(freshness(stored, data, changed)).toBe("stale");
  });

  test("a rule-set version bump makes the proposal stale", () => {
    const bumped = structuredClone(data) as typeof data;
    bumped.rules["2551Q"].version = 2;
    expect(computeInputsHash(bumped, base, "2551Q", "2026-Q3")).not.toBe(stored.inputs_hash);
    expect(computeInputsHash(data, base, "2551Q", "2026-Q3")).toBe(stored.inputs_hash);
  });

  test("a rate-row change makes the proposal stale and changes the tax (PT at 2.5% for the test)", () => {
    const repriced = structuredClone(data) as typeof data;
    const row = (repriced.rates.pt_rate_rows as Array<{ code: string; rate_bps: number; effective_to: string | null; default_on: boolean }>).find(
      (r) => r.code === "PT010" && r.default_on && r.effective_to === null,
    )!;
    row.rate_bps = 250;
    const after = buildProposal(repriced, base, "2551Q", "2026-Q3");
    expect(after.expected_readback["frm2551Qv2018:txt14"]).toBe("7250.00");
    expect(freshness(stored, repriced, base)).toBe("stale");
  });

  test("a blocked proposal is refused for fill even when fresh", () => {
    const blocked = proposal(fixture("c-reversal-credit-note-2026"), "2551Q", "2026-Q3");
    expect(blocked.status).toBe("blocked");
    expect(() => assertFillable(blocked, data, fixture("c-reversal-credit-note-2026"))).toThrow(/not_ready/);
  });
});
