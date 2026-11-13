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
