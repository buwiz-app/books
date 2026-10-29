//! A proposal: accounting measures with lineage, and the fill arguments the
//! bridge derived from them for Forms input boxes. Port of
//! `poc/src/proposal.ts` (`buildProposal()`, `scopeInput()`,
//! `onlyProfileId()`, `computeInputsHash()`, `freshness()`,
//! `assertFillKey()`, `assertFillable()` and the crosswalk discipline).
//!
//! A proposal carries no tax amount, rate, due date or penalty: Forms
//! computes those. Only crosswalk input keys with a fill policy are ever
//! proposed (BRIDGE.INPUT_ONLY). `inputs_hash` covers every input the
//! proposal read, the Forms context included, so a change makes the stored
//! proposal stale and refuses fill. The same inputs give byte-identical
//! canonical JSON, which is what the vectors in `fixtures/vectors/` check.

use std::collections::{BTreeMap, HashSet};

use serde_json::{Map, Value, json};

use crate::bridge::{BridgeCtx, Item, Lineage, Outcome, counted_once, run_bridge};
use crate::canonical::{js_cmp, sha256_hex, sha256_of};
use crate::date::Period;
use crate::engine_error::EngineError;
use crate::input::{FormCode, FormContext, NormEntry, NormInput, form_context_to_value};
use crate::measures::MeasureResult;
use crate::money;
use crate::posting::post_ledger;
use crate::rules::{Crosswalk, Field, LoadedData};

pub const PROPOSAL_FORMAT: &str = "buwiz-poc-proposal/3";

/// Fill policies: the only crosswalk policies that may produce a sent value.
const FILL_POLICIES: [&str; 3] = ["fill_mvp", "fill_p6", "fill_p7"];

/// Election inputs that Forms owns and has not locked against the profile
/// election (verify.md s2 #3, #10; B7).
const ELECTION_RULES: [&str; 4] = ["1701MS.I.12", "1701MS.I.17", "1701MS.I.18", "2551Q.I.13"];

fn is_fill_policy(policy: &str) -> bool {
    FILL_POLICIES.contains(&policy)
}

/// A measure as a proposal writes it (`MeasureOut`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureOut {
    pub value: String,
    pub window_start: String,
    pub window_end: String,
    pub netted_pairs: i64,
    pub breakdown: Option<BTreeMap<String, String>>,
    pub lines: Vec<Lineage>,
}

/// The lineage of one proposed key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyLineage {
    pub mapping_id: String,
    pub measures: Vec<String>,
    pub rule_set_version: i64,
    pub fixture_sha256: String,
    pub netted_pairs: i64,
    pub lines: Vec<Lineage>,
    pub note: Option<String>,
}

/// A key the proposal does not fill, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyReason {
    pub key: String,
    pub reason: String,
}

/// `Proposal`. [`Proposal::to_value`] is the JSON form the vectors hold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proposal {
    pub form_code: FormCode,
    pub tax_year: i64,
    pub period_key: String,
    pub period_start: String,
    pub period_end: String,
    pub as_of: String,
    pub profile_digest: String,
    /// The book's accounting basis (ADR 0008).
    pub accounting_basis: String,
    /// "ready" or "blocked".
    pub status: String,
    pub rule_set_id: String,
    pub rule_set_version: i64,
    pub schema_sha256: String,
    pub schema_pin: String,
    pub catalog_id: String,
    pub catalog_version: i64,
    pub form_context: FormContext,
    pub fixture_sha256: String,
    pub inputs_hash: String,
    pub measures: BTreeMap<String, MeasureOut>,
    pub blocking: Vec<Item>,
    pub warnings: Vec<Item>,
    pub fill_args: BTreeMap<String, String>,
    pub reference_values: BTreeMap<String, String>,
    pub lineage: BTreeMap<String, KeyLineage>,
    pub excluded_keys: Vec<KeyReason>,
    pub refused_keys: Vec<KeyReason>,
    pub output_sha256: String,
}

fn text_map(map: &BTreeMap<String, String>) -> Value {
    Value::Object(
        map.iter()
            .map(|(k, v)| (k.clone(), Value::String(v.clone())))
            .collect(),
    )
}

fn lineage_value(lines: &[Lineage]) -> Value {
    Value::Array(
        lines
            .iter()
            .map(|l| json!({"line_id": l.line_id, "entry_id": l.entry_id, "amount": l.amount}))
            .collect(),
    )
}

fn items_value(items: &[Item]) -> Value {
    Value::Array(
        items
            .iter()
            .map(|i| {
                let mut o = Map::new();
                o.insert("code".into(), Value::String(i.code.clone()));
                o.insert("detail".into(), Value::String(i.detail.clone()));
                if !i.ids.is_empty() {
                    o.insert("ids".into(), json!(i.ids));
                }
                Value::Object(o)
            })
            .collect(),
    )
}

fn reasons_value(keys: &[KeyReason]) -> Value {
    Value::Array(
        keys.iter()
            .map(|k| json!({"key": k.key, "reason": k.reason}))
            .collect(),
    )
}

impl MeasureOut {
    pub fn to_value(&self) -> Value {
        let mut o = Map::new();
        o.insert("value".into(), Value::String(self.value.clone()));
        o.insert(
            "window".into(),
            json!({"start": self.window_start, "end": self.window_end}),
        );
        o.insert("netted_pairs".into(), json!(self.netted_pairs));
        if let Some(b) = &self.breakdown {
            o.insert("breakdown".into(), text_map(b));
        }
        o.insert("lines".into(), lineage_value(&self.lines));
        Value::Object(o)
    }
}

impl KeyLineage {
    pub fn to_value(&self) -> Value {
        let mut o = Map::new();
        o.insert("mapping_id".into(), Value::String(self.mapping_id.clone()));
        o.insert("measures".into(), json!(self.measures));
        o.insert("rule_set_version".into(), json!(self.rule_set_version));
        o.insert(
            "fixture_sha256".into(),
            Value::String(self.fixture_sha256.clone()),
        );
        o.insert("netted_pairs".into(), json!(self.netted_pairs));
        o.insert("lines".into(), lineage_value(&self.lines));
        if let Some(note) = &self.note {
            o.insert("note".into(), Value::String(note.clone()));
        }
        Value::Object(o)
    }
}

impl Proposal {
    /// The proposal without `output_sha256`, the object that digest covers.
    pub fn body_value(&self) -> Value {
        let measures: Map<String, Value> = self
            .measures
            .iter()
            .map(|(k, m)| (k.clone(), m.to_value()))
            .collect();
        let lineage: Map<String, Value> = self
            .lineage
            .iter()
            .map(|(k, l)| (k.clone(), l.to_value()))
            .collect();
        json!({
            "proposal_format": PROPOSAL_FORMAT,
            "form_code": self.form_code.as_str(),
            "tax_year": self.tax_year,
            "period_key": self.period_key,
            "period_start": self.period_start,
            "period_end": self.period_end,
            "as_of": self.as_of,
            "profile_digest": self.profile_digest,
            "accounting_basis": self.accounting_basis,
            "status": self.status,
            "rule_set": {
                "rule_set_id": self.rule_set_id,
                "rule_set_version": self.rule_set_version,
                "schema_sha256": self.schema_sha256,
                "schema_pin": self.schema_pin,
            },
            "measures_catalog": {"catalog_id": self.catalog_id, "version": self.catalog_version},
            "form_context": form_context_to_value(&self.form_context),
            "fixture_sha256": self.fixture_sha256,
            "inputs_hash": self.inputs_hash,
            "measures": measures,
            "blocking": items_value(&self.blocking),
            "warnings": items_value(&self.warnings),
            "fill_args": text_map(&self.fill_args),
            "reference_values": text_map(&self.reference_values),
            "lineage": lineage,
            "excluded_keys": reasons_value(&self.excluded_keys),
            "refused_keys": reasons_value(&self.refused_keys),
        })
    }

    /// The whole proposal as JSON, `output_sha256` included.
    pub fn to_value(&self) -> Value {
        let mut body = self.body_value();
        if let Value::Object(o) = &mut body {
            o.insert(
                "output_sha256".into(),
                Value::String(self.output_sha256.clone()),
            );
        }
        body
    }

    pub fn is_ready(&self) -> bool {
        self.status == "ready"
    }
}

/// Port of `scopeInput()`: the input restricted to one profile. Everything
/// a proposal may read comes through this.
pub fn scope_input(input: &NormInput, tax_profile_id: &str) -> Result<NormInput, EngineError> {
    let profile = input
        .profiles
        .iter()
        .find(|p| p.tax_profile_id == tax_profile_id)
        .ok_or_else(|| {
            EngineError::proposal(
                "no_profile",
                format!("profile {tax_profile_id} is not in the input"),
            )
        })?;
    let entries: Vec<NormEntry> = input
        .entries
        .iter()
        .filter(|e| e.tax_profile_id == tax_profile_id)
        .cloned()
        .collect();
    let ids: HashSet<&str> = entries.iter().map(|e| e.entry_id.as_str()).collect();
    let acks = input
        .acknowledged_entry_ids
        .iter()
        .filter(|id| ids.contains(id.as_str()))
        .cloned()
        .collect();
    Ok(NormInput {
        input_format: input.input_format.clone(),
        as_of: input.as_of.clone(),
        profiles: vec![profile.clone()],
        acknowledged_entry_ids: acks,
        entries,
    })
}

/// Port of `onlyProfileId()`.
pub fn only_profile_id(input: &NormInput) -> Result<String, EngineError> {
    match input.profiles.as_slice() {
        [only] => Ok(only.tax_profile_id.clone()),
        _ => Err(EngineError::proposal(
            "profile_required",
            "pass taxProfileId when the input has several profiles",
        )),
    }
}

fn parse_period(key: &str) -> Result<Period, EngineError> {
    Period::parse(key)
        .ok_or_else(|| EngineError::proposal("bad_period", format!("bad period key {key}")))
}

/// `hashedInputs()`: the period-scoped inputs covered by staleness: the
/// accounting basis, the period status, the acknowledgements, the entries
/// that can affect this period (dated from 1 January up to the period end,
/// or certificates whose period ends in that range, plus the originals of
/// reversals), the Forms context, the rule set, the measures catalogue and
/// the crosswalk.
fn hashed_inputs(
    data: &LoadedData,
    scope: &NormInput,
    form: FormCode,
    period: &Period,
    form_context: &FormContext,
) -> Result<Value, EngineError> {
    let profile = &scope.profiles[0];
    let year = i64::from(period.start.year);
    let year_start = format!("{year:04}-01-01");
    let period_start = period.start.to_string();
    let period_end = period.end.to_string();
    let in_window = |e: &NormEntry| -> bool {
        (e.entry_date >= year_start && e.entry_date <= period_end)
            || e.lines.iter().any(|l| {
                l.period_end
                    .as_deref()
                    .is_some_and(|end| end >= year_start.as_str() && end <= period_end.as_str())
            })
    };
    // Indices into scope.entries: the PoC's Set holds entry objects.
    let mut scoped: Vec<usize> = (0..scope.entries.len())
        .filter(|i| in_window(&scope.entries[*i]))
        .collect();
    for i in scoped.clone() {
        let Some(target) = &scope.entries[i].reverses_entry_id else {
            continue;
        };
        if let Some(original) = scope.entries.iter().position(|e| &e.entry_id == target)
            && !scoped.contains(&original)
        {
            scoped.push(original);
        }
    }
    scoped.sort_by(|a, b| js_cmp(&scope.entries[*a].entry_id, &scope.entries[*b].entry_id));
    let scoped_ids: HashSet<&str> = scoped
        .iter()
        .map(|i| scope.entries[*i].entry_id.as_str())
        .collect();
    let rs = data.rules(form);
    let acks: Vec<&String> = scope
        .acknowledged_entry_ids
        .iter()
        .filter(|id| scoped_ids.contains(id.as_str()))
        .collect();
    Ok(json!({
        "form": form.as_str(),
        "tax_year": year,
        "period_key": period.key,
        "period_start": period_start,
        "period_end": period_end,
        "profile_digest": sha256_hex(&profile.tax_profile_id),
        "accounting_basis": profile.accounting_basis,
        "period_status": profile.periods.get(&period.key).map_or("unset", String::as_str),
        "acknowledged_in_scope": acks,
        "entries": scoped.iter().map(|i| scope.entries[*i].to_value()).collect::<Vec<_>>(),
        "form_context": form_context_to_value(form_context),
        "rule_set": {
            "rule_set_id": rs.rule_set_id,
            "rule_set_version": rs.version,
            "schema_sha256": rs.schema_sha256,
            "mappings_sha256": sha256_of(&rs.raw_mappings)?,
            "form_context_inputs_sha256": sha256_of(&rs.raw_form_context_inputs)?,
        },
        "measures_catalog_sha256": sha256_of(&data.measures)?,
        "crosswalk_sha256": sha256_of(&data.crosswalk(form).raw_fields)?,
    }))
}

/// `JSON.stringify` of a value, for a violation message.
fn quoted(value: &str) -> String {
    Value::String(value.to_string()).to_string()
}

fn violation(cw: &Crosswalk, message: String) -> EngineError {
    EngineError::Crosswalk(format!("{}: {message}", cw.form.as_str()))
}

/// `checkValue()`: the value matches the box's printed type.
fn check_value(cw: &Crosswalk, key: &str, field: &Field, value: &str) -> Result<(), EngineError> {
    let fail = |why: &str| {
        Err(violation(
            cw,
            format!("{key} value {} {why}", quoted(value)),
        ))
    };
    match field.value_type.as_str() {
        "money" => {
            let body = value.strip_prefix('-').unwrap_or(value);
            let ok = body.split_once('.').is_some_and(|(whole, cents)| {
                !whole.is_empty()
                    && whole.bytes().all(|b| b.is_ascii_digit())
                    && cents.len() == 2
                    && cents.bytes().all(|b| b.is_ascii_digit())
            });
            if ok {
                Ok(())
            } else {
                fail("is not two-place decimal text")
            }
        }
        "whole" => {
            if !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()) {
                Ok(())
            } else {
                fail("is not a whole number")
            }
        }
        "choice" => {
            if field
                .options
                .as_ref()
                .is_some_and(|o| o.iter().any(|v| v == value))
            {
                Ok(())
            } else {
                fail("is not one of the printed option values")
            }
        }
        "text" => {
            if value.is_empty() {
                fail("is empty")
            } else {
                Ok(())
            }
        }
        other => fail(&format!(
            "has value_type {other}, which the bridge never sends"
        )),
    }
}

/// `checkCited()`: the governing mapping is one the crosswalk names.
fn check_cited(
    cw: &Crosswalk,
    key: &str,
    field: &Field,
    mapping_id: &str,
) -> Result<(), EngineError> {
    if field.rule_ids.iter().any(|r| r == mapping_id) {
        Ok(())
    } else {
        Err(violation(
            cw,
            format!(
                "{key} is governed by {}, not {mapping_id}",
                field.rule_ids.join("|")
            ),
        ))
    }
}

/// Port of `assertFillKey()`, the crosswalk gate for anything Books sends
/// (BRIDGE.INPUT_ONLY): the key must be a sendable fill key, input-class,
/// with a fill policy, cited by the mapping that governs it, and the value
/// must match the printed type.
pub fn assert_fill_key<'a>(
    cw: &'a Crosswalk,
    key: &str,
    value: &str,
    mapping_id: &str,
) -> Result<&'a Field, EngineError> {
    let field = cw
        .by_fill_key(key)
        .ok_or_else(|| violation(cw, format!("{key} is not a crosswalk fill key")))?;
    if field.fill_class != "input" {
        return Err(violation(
            cw,
            format!("{key} is {}; never proposed", field.fill_class),
        ));
    }
    if !is_fill_policy(&field.books_policy) {
        return Err(violation(
            cw,
            format!("{key} policy is {}; not sendable", field.books_policy),
        ));
    }
    check_cited(cw, key, field, mapping_id)?;
    check_value(cw, key, field, value)?;
    Ok(field)
}

/// `measureOut()`.
fn measure_out(r: &MeasureResult) -> MeasureOut {
    MeasureOut {
        value: money::format(r.value),
        window_start: r.window.start.clone(),
        window_end: r.window.end.clone(),
        netted_pairs: r.netted_pairs,
        breakdown: r.breakdown.as_ref().map(|b| {
            b.iter()
                .map(|(k, v)| (k.clone(), money::format(*v)))
                .collect()
        }),
        lines: crate::bridge::lineage(&r.lines),
    }
}

/// `uniqueCodes()`.
fn unique_codes(outcome: &Outcome) -> String {
    crate::bridge::unique_sorted(outcome.blocking.iter().map(|b| b.code.clone())).join(", ")
}

fn sort_items(items: &[Item]) -> Vec<Item> {
    let mut out = items.to_vec();
    out.sort_by(|a, b| js_cmp(&a.code, &b.code).then_with(|| js_cmp(&a.detail, &b.detail)));
    out
}

/// `assemble()`.
fn assemble(
    data: &LoadedData,
    scope: &NormInput,
    form: FormCode,
    period: &Period,
    form_context: &FormContext,
    outcome: &Outcome,
    inputs_hash: String,
) -> Result<Proposal, EngineError> {
    let cw = data.crosswalk(form);
    let rs = data.rules(form);
    let profile = &scope.profiles[0];
    let blocked = !outcome.blocking.is_empty();
    let values = if blocked {
        &[][..]
    } else {
        &outcome.values[..]
    };
    let input_sha = sha256_of(&scope.to_value())?;
    let group = counted_once(form);

    let mut fill_args = BTreeMap::new();
    let mut reference_values = BTreeMap::new();
    let mut lineage = BTreeMap::new();
    let mut line_owner: BTreeMap<String, String> = BTreeMap::new();
    for v in values {
        if v.mode == "fill" {
            assert_fill_key(cw, &v.key, &v.value, &v.mapping_id)?;
            if group.contains(&v.key.as_str()) {
                for line in &v.lines {
                    if let Some(owner) = line_owner.get(&line.line_id)
                        && *owner != v.key
                    {
                        return Err(EngineError::ledger(
                            "counted_twice",
                            format!("{} feeds both {owner} and {}", line.line_id, v.key),
                        ));
                    }
                    line_owner.insert(line.line_id.clone(), v.key.clone());
                }
            }
            fill_args.insert(v.key.clone(), v.value.clone());
        } else {
            let field = cw
                .by_key(&v.key)
                .filter(|f| f.fill_class == "input" && !is_fill_policy(&f.books_policy))
                .ok_or_else(|| {
                    violation(
                        cw,
                        format!(
                            "{} is not an evidence input; reference values are never sent",
                            v.key
                        ),
                    )
                })?;
            check_cited(cw, &v.key, field, &v.mapping_id)?;
            check_value(cw, &v.key, field, &v.value)?;
            reference_values.insert(v.key.clone(), v.value.clone());
        }
        lineage.insert(
            v.key.clone(),
            KeyLineage {
                mapping_id: v.mapping_id.clone(),
                measures: v.measures.clone(),
                rule_set_version: rs.version,
                fixture_sha256: input_sha.clone(),
                netted_pairs: v.netted_pairs,
                lines: v.lines.clone(),
                note: v.note.clone(),
            },
        );
    }

    let mut excluded_keys = Vec::new();
    for field in &cw.fields {
        let Some(fill_key) = &field.fill_key else {
            continue;
        };
        if !is_fill_policy(&field.books_policy) || fill_args.contains_key(fill_key) {
            continue;
        }
        let reason = if blocked {
            format!("proposal blocked: {}", unique_codes(outcome))
        } else {
            outcome
                .excluded_reasons
                .get(fill_key)
                .cloned()
                .unwrap_or_else(|| "not produced for this period".to_string())
        };
        excluded_keys.push(KeyReason {
            key: fill_key.clone(),
            reason,
        });
    }

    let mut refused_keys = Vec::new();
    for field in &cw.fields {
        let Some(fill_key) = &field.fill_key else {
            continue;
        };
        if is_fill_policy(&field.books_policy) || fill_args.contains_key(fill_key) {
            continue;
        }
        let reason = if field
            .rule_ids
            .iter()
            .any(|r| ELECTION_RULES.contains(&r.as_str()))
        {
            "Forms-owned election input: the bridge may read it as form_context, never fills or decides it (B7: not locked against the profile election)".to_string()
        } else if field.books_policy == "refuse_mvp_spouse" {
            "spouse (B-side) key; refused in MVP (phase P7)".to_string()
        } else if let Some(lock) = field.fill_class.strip_prefix("locked_") {
            format!("Forms {lock} lock; Books never fills it")
        } else if field.books_source == "forms" {
            "Forms decides it (ATC selection, threshold or classification); Books never sends it"
                .to_string()
        } else {
            continue;
        };
        refused_keys.push(KeyReason {
            key: fill_key.clone(),
            reason,
        });
    }
    excluded_keys.sort_by(|a, b| js_cmp(&a.key, &b.key));
    refused_keys.sort_by(|a, b| js_cmp(&a.key, &b.key));

    let measures: BTreeMap<String, MeasureOut> = if blocked {
        BTreeMap::new()
    } else {
        outcome
            .measures
            .iter()
            .map(|(k, r)| (k.clone(), measure_out(r)))
            .collect()
    };

    let mut proposal = Proposal {
        form_code: form,
        tax_year: i64::from(period.start.year),
        period_key: period.key.clone(),
        period_start: period.start.to_string(),
        period_end: period.end.to_string(),
        as_of: scope.as_of.clone(),
        profile_digest: sha256_hex(&profile.tax_profile_id),
        accounting_basis: profile.accounting_basis.clone(),
        status: if blocked { "blocked" } else { "ready" }.to_string(),
        rule_set_id: rs.rule_set_id.clone(),
        rule_set_version: rs.version,
        schema_sha256: rs.schema_sha256.clone(),
        schema_pin: rs.schema_pin.clone(),
        catalog_id: data.measures_catalog_id.clone(),
        catalog_version: data.measures_version,
        form_context: form_context.clone(),
        fixture_sha256: input_sha,
        inputs_hash,
        measures,
        blocking: sort_items(&outcome.blocking),
        warnings: sort_items(&outcome.warnings),
        fill_args,
        reference_values,
        lineage,
        excluded_keys,
        refused_keys,
        output_sha256: String::new(),
    };
    proposal.output_sha256 = sha256_of(&proposal.body_value())?;
    Ok(proposal)
}

/// Port of `buildProposal()`. Same inputs give byte-identical canonical
/// JSON. `form_context` holds the values the bridge reads from the Forms
/// draft (for example the 1701-MS tax option); Books never supplies them
/// itself. `tax_profile_id` may be `None` when the input has one profile.
pub fn build_proposal(
    data: &LoadedData,
    input: &NormInput,
    form: FormCode,
    period_key: &str,
    form_context: &FormContext,
    tax_profile_id: Option<&str>,
) -> Result<Proposal, EngineError> {
    let pid = match tax_profile_id {
        Some(id) => id.to_string(),
        None => only_profile_id(input)?,
    };
    let scope = scope_input(input, &pid)?;
    let period = parse_period(period_key)?;
    if period.kind != form.period_kind() {
        let kind = match form.period_kind() {
            crate::date::PeriodKind::Quarter => "quarter",
            crate::date::PeriodKind::Month => "month",
            crate::date::PeriodKind::Year => "year",
        };
        return Err(EngineError::proposal(
            "period_kind",
            format!("{} needs a {kind} period; got {period_key}", form.as_str()),
        ));
    }
    let inputs_hash = sha256_of(&hashed_inputs(data, &scope, form, &period, form_context)?)?;
    let ledger = post_ledger(&scope.entries)?;
    let acks: HashSet<String> = scope.acknowledged_entry_ids.iter().cloned().collect();
    let outcome = run_bridge(&BridgeCtx {
        data,
        form,
        profile: &scope.profiles[0],
        ledger: &ledger,
        period: &period,
        acks: &acks,
        form_context,
    })?;
    assemble(
        data,
        &scope,
        form,
        &period,
        form_context,
        &outcome,
        inputs_hash,
    )
}

/// [`build_proposal`] from JSON: a normalized input (such as a vector's
/// `input`) and a Forms context object, with the form code as text.
pub fn build_proposal_from_value(
    data: &LoadedData,
    input: &Value,
    form: &str,
    period_key: &str,
    form_context: &Value,
    tax_profile_id: Option<&str>,
) -> Result<Proposal, EngineError> {
    let form = FormCode::parse(form).ok_or_else(|| {
        EngineError::proposal("bad_form", format!("{form} is not a form the bridge maps"))
    })?;
    let input = NormInput::from_value(input)?;
    let ctx = crate::input::form_context_from_value(form_context)?;
    build_proposal(data, &input, form, period_key, &ctx, tax_profile_id)
}

/// Port of `computeInputsHash()`: the period-scoped input hash alone. It
/// does not validate the ledger, so it works on a changed input too.
pub fn compute_inputs_hash(
    data: &LoadedData,
    input: &NormInput,
    form: FormCode,
    period_key: &str,
    form_context: &FormContext,
    tax_profile_id: Option<&str>,
) -> Result<String, EngineError> {
    let pid = match tax_profile_id {
        Some(id) => id.to_string(),
        None => only_profile_id(input)?,
    };
    let scope = scope_input(input, &pid)?;
    let period = parse_period(period_key)?;
    Ok(sha256_of(&hashed_inputs(
        data,
        &scope,
        form,
        &period,
        form_context,
    )?)?)
}

/// Freshness of a stored proposal against the current inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Freshness {
    Fresh,
    Stale,
}

/// Port of `freshness()`: fresh when the inputs still match the stored
/// proposal, stale when any covered input changed. `form_context` is the
/// Forms draft's current context; `None` means the one the proposal was
/// built with.
pub fn freshness(
    stored: &Proposal,
    data: &LoadedData,
    input: &NormInput,
    form_context: Option<&FormContext>,
    tax_profile_id: Option<&str>,
) -> Result<Freshness, EngineError> {
    let ctx = form_context.unwrap_or(&stored.form_context);
    let current = compute_inputs_hash(
        data,
        input,
        stored.form_code,
        &stored.period_key,
        ctx,
        tax_profile_id,
    )?;
    Ok(if current == stored.inputs_hash {
        Freshness::Fresh
    } else {
        Freshness::Stale
    })
}

/// Port of `assertFillable()`: fill is refused unless the proposal is ready
/// and fresh. Returns the fill arguments when it is allowed.
pub fn assert_fillable<'a>(
    stored: &'a Proposal,
    data: &LoadedData,
    input: &NormInput,
    form_context: Option<&FormContext>,
    tax_profile_id: Option<&str>,
) -> Result<&'a BTreeMap<String, String>, EngineError> {
    if !stored.is_ready() {
        return Err(EngineError::proposal(
            "not_ready",
            format!("proposal is {}; fill refused", stored.status),
        ));
    }
    if freshness(stored, data, input, form_context, tax_profile_id)? != Freshness::Fresh {
        return Err(EngineError::proposal(
            "stale",
            "the inputs changed after this proposal was made; regenerate it before any fill",
        ));
    }
    Ok(&stored.fill_args)
}
