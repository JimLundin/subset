from pathlib import Path
import json
import subprocess

root = Path(__file__).resolve().parents[3]
project = root / 'target/issue5/conductor'
paths = ['crates/agents/src/bin/s1.rs', 'crates/client/src/host_view.rs', 'crates/daemon/tests/e2e.rs', 'crates/agents/src/install.rs']

def run(*args, cwd=root):
    return subprocess.check_output(args, cwd=cwd, text=True)

def source(path):
    return run('git', 'show', f'HEAD:{path}', cwd=project)

def replace_once(text, old, new):
    assert text.count(old) == 1, old
    return text.replace(old, new)

assert run('git', 'rev-parse', 'HEAD', cwd=project).strip() == 'dac5e9cab66b244c70725fe375112a59efdf6768'
before = {p: source(p) for p in paths}
after = dict(before)
p = paths[0]
helper = '''fn chosen(claude: &Path, resume: Option<SessionId>, stdout_log: Option<PathBuf>) -> ClaudeOptions {
    ClaudeOptions {
        model: Some(String::from(CHOSEN_MODEL)),
        effort: Some(CHOSEN_EFFORT),
        ..options(claude, resume, stdout_log)
    }
}

'''
after[p] = replace_once(after[p], helper, '')
after[p] = replace_once(after[p], ') -> Result<(Verdict, Verdict, Verdict)> {\n    let log', ''') -> Result<(Verdict, Verdict, Verdict)> {
    let chosen = |resume, stdout_log| ClaudeOptions {
        model: Some(String::from(CHOSEN_MODEL)),
        effort: Some(CHOSEN_EFFORT),
        ..options(claude, resume, stdout_log)
    };
    let log''')
after[p] = replace_once(after[p], 'chosen(claude, None, log)', 'chosen(None, log)')
after[p] = replace_once(after[p], 'chosen(\n        claude,\n', 'chosen(\n')

p = paths[1]
helper = '''    fn reasoned(piece: &str) -> EventKind {
        EventKind::ReasoningDelta { text: text(piece) }
    }

'''
after[p] = replace_once(after[p], helper, '')
for name in ['reasoning_streams_into_one_item_that_its_text_replaces_and_a_close_is_kept', 'a_close_settles_the_reasoning_or_reply_that_was_streaming']:
    after[p] = replace_once(after[p], f'    fn {name}() {{\n', f'    fn {name}() {{\n        let reasoned = |piece| EventKind::ReasoningDelta {{ text: text(piece) }};\n')

p = paths[2]
start = after[p].index('/// The thread, model and effort a `thread/start` or `thread/resume` names.')
end = after[p].index('const COUNT:', start)
after[p] = after[p][:start] + after[p][end:]
for name in ['a_codex_session_continues_on_the_default_when_a_new_credential_drops_its_model', 'a_killed_codex_server_continues_the_thread']:
    after[p] = replace_once(after[p], f'async fn {name}() {{\n', f'''async fn {name}() {{
    let thread_settings = |params: &serde_json::Value| (
        params.get("threadId").cloned(),
        params.get("model").cloned(),
        params.pointer("/config/model_reasoning_effort").cloned(),
    );
''')

p = paths[3]
start = after[p].index('async fn unpack(')
end = after[p].index('/// Streams', start)
body = after[p][start:end]
body = replace_once(body, ') -> Result<(), InstallError> {\n', ''') -> Result<(), InstallError> {
    let download_error = |error| io(download, error);
    let dir_error = |error| io(dir, error);
''')
body = body.replace('.map_err(|error| io(download, error))', '.map_err(download_error)')
body = body.replace('.map_err(|error| io(dir, error))', '.map_err(dir_error)')
after[p] = after[p][:start] + body + after[p][end:]
start = after[p].index('async fn sha256(')
end = after[p].index('async fn digest(', start)
body = after[p][start:end]
body = replace_once(body, ') -> Result<String, InstallError> {\n', ''') -> Result<String, InstallError> {
    let path_error = |error| io(path, error);
''')
body = body.replace('.map_err(|error| io(path, error))', '.map_err(path_error)')
after[p] = after[p][:start] + body + after[p][end:]

for phase, versions in [('before', before), ('after', after)]:
    for path, text in versions.items():
        output = root / 'target/issue5' / phase / path
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(text)
        if phase == 'after':
            (project / path).write_text(text)

# Format only the three changed files, then retain the exact formatted after.
run('rustfmt', '--edition', '2024', *paths, cwd=project)
for path in paths:
    (root / 'target/issue5/after' / path).write_text((project / path).read_text())

evidence = root / 'target/issue5/measurements.json'
binary = str(root / 'target/debug/subset')
measured = {}
for phase in ['before', 'after']:
    base = root / 'target/issue5' / phase
    config = base / 'subset.toml'
    config.write_text((root / 'subset.toml').read_text())
    measured[phase] = {
        'score': json.loads(run(binary, 'score', *paths, '--config', 'subset.toml', '--by', 'file', '--json', cwd=base)),
        'functions': json.loads(run(binary, 'score', *paths, '--config', 'subset.toml', '--by', 'fn', '--json', cwd=base)),
        'measure': json.loads(run(binary, 'measure', *paths, '--json', cwd=base)),
    }
    result = subprocess.run([binary, 'check'], cwd=base, text=True, capture_output=True)
    measured[phase]['check'] = {'exit': result.returncode, 'stdout': result.stdout, 'stderr': result.stderr}
evidence.write_text(json.dumps(measured, indent=2) + '\n')
print(evidence)
print(run('git', 'diff', '--stat', cwd=project))
