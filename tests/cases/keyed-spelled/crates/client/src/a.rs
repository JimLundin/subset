struct Row {
    marker: &'static str,
    name: &'static str,
}

const DEVICES: &[Row] = &[Row { marker: "a", name: "A" }, Row { marker: "b", name: "B" }];

fn scan(user_agent: &str) -> Option<&Row> {
    DEVICES.iter().find(|row| user_agent.contains(row.marker))
}

fn prefix(user_agent: &str) -> Option<&Row> {
    DEVICES.iter().find(|row| user_agent.starts_with(row.marker))
}

fn suffix(user_agent: &str) -> Option<&Row> {
    DEVICES.iter().find(|row| user_agent.ends_with(row.marker))
}
