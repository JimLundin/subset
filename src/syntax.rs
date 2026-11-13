//! Small readings of `syn` nodes that several parts of the walk share.

use proc_macro2::{Span, TokenTree};
use syn::punctuated::Punctuated;
use syn::{Attribute, Block, Expr, Local, Meta, Pat, Stmt, Type, TypeParamBound};

/// A position in a file: the 1-based line and column, and the byte offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Pos {
    pub(crate) line: usize,
    pub(crate) col: usize,
    pub(crate) byte: usize,
}

/// The bytes a node covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Extent {
    pub(crate) start: usize,
    pub(crate) end: usize,
}

impl Extent {
    pub(crate) fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// The bytes from the start of `self` to the end of `last`.
    pub(crate) fn to(&self, last: Extent) -> Extent {
        Extent {
            start: self.start,
            end: last.end,
        }
    }

    /// `inner`, when it lies within `self`.
    pub(crate) fn holding(&self, inner: Extent) -> Option<Extent> {
        Some(inner).filter(|i| self.start <= i.start && i.end <= self.end)
    }
}

pub(crate) fn pos(span: Span) -> Pos {
    let start = span.start();
    Pos {
        line: start.line,
        col: start.column.saturating_add(1),
        byte: span.byte_range().start,
    }
}

pub(crate) fn extent(span: Span) -> Extent {
    let bytes = span.byte_range();
    Extent {
        start: bytes.start,
        end: bytes.end,
    }
}

pub(crate) fn segments(path: &syn::Path) -> Vec<String> {
    path.segments.iter().map(|s| s.ident.to_string()).collect()
}

pub(crate) fn path_string(path: &syn::Path) -> String {
    segments(path).join("::")
}

pub(crate) fn last_name(path: &syn::Path) -> String {
    path.segments
        .last()
        .map(|s| s.ident.to_string())
        .unwrap_or_default()
}

/// The last path segment of a type, through references and parentheses.
pub(crate) fn type_name(ty: &Type) -> Option<String> {
    match ty {
        Type::Path(t) => t.path.segments.last().map(|s| s.ident.to_string()),
        Type::Reference(t) => type_name(t.elem.as_ref()),
        Type::Paren(t) => type_name(t.elem.as_ref()),
        _ => None,
    }
}

/// Every path segment a type names, generic arguments included.
pub(crate) fn type_segments(ty: &Type) -> Vec<String> {
    match ty {
        Type::Path(t) => t
            .qself
            .iter()
            .flat_map(|q| type_segments(q.ty.as_ref()))
            .chain(path_segments_deep(&t.path))
            .collect(),
        Type::TraitObject(t) => bounds_segments(&t.bounds),
        Type::ImplTrait(t) => bounds_segments(&t.bounds),
        _ => inner_types(ty)
            .into_iter()
            .flat_map(type_segments)
            .collect(),
    }
}

fn bounds_segments(bounds: &Punctuated<TypeParamBound, syn::token::Plus>) -> Vec<String> {
    bounds
        .iter()
        .flat_map(|b| match b {
            TypeParamBound::Trait(t) => path_segments_deep(&t.path),
            _ => Vec::new(),
        })
        .collect()
}

fn path_segments_deep(path: &syn::Path) -> Vec<String> {
    path.segments
        .iter()
        .flat_map(|s| {
            std::iter::once(s.ident.to_string()).chain(
                argument_types(&s.arguments)
                    .into_iter()
                    .flat_map(type_segments),
            )
        })
        .collect()
}

/// The types a path segment's arguments hold: `<A, B>`, `Item = C`, `(D) -> E`.
pub(crate) fn argument_types(args: &syn::PathArguments) -> Vec<&Type> {
    match args {
        syn::PathArguments::AngleBracketed(a) => a
            .args
            .iter()
            .filter_map(|arg| match arg {
                syn::GenericArgument::Type(t) => Some(t),
                syn::GenericArgument::AssocType(t) => Some(&t.ty),
                _ => None,
            })
            .collect(),
        syn::PathArguments::Parenthesized(p) => {
            p.inputs.iter().chain(returned(&p.output)).collect()
        }
        syn::PathArguments::None => Vec::new(),
    }
}

/// The type a fn signature or `Fn(..)` bound returns, when it names one.
pub(crate) fn returned(output: &syn::ReturnType) -> Option<&Type> {
    match output {
        syn::ReturnType::Type(_, t) => Some(t.as_ref()),
        syn::ReturnType::Default => None,
    }
}

/// The types directly inside a type that isn't a path or a bound list.
pub(crate) fn inner_types(ty: &Type) -> Vec<&Type> {
    match ty {
        Type::Array(t) => Vec::from([t.elem.as_ref()]),
        Type::Group(t) => Vec::from([t.elem.as_ref()]),
        Type::Paren(t) => Vec::from([t.elem.as_ref()]),
        Type::Ptr(t) => Vec::from([t.elem.as_ref()]),
        Type::Reference(t) => Vec::from([t.elem.as_ref()]),
        Type::Slice(t) => Vec::from([t.elem.as_ref()]),
        Type::Tuple(t) => t.elems.iter().collect(),
        Type::BareFn(t) => t
            .inputs
            .iter()
            .map(|a| &a.ty)
            .chain(returned(&t.output))
            .collect(),
        _ => Vec::new(),
    }
}

/// `()` anywhere in a type, as in `Vec<()>`.
pub(crate) fn unit_in(ty: &Type) -> Option<&Type> {
    match ty {
        Type::Tuple(t) => match t.elems.first() {
            None => Some(ty),
            Some(_) => t.elems.iter().find_map(unit_in),
        },
        Type::Path(t) => t
            .path
            .segments
            .iter()
            .flat_map(|s| argument_types(&s.arguments))
            .find_map(unit_in),
        _ => inner_types(ty).into_iter().find_map(unit_in),
    }
}

pub(crate) fn unparen(expr: &Expr) -> &Expr {
    match expr {
        Expr::Paren(e) => unparen(e.expr.as_ref()),
        _ => expr,
    }
}

/// The expression under any trailing `?` and `.await`.
pub(crate) fn strip(expr: &Expr) -> &Expr {
    match expr {
        Expr::Try(e) => strip(e.expr.as_ref()),
        Expr::Await(e) => strip(e.base.as_ref()),
        other => other,
    }
}

/// The attribute's arguments with all whitespace removed.
pub(crate) fn attr_tokens(attr: &Attribute) -> String {
    match &attr.meta {
        Meta::List(list) => list.tokens.to_string().split_whitespace().collect(),
        _ => String::new(),
    }
}

/// The paths a `#[derive(..)]` lists; none for any other attribute.
pub(crate) fn derived(attr: &Attribute) -> Vec<syn::Path> {
    attr.path()
        .is_ident("derive")
        .then(|| attr.parse_args_with(Punctuated::<syn::Path, syn::token::Comma>::parse_terminated))
        .and_then(Result::ok)
        .map(|names| names.into_iter().collect())
        .unwrap_or_default()
}

/// The `#` of the first derive among `attrs` whose path, as written, is one
/// of `names`.
pub(crate) fn derive_of(attrs: &[Attribute], names: &[&str]) -> Option<Span> {
    attrs
        .iter()
        .find(|a| {
            derived(a)
                .iter()
                .any(|p| names.contains(&path_string(p).as_str()))
        })
        .map(|a| a.pound_token.span)
}

/// The `#` of the first derive among `attrs` that names one of `names` in
/// its last segment.
pub(crate) fn derive_named(attrs: &[Attribute], names: &[&str]) -> Option<Span> {
    attrs
        .iter()
        .find(|a| {
            derived(a)
                .iter()
                .any(|p| names.contains(&last_name(p).as_str()))
        })
        .map(|a| a.pound_token.span)
}

/// `#[cfg(test)]` exactly.
pub(crate) fn cfg_test(attr: &Attribute) -> Option<&Attribute> {
    Some(attr).filter(|a| a.path().is_ident("cfg") && attr_tokens(a) == "test")
}

/// `#[test]`, `#[tokio::test]` or `#[cfg(test)]`.
pub(crate) fn test_attr(attrs: &[Attribute]) -> Option<&Attribute> {
    attrs.iter().find(|a| {
        a.path().is_ident("test") || path_string(a.path()) == "tokio::test" || cfg_test(a).is_some()
    })
}

/// The leading identifier of each comma-separated argument of an attribute.
pub(crate) fn meta_keys(attr: &Attribute) -> Vec<String> {
    let tokens = match &attr.meta {
        Meta::List(list) => list.tokens.clone().into_iter().collect(),
        _ => Vec::new(),
    };
    tokens
        .iter()
        .fold((Vec::new(), Expecting::Key), |(keys, state), tree| {
            match (tree, state) {
                (TokenTree::Ident(ident), Expecting::Key) => {
                    let keys = keys.into_iter().chain(Some(ident.to_string())).collect();
                    (keys, Expecting::Rest)
                }
                (TokenTree::Punct(p), _) => match p.as_char() {
                    ',' => (keys, Expecting::Key),
                    _ => (keys, Expecting::Rest),
                },
                _ => (keys, Expecting::Rest),
            }
        })
        .0
}

#[derive(Clone, Copy)]
enum Expecting {
    Key,
    Rest,
}

/// The identifier patterns in a pattern, outermost first.
pub(crate) fn idents(pat: &Pat) -> Vec<&syn::PatIdent> {
    let inner = match pat {
        Pat::Ident(p) => p.subpat.iter().map(|(_, sub)| sub.as_ref()).collect(),
        Pat::Or(p) => p.cases.iter().collect(),
        Pat::Paren(p) => Vec::from([p.pat.as_ref()]),
        Pat::Reference(p) => Vec::from([p.pat.as_ref()]),
        Pat::Slice(p) => p.elems.iter().collect(),
        Pat::Struct(p) => p.fields.iter().map(|f| f.pat.as_ref()).collect(),
        Pat::Tuple(p) => p.elems.iter().collect(),
        Pat::TupleStruct(p) => p.elems.iter().collect(),
        Pat::Type(p) => Vec::from([p.pat.as_ref()]),
        _ => Vec::new(),
    };
    let own = match pat {
        Pat::Ident(p) => Some(p),
        _ => None,
    };
    own.into_iter()
        .chain(inner.into_iter().flat_map(idents))
        .collect()
}

/// The names a pattern binds with `mut`.
pub(crate) fn mut_bindings(pat: &Pat) -> Vec<String> {
    idents(pat)
        .into_iter()
        .filter(|p| p.mutability.is_some())
        .map(|p| p.ident.to_string())
        .collect()
}

/// The expressions directly inside `expr`, except those in its blocks.
pub(crate) fn child_exprs(expr: &Expr) -> Vec<&Expr> {
    match expr {
        Expr::Array(e) => e.elems.iter().collect(),
        Expr::Assign(e) => Vec::from([e.left.as_ref(), e.right.as_ref()]),
        Expr::Await(e) => Vec::from([e.base.as_ref()]),
        Expr::Binary(e) => Vec::from([e.left.as_ref(), e.right.as_ref()]),
        Expr::Break(e) => e.expr.iter().map(AsRef::as_ref).collect(),
        Expr::Call(e) => std::iter::once(e.func.as_ref()).chain(&e.args).collect(),
        Expr::Cast(e) => Vec::from([e.expr.as_ref()]),
        Expr::Closure(e) => Vec::from([e.body.as_ref()]),
        Expr::Field(e) => Vec::from([e.base.as_ref()]),
        Expr::ForLoop(e) => Vec::from([e.expr.as_ref()]),
        Expr::Group(e) => Vec::from([e.expr.as_ref()]),
        Expr::If(e) => std::iter::once(e.cond.as_ref())
            .chain(e.else_branch.iter().map(|(_, e)| e.as_ref()))
            .collect(),
        Expr::Index(e) => Vec::from([e.expr.as_ref(), e.index.as_ref()]),
        Expr::Let(e) => Vec::from([e.expr.as_ref()]),
        Expr::Match(e) => std::iter::once(e.expr.as_ref())
            .chain(e.arms.iter().flat_map(|a| {
                a.guard
                    .iter()
                    .map(|(_, g)| g.as_ref())
                    .chain(std::iter::once(a.body.as_ref()))
            }))
            .collect(),
        Expr::MethodCall(e) => std::iter::once(e.receiver.as_ref())
            .chain(&e.args)
            .collect(),
        Expr::Paren(e) => Vec::from([e.expr.as_ref()]),
        Expr::Range(e) => e.start.iter().chain(&e.end).map(AsRef::as_ref).collect(),
        Expr::RawAddr(e) => Vec::from([e.expr.as_ref()]),
        Expr::Reference(e) => Vec::from([e.expr.as_ref()]),
        Expr::Repeat(e) => Vec::from([e.expr.as_ref(), e.len.as_ref()]),
        Expr::Return(e) => e.expr.iter().map(AsRef::as_ref).collect(),
        Expr::Struct(e) => e
            .fields
            .iter()
            .map(|f| &f.expr)
            .chain(e.rest.iter().map(AsRef::as_ref))
            .collect(),
        Expr::Try(e) => Vec::from([e.expr.as_ref()]),
        Expr::Tuple(e) => e.elems.iter().collect(),
        Expr::Unary(e) => Vec::from([e.expr.as_ref()]),
        Expr::While(e) => Vec::from([e.cond.as_ref()]),
        Expr::Yield(e) => e.expr.iter().map(AsRef::as_ref).collect(),
        _ => Vec::new(),
    }
}

/// The blocks directly inside `expr`.
pub(crate) fn child_blocks(expr: &Expr) -> Vec<&Block> {
    match expr {
        Expr::Async(e) => Vec::from([&e.block]),
        Expr::Block(e) => Vec::from([&e.block]),
        Expr::Const(e) => Vec::from([&e.block]),
        Expr::ForLoop(e) => Vec::from([&e.body]),
        Expr::If(e) => Vec::from([&e.then_branch]),
        Expr::Loop(e) => Vec::from([&e.body]),
        Expr::TryBlock(e) => Vec::from([&e.block]),
        Expr::Unsafe(e) => Vec::from([&e.block]),
        Expr::While(e) => Vec::from([&e.body]),
        _ => Vec::new(),
    }
}

/// The expressions a statement holds at its top.
pub(crate) fn stmt_exprs(stmt: &Stmt) -> Vec<&Expr> {
    match stmt {
        Stmt::Local(l) => l
            .init
            .iter()
            .flat_map(|i| {
                std::iter::once(i.expr.as_ref()).chain(i.diverge.iter().map(|(_, e)| e.as_ref()))
            })
            .collect(),
        Stmt::Expr(e, _) => Vec::from([e]),
        _ => Vec::new(),
    }
}

/// The expressions directly inside `expr`, its blocks' statements included.
pub(crate) fn inner_exprs(expr: &Expr) -> Vec<&Expr> {
    child_exprs(expr)
        .into_iter()
        .chain(
            child_blocks(expr)
                .into_iter()
                .flat_map(|b| b.stmts.iter().flat_map(stmt_exprs)),
        )
        .collect()
}

/// `expr` and every expression inside it, outside nested items.
pub(crate) fn descendants(expr: &Expr) -> Vec<&Expr> {
    std::iter::once(expr)
        .chain(inner_exprs(expr).into_iter().flat_map(descendants))
        .collect()
}

/// Every expression in a block, outside nested items.
pub(crate) fn block_descendants(block: &Block) -> Vec<&Expr> {
    block
        .stmts
        .iter()
        .flat_map(stmt_exprs)
        .flat_map(descendants)
        .collect()
}

/// Every `let` in a block and the blocks inside it, outer ones first.
pub(crate) fn locals(block: &Block) -> Vec<&Local> {
    block
        .stmts
        .iter()
        .flat_map(|stmt| {
            let own = match stmt {
                Stmt::Local(l) => Some(l),
                _ => None,
            };
            own.into_iter()
                .chain(stmt_exprs(stmt).into_iter().flat_map(expr_locals))
        })
        .collect()
}

fn expr_locals(expr: &Expr) -> Vec<&Local> {
    child_exprs(expr)
        .into_iter()
        .flat_map(expr_locals)
        .chain(child_blocks(expr).into_iter().flat_map(locals))
        .collect()
}

/// A plain name, as `x`.
pub(crate) fn ident_of(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Path(p) => p.path.get_ident().map(ToString::to_string),
        _ => None,
    }
}
