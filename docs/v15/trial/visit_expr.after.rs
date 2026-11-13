    fn visit_expr(&mut self, e: &'ast Expr) {
        let outer = self.cx.clone();
        let cx = self.at(e.span());
        self.cx.place = None;
        self.expr_common(e, &cx);
        self.expr_kind(e, cx);
        self.cx = outer;
    }
    /// The ids, names and rules every expression gets, whatever its kind.
    fn expr_common(&mut self, e: &Expr, cx: &Cx) {
        let node = e.span();
        if let Some((at, name)) = expr_id(e) {
            self.expr_named(at, node, &name);
        }
        self.hits.extend(self.expr_names(e));
        if checks::library_value(&cx.slot).is_some() {
            self.hits.push(mark(node, Fixed::IfRestricted));
        }
        self.hits.extend(checks::expr(e, cx, &self.env));
    }

    /// The listed operator traits and readers an expression names.
    fn expr_names(&self, e: &Expr) -> Vec<Hit> {
        match e {
            Expr::Path(p) if p.qself.is_none() => checks::named(
                &syntax::segments(&p.path),
                checks::Named::Path,
                p.span(),
                &self.env,
            ),
            Expr::MethodCall(m) if self.is_operator_method(m) => {
                Vec::from([flag(m.method.span(), Rule::OperatorMethod)])
            }
            _ => Vec::new(),
        }
    }

    fn is_operator_method(&self, m: &syn::ExprMethodCall) -> bool {
        self.env.operator_methods.contains(&m.method.to_string())
    }

    /// Each expression kind's own walk.
    fn expr_kind(&mut self, e: &Expr, cx: Cx) {
        match e {
            Expr::Call(c) => self.walk_call(e, c, &cx),
            Expr::MethodCall(m) => self.walk_method(e, m, &cx),
            Expr::Struct(s) => self.walk_struct(e, s, &cx),
            Expr::Closure(c) => self.walk_closure(e, c, cx),
            Expr::Reference(r) => self.walk_reference(e, r, &cx),
            Expr::Binary(b) => self.walk_binary(e, b, &cx),
            Expr::Paren(p) => self.walk_wrapped(e, &p.expr, &cx),
            Expr::Group(g) => self.walk_wrapped(e, &g.expr, &cx),
            Expr::Loop(l) => self.walk_loop(e, l, &cx),
            Expr::ForLoop(f) => self.walk_for(f),
            Expr::Match(m) => self.walk_match(e, m),
            Expr::If(i) => self.walk_if(e, i),
            Expr::While(w) => self.walk_while(e, w),
            _ => visit::visit_expr(self, e),
        }
    }

    fn walk_call(&mut self, e: &Expr, c: &syn::ExprCall, cx: &Cx) {
        let callee = callee_of(c.func.as_ref(), &self.env);
        self.hits.extend(checks::call(c, &callee, cx));
        for a in c.args.iter() {
            self.slot(a.span(), Slot::CallArg(callee.clone()));
        }
        visit::visit_expr(self, e);
    }

    fn walk_method(&mut self, e: &Expr, m: &syn::ExprMethodCall, cx: &Cx) {
        let method = m.method.to_string();
        self.slot(m.receiver.span(), Slot::MethodReceiver);
        if self.env.lists.adapters.contains(&method) {
            self.hits.push(adapter_refinement(e.span()));
        }
        self.hits.extend(checks::method(m, cx, &self.env));
        if self.iterates_receiver(&method, cx) {
            self.iterate(&m.receiver);
        }
        self.method_args(m, &method);
        visit::visit_expr(self, e);
    }

    /// `into_iter` on a range returns the range itself, so it iterates only
    /// when its own result is iterated.
    fn iterates_receiver(&self, method: &str, cx: &Cx) -> bool {
        checks::iterates(method, &self.env).is_some()
            || method == "contains"
            || (method == "into_iter" && matches!(cx.iterated, Within::In))
    }

    fn method_args(&mut self, m: &syn::ExprMethodCall, method: &str) {
        let receiver = receiver_owner(&m.receiver);
        for a in m.args.iter() {
            let slot = Slot::MethodArg {
                method: method.to_string(),
                receiver,
            };
            self.slot(a.span(), slot);
        }
        if method == "zip" || method == "chain" {
            for a in m.args.iter() {
                self.iterate(a);
            }
        }
    }

    fn walk_struct(&mut self, e: &Expr, s: &syn::ExprStruct, cx: &Cx) {
        let owner = ours(&s.path, &self.env);
        let wire = if self.env.facts.wire.contains(&syntax::last_name(&s.path)) {
            Within::In
        } else {
            Within::Out
        };
        if !self.builds.contains(&key(e.span())) {
            self.hits.push(built(&s.path, &self.cx.module));
        }
        self.hits.extend(checks::struct_expr(s, owner, cx));
        for f in s.fields.iter() {
            self.slot(f.expr.span(), Slot::FieldValue { owner, wire });
        }
        visit::visit_expr(self, e);
    }

    fn walk_closure(&mut self, e: &Expr, c: &syn::ExprClosure, cx: Cx) {
        if let Slot::LocalClosure = cx.slot {
            for input in &c.inputs {
                self.slot(input.span(), Slot::LocalClosureParam);
            }
        }
        let name = if matches!(cx.slot, Slot::MethodArg { .. }) {
            "expr.closure.adapter"
        } else {
            "expr.closure"
        };
        self.id(c.span(), c.span(), name);
        self.closure_modifiers(c);
        self.hits.extend(checks::closure(c, &cx.slot, &self.env));
        if self.is_predicate_adapter(&cx.slot) {
            self.predicate(&c.body);
            self.cx.in_predicate = Within::In;
        }
        self.slot(c.body.span(), Slot::ClosureBody(Box::new(cx.slot)));
        visit::visit_expr(self, e);
    }

    fn closure_modifiers(&mut self, c: &syn::ExprClosure) {
        if let Some(token) = c.asyncness {
            self.id(token.span, token.span, "expr.closure.async");
        }
        if let Some(token) = c.constness {
            self.id(token.span, token.span, "expr.closure.const");
        }
        if let Some(token) = c.movability {
            self.id(token.span, token.span, "expr.closure.static");
        }
    }

    fn is_predicate_adapter(&self, slot: &Slot) -> bool {
        matches!(slot, Slot::MethodArg { method, .. } if self.env.lists.predicate_adapters.contains(method))
    }

    fn walk_reference(&mut self, e: &Expr, r: &syn::ExprReference, cx: &Cx) {
        match r.mutability {
            Some(_) => self.slot(r.expr.span(), Slot::MutableArray),
            None => self.keep_slot(&r.expr, cx),
        }
        visit::visit_expr(self, e);
    }

    /// A shared reference preserves a literal array's slot.
    fn keep_slot(&mut self, inner: &Expr, cx: &Cx) {
        if is_array_slot(&cx.slot) {
            self.slot(inner.span(), cx.slot.clone());
        }
    }

    fn walk_binary(&mut self, e: &Expr, b: &syn::ExprBinary, cx: &Cx) {
        if matches!(cx.predicate, Within::In) && matches!(b.op, BinOp::And(_) | BinOp::Or(_)) {
            self.predicates.insert(key(b.left.span()));
            self.predicates.insert(key(b.right.span()));
        }
        visit::visit_expr(self, e);
    }

    /// A parenthesis or an invisible group passes its slot and iteration on.
    fn walk_wrapped(&mut self, e: &Expr, inner: &Expr, cx: &Cx) {
        if !matches!(cx.slot, Slot::Other) {
            self.slot(inner.span(), cx.slot.clone());
        }
        if let Within::In = cx.iterated {
            self.iterate(inner);
        }
        visit::visit_expr(self, e);
    }

    fn walk_loop(&mut self, e: &Expr, l: &syn::ExprLoop, cx: &Cx) {
        self.cx.lp = if is_actor_loop(e, cx) {
            Loop::Actor
        } else {
            Loop::Other
        };
        for a in l.attrs.iter() {
            self.visit_attribute(a);
        }
        self.visit_block(&l.body);
    }

    fn walk_for(&mut self, f: &syn::ExprForLoop) {
        self.iterate(&f.expr);
        for a in f.attrs.iter() {
            self.visit_attribute(a);
        }
        self.visit_pat(&f.pat);
        self.visit_expr(&f.expr);
        self.cx.lp = Loop::Other;
        self.visit_block(&f.body);
    }

    fn walk_match(&mut self, e: &Expr, m: &syn::ExprMatch) {
        self.hits.extend(checks::guards(m));
        visit::visit_expr(self, e);
    }

    fn walk_if(&mut self, e: &Expr, i: &syn::ExprIf) {
        match i.cond.as_ref() {
            Expr::Let(cond) => self.walk_if_let(e, i, cond),
            cond => self.walk_plain_if(e, i, cond),
        }
    }

    fn walk_if_let(&mut self, e: &Expr, i: &syn::ExprIf, cond: &syn::ExprLet) {
        match &i.else_branch {
            Some(_) => self.id(i.if_token.span, e.span(), "expr.if_let"),
            None => {
                self.id(i.if_token.span, e.span(), "expr.if_let.single");
                let head = syntax::extent(i.if_token.span).to(syntax::extent(cond.expr.span()));
                self.hits.push(Hit::Fold(head));
            }
        }
        if checks::filtered_once(&cond.expr).is_some() {
            self.hits.push(flag(i.if_token.span, Rule::LiftMatched));
        }
        self.let_head(&i.attrs, cond);
        self.visit_block(&i.then_branch);
        if let Some((_, otherwise)) = &i.else_branch {
            self.visit_expr(otherwise);
        }
    }

    fn walk_plain_if(&mut self, e: &Expr, i: &syn::ExprIf, cond: &Expr) {
        let name = if checks::has_let(cond).is_some() {
            "expr.if_let"
        } else {
            "expr.if"
        };
        self.id(i.if_token.span, e.span(), name);
        visit::visit_expr(self, e);
    }

    fn walk_while(&mut self, e: &Expr, w: &syn::ExprWhile) {
        self.cx.lp = Loop::Other;
        match w.cond.as_ref() {
            Expr::Let(cond) => {
                self.id(w.while_token.span, e.span(), "expr.while_let");
                self.let_head(&w.attrs, cond);
                self.visit_block(&w.body);
            }
            _ => {
                self.id(w.while_token.span, e.span(), "expr.while");
                visit::visit_expr(self, e);
            }
        }
    }

fn callee_of(func: &Expr, env: &Env) -> Callee {
    match func {
        Expr::Path(p) => Callee {
            path: syntax::segments(&p.path),
            owner: ours(&p.path, env),
        },
        _ => Callee {
            path: Vec::new(),
            owner: Owner::Theirs,
        },
    }
}

fn receiver_owner(receiver: &Expr) -> Owner {
    if syntax::ident_of(receiver).as_deref() == Some("self") {
        Owner::Ours
    } else {
        Owner::Theirs
    }
}

fn adapter_refinement(node: Span) -> Hit {
    Hit::Refine(Refinement {
        extent: syntax::extent(node),
        from: Some("expr.method_call".to_string()),
        to: "expr.method_call.adapter".to_string(),
    })
}

fn is_array_slot(slot: &Slot) -> bool {
    matches!(
        slot,
        Slot::CallArg(_) | Slot::MethodArg { .. } | Slot::MethodReceiver | Slot::ConstInit
    )
}

fn is_actor_loop(e: &Expr, cx: &Cx) -> bool {
    matches!(cx.func.as_ref().and_then(|f| f.actor_loop), Some(extent) if extent == syntax::extent(e.span()))
}
