struct Row {
    marker: &'static str,
    name: &'static str,
}

const DEVICES: &[Row] = &[
    Row { marker: "iPhone", name: "iPhone" },
    Row { marker: "Linux", name: "Linux" },
];

pub(crate) fn device_name(user_agent: &str) -> ClientName {
    let name = DEVICES
        .iter()
        .find(|row| user_agent.contains(row.marker))
        .map_or("Browser", |row| row.name);
    ClientName(String::from(name))
}
