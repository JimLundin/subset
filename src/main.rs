//! `subset`: enforce and measure the Conductor Rust subset (docs/subset.md).

mod checks;
mod config;
mod facts;
mod history;
mod measure;
mod report;
mod score;
mod syntax;
mod table;
mod walk;

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;
use std::rc::Rc;

use config::Config;
use facts::{CfgItem, Fact, Facts, FileNames, Tables};
use walk::{Env, Finding, Hit, Mark, Occurrence, Within};

const USAGE: &str = "usage:
  subset check [--config subset.toml]
  subset report [--config subset.toml] [--json]
  subset measure <paths...> [--json]
  subset score [paths...] [--config subset.toml] [--json] [--by crate|file|fn|construct]
  subset report-html --out <dir> [--config subset.toml] [--compare <dir>]... [--history <n>]
  subset ids
  subset score-ids
  subset rules";

const VALUE_FLAGS: &[&str] = &["--config", "--out", "--compare", "--history", "--by"];

/// An error that ends the run with exit code 2.
pub(crate) struct Fatal(pub(crate) String);

pub(crate) struct Violation {
    pub(crate) path: String,
    pub(crate) id: String,
    pub(crate) reason: String,
    pub(crate) line: usize,
    pub(crate) col: usize,
}

impl Violation {
    fn location(&self) -> String {
        format!("{}:{}:{}", self.path, self.line, self.col)
    }
}

fn main() -> ExitCode {
    let args = std::env::args().skip(1).collect::<Vec<String>>();
    match run(&args) {
        Ok(code) => code,
        Err(Fatal(message)) => {
            let _ = writeln!(std::io::stderr(), "subset: {message}");
            ExitCode::from(2)
        }
    }
}

fn usage(problem: &str) -> Fatal {
    Fatal(format!("{problem}{USAGE}"))
}

/// The value after `flag`.
fn value(args: &[String], flag: &str) -> Option<String> {
    args.iter().skip_while(|a| a.as_str() != flag).nth(1).cloned()
}

fn run(args: &[String]) -> Result<ExitCode, Fatal> {
    let (command, rest) = args.split_first().ok_or_else(|| usage(""))?;
    let previous = std::iter::once(None).chain(rest.iter().map(Some));
    let paths = rest
        .iter()
        .zip(previous)
        .filter(|(arg, before)| {
            !arg.starts_with("--") && !before.is_some_and(|b| VALUE_FLAGS.contains(&b.as_str()))
        })
        .map(|(arg, _)| PathBuf::from(arg))
        .collect::<Vec<PathBuf>>();
    let unknown = rest
        .iter()
        .find(|a| a.starts_with("--") && !VALUE_FLAGS.contains(&a.as_str()) && a.as_str() != "--json");
    let missing = rest
        .last()
        .filter(|last| VALUE_FLAGS.contains(&last.as_str()));
    match (unknown, missing) {
        (Some(flag), _) => Err(usage(&format!("unknown flag {flag}\n"))),
        (None, Some(_)) => Err(usage("")),
        (None, None) => Ok(()),
    }?;
    let config = PathBuf::from(value(rest, "--config").unwrap_or("subset.toml".to_string()));
    let json = rest
        .iter()
        .find(|a| a.as_str() == "--json")
        .map_or(Within::Out, |_| Within::In);
    let by = value(rest, "--by")
        .map(|v| score::Level::parse(&v).ok_or_else(|| usage(&format!("unknown level {v}\n"))))
        .transpose()?
        .unwrap_or(score::Level::Crate);
    let history = value(rest, "--history")
        .map(|v| {
            v.parse::<usize>()
                .map_err(|_| Fatal(format!("--history takes a count, not {v}")))
        })
        .transpose()?;
    let compare = rest
        .iter()
        .zip(rest.iter().skip(1))
        .filter(|(flag, _)| flag.as_str() == "--compare")
        .map(|(_, dir)| PathBuf::from(dir))
        .collect::<Vec<PathBuf>>();
    match (command.as_str(), paths.first(), value(rest, "--out")) {
        ("check", _, _) => check(&config),
        ("report", _, _) => report(&config, json),
        ("measure", Some(_), _) => measure(&paths, json),
        ("score", _, _) => score(&config, &paths, by, json),
        ("report-html", _, Some(out)) => report_html(&config, Path::new(&out), &compare, history),
        ("report-html", _, None) => Err(usage("report-html needs --out <dir>\n")),
        ("ids", _, _) => print(table::ids_table()),
        ("score-ids", _, _) => print(score::ids_table()),
        ("rules", _, _) => print(table::rules_table()),
        _ => Err(usage("")),
    }
}

fn print(text: String) -> Result<ExitCode, Fatal> {
    std::io::stdout()
        .write_all(text.as_bytes())
        .map_err(|e| Fatal(format!("stdout: {e}")))?;
    Ok(ExitCode::SUCCESS)
}

fn check(config: &Path) -> Result<ExitCode, Fatal> {
    let violations = violations(config)?;
    let text = violations
        .iter()
        .map(|v| format!("{}: {}: {}\n", v.location(), v.id, v.reason))
        .collect::<String>();
    print(text)?;
    Ok(violations.first().map_or(ExitCode::SUCCESS, |_| ExitCode::FAILURE))
}

fn report(config: &Path, json: Within) -> Result<ExitCode, Fatal> {
    let by_id = violations(config)?
        .into_iter()
        .fold(BTreeMap::<String, Vec<Violation>>::new(), |mut by_id, v| {
            by_id.entry(v.id.clone()).or_default().push(v);
            by_id
        });
    let total = by_id.values().map(Vec::len).sum::<usize>();
    let rows = by_id
        .into_iter()
        .map(|(id, seen)| ((std::cmp::Reverse(seen.len()), id.clone()), (id, seen)))
        .collect::<BTreeMap<(std::cmp::Reverse<usize>, String), (String, Vec<Violation>)>>()
        .into_values()
        .collect::<Vec<(String, Vec<Violation>)>>();
    let text = match json {
        Within::In => {
            let ids = rows
                .iter()
                .map(|(id, seen)| {
                    let (files, examples) = summary(seen);
                    object(Vec::from([
                        ("count", seen.len().into()),
                        ("examples", examples.into()),
                        ("files", files.into()),
                        ("id", id.clone().into()),
                        ("reason", seen.first().map(|v| v.reason.clone()).unwrap_or_default().into()),
                    ]))
                })
                .collect::<Vec<serde_json::Value>>();
            let out = object(Vec::from([("ids", ids.into()), ("total", total.into())]));
            format!("{}\n", serde_json::to_string_pretty(&out).unwrap_or_default())
        }
        Within::Out => rows
            .iter()
            .map(|(id, seen)| {
                let (files, examples) = summary(seen);
                let noun = match files {
                    1 => "file",
                    _ => "files",
                };
                format!("{id} {} in {files} {noun}: {}\n", seen.len(), examples.join(" "))
            })
            .collect(),
    };
    print(text)
}

/// How many files a group of violations is in, and its first locations.
fn summary(seen: &[Violation]) -> (usize, Vec<String>) {
    let files = seen.iter().map(|v| v.path.as_str()).collect::<BTreeSet<&str>>();
    let examples = seen.iter().take(5).map(Violation::location).collect::<Vec<String>>();
    (files.len(), examples)
}

/// A JSON object of the given fields.
pub(crate) fn object(fields: Vec<(&str, serde_json::Value)>) -> serde_json::Value {
    serde_json::Value::Object(
        fields
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
    )
}

/// The files named, and every `*.rs` file under the directories named.
fn rust_files(paths: &[PathBuf]) -> Result<BTreeSet<PathBuf>, Fatal> {
    paths
        .iter()
        .map(|path| match Some(path).filter(|p| p.is_dir()) {
            Some(dir) => glob_files(&dir.join("**/*.rs")),
            None => Ok(Vec::from([path.clone()])),
        })
        .collect::<Result<Vec<Vec<PathBuf>>, Fatal>>()
        .map(|lists| lists.into_iter().flatten().collect())
}

fn measure(paths: &[PathBuf], json: Within) -> Result<ExitCode, Fatal> {
    let rows = rust_files(paths)?
        .iter()
        .map(|file| {
            let src = read(file)?;
            let volume = measure::measure(&src)
                .map_err(|e| Fatal(format!("{}: cannot lex: {e}", display(file))))?;
            Ok((display(file), volume))
        })
        .collect::<Result<Vec<(String, measure::Volume)>, Fatal>>()?;
    let tokens = rows.iter().map(|(_, v)| v.tokens).sum::<usize>();
    let lines = rows.iter().map(|(_, v)| v.lines).sum::<usize>();
    let text = match json {
        Within::In => {
            let files = rows
                .iter()
                .map(|(path, v)| {
                    object(Vec::from([
                        ("lines", v.lines.into()),
                        ("path", path.clone().into()),
                        ("tokens", v.tokens.into()),
                    ]))
                })
                .collect::<Vec<serde_json::Value>>();
            let total = object(Vec::from([("lines", lines.into()), ("tokens", tokens.into())]));
            let out = object(Vec::from([("files", files.into()), ("total", total)]));
            format!("{}\n", serde_json::to_string_pretty(&out).unwrap_or_default())
        }
        Within::Out => rows
            .iter()
            .map(|(path, v)| format!("{path}: {} tokens, {} lines\n", v.tokens, v.lines))
            .chain(Some(format!("total: {tokens} tokens, {lines} lines\n")))
            .collect(),
    };
    print(text)
}

fn score(config_path: &Path, paths: &[PathBuf], by: score::Level, json: Within) -> Result<ExitCode, Fatal> {
    let config = config::load(config_path)?;
    let root = root_of(config_path);
    let files = match paths.first() {
        None => included(&config, root)?,
        Some(_) => rust_files(paths)?,
    };
    let sources = sources(root, &files)?;
    let manifests = manifests(root, &sources);
    let scored = scored(&config, config_path, sources, &manifests)?;
    let weights = weights(&config)?;
    let text = score::output(&scored, by, json, weights);
    print(text)
}

fn weights(config: &Config) -> Result<&config::Weights, Fatal> {
    config
        .weights
        .as_ref()
        .ok_or_else(|| Fatal("the config has no [weights] table".to_string()))
}

fn report_html(
    config_path: &Path,
    out: &Path,
    compare: &[PathBuf],
    history: Option<usize>,
) -> Result<ExitCode, Fatal> {
    let config = config::load(config_path)?;
    let root = root_of(config_path);
    let weights = weights(&config)?;
    let ours = sources(root, &included(&config, root)?)?;
    let dirs = manifests(root, &ours);
    let own = report::Group {
        title: config.include.join(", "),
        note: "The files under <code>include</code> in the config: the code written in the subset."
            .to_string(),
        comparison: Within::Out,
        files: scored(&config, config_path, ours, &dirs)?,
    };
    let others = compare
        .iter()
        .map(|dir| {
            let theirs = sources(root, &rust_files(std::slice::from_ref(dir))?)?;
            let dirs = manifests(root, &theirs);
            Ok(report::Group {
                title: format!("{}/", display(dir).trim_end_matches('/')),
                note: "Outside <code>include</code>, scored under the same rules for comparison. \
                       Code the subset doesn't cover counts as violations."
                    .to_string(),
                comparison: Within::In,
                files: scored(&config, config_path, theirs, &dirs)?,
            })
        })
        .collect::<Result<Vec<report::Group>, Fatal>>()?;
    let snapshots = match history {
        Some(n) => history::snapshots(&config, config_path, root, n),
        None => Err("History wasn't generated. Pass --history <n> to add it.".to_string()),
    };
    let stamp = match history::head(root) {
        Some(head) => format!("{} at {head}.", config.include.join(", ")),
        None => format!("{}.", config.include.join(", ")),
    };
    let groups = std::iter::once(own).chain(others).collect::<Vec<report::Group>>();
    let html = report::render(&stamp, &groups, &snapshots, weights);
    std::fs::create_dir_all(out).map_err(|e| Fatal(format!("{}: {e}", display(out))))?;
    let index = out.join("index.html");
    std::fs::write(&index, html).map_err(|e| Fatal(format!("{}: {e}", display(&index))))?;
    Ok(ExitCode::SUCCESS)
}

/// The directories, relative to `root`, that hold a `Cargo.toml` above the
/// given files.
fn manifests(root: &Path, sources: &[(PathBuf, String)]) -> BTreeSet<PathBuf> {
    sources
        .iter()
        .flat_map(|(path, _)| path.ancestors().skip(1).map(Path::to_path_buf))
        .filter(|dir| root.join(dir).join("Cargo.toml").is_file())
        .collect()
}

/// Scores `sources` under the config's allowed ids and weights.
pub(crate) fn scored(
    config: &Config,
    config_path: &Path,
    sources: Vec<(PathBuf, String)>,
    manifests: &BTreeSet<PathBuf>,
) -> Result<Vec<score::ScoredFile>, Fatal> {
    let weights = weights(config)?;
    analyse(config, sources)?
        .iter()
        .map(|a| score::file(a, weights, &crate_of(&a.path, manifests)))
        .collect()
}

/// The nearest directory above `path` that holds a `Cargo.toml`, or `.`
/// when there is none.
pub(crate) fn crate_of(path: &Path, manifests: &BTreeSet<PathBuf>) -> String {
    path.ancestors()
        .skip(1)
        .find(|dir| manifests.contains(dir.to_path_buf().as_path()))
        .filter(|dir| !dir.as_os_str().is_empty())
        .map_or_else(|| ".".to_string(), display)
}

/// Every disallowed construct under `include`, in path and position order.
fn violations(config_path: &Path) -> Result<Vec<Violation>, Fatal> {
    let config = config::load(config_path)?;
    let root = root_of(config_path);
    let sources = sources(root, &included(&config, root)?)?;
    Ok(analyse(&config, sources)?
        .into_iter()
        .flat_map(|a| a.violations)
        .collect())
}

/// The directory paths in the config are relative to.
pub(crate) fn root_of(config_path: &Path) -> &Path {
    match config_path.parent() {
        Some(parent) => match parent.as_os_str().to_str() {
            Some("") => Path::new("."),
            _ => parent,
        },
        None => Path::new("."),
    }
}

fn included(config: &Config, root: &Path) -> Result<BTreeSet<PathBuf>, Fatal> {
    config
        .include
        .iter()
        .map(|pattern| glob_files(&root.join(pattern)))
        .collect::<Result<Vec<Vec<PathBuf>>, Fatal>>()
        .map(|lists| lists.into_iter().flatten().collect())
}

/// Each file's path relative to `root`, and its text.
fn sources(root: &Path, files: &BTreeSet<PathBuf>) -> Result<Vec<(PathBuf, String)>, Fatal> {
    files
        .iter()
        .map(|file| {
            let relative = file.strip_prefix(root).unwrap_or(file).to_path_buf();
            Ok((relative, read(file)?))
        })
        .collect()
}

/// One file as `check` sees it, with what the score needs on top.
pub(crate) struct Analysed {
    /// Relative to the config's directory.
    pub(crate) path: PathBuf,
    pub(crate) src: String,
    pub(crate) occurrences: Vec<Occurrence>,
    /// Per occurrence, whether it is a violation.
    pub(crate) violating: Vec<Within>,
    pub(crate) violations: Vec<Violation>,
    pub(crate) hits: Vec<Hit>,
}

/// A parsed file and where it stands.
struct Parsed {
    path: PathBuf,
    src: String,
    file: syn::File,
}

/// Every file's analysis. Paths are relative to the config's directory.
pub(crate) fn analyse(config: &Config, sources: Vec<(PathBuf, String)>) -> Result<Vec<Analysed>, Fatal> {
    let results = sources
        .into_iter()
        .map(|(path, src)| match syn::parse_file(&src) {
            Ok(file) => Ok(Parsed { path, src, file }),
            Err(e) => {
                let start = e.span().start();
                Err(format!(
                    "{}:{}:{}: parse error: {e}",
                    display(&path),
                    start.line,
                    start.column.saturating_add(1)
                ))
            }
        })
        .collect::<Vec<Result<Parsed, String>>>();
    let errors = results
        .iter()
        .filter_map(|r| r.as_ref().err().cloned())
        .collect::<Vec<String>>();
    match errors.first() {
        Some(_) => Err(Fatal(errors.join("\n"))),
        None => Ok(()),
    }?;
    let parsed = results.into_iter().filter_map(Result::ok).collect::<Vec<Parsed>>();
    let lists = config.lists.clone();
    let none = Rc::new(Facts::none());
    let first = parsed
        .iter()
        .map(|p| {
            let env = environment(p, &none, &lists, &[], config);
            let facts = walk::file(&p.file, env)
                .into_iter()
                .filter_map(|h| match h {
                    Hit::Fact(f) => Some(f),
                    _ => None,
                })
                .collect();
            (components(&p.path), facts)
        })
        .collect::<Vec<(Vec<String>, Vec<Fact>)>>();
    let facts = Rc::new(Facts::gather(&first));
    parsed
        .into_iter()
        .zip(&first)
        .map(|(p, (_, own))| {
            let env = environment(&p, &facts, &lists, own, config);
            let hits = walk::file(&p.file, env);
            Ok(judge(p, hits, config))
        })
        .collect()
}

fn components(path: &Path) -> Vec<String> {
    path.components()
        .filter_map(|c| match c {
            Component::Normal(part) => part.to_str().map(ToString::to_string),
            _ => None,
        })
        .collect()
}

/// What one file's walk reads.
fn environment(p: &Parsed, facts: &Rc<Facts>, lists: &Rc<facts::Lists>, own: &[Fact], config: &Config) -> Env {
    let parts = components(&p.path);
    let names = FileNames::of(own, facts);
    let operator_methods = names.operator_methods(&lists.operators);
    let cfg_items = own
        .iter()
        .filter_map(|f| match f {
            Fact::Cfg(item) => Some(item.clone()),
            _ => None,
        })
        .collect::<Vec<CfgItem>>();
    let crate_root = match (parts.iter().rev().nth(1).map(String::as_str), parts.last().map(String::as_str)) {
        (Some("src"), Some("lib.rs" | "main.rs")) => Within::In,
        _ => Within::Out,
    };
    let external = config
        .weights
        .as_ref()
        .and_then(|w| w.external.iter().find(|g| covers(g, &p.path).is_some()))
        .map_or(Within::Out, |_| Within::In);
    Env {
        facts: facts.clone(),
        lists: lists.clone(),
        names,
        operator_methods,
        cfg_items,
        tables: Tables::of(own),
        test_file: parts
            .iter()
            .find(|c| c.as_str() == "tests")
            .map_or(Within::Out, |_| Within::In),
        crate_root,
        external,
    }
}

/// The glob, relative to the config's directory, when it covers a path.
pub(crate) fn covers(pattern: &glob::Pattern, path: &Path) -> Option<()> {
    let options = glob::MatchOptions {
        require_literal_separator: true,
        ..glob::MatchOptions::new()
    };
    Some(()).filter(|_| pattern.matches_path_with(path, options))
}

/// The violations of one file and which occurrences they are.
fn judge(p: Parsed, hits: Vec<Hit>, config: &Config) -> Analysed {
    let relative = display(&p.path);
    let here = config
        .scopes
        .iter()
        .filter(|s| s.paths.iter().any(|g| covers(g, &p.path).is_some()))
        .collect::<Vec<&config::Scope>>();
    let elsewhere = config
        .scopes
        .iter()
        .filter(|s| !s.paths.iter().any(|g| covers(g, &p.path).is_some()))
        .collect::<Vec<&config::Scope>>();
    let occurrences = hits
        .iter()
        .filter_map(|h| match h {
            Hit::Id(o) => Some(o.clone()),
            _ => None,
        })
        .collect::<Vec<Occurrence>>();
    let findings = hits
        .iter()
        .filter_map(|h| match h {
            Hit::Flag(f) => Some(f),
            _ => None,
        })
        .filter(|f| allowed(config, &here, &table::row(f.rule).0).is_some())
        .collect::<Vec<&Finding>>();
    let found = findings
        .iter()
        .filter_map(|f| found_at(&occurrences, f))
        .collect::<BTreeSet<usize>>();
    let violating = occurrences
        .iter()
        .enumerate()
        .map(|(i, o)| match (allowed(config, &here, &o.id), found.get(&i)) {
            (Some(_), None) => Within::Out,
            _ => Within::In,
        })
        .collect();
    let plain = occurrences
        .iter()
        .filter(|o| allowed(config, &here, &o.id).is_none())
        .map(|o| Violation {
            path: relative.clone(),
            reason: elsewhere
                .iter()
                .find(|s| s.allowed.contains(&o.id))
                .map(|s| s.rule.clone())
                .or_else(|| table::forbidden(&o.id))
                .unwrap_or_else(|| o.reason.clone()),
            id: o.id.clone(),
            line: o.at.line,
            col: o.at.col,
        });
    let shaped = findings.iter().map(|f| {
        let (id, message) = table::row(f.rule);
        Violation {
            path: relative.clone(),
            id,
            reason: message,
            line: f.at.line,
            col: f.at.col,
        }
    });
    let violations = plain
        .chain(shaped)
        .enumerate()
        .map(|(i, v)| ((v.line, v.col, i), v))
        .collect::<BTreeMap<(usize, usize, usize), Violation>>()
        .into_values()
        .collect();
    Analysed {
        path: p.path,
        src: p.src,
        occurrences,
        violating,
        violations,
        hits,
    }
}

/// Whether the global list or a scope covering the file allows `id`.
fn allowed(config: &Config, here: &[&config::Scope], id: &str) -> Option<()> {
    config
        .allowed
        .get(id)
        .and(Some(()))
        .or(here.iter().find(|s| s.allowed.contains(id)).and(Some(())))
}

/// The occurrence a finding is about: the one of its id at its position,
/// or else the smallest one of its id that contains it.
fn found_at(occurrences: &[Occurrence], finding: &Finding) -> Option<usize> {
    let id = table::row(finding.rule).0;
    let same = occurrences
        .iter()
        .enumerate()
        .filter(|(_, o)| o.id == id);
    same.clone()
        .find(|(_, o)| (o.at.line, o.at.col) == (finding.at.line, finding.at.col))
        .or_else(|| {
            same.filter(|(_, o)| o.extent.start <= finding.at.byte && finding.at.byte < o.extent.end)
                .min_by_key(|(_, o)| o.extent.len())
        })
        .map(|(i, _)| i)
}

/// The boundary marks of a file's hits.
pub(crate) fn marks(hits: &[Hit]) -> Vec<&Mark> {
    hits.iter()
        .filter_map(|h| match h {
            Hit::Mark(m) => Some(m),
            _ => None,
        })
        .collect()
}

pub(crate) fn glob_files(pattern: &Path) -> Result<Vec<PathBuf>, Fatal> {
    let pattern = pattern.to_string_lossy();
    glob::glob(&pattern)
        .map_err(|e| Fatal(format!("include {pattern}: {e}")))?
        .map(|entry| entry.map_err(|e| Fatal(format!("include {pattern}: {e}"))))
        .filter(|entry| entry.as_ref().map_or(true, |p| p.is_file()))
        .collect()
}

pub(crate) fn read(path: &Path) -> Result<String, Fatal> {
    std::fs::read_to_string(path).map_err(|e| Fatal(format!("{}: {e}", display(path))))
}

pub(crate) fn display(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
