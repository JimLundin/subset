struct Row {
    marker: &'static str,
    name: &'static str,
}

const DEVICES: &[Row] = &[Row { marker: "a", name: "A" }, Row { marker: "b", name: "B" }];

const NAMES: &[&str] = &["a", "b"];

fn both(name: &str, kind: Kind) -> Option<&Row> {
    DEVICES.iter().find(|row| row.name == name && row.kind == kind)
}

fn any(name: &str) -> Has {
    Has::from(DEVICES.iter().any(|row| row.name == name))
}

fn numbered(name: &str) -> Option<(usize, &Row)> {
    DEVICES.iter().enumerate().find(|(_, row)| row.name == name)
}

fn destructured(name: &str) -> Option<&Row> {
    DEVICES.iter().find(|Row { name: n, .. }| n.eq(&name))
}

fn block(name: &str) -> Option<&Row> {
    DEVICES.iter().find(|row| { row.name == name })
}

fn negated(name: &str) -> Option<&Row> {
    DEVICES.iter().find(|row| !(row.name != name))
}

fn path(name: &str) -> Option<&Row> {
    DEVICES.iter().find(|row| PartialEq::eq(row.name, name))
}

fn ordering(name: &str) -> Option<&Row> {
    DEVICES.iter().find(|row| row.name.cmp(name).is_eq())
}

fn bound(name: &str) -> Option<&Row> {
    let rows = DEVICES;
    rows.iter().find(|row| row.name == name)
}

fn bare(name: &str) -> Option<usize> {
    NAMES.iter().position(|n| n == &name)
}

fn bare_find(name: &str) -> Option<&&str> {
    NAMES.iter().find(|n| n.eq(&&name))
}
