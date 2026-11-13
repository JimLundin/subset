const ALPN: &[u8] = b"conductor/1";

fn total(sizes: &[u64]) -> Option<u64> {
    sizes.iter().copied().try_fold(0, |sum, size| sum.checked_add(size))
}
