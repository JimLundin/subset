mod bounds {
    pub const LOW: &str = "a";
    pub const HIGH: &str = "z";
}

fn string(value: &str) -> &str {
    match value {
        text @ bounds::LOW..=bounds::HIGH => text,
        _ => value,
    }
}
