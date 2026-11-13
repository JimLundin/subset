/// The shape rules on one type node.
pub(crate) fn ty(t: &Type, cx: &Cx, env: &Env) -> Vec<Hit> {
    let marks = [fixed_mark(t, &cx.slot), foreign_mut_mark(t, env)];
    type_names(t, env)
        .into_iter()
        .chain(marks.into_iter().flatten())
        .chain(type_rules(t, cx, env))
        .collect()
}

fn type_names(t: &Type, env: &Env) -> Vec<Hit> {
    match t {
        Type::Path(p) if p.qself.is_none() => {
            named(&syntax::segments(&p.path), Named::Type, p.span(), env)
        }
        _ => Vec::new(),
    }
}

/// A type whose shape a library or `main` fixes.
fn fixed_mark(t: &Type, slot: &Slot) -> Option<Hit> {
    if library_type_arg(slot).is_some() || matches!(slot, Slot::ClapField | Slot::MainReturn) {
        Some(mark(t.span(), Fixed::IfRestricted))
    } else {
        None
    }
}

/// A `&mut` of a type we don't define is always the boundary.
fn foreign_mut_mark(t: &Type, env: &Env) -> Option<Hit> {
    if is_foreign_mut(t, env) {
        Some(mark(t.span(), Fixed::Always))
    } else {
        None
    }
}

fn is_foreign_mut(t: &Type, env: &Env) -> bool {
    matches!(t, Type::Reference(r) if r.mutability.is_some() && !is_our_path(r.elem.as_ref(), env))
}

fn is_our_path(t: &Type, env: &Env) -> bool {
    matches!(t, Type::Path(p) if matches!(crate::walk::ours(&p.path, env), Owner::Ours))
}

/// Each type kind's own rule.
fn type_rules(t: &Type, cx: &Cx, env: &Env) -> Vec<Hit> {
    match t {
        Type::Tuple(tu) => unit_type(tu, &cx.slot),
        Type::Reference(r) => reference_type(t, r, &cx.slot, env),
        Type::Slice(s) => slice_type(s, cx),
        Type::Array(a) => array_type(a, cx),
        Type::ImplTrait(i) => impl_trait_type(i, cx),
        Type::TraitObject(o) => dyn_type(o, &cx.slot),
        Type::Path(p) => bool_type(p, &cx.slot),
        _ => Vec::new(),
    }
}

fn flagged(at: Span, rule: Rule) -> Vec<Hit> {
    Vec::from([flag(at, rule)])
}

fn unit_type(tu: &syn::TypeTuple, slot: &Slot) -> Vec<Hit> {
    if tu.elems.is_empty() && !is_unit_slot(slot) {
        flagged(tu.span(), Rule::UnitType)
    } else {
        Vec::new()
    }
}

fn is_unit_slot(slot: &Slot) -> bool {
    library_type_arg(slot).is_some() || matches!(slot, Slot::MainReturn)
}

fn reference_type(t: &Type, r: &syn::TypeReference, slot: &Slot, env: &Env) -> Vec<Hit> {
    static_str_hit(t, r, slot)
        .into_iter()
        .chain(mut_ours_hit(r, env))
        .collect()
}

/// `&'static str` only as a field of a scanned table's row.
fn static_str_hit(t: &Type, r: &syn::TypeReference, slot: &Slot) -> Option<Hit> {
    if static_str(t).is_some() && !matches!(slot, Slot::RowField) {
        r.lifetime.as_ref().map(|l| flag(l.span(), Rule::StaticStr))
    } else {
        None
    }
}

fn mut_ours_hit(r: &syn::TypeReference, env: &Env) -> Option<Hit> {
    if is_our_path(r.elem.as_ref(), env) {
        r.mutability.map(|token| flag(token.span, Rule::RefMutOurs))
    } else {
        None
    }
}

fn slice_type(s: &syn::TypeSlice, cx: &Cx) -> Vec<Hit> {
    if is_slice_view(cx) {
        Vec::new()
    } else {
        flagged(s.span(), Rule::Slice)
    }
}

/// A slice behind a shared reference: a returned view, or a parameter or
/// constant.
fn is_slice_view(cx: &Cx) -> bool {
    matches!(&cx.slot, Slot::RefTarget(outer) if is_shared_output(cx) || is_input_view(outer, cx))
}

fn is_shared_output(cx: &Cx) -> bool {
    matches!(cx.in_return, Within::In) && matches!(cx.mutable_type, Within::Out)
}

fn is_input_view(outer: &Slot, cx: &Cx) -> bool {
    matches!(cx.in_return, Within::Out) && matches!(outer, Slot::Param | Slot::ConstType)
}

fn array_type(a: &syn::TypeArray, cx: &Cx) -> Vec<Hit> {
    if is_fixed_size(&a.len) && is_array_mutability(a, cx) {
        Vec::new()
    } else {
        flagged(a.span(), Rule::ArrayType)
    }
}

fn is_fixed_size(len: &Expr) -> bool {
    matches!(
        len,
        Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Int(_),
            ..
        }) | Expr::Path(_)
    )
}

/// An immutable array, or a mutable byte buffer or library array.
fn is_array_mutability(a: &syn::TypeArray, cx: &Cx) -> bool {
    matches!(cx.mutable_type, Within::Out) || is_byte(a.elem.as_ref()) || cx.library().is_some()
}

fn is_byte(t: &Type) -> bool {
    matches!(t, Type::Path(p) if p.path.is_ident("u8"))
}

fn impl_trait_type(i: &syn::TypeImplTrait, cx: &Cx) -> Vec<Hit> {
    if matches!(cx.in_return, Within::In) || matches!(cx.slot, Slot::AskBuild) {
        Vec::new()
    } else {
        flagged(i.impl_token.span, Rule::ImplTraitArg)
    }
}

fn dyn_type(o: &syn::TypeTraitObject, slot: &Slot) -> Vec<Hit> {
    if is_fn_mut(o) && is_closure_arg(slot) {
        Vec::new()
    } else {
        flagged(o.span(), Rule::Dyn)
    }
}

fn is_fn_mut(o: &syn::TypeTraitObject) -> bool {
    matches!(o.bounds.first(), Some(TypeParamBound::Trait(b)) if o.bounds.len() == 1 && syntax::last_name(&b.path) == "FnMut")
}

fn is_closure_arg(slot: &Slot) -> bool {
    matches!(slot, Slot::GenericArg { segment, .. } if segment == "Closure")
}

fn bool_type(p: &syn::TypePath, slot: &Slot) -> Vec<Hit> {
    if p.qself.is_none() && p.path.is_ident("bool") && !is_bool_slot(slot) {
        flagged(p.span(), Rule::BoolType)
    } else {
        Vec::new()
    }
}

fn is_bool_slot(slot: &Slot) -> bool {
    library_type_arg(slot).is_some() || matches!(slot, Slot::ClapField | Slot::WireField)
}
