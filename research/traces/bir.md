## BIR rules and form editions, as of 2026-10-29

**Bottom line**
- The forms in use are still the January 2018 (ENCS) editions of 2551Q, 1601-C, 1701Q, 1701, 1701A, 2306, 2307 and 1600-PT. The 1701-MS is the August 2024 edition, now e-fileable through eBIRForms 7.9.6.0 (RMC 36-2026, RMC 37-2026). No 2025–26 revision of any other form was found.
- Sec 116 percentage tax is 3% of gross quarterly sales. The 1% window ended 30 June 2023 (RMC 69-2023). Under amended NIRC §128(1), the 2551Q is quarterly, due 25 days after quarter-end. No BIR issuance removing the 2551M was found.
- 8% electors do not file the 2551Q. A breach of the ₱3M threshold mid-year moves the year to graduated rates, with percentage tax from the start of the year until VAT liability.
- Percentage tax withheld is credited on the 2551Q through 2307 WB-series codes (WB080/WB082), not the PT-series. Income-tax withholding goes through WI/WC codes.
- The 1601-C employee-share line (item 19) excludes employer contributions. The ₱90,000 cap on 13th-month and other benefits is confirmed.
- The RMC 20-2026 Annex C that supposedly says 1701-MS can be used without a COR could not be retrieved, so that claim is unverified.

### 0. Scope, method, repos
- Handover read: `Buwiz_Books_Rox_Claude_Handover.md` lines 115–141 and 175–180. No repository was read in this track, so git HEAD is N/A.
- Method: BIR PDFs were fetched with curl into fresh scratch directories and read as page images or text. WebFetch could not extract PDF text. Statutes came from lawphil.net HTML. Several BIR CDN paths returned HTTP 403 (section 6).
- Labels: CONFIRMED = read in primary text or image. SECONDARY = search snippet or non-BIR source only. UNVERIFIED = not confirmed from a primary source. MISSING = searched and not found (method stated). UNKNOWN = not determinable. INFERENCE = my reasoning from confirmed facts.

### 1. Form editions

| Form | Edition on printed form | Status 2026-10-29 | Evidence |
|---|---|---|---|
| 2551Q Quarterly Percentage Tax Return | January 2018 (ENCS) | CONFIRMED latest found. No 2025–26 revision (MISSING: searches for 2025/2026 revision). | Printed form read. Second CDN copy `2551Q Jan 2018 ENCS final rev 3_copy.pdf` has the same label and PT rows. RMC 36-2026 item 5 bug-fix list names "2551Qv2018" (CONFIRMED, image). RMC 26-2018 (25 Apr 2018) circularized it (CONFIRMED). |
| 1601-C Monthly Remittance Return of Income Taxes Withheld on Compensation | January 2018 (ENCS) | CONFIRMED latest found (MISSING for later editions). | Printed form read. RMC 36-2026 bug-fix list does not name 1601-C. |
| 1701-MS Annual IT Return for Individuals Classified as MICRO or SMALL Taxpayers | August 2024 (08/24) | CONFIRMED current. Named in eBIRForms v7.9.6.0 (RMC 36-2026 item 1). | Printed form; RMC 34-2025 Annex A (Aug 2024 guide); RMC 37-2026. |
| 1701Q Quarterly IT Return (individuals, estates, trusts) | January 2018 (ENCS) | CONFIRMED printed. Guide also January 2018 (ENCS). | Printed form; `1701Q Guide Jan 2018_copy.pdf`. |
| 1701 Annual IT Return (incl. mixed income) | January 2018 (ENCS), 4 pp. Has TABLE 1 (2018–2022) and TABLE 2 (2023 onward). | CONFIRMED printed. eBIRForms "1701v2018" (RMC 20-2026 digest). | Printed form. |
| 1701A Annual IT Return (purely business/profession; OSD or 8%) | January 2018 | CONFIRMED printed. eBIRForms "1701A" (RMC 20-2026 digest). | Printed form. |
| 2306 Certificate of Final Tax Withheld at Source | January 2018 (ENCS) v4 | CONFIRMED printed. | Printed form. |
| 2307 Certificate of Creditable Tax Withheld at Source | January 2018 (ENCS) v3 | CONFIRMED printed. | Printed form. |
| 1600-PT Monthly Remittance Return of Other Percentage Taxes Withheld | January 2018 | CONFIRMED printed. Due date UNVERIFIED. | Printed form. |
| 2551M Monthly Percentage Tax Return | none found | UNVERIFIED status. | RMC 26-2018 still describes 2551M payments (CONFIRMED). A stale, undated eBIRForms list shows 2551M "September 2005 (ENCS)". No withdrawing issuance found (MISSING). |

The undated eBIRForms list (`75037Annex A-Available BIR Forms List.pdf`) shows 1601-C "July 2008 (ENCS)", 2551Q "February 2002 (ENCS)" and 1701Q "July 2008 (ENCS)". It is stale and was used only as negative evidence.

### 2. Bookkeeping-relevant line items (printed)

**2551Q (January 2018 ENCS), CONFIRMED from printed form**
- Item 13 (individuals whose sales are subject to Sec 116 PT): "What income tax rates are you availing? (choose one) (To be filled out only on the initial quarter of the taxable year)". Options: "Graduated income tax rate on net taxable income" / "8% income tax rate on gross sales/receipts/others".
- Part II: 14 Total Tax Due (From Schedule 1 Item 7). 15 "Creditable Percentage Tax Withheld per BIR Form No. 2307". 16 Tax Paid in Return Previously Filed (amended). 17 Other Tax Credit/Payment. 18 Total Tax Credits/Payments (15–17). 19 Tax Still Payable/(Overpayment) (14 less 18). 20 Surcharge. 21 Interest. 22 Compromise. 23 Total Penalties (20–22). 24 Total Amount Payable/(Overpayment).
- Part III: 25 Cash/Bank Debit Memo; 26 Check; 27 Tax Debit Memo; 28 Others.
- Schedule 1 (p.2): columns ATC | Taxable Amount | Tax Rate | Tax Due; rows 1–6; 7 "Total Tax Due (Sum of Items 1 to 6)(To Part II Item 14)".
- Table 1 ATCs: PT 010 "Persons exempt from VAT under Sec. 109(BB) (Sec. 116)" 3%. PT 040 carriers/garages (Sec 117) 3%. PT 041 international carriers (Sec 118) 3%. PT 060 gas/water franchises 2%. PT 070 radio/TV ≤₱10M 3%. PT 090 overseas dispatch 10%. PT 101/105 lending (Sec 121) 1%/5% by maturity. PT 102 dividends 0%. PT 103 royalties/rentals 7%. PT 104 net trading gains 7%. PT 113/114 (Sec 122) 5%/1%. PT 115 other gross income 5%. PT 120 life insurance 2%. PT 130 insurance agents 4%. PT 132 property owners 5%. PT 140/150/160/170/180 (Sec 125) 18%/18%/10%/15%/30%.
- Guide (Jan 2018): "The return shall be filed and the tax paid within twenty-five (25) days" (2551Q guide line 53). "The tax is based on gross sales, receipts or earnings."

**1601-C (January 2018 ENCS), CONFIRMED from printed form**
- Header: 1 For the Month; 2 amended; 3 any taxes withheld; 5 ATC "WW010"; 11 category of withholding agent (private/government); 13 payees with tax relief.
- Part II: 14 Total Amount of Compensation. Less non-taxable: 15 Statutory Minimum Wage for MWEs. 16 Holiday Pay, Overtime Pay, Night Shift Differential Pay, Hazard Pay (for MWEs only). 17 13th Month Pay and Other Benefits. 18 De Minimis Benefits. 19 SSS, GSIS, PHIC, HDMF Mandatory Contributions & Union Dues (employee's share only). 20 Other Non-Taxable Compensation (specify). 21 Total Non-Taxable (15–20). 22 Total Taxable Compensation (14 less 21). 23 Less: "Taxable compensation not subject to withholding tax (for employees, other than MWEs, receiving P250,000 & below for the year)". 24 Net Taxable Compensation. 25 Total Taxes Withheld. 26 Adjustment from previous month/s (Part IV Sch. 1 item 4). 27 Taxes Withheld for Remittance. 28 Tax remitted in amended return. 29 Other remittances. 30 Total remittances. 31 Tax Still Due/(Over-remittance). 32 Surcharge. 33 Interest. 34 Compromise. 35 Total Penalties. 36 TOTAL AMOUNT STILL DUE/(Over-remittance).
- Part IV Schedule I: columns 1 Previous Month/s, 2 Date Paid, 3 Drawee Bank, 4 Number, 5 Tax Paid, 6 Should be Tax Due for the Month, 7 = 6 less 5. Item 4 total feeds Part II item 26.
- Page 2 note: "The last 5 digits of the 14-digit TIN refers to the branch code."

**1701-MS (August 2024), CONFIRMED from printed form**
- Part I: 11 Source of Income (Business / Mixed / Profession / Compensation Income). 12 Income subject to (Graduated / 8% / Exempt / Special), with the note "If with two (2) or more activities OR multiple tax regimes or tax rates, please use BIR Form No. 1701". 17 Method of Deduction (Itemized, Sec 34(A–J), or OSD, 40% of gross sales, Sec 34(K)). 18 ATC (Table 1).
- Part II: 19 IT due special rate; 20 less share of other agency; 21 net; 22 IT due regular rate; 23 total IT due; 24 less total credits (Part V item 10); 25 payable/(overpayment); 26 "Less: Tax Due Allowed for 2nd Installment to be paid on or before October 15 (50% or less of Item 23)"; 27–31 amount payable, penalties, totals, overpayment options.
- Part IV A (compensation): 1 Gross Compensation; 2 Non-Taxable/Exempt; 3 Taxable Compensation; 4 Tax Due (Item 3 × Table 2 rate).
- Part IV B1 (graduated/OSD): 5 Sales/Revenues/Fees; 6 Less Sales Returns, Allowances and Discounts; 7 Net Sales; 8 Less Cost of Sales/Services (only if itemized); 9 Gross Income from Operation; 10A–10D itemized deductions (10C = NOLCO); 11 OSD (40% of Item 7); 12 Net Income (itemized: 9 less 10D; OSD: 9 less 11); 13 Other Non-Operating Income; 14 Taxable Income–Business; 15 Total Taxable Income; 16–17 special/exempt rate; 18A → Part II 19; 18B → Part II 22.
- Part IV B2 (8%): 19 Sales/Revenues/Fees (net of returns, allowances, discounts); 20 Other Non-Operating Income; 21 Total Income; 22 "Less: Allowable reduction ... of purely self-employed individuals and/or professionals in the amount of P250,000 (not applicable if with compensation income)"; 23 Taxable Income; 24 Tax Due–Business (23 × 8%); 25 Total Tax Due–Compensation & Business (4 + 24) → Part II 22.
- Part V credits: 1 prior-year excess; 2 tax payments Q1–Q3; 3 creditable tax withheld Q1–Q3; 4 "Creditable Tax Withheld per BIR Form No. 2307 or BIR Form No. 1606 for the 4th Quarter"; 5 "per BIR Form No. 2316"; 6 amended-return payment; 7 foreign tax credits; 8 special credits; 9 other; 10 total → Part II 24.
- ATCs: II011 Compensation; II012 Business–Graduated; II013 Mixed–Graduated; II014 Profession–Graduated; II015 Business–8%; II016 Mixed–8%; II017 Profession–8%.
- Table 2 (from 1 Jan 2023): ≤250k 0%; 250k–400k 15% of excess over 250k; 400k–800k ₱22,500 + 20%; 800k–2M ₱102,500 + 25%; 2M–8M ₱402,500 + 30%; >8M ₱2,202,500 + 35%.
- Rounding (Part II header): "DO NOT enter Centavos; 49 Centavos or Less drop down; 50 or more round up."

**1701Q / 1701 / 1701A, PARTIAL**
- 1701Q: quarter First/Second/Third; ATC family II011–II017; rate choice (graduated vs 8% "in lieu of Graduated Rates ... & Percentage Tax"). Printed note: the 8% choice is "irrevocable for this taxable year", but "shall automatically be changed to graduated IT rates when gross sales/receipts and other non-operating income exceed Three million pesos (P3M)". The full credit-line list was not transcribed.
- 1701: credit lines 59 "Creditable Tax Withheld for the First Three (3) Quarters" and 60 "Creditable Tax Withheld per BIR Form No. 2307 for the 4th Quarter". Tables 1 and 2 as above.
- 1701A: "Individuals Earning Income PURELY from Business/Profession [those under the graduated income tax rates with OSD ... OR those who opted to avail of the 8% flat income tax rate]".

### 3. Rules

**3.1 Percentage tax (Sec 116) and 2551Q**

| # | Rule | Status | Citation | Implication for Books/Forms |
|---|---|---|---|---|
| PT-1 | Rate 3% of gross quarterly sales or receipts for non-VAT persons with VAT-exempt sales; cooperatives exempt | CONFIRMED (current text) | RA 11534 §13 amending NIRC §116 (`ra11534.clean.txt` line 140); RA 11976 §24 (`ra11976.norm2.txt` lines 189–190, heading typed "Section 16") | 2551Q base rate 3% |
| PT-2 | 1% from 1 Jul 2020 to 30 Jun 2023; reverted to 3% from 1 Jul 2023 | CONFIRMED | RA 11534 §13 proviso; RMC 69-2023 (20 Jun 2023): rate "shall now revert to three percent (3%)" for corporations and self-employed/professionals ≤₱3M, except cooperatives and 8% electors | Prior-period 2551Q needs a dated rate table (1%/3%) |
| PT-3 | Quarterly return; due "within twenty-five (25) days after the end of each taxable quarter"; may be filed to an AAB, RDO/RCO, or an authorized tax software provider; a VAT-cancelled person's tax accrues from cancellation date | CONFIRMED | NIRC §128(1) as amended by RA 11976 §29 (`ra11976.norm2.txt` lines 209–212); 2551Q guide line 53 | Deadline = quarter-end + 25 days. VAT cancellation starts a PT accrual |
| PT-4 | Monthly 2551M route | Statute now states only a quarterly return. No BIR issuance withdrawing 2551M found (MISSING: searched 2024–26 for 2551M discontinuance). RMC 26-2018 (2018) allowed monthly 2551M payments credited on the 2551Q | `ra11976.norm2.txt` lines 209–212; RMC 26-2018 items a.v | Do not rely on monthly 2551M without BIR confirmation. Premise only PARTIAL |
| PT-5 | ATC PT010 = "Persons exempt from VAT under Sec. 109(BB) (Sec. 116)", 3% | CONFIRMED (printed) | 2551Q Table 1 | Default PT return ATC for §116 persons |
| PT-6 | Lettering: the VAT-exempt test is §109(BB) under TRAIN and §109(CC) under EOPT | CONFIRMED | `ra10963.clean.txt` line 330; `ra11976.norm2.txt` lines 131, 190 | Keep ATC mapping by concept. The printed PT010 label uses pre-EOPT lettering |
| PT-7 | ₱3,000,000 VAT-exempt threshold is "adjusted to its present values using the consumer price index ... every three (3) years" | CONFIRMED (text). Whether an adjusted figure has been published: UNKNOWN | `ra11976.norm2.txt` line 131 | Do not hard-code ₱3M. It also drives the 8% test |
| PT-8 | PT taxable amount = total gross sales/receipts for the quarter | CONFIRMED (RMC 26-2018 item iii). No printed deduction for sales returns found in the 2551Q form or guide | RMC 26-2018 | Handover example deducts a ₱10,000 "eligible return" before 3%. Unverified against printed instructions |
| PT-9 | RR 3-2024 (from brief) | MISSING (searched "Revenue Regulations No. 3-2024 percentage tax 2551Q": no hits) | — | UNKNOWN. Do not encode |
| PT-10 | CREATE MORE Act effect on §116 or §24(A)(2)(b) | UNVERIFIED (not checked; named only in a ClearTax snippet) | — | — |

**3.2 8% option**

| # | Rule | Status | Citation | Implication |
|---|---|---|---|---|
| 8-1 | 8% of gross sales/receipts and other non-operating income in excess of ₱250,000, in lieu of graduated IT and §116 PT; for purely self-employed/professionals whose gross sales ≤ VAT threshold | CONFIRMED | TRAIN §5, NIRC §24(A)(2)(b) (`ra10963.clean.txt` line 64); RR 8-2018 digest (`digest-rr8-2018/doc.txt` lines 48–52); RMO 23-2018 pt 2(b) | 8% base = receipts + other non-operating income − ₱250k |
| 8-2 | Who may elect: self-employed/professional/mixed earners with receipts + other non-operating income ≤ VAT threshold in the year; registered and subject only to §116 PT, or exempt from VAT/other PT; must signify election | CONFIRMED | RMO 23-2018 pts 2, 4 | Eligibility needs a per-year receipts test and registration profile |
| 8-3 | Excluded: purely compensation earners; VAT-registered regardless of amount; exempt taxpayers whose receipts exceeded ₱3M in the year; subject to other Title V PT except §116; GPP partners; income-tax-exempt individuals; BMBE status (cannot combine) | CONFIRMED | RMO 23-2018 pt 5; BMBE paragraph after pt 5 | Flag VAT status, GPP membership, BMBE status, exemption |
| 8-4 | Election timing: new registrant — on BIR 1901 and/or 1701Q, or the initial-quarter return (2551Q and/or 1701Q) after commencing. Existing — BIR 1905 at start of year (end-dates the 2551Q form type) with 8% selected in the initial quarterly IT return; or 1st-quarter PT return and/or 1st-quarter IT return. Otherwise graduated | CONFIRMED | RMO 23-2018 pt 7.1–7.2; RR 8-2018 digest (`digest-rr8-2018/doc.txt` lines 53–58); 2551Q item 13 | Store election event (form, quarter, date) per taxable year |
| 8-5 | Re-elect each taxable year; irrevocable and no amendment within that year | CONFIRMED | RMO 23-2018 pts 6, 8 (with example); RR 8-2018 digest | Year-scoped election record |
| 8-6 | Effect on 2551Q: an 8% individual is "Not required to file the Quarterly Percentage Tax Return"; must file quarterly IT returns and annual IT return and keep books. Stays registered under "Percentage Tax – Quarterly", with the 2551Q form type end-dated | CONFIRMED | RMO 23-2018 pts 9(a)–(e), 14 | Suppress 2551Q generation for 8% years even if the COR lists it |
| 8-7 | Mid-year breach (receipts > ₱3M any time in the year): automatic graduated rates for the year; VAT prospectively; "Percentage tax shall be imposed from the beginning of the year until taxpayer is liable to VAT"; graduated tax computed with credit for prior-quarter 8% payments | CONFIRMED | RMO 23-2018 pt 9 (final paragraph); RR 8-2018 digest (`digest-rr8-2018/doc.txt` lines 74–79); 1701Q printed note | Needs a running receipts tally, a reclassification event, and an 8% payments credit ledger |
| 8-8 | Mixed income: compensation at graduated rates; business receipts at 8% only if ≤ VAT threshold (else graduated). The ₱250k reduction is not available to mixed earners, because it is already in the first graduated bracket on compensation. Total tax = graduated tax on compensation + 8% × total business receipts (no second ₱250k subtraction) | CONFIRMED | RMO 23-2018 pts 3, 10–12; RR 8-2018 digest (lines 92–119); 1701-MS Part IV B2 item 22 | Split compensation and business receipts. No ₱250k line for mixed earners |
| 8-9 | Mixed earners on graduated rates with receipts ≤ threshold remain subject to 3% PT and 2551Q | INFERENCE from RMO pt 2(a), 3.2(a)(i) and RMC 69-2023 | — | Graduated mixed earners may still need 2551Q |
| 8-10 | Amendments to RR 8-2018 or RMO 23-2018 after 2018 | MISSING (searched "RR 8-2018 amendment" and bir.gov.ph/bir-cdn domains: only 2018–19 items). RA 11976 text contains no "8%" string (grep of full 73 KB text). 1701-MS (Aug 2024) still has the 8% fields | `ra11976.norm2.txt` | Framework appears unchanged through Aug 2024. After that UNKNOWN |
| 8-11 | RR 8-2018 full text | UNVERIFIED: BIR CDN RR 8-2018 returned 403 (curl and WebFetch). Only the BIR-hosted digest was read | — | Cite the digest and RMO 23-2018 |

**3.3 1701-MS**

| # | Rule | Status | Citation | Implication |
|---|---|---|---|---|
| M-1 | Title: annual IT return for individuals classified as micro or small taxpayers | CONFIRMED | 1701-MS p.1 header | — |
| M-2 | Tiers by taxable-year gross sales: micro < ₱3,000,000; small ₱3,000,000 to < ₱20,000,000; medium ₱20,000,000 to < ₱1,000,000,000; large ≥ ₱1,000,000,000 | CONFIRMED | RR 8-2024 (11 Apr 2024) §2 A–D (`rr8-2024b/doc.txt` lines 7–17); NIRC §21(b) as amended by RA 11976 (`ra11976.norm2.txt` lines 27–37) | Store tier per year. The lawphil transcription prints "P3,000.00"/"P1,000,000.00" (off by 1,000) against the words. Treat as transcription error |
| M-3 | Gross sales for classification = total sales revenue, net of VAT, no other deductions; business income only; excludes employer-employee compensation, passive income under §§24, 25, 27, 28, and income excluded under §32(B) | CONFIRMED | RR 8-2024 (`rr8-2024b/doc.txt` lines 19–23) | Different base from the 8% test (receipts + other non-operating income). Compute both |
| M-4 | Initial tiers: 2022-and-prior registrants by 2022 gross sales; 2023–24 registrants before effectivity = micro (VAT-registered = small); new registrants by declared registration form; reclassification by threshold, notified by separate issuance | CONFIRMED | RR 8-2024 (`rr8-2024b/doc.txt` lines 24–39) | Keep tier history and a flag on default micro |
| M-5 | Who files: individuals engaged in trade, business or profession (incl. mixed income) classified micro or small. Includes resident citizens; resident aliens and non-resident individuals engaged in PH business; trustees/guardians/estates where the estate or minor is in trade; individuals with compensation income too | CONFIRMED (items 1–4 read) | RMC 34-2025 Annex A (`rmc34-2025/raw.txt` lines 355–370) | Medium and large not eligible (INFERENCE from "shall be filed by ... micro ... or small") |
| M-6 | Two or more activities or multiple tax regimes/rates → use 1701 | CONFIRMED (printed note, 1701-MS Part I item 12) | 1701-MS p.1 | Routing rule 1701-MS vs 1701 |
| M-7 | COR precondition | MISSING: no COR requirement in RMC 34-2025 Annex A, RMC 20-2026 digest (grep "certificate/COR/registration"), RMC 36-2026, RMC 37-2026. The handover's claim that the 2026 Q&A (RMC 20-2026 Annex C(3)) allows 1701-MS without a COR listing is UNVERIFIED (Annex C returned 403) | — | Do not gate 1701-MS on COR until Annex C is read |
| M-8 | Filing and payment channels (current) | CONFIRMED (RMC 37-2026, amends RMC 20-2026 dated 16 Mar 2026; printed date 27 Apr 2026; receipt stamp 28 Apr 2026). 1701-MS manual: with tax payable → any AAB; without → any RDO, with proof of payment. Electronic: Offline eBIRForms Package 7.9.6. Online payment via ePayment gateways (Sec III(2)). 1701A and 1701: eBIRForms 7.9.5 or eFPS | 1701-MS is not on the eFPS list (RMC 20-2026 digest, section A: eFPS lists 1700, 1701, 1701A, 1702RT/EX/MX). Target eBIRForms 7.9.6 for 1701-MS |
| M-9 | Deadline: "on or before the 15th day of April of each year covering income of the preceding taxable year". 2025 AITR extended to 15 May 2026 by RMC 30-2026 | CONFIRMED (RMC 34-2025 Annex A; RMC 36-2026 item 1). RMC 30-2026 itself not read | — | — |
| M-10 | Attachments: list includes "BIR Form No. 2307 [Certificate of Creditable Tax Withheld at Source]" | CONFIRMED | RMC 34-2025 Annex A (`rmc34-2025/raw.txt` line 667) | Keep 2307 copies attached to 1701-MS |
| M-11 | Handover example: ₱1,190,000 sales, OSD 40% → Item 11 ₱476,000; Item 12 ₱714,000; graduated tax (Table 2) ₱85,300 before credits, no compensation. 8% alternative: 8% × (₱1,190,000 − ₱250,000) = ₱75,200, if eligible | Arithmetic from printed Table 2 and Part IV B2. Not a filing check | — | Credits, prior payments and eligibility not checked |

**3.4 1601-C non-taxable compensation (employee share only)**

| 1601-C item | Printed label | Legal basis | Status |
|---|---|---|---|
| 15 | Statutory Minimum Wage for MWEs | RR 11-2018 §2.78.1(B)(13): MWE compensation at SMW (private), and public employees paid not more than SMW. TRAIN §5 proviso: MWEs exempt on taxable income | CONFIRMED |
| 16 | Holiday, OT, NSD, hazard pay (MWEs only) | Holiday pay: TRAIN §5 proviso ("holiday pay ... shall likewise be exempt from income tax", `ra10963.clean.txt` line 63). OT, NSD and hazard pay: legal basis UNVERIFIED | Partial |
| 17 | 13th-month pay and other benefits | NIRC §32(B)(7)(e) as amended by TRAIN §9: "shall not exceed Ninety thousand pesos (₱90,000)" (`ra10963.clean.txt` lines 101–106). RR 11-2018 §2.78.1(B)(11): 13th month + Christmas bonus, productivity incentives, loyalty award, gifts; cap may be raised by Finance Secretary | CONFIRMED |
| 18 | De minimis benefits | RR 11-2018 §2.78.1 item (3): (a) monetized unused VL ≤10 days (private); (b) monetized VL/SL (government); (c) medical cash allowance to dependents ≤₱1,500 per semester (₱250/mo); (d) rice subsidy ≤₱2,000 or 50 kg sack/month; (e) uniform/clothing ≤₱6,000/yr. Other items not in the RR 11-2018 text read | (a)–(e) CONFIRMED; rest UNVERIFIED |
| 19 | SSS, GSIS, PHIC, HDMF contributions and union dues — "employee's share only" | RR 11-2018 §2.78.1(B)(12): "GSIS, SSS, Medicare and Pag-Ibig contributions, and union dues of individual employees." NIRC §32(B)(7)(f) text not in the amending statute | RR CONFIRMED; statute UNVERIFIED |
| 20 | Other non-taxable (specify) | Printed only | — |
| 23 | Less: taxable compensation not subject to withholding (non-MWE employees receiving ₱250,000 & below for the year) | Printed. Legal basis UNVERIFIED. The only ₱250,000 text in RR 11-2018 (lines 794–830) concerns §2.57.2 lone-payor income payments, not compensation | Partial |

**3.5 Creditable and final withholding**

| # | Rule | Status | Citation | Implication |
|---|---|---|---|---|
| W-1 | 2551Q item 15 = creditable PT withheld per 2307 | CONFIRMED (printed) | 2551Q p.1 | PT credit line on 2551Q |
| W-2 | WB080 = persons exempt from VAT under Sec 109(BB), creditable, government withholding agent, 3%. WB082 = same, private agent, 3%. WB084 = same, final, "Section 116 applies", 3% | CONFIRMED | 1600-PT Table 1 (`form1600pt/doc.txt` lines 142–144); 2307 (`form2307b/doc.txt` lines 158, 161); 2306 item 42 | The brief's "PT..." example is wrong. Government-withholding code for §116 persons is WB080 |
| W-3 | 2307 table lists WB-series percentage-tax codes (e.g. WB030 carriers 3%, WB090 overseas 10%, WB130 international carriers 3%, WB080/WB082) and WI/WC income-tax codes (WI010/WI011 "did not exceed P 3M" / "more than P 3M or VAT Registered") | CONFIRMED (23 WB lines, 68 WI/WC lines) | `form2307b/doc.txt` lines 74–75, 158–197 | Route by series: WB → 2551Q item 15; WI/WC → IT credits (INFERENCE from form headings) |
| W-4 | 2306 is a Certificate of Final Tax Withheld; its "substituted filing" declaration relies on RR 14-2003 optional registration for 3% final PT withholding (WB084) | CONFIRMED (printed) | 2306 declaration lines 67–71 | Final PT is not a 2551Q credit (INFERENCE: item 15 cites only 2307) |
| W-5 | 1701-MS Part V: items 1–10 as above; item 3 Q1–Q3 creditable tax; item 4 2307 or 1606 for Q4; item 5 2316 | CONFIRMED (printed) | 1701-MS p.2 | Income-tax credits, separate from PT |
| W-6 | 1600-PT: "Monthly Remittance Return of Other Percentage Taxes Withheld"; government and private withholding agents | Form CONFIRMED. Due date (search snippet: 10th of following month) UNVERIFIED: eFPS guidance page returned a TLS error; guidelines PDF not fetched | `form1600pt/doc.txt` | — |
| W-7 | Double-count risk | INFERENCE: route by ATC series and return type, as the handover warns (line 133) | — | Keep ATC per line |

**3.6 Deadlines and TIN**

| # | Item | Status | Citation |
|---|---|---|---|
| D-1 | 2551Q: 25 days after quarter-end | CONFIRMED | RA 11976 §29; 2551Q guide line 53 |
| D-2 | 1701Q: quarters due May 15, Aug 15, Nov 15 of the current taxable year | CONFIRMED printed. A secondary eFPS help page reportedly says April 15 for Q1 (not read) | 1701Q guide Jan 2018, lines 40–42 |
| D-3 | 1601-C: 10th of following month; December withholding due January 15; eFPS per applicable issuances | CONFIRMED printed | 1601-C p.2 |
| D-4 | 1701-MS/1701/1701A: April 15 for prior year; 1701-MS 2nd installment October 15 | CONFIRMED | RMC 34-2025 Annex A; RMC 36-2026; 1701-MS item 26 |
| D-5 | TIN branch code: 3 → 5 digits "in all tax returns" | CONFIRMED (RMC 36-2026 item 3, image). Effective date not stated. The Jan 2018 printed notes already say "14-digit TIN" with last 5 digits as branch code (1601-C p.2; 2551Q guide). Inconsistent with a 3-digit field | RMC 36-2026 |
| D-6 | eBIRForms v7.9.6.0 bug-fixes include 2551Qv2018 | CONFIRMED | RMC 36-2026 item 5 |

### 4. Brief premises: confirmed, corrected, unverified

| Brief premise | Finding |
|---|---|
| 2551Q Jan 2018 is current | CONFIRMED (no newer edition found) |
| 1601-C Jan 2018 is current | CONFIRMED (no newer edition found) |
| 1701-MS Aug 2024 is current | CONFIRMED (named in RMC 36-2026) |
| 1701Q, 1701, 1701A editions | CONFIRMED Jan 2018 (ENCS) for 1701Q/1701; Jan 2018 for 1701A |
| Sec 116 3% → 1% (2020–23) → 3% | CONFIRMED |
| EOPT quarterly filing | CONFIRMED (§128(1)) |
| Monthly 2551M removed | NOT CONFIRMED. The statute reads quarterly only; no withdrawing issuance found |
| RR 3-2024 effects | MISSING; not identified |
| PT010 for 3% | CONFIRMED (return ATC). Government-withholding ATC for §116 persons is WB080, not PT |
| 2307 for PT withheld "e.g. government ATC PT..." | Corrected: WB080/WB082 (creditable), WB084 (final, 2306) |
| 1701-MS usable without COR per 2026 FAQ | UNVERIFIED (Annex C not retrievable) |
| 8% and ₱3M threshold; ₱250k | CONFIRMED for 2018 rules. Threshold is CPI-indexed under EOPT; lettering changed to §109(CC) |
| EOPT micro thresholds | Words CONFIRMED; lawphil numerals are off by 1,000 |
| Workbook 2551Q example: ₱300,000 less ₱10,000 return, 3% | The ₱10,000 deduction is not supported by printed instructions read (UNVERIFIED) |

### 5. Implications for Books/Forms

- **Tax profile, per taxable year:** store (a) RR 8-2024 tier and its history; (b) 8% election record (form 1901/1905/2551Q item 13/1701Q, quarter, date; irrevocable within year); (c) VAT-registration status and the date of any ₱3M breach; (d) GPP, BMBE and income-exemption flags; (e) count of activities and tax regimes (1701 vs 1701-MS routing); (f) withholding-agent category (government vs private) for WB080/WB082.
- **Three bases, not one:** (i) RR 8-2024 gross sales (net of VAT, business only); (ii) 8% test (receipts + other non-operating income vs VAT threshold, CPI-indexed); (iii) §116 PT base (quarter's gross receipts). Classification cutoffs are fixed amounts under RR 8-2024, so do not assume they track the indexed VAT threshold.
- **Percentage-tax gate:** `percentage-tax.ts` blocking 2551Q when `electedEightPercent` is true (handover line 138) is right for a valid 8% year without breach. For a breach year, RMO 23-2018 pt 9 imposes PT from the start of the year until VAT liability, so the block would hide a PT period. The mechanics of that period are UNVERIFIED.
- **Credits:** route WB-series 2307 lines to 2551Q item 15; WI/WC lines to income-tax credits (1701-MS Part V 3–5; 1701 items 59–60; 1701Q). Treat 2306 (WB084) as final, not credited.
- **Payroll (1601-C):** employer contributions never go in item 19. Carry an MWE flag for items 15–16. Cap item 17 at ₱90,000 per employee-year, but keep the cap configurable. Only de minimis (a)–(e) are verified; other amounts need a source. Item 23 needs the ₱250,000 non-MWE test.
- **Calendar:** 2551Q quarter-end + 25 days; 1701Q May 15 / Aug 15 / Nov 15; 1601-C 10th of following month (December → January 15); AITR April 15, 1701-MS second installment October 15; 2025 AITR extended to May 15, 2026.
- **TIN:** accept 14-digit TIN with 5-digit branch code, per RMC 36-2026 item 3 and the printed notes (INFERENCE: 9+5 structure).
- **Form versions for export:** 2551Q v2018; 1701 v2018; 1701A; 1701-MS (Aug 2024) via eBIRForms 7.9.6.0 only; 1601-C version label for eBIRForms UNKNOWN (package list stale).
- **Rounding:** 1701-MS whole-peso rule (49 centavos drop; 50+ round up). Compute centavos and round at the return level.

### 6. UNVERIFIED, MISSING, UNKNOWN

- **RR 8-2018 full text:** HTTP 403 on `BIR/pdf/RR%20No.%208-2018.pdf` and `local/pdf/RR%208-2018.pdf`. Only the digest was read.
- **RMC 20-2026 Annex C(3):** HTTP 403 on `BIR/pdf/RMC%20No.%2020-2026%20Annex%20C%20%283%29.pdf` and variants. Only the RMC 20-2026 digest (3 pp) was read. The full RMC 20-2026 text was not retrieved.
- **RMC 30-2026:** not retrieved (quoted only in RMC 36-2026).
- **RR 3-2024:** MISSING (searched by number and subject).
- **2551M status:** UNVERIFIED (searched for discontinuance 2024–26; none found).
- **1600-PT due date and guidelines:** UNVERIFIED (eFPS page TLS error; guidelines PDF not fetched).
- **1701Q, 1701, 1701A full line lists:** PARTIAL.
- **NIRC §32(B)(7)(f) and MWE OT/NSD/hazard legal basis; de minimis beyond (a)–(e); 1601-C item 23 legal basis:** UNVERIFIED.
- **VAT-threshold CPI adjustment published:** UNKNOWN.
- **Post-2024 changes to §116, §24(A)(2)(b), RR 8-2018:** MISSING (searched; RA 11976 text has no "8%" string). CREATE MORE Act: not checked.
- **BIR issuances after May 2026:** UNKNOWN. RMC 42-2026 (7 May 2026) seen in a search snippet only, not read.
- **RMC 37-2026 date:** printed 27 Apr 2026, receipt stamp 28 Apr 2026. Secondary sources give 27 or 28 Apr.
- **RR 8-2024 full text and RMO 23-2018:** read (CONFIRMED).
- **HTTP 403 list:** `RR%20No.%208-2024.pdf` (the working path is `RR%208-2024.pdf`); 2307 variants other than `v3`.

### 7. Local copies (for recheck)

Printed forms (read as images):
- 2551Q: `/Users/uriah/.claude/projects/-Volumes-goldcoders-OSS-buwiz-books/d723bb20-1674-49c1-8fb6-568f056d2c7e/tool-results/webfetch-1793245358945-r3gsy0.pdf`
- 1601-C: `/Users/uriah/.claude/projects/-Volumes-goldcoders-OSS-buwiz-books/d723bb20-1674-49c1-8fb6-568f056d2c7e/tool-results/webfetch-1793245358973-nfwd34.pdf`
- 1701-MS: `/Users/uriah/.claude/projects/-Volumes-goldcoders-OSS-buwiz-books/d723bb20-1674-49c1-8fb6-568f056d2c7e/tool-results/webfetch-1793245358957-cy02wi.pdf`

Downloads and extracted text: `/private/tmp/claude-501/-Volumes-goldcoders-OSS-buwiz-books/d723bb20-1674-49c1-8fb6-568f056d2c7e/scratchpad/bir-dl/` with subfolders `rmc34-2025` (`raw.txt`), `rmc36-2026`, `rmc37-2026`, `rmc20-2026-digest`, `rmo23-2018`, `digest-rr8-2018`, `rmc69-2023`, `rr11-2018`, `rr8-2024b`, `rmc26-2018`, `guide2551q`, `form2551qrev3`, `form2306`, `form2307b`, `form1701`, `form1701a`, `form1701q`, `guide1701q`, `form1600pt`, `ebir-list`. Statutes: `statutes/ra10963.clean.txt`, `statutes/ra11534.clean.txt`, `statutes/ra11976.norm2.txt`.