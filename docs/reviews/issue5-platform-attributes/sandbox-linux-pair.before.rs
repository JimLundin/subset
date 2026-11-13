use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sandbox {
    Available,
    Missing,
}

const SANDBOX_TOOLS: &[&str] = &["bwrap", "socat"];

pub fn sandbox() -> Sandbox {
    let path = std::env::var_os("PATH").unwrap_or_default();
    let dirs = std::env::split_paths(&path).collect::<Vec<PathBuf>>();
    let missing = SANDBOX_TOOLS
        .iter()
        .find(|tool| !dirs.iter().any(|dir| dir.join(tool).is_file()));
    match (std::env::consts::OS, missing) {
        ("macos", _) | ("linux", None) => Sandbox::Available,
        (_, _) => Sandbox::Missing,
    }
}
