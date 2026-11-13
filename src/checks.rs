//! The shape rules of accepted proposals, as checks the walk calls at the
//! node each rule reads. A rule applies only where its id is allowed; where
//! the id isn't, the id itself is the violation.

use std::collections::{BTreeMap, BTreeSet};

use proc_macro2::{Span, TokenTree};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{
    Attribute, BinOp, Block, Expr, Fields, FnArg, GenericParam, Local, Meta, Pat, PathArguments,
    Stmt, Type, TypeParamBound, UnOp, Visibility,
};

use crate::facts::{CfgItem, Element, Fact, Facts};
use crate::syntax;
use crate::table::Rule;
use crate::walk::{
    Callee, Cx, Env, Fixed, Func, Generics, Hit, Host, Owner, Receiver, Refinement, Slot, Within,
    flag, join, mark, unless, when,
};

/// Where a named library item stands (proposals 0038, 0039).
#[derive(Clone, Copy)]
pub(crate) enum Named {
    Use,
    Path,
    Type,
}

/// The rule a path breaks when it names a fallible operator trait or a
/// consuming reader, as the code writes it. A bare name the file defines or
/// imports from a path of ours is ours.
pub(crate) fn named(segments: &[String], kind: Named, at: Span, env: &Env) -> Vec<Hit> {
    let operator = env
        .lists
        .operators
        .iter()
        .find(|o| named_at(&o.module, &o.name, segments, env).is_some());
    let reader = env
        .lists
        .readers
        .iter()
        .find(|r| named_at(&r.module, &r.name, segments, env).is_some());
    let rule = match (operator, reader, kind) {
        (Some(_), _, Named::Use) => Some(Rule::OperatorUse),
        (Some(_), _, Named::Path) => Some(Rule::OperatorPath),
        (Some(_), _, Named::Type) => Some(Rule::OperatorType),
        (None, Some(_), Named::Use) => Some(Rule::ReaderUse),
        (None, Some(_), Named::Path) => Some(Rule::ReaderPath),
        (None, Some(_), Named::Type) => Some(Rule::ReaderType),
        (None, None, _) => None,
    };
    rule.map(|r| flag(at, r)).into_iter().collect()
}

/// Where `segments` name the item `module::name`, unless a bare name is ours.
fn named_at(module: &[String], name: &String, segments: &[String], env: &Env) -> Option<usize> {
    let ours = env.names.ours.get(name).or(env.names.defined.get(name));
    found_in(module, name, segments).filter(|i| i > &0 || ours.is_none())
}

/// The index of the segment that names the item `module::name` in a path as
/// written, where the segments before it agree with the item's module.
pub(crate) fn found_in(module: &[String], name: &str, segments: &[String]) -> Option<usize> {
    segments.iter().enumerate().position(|(i, s)| {
        s == name && module.ends_with(&std_root(segments.iter().take(i).cloned().collect()))
    })
}

/// `core::` and `alloc::` re-export into `std::`, so they read as `std::`.
pub(crate) fn std_root(segments: Vec<String>) -> Vec<String> {
    match segments.split_first() {
        Some((first, rest)) => match first.as_str() {
            "core" | "alloc" => std::iter::once("std".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            _ => segments,
        },
        None => segments,
    }
}

/// `pub(crate)`, the one restricted visibility 0003 allows.
pub(crate) fn pub_crate(vis: &Visibility) -> Option<&syn::VisRestricted> {
    match vis {
        Visibility::Restricted(r) => {
            Some(r).filter(|r| r.in_token.is_none() && r.path.is_ident("crate"))
        }
        _ => None,
    }
}

pub(crate) fn globs(tree: &syn::UseTree, prefix: &Vec<String>) -> Vec<Hit> {
    match tree {
        syn::UseTree::Path(p) => {
            let longer = prefix
                .iter()
                .cloned()
                .chain(Some(p.ident.to_string()))
                .collect();
            globs(&p.tree, &longer)
        }
        syn::UseTree::Glob(g) => match prefix.join("::").as_str() {
            "dioxus::prelude" => Vec::new(),
            _ => Vec::from([flag(g.star_token.span, Rule::Glob)]),
        },
        syn::UseTree::Group(g) => g.items.iter().flat_map(|t| globs(t, prefix)).collect(),
        _ => Vec::new(),
    }
}

/// The other platform's predicate of an allowed `cfg` predicate.
fn complement(predicate: &str) -> Option<&str> {
    match predicate {
        "unix" => Some("not(unix)"),
        "not(unix)" => Some("unix"),
        "target_family=\"wasm\"" => Some("not(target_family=\"wasm\")"),
        "not(target_family=\"wasm\")" => Some("target_family=\"wasm\""),
        _ => None,
    }
}

/// The predicate of an allowed `cfg`, with whitespace removed.
pub(crate) fn cfg_predicate(attr: &Attribute) -> Option<String> {
    let predicate = syntax::attr_tokens(attr);
    complement(&predicate).map(|_| predicate.clone())
}

pub(crate) fn cfg(attr: &Attribute, env: &Env, host: &Host) -> Vec<Hit> {
    let at = attr.pound_token.span;
    match (cfg_predicate(attr), host, env.test_file) {
        (None, _, _) => Vec::from([flag(at, Rule::CfgPredicate)]),
        (Some(_), Host::Cfg(None), _) | (Some(_), Host::Cfg(Some(_)), Within::In) => Vec::new(),
        (Some(predicate), Host::Cfg(Some(item)), Within::Out) => {
            let wanted = CfgItem {
                predicate: complement(&predicate).unwrap_or_default().to_string(),
                ..item.clone()
            };
            unless(
                env.cfg_items.iter().find(|c| c == &&wanted).and(Some(())),
                flag(at, Rule::CfgPair),
            )
        }
        (Some(_), _, _) => Vec::from([flag(at, Rule::CfgHost)]),
    }
}

/// `#[serde]` keys 0012 and 0043 allow.
const SERDE_KEYS: &[&str] = &[
    "transparent",
    "tag",
    "content",
    "rename_all",
    "rename",
    "untagged",
    "skip_serializing_if",
];

pub(crate) fn serde(attr: &Attribute) -> Vec<Hit> {
    let keys = syntax::meta_keys(attr);
    let unlisted = keys.iter().find(|k| !SERDE_KEYS.contains(&k.as_str()));
    let skip = keys
        .iter()
        .find(|k| k.as_str() == "skip_serializing_if")
        .filter(|_| !syntax::attr_tokens(attr).contains("skip_serializing_if=\"Option::is_none\""));
    when(
        unlisted.or(skip).and(Some(())),
        flag(attr.pound_token.span, Rule::SerdeKeys),
    )
}

/// What a `thiserror::Error` derive is on.
#[derive(Clone, Copy)]
pub(crate) enum ErrorHost {
    Named,
    Tuple,
    Enum,
}

pub(crate) fn thiserror(
    attrs: &[Attribute],
    ident: &syn::Ident,
    host: ErrorHost,
    env: &Env,
) -> Vec<Hit> {
    let name = ident.to_string();
    let error = Some(&name)
        .filter(|n| n.ends_with("Error"))
        .or(env.facts.error_types.get(&name));
    let fits = match host {
        ErrorHost::Tuple => None,
        ErrorHost::Named | ErrorHost::Enum => error,
    };
    match syntax::derive_of(attrs, &["thiserror::Error"]) {
        Some(at) => unless(fits.and(Some(())), flag(at, Rule::ThiserrorHost)),
        None => Vec::new(),
    }
}

/// Each error variant, or an error struct, carries one `#[error("…")]`.
pub(crate) fn messages(item: &[Attribute], attrs: &[Attribute], at: Span) -> Vec<Hit> {
    let messages = attrs
        .iter()
        .filter(|a| a.path().is_ident("error"))
        .collect::<Vec<&Attribute>>();
    let one = match (messages.first(), messages.len()) {
        (Some(only), 1) => has_message(only),
        _ => None,
    };
    let flagged = match (one, messages.first()) {
        (Some(_), _) => Vec::new(),
        (None, None) => Vec::from([flag(at, Rule::ErrorMessage)]),
        (None, Some(first)) => Vec::from([flag(first.pound_token.span, Rule::ErrorMessage)]),
    };
    match syntax::derive_of(item, &["thiserror::Error"]) {
        Some(_) => flagged,
        None => Vec::new(),
    }
}

fn has_message(attr: &Attribute) -> Option<()> {
    match &attr.meta {
        Meta::List(list) => match list.tokens.clone().into_iter().next() {
            Some(TokenTree::Literal(lit)) => lit.to_string().starts_with('"').then_some(()),
            _ => None,
        },
        _ => None,
    }
}

pub(crate) fn ord(attrs: &[Attribute], newtype: Within) -> Vec<Hit> {
    match (syntax::derive_named(attrs, &["Ord", "PartialOrd"]), newtype) {
        (Some(at), Within::Out) => Vec::from([flag(at, Rule::OrdHost)]),
        _ => Vec::new(),
    }
}

pub(crate) fn from_once(variants: &Punctuated<syn::Variant, syn::token::Comma>) -> Vec<Hit> {
    let held = variants
        .iter()
        .flat_map(|v| v.fields.iter().map(|f| &f.ty))
        .collect::<Vec<&Type>>();
    variants
        .iter()
        .flat_map(|v| v.fields.iter())
        .filter_map(|field| {
            let from = field.attrs.iter().find(|a| a.path().is_ident("from"))?;
            match held.iter().filter(|t| t == &&&field.ty).count() {
                0 | 1 => None,
                _ => Some(from.pound_token.span),
            }
        })
        .map(|at| flag(at, Rule::FromOnce))
        .collect()
}

pub(crate) fn impl_name(i: &syn::ItemImpl) -> String {
    let ty = syntax::type_name(i.self_ty.as_ref()).unwrap_or_default();
    match &i.trait_ {
        Some((_, path, _)) => format!("{} for {ty}", syntax::path_string(path)),
        None => ty,
    }
}

pub(crate) fn newtype(fields: &Fields) -> Option<&syn::Field> {
    match fields {
        Fields::Unnamed(f) => match f.unnamed.len() {
            1 => f.unnamed.first(),
            _ => None,
        },
        _ => None,
    }
}

/// `&str` or `&'static str`.
pub(crate) fn str_ref(ty: &Type) -> Option<&Type> {
    match ty {
        Type::Reference(r) => match (r.mutability, r.elem.as_ref()) {
            (None, Type::Path(p)) => match p.qself {
                None => p.path.is_ident("str").then_some(ty),
                Some(_) => None,
            },
            _ => None,
        },
        _ => None,
    }
}

/// `&'static str`.
pub(crate) fn static_str(ty: &Type) -> Option<&Type> {
    match ty {
        Type::Reference(r) => r
            .lifetime
            .as_ref()
            .filter(|l| l.ident == "static")
            .and(str_ref(ty)),
        _ => None,
    }
}

/// The element type of `&[T]`, `[T; N]` or `&[T; N]`.
pub(crate) fn table_element(ty: &Type) -> Option<&Type> {
    match ty {
        Type::Reference(r) => match r.elem.as_ref() {
            Type::Slice(s) => Some(s.elem.as_ref()),
            Type::Array(a) => Some(a.elem.as_ref()),
            _ => None,
        },
        Type::Array(a) => Some(a.elem.as_ref()),
        _ => None,
    }
}

/// Whether a tuple type holds a `&str`, through nested tuples.
fn holds_str(ty: &Type) -> Option<&Type> {
    match ty {
        Type::Tuple(t) => t.elems.iter().find_map(holds_str),
        _ => str_ref(ty),
    }
}

/// A `const` table of tuples with a `&str` in them, whose row is a named
/// struct under proposal 0042.
pub(crate) fn tuple_rows(ty: &Type) -> Vec<Hit> {
    match table_element(ty) {
        Some(row) => match (row, holds_str(row)) {
            (Type::Tuple(_), Some(_)) => Vec::from([flag(row.span(), Rule::StrTupleRow)]),
            _ => Vec::new(),
        },
        None => Vec::new(),
    }
}

#[derive(Clone, Copy)]
enum Phase {
    Prefix,
    Names,
}

/// A path as a module path within the file: `self::` and `super::`
/// resolved, a bare name in the current module. `crate::` paths and paths
/// with generic arguments don't resolve.
pub(crate) fn resolve(path: &syn::Path, module: &[String]) -> Option<Vec<String>> {
    let generic = path
        .segments
        .iter()
        .find(|s| !matches!(s.arguments, PathArguments::None));
    match (path.leading_colon, generic) {
        (None, None) => path
            .segments
            .iter()
            .try_fold((module.to_vec(), Phase::Prefix), |(out, phase), s| {
                let name = s.ident.to_string();
                match (phase, name.as_str()) {
                    (Phase::Prefix, "self") => Some((out, Phase::Prefix)),
                    (Phase::Prefix, "super") => out
                        .split_last()
                        .map(|(_, rest)| (rest.to_vec(), Phase::Prefix)),
                    (Phase::Prefix, "crate") => None,
                    _ => Some((out.into_iter().chain(Some(name)).collect(), Phase::Names)),
                }
            })
            .map(|(out, _)| out),
        _ => None,
    }
}

/// The elements of an array literal, behind `&` and parentheses.
pub(crate) fn literal_rows(expr: &Expr) -> Vec<&Expr> {
    match expr {
        Expr::Reference(e) => literal_rows(e.expr.as_ref()),
        Expr::Paren(e) => literal_rows(e.expr.as_ref()),
        Expr::Array(a) => a.elems.iter().collect(),
        _ => Vec::new(),
    }
}

/// A fn body that is one `loop`, after at most one `let mut` state binding.
pub(crate) fn actor_loop(block: &Block) -> Option<(Option<&Local>, &syn::ExprLoop)> {
    let (last, before) = block.stmts.split_last()?;
    let body = match last {
        Stmt::Expr(Expr::Loop(body), _) => Some(body),
        _ => None,
    }?;
    let state = match (before.first(), before.len()) {
        (None, _) => Some(None),
        (Some(Stmt::Local(local)), 1) => syntax::mut_bindings(&local.pat)
            .first()
            .map(|_| Some(local)),
        _ => None,
    }?;
    Some((state, body))
}

/// One turn takes the next input, then handles it with at most one call or
/// `match`.
pub(crate) fn turn(body: &Block) -> Option<()> {
    match (body.stmts.first(), body.stmts.get(1), body.stmts.len()) {
        (Some(take), _, 1) => takes_input(take),
        (Some(take), Some(handle), 2) => takes_input(take).and(handles(handle)),
        _ => None,
    }
}

fn takes_input(stmt: &Stmt) -> Option<()> {
    match stmt {
        Stmt::Macro(m) => select(&m.mac),
        Stmt::Local(local) => local.init.as_ref().and_then(|init| match init.diverge {
            None => awaits(init.expr.as_ref()).or(select_expr(init.expr.as_ref())),
            Some(_) => None,
        }),
        Stmt::Expr(Expr::Macro(m), _) => select(&m.mac),
        Stmt::Expr(Expr::Match(m), _) => awaits(m.expr.as_ref()).or(m
            .arms
            .iter()
            .all(|arm| {
                select_expr(arm.body.as_ref())
                    .or(awaits(arm.body.as_ref()))
                    .is_some()
            })
            .then_some(())),
        Stmt::Expr(expr, _) => awaits(expr),
        Stmt::Item(_) => None,
    }
}

fn handles(stmt: &Stmt) -> Option<()> {
    match stmt {
        Stmt::Expr(expr, _) => match syntax::strip(expr) {
            Expr::Call(_) | Expr::MethodCall(_) | Expr::Match(_) => Some(()),
            Expr::Assign(a) => match syntax::strip(a.right.as_ref()) {
                Expr::Call(_) | Expr::MethodCall(_) => Some(()),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}

fn select(mac: &syn::Macro) -> Option<()> {
    match syntax::last_name(&mac.path).as_str() {
        "select" => Some(()),
        _ => None,
    }
}

fn select_expr(expr: &Expr) -> Option<()> {
    match expr {
        Expr::Macro(m) => select(&m.mac),
        _ => None,
    }
}

/// An `.await` in `expr`, outside any closure or async block it contains.
fn awaits(expr: &Expr) -> Option<()> {
    match expr {
        Expr::Await(_) => Some(()),
        Expr::Closure(_) | Expr::Async(_) => None,
        _ => syntax::inner_exprs(expr).into_iter().find_map(awaits),
    }
}

/// The type a `let mut` loop state is built from: its ascribed type, the
/// struct it constructs, or the type of the associated fn it calls.
fn state_type(local: &Local) -> Option<String> {
    match (
        &local.pat,
        local.init.as_ref().map(|i| syntax::strip(i.expr.as_ref())),
    ) {
        (Pat::Type(t), _) => syntax::type_name(t.ty.as_ref()),
        (_, Some(Expr::Struct(e))) => Some(syntax::last_name(&e.path)),
        (_, Some(Expr::Call(call))) => match call.func.as_ref() {
            Expr::Path(p) => {
                let names = syntax::segments(&p.path);
                names.iter().rev().nth(1).cloned()
            }
            _ => None,
        },
        _ => None,
    }
}

/// The loop states a fn shows: the `Self` of a `mut self` fn whose body is
/// an actor loop, and the type its `let mut` state is built from.
pub(crate) fn loop_states(sig: &syn::Signature, block: &Block, cx: &Cx) -> Vec<Hit> {
    let owned_mut = match sig.inputs.first() {
        Some(FnArg::Receiver(r)) => match (&r.reference, r.mutability) {
            (None, Some(_)) => cx.imp.as_ref().and_then(|i| i.self_ty.clone()),
            _ => None,
        },
        _ => None,
    };
    match actor_loop(block) {
        Some((state, _)) => owned_mut
            .into_iter()
            .chain(state.and_then(state_type))
            .map(|name| Hit::Fact(Fact::LoopState(name)))
            .collect(),
        None => Vec::new(),
    }
}

/// Everything the rules read about one fn, gathered at its head.
pub(crate) fn func(sig: &syn::Signature, block: &Block, env: &Env) -> Func {
    let receiver = match sig.inputs.first() {
        Some(FnArg::Receiver(r)) => match (&r.reference, r.mutability) {
            (Some(_), Some(_)) => Receiver::RefMut,
            (Some(_), None) => Receiver::Shared,
            (None, Some(_)) => Receiver::OwnedMut,
            (None, None) => Receiver::Owned,
        },
        _ => Receiver::None,
    };
    let actor = actor_loop(block);
    let state = actor.and_then(|(state, _)| state);
    let assigned = syntax::block_descendants(block)
        .into_iter()
        .filter_map(|e| match e {
            Expr::Assign(a) => syntax::ident_of(a.left.as_ref()),
            _ => None,
        })
        .collect();
    let params = sig
        .inputs
        .iter()
        .filter_map(|arg| match arg {
            FnArg::Typed(t) => match (t.pat.as_ref(), syntax::type_name(t.ty.as_ref())) {
                (Pat::Ident(p), Some(ty)) => Some((p.ident.to_string(), ty)),
                _ => None,
            },
            FnArg::Receiver(_) => None,
        })
        .collect();
    let locals = syntax::locals(block);
    let some_bindings = locals
        .iter()
        .filter_map(|l| named_local(l))
        .filter(|(_, init)| some_call(init).is_some())
        .map(|(name, _)| name)
        .collect();
    let stream_bindings = locals.iter().filter_map(|l| named_local(l)).fold(
        BTreeSet::new(),
        |names, (name, init)| match stream_root(init, &names) {
            Some(_) => names.into_iter().chain(Some(name)).collect(),
            None => names,
        },
    );
    let table_bindings = locals
        .iter()
        .filter_map(|l| named_local(l))
        .filter(|(_, init)| env.tables.scanned(init, &BTreeMap::new()).is_some())
        .map(|(name, init)| (name, init.clone()))
        .collect();
    Func {
        receiver,
        actor_loop: actor.map(|(_, body)| syntax::extent(body.span())),
        state: state.map(|s| syntax::extent(s.span())),
        state_names: state
            .map(|s| syntax::mut_bindings(&s.pat))
            .unwrap_or_default(),
        assigned,
        params,
        some_bindings,
        stream_bindings,
        table_bindings,
    }
}

/// A `let name = init`, as its name and its init.
fn named_local(local: &Local) -> Option<(String, &Expr)> {
    match (&local.pat, &local.init) {
        (Pat::Ident(p), Some(init)) => Some((p.ident.to_string(), init.expr.as_ref())),
        _ => None,
    }
}

pub(crate) fn fn_attrs(attrs: &[Attribute], sig: &syn::Signature) -> Vec<Hit> {
    attrs
        .iter()
        .filter_map(|attr| {
            let at = attr.pound_token.span;
            let name = syntax::path_string(attr.path());
            match (name.as_str(), &attr.meta, sig.asyncness) {
                ("tokio::test", Meta::Path(_), Some(_)) => None,
                ("tokio::test", _, _) => Some(flag(at, Rule::TokioTest)),
                ("tokio::main", _, _) => match sig.ident.to_string().as_str() {
                    "main" => None,
                    _ => Some(flag(at, Rule::TokioMain)),
                },
                _ => None,
            }
        })
        .collect()
}

pub(crate) fn receiver(sig: &syn::Signature, func: &Func, cx: &Cx, env: &Env) -> Vec<Hit> {
    let token = match sig.inputs.first() {
        Some(FnArg::Receiver(r)) => r.mutability,
        _ => None,
    };
    let state = cx.imp.as_ref().and_then(|i| match i.kind {
        crate::walk::ImplKind::Library => Some(()),
        _ => i.self_ty.as_ref().and_then(|t| {
            env.facts
                .loop_states
                .get(t)
                .or(env.facts.owned_by_state.get(t))
                .or(env.facts.visitors.get(t))
                .or(env.lists.endpoints.iter().find(|e| e == &t))
                .and(Some(()))
        }),
    });
    match (token, func.receiver, func.actor_loop, state) {
        (Some(t), Receiver::OwnedMut, None, _) => Vec::from([flag(t.span, Rule::MutSelf)]),
        (Some(t), Receiver::RefMut, _, None) => Vec::from([flag(t.span, Rule::RefMutSelf)]),
        _ => Vec::new(),
    }
}

/// The type of ours that `segments` name, as in `Attempt`, `Self` or
/// `crate::link::Attempt`.
fn our_type(segments: &[String], cx: &Cx, env: &Env) -> Option<String> {
    let (last, before) = segments.split_last()?;
    match (last.as_str(), before.first()) {
        ("Self", None) => cx.imp.as_ref().and_then(|i| i.self_ty.clone()),
        (_, None) => env
            .names
            .bare_type_is_ours(last, &env.facts)
            .map(|_| last.clone())
            .filter(|name| env.facts.types.contains(name)),
        (_, Some(first)) => env
            .names
            .rooted_in_ours(first, &env.facts)
            .map(|_| last.clone())
            .filter(|name| env.facts.types.contains(name)),
    }
}

/// The type of ours a `mut` binding holds, where the syntax names it: its
/// declared type, the struct or variant its value builds, or the type whose
/// associated fn its value comes from. A type that 0008 or 0035 lets a
/// binding hold `mut` doesn't count.
fn mut_of_ours(ty: Option<&Type>, init: Option<&Expr>, cx: &Cx, env: &Env) -> Option<String> {
    let ours = match (ty, init.map(syntax::strip)) {
        (Some(Type::Path(t)), _) => match t.qself {
            None => our_type(&syntax::segments(&t.path), cx, env),
            Some(_) => None,
        },
        (Some(_), _) => None,
        (None, Some(Expr::Struct(e))) => {
            let names = syntax::segments(&e.path);
            our_type(&names, cx, env).or_else(|| our_type(&shorter(&names)?, cx, env))
        }
        (None, Some(Expr::Call(call))) => match call.func.as_ref() {
            Expr::Path(p) => our_type(&shorter(&syntax::segments(&p.path))?, cx, env),
            _ => None,
        },
        (None, _) => None,
    }?;
    let exempt = env
        .facts
        .loop_states
        .get(&ours)
        .or(env.facts.visitors.get(&ours))
        .or(env.lists.endpoints.iter().find(|e| e == &&ours));
    match exempt {
        Some(_) => None,
        None => Some(ours),
    }
}

/// A path without its last segment.
fn shorter(names: &[String]) -> Option<Vec<String>> {
    names.split_last().map(|(_, rest)| rest.to_vec())
}

pub(crate) fn mut_params(sig: &syn::Signature, cx: &Cx, env: &Env) -> Vec<Hit> {
    sig.inputs
        .iter()
        .filter_map(|arg| match arg {
            FnArg::Typed(t) => match t.pat.as_ref() {
                Pat::Ident(p) => p
                    .mutability
                    .and(mut_of_ours(Some(t.ty.as_ref()), None, cx, env))
                    .map(|_| flag(p.span(), Rule::ParamMutOurs)),
                _ => None,
            },
            FnArg::Receiver(_) => None,
        })
        .collect()
}

/// A `let`: its id and the rules on its `mut`, with whether its pattern is
/// a `stmt.let.mut`'s.
pub(crate) fn local(local: &Local, cx: &Cx, env: &Env) -> (Vec<Hit>, Within) {
    let names = syntax::mut_bindings(&local.pat);
    let name = match (local.init.as_ref().map(|i| &i.diverge), names.first()) {
        (Some(Some(_)), _) => "stmt.let.else",
        (_, Some(_)) => "stmt.let.mut",
        (None, None) => "stmt.let.uninit",
        (Some(None), None) => "stmt.let",
    };
    let extent = syntax::extent(local.span());
    let state = cx
        .func
        .as_ref()
        .and_then(|f| f.state)
        .filter(|s| s == &extent);
    let discard = match &local.pat {
        Pat::Wild(_) => Some(Hit::Refine(Refinement {
            extent,
            from: Some("stmt.let".to_string()),
            to: "stmt.let.discard".to_string(),
        })),
        _ => None,
    };
    let mutable = names.first().filter(|_| state.is_none());
    let reassigned = mutable.and(assigned_here(&names, cx));
    let (binding, ty) = match &local.pat {
        Pat::Type(t) => (t.pat.as_ref(), Some(t.ty.as_ref())),
        pat => (pat, None),
    };
    let ours = match (binding, state) {
        (Pat::Ident(p), None) => p.mutability.and(mut_of_ours(
            ty,
            local.init.as_ref().map(|i| i.expr.as_ref()),
            cx,
            env,
        )),
        _ => None,
    };
    let let_mut = match name {
        "stmt.let.mut" => Within::In,
        _ => Within::Out,
    };
    let reason = env.lists.reasons.get(name).cloned().unwrap_or_default();
    let hits = std::iter::once(Hit::Id(crate::walk::Occurrence {
        id: name.to_string(),
        reason,
        at: syntax::pos(local.let_token.span),
        extent,
    }))
    .chain(discard)
    .chain(when(
        reassigned.and(Some(())),
        flag(local.let_token.span, Rule::LetMut),
    ))
    .chain(when(
        ours.and(Some(())),
        flag(local.let_token.span, Rule::LetMutOurs),
    ))
    .chain(when(
        mutable.and(Some(())),
        mark(local.span(), Fixed::Always),
    ))
    .collect();
    (hits, let_mut)
}

fn assigned_here(names: &[String], cx: &Cx) -> Option<()> {
    cx.func
        .as_ref()
        .and_then(|f| names.iter().find(|n| f.assigned.contains(n.as_str())))
        .and(Some(()))
}

pub(crate) fn pat_ident(p: &syn::PatIdent, cx: &Cx) -> Vec<Hit> {
    match (p.mutability, cx.in_let) {
        (Some(_), Within::Out) => join(Vec::from([
            when(
                assigned_here(&Vec::from([p.ident.to_string()]), cx),
                flag(p.span(), Rule::PatMut),
            ),
            Vec::from([mark(p.span(), Fixed::Always)]),
        ])),
        _ => Vec::new(),
    }
}

/// The `build` parameter of 0026's private `ask<T>`, or its shared free
/// function `ask<M, T>` amendment. Both parameters are unbounded; the shared
/// form takes `&mpsc::Sender<M>` and may be `pub(crate)`.
pub(crate) fn ask_build(vis: &Visibility, sig: &syn::Signature, method: Within) -> Option<()> {
    let (message, reply) = match (
        sig.generics.params.first(),
        sig.generics.params.last(),
        sig.generics.params.len(),
    ) {
        (Some(t), _, 1) => Some((None, unbounded(t)?)),
        (Some(m), Some(t), 2) => Some((Some(unbounded(m)?), unbounded(t)?)),
        _ => None,
    }?;
    match (
        sig.ident.to_string().as_str(),
        sig.inputs.len(),
        &sig.generics.where_clause,
    ) {
        ("ask", 2, None) => Some(()),
        _ => None,
    }?;
    match (message, vis, method) {
        (None, Visibility::Inherited, _) => Some(()),
        (Some(_), Visibility::Inherited, Within::Out) => Some(()),
        (Some(_), _, Within::Out) => pub_crate(vis).and(Some(())),
        _ => None,
    }?;
    let build = match sig.inputs.last() {
        Some(FnArg::Typed(build)) => Some(build),
        _ => None,
    }?;
    let bound = match build.ty.as_ref() {
        Type::ImplTrait(bound) => Some(bound),
        _ => None,
    }?;
    let fn_once = match (bound.bounds.first(), bound.bounds.len()) {
        (Some(TypeParamBound::Trait(b)), 1) => Some(b),
        _ => None,
    }?;
    let segment = fn_once.path.segments.last()?;
    match (
        fn_once.path.segments.len(),
        segment.ident.to_string().as_str(),
    ) {
        (1, "FnOnce") => Some(()),
        _ => None,
    }?;
    let args = match &segment.arguments {
        PathArguments::Parenthesized(args) => Some(args),
        _ => None,
    }?;
    match (args.inputs.first(), args.inputs.len()) {
        (Some(sender), 1) => sender_type(sender, "oneshot", &reply.ident),
        _ => None,
    }?;
    let output = match syntax::returned(&args.output) {
        Some(Type::Path(p)) => Some(p),
        _ => None,
    }?;
    match message {
        None => Some(()),
        Some(m) => {
            let inbox = match sig.inputs.first() {
                Some(FnArg::Typed(inbox)) => match inbox.ty.as_ref() {
                    Type::Reference(r) => match (&r.mutability, &r.lifetime) {
                        (None, None) => Some(r.elem.as_ref()),
                        _ => None,
                    },
                    _ => None,
                },
                _ => None,
            }?;
            sender_type(inbox, "mpsc", &m.ident)?;
            match build.pat.as_ref() {
                Pat::Ident(p) => match p.ident.to_string().as_str() {
                    "build" => Some(()),
                    _ => None,
                },
                _ => None,
            }?;
            Some(output)
                .filter(|p| p.qself.is_none() && p.path.is_ident(&m.ident))
                .and(Some(()))?;
            let returned = syntax::returned(&sig.output)?;
            Some(syntax::type_segments(returned))
                .filter(|names| !names.contains(&m.ident.to_string()))
                .and(Some(()))
        }
    }
}

/// A helper's type parameter has neither a bound nor a default.
fn unbounded(param: &GenericParam) -> Option<&syn::TypeParam> {
    match param {
        GenericParam::Type(t) => Some(t).filter(|t| t.bounds.is_empty() && t.default.is_none()),
        _ => None,
    }
}

/// A channel sender with exactly the helper's named type as its item.
fn sender_type(ty: &Type, channel: &str, item: &syn::Ident) -> Option<()> {
    let sender = match ty {
        Type::Path(p) => Some(p).filter(|p| p.qself.is_none()),
        _ => None,
    }?;
    match &sender.path.segments.last()?.arguments {
        PathArguments::AngleBracketed(a) => match (a.args.first(), a.args.len()) {
            (Some(syn::GenericArgument::Type(Type::Path(p))), 1) => Some(p)
                .filter(|p| p.qself.is_none() && p.path.is_ident(item))
                .and(Some(())),
            _ => None,
        },
        _ => None,
    }?;
    let names = syntax::segments(&sender.path);
    let tail = Vec::from([channel.to_string(), "Sender".to_string()]);
    names.ends_with(&tail).then_some(())
}

/// The codec bounds of proposal 0013.
const CODEC_BOUNDS: &[&str] = &["Serialize", "DeserializeOwned"];

pub(crate) fn generic_param(p: &GenericParam, g: &syn::Generics, cx: &Cx, env: &Env) -> Vec<Hit> {
    let library_bounds = match p {
        GenericParam::Type(t) => t.bounds.first().and(
            t.bounds
                .iter()
                .all(|b| library_bound(b, env).is_some())
                .then_some(()),
        ),
        _ => None,
    };
    let fixed = cx.foreign().and(Some(())).or(library_bounds);
    let rule = match p {
        GenericParam::Type(t) => {
            let single = match (t.bounds.first(), t.bounds.len(), &g.where_clause) {
                (Some(TypeParamBound::Trait(b)), 1, None) => CODEC_BOUNDS
                    .contains(&syntax::last_name(&b.path).as_str())
                    .then_some(()),
                _ => None,
            };
            let codec = match cx.generics {
                Generics::Fn => single,
                Generics::Ask => Some(()),
                Generics::Item => None,
            };
            unless(
                cx.library().and(Some(())).or(codec),
                flag(p.span(), Rule::TypeParam),
            )
        }
        GenericParam::Lifetime(_) => unless(
            cx.library().and(Some(())),
            flag(p.span(), Rule::LifetimeParam),
        ),
        GenericParam::Const(_) => Vec::new(),
    };
    join(Vec::from([
        when(fixed, mark(p.span(), Fixed::Always)),
        rule,
    ]))
}

/// A trait bound naming a trait we don't define.
fn library_bound(bound: &TypeParamBound, env: &Env) -> Option<()> {
    match bound {
        TypeParamBound::Trait(b) => match crate::walk::ours(&b.path, env) {
            Owner::Theirs => Some(()),
            Owner::Ours => None,
        },
        _ => None,
    }
}

/// Whether `ty` has the shape of `pattern`. `_` matches any type. Paths match
/// on their last segment and on their generic arguments where the pattern
/// gives them. A pattern's leading segments must match where the type spells
/// them too; a bare name matches only if it isn't one we define.
pub(crate) fn type_matches(pattern: &Type, ty: &Type, facts: &Facts) -> Option<()> {
    match (pattern, ty) {
        (Type::Infer(_), _) => Some(()),
        (Type::Paren(p), _) => type_matches(p.elem.as_ref(), ty, facts),
        (_, Type::Paren(t)) => type_matches(pattern, t.elem.as_ref(), facts),
        (Type::Reference(p), Type::Reference(t)) => match (p.mutability, t.mutability) {
            (Some(_), Some(_)) | (None, None) => {
                type_matches(p.elem.as_ref(), t.elem.as_ref(), facts)
            }
            _ => None,
        },
        (Type::Path(p), Type::Path(t)) => match (&p.qself, &t.qself) {
            (None, None) => path_matches(&p.path, &t.path, facts),
            _ => None,
        },
        (Type::TraitObject(p), Type::TraitObject(t)) => p
            .bounds
            .iter()
            .all(|pb| {
                t.bounds
                    .iter()
                    .any(|tb| bounds_match(pb, tb, facts).is_some())
            })
            .then_some(()),
        _ => None,
    }
}

fn bounds_match(pattern: &TypeParamBound, bound: &TypeParamBound, facts: &Facts) -> Option<()> {
    match (pattern, bound) {
        (TypeParamBound::Trait(p), TypeParamBound::Trait(b)) => {
            path_matches(&p.path, &b.path, facts)
        }
        _ => None,
    }
}

fn argument_matches(
    pattern: &syn::GenericArgument,
    arg: &syn::GenericArgument,
    facts: &Facts,
) -> Option<()> {
    match (pattern, arg) {
        (syn::GenericArgument::Type(p), syn::GenericArgument::Type(t)) => type_matches(p, t, facts),
        _ => None,
    }
}

fn path_matches(pattern: &syn::Path, path: &syn::Path, facts: &Facts) -> Option<()> {
    let want = syntax::segments(pattern);
    let have = syntax::segments(path);
    let shared = want.len().min(have.len());
    let want_tail = want.iter().rev().take(shared).collect::<Vec<&String>>();
    let have_tail = have.iter().rev().take(shared).collect::<Vec<&String>>();
    let bare_and_ours = match have.len().cmp(&want.len()) {
        std::cmp::Ordering::Less => facts.owner_of(&have),
        _ => None,
    };
    let p_last = pattern.segments.last()?;
    let t_last = path.segments.last()?;
    let args = match (&p_last.arguments, &t_last.arguments) {
        (PathArguments::None, _) => Some(()),
        (PathArguments::AngleBracketed(p), PathArguments::AngleBracketed(t)) => p
            .args
            .iter()
            .zip(&t.args)
            .all(|(p, t)| argument_matches(p, t, facts).is_some())
            .then_some(())
            .filter(|_| p.args.len() == t.args.len()),
        _ => None,
    };
    match (bare_and_ours, args) {
        (None, Some(_)) => match (shared, want_tail) {
            (0, _) => None,
            (_, tail) if tail == have_tail => Some(()),
            _ => None,
        },
        _ => None,
    }
}

/// A std container that typing sees through, such as `Option<T>`.
pub(crate) fn transparent(ty: &Type, env: &Env) -> Option<()> {
    match ty {
        Type::Path(p) => match (&p.qself, crate::walk::ours(&p.path, env)) {
            (None, Owner::Theirs) => {
                let last = syntax::last_name(&p.path);
                env.lists
                    .transparent
                    .iter()
                    .find(|t| t == &&last)
                    .and(Some(()))
            }
            _ => None,
        },
        _ => None,
    }
}

/// A type argument of a type or fn we don't define (proposal 0023).
fn library_type_arg(slot: &Slot) -> Option<()> {
    match slot {
        Slot::GenericArg {
            owner: Owner::Theirs,
            ..
        } => Some(()),
        _ => None,
    }
}

/// An argument or field value of a fn, method or struct we don't define
/// (proposal 0023). A method on any receiver but `self` counts as not ours.
pub(crate) fn library_value(slot: &Slot) -> Option<()> {
    match slot {
        Slot::CallArg(Callee {
            owner: Owner::Theirs,
            ..
        })
        | Slot::MethodArg {
            receiver: Owner::Theirs,
            ..
        }
        | Slot::FieldValue {
            owner: Owner::Theirs,
            ..
        } => Some(()),
        _ => None,
    }
}

/// The shape rules on one type node.
pub(crate) fn ty(t: &Type, cx: &Cx, env: &Env) -> Vec<Hit> {
    let slot = &cx.slot;
    let named = match t {
        Type::Path(p) => match p.qself {
            None => named(&syntax::segments(&p.path), Named::Type, p.span(), env),
            Some(_) => Vec::new(),
        },
        _ => Vec::new(),
    };
    let fixed_slot = library_type_arg(slot).or(match slot {
        Slot::ClapField | Slot::MainReturn => Some(()),
        _ => None,
    });
    let mut_ref = match t {
        Type::Reference(r) => r.mutability.and(match r.elem.as_ref() {
            Type::Path(p) => match crate::walk::ours(&p.path, env) {
                Owner::Ours => None,
                Owner::Theirs => Some(()),
            },
            _ => Some(()),
        }),
        _ => None,
    };
    let rules = match t {
        Type::Tuple(tu) => match tu.elems.first() {
            None => unless(
                library_type_arg(slot).or(match slot {
                    Slot::MainReturn => Some(()),
                    _ => None,
                }),
                flag(tu.span(), Rule::UnitType),
            ),
            Some(_) => Vec::new(),
        },
        Type::Reference(r) => {
            let static_str = r
                .lifetime
                .as_ref()
                .and(static_str(t))
                .and(r.lifetime.as_ref());
            let row = match slot {
                Slot::RowField => Some(()),
                _ => None,
            };
            let ours = match (r.mutability, r.elem.as_ref()) {
                (Some(token), Type::Path(p)) => match crate::walk::ours(&p.path, env) {
                    Owner::Ours => Some(token),
                    Owner::Theirs => None,
                },
                _ => None,
            };
            join(Vec::from([
                static_str
                    .map(|l| unless(row, flag(l.span(), Rule::StaticStr)))
                    .unwrap_or_default(),
                ours.map(|token| flag(token.span, Rule::RefMutOurs))
                    .into_iter()
                    .collect(),
            ]))
        }
        Type::Slice(s) => {
            let read_only = match slot {
                Slot::RefTarget(outer) => match outer.as_ref() {
                    Slot::Param | Slot::ConstType => Some(()),
                    _ => None,
                },
                _ => None,
            };
            unless(read_only, flag(s.span(), Rule::Slice))
        }
        Type::Array(a) => {
            let bytes = match a.elem.as_ref() {
                Type::Path(p) => p.path.is_ident("u8").then_some(()),
                _ => None,
            };
            unless(bytes, flag(a.span(), Rule::ArrayType))
        }
        Type::ImplTrait(i) => match (cx.in_return, slot) {
            (Within::In, _) | (_, Slot::AskBuild) => Vec::new(),
            _ => Vec::from([flag(i.impl_token.span, Rule::ImplTraitArg)]),
        },
        Type::TraitObject(o) => {
            let fn_mut = match (o.bounds.first(), o.bounds.len()) {
                (Some(TypeParamBound::Trait(b)), 1) => match syntax::last_name(&b.path).as_str() {
                    "FnMut" => Some(()),
                    _ => None,
                },
                _ => None,
            };
            let in_closure = match slot {
                Slot::GenericArg { segment, .. } => match segment.as_str() {
                    "Closure" => Some(()),
                    _ => None,
                },
                _ => None,
            };
            unless(fn_mut.and(in_closure), flag(o.span(), Rule::Dyn))
        }
        Type::Path(p) => match (
            &p.qself,
            p.path.get_ident().map(ToString::to_string).as_deref(),
        ) {
            (None, Some("bool")) => unless(
                library_type_arg(slot).or(match slot {
                    Slot::ClapField | Slot::WireField => Some(()),
                    _ => None,
                }),
                flag(p.span(), Rule::BoolType),
            ),
            _ => Vec::new(),
        },
        _ => Vec::new(),
    };
    join(Vec::from([
        named,
        when(fixed_slot, mark(t.span(), Fixed::IfRestricted)),
        when(mut_ref, mark(t.span(), Fixed::Always)),
        rules,
    ]))
}

/// `env!` of a variable Cargo sets at build time (proposal 0022).
pub(crate) fn build_constant(mac: &syn::Macro) -> Option<String> {
    let args = mac
        .parse_body_with(Punctuated::<Expr, syn::token::Comma>::parse_terminated)
        .ok()?;
    match args.first() {
        Some(Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(s),
            ..
        })) => {
            let name = s.value();
            match name.as_str() {
                "CARGO_PKG_VERSION" | "CARGO_PKG_NAME" | "CARGO_MANIFEST_DIR" => Some(name),
                _ => None,
            }
        }
        _ => None,
    }
}

/// The parts of `matches!(scrutinee, pattern if guard)`.
struct Matches {
    scrutinee: Expr,
    pattern: Pat,
    guard: Option<Expr>,
}

fn parse_matches(mac: &syn::Macro) -> Option<Matches> {
    mac.parse_body_with(matches_body).ok()
}

fn matches_body(input: syn::parse::ParseStream) -> syn::Result<Matches> {
    let scrutinee = input.parse::<Expr>()?;
    input.parse::<syn::token::Comma>()?;
    let pattern = Pat::parse_multi_with_leading_vert(input)?;
    let guard = input
        .parse::<Option<syn::token::If>>()?
        .map(|_| input.parse::<Expr>())
        .transpose()?;
    let _ = input.parse::<Option<syn::token::Comma>>()?;
    Ok(Matches {
        scrutinee,
        pattern,
        guard,
    })
}

/// No inner guard, no `true` / `false` pattern and a scrutinee that is a
/// place or a call: nothing carries a boolean through the macro.
fn plain(m: &Matches) -> Option<()> {
    match (&m.guard, bool_pattern(&m.pattern)) {
        (None, None) => place_or_call(&m.scrutinee),
        _ => None,
    }
}

fn bool_pattern(pat: &Pat) -> Option<()> {
    match pat {
        Pat::Lit(l) => match l.lit {
            syn::Lit::Bool(_) => Some(()),
            _ => None,
        },
        Pat::Ident(p) => p
            .subpat
            .as_ref()
            .and_then(|(_, sub)| bool_pattern(sub.as_ref())),
        Pat::Or(p) => p.cases.iter().find_map(bool_pattern),
        Pat::Paren(p) => bool_pattern(p.pat.as_ref()),
        Pat::Reference(p) => bool_pattern(p.pat.as_ref()),
        Pat::Slice(p) => p.elems.iter().find_map(bool_pattern),
        Pat::Struct(p) => p.fields.iter().find_map(|f| bool_pattern(f.pat.as_ref())),
        Pat::Tuple(p) => p.elems.iter().find_map(bool_pattern),
        Pat::TupleStruct(p) => p.elems.iter().find_map(bool_pattern),
        Pat::Type(p) => bool_pattern(p.pat.as_ref()),
        _ => None,
    }
}

/// A place, a literal or a call whose receiver and arguments are such, or a
/// closure passed to one: no operator, block or branch.
fn place_or_call(expr: &Expr) -> Option<()> {
    match expr {
        Expr::Path(_) | Expr::Closure(_) => Some(()),
        Expr::Tuple(t) => t.elems.iter().try_fold((), |_, e| place_or_call(e)),
        Expr::Lit(l) => match l.lit {
            syn::Lit::Bool(_) => None,
            _ => Some(()),
        },
        Expr::Field(e) => place_or_call(e.base.as_ref()),
        Expr::Reference(e) => place_or_call(e.expr.as_ref()),
        Expr::Paren(e) => place_or_call(e.expr.as_ref()),
        Expr::Unary(e) => match e.op {
            UnOp::Deref(_) => place_or_call(e.expr.as_ref()),
            _ => None,
        },
        Expr::MethodCall(e) => place_or_call(e.receiver.as_ref())
            .and(e.args.iter().try_fold((), |_, e| place_or_call(e))),
        Expr::Call(e) => {
            place_or_call(e.func.as_ref()).and(e.args.iter().try_fold((), |_, e| place_or_call(e)))
        }
        _ => None,
    }
}

pub(crate) fn matches_macro(m: &syn::Macro, cx: &Cx) -> Vec<Hit> {
    let named = Some(m).filter(|m| m.path.is_ident("matches"));
    match (named, cx.predicate) {
        (None, _) => Vec::new(),
        (Some(_), Within::Out) => Vec::from([flag(m.path.span(), Rule::PredicateMatches)]),
        (Some(_), Within::In) => unless(
            parse_matches(m).as_ref().and_then(plain),
            flag(m.path.span(), Rule::MatchesForm),
        ),
    }
}

/// The fields of a row, counted through nested tuples: `(a, (b, c))` has
/// three.
pub(crate) fn leaves(expr: &Expr) -> usize {
    match expr {
        Expr::Tuple(t) => t.elems.iter().map(leaves).sum(),
        _ => 1,
    }
}

fn field_of_self(expr: &Expr) -> Option<()> {
    match expr {
        Expr::Field(f) => match f.base.as_ref() {
            Expr::Path(p) => p.path.is_ident("self").then_some(()),
            base => field_of_self(base),
        },
        _ => None,
    }
}

pub(crate) fn assign(a: &syn::ExprAssign, cx: &Cx) -> Vec<Hit> {
    let to_self = cx
        .func
        .as_ref()
        .and_then(|f| match f.receiver {
            Receiver::RefMut | Receiver::OwnedMut => Some(()),
            _ => None,
        })
        .and(field_of_self(a.left.as_ref()));
    let to_state = syntax::ident_of(a.left.as_ref()).and_then(|name| {
        cx.func
            .as_ref()
            .and_then(|f| f.state_names.iter().find(|n| n == &&name).and(Some(())))
    });
    unless(to_self.or(to_state), flag(a.eq_token.span, Rule::Assign))
}

/// An async block as a spawn argument or the whole body of an `unfold`,
/// `then` or `use_future` closure (proposal 0006).
pub(crate) fn async_placed(slot: &Slot) -> Option<()> {
    match slot {
        Slot::CallArg(callee) => match callee.last() {
            "spawn" => Some(()),
            _ => None,
        },
        Slot::ClosureBody(outer) => match outer.as_ref() {
            Slot::CallArg(callee) => match callee.last() {
                "unfold" | "use_future" => Some(()),
                _ => None,
            },
            Slot::MethodArg { method, .. } => match method.as_str() {
                "then" => Some(()),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}

pub(crate) fn comparison_op(op: &BinOp) -> Option<()> {
    match op {
        BinOp::Eq(_) | BinOp::Ne(_) | BinOp::Lt(_) | BinOp::Le(_) | BinOp::Gt(_) | BinOp::Ge(_) => {
            Some(())
        }
        _ => None,
    }
}

pub(crate) fn predicate_rule(op: &BinOp) -> Option<Rule> {
    match op {
        BinOp::Eq(_) => Some(Rule::PredicateEq),
        BinOp::Ne(_) => Some(Rule::PredicateNe),
        BinOp::Lt(_) => Some(Rule::PredicateLt),
        BinOp::Le(_) => Some(Rule::PredicateLe),
        BinOp::Gt(_) => Some(Rule::PredicateGt),
        BinOp::Ge(_) => Some(Rule::PredicateGe),
        BinOp::And(_) => Some(Rule::PredicateAnd),
        BinOp::Or(_) => Some(Rule::PredicateOr),
        _ => None,
    }
}

/// Methods that compare by name, as `a.eq(&b)` (proposal 0040).
const COMPARE_METHODS: &[&str] = &["eq", "ne", "lt", "le", "gt", "ge"];
/// `Ordering`'s tests, as in `a.cmp(&b).is_eq()`.
const ORDERING_TESTS: &[&str] = &["is_eq", "is_ne", "is_lt", "is_le", "is_gt", "is_ge"];
const COMPARE_TRAITS: &[&str] = &["PartialEq", "PartialOrd", "Ord"];
/// The std range types, whose construction by name is a range expression.
const RANGE_TYPES: &[&str] = &[
    "Range",
    "RangeInclusive",
    "RangeFrom",
    "RangeTo",
    "RangeToInclusive",
    "RangeFull",
];
/// The variants of `std::ops::Bound`, whose construction builds a range.
const BOUNDS: &[&str] = &["Included", "Excluded", "Unbounded"];
/// Callees whose closure argument 0006, 0017, 0018 and 0026 allow.
const CLOSURE_CALLEES: &[&str] = &["unfold", "use_signal", "use_future", "use_hook", "ask"];

/// Holds when `found` is empty.
pub(crate) fn absent(found: Option<()>) -> Option<()> {
    match found {
        Some(_) => None,
        None => Some(()),
    }
}

/// `drop`, `mem::drop` or `std::mem::drop`.
fn is_drop(path: &syn::Path) -> Option<()> {
    let names = syntax::segments(path);
    let tail = Vec::from(["mem".to_string(), "drop".to_string()]);
    match (names.first().map(String::as_str), names.len()) {
        (Some("drop"), 1) => Some(()),
        _ => names.ends_with(&tail).then_some(()),
    }
}

/// The name before a path's last, as `Ord` in `Ord::cmp`.
fn second_last(path: &[String]) -> Option<&str> {
    path.iter().rev().nth(1).map(String::as_str)
}

pub(crate) fn call(c: &syn::ExprCall, callee: &Callee, cx: &Cx) -> Vec<Hit> {
    let last = callee.last();
    let compares = second_last(&callee.path)
        .filter(|t| COMPARE_TRAITS.contains(t))
        .filter(|_| COMPARE_METHODS.contains(&last) || last == "cmp")
        .filter(|_| matches!(cx.in_predicate, Within::Out));
    let drop = match c.func.as_ref() {
        Expr::Path(p) => is_drop(&p.path),
        _ => None,
    };
    let named = match c.args.first() {
        Some(Expr::Path(p)) => p.path.get_ident().and(Some(())),
        _ => None,
    };
    let range_fn = second_last(&callee.path)
        .filter(|t| RANGE_TYPES.contains(t))
        .filter(|_| matches!(cx.iterated, Within::Out));
    let bound = Some(last).filter(|l| BOUNDS.contains(l));
    let range = range_fn
        .or(bound)
        .filter(|_| matches!(callee.owner, Owner::Theirs));
    join(Vec::from([
        when(compares.and(Some(())), flag(c.func.span(), Rule::LiftEq)),
        when(
            drop.and(absent(named)),
            flag(c.func.span(), Rule::DropDiscard),
        ),
        when(range.and(Some(())), flag(c.func.span(), Rule::RangeCall)),
    ]))
}

pub(crate) fn path_expr(p: &syn::ExprPath, cx: &Cx, env: &Env) -> Vec<Hit> {
    let argument = match cx.slot {
        Slot::CallArg(_) | Slot::MethodArg { .. } => Some(()),
        _ => None,
    };
    let drop = argument.and(is_drop(&p.path));
    let last = syntax::last_name(&p.path);
    let range = match (last.as_str(), crate::walk::ours(&p.path, env)) {
        ("RangeFull" | "Unbounded", Owner::Theirs) => Some(()),
        _ => None,
    };
    match (drop, range) {
        (Some(_), _) => Vec::from([flag(p.span(), Rule::DropFn)]),
        (None, Some(_)) => Vec::from([flag(p.span(), Rule::RangePath)]),
        (None, None) => Vec::new(),
    }
}

pub(crate) fn struct_expr(s: &syn::ExprStruct, owner: Owner, cx: &Cx) -> Vec<Hit> {
    let last = syntax::last_name(&s.path);
    let range = match (owner, cx.iterated) {
        (Owner::Theirs, Within::Out) => RANGE_TYPES.contains(&last.as_str()).then_some(()),
        _ => None,
    };
    let base = s.rest.as_ref().and_then(|rest| match rest.as_ref() {
        Expr::Path(_) | Expr::Call(_) | Expr::MethodCall(_) => None,
        _ => Some(()),
    });
    join(Vec::from([
        when(range, flag(s.path.span(), Rule::RangeStruct)),
        when(base, flag(s.span(), Rule::StructUpdate)),
    ]))
}

pub(crate) fn closure(c: &syn::ExprClosure, slot: &Slot, env: &Env) -> Vec<Hit> {
    match slot {
        Slot::MethodArg { method, .. } => match takes_closure(method, env) {
            Some(_) => Vec::new(),
            None => Vec::from([flag(c.span(), Rule::ClosureMethod)]),
        },
        Slot::CallArg(callee) => {
            let names = callee
                .path
                .iter()
                .map(String::as_str)
                .collect::<Vec<&str>>();
            let wrapped = names.ends_with(&["Closure", "new"]).then_some(());
            let listed = CLOSURE_CALLEES.contains(&callee.last()).then_some(());
            unless(wrapped.or(listed), flag(c.span(), Rule::Closure))
        }
        _ => Vec::from([flag(c.span(), Rule::Closure)]),
    }
}

fn predicate_method(method: &str, env: &Env) -> Option<()> {
    env.lists
        .predicate_adapters
        .iter()
        .find(|m| m.as_str() == method)
        .and(Some(()))
}

fn takes_closure(method: &str, env: &Env) -> Option<()> {
    predicate_method(method, env).or(env
        .lists
        .closure_methods
        .iter()
        .find(|m| m.as_str() == method)
        .and(Some(())))
}

/// Whether a method called on a range iterates it.
pub(crate) fn iterates(method: &str, env: &Env) -> Option<()> {
    takes_closure(method, env).or(env
        .lists
        .adapters
        .iter()
        .find(|m| m.as_str() == method)
        .and(Some(())))
}

/// A closure body that is one comparison, through parentheses, `!` and a
/// block holding only its tail.
fn comparison(body: &Expr) -> Option<()> {
    match body {
        Expr::Paren(e) => comparison(e.expr.as_ref()),
        Expr::Unary(e) => match e.op {
            UnOp::Not(_) => comparison(e.expr.as_ref()),
            _ => None,
        },
        Expr::Block(e) => match (&e.label, e.block.stmts.first(), e.block.stmts.len()) {
            (None, Some(Stmt::Expr(tail, None)), 1) => comparison(tail),
            _ => None,
        },
        Expr::Binary(e) => comparison_op(&e.op),
        _ => None,
    }
}

/// `Some(x)`, an `Option` built at the call, by any path (proposal 0040).
fn some_call(expr: &Expr) -> Option<()> {
    match expr {
        Expr::Call(c) => match c.func.as_ref() {
            Expr::Path(p) => match syntax::last_name(&p.path).as_str() {
                "Some" => Some(()),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}

/// An `Option` built only to ask one question: `Some(x)`, a name bound to
/// one in this fn, or either through `into_iter()` or `iter()`.
fn built_option(expr: &Expr, some_bindings: &BTreeSet<String>) -> Option<()> {
    match expr {
        Expr::MethodCall(m) => match m.method.to_string().as_str() {
            "into_iter" | "iter" => built_option(m.receiver.as_ref(), some_bindings),
            _ => None,
        },
        Expr::Path(_) => syntax::ident_of(expr)
            .filter(|name| some_bindings.contains(name))
            .and(Some(())),
        Expr::Paren(e) => built_option(e.expr.as_ref(), some_bindings),
        other => some_call(other),
    }
}

/// `opt.filter(|x| <one comparison>)`, a lift when matched at once.
pub(crate) fn filtered_once(expr: &Expr) -> Option<()> {
    match expr {
        Expr::MethodCall(m) => match (m.method.to_string().as_str(), m.args.first()) {
            ("filter", Some(Expr::Closure(c))) => comparison(c.body.as_ref()),
            _ => None,
        },
        _ => None,
    }
}

/// A chain of method calls that starts at `stream::iter(..)`, a bare `iter(..)`
/// imported from a stream module, or a name bound to such a chain.
fn stream_root(expr: &Expr, streams: &BTreeSet<String>) -> Option<()> {
    match expr {
        Expr::MethodCall(m) => stream_root(m.receiver.as_ref(), streams),
        Expr::Await(a) => stream_root(a.base.as_ref(), streams),
        Expr::Paren(e) => stream_root(e.expr.as_ref(), streams),
        Expr::Call(c) => match c.func.as_ref() {
            Expr::Path(p) => {
                let path = syntax::segments(&p.path);
                let tail = Vec::from(["stream".to_string(), "iter".to_string()]);
                let bare = Vec::from(["iter".to_string()]);
                path.ends_with(&tail).then_some(()).or(match path {
                    p if p == bare => Some(()),
                    _ => None,
                })
            }
            _ => None,
        },
        Expr::Path(_) => syntax::ident_of(expr)
            .filter(|name| streams.contains(name))
            .and(Some(())),
        _ => None,
    }
}

/// Whether `x.0` reads the field of an ordered newtype: `x` is `self` in an
/// impl of one, or a parameter declared as one.
fn ordered_newtype_field(expr: &Expr, cx: &Cx, env: &Env) -> Option<()> {
    let field = match expr {
        Expr::Field(f) => Some(f),
        _ => None,
    }?;
    let index = match &field.member {
        syn::Member::Unnamed(index) => Some(index.index),
        syn::Member::Named(_) => None,
    }?;
    let name = syntax::ident_of(field.base.as_ref())?;
    let ty = match name.as_str() {
        "self" => cx.imp.as_ref().and_then(|i| i.self_ty.clone()),
        _ => cx.func.as_ref().and_then(|f| f.params.get(&name).cloned()),
    }?;
    match index {
        0 => env.facts.ord_newtypes.get(&ty).and(Some(())),
        _ => None,
    }
}

pub(crate) fn method(m: &syn::ExprMethodCall, cx: &Cx, env: &Env) -> Vec<Hit> {
    let method = m.method.to_string();
    let at = m.method.span();
    let no_func = BTreeSet::new();
    let some_bindings = cx.func.as_ref().map_or(&no_func, |f| &f.some_bindings);
    let streams = cx.func.as_ref().map_or(&no_func, |f| &f.stream_bindings);
    let ord = match method.as_str() {
        "cmp" => ordered_newtype_field(m.receiver.as_ref(), cx, env),
        _ => None,
    };
    let count = match (method.as_str(), m.args.first(), m.receiver.as_ref()) {
        ("count", None, Expr::MethodCall(r)) => match r.method.to_string().as_str() {
            "filter_map" => Some(()),
            _ => None,
        },
        _ => None,
    };
    let lift = match (method.as_str(), m.args.first()) {
        ("filter" | "find", Some(Expr::Closure(c))) => {
            built_option(m.receiver.as_ref(), some_bindings).and(comparison(c.body.as_ref()))
        }
        _ => None,
    };
    let by_name = COMPARE_METHODS
        .contains(&method.as_str())
        .then_some(())
        .filter(|_| m.args.len() == 1);
    let ordering = match m.receiver.as_ref() {
        Expr::MethodCall(r) => ORDERING_TESTS
            .contains(&method.as_str())
            .then_some(())
            .filter(|_| r.method == "cmp" || r.method == "partial_cmp"),
        _ => None,
    };
    let lift_eq = by_name
        .or(ordering)
        .filter(|_| matches!(cx.in_predicate, Within::Out));
    let for_each = match method.as_str() {
        "for_each" | "try_for_each" => Some(()),
        _ => None,
    };
    let kept = m
        .turbofish
        .as_ref()
        .filter(|t| t.args.iter().find_map(unit_arg).is_none());
    let collected = match (method.as_str(), kept) {
        ("try_collect", None) => stream_root(m.receiver.as_ref(), streams),
        _ => None,
    };
    let mapped = match (method.as_str(), m.args.first(), m.receiver.as_ref()) {
        ("map", Some(Expr::Closure(c)), Expr::Await(a)) => match (c.body.as_ref(), a.base.as_ref())
        {
            (Expr::Tuple(t), Expr::MethodCall(inner)) => {
                match (t.elems.first(), inner.method.to_string().as_str()) {
                    (None, "try_collect") => stream_root(inner.receiver.as_ref(), streams),
                    _ => None,
                }
            }
            _ => None,
        },
        _ => None,
    };
    let split = match method.as_str() {
        "split_at" | "split_at_mut" => Some(()),
        _ => None,
    };
    let rposition = match method.as_str() {
        "rposition" => Some(()),
        _ => None,
    };
    let keyed = predicate_method(&method, env).or(rposition).and_then(|_| {
        let bindings = cx
            .func
            .as_ref()
            .map(|f| f.table_bindings.clone())
            .unwrap_or_default();
        let scan = env.tables.scanned(m.receiver.as_ref(), &bindings)?;
        match m.args.first() {
            Some(Expr::Closure(c)) => keyed(c, &scan),
            _ => None,
        }
    });
    join(Vec::from([
        when(ord, flag(at, Rule::OrdInner)),
        when(count, flag(at, Rule::Count)),
        when(lift, flag(at, Rule::LiftFilter)),
        when(lift_eq, flag(at, Rule::LiftEq)),
        when(for_each, flag(at, Rule::ForEach)),
        when(collected, flag(at, Rule::StreamEffects)),
        when(mapped, flag(at, Rule::StreamEffects)),
        when(split, flag(at, Rule::SplitAt)),
        when(keyed, flag(at, Rule::KeyedTable)),
    ]))
}

/// A turbofish argument that names `()`.
fn unit_arg(arg: &syn::GenericArgument) -> Option<()> {
    match arg {
        syn::GenericArgument::Type(t) => syntax::unit_in(t).and(Some(())),
        _ => None,
    }
}

/// One statement of a `for` body (proposal 0041): a call, optionally
/// `.await` and `?`, or a call discarded with `let _ =` (0025). An `.await`
/// is always followed by `?`, and the call builds no value on the way.
fn effect_stmt(stmt: &Stmt) -> Option<()> {
    match stmt {
        Stmt::Expr(e, Some(_)) => effect_call(e),
        Stmt::Local(local) => match (&local.pat, &local.init) {
            (Pat::Wild(_), Some(init)) => match init.diverge {
                None => effect_call(init.expr.as_ref()),
                Some(_) => None,
            },
            _ => None,
        },
        _ => None,
    }
}

fn effect_call(expr: &Expr) -> Option<()> {
    let call = match expr {
        Expr::Try(t) => match t.expr.as_ref() {
            Expr::Await(a) => a.base.as_ref(),
            other => other,
        },
        other => other,
    };
    match call {
        Expr::Call(_) | Expr::MethodCall(_) => match building(call, Building::Call, Within::Out) {
            Some(_) => None,
            None => Some(()),
        },
        _ => None,
    }
}

#[derive(Clone, Copy)]
enum Building {
    /// An effect call: no `.await` inside it either, so one call is one step.
    Call,
    /// A `for`'s iterator expression.
    Iterator,
}

/// A block with statements, assignment, `let`, `if`, `match` or loop in
/// `expr`, outside the bodies of the closures it passes to adapters, and no
/// assignment even there.
fn building(expr: &Expr, kind: Building, closure: Within) -> Option<()> {
    let own = match (expr, closure, kind) {
        (Expr::Assign(_) | Expr::Let(_), _, _) => Some(()),
        (
            Expr::Block(_)
            | Expr::If(_)
            | Expr::Match(_)
            | Expr::Loop(_)
            | Expr::While(_)
            | Expr::ForLoop(_)
            | Expr::Async(_),
            Within::Out,
            _,
        ) => Some(()),
        (Expr::Await(_), _, Building::Call) => Some(()),
        _ => None,
    };
    let inner = match expr {
        Expr::Closure(_) => Within::In,
        _ => closure,
    };
    own.or_else(|| {
        syntax::inner_exprs(expr)
            .into_iter()
            .find_map(|e| building(e, kind, inner))
    })
}

pub(crate) fn for_loop(f: &syn::ExprForLoop) -> Vec<Hit> {
    let body = f
        .body
        .stmts
        .first()
        .and(f.body.stmts.iter().try_fold((), |_, s| effect_stmt(s)));
    join(Vec::from([
        unless(body, flag(f.for_token.span, Rule::ForBody)),
        when(
            building(f.expr.as_ref(), Building::Iterator, Within::Out),
            flag(f.for_token.span, Rule::ForIter),
        ),
    ]))
}

/// The names a pattern binds.
fn pattern_bindings(pat: &Pat) -> BTreeSet<String> {
    syntax::idents(pat)
        .into_iter()
        .filter(|p| binds(&p.ident).is_some())
        .map(|p| p.ident.to_string())
        .collect()
}

/// A plain lowercase name binds any value; `None` or a `CONST` names one.
fn binds(ident: &syn::Ident) -> Option<()> {
    ident
        .to_string()
        .chars()
        .next()
        .filter(|c| c.is_lowercase() || c == &'_')
        .and(Some(()))
}

/// Whether `expr` is a place or call chain whose root is one of `names`.
fn rooted_in(expr: &Expr, names: &BTreeSet<String>) -> Option<()> {
    match expr {
        Expr::Path(_) => syntax::ident_of(expr)
            .filter(|n| names.contains(n))
            .and(Some(())),
        Expr::Field(e) => rooted_in(e.base.as_ref(), names),
        Expr::MethodCall(e) => rooted_in(e.receiver.as_ref(), names),
        Expr::Reference(e) => rooted_in(e.expr.as_ref(), names),
        Expr::Paren(e) => rooted_in(e.expr.as_ref(), names),
        Expr::Unary(e) => match e.op {
            UnOp::Deref(_) => rooted_in(e.expr.as_ref(), names),
            _ => None,
        },
        _ => None,
    }
}

/// A value a pattern can name: a literal, a unit variant or constant, or a
/// constructor of such values.
fn constant(expr: &Expr) -> Option<()> {
    match expr {
        Expr::Lit(_) => Some(()),
        Expr::Path(p) => p
            .path
            .segments
            .last()
            .filter(|s| s.ident.to_string().starts_with(char::is_uppercase))
            .and(Some(())),
        Expr::Reference(e) => constant(e.expr.as_ref()),
        Expr::Paren(e) => constant(e.expr.as_ref()),
        Expr::Call(c) => {
            constant(c.func.as_ref()).and(c.args.iter().try_fold((), |_, e| constant(e)))
        }
        Expr::Tuple(t) => t
            .elems
            .first()
            .and(t.elems.iter().try_fold((), |_, e| constant(e))),
        Expr::Struct(s) => match s.rest {
            None => s.fields.iter().try_fold((), |_, f| constant(&f.expr)),
            Some(_) => None,
        },
        _ => None,
    }
}

/// Whether a call happens in `expr`: what a pattern can't express.
fn calls(expr: &Expr) -> Option<()> {
    match expr {
        Expr::Call(_) | Expr::MethodCall(_) => Some(()),
        Expr::Field(e) => calls(e.base.as_ref()),
        Expr::Reference(e) => calls(e.expr.as_ref()),
        Expr::Paren(e) => calls(e.expr.as_ref()),
        _ => None,
    }
}

/// Whether every value `specific` matches, `general` matches too, judged by
/// the patterns' structure (proposal 0040).
fn covers(general: &Pat, specific: &Pat) -> Option<()> {
    match (general, specific) {
        (Pat::Wild(_), _) => Some(()),
        (Pat::Paren(g), s) => covers(g.pat.as_ref(), s),
        (g, Pat::Paren(s)) => covers(g, s.pat.as_ref()),
        _ => covers_ident(general, specific),
    }
}

fn subpattern(pat: &Pat) -> Option<&Pat> {
    match pat {
        Pat::Ident(p) => p.subpat.as_ref().map(|(_, s)| s.as_ref()),
        _ => None,
    }
}

fn covers_ident(general: &Pat, specific: &Pat) -> Option<()> {
    let binding = match general {
        Pat::Ident(g) => match g.subpat {
            None => binds(&g.ident),
            Some(_) => None,
        },
        _ => None,
    };
    match (binding, subpattern(general), subpattern(specific)) {
        (Some(_), _, _) => Some(()),
        (None, Some(inner), _) => covers(inner, specific),
        (None, None, Some(inner)) => covers(general, inner),
        (None, None, None) => covers_rest(general, specific),
    }
}

fn covers_rest(general: &Pat, specific: &Pat) -> Option<()> {
    match (general, specific) {
        (g, Pat::Or(s)) => s.cases.iter().try_fold((), |_, case| covers(g, case)),
        (Pat::Or(g), s) => g.cases.iter().find_map(|case| covers(case, s)),
        (Pat::Reference(g), Pat::Reference(s)) => covers(g.pat.as_ref(), s.pat.as_ref()),
        (Pat::Tuple(g), Pat::Tuple(s)) => all_cover(&g.elems, &s.elems),
        (Pat::TupleStruct(g), Pat::TupleStruct(s)) => {
            same_path(&g.path, &s.path).and(all_cover(&g.elems, &s.elems))
        }
        (Pat::Struct(g), Pat::Struct(s)) => {
            same_path(&g.path, &s.path).and(g.fields.iter().try_fold((), |_, field| {
                match s.fields.iter().find(|f| f.member == field.member) {
                    Some(theirs) => covers(field.pat.as_ref(), theirs.pat.as_ref()),
                    None => wild_covered(field.pat.as_ref()),
                }
            }))
        }
        (g, s) => same(g, s),
    }
}

/// Whether `general` covers `_`.
fn wild_covered(general: &Pat) -> Option<()> {
    let wild = Pat::Wild(syn::PatWild {
        attrs: Vec::new(),
        underscore_token: syn::token::Underscore(Span::call_site()),
    });
    covers(general, &wild)
}

fn same(general: &Pat, specific: &Pat) -> Option<()> {
    match general {
        g if g == specific => Some(()),
        _ => None,
    }
}

fn same_path(a: &syn::Path, b: &syn::Path) -> Option<()> {
    match syntax::segments(a) {
        names if names == syntax::segments(b) => Some(()),
        _ => None,
    }
}

fn all_cover(
    general: &Punctuated<Pat, syn::token::Comma>,
    specific: &Punctuated<Pat, syn::token::Comma>,
) -> Option<()> {
    let rest = general
        .iter()
        .chain(specific)
        .find(|p| matches!(p, Pat::Rest(_)));
    match rest {
        Some(_) => None,
        None => match general.len() {
            n if n == specific.len() => general
                .iter()
                .zip(specific)
                .try_fold((), |_, (g, s)| covers(g, s)),
            _ => None,
        },
    }
}

/// The guards of a `match` (proposal 0040).
pub(crate) fn guards(m: &syn::ExprMatch) -> Vec<Hit> {
    m.arms
        .iter()
        .enumerate()
        .flat_map(|(i, arm)| match &arm.guard {
            Some((token, guard)) => {
                let at = token.span;
                let bound = pattern_bindings(&arm.pat);
                let shape = match guard.as_ref() {
                    Expr::Binary(e) => match comparison_op(&e.op) {
                        Some(_) => join(Vec::from([
                            unless(
                                rooted_in(e.left.as_ref(), &bound)
                                    .or(rooted_in(e.right.as_ref(), &bound)),
                                flag(at, Rule::GuardBound),
                            ),
                            when(
                                constant(e.left.as_ref()).or(constant(e.right.as_ref())),
                                flag(at, Rule::GuardPattern),
                            ),
                        ])),
                        None => Vec::from([flag(at, Rule::Guard)]),
                    },
                    Expr::Macro(e) => {
                        let named = Some(&e.mac).filter(|m| m.path.is_ident("matches"));
                        match named.map(parse_matches) {
                            Some(Some(parsed)) => join(Vec::from([
                                unless(
                                    rooted_in(&parsed.scrutinee, &bound),
                                    flag(at, Rule::GuardBound),
                                ),
                                unless(calls(&parsed.scrutinee), flag(at, Rule::GuardPattern)),
                            ])),
                            Some(None) => Vec::new(),
                            None => Vec::from([flag(at, Rule::Guard)]),
                        }
                    }
                    _ => Vec::from([flag(at, Rule::Guard)]),
                };
                let fallback =
                    m.arms.iter().skip(i.saturating_add(1)).find(|later| {
                        later.guard.is_none() && covers(&later.pat, &arm.pat).is_some()
                    });
                join(Vec::from([
                    shape,
                    unless(fallback.and(Some(())), flag(at, Rule::GuardFallback)),
                ]))
            }
            None => Vec::new(),
        })
        .collect()
}

/// A table that an iterator chain reads, and whether `enumerate` pairs each
/// row with its index.
pub(crate) struct Scan {
    pub(crate) keys: Element,
    pub(crate) enumerated: Within,
}

/// Whether a predicate closure over `scan`'s rows tests a string key for
/// equality with a value that isn't a literal, in any spelling: `==`,
/// `!(.. != ..)`, `.eq`, `PartialEq::eq`, `cmp(..).is_eq()`, inside `&&`,
/// `||` or a block.
fn keyed(closure: &syn::ExprClosure, scan: &Scan) -> Option<()> {
    let pat = unref(closure.inputs.first()?);
    let row = match (scan.enumerated, pat) {
        (Within::Out, pat) => Some(pat),
        (Within::In, Pat::Tuple(t)) => match t.elems.len() {
            2 => t.elems.last(),
            _ => None,
        },
        (Within::In, _) => None,
    }?;
    compares(
        &binding(row, &scan.keys),
        closure.body.as_ref(),
        Within::Out,
        &scan.keys,
    )
}

/// How a predicate closure binds a row.
enum Binding {
    /// The whole row, by name.
    Whole(String),
    /// The row's string keys, by the names a pattern binds them to.
    Parts(BTreeSet<String>),
}

fn binding(pat: &Pat, keys: &Element) -> Binding {
    let fields = match keys {
        Element::Tuple(fields) | Element::Fields(fields) => fields.clone(),
        _ => BTreeSet::new(),
    };
    match unref(pat) {
        Pat::Ident(p) => Binding::Whole(p.ident.to_string()),
        Pat::Tuple(t) => Binding::Parts(
            t.elems
                .iter()
                .enumerate()
                .filter(|(i, _)| fields.contains(&i.to_string()))
                .filter_map(|(_, p)| pat_name(p))
                .collect(),
        ),
        Pat::Struct(s) => Binding::Parts(
            s.fields
                .iter()
                .filter(|f| {
                    member_name(&f.member)
                        .filter(|n| fields.contains(n))
                        .is_some()
                })
                .filter_map(|f| pat_name(f.pat.as_ref()))
                .collect(),
        ),
        _ => Binding::Parts(BTreeSet::new()),
    }
}

/// Whether `expr`, read as a predicate (negated when `negated`), holds an
/// equality between a key and a value that isn't a literal.
fn compares(b: &Binding, expr: &Expr, negated: Within, keys: &Element) -> Option<()> {
    let eq = match negated {
        Within::Out => "eq",
        Within::In => "ne",
    };
    let flipped = match negated {
        Within::Out => Within::In,
        Within::In => Within::Out,
    };
    match expr {
        Expr::Paren(e) => compares(b, e.expr.as_ref(), negated, keys),
        Expr::Block(e) => match (e.block.stmts.first(), e.block.stmts.len()) {
            (Some(Stmt::Expr(tail, None)), 1) => compares(b, tail, negated, keys),
            _ => None,
        },
        Expr::Unary(e) => match e.op {
            UnOp::Not(_) => compares(b, e.expr.as_ref(), flipped, keys),
            _ => None,
        },
        Expr::Binary(e) => match (e.op, negated) {
            (BinOp::And(_) | BinOp::Or(_), _) => compares(b, e.left.as_ref(), negated, keys)
                .or_else(|| compares(b, e.right.as_ref(), negated, keys)),
            (BinOp::Eq(_), Within::Out) | (BinOp::Ne(_), Within::In) => {
                equal(b, e.left.as_ref(), e.right.as_ref(), keys)
            }
            _ => None,
        },
        Expr::MethodCall(e) => {
            let method = e.method.to_string();
            let test = format!("is_{eq}");
            let named = match method.as_str() {
                m if m == eq => e
                    .args
                    .first()
                    .filter(|_| e.args.len() == 1)
                    .and_then(|arg| equal(b, e.receiver.as_ref(), arg, keys)),
                _ => None,
            };
            let ordering = match method.as_str() {
                m if m == test => Some(()),
                _ => None,
            }
            .and(match e.receiver.as_ref() {
                Expr::MethodCall(c) => Some(c)
                    .filter(|c| c.method == "cmp" || c.method == "partial_cmp")
                    .and_then(|c| c.args.first().filter(|_| c.args.len() == 1).map(|a| (c, a))),
                _ => None,
            })
            .and_then(|(c, arg)| equal(b, c.receiver.as_ref(), arg, keys));
            named.or(ordering)
        }
        Expr::Call(e) => {
            let named = match e.func.as_ref() {
                Expr::Path(p) => match syntax::last_name(&p.path).as_str() {
                    name if name == eq => Some(()),
                    _ => None,
                },
                _ => None,
            };
            match (named, e.args.first(), e.args.last(), e.args.len()) {
                (Some(_), Some(a), Some(z), 2) => equal(b, a, z, keys),
                _ => None,
            }
        }
        _ => None,
    }
}

fn equal(b: &Binding, x: &Expr, y: &Expr, keys: &Element) -> Option<()> {
    let one = key(b, x, keys).and(match literal(y) {
        Some(_) => None,
        None => Some(()),
    });
    let other = key(b, y, keys).and(match literal(x) {
        Some(_) => None,
        None => Some(()),
    });
    one.or(other)
}

fn key(b: &Binding, expr: &Expr, keys: &Element) -> Option<()> {
    match (b, expr, keys) {
        (_, Expr::Paren(e), _) => key(b, e.expr.as_ref(), keys),
        (_, Expr::Reference(e), _) => key(b, e.expr.as_ref(), keys),
        (_, Expr::Unary(e), _) => match e.op {
            UnOp::Deref(_) => key(b, e.expr.as_ref(), keys),
            _ => None,
        },
        (Binding::Whole(name), Expr::Path(p), Element::Str) => p.path.is_ident(name).then_some(()),
        (
            Binding::Whole(name),
            Expr::Field(f),
            Element::Tuple(fields) | Element::Fields(fields),
        ) => {
            let field = match &f.member {
                syn::Member::Named(i) => i.to_string(),
                syn::Member::Unnamed(i) => i.index.to_string(),
            };
            fields
                .contains(&field)
                .then_some(())
                .and(key_base(f.base.as_ref(), name))
        }
        (Binding::Parts(names), Expr::Path(_), _) => syntax::ident_of(expr)
            .filter(|n| names.contains(n))
            .and(Some(())),
        _ => None,
    }
}

fn key_base(base: &Expr, name: &str) -> Option<()> {
    match base {
        Expr::Paren(e) => key_base(e.expr.as_ref(), name),
        Expr::Unary(e) => match e.op {
            UnOp::Deref(_) => key_base(e.expr.as_ref(), name),
            _ => None,
        },
        Expr::Path(p) => p.path.is_ident(name).then_some(()),
        _ => None,
    }
}

fn unref(pat: &Pat) -> &Pat {
    match pat {
        Pat::Reference(p) => unref(p.pat.as_ref()),
        Pat::Type(p) => unref(p.pat.as_ref()),
        _ => pat,
    }
}

fn pat_name(pat: &Pat) -> Option<String> {
    match unref(pat) {
        Pat::Ident(p) => Some(p.ident.to_string()),
        _ => None,
    }
}

fn literal(expr: &Expr) -> Option<()> {
    match expr {
        Expr::Paren(e) => literal(e.expr.as_ref()),
        Expr::Reference(e) => literal(e.expr.as_ref()),
        Expr::Lit(_) => Some(()),
        _ => None,
    }
}

/// The shape rules of one expression that read only the node and its slot.
pub(crate) fn expr(e: &Expr, cx: &Cx, env: &Env) -> Vec<Hit> {
    match e {
        Expr::Array(a) => {
            let table = a
                .elems
                .iter()
                .find(|row| matches!(row, Expr::Tuple(_)) && leaves(row) > 2)
                .filter(|_| a.elems.len() >= 2)
                .and(Some(()));
            let placed = match cx.slot {
                Slot::CallArg(_) | Slot::MethodArg { .. } | Slot::ConstInit => Some(()),
                _ => None,
            };
            when(table, flag(a.span(), Rule::TupleTable))
                .into_iter()
                .chain(unless(placed, flag(a.span(), Rule::ArrayValue)))
                .collect()
        }
        Expr::Assign(a) => assign(a, cx),
        Expr::Async(a) => unless(
            async_placed(&cx.slot),
            flag(a.async_token.span, Rule::AsyncBlock),
        ),
        Expr::Binary(b) => match cx.predicate {
            Within::In => Vec::new(),
            Within::Out => predicate_rule(&b.op)
                .map(|r| flag(b.op.span(), r))
                .into_iter()
                .collect(),
        },
        Expr::Break(b) => {
            let valued = b
                .expr
                .as_ref()
                .filter(|_| matches!(cx.lp, crate::walk::Loop::Actor));
            unless(valued.and(Some(())), flag(b.break_token.span, Rule::Break))
        }
        Expr::Lit(l) => match (&l.lit, library_value(&cx.slot), &cx.slot) {
            (
                syn::Lit::Bool(_),
                None,
                Slot::FieldValue {
                    wire: Within::In, ..
                },
            ) => Vec::new(),
            (syn::Lit::Bool(_), None, _) => Vec::from([flag(l.span(), Rule::BoolLit)]),
            _ => Vec::new(),
        },
        Expr::Loop(l) => {
            let actor = cx
                .func
                .as_ref()
                .and_then(|f| f.actor_loop)
                .filter(|extent| extent == &syntax::extent(e.span()));
            let rule = match (actor, turn(&l.body)) {
                (None, _) => Some(Rule::LoopPosition),
                (Some(_), None) => Some(Rule::LoopTurn),
                (Some(_), Some(_)) => None,
            };
            rule.map(|r| flag(l.loop_token.span, r))
                .into_iter()
                .collect()
        }
        Expr::Match(m) => when(
            filtered_once(&m.expr),
            flag(m.match_token.span, Rule::LiftMatched),
        ),
        Expr::Path(p) => path_expr(p, cx, env),
        Expr::Repeat(r) => {
            let literal = match r.expr.as_ref() {
                Expr::Lit(_) => Some(()),
                _ => None,
            };
            unless(literal, flag(r.span(), Rule::Repeat))
        }
        Expr::Tuple(t) => match (t.elems.first(), library_value(&cx.slot)) {
            (None, None) => Vec::from([flag(t.span(), Rule::UnitValue)]),
            _ => Vec::new(),
        },
        Expr::Unary(u) => match (u.op, cx.predicate) {
            (UnOp::Not(_), Within::Out) => Vec::from([flag(u.op.span(), Rule::PredicateNot)]),
            _ => Vec::new(),
        },
        Expr::ForLoop(f) => for_loop(f),
        _ => Vec::new(),
    }
}

/// A let-chain: a `let` joined by `&&`.
pub(crate) fn has_let(cond: &Expr) -> Option<&Expr> {
    match cond {
        Expr::Let(_) => Some(cond),
        Expr::Binary(b) => match b.op {
            BinOp::And(_) => has_let(&b.left).or(has_let(&b.right)),
            _ => None,
        },
        _ => None,
    }
}

/// `name @ Variant { .. }`: a binding that keeps the whole value a variant
/// pattern matched (proposal 0044).
pub(crate) fn ident_at(p: &syn::PatIdent) -> Vec<Hit> {
    match &p.subpat {
        Some((at, sub)) => {
            let variant = match sub.as_ref() {
                Pat::Struct(_) | Pat::TupleStruct(_) | Pat::Path(_) => Some(()),
                _ => None,
            };
            let plain = match (p.by_ref, p.mutability) {
                (None, None) => variant,
                _ => None,
            };
            unless(plain, flag(at.span, Rule::IdentAt))
        }
        None => Vec::new(),
    }
}

/// The name of a named field.
fn member_name(member: &syn::Member) -> Option<String> {
    match member {
        syn::Member::Named(name) => Some(name.to_string()),
        syn::Member::Unnamed(_) => None,
    }
}
