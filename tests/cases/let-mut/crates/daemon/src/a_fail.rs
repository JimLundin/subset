fn count(n: u64) -> Option<u64> {
    let mut total = n;
    total = total.checked_add(1)?;
    Some(total)
}
