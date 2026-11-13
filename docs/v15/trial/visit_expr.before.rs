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
