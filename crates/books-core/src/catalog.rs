//! The kind catalogue (`data/categories.v1.json`: 62 kinds, each with its
//! default line facts), the closed vocabularies of line facts
//! (`data/tax-attributes.v1.json`) and the chart presets (`data/presets/`).
//! All three are compiled in, so the binary and the PoC read the same bytes.
//!
//! A kind is accounting meaning, never form meaning (invariant 8): the
//! bridge reads line facts, not account numbers.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use serde_json::Value;

const CATEGORIES: &str = include_str!("../../../data/categories.v1.json");
const ATTRIBUTES: &str = include_str!("../../../data/tax-attributes.v1.json");
const PH_PRESET: &str = include_str!("../../../data/presets/ph-small-business.v1.json");

/// Account types, as web Books names them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AccountType {
    Asset,
    Liability,
    Equity,
    Revenue,
    CostOfRevenue,
    Expense,
    OtherIncome,
    OtherExpense,
}

impl AccountType {
    pub fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "asset" => Self::Asset,
            "liability" => Self::Liability,
            "equity" => Self::Equity,
            "revenue" => Self::Revenue,
            "cost_of_revenue" => Self::CostOfRevenue,
            "expense" => Self::Expense,
            "other_income" => Self::OtherIncome,
            "other_expense" => Self::OtherExpense,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Asset => "asset",
            Self::Liability => "liability",
            Self::Equity => "equity",
            Self::Revenue => "revenue",
            Self::CostOfRevenue => "cost_of_revenue",
            Self::Expense => "expense",
            Self::OtherIncome => "other_income",
            Self::OtherExpense => "other_expense",
        }
    }

    /// The Accounts page groups: Assets, Liabilities, Equity, Revenue, Expenses.
    pub fn group(self) -> &'static str {
        match self {
            Self::Asset => "Assets",
            Self::Liability => "Liabilities",
            Self::Equity => "Equity",
            Self::Revenue | Self::OtherIncome => "Revenue",
            Self::CostOfRevenue | Self::Expense | Self::OtherExpense => "Expenses",
        }
    }

    /// A debit raises the balance (assets and expenses); a credit raises it
    /// for the rest.
    pub fn debit_normal(self) -> bool {
        matches!(
            self,
            Self::Asset | Self::CostOfRevenue | Self::Expense | Self::OtherExpense
        )
    }

    /// Income statement (P&L) or balance sheet.
    pub fn is_pl(self) -> bool {
        !matches!(self, Self::Asset | Self::Liability | Self::Equity)
    }
}

/// The facts a line carries, as text in a closed vocabulary. `None` is
/// "not set": [`Facts::normalized`] fills the defaults the PoC uses.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Facts {
    pub measure_side: Option<String>,
    pub income_class: Option<String>,
    pub contra_kind: Option<String>,
    pub document_kind: Option<String>,
    pub person_scope: Option<String>,
    pub party_ref: Option<String>,
    pub comp_component: Option<String>,
    pub contribution_side: Option<String>,
    pub contribution_scheme: Option<String>,
    pub credit_kind: Option<String>,
    pub certificate_atc: Option<String>,
    pub certificate_status: Option<String>,
    pub period_start: Option<String>,
    pub period_end: Option<String>,
    pub final_tax_withheld: Option<String>,
}

/// The fact names, in the PoC's order (`NormLine`).
pub const FACT_NAMES: [&str; 15] = [
    "measure_side",
    "income_class",
    "contra_kind",
    "document_kind",
    "person_scope",
    "party_ref",
    "comp_component",
    "contribution_side",
    "contribution_scheme",
    "credit_kind",
    "certificate_atc",
    "certificate_status",
    "period_start",
    "period_end",
    "final_tax_withheld",
];

impl Facts {
    pub fn get(&self, name: &str) -> Option<&str> {
        self.slot(name).and_then(|s| s.as_deref())
    }

    fn slot(&self, name: &str) -> Option<&Option<String>> {
        Some(match name {
            "measure_side" => &self.measure_side,
            "income_class" => &self.income_class,
            "contra_kind" => &self.contra_kind,
            "document_kind" => &self.document_kind,
            "person_scope" => &self.person_scope,
            "party_ref" => &self.party_ref,
            "comp_component" => &self.comp_component,
            "contribution_side" => &self.contribution_side,
            "contribution_scheme" => &self.contribution_scheme,
            "credit_kind" => &self.credit_kind,
            "certificate_atc" => &self.certificate_atc,
            "certificate_status" => &self.certificate_status,
            "period_start" => &self.period_start,
            "period_end" => &self.period_end,
            "final_tax_withheld" => &self.final_tax_withheld,
            _ => return None,
        })
    }

    pub fn set(&mut self, name: &str, value: Option<String>) -> bool {
        let slot = match name {
            "measure_side" => &mut self.measure_side,
            "income_class" => &mut self.income_class,
            "contra_kind" => &mut self.contra_kind,
            "document_kind" => &mut self.document_kind,
            "person_scope" => &mut self.person_scope,
            "party_ref" => &mut self.party_ref,
            "comp_component" => &mut self.comp_component,
            "contribution_side" => &mut self.contribution_side,
            "contribution_scheme" => &mut self.contribution_scheme,
            "credit_kind" => &mut self.credit_kind,
            "certificate_atc" => &mut self.certificate_atc,
            "certificate_status" => &mut self.certificate_status,
            "period_start" => &mut self.period_start,
            "period_end" => &mut self.period_end,
            "final_tax_withheld" => &mut self.final_tax_withheld,
            _ => return false,
        };
        *slot = value;
        true
    }

    /// `over` wins where it sets a fact.
    pub fn overlay(&self, over: &Facts) -> Facts {
        let mut out = self.clone();
        for name in FACT_NAMES {
            if let Some(v) = over.get(name) {
                out.set(name, Some(v.to_string()));
            }
        }
        out
    }

    /// The PoC's defaults (`normalizeLine`): income class `none`, document
    /// `other`, person `self`, contribution side `none`, credit `none`,
    /// final tax `unknown`.
    pub fn normalized(&self) -> Facts {
        let mut out = self.clone();
        let fill = |slot: &mut Option<String>, value: &str| {
            if slot.is_none() {
                *slot = Some(value.to_string());
            }
        };
        fill(&mut out.income_class, "none");
        fill(&mut out.document_kind, "other");
        fill(&mut out.person_scope, "self");
        fill(&mut out.contribution_side, "none");
        fill(&mut out.credit_kind, "none");
        fill(&mut out.final_tax_withheld, "unknown");
        out
    }

    /// Facts set, as `{name: value}` (absent ones left out).
    pub fn to_json(&self) -> Value {
        let mut map = serde_json::Map::new();
        for name in FACT_NAMES {
            if let Some(v) = self.get(name) {
                map.insert(name.to_string(), Value::String(v.to_string()));
            }
        }
        Value::Object(map)
    }

    /// Facts from `{name: value | null}`. Unknown names are an error.
    pub fn from_json(value: &Value) -> Result<Facts, String> {
        let mut out = Facts::default();
        let Some(map) = value.as_object() else {
            return Err("facts must be an object".into());
        };
        for (name, v) in map {
            let text = match v {
                Value::Null => None,
                Value::String(s) => Some(s.clone()),
                _ => return Err(format!("fact {name} must be text")),
            };
            if !out.set(name, text) {
                return Err(format!("{name} is not a line fact"));
            }
        }
        Ok(out)
    }
}

/// One kind of the catalogue.
#[derive(Debug, Clone)]
pub struct Kind {
    pub id: String,
    pub name: String,
    pub number: Option<String>,
    pub account_type: AccountType,
    /// A trade receivable or payable: only on an accrual book (ADR 0008).
    pub accrual_only: bool,
    pub defaults: Facts,
    pub measures: Vec<String>,
    pub counterparty: Option<String>,
}

/// One account of a chart preset.
#[derive(Debug, Clone)]
pub struct PresetAccount {
    pub number: String,
    pub name: String,
    pub account_type: AccountType,
    /// A kind id, or `unsure`.
    pub kind: String,
}

pub struct Catalog {
    pub kinds: Vec<Kind>,
    by_id: BTreeMap<String, usize>,
    vocab: BTreeMap<String, BTreeSet<String>>,
    pub preset: Vec<PresetAccount>,
    pub preset_id: String,
    pub preset_version: i64,
}

fn text(v: &Value) -> Option<String> {
    v.as_str().map(str::to_string)
}

impl Catalog {
    fn load() -> Self {
        let cats: Value = serde_json::from_str(CATEGORIES).expect("categories.v1.json parses");
        let mut kinds = Vec::new();
        for c in cats["categories"].as_array().expect("categories[]") {
            let ty = c["books_account_type"].as_str().unwrap_or("");
            let ty = ty.split(' ').next().unwrap_or("");
            let d = &c["default_line_attributes"];
            let mut defaults = Facts::default();
            for name in FACT_NAMES {
                defaults.set(name, text(&d[name]));
            }
            kinds.push(Kind {
                id: text(&c["id"]).expect("a category id"),
                name: text(&c["name"]).expect("a category name"),
                number: text(&c["coa"]["account_number"]),
                account_type: AccountType::parse(ty).expect("a known books_account_type"),
                accrual_only: c["accrual_only"].as_bool().unwrap_or(false),
                defaults,
                measures: c["measures"]
                    .as_array()
                    .map(|m| m.iter().filter_map(text).collect())
                    .unwrap_or_default(),
                counterparty: text(&d["counterparty_kind"]),
            });
        }
        let by_id = kinds
            .iter()
            .enumerate()
            .map(|(i, k)| (k.id.clone(), i))
            .collect();

        let attrs: Value = serde_json::from_str(ATTRIBUTES).expect("tax-attributes.v1.json parses");
        let mut vocab = BTreeMap::new();
        for a in attrs["line_attributes"]
            .as_array()
            .expect("line_attributes[]")
        {
            let values: BTreeSet<String> = a["allowed_values"]
                .as_array()
                .map(|v| {
                    v.iter()
                        .filter_map(Value::as_str)
                        .filter(|s| s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'))
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default();
            vocab.insert(text(&a["name"]).expect("an attribute name"), values);
        }

        let preset: Value = serde_json::from_str(PH_PRESET).expect("the PH preset parses");
        let accounts = preset["accounts"]
            .as_array()
            .expect("accounts[]")
            .iter()
            .map(|a| PresetAccount {
                number: text(&a["number"]).expect("a number"),
                name: text(&a["name"]).expect("a name"),
                account_type: AccountType::parse(a["type"].as_str().unwrap_or(""))
                    .expect("a known type"),
                kind: text(&a["kind"]).expect("a kind"),
            })
            .collect();
        Self {
            kinds,
            by_id,
            vocab,
            preset: accounts,
            preset_id: text(&preset["preset_id"]).expect("preset_id"),
            preset_version: preset["version"].as_i64().unwrap_or(1),
        }
    }

    pub fn get() -> &'static Catalog {
        static CATALOG: OnceLock<Catalog> = OnceLock::new();
        CATALOG.get_or_init(Catalog::load)
    }

    pub fn kind(&self, id: &str) -> Option<&Kind> {
        self.by_id.get(id).map(|i| &self.kinds[*i])
    }

    /// Whether `value` is in the closed vocabulary of fact `name`.
    pub fn allowed(&self, name: &str, value: &str) -> bool {
        self.vocab.get(name).is_some_and(|v| v.contains(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_catalogue_has_sixty_two_kinds_and_two_accrual_only() {
        let c = Catalog::get();
        assert_eq!(c.kinds.len(), 62);
        let accrual: Vec<&str> = c
            .kinds
            .iter()
            .filter(|k| k.accrual_only)
            .map(|k| k.id.as_str())
            .collect();
        assert_eq!(accrual, ["accounts_receivable_gross", "accounts_payable"]);
        let sales = c.kind("sales_revenue").unwrap();
        assert_eq!(
            sales.defaults.income_class.as_deref(),
            Some("business_sales")
        );
        assert_eq!(sales.defaults.measure_side.as_deref(), Some("credit"));
        assert_eq!(
            c.kind("refunds_and_discounts")
                .unwrap()
                .defaults
                .contra_kind
                .as_deref(),
            Some("return")
        );
    }

    #[test]
    fn vocabularies_are_closed() {
        let c = Catalog::get();
        assert!(c.allowed("income_class", "business_sales"));
        assert!(c.allowed("document_kind", "credit_note"));
        assert!(!c.allowed("income_class", "vat_sales"));
        assert!(
            !c.allowed("party_ref", "opaque"),
            "party_ref is free text, checked by grammar"
        );
    }

    #[test]
    fn the_preset_numbers_every_kind_once() {
        let c = Catalog::get();
        let mut numbers = BTreeSet::new();
        for a in &c.preset {
            assert!(numbers.insert(a.number.clone()), "duplicate {}", a.number);
            assert!(
                a.kind == "unsure" || c.kind(&a.kind).is_some(),
                "{}",
                a.kind
            );
        }
        for k in &c.kinds {
            assert!(
                c.preset.iter().any(|a| a.kind == k.id),
                "{} has no account",
                k.id
            );
        }
        assert_eq!(c.preset.len(), 66);
    }

    #[test]
    fn facts_overlay_and_normalize() {
        let base = Catalog::get()
            .kind("sales_revenue")
            .unwrap()
            .defaults
            .clone();
        let over = Facts {
            document_kind: Some("sale_invoice".into()),
            ..Facts::default()
        };
        let f = base.overlay(&over).normalized();
        assert_eq!(f.document_kind.as_deref(), Some("sale_invoice"));
        assert_eq!(f.person_scope.as_deref(), Some("self"));
        assert_eq!(f.final_tax_withheld.as_deref(), Some("unknown"));
        let round = Facts::from_json(&f.to_json()).unwrap();
        assert_eq!(round, f);
        assert!(Facts::from_json(&serde_json::json!({"atc": "WC010"})).is_err());
    }
}
