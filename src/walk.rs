//! One walk over a parsed file. It gives every construct its id, checks the
//! shape rules where the syntax shows them, notes what the score needs, and
//! reports the facts other files' walks read. syn's `Visit` drives it
//! (proposal 0045): each override handles its node and lets syn visit the
//! children, which read the slot their parent gave them by their extent.

use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use proc_macro2::Span;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{
    Attribute, BinOp, Expr, Fields, FnArg, GenericArgument, GenericParam, Item, Lit, Pat, Stmt,
    Type, TypeParamBound, UnOp, Visibility,
};

use crate::checks;
use crate::facts::{CfgItem, CfgKind, Element, Fact, Facts, FileNames, Lists, Tables, UsePath};
use crate::syntax::{self, Extent, Pos};
use crate::table::{self, Rule};

/// What the walk reports about one node.
pub(crate) enum Hit {
    /// The construct a node is.
    Id(Occurrence),
    /// A shape rule the node breaks.
    Flag(Finding),
    /// A node whose shape an item we don't define fixes.
    Mark(Mark),
    /// An id the score names more finely.
    Refine(Refinement),
    /// A node whose tokens all cost its own weight.
    Fold(Extent),
    /// A field of a wire type in an external protocol module.
    Wire(Extent),
    /// A type in a field or signature of our own.
    Typed(Typed),
    /// A fn the score rolls tokens up into.
    Function(FnSpan),
    /// A name other files' walks need.
    Fact(Fact),
}

#[derive(Debug, Clone)]
pub(crate) struct Occurrence {
    pub(crate) id: String,
    pub(crate) reason: String,
    pub(crate) at: Pos,
    /// The whole construct, from its first token to its last.
    pub(crate) extent: Extent,
}

pub(crate) struct Finding {
    pub(crate) rule: Rule,
    pub(crate) at: Pos,
}

pub(crate) struct Mark {
    pub(crate) extent: Extent,
    pub(crate) fixed: Fixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Fixed {
    /// The node exists only because a library demands it.
    Always,
    /// The node is an argument, field value or type argument of an item we
    /// don't define; the score relieves only restricted tiers there.
    IfRestricted,
}

pub(crate) struct Refinement {
    pub(crate) extent: Extent,
    /// The id refined, or `None` for any type id but the lifetimes.
    pub(crate) from: Option<String>,
    pub(crate) to: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TypeClass {
    Own,
    Primitive,
    Untyped,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Place {
    Field,
    Signature,
}

pub(crate) struct Typed {
    pub(crate) extent: Extent,
    pub(crate) class: TypeClass,
    pub(crate) place: Place,
}

pub(crate) struct FnSpan {
    pub(crate) name: String,
    pub(crate) line: usize,
    pub(crate) extent: Extent,
}

/// Two states of a context flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Within {
    In,
    Out,
}

/// Whose item a path names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Owner {
    Ours,
    Theirs,
}

/// The syntactic slot a node fills in its parent.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Slot {
    Other,
    CallArg(Callee),
    /// A temporary array may stand directly before a method call (0047).
    MethodReceiver,
    MethodArg {
        method: String,
        receiver: Owner,
    },
    FieldValue {
        owner: Owner,
        wire: Within,
    },
    ClosureBody(Box<Slot>),
    ConstInit,
    ConstType,
    Param,
    RefTarget(Box<Slot>),
    GenericArg {
        segment: String,
        owner: Owner,
    },
    MainReturn,
    ClapField,
    /// A field type of a `const` table's row struct (proposal 0042).
    RowField,
    /// A field type of a serde wire type (proposal 0043).
    WireField,
    /// The `build` parameter of an `ask` helper (proposal 0026).
    AskBuild,
    /// A local closure whose uses stay in its block (proposal 0046).
    LocalClosure,
    LocalClosureParam,
    /// A `..` directly in a slice pattern, or bound there by `@` (0047).
    SliceRest,
    /// A literal directly lent mutably or used to initialize `let mut`.
    MutableArray,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Callee {
    pub(crate) path: Vec<String>,
    pub(crate) owner: Owner,
}

impl Callee {
    pub(crate) fn last(&self) -> &str {
        self.path.last().map_or("", String::as_str)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ImplKind {
    Inherent,
    /// A trait on the 0009 list, whose signatures may demand generics.
    Library,
    OtherTrait,
}

pub(crate) struct Imp {
    /// The last segment of the self type, if it names one.
    pub(crate) self_ty: Option<String>,
    pub(crate) kind: ImplKind,
    pub(crate) trait_owner: Owner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Receiver {
    None,
    Shared,
    Owned,
    OwnedMut,
    RefMut,
}

/// What the rules need to know about the fn being walked.
pub(crate) struct Func {
    pub(crate) receiver: Receiver,
    /// The extent of the fn's actor loop, if its body is one.
    pub(crate) actor_loop: Option<Extent>,
    /// The extent of the loop's `let mut` state.
    pub(crate) state: Option<Extent>,
    pub(crate) state_names: Vec<String>,
    /// Plain names assigned with `=` anywhere in the body.
    pub(crate) assigned: BTreeSet<String>,
    /// Each parameter bound to a plain name, with its type's last segment.
    pub(crate) params: BTreeMap<String, String>,
    /// Names bound by `let name = Some(..)`.
    pub(crate) some_bindings: BTreeSet<String>,
    /// Names bound to a stream that starts at `stream::iter`.
    pub(crate) stream_bindings: BTreeSet<String>,
    /// Names bound to a `const` table or a chain over one (proposal 0042).
    pub(crate) table_bindings: BTreeMap<String, Expr>,
}

/// The innermost loop around a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Loop {
    None,
    Actor,
    Other,
}

/// The item whose fields a field walk is in, as typing sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ItemKind {
    Other,
    Newtype,
    Wire,
}

/// Whose generics a generics walk is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Generics {
    Item,
    Fn,
    /// A struct, enum or inherent impl carrying typed domain data (0052).
    Data,
    /// An `ask` helper's (proposal 0026).
    Ask,
}

/// The context a node stands in, as the checks read it.
#[derive(Clone)]
pub(crate) struct Cx {
    pub(crate) slot: Slot,
    /// The node is part of a predicate's boolean tree (proposal 0028).
    pub(crate) predicate: Within,
    /// Inside the body of a predicate closure.
    pub(crate) in_predicate: Within,
    /// The node is iterated, where a range built by name may stand.
    pub(crate) iterated: Within,
    pub(crate) test: Within,
    pub(crate) in_return: Within,
    pub(crate) mutable_type: Within,
    /// Inside the pattern of a `let` already reported as `stmt.let.mut`.
    pub(crate) let_mut: Within,
    pub(crate) in_let: Within,
    pub(crate) imp: Option<Rc<Imp>>,
    pub(crate) func: Option<Rc<Func>>,
    pub(crate) lp: Loop,
    pub(crate) place: Option<Place>,
    /// Whether the score's typing notes the types walked.
    pub(crate) typing: Within,
    pub(crate) item: ItemKind,
    pub(crate) module: Rc<Vec<String>>,
    pub(crate) generics: Generics,
}

impl Cx {
    /// The impl walked, when its trait is one we don't define.
    pub(crate) fn foreign(&self) -> Option<&Imp> {
        self.imp
            .as_deref()
            .filter(|i| matches!(i.trait_owner, Owner::Theirs))
    }

    /// The impl walked, when its trait is on the 0009 list.
    pub(crate) fn library(&self) -> Option<&Imp> {
        self.imp
            .as_deref()
            .filter(|i| matches!(i.kind, ImplKind::Library))
    }
}

/// Where an attribute stands, for the rules on `cfg` and `allow`.
#[derive(Clone)]
pub(crate) enum Host {
    /// An item that may carry `cfg`, needing a counterpart when it names one.
    Cfg(Option<CfgItem>),
    /// `#![allow(clippy::useless_format)]` atop a crate root.
    RootAllow,
    Field {
        wire: Within,
        error: Within,
    },
    Variant {
        wire: Within,
        unit: Within,
        tagged: Within,
    },
    Other,
}

/// What one file's walk reads: facts across files and the file's own.
pub(crate) struct Env {
    pub(crate) facts: Rc<Facts>,
    pub(crate) lists: Rc<Lists>,
    pub(crate) names: FileNames,
    /// The methods of the fallible operator traits this file imports.
    pub(crate) operator_methods: BTreeSet<String>,
    pub(crate) cfg_items: Vec<CfgItem>,
    pub(crate) tables: Tables,
    pub(crate) test_file: Within,
    pub(crate) crate_root: Within,
    /// The file mirrors a protocol someone else defines.
    pub(crate) external: Within,
}

pub(crate) fn flag(at: Span, rule: Rule) -> Hit {
    Hit::Flag(Finding {
        rule,
        at: syntax::pos(at),
    })
}

pub(crate) fn mark(node: Span, fixed: Fixed) -> Hit {
    Hit::Mark(Mark {
        extent: syntax::extent(node),
        fixed,
    })
}

/// Concatenates the hits of a node's parts in order.
pub(crate) fn join(parts: Vec<Vec<Hit>>) -> Vec<Hit> {
    parts.into_iter().flatten().collect()
}

/// One hit when `found` holds a value.
pub(crate) fn when(found: Option<()>, hit: Hit) -> Vec<Hit> {
    found.map(|_| hit).into_iter().collect()
}

/// One hit when `found` is empty.
pub(crate) fn unless(found: Option<()>, hit: Hit) -> Vec<Hit> {
    match found {
        Some(_) => Vec::new(),
        None => Vec::from([hit]),
    }
}

/// Whether a path names something of ours, as `Facts::owner` reads it.
pub(crate) fn ours(path: &syn::Path, env: &Env) -> Owner {
    env.facts.owner(&syntax::segments(path))
}

/// Attributes of the language. Every other attribute is a library's.
const BUILTIN_ATTRS: &[&str] = &[
    "allow",
    "automatically_derived",
    "cfg",
    "cfg_attr",
    "cold",
    "deny",
    "deprecated",
    "derive",
    "doc",
    "expect",
    "forbid",
    "ignore",
    "inline",
    "macro_export",
    "must_use",
    "non_exhaustive",
    "path",
    "repr",
    "should_panic",
    "test",
    "track_caller",
    "warn",
];
/// Derives of the standard library; any other derive is a library's.
const STD_DERIVES: &[&str] = &[
    "Clone",
    "Copy",
    "Debug",
    "Default",
    "Eq",
    "Hash",
    "Ord",
    "PartialEq",
    "PartialOrd",
];
/// Macros of the standard library. Every other macro is a library's.
const STD_MACROS: &[&str] = &[
    "assert",
    "assert_eq",
    "assert_ne",
    "cfg",
    "column",
    "compile_error",
    "concat",
    "dbg",
    "debug_assert",
    "debug_assert_eq",
    "debug_assert_ne",
    "env",
    "eprint",
    "eprintln",
    "file",
    "format",
    "format_args",
    "include_bytes",
    "include_str",
    "line",
    "matches",
    "panic",
    "print",
    "println",
    "stringify",
    "todo",
    "unimplemented",
    "unreachable",
    "vec",
    "write",
    "writeln",
];
/// Trait impls the 0009 list allows.
const LIBRARY_TRAITS: &[&str] = &[
    "Drop",
    "Debug",
    "Serialize",
    "Deserialize",
    "ProtocolHandler",
    "Visit",
];
const INTERIOR_MUT: &[&str] = &[
    "Cell",
    "RefCell",
    "OnceCell",
    "OnceLock",
    "UnsafeCell",
    "Mutex",
    "RwLock",
];

/// The hits of one file.
pub(crate) fn file(file: &syn::File, env: Env) -> Vec<Hit> {
    let mut walk = Walk {
        cx: Cx {
            slot: Slot::Other,
            predicate: Within::Out,
            in_predicate: Within::Out,
            iterated: Within::Out,
            test: env.test_file,
            in_return: Within::Out,
            mutable_type: Within::Out,
            let_mut: Within::Out,
            in_let: Within::Out,
            imp: None,
            func: None,
            lp: Loop::None,
            place: None,
            typing: Within::In,
            item: ItemKind::Other,
            module: Rc::new(Vec::new()),
            generics: Generics::Item,
        },
        env,
        hits: Vec::new(),
        slots: BTreeMap::new(),
        predicates: BTreeSet::new(),
        iterated: BTreeSet::new(),
        builds: BTreeSet::new(),
        statics: BTreeSet::new(),
        hosts: BTreeMap::new(),
    };
    walk.visit_file(file);
    walk.hits
}

/// A node's identity in the side tables: the bytes it covers.
fn key(span: Span) -> (usize, usize) {
    let bytes = span.byte_range();
    (bytes.start, bytes.end)
}

struct Walk {
    env: Env,
    hits: Vec<Hit>,
    /// The context of the subtree being walked.
    cx: Cx,
    /// The slot each expression or type fills, by its extent.
    slots: BTreeMap<(usize, usize), Slot>,
    /// The nodes of predicates' boolean trees, by extent.
    predicates: BTreeSet<(usize, usize)>,
    /// The expressions iterated, where a range built by name may stand.
    iterated: BTreeSet<(usize, usize)>,
    /// Struct literals that are rows of a `const` table of their own type.
    builds: BTreeSet<(usize, usize)>,
    /// The `'static` of each `&'static str`, which 0042 checks, not 0009.
    statics: BTreeSet<(usize, usize)>,
    /// Where each attribute that may carry `cfg` stands.
    hosts: BTreeMap<(usize, usize), Host>,
}

impl Walk {
    /// The context of the node at `node`.
    fn at(&self, node: Span) -> Cx {
        let at = key(node);
        Cx {
            slot: self.slots.get(&at).cloned().unwrap_or(Slot::Other),
            predicate: self.predicates.get(&at).map_or(Within::Out, |_| Within::In),
            iterated: self.iterated.get(&at).map_or(Within::Out, |_| Within::In),
            ..self.cx.clone()
        }
    }

    fn id(&mut self, at: Span, node: Span, name: &str) {
        let reason = self
            .env
            .lists
            .reasons
            .get(name)
            .cloned()
            .unwrap_or_default();
        self.named(at, node, name.to_string(), &reason);
    }

    fn named(&mut self, at: Span, node: Span, name: String, reason: &str) {
        self.hits.push(Hit::Id(Occurrence {
            id: name,
            reason: reason.to_string(),
            at: syntax::pos(at),
            extent: syntax::extent(node),
        }));
    }

    fn fact(&mut self, fact: Fact) {
        self.hits.push(Hit::Fact(fact));
    }

    fn slot(&mut self, node: Span, slot: Slot) {
        self.slots.insert(key(node), slot);
    }

    /// Range endpoints are pattern syntax, including signed literals.
    fn range_bound(&mut self, bound: &Expr) {
        match bound {
            Expr::Lit(lit) => self.visit_pat(&Pat::Lit(lit.clone())),
            Expr::Path(path) => self.visit_pat(&Pat::Path(path.clone())),
            other => self.visit_expr(other),
        }
    }

    fn host(&mut self, attrs: &[Attribute], host: Host) {
        self.hosts
            .extend(attrs.iter().map(|a| (key(a.span()), host.clone())));
    }

    fn cfg_facts(&mut self, attrs: &[Attribute], kind: CfgKind, name: &str) {
        let items = attrs
            .iter()
            .filter(|a| a.path().is_ident("cfg") && syntax::cfg_test(a).is_none())
            .filter_map(checks::cfg_predicate)
            .map(|predicate| {
                Hit::Fact(Fact::Cfg(CfgItem {
                    kind,
                    name: name.to_string(),
                    predicate,
                }))
            });
        self.hits.extend(items);
    }

    /// The facts of a struct, enum, union or type alias.
    fn defines(&mut self, ident: &syn::Ident, attrs: &[Attribute]) {
        let name = ident.to_string();
        self.fact(Fact::Type(name.clone()));
        self.fact(Fact::Defined(name.clone()));
        let wire = syntax::derive_named(attrs, &["Serialize", "Deserialize"]);
        self.hits
            .extend(wire.map(|_| Hit::Fact(Fact::Wire(name.clone()))));
    }

    /// A struct or enum with a library derive, in a file that mirrors an
    /// external protocol, is a wire type whose fields cost the boundary
    /// weight.
    fn kind_of(&self, attrs: &[Attribute], otherwise: ItemKind) -> ItemKind {
        let library = attrs
            .iter()
            .flat_map(syntax::derived)
            .find(|p| !STD_DERIVES.contains(&syntax::last_name(p).as_str()));
        match (self.env.external, library) {
            (Within::In, Some(_)) => ItemKind::Wire,
            _ => otherwise,
        }
    }

    /// The type a field of `attrs`'s item fills: a serde wire type's.
    fn field_slots(&mut self, fields: &Fields, slot: &Slot) {
        let slots = fields.iter().map(|f| (key(f.ty.span()), slot.clone()));
        self.slots.extend(slots);
    }

    fn data_fields(&mut self, attrs: &[Attribute], fields: &Fields) {
        let wire = syntax::derive_named(attrs, &["Serialize", "Deserialize"])
            .map_or(Within::Out, |_| Within::In);
        let error = syntax::derive_named(attrs, &["Error"]).map_or(Within::Out, |_| Within::In);
        let host = Host::Field { wire, error };
        for field in fields {
            self.host(&field.attrs, host.clone());
        }
        if let Within::In = error {
            self.hits.extend(checks::sources(fields));
        }
    }

    fn data_variant(
        &mut self,
        attrs: &[Attribute],
        variant: &syn::Variant,
        wire: Within,
        tagged: Within,
    ) {
        self.data_fields(attrs, &variant.fields);
        self.host(
            &variant.attrs,
            Host::Variant {
                wire,
                tagged,
                unit: match variant.fields {
                    Fields::Unit => Within::In,
                    _ => Within::Out,
                },
            },
        );
    }

    /// A fn item or method: the rules on its head and the context of its
    /// body.
    fn function(
        &mut self,
        attrs: &[Attribute],
        vis: &Visibility,
        sig: &syn::Signature,
        block: &syn::Block,
        method: Within,
    ) {
        let func = Rc::new(checks::func(sig, block, &self.env));
        let cx = Cx {
            func: Some(func.clone()),
            generics: checks::ask_build(vis, sig, method).map_or(Generics::Fn, |_| Generics::Ask),
            ..self.cx.clone()
        };
        self.hits.extend(checks::loop_states(sig, block, &cx));
        self.hits.extend(checks::fn_attrs(attrs, sig));
        self.cfg_facts(attrs, CfgKind::Fn, &sig.ident.to_string());
        self.hits
            .extend(checks::receiver(sig, &func, &cx, &self.env));
        self.hits.extend(checks::mut_params(sig, &cx, &self.env));
        self.host(
            attrs,
            Host::Cfg(Some(CfgItem::new(CfgKind::Fn, sig.ident.to_string()))),
        );
        self.cx = cx;
    }

    /// A `const`: its value is a table when its type is a slice or array.
    fn constant(&mut self, ident: &syn::Ident, t: &Type, init: &Expr) {
        let row = checks::table_element(t);
        let row_key = match row {
            Some(Type::Path(p)) => match p.qself {
                None => checks::resolve(&p.path, &self.cx.module),
                Some(_) => None,
            },
            _ => None,
        };
        let builds = checks::literal_rows(init)
            .into_iter()
            .filter_map(|e| match e {
                Expr::Struct(s) => checks::resolve(&s.path, &self.cx.module)
                    .filter(|k| row_key.as_ref() == Some(k))
                    .map(|_| key(e.span())),
                _ => None,
            });
        self.builds.extend(builds.collect::<Vec<(usize, usize)>>());
        let element = match (row, &row_key) {
            (Some(Type::Tuple(t)), _) => Some(Element::Tuple(
                t.elems
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| checks::str_ref(e).is_some())
                    .map(|(i, _)| i.to_string())
                    .collect(),
            )),
            (Some(_), Some(key)) => Some(Element::Row(key.clone())),
            (Some(r), None) => Some(checks::str_ref(r).map_or(Element::Other, |_| Element::Str)),
            (None, _) => None,
        };
        let held = match (row, t) {
            (None, Type::Path(p)) => match p.qself {
                None => Some(built(&p.path, &self.cx.module)),
                Some(_) => None,
            },
            _ => None,
        };
        self.hits.extend(element.map(|element| {
            Hit::Fact(Fact::Table {
                name: ident.to_string(),
                element,
            })
        }));
        self.hits.extend(held);
        self.hits.extend(checks::tuple_rows(t));
        self.slot(t.span(), Slot::ConstType);
        self.slot(init.span(), Slot::ConstInit);
    }

    /// Marks the boolean tree of a predicate: `&&`, `||` and `!` over
    /// comparisons, `matches!` and other leaves (proposal 0028).
    fn predicate(&mut self, expr: &Expr) {
        let extent = key(expr.span());
        match expr {
            Expr::Paren(e) => self.predicate(&e.expr),
            Expr::Block(e) => {
                if let (None, Some(Stmt::Expr(tail, None)), 1) =
                    (&e.label, e.block.stmts.first(), e.block.stmts.len())
                {
                    self.predicate(tail);
                }
            }
            Expr::Binary(e) => match (e.op, checks::comparison_op(&e.op)) {
                (BinOp::And(_) | BinOp::Or(_), _) => {
                    self.predicates.insert(extent);
                    self.predicate(&e.left);
                    self.predicate(&e.right);
                }
                (_, Some(_)) => {
                    self.predicates.insert(extent);
                }
                _ => {}
            },
            Expr::Unary(e) => {
                if let UnOp::Not(_) = e.op {
                    self.predicates.insert(extent);
                    self.predicate(&e.expr);
                }
            }
            Expr::Macro(e) => {
                self.predicates
                    .extend(e.mac.path.is_ident("matches").then_some(key(e.mac.span())));
            }
            _ => {}
        }
    }

    fn iterate(&mut self, expr: &Expr) {
        self.iterated.insert(key(syntax::unparen(expr).span()));
    }

    /// The `if let` and `while let` heads, whose `let` is part of them.
    fn let_head(&mut self, attrs: &[Attribute], cond: &syn::ExprLet) {
        for a in attrs.iter() {
            self.visit_attribute(a);
        }
        self.visit_pat(&cond.pat);
        self.visit_expr(&cond.expr);
    }
}

/// A struct built outside a table's literal, or held by a `const` of its
/// own type.
fn built(path: &syn::Path, module: &[String]) -> Hit {
    Hit::Fact(Fact::Built {
        key: checks::resolve(path, module),
        name: syntax::last_name(path),
    })
}

/// The class typing gives a type.
pub(crate) fn classify(t: &Type, env: &Env) -> TypeClass {
    let untyped = env
        .lists
        .untyped
        .iter()
        .find(|p| checks::type_matches(p, t, &env.facts).is_some());
    let primitive = env
        .lists
        .primitives
        .iter()
        .find(|p| checks::type_matches(p, t, &env.facts).is_some());
    match (untyped, primitive, t) {
        (Some(_), _, _) => TypeClass::Untyped,
        (None, Some(_), _) => TypeClass::Primitive,
        (None, None, Type::Reference(r)) => match classify(&r.elem, env) {
            TypeClass::Primitive => TypeClass::Primitive,
            _ => TypeClass::Other,
        },
        (None, None, Type::Path(p)) => match (&p.qself, ours(&p.path, env)) {
            (None, Owner::Ours) => TypeClass::Own,
            _ => TypeClass::Other,
        },
        (None, None, _) => TypeClass::Other,
    }
}

/// Interior mutability: the first segment naming a std cell, lock or atomic.
fn interior_mut(p: &syn::Path) -> Option<String> {
    p.segments
        .iter()
        .map(|s| s.ident.to_string())
        .find(|name| INTERIOR_MUT.contains(&name.as_str()) || name.starts_with("Atomic"))
}

fn binary_id(op: &BinOp) -> &str {
    match op {
        BinOp::Add(_) => "expr.binary.add",
        BinOp::Sub(_) => "expr.binary.sub",
        BinOp::Mul(_) => "expr.binary.mul",
        BinOp::Div(_) => "expr.binary.div",
        BinOp::Rem(_) => "expr.binary.rem",
        BinOp::And(_) => "expr.binary.and",
        BinOp::Or(_) => "expr.binary.or",
        BinOp::BitXor(_) => "expr.binary.bit_xor",
        BinOp::BitAnd(_) => "expr.binary.bit_and",
        BinOp::BitOr(_) => "expr.binary.bit_or",
        BinOp::Shl(_) => "expr.binary.shl",
        BinOp::Shr(_) => "expr.binary.shr",
        BinOp::Eq(_) => "expr.binary.eq",
        BinOp::Lt(_) => "expr.binary.lt",
        BinOp::Le(_) => "expr.binary.le",
        BinOp::Ne(_) => "expr.binary.ne",
        BinOp::Ge(_) => "expr.binary.ge",
        BinOp::Gt(_) => "expr.binary.gt",
        BinOp::AddAssign(_) => "expr.binary.add_assign",
        BinOp::SubAssign(_) => "expr.binary.sub_assign",
        BinOp::MulAssign(_) => "expr.binary.mul_assign",
        BinOp::DivAssign(_) => "expr.binary.div_assign",
        BinOp::RemAssign(_) => "expr.binary.rem_assign",
        BinOp::BitXorAssign(_) => "expr.binary.bit_xor_assign",
        BinOp::BitAndAssign(_) => "expr.binary.bit_and_assign",
        BinOp::BitOrAssign(_) => "expr.binary.bit_or_assign",
        BinOp::ShlAssign(_) => "expr.binary.shl_assign",
        BinOp::ShrAssign(_) => "expr.binary.shr_assign",
        _ => "unknown.expr",
    }
}

/// An id at a token.
fn token(span: Span, name: &str) -> Option<(Span, String)> {
    Some((span, name.to_string()))
}

/// A refinement of a type's id for the score.
fn refine(extent: Extent, to: &str) -> Hit {
    Hit::Refine(Refinement {
        extent,
        from: None,
        to: to.to_string(),
    })
}

/// The attributes of an item that can carry a test attribute.
fn item_attrs(item: &Item) -> Option<&Vec<Attribute>> {
    match item {
        Item::Const(i) => Some(&i.attrs),
        Item::Enum(i) => Some(&i.attrs),
        Item::Fn(i) => Some(&i.attrs),
        Item::Impl(i) => Some(&i.attrs),
        Item::Mod(i) => Some(&i.attrs),
        Item::Static(i) => Some(&i.attrs),
        Item::Struct(i) => Some(&i.attrs),
        Item::Trait(i) => Some(&i.attrs),
        Item::Type(i) => Some(&i.attrs),
        Item::Union(i) => Some(&i.attrs),
        Item::Use(i) => Some(&i.attrs),
        _ => None,
    }
}

/// The id of an expression, at the token it is reported at.
fn expr_id(expr: &Expr) -> Option<(Span, String)> {
    match expr {
        Expr::Array(e) => token(e.span(), "expr.array"),
        Expr::Assign(e) => token(e.eq_token.span, "expr.assign"),
        Expr::Async(e) => token(e.async_token.span, "expr.async"),
        Expr::Await(e) => token(e.await_token.span, "expr.await"),
        Expr::Binary(e) => token(e.op.span(), binary_id(&e.op)),
        Expr::Block(e) => token(
            e.span(),
            e.label
                .as_ref()
                .map_or("expr.block", |_| "expr.block.labeled"),
        ),
        Expr::Break(e) => token(e.break_token.span, "expr.break"),
        Expr::Call(e) => token(e.span(), "expr.call"),
        Expr::Cast(e) => token(e.as_token.span, "expr.cast"),
        Expr::Const(e) => token(e.const_token.span, "expr.const_block"),
        Expr::Continue(e) => token(e.continue_token.span, "expr.continue"),
        Expr::Field(e) => token(e.dot_token.span, "expr.field"),
        Expr::ForLoop(e) => token(e.for_token.span, "expr.for"),
        Expr::Group(e) => token(e.span(), "expr.group"),
        Expr::Index(e) => token(e.bracket_token.span.open(), "expr.index"),
        Expr::Infer(e) => token(e.span(), "expr.infer"),
        Expr::Let(e) => token(e.let_token.span, "expr.let"),
        Expr::Lit(e) => token(
            e.span(),
            match e.lit {
                Lit::Bool(_) => "expr.lit.bool",
                _ => "expr.lit",
            },
        ),
        Expr::Loop(e) => token(e.loop_token.span, "expr.loop"),
        Expr::Match(e) => token(e.match_token.span, "expr.match"),
        Expr::MethodCall(e) => token(e.method.span(), "expr.method_call"),
        Expr::Paren(e) => token(e.span(), "expr.paren"),
        Expr::Path(e) => match (&e.qself, interior_mut(&e.path)) {
            (None, Some(cell)) => Some((e.span(), format!("expr.interior_mut.{cell}"))),
            (Some(_), _) => token(e.span(), "expr.path.qualified"),
            (None, None) => token(e.span(), "expr.path"),
        },
        Expr::Range(e) => token(e.limits.span(), "expr.range"),
        Expr::RawAddr(e) => token(e.span(), "expr.raw_addr"),
        Expr::Reference(e) => token(
            e.and_token.span,
            e.mutability
                .map_or("expr.reference", |_| "expr.reference.mut"),
        ),
        Expr::Repeat(e) => token(e.span(), "expr.repeat"),
        Expr::Return(e) => token(e.return_token.span, "expr.return"),
        Expr::Struct(e) => token(
            e.span(),
            e.rest
                .as_ref()
                .map_or("expr.struct", |_| "expr.struct.update"),
        ),
        Expr::Try(e) => token(e.question_token.span, "expr.try"),
        Expr::TryBlock(e) => token(e.try_token.span, "expr.try_block"),
        Expr::Tuple(e) => token(
            e.span(),
            e.elems.first().map_or("expr.unit", |_| "expr.tuple"),
        ),
        Expr::Unary(e) => token(
            e.op.span(),
            match e.op {
                UnOp::Deref(_) => "expr.unary.deref",
                UnOp::Not(_) => "expr.unary.not",
                UnOp::Neg(_) => "expr.unary.neg",
                _ => "unknown.expr",
            },
        ),
        Expr::Unsafe(e) => token(e.unsafe_token.span, "expr.unsafe"),
        Expr::Yield(e) => token(e.yield_token.span, "expr.yield"),
        Expr::Closure(_) | Expr::If(_) | Expr::Macro(_) | Expr::While(_) => None,
        _ => token(expr.span(), "unknown.expr"),
    }
}

/// The id of a type, at the token it is reported at.
fn type_id(t: &Type, cx: &Cx) -> Option<(Span, String)> {
    match t {
        Type::Array(a) => token(a.span(), "type.array"),
        Type::BareFn(f) => token(f.fn_token.span, "type.fn_ptr"),
        Type::Group(g) => token(g.span(), "type.group"),
        Type::ImplTrait(i) => token(
            i.impl_token.span,
            match cx.in_return {
                Within::In => "type.impl_trait.ret",
                Within::Out => "type.impl_trait.arg",
            },
        ),
        Type::Infer(i) => token(i.span(), "type.infer"),
        Type::Macro(_) => None,
        Type::Never(n) => token(n.span(), "type.never"),
        Type::Paren(p) => token(p.span(), "type.paren"),
        Type::Path(p) => match (
            &p.qself,
            interior_mut(&p.path),
            p.path.get_ident().map(ToString::to_string).as_deref(),
        ) {
            (None, Some(cell), _) => Some((p.span(), format!("type.interior_mut.{cell}"))),
            (Some(_), _, _) => token(p.span(), "type.path.qualified"),
            (None, None, Some("bool")) => token(p.span(), "type.bool"),
            (None, None, _) => token(p.span(), "type.path"),
        },
        Type::Ptr(p) => token(p.span(), "type.ptr"),
        Type::Reference(r) => token(
            r.and_token.span,
            r.mutability
                .map_or("type.reference", |_| "type.reference.mut"),
        ),
        Type::Slice(s) => token(s.span(), "type.slice"),
        Type::TraitObject(o) => token(o.span(), "type.dyn"),
        Type::Tuple(tu) => token(
            tu.span(),
            tu.elems.first().map_or("type.unit", |_| "type.tuple"),
        ),
        _ => token(t.span(), "unknown.type"),
    }
}

impl<'ast> Visit<'ast> for Walk {
    fn visit_block(&mut self, block: &'ast syn::Block) {
        for span in checks::local_closures(block, &self.env) {
            self.slot(span, Slot::LocalClosure);
        }
        visit::visit_block(self, block);
    }

    fn visit_file(&mut self, file: &'ast syn::File) {
        let hosts = file.attrs.iter().map(|a| {
            let allowed = Some(a).filter(|a| {
                matches!(self.env.crate_root, Within::In)
                    && a.path().is_ident("allow")
                    && syntax::attr_tokens(a) == "clippy::useless_format"
            });
            let host = match (allowed, self.env.test_file) {
                (Some(_), _) => Host::RootAllow,
                (None, Within::In) => Host::Cfg(None),
                (None, Within::Out) => Host::Other,
            };
            (key(a.span()), host)
        });
        self.hosts
            .extend(hosts.collect::<Vec<((usize, usize), Host)>>());
        visit::visit_file(self, file);
    }

    fn visit_item(&mut self, item: &'ast Item) {
        let outer = self.cx.clone();
        self.cx = Cx {
            place: None,
            item: ItemKind::Other,
            generics: Generics::Item,
            ..self.cx.clone()
        };
        let test = item_attrs(item)
            .and_then(|a| syntax::test_attr(a))
            .map_or(self.cx.test, |_| Within::In);
        self.cx.test = test;
        let token = match item {
            Item::Const(i) => Some((i.const_token.span, "item.const")),
            Item::Enum(i) => Some((i.enum_token.span, "item.enum")),
            Item::ExternCrate(i) => Some((i.extern_token.span, "item.extern_crate")),
            Item::Fn(i) => Some((i.sig.fn_token.span, "item.fn")),
            Item::ForeignMod(i) => Some((i.abi.extern_token.span, "item.foreign_mod")),
            Item::Impl(i) => Some((
                i.impl_token.span,
                i.trait_
                    .as_ref()
                    .map_or("item.impl.inherent", |_| "item.impl.trait"),
            )),
            Item::Macro(_) => None,
            Item::Mod(i) => Some((i.mod_token.span, "item.mod")),
            Item::Static(i) => Some((
                i.static_token.span,
                match i.mutability {
                    syn::StaticMutability::Mut(_) => "item.static.mut",
                    _ => "item.static",
                },
            )),
            Item::Struct(i) => Some((
                i.struct_token.span,
                match &i.fields {
                    Fields::Named(_) => "item.struct.named",
                    Fields::Unnamed(f) => match f.unnamed.len() {
                        1 => "item.struct.newtype",
                        _ => "item.struct.tuple",
                    },
                    Fields::Unit => "item.struct.unit",
                },
            )),
            Item::Trait(i) => Some((i.trait_token.span, "item.trait")),
            Item::TraitAlias(i) => Some((i.trait_token.span, "item.trait_alias")),
            Item::Type(i) => Some((i.type_token.span, "item.type")),
            Item::Union(i) => Some((i.union_token.span, "item.union")),
            Item::Use(i) => Some((i.use_token.span, "item.use")),
            _ => Some((item.span(), "unknown.item")),
        };
        let rules = match item {
            Item::Macro(i) => i
                .ident
                .as_ref()
                .filter(|_| i.mac.path.is_ident("macro_rules")),
            _ => None,
        };
        match (token, rules, item) {
            (_, Some(_), Item::Macro(i)) => {
                self.id(i.mac.path.span(), item.span(), "item.macro_rules");
                for a in i.attrs.iter() {
                    self.visit_attribute(a);
                }
            }
            (Some((at, name)), _, _) => {
                self.id(at, item.span(), name);
                visit::visit_item(self, item);
            }
            _ => visit::visit_item(self, item),
        }
        self.cx = outer;
    }

    fn visit_item_fn(&mut self, i: &'ast syn::ItemFn) {
        let name = i.sig.ident.to_string();
        self.fact(Fact::Fn(name.clone()));
        self.hits.push(Hit::Function(FnSpan {
            name: name.clone(),
            line: i.sig.ident.span().start().line,
            extent: syntax::extent(i.span()),
        }));
        let main = syntax::returned(&i.sig.output).filter(|_| name == "main");
        self.slots
            .extend(main.map(|t| (key(t.span()), Slot::MainReturn)));
        self.function(&i.attrs, &i.vis, &i.sig, &i.block, Within::Out);
        visit::visit_item_fn(self, i);
    }

    fn visit_impl_item(&mut self, item: &'ast syn::ImplItem) {
        let outer = self.cx.clone();
        let token = match item {
            syn::ImplItem::Const(i) => Some((i.const_token.span, "item.const")),
            syn::ImplItem::Fn(i) => Some((i.sig.fn_token.span, "item.fn")),
            syn::ImplItem::Type(i) => Some((i.type_token.span, "item.impl.type")),
            syn::ImplItem::Macro(_) => None,
            _ => Some((item.span(), "unknown.impl_item")),
        };
        if let Some((at, name)) = token {
            self.id(at, item.span(), name);
        }
        visit::visit_impl_item(self, item);
        self.cx = outer;
    }

    fn visit_impl_item_fn(&mut self, i: &'ast syn::ImplItemFn) {
        let owner = self
            .cx
            .imp
            .as_ref()
            .map(|imp| imp.self_ty.clone().unwrap_or_else(|| "_".to_string()));
        let name = owner.map_or(i.sig.ident.to_string(), |ty| {
            format!("{ty}::{}", i.sig.ident)
        });
        self.hits.push(Hit::Function(FnSpan {
            name,
            line: i.sig.ident.span().start().line,
            extent: syntax::extent(i.span()),
        }));
        let test = syntax::test_attr(&i.attrs).map_or(self.cx.test, |_| Within::In);
        self.cx.test = test;
        self.function(&i.attrs, &i.vis, &i.sig, &i.block, Within::In);
        visit::visit_impl_item_fn(self, i);
    }

    fn visit_trait_item(&mut self, item: &'ast syn::TraitItem) {
        let token = match item {
            syn::TraitItem::Const(i) => Some((i.const_token.span, "item.trait.const")),
            syn::TraitItem::Fn(i) => Some((i.sig.fn_token.span, "item.trait.fn")),
            syn::TraitItem::Type(i) => Some((i.type_token.span, "item.trait.type")),
            syn::TraitItem::Macro(_) => None,
            _ => Some((item.span(), "unknown.trait_item")),
        };
        for (at, name) in token.iter() {
            self.id(at.to_owned(), item.span(), name);
        }
        visit::visit_trait_item(self, item);
    }

    fn visit_foreign_item(&mut self, item: &'ast syn::ForeignItem) {
        let token = match item {
            syn::ForeignItem::Fn(i) => Some((i.sig.fn_token.span, "item.foreign.fn")),
            syn::ForeignItem::Static(i) => Some((i.static_token.span, "item.foreign.static")),
            syn::ForeignItem::Type(i) => Some((i.type_token.span, "item.foreign.type")),
            syn::ForeignItem::Macro(_) => None,
            _ => Some((item.span(), "unknown.foreign_item")),
        };
        for (at, name) in token.iter() {
            self.id(at.to_owned(), item.span(), name);
        }
        visit::visit_foreign_item(self, item);
    }

    fn visit_item_impl(&mut self, i: &'ast syn::ItemImpl) {
        let trait_owner = i
            .trait_
            .as_ref()
            .map_or(Owner::Ours, |(_, path, _)| ours(path, &self.env));
        let kind = match i
            .trait_
            .as_ref()
            .map(|(_, path, _)| syntax::last_name(path))
        {
            None => ImplKind::Inherent,
            Some(name) => LIBRARY_TRAITS
                .contains(&name.as_str())
                .then_some(())
                .map_or(ImplKind::OtherTrait, |_| ImplKind::Library),
        };
        self.hits
            .extend(i.unsafety.map(|t| t.span).iter().map(|at| {
                let reason = self
                    .env
                    .lists
                    .reasons
                    .get("item.impl.unsafe")
                    .cloned()
                    .unwrap_or_default();
                Hit::Id(Occurrence {
                    id: "item.impl.unsafe".to_string(),
                    reason,
                    at: syntax::pos(at.to_owned()),
                    extent: syntax::extent(at.to_owned()),
                })
            }));
        if let Owner::Theirs = trait_owner {
            self.hits.push(mark(i.span(), Fixed::Always));
        }
        if let ImplKind::OtherTrait = kind {
            self.hits.push(flag(i.impl_token.span, Rule::ImplTrait));
        }
        let visitor = i
            .trait_
            .as_ref()
            .filter(|(_, path, _)| syntax::last_name(path) == "Visit")
            .and(syntax::type_name(&i.self_ty));
        self.hits
            .extend(visitor.map(|name| Hit::Fact(Fact::Visitor(name))));
        let name = checks::impl_name(i);
        self.cfg_facts(&i.attrs, CfgKind::Impl, &name);
        self.host(&i.attrs, Host::Cfg(Some(CfgItem::new(CfgKind::Impl, name))));
        self.cx.imp = Some(Rc::new(Imp {
            self_ty: syntax::type_name(&i.self_ty),
            kind,
            trait_owner,
        }));
        self.cx.generics = match kind {
            ImplKind::Inherent => Generics::Data,
            _ => Generics::Item,
        };
        visit::visit_item_impl(self, i);
    }

    fn visit_item_struct(&mut self, i: &'ast syn::ItemStruct) {
        self.cx.generics = Generics::Data;
        self.data_fields(&i.attrs, &i.fields);
        let newtype = checks::newtype(&i.fields);
        self.cx.item = self.kind_of(
            &i.attrs,
            newtype.map_or(ItemKind::Other, |_| ItemKind::Newtype),
        );
        let name = i.ident.to_string();
        let key = self
            .cx
            .module
            .iter()
            .cloned()
            .chain(Some(name.clone()))
            .collect::<Vec<String>>();
        self.defines(&i.ident, &i.attrs);
        self.fact(Fact::Fields(
            name.clone(),
            i.fields
                .iter()
                .flat_map(|f| syntax::type_segments(&f.ty))
                .collect(),
        ));
        let named = match &i.fields {
            Fields::Named(f) => Some(Fact::Struct {
                key: key.clone(),
                strings: f
                    .named
                    .iter()
                    .filter(|f| checks::str_ref(&f.ty).is_some())
                    .filter_map(|f| f.ident.as_ref().map(ToString::to_string))
                    .collect(),
            }),
            _ => None,
        };
        self.hits.extend(named.map(Hit::Fact));
        self.hits.extend(
            newtype
                .and(syntax::derive_named(&i.attrs, &["Ord"]))
                .map(|_| Hit::Fact(Fact::OrdNewtype(name))),
        );
        let host = match &i.fields {
            Fields::Named(_) => checks::ErrorHost::Named,
            _ => checks::ErrorHost::Tuple,
        };
        self.hits
            .extend(checks::thiserror(&i.attrs, &i.ident, host, &self.env));
        self.hits
            .extend(checks::messages(&i.attrs, &i.attrs, i.ident.span()));
        self.hits.extend(checks::ord(
            &i.attrs,
            newtype.map_or(Within::Out, |_| Within::In),
        ));
        let pub_fields = i.fields.iter().filter_map(|f| match &f.vis {
            Visibility::Public(token) => Some(flag(token.span, Rule::PubField)),
            _ => None,
        });
        let pub_fields = pub_fields.collect::<Vec<Hit>>();
        self.hits
            .extend(checks::pub_crate(&i.vis).map_or(Vec::new(), |_| pub_fields));
        let clap = syntax::derive_of(&i.attrs, &["Parser", "clap::Parser"]);
        let wire = syntax::derive_named(&i.attrs, &["Serialize", "Deserialize"]);
        let slot = match (self.env.tables.rows.get(&key), clap, wire) {
            (Some(_), _, _) => Slot::RowField,
            (None, Some(_), _) => Slot::ClapField,
            (None, None, Some(_)) => Slot::WireField,
            (None, None, None) => Slot::Other,
        };
        self.field_slots(&i.fields, &slot);
        visit::visit_item_struct(self, i);
    }

    fn visit_item_enum(&mut self, i: &'ast syn::ItemEnum) {
        self.cx.generics = Generics::Data;
        let wire = syntax::derive_named(&i.attrs, &["Serialize", "Deserialize"])
            .map_or(Within::Out, |_| Within::In);
        let tagged = i
            .attrs
            .iter()
            .any(|a| a.path().is_ident("serde") && syntax::meta_keys(a).iter().any(|k| k == "tag"))
            .then_some(())
            .map_or(Within::Out, |_| Within::In);
        for variant in &i.variants {
            self.data_variant(&i.attrs, variant, wire, tagged);
        }
        self.cx.item = self.kind_of(&i.attrs, ItemKind::Other);
        let name = i.ident.to_string();
        self.defines(&i.ident, &i.attrs);
        self.fact(Fact::Enum(name.clone()));
        let payloads = i
            .variants
            .iter()
            .flat_map(|v| v.fields.iter().flat_map(|f| syntax::type_segments(&f.ty)))
            .collect();
        self.fact(Fact::Fields(name, payloads));
        self.hits.extend(checks::thiserror(
            &i.attrs,
            &i.ident,
            checks::ErrorHost::Enum,
            &self.env,
        ));
        let messages = i
            .variants
            .iter()
            .flat_map(|v| checks::messages(&i.attrs, &v.attrs, v.ident.span()));
        self.hits.extend(messages.collect::<Vec<Hit>>());
        self.hits.extend(checks::ord(&i.attrs, Within::Out));
        self.hits.extend(checks::from_once(&i.variants));
        let slot = syntax::derive_named(&i.attrs, &["Serialize", "Deserialize"])
            .map_or(Slot::Other, |_| Slot::WireField);
        for v in i.variants.iter() {
            self.field_slots(&v.fields, &slot);
        }
        visit::visit_item_enum(self, i);
    }

    fn visit_item_const(&mut self, i: &'ast syn::ItemConst) {
        self.constant(&i.ident, &i.ty, &i.expr);
        visit::visit_item_const(self, i);
    }

    fn visit_impl_item_const(&mut self, i: &'ast syn::ImplItemConst) {
        self.constant(&i.ident, &i.ty, &i.expr);
        visit::visit_impl_item_const(self, i);
    }

    fn visit_item_use(&mut self, i: &'ast syn::ItemUse) {
        let paths = use_paths(&i.tree, &Vec::new());
        self.hits.extend(checks::globs(&i.tree, &Vec::new()));
        let named = paths
            .iter()
            .filter(|p| p.name.is_some())
            .flat_map(|p| checks::named(&p.segments, checks::Named::Use, p.span, &self.env));
        self.hits.extend(named.collect::<Vec<Hit>>());
        self.hits
            .extend(paths.into_iter().map(|p| Hit::Fact(Fact::Use(p))));
        self.cfg_facts(&i.attrs, CfgKind::Use, "");
        self.host(
            &i.attrs,
            Host::Cfg(Some(CfgItem::new(CfgKind::Use, String::new()))),
        );
        visit::visit_item_use(self, i);
    }

    fn visit_item_mod(&mut self, i: &'ast syn::ItemMod) {
        self.fact(Fact::Mod(i.ident.to_string()));
        self.host(&i.attrs, Host::Cfg(None));
        let module = self
            .cx
            .module
            .iter()
            .cloned()
            .chain(Some(i.ident.to_string()))
            .collect();
        self.cx.module = Rc::new(module);
        visit::visit_item_mod(self, i);
    }

    fn visit_item_trait(&mut self, i: &'ast syn::ItemTrait) {
        self.fact(Fact::Type(i.ident.to_string()));
        let modifiers = i
            .unsafety
            .map(|t| (t.span, "item.trait.unsafe"))
            .into_iter()
            .chain(i.auto_token.map(|t| (t.span, "item.trait.auto")));
        for (at, name) in modifiers {
            self.id(at, at, name);
        }
        visit::visit_item_trait(self, i);
    }

    fn visit_item_type(&mut self, i: &'ast syn::ItemType) {
        self.defines(&i.ident, &[]);
        visit::visit_item_type(self, i);
    }

    fn visit_item_union(&mut self, i: &'ast syn::ItemUnion) {
        self.defines(&i.ident, &[]);
        visit::visit_item_union(self, i);
    }

    fn visit_variant(&mut self, v: &'ast syn::Variant) {
        let discriminant = v.discriminant.as_ref().map(|(eq, value)| {
            (
                eq.span,
                syntax::extent(eq.span).to(syntax::extent(value.span())),
            )
        });
        if let Some((at, node)) = discriminant {
            self.id_over(at, node, "item.enum.discriminant");
        }
        visit::visit_variant(self, v);
    }

    fn visit_field(&mut self, f: &'ast syn::Field) {
        let outer = self.cx.clone();
        for a in f.attrs.iter() {
            self.visit_attribute(a);
        }
        self.visit_visibility(&f.vis);
        let class = classify(&f.ty, &self.env);
        match (self.cx.item, class) {
            (ItemKind::Wire, _) => self.hits.push(Hit::Wire(syntax::extent(f.span()))),
            (ItemKind::Newtype, TypeClass::Untyped) => {}
            (ItemKind::Newtype, _) => {
                self.hits.push(Hit::Refine(Refinement {
                    extent: syntax::extent(f.ty.span()),
                    from: None,
                    to: "type.field".to_string(),
                }));
                self.hits.push(Hit::Fold(syntax::extent(f.ty.span())));
                self.cx.typing = Within::Out;
            }
            (ItemKind::Other, _) => self.cx.place = Some(Place::Field),
        }
        self.visit_type(&f.ty);
        self.cx = outer;
    }

    fn visit_signature(&mut self, sig: &'ast syn::Signature) {
        let outer = self.cx.clone();
        let modifiers = sig
            .constness
            .map(|t| (t.span, "item.fn.const"))
            .into_iter()
            .chain(sig.asyncness.map(|t| (t.span, "item.fn.async")))
            .chain(sig.unsafety.map(|t| (t.span, "item.fn.unsafe")))
            .chain(
                sig.abi
                    .as_ref()
                    .map(|a| (a.extern_token.span, "item.fn.extern")),
            )
            .chain(
                sig.variadic
                    .as_ref()
                    .and_then(|v| v.dots.spans.first().copied())
                    .map(|s| (s, "item.fn.variadic")),
            );
        for (at, name) in modifiers {
            self.id(at, at, name);
        }
        let last = sig.inputs.len().saturating_sub(1);
        let typed = sig
            .inputs
            .iter()
            .enumerate()
            .filter_map(|(i, arg)| match arg {
                FnArg::Typed(t) => Some((i, t)),
                FnArg::Receiver(_) => None,
            });
        let slots = typed.filter_map(|(i, t)| {
            let slot = match (self.cx.generics, i) {
                (Generics::Ask, i) if i == last => Some(Slot::AskBuild),
                (Generics::Ask, _) => None,
                _ => Some(Slot::Param),
            };
            slot.map(|s| (key(t.ty.span()), s))
        });
        self.slots
            .extend(slots.collect::<Vec<((usize, usize), Slot)>>());
        self.visit_generics(&sig.generics);
        self.cx.generics = Generics::Item;
        self.cx.place = self.cx.foreign().map_or(Some(Place::Signature), |_| None);
        for arg in sig.inputs.iter() {
            self.visit_fn_arg(arg);
        }
        self.cx.in_return = Within::In;
        self.visit_return_type(&sig.output);
        self.cx = outer;
    }

    fn visit_receiver(&mut self, r: &'ast syn::Receiver) {
        for a in r.attrs.iter() {
            self.visit_attribute(a);
        }
        for t in r.mutability.iter() {
            self.id(t.span, r.span(), "item.fn.receiver.mut");
        }
        self.hits.extend(when(
            r.mutability.and(self.cx.foreign()).and(Some(())),
            mark(r.span(), Fixed::Always),
        ));
        let lifetime = r.reference.as_ref().and_then(|(_, l)| l.as_ref());
        for l in lifetime.iter() {
            self.id(l.span(), l.span(), "type.lifetime");
            self.visit_lifetime(l);
        }
        match (r.colon_token, &r.reference, r.mutability) {
            (Some(_), _, _) => {
                self.id(
                    r.self_token.span,
                    r.self_token.span,
                    "item.fn.receiver.typed",
                );
                self.visit_type(&r.ty);
            }
            // `&mut self` stands for `&mut Self`, a `&mut` of a type that
            // isn't a path of ours.
            (None, Some(_), Some(_)) => self.hits.push(mark(r.ty.span(), Fixed::Always)),
            (None, _, _) => {}
        }
    }

    fn visit_generics(&mut self, g: &'ast syn::Generics) {
        let cx = self.cx.clone();
        let rules = g
            .params
            .iter()
            .flat_map(|p| checks::generic_param(p, g, &cx, &self.env));
        self.hits.extend(rules.collect::<Vec<Hit>>());
        self.cx.generics = Generics::Item;
        visit::visit_generics(self, g);
        self.cx.generics = cx.generics;
    }

    fn visit_generic_param(&mut self, p: &'ast GenericParam) {
        let name = match p {
            GenericParam::Type(_) => "generics.type_param",
            GenericParam::Lifetime(_) => "generics.lifetime",
            GenericParam::Const(_) => "generics.const_param",
        };
        self.id(p.span(), p.span(), name);
        visit::visit_generic_param(self, p);
    }

    fn visit_lifetime_param(&mut self, p: &'ast syn::LifetimeParam) {
        for a in p.attrs.iter() {
            self.visit_attribute(a);
        }
    }

    fn visit_where_clause(&mut self, w: &'ast syn::WhereClause) {
        self.id(w.where_token.span, w.span(), "generics.where");
        visit::visit_where_clause(self, w);
    }

    fn visit_bound_lifetimes(&mut self, b: &'ast syn::BoundLifetimes) {
        self.id(b.for_token.span, b.span(), "generics.hrtb");
    }

    fn visit_generic_argument(&mut self, arg: &'ast GenericArgument) {
        let name = match arg {
            GenericArgument::Lifetime(_) => Some("type.lifetime"),
            GenericArgument::Type(_) => None,
            GenericArgument::Const(_) => Some("generics.const_arg"),
            GenericArgument::AssocType(_)
            | GenericArgument::AssocConst(_)
            | GenericArgument::Constraint(_) => Some("generics.assoc_binding"),
            _ => Some("unknown.generic_arg"),
        };
        for n in name.iter() {
            self.id(arg.span(), arg.span(), n);
        }
        visit::visit_generic_argument(self, arg);
    }

    fn visit_type_param_bound(&mut self, b: &'ast TypeParamBound) {
        match b {
            TypeParamBound::Trait(_) => visit::visit_type_param_bound(self, b),
            TypeParamBound::Lifetime(l) => {
                self.id(l.span(), l.span(), "type.lifetime");
                self.visit_lifetime(l);
            }
            _ => self.id(b.span(), b.span(), "unknown.bound"),
        }
    }

    fn visit_lifetime(&mut self, l: &'ast syn::Lifetime) {
        let rules = match self.statics.get(&key(l.span())) {
            Some(_) => Vec::new(),
            None => join(Vec::from([
                when(
                    self.cx.foreign().and(Some(())),
                    mark(l.span(), Fixed::Always),
                ),
                unless(
                    self.cx.library().and(Some(())),
                    flag(l.span(), Rule::Lifetime),
                ),
            ])),
        };
        self.hits.extend(rules);
    }

    fn visit_label(&mut self, _: &'ast syn::Label) {}

    fn visit_visibility(&mut self, vis: &'ast Visibility) {
        match vis {
            Visibility::Public(token) => self.id(token.span, token.span, "vis.pub"),
            Visibility::Restricted(r) => {
                self.id(r.pub_token.span, r.span(), "vis.restricted");
                self.hits.extend(unless(
                    checks::pub_crate(vis).and(Some(())),
                    flag(r.pub_token.span, Rule::VisCrate),
                ));
            }
            _ => {}
        }
    }

    fn visit_attribute(&mut self, attr: &'ast Attribute) {
        let at = attr.pound_token.span;
        let path = syntax::path_string(attr.path());
        let derived = syntax::derived(attr);
        let unlisted = derived
            .iter()
            .map(syntax::path_string)
            .filter(|name| !self.env.lists.derives.contains(name))
            .collect::<Vec<String>>();
        let parsed = attr.parse_args_with(
            syn::punctuated::Punctuated::<syn::Path, syn::token::Comma>::parse_terminated,
        );
        match (
            path.as_str(),
            syntax::cfg_test(attr),
            parsed,
            unlisted.first(),
        ) {
            ("doc", _, _, _) => self.id(at, attr.span(), "attr.doc"),
            ("test", _, _, _) | (_, Some(_), _, _) => self.id(at, attr.span(), "attr.cfg_test"),
            ("derive", _, Err(_), _) => self.id(at, attr.span(), "unknown.attr"),
            ("derive", _, Ok(_), None) => self.id(at, attr.span(), "attr.derive"),
            ("derive", _, Ok(_), Some(_)) => {
                for name in unlisted.iter() {
                    self.named(
                        at,
                        attr.span(),
                        format!("attr.derive.{name}"),
                        table::UNLISTED_DERIVE,
                    );
                }
            }
            _ => self.named(
                at,
                attr.span(),
                format!("attr.{path}"),
                table::UNLISTED_ATTR,
            ),
        }
        let library = match path.as_str() {
            "derive" => derived
                .iter()
                .find(|p| !STD_DERIVES.contains(&syntax::last_name(p).as_str()))
                .and(Some(())),
            _ => checks::absent(BUILTIN_ATTRS.contains(&path.as_str()).then_some(())),
        };
        self.hits
            .extend(when(library, mark(attr.span(), Fixed::Always)));
        let host = self
            .hosts
            .get(&key(attr.span()))
            .cloned()
            .unwrap_or(Host::Other);
        let rules = match (path.as_str(), syntax::cfg_test(attr), &host) {
            ("cfg", None, _) => checks::cfg(attr, &self.env, &host),
            ("allow", _, Host::RootAllow) => Vec::new(),
            ("allow", _, _) => Vec::from([flag(at, Rule::Allow)]),
            ("serde", _, _) => checks::serde(attr, &host),
            ("source", _, _) => checks::source_attr(attr, &host),
            ("arg", _, _) => when(
                syntax::attr_tokens(attr)
                    .contains("value_parser=[")
                    .then_some(()),
                flag(at, Rule::ValueParser),
            ),
            _ => Vec::new(),
        };
        self.hits.extend(rules);
    }

    fn visit_use_glob(&mut self, g: &'ast syn::UseGlob) {
        self.id(g.star_token.span, g.star_token.span, "item.use.glob");
    }

    fn visit_use_rename(&mut self, r: &'ast syn::UseRename) {
        self.id(r.as_token.span, r.span(), "item.use.rename");
    }

    fn visit_path_segment(&mut self, s: &'ast syn::PathSegment) {
        let name = s.ident.to_string();
        let defined = self
            .env
            .facts
            .types
            .get(&name)
            .or(self.env.facts.fns.get(&name));
        // Our `Result` aliases only fill in std's error type, so their
        // `Result<(), E>` is still 0005's.
        let owner = match (name.as_str(), defined) {
            ("Result", _) | (_, None) => Owner::Theirs,
            (_, Some(_)) => Owner::Ours,
        };
        let args = match &s.arguments {
            syn::PathArguments::AngleBracketed(a) => a
                .args
                .iter()
                .filter_map(|arg| match arg {
                    GenericArgument::Type(t) => Some(t),
                    GenericArgument::AssocType(b) => Some(&b.ty),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        };
        let error = match name.as_str() {
            "Result" => args.get(1).and_then(|t| syntax::type_name(t)),
            _ => None,
        };
        self.hits
            .extend(error.map(|e| Hit::Fact(Fact::ErrorType(e))));
        let slots = args.iter().map(|t| {
            let slot = Slot::GenericArg {
                segment: name.clone(),
                owner,
            };
            (key(t.span()), slot)
        });
        self.slots
            .extend(slots.collect::<Vec<((usize, usize), Slot)>>());
        visit::visit_path_segment(self, s);
    }

    fn visit_type(&mut self, t: &'ast Type) {
        let outer = self.cx.clone();
        let cx = self.at(t.span());
        let extent = syntax::extent(t.span());
        for (at, name) in type_id(t, &cx).iter() {
            self.typed_id(at.to_owned(), t.span(), name);
        }
        let lifetime = match t {
            Type::Reference(r) => r.lifetime.as_ref(),
            _ => None,
        };
        let statics = checks::static_str(t).and(lifetime);
        let lifetime_id = statics.map_or("type.lifetime", |_| "type.lifetime.static_str");
        for l in lifetime.iter() {
            self.id(l.span(), l.span(), lifetime_id);
        }
        self.statics.extend(statics.map(|l| key(l.span())));
        self.hits.extend(checks::ty(t, &cx, &self.env));
        let class = classify(t, &self.env);
        let typed = cx.place.map(|place| {
            Hit::Typed(Typed {
                extent,
                class,
                place,
            })
        });
        let noted = match (
            cx.typing,
            cx.place,
            class,
            checks::transparent(t, &self.env),
        ) {
            (Within::Out, _, _, _) => Vec::new(),
            (Within::In, _, TypeClass::Untyped, _) => {
                Vec::from([refine(extent, "type.untyped"), Hit::Fold(extent)])
                    .into_iter()
                    .chain(typed)
                    .collect()
            }
            (Within::In, Some(_), TypeClass::Primitive, _) => {
                Vec::from([refine(extent, "type.primitive"), Hit::Fold(extent)])
                    .into_iter()
                    .chain(typed)
                    .collect()
            }
            (Within::In, Some(Place::Field), _, transparent) => Some(refine(extent, "type.field"))
                .into_iter()
                .chain(checks::absent(transparent).and(typed))
                .collect(),
            (Within::In, Some(Place::Signature), _, transparent) => {
                checks::absent(transparent).and(typed).into_iter().collect()
            }
            (Within::In, None, _, _) => Vec::new(),
        };
        self.hits.extend(noted);
        // A primitive or untyped type folds its insides into itself.
        self.cx.typing = match (class, cx.place) {
            (TypeClass::Untyped, _) | (TypeClass::Primitive, Some(_)) => Within::Out,
            _ => cx.typing,
        };
        if let Type::Reference(r) = t {
            self.cx.mutable_type = r.mutability.map_or(cx.mutable_type, |_| Within::In);
            self.slot(r.elem.span(), Slot::RefTarget(Box::new(cx.slot)));
        }
        visit::visit_type(self, t);
        self.cx = outer;
    }

    fn visit_expr(&mut self, e: &'ast Expr) {
        let outer = self.cx.clone();
        let cx = self.at(e.span());
        self.cx.place = None;
        let node = e.span();
        for (at, name) in expr_id(e).iter() {
            self.expr_named(at.to_owned(), node, name);
        }
        let named = match e {
            Expr::Path(p) => match p.qself {
                None => checks::named(
                    &syntax::segments(&p.path),
                    checks::Named::Path,
                    p.span(),
                    &self.env,
                ),
                Some(_) => Vec::new(),
            },
            Expr::MethodCall(m) => when(
                self.env
                    .operator_methods
                    .get(&m.method.to_string())
                    .and(Some(())),
                flag(m.method.span(), Rule::OperatorMethod),
            ),
            _ => Vec::new(),
        };
        self.hits.extend(named);
        self.hits.extend(when(
            checks::library_value(&cx.slot),
            mark(node, Fixed::IfRestricted),
        ));
        let rules = checks::expr(e, &cx, &self.env);
        self.hits.extend(rules);
        match e {
            Expr::Call(c) => {
                let callee = match c.func.as_ref() {
                    Expr::Path(p) => Callee {
                        path: syntax::segments(&p.path),
                        owner: ours(&p.path, &self.env),
                    },
                    _ => Callee {
                        path: Vec::new(),
                        owner: Owner::Theirs,
                    },
                };
                self.hits.extend(checks::call(c, &callee, &cx));
                for a in c.args.iter() {
                    self.slot(a.span(), Slot::CallArg(callee.clone()));
                }
                visit::visit_expr(self, e);
            }
            Expr::MethodCall(m) => {
                self.slot(m.receiver.span(), Slot::MethodReceiver);
                let method = m.method.to_string();
                self.hits
                    .extend(
                        self.env
                            .lists
                            .adapters
                            .iter()
                            .find(|a| a == &&method)
                            .map(|_| {
                                Hit::Refine(Refinement {
                                    extent: syntax::extent(node),
                                    from: Some("expr.method_call".to_string()),
                                    to: "expr.method_call.adapter".to_string(),
                                })
                            }),
                    );
                self.hits.extend(checks::method(m, &cx, &self.env));
                // `into_iter` on a range returns the range itself, so it
                // iterates only when its own result is iterated.
                let iterates =
                    checks::iterates(&method, &self.env).or(match (method.as_str(), cx.iterated) {
                        ("into_iter", Within::In) | ("contains", _) => Some(()),
                        _ => None,
                    });
                for _ in iterates.iter() {
                    self.iterate(&m.receiver);
                }
                let receiver = match syntax::ident_of(&m.receiver).as_deref() {
                    Some("self") => Owner::Ours,
                    _ => Owner::Theirs,
                };
                for a in m.args.iter() {
                    self.slot(
                        a.span(),
                        Slot::MethodArg {
                            method: method.clone(),
                            receiver,
                        },
                    );
                }
                match method.as_str() {
                    "zip" | "chain" => {
                        for a in m.args.iter() {
                            self.iterate(a);
                        }
                    }
                    _ => {}
                }
                visit::visit_expr(self, e);
            }
            Expr::Struct(s) => {
                let owner = ours(&s.path, &self.env);
                let wire = self
                    .env
                    .facts
                    .wire
                    .get(&syntax::last_name(&s.path))
                    .map_or(Within::Out, |_| Within::In);
                let outside = match self.builds.get(&key(node)) {
                    Some(_) => None,
                    None => Some(built(&s.path, &self.cx.module)),
                };
                self.hits.extend(outside);
                self.hits.extend(checks::struct_expr(s, owner, &cx));
                for f in s.fields.iter() {
                    self.slot(f.expr.span(), Slot::FieldValue { owner, wire });
                }
                visit::visit_expr(self, e);
            }
            Expr::Closure(c) => {
                if let Slot::LocalClosure = cx.slot {
                    for input in &c.inputs {
                        self.slot(input.span(), Slot::LocalClosureParam);
                    }
                }
                let name = match cx.slot {
                    Slot::MethodArg { .. } => "expr.closure.adapter",
                    _ => "expr.closure",
                };
                self.id(c.span(), c.span(), name);
                let modifiers = c
                    .asyncness
                    .map(|t| (t.span, "expr.closure.async"))
                    .into_iter()
                    .chain(c.constness.map(|t| (t.span, "expr.closure.const")))
                    .chain(c.movability.map(|t| (t.span, "expr.closure.static")));
                for (at, name) in modifiers {
                    self.id(at, at, name);
                }
                self.hits.extend(checks::closure(c, &cx.slot, &self.env));
                let predicate = match &cx.slot {
                    Slot::MethodArg { method, .. } => self
                        .env
                        .lists
                        .predicate_adapters
                        .iter()
                        .find(|m| m == &method)
                        .and(Some(())),
                    _ => None,
                };
                for _ in predicate.iter() {
                    self.predicate(&c.body);
                }
                self.cx.in_predicate = predicate.map_or(self.cx.in_predicate, |_| Within::In);
                self.slot(c.body.span(), Slot::ClosureBody(Box::new(cx.slot)));
                visit::visit_expr(self, e);
            }
            Expr::Reference(r) => {
                if let Some(_token) = r.mutability {
                    self.slot(r.expr.span(), Slot::MutableArray);
                }
                // A shared reference preserves a literal array's slot.
                if let (
                    None,
                    Slot::CallArg(_)
                    | Slot::MethodArg { .. }
                    | Slot::MethodReceiver
                    | Slot::ConstInit,
                ) = (r.mutability, &cx.slot)
                {
                    self.slot(r.expr.span(), cx.slot.clone());
                }
                visit::visit_expr(self, e);
            }
            Expr::Binary(b) => {
                if let (Within::In, BinOp::And(_) | BinOp::Or(_)) = (cx.predicate, b.op) {
                    self.predicates.insert(key(b.left.span()));
                    self.predicates.insert(key(b.right.span()));
                }
                visit::visit_expr(self, e);
            }
            Expr::Paren(p) => {
                match cx.slot {
                    Slot::Other => {}
                    _ => self.slot(p.expr.span(), cx.slot.clone()),
                }
                match cx.iterated {
                    Within::In => self.iterate(&p.expr),
                    Within::Out => {}
                }
                visit::visit_expr(self, e);
            }
            Expr::Group(g) => {
                match cx.slot {
                    Slot::Other => {}
                    _ => self.slot(g.expr.span(), cx.slot.clone()),
                }
                if let Within::In = cx.iterated {
                    self.iterate(&g.expr);
                }
                visit::visit_expr(self, e);
            }
            Expr::Loop(l) => {
                let actor = cx
                    .func
                    .as_ref()
                    .and_then(|f| f.actor_loop)
                    .filter(|extent| extent == &syntax::extent(node));
                self.cx.lp = actor.map_or(Loop::Other, |_| Loop::Actor);
                for a in l.attrs.iter() {
                    self.visit_attribute(a);
                }
                self.visit_block(&l.body);
            }
            Expr::ForLoop(f) => {
                self.iterate(&f.expr);
                for a in f.attrs.iter() {
                    self.visit_attribute(a);
                }
                self.visit_pat(&f.pat);
                self.visit_expr(&f.expr);
                self.cx.lp = Loop::Other;
                self.visit_block(&f.body);
            }
            Expr::Match(m) => {
                self.hits.extend(checks::guards(m));
                visit::visit_expr(self, e);
            }
            Expr::If(i) => match i.cond.as_ref() {
                Expr::Let(cond) => {
                    let name = i
                        .else_branch
                        .as_ref()
                        .map_or("expr.if_let.single", |_| "expr.if_let");
                    self.id(i.if_token.span, node, name);
                    self.hits.extend(match i.else_branch {
                        Some(_) => None,
                        None => Some(Hit::Fold(
                            syntax::extent(i.if_token.span).to(syntax::extent(cond.expr.span())),
                        )),
                    });
                    self.hits.extend(when(
                        checks::filtered_once(&cond.expr),
                        flag(i.if_token.span, Rule::LiftMatched),
                    ));
                    self.let_head(&i.attrs, cond);
                    self.visit_block(&i.then_branch);
                    for (_, e) in i.else_branch.iter() {
                        self.visit_expr(e);
                    }
                }
                cond => {
                    let name = checks::has_let(cond).map_or("expr.if", |_| "expr.if_let");
                    self.id(i.if_token.span, node, name);
                    visit::visit_expr(self, e);
                }
            },
            Expr::While(w) => {
                self.cx.lp = Loop::Other;
                match w.cond.as_ref() {
                    Expr::Let(cond) => {
                        self.id(w.while_token.span, node, "expr.while_let");
                        self.let_head(&w.attrs, cond);
                        self.visit_block(&w.body);
                    }
                    _ => {
                        self.id(w.while_token.span, node, "expr.while");
                        visit::visit_expr(self, e);
                    }
                }
            }
            _ => visit::visit_expr(self, e),
        }
        self.cx = outer;
    }

    fn visit_arm(&mut self, a: &'ast syn::Arm) {
        if let Some((token, guard)) = &a.guard {
            self.guard(token.span, guard);
        }
        visit::visit_arm(self, a);
    }

    fn visit_stmt(&mut self, s: &'ast Stmt) {
        let discard = match s {
            Stmt::Expr(Expr::MethodCall(m), Some(_)) => {
                match (m.method.to_string().as_str(), m.args.first()) {
                    ("ok", None) => Some(flag(m.method.span(), Rule::OkDiscard)),
                    _ => None,
                }
            }
            _ => None,
        };
        self.hits.extend(discard);
        visit::visit_stmt(self, s);
    }

    fn visit_local(&mut self, local: &'ast syn::Local) {
        let outer = self.cx.clone();
        let cx = self.cx.clone();
        let (hits, let_mut) = checks::local(local, &cx, &self.env);
        self.hits.extend(hits);
        for a in local.attrs.iter() {
            self.visit_attribute(a);
        }
        self.cx.let_mut = let_mut;
        self.cx.in_let = Within::In;
        self.visit_pat(&local.pat);
        self.cx = outer;
        if let (Some(_), Some(init)) = (syntax::mut_bindings(&local.pat).first(), &local.init) {
            self.slot(init.expr.span(), Slot::MutableArray);
        }
        for i in local.init.iter() {
            self.visit_local_init(i);
        }
    }

    fn visit_pat(&mut self, p: &'ast Pat) {
        let outer = self.cx.clone();
        let token = match p {
            Pat::Const(c) => Some((c.const_token.span, "pat.const".to_string())),
            Pat::Ident(i) => {
                let mutable = match (i.mutability, self.cx.let_mut) {
                    (Some(_), Within::Out) => Within::In,
                    _ => Within::Out,
                };
                let name = match (i.by_ref, mutable) {
                    (None, Within::Out) => "pat.ident",
                    (None, Within::In) => "pat.ident.mut",
                    (Some(_), Within::Out) => "pat.ident.ref",
                    (Some(_), Within::In) => "pat.ident.ref_mut",
                };
                Some((i.span(), name.to_string()))
            }
            Pat::Lit(l) => Some((
                l.span(),
                match l.lit {
                    Lit::Bool(_) => "pat.lit.bool".to_string(),
                    _ => "pat.lit".to_string(),
                },
            )),
            Pat::Macro(_) => None,
            Pat::Or(o) => Some((o.span(), "pat.or".to_string())),
            Pat::Paren(pa) => Some((pa.span(), "pat.paren".to_string())),
            Pat::Path(pa) => Some((pa.span(), "pat.path".to_string())),
            Pat::Range(r) => Some((r.limits.span(), "pat.range".to_string())),
            Pat::Reference(r) => Some((r.and_token.span, "pat.reference".to_string())),
            Pat::Rest(r) => Some((r.span(), "pat.rest".to_string())),
            Pat::Slice(s) => Some((s.span(), "pat.slice".to_string())),
            Pat::Struct(s) => Some((s.span(), "pat.struct".to_string())),
            Pat::Tuple(t) => Some((t.span(), "pat.tuple".to_string())),
            Pat::TupleStruct(t) => Some((t.span(), "pat.tuple_struct".to_string())),
            Pat::Type(t) => Some((t.colon_token.span, "pat.type".to_string())),
            Pat::Wild(w) => Some((w.span(), "pat.wild".to_string())),
            _ => Some((p.span(), "unknown.pat".to_string())),
        };
        for (at, name) in token.iter() {
            self.id(at.to_owned(), p.span(), name);
        }
        match p {
            Pat::Reference(reference) => {
                self.hits.extend(when(
                    reference.mutability.and(Some(())),
                    flag(reference.and_token.span, Rule::SharedPattern),
                ));
                visit::visit_pat(self, p);
            }
            Pat::Tuple(tuple) => {
                for rest in tuple.elems.iter().filter(|p| matches!(p, Pat::Rest(_))) {
                    self.slot(rest.span(), Slot::SliceRest);
                }
                visit::visit_pat(self, p);
            }
            Pat::TupleStruct(tuple) => {
                for rest in tuple.elems.iter().filter(|p| matches!(p, Pat::Rest(_))) {
                    self.slot(rest.span(), Slot::SliceRest);
                }
                visit::visit_pat(self, p);
            }
            Pat::Range(range) => {
                self.hits.extend(checks::range_pattern(range));
                for attr in &range.attrs {
                    self.visit_attribute(attr);
                }
                for bound in range.start.iter().chain(&range.end) {
                    self.range_bound(bound);
                }
            }
            Pat::Slice(slice) => {
                for rest in slice.elems.iter().flat_map(checks::slice_rests) {
                    self.slot(rest, Slot::SliceRest);
                }
                visit::visit_pat(self, p);
            }
            Pat::Rest(rest) => {
                let placed = self
                    .slots
                    .get(&key(rest.span()))
                    .filter(|slot| matches!(slot, Slot::SliceRest))
                    .and(Some(()));
                self.hits
                    .extend(unless(placed, flag(rest.span(), Rule::SliceRest)));
            }
            Pat::Type(t) => {
                self.hits
                    .extend(checks::closure_param(t, self.slots.get(&key(p.span()))));
                visit::visit_pat(self, p);
            }
            Pat::Ident(i) => {
                self.hits
                    .extend(i.subpat.iter().map(|(at, _)| at.span).map(|at| {
                        let reason = self
                            .env
                            .lists
                            .reasons
                            .get("pat.ident.at")
                            .cloned()
                            .unwrap_or_default();
                        Hit::Id(Occurrence {
                            id: "pat.ident.at".to_string(),
                            reason,
                            at: syntax::pos(at),
                            extent: syntax::extent(at),
                        })
                    }));
                self.hits.extend(checks::pat_ident(i, &self.cx));
                self.hits.extend(checks::ident_at(i));
                visit::visit_pat(self, p);
            }
            Pat::Lit(_) | Pat::Path(_) => {}
            _ => visit::visit_pat(self, p),
        }
        self.cx = outer;
    }

    fn visit_macro(&mut self, m: &'ast syn::Macro) {
        let full = syntax::path_string(&m.path);
        let last = syntax::last_name(&m.path);
        if let "include_str" = last.as_str() {
            self.hits
                .extend(checks::embedded_string(m, &self.at(m.span()).slot));
        }
        let test = match self.cx.test {
            Within::In => self.env.lists.test_macros.clone(),
            Within::Out => Vec::new(),
        };
        let listed = self
            .env
            .lists
            .macros
            .iter()
            .chain(&test)
            .find(|n| n.as_str() == full || n.as_str() == last);
        let env_var = match full.as_str() {
            "env" => Some(checks::build_constant(m)),
            _ => None,
        };
        match (env_var, listed) {
            (Some(None), _) => self.named(
                m.path.span(),
                m.span(),
                "macro.env".to_string(),
                &table::message(Rule::Env),
            ),
            (_, Some(_)) => self.id(m.path.span(), m.span(), "macro.allowed"),
            (_, None) => self.named(
                m.path.span(),
                m.span(),
                format!("macro.{full}"),
                table::UNLISTED_MACRO,
            ),
        }
        self.hits.extend(unless(
            STD_MACROS.contains(&full.as_str()).then_some(()),
            mark(m.span(), Fixed::Always),
        ));
        let cx = Cx {
            predicate: self
                .predicates
                .get(&key(m.span()))
                .map_or(Within::Out, |_| Within::In),
            ..self.cx.clone()
        };
        self.hits.extend(checks::matches_macro(m, &cx));
    }
}

impl Walk {
    fn typed_id(&mut self, at: Span, node: Span, name: &str) {
        match Some(name).filter(|n| n.starts_with("type.interior_mut.")) {
            Some(_) => self.named(at, node, name.to_string(), table::INTERIOR_MUT_TYPE),
            None => self.id(at, node, name),
        }
    }

    fn expr_named(&mut self, at: Span, node: Span, name: &str) {
        match Some(name).filter(|n| n.starts_with("expr.interior_mut.")) {
            Some(_) => self.named(at, node, name.to_string(), table::INTERIOR_MUT_EXPR),
            None => self.id(at, node, name),
        }
    }

    /// An id whose node is an extent rather than a span.
    fn id_over(&mut self, at: Span, node: Extent, name: &str) {
        let reason = self
            .env
            .lists
            .reasons
            .get(name)
            .cloned()
            .unwrap_or_default();
        self.hits.push(Hit::Id(Occurrence {
            id: name.to_string(),
            reason,
            at: syntax::pos(at),
            extent: node,
        }));
    }

    /// A match guard: its id, and its place in the predicate trees.
    fn guard(&mut self, token: Span, guard: &Expr) {
        let node = syntax::extent(token).to(syntax::extent(guard.span()));
        self.id_over(token, node, "expr.match.guard");
        let predicate = match guard {
            Expr::Binary(b) => checks::comparison_op(&b.op).and(Some(key(guard.span()))),
            Expr::Macro(m) => m.mac.path.is_ident("matches").then_some(key(m.mac.span())),
            _ => None,
        };
        self.predicates.extend(predicate);
    }
}

/// Every path a `use` tree brings in.
pub(crate) fn use_paths(tree: &syn::UseTree, prefix: &Vec<String>) -> Vec<UsePath> {
    match tree {
        syn::UseTree::Path(p) => {
            let longer = prefix
                .iter()
                .cloned()
                .chain(Some(p.ident.to_string()))
                .collect();
            use_paths(&p.tree, &longer)
        }
        syn::UseTree::Name(n) => Vec::from([leaf(prefix, &n.ident, Some(n.ident.to_string()))]),
        syn::UseTree::Rename(r) => Vec::from([leaf(prefix, &r.ident, Some(r.rename.to_string()))]),
        syn::UseTree::Glob(g) => Vec::from([UsePath {
            segments: prefix.clone(),
            name: None,
            span: g.star_token.span,
        }]),
        syn::UseTree::Group(g) => g.items.iter().flat_map(|t| use_paths(t, prefix)).collect(),
    }
}

fn leaf(prefix: &[String], ident: &syn::Ident, name: Option<String>) -> UsePath {
    UsePath {
        segments: prefix
            .iter()
            .cloned()
            .chain(Some(ident.to_string()))
            .collect(),
        name,
        span: ident.span(),
    }
}
