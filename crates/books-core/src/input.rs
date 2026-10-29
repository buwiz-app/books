//! The normalized input the measures engine reads: profiles with their
//! accounting basis and period statuses, entries, lines carrying
//! accounting facts only, and the acknowledged return events. Port of
//! `poc/src/types.ts` and of the normalizing half of `poc/src/load.ts`
//! (`normalizeFixture()`, `normalizeEntry()`, `normalizeLine()`,
//! `normalizeProfile()`, `checkCashBasis()`).
//!
//! Money stays two-place decimal text here, exactly as the PoC keeps it;
//! the posting step parses it to integer centavos. Memos and labels are
//! dropped by normalizing, so they cannot reach a hash or a proposal. A
//! line or profile carrying a compliance attribute (tax type, ATC,
//! treatment, regime, election) is refused (ADR 0007).

use std::collections::{BTreeMap, HashMap, HashSet};

use serde_json::{Map, Value, json};

use crate::canonical::js_cmp;
use crate::date::{PeriodKind, days_in_month};
use crate::engine_error::EngineError;
use crate::money;
use crate::rules::Vocab;

pub const INPUT_FORMAT: &str = "buwiz-poc-input/3";
pub const FIXTURE_FORMAT: &str = "buwiz-poc-fixture/3";

/// The forms the bridge maps (`FormCode` in types.ts).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FormCode {
    F2551Q,
    F1601C,
    F1701MS,
}

impl FormCode {
    pub const ALL: [FormCode; 3] = [FormCode::F2551Q, FormCode::F1601C, FormCode::F1701MS];

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "2551Q" => Some(Self::F2551Q),
            "1601C" => Some(Self::F1601C),
            "1701MS" => Some(Self::F1701MS),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::F2551Q => "2551Q",
            Self::F1601C => "1601C",
            Self::F1701MS => "1701MS",
        }
    }

    /// `FORM_PERIOD_KIND`: 2551Q quarterly, 1601-C monthly, 1701-MS annual.
    pub fn period_kind(self) -> PeriodKind {
        match self {
            Self::F2551Q => PeriodKind::Quarter,
            Self::F1601C => PeriodKind::Month,
            Self::F1701MS => PeriodKind::Year,
        }
    }
}

/// Values the bridge reads from the Forms draft or profile (for example
/// `/taxpayer/tax_option`). Books never decides, infers or defaults them.
pub type FormContext = BTreeMap<String, String>;

/// One journal line after normalization (`NormLine`). Every field is
/// written to JSON, `null` included, so the hash matches the PoC's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormLine {
    pub line_id: String,
    pub account: String,
    pub debit: String,
    pub credit: String,
    pub measure_side: Option<String>,
    pub income_class: String,
    pub contra_kind: Option<String>,
    pub document_kind: String,
    pub person_scope: String,
    pub party_ref: Option<String>,
    pub comp_component: Option<String>,
    pub contribution_side: String,
    pub contribution_scheme: Option<String>,
    pub credit_kind: String,
    pub certificate_atc: Option<String>,
    pub certificate_status: Option<String>,
    pub period_start: Option<String>,
    pub period_end: Option<String>,
    pub final_tax_withheld: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormEntry {
    pub entry_id: String,
    pub tax_profile_id: String,
    pub entry_date: String,
    pub reverses_entry_id: Option<String>,
    pub lines: Vec<NormLine>,
}

/// A business profile as Books sees it (`NormProfile`): the opaque link
/// id, the accounting basis ("cash" or "accrual", ADR 0008) and period
/// statuses ("open" or "closed"). No tax facts, no elections.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormProfile {
    pub tax_profile_id: String,
    pub accounting_basis: String,
    pub periods: BTreeMap<String, String>,
}

/// The normalized input (`NormInput`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormInput {
    pub input_format: String,
    pub as_of: String,
    pub profiles: Vec<NormProfile>,
    pub acknowledged_entry_ids: Vec<String>,
    pub entries: Vec<NormEntry>,
}

fn opt(value: &Option<String>) -> Value {
    value
        .as_ref()
        .map_or(Value::Null, |s| Value::String(s.clone()))
}

impl NormLine {
    pub fn to_value(&self) -> Value {
        json!({
            "line_id": self.line_id,
            "account": self.account,
            "debit": self.debit,
            "credit": self.credit,
            "measure_side": opt(&self.measure_side),
            "income_class": self.income_class,
            "contra_kind": opt(&self.contra_kind),
            "document_kind": self.document_kind,
            "person_scope": self.person_scope,
            "party_ref": opt(&self.party_ref),
            "comp_component": opt(&self.comp_component),
            "contribution_side": self.contribution_side,
            "contribution_scheme": opt(&self.contribution_scheme),
            "credit_kind": self.credit_kind,
            "certificate_atc": opt(&self.certificate_atc),
            "certificate_status": opt(&self.certificate_status),
            "period_start": opt(&self.period_start),
            "period_end": opt(&self.period_end),
            "final_tax_withheld": self.final_tax_withheld,
        })
    }

    /// Reads a line already in normalized form (no vocabulary check; use
    /// [`normalize_line`] for raw input).
    pub fn from_value(v: &Value) -> Result<Self, EngineError> {
        let o = object(v, "line")?;
        Ok(Self {
            line_id: req_str(o, "line_id")?,
            account: req_str(o, "account")?,
            debit: req_str(o, "debit")?,
            credit: req_str(o, "credit")?,
            measure_side: opt_str(o, "measure_side")?,
            income_class: req_str(o, "income_class")?,
            contra_kind: opt_str(o, "contra_kind")?,
            document_kind: req_str(o, "document_kind")?,
            person_scope: req_str(o, "person_scope")?,
            party_ref: opt_str(o, "party_ref")?,
            comp_component: opt_str(o, "comp_component")?,
            contribution_side: req_str(o, "contribution_side")?,
            contribution_scheme: opt_str(o, "contribution_scheme")?,
            credit_kind: req_str(o, "credit_kind")?,
            certificate_atc: opt_str(o, "certificate_atc")?,
            certificate_status: opt_str(o, "certificate_status")?,
            period_start: opt_str(o, "period_start")?,
            period_end: opt_str(o, "period_end")?,
            final_tax_withheld: req_str(o, "final_tax_withheld")?,
        })
    }
}

impl NormEntry {
    pub fn to_value(&self) -> Value {
        json!({
            "entry_id": self.entry_id,
            "tax_profile_id": self.tax_profile_id,
            "entry_date": self.entry_date,
            "reverses_entry_id": opt(&self.reverses_entry_id),
            "lines": self.lines.iter().map(NormLine::to_value).collect::<Vec<_>>(),
        })
    }

    pub fn from_value(v: &Value) -> Result<Self, EngineError> {
        let o = object(v, "entry")?;
        let lines = match o.get("lines") {
            Some(Value::Array(items)) => items
                .iter()
                .map(NormLine::from_value)
                .collect::<Result<Vec<_>, _>>()?,
            _ => return Err(EngineError::input("entry.lines must be an array")),
        };
        Ok(Self {
            entry_id: req_str(o, "entry_id")?,
            tax_profile_id: req_str(o, "tax_profile_id")?,
            entry_date: req_str(o, "entry_date")?,
            reverses_entry_id: opt_str(o, "reverses_entry_id")?,
            lines,
        })
    }
}

impl NormProfile {
    pub fn to_value(&self) -> Value {
        let periods: Map<String, Value> = self
            .periods
            .iter()
            .map(|(k, v)| (k.clone(), Value::String(v.clone())))
            .collect();
        json!({
            "tax_profile_id": self.tax_profile_id,
            "accounting_basis": self.accounting_basis,
            "periods": periods,
        })
    }

    pub fn from_value(v: &Value) -> Result<Self, EngineError> {
        let o = object(v, "profile")?;
        Ok(Self {
            tax_profile_id: req_str(o, "tax_profile_id")?,
            accounting_basis: req_str(o, "accounting_basis")?,
            periods: string_map(o.get("periods"), "profile.periods")?,
        })
    }
}

impl NormInput {
    pub fn to_value(&self) -> Value {
        json!({
            "input_format": self.input_format,
            "as_of": self.as_of,
            "profiles": self.profiles.iter().map(NormProfile::to_value).collect::<Vec<_>>(),
            "acknowledged_entry_ids": self.acknowledged_entry_ids,
            "entries": self.entries.iter().map(NormEntry::to_value).collect::<Vec<_>>(),
        })
    }

    /// Reads an input already in normalized form, such as a vector's
    /// `input`. It checks shape only; [`normalize_fixture`] is the path that
    /// enforces the vocabularies and the cash-basis rule.
    pub fn from_value(v: &Value) -> Result<Self, EngineError> {
        let o = object(v, "input")?;
        let list = |key: &str| -> Result<&Vec<Value>, EngineError> {
            match o.get(key) {
                Some(Value::Array(items)) => Ok(items),
                _ => Err(EngineError::input(format!("input.{key} must be an array"))),
            }
        };
        Ok(Self {
            input_format: req_str(o, "input_format")?,
            as_of: req_str(o, "as_of")?,
            profiles: list("profiles")?
                .iter()
                .map(NormProfile::from_value)
                .collect::<Result<_, _>>()?,
            acknowledged_entry_ids: list("acknowledged_entry_ids")?
                .iter()
                .map(|id| {
                    id.as_str().map(str::to_string).ok_or_else(|| {
                        EngineError::input("acknowledged_entry_ids holds strings only")
                    })
                })
                .collect::<Result<_, _>>()?,
            entries: list("entries")?
                .iter()
                .map(NormEntry::from_value)
                .collect::<Result<_, _>>()?,
        })
    }
}

/// Reads a Forms context object: every value is text.
pub fn form_context_from_value(v: &Value) -> Result<FormContext, EngineError> {
    string_map(Some(v), "form_context")
}

pub fn form_context_to_value(ctx: &FormContext) -> Value {
    Value::Object(
        ctx.iter()
            .map(|(k, v)| (k.clone(), Value::String(v.clone())))
            .collect(),
    )
}

fn object<'a>(v: &'a Value, what: &str) -> Result<&'a Map<String, Value>, EngineError> {
    v.as_object()
        .ok_or_else(|| EngineError::input(format!("{what} must be an object")))
}

fn req_str(o: &Map<String, Value>, key: &str) -> Result<String, EngineError> {
    o.get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| EngineError::input(format!("{key} must be text")))
}

fn opt_str(o: &Map<String, Value>, key: &str) -> Result<Option<String>, EngineError> {
    match o.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(EngineError::input(format!("{key} must be text or null"))),
    }
}

fn string_map(v: Option<&Value>, what: &str) -> Result<BTreeMap<String, String>, EngineError> {
    match v {
        Some(Value::Object(map)) => map
            .iter()
            .map(|(k, v)| {
                v.as_str()
                    .map(|s| (k.clone(), s.to_string()))
                    .ok_or_else(|| EngineError::input(format!("{what}.{k} must be text")))
            })
            .collect(),
        _ => Err(EngineError::input(format!("{what} must be an object"))),
    }
}

/// Port of `isValidDate()`: `YYYY-MM-DD` naming a real day. As in the PoC
/// (which builds the day with `Date.UTC`, mapping years 0 to 99 onto the
/// 1900s), years below 100 are not valid.
pub fn is_valid_date(text: &str) -> bool {
    let b = text.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return false;
    }
    let digits = |s: &[u8]| s.iter().all(u8::is_ascii_digit);
    if !digits(&b[0..4]) || !digits(&b[5..7]) || !digits(&b[8..10]) {
        return false;
    }
    let (Ok(year), Ok(month), Ok(day)) = (
        text[0..4].parse::<i32>(),
        text[5..7].parse::<u8>(),
        text[8..10].parse::<u8>(),
    ) else {
        return false;
    };
    year >= 100 && (1..=12).contains(&month) && day >= 1 && day <= days_in_month(year, month)
}

fn is_opaque(text: &str, extra: &[u8], max: usize) -> bool {
    !text.is_empty()
        && text.len() <= max
        && text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || extra.contains(&b))
}

/// `^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$`.
fn is_uuid(text: &str) -> bool {
    let groups: Vec<&str> = text.split('-').collect();
    groups.len() == 5
        && groups
            .iter()
            .zip([8, 4, 4, 4, 12])
            .all(|(g, n)| g.len() == n && g.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')))
}

/// `^\*\*\*-\*\*\*-\*\*\*-\d{5}$`.
fn is_masked_display(text: &str) -> bool {
    text.len() == 17
        && text.starts_with("***-***-***-")
        && text.as_bytes()[12..].iter().all(u8::is_ascii_digit)
}

/// `^\d{4}(-Q[1-4]|-\d{2})?$`.
fn is_period_key(text: &str) -> bool {
    let b = text.as_bytes();
    if b.len() < 4 || !b[0..4].iter().all(u8::is_ascii_digit) {
        return false;
    }
    match &b[4..] {
        [] => true,
        [b'-', b'Q', q] => (b'1'..=b'4').contains(q),
        [b'-', m1, m2] => m1.is_ascii_digit() && m2.is_ascii_digit(),
        _ => false,
    }
}

/// `String(value)` for an error message.
fn shown(value: Option<&Value>) -> String {
    match value {
        None => "undefined".to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

/// `value ?? fallback` for a text field.
fn or_default<'a>(raw: &'a Map<String, Value>, key: &str, fallback: &'a Value) -> &'a Value {
    match raw.get(key) {
        None | Some(Value::Null) => fallback,
        Some(v) => v,
    }
}

fn require_in(set: &HashSet<String>, value: &Value, label: &str) -> Result<String, EngineError> {
    match value.as_str() {
        Some(s) if set.contains(s) => Ok(s.to_string()),
        _ => Err(EngineError::input(format!(
            "{label}: \"{}\" is not in the closed vocabulary",
            shown(Some(value))
        ))),
    }
}

fn optional_in(
    set: &HashSet<String>,
    value: Option<&Value>,
    label: &str,
) -> Result<Option<String>, EngineError> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(v) => require_in(set, v, label).map(Some),
    }
}

fn money_text(raw: &Map<String, Value>, key: &str, label: &str) -> Result<String, EngineError> {
    match raw.get(key) {
        None | Some(Value::Null) => Ok("0.00".to_string()),
        Some(Value::String(s)) => {
            money::parse(s).map_err(|e| EngineError::Money(format!("{label}: {e}")))?;
            Ok(s.clone())
        }
        Some(other) => Err(EngineError::Money(format!(
            "{label}: money must be decimal text, got a JSON {}",
            match other {
                Value::Number(_) => "number",
                Value::Bool(_) => "boolean",
                Value::Array(_) | Value::Object(_) => "object",
                _ => "value",
            }
        ))),
    }
}

fn date_or_null(
    raw: &Map<String, Value>,
    key: &str,
    label: &str,
) -> Result<Option<String>, EngineError> {
    match raw.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) if is_valid_date(s) => Ok(Some(s.clone())),
        Some(_) => Err(EngineError::input(format!("{label}.{key} is not a date"))),
    }
}

const BANNED_LINE: [&str; 6] = [
    "tax_types",
    "treatment",
    "atc",
    "tax_side",
    "regime",
    "vat_status",
];
const BANNED_PROFILE: [&str; 4] = ["facts", "regime", "elections", "prior_filings"];

/// Port of `normalizeLine()`.
pub fn normalize_line(raw: &Value, vocab: &Vocab) -> Result<NormLine, EngineError> {
    let empty = Map::new();
    let raw = raw.as_object().unwrap_or(&empty);
    let id = raw
        .get("line_id")
        .and_then(Value::as_str)
        .unwrap_or("(no line_id)")
        .to_string();
    let account = require_in(
        &vocab.account,
        raw.get("account").unwrap_or(&Value::Null),
        &format!("{id}.account"),
    )?;
    let debit = money_text(raw, "debit", &format!("{id}.debit"))?;
    let credit = money_text(raw, "credit", &format!("{id}.credit"))?;
    let none = Value::String("none".to_string());
    let other = Value::String("other".to_string());
    let self_scope = Value::String("self".to_string());
    let unknown = Value::String("unknown".to_string());
    let measure_side = optional_in(
        &vocab.measure_side,
        raw.get("measure_side"),
        &format!("{id}.measure_side"),
    )?;
    let income_class = require_in(
        &vocab.income_class,
        or_default(raw, "income_class", &none),
        &format!("{id}.income_class"),
    )?;
    let contra_kind = optional_in(
        &vocab.contra_kind,
        raw.get("contra_kind"),
        &format!("{id}.contra_kind"),
    )?;
    let document_kind = require_in(
        &vocab.document_kind,
        or_default(raw, "document_kind", &other),
        &format!("{id}.document_kind"),
    )?;
    let person_scope = require_in(
        &vocab.person_scope,
        or_default(raw, "person_scope", &self_scope),
        &format!("{id}.person_scope"),
    )?;
    let party_ref = match raw.get("party_ref") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) if is_opaque(s, b"_-", 40) => Some(s.clone()),
        Some(_) => {
            return Err(EngineError::input(format!(
                "{id}.party_ref must be an opaque id"
            )));
        }
    };
    let comp_component = optional_in(
        &vocab.comp_component,
        raw.get("comp_component"),
        &format!("{id}.comp_component"),
    )?;
    let contribution_side = require_in(
        &vocab.contribution_side,
        or_default(raw, "contribution_side", &none),
        &format!("{id}.contribution_side"),
    )?;
    let contribution_scheme = optional_in(
        &vocab.contribution_scheme,
        raw.get("contribution_scheme"),
        &format!("{id}.contribution_scheme"),
    )?;
    let credit_kind = require_in(
        &vocab.credit_kind,
        or_default(raw, "credit_kind", &none),
        &format!("{id}.credit_kind"),
    )?;
    let certificate_atc = optional_in(
        &vocab.certificate_atc,
        raw.get("certificate_atc"),
        &format!("{id}.certificate_atc"),
    )?;
    let certificate_status = optional_in(
        &vocab.certificate_status,
        raw.get("certificate_status"),
        &format!("{id}.certificate_status"),
    )?;
    let final_tax_withheld = require_in(
        &vocab.final_tax_withheld,
        or_default(raw, "final_tax_withheld", &unknown),
        &format!("{id}.final_tax_withheld"),
    )?;
    let period_start = date_or_null(raw, "period_start", &id)?;
    let period_end = date_or_null(raw, "period_end", &id)?;

    for banned in BANNED_LINE {
        if raw.contains_key(banned) {
            return Err(EngineError::input(format!(
                "{id}.{banned} is a compliance attribute; Books lines carry accounting facts only (ADR-0007)"
            )));
        }
    }
    let is_certificate = document_kind.starts_with("certificate_");
    if (income_class != "none" || comp_component.is_some() || is_certificate)
        && measure_side.is_none()
    {
        return Err(EngineError::input(format!(
            "{id}: a measured line (income_class, comp_component or certificate) needs measure_side"
        )));
    }
    if income_class == "contra_sales" && contra_kind.is_none() {
        return Err(EngineError::input(format!(
            "{id}: a contra_sales line needs contra_kind (return, allowance or discount)"
        )));
    }
    if income_class != "contra_sales" && contra_kind.is_some() {
        return Err(EngineError::input(format!(
            "{id}: contra_kind is only for contra_sales lines"
        )));
    }
    Ok(NormLine {
        line_id: id,
        account,
        debit,
        credit,
        measure_side,
        income_class,
        contra_kind,
        document_kind,
        person_scope,
        party_ref,
        comp_component,
        contribution_side,
        contribution_scheme,
        credit_kind,
        certificate_atc,
        certificate_status,
        period_start,
        period_end,
        final_tax_withheld,
    })
}

/// Port of `normalizeEntry()`.
pub fn normalize_entry(
    raw: &Value,
    profile_ids: &HashSet<String>,
    vocab: &Vocab,
) -> Result<NormEntry, EngineError> {
    let empty = Map::new();
    let o = raw.as_object().unwrap_or(&empty);
    let id = match o.get("entry_id") {
        Some(Value::String(s)) if is_opaque(s, b"._-", 40) => s.clone(),
        other => return Err(EngineError::input(format!("bad entry_id {}", shown(other)))),
    };
    let profile = o.get("tax_profile_id").and_then(Value::as_str);
    let Some(profile) = profile.filter(|p| profile_ids.contains(*p)) else {
        return Err(EngineError::input(format!(
            "{id}: tax_profile_id is not a profile in this input"
        )));
    };
    let entry_date = match o.get("entry_date") {
        Some(Value::String(s)) if is_valid_date(s) => s.clone(),
        _ => {
            return Err(EngineError::input(format!(
                "{id}: entry_date must be YYYY-MM-DD"
            )));
        }
    };
    let lines = match o.get("lines") {
        Some(Value::Array(items)) if items.len() >= 2 => items,
        _ => {
            return Err(EngineError::input(format!(
                "{id}: needs at least two lines"
            )));
        }
    };
    let reverses_entry_id = match o.get("reverses_entry_id") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) => Some(s.clone()),
        Some(_) => {
            return Err(EngineError::input(format!(
                "{id}: reverses_entry_id must be a string"
            )));
        }
    };
    Ok(NormEntry {
        entry_id: id,
        tax_profile_id: profile.to_string(),
        entry_date,
        reverses_entry_id,
        lines: lines
            .iter()
            .map(|line| normalize_line(line, vocab))
            .collect::<Result<_, _>>()?,
    })
}

/// Port of `normalizeProfile()`. The display text is checked to be the
/// masked form and then dropped.
pub fn normalize_profile(raw: &Value) -> Result<NormProfile, EngineError> {
    let empty = Map::new();
    let o = raw.as_object().unwrap_or(&empty);
    let id = match o.get("tax_profile_id") {
        Some(Value::String(s)) if is_uuid(s) => s.clone(),
        _ => return Err(EngineError::input("tax_profile_id must be an opaque UUID")),
    };
    if !o
        .get("display")
        .and_then(Value::as_str)
        .is_some_and(is_masked_display)
    {
        return Err(EngineError::input(
            "display must be the masked form ***-***-***-NNNNN",
        ));
    }
    for banned in BANNED_PROFILE {
        if o.contains_key(banned) {
            return Err(EngineError::input(format!(
                "profile.{banned}: elections, tax facts and prior filings are Forms' (ADR-0007)"
            )));
        }
    }
    let basis = match o.get("accounting_basis") {
        None | Some(Value::Null) => DEFAULT_ACCOUNTING_BASIS.to_string(),
        Some(Value::String(s)) if ACCOUNTING_BASES.contains(&s.as_str()) => s.clone(),
        Some(_) => {
            return Err(EngineError::input(
                "accounting_basis must be cash or accrual",
            ));
        }
    };
    let mut periods = BTreeMap::new();
    match o.get("periods") {
        None | Some(Value::Null) => {}
        Some(Value::Object(map)) => {
            for (key, status) in map {
                if !is_period_key(key) {
                    return Err(EngineError::input(format!("bad period key {key}")));
                }
                match status.as_str() {
                    Some(s @ ("open" | "closed")) => {
                        periods.insert(key.clone(), s.to_string());
                    }
                    _ => {
                        return Err(EngineError::input(format!(
                            "period {key} must be open or closed"
                        )));
                    }
                }
            }
        }
        Some(_) => return Err(EngineError::input("profile.periods must be an object")),
    }
    Ok(NormProfile {
        tax_profile_id: id,
        accounting_basis: basis,
        periods,
    })
}

/// `ACCOUNTING_BASES` and `DEFAULT_ACCOUNTING_BASIS` (owner decision,
/// 2026-10-29: cash by default).
pub const ACCOUNTING_BASES: [&str; 2] = ["cash", "accrual"];
pub const DEFAULT_ACCOUNTING_BASIS: &str = "cash";

/// Port of `checkCashBasis()` (ADR 0008). A cash-basis book posts a sale on
/// the day the money is received and an expense on the day it is paid, so
/// it has no trade receivable or payable lines, and its sales line carries
/// the invoice it settles (`sale_invoice`), never `payment_receipt`.
pub fn check_cash_basis(entry: &NormEntry, basis: &str, vocab: &Vocab) -> Result<(), EngineError> {
    if basis != "cash" {
        return Ok(());
    }
    for line in &entry.lines {
        if vocab.accrual_only_account.contains(&line.account) {
            return Err(EngineError::input(format!(
                "{}: {} is an accrual account; a cash-basis book posts the sale when the money is received and the expense when it is paid",
                line.line_id, line.account
            )));
        }
        if line.income_class == "business_sales" && line.document_kind == "payment_receipt" {
            return Err(EngineError::input(format!(
                "{}: on a cash-basis book the sales line carries the invoice it settles (sale_invoice), not payment_receipt",
                line.line_id
            )));
        }
    }
    Ok(())
}

/// Port of `normalizeFixture()`: a fixture file to the normalized input.
pub fn normalize_fixture(raw: &Value, vocab: &Vocab) -> Result<NormInput, EngineError> {
    let empty = Map::new();
    let o = raw.as_object().unwrap_or(&empty);
    if o.get("fixture_format").and_then(Value::as_str) != Some(FIXTURE_FORMAT) {
        return Err(EngineError::input(
            "fixture_format must be buwiz-poc-fixture/3",
        ));
    }
    let as_of = match o.get("as_of") {
        Some(Value::String(s)) if is_valid_date(s) => s.clone(),
        _ => return Err(EngineError::input("as_of must be YYYY-MM-DD")),
    };
    let array = |key: &str| -> Result<&Vec<Value>, EngineError> {
        o.get(key)
            .and_then(Value::as_array)
            .ok_or_else(|| EngineError::input(format!("{key} must be an array")))
    };
    let profiles = array("profiles")?
        .iter()
        .map(normalize_profile)
        .collect::<Result<Vec<_>, _>>()?;
    let profile_ids: HashSet<String> = profiles.iter().map(|p| p.tax_profile_id.clone()).collect();
    if profile_ids.len() != profiles.len() {
        return Err(EngineError::input("duplicate tax_profile_id"));
    }
    let entries = array("entries")?
        .iter()
        .map(|e| normalize_entry(e, &profile_ids, vocab))
        .collect::<Result<Vec<_>, _>>()?;
    let basis_of: HashMap<&str, &str> = profiles
        .iter()
        .map(|p| (p.tax_profile_id.as_str(), p.accounting_basis.as_str()))
        .collect();
    for entry in &entries {
        let basis = basis_of
            .get(entry.tax_profile_id.as_str())
            .copied()
            .unwrap_or(DEFAULT_ACCOUNTING_BASIS);
        check_cash_basis(entry, basis, vocab)?;
    }
    let entry_ids: HashSet<&str> = entries.iter().map(|e| e.entry_id.as_str()).collect();
    if entry_ids.len() != entries.len() {
        return Err(EngineError::input("duplicate entry_id"));
    }
    let mut acks: Vec<String> = Vec::new();
    match o.get("acknowledged_entry_ids") {
        None | Some(Value::Null) => {}
        Some(Value::Array(items)) => {
            for item in items {
                match item.as_str() {
                    Some(id) => acks.push(id.to_string()),
                    None => {
                        return Err(EngineError::input(format!(
                            "acknowledged entry {} does not exist",
                            shown(Some(item))
                        )));
                    }
                }
            }
        }
        Some(_) => {
            return Err(EngineError::input(
                "acknowledged_entry_ids must be an array",
            ));
        }
    }
    acks.sort_by(|a, b| js_cmp(a, b));
    for id in &acks {
        if !entry_ids.contains(id.as_str()) {
            return Err(EngineError::input(format!(
                "acknowledged entry {id} does not exist"
            )));
        }
    }
    Ok(NormInput {
        input_format: INPUT_FORMAT.to_string(),
        as_of,
        profiles,
        acknowledged_entry_ids: acks,
        entries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_and_keys() {
        assert!(is_valid_date("2026-02-28"));
        assert!(!is_valid_date("2026-02-29"));
        assert!(is_valid_date("2024-02-29"));
        assert!(!is_valid_date("0050-01-01"));
        assert!(!is_valid_date("2026-1-01"));
        assert!(is_period_key("2026") && is_period_key("2026-Q4") && is_period_key("2026-09"));
        assert!(!is_period_key("2026-Q5") && !is_period_key("2026-9"));
        assert!(is_uuid("0a1b2c3d-0000-4000-8000-00000000000a"));
        assert!(!is_uuid("0A1B2C3D-0000-4000-8000-00000000000a"));
        assert!(is_masked_display("***-***-***-00000"));
        assert!(!is_masked_display("123-456-789-00000"));
    }

    fn fixture(line_extra: Value, basis: &str) -> Value {
        let mut sale = json!({"line_id": "X-1.2", "account": "sales_revenue", "credit": "10.00",
            "income_class": "business_sales", "measure_side": "credit", "document_kind": "sale_invoice"});
        for (k, v) in line_extra.as_object().unwrap() {
            sale[k] = v.clone();
        }
        json!({
            "fixture_format": FIXTURE_FORMAT,
            "as_of": "2026-10-29",
            "profiles": [{"tax_profile_id": "0a1b2c3d-0000-4000-8000-00000000000a",
                "display": "***-***-***-00000", "accounting_basis": basis, "periods": {"2026-Q3": "closed"},
                "label": "dropped"}],
            "entries": [{"entry_id": "X-1", "tax_profile_id": "0a1b2c3d-0000-4000-8000-00000000000a",
                "entry_date": "2026-07-01", "memo": "MEMO-SYNTH dropped",
                "lines": [{"line_id": "X-1.1", "account": "bank_accounts", "debit": "10.00"}, sale]}],
        })
    }

    #[test]
    fn normalizes_and_refuses_like_the_poc() {
        let data = crate::rules::load_data().unwrap();
        let input = normalize_fixture(&fixture(json!({}), "cash"), &data.vocab).unwrap();
        let line = &input.entries[0].lines[0];
        assert_eq!(
            (line.credit.as_str(), line.income_class.as_str()),
            ("0.00", "none")
        );
        assert!(
            !crate::canonical::canonical(&input.to_value())
                .unwrap()
                .contains("MEMO")
        );

        for (extra, basis) in [
            (json!({"atc": "WI010"}), "cash"),
            (json!({"treatment": "vat"}), "accrual"),
            (json!({"credit": 10}), "accrual"),
            (json!({"document_kind": "payment_receipt"}), "cash"),
            (json!({"account": "accounts_receivable_gross"}), "cash"),
            (json!({"measure_side": null}), "accrual"),
            (json!({"contra_kind": "return"}), "accrual"),
        ] {
            assert!(
                normalize_fixture(&fixture(extra.clone(), basis), &data.vocab).is_err(),
                "{extra}"
            );
        }
        assert!(
            normalize_fixture(
                &fixture(json!({"document_kind": "payment_receipt"}), "accrual"),
                &data.vocab
            )
            .is_ok()
        );

        let mut elected = fixture(json!({}), "cash");
        elected["profiles"][0]["elections"] = json!({});
        assert!(normalize_fixture(&elected, &data.vocab).is_err());
    }
}
