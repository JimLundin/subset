"""Reproduce pinned Conductor callback evidence; only target/ is mutated.

Run from the Subset root with Python 3 on Linux/WSL:
python3 docs/reviews/issue5-callbacks-actors/measure.py --conductor /path/to/conductor --subset /path/to/subset
The source checkout may be ahead: every source read names COMMIT explicitly.
"""
from pathlib import Path
import argparse
import difflib
import io
import json
import os
import re
import subprocess
import tarfile
import tomllib

COMMIT = '79568ce8a55b266fe1cf96b17dd5b2bc2dfa0c8c'
ROOT = Path(__file__).resolve().parents[3]
EVIDENCE = Path(__file__).resolve().parent
SCRATCH = ROOT / 'target/issue5-callbacks'
INSTALL = 'crates/agents/src/install.rs'
CLAUDE = 'crates/agents/src/claude/session.rs'
APP = 'crates/app/src/session.rs'
SESSION = 'crates/daemon/src/session.rs'


def run(*args, cwd=ROOT):
    result = subprocess.run(args, cwd=cwd, text=True, capture_output=True)
    return {'command': list(map(str, args)), 'exit': result.returncode,
            'stdout': result.stdout, 'stderr': result.stderr}


def once(text, old, new):
    assert text.count(old) == 1, old
    return text.replace(old, new)


def region(text, start, end, transform):
    first = text.index(start)
    last = text.index(end, first)
    return text[:first] + transform(text[first:last]) + text[last:]


def variants(before):
    local = before[INSTALL]
    def unpack(body):
        body = once(body, ') -> Result<(), InstallError> {\n',
                    ') -> Result<(), InstallError> {\n'
                    '    let download_error = |error| io(download, error);\n'
                    '    let dir_error = |error| io(dir, error);\n')
        return body.replace('.map_err(|error| io(download, error))', '.map_err(download_error)').replace(
            '.map_err(|error| io(dir, error))', '.map_err(dir_error)')
    local = region(local, 'async fn unpack(', '/// Streams', unpack)
    def sha256(body):
        body = once(body, ') -> Result<String, InstallError> {\n',
                    ') -> Result<String, InstallError> {\n    let path_error = |error| io(path, error);\n')
        return body.replace('.map_err(|error| io(path, error))', '.map_err(path_error)')
    local = region(local, 'async fn sha256(', 'async fn digest(', sha256)

    returned = re.sub(r'\.map_err\(\|error\| io\(([^\n]+?), error\)\)',
                      r'.map_err(io_error(\1))', before[INSTALL])
    returned = once(returned, 'fn io(path: &Path, error: std::io::Error) -> InstallError {',
                    "fn io_error(path: &Path) -> impl FnOnce(std::io::Error) -> InstallError {\n"
                    '    move |error| io(path, error)\n}\n\n'
                    'fn io(path: &Path, error: std::io::Error) -> InstallError {')

    from_fn = once(before[CLAUDE],
                   '''let errors = Box::pin(n0_future::stream::unfold(events, |events| async move {
        events.try_recv().ok().map(|event| (event, events))
    }))
    .filter_map(|event| match event {''',
                   '''let errors = std::iter::from_fn(|| events.try_recv().ok())
    .find_map(|event| match event {''')
    from_fn = region(from_fn, 'async fn refusal(', 'async fn stderr_reason(',
                     lambda body: once(body, '    })\n    .next()\n    .await;', '    });'))

    free_ui = before[APP]
    for decision in ['Deny', 'Approve', 'ApproveForSession']:
        free_ui = once(free_ui,
                       '{ on_decide((request, Decision::' + decision + ')); document::eval(&focus_on(request)); }',
                       'decide_and_focus(on_decide, request, Decision::' + decision + ')')
    free_ui = once(free_ui, "/// A request card's element id", '''fn decide_and_focus(
    on_decide: EventHandler<(RequestId, Decision)>,
    request: RequestId,
    decision: Decision,
) {
    on_decide((request, decision));
    document::eval(&focus_on(request));
}

/// A request card's element id''')

    actor = once(before[SESSION], '''        // The first order starts the agent, so the actor's loop does it all.
        let (reply, _) = oneshot::channel();
        let (launch, offered) = launch;
        let _ = inbox.try_send(SessionMsg {
            order: Order::Start(launch, offered),
            reply,
        });
        tokio::spawn(actor.run(messages));''',
                 '        tokio::spawn(actor.run(messages, launch));')
    actor = once(actor,
                 'async fn run(mut self, mut inbox: mpsc::Receiver<SessionMsg>) -> Result<Infallible, Stopped> {\n        loop {',
                 '''async fn run(
        mut self,
        mut inbox: mpsc::Receiver<SessionMsg>,
        launch: (Launch, Models),
    ) -> Result<Infallible, Stopped> {
        let (launch, offered) = launch;
        let _ = self.order(Order::Start(launch, offered)).await?;
        loop {''')

    blocking = once(before[INSTALL], 'use std::fs::{File, TryLockError};', 'use std::fs::File;')
    blocking = once(blocking, '/// How often a process waiting for another\'s install checks the lock.\nconst LOCK_POLL: Duration = Duration::from_millis(200);\n', '')
    blocking = once(blocking, '    hold(&lock, &lock_path).await?;', '    let _lock = hold(lock, &lock_path).await?;')
    blocking = region(blocking, '/// Waits until this process holds the lock', '/// Removes everything', lambda _: '''/// Waits until this process holds the lock, returning the file that releases it.
async fn hold(lock: File, path: &Path) -> Result<File, InstallError> {
    tokio::task::spawn_blocking(move || {
        lock.lock()?;
        Ok::<File, std::io::Error>(lock)
    })
    .await
    .map_err(|error| io(path, std::io::Error::other(error)))?
    .map_err(|error| io(path, error))
}

''')
    return {'local': {INSTALL: local}, 'returned': {INSTALL: returned},
            'from-fn': {CLAUDE: from_fn}, 'ui-free': {APP: free_ui},
            'actor-setup': {SESSION: actor}, 'blocking': {INSTALL: blocking}}


def main():
    os.environ['PATH'] = str(Path.home() / '.cargo/bin') + os.pathsep + os.environ['PATH']
    parser = argparse.ArgumentParser()
    parser.add_argument('--conductor', required=True, type=Path)
    parser.add_argument('--subset', required=True, type=Path)
    parser.add_argument('--subset-revision', default='64352fdd5935de3954ee06b4598789cb89f67a03')
    parser.add_argument('--archive', action='store_true', help='Materialize compile project from pinned git archive')
    parser.add_argument('--no-project', action='store_true', help='Only refresh measurements/evidence')
    args = parser.parse_args()
    paths = [INSTALL, CLAUDE, APP, SESSION]
    before = {p: subprocess.check_output(['git', '-C', str(args.conductor), 'show', f'{COMMIT}:{p}'], text=True) for p in paths}
    for path, content in before.items():
        output = EVIDENCE / 'pinned' / path
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(content)
    versions = variants(before)
    results = {'source_commit': COMMIT, 'subset_revision': args.subset_revision, 'config_version': 14}
    config = (ROOT / 'subset.toml').read_text()
    for phase, changed in [('before', before), *versions.items()]:
        base = SCRATCH / phase
        for path, content in changed.items():
            output = base / path
            output.parent.mkdir(parents=True, exist_ok=True)
            output.write_text(content)
        (base / 'subset.toml').write_text(config)
        formatting = run('rustfmt', '--edition', '2024', *changed, cwd=base)
        assert formatting['exit'] == 0, formatting
        paths_phase = list(changed)
        data = {}
        for label, command in [('files', ['score', *paths_phase, '--by', 'file', '--json']),
                               ('functions', ['score', *paths_phase, '--by', 'fn', '--json']),
                               ('constructs', ['score', *paths_phase, '--by', 'construct', '--json']),
                               ('measure', ['measure', *paths_phase, '--json']), ('check', ['check', *paths_phase])]:
            output = run(str(args.subset), *command, cwd=base)
            if label != 'check':
                assert output['exit'] == 0, output
                output['data'] = json.loads(output['stdout'])
            data[label] = output
        results[phase] = data
        for path in paths_phase:
            evidence_path = EVIDENCE / phase / path
            evidence_path.parent.mkdir(parents=True, exist_ok=True)
            evidence_path.write_text((base / path).read_text())
        if phase != 'before':
            diff = ''.join(''.join(difflib.unified_diff(
                (SCRATCH/'before'/path).read_text().splitlines(keepends=True),
                (base/path).read_text().splitlines(keepends=True),
                fromfile='a/'+path, tofile='b/'+path)) for path in paths_phase)
            (EVIDENCE / (phase + '.diff')).write_text(diff)
    # Reprice exactly the tokens owned by unsupported nodes. Inline library
    # arguments are Fixed::IfRestricted, so their tolerated tier activates
    # the existing boundary price after the violation is removed.
    tiers = tomllib.loads(config)['weights']['tiers']
    from_fn = results['from-fn']
    outside = [row for row in from_fn['constructs']['data']['rows'] if row['tier'] == 'outside']
    assert len(outside) == 1 and outside[0]['name'] == 'expr.closure'
    charged = outside[0]['tokens']
    assert charged == 2
    before_fn = next(row for row in results['before']['functions']['data']['rows'] if row['name'].endswith(' refusal'))
    after_fn = next(row for row in from_fn['functions']['data']['rows'] if row['name'].endswith(' refusal'))
    adjustment = (tiers['outside'] - tiers['boundary']) * charged
    admitted = after_fn['score'] - adjustment
    results['repricing'] = {'from-fn': {
        'proposed_tier': 'tolerated', 'boundary_applies': True,
        'reason': 'Direct library CallArg: library_value -> Fixed::IfRestricted -> score::judge boundary',
        'outside_charged_tokens': charged, 'outside_weight': tiers['outside'],
        'boundary_weight': tiers['boundary'], 'adjustment': adjustment,
        'affected_before_score': before_fn['score'], 'affected_raw_score': after_fn['score'],
        'affected_admitted_score': admitted,
        'raw_percent_reduction': 100 * (before_fn['score'] - after_fn['score']) / before_fn['score'],
        'admitted_percent_reduction': 100 * (before_fn['score'] - admitted) / before_fn['score'],
        'whole_file_admitted_score': from_fn['files']['data']['total']['score'] - adjustment,
    }}
    (EVIDENCE / 'measurements.json').write_text(json.dumps(results, indent=2) + '\n')
    if args.no_project:
        print(EVIDENCE / 'measurements.json')
        return
    # The combined rewrite is compilation evidence only, not accepted policy.
    project = SCRATCH / 'conductor'
    if args.archive:
        data = subprocess.check_output(['git', '-C', str(args.conductor), 'archive', COMMIT])
        project.mkdir(parents=True, exist_ok=True)
        with tarfile.open(fileobj=io.BytesIO(data)) as archive:
            archive.extractall(project, filter='data')
    assert (project / 'Cargo.toml').exists(), 'Pass --archive to create the compile project'
    combined = dict(before)
    combined.update(versions['blocking'])
    combined.update(versions['from-fn'])
    combined.update(versions['ui-free'])
    combined.update(versions['actor-setup'])
    # Apply the returned mapping experiment to a separate compile copy later.
    for path, content in combined.items():
        (project/path).write_text(content)
    print(EVIDENCE / 'measurements.json')


if __name__ == '__main__':
    main()
