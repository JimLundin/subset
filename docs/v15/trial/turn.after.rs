/// A loop turn takes one input and handles it with one call or `match`.
fn is_turn(body: &Block) -> bool {
    let stmts = body.stmts.as_slice();
    matches!(stmts, [take] if takes_input(take))
        || matches!(stmts, [take, handle] if takes_input(take) && handles(handle))
}

fn takes_input(stmt: &Stmt) -> bool {
    matches!(stmt, Stmt::Macro(m) if is_select(&m.mac))
        || matches!(stmt, Stmt::Local(local) if binds_input(local))
        || matches!(stmt, Stmt::Expr(expr, _) if expr_takes_input(expr))
}

fn expr_takes_input(expr: &Expr) -> bool {
    match expr {
        Expr::Match(m) => {
            awaits(m.expr.as_ref()) || m.arms.iter().all(|arm| is_input(arm.body.as_ref()))
        }
        other => is_input(other),
    }
}

/// A `let` with no `else` whose value is awaited or selected.
fn binds_input(local: &Local) -> bool {
    matches!(&local.init, Some(init) if init.diverge.is_none() && is_input(init.expr.as_ref()))
}

fn is_input(expr: &Expr) -> bool {
    awaits(expr) || matches!(expr, Expr::Macro(m) if is_select(&m.mac))
}

fn handles(stmt: &Stmt) -> bool {
    matches!(stmt, Stmt::Expr(expr, _) if is_handler(syntax::strip(expr)))
}

fn is_handler(expr: &Expr) -> bool {
    is_call(expr)
        || matches!(expr, Expr::Match(_))
        || matches!(expr, Expr::Assign(a) if is_call(syntax::strip(a.right.as_ref())))
}

fn is_call(expr: &Expr) -> bool {
    matches!(expr, Expr::Call(_) | Expr::MethodCall(_))
}

fn is_select(mac: &syn::Macro) -> bool {
    syntax::last_name(&mac.path) == "select"
}

/// An `.await` in `expr`, outside any closure or async block it contains.
fn awaits(expr: &Expr) -> bool {
    matches!(expr, Expr::Await(_)) || (is_open(expr) && inner_awaits(expr))
}

/// An expression whose insides run where it stands.
fn is_open(expr: &Expr) -> bool {
    !matches!(expr, Expr::Closure(_) | Expr::Async(_))
}

fn inner_awaits(expr: &Expr) -> bool {
    syntax::inner_exprs(expr).into_iter().any(awaits)
}

