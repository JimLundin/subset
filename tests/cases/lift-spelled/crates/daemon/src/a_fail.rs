fn bound(h: Hello, t: Token) -> Option<Hello> {
    let lifted = Some(h);
    lifted.filter(|h| h.token == t)
}

fn full(h: Hello, t: Token) -> Option<Hello> {
    std::option::Option::Some(h).filter(|h| h.token == t)
}

fn found(h: Hello, t: Token) -> Option<Hello> {
    Some(h).into_iter().find(|h| h.token == t)
}

fn negated(h: Hello, t: Token) -> Option<Hello> {
    Some(h).filter(|h| !(h.token != t))
}

fn less(a: Seq, b: Seq, x: X) -> Option<X> {
    a.lt(&b).then_some(x)
}

fn at_most(a: Seq, b: Seq, x: X) -> Option<X> {
    a.le(&b).then_some(x)
}

fn ordered(a: Seq, b: Seq, x: X) -> Option<X> {
    a.cmp(&b).is_eq().then_some(x)
}

fn path(a: Seq, b: Seq, x: X) -> Option<X> {
    PartialEq::eq(&a, &b).then_some(x)
}

fn paren(a: Seq, b: Seq, x: X) -> Option<X> {
    (a.eq(&b)).then_some(x)
}

fn named(a: Seq, b: Seq, x: X) -> Option<X> {
    let same = a.eq(&b);
    same.then_some(x)
}

fn adopt(cursor: Option<Cursor>, epoch: Epoch) -> Keep {
    match cursor.filter(|c| c.epoch == epoch) {
        Some(_) => Keep::Yes,
        None => Keep::No,
    }
}
