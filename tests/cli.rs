//! Drives the `subset` binary over the cases in `tests/cases` and the docs.
//!
//! Each case is a directory of files laid out as in a project. It is checked
//! with its own `subset.toml`, or the repository's when it has none. `cmd`
//! lists the commands to run, one per line (`check` when there is none), and
//! `expected` holds what they print. `SUBSET_BLESS=1 cargo test` rewrites
//! `expected` from the binary's output.

use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// A fresh scratch directory.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("subset-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch directory");
    dir
}

/// Copies the files of `from` into `to`, recursively.
fn copy(from: &Path, to: &Path) {
    let entries = std::fs::read_dir(from).expect("case directory");
    for entry in entries {
        copy_entry(&entry.expect("directory entry").path(), to);
    }
}

fn copy_entry(path: &Path, to: &Path) {
    let target = to.join(path.file_name().expect("file name"));
    let _ = match Some(path).filter(|p| p.is_dir()) {
        Some(dir) => std::fs::create_dir_all(&target).map(|_| copy(dir, &target)),
        None => std::fs::copy(path, &target).and(Ok(())),
    };
}

/// The binary under test: the `subset` in the nearest directory above this
/// test's own executable that holds one.
fn binary() -> PathBuf {
    let exe = std::env::current_exe().expect("test executable");
    exe.ancestors()
        .map(|dir| dir.join("subset"))
        .find(|path| path.is_file())
        .expect("the subset binary")
}

/// What `subset args` prints when run in `dir`: its exit code, its output
/// and any error output.
fn run(dir: &Path, args: &[&str]) -> String {
    let out = Command::new(binary())
        .args(args)
        .current_dir(dir)
        .output()
        .expect("run subset");
    let code = out
        .status
        .code()
        .map_or("signal".to_string(), |c| c.to_string());
    let stderr = String::from_utf8_lossy(&out.stderr);
    let errors = match stderr.as_ref() {
        "" => String::new(),
        text => format!("stderr:\n{text}"),
    };
    format!(
        "exit {code}\n{}{errors}",
        String::from_utf8_lossy(&out.stdout)
    )
}

/// Text with Windows line ends read as Unix ones.
fn lf(text: &str) -> String {
    text.replace("\r\n", "\n")
}

fn read(path: &Path) -> String {
    lf(&std::fs::read_to_string(path).unwrap_or_default())
}

/// Runs one case; a difference from `expected` is its name.
fn case(name: &str) -> Option<String> {
    let case = root().join("tests/cases").join(name);
    let dir = scratch(&format!("case-{name}"));
    copy(&case, &dir);
    let _ = std::fs::remove_file(dir.join("cmd"));
    let _ = std::fs::remove_file(dir.join("expected"));
    let config = dir.join("subset.toml");
    let _ = Some(&config)
        .filter(|c| !c.is_file())
        .map(|c| std::fs::copy(root().join("subset.toml"), c));
    let commands = match read(&case.join("cmd")).as_str() {
        "" => "check\n".to_string(),
        text => text.to_string(),
    };
    let got = commands
        .lines()
        .map(|c| {
            format!(
                "$ {c}\n{}",
                run(&dir, &c.split_whitespace().collect::<Vec<&str>>())
            )
        })
        .collect::<String>();
    let blessed = std::env::var("SUBSET_BLESS")
        .ok()
        .map(|_| std::fs::write(case.join("expected"), &got));
    match (blessed, read(&case.join("expected"))) {
        (Some(_), _) => None,
        (None, expected) if expected == got => None,
        (None, _) => Some(format!("{name}:\n{got}")),
    }
}

#[test]
fn every_case_prints_what_it_expects() {
    let mut names = std::fs::read_dir(root().join("tests/cases"))
        .expect("tests/cases")
        .filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().to_string()))
        .collect::<Vec<String>>();
    names.sort();
    let failures = names
        .iter()
        .filter_map(|n| case(n))
        .collect::<Vec<String>>();
    assert_eq!(failures, Vec::<String>::new());
}

#[test]
fn the_checker_passes_its_own_subset() {
    assert_eq!(run(&root(), &["check"]), "exit 0\n");
}

/// The README holds the output of `subset command`.
fn readme_lists(command: &str) -> String {
    let table = run(&root(), &[command]);
    let readme = read(&root().join("README.md"));
    let listed = readme.contains(table.trim_start_matches("exit 0\n"));
    format!("{command}: {listed}")
}

#[test]
fn the_readme_lists_every_id_rule_and_score_only_id() {
    assert_eq!(
        Vec::from([
            readme_lists("ids"),
            readme_lists("rules"),
            readme_lists("score-ids")
        ]),
        Vec::from(["ids: true", "rules: true", "score-ids: true"])
    );
}

/// The rust blocks of `docs/examples.md`, each placed where its first-line
/// comment names, or as a file of an examples crate.
fn examples() -> Vec<(String, String)> {
    let text = read(&root().join("docs/examples.md"));
    text.split("```rust\n")
        .skip(1)
        .enumerate()
        .map(|(n, block)| {
            let code = block.split("```").next().unwrap_or_default().to_string();
            let path = code
                .lines()
                .next()
                .and_then(|first| first.strip_prefix("// "))
                .filter(|p| p.starts_with("crates/"))
                .map_or(format!("crates/examples/src/e{n}.rs"), ToString::to_string);
            (path, code)
        })
        .collect()
}

#[test]
fn every_example_passes_the_subset() {
    let dir = scratch("examples");
    let files = examples();
    std::fs::copy(root().join("subset.toml"), dir.join("subset.toml")).expect("config");
    for (path, code) in &files {
        write(&dir.join(path), code);
    }
    assert_eq!(
        (files.len(), run(&dir, &["check"])),
        (40, "exit 0\n".to_string())
    );
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
    std::fs::write(path, text).expect("file");
}

#[derive(Debug, PartialEq)]
struct RefusalMeasure {
    score: f64,
    tokens: u64,
    lines: u64,
}

/// Reprice the complete retained Conductor function, including its library
/// closure boundary, rather than measuring a simplified example.
#[test]
fn synchronous_receiver_drain_preserves_the_accepted_measurement() {
    assert_eq!(
        refusal_measure("before"),
        RefusalMeasure {
            score: 131.0,
            tokens: 143,
            lines: 18,
        }
    );
    assert_eq!(
        refusal_measure("from-fn"),
        RefusalMeasure {
            score: 100.5,
            tokens: 117,
            lines: 13,
        }
    );
}

fn refusal_measure(phase: &str) -> RefusalMeasure {
    let dir = scratch(&format!("receiver-drain-{phase}"));
    let path = "crates/agents/src/claude/session.rs";
    let retained = root()
        .join("docs/reviews/issue5-callbacks-actors")
        .join(phase)
        .join(path);
    write(&dir.join(path), &read(&retained));
    std::fs::copy(root().join("subset.toml"), dir.join("subset.toml")).expect("config");
    assert_eq!(run(&dir, &["check"]), "exit 0\n", "{phase}");
    let output = run(&dir, &["score", path, "--by", "fn", "--json"]);
    let data = serde_json::from_str::<serde_json::Value>(
        output.strip_prefix("exit 0\n").expect("score succeeded"),
    )
    .expect("function measurements");
    let row = data
        .get("rows")
        .and_then(serde_json::Value::as_array)
        .expect("measurement rows")
        .iter()
        .find(|row| {
            row.get("name")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|name| name.ends_with(" refusal"))
        })
        .expect("complete refusal function");
    RefusalMeasure {
        score: row
            .get("score")
            .and_then(serde_json::Value::as_f64)
            .expect("score"),
        tokens: row
            .get("tokens")
            .and_then(serde_json::Value::as_u64)
            .expect("tokens"),
        lines: row
            .get("lines")
            .and_then(serde_json::Value::as_u64)
            .expect("lines"),
    }
}

/// Rust supplies type, ownership and exhaustiveness checks beyond syn.
#[test]
fn permitted_patterns_compile_and_refutable_lengths_need_fallbacks() {
    let dir = scratch("compiled-patterns");
    compile_pattern(
        "tests/cases/array-receiver/crates/daemon/src/a.rs",
        &dir,
        0,
        "",
    );
    compile_pattern("tests/cases/ident-at/crates/daemon/src/a.rs", &dir, 0, "");
    compile_pattern(
        "tests/cases/slice-pattern/crates/daemon/src/a.rs",
        &dir,
        0,
        "",
    );
    compile_pattern("tests/compile/slice-missing-fallback.rs", &dir, 1, "E0004");
    compile_pattern(
        "tests/compile/slice-rest-missing-empty.rs",
        &dir,
        1,
        "E0004",
    );
    compile_pattern(
        "tests/compile/slice-guard-missing-fallback.rs",
        &dir,
        1,
        "E0004",
    );
    compile_pattern("tests/compile/at-moved-fields.rs", &dir, 1, "E0382");
}

fn compile_pattern(path: &str, dir: &Path, code: i32, diagnostic: &str) {
    let out = Command::new(std::env::var("RUSTC").unwrap_or("rustc".to_string()))
        .arg(root().join(path))
        .args([
            "--edition=2024",
            "--crate-type=lib",
            "--crate-name=pattern_case",
            "--emit=metadata",
        ])
        .arg("--out-dir")
        .arg(dir)
        .output()
        .expect("compile pattern fixture");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(code), "{path}: {stderr}");
    assert_eq!(
        stderr.find(diagnostic).map(|_| diagnostic),
        Some(diagnostic),
        "{path}: {stderr}"
    );
}

#[test]
fn cfg_owner_imports_local_types_and_reordered_binders_compile() {
    let dir = scratch("cfg-owners");
    let positive = read(&root().join("tests/cases/cfg-scope/crates/daemon/src/a.rs"));
    let (_, tail) = positive
        .split_once("mod owners {")
        .expect("self-contained owner cases");
    let source = dir.join("owners.rs");
    write(&source, &format!("mod owners {{{tail}"));
    compile_pattern(source.to_str().expect("fixture path"), &dir, 0, "");
}

#[test]
fn range_patterns_compile_with_exhaustive_and_irrefutable_positions() {
    let dir = scratch("compiled-ranges");
    compile_pattern(
        "tests/cases/pattern-range/crates/daemon/src/a.rs",
        &dir,
        0,
        "",
    );
    compile_pattern("tests/compile/range-missing-negative.rs", &dir, 1, "E0004");
    compile_pattern(
        "tests/compile/range-missing-guard-fallback.rs",
        &dir,
        1,
        "E0004",
    );
    compile_pattern("tests/compile/range-refutable-binding.rs", &dir, 1, "E0005");
    compile_pattern(
        "tests/compile/range-refutable-parameter.rs",
        &dir,
        1,
        "E0005",
    );
    compile_pattern(
        "tests/compile/range-nonnumeric-constant.rs",
        &dir,
        1,
        "E0029",
    );
    compile_pattern(
        "tests/compile/range-character-missing-fallback.rs",
        &dir,
        1,
        "E0004",
    );
    compile_pattern("tests/compile/range-reversed.rs", &dir, 1, "E0030");
}

#[test]
fn immutable_models_and_structural_patterns_preserve_types_and_ownership() {
    let dir = scratch("compiled-domain-models");
    compile_pattern(
        "tests/cases/domain-data/crates/daemon/src/a.rs",
        &dir,
        0,
        "",
    );
    compile_pattern(
        "tests/cases/structural-pattern/crates/daemon/src/a.rs",
        &dir,
        0,
        "",
    );
    compile_pattern("tests/compile/array-wrong-cardinality.rs", &dir, 1, "E0308");
    compile_pattern("tests/compile/array-moved.rs", &dir, 1, "E0382");
    compile_pattern("tests/compile/slice-output-dangling.rs", &dir, 1, "E0515");
    compile_pattern("tests/compile/slice-output-ambiguous.rs", &dir, 1, "E0106");
    compile_pattern("tests/compile/generic-wrong-identity.rs", &dir, 1, "E0308");
    compile_pattern("tests/compile/shared-pattern-move.rs", &dir, 1, "E0507");
}

#[test]
fn boundary_forms_preserve_wire_and_error_semantics() {
    let dir = scratch("boundary-data");
    copy(&root().join("tests/cases/boundary-data"), &dir);
    boundary_tests(&dir);
    write(
        &dir.join("crates/agents/src/codex/pins.toml"),
        "provider = \"claude\"\n",
    );
    let source = dir.join("crates/agents/src/codex/protocol.rs");
    write(
        &source,
        &read(&source).replace("provider = \\\"codex\\\"", "provider = \\\"claude\\\""),
    );
    boundary_tests(&dir);
}

fn boundary_tests(dir: &Path) {
    let out = Command::new(std::env::var("CARGO").unwrap_or("cargo".to_string()))
        .args(["test", "--offline", "--locked", "--quiet"])
        .arg("--manifest-path")
        .arg(dir.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(root().join("target/issue6-additions/boundary-build"))
        .output()
        .expect("run boundary fixture tests");
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .output()
        .expect("run git");
    String::from_utf8_lossy(&out.stderr).to_string()
}

#[test]
fn report_html_writes_one_static_page_with_history() {
    let dir = scratch("report-html");
    copy(&root().join("tests/cases/score-attribution"), &dir);
    let _ = std::fs::remove_file(dir.join("cmd"));
    let _ = std::fs::remove_file(dir.join("expected"));
    write(&dir.join("other/main.rs"), "fn main() { loop {} }\n");
    git(&dir, &["init", "-q"]);
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-q", "-m", "one"]);
    write(&dir.join("lights/src/lib.rs"), "enum Light { On, Off }\n");
    git(&dir, &["commit", "-q", "-am", "two"]);
    let printed = run(
        &dir,
        &[
            "report-html",
            "--out",
            "site",
            "--compare",
            "other",
            "--history",
            "5",
        ],
    );
    let html = read(&dir.join("site/index.html"));
    let missing = Vec::from([
        "Comparison",
        "Score per crate at each of the last 2 commits",
        "Files by score",
        "Constructs by score",
        "lights",
    ])
    .into_iter()
    .filter(|s| !html.contains(s))
    .collect::<Vec<&str>>();
    let scripts = html.matches("<script").count();
    assert_eq!(
        (printed.as_str(), scripts, missing),
        ("exit 0\n", 0, Vec::<&str>::new())
    );
}

/// The strings of a TOML list.
fn strings(value: Option<&toml::Value>) -> Vec<String> {
    value
        .and_then(toml::Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(toml::Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<String>>()
        })
        .unwrap_or_default()
}

/// A new proposal can't forget its weight: every id the repository's config
/// allows, and every score-only id, has an explicit tier.
#[test]
fn every_allowed_id_has_a_weight() {
    let config = read(&root().join("subset.toml"))
        .parse::<toml::Table>()
        .expect("subset.toml");
    let scoped = config
        .get("scope")
        .and_then(toml::Value::as_array)
        .map(|scopes| {
            scopes
                .iter()
                .flat_map(|s| strings(s.get("allowed")))
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();
    let score_only = run(&root(), &["score-ids"])
        .lines()
        .filter_map(|l| l.strip_prefix("| `"))
        .filter_map(|l| l.split('`').next())
        .filter(|id| id != &"field.wire")
        .map(ToString::to_string)
        .collect::<Vec<String>>();
    let weighted = config
        .get("weights")
        .and_then(|w| w.get("ids"))
        .and_then(toml::Value::as_table);
    let missing = strings(config.get("allowed"))
        .into_iter()
        .chain(scoped)
        .chain(score_only)
        .filter(|id| weighted.filter(|w| w.contains_key(id.as_str())).is_none())
        .collect::<Vec<String>>();
    assert_eq!(missing, Vec::<String>::new());
}
