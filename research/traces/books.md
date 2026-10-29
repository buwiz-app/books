**Buwiz Books: ledger, COA, and PH tax track (read-only)**

**Repo and HEAD:** `/Volumes/goldcoders/OSS/buwiz-books`, git HEAD `1662ca37404ccc6c270ad48888eda3505ec45165` (1662ca3, "Add Cloud Agent development environment (#2)"). This matches the handover's observed head. The working tree is dirty only outside `src/`, `server/`, `drizzle/`, and `tests/` (`M skills-lock.json`; untracked `.claude/skills/impeccable-rust/`, `.cursor/skills/`, `.handover-to-cursor.md`). None of those were read.

**Method:** read-only file reads and grep. One read-only Bun probe at `/private/tmp/claude-501/-Volumes-goldcoders-OSS-buwiz-books/d723bb20-1674-49c1-8fb6-568f056d2c7e/scratchpad/verify/verify-ph-tax.ts` imports the pure tax modules by absolute path, using synthetic data. It writes nothing to the repo. No database was used and no test suites were run.

**Labels:** CONFIRMED (read in code, or executed by the probe and tagged [PROBE]), PARTIAL, MISSING (searched, method given), UNKNOWN. INFERENCE marks derived claims.

---

## 1. Ranked findings

1. **[CONFIRMED][PROBE A-C] 1601-C does not net employee mandatory contributions.** `compensationFromPayrollLine` (`/Volumes/goldcoders/OSS/buwiz-books/src/lib/tax/form-1601c.ts:52-109`) has no input for `sssEmployeeShare`, `philHealthEmployeeShare`, `pagIbigEmployeeShare`, or `unionDues`. Its only caller is `issue-1601c.ts:52`. Using the handover's worked example (basic 40,000, de minimis 2,000, employee SSS 2,000), 1601-C taxable compensation comes out at 40,000. `segregate()` (`src/lib/tax/compensation.ts:143-149`, used by `src/lib/tax/engine.ts:75`) gives 38,000. The only unit test for this function (`tests/unit/lib/tax/form-1601c.test.ts:173-185`) covers MWE and exempt pay only. The wiring test (`tests/unit/tax-reference-wiring.test.ts:593-594`) checks for the function name only.
2. **[MISSING] No 1701 family in code.** `TaxFormCode` (`src/lib/tax/filing-period.ts:30-38`) is 1601C, 1604C, 2316, 0619E, 1601EQ, 1604E, 2550Q, 2551Q. `ComputedReturnForm` (`src/db/schema/tax-stage-remainder.ts:21`) is 2550Q, 2551Q, 1601C, 1601EQ, 0619E, QAP, SLSP. Searching "1701" across `src`, `server`, `tests`, `drizzle`, and `docs` returns only design docs (`docs/tax/DECISIONS.md:212,381`; `docs/tax/IMPLEMENTATION-PLAN.md:217`; `docs/tax/2026-08-16-bir-withholding-coa-mapping-and-ebirforms-plan.md:279,308,313,489,618,744`).
3. **[CONFIRMED] The ledger has no tax attributes.** No ATC, tax code, VAT treatment, or document reference on `journal_lines` (`src/db/schema/journals.ts:178-215`), `accounts` (`src/db/schema/accounts.ts:22-66`), `bill_line_items` (`src/db/schema/bills.ts:110-135`), or `invoice_line_items` (`src/db/schema/invoices.ts:97-118`). The only tax field is the invoice header `taxAmount` (`invoices.ts:66`).
4. **[CONFIRMED] 2551Q inputs are caller-supplied, and the 8% election is not read from storage.** `savePercentageTaxReturn` takes `grossReceipts` and `electedEightPercent` from the request (`src/routes/api/-tax-returns.ts:83-88,90-130`). The election table `org_tax_year_elections` is written by `upsertTaxYearElection` (`src/routes/api/-tax-settings.ts:179-237`) and is not read on the 2551Q path. The UI defaults `electedEightPercent` to `true` (`src/routes/tax.compute.tsx:37`) and hard-codes `quarter: 1, year: 2026` on save (`tax.compute.tsx:108`). The 8% conflict is only a string in `blockingIssues` (`src/lib/tax/percentage-tax.ts:354-360`). Figures are still computed [PROBE F].
5. **[CONFIRMED] Filing gates do not enforce the 1601-C reconciliation.** `buildFilingWorkspace` accepts `reconciliationIssues` (`src/lib/tax/filing-workspace.ts:94,176-179`). `assemblePayrollFilingWorkspace` never passes it (`src/lib/tax/assemble-payroll-filing-workspace.ts:218-235`). The 1601-C reconciliation blockers are stored only as a count (`issue-1601c.ts:64,76`).
6. **[CONFIRMED by code path, not executed] A filed payroll period can be re-snapshotted.** `takeFilingSnapshot` (`src/routes/api/-filing.ts:87-174`) checks only the pre-snapshot workspace stages. It never checks `filedAt` or `filingReference`, and it overwrites `snapshot_checksum` and `snapshot_taken_at` (`-filing.ts:161-168`). The filed-state refusal exists only in `markPeriodFiled` (`-filing.ts:198-200`). No test covers re-snapshotting after filing. `tests/integration/payroll-filing-state.test.ts:49-55,68` covers refusing filed-without-snapshot and half-snapshot only.
7. **[CONFIRMED] Snapshot evidence is a hash only.** Only `snapshot_checksum` and `snapshot_taken_at` are persisted (`drizzle/0046_payroll_filing_state.sql:15-16,30-31`). There is no snapshot payload table in `src/db/schema` or in any `CREATE TABLE` in `drizzle/*.sql`. The export header says "as-filed snapshots included" (`src/lib/export-ph.ts:6`), but the export serialises `payroll_runs` and `tax_computed_returns` rows only (`export-ph.ts:147,177`).
8. **[CONFIRMED][PROBE D] Semi-monthly payroll collides on the 1601-C key.** `issueForm1601C` builds from one run's lines (`issue-1601c.ts:25,52`). `buildForm1601C` keys the period to the full calendar month (`src/lib/tax/form-1601c.ts:286-287`). The upsert key is (org, form, periodStart, periodEnd) (`issue-1601c.ts:67-73`; `src/db/schema/tax-stage-remainder.ts:142-147`). A second half-month run overwrites the first run's payload. INFERENCE: whether this matters depends on live use of semi-monthly payroll, which is UNKNOWN.
9. **[CONFIRMED] Period lock is one org-wide date, can be moved backward, and tax filing does not consult it.** The date is `organization.closedThrough` (`src/db/schema/auth.ts:92`). `setClosedThrough` checks readiness only when advancing (`src/routes/api/transactions/-_period-close.ts:46-110`). `issueForm1601C`, `takeFilingSnapshot`, and `markPeriodFiled` never call `isDateInLockedPeriod`. Payroll, CWT, and EWT posting do (`src/lib/tax/payroll-journal.ts:140-150`; `post-cwt-receivable.ts:56-66`; `post-ewt-remittance.ts:112`).
10. **[CONFIRMED by code, not exercised] Read-only roles can obtain 2316 PDFs.** `issuePayrollFilingArtifacts` uses `withSessionOrgContext`, which has no permission check (`src/lib/server-context.ts:117-124`). It returns base64 2316 PDFs and the alphalist (`src/routes/api/-payroll-runs.ts:448-466`). `clientApprover` and `reportViewer` hold only `journal: view` (`src/lib/permissions.ts:131-176`).
11. **[CONFIRMED absence] PH VAT and percentage-tax control accounts are never posted.** `ph_output_vat` 21700, `ph_input_vat` 13800, `ph_creditable_vat_withheld` 12610, `ph_vat_payable_net` 21750, `ph_percentage_tax_payable` 21800, `ph_percentage_tax_expense` 94100, `ph_input_vat_exempt_expense` 68400, and `ph_employee_advances` 12700 are defined in `src/lib/tax/ph-chart.ts`. Grep finds no posting path that references them. Invoice tax posts through the generic `sales_tax_payable` 21500 (`src/lib/coa/presets/base-mappings.ts:59`; `src/lib/invoice-journal.ts:147-167`).
12. **[CONFIRMED] No token path for a mini app.** Auth is Better Auth cookie sessions (`src/lib/auth-middleware.ts:32-40`). The plugin list is emailOTP and organization only (`src/lib/auth.ts:144-234`). There is no API-key, bearer, or OAuth-provider plugin. The only bearer check is `INBOX_WORKER_SECRET` on internal worker endpoints (`server/routes/api/internal/worker.post.ts:18-25`).
13. **[MISSING] No Buwiz Forms, Rox, or MCP integration in this repo.** Method in §6.

---

## 2. Ledger and tax data model

| Concept | Location | What is stored | Gaps |
|---|---|---|---|
| Accounts | `src/db/schema/accounts.ts:22-66` | `organizationId` as text (no FK); `accountNumber` varchar(10), unique per org (`:63-65`); `accountType` one of 8 (`src/db/schema/account-constants.ts:9-18`); `subtype` (`account-constants.ts:37-114`); `parentId`; `status`/`isActive`/`isSystem`; per-account party-type arrays (`:47-48`) | **[CONFIRMED]** No tax, ATC, currency, or normal-balance attribute. Root ranges in `account-constants.ts:165-174`. |
| Journal headers | `src/db/schema/journals.ts:62-163` | `transactionDate` date (`:75`); `transactionType` pay_in/pay_out/journal/transfer (`:35-40`); `source` enum (`:46-57`); `partyId` (`:85`); `totalAmount` decimal(20,8) (`:88`); `status` draft/posted/voided (`:43,94`); `sourceDocumentId`/`Type` with no FK (`:97-98`); `postedAt`/`voidedAt` (`:104-105`); amendment lineage `reversesHeaderId`/`replacesHeaderId`/`amendmentReason` (`:112-118`); `duplicateOfHeaderId` (`:123-126`); `idempotencyKey` unique per org (`:74,146-148`) | **[CONFIRMED]** Legacy `matchedFromIds`/`isMatched` (`:131-135`). |
| Journal lines | `journals.ts:178-215` | `accountId` FK (`:187`); `debit`/`credit` decimal(20,8) (`:192-193`); `originalDebit`/`originalCredit`/`originalCurrency`/`exchangeRate` (`:194-198`); `lineDescription` (`:201`); line `partyId` (`:204`); department/location (`:207-208`); `sortOrder` (`:211`) | **[CONFIRMED]** No tax fields. |
| Duplicate and posting predicate | `journals.ts:171-173` (`effectiveJournalPredicate` = `duplicateOfHeaderId IS NULL`) | Used by `src/services/reports.ts:61`, `src/routes/api/-reports.ts:162,291,385`, `export-transactions.ts:256`, reconciliation and review code | **[CONFIRMED]** Not used by `controlAccountMovementForRun` (`assemble-payroll-filing-workspace.ts:291-305`). |
| Reports | `src/services/reports.ts:91,117,140,150` | Balance sheet, P&L, cash flow, and trial balance read journal tables live. Posted status or voided-after-as-of, plus the duplicate predicate (`:50-61`) | **[CONFIRMED]** Projections (`src/db/schema/reporting-projections.ts:30-134`) are used by business-group code only. |
| Balance integrity | `drizzle/0041_journal_balance_constraint.sql:1-12` | Deferrable DB-level balance guarantee for posted journals, per the file header | **[PARTIAL]** Body not read in detail. |
| Period close | `src/db/schema/auth.ts:86-101` (`closedThrough` text ISO date); `src/lib/period-lock-policy.ts:7-9` (`transactionDate <= closedThrough`); `src/lib/period-close.ts:12-25` | One org-wide date | **[CONFIRMED]** No per-tax-period lock. |
| Counterparties | `src/db/schema/parties.ts:20-30,32` (partyType enum: vendor, customer, both, employee, shareholder, lender, bank, government, other); generic `taxId` varchar(50) | **[CONFIRMED]** Party tax identity in `src/db/schema/party-tax.ts:60-155`: `tin` (`:71`); `branchCode` default '00000' (`:80`); name components (`:84-88`); `payeeType` (`:51-58,96`); `statusCode` (`:42-49,97`); employee fields (`:100-113`); `defaultAtc` (`:135`); `swornDeclarationYear` (`:143`); `isVatRegistered` (`:146`). Unique per (org, party) (`:152`). | **[CONFIRMED]** A payee's ATC is a default on the party, not per line. |
| Org tax profile | `src/db/schema/tax-reference.ts:163-199` | `tin`, `branchCode`, `rdoCode`, `registeredName`, `taxpayerClassification` micro/small/medium/large (`:177-179`), eFPS fields (`:180-182`), `fiscalYearEndMonth` (`:184`), `accn` and approved series (`:186-189`), `booksAsOf` (`:191`), `referenceDatasetVersion` (`:193`) | **[MISSING]** Entity or taxpayer type (individual, corporate, partnership) and COR data. Searched "certificate of registration" and "COR" across `src`; no hits. |
| Branches | `tax-reference.ts:208-225` | `org_tax_branches` | **[PARTIAL]** Stored; `src/lib/export-ph.ts` and `-tax-settings.ts` reference it. |
| Global reference | `tax-reference.ts:39-153` (datasets, withholding tables with Annex D/E, de minimis ceilings) | Effective-dated, no `organizationId`, no RLS policies (`drizzle/rls_policies.sql:794-805`) | **[CONFIRMED]** `DATASET_V1` version "2026-08-16", `lastVerifiedAt` null per its note (`src/lib/tax/reference-catalog.ts:71-84`). |
| Elections | `src/db/schema/tax-stage-remainder.ts:23-41` (`org_tax_year_elections`, unique per org and year) | `regime`, `electedViaForm`, `irrevocable`, `hasCompensationIncome` | **[PARTIAL]** Write-side guard at `-tax-settings.ts:179-237`: regime cannot change once set, 8% rejected for `taxpayerKind === "corporation"`. `hasCompensationIncome` is updated on conflict with no guard (`:219-222`), though it drives the ₱250,000 deduction. Read only by `export-ph.ts` and `-tax-settings.ts`, not by the 2551Q path. |
| Registrations | `tax-stage-remainder.ts:43-65` (`org_tax_registrations`, `regimeKind`, `value`, `effectiveFrom`/`effectiveTo`) | Stored | **[CONFIRMED]** Read only by `-tax-parties.ts:100-128` and `export-ph.ts`. Not used by any regime or filing logic. |
| EWT register | `tax-stage-remainder.ts:88-123` (`tax_withholding_payments`: `payeeTin` text NOT NULL, `atc`, `incomePayment`, `taxWithheld`, `journalHeaderId`, `remittedAt`) | Register rows | **[CONFIRMED]** Capture writes the register only (`src/routes/api/-tax-ewt.ts:56-115`). Remittance posts Dr 21600, Cr cash, and stamps `remittedAt` (`src/lib/tax/post-ewt-remittance.ts:150-210`). |
| Computed returns | `tax-stage-remainder.ts:125-149` (`tax_computed_returns`, `formCode` ComputedReturnForm, `payload` jsonb, `blockingIssueCount`, unique (org, form, periodStart, periodEnd)) | One row per form and period | **[CONFIRMED]** Upserts overwrite (`issue-1601c.ts:67-79`; `-tax-returns.ts:111-123`). No version history. |
| Certificates (2307) | `src/db/schema/tax-certificates.ts:31-97` (`certificateType` received/issued at `:19,37-40`; `payorTin` text NOT NULL `:47`; `periodStart`/`periodEnd` **text** `:53-54`; `atc` `:57`; `incomePayment`/`taxWithheld` `:58-59`; `certificateStatus` pending/received/lost/disputed `:29,61-64`; `journalHeaderId` `:66`) | Unique on (org, payorTin, certificateNumber, periodStart, periodEnd) where certificate_number is not null (`:81-89`) | **[CONFIRMED]** Period columns are text, unlike every other period column. |
| Payroll runs | `src/db/schema/payroll.ts:231-297` | `status` draft/imported/computed/acknowledged/locked (`:54,246`); `journalHeaderId` (`:251`); `importedDocumentId` (`:257`); `referenceDatasetVersion` (`:259`); acknowledgement fields (`:261-271`); `snapshotChecksum`/`snapshotTakenAt`/`filingReference`/`filedAt`/`lockedAt` (`:279-283`); unique (org, year, period, periodIndex) (`:289-294`) | **[CONFIRMED]** `lockedAt` is never written (grep). Payroll `filingPeriod.formCode` is "1604C" (`assemble-payroll-filing-workspace.ts:81`). |
| Payroll lines | `payroll.ts:312-441` | Compensation buckets (`:324-364`); employee shares and union dues (`:367-370`); `reportedTaxWithheld`/`computedTaxWithheld`/`varianceAmount` (`:374-378`); **expected** employee and employer shares only (`:394-411`); `contributionCheckStatus` (`:418`); `withholdingPath` (`:430`) | **[CONFIRMED]** No reported employer share column. |
| Employee year state | `payroll.ts:142-220` | YTD accumulators, cumulative-method latch, opening balances | **[PARTIAL]** Not read in depth. |
| Prior employer 2316 | `payroll.ts:89-132` | Prior employer figures and `periodsCovered` | **[PARTIAL]** |
| Documents | `src/db/schema/documents.ts:22-32` (types: statement, bill, invoice, receipt, payslip, contract, tax_form, other); `:53` (`documents`: `storagePath`, `contentHash`, R2 keys); `:181-200` (`document_attachments`, polymorphic `linkableType` varchar, no FK) | Evidence files | **[PARTIAL]** No FK integrity on attachments. |
| Bills and invoices | `bills.ts:40` (header); `bills.ts:110-135` (lines: `amount` decimal(15,2), expense `accountId`); `invoices.ts:36` (header `taxAmount` decimal(15,2) at `:66`); `invoices.ts:97-118` (lines: `quantity` decimal(10,4), `unitPrice`/`amount` decimal(15,2), `revenueAccountId`) | | **[CONFIRMED]** Precision differs from journal lines (decimal(20,8)). No tax or VAT attributes. No ACCN or series columns on invoices (grep `accn\|series` returns nothing). |
| Credit notes | Grep `credit.?note` across all `src` | | **[MISSING]** |
| Sales returns | `base.ts` `refunds_and_discounts` 49000 (`:339-343`) is the only contra-revenue account, mapped from invoice "discounts" (`base-mappings.ts:60`) | | **[MISSING]** No sales-return document or flow. |
| Receipts | Invoice payments (`src/lib/invoice-payments.ts`); `pay_in` type | | **[PARTIAL]** No official receipt entity. |
| Category mappings | `src/db/schema/category-mappings.ts:10-34` (`mapping_type`, `source_key`, target account, unique (org, type, key)) | | **[CONFIRMED]** Mapping types are only bank, bill, invoice (`src/lib/coa/mapping-types.ts:11`). |
| Organization secrets | `auth.ts:104-115` | Third-party keys | **[PARTIAL]** `src/lib/org-secrets.ts` imports the crypto helper, so encryption is applied in the app layer. Not verified in detail. |
| Firms and accountant access | `src/db/schema/inbox.ts:77-141` (`firms`, `firm_members`, `firm_clients`, `firm_member_client_access`) | Tables only | **[MISSING]** No application code uses them. Only `src/lib/migrations/verifiers/0019.ts:30-33` lists them. |

---

## 3. Chart of accounts

### 3a. Base preset keys used by PH and tax code

Source: `src/lib/coa/presets/base.ts`. Line numbers are the key lines.

| Key | Number | Type / subtype | Used by |
|---|---|---|---|
| `sales_revenue` | 41000 | revenue / sales_revenue (`:310-314`) | invoice `default_revenue` (`base-mappings.ts:55`) |
| `refunds_and_discounts` | 49000 | revenue / refunds_discounts (`:339-343`) | invoice `discounts` (`base-mappings.ts:60`) |
| `cost_of_goods_sold` | 51000 | cost_of_revenue / cost_of_goods (`:360-364`) | bill `cost_of_goods` (`base-mappings.ts:40`). No COGS computation. |
| `salaries` | 61100 | payroll_expenses (`:412-416`) | `ph-account-resolver.ts:35-39`, payroll Dr (`payroll-journal.ts:221-225`) |
| `employer_payroll_taxes` | 61600 | payroll_expenses (`:441-445`) | Employer SSS/PhilHealth/Pag-IBIG expense (`payroll-journal.ts:226-230`) |
| `payroll_expenses` family | 61000 with children 61400, 61500, 61700 | `:403-460` | |
| `professional_fees` family | 63000 (63300 tax_and_accounting) | `:523-560` | |
| `facilities` / `rent` / `utilities` | 66000 / 66100 / 66200 | `:623-650` | |
| `general_operations` / `insurance` / `bank_fees` | 68000 / 68200 / 68300 | `:696-730` | |
| `uncategorized_expense` | 69999 | `:737-741` | bill `default_expense` (`base-mappings.ts:22`) |
| `interest_expense` | 91000 | `:791-795` | |
| `taxes` | 94000 | other_expense / taxes (`:801-805`) | bill `taxes` (`base-mappings.ts:41`) |
| `bank_accounts` | 11000 | `:32-36` | |
| `accounts_receivable` | 12000 (gross 12010) | `:51-64` | invoice AR mapping; CWT credit (`post-cwt-receivable.ts:69-73`) |
| `accounts_payable` | 21000 | `:152-156` | bill AP mapping |
| `sales_tax_payable` | 21500 | other_current_liabilities (`:239-243`) | invoice tax credit (`invoice-journal.ts:147-167`) |
| `payroll_liabilities` | 25100 (family parent) | `:230-234` | Used by `ph-account-resolver.ts:38`; not posted to |
| `asset_clearing` | 11500 | `:42-46` | invoice `payment_clearing` (`base-mappings.ts:61`) |

**[CONFIRMED]** `base-mappings.ts` has no payroll or tax mapping types. The PH accounts resolve by number, not by mapping.

### 3b. PH chart (`src/lib/tax/ph-chart.ts`, 21 entries at `:68-262`)

Reserved bands are at `ph-chart.ts:51-60`. The lock test is `tests/unit/lib/coa/ph-chart-lock.test.ts`, which includes `ph_union_dues_payable` (25150). The preset header says "20" (`src/lib/coa/presets/philippines-smb.ts:2`), which is stale.

| Key | Number | Stage | Posting path (status) |
|---|---|---|---|
| `ph_cwt_receivable` | 12600 | 3a | **Dr** in `src/lib/tax/post-cwt-receivable.ts:102-105` (CONFIRMED) |
| `ph_creditable_vat_withheld` | 12610 | 6 | **MISSING** |
| `ph_employee_advances` | 12700 | 5b | **MISSING** |
| `ph_employee_tax_advanced` | 12800 | 5b | Referenced in `src/lib/tax/annualization-posting.ts` |
| `ph_input_vat` / `_unpaid` | 13800 / 13810 | 3b | **MISSING** |
| `ph_ewt_payable` | 21600 | 3b | **Cr** `src/lib/manual-bill-payment.ts:142-147`; **Dr** `post-ewt-remittance.ts:150-152` (CONFIRMED) |
| `ph_output_vat` / `_uncollected` | 21700 / 21710 | 6 | **MISSING** (only a comment, `src/lib/tax/vat.ts:23`) |
| `ph_vat_payable_net` | 21750 | 6 | **MISSING** |
| `ph_percentage_tax_payable` | 21800 | 7 | **MISSING** |
| `ph_wtc_payable` | 25110 | 5b | **Cr** `payroll-journal.ts:232-233`; read by `controlAccountMovementForRun` (`assemble-payroll-filing-workspace.ts:282`) |
| `ph_sss_payable` / `ph_philhealth_payable` / `ph_pagibig_payable` | 25120 / 25130 / 25140 | 5b | **Cr** employee plus employer share, `payroll-journal.ts:237-251` |
| `ph_union_dues_payable` | 25150 | 5b | **Cr** `payroll-journal.ts:258-260` |
| `ph_net_pay_payable` | 25170 | 5b | **Cr** `payroll-journal.ts:263-264` |
| `ph_employee_tax_refund_payable` | 25175 | 5b | Referenced in `annualization-posting.ts` |
| `ph_unrecovered_employee_tax` | 61950 | 5b | Referenced in `annualization-posting.ts` |
| `ph_input_vat_exempt_expense` | 68400 | 6 | **MISSING** |
| `ph_percentage_tax_expense` | 94100 | 7 | **MISSING** |

**[CONFIRMED]** Twelve of 21 PH accounts have no posting code. Grep for these keys across `src` returns nothing outside `ph-chart.ts`, `philippines-smb.ts`, and `ph-account-resolver.ts`.

### 3c. Resolution and completeness

- **[CONFIRMED]** PH accounts resolve by exact frozen number plus `isActive`. Missing accounts fail loudly with `MissingPhAccountError` (`src/lib/tax/ph-account-resolver.ts:91-112`). The resolver comments explain why PH accounts are not in `category_mappings`: `allMappingKeys()` is global, so a new mapping type would fail every shipped preset's completeness check (`ph-account-resolver.ts:4-14`; `philippines-smb.ts:13-16`).
- **[CONFIRMED]** `allMappingKeys()` (`src/lib/coa/mapping-registry.ts:28-32`) is consumed by `validate-preset.ts:172`, `plan-preset.ts:348`, `execute-plan.ts:291`, `src/lib/jobs/handlers/coa-scaffold.ts:277`, and `src/routes/api/-category-mappings.ts:145`.
- **[CONFIRMED]** Tier 1 mappings are type-guarded (`resolve-mapped-account.ts:93-126`). Tier 2 subtype fallback is deterministic (`:128-177`). There is no "first account of roughly the right type" fallback (`:10-20`). Posting paths use `requireMappedAccountId` (`:205-217`).
- **[CONFIRMED]** `ph-chart.ts:205-208` records a deliberate chart addition for union dues.

---

## 4. Tax computation

### 4a. 1601-C

- **[CONFIRMED]** Reconciliation: Δ control account equals Σ detail equals the return total (`form-1601c.ts:231-252`). `controlAccountMovement` is null when unposted, which blocks (`:236-240`).
- **[CONFIRMED]** Control movement is credits minus debits on 25110 for posted journals (`assemble-payroll-filing-workspace.ts:277-311`). When `journalHeaderId` is set, it is scoped to that one header. Otherwise it uses a date range with no duplicate predicate (`:284-305`). The run-scoped path is the normal one.
- **[CONFIRMED]** Tax per line is `reportedTaxWithheld ?? computedTaxWithheld ?? "0"` (`form-1601c.ts:105`), matching the posting summary (`payroll-posting-summary.ts:102`).
- **[CONFIRMED]** Due dates (`form-1601c.ts:178-199`): 10th of the following month; December 15 January; eFPS groups A=15 to E=11, with unknown group taking the earliest date. [PROBE C] September 2026 gives 2026-10-10.
- **[CONFIRMED]** Blocking issues cover: negative taxable base, no posting, reconciliation mismatch, empty return, missing employee identifier, eFPS group not set (`form-1601c.ts:204-274`).
- **[CONFIRMED]** `issueForm1601C` does not check period lock or payroll frequency (`issue-1601c.ts:12-84`). Its lines query has no `organizationId` predicate (`:25`), relying on the run check at `:18-23`. This departs from the explicit-predicate rule stated in `payroll.ts:23-27`.
- **[CONFIRMED]** Exposed as `issuePayroll1601C` (`src/routes/api/-payroll-runs.ts` ~L467-480), guarded by `journal update` plus `assertPhTaxWritable`.
- **[CONFIRMED]** Output formatting: `fromScaled` trims trailing zeros, so amounts appear as "42000" [PROBE A, C]. The 2551Q path uses `toPesoString`, which gives "9000.00" [PROBE E]. Two formats coexist (`src/lib/inbox/money.ts:36-42`; `src/lib/tax/money.ts:37-44`).

### 4b. Payroll contribution path

- **[CONFIRMED]** `segregate()` nets employee contributions correctly: `taxableRegular = clampAtZero(grossRegular - employeeContributions)` (`src/lib/tax/compensation.ts:143-149`). `grossCompensation` adds them back (`:184`). The engine consumes this (`src/lib/tax/engine.ts:74-75`), and the run service passes employee shares in (`src/lib/tax/payroll-run-service.ts:325-329`). The withholding engine is correct.
- **[CONFIRMED]** The 1601-C builder is the outlier (§4a, finding 1). The 2316 path does include the employee shares (`src/lib/tax/issue-payroll-artifacts.ts:280-283`, and `form-2316.ts:56,140`).
- **[CONFIRMED]** Employer shares posted and reported are **expected** values from the engine (`payroll-run-service.ts:601-603`; `payroll-posting-summary.ts:107-109`). There is no reported-employer column, so actual remittance cannot be reconciled to the register.
- **[PARTIAL]** EC (Employees' Compensation) is folded into the employer SSS total (`src/lib/tax/contributions.ts:101-109`; `sss-brackets.ts:14-18`). The posting comment says "employee + employer + EC" (`payroll-journal.ts:14`), but there is no separate EC line.

### 4c. Payroll posting (`src/lib/tax/payroll-journal.ts:117-304`)

- **[CONFIRMED]** Requires status computed or acknowledged (`:136-138`). Row lock (`:125-132`). Not already posted (`:135`). Period lock on `run.periodEnd` (`:140-150`). D-N7 variance gate (`:160-167`). Required accounts resolved together (`:173-182`).
- **[CONFIRMED]** Idempotency: `idempotencyKey` `payroll-run:<runId>` (`:207`) with a unique index (`journals.ts:146-148`), plus the `journalHeaderId` guard (`payroll-journal.ts:135`).
- **[CONFIRMED]** Header `source: 'manual'` (`payroll-journal.ts:198`). Payroll journals are identifiable only by `sourceDocumentType: 'payroll_run'` (`:204-206`).
- **[CONFIRMED]** Entry (`:215-267`): Dr 61100 gross, Dr 61600 employer share; Cr 25110 tax, Cr 25120/25130/25140 employee plus employer, Cr 25150 union, Cr 25170 net.
- **[CONFIRMED]** Net pay is derived per line (`payroll-posting-summary.ts:111-136`) and throws if negative (`:125-134`). It is not compared with any register net, since none is stored. The comment at `payroll-journal.ts:24-31` says a disagreeing register "is reported". The code has no such check. **Stale comment.**
- **[CONFIRMED][PROBE I]** Totals for the synthetic line: gross 42,000; tax 1,000; SSS employee 2,000; SSS employer 2,400; net 39,000; employer expense 2,400.
- **[CONFIRMED]** Run status becomes `locked` and `journalHeaderId` is set (`payroll-journal.ts:282-287`). An activity log entry is written (`:289-301`).

### 4d. 2551Q, 8% option, VAT threshold (`src/lib/tax/percentage-tax.ts`)

- **[CONFIRMED]** Rate by date (`:65-68`): 1% for 2020-07-01 to 2023-06-30, otherwise 3%. [PROBE H]
- **[CONFIRMED]** `assessRegime` (`:100-148`): gross above ₱3,000,000 forces VAT (strictly greater, `:106`). VAT-registered forces VAT (`:123-129`). 8% is eligible only for individuals (`:131-142`). `hasCompensationIncome` is accepted but unused here (`:86`).
- **[CONFIRMED]** 8% computation (`:167-191`): ₱250,000 deduction only without compensation income (`:173`). Base clamped at zero (`:180-181`). Rate 800 bps (`:182`). The caller supplies `hasCompensationIncome` (`tax.compute.tsx:36`, default false).
- **[CONFIRMED]** 2551Q builder (`:330-385`): calendar quarter (`:341-342`); due 25 days after quarter end (`:344-345`). [PROBE E] 2026 Q3 gives due date 2026-10-25. Credits come from input (`:350`). The route does not accept credits (`-tax-returns.ts:83-88`). Excess credits are surfaced (`:382`).
- **[CONFIRMED]** `electedEightPercent` adds a blocking string (`:354-360`). VAT threshold breach adds another (`:361-366`). Figures are still produced [PROBE F: taxDue 9000.00 with the block present].
- **[CONFIRMED]** Gross receipts are user input (`-tax-returns.ts:86`; `tax.compute.tsx:34`, default "2500000"). No ledger aggregation. Sales returns are not an input. Typing 290,000 gives 8,700 [PROBE G].
- **[PARTIAL]** The election is coarse. The doc comments describe effective-year and transition rules (`percentage-tax.ts:19-21,34-37`), but the code has no effective-date or mid-year transition logic. `hasCompensationIncome` for the 8% deduction comes from the 2551Q caller, not from the election row.
- **[CONFIRMED]** `VAT_THRESHOLD` is defined twice: `percentage-tax.ts:50` and `vat.ts:70`.
- **[CONFIRMED]** `savePercentageTaxReturn` upserts over the same period key with no history (`-tax-returns.ts:111-123`).

### 4e. 2550Q (`src/lib/tax/vat.ts`)

- **[CONFIRMED]** `buildVatReturn` takes caller-supplied `outputVat`, `creditableInputVat`, and optional carryovers (`vat.ts:283-296`). The route accepts only a subset (`-tax-returns.ts:32-38`). No ledger VAT aggregation (MISSING). Output VAT has no posting path (§3b).
- **[CONFIRMED]** `VAT_RATE_BPS` 1200 (`vat.ts:67`).

### 4f. EWT, QAP, 0619-E, 1601-EQ (`src/lib/tax/ewt.ts`)

- **[CONFIRMED]** ATC selection by payment type and payee type, with overrides for professional fees: WI011 when no sworn declaration (`ewt.ts:120-123`), WC011 for gross income over 720k (`:124-126`). Duty map at `:70-74`.
- **[CONFIRMED]** Expected rates (`src/lib/tax/certificate-2307.ts:92-110`): WI010 500 bps, WI011 1000, WC010 1000, WC011 1500, WI and WC 100 500, 120 200, 158 100, 160 200. Current legal correctness: **UNKNOWN**.
- **[CONFIRMED]** `remittanceObligationsFor` (`ewt.ts:254`), `buildQap` grouped by TIN and ATC (`:335-420`), `reconcileQuarter` (`:420`).
- **[CONFIRMED]** Bill payment with EWT: Dr AP, Cr cash, Cr 21600 (`src/lib/manual-bill-payment.ts:120-147` via `src/lib/tax/bill-payment-ewt.ts`).

### 4g. Creditable withholding (2307, received)

- **[CONFIRMED]** `postCwtReceivable` (`src/lib/tax/post-cwt-receivable.ts:23-113`): requires a received 2307 (`:40-49`), refuses a second posting via `journalHeaderId` (`:51-53`), requires non-zero withholding, and refuses locked periods (`:56-66`). Dr 12600, Cr mapped AR at `certificate.periodEnd` (a text column).
- **[CONFIRMED]** It does **not** check `certificateStatus` (zero references in the file). The SAWT builder filters on `received` (`src/lib/tax/certificate-2307.ts:293`). **PARTIAL:** a pending, lost, or disputed certificate can post the receivable.
- **[CONFIRMED]** Received certificates created through the UI are stamped `received` (`src/routes/api/-tax-certificates.ts:79`; `-tax-ewt.ts:282`).

### 4h. Filing workflow, snapshot, and versioning

- **[CONFIRMED]** Stage order (`src/lib/tax/filing-workspace.ts:40-59`): opening balances, computation, variance review, posting, reconciliation, preflight, snapshot, submission. `canFile` requires no blockers and an allowed transition (`:256`).
- **[CONFIRMED]** Payroll gates (`assemble-payroll-filing-workspace.ts:241-275`): snapshot requires stages up to preflight. File requires everything except submission. Gap: `reconciliationIssues` is never supplied (§1 finding 5). Re-snapshot after filing is allowed (§1 finding 6).
- **[CONFIRMED]** `canTransition` (`src/lib/tax/filing-period.ts:104-135`) refuses filed or amended periods with no checksum or reference. The `amended` state has no writer (grep). `amendmentSequence` is hard-coded to 0 (`-filing.ts:151`; `assemble-payroll-filing-workspace.ts:87`). **MISSING:** tax amendment workflow.
- **[CONFIRMED]** Snapshot canonicalisation (`src/lib/tax/filing-snapshot.ts`): `sha256/canonical-v1` (`:82`); sorted keys and lines (`:138-176`); trailing-zero money normalised (`:98-122`); duplicate line keys refused (`:150-158`); reference dataset version required (`:139-143`). Floats are accepted and stringified (`:101-108`). Snapshot covers about 27 per-employee figures including employee shares and withheld (`-filing.ts:113-144`).
- **[CONFIRMED]** `verifySnapshot` (`filing-snapshot.ts:204-221`) detects accidental edits only. Its comment says so (`:196-203`). It cannot be checked against stored data, because the payload is not stored (§1 finding 7).
- **[CONFIRMED]** Idempotency: payroll journal (§4c). Tax returns overwrite by natural key (no version). EWT remittance uses `remittedAt` and a window filter (`post-ewt-remittance.ts:50-54`). CWT uses a journal pin (`post-cwt-receivable.ts:51-53`).
- **[PARTIAL]** Staleness: `reference-data-staleness.ts:45-46` uses 45 and 90 day thresholds for reference datasets only. **MISSING:** any detection of ledger or payroll changes after computation.
- **[CONFIRMED]** Reference dataset is pinned per run (`payroll.ts:259`) and per org (`tax-reference.ts:193`).

### 4i. Money and rounding

- **[CONFIRMED]** Tax money is BigInt at scale 8 (`src/lib/tax/money.ts:19-22`, via `src/lib/inbox/money.ts`). Rounding is half-up to centavos, applied once per computation (`:63-97`). `divideHalfUp` and `multiplyByPeriods` are also present (`:117-133`).
- **[CONFIRMED]** `src/lib/money.ts` uses JS numbers for integer cents with a safe-integer check (`:7-26`). `invoice-journal.ts:221-223` uses it.
- **[CONFIRMED] Stale comment:** `src/lib/tax/money.ts:4-9` says bill and invoice posting use neither helper and sum with `parseFloat`. Invoice posting imports `moneyToCents` (`src/lib/invoice-journal.ts:9,221-223`).

---

## 5. APIs, auth, and tenancy

### 5a. Server surface

- **[CONFIRMED]** 315 `createServerFn` calls across `src/routes/api/**`. These are TanStack Start RPC endpoints. A `_serverFn` route segment is present in `node_modules/@tanstack/start-plugin-core` build output. **UNKNOWN:** exact wire format (not verified).
- **[CONFIRMED]** Nitro routes (`server/routes/api/**`): better-auth catch-all `auth/[...all].ts`; `organization/active.get.ts` (cookie-session GET returning id, name, slug, logo, currency; `:1-35`); `dev/login.post.ts` (gated by `NODE_ENV !== "production"` and `DEV_LOGIN_BYPASS=true`); `internal/worker.post.ts` and `inbox-worker.post.ts` (Bearer `INBOX_WORKER_SECRET`); Stripe and PayPal webhooks; inbound email; manifest.

**Ledger reads**
- `getBalanceSheet`, `getProfitLoss`, `getPortfolioProfitLoss`, `getCashFlow`, `getTrialBalance`, `getApAging`, `getArAging` (`src/routes/api/-reports.ts:56,65,79,104,113,122,253`). Permission is `report:view`.
- `listTransactions`, `getTransaction`, `listAccountBalances` (`src/routes/api/transactions/-_queries.ts:42,321,472`).
- `listAccounts`, `getAccount`, `getAccountTransactionSummary` (`src/routes/api/-accounts.ts:100,175,590`).

**Tax reads**
- `getFilingWorkspace` (`-filing.ts:60`); `listComputedReturns` (`-tax-returns.ts:14`); `getPayrollVarianceReport` (`-payroll-variances.ts:72`); `listPayrollRuns` (`-payroll-runs.ts:181`); `getOrgTaxProfile` (`-payroll-runs.ts:388`); `listPayeeTaxProfiles` (`-tax-parties.ts:13`); `listTaxRegistrations` (`-tax-parties.ts:100`); `listWithholdingPayments` (`-tax-ewt.ts:19`); `getTaxSettings`, `listDeadlineOverrides`, `getTaxReferenceStaleness` (`-tax-settings.ts:13,239,258`); `getTaxModuleState` (`-tax-module-state.ts:20`); `getClosedThroughServerFn` (`transactions/-_period-close.ts:27`).

**Writes**
- Journal: `createTransaction`, `updateTransaction`, `voidTransaction`, `postTransaction` (`transactions/-_mutations.ts:36,84,290,372`); batch update and delete (`-_batch.ts:32,442`).
- Payroll: `importPayrollRegister` (`-payroll-runs.ts:46`); `createPayrollRun` (`:208`); `computePayrollFilingRun` (`:82`); `postPayrollFilingRun` (`:115`); `postAnnualizationTrueUp` (`:148`); `upsertEmployeeTaxProfile` (`:251`); `upsertOrgTaxProfile` (`:414`); `issuePayroll1601C` (~L467).
- Filing: `takeFilingSnapshot` (`-filing.ts:87`); `markPeriodFiled` (`:188`); `saveVatReturn`, `savePercentageTaxReturn`, `saveSlspReturn` (`-tax-returns.ts:40,90,148`).
- EWT: `captureWithholdingPayment` (`-tax-ewt.ts:56`); `buildStoredQap`, `saveStoredQap`, `remitWithholding`, `issueStoredQapDat` (`:117,151,311,336`).
- Settings: `upsertTaxSettings`, `upsertTaxBranch`, `upsertTaxYearElection` (`-tax-settings.ts:85,136,179`); `upsertPayeeTaxProfile`, `addTaxRegistration` (`-tax-parties.ts:40,128`); `updateOrganizationCountry` (`-tax-module-state.ts:36`); `setClosedThrough` (`transactions/-_period-close.ts:46`).

**Exports**
- `src/lib/export-ph.ts` registers PH tables for export (`:71,89,107,137,147,156,177`). The header says new PH tables must be registered and that a wiring test enforces it (`:1-12`).
- `exportData`, `validateImport`, `executeImport`, `listExportableRecords` (`src/routes/api/-export-import.ts:132,921,989,1423`).
- `exportTransactionsCSV`, `exportFinancialPackage` (`src/routes/api/-export-transactions.ts:225,492`).
- Client-side jsPDF for 2307 and 2316 (`src/lib/tax/form-2307-pdf.ts:42,97`; `form-2316-pdf.ts:62,271`).
- DAT encoders (`src/lib/tax/dat-encoder.ts:274`; `issue-qap-dat.ts:34`; `issue-payroll-artifacts.ts:184`).

### 5b. Auth and roles

- **[CONFIRMED]** Better Auth with Google OAuth, Email OTP (Resend), and the organization plugin (`src/lib/auth.ts:62-245`, plugins at `:144-234`). Roles map owner to superuser, plus admin, member, client_approver, report_viewer (`:178-186`). Owners only may create organizations (`:219-232`).
- **[CONFIRMED]** `requireSession` calls `auth.api.getSession` (`src/lib/auth-middleware.ts:32-40`). `getSessionContext` resolves the active or oldest membership, and the role is always read live from `auth_members` (`:44-115`).
- **[CONFIRMED]** Permissions (`src/lib/permissions.ts`): `journal` = view, create, update, delete, post, void, match, unmatch (`:22`). `member` gets view, create, update (`:106`). `clientApprover` gets view (`:134`). `reportViewer` gets journal view and report view/export (`:167,176`). Owner and admin get all (`:53-90`).
- **[CONFIRMED]** Wrappers (`src/lib/server-context.ts`): `withSessionOrgContext` has no permission check (`:117-124`). `withPermissionOrgContext` and `withMutationPermissionOrgContext` check permission (`:126-147`). `withSessionUserContext` is cross-organisation (`:98-104`).
- **[CONFIRMED]** `withOrgContext` sets `app.current_organization_id`, `app.current_user_id`, and `app.user_role` transaction-locally (`src/db/index.ts:74-100`). `dbAdmin` bypasses for context-free paths (`:46-57`).
- **[CONFIRMED]** RLS policies exist for every org-scoped tax and payroll table (`drizzle/rls_policies.sql:810-930`), plus journal headers and lines (`:82,187`). Global tables have none (`:794-805`).
- **[CONFIRMED]** RLS hardening Section B (FORCE, drop the NULL bypass) is commented out (`drizzle/rls_hardening.sql:254-262`). In the repo, application predicates are the boundary. **UNKNOWN:** whether production connects as the table owner (`src/db/index.ts:46-50` says `dbAdmin === db` until the app role flips).
- **[PARTIAL]** Cross-organisation enterprise access: `getAccessibleGroupEntitiesForGroups` (`src/lib/business-groups/service.ts:556-`) is used by `getPortfolioProfitLoss`. This is not an accountant-for-client model.
- **[MISSING]** Accountant-for-client authorisation (firm tables unused, §2).
- **[CONFIRMED]** Machine auth: only `INBOX_WORKER_SECRET` (`server/routes/api/internal/worker.post.ts:18-25`; `inbox-worker.post.ts:20-27`; `server/plugins/job-drain.ts:18-40`). Other bearer and API key hits are outbound AI and Resend keys.
- **[CONFIRMED]** `journal update` covers snapshot, file, 1601-C issue, and register import (`-filing.ts:96,190`; `-payroll-runs.ts`). A `member` can therefore snapshot and mark a period filed. Whether that is intended is **UNKNOWN**.

### 5c. Machine token path for a mini app

- **[MISSING]** No API keys, personal access tokens, OAuth provider, or bearer for user APIs (grep for `apiKey`, `api_key`, `bearer`, `oauth`, `mcp`, `personal access`).
- **INFERENCE:** a native Rox host cannot call Books server functions without new token auth (for example a Better Auth bearer or API-key plugin) or a forwarded session cookie. Both are new work.

---

## 6. Integration with Buwiz Forms, Rox, MCP, and export

- **[MISSING]** Searched `src`, `server`, `internal-docs`, `docs`, `.agent`, `README.md`, `AGENTS.md`, `CLAUDE.md`, and `package.json` for `rox`, `buwiz-forms`, `bir-core`, `bir_form`, `FormDraft`, `bir-mcp`, `mcp`, `modelcontextprotocol`, `mini-app`. Only hit is the Neon MCP reference in `.agent/skills/neon-postgres/SKILL.md`. No `.mcp.json` at depth two. No MCP SDK in `package.json`.
- **[CONFIRMED]** BIR outputs are generated in-house (DAT, PDF, computed payloads). Nothing imports from or exports to Buwiz Forms.
- **[CONFIRMED]** `docs/tax/2026-08-16-bir-withholding-coa-mapping-and-ebirforms-plan.md` refers to the BIR eBIRForms tool, not Buwiz Forms.
- **[CONFIRMED]** `src/routes` has `tax.compute`, `tax.ewt`, `tax.certificates`, `tax.parties`, `tax.settings`, `tax.deadlines`, `payroll`, and `payroll_.$runId`. No 1701 route.

---

## 7. Reuse versus coupling

Coupling is by direct imports only. Transitive imports were not fully audited.

| Module | Coupling | Reuse for a mini-app backend |
|---|---|---|
| `src/lib/tax/form-1601c.ts`, `percentage-tax.ts`, `compensation.ts`, `engine.ts`, `withholding.ts`, `vat.ts`, `ewt.ts`, `certificate-2307.ts`, `filing-period.ts`, `filing-workspace.ts`, `filing-snapshot.ts`, `payroll-posting-summary.ts`, `money.ts` | None framework or DB | **Pure.** Reusable as-is, subject to the defects above. |
| `src/lib/coa/mapping-registry.ts`, `mapping-types.ts`, `presets/*`; `src/lib/tax/ph-chart.ts` | None | Data and pure logic. |
| `src/lib/tax/ph-account-resolver.ts`, `src/lib/coa/resolve-mapped-account.ts` | Drizzle, `DbExecutor` type | Executor-injected. |
| `src/lib/tax/assemble-payroll-filing-workspace.ts`, `payroll-journal.ts` (`postPayrollRun(db, input)`), `issue-1601c.ts` | Drizzle schema, `@/db` type | Executor-injected. |
| `src/services/reports.ts` (`computeBalanceSheet` and others) | `../db` type, schema | Executor-injected, takes `orgId`. |
| `src/lib/period-close.ts` | `import { db }` runtime value and default parameter | **Coupled** to the singleton. |
| `src/db/index.ts` | Creates the pool at import from `DATABASE_URL` (`:23,43`) | Singleton. The `withOrgContext` pattern is reusable. |
| `src/lib/server-context.ts` | `@tanstack/react-start/server` (`:1`) | **TanStack-coupled.** |
| `src/lib/auth-middleware.ts`, `src/lib/auth.ts` | Better Auth singleton and `db` (`auth-middleware.ts:1,10`); Resend and brand | **Coupled.** |
| `src/routes/api/**` (315 server functions) | TanStack Start RPC | Not an HTTP API. |
| `server/routes/api/**` | h3 and Nitro | HTTP, few routes, cookie only. |

**Options (facts, not a recommendation):**
- **A.** Import the pure engines into a new backend. Supply a DB executor and re-implement tenancy and auth. Avoid the `@/db` singleton and the `period-close.ts` default. Keep the `payroll-posting-summary.ts` split, which was designed so unit tests stay hermetic (`payroll-posting-summary.ts:1-6`).
- **B.** Add an HTTP adapter in Books, a new Nitro route wrapping `withPermissionOrgContext`. This needs new token auth first.
- **C.** Put the adapter in the mini-app or Forms side. It still needs the same auth.

---

## 8. Gaps by severity

**P0: correctness and evidence**
1. 1601-C employee contributions not netted (§1.1). Fix plus a test that fails on the current code.
2. Re-snapshot after filing not refused (§1.6).
3. 1601-C reconciliation not passed to the filing workspace (§1.5).
4. Snapshot payload not persisted; export header overclaims (§1.7).
5. 2551Q 8% election client-controlled and not checked against stored election; UI default `true` (§1.4). The election write path also lets `hasCompensationIncome` change after election (`-tax-settings.ts:219-222`).
6. Semi-monthly 1601-C collision (§1.8).
7. Period lock reopen unrestricted; tax filing ignores lock (§1.9).

**P1: authorisation, privacy, ledger integrity**
8. `issuePayrollFilingArtifacts` has no permission gate (§1.10).
9. TIN columns are plaintext. The tax, party, and certificate modules import no crypto helper. Crypto is used only for org secrets, financial-account secrets, and AI credentials. At-rest encryption elsewhere is **UNKNOWN**.
10. RLS hardening not applied in repo (§5b).
11. Control movement's date-range path ignores the duplicate predicate (`assemble-payroll-filing-workspace.ts:291-305`). **Latent**: `takeFilingSnapshot` can reach it only when unposted, and the posting gate blocks that first (`:241-248`).
12. INFERENCE: `amendPostedJournal` has no payroll-source guard (`src/lib/journal-amendment.ts:100-135`; grep shows no `payroll` string in the file). A payroll journal can be reversed or replaced, but `payroll_runs.journalHeaderId` keeps pointing at the original. The 1601-C movement is run-scoped to that header, so it may not reflect the reversal. Not tested.
13. Bill and invoice line amounts are decimal(15,2) against decimal(20,8) for journal lines. Precision differs by table.

**P2: missing capability and rules to verify**
14. No VAT, percentage-tax, creditable-VAT, or employee-advance posting (§3b).
15. Invoice tax posts to generic `sales_tax_payable` 21500, not PH output VAT (§1.11).
16. No credit notes, sales-return documents, or ACCN or series on invoices (§2).
17. Org tax profile has no taxpayer or entity type (individual, corporate, partnership) and no COR (§2).
18. Accountant-for-client access missing (§2, §5b).
19. CWT posting ignores `certificateStatus` (§4g).
20. `tax_certificates` period columns are text (§2).
21. 1601-C and 2551Q formatting inconsistent (§4a, §4i).
22. `VAT_THRESHOLD` duplicated (§4d).
23. Tax amendment path missing. `amendmentSequence` is always 0 and `amended` is never reached (§4h).
24. `lockedAt` never written (§2).
25. Registrations do not drive regime selection (§2).
26. `isIndividual`, `hasCompensationIncome`, and `payeeType` are UI state (`tax.compute.tsx:30,35-36`), not profile-driven.
27. No offline or local store anywhere (grep on `src/db` and `src/lib`).
28. No 1701 support (§1.2).

---

## 9. Tests (existing; none run)

- `tests/unit/lib/tax/form-1601c.test.ts`: `buildForm1601C` L35-122; `dueDateFor` L125-171; `compensationFromPayrollLine` L173-185 (MWE only).
- `tests/unit/lib/tax/percentage-tax.test.ts`: rates L17-30; `assessRegime` L32-75; `computeEightPercent` L77-124; `computePercentageTax` L126-139; `monitorThreshold` L141-168; `eightPercentBreachOutcome` L170-200; `buildPercentageTaxReturn` L204-244 (includes the 8% block at L229).
- `tests/unit/lib/tax/filing-snapshot.test.ts` L32-205; `filing-workspace.test.ts`; `filing-period.test.ts`; `compensation.test.ts`; `withholding.test.ts`; `payroll-journal.test.ts`; `payroll-period.test.ts`; `money.test.ts`; `tests/unit/money-discipline.test.ts`.
- `tests/unit/tax-reference-wiring.test.ts:590-596`: string-presence test that `issue-1601c.ts` contains `compensationFromPayrollLine`.
- `tests/unit/lib/coa/ph-chart-lock.test.ts`; `ph-number-reservation.test.ts`; `mapping-registry.test.ts`; `plan-preset.test.ts`.
- `tests/unit/lib/period-close.test.ts`; `period-close-adapter.test.ts`.
- Integration (need Postgres, not run): `tests/integration/payroll-filing-glue.test.ts` (run-scoped movement, L287-356); `payroll-filing-state.test.ts` (L49-55, L68); `payroll-run-service.test.ts`; `payroll-compute-idempotency.test.ts`; `journal-amendment.test.ts`; `tax-certificates.test.ts`; `ph-filing-pack2.test.ts`; `coa-resolve-mapped-account.test.ts`.
- **Not covered by any test I found:** 1601-C contribution netting; re-snapshot after filing; 1601-C reconciliation gate; semi-monthly 1601-C key; 2551Q election read from storage; payroll journal reversal.

---

## 10. UNKNOWN, and what needs the owner or external verification

- Production DB role (owner or `app_runtime`), and therefore whether RLS is enforced.
- Current BIR rules: whether the 1601-C taxable compensation line nets employee contributions. The repo's own comment says employee shares are deductions (`compensation.ts:95-99`), and the workbook example nets them. Confirm against the current form instructions.
- 8% election effective-year and transition rules (`percentage-tax.ts` comments versus code).
- ATC rate table and ATC selection rules (`ewt.ts:70-127`; `certificate-2307.ts:92-110`).
- VAT threshold ₱3,000,000 (`percentage-tax.ts:50`, `vat.ts:70`) and the CREATE 1% window (`percentage-tax.ts:65-68`).
- Whether any live tenant uses semi-monthly payroll (finding 8).
- Whether any Rox or Buwiz mini app calls Books. Nothing in this repo.
- Runtime state of any database. No DB was accessed.
- Not audited: bill and invoice posting internals beyond mappings; reconciliation module; annualization arithmetic; `dat-layouts.ts`; `benefits.ts`; inbox ingestion; `journal-amendment.ts` beyond the guard list.

---

## 11. Probe evidence

Script: `/private/tmp/claude-501/-Volumes-goldcoders-OSS-buwiz-books/d723bb20-1674-49c1-8fb6-568f056d2c7e/scratchpad/verify/verify-ph-tax.ts`. Run with `/Users/uriah/.bun/bin/bun run` from the repo root. Bun 1.4.2. Synthetic data only. No writes to the repo.

- **A** `compensationFromPayrollLine` (basic 40000, de minimis 2000, employee SSS 2000, reported tax 1000): gross 42000, nonTaxable 2000, tax 1000. No contribution field in the output.
- **B** `segregate()` on the same figures: `taxableRegular` 38000, `employeeContributions` 2000, `totalNonTaxable` 2000, gross 42000.
- **C** `buildForm1601C` (September 2026, movement 1000): total 42000, nonTaxable 2000, **taxable 40000**, tax 1000, reconciled true, period 2026-09-01 to 2026-09-30, due 2026-10-10, no blocking issues.
- **D** Two half-month runs for the same month: both give periodStart 2026-09-01 and periodEnd 2026-09-30, with tax 500 and 600. Same key.
- **E** 2551Q, gross 300000, 3%: taxDue 9000.00, due 2026-10-25, no blocks.
- **F** Same with `electedEightPercent: true`: taxDue 9000.00 and a blocking string about the 8% election "IN LIEU OF percentage tax".
- **G** Caller-typed gross 290000: taxDue 8700.00.
- **H** `percentageTaxRateBps`: 300 for 2026-09-30; 100 for 2022-03-31.
- **I** `summarizePayrollPosting` (same line, expected employer SSS 2400): gross 42000, tax 1000, SSS employee 2000, SSS employer 2400, net 39000, employer expense 2400.