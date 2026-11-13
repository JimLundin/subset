use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sandbox {
    Available,
    Missing,
}

#[cfg(target_os = "linux")]
pub fn sandbox() -> Sandbox {
    const SANDBOX_TOOLS: &[&str] = &["bwrap", "socat"];
    let path = std::env::var_os("PATH").unwrap_or_default();
    let dirs = std::env::split_paths(&path).collect::<Vec<PathBuf>>();
    let missing = SANDBOX_TOOLS
        .iter()
        .find(|tool| !dirs.iter().any(|dir| dir.join(tool).is_file()));
    match missing {
        None => Sandbox::Available,
        Some(_) => Sandbox::Missing,
    }
}

#[cfg(not(target_os = "linux"))]
pub fn sandbox() -> Sandbox {
    match std::env::consts::OS {
        "macos" => Sandbox::Available,
        _ => Sandbox::Missing,
    }
}
