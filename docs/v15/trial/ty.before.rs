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
            let read_only = match (slot, cx.in_return, cx.mutable_type) {
                (Slot::RefTarget(_), Within::In, Within::Out) => Some(()),
                (Slot::RefTarget(outer), Within::Out, _) => match outer.as_ref() {
                    Slot::Param | Slot::ConstType => Some(()),
                    _ => None,
                },
                _ => None,
            };
            unless(read_only, flag(s.span(), Rule::Slice))
        }
        Type::Array(a) => {
            let fixed_size = match a.len {
                Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Int(_),
                    ..
                })
                | Expr::Path(_) => Some(()),
                _ => None,
            };
            let mutable = match cx.mutable_type {
                Within::Out => Some(()),
                Within::In => match a.elem.as_ref() {
                    Type::Path(p) => p
                        .path
                        .is_ident("u8")
                        .then_some(())
                        .or(cx.library().and(Some(()))),
                    _ => cx.library().and(Some(())),
                },
            };
            unless(fixed_size.and(mutable), flag(a.span(), Rule::ArrayType))
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
