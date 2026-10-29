import { randomBytes, randomInt } from "node:crypto";

/**
 * Privacy scanner for everything a proposal, vector or fixture carries.
 *
 * Findings:
 *   tin_like  a run of 9 digits in 3-3-3 groups, dashed or not (the regex from AGENTS.md / docs/06 DS-G)
 *   ticket    the prefix tkt_ (a BIR or Books ticket value)
 *   needle    any caller-supplied marker, such as a memo marker
 *
 * Exempt: a whole string that is a 40-hex git commit or a 64-hex SHA-256 digest. Those are identifiers, not
 * TINs, and their digit runs would otherwise match. Also exempt from the TIN check only: a plain signed integer
 * string under a key ending in `_centavos` (₱3,000,000.00 is "300000000"). Tickets and needles are still checked
 * there. Any other string is scanned in full, including keys.
 * AGENTS.md invariant 14: match by field context as well as digit count where that is possible.
 */

export const TIN_LIKE = /\d{3}-?\d{3}-?\d{3}/;
export const TICKET = /tkt_/;
const DIGEST = /^(?:[0-9a-f]{40}|[0-9a-f]{64})$/;
const CENTAVOS_KEY = /_centavos$/;
const INTEGER_TEXT = /^-?\d+$/;

export interface Finding {
  path: string;
  kind: "tin_like" | "ticket" | "needle";
}

export function scanValue(value: unknown, needles: string[] = []): Finding[] {
  const found: Finding[] = [];
  const checkText = (text: string, path: string, isMoney = false): void => {
    if (!isMoney && TIN_LIKE.test(text)) found.push({ path, kind: "tin_like" });
    if (TICKET.test(text)) found.push({ path, kind: "ticket" });
    for (const needle of needles) {
      if (needle.length > 0 && text.includes(needle)) found.push({ path, kind: "needle" });
    }
  };
  const walk = (node: unknown, path: string, key = ""): void => {
    if (typeof node === "string") {
      if (!DIGEST.test(node)) checkText(node, path, CENTAVOS_KEY.test(key) && INTEGER_TEXT.test(node));
      return;
    }
    if (Array.isArray(node)) {
      node.forEach((item, index) => walk(item, `${path}[${index}]`));
      return;
    }
    if (node !== null && typeof node === "object") {
      for (const [childKey, child] of Object.entries(node)) {
        checkText(childKey, `${path}.<key:${childKey}>`);
        walk(child, `${path}.${childKey}`, childKey);
      }
    }
  };
  walk(value, "$");
  return found;
}

const CROCKFORD = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/**
 * Canary values for the scanner's self-test (docs/06 DS-G). Generated at run time and never written to the
 * repo. A scanner that cannot catch a planted canary proves nothing.
 */
export function makeCanaries(): { tin: string; ticket: string; memo: string } {
  const tin = String(randomInt(100_000_000, 999_999_999));
  let ticketBody = "";
  for (let i = 0; i < 26; i += 1) ticketBody += CROCKFORD[randomInt(0, CROCKFORD.length)];
  return {
    tin,
    ticket: `tkt_${ticketBody}`,
    memo: `MEMO-CANARY-${randomBytes(6).toString("hex")}`,
  };
}
