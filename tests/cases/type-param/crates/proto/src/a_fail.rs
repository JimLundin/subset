pub struct Tagged<T> {
    value: T,
}

fn copy<T: Clone>(value: &T) -> T {
    value.clone()
}
