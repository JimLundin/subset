struct Row {
    marker: &'static str,
    name: &'static str,
}

const DEVICES: &[Row] = &[Row { marker: "iPhone", name: "iPhone" }];

struct Label {
    text: &'static str,
}

const PAIRS: &[(&'static str, Kind)] = &[("a", Kind::A)];

fn extra() -> Row {
    Row { marker: "Mac", name: "Mac" }
}

fn label(kind: Kind) -> &'static str {
    match kind {
        Kind::A => "a",
    }
}

fn by_name(name: &str) -> Option<&Row> {
    DEVICES.iter().find(|row| row.name == name)
}

fn by_eq(name: &str) -> Option<usize> {
    DEVICES.iter().position(|row| name.eq(row.marker))
}

fn kind(name: &str) -> Option<usize> {
    PAIRS.iter().position(|(key, _)| key.eq(&name))
}
