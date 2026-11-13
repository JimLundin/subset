use std::ops::{Add, Shl};
use core::ops::SubAssign;

fn total(a: u64, b: u64) -> u64 {
    a.add(b)
}

fn sum(a: u64, b: u64) -> u64 {
    Add::add(a, b)
}

fn product(a: u64, b: u64) -> u64 {
    std::ops::Mul::mul(a, b)
}

fn high(bits: u64) -> u64 {
    bits.shl(70)
}
