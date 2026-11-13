from pathlib import Path
import subprocess
import re
import difflib

ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / 'docs/reviews/issue5-comparisons-references'
source = (OUT / 'before/crates/agents/src/claude/protocol.rs').read_text()
exec((OUT / 'reproduce.py').read_text().split('paths = [')[0])
types = source[source.index('/// The CLI\'s suggestion to allow'):source.index('/// `AskUserQuestion`\'s input')]
impl = source[source.index('impl Rule {'):source.index('/// The session grant')]
grant = item(source, 'grant')
chain = grant[grant.index('    Some(rules)'):]
construct = chain[chain.index('        .map(|rules| Grant {')+len('        .map(|rules| '):]
construct = construct.rsplit('        })\n}', 1)[0] + '        }'
prefix = grant[:grant.index('    Some(rules)')]
slice_version = prefix + '    match rules.as_slice() {\n        [] => None,\n        _ => Some(' + construct.lstrip() + '),\n    }\n}\n'
manual_version = prefix + '    match Some(rules).filter(|rules| !rules.is_empty()) {\n        Some(rules) => Some(' + construct.lstrip() + '),\n        None => None,\n    }\n}\n'
variants = {'published': grant, 'slice': slice_version, 'manual-map': manual_version}
crate = ROOT / 'target/issue5-review/grant-check'
(crate / 'src').mkdir(parents=True, exist_ok=True)
(crate / 'Cargo.toml').write_text('''[package]
name = "issue5-grant-evidence"
version = "0.0.0"
edition = "2024"
[workspace]
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
''')
main = '''
fn main() {
    assert_eq!(grant(None), None);
    assert_eq!(grant(Some(serde_json::json!([]))), None);
    assert_eq!(grant(Some(serde_json::json!([{ "type": "setMode", "mode": "dontAsk" }]))), None);
    let offered = serde_json::json!([{ "type": "addRules", "rules": [{ "toolName": "Bash", "ruleContent": "git status" }], "behavior": "allow" }]);
    let value = grant(Some(offered)).unwrap();
    assert_eq!(value.label, "Bash(git status)");
    assert_eq!(serde_json::to_value(value.update).unwrap()["destination"], "session");
}
'''
for name, function in variants.items():
    path = OUT / 'grant' / f'{name}.rs'
    path.parent.mkdir(exist_ok=True)
    path.write_text('use serde::{Deserialize, Serialize};\nuse serde_json::Value;\n\n' + types + impl + function)
    (crate / 'src/main.rs').write_text(path.read_text() + main)
    logs = []
    for args in [['cargo', 'clippy', '--', '-D', 'warnings'], ['cargo', 'run', '--quiet']]:
        result = subprocess.run(args, cwd=crate, text=True, capture_output=True)
        logs.append('$ ' + ' '.join(args) + '\n' + result.stdout + result.stderr + f'Exit: {result.returncode}\n')
    (OUT / 'grant' / f'{name}-validation.txt').write_text('\n'.join(logs))
    print(name, logs[0][-500:])
    folder = crate / 'variants' / name
    target = folder / 'crates/agents/src/claude/protocol.rs'
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(source.replace(grant, function))
    config = (OUT / 'before/subset.toml').read_text()
    config = re.sub(r'(?m)^include = .*$', 'include = ["crates/agents/src/claude/protocol.rs"]', config)
    (folder / 'subset.toml').write_text(config)
    for label, args in [('check', ['check']), ('score', ['score', 'crates/agents/src/claude/protocol.rs', '--by', 'fn', '--json']), ('measure', ['measure', 'crates/agents/src/claude/protocol.rs', '--json'])]:
        result = subprocess.run([str(CHECKER), *args], cwd=folder, text=True, capture_output=True)
        (OUT / 'grant' / f'{name}-{label}.txt').write_text('$ subset ' + ' '.join(args) + '\n' + result.stdout + result.stderr + f'Exit: {result.returncode}\n')
(OUT / 'grant/supported-slice.diff').write_text(''.join(difflib.unified_diff(grant.splitlines(True), slice_version.splitlines(True), 'published/grant', 'slice/grant', n=0)))
