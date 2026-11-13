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

