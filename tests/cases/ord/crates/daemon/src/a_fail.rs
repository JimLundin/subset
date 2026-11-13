#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum Phase {
    Early,
    Late,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct Mark(u64);

impl Mark {
    fn after(self, other: Mark) -> Ordering {
        self.0.cmp(&other.0)
    }
}

fn order(a: Mark, b: &Mark, c: Count) -> Ordering {
    a.0.cmp(&b.0).then(c.0.cmp(&a.0))
}
