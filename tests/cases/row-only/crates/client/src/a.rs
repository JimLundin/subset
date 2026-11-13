mod table {
    struct Row {
        marker: &'static str,
    }

    const DEVICES: &[Row] = &[Row { marker: "a" }, Row { marker: "b" }];
}
