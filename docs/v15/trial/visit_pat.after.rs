    fn visit_pat(&mut self, p: &'ast Pat) {
        let outer = self.cx.clone();
        if let Some(found) = self.pat_id(p) {
            self.id(found.at, p.span(), &found.name);
        }
        self.pat_kind(p);
        self.cx = outer;
    }
    /// The id of a pattern, and the span it is reported at.
    fn pat_id(&self, p: &Pat) -> Option<PatId> {
        match p {
            Pat::Const(c) => pat_id(c.const_token.span, "pat.const"),
            Pat::Ident(i) => pat_id(i.span(), self.ident_id(i)),
            Pat::Lit(l) => pat_id(l.span(), lit_id(&l.lit)),
            Pat::Macro(_) => None,
            Pat::Or(o) => pat_id(o.span(), "pat.or"),
            Pat::Paren(pa) => pat_id(pa.span(), "pat.paren"),
            Pat::Path(pa) => pat_id(pa.span(), "pat.path"),
            Pat::Range(r) => pat_id(r.limits.span(), "pat.range"),
            Pat::Reference(r) => pat_id(r.and_token.span, "pat.reference"),
            Pat::Rest(r) => pat_id(r.span(), "pat.rest"),
            Pat::Slice(s) => pat_id(s.span(), "pat.slice"),
            Pat::Struct(s) => pat_id(s.span(), "pat.struct"),
            Pat::Tuple(t) => pat_id(t.span(), "pat.tuple"),
            Pat::TupleStruct(t) => pat_id(t.span(), "pat.tuple_struct"),
            Pat::Type(t) => pat_id(t.colon_token.span, "pat.type"),
            Pat::Wild(w) => pat_id(w.span(), "pat.wild"),
            _ => pat_id(p.span(), "unknown.pat"),
        }
    }

    /// A `mut` binding inside a `let` already reported as `stmt.let.mut`
    /// is a plain binding.
    fn ident_id(&self, i: &syn::PatIdent) -> &str {
        match (i.by_ref, i.mutability, self.cx.let_mut) {
            (None, Some(_), Within::Out) => "pat.ident.mut",
            (None, _, _) => "pat.ident",
            (Some(_), Some(_), Within::Out) => "pat.ident.ref_mut",
            (Some(_), _, _) => "pat.ident.ref",
        }
    }

    /// Each pattern kind's own walk.
    fn pat_kind(&mut self, p: &Pat) {
        match p {
            Pat::Reference(r) => self.walk_ref_pat(p, r),
            Pat::Tuple(t) => self.walk_rests(p, &t.elems),
            Pat::TupleStruct(t) => self.walk_rests(p, &t.elems),
            Pat::Range(r) => self.walk_range_pat(r),
            Pat::Slice(s) => self.walk_slice_pat(p, s),
            Pat::Rest(r) => self.walk_rest(r),
            Pat::Type(t) => self.walk_typed_pat(p, t),
            Pat::Ident(i) => self.walk_ident_pat(p, i),
            Pat::Lit(_) | Pat::Path(_) => {}
            _ => visit::visit_pat(self, p),
        }
    }

    fn walk_ref_pat(&mut self, p: &Pat, r: &syn::PatReference) {
        if r.mutability.is_some() {
            self.hits.push(flag(r.and_token.span, Rule::SharedPattern));
        }
        visit::visit_pat(self, p);
    }

    /// A `..` directly in a tuple or tuple-struct pattern is placed.
    fn walk_rests(&mut self, p: &Pat, elems: &Punctuated<Pat, Comma>) {
        for rest in elems.iter().filter(|elem| matches!(elem, Pat::Rest(_))) {
            self.slot(rest.span(), Slot::SliceRest);
        }
        visit::visit_pat(self, p);
    }

    fn walk_range_pat(&mut self, range: &syn::PatRange) {
        self.hits.extend(checks::range_pattern(range));
        for attr in &range.attrs {
            self.visit_attribute(attr);
        }
        for bound in range.start.iter().chain(&range.end) {
            self.range_bound(bound);
        }
    }

    fn walk_slice_pat(&mut self, p: &Pat, slice: &syn::PatSlice) {
        for rest in slice.elems.iter().flat_map(checks::slice_rests) {
            self.slot(rest, Slot::SliceRest);
        }
        visit::visit_pat(self, p);
    }

    fn walk_rest(&mut self, rest: &syn::PatRest) {
        if !self.is_placed_rest(rest) {
            self.hits.push(flag(rest.span(), Rule::SliceRest));
        }
    }

    fn is_placed_rest(&self, rest: &syn::PatRest) -> bool {
        matches!(self.slots.get(&key(rest.span())), Some(Slot::SliceRest))
    }

    fn walk_typed_pat(&mut self, p: &Pat, t: &syn::PatType) {
        let slot = self.slots.get(&key(p.span()));
        self.hits.extend(checks::closure_param(t, slot));
        visit::visit_pat(self, p);
    }

    fn walk_ident_pat(&mut self, p: &Pat, i: &syn::PatIdent) {
        if let Some((at, _)) = &i.subpat {
            self.hits.push(self.at_binding(at.span));
        }
        self.hits.extend(checks::pat_ident(i, &self.cx));
        self.hits.extend(checks::ident_at(i));
        visit::visit_pat(self, p);
    }

    /// The `pat.ident.at` occurrence of a `name @ pattern` binding.
    fn at_binding(&self, at: Span) -> Hit {
        let reason = self.env.lists.reasons.get("pat.ident.at").cloned();
        Hit::Id(Occurrence {
            id: "pat.ident.at".to_string(),
            reason: reason.unwrap_or_default(),
            at: syntax::pos(at),
            extent: syntax::extent(at),
        })
    }

/// The id of a pattern, and the span it is reported at.
struct PatId {
    at: Span,
    name: String,
}

fn pat_id(at: Span, name: &str) -> Option<PatId> {
    Some(PatId {
        at,
        name: name.to_string(),
    })
}

fn lit_id(lit: &Lit) -> &str {
    match lit {
        Lit::Bool(_) => "pat.lit.bool",
        _ => "pat.lit",
    }
}
