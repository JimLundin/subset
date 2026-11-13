//! Names gathered from every checked file before any file is checked, so a
//! rule can tell our own types and functions from a library's. They are
//! names, not resolved paths: a library type that shares a name with one of
//! ours counts as ours.

use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use proc_macro2::Span;
use syn::{Expr, Type};

use crate::checks::{self, Scan};
use crate::syntax::Extent;
use crate::walk::{Owner, Within};

/// What one file's walk reports for the walks of every file.
pub(crate) enum Fact {
    /// A struct, enum, union, trait or type alias.
    Type(String),
    /// A struct, enum, union or type alias defined in this file.
    Defined(String),
    Enum(String),
    Mod(String),
    /// A free fn.
    Fn(String),
    /// A type named as the `E` of some `Result<T, E>`.
    ErrorType(String),
    /// An actor's loop state (proposals 0007, 0008).
    LoopState(String),
    /// A struct or enum, with every name in the types of its fields.
    Fields(String, Vec<String>),
    /// A newtype that derives `Ord` (proposal 0024).
    OrdNewtype(String),
    /// A type that implements syn's `Visit` (proposal 0045).
    Visitor(String),
    /// A type with serde derives (proposal 0043).
    Wire(String),
    Use(UsePath),
    /// A struct with named fields, by its module path, with its `&str` fields.
    Struct {
        key: Vec<String>,
        strings: Vec<String>,
    },
    /// A `const` whose type is a slice or array.
    Table {
        name: String,
        element: Element,
    },
    /// A struct built outside a table's literal, by module path where the
    /// path resolves in the file, and by name.
    Built {
        key: Option<Vec<String>>,
        name: String,
    },
    /// A `cfg` fn, impl or use.
    Cfg(CfgItem),
}

/// One path a `use` item brings in, with the name it binds. A glob has none.
#[derive(Clone)]
pub(crate) struct UsePath {
    pub(crate) segments: Vec<String>,
    pub(crate) name: Option<String>,
    /// The span of the path's last segment, or of the `*`.
    pub(crate) span: Span,
}

/// What a `const` table's rows hold, and what a keyed lookup compares.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Element {
    /// A tuple row, with the indices of its `&str` fields.
    Tuple(BTreeSet<String>),
    /// A row struct's `&str` fields, by name or index.
    Fields(BTreeSet<String>),
    /// A row struct, by module path.
    Row(Vec<String>),
    /// The rows themselves, in a table of `&str`.
    Str,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CfgKind {
    Fn,
    Impl,
    Use,
}

/// A `cfg` item that needs a counterpart: same kind and name, the other
/// platform's predicate in its lexical container and logical impl owner.
/// Every `use` counts as one name within its container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CfgItem {
    pub(crate) kind: CfgKind,
    pub(crate) name: String,
    pub(crate) predicate: String,
    pub(crate) module: Vec<String>,
    pub(crate) block: Option<Extent>,
    pub(crate) owner: Option<Rc<CfgOwner>>,
}

/// A logical impl owner, including its qualified type and trait arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CfgOwner {
    pub(crate) self_ty: Type,
    pub(crate) trait_path: Option<syn::Path>,
    pub(crate) type_params: usize,
    pub(crate) lifetime_params: usize,
    pub(crate) path_blocks: Vec<Vec<Extent>>,
}

#[derive(Clone)]
pub(crate) struct CfgName {
    pub(crate) module: Vec<String>,
    pub(crate) blocks: Vec<Extent>,
    pub(crate) name: String,
    pub(crate) target: Option<Vec<String>>,
    pub(crate) absolute: Within,
    pub(crate) kind: CfgBindingKind,
}

#[derive(Clone, Copy)]
pub(crate) enum CfgBindingKind {
    Module,
    Type,
    Import,
}

/// The module, enclosing blocks, and pre-gathered bindings of one owner.
#[derive(Clone)]
pub(crate) struct CfgScope {
    pub(crate) module: Vec<String>,
    pub(crate) blocks: Vec<Extent>,
    pub(crate) names: Rc<Vec<CfgName>>,
}

impl CfgItem {
    pub(crate) fn new(kind: CfgKind, name: String) -> CfgItem {
        CfgItem {
            kind,
            name,
            predicate: String::new(),
            module: Vec::new(),
            block: None,
            owner: None,
        }
    }
}

pub(crate) struct Facts {
    /// Our crates' names: the directory above each file's `src`, `tests` or
    /// `examples`.
    pub(crate) crates: BTreeSet<String>,
    pub(crate) mods: BTreeSet<String>,
    pub(crate) types: BTreeSet<String>,
    pub(crate) fns: BTreeSet<String>,
    pub(crate) error_types: BTreeSet<String>,
    pub(crate) loop_states: BTreeSet<String>,
    /// Types of ours a loop state owns: named in the type of one of its
    /// fields, or in a variant payload of an enum of ours it owns.
    pub(crate) owned_by_state: BTreeSet<String>,
    pub(crate) ord_newtypes: BTreeSet<String>,
    pub(crate) wire: BTreeSet<String>,
    /// Types that implement syn's `Visit`, whose methods may take `&mut
    /// self` as an endpoint's do (proposal 0045).
    pub(crate) visitors: BTreeSet<String>,
}

const CRATE_DIRS: &[&str] = &["src", "tests", "examples"];
const ROOTS: &[&str] = &["crate", "self", "super", "Self"];
const PATH_ROOTS: &[&str] = &["crate", "self", "super"];

/// The kinds of fact that name one thing.
#[derive(Clone, Copy)]
enum Kind {
    Type,
    Enum,
    Mod,
    Fn,
    ErrorType,
    LoopState,
    OrdNewtype,
    Wire,
    Defined,
    Visitor,
}

/// The names facts of one kind give.
fn names(all: &[&Fact], kind: Kind) -> BTreeSet<String> {
    all.iter()
        .filter_map(|f| name_of(f, kind))
        .cloned()
        .collect()
}

fn name_of(fact: &Fact, kind: Kind) -> Option<&String> {
    match (fact, kind) {
        (Fact::Type(n), Kind::Type)
        | (Fact::Enum(n), Kind::Enum)
        | (Fact::Mod(n), Kind::Mod)
        | (Fact::Fn(n), Kind::Fn)
        | (Fact::ErrorType(n), Kind::ErrorType)
        | (Fact::LoopState(n), Kind::LoopState)
        | (Fact::OrdNewtype(n), Kind::OrdNewtype)
        | (Fact::Wire(n), Kind::Wire)
        | (Fact::Defined(n), Kind::Defined)
        | (Fact::Visitor(n), Kind::Visitor) => Some(n),
        _ => None,
    }
}

impl Facts {
    pub(crate) fn none() -> Facts {
        Facts::gather(&Vec::new())
    }

    /// The facts of every file, each given with its path.
    pub(crate) fn gather(files: &[(Vec<String>, Vec<Fact>)]) -> Facts {
        let all = files
            .iter()
            .flat_map(|(_, facts)| facts)
            .collect::<Vec<&Fact>>();
        let types = names(&all, Kind::Type);
        let enums = names(&all, Kind::Enum);
        let loop_states = names(&all, Kind::LoopState);
        let fields = all
            .iter()
            .filter_map(|f| match f {
                Fact::Fields(owner, names) => Some((owner, names)),
                _ => None,
            })
            .fold(
                BTreeMap::<String, BTreeSet<String>>::new(),
                |mut map, (owner, names)| {
                    map.entry(owner.clone())
                        .or_insert_with(BTreeSet::new)
                        .extend(names.iter().cloned());
                    map
                },
            );
        let direct = loop_states
            .iter()
            .filter_map(|state| fields.get(state))
            .flatten()
            .filter(|name| types.contains(name.as_str()))
            .cloned()
            .collect::<BTreeSet<String>>();
        let frontier = direct
            .iter()
            .filter(|n| enums.contains(n.as_str()))
            .cloned()
            .collect();
        Facts {
            crates: files
                .iter()
                .filter_map(|(path, _)| {
                    path.iter()
                        .zip(path.iter().skip(1))
                        .find(|(_, next)| CRATE_DIRS.contains(&next.as_str()))
                        .map(|(name, _)| name.clone())
                })
                .collect(),
            mods: names(&all, Kind::Mod),
            fns: names(&all, Kind::Fn),
            error_types: names(&all, Kind::ErrorType),
            owned_by_state: owned(frontier, direct, &fields, &enums, &types),
            ord_newtypes: names(&all, Kind::OrdNewtype),
            wire: names(&all, Kind::Wire),
            visitors: names(&all, Kind::Visitor),
            types,
            loop_states,
        }
    }

    /// Whether a path names something of ours: a path from `crate`, `self`,
    /// `super`, one of our crates or modules, a single name we define, or an
    /// associated item of a type we define.
    pub(crate) fn owner_of(&self, segments: &[String]) -> Option<()> {
        let first = segments.first()?;
        let rooted = ROOTS
            .contains(&first.as_str())
            .then_some(())
            .or(self.crates.get(first).and(Some(())))
            .or(self.mods.get(first).and(Some(())));
        match (segments.len(), rooted) {
            (1, _) => self.types.get(first).or(self.fns.get(first)).and(Some(())),
            (_, Some(_)) => Some(()),
            (2, None) => self.types.get(first).and(Some(())),
            _ => None,
        }
    }

    pub(crate) fn owner(&self, segments: &[String]) -> Owner {
        self.owner_of(segments)
            .map_or(Owner::Theirs, |_| Owner::Ours)
    }

    /// `crate`, `self`, `super`, one of our crates or one of our modules.
    fn root(&self, first: &str) -> Option<()> {
        PATH_ROOTS
            .contains(&first)
            .then_some(())
            .or(self.crates.get(first).and(Some(())))
            .or(self.mods.get(first).and(Some(())))
    }
}

/// The types an enum of ours owns through its payloads, wherever the enum
/// is owned: an enum is transparent, as `Option` is.
fn owned(
    frontier: Vec<String>,
    found: BTreeSet<String>,
    fields: &BTreeMap<String, BTreeSet<String>>,
    enums: &BTreeSet<String>,
    types: &BTreeSet<String>,
) -> BTreeSet<String> {
    match frontier.split_first() {
        None => found,
        Some((name, rest)) => {
            let fresh = fields
                .get(name)
                .into_iter()
                .flatten()
                .filter(|p| types.contains(p.as_str()) && !found.contains(p.as_str()))
                .cloned()
                .collect::<Vec<String>>();
            let next = rest
                .iter()
                .cloned()
                .chain(fresh.iter().filter(|p| enums.contains(p.as_str())).cloned())
                .collect();
            let found = found.into_iter().chain(fresh).collect();
            owned(next, found, fields, enums, types)
        }
    }
}

/// The names one file defines and imports.
pub(crate) struct FileNames {
    /// Structs, enums, unions and type aliases defined in the file.
    pub(crate) defined: BTreeSet<String>,
    /// Names imported from a path of ours.
    pub(crate) ours: BTreeSet<String>,
    /// Names imported from a path we don't define.
    pub(crate) foreign: BTreeSet<String>,
    /// The file has a glob import of a path of ours, as `use super::*`.
    pub(crate) our_glob: Within,
    pub(crate) uses: Vec<UsePath>,
}

impl FileNames {
    pub(crate) fn of(facts: &[Fact], all: &Facts) -> FileNames {
        let uses = facts
            .iter()
            .filter_map(|f| match f {
                Fact::Use(u) => Some(u.clone()),
                _ => None,
            })
            .collect::<Vec<UsePath>>();
        FileNames {
            defined: facts
                .iter()
                .filter_map(|f| name_of(f, Kind::Defined))
                .cloned()
                .collect(),
            ours: use_names(&uses, all, Owner::Ours),
            foreign: use_names(&uses, all, Owner::Theirs),
            our_glob: uses
                .iter()
                .find(|u| u.name.is_none() && rooted(u, all).is_some())
                .map_or(Within::Out, |_| Within::In),
            uses,
        }
    }

    /// Whether a path that starts with `first` starts from something of
    /// ours in this file.
    pub(crate) fn rooted_in_ours(&self, first: &str, facts: &Facts) -> Option<()> {
        match self.foreign.get(first) {
            Some(_) => None,
            None => self.ours.get(first).and(Some(())).or(facts.root(first)),
        }
    }

    /// Whether a bare type name in this file is a type of ours: imported
    /// from a path of ours, defined here, or reachable through a glob of
    /// ours. A name imported from a library never is.
    pub(crate) fn bare_type_is_ours(&self, name: &str, facts: &Facts) -> Option<()> {
        let reachable = self
            .ours
            .get(name)
            .or(self.defined.get(name))
            .and(Some(()))
            .or(match self.our_glob {
                Within::In => Some(()),
                Within::Out => None,
            });
        match (facts.types.get(name), self.foreign.get(name)) {
            (Some(_), None) => reachable,
            _ => None,
        }
    }

    /// The methods of the listed operator traits this file imports, by name
    /// or through a glob of the trait's module (proposal 0038).
    pub(crate) fn operator_methods(&self, operators: &[Operator]) -> BTreeSet<String> {
        operators
            .iter()
            .filter(|o| self.uses.iter().any(|u| imports(o, u).is_some()))
            .flat_map(|o| o.methods.iter().cloned())
            .collect()
    }
}

/// A `use` path that starts from something of ours.
fn rooted(u: &UsePath, all: &Facts) -> Option<()> {
    u.segments.first().and_then(|first| all.root(first))
}

/// The names a file imports from paths of ours, or from paths of others.
fn use_names(uses: &[UsePath], all: &Facts, owner: Owner) -> BTreeSet<String> {
    uses.iter()
        .filter(|u| match owner {
            Owner::Ours => rooted(u, all).is_some(),
            Owner::Theirs => rooted(u, all).is_none(),
        })
        .filter_map(|u| u.name.clone())
        .collect()
}

/// Whether a `use` path brings an operator trait into scope.
fn imports(o: &Operator, u: &UsePath) -> Option<()> {
    match &u.name {
        Some(_) => {
            let last = u.segments.len().checked_sub(1);
            checks::found_in(&o.module, &o.name, &u.segments)
                .filter(|i| last.as_ref() == Some(i))
                .and(Some(()))
        }
        None => u
            .segments
            .first()
            .filter(|_| o.module.ends_with(&checks::std_root(u.segments.clone())))
            .and(Some(())),
    }
}

/// A trait or type the config names by path, as `std::ops::Add`, with the
/// methods it gives.
pub(crate) struct Operator {
    pub(crate) module: Vec<String>,
    pub(crate) name: String,
    pub(crate) methods: Vec<String>,
}

impl Operator {
    pub(crate) fn parse(path: &str, methods: Vec<String>) -> Operator {
        let segments = checks::std_root(path.split("::").map(ToString::to_string).collect());
        let (name, module) = segments
            .split_last()
            .map(|(name, module)| (name.clone(), module.to_vec()))
            .unwrap_or_default();
        Operator {
            module,
            name,
            methods,
        }
    }
}

/// The lists in the config that the walk reads.
pub(crate) struct Lists {
    pub(crate) derives: Vec<String>,
    pub(crate) macros: Vec<String>,
    pub(crate) test_macros: Vec<String>,
    /// Types of ours whose methods may take `&mut self` anywhere (0035).
    pub(crate) endpoints: Vec<String>,
    /// Methods whose closure may have a boolean body (0028, 0037).
    pub(crate) predicate_adapters: Vec<String>,
    /// Methods that may take a closure, besides the predicate adapters.
    pub(crate) closure_methods: Vec<String>,
    /// The std `Iterator` methods (`[weights] adapters`).
    pub(crate) adapters: Vec<String>,
    /// Operator traits never named or called by name (0038).
    pub(crate) operators: Vec<Operator>,
    /// Types and traits never named (0039).
    pub(crate) readers: Vec<Operator>,
    /// Each construct id's meaning, the reason a violation gives.
    pub(crate) reasons: BTreeMap<String, String>,
    pub(crate) primitives: Vec<Type>,
    pub(crate) untyped: Vec<Type>,
    pub(crate) transparent: Vec<String>,
}

/// The `const` tables of a file and the row types they hold (proposal 0042).
pub(crate) struct Tables {
    /// Each table's name, with what a keyed lookup compares in it.
    pub(crate) keys: BTreeMap<String, Element>,
    /// The row structs, by module path: each is the element type of a
    /// `const` table and is built only as an element of a table's literal.
    pub(crate) rows: BTreeSet<Vec<String>>,
}

impl Tables {
    pub(crate) fn of(facts: &[Fact]) -> Tables {
        let structs = facts
            .iter()
            .filter_map(|f| match f {
                Fact::Struct { key, strings } => Some((key.clone(), strings.clone())),
                _ => None,
            })
            .collect::<BTreeMap<Vec<String>, Vec<String>>>();
        let tables = facts
            .iter()
            .filter_map(|f| match f {
                Fact::Table { name, element } => Some((name, element)),
                _ => None,
            })
            .collect::<Vec<(&String, &Element)>>();
        let (outside, outside_names) = facts
            .iter()
            .filter_map(|f| match f {
                Fact::Built { key, name } => Some((key, name)),
                _ => None,
            })
            .fold(
                (BTreeSet::<Vec<String>>::new(), BTreeSet::<String>::new()),
                |(keys, names), (key, name)| match key
                    .as_ref()
                    .filter(|k| structs.contains_key(k.as_slice()))
                {
                    Some(k) => (keys.into_iter().chain(Some(k.clone())).collect(), names),
                    None => (keys, names.into_iter().chain(Some(name.clone())).collect()),
                },
            );
        let rows = structs
            .keys()
            .filter(|key| {
                tables
                    .iter()
                    .any(|(_, e)| e == &&Element::Row(key.to_vec()))
            })
            .filter(|key| {
                !outside.contains(key.as_slice())
                    && !outside_names.contains(key.last().map_or("", String::as_str))
            })
            .cloned()
            .collect();
        let keys = tables
            .iter()
            .filter_map(|(name, element)| {
                let keys = match element {
                    Element::Tuple(fields) => Some(Element::Fields(fields.clone())),
                    Element::Row(key) => structs
                        .get(key)
                        .map(|strings| Element::Fields(strings.iter().cloned().collect())),
                    Element::Str => Some(Element::Str),
                    _ => None,
                }?;
                match &keys {
                    Element::Fields(fields) => {
                        fields.first().map(|_| (name.to_string(), keys.clone()))
                    }
                    _ => Some((name.to_string(), keys)),
                }
            })
            .collect();
        Tables { keys, rows }
    }

    /// The table that `expr` reads: a `const` table by name, or a name bound
    /// to one in `bindings`, behind `&`, parentheses and the methods that
    /// pass rows through or `enumerate`.
    pub(crate) fn scanned(&self, expr: &Expr, bindings: &BTreeMap<String, Expr>) -> Option<Scan> {
        match expr {
            Expr::MethodCall(e) => match e.method.to_string().as_str() {
                "enumerate" => self
                    .scanned(e.receiver.as_ref(), bindings)
                    .map(|inner| Scan {
                        enumerated: Within::In,
                        ..inner
                    }),
                method => PASS_THROUGH
                    .contains(&method)
                    .then(|| self.scanned(e.receiver.as_ref(), bindings))
                    .flatten(),
            },
            Expr::Reference(e) => self.scanned(e.expr.as_ref(), bindings),
            Expr::Paren(e) => self.scanned(e.expr.as_ref(), bindings),
            Expr::Path(e) => {
                let bound = e
                    .path
                    .get_ident()
                    .and_then(|i| bindings.get(&i.to_string()));
                match bound {
                    Some(init) => self.scanned(init, &BTreeMap::new()),
                    None => {
                        let name = e.path.segments.last()?.ident.to_string();
                        self.keys.get(&name).map(|keys| Scan {
                            keys: keys.clone(),
                            enumerated: Within::Out,
                        })
                    }
                }
            }
            _ => None,
        }
    }
}

/// Methods between a table and its predicate that pass rows through.
const PASS_THROUGH: &[&str] = &[
    "iter",
    "into_iter",
    "copied",
    "cloned",
    "rev",
    "skip",
    "take",
    "step_by",
    "filter",
    "skip_while",
    "take_while",
    "by_ref",
];
