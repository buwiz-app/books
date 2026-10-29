# ADR 0003: Books links to a tax profile by its opaque cloud `id`, never by TIN

Status: Accepted (peer contract from the buwiz-cloud session, `[decisions.md]`, 2026-10-29).
Date: 2026-10-29. Writer: W3.

## Context

- A tax profile is a TIN: a 9-digit root and a 5-digit branch. The registration key is the raw TIN `[decisions.md, cloud]`.
- The cloud `id` is a stable opaque UUID, never reused `[decisions.md, cloud]`.
- Several copies of TINs already exist outside BIR:
  - `tin_claims.tin_root` is plaintext in buwiz-cloud `migrations/0001_init.sql:79` [V];
  - `device_filings.tin` is plaintext in buwiz-server `migrations/postgres/0014_device_filings.sql:15` `[identity §2.4]`;
  - the TypeScript Books repo stores the org TIN in plaintext at `src/db/schema/tax-reference.ts:168` `[identity §2.4]`.
- At 9f5ee16 the cloud response carried `tin`, `tin_masked`, `registered_name` and `activated_at` in every profile `src/claims.rs:429-432` [V]. buwiz-server PR #4 (head `42190a5`, open, CI running on 2026-10-29) gives a `books`-scope token `{id, tin_masked, registered_name, activated_at, permissions:["profile.view"]}`, with no `tin`.
- No system records another system's ID today `[identity §3]`.

## Decision

- Books stores the cloud `id` (`business.cloud_tax_profile_id`) and nothing else from the tax profile, except a masked display for the UI cache.
- Books never stores a TIN, and never joins on TIN equality.
- Books reads `id`, `tin_masked`, `registered_name` and `permissions` only. It refuses any response with a `tin` field or other TIN-shaped fields.
- Books never relies on `filing.submit`.
- Approval state comes from the list. An id that leaves the list makes the link inactive, and fills are refused. The list filters `status = 'approved'` `[src/claims.rs:418]` [V].

## Alternatives considered

| Option | Why not |
|---|---|
| Store the TIN in Books | Violates DD-3 and the peer contract. Adds a plaintext copy. |
| Store a hash of the TIN in Books and join on it | Still a join on a TIN-derived key. The server's HMAC uses a pepper that the desktop cannot compute `[identity §3]`. |
| Link by name or by business address | Not unique. Changes with the registration. |

## Consequences

- Books depends on the cloud for the link. Offline use still works. A link is not verified offline, and the UI says so.
- B1 must close before P5: confirm buwiz-server PR #4 is merged.
- If an approval is withdrawn, the id leaves the list. Books must show the link as inactive and refuse fills `[claims.rs:418]` [V].
- The link needs one `/me` call only if Books needs the user's id. The user id is optional and hashed (`[01 section 5.3]`).

## Evidence

- `[decisions.md, cloud]`
- `[identity §2.4, §3]`
- `migrations/0001_init.sql:79` [V]; `src/claims.rs:418, 429-433` [V] (9f5ee16); `README.md:74` [V]
- `src/oauth.rs:21` [V] (client `buwiz-books`)

## Revisit when

- The cloud contract changes.
- A TIN-free exclusivity key exists on the cloud side.
