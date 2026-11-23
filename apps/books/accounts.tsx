// Accounts (Category Manager): ported 1:1 from the old web app's /accounts
// (CategoryManagerLayout + CategoryTree/CategoryRow + FilterPanel +
// CategoryDetail/CategoryDetailEmpty + the New/Edit Category form).
// Books has no parent/child, description, icon or update-account tool, so the
// tree is synthesized: each type's kind-"unsure" account named like the type
// (Assets, Liabilities, ...) is the root, and multi-account kinds become
// expandable parents (the lowest-numbered member is the parent row, matching
// the old app's "Bank Accounts", "Payroll Expenses", ... parents).

// ── Icons (icons.ts stroke SVGs as data URIs; Avatar tints them) ──
const acctSvg = (inner: string, width: number) =>
  `data:image/svg+xml,${encodeURIComponent(
    `<svg viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="${width}" stroke-linecap="round" stroke-linejoin="round">${inner}</svg>`,
  )}`;

// Per-type row icons (CategoryRow.typeIcons + the Filter by Type tiles).
const ACCT_TYPE_ICONS: Record<string, string> = {
  asset: acctSvg('<rect x="3" y="3" width="18" height="18" rx="2"/><path d="M3 9h18"/><path d="M9 21V9"/>', 2),
  liability: acctSvg('<path d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2"/><rect x="9" y="3" width="6" height="4" rx="1"/>', 2),
  equity: acctSvg('<path d="M3 3v18h18"/><path d="M7 16l4-8 4 5 5-6"/>', 2),
  revenue: acctSvg('<line x1="12" y1="1" x2="12" y2="23"/><path d="M17 5H9.5a3.5 3.5 0 000 7h5a3.5 3.5 0 010 7H6"/>', 2),
  expense: acctSvg('<rect x="2" y="5" width="20" height="14" rx="2"/><path d="M2 10h20"/>', 2),
  other_expense: acctSvg('<circle cx="12" cy="12" r="10"/><path d="M16 8l-8 8"/><path d="M8 8l8 8"/>', 2),
  other_income: acctSvg('<circle cx="12" cy="12" r="10"/><path d="M8 14s1.5 2 4 2 4-2 4-2"/><line x1="9" y1="9" x2="9.01" y2="9"/><line x1="15" y1="9" x2="15.01" y2="9"/>', 2),
  cost_of_revenue: acctSvg('<path d="M21 16V8a2 2 0 00-1-1.73l-7-4a2 2 0 00-2 0l-7 4A2 2 0 003 8v8a2 2 0 001 1.73l7 4a2 2 0 002 0l7-4A2 2 0 0021 16z"/>', 2),
};

const ACCT_ICONS = {
  branch: acctSvg('<path d="M3 3v10.2c0 1.68 0 2.52.327 3.162a3 3 0 001.311 1.311C5.28 18 6.12 18 7.8 18H15m0 0a3 3 0 106 0 3 3 0 00-6 0M3 8h12m0 0a3 3 0 106 0 3 3 0 00-6 0"/>', 2),
  pencil: acctSvg('<path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z"/>', 2),
  chevR: acctSvg('<polyline points="9 18 15 12 9 6"/>', 2),
  chevD: acctSvg('<polyline points="6 9 12 15 18 9"/>', 2),
  upRight: acctSvg('<line x1="7" y1="17" x2="17" y2="7"/><polyline points="7 7 17 7 17 17"/>', 2),
  funnel: acctSvg('<path d="M22 3H2l8 9.46V19l4 2v-8.54L22 3z"/>', 2),
  folder: acctSvg('<path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z"/>', 2),
  doc: acctSvg('<path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/><path d="M14 2v6h6"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/><line x1="10" y1="9" x2="8" y2="9"/>', 2),
  check: acctSvg('<polyline points="20 6 9 17 4 12"/>', 3),
  trash: acctSvg('<path d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/>', 2),
};

// typeColors (CategoryRow) — light/dark foreground + tile background.
// id, label, light fg, light bg, dark fg, dark bg
const ACCT_TYPES: [string, string, string, string, string, string][] = [
  ["asset", "Asset", "#2e7d32", "#e8f5e9", "#66bb6a", "#1b3a2a"],
  ["revenue", "Revenue", "#2e7d32", "#e8f5e9", "#66bb6a", "#1b3a2a"],
  ["other_income", "Other Income", "#00695c", "#e0f2f1", "#4db6ac", "#1a332f"],
  ["liability", "Liability", "#c62828", "#fce4ec", "#ef5350", "#3a1a24"],
  ["expense", "Operating Expenses", "#e65100", "#fff3e0", "#ff9800", "#3a2a1a"],
  ["other_expense", "Other Expenses", "#c62828", "#fce4ec", "#ef5350", "#3a1a24"],
  ["equity", "Equity", "#1565c0", "#e3f2fd", "#42a5f5", "#1a2a3d"],
  ["cost_of_revenue", "Cost of Revenue", "#e65100", "#fff3e0", "#ff9800", "#3a2a1a"],
];
const acctType = (id: string) => ACCT_TYPES.find((t) => t[0] === id) || ACCT_TYPES[0];
const acctTypeLabel = (id: string) => acctType(id)[1];

// Chip and drawer dots use the ACCT_TYPES tile palette (owner: sidebar colours
// match the main content), not the old FilterPanel TYPE_OPTIONS palette.

// Kind (subtype) labels — data/categories.v1.json id → name.
const ACCT_KIND_LABELS: Record<string, string> = {
  bank_accounts: "Bank Accounts", accounts_receivable_gross: "Accounts Receivable - Gross",
  inventory: "Inventory", prepaid_expenses: "Prepaid Expenses", computer_assets: "Computer Assets",
  accounts_payable: "Accounts Payable", owners_equity: "Owners Equity", owner_drawings: "Owner Drawings",
  loan_payable: "Loans Payable", sales_revenue: "Sales Revenue", subscription_revenue: "Subscription Revenue",
  transaction_revenue: "Transaction Revenue", refunds_and_discounts: "Refunds & Discounts",
  other_non_operating_income: "Other Non-Operating Income", interest_income: "Interest Income",
  cost_of_goods_sold: "Cost of Goods Sold", direct_service_cost: "Direct Cost of Services",
  salaries: "Salaries", employee_benefits: "Employee Benefits", retirement_contributions: "Retirement Contributions",
  employer_payroll_taxes: "Employer Payroll Taxes", payroll_processing_fees: "Payroll Processing Fees",
  payment_processing_fees: "Payment Processing Fees", hosting_fees: "Hosting Fees",
  consultants_and_contractors: "Consultants and Contractors", legal: "Legal",
  tax_and_accounting: "Tax & Accounting", advertising: "Advertising",
  business_applications_and_software: "Business Applications & Software", rent: "Rent",
  utilities: "Utilities", repairs_and_maintenance: "Repairs and Maintenance",
  office_supplies: "Office Supplies", airfare: "Airfare", meals: "Meals",
  bank_fees: "Bank Fees", interest_expense: "Interest Expense", depreciation_expense: "Depreciation Expense",
  ph_cwt_receivable: "Creditable Withholding Tax Receivable", pct_tax_credit_receivable: "Percentage Tax Withheld Receivable",
  ph_percentage_tax_payable: "Percentage Tax Payable", ph_percentage_tax_expense: "Percentage Tax Expense",
  income_tax_paid: "Income Tax Payments / Credits", income_tax_payable: "Income Tax Payable",
  ph_wtc_payable: "Withholding Tax on Compensation Payable", ph_sss_payable: "SSS Contributions Payable",
  ph_philhealth_payable: "PhilHealth Contributions Payable", ph_pagibig_payable: "Pag-IBIG Contributions Payable",
  ph_union_dues_payable: "Union Dues Payable", ph_net_pay_payable: "Net Pay Payable",
  ph_employee_tax_refund_payable: "Employee Tax Refund Payable", ph_ewt_payable: "Expanded Withholding Tax Payable",
  ph_input_vat: "Input VAT", ph_input_vat_unpaid: "Input VAT - Unpaid Payables",
  ph_output_vat: "Output VAT", ph_output_vat_uncollected: "Output VAT - Uncollected Receivables",
  ph_vat_payable_net: "VAT Payable - Net", ph_creditable_vat_withheld: "Creditable VAT Withheld",
  ph_input_vat_exempt_expense: "Input VAT Attributable to Exempt Sales", ph_employee_advances: "Employee Advances & Receivables",
  ph_employee_tax_advanced: "Employee Receivable - Tax Advanced", ph_unrecovered_employee_tax: "Unrecovered Employee Tax",
};
const acctKindLabel = (id: string) =>
  ACCT_KIND_LABELS[id] ||
  (id === "unsure" ? "Not sure" : id === "mixed" ? "Mixed" : String(id).split("_").map((w) => w.charAt(0).toUpperCase() + w.slice(1)).join(" "));

const acctP = (d: string) => acctSvg(`<path d="${d}"/>`, 1.5);
const ACCT_ICON_PATHS: Record<string, string> = {
  Accounting01: acctSvg("<g><path d=\"M3 7.8c0-1.6 0-2.5.3-3.1a3 3 0 0 1 1.3-1.3C5.2 3 6.1 3 7.8 3h8.4c1.6 0 2.5 0 3.1.3a3 3 0 0 1 1.3 1.3C21 5.2 21 6.1 21 7.8v8.4c0 1.6 0 2.5-.3 3.1a3 3 0 0 1-1.3 1.3C18.7 21 17.8 21 16.2 21H7.8c-1.6 0-2.5 0-3.1-.3a3 3 0 0 1-1.3-1.3C3 18.7 3 17.8 3 16.2z\"/><path d=\"M3 15.5h18M3 8.5h18M7 7v3M9 14v3M12 7v3M17 7v3M15 14v3\"/></g>", 1.5),
  Advertising: acctP(\"m17 3-5 4-5-4m12 8h.0M19 14h.0M19 17h.0M6.8 21h10.4c1.6 0 2.5 0 3.1-.3a3 3 0 0 0 1.3-1.3C22 18.7 22 17.8 22 16.2v-4.4c0-1.6 0-2.5-.3-3.1a3 3 0 0 0-1.3-1.3C19.7 7 18.8 7 17.2 7H6.8c-1.6 0-2.5 0-3.1.3a3 3 0 0 0-1.3 1.3C2 9.2 2 10.1 2 11.8v4.4c0 1.6 0 2.5.3 3.1a3 3 0 0 0 1.3 1.3C4.2 21 5.1 21 6.8 21m-.0-3h7.4c.6 0 .9 0 1.1-.1.2-.1.3-.3.4-.5.1-.2.1-.6.1-1.4v-3.7c0-.7 0-1.1-.1-1.4a1.2 1.2 0 0 0-.4-.5C15.1 10 14.8 10 14.2 10H6.7c-.6 0-.9 0-1.1.1a1.2 1.2 0 0 0-.4.5c-.1.2-.1.6-.1 1.4v3.7c0 .7 0 1.1.1 1.4.1.2.2.4.4.5.2.1.5.1 1.1.1\"),
  Apps: acctP(\"M20 16V7.2c0-1.1 0-1.6-.2-2.1a2 2 0 0 0-.8-.8C18.4 4 17.9 4 16.8 4H7.2c-1.1 0-1.6 0-2.1.2a2 2 0 0 0-.8.8C4 5.5 4 6.0 4 7.2V16m.6 4h14.6c.6 0 .9 0 1.1-.0a2 2 0 0 0 1.4-1.4c.0-.2.0-.5.0-1.1 0-.3 0-.4-.0-.5a1 1 0 0 0-.7-.7C21.1 16 20.9 16 20.6 16H3.3c-.3 0-.4 0-.5.0a1 1 0 0 0-.7.7C2 16.8 2 17.0 2 17.3c0 .6 0 .9.0 1.1a2 2 0 0 0 1.4 1.4c.2.0.5.0 1.1.0M9.8 12.9c-.9 0-1.8-.8-1.8-1.9 0-1.0.7-1.8 1.6-1.9.1-1.2 1.1-2.1 2.3-2.1s2.1.9 2.3 2.1C15.2 9.1 16 9.9 16 11c0 1.0-.8 1.9-1.8 1.9z\"),
  Bank: acctP(\"M5 9v8m4.5-8v8m5-8v8M19 9v8M3 18.6v.8c0 .5 0 .8.1 1.0a1 1 0 0 0 .4.4C3.7 21 4.0 21 4.6 21h14.8c.5 0 .8 0 1.0-.1a1 1 0 0 0 .4-.4C21 20.2 21 19.9 21 19.4v-.8c0-.5 0-.8-.1-1.0a1 1 0 0 0-.4-.4C20.2 17 19.9 17 19.4 17H4.6c-.5 0-.8 0-1.0.1a1 1 0 0 0-.4.4C3 17.7 3 18.0 3 18.6m8.6-15.5-7.4 1.6c-.4.0-.6.1-.8.2a1 1 0 0 0-.3.4C3 5.5 3 5.8 3 6.2V7.4c0 .5 0 .8.1 1.0a1 1 0 0 0 .4.4C3.7 9 4.0 9 4.6 9h14.8c.5 0 .8 0 1.0-.1a1 1 0 0 0 .4-.4C21 8.2 21 7.9 21 7.4V6.2c0-.4 0-.6-.0-.8a1 1 0 0 0-.3-.4c-.1-.1-.3-.1-.8-.2l-7.4-1.6a2 2 0 0 0-.2-.0 1 1 0 0 0-.1 0c-.0.0-.1.0-.2.0\"),
  BankNoteScissors01: acctSvg("<g><path d=\"M2 14.5V8.2c0-1.1 0-1.6.2-2.1a2 2 0 0 1 .8-.8C3.5 5 4.0 5 5.2 5h13.6c1.1 0 1.6 0 2.1.2a2 2 0 0 1 .8.8C22 6.5 22 7.0 22 8.2v7.6c0 1.1 0 1.6-.2 2.1a2 2 0 0 1-.8.8C20.4 19 19.9 19 18.8 19h-8.3M2.9 18.9l6.7-2.1m-2.6-2.7-2.2 6.7m-2.9-3.8a1.1 1.1 0 1 1 1.6 1.6 1.1 1.1 0 0 1-1.6-1.6m3.2 3.3a1.1 1.1 0 1 1 1.6 1.6 1.1 1.1 0 0 1-1.6-1.6\"/><path d=\"M13.5 9.7h-1.8a1.1 1.1 0 0 0 0 2.2h.7a1.1 1.1 0 0 1 0 2.2H10.5M12 9v.7m0 4.5V15\"/></g>", 1.5),
  CalendarDollar01: acctP(\"M21 8h-.0m0 0H3.0m17.9 0q.0.3.0.8v8.4c0 1.6 0 2.5-.3 3.1a3 3 0 0 1-1.3 1.3C18.7 22 17.8 22 16.2 22H7.8c-1.6 0-2.5 0-3.1-.3a3 3 0 0 1-1.3-1.3C3 19.7 3 18.8 3 17.2V8.8q0-.4.0-.8m17.9 0c-.0-1.1-.0-1.8-.3-2.3a3 3 0 0 0-1.3-1.3C18.7 4 17.8 4 16.2 4H16M3.0 8H3m.0 0c.0-1.1.0-1.8.3-2.3a3 3 0 0 1 1.3-1.3C5.2 4 6.1 4 7.8 4H8m0 0h8M8 4V2m0 2v1m8-1V2m0 2v1m-4 7.0h.9c.5 0 1.1.2 1.3.7M12 12.0h-.8c-.9 0-1.6.6-1.6 1.4 0 .8.7 1.4 1.6 1.4h1.6c.9 0 1.6.6 1.6 1.4 0 .8-.7 1.4-1.6 1.4H12m0-5.8V11m0 6.9h-.9c-.5 0-1.1-.2-1.3-.7m2.3.7V19\"),
  CoinsHand: acctP(\"M13.5 8.3a3.5 3.5 0 1 1 .9-4.7M6 20.0h2.6q.5 0 1.0.1l2.7.6c.5.1 1.2.1 1.8.0l3.0-.5a4.2 4.2 0 0 0 2.1-1.1l2.1-2.1a1.5 1.5 0 0 0 0-2.1 1.6 1.6 0 0 0-2.0-.1l-2.5 1.8c-.3.2-.7.4-1.2.4h-2.4 1.5c.8 0 1.5-.6 1.5-1.5v-.3c0-.7-.4-1.3-1.1-1.4l-2.3-.5A5 5 0 0 0 11.6 13c-.9 0-2.7.7-2.7.7L6 15.0M20 6.5a3.5 3.5 0 1 1-7 0 3.5 3.5 0 0 1 7 0M2 14.6v5.8c0 .5 0 .8.1 1.0a1 1 0 0 0 .4.4C2.7 22 3.0 22 3.6 22h.8c.5 0 .8 0 1.0-.1a1 1 0 0 0 .4-.4C6 21.2 6 20.9 6 20.4v-5.8c0-.5 0-.8-.1-1.0a1 1 0 0 0-.4-.4C5.2 13 4.9 13 4.4 13h-.8c-.5 0-.8 0-1.0.1a1 1 0 0 0-.4.4C2 13.7 2 14.0 2 14.6\"),
  CoinsHand02: acctP(\"M13.5 15.6a3.5 3.5 0 1 0 .9 4.7M6 3.9h2.6q.5 0 1.0-.1l2.7-.6a4.3 4.3 0 0 1 1.8-.0l3.0.5a4.2 4.2 0 0 1 2.1 1.1l2.1 2.1c.6.5.6 1.5 0 2.1a1.6 1.6 0 0 1-2.0.1l-2.5-1.8a2.1 2.1 0 0 0-1.2-.4h-2.4 1.5c.8 0 1.5.6 1.5 1.5v.3c0 .7-.4 1.3-1.1 1.4l-2.3.5a5 5 0 0 1-1.1.1c-.9 0-2.7-.7-2.7-.7L6 8.9M20 17.5a3.5 3.5 0 1 0-7 0 3.5 3.5 0 0 0 7 0M2 9.4V3.6c0-.5 0-.8.1-1.0a1 1 0 0 1 .4-.4C2.7 2 3.0 2 3.6 2h.8c.5 0 .8 0 1.0.1a1 1 0 0 1 .4.4C6 2.7 6 3.0 6 3.6v5.8c0 .5 0 .8-.1 1.0a1 1 0 0 1-.4.4C5.2 11 4.9 11 4.4 11h-.8c-.5 0-.8 0-1.0-.1a1 1 0 0 1-.4-.4C2 10.2 2 9.9 2 9.4\"),
  CoinsStacked03: acctP(\"M13 5c0 1.1-2.4 2-5.5 2S2 6.1 2 5m11 0c0-1.1-2.4-2-5.5-2S2 3.8 2 5m11 0v1.5M2 5v12c0 1.1 2.4 2 5.5 2m0-8q-.2 0-.5-.0C4.1 10.9 2 10.0 2 9m5.5 6C4.4 15 2 14.1 2 13m20-1.5c0 1.1-2.4 2-5.5 2s-5.5-.8-5.5-2m11 0c0-1.1-2.4-2-5.5-2s-5.5.8-5.5 2m11 0V19c0 1.1-2.4 2-5.5 2s-5.5-.8-5.5-2v-7.5m11 3.7c0 1.1-2.4 2-5.5 2s-5.5-.8-5.5-2\"),
  ComputerAsset: acctP(\"M20 6v8a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h12a2 2 0 0 1 2 2z M12 16v5 M8 21h8\"),
  Consultant: acctP(\"M16 3.4a4.5 4.5 0 0 1 0 8.0m2 5.2c1.5.6 2.8 1.7 4 3.2M2 20c1.9-2.4 4.5-4 7.5-4s5.5 1.5 7.5 4M14 7.5a4.5 4.5 0 1 1-9 0 4.5 4.5 0 0 1 9 0\"),
  DepreciationAndAmortization: acctP(\"m22 17-7.8-7.8c-.3-.3-.5-.5-.8-.6a1 1 0 0 0-.6 0c-.2.0-.4.2-.8.6L9.1 11.8c-.3.3-.5.5-.8.6a1 1 0 0 1-.6 0c-.2-.0-.4-.2-.8-.6L2 7m20 10h-7m7 0v-7\"),
  HostingFee: acctP(\"M4 16.2A4.5 4.5 0 0 1 6.0 8.0a6.0 6.0 0 0 1 11.8 0A4.5 4.5 0 0 1 20 16.2M8 16l4-4m0 0 4 4m-4-4v9\"),
  Inventory: acctP(\"M20.5 7.2 12 12m0 0L3.5 7.2M12 12v9.5m2-.6-1.2.6c-.2.1-.4.2-.5.2a1 1 0 0 1-.4 0c-.1-.0-.2-.1-.5-.2l-7.4-4.1c-.3-.1-.4-.2-.5-.3a1 1 0 0 1-.2-.3C3 16.5 3 16.4 3 16.0V7.9c0-.3 0-.5.0-.6a1 1 0 0 1 .2-.3c.1-.1.2-.2.5-.3l7.4-4.1c.2-.1.4-.2.5-.2a1 1 0 0 1 .4 0c.1.0.2.1.5.2l7.4 4.1c.3.1.4.2.5.3a1 1 0 0 1 .2.3c.0.1.0.3.0.6V12.5m-13.5-8 9 5M16 18l2 2 4-4\"),
  Iou: acctP(\"M5 9.5v5m10-5V13c0 1 1.2 1.4 2 1.5.7 0 2-.5 2-1.5V9.5M2 8.2v7.6c0 1.1 0 1.6.2 2.1a2 2 0 0 0 .8.8C3.5 19 4.0 19 5.2 19h13.6c1.1 0 1.6 0 2.1-.2a2 2 0 0 0 .8-.8C22 17.4 22 16.9 22 15.8V8.2c0-1.1 0-1.6-.2-2.1a2 2 0 0 0-.8-.8C20.4 5 19.9 5 18.8 5H5.2c-1.1 0-1.6 0-2.1.2a2 2 0 0 0-.8.8C2 6.5 2 7.0 2 8.2M12.5 12a2.5 2.5 0 1 1-5 0 2.5 2.5 0 0 1 5 0\"),
  Legal: acctP(\"M2.5 13h6m7 0h6M12 7v14m0-14a2.5 2.5 0 0 0 2.5-2.5M12 7a2.5 2.5 0 0 1-2.5-2.5M4 21h16M4 4.5h5.5m0 0a2.5 2.5 0 0 1 5 0m0 0H20M8.8 14.3C8.4 15.8 7.1 17 5.5 17s-2.9-1.1-3.3-2.6c-.0-.1-.0-.1-.0-.4a3 3 0 0 1 .1-.6c.0-.2.1-.3.3-.6L5.5 8l2.9 4.6c.1.2.2.4.3.6.0.1.1.5.1.6-.0.2-.0.3-.0.4m13 0C21.4 15.8 20.1 17 18.5 17s-2.9-1.1-3.3-2.6c-.0-.1-.0-.1-.0-.4-.0-.1.0-.5.1-.6.0-.2.1-.3.3-.6L18.5 8l2.9 4.6c.1.2.2.4.3.6.0.1.1.5.1.6-.0.2-.0.3-.0.4\"),
  Meals: acctSvg("<clipPath id=\"glass-fork-01_svg__a\"><path d=\"M0 0h24v24H0z\"/></clipPath><g clip-path=\"url(#glass-fork-01_svg__a)\"><path d=\"M4 21h3.5m0 0H11m-3.5 0v-6m0 0c3.7.0 4.4-2.7 4.5-9.5a.9.9 0 0 0-.9-.9H4.0a1 1 0 0 0-.9 1.0c.1 6.2.5 9.5 4.4 9.4M18 21v-9m0 0c2.8-.0 3-1 3-7.5M18 12c-3.1.0-2.9-1.9-3-7.5m3 7.5V4.5\"/></g>", 1.5),
  Opex: acctP(\"m14 22-2-2-2.7 2-2.5-2L4 22V6.8c0-1.6 0-2.5.3-3.1a3 3 0 0 1 1.3-1.3C6.2 2 7.1 2 8.8 2h6.4c1.6 0 2.5 0 3.1.3a3 3 0 0 1 1.3 1.3C20 4.2 20 5.1 20 6.8v4.7m1 3.5h-2m0 0h-.5a1.5 1.5 0 0 0 0 3h1a1.5 1.5 0 0 1 0 3H19m0-6v-1m0 7h-2m2 0v1M8 8h8m-8 4h5m-5 4h2\"),
  OtherIncome: acctSvg("<g><path d=\"M12 2a10 10 0 0 1 8.0 4.1L12 12z\"/><path d=\"M21.5 9c.2.9.4 1.9.4 3 0 5.5-4.4 10-10 10S2 17.5 2 12c0-4.4 2.9-8.2 7-9.5\"/></g>", 1.5),
  PaperClip: acctP(\"m21.1 10.9-9.0 9.0a5.2 5.2 0 0 1-7.4-7.4l9.0-9.0a3.5 3.5 0 0 1 4.9 4.9l-8.6 8.6a1.7 1.7 0 1 1-2.4-2.4l7.6-7.6\"),
  PaymentProcessingFee: acctP(\"M22 9H2m3.5 9h-.3c-1.1 0-1.6 0-2.1-.2a2 2 0 0 1-.8-.8C2 16.4 2 15.9 2 14.8V7.2c0-1.1 0-1.6.2-2.1a2 2 0 0 1 .8-.8C3.5 4 4.0 4 5.2 4h13.6c1.1 0 1.6 0 2.1.2a2 2 0 0 1 .8.8C22 5.5 22 6.0 22 7.2v7.6c0 1.1 0 1.6-.2 2.1a2 2 0 0 1-.8.8C20.4 18 19.9 18 18.8 18h-.3m-9.7-2.2L17 20m0-5-8.2 4.2M9.5 13a1.5 1.5 0 1 1 0 3 1.5 1.5 0 0 1 0-3m0 6a1.5 1.5 0 1 1 0 3 1.5 1.5 0 0 1 0-3\"),
  PayrollClearing: acctP(\"M6 13a3 3 0 1 0 0-6 3 3 0 0 0 0 6z M1 20a5 5 0 0 1 5-5h0a5 5 0 0 1 5 5 M14 8h7 M18 5l3 3-3 3 M21 16H14 M17 13l-3 3 3 3\"),
  Percent: acctP(\"M19 5 5 19M9 7a2 2 0 1 1-4 0 2 2 0 0 1 4 0m10 10a2 2 0 1 1-4 0 2 2 0 0 1 4 0\"),
  RefundAndDiscount: acctP(\"M9 9h.0M15 15h.0M16 8l-8 8m9.9-11.0c.2.4.6.8 1.1 1.1l1.7.7a2.0 2.0 0 0 1 1.1 2.6l-.7 1.7a2.0 2.0 0 0 0 0 1.5l.7 1.7a2.0 2.0 0 0 1-1.1 2.6L19 17.9A2.0 2.0 0 0 0 17.9 19l-.7 1.7a2.0 2.0 0 0 1-2.6 1.1l-1.7-.7a2.0 2.0 0 0 0-1.5 0l-1.7.7a2.0 2.0 0 0 1-2.6-1.1L6.1 19.0A2.0 2.0 0 0 0 5 17.9l-1.7-.7a2.0 2.0 0 0 1-1.1-2.6l.7-1.7a2.0 2.0 0 0 0 0-1.5l-.7-1.7a2.0 2.0 0 0 1 1.1-2.6L5 6.0c.4-.2.8-.6 1.1-1.0l.7-1.7a2.0 2.0 0 0 1 2.6-1.1l1.7.7a2.0 2.0 0 0 0 1.5-.0l1.7-.7a2.0 2.0 0 0 1 2.6 1.1l.7 1.7zM9.5 9a.5.5 0 1 1-1 0 .5.5 0 0 1 1 0m6 6a.5.5 0 1 1-1 0 .5.5 0 0 1 1 0\"),
  Rent: acctP(\"M6 20.0h2.6q.5 0 1.0.1l2.7.6c.5.1 1.2.1 1.8.0l3.0-.5a4.2 4.2 0 0 0 2.1-1.1l2.1-2.0a1.5 1.5 0 0 0 0-2.1 1.6 1.6 0 0 0-2.0-.1l-2.5 1.8c-.3.2-.7.4-1.2.4h-2.4 1.5c.8 0 1.5-.6 1.5-1.5v-.3c0-.7-.4-1.3-1.1-1.4l-2.3-.5A5 5 0 0 0 11.6 13c-.9 0-2.7.7-2.7.7L6 15.0m11.9-9.2.0.0M2 14.6v5.8c0 .5 0 .8.1 1.0a1 1 0 0 0 .4.4C2.7 22 3.0 22 3.6 22h.8c.5 0 .8 0 1.0-.1a1 1 0 0 0 .4-.4C6 21.2 6 20.9 6 20.4v-5.8c0-.5 0-.8-.1-1.0a1 1 0 0 0-.4-.4c-.2-.1-.4-.1-1.0-.1h-.8c-.5 0-.8 0-1.0.1a1 1 0 0 0-.4.4C2 13.7 2 14.0 2 14.6m14.6-5.8c1.7.7 3.7.0 4.4-1.5s-.0-3.6-1.8-4.3c-1.7-.7-3.7-.0-4.4 1.5q-.0.2-.1.4c-.0.2-.0.3-.1.4a.4.4 0 0 1-.1.1c-.0.0-.1.0-.3.1l-3.8 1.3a1.2 1.2 0 0 0-.2.1.5.5 0 0 0-.1.1c-.0.0-.0.1-.1.2l-.3.8c-.1.2-.1.4-.1.5a.5.5 0 0 0 .1.3c.0.1.2.1.5.3l.9.4c.1.0.1.0.2.0q.0.0.1.0c.0-.0.1-.0.2-.0l3.8-1.3c.1-.0.2-.0.3-.1a.5.5 0 0 1 .2.0c.0.0.1.0.4.2q.1.1.4.2\"),
  Repairs: acctP(\"M15.6 7.6c-.3-.3-.5-.5-.6-.8a1 1 0 0 1 0-.6c.0-.2.2-.4.6-.8L18.4 2.5a6 6 0 0 0-8.3 6.8c.1.4.1.7.1.8a.8.8 0 0 1-.1.3c-.0.1-.2.2-.4.5L3.5 17.5a2.1 2.1 0 0 0 3 3l6.2-6.2c.2-.2.4-.4.5-.4a.8.8 0 0 1 .3-.1c.1-.0.3.0.8.1q.6.1 1.4.1a6 6 0 0 0 5.4-8.4L18.6 8.3c-.3.3-.5.5-.8.6a1 1 0 0 1-.6 0c-.2-.0-.4-.2-.8-.6z\"),
  Retirement: acctP(\"M21 20s-1.1-.4-2-.6c-2.5-.7-4.7-.0-7 .6m-9 0s1.1.4 2 .6q.3.1.7.1M12.2 6c-3.0-.7-5.9-.2-9.7.5 5.5-6.2 8.0-4.5 10-.5 3.5-4.2 7.0-5.0 9.5.5-3.4-.9-6.3-1.3-9.7-.5m0 0v3.2m0-3.2c7.4 1.7 8.6 3.5 6.2 9-.9-3.0-1.9-4.5-4.4-5.3M12.2 6C5.7 8.0 3.1 10.0 6 15c1.0-2.8 2.1-4.5 4.5-5.3m1.7-.4a10 10 0 0 0-1.7.4m1.7-.4c.6.1 1.2.2 1.7.4m-3.5 0c.1 4.9-1.0 7.3-4.7 11.2m0 0C8 21.2 10 20.6 12 20m0 0c1.5-3.9 2.1-6.2 2.0-10.3\"),
  Salary: acctP(\"M14 15.5H7.5c-1.3 0-2.0 0-2.6.1a4 4 0 0 0-2.6 2.6C2 18.9 2 19.6 2 21m20-8.7h-2.5m0 0h-.6a1.8 1.8 0 0 0 0 3.7h1.2a1.8 1.8 0 0 1 0 3.7H19.5m0-7.5V11m0 8.7H17m2.5 0V21m-5-13.5a4.5 4.5 0 1 1-9 0 4.5 4.5 0 0 1 9 0\"),
  Shareholders: acctP(\"M6 20.0h2.6q.5 0 1.0.1l2.7.6c.5.1 1.2.1 1.8.0l3.0-.5a4.2 4.2 0 0 0 2.1-1.1l2.1-2.1a1.5 1.5 0 0 0 0-2.1 1.6 1.6 0 0 0-2.0-.1l-2.5 1.8c-.3.2-.7.4-1.2.4h-2.4 1.5c.8 0 1.5-.6 1.5-1.5v-.3c0-.7-.4-1.3-1.1-1.4l-2.3-.5A5 5 0 0 0 11.6 13c-.9 0-2.7.7-2.7.7L6 15.0M21 3.2h-2.5m0 0h-.6a1.8 1.8 0 0 0 0 3.7h1.2a1.8 1.8 0 0 1 0 3.7H18.5m0-7.5V2m0 8.7H16m2.5 0V12M2 14.6v5.8c0 .5 0 .8.1 1.0a1 1 0 0 0 .4.4C2.7 22 3.0 22 3.6 22h.8c.5 0 .8 0 1.0-.1a1 1 0 0 0 .4-.4C6 21.2 6 20.9 6 20.4v-5.8c0-.5 0-.8-.1-1.0a1 1 0 0 0-.4-.4C5.2 13 4.9 13 4.4 13h-.8c-.5 0-.8 0-1.0.1a1 1 0 0 0-.4.4C2 13.7 2 14.0 2 14.6\"),
  SubscriptionRevenue: acctP(\"M17 5.1A8.5 8.5 0 0 1 12 20.5h-.5M7 18.8A8.5 8.5 0 0 1 12 3.5h.5m.5 18.9-2-2 2-2M11 5.6l2-2-2-2\"),
  SvgTagDollar: acctP(\"M17.5 6.5h.0M14 9h-2m0 0h-.5a1.5 1.5 0 0 0 0 3h1a1.5 1.5 0 0 1 0 3H12m0-6V8m0 7h-2m2 0v1m9.7-10.8v4.4c0 .4 0 .7-.0.9a2 2 0 0 1-.2.5c-.1.2-.2.3-.6.7l-7.6 7.6c-1.1 1.1-1.7 1.7-2.4 2.0a3 3 0 0 1-1.8 0c-.6-.2-1.2-.8-2.4-2.0l-2.2-2.2c-1.1-1.1-1.7-1.7-2.0-2.4a3 3 0 0 1 0-1.8c.2-.6.8-1.2 2.0-2.4l7.6-7.6c.3-.3.5-.5.7-.6q.2-.1.5-.2c.2-.0.4-.0.9-.0h4.4c1.1 0 1.6 0 2.1.2a2 2 0 0 1 .8.8c.2.4.2.9.2 2.1M18 6.5a.5.5 0 1 1-1 0 .5.5 0 0 1 1 0\"),
  Transaction: acctSvg("<g><path d=\"M4 6.8c0-1.6 0-2.5.3-3.1a3 3 0 0 1 1.3-1.3C6.2 2 7.1 2 8.8 2h6.4c1.6 0 2.5 0 3.1.3a3 3 0 0 1 1.3 1.3C20 4.2 20 5.1 20 6.8V22l-2.7-2-2.5 2L12 20l-2.7 2-2.5-2L4 22zM8 8h8M8 12h5M8 16h2\"/></g>", 1.5),
  Travel: acctP(\"M17.7 2.8a2.4 2.4 0 0 1 3.5 3.4l-2.7 2.9c-.2.2-.3.3-.3.4a1 1 0 0 0-.1.3c-.0.1.0.3.0.6l1.7 7.4c.0.3.1.4.0.6a1 1 0 0 1-.1.3c-.0.1-.1.2-.4.4l-.3.3c-.6.6-.9.9-1.2.9a1 1 0 0 1-.7-.1c-.2-.1-.3-.5-.6-1.4l-2.0-6.0-3.3 3.3c-.2.2-.3.3-.3.4a1 1 0 0 0-.1.3c-.0.1-.0.2.0.5l.1 1.6c.0.2.0.4.0.5a1 1 0 0 1-.1.3c-.0.1-.1.2-.3.4l-.1.1c-.4.4-.7.7-.9.7a1 1 0 0 1-.6-.0c-.2-.1-.4-.3-.8-.9L6.1 18.5c-.0-.1-.1-.1-.1-.1a1 1 0 0 0-.1-.1c-.0-.0-.0-.0-.1-.1L3.3 16.5c-.5-.3-.8-.5-.9-.8a1 1 0 0 1-.0-.6c.0-.2.3-.4.7-.9l.1-.1c.2-.2.3-.3.4-.3a1 1 0 0 1 .3-.1c.1-.0.2-.0.5.0l1.6.1c.2.0.4.0.5.0a1 1 0 0 0 .3-.1c.1-.0.2-.1.4-.3l3.3-3.3-6.0-2.0c-.8-.2-1.2-.4-1.4-.6a1 1 0 0 1-.1-.7c.0-.3.3-.6.9-1.2l.3-.3c.2-.2.3-.3.4-.4a1 1 0 0 1 .3-.1c.1-.0.3.0.6.0l7.4 1.7c.3.0.4.1.6.0a1 1 0 0 0 .4-.1c.1-.0.2-.1.4-.4z\"),
  TrendUp01: acctP(\"m22 7-7.8 7.8c-.3.3-.5.5-.8.6a1 1 0 0 1-.6 0c-.2-.0-.4-.2-.8-.6L9.1 12.1c-.3-.3-.5-.5-.8-.6a1 1 0 0 0-.6 0c-.2.0-.4.2-.8.6L2 17M22 7h-7m7 0v7\"),
  UserCoins01: acctP(\"M12 15.5H7.5c-1.3 0-2.0 0-2.6.1a4 4 0 0 0-2.6 2.6C2 18.9 2 19.6 2 21m20-4.7c0 .6-1.5 1.2-3.5 1.2S15 16.8 15 16.2m7 0C22 15.5 20.4 15 18.5 15s-3.5.5-3.5 1.2m7 0v4.5C22 21.4 20.4 22 18.5 22s-3.5-.5-3.5-1.2v-4.5m7 2.2c0 .6-1.5 1.2-3.5 1.2S15 19.1 15 18.5m-.5-11a4.5 4.5 0 1 1-9 0 4.5 4.5 0 0 1 9 0\"),
  Utilities: acctSvg("<g><path d=\"m2 9.5 9.0-6.7c.3-.2.5-.3.7-.4a1 1 0 0 1 .5 0c.1.0.3.1.7.4L22 9.5M4 8v9.8c0 1.1 0 1.6.2 2.1a2 2 0 0 0 .8.8C5.5 21 6.0 21 7.2 21h9.6c1.1 0 1.6 0 2.1-.2a2 2 0 0 0 .8-.8C20 19.4 20 18.9 20 17.8V8l-6.0-4.5c-.6-.5-1.0-.7-1.4-.8a2 2 0 0 0-1.0 0c-.3.1-.7.3-1.4.8z\"/><path d=\"M12.6 9 10 13h4l-2.6 4\"/></g>", 1.5),
};
const ACCT_KIND_ICON: Record<string, string> = {
  accounts_payable: 'CoinsHand02',
  accounts_receivable_gross: 'CoinsHand',
  advertising: 'Advertising',
  airfare: 'Travel',
  bank_accounts: 'Bank',
  bank_fees: 'BankNoteScissors01',
  business_applications_and_software: 'Apps',
  computer_assets: 'ComputerAsset',
  consultants_and_contractors: 'Consultant',
  cost_of_goods_sold: 'BankNoteScissors01',
  depreciation_expense: 'DepreciationAndAmortization',
  direct_service_cost: 'Opex',
  employee_benefits: 'UserCoins01',
  employer_payroll_taxes: 'UserCoins01',
  hosting_fees: 'HostingFee',
  income_tax_paid: 'CoinsStacked03',
  income_tax_payable: 'CoinsStacked03',
  interest_expense: 'Percent',
  interest_income: 'TrendUp01',
  inventory: 'Inventory',
  legal: 'Legal',
  loan_payable: 'Iou',
  meals: 'Meals',
  office_supplies: 'PaperClip',
  other_non_operating_income: 'OtherIncome',
  owner_drawings: 'Shareholders',
  owners_equity: 'Shareholders',
  payment_processing_fees: 'PaymentProcessingFee',
  payroll_processing_fees: 'PayrollClearing',
  pct_tax_credit_receivable: 'Percent',
  ph_creditable_vat_withheld: 'Percent',
  ph_cwt_receivable: 'Percent',
  ph_employee_advances: 'UserCoins01',
  ph_employee_tax_advanced: 'UserCoins01',
  ph_employee_tax_refund_payable: 'RefundAndDiscount',
  ph_ewt_payable: 'Percent',
  ph_input_vat: 'Percent',
  ph_input_vat_exempt_expense: 'Percent',
  ph_input_vat_unpaid: 'Percent',
  ph_net_pay_payable: 'PayrollClearing',
  ph_output_vat: 'Percent',
  ph_output_vat_uncollected: 'Percent',
  ph_pagibig_payable: 'UserCoins01',
  ph_percentage_tax_expense: 'Percent',
  ph_percentage_tax_payable: 'Percent',
  ph_philhealth_payable: 'UserCoins01',
  ph_sss_payable: 'UserCoins01',
  ph_union_dues_payable: 'UserCoins01',
  ph_unrecovered_employee_tax: 'UserCoins01',
  ph_vat_payable_net: 'Percent',
  ph_wtc_payable: 'UserCoins01',
  prepaid_expenses: 'CalendarDollar01',
  refunds_and_discounts: 'RefundAndDiscount',
  rent: 'Rent',
  repairs_and_maintenance: 'Repairs',
  retirement_contributions: 'Retirement',
  salaries: 'Salary',
  sales_revenue: 'SvgTagDollar',
  subscription_revenue: 'SubscriptionRevenue',
  tax_and_accounting: 'Accounting01',
  transaction_revenue: 'Transaction',
  utilities: 'Utilities',
};

// The names of the type-root accounts in a chart like the seeded one.
const ACCT_ROOT_NAMES: Record<string, RegExp> = {
  asset: /^assets?$/i, liability: /^liabilit/i, equity: /^equity$/i, revenue: /^revenue$/i,
  cost_of_revenue: /^cost of revenue/i, expense: /expenses/i, other_income: /other income/i,
  other_expense: /other expense/i,
};

/** The account whose activity drawer is open (Accounts page). */
const accountDrawer = signal("");

interface AcctNode {
  a: Any;
  level: number;
  kids: AcctNode[];
  kind: string;
}

function AccountsPage(p: { go: (page: string, filter?: string) => void }) {
  const types = signal([] as string[]);
  const statusSel = signal(["active"] as string[]);
  const kindsSel = signal([] as string[]);
  const find = signal("");
  const selected = signal(null as Any);
  const mode = signal(""); // "" | "create" | "edit"
  const parentKind = signal("unsure");
  const catMenu = signal(false);
  const filterPop = signal(false);
  const mFilter = signal(false);
  const ncName = signal("");
  const ncNumber = signal("");
  const ncType = signal("asset");
  const ncKind = signal("unsure");
  const ncBusy = signal(false);
  const collapsed = signal({} as Record<string, boolean>);
  const openSections = signal({ status: true, type: true, kind: true } as Record<string, boolean>);
  const kindMenu = signal(false);
  let catMenuAnchor: ViewRef | undefined;
  let filterAnchor: ViewRef | undefined;
  let kindAnchor: ViewRef | undefined;

  const data = resource(
    () => ({ v: ver() }),
    () => buwiz.call("books_accounts_list", { group: "all" }),
  );
  const d = () => data() as Any;
  const allRows = () => (d()?.rows || []) as Any[];
  const bookKinds = () => {
    const seen = new Set<string>();
    for (const a of allRows()) {
      const k = String(a.kind || "unsure");
      if (k !== "unsure" && k !== "mixed") seen.add(k);
    }
    return Array.from(seen).sort((x, y) => acctKindLabel(x).localeCompare(acctKindLabel(y)));
  };

  const filtered = () => {
    const q = find().trim().toLowerCase();
    let rows = allRows();
    if (statusSel().includes("active") && !statusSel().includes("deactivated"))
      rows = rows.filter((a: Any) => !a.off);
    if (statusSel().includes("deactivated") && !statusSel().includes("active"))
      rows = rows.filter((a: Any) => a.off);
    if (types().length) rows = rows.filter((a: Any) => types().includes(String(a.type)));
    if (kindsSel().length) rows = rows.filter((a: Any) => kindsSel().includes(String(a.kind)));
    if (q)
      rows = rows.filter((a: Any) =>
        `${a.number} ${a.name} ${a.tag_label || ""} ${acctKindLabel(String(a.kind || ""))}`
          .toLowerCase()
          .includes(q),
      );
    return rows;
  };

  /** The displayed tree: type roots → kind groups → accounts (see header). */
  const tree = (): AcctNode[] => {
    const rows = filtered().slice().sort((x: Any, y: Any) => String(x.number).localeCompare(String(y.number)));
    const roots: AcctNode[] = [];
    const used = new Set<string>();
    const num = (a: Any) => parseInt(String(a.number || "0"), 10) || 0;
    for (const [type, nameRe] of Object.entries(ACCT_ROOT_NAMES)) {
      const ofType = rows.filter((a: Any) => String(a.type) === type);
      const root = ofType
        .filter((a: Any) => (a.kind === "unsure" || a.kind === "mixed") && nameRe.test(String(a.name)))
        .sort((x: Any, y: Any) => num(x) - num(y))[0];
      if (!root) continue;
      used.add(String(root.ticket));
      const kids: AcctNode[] = [];
      const byKind = new Map<string, Any[]>();
      for (const a of ofType) {
        if (String(a.ticket) === String(root.ticket)) continue;
        const k = String(a.kind || "unsure");
        if (k === "unsure" || k === "mixed") {
          kids.push({ a, level: 1, kids: [], kind: "" });
          continue;
        }
        if (!byKind.has(k)) byKind.set(k, []);
        byKind.get(k)!.push(a);
      }
      for (const [k, members] of byKind) {
        members.sort((x: Any, y: Any) => num(x) - num(y));
        if (members.length > 1) {
          const [head, ...rest] = members;
          kids.push({ a: head, level: 1, kids: rest.map((c) => ({ a: c, level: 2, kids: [], kind: "" })), kind: k });
        } else {
          kids.push({ a: members[0], level: 1, kids: [], kind: "" });
        }
      }
      kids.sort((x, y) => num(x.a) - num(y.a));
      roots.push({ a: root, level: 0, kids, kind: "root" });
    }
    // Types with no matching root: kind groups at level 0.
    const orphans = rows.filter((a: Any) => !used.has(String(a.ticket)) && !roots.some((r) => r.kids.some((k) => k.a.ticket === a.ticket || k.kids.some((c) => c.a.ticket === a.ticket))));
    const orphanKinds = new Map<string, Any[]>();
    for (const a of orphans) {
      const k = String(a.kind || "unsure");
      if (!orphanKinds.has(k)) orphanKinds.set(k, []);
      orphanKinds.get(k)!.push(a);
    }
    for (const [k, members] of orphanKinds) {
      members.sort((x: Any, y: Any) => num(x) - num(y));
      if (members.length > 1 && k !== "unsure" && k !== "mixed") {
        const [head, ...rest] = members;
        roots.push({ a: head, level: 0, kids: rest.map((c) => ({ a: c, level: 1, kids: [], kind: "" })), kind: k });
      } else {
        for (const a of members) roots.push({ a, level: 0, kids: [], kind: "" });
      }
    }
    roots.sort((x, y) => num(x.a) - num(y.a));
    return roots;
  };

  const flatTree = (): AcctNode[] => {
    const out: AcctNode[] = [];
    const walk = (n: AcctNode) => {
      out.push(n);
      const open = collapsed()[String(n.a.ticket)] === undefined ? n.level === 0 : !collapsed()[String(n.a.ticket)];
      if (open) n.kids.forEach(walk);
    };
    tree().forEach(walk);
    return out;
  };

  const toggleKind = (k: string) =>
    kindsSel.set(kindsSel().includes(k) ? kindsSel().filter((x) => x !== k) : [...kindsSel(), k]);
  const toggleType = (t: string) =>
    types.set(types().includes(t) ? types().filter((x) => x !== t) : [...types(), t]);
  const toggleStatus = (s: string) => {
    // Radio-style like the reference: picking the active status clears it.
    if (statusSel().length === 1 && statusSel()[0] === s) statusSel.set([]);
    else statusSel.set([s]);
  };
  const clearFilters = () => {
    types.set([]);
    statusSel.set(["active"]);
    kindsSel.set([]);
    find.set("");
  };
  const hasFilters = () =>
    types().length > 0 || kindsSel().length > 0 || find().trim().length > 0 || !(statusSel().length === 1 && statusSel()[0] === "active");

  const openCreate = (kind?: string, type?: string) => {
    mode.set("create");
    ncName.set("");
    ncNumber.set("");
    ncType.set(type || "asset");
    ncKind.set(kind || "unsure");
    parentKind.set(kind || "unsure");
    mFilter.set(true); // opens the side sheet below lg
  };
  const openEdit = (a: Any) => {
    mode.set("edit");
    ncName.set(String(a.name || ""));
    ncNumber.set(String(a.number || ""));
    ncType.set(String(a.type || "asset"));
    ncKind.set(String(a.kind || "unsure"));
    mFilter.set(true);
  };
  const closeSide = () => {
    selected.set(null);
    mode.set("");
    mFilter.set(false);
  };

  const nextNumber = () => {
    let hi = 0;
    for (const a of allRows()) {
      const n = parseInt(String(a.number || "0"), 10);
      if (!isNaN(n) && n > hi) hi = n;
    }
    return String(hi + 10).slice(0, 12);
  };
  const createCategory = async () => {
    if (ncBusy() || !ncName().trim()) return;
    ncBusy.set(true);
    const name = ncName().trim();
    const number = ncNumber().trim() || nextNumber();
    try {
      // A catalog kind fixes the account type: omit `type` and the server
      // derives it, so a kind can never save with a stale type. `type` is
      // only the user's pick for unsure/mixed kinds (PR37 finding 3).
      const args: Any = { number, name, kind: ncKind() };
      if (ncKind() === "unsure" || ncKind() === "mixed") args.type = ncType();
      const r = await act("books_app_account_create", args);
      const acct = r?.account || {};
      if (acct.ticket) selected.set({ ...acct, name, number, type: ncType() });
      mode.set("");
      ncName.set("");
      ncNumber.set("");
      ncKind.set("unsure");
    } catch {
      // act() already toasts the refusal.
    } finally {
      ncBusy.set(false);
    }
  };

  const chip = (label: string, dot: string, onX: () => void) => (
    <View class="flex-row items-center gap-1.5 px-2.5 py-0.5 rounded-[20px] border border-slate-300 dark:border-slate-600 bg-slate-50 dark:bg-slate-700">
      <View class="w-2 h-2 rounded-full" style={{ backgroundColor: dot }} />
      <Text class="text-[12px] font-semibold text-slate-700 dark:text-slate-200">{label}</Text>
      <View class="px-0.5" onPress={onX} activeStyle={{ opacity: 0.6 }}>
        <Text class="text-[12px] font-bold text-slate-400 hover:text-slate-600 dark:hover:text-slate-200">×</Text>
      </View>
    </View>
  );

  const checkRow = (label: string, dot: string, on: boolean, onPress: () => void) => (
    <View
      class="flex-row items-center gap-2.5 px-2 py-1.5 rounded-md transition-colors hover:bg-gray-50 dark:hover:bg-white/5"
      onPress={onPress}
      activeStyle={{ opacity: 0.7 }}
    >
      <View class={`w-[18px] h-[18px] rounded-[4px] border items-center justify-center ${on ? "bg-[#2a9d8f] border-[#2a9d8f]" : "border-[#d1d5db] dark:border-slate-500"}`}>
        <If when={on}>
          <Avatar src={ACCT_ICONS.check} class="w-3 h-3" imageClass="tint-white" fallbackDelay={Infinity} accessibilityLabel="" />
        </If>
      </View>
      <If when={Boolean(dot)}>
        <View class="w-2.5 h-2.5 rounded-[3px]" style={{ backgroundColor: dot }} />
      </If>
      <Text class={`flex-1 text-[13px] ${on ? `font-semibold ${C.ink}` : C.mute}`}>{label}</Text>
    </View>
  );

  const section = (key: string, title: string, count: number, body: Any) => (
    <View class="flex-col">
      <View
        class="flex-row items-center justify-between px-2 py-2"
        onPress={() => openSections.set({ ...openSections(), [key]: !openSections()[key] })}
        activeStyle={{ opacity: 0.7 }}
      >
        <Text class={`text-[11px] font-semibold uppercase tracking-wider ${C.dim}`}>{title}</Text>
        <View class="flex-row items-center gap-1.5">
          <If when={count > 0}>
            <View class="min-w-[18px] h-[18px] px-1 rounded-full bg-[#2a9d8f] items-center justify-center">
              <Text class="text-[10px] font-bold text-white">{String(count)}</Text>
            </View>
          </If>
          <Avatar
            src={ACCT_ICONS.chevD}
            class={`w-3.5 h-3.5 transition-transform ${openSections()[key] ? "" : "-rotate-90"}`}
            imageClass="tint-slate-400"
            fallbackDelay={Infinity}
            accessibilityLabel=""
          />
        </View>
      </View>
      <If when={Boolean(openSections()[key])}>{body}</If>
    </View>
  );

  const iconBtn = (a: Any, icon: string, tip: string, onPress: () => void, extra = "") => (
    <View
      class={`w-9 h-9 rounded-full items-center justify-center transition-colors hover:bg-slate-200/60 dark:hover:bg-slate-600 ${extra || "opacity-0 group-hover:opacity-100"}`}
      onPress={(e: Any) => {
        e?.stopPropagation?.();
        onPress();
      }}
      activeStyle={{ opacity: 0.6 }}
      title={tip}
    >
      <Avatar
        src={icon}
        class="w-4 h-4"
        imageClass="tint-slate-500 dark:tint-slate-300 group-hover:tint-slate-700 dark:group-hover:tint-white"
        fallbackDelay={Infinity}
        accessibilityLabel={tip}
      />
    </View>
  );

  const catRow = (n: AcctNode) => {
    const a = n.a;
    const t = acctType(String(a.type));
    const isSel = () => selected()?.ticket === a.ticket;
    // Roots with children start expanded; deeper parents start collapsed
    // (CategoryTree.tsx:114-122). `collapsed` only stores user toggles.
    const expanded = () => collapsed()[String(a.ticket)] === undefined ? n.level === 0 : !collapsed()[String(a.ticket)];
    const hasKids = n.kids.length > 0;
    return (
      <View
        class={`group grid grid-cols-12 items-center py-3 px-4 transition-colors border-b border-[#f0f1f3] dark:border-slate-700 ${
          isSel() ? "bg-[#e8f4fe] dark:bg-slate-700" : "hover:bg-[#f8f9fb] dark:hover:bg-slate-800"
        }`}
        onPress={() => {
          selected.set(isSel() ? null : a);
          mode.set("");
        }}
        activeStyle={{ opacity: 0.85 }}
      >
        {/* Category — icon box, name, number, sub-line */}
        <View class="col-span-9 sm:col-span-6 flex-row items-center gap-3 min-w-0" style={{ paddingLeft: 16 + n.level * 24 }}>
          <View
            class={`w-9 h-9 rounded-lg items-center justify-center shrink-0 ${hasKids ? "" : ""}`}
            style={{ backgroundColor: t[3] }}
            onPress={(e: Any) => {
              if (!hasKids) return;
              e?.stopPropagation?.();
              collapsed.set({ ...collapsed(), [String(a.ticket)]: expanded() });
            }}
          >
            {/* expanded → icon, chevron-down on row hover; collapsed → chevron-right */}
            <Avatar
              src={ACCT_ICON_PATHS[ACCT_KIND_ICON[String(a.kind || "unsure")]] || ACCT_TYPE_ICONS[String(a.type)] || ACCT_TYPE_ICONS.asset}
              class={`w-[18px] h-[18px] ${hasKids ? (expanded() ? "group-hover:hidden" : "hidden") : ""}`}
              imageClass={`tint-[${t[2]}] dark:tint-[${t[4]}]`}
              fallbackDelay={Infinity}
              accessibilityLabel=""
            />
            <If when={hasKids}>
              <Avatar
                src={expanded() ? ACCT_ICONS.chevD : ACCT_ICONS.chevR}
                class={`w-4 h-4 absolute ${expanded() ? "hidden group-hover:flex" : "flex"}`}
                imageClass={`tint-[${t[2]}] dark:tint-[${t[4]}]`}
                fallbackDelay={Infinity}
                accessibilityLabel=""
              />
            </If>
          </View>
          <View class="flex-1 min-w-0">
            <View class="flex-row items-center gap-2 min-w-0">
              <Text class={`text-[15px] font-semibold truncate ${a.off ? C.dim : C.ink}`}>{a.name}</Text>
              <If when={Boolean(a.number)}>
                <Text class={`text-[13px] ${C.dim} shrink-0`}>{a.number}</Text>
              </If>
            </View>
            <Text class={`text-[13px] truncate ${a.off ? C.dim : C.mute}`}>
              {a.off ? "Accrual books only" : `${a.tag_label || acctTypeLabel(String(a.type))} · ${acctKindLabel(String(a.kind || "unsure"))}`}
            </Text>
          </View>
        </View>
        {/* Connections — Books has no financial connections; column kept for layout parity */}
        <View class="hidden sm:col-span-3 sm:flex justify-center" />
        {/* Actions — 3 icon buttons revealed on row hover */}
        <View class="col-span-3 flex-row justify-end items-center gap-1">
          {iconBtn(a, ACCT_ICONS.branch, "Add child category", () => openCreate(String(a.kind || "unsure"), String(a.type)))}
          {iconBtn(a, ACCT_ICONS.pencil, "Edit category", () => openEdit(a))}
          {iconBtn(
            a,
            isSel() ? ACCT_ICONS.upRight : ACCT_ICONS.chevR,
            "Open category",
            () => {
              catDetailTicket.set(String(a.ticket));
              p.go("category");
            },
            "opacity-0 group-hover:opacity-70",
          )}
        </View>
      </View>
    );
  };

  // ── Side panel bodies ──
  const filterTiles = () => (
    <View class="flex-1 flex-col">
      <View class="shrink-0 bg-[#2a9d8f] px-5 py-4 flex-row items-center">
        <Text class="text-[15px] font-semibold text-white">Filter by Type</Text>
      </View>
      <View class="p-6 grid grid-cols-3 gap-4">
        <Each of={ACCT_TYPES}>
          {(t: Any) => {
            const on = () => types().includes(t()[0]);
            return (
              <View
                class="flex-col items-center gap-2"
                onPress={() => toggleType(t()[0])}
                activeStyle={{ opacity: 0.7 }}
              >
                <View
                  class="w-14 h-14 rounded-xl items-center justify-center transition-transform duration-150"
                  style={{
                    backgroundColor: on() ? t()[3] : "#f0f2f5",
                    borderColor: on() ? t()[2] : "transparent",
                    borderWidth: 2.5,
                    borderStyle: "solid",
                    boxShadow: on() ? `0 2px 8px ${t()[2]}33` : "none",
                  }}
                  hoverStyle={{
                    scale: 1.08,
                    boxShadow: on() ? `0 2px 8px ${t()[2]}33` : "0 4px 12px rgba(0,0,0,0.1)",
                  }}
                >
                  <Avatar
                    src={ACCT_TYPE_ICONS[t()[0]]}
                    class="w-7 h-7"
                    imageClass={on() ? `tint-[${t()[2]}]` : "tint-[#9ca3af]"}
                    fallbackDelay={Infinity}
                    accessibilityLabel={t()[1]}
                  />
                </View>
                <Text
                  class="text-[0.7rem] text-center leading-tight"
                  style={{ color: on() ? t()[2] : "#6b7c93", fontWeight: on() ? "700" : "500" }}
                >
                  {t()[1]}
                </Text>
              </View>
            );
          }}
        </Each>
      </View>
    </View>
  );

  const detailCard = () => {
    const a = selected();
    if (!a) return <View />;
    const t = acctType(String(a.type));
    return (
      <View class="flex-1 flex-col">
        <View class="shrink-0 bg-[#2a9d8f] items-center pt-5 pb-3">
          <View class="w-10 h-10 rounded-lg bg-gray-100/80 dark:bg-transparent items-center justify-center">
            <Avatar src={ACCT_ICONS.doc} class="w-[18px] h-[18px]" imageClass="tint-[#6b7c93] dark:tint-white" fallbackDelay={Infinity} accessibilityLabel="" />
          </View>
        </View>
        <View class="px-5 pt-6 pb-4">
          <View class="absolute top-3 right-5 flex-row items-center gap-1">
            <View
              class="w-7 h-7 rounded-full items-center justify-center transition-colors hover:bg-[rgba(50,73,127,0.08)] dark:hover:bg-slate-700"
              onPress={() => openCreate(String(a.kind || "unsure"), String(a.type))}
              activeStyle={{ opacity: 0.7 }}
              title="Add child category"
            >
              <Avatar src={ACCT_ICONS.branch} class="w-3.5 h-3.5" imageClass="tint-[#6b7c93] dark:tint-slate-400" fallbackDelay={Infinity} accessibilityLabel="Add child" />
            </View>
            <View
              class="w-7 h-7 rounded-full items-center justify-center transition-colors hover:bg-[rgba(50,73,127,0.08)] dark:hover:bg-slate-700"
              onPress={() => openEdit(a)}
              activeStyle={{ opacity: 0.7 }}
              title="Edit"
            >
              <Avatar src={ACCT_ICONS.pencil} class="w-3.5 h-3.5" imageClass="tint-[#6b7c93] dark:tint-slate-400" fallbackDelay={Infinity} accessibilityLabel="Edit" />
            </View>
          </View>
          <View class="flex-row items-start gap-3 pr-16">
            <View class="w-10 h-10 rounded-lg items-center justify-center shrink-0" style={{ backgroundColor: t[3] }}>
              <Avatar src={ACCT_ICON_PATHS[ACCT_KIND_ICON[String(a.kind || "unsure")]] || ACCT_TYPE_ICONS[String(a.type)] || ACCT_TYPE_ICONS.asset} class="w-6 h-6" imageClass={`tint-[${t[2]}] dark:tint-[${t[4]}]`} fallbackDelay={Infinity} accessibilityLabel="" />
            </View>
            <View class="flex-1 min-w-0">
              <Text class="text-[18px] font-bold text-[#32497f] dark:text-slate-200 leading-snug">{a.name}</Text>
              <If when={Boolean(a.number)}>
                <Text class={`text-[12px] ${C.dim} mt-0.5`}>{a.number}</Text>
              </If>
            </View>
          </View>
          <Text class="text-[13px] text-[#6b7c93] mt-3 leading-relaxed">
            {`${a.tag_label || acctTypeLabel(String(a.type))} · ${acctKindLabel(String(a.kind || "unsure"))}`}
          </Text>
        </View>
        <View class="mx-4 mb-4 bg-[#f5f7fb] dark:bg-slate-800 rounded-xl p-4 flex-1">
          <View class="flex-row items-center gap-2">
            <Avatar src={ACCT_ICON_PATHS[ACCT_KIND_ICON[String(a.kind || "unsure")]] || ACCT_TYPE_ICONS[String(a.type)] || ACCT_TYPE_ICONS.asset} class="w-4 h-4" imageClass={`tint-[${t[2]}] dark:tint-[${t[4]}]`} fallbackDelay={Infinity} accessibilityLabel="" />
            <Text class={`text-[13px] font-semibold ${C.ink}`}>{a.name}</Text>
          </View>
          <View class="gap-1 mt-3">
            <Text class={`text-[11px] font-semibold uppercase tracking-wider ${C.dim}`}>Current Balance</Text>
            <Text class={`text-[24px] font-semibold font-mono ${C.ink}`}>{a.balance_shown || "0.00"}</Text>
          </View>
          <If when={!a.off}>
            <View class="gap-2 pt-3">
              <View
                class="h-[38px] rounded-[10px] bg-[#0d9488] items-center justify-center transition-colors hover:bg-[#0f766e]"
                onPress={() => accountDrawer.set(String(a.ticket || ""))}
                activeStyle={{ opacity: 0.85 }}
              >
                <Text class="text-[13px] font-semibold text-white">View Monthly Activity</Text>
              </View>
              <Btn label="Open in Ledger" onPress={() => openLedger({ account: String(a.ticket || "") })} />
            </View>
          </If>
        </View>
      </View>
    );
  };

  const kindField = () => (
    <View class="gap-1.5">
      <Text class={`text-[11px] font-medium ${C.mute}`}>Sub-type</Text>
      <View
        ref={kindAnchor}
        class={`flex-row items-center justify-between h-[38px] rounded-lg border px-3 transition-colors ${C.line} ${mode() === "edit" ? "" : "hover:border-[#cbd5e1] dark:hover:border-[#3f3f46]"}`}
        onPress={() => (mode() === "edit" ? null : kindMenu.set(true))}
        activeStyle={mode() === "edit" ? undefined : { opacity: 0.7 }}
      >
        <Text class={`text-[13px] ${C.ink}`}>{acctKindLabel(ncKind())}</Text>
        <Avatar src={ACCT_ICONS.chevD} class="w-3.5 h-3.5" imageClass="tint-slate-400" fallbackDelay={Infinity} accessibilityLabel="" />
      </View>
      <If when={kindMenu()}>
        <Overlay anchor={kindAnchor} placement="bottom" align="start" offset={2} margin={8} onCancel={() => kindMenu.set(false)} onPressOutside={() => kindMenu.set(false)}>
          <View class={`w-[240px] max-h-[280px] rounded-xl border py-1 shadow-lg overflow-hidden ${C.panel} ${C.line}`}>
            <Scroll class="max-h-[276px]">
              <View
                class="flex-row items-center gap-2.5 px-4 py-2.5 transition-colors hover:bg-[#f8fafc] dark:hover:bg-white/5"
                onPress={() => { ncKind.set("unsure"); kindMenu.set(false); }}
                activeStyle={{ opacity: 0.7 }}
              >
                <Text class={`text-[13px] ${ncKind() === "unsure" ? `font-semibold ${C.ink}` : C.mute}`}>Not sure</Text>
              </View>
              <Each of={bookKinds()}>
                {(k: Any) => (
                  <View
                    class="flex-row items-center gap-2.5 px-4 py-2.5 transition-colors hover:bg-[#f8fafc] dark:hover:bg-white/5"
                    // a kind fixes the account type, so the type field is
                    // hidden once a catalog kind is picked (PR37 finding 3)
                    onPress={() => {
                      ncKind.set(k());
                      kindMenu.set(false);
                    }}
                    activeStyle={{ opacity: 0.7 }}
                  >
                    <Text class={`text-[13px] ${ncKind() === k() ? `font-semibold ${C.ink}` : C.mute}`}>{acctKindLabel(k())}</Text>
                  </View>
                )}
              </Each>
            </Scroll>
          </View>
        </Overlay>
      </If>
    </View>
  );

  const formCard = () => {
    const editing = mode() === "edit";
    return (
      <View class="flex-1 flex-col">
        <View class="shrink-0 bg-[#2a9d8f] px-5 py-4 flex-row items-center gap-2.5">
          <Text class="text-[15px] font-semibold text-white">{editing ? "Edit Category" : "New Category"}</Text>
        </View>
        <Scroll class="max-h-[640px]">
          <View class="p-5 gap-3.5">
            <View class="gap-1.5">
              <Text class={`text-[11px] font-medium ${C.mute}`}>Category Name</Text>
              <If when={editing}>
                <View class={`rounded-lg border px-3 py-2 ${C.line}`}>
                  <Text class={`text-[13px] ${C.ink}`}>{ncName()}</Text>
                </View>
              </If>
              <If when={!editing}>
                <TextBox label="Category name" value={ncName()} onChange={(v) => ncName.set(v)} placeholder="e.g., Office Supplies" entry />
              </If>
            </View>
            <View class="gap-1.5">
              <Text class={`text-[11px] font-medium ${C.mute}`}>Account Number</Text>
              <If when={editing}>
                <View class={`rounded-lg border px-3 py-2 ${C.line}`}>
                  <Text class={`text-[13px] font-mono ${C.ink}`}>{ncNumber()}</Text>
                </View>
              </If>
              <If when={!editing}>
                <TextBox label="Account number" value={ncNumber()} onChange={(v) => ncNumber.set(String(v).replace(/[^0-9]/g, "").slice(0, 12))} placeholder="e.g., 61000" entry />
              </If>
            </View>
            {kindField()}
            {/* A catalog kind sets the type, so the picker only shows for
                unsure/mixed; editing keeps it read-only. */}
            <If when={editing || ncKind() === "unsure" || ncKind() === "mixed"}>
            <View class="gap-1.5">
              <Text class={`text-[11px] font-medium ${C.mute}`}>Category Type</Text>
              <View class="flex-row flex-wrap gap-1.5">
                <Each of={ACCT_TYPES}>
                  {(t: Any) => (
                    <View
                      class={`flex-row items-center gap-1.5 rounded-lg border px-2.5 py-1.5 transition-colors ${
                        ncType() === t()[0]
                          ? "border-[#0d9488] bg-[#f0fdfa] dark:bg-[#134e4a]/30"
                          : editing
                            ? `${C.line}`
                            : `${C.line} hover:border-[#cbd5e1] dark:hover:border-[#3f3f46]`
                      }`}
                      onPress={() => (editing ? null : ncType.set(t()[0]))}
                      activeStyle={editing ? undefined : { opacity: 0.7 }}
                    >
                      <Avatar src={ACCT_TYPE_ICONS[t()[0]]} class="w-3.5 h-3.5" imageClass={`tint-[${t()[2]}]`} fallbackDelay={Infinity} accessibilityLabel="" />
                      <Text class={`text-[11.5px] ${ncType() === t()[0] ? `font-semibold ${C.ink}` : C.mute}`}>{t()[1]}</Text>
                    </View>
                  )}
                </Each>
              </View>
            </View>
            </If>
            <If when={editing}>
              <Text class={`text-[11.5px] ${C.dim} leading-snug`}>
                {"Saving edits needs an account-update tool Books doesn't have yet — the form is read-only for now."}
              </Text>
            </If>
            <View class="flex-row gap-2 pt-1">
              <View class="flex-1">
                <Btn label={editing ? "Close" : "Cancel"} onPress={() => mode.set("")} />
              </View>
              <If when={!editing}>
                <View
                  class={`flex-1 h-[34px] rounded-lg items-center justify-center transition-colors ${
                    ncName().trim() ? "bg-[#0d9488] hover:bg-[#0f766e]" : "bg-[#0d9488] opacity-40"
                  }`}
                  onPress={createCategory}
                  activeStyle={{ opacity: 0.85 }}
                >
                  <Text class="text-[13px] font-semibold text-white">{ncBusy() ? "Creating…" : "Create Category"}</Text>
                </View>
              </If>
            </View>
          </View>
        </Scroll>
      </View>
    );
  };

  const imported = resource(
    () => ({ v: ver() }),
    () => buwiz.call("books_import_get", {}),
  );
  const waiting = () => {
    const c = (imported() as Any)?.counts;
    return c ? c.to_confirm + c.not_matched : 0;
  };

  return (
    <View class="flex-1 flex-col overflow-hidden relative">
      {/* Top Navigation — ‹ Back + New Category split pill */}
      <View class="shrink-0 flex-row items-center justify-between px-3 sm:px-6 py-2 sm:py-3">
        <View
          class="flex-row items-center gap-1.5 py-1.5 pr-2"
          onPress={() => p.go("today")}
          activeStyle={{ opacity: 0.7 }}
        >
          <Text class={`text-[14px] font-medium transition-colors ${C.ink} hover:text-[#0d9488] dark:hover:text-[#2dd4bf]`}>‹ Back</Text>
        </View>
        <View ref={catMenuAnchor} class="relative flex-row items-stretch rounded-full bg-[#0d9488] shadow-sm">
          <View
            class="px-4 pl-5 h-[38px] flex-row items-center gap-1.5 rounded-l-full transition-colors hover:bg-[#0f766e]"
            onPress={() => openCreate()}
            activeStyle={{ opacity: 0.85 }}
          >
            <Text class="text-[14px] font-bold text-white">+</Text>
            <Text class="text-[13px] font-semibold text-white">New Category</Text>
          </View>
          <View class="w-px my-2 bg-white/30" />
          <View
            class="px-3 h-[38px] items-center justify-center rounded-r-full transition-colors hover:bg-[#0f766e]"
            onPress={() => catMenu.set(!catMenu())}
            activeStyle={{ opacity: 0.85 }}
          >
            <Text class="text-[12px] text-white">⌄</Text>
          </View>
        </View>
      </View>

      {/* Two-Panel Layout */}
      <View class="flex-1 min-h-0 flex-row gap-6 px-3 pb-3 sm:px-6 sm:pb-6 overflow-hidden">
        {/* Main Panel — rounded-2xl card with teal Category Manager header */}
        <View class={`flex-1 min-h-0 rounded-[16px] border overflow-hidden flex-col bg-white dark:bg-slate-800 ${C.line}`}>
          <View class="shrink-0 bg-[#2a9d8f] px-5 py-4 flex-row items-center gap-2.5">
            <Avatar src={ACCT_ICONS.folder} class="w-5 h-5" imageClass="tint-white" fallbackDelay={Infinity} accessibilityLabel="" />
            <Text class="text-[16px] font-semibold text-white">Category Manager</Text>
          </View>

          {/* Filter bar — funnel button, chips, bare Lookup input (wraps), Clear */}
          <View class={`shrink-0 flex-row flex-wrap items-center gap-1.5 px-4 py-3 border-b bg-white dark:bg-slate-800 ${C.line} focus-within:border-b-[#2a9d8f] transition-colors`}>
            <View
              ref={filterAnchor}
              class={`w-9 h-9 items-center justify-center rounded-md transition-colors ${
                filterPop() ? "bg-[#f0fdfa] dark:bg-[#134e4a]/30" : "hover:bg-gray-100 dark:hover:bg-white/5"
              }`}
              onPress={() => filterPop.set(true)}
              activeStyle={{ opacity: 0.7 }}
              title="Filters"
            >
              <Avatar
                src={ACCT_ICONS.funnel}
                class="w-4 h-4"
                imageClass={filterPop() ? "tint-[#0d9488] dark:tint-[#2dd4bf]" : `tint-slate-500 dark:tint-slate-400`}
                fallbackDelay={Infinity}
                accessibilityLabel="Filters"
              />
            </View>
            <Each of={statusSel()}>
              {(s: Any) =>
                chip(
                  s() === "active" ? "Active" : "Deactivated",
                  s() === "active" ? "#22c55e" : "#94a3b8",
                  () => statusSel.set(statusSel().filter((x) => x !== s())),
                )
              }
            </Each>
            <Each of={types()}>
              {(t: Any) => chip(acctTypeLabel(t()), acctType(t())[2], () => toggleType(t()))}
            </Each>
            <Each of={kindsSel()}>
              {(k: Any) => chip(acctKindLabel(k()), "#2a9d8f", () => toggleKind(k()))}
            </Each>
            <Input
              accessibilityLabel="Lookup category"
              value={find()}
              placeholder="Lookup category..."
              onChange={(e: Any) => find.set(typeof e === "string" ? e : e.value)}
              class="h-11 min-w-[220px] flex-1 border-none bg-transparent outline-none text-base sm:text-sm text-[#32497f] dark:text-slate-200 placeholder:text-[#6b7c93] dark:placeholder:text-slate-500"
            />
            <If when={hasFilters()}>
              <View class="px-2 py-1" onPress={clearFilters} activeStyle={{ opacity: 0.7 }}>
                <Text class="text-[12px] font-semibold text-[#0d9488] transition-colors hover:text-[#0f766e] dark:text-[#2dd4bf] dark:hover:text-[#5eead4]">Clear</Text>
              </View>
            </If>
          </View>

          {/* Filter drawer — Status / Type / Sub-type sections */}
          <If when={filterPop()}>
            <Overlay anchor={filterAnchor} placement="bottom" align="start" offset={2} margin={8} onCancel={() => filterPop.set(false)} onPressOutside={() => filterPop.set(false)}>
              <View class={`w-[264px] rounded-xl border py-2 px-1 shadow-lg ${C.panel} ${C.line}`}>
                {section(
                  "status",
                  "Status",
                  statusSel().length === 1 && statusSel()[0] === "active" ? 0 : statusSel().length,
                  <View>
                    {checkRow("Active", "#22c55e", statusSel().includes("active"), () => toggleStatus("active"))}
                    {checkRow("Deactivated", "#94a3b8", statusSel().includes("deactivated"), () => toggleStatus("deactivated"))}
                  </View>,
                )}
                {section(
                  "type",
                  "Type",
                  types().length,
                  <View>
                    <Each of={ACCT_TYPES}>
                      {(t: Any) => checkRow(t()[1], t()[2], types().includes(t()[0]), () => toggleType(t()[0]))}
                    </Each>
                  </View>,
                )}
                {section(
                  "kind",
                  "Sub-type",
                  kindsSel().length,
                  <View>
                    <Each of={bookKinds()}>
                      {(k: Any) => checkRow(acctKindLabel(k()), "", kindsSel().includes(k()), () => toggleKind(k()))}
                    </Each>
                  </View>,
                )}
              </View>
            </Overlay>
          </If>

          <Loading res={data} />
          <If when={Boolean(d())}>
            <Scroll class="flex-1 min-h-0">
              {/* Column headers */}
              <View class={`grid shrink-0 grid-cols-12 items-center border-b border-gray-200 dark:border-slate-700 px-3 py-2 sm:px-4`}>
                <Text class={`col-span-9 sm:col-span-6 text-[11px] font-medium uppercase tracking-wider ${C.dim}`}>Category</Text>
                <Text class={`hidden text-center sm:col-span-3 sm:block text-[11px] font-medium uppercase tracking-wider ${C.dim}`}>Connections</Text>
                <View class="col-span-3" />
              </View>
              <Each of={flatTree()} key={rowKey}>
                {(n: Any) => catRow(n())}
              </Each>
              <If when={flatTree().length === 0}>
                <View class="flex-col items-center justify-center p-12 gap-2">
                  <Avatar src={ACCT_ICONS.folder} class="w-10 h-10 opacity-40" imageClass="tint-slate-400" fallbackDelay={Infinity} accessibilityLabel="" />
                  <Text class={`text-[14px] ${C.mute}`}>No categories found</Text>
                  <Text class={`text-[12px] text-center ${C.dim}`}>Try a different search or clear the filters.</Text>
                </View>
              </If>
              <If when={Boolean(d()?.note)}>
                <Text class={`text-[12px] px-4 py-3 ${C.dim}`}>{d()?.note || ""}</Text>
              </If>
              <View class="h-24 lg:hidden" />
            </Scroll>
          </If>
        </View>

        {/* Right Side Panel — Filter by Type tiles / detail card / form */}
        <SidePanel
          open={() => Boolean(selected()) || mode() !== "" || mFilter()}
          onBack={closeSide}
          fill
          body={() =>
            mode() !== "" || Boolean(selected()) ? (
              <View class={`flex-1 min-h-0 rounded-[16px] border overflow-hidden flex-col bg-white dark:bg-slate-800 ${C.line}`}>
                {mode() !== "" ? formCard() : detailCard()}
              </View>
            ) : (
              <View class={`w-[360px] max-w-full rounded-[16px] border overflow-hidden flex-col bg-white dark:bg-slate-800 ${C.line} shadow-[0_4px_24px_rgba(0,0,0,0.08)]`}>
                {filterTiles()}
              </View>
            )
          }
        />
        <If when={!Boolean(selected()) && mode() === "" && !mFilter()}>
          <PanelFab label="Open filters" icon="panelRight" onPress={() => mFilter.set(true)} />
        </If>
      </View>

      {/* "+ New Category" chevron menu */}
      <If when={catMenu()}>
        <Overlay anchor={catMenuAnchor} placement="bottom" align="end" offset={2} margin={8} onPressOutside={() => catMenu.set(false)} onCancel={() => catMenu.set(false)}>
          <View class={`w-[210px] rounded-xl border py-1 shadow-lg ${C.panel} ${C.line}`}>
            <View
              class="flex-row items-center gap-2.5 px-4 py-3 transition-colors hover:bg-[#f8fafc] dark:hover:bg-[#1c1c1f]"
              activeStyle={{ opacity: 0.7 }}
              onPress={() => {
                catMenu.set(false);
                p.go("import");
              }}
            >
              <Text class="text-[13px] text-[#94a3b8]">⇪</Text>
              <Text class={`text-[13px] ${C.ink}`}>Import Categories</Text>
            </View>
            <If when={waiting() > 0}>
              <View
                class={`flex-row items-center gap-2.5 px-4 py-3 border-t transition-colors hover:bg-[#f8fafc] dark:hover:bg-[#1c1c1f] ${C.line}`}
                activeStyle={{ opacity: 0.7 }}
                onPress={() => {
                  catMenu.set(false);
                  p.go("import");
                }}
              >
                <Text class="text-[13px] text-[#94a3b8]">⚯</Text>
                <Text class={`text-[13px] ${C.ink}`}>{`Match imported (${String(waiting())})`}</Text>
              </View>
            </If>
          </View>
        </Overlay>
      </If>
    </View>
  );
}
