fn mutable_literal() -> External {
    let mut values = [1, 2];
    borrow(&mut values)
}
fn mutable_group() -> External { borrow(&mut (([1, 2]))) }
fn mutable_domain(values: &mut [Duration; 4]) -> External { borrow(values) }
fn indexing(values: [u8; 2]) -> u8 { values[0] }
struct StoredSlice { bytes: &[u8] }
fn mutable_output(bytes: &mut [u8]) -> &mut [u8] { bytes }
fn mutable_nested(bytes: &mut [u8]) -> Option<&mut [u8]> { Some(bytes) }
struct Bounded<T: Clone> { value: T }
struct Defaulted<T = String> { value: T }
struct Counted<const N: usize> { values: [u8; N] }
struct Borrowed<'a> { value: &'a str }
fn bounded<T: Clone>(value: T) -> T { value }
fn predicate(value: bool) -> bool { value }
