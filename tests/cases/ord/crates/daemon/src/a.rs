#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Seq(u64);

fn fresh(seen: Seq, head: Seq) -> Freshness {
    match seen.cmp(&head) {
        Ordering::Less => Freshness::Behind,
        Ordering::Equal | Ordering::Greater => Freshness::Current,
    }
}
