fn label(name: Option<&str>, kind: Option<Kind>) -> String {
    if let Some(name) = name {
        name.to_string()
    } else {
        String::new()
    }
}

fn note(name: Option<&str>, kind: Option<Kind>) {
    if let Some(name) = name {
        log(name);
    } else if let Some(kind) = kind {
        log(kind.label());
    }
    if let Some(name) = name && let Some(kind) = kind {
        log(name);
    }
}
