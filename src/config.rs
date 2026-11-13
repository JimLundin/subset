//! `subset.toml`, read by hand from its TOML table: the allowed ids, the
//! scopes, the lists the rules read and the score's weights.

use std::collections::{BTreeMap, BTreeSet};

use toml::{Table, Value};

use crate::facts::{Lists, Operator};
use crate::table;
use crate::{Fatal, display};

pub(crate) struct Config {
    pub(crate) include: Vec<String>,
    pub(crate) allowed: BTreeSet<String>,
    pub(crate) scopes: Vec<Scope>,
    pub(crate) lists: std::rc::Rc<Lists>,
    pub(crate) weights: Option<Weights>,
}

/// Ids allowed only in the files under `paths`. Elsewhere a use of one is
/// reported with `rule` as its reason.
pub(crate) struct Scope {
    pub(crate) rule: String,
    pub(crate) paths: Vec<glob::Pattern>,
    pub(crate) allowed: BTreeSet<String>,
}

/// How the subset regards a construct. `Outside` is any violation, and
/// `Boundary` any construct whose shape an item we don't define fixes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tier {
    Encouraged,
    Neutral,
    Tolerated,
    Costly,
    Outside,
    Boundary,
}

pub(crate) const TIERS: &[Tier] = &[
    Tier::Encouraged,
    Tier::Neutral,
    Tier::Tolerated,
    Tier::Costly,
    Tier::Outside,
    Tier::Boundary,
];

impl Tier {
    pub(crate) fn name(&self) -> &str {
        match self {
            Tier::Encouraged => "encouraged",
            Tier::Neutral => "neutral",
            Tier::Tolerated => "tolerated",
            Tier::Costly => "costly",
            Tier::Outside => "outside",
            Tier::Boundary => "boundary",
        }
    }

    fn parse(name: &str) -> Option<Tier> {
        TIERS.iter().find(|t| t.name() == name).copied()
    }

    /// A tier the boundary context relieves where an argument, field value
    /// or type argument of an item we don't define stands in it.
    pub(crate) fn restricted(&self) -> Option<()> {
        match self {
            Tier::Tolerated | Tier::Costly => Some(()),
            _ => None,
        }
    }
}

pub(crate) struct Weights {
    /// Each tier's weight, in the order of `TIERS`.
    pub(crate) tiers: Vec<f64>,
    /// The tier of each allowed id. An allowed id missing here is neutral.
    pub(crate) ids: BTreeMap<String, Tier>,
    /// Globs of the modules that mirror a protocol someone else defines.
    pub(crate) external: Vec<glob::Pattern>,
}

impl Weights {
    pub(crate) fn weight(&self, tier: Tier) -> f64 {
        TIERS
            .iter()
            .zip(&self.tiers)
            .find(|(t, _)| t == &&tier)
            .map_or(0.0, |(_, w)| w.to_owned())
    }

    pub(crate) fn tier(&self, id: &str) -> Tier {
        self.ids.get(id).copied().unwrap_or(Tier::Neutral)
    }
}

const KEYS: &[&str] = &[
    "version",
    "include",
    "allowed",
    "derives",
    "macros",
    "test_macros",
    "endpoints",
    "predicate_adapters",
    "closure_methods",
    "fallible_operators",
    "consuming_readers",
    "scope",
    "weights",
];
const SCOPE_KEYS: &[&str] = &["rule", "paths", "allowed"];
const WEIGHT_KEYS: &[&str] = &[
    "tiers",
    "ids",
    "adapters",
    "primitives",
    "transparent",
    "untyped",
    "external_protocols",
];

/// The filtering adapters of proposal 0028: the default `predicate_adapters`.
const PREDICATE_ADAPTERS: &[&str] = &[
    "filter",
    "find",
    "any",
    "all",
    "position",
    "take_while",
    "skip_while",
    "retain",
];
/// The standard library's `Iterator`, `Option` and `Result` methods that take
/// a closure: the default `closure_methods`.
const CLOSURE_METHODS: &[&str] = &[
    "and_then",
    "filter_map",
    "find_map",
    "flat_map",
    "fold",
    "for_each",
    "get_or_insert_with",
    "inspect",
    "inspect_err",
    "is_err_and",
    "is_none_or",
    "is_ok_and",
    "is_some_and",
    "map",
    "map_err",
    "map_or",
    "map_or_else",
    "map_while",
    "max_by",
    "max_by_key",
    "min_by",
    "min_by_key",
    "ok_or_else",
    "or_else",
    "partition",
    "reduce",
    "rfind",
    "rposition",
    "scan",
    "try_fold",
    "try_for_each",
    "unwrap_or_else",
];
/// The standard library's iterator methods: the default `adapters`.
pub(crate) const ITERATOR_ADAPTERS: &[&str] = &[
    "all",
    "any",
    "chain",
    "cloned",
    "collect",
    "copied",
    "count",
    "cycle",
    "enumerate",
    "filter",
    "filter_map",
    "find",
    "find_map",
    "flat_map",
    "flatten",
    "fold",
    "fuse",
    "inspect",
    "last",
    "map",
    "map_while",
    "max",
    "max_by",
    "max_by_key",
    "min",
    "min_by",
    "min_by_key",
    "nth",
    "partition",
    "peekable",
    "position",
    "product",
    "reduce",
    "rev",
    "rposition",
    "scan",
    "skip",
    "skip_while",
    "step_by",
    "sum",
    "take",
    "take_while",
    "try_fold",
    "unzip",
    "zip",
];
/// The standard library's primitive data: text, numbers and bytes.
const PRIMITIVES: &[&str] = &[
    "String", "str", "char", "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64",
    "u128", "usize", "f32", "f64", "Vec<u8>",
];
/// The standard library's ways to hold data without a type.
const UNTYPED: &[&str] = &["HashMap<String, _>", "BTreeMap<String, _>", "dyn Any"];
/// The standard library's containers, which typing sees through.
const TRANSPARENT: &[&str] = &[
    "Option", "Result", "Vec", "VecDeque", "Box", "Rc", "Arc", "Cow", "HashMap", "HashSet",
    "BTreeMap", "BTreeSet",
];
/// The `std::ops` traits whose operation can overflow or panic.
const OPERATORS: &[&str] = &["Add", "Sub", "Mul", "Div", "Rem", "Neg", "Shl", "Shr"];

pub(crate) fn load(path: &std::path::Path) -> Result<Config, Fatal> {
    let name = display(path);
    let text = crate::read(path)?;
    let table = text
        .parse::<Table>()
        .map_err(|e| Fatal(format!("{name}: {e}")))?;
    parse(&table).map_err(|e| Fatal(format!("{name}: {e}")))
}

fn parse(table: &Table) -> Result<Config, String> {
    unknown(table, KEYS, "")?;
    table
        .get("version")
        .ok_or_else(|| "missing field `version`".to_string())?;
    let weights = table
        .get("weights")
        .map(|w| weights(w.as_table().ok_or("`weights` is not a table")?))
        .transpose()?;
    let adapters = match table.get("weights").and_then(|w| w.get("adapters")) {
        Some(list) => strings_of(list, "adapters")?,
        None => defaults(ITERATOR_ADAPTERS),
    };
    let lists = std::rc::Rc::new(Lists {
        derives: list(table, "derives", &[])?,
        macros: list(table, "macros", &[])?,
        test_macros: list(table, "test_macros", &[])?,
        endpoints: list(table, "endpoints", &[])?,
        predicate_adapters: list(table, "predicate_adapters", PREDICATE_ADAPTERS)?,
        closure_methods: list(table, "closure_methods", CLOSURE_METHODS)?,
        adapters,
        operators: operators(table.get("fallible_operators"))?,
        readers: list(table, "consuming_readers", &["bytes::Buf"])?
            .iter()
            .map(|path| Operator::parse(path, Vec::new()))
            .collect(),
        reasons: table::IDS
            .iter()
            .map(|r| (r.id.to_string(), r.meaning.to_string()))
            .collect(),
        primitives: types(&weight_list(table, "primitives", PRIMITIVES)?)?,
        untyped: types(&weight_list(table, "untyped", UNTYPED)?)?,
        transparent: weight_list(table, "transparent", TRANSPARENT)?,
    });
    let scopes = match table.get("scope") {
        Some(Value::Array(scopes)) => scopes
            .iter()
            .map(scope)
            .collect::<Result<Vec<Scope>, String>>()?,
        Some(_) => Err("`scope` is not an array of tables".to_string())?,
        None => Vec::new(),
    };
    Ok(Config {
        include: required(table, "include")?,
        allowed: required(table, "allowed")?.into_iter().collect(),
        scopes,
        lists,
        weights,
    })
}

/// An error naming the first key of `table` that isn't in `keys`.
fn unknown(table: &Table, keys: &[&str], within: &str) -> Result<(), String> {
    match table.keys().find(|k| !keys.contains(&k.as_str())) {
        Some(key) => Err(format!("unknown field `{within}{key}`")),
        None => Ok(()),
    }
}

fn defaults(list: &[&str]) -> Vec<String> {
    list.iter().map(ToString::to_string).collect()
}

fn strings_of(value: &Value, key: &str) -> Result<Vec<String>, String> {
    value
        .as_array()
        .ok_or_else(|| format!("`{key}` is not a list"))?
        .iter()
        .map(|v| {
            v.as_str()
                .map(ToString::to_string)
                .ok_or_else(|| format!("`{key}` holds a value that is not a string"))
        })
        .collect()
}

fn list(table: &Table, key: &str, default: &[&str]) -> Result<Vec<String>, String> {
    table
        .get(key)
        .map(|v| strings_of(v, key))
        .unwrap_or_else(|| Ok(defaults(default)))
}

/// A list in `[weights]`, or its default.
fn weight_list(table: &Table, key: &str, default: &[&str]) -> Result<Vec<String>, String> {
    table
        .get("weights")
        .and_then(|w| w.get(key))
        .map(|v| strings_of(v, key))
        .unwrap_or_else(|| Ok(defaults(default)))
}

fn required(table: &Table, key: &str) -> Result<Vec<String>, String> {
    strings_of(
        table
            .get(key)
            .ok_or_else(|| format!("missing field `{key}`"))?,
        key,
    )
}

pub(crate) fn globs(patterns: &[String]) -> Result<Vec<glob::Pattern>, String> {
    patterns
        .iter()
        .map(|p| glob::Pattern::new(p).map_err(|e| format!("path {p}: {e}")))
        .collect()
}

fn scope(value: &Value) -> Result<Scope, String> {
    let table = value
        .as_table()
        .ok_or("`scope` holds a value that is not a table")?;
    unknown(table, SCOPE_KEYS, "scope.")?;
    Ok(Scope {
        rule: table
            .get("rule")
            .and_then(Value::as_str)
            .ok_or("missing field `rule`")?
            .to_string(),
        paths: globs(&required(table, "paths")?)?,
        allowed: required(table, "allowed")?.into_iter().collect(),
    })
}

fn operators(value: Option<&Value>) -> Result<Vec<Operator>, String> {
    match value {
        Some(v) => v
            .as_table()
            .ok_or("`fallible_operators` is not a table")?
            .iter()
            .map(|(path, methods)| Ok(Operator::parse(path, strings_of(methods, path)?)))
            .collect(),
        None => Ok(OPERATORS
            .iter()
            .flat_map(|name| {
                let method = name.to_lowercase();
                let plain =
                    Operator::parse(&format!("std::ops::{name}"), Vec::from([method.clone()]));
                let assign = Operator::parse(
                    &format!("std::ops::{name}Assign"),
                    Vec::from([format!("{method}_assign")]),
                );
                match name.to_owned() {
                    "Neg" => Vec::from([plain]),
                    _ => Vec::from([plain, assign]),
                }
            })
            .collect()),
    }
}

fn types(patterns: &[String]) -> Result<Vec<syn::Type>, String> {
    patterns
        .iter()
        .map(|p| syn::parse_str(p).map_err(|e| format!("[weights] type pattern {p}: {e}")))
        .collect()
}

fn weights(table: &Table) -> Result<Weights, String> {
    unknown(table, WEIGHT_KEYS, "weights.")?;
    let tiers = table
        .get("tiers")
        .and_then(Value::as_table)
        .ok_or("missing field `tiers`")?;
    unknown(
        tiers,
        &TIERS.iter().map(Tier::name).collect::<Vec<&str>>(),
        "weights.tiers.",
    )?;
    let weights = TIERS
        .iter()
        .map(|t| match tiers.get(t.name()) {
            Some(Value::Float(f)) => Ok(f.to_owned()),
            Some(Value::Integer(i)) => i32::try_from(i.to_owned())
                .map(f64::from)
                .map_err(|e| e.to_string()),
            _ => Err(format!("missing field `{}`", t.name())),
        })
        .collect::<Result<Vec<f64>, String>>()?;
    let ids = match table.get("ids") {
        Some(Value::Table(ids)) => ids
            .iter()
            .map(|(id, tier)| {
                tier.as_str()
                    .and_then(Tier::parse)
                    .map(|t| (id.clone(), t))
                    .ok_or_else(|| format!("unknown tier for `{id}`"))
            })
            .collect::<Result<BTreeMap<String, Tier>, String>>()?,
        _ => Err("missing field `ids`".to_string())?,
    };
    Ok(Weights {
        tiers: weights,
        ids,
        external: globs(&list(table, "external_protocols", &[])?)?,
    })
}
