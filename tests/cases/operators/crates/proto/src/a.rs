use std::ops::BitXor;

pub(crate) struct Add {}

fn unxor(a: u8, b: u8) -> u8 {
    a.bitxor(b)
}

fn sub(a: Seq, b: Seq) -> Option<Seq> {
    a.sub(b)
}

fn make() -> Add {
    Add {}
}
