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

const ACCT_ICON_PATHS: Record<string, string> = {
  Accounting01: acctSvg("<g stroke=\"currentColor\" stroke-linecap=\"round\" stroke-width=\"1.5\"><path stroke-linejoin=\"round\" d=\"M3 7.8c0-1.68 0-2.52.327-3.162a3 3 0 0 1 1.311-1.311C5.28 3 6.12 3 7.8 3h8.4c1.68 0 2.52 0 3.162.327a3 3 0 0 1 1.311 1.311C21 5.28 21 6.12 21 7.8v8.4c0 1.68 0 2.52-.327 3.162a3 3 0 0 1-1.311 1.311C18.72 21 17.88 21 16.2 21H7.8c-1.68 0-2.52 0-3.162-.327a3 3 0 0 1-1.311-1.311C3 18.72 3 17.88 3 16.2z\"></path><path d=\"M3 15.5h18M3 8.5h18M7 7v3M9 14v3M12 7v3M17 7v3M15 14v3\"></path></g>", 1.5),
  Advertising: acctSvg("<path stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" stroke-width=\"1.5\" d=\"m17 3-5 4-5-4m12 8h.01M19 14h.01M19 17h.01M6.8 21h10.4c1.68 0 2.52 0 3.162-.327a3 3 0 0 0 1.311-1.311C22 18.72 22 17.88 22 16.2v-4.4c0-1.68 0-2.52-.327-3.162a3 3 0 0 0-1.311-1.311C19.72 7 18.88 7 17.2 7H6.8c-1.68 0-2.52 0-3.162.327a3 3 0 0 0-1.311 1.311C2 9.28 2 10.12 2 11.8v4.4c0 1.68 0 2.52.327 3.162a3 3 0 0 0 1.311 1.311C4.28 21 5.12 21 6.8 21m-.04-3h7.48c.616 0 .924 0 1.16-.145.206-.128.375-.332.48-.583.12-.285.12-.659.12-1.405v-3.734c0-.746 0-1.12-.12-1.405a1.24 1.24 0 0 0-.48-.583C15.163 10 14.855 10 14.24 10H6.76c-.616 0-.924 0-1.16.145a1.24 1.24 0 0 0-.48.583c-.12.285-.12.659-.12 1.405v3.734c0 .746 0 1.12.12 1.405.105.25.274.455.48.583.236.145.544.145 1.16.145\"></path>", 1.5),
  Apps: acctSvg("<path stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" stroke-miterlimit=\"1.414\" stroke-width=\"1.5\" d=\"M20 16V7.2c0-1.12 0-1.68-.218-2.108a2 2 0 0 0-.874-.874C18.48 4 17.92 4 16.8 4H7.2c-1.12 0-1.68 0-2.108.218a2 2 0 0 0-.874.874C4 5.52 4 6.08 4 7.2V16m.667 4h14.666c.62 0 .93 0 1.185-.068a2 2 0 0 0 1.414-1.414c.068-.255.068-.565.068-1.185 0-.31 0-.465-.034-.592a1 1 0 0 0-.707-.707C21.132 16 20.977 16 20.667 16H3.333c-.31 0-.465 0-.592.034a1 1 0 0 0-.707.707C2 16.868 2 17.023 2 17.333c0 .62 0 .93.068 1.185a2 2 0 0 0 1.414 1.414c.255.068.565.068 1.185.068M9.8 12.929c-.994 0-1.8-.864-1.8-1.929 0-1.004.717-1.83 1.632-1.92.187-1.22 1.176-2.151 2.368-2.151s2.18.93 2.368 2.15C15.283 9.172 16 9.997 16 11c0 1.065-.806 1.929-1.8 1.929z\"></path>", 1.5),
  Bank: acctSvg("<path stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" d=\"M5 9v8m4.5-8v8m5-8v8M19 9v8M3 18.6v.8c0 .56 0 .84.109 1.054a1 1 0 0 0 .437.437C3.76 21 4.04 21 4.6 21h14.8c.56 0 .84 0 1.054-.109a1 1 0 0 0 .437-.437C21 20.24 21 19.96 21 19.4v-.8c0-.56 0-.84-.109-1.054a1 1 0 0 0-.437-.437C20.24 17 19.96 17 19.4 17H4.6c-.56 0-.84 0-1.054.109a1 1 0 0 0-.437.437C3 17.76 3 18.04 3 18.6m8.653-15.523-7.4 1.645c-.447.099-.67.149-.838.269a1 1 0 0 0-.334.417C3 5.597 3 5.826 3 6.283V7.4c0 .56 0 .84.109 1.054a1 1 0 0 0 .437.437C3.76 9 4.04 9 4.6 9h14.8c.56 0 .84 0 1.054-.109a1 1 0 0 0 .437-.437C21 8.24 21 7.96 21 7.4V6.283c0-.457 0-.686-.081-.875a1 1 0 0 0-.334-.417c-.167-.12-.39-.17-.838-.27l-7.4-1.644a2 2 0 0 0-.26-.049 1 1 0 0 0-.174 0c-.066.006-.13.02-.26.05\"></path>", 1.5),
  BankNoteScissors01: acctSvg("<g stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" stroke-width=\"1.5\"><path d=\"M2 14.5V8.2c0-1.12 0-1.68.218-2.108a2 2 0 0 1 .874-.874C3.52 5 4.08 5 5.2 5h13.6c1.12 0 1.68 0 2.108.218a2 2 0 0 1 .874.874C22 6.52 22 7.08 22 8.2v7.6c0 1.12 0 1.68-.218 2.108a2 2 0 0 1-.874.874C20.48 19 19.92 19 18.8 19h-8.3M2.917 18.972l6.798-2.117m-2.684-2.754-2.289 6.743m-2.916-3.817a1.154 1.154 0 1 1 1.611 1.652 1.154 1.154 0 0 1-1.61-1.652m3.222 3.305a1.154 1.154 0 1 1 1.61 1.652 1.154 1.154 0 0 1-1.61-1.652\"></path><path d=\"M13.5 9.75h-1.875a1.125 1.125 0 0 0 0 2.25h.75a1.125 1.125 0 0 1 0 2.25H10.5M12 9v.75m0 4.5V15\"></path></g>", 1.5),
  CalendarDollar01: acctSvg("<path stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" stroke-width=\"1.5\" d=\"M21 8h-.002m0 0H3.002m17.996 0q.003.366.002.8v8.4c0 1.68 0 2.52-.327 3.162a3 3 0 0 1-1.311 1.311C18.72 22 17.88 22 16.2 22H7.8c-1.68 0-2.52 0-3.162-.327a3 3 0 0 1-1.311-1.311C3 19.72 3 18.88 3 17.2V8.8q0-.434.002-.8m17.996 0c-.008-1.165-.055-1.831-.325-2.362a3 3 0 0 0-1.311-1.311C18.72 4 17.88 4 16.2 4H16M3.002 8H3m.002 0c.008-1.165.055-1.831.325-2.362a3 3 0 0 1 1.311-1.311C5.28 4 6.12 4 7.8 4H8m0 0h8M8 4V2m0 2v1m8-1V2m0 2v1m-4 7.09h.944c.599 0 1.118.296 1.379.728M12 12.091h-.833c-.92 0-1.667.651-1.667 1.455 0 .803.746 1.454 1.667 1.454h1.666c.92 0 1.667.651 1.667 1.455 0 .803-.746 1.454-1.667 1.454H12m0-5.818V11m0 6.91h-.944c-.599 0-1.118-.296-1.379-.728m2.323.727V19\"></path>", 1.5),
  CoinsHand: acctSvg("<path stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" d=\"M13.53 8.352a3.5 3.5 0 1 1 .941-4.704M6 20.088h2.61q.512 0 1.009.12l2.758.67c.598.147 1.222.16 1.826.043l3.05-.593a4.2 4.2 0 0 0 2.127-1.107l2.158-2.1a1.503 1.503 0 0 0 0-2.168 1.61 1.61 0 0 0-2.06-.143l-2.515 1.835c-.36.263-.799.405-1.25.405h-2.428 1.546c.871 0 1.577-.687 1.577-1.534v-.307c0-.703-.492-1.317-1.194-1.487l-2.385-.58A5 5 0 0 0 11.643 13c-.965 0-2.711.799-2.711.799L6 15.025M20 6.5a3.5 3.5 0 1 1-7 0 3.5 3.5 0 0 1 7 0M2 14.6v5.8c0 .56 0 .84.109 1.054a1 1 0 0 0 .437.437C2.76 22 3.04 22 3.6 22h.8c.56 0 .84 0 1.054-.109a1 1 0 0 0 .437-.437C6 21.24 6 20.96 6 20.4v-5.8c0-.56 0-.84-.109-1.054a1 1 0 0 0-.437-.437C5.24 13 4.96 13 4.4 13h-.8c-.56 0-.84 0-1.054.109a1 1 0 0 0-.437.437C2 13.76 2 14.04 2 14.6\"></path>", 1.5),
  CoinsHand02: acctSvg("<path stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" stroke-width=\"1.5\" d=\"M13.53 15.648a3.5 3.5 0 1 0 .941 4.704M6 3.912h2.61q.512 0 1.009-.12l2.758-.67a4.3 4.3 0 0 1 1.826-.043l3.05.593a4.2 4.2 0 0 1 2.127 1.107l2.158 2.1c.616.598.616 1.569 0 2.168a1.61 1.61 0 0 1-2.06.143l-2.515-1.835a2.1 2.1 0 0 0-1.25-.405h-2.428 1.546c.871 0 1.577.687 1.577 1.534v.307c0 .703-.492 1.317-1.194 1.487l-2.385.58a5 5 0 0 1-1.186.142c-.965 0-2.711-.799-2.711-.799L6 8.975M20 17.5a3.5 3.5 0 1 0-7 0 3.5 3.5 0 0 0 7 0M2 9.4V3.6c0-.56 0-.84.109-1.054a1 1 0 0 1 .437-.437C2.76 2 3.04 2 3.6 2h.8c.56 0 .84 0 1.054.109a1 1 0 0 1 .437.437C6 2.76 6 3.04 6 3.6v5.8c0 .56 0 .84-.109 1.054a1 1 0 0 1-.437.437C5.24 11 4.96 11 4.4 11h-.8c-.56 0-.84 0-1.054-.109a1 1 0 0 1-.437-.437C2 10.24 2 9.96 2 9.4\"></path>", 1.5),
  CoinsStacked03: acctSvg("<path stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" d=\"M13 5c0 1.105-2.462 2-5.5 2S2 6.105 2 5m11 0c0-1.105-2.462-2-5.5-2S2 3.895 2 5m11 0v1.5M2 5v12c0 1.105 2.462 2 5.5 2m0-8q-.253 0-.5-.008C4.197 10.9 2 10.043 2 9m5.5 6C4.462 15 2 14.105 2 13m20-1.5c0 1.105-2.462 2-5.5 2s-5.5-.895-5.5-2m11 0c0-1.105-2.462-2-5.5-2s-5.5.895-5.5 2m11 0V19c0 1.105-2.462 2-5.5 2s-5.5-.895-5.5-2v-7.5m11 3.75c0 1.105-2.462 2-5.5 2s-5.5-.895-5.5-2\"/>", 1.5),
  ComputerAsset: acctSvg("<path stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" stroke-width=\"1.5\" d=\"M20 6v8a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h12a2 2 0 0 1 2 2z M12 16v5 M8 21h8\"></path>", 1.5),
  Consultant: acctSvg("<path stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" d=\"M16 3.468a4.5 4.5 0 0 1 0 8.064m2 5.234c1.512.684 2.873 1.799 4 3.234M2 20c1.946-2.477 4.59-4 7.5-4s5.554 1.523 7.5 4M14 7.5a4.5 4.5 0 1 1-9 0 4.5 4.5 0 0 1 9 0\"></path>", 1.5),
  DepreciationAndAmortization: acctSvg("<path stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" d=\"m22 17-7.869-7.869c-.396-.396-.594-.594-.822-.668a1 1 0 0 0-.618 0c-.228.074-.426.272-.822.668L9.13 11.87c-.396.396-.594.594-.822.668a1 1 0 0 1-.618 0c-.228-.074-.426-.272-.822-.668L2 7m20 10h-7m7 0v-7\"></path>", 1.5),
  HostingFee: acctSvg("<path stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" d=\"M4 16.242A4.5 4.5 0 0 1 6.08 8.02a6.002 6.002 0 0 1 11.84 0A4.5 4.5 0 0 1 20 16.242M8 16l4-4m0 0 4 4m-4-4v9\"></path>", 1.5),
  Inventory: acctSvg("<path stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" stroke-width=\"1.5\" d=\"M20.5 7.278 12 12m0 0L3.5 7.278M12 12v9.5m2-.611-1.223.68c-.284.157-.425.236-.575.267a1 1 0 0 1-.403 0c-.15-.031-.292-.11-.576-.268l-7.4-4.11c-.3-.167-.45-.25-.558-.369a1 1 0 0 1-.215-.364C3 16.573 3 16.401 3 16.06V7.942c0-.343 0-.514.05-.667a1 1 0 0 1 .215-.364c.109-.119.258-.202.558-.368l7.4-4.111c.284-.158.425-.237.576-.267a1 1 0 0 1 .402 0c.15.03.292.11.576.267l7.4 4.11c.3.167.45.25.558.369a1 1 0 0 1 .214.364c.051.153.051.324.051.667V12.5m-13.5-8 9 5M16 18l2 2 4-4\"></path>", 1.5),
  Iou: acctSvg("<path stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" stroke-width=\"1.5\" d=\"M5 9.5v5m10-5V13c0 1 1.268 1.496 2 1.5.75 0 2-.5 2-1.5V9.5M2 8.2v7.6c0 1.12 0 1.68.218 2.108a2 2 0 0 0 .874.874C3.52 19 4.08 19 5.2 19h13.6c1.12 0 1.68 0 2.108-.218a2 2 0 0 0 .874-.874C22 17.48 22 16.92 22 15.8V8.2c0-1.12 0-1.68-.218-2.108a2 2 0 0 0-.874-.874C20.48 5 19.92 5 18.8 5H5.2c-1.12 0-1.68 0-2.108.218a2 2 0 0 0-.874.874C2 6.52 2 7.08 2 8.2M12.5 12a2.5 2.5 0 1 1-5 0 2.5 2.5 0 0 1 5 0\"/>", 1.5),
  Legal: acctSvg("<path stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" stroke-width=\"1.5\" d=\"M2.5 13h6m7 0h6M12 7v14m0-14a2.5 2.5 0 0 0 2.5-2.5M12 7a2.5 2.5 0 0 1-2.5-2.5M4 21h16M4 4.5h5.5m0 0a2.5 2.5 0 0 1 5 0m0 0H20M8.88 14.336C8.48 15.871 7.12 17 5.5 17s-2.98-1.13-3.38-2.664c-.033-.125-.05-.188-.05-.438a3 3 0 0 1 .105-.653c.08-.237.167-.371.34-.64L5.5 8l2.986 4.606c.173.268.26.402.34.639.05.145.107.5.106.653-.002.25-.018.313-.051.438m13 0C21.48 15.871 20.12 17 18.5 17s-2.98-1.13-3.38-2.664c-.033-.125-.05-.188-.05-.438-.002-.154.055-.508.105-.653.08-.237.167-.371.34-.64L18.5 8l2.986 4.606c.173.268.26.402.34.639.05.145.107.5.106.653-.002.25-.018.313-.051.438\"></path>", 1.5),
  Meals: acctSvg("<clipPath id=\"glass-fork-01_svg__a\"><path d=\"M0 0h24v24H0z\"></path></clipPath><g clip-path=\"url(#glass-fork-01_svg__a)\"><path stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" stroke-width=\"1.5\" d=\"M4 21h3.5m0 0H11m-3.5 0v-6m0 0c3.78.049 4.475-2.752 4.5-9.502a.996.996 0 0 0-.995-.998H4.02a1 1 0 0 0-.998 1.022c.144 6.254.504 9.589 4.478 9.478M18 21v-9m0 0c2.865-.005 3-1 3-7.5M18 12c-3.156.035-2.941-1.989-3-7.5m3 7.5V4.5\"></path></g>", 1.5),
  Opex: acctSvg("<path stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" stroke-width=\"1.5\" d=\"m14 22-2-2-2.75 2-2.5-2L4 22V6.8c0-1.68 0-2.52.327-3.162a3 3 0 0 1 1.311-1.311C6.28 2 7.12 2 8.8 2h6.4c1.68 0 2.52 0 3.162.327a3 3 0 0 1 1.311 1.311C20 4.28 20 5.12 20 6.8v4.7m1 3.5h-2m0 0h-.5a1.5 1.5 0 0 0 0 3h1a1.5 1.5 0 0 1 0 3H19m0-6v-1m0 7h-2m2 0v1M8 8h8m-8 4h5m-5 4h2\"></path>", 1.5),
  OtherIncome: acctSvg("<g stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" stroke-width=\"1.5\"><path d=\"M12 2a10 10 0 0 1 8.09 4.122L12 12z\"></path><path d=\"M21.542 9c.298.947.458 1.955.458 3 0 5.523-4.477 10-10 10S2 17.523 2 12c0-4.478 2.943-8.268 7-9.542\"></path></g>", 1.5),
  PaperClip: acctSvg("<path stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" d=\"m21.153 10.9-9.016 9.015a5.25 5.25 0 0 1-7.425-7.425l9.016-9.015a3.5 3.5 0 0 1 4.95 4.95l-8.662 8.662a1.75 1.75 0 1 1-2.475-2.475l7.601-7.602\"></path>", 1.5),
  PaymentProcessingFee: acctSvg("<path stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" stroke-width=\"1.5\" d=\"M22 9H2m3.5 9h-.3c-1.12 0-1.68 0-2.108-.218a2 2 0 0 1-.874-.874C2 16.48 2 15.92 2 14.8V7.2c0-1.12 0-1.68.218-2.108a2 2 0 0 1 .874-.874C3.52 4 4.08 4 5.2 4h13.6c1.12 0 1.68 0 2.108.218a2 2 0 0 1 .874.874C22 5.52 22 6.08 22 7.2v7.6c0 1.12 0 1.68-.218 2.108a2 2 0 0 1-.874.874C20.48 18 19.92 18 18.8 18h-.3m-9.75-2.2L17 20m0-5-8.25 4.2M9.5 13a1.5 1.5 0 1 1 0 3 1.5 1.5 0 0 1 0-3m0 6a1.5 1.5 0 1 1 0 3 1.5 1.5 0 0 1 0-3\"></path>", 1.5),
  PayrollClearing: acctSvg("<path stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" stroke-width=\"1.5\" d=\"M6 13a3 3 0 1 0 0-6 3 3 0 0 0 0 6z M1 20a5 5 0 0 1 5-5h0a5 5 0 0 1 5 5 M14 8h7 M18 5l3 3-3 3 M21 16H14 M17 13l-3 3 3 3\"></path>", 1.5),
  Percent: acctSvg("<path stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" stroke-width=\"1.5\" d=\"M19 5 5 19M9 7a2 2 0 1 1-4 0 2 2 0 0 1 4 0m10 10a2 2 0 1 1-4 0 2 2 0 0 1 4 0\"></path>", 1.5),
  RefundAndDiscount: acctSvg("<path stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" stroke-width=\"1.5\" d=\"M9 9h.01M15 15h.01M16 8l-8 8m9.901-11.001c.206.498.601.893 1.1 1.1l1.744.723a2.033 2.033 0 0 1 1.1 2.656l-.722 1.744a2.03 2.03 0 0 0 0 1.556l.722 1.744a2.033 2.033 0 0 1-1.1 2.656L19 17.902A2.03 2.03 0 0 0 17.9 19l-.723 1.745a2.03 2.03 0 0 1-2.656 1.1l-1.744-.722a2.03 2.03 0 0 0-1.555 0l-1.745.723a2.033 2.033 0 0 1-2.654-1.1L6.1 19.001A2.03 2.03 0 0 0 5 17.9l-1.744-.723a2.033 2.033 0 0 1-1.1-2.654l.721-1.744a2.03 2.03 0 0 0 0-1.556l-.722-1.746a2.033 2.033 0 0 1 1.1-2.657L5 6.098c.498-.206.893-.6 1.1-1.098l.723-1.745a2.033 2.033 0 0 1 2.656-1.1l1.744.722a2.03 2.03 0 0 0 1.555-.001l1.746-.72a2.03 2.03 0 0 1 2.655 1.1l.723 1.746zM9.5 9a.5.5 0 1 1-1 0 .5.5 0 0 1 1 0m6 6a.5.5 0 1 1-1 0 .5.5 0 0 1 1 0\"></path>", 1.5),
  Rent: acctSvg("<path stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" stroke-width=\"1.5\" d=\"M6 20.087h2.61q.512 0 1.009.122l2.758.67c.598.146 1.222.16 1.826.042l3.05-.593a4.2 4.2 0 0 0 2.127-1.108l2.158-2.098a1.503 1.503 0 0 0 0-2.169 1.61 1.61 0 0 0-2.06-.143l-2.515 1.835c-.36.263-.799.405-1.25.405h-2.428 1.546c.871 0 1.577-.687 1.577-1.534v-.307c0-.704-.492-1.317-1.194-1.487l-2.385-.58A5 5 0 0 0 11.643 13c-.965 0-2.711.799-2.711.799L6 15.025m11.958-9.24.005.003M2 14.6v5.8c0 .56 0 .84.109 1.054a1 1 0 0 0 .437.437C2.76 22 3.04 22 3.6 22h.8c.56 0 .84 0 1.054-.11a1 1 0 0 0 .437-.436C6 21.24 6 20.96 6 20.4v-5.8c0-.56 0-.84-.109-1.054a1 1 0 0 0-.437-.437c-.214-.11-.494-.11-1.054-.11h-.8c-.56 0-.84 0-1.054.11a1 1 0 0 0-.437.437C2 13.76 2 14.04 2 14.6m14.632-5.82c1.742.77 3.747.055 4.48-1.599s-.088-3.618-1.83-4.39c-1.741-.77-3.746-.055-4.478 1.599q-.09.204-.15.415c-.065.23-.098.345-.139.41a.45.45 0 0 1-.138.145c-.062.044-.15.076-.329.14l-3.815 1.36a1.2 1.2 0 0 0-.248.106.5.5 0 0 0-.127.116c-.04.051-.067.112-.121.234l-.384.867c-.123.28-.185.42-.175.551a.56.56 0 0 0 .133.32c.088.104.236.17.53.3l.913.404c.129.057.193.085.26.098q.089.018.178.008c.067-.008.131-.03.26-.077l3.815-1.36c.178-.064.267-.096.346-.102a.5.5 0 0 1 .208.022c.076.023.184.088.4.218q.196.118.411.214\"></path>", 1.5),
  Repairs: acctSvg("<path stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" d=\"M15.631 7.631c-.396-.396-.594-.594-.668-.822a1 1 0 0 1 0-.618c.074-.228.272-.426.668-.822L18.47 2.53a6 6 0 0 0-8.3 6.895c.12.49.179.734.168.888a.85.85 0 0 1-.11.392c-.07.138-.207.274-.48.547L3.5 17.5a2.121 2.121 0 0 0 3 3l6.248-6.248c.273-.273.41-.41.547-.48a.85.85 0 0 1 .392-.11c.154-.011.399.049.888.168q.687.168 1.425.17a6 6 0 0 0 5.47-8.47L18.63 8.37c-.396.396-.594.594-.822.668a1 1 0 0 1-.618 0c-.228-.074-.426-.272-.822-.668z\"></path>", 1.5),
  Retirement: acctSvg("<path stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" stroke-width=\"1.5\" d=\"M21 20s-1.19-.47-2-.698c-2.56-.722-4.78-.012-7 .698m-9 0s1.19.47 2 .698q.38.107.751.174M12.25 6c-3.021-.767-5.908-.278-9.75.5 5.564-6.257 8.017-4.529 10-.5 3.54-4.21 7.022-5.089 9.5.5-3.418-.917-6.334-1.302-9.75-.5m0 0v3.224m0-3.224c7.445 1.767 8.606 3.59 6.25 9-.99-3.032-1.987-4.535-4.494-5.35M12.25 6C5.722 8.016 3.197 10.009 6 15c1.07-2.83 2.13-4.519 4.5-5.35m1.75-.426a10 10 0 0 0-1.75.425m1.75-.425c.658.118 1.24.257 1.756.425m-3.506 0c.135 4.92-1.044 7.338-4.749 11.223m0 0C8 21.279 10 20.64 12 20m0 0c1.52-3.97 2.103-6.219 2.006-10.35\"></path>", 1.5),
  Salary: acctSvg("<path stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" stroke-width=\"1.5\" d=\"M14 15.5H7.5c-1.396 0-2.093 0-2.661.172a4 4 0 0 0-2.667 2.667C2 18.907 2 19.604 2 21m20-8.75h-2.5m0 0h-.625a1.875 1.875 0 0 0 0 3.75h1.25a1.875 1.875 0 0 1 0 3.75H19.5m0-7.5V11m0 8.75H17m2.5 0V21m-5-13.5a4.5 4.5 0 1 1-9 0 4.5 4.5 0 0 1 9 0\"></path>", 1.5),
  Shareholders: acctSvg("<path stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" stroke-width=\"1.5\" d=\"M6 20.087h2.61q.512 0 1.009.122l2.758.67c.598.146 1.222.16 1.826.042l3.05-.593a4.2 4.2 0 0 0 2.127-1.107l2.158-2.1a1.503 1.503 0 0 0 0-2.168 1.61 1.61 0 0 0-2.06-.143l-2.515 1.835c-.36.263-.799.405-1.25.405h-2.428 1.546c.871 0 1.577-.687 1.577-1.534v-.307c0-.703-.492-1.317-1.194-1.487l-2.385-.58A5 5 0 0 0 11.643 13c-.965 0-2.711.799-2.711.799L6 15.025M21 3.25h-2.5m0 0h-.625a1.875 1.875 0 0 0 0 3.75h1.25a1.875 1.875 0 0 1 0 3.75H18.5m0-7.5V2m0 8.75H16m2.5 0V12M2 14.6v5.8c0 .56 0 .84.109 1.054a1 1 0 0 0 .437.437C2.76 22 3.04 22 3.6 22h.8c.56 0 .84 0 1.054-.109a1 1 0 0 0 .437-.437C6 21.24 6 20.96 6 20.4v-5.8c0-.56 0-.84-.109-1.054a1 1 0 0 0-.437-.437C5.24 13 4.96 13 4.4 13h-.8c-.56 0-.84 0-1.054.109a1 1 0 0 0-.437.437C2 13.76 2 14.04 2 14.6\"></path>", 1.5),
  SubscriptionRevenue: acctSvg("<path stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" stroke-width=\"1.5\" d=\"M17 5.126A8.5 8.5 0 0 1 12 20.5h-.5M7 18.874A8.5 8.5 0 0 1 12 3.5h.5m.5 18.9-2-2 2-2M11 5.6l2-2-2-2\"></path>", 1.5),
  SvgTagDollar: acctSvg("<path stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" stroke-width=\"1.5\" d=\"M17.5 6.5h.01M14 9h-2m0 0h-.5a1.5 1.5 0 0 0 0 3h1a1.5 1.5 0 0 1 0 3H12m0-6V8m0 7h-2m2 0v1m9.757-10.8v4.475c0 .489 0 .733-.055.963a2 2 0 0 1-.24.579c-.123.201-.296.374-.642.72l-7.668 7.669c-1.189 1.188-1.783 1.782-2.468 2.004a3 3 0 0 1-1.854 0c-.685-.222-1.279-.816-2.467-2.004l-2.212-2.212c-1.188-1.188-1.782-1.782-2.004-2.467a3 3 0 0 1 0-1.854c.222-.685.816-1.28 2.004-2.467l7.67-7.669c.345-.346.518-.519.72-.642q.27-.165.578-.24c.23-.055.475-.055.964-.055h4.474c1.12 0 1.68 0 2.108.218a2 2 0 0 1 .874.874c.218.428.218.988.218 2.108M18 6.5a.5.5 0 1 1-1 0 .5.5 0 0 1 1 0\"></path>", 1.5),
  Transaction: acctSvg("<g stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" stroke-width=\"1.5\"><path d=\"M4 6.8c0-1.68 0-2.52.327-3.162a3 3 0 0 1 1.311-1.311C6.28 2 7.12 2 8.8 2h6.4c1.68 0 2.52 0 3.162.327a3 3 0 0 1 1.311 1.311C20 4.28 20 5.12 20 6.8V22l-2.75-2-2.5 2L12 20l-2.75 2-2.5-2L4 22zM8 8h8M8 12h5M8 16h2\"></path></g>", 1.5),
  Travel: acctSvg("<path stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" d=\"M17.745 2.813a2.483 2.483 0 0 1 3.58 3.442l-2.78 2.91c-.217.228-.326.342-.393.475a1 1 0 0 0-.104.38c-.01.148.025.302.096.61l1.728 7.487c.072.315.109.473.097.625a1 1 0 0 1-.11.387c-.072.136-.186.25-.415.479l-.37.37c-.607.607-.91.91-1.22.965a1 1 0 0 1-.77-.182c-.253-.188-.389-.595-.66-1.408l-2.01-6.029-3.345 3.345c-.2.2-.3.3-.367.417a1 1 0 0 0-.118.337c-.02.133-.005.274.026.554l.184 1.654c.03.28.046.42.025.554a1 1 0 0 1-.118.337c-.067.117-.167.217-.366.417l-.198.197c-.473.473-.71.71-.972.778a1 1 0 0 1-.69-.068c-.244-.118-.43-.397-.8-.953L6.106 18.54c-.066-.1-.1-.15-.138-.194a1 1 0 0 0-.111-.112c-.045-.038-.095-.071-.194-.138L3.31 16.528c-.557-.371-.835-.557-.954-.801a1 1 0 0 1-.068-.69c.069-.262.305-.499.778-.972l.198-.197c.2-.2.3-.3.417-.366a1 1 0 0 1 .336-.119c.134-.02.274-.005.555.026l1.653.184c.28.03.421.046.555.025a1 1 0 0 0 .336-.118c.118-.067.218-.166.417-.366l3.346-3.346-6.03-2.01c-.813-.27-1.22-.406-1.407-.659a1 1 0 0 1-.182-.77c.055-.31.358-.613.964-1.22l.37-.37c.23-.23.344-.344.48-.415a1 1 0 0 1 .386-.11c.153-.012.31.024.626.097l7.459 1.721c.31.072.465.108.614.097a1 1 0 0 0 .412-.122c.132-.072.242-.186.464-.415z\"></path>", 1.5),
  TrendUp01: acctSvg("<path stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" d=\"m22 7-7.869 7.869c-.396.396-.594.594-.822.668a1 1 0 0 1-.618 0c-.228-.074-.426-.272-.822-.668L9.13 12.13c-.396-.396-.594-.594-.822-.668a1 1 0 0 0-.618 0c-.228.074-.426.272-.822.668L2 17M22 7h-7m7 0v7\"></path>", 1.5),
  UserCoins01: acctSvg("<path stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" stroke-width=\"1.5\" d=\"M12 15.5H7.5c-1.396 0-2.093 0-2.661.172a4 4 0 0 0-2.667 2.667C2 18.907 2 19.604 2 21m20-4.783c0 .673-1.567 1.218-3.5 1.218S15 16.89 15 16.217m7 0C22 15.545 20.433 15 18.5 15s-3.5.545-3.5 1.217m7 0v4.566C22 21.455 20.433 22 18.5 22s-3.5-.545-3.5-1.217v-4.566m7 2.283c0 .672-1.567 1.217-3.5 1.217S15 19.172 15 18.5m-.5-11a4.5 4.5 0 1 1-9 0 4.5 4.5 0 0 1 9 0\"></path>", 1.5),
  Utilities: acctSvg("<g stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" stroke-width=\"1.5\"><path d=\"m2 9.5 9.04-6.78c.344-.258.516-.387.705-.437a1 1 0 0 1 .51 0c.189.05.36.179.705.437L22 9.5M4 8v9.8c0 1.12 0 1.68.218 2.108a2 2 0 0 0 .874.874C5.52 21 6.08 21 7.2 21h9.6c1.12 0 1.68 0 2.108-.218a2 2 0 0 0 .874-.874C20 19.48 20 18.92 20 17.8V8l-6.08-4.56c-.688-.516-1.033-.775-1.41-.874a2 2 0 0 0-1.02 0c-.377.1-.722.358-1.41.874z\"></path><path d=\"M12.667 9 10 13h4l-2.667 4\"></path></g>", 1.5),
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
