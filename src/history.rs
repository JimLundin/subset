//! The score per crate at past commits, read through git.

use std::collections::BTreeSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::config::Config;
use crate::score::{self, Level};
use crate::{covers, display};

/// The score per crate at one commit.
pub(crate) struct Snapshot {
    pub(crate) sha: String,
    /// The commit date, `YYYY-MM-DD`.
    pub(crate) date: String,
    pub(crate) crates: Vec<(String, f64)>,
}

/// The score per crate at each of the last `n` commits that changed files
/// under `include`, under today's config, oldest first.
pub(crate) fn snapshots(
    config: &Config,
    config_path: &Path,
    root: &Path,
    n: usize,
) -> Result<Vec<Snapshot>, String> {
    let weights = config.weights.as_ref().ok_or("no [weights] table")?;
    let include = crate::config::globs(&config.include)?;
    let count = format!("-n{n}");
    let globs = config
        .include
        .iter()
        .map(|p| format!(":(glob){p}"))
        .collect::<Vec<String>>();
    let args = Vec::from(["log", count.as_str(), "--format=%H %cs", "--"])
        .into_iter()
        .chain(globs.iter().map(String::as_str))
        .collect::<Vec<&str>>();
    let log = git(root, &args)?;
    let commits = log
        .lines()
        .filter_map(|line| line.split_once(' '))
        .map(|(sha, date)| (sha.to_string(), date.to_string()))
        .rev()
        .collect::<Vec<(String, String)>>();
    commits
        .into_iter()
        .map(|(sha, date)| {
            let tree = git(root, &Vec::from(["ls-tree", "-r", "--name-only", sha.as_str()]))?
                .lines()
                .map(PathBuf::from)
                .collect::<Vec<PathBuf>>();
            let manifests = tree
                .iter()
                .filter(|p| p.file_name().and_then(|n| n.to_str()) == Some("Cargo.toml"))
                .filter_map(|p| p.parent().map(Path::to_path_buf))
                .collect::<BTreeSet<PathBuf>>();
            let rust = tree
                .iter()
                .filter(|p| include.iter().any(|g| covers(g, p).is_some()))
                .cloned()
                .collect::<Vec<PathBuf>>();
            let sources = read(root, &sha, &rust)?;
            let scored = crate::scored(config, config_path, sources, &manifests);
            Ok(scored.ok().map(|files| Snapshot {
                crates: score::rows(&files, Level::Crate, weights)
                    .iter()
                    .map(|r| (r.name.clone(), r.score(weights)))
                    .collect(),
                sha,
                date,
            }))
        })
        .collect::<Result<Vec<Option<Snapshot>>, String>>()
        .map(|all| all.into_iter().flatten().collect())
}

/// The text of each of `paths` at `sha`, read with one `git cat-file`.
fn read(root: &Path, sha: &str, paths: &[PathBuf]) -> Result<Vec<(PathBuf, String)>, String> {
    let mut child = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["cat-file", "--batch"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| format!("git cat-file: {e}"))?;
    let requests = paths
        .iter()
        .map(|p| format!("{sha}:./{}\n", display(p)))
        .collect::<String>();
    let mut stdin = child.stdin.take().ok_or("git cat-file: no stdin")?;
    stdin
        .write_all(requests.as_bytes())
        .map_err(|e| format!("git cat-file: {e}"))?;
    drop(stdin);
    let out = child
        .wait_with_output()
        .map_err(|e| format!("git cat-file: {e}"))?;
    blobs(&out.stdout, paths)
}

/// The blobs of `git cat-file --batch` output, one per path.
fn blobs(bytes: &[u8], paths: &[PathBuf]) -> Result<Vec<(PathBuf, String)>, String> {
    match paths.split_first() {
        None => Ok(Vec::new()),
        Some((path, rest)) => {
            let end = bytes
                .iter()
                .position(|b| b == &b'\n')
                .ok_or("git cat-file: short output")?;
            let (header, after) = bytes.split_at_checked(end).ok_or("git cat-file: short output")?;
            let header = String::from_utf8_lossy(header);
            let size = header
                .split_whitespace()
                .nth(2)
                .and_then(|s| s.parse::<usize>().ok())
                .ok_or_else(|| format!("git cat-file: {}: {}", display(path), header.trim()))?;
            let (body, after) = after
                .split_at_checked(1)
                .and_then(|(_, body)| body.split_at_checked(size))
                .ok_or("git cat-file: short output")?;
            let text = String::from_utf8(body.to_vec()).map_err(|e| format!("{}: {e}", display(path)))?;
            let after = after.split_at_checked(1).map_or(after, |(_, rest)| rest);
            Ok(std::iter::once((path.clone(), text))
                .chain(blobs(after, rest)?)
                .collect())
        }
    }
}

/// The short sha and date of `HEAD`, if `root` is in a git repository.
pub(crate) fn head(root: &Path) -> Option<String> {
    git(root, &Vec::from(["log", "-1", "--format=%h, %cs"]))
        .ok()
        .map(|s| s.trim().to_string())
}

fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|e| format!("git: {e}"))?;
    let first = args
        .first()
        .map(ToString::to_string)
        .unwrap_or_default();
    Some(&out)
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .ok_or_else(|| format!("git {first}: {}", String::from_utf8_lossy(&out.stderr).trim()))
}
