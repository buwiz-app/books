//! The versioned data the bridge reads, compiled in so the binary and the
//! PoC read the same bytes: the measures catalogue
//! (`data/measures.v1.json`), the mapping rule sets (`data/rules/`), the
//! crosswalks (`data/crosswalk/`), the kind catalogue and the closed
//! vocabularies of line facts. Port of `poc/src/load.ts` `loadData()`,
//! `vocabFrom()` and `indexCrosswalk()`, with the same pin checks: a rule
//! set and its crosswalk name the same schema digest, and every rule set
//! reads the loaded measures catalogue.
//!
//! There is no rate, threshold or due-date file: Books loads no compliance
//! data (ADR 0007). The raw JSON of the catalogue, the mappings, the Forms
//! context inputs and the crosswalk fields is kept, because the staleness
//! hash is taken over it.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::OnceLock;

use serde_json::{Map, Value};

use crate::engine_error::EngineError;
use crate::input::FormCode;

const MEASURES: &str = include_str!("../../../data/measures.v1.json");
const COMMON: &str = include_str!("../../../data/rules/common.rules.json");
const CATEGORIES: &str = include_str!("../../../data/categories.v1.json");
const ATTRIBUTES: &str = include_str!("../../../data/tax-attributes.v1.json");
const RULES_2551Q: &str = include_str!("../../../data/rules/2551q-v2018.rules.json");
const RULES_1601C: &str = include_str!("../../../data/rules/1601c-v2018.rules.json");
const RULES_1701MS: &str = include_str!("../../../data/rules/1701ms-v2024.rules.json");
const CROSSWALK_2551Q: &str = include_str!("../../../data/crosswalk/2551q-v2018.json");
const CROSSWALK_1601C: &str = include_str!("../../../data/crosswalk/1601c-v2018.json");
const CROSSWALK_1701MS: &str = include_str!("../../../data/crosswalk/1701ms-v2024.json");

/// Closed vocabularies (`Vocab`), from `data/tax-attributes.v1.json` and the
/// kind catalogue.
#[derive(Debug, Clone)]
pub struct Vocab {
    pub measure_side: HashSet<String>,
    pub income_class: HashSet<String>,
    pub contra_kind: HashSet<String>,
    pub document_kind: HashSet<String>,
    pub person_scope: HashSet<String>,
    pub comp_component: HashSet<String>,
    pub contribution_side: HashSet<String>,
    pub contribution_scheme: HashSet<String>,
    pub credit_kind: HashSet<String>,
    pub certificate_atc: HashSet<String>,
    pub certificate_status: HashSet<String>,
    pub final_tax_withheld: HashSet<String>,
    pub account: HashSet<String>,
    /// Trade receivable and payable kinds (`accrual_only`). A cash-basis
    /// book refuses them.
    pub accrual_only_account: HashSet<String>,
}

/// One crosswalk entry (`Field`), reduced to what the bridge reads.
#[derive(Debug, Clone)]
pub struct Field {
    pub key: Option<String>,
    pub key_pattern: Option<String>,
    pub fill_key: Option<String>,
    pub fill_class: String,
    pub books_policy: String,
    pub books_source: String,
    pub rule_ids: Vec<String>,
    pub value_type: String,
    /// The printed option values, when the box has options.
    pub options: Option<Vec<String>>,
}

/// A form's crosswalk (`Crosswalk`), indexed by key and by fill key.
#[derive(Debug, Clone)]
pub struct Crosswalk {
    pub form: FormCode,
    pub schema_file: String,
    pub schema_sha256: String,
    pub schema_pin: String,
    pub fields: Vec<Field>,
    /// The `fields` array exactly as loaded (hashed for staleness).
    pub raw_fields: Value,
    by_key: HashMap<String, usize>,
    by_fill_key: HashMap<String, usize>,
}

impl Crosswalk {
    pub fn by_key(&self, key: &str) -> Option<&Field> {
        self.by_key.get(key).map(|i| &self.fields[*i])
    }

    pub fn by_fill_key(&self, key: &str) -> Option<&Field> {
        self.by_fill_key.get(key).map(|i| &self.fields[*i])
    }
}

/// `FormContextCondition`: a mapping applies when the Forms value at
/// `read` equals `equals`.
#[derive(Debug, Clone)]
pub struct FormContextCondition {
    pub read: String,
    pub equals: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aggregation {
    Sum,
    ChoiceYesIfNonzero,
    /// 1601C.II.17, the one input-level tax rule in the bridge.
    Nontaxable13thMonthCap,
}

/// One measure-to-input mapping (`Mapping`).
#[derive(Debug, Clone)]
pub struct Mapping {
    pub id: String,
    pub printed_item: String,
    /// "fill" or "reference".
    pub mode: String,
    pub target_key: String,
    pub measures: Vec<String>,
    pub aggregation: Aggregation,
    pub aggregation_params: Option<BTreeMap<String, String>>,
    pub period_basis: String,
    pub person_scope: String,
    /// "Q1-Q3" or "Q4" when the mapping reads part of the year.
    pub sub_window: Option<String>,
    pub form_context: Vec<FormContextCondition>,
    pub non_negative: bool,
    /// "open" or "settled".
    pub review_status: String,
    pub review_note: String,
    /// "implemented" or "fail_closed".
    pub poc: String,
    pub fail_closed_code: Option<String>,
    pub input_level_tax_rule: Option<String>,
}

/// A Forms value a rule set reads (`FormContextInput`).
#[derive(Debug, Clone)]
pub struct FormContextInput {
    pub path: String,
    pub source: String,
    pub supported_values: Vec<String>,
    pub required: bool,
    pub required_when: Option<BTreeMap<String, String>>,
}

/// A mapping rule set (`RuleSet`).
#[derive(Debug, Clone)]
pub struct RuleSet {
    pub rule_set_id: String,
    pub version: i64,
    pub schema_sha256: String,
    pub schema_pin: String,
    pub measures_catalog: String,
    pub form_context_inputs: Vec<FormContextInput>,
    pub mappings: Vec<Mapping>,
    /// `mappings` and `form_context_inputs` exactly as loaded.
    pub raw_mappings: Value,
    pub raw_form_context_inputs: Value,
}

/// Everything `loadData()` returns.
#[derive(Debug, Clone)]
pub struct LoadedData {
    /// `data/measures.v1.json` exactly as loaded (hashed for staleness).
    pub measures: Value,
    pub measures_catalog_id: String,
    pub measures_version: i64,
    pub common: Value,
    pub categories: Value,
    pub attributes: Value,
    rules: [RuleSet; 3],
    crosswalk: [Crosswalk; 3],
    pub vocab: Vocab,
}

fn slot(form: FormCode) -> usize {
    match form {
        FormCode::F2551Q => 0,
        FormCode::F1601C => 1,
        FormCode::F1701MS => 2,
    }
}

impl LoadedData {
    pub fn rules(&self, form: FormCode) -> &RuleSet {
        &self.rules[slot(form)]
    }

    pub fn crosswalk(&self, form: FormCode) -> &Crosswalk {
        &self.crosswalk[slot(form)]
    }

    /// The `poc` status of a catalogue measure ("implemented" or
    /// "planned"), or `None` when the catalogue has no such measure.
    pub fn measure_status(&self, id: &str) -> Option<&str> {
        self.measures
            .get("measures")?
            .as_array()?
            .iter()
            .find(|m| m.get("id").and_then(Value::as_str) == Some(id))?
            .get("poc")?
            .as_str()
    }
}

fn bad(what: &str) -> EngineError {
    EngineError::Data(format!("data: {what}"))
}

fn parse(text: &str, file: &str) -> Result<Value, EngineError> {
    serde_json::from_str(text).map_err(|e| bad(&format!("{file} is not JSON: {e}")))
}

fn obj<'a>(v: &'a Value, what: &str) -> Result<&'a Map<String, Value>, EngineError> {
    v.as_object()
        .ok_or_else(|| bad(&format!("{what} is not an object")))
}

fn text(o: &Map<String, Value>, key: &str, what: &str) -> Result<String, EngineError> {
    o.get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| bad(&format!("{what}.{key} is not text")))
}

fn opt_text(o: &Map<String, Value>, key: &str, what: &str) -> Result<Option<String>, EngineError> {
    match o.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(bad(&format!("{what}.{key} is not text"))),
    }
}

fn texts(v: Option<&Value>, what: &str) -> Result<Vec<String>, EngineError> {
    v.and_then(Value::as_array)
        .ok_or_else(|| bad(&format!("{what} is not a list")))?
        .iter()
        .map(|s| {
            s.as_str()
                .map(str::to_string)
                .ok_or_else(|| bad(&format!("{what} holds a non-text value")))
        })
        .collect()
}

fn text_map(
    v: Option<&Value>,
    what: &str,
) -> Result<Option<BTreeMap<String, String>>, EngineError> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Object(map)) => map
            .iter()
            .map(|(k, v)| {
                v.as_str()
                    .map(|s| (k.clone(), s.to_string()))
                    .ok_or_else(|| bad(&format!("{what}.{k} is not text")))
            })
            .collect::<Result<_, _>>()
            .map(Some),
        Some(_) => Err(bad(&format!("{what} is not an object"))),
    }
}

fn integer(o: &Map<String, Value>, key: &str, what: &str) -> Result<i64, EngineError> {
    o.get(key)
        .and_then(Value::as_i64)
        .ok_or_else(|| bad(&format!("{what}.{key} is not an integer")))
}

/// Port of `vocabFrom()`. Descriptive allowed values (for example "opaque
/// id (never a TIN, never a name)") are dropped by the `[A-Za-z0-9_]+` test.
fn vocab_from(attributes: &Value, categories: &Value) -> Result<Vocab, EngineError> {
    let specs = attributes
        .get("line_attributes")
        .and_then(Value::as_array)
        .ok_or_else(|| bad("tax-attributes.v1.json has no line_attributes"))?;
    let allowed = |name: &str| -> Result<HashSet<String>, EngineError> {
        let spec = specs
            .iter()
            .find(|a| a.get("name").and_then(Value::as_str) == Some(name))
            .ok_or_else(|| bad(&format!("tax-attributes.v1.json has no attribute {name}")))?;
        Ok(texts(spec.get("allowed_values"), name)?
            .into_iter()
            .filter(|v| !v.is_empty() && v.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'))
            .collect())
    };
    let kinds = categories
        .get("categories")
        .and_then(Value::as_array)
        .ok_or_else(|| bad("categories.v1.json has no categories"))?;
    let mut account = HashSet::new();
    let mut accrual_only_account = HashSet::new();
    for kind in kinds {
        let id = text(obj(kind, "category")?, "id", "category")?;
        if kind.get("accrual_only") == Some(&Value::Bool(true)) {
            accrual_only_account.insert(id.clone());
        }
        account.insert(id);
    }
    Ok(Vocab {
        measure_side: allowed("measure_side")?,
        income_class: allowed("income_class")?,
        contra_kind: allowed("contra_kind")?,
        document_kind: allowed("document_kind")?,
        person_scope: allowed("person_scope")?,
        comp_component: allowed("comp_component")?,
        contribution_side: allowed("contribution_side")?,
        contribution_scheme: allowed("contribution_scheme")?,
        credit_kind: allowed("credit_kind")?,
        certificate_atc: allowed("certificate_atc")?,
        certificate_status: allowed("certificate_status")?,
        final_tax_withheld: allowed("final_tax_withheld")?,
        account,
        accrual_only_account,
    })
}

fn field_from(v: &Value, form: &str) -> Result<Field, EngineError> {
    let what = format!("{form} crosswalk field");
    let o = obj(v, &what)?;
    let options = match o.get("options") {
        None | Some(Value::Null) => None,
        Some(Value::Array(items)) => Some(
            items
                .iter()
                .filter_map(|opt| opt.get("value").and_then(Value::as_str).map(str::to_string))
                .collect(),
        ),
        Some(_) => return Err(bad(&format!("{what}.options is not a list"))),
    };
    Ok(Field {
        key: opt_text(o, "key", &what)?,
        key_pattern: opt_text(o, "key_pattern", &what)?,
        fill_key: opt_text(o, "fill_key", &what)?,
        fill_class: text(o, "fill_class", &what)?,
        books_policy: text(o, "books_policy", &what)?,
        books_source: text(o, "books_source", &what)?,
        rule_ids: texts(o.get("rule_ids"), &format!("{what}.rule_ids"))?,
        value_type: text(o, "value_type", &what)?,
        options,
    })
}

/// Port of `indexCrosswalk()`.
fn index_crosswalk(form: FormCode, raw: &Value) -> Result<Crosswalk, EngineError> {
    let name = form.as_str();
    let o = obj(raw, name)?;
    let raw_fields = o
        .get("fields")
        .filter(|f| f.is_array())
        .cloned()
        .ok_or_else(|| bad(&format!("{name}: crosswalk has no fields")))?;
    let mut fields = Vec::new();
    let mut by_key = HashMap::new();
    let mut by_fill_key = HashMap::new();
    for (i, v) in raw_fields.as_array().into_iter().flatten().enumerate() {
        let field = field_from(v, name)?;
        let id = field
            .key
            .clone()
            .or_else(|| field.key_pattern.clone())
            .ok_or_else(|| {
                bad(&format!(
                    "{name}: crosswalk entry has neither key nor key_pattern"
                ))
            })?;
        if by_key.insert(id.clone(), i).is_some() {
            return Err(bad(&format!("{name}: duplicate crosswalk key {id}")));
        }
        if let Some(fill_key) = &field.fill_key
            && by_fill_key.insert(fill_key.clone(), i).is_some()
        {
            return Err(bad(&format!("{name}: duplicate fill_key {fill_key}")));
        }
        fields.push(field);
    }
    Ok(Crosswalk {
        form,
        schema_file: text(o, "schema_file", name)?,
        schema_sha256: text(o, "schema_sha256", name)?,
        schema_pin: text(o, "source_commit", name)?,
        fields,
        raw_fields,
        by_key,
        by_fill_key,
    })
}

fn mapping_from(v: &Value) -> Result<Mapping, EngineError> {
    let o = obj(v, "mapping")?;
    let id = text(o, "id", "mapping")?;
    let what = format!("mapping {id}");
    let aggregation = match text(o, "aggregation", &what)?.as_str() {
        "sum" => Aggregation::Sum,
        "choice_yes_if_nonzero" => Aggregation::ChoiceYesIfNonzero,
        "nontaxable_13th_month_cap" => Aggregation::Nontaxable13thMonthCap,
        other => return Err(bad(&format!("{what}: unknown aggregation {other}"))),
    };
    let form_context = match o.get("form_context") {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Array(items)) => items
            .iter()
            .map(|c| {
                let c = obj(c, &what)?;
                Ok(FormContextCondition {
                    read: text(c, "read", &what)?,
                    equals: text(c, "equals", &what)?,
                })
            })
            .collect::<Result<_, EngineError>>()?,
        Some(_) => return Err(bad(&format!("{what}.form_context is not a list"))),
    };
    let review = obj(
        o.get("review").unwrap_or(&Value::Null),
        &format!("{what}.review"),
    )?;
    let non_negative = match o.get("non_negative") {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(_) => return Err(bad(&format!("{what}.non_negative is not a boolean"))),
    };
    Ok(Mapping {
        printed_item: text(o, "printed_item", &what)?,
        mode: text(o, "mode", &what)?,
        target_key: text(o, "target_key", &what)?,
        measures: texts(o.get("measures"), &format!("{what}.measures"))?,
        aggregation,
        aggregation_params: text_map(
            o.get("aggregation_params"),
            &format!("{what}.aggregation_params"),
        )?,
        period_basis: text(o, "period_basis", &what)?,
        person_scope: text(o, "person_scope", &what)?,
        sub_window: opt_text(o, "sub_window", &what)?,
        form_context,
        non_negative,
        review_status: text(review, "status", &what)?,
        review_note: text(review, "note", &what)?,
        poc: text(o, "poc", &what)?,
        fail_closed_code: opt_text(o, "fail_closed_code", &what)?,
        input_level_tax_rule: opt_text(o, "input_level_tax_rule", &what)?,
        id,
    })
}

fn rule_set_from(raw: &Value, form: FormCode) -> Result<RuleSet, EngineError> {
    let name = form.as_str();
    let o = obj(raw, name)?;
    let raw_mappings = o
        .get("mappings")
        .filter(|m| m.is_array())
        .cloned()
        .ok_or_else(|| bad(&format!("{name}: rule set has no mappings")))?;
    let raw_form_context_inputs = o
        .get("form_context_inputs")
        .filter(|m| m.is_array())
        .cloned()
        .ok_or_else(|| bad(&format!("{name}: rule set has no form_context_inputs")))?;
    let mappings = raw_mappings
        .as_array()
        .into_iter()
        .flatten()
        .map(mapping_from)
        .collect::<Result<_, _>>()?;
    let form_context_inputs = raw_form_context_inputs
        .as_array()
        .into_iter()
        .flatten()
        .map(|v| {
            let i = obj(v, "form_context_input")?;
            Ok(FormContextInput {
                path: text(i, "path", "form_context_input")?,
                source: text(i, "source", "form_context_input")?,
                supported_values: texts(i.get("supported_values"), "supported_values")?,
                required: i.get("required") == Some(&Value::Bool(true)),
                required_when: text_map(i.get("required_when"), "required_when")?,
            })
        })
        .collect::<Result<_, EngineError>>()?;
    Ok(RuleSet {
        rule_set_id: text(o, "rule_set_id", name)?,
        version: integer(o, "version", name)?,
        schema_sha256: text(o, "schema_sha256", name)?,
        schema_pin: text(o, "schema_pin", name)?,
        measures_catalog: text(o, "measures_catalog", name)?,
        form_context_inputs,
        mappings,
        raw_mappings,
        raw_form_context_inputs,
    })
}

fn sources(form: FormCode) -> (&'static str, &'static str) {
    match form {
        FormCode::F2551Q => (RULES_2551Q, CROSSWALK_2551Q),
        FormCode::F1601C => (RULES_1601C, CROSSWALK_1601C),
        FormCode::F1701MS => (RULES_1701MS, CROSSWALK_1701MS),
    }
}

/// Port of `loadData()`: parses every compiled-in data file and checks
/// that the rule and crosswalk pins agree.
pub fn load_data() -> Result<LoadedData, EngineError> {
    let measures = parse(MEASURES, "measures.v1.json")?;
    let common = parse(COMMON, "rules/common.rules.json")?;
    let categories = parse(CATEGORIES, "categories.v1.json")?;
    let attributes = parse(ATTRIBUTES, "tax-attributes.v1.json")?;
    let catalog = obj(&measures, "measures.v1.json")?;
    let measures_catalog_id = text(catalog, "catalog_id", "measures.v1.json")?;
    let measures_version = integer(catalog, "version", "measures.v1.json")?;
    let mut rules = Vec::new();
    let mut crosswalk = Vec::new();
    for form in FormCode::ALL {
        let (rules_text, crosswalk_text) = sources(form);
        let rule_set = rule_set_from(&parse(rules_text, form.as_str())?, form)?;
        let cw = index_crosswalk(form, &parse(crosswalk_text, form.as_str())?)?;
        if rule_set.schema_sha256 != cw.schema_sha256 {
            return Err(bad(&format!(
                "{}: rules and crosswalk point at different schema digests",
                form.as_str()
            )));
        }
        if rule_set.measures_catalog != measures_catalog_id {
            return Err(bad(&format!(
                "{}: rule set reads {}, catalog is {}",
                form.as_str(),
                rule_set.measures_catalog,
                measures_catalog_id
            )));
        }
        rules.push(rule_set);
        crosswalk.push(cw);
    }
    let vocab = vocab_from(&attributes, &categories)?;
    let rules: [RuleSet; 3] = rules.try_into().map_err(|_| bad("three rule sets"))?;
    let crosswalk: [Crosswalk; 3] = crosswalk.try_into().map_err(|_| bad("three crosswalks"))?;
    Ok(LoadedData {
        measures,
        measures_catalog_id,
        measures_version,
        common,
        categories,
        attributes,
        rules,
        crosswalk,
        vocab,
    })
}

/// The compiled-in data, loaded once per process.
pub fn shared() -> Result<&'static LoadedData, EngineError> {
    static DATA: OnceLock<Result<LoadedData, EngineError>> = OnceLock::new();
    DATA.get_or_init(load_data).as_ref().map_err(Clone::clone)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_with_agreeing_pins() {
        let data = load_data().unwrap();
        assert_eq!(data.measures_catalog_id, "measures-v1");
        for form in FormCode::ALL {
            assert_eq!(
                data.rules(form).schema_sha256,
                data.crosswalk(form).schema_sha256
            );
        }
        assert!(data.vocab.account.contains("bank_accounts"));
        assert!(!data.vocab.document_kind.is_empty());
        assert!(
            data.crosswalk(FormCode::F2551Q)
                .by_fill_key("txtATCAmt1")
                .is_some()
        );
        assert_eq!(
            data.measure_status("certificates_2316_received"),
            Some("planned")
        );
    }
}
