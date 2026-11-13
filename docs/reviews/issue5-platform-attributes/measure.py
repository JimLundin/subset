"""Reproduce issue #5 platform evidence from the published Conductor commit.

Run with Python on Linux/WSL. All generated build/source files stay under this
worktree's target/issue5-platform-evidence. The Conductor checkout is read only.
"""
import argparse
import difflib
import hashlib
import json
import os
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
COMMIT = '79568ce8a55b266fe1cf96b17dd5b2bc2dfa0c8c'
SUBSET_COMMIT = '64352fdd5935de3954ee06b4598789cb89f67a03'
SESSION = 'crates/agents/src/claude/session.rs'
PROCESS = 'crates/agents/src/process.rs'
MODULE = 'crates/agents/src/claude/mod.rs'
INSTALL = 'crates/agents/src/install.rs'


def run(args, cwd=None, env=None):
    result = subprocess.run([str(a) for a in args], cwd=cwd, env=env,
                            text=True, capture_output=True)
    return {'exit': result.returncode, 'stdout': result.stdout,
            'stderr': result.stderr}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--conductor', type=Path, default=Path('/home/jim/projects/apps/conductor'))
    parser.add_argument('--checker', type=Path, default=ROOT / 'target/debug/subset')
    parser.add_argument('--baseline-checker', type=Path)
    parser.add_argument('--compile', action='store_true')
    args = parser.parse_args()
    scratch = ROOT / 'target/issue5-platform-evidence'
    scratch.mkdir(parents=True, exist_ok=True)

    def source(path):
        result = run(['git', '-C', args.conductor, 'show', f'{COMMIT}:{path}'])
        if result['exit']:
            raise RuntimeError(result)
        return result['stdout']

    config = source('subset.toml')
    session, process, module, install = map(source, [SESSION, PROCESS, MODULE, INSTALL])
    files = run(['git', '-C', args.conductor, 'ls-tree', '-r', '--name-only', COMMIT])['stdout']
    inventory = []
    for path in files.splitlines():
        if path.startswith('crates/') and path.endswith('.rs'):
            for line_number, line in enumerate(source(path).splitlines(), 1):
                if any(marker in line for marker in ('#[cfg(', '#![cfg(', 'consts::OS', 'check_version')):
                    inventory.append({'path': path, 'line': line_number, 'source': line})
    (HERE / 'inventory.json').write_text(json.dumps(inventory, indent=2) + '\n')

    sandbox_start = session.index('pub fn sandbox() -> Sandbox {')
    sandbox_end = session.index('\n}\n', sandbox_start) + 3
    sandbox_before = session[sandbox_start:sandbox_end]
    tools = 'const SANDBOX_TOOLS: &[&str] = &["bwrap", "socat"];'
    sandbox_after = '''#[cfg(target_os = "linux")]
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
'''
    sandbox_full_after = session.replace(tools + '\n', '').replace(sandbox_before, sandbox_after)
    sandbox_context = '''use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sandbox {
    Available,
    Missing,
}

'''
    process_start = process.index('#[cfg(not(unix))]')
    process_end = process.index('\n/// Closes stdin', process_start)
    process_before = 'use tokio::process::Command;\n\n' + process[process_start:process_end]
    process_after = process_before.replace('#[cfg(not(unix))]', '#[cfg(target_os = "windows")]')
    process_after = process_after.replace('#[cfg(unix)]', '#[cfg(not(target_os = "windows"))]')
    process_full_after = process.replace('#[cfg(not(unix))]', '#[cfg(target_os = "windows")]')
    process_full_after = process_full_after.replace('#[cfg(unix)]', '#[cfg(not(target_os = "windows"))]')

    test_reexport = '#[cfg(test)]\npub(crate) use protocol::check_version;\n'
    import_before = 'use crate::claude::check_version;\n'
    import_after = 'use crate::claude::protocol::check_version;\n'
    module_after = module.replace('mod protocol;', 'pub(crate) mod protocol;').replace(test_reexport, '')
    install_after = install.replace(import_before.strip(), import_after.strip())
    candidates = {
        'sandbox-linux-pair': {
            'affected': (sandbox_context + tools + '\n\n' + sandbox_before,
                         sandbox_context + sandbox_after),
            'files': ({SESSION: session}, {SESSION: sandbox_full_after}),
            'friction_points': 1,
            'new_syntax': True,
        },
        'windows-no-window-pair': {
            'affected': (process_before, process_after),
            'files': ({PROCESS: process}, {PROCESS: process_full_after}),
            'friction_points': 1,
            'new_syntax': True,
        },
        'direct-test-module-access': {
            'affected': ('mod protocol;\n' + test_reexport + import_before,
                         'pub(crate) mod protocol;\n' + import_after),
            'files': ({MODULE: module, INSTALL: install},
                      {MODULE: module_after, INSTALL: install_after}),
            'friction_points': 1,
            'new_syntax': False,
        },
    }
    results = {'conductor_commit': COMMIT, 'subset_baseline_commit': SUBSET_COMMIT,
               'configuration_version': 14, 'checker_sha256': hashlib.sha256(args.checker.read_bytes()).hexdigest(),
               'proposed_tier': 'tolerated', 'proposed_weight': 2.0,
               'boundary_applies': False, 'candidates': {}}

    def measure_tree(tree):
        (tree / 'subset.toml').write_text(config)
        paths = sorted(str(p.relative_to(tree)) for p in (tree / 'crates').rglob('*.rs'))
        data = {}
        for key, command in (('score', ['score', *paths, '--by', 'file', '--json']),
                             ('constructs', ['score', *paths, '--by', 'construct', '--json']),
                             ('measure', ['measure', *paths, '--json'])):
            result = run([args.checker, *command], cwd=tree)
            if result['exit']:
                raise RuntimeError(result)
            data[key] = json.loads(result['stdout'])
        data['check'] = run([args.checker, 'check'], cwd=tree)
        cfg = next((r for r in data['constructs']['rows'] if r['name'] == 'attr.cfg'), {'tokens': 0})
        raw = data['score']['total']['score']
        outside_cfg_tokens = cfg['tokens'] if data['check']['exit'] and 'attr.cfg:' in data['check']['stdout'] else 0
        data['pricing'] = {'raw_score': raw, 'outside_cfg_tokens': outside_cfg_tokens,
                           'arithmetic': f'{raw} - (5.0 - 2.0) * {outside_cfg_tokens}',
                           'repriced_score': raw - 3.0 * outside_cfg_tokens}
        return data

    for name, candidate in candidates.items():
        measured = {'friction_points': candidate['friction_points'],
                    'new_syntax': candidate['new_syntax']}
        for shape in ('affected', 'files'):
            measured[shape] = {}
            for i, version in enumerate(('before', 'after')):
                tree = scratch / name / shape / version
                if shape == 'affected':
                    sources = {'crates/agents/src/claude/example.rs': candidate[shape][i]}
                    (HERE / f'{name}.{version}.rs').write_text(candidate[shape][i])
                else:
                    sources = candidate[shape][i]
                for path, text in sources.items():
                    destination = tree / path
                    destination.parent.mkdir(parents=True, exist_ok=True)
                    destination.write_text(text)
                measured[shape][version] = measure_tree(tree)
            before = measured[shape]['before']['pricing']['repriced_score']
            after = measured[shape]['after']['pricing']['repriced_score']
            measured[shape]['score_drop_percent'] = (before - after) / before * 100
        diffs = []
        before_files, after_files = candidate['files']
        for path in before_files:
            diffs += list(difflib.unified_diff(before_files[path].splitlines(keepends=True),
                                             after_files[path].splitlines(keepends=True),
                                             fromfile='a/' + path, tofile='b/' + path, n=0))
        (HERE / f'{name}.diff').write_text(''.join(diffs))
        results['candidates'][name] = measured

    regression = ROOT / 'tests/cases/cfg-scope'
    if (regression / 'subset.toml').exists():
        raise RuntimeError('unexpected regression config')
    reg_tree = scratch / 'regression'
    import shutil
    shutil.copytree(regression / 'crates', reg_tree / 'crates', dirs_exist_ok=True)
    (reg_tree / 'subset.toml').write_text(config)
    results['regression_after'] = run([args.checker, 'check'], cwd=reg_tree)
    if args.baseline_checker:
        results['regression_before'] = run([args.baseline_checker, 'check'], cwd=reg_tree)

    cases = {
        'unix_pair': ('src/lib.rs', '#[cfg(unix)]\nfn port() -> u8 { 1 }\n#[cfg(not(unix))]\nfn port() -> u8 { 2 }\n'),
        'wasm_pair': ('src/lib.rs', '#[cfg(target_family = "wasm")]\nfn port() -> u8 { 1 }\n#[cfg(not(target_family = "wasm"))]\nfn port() -> u8 { 2 }\n'),
        'missing_pair': ('src/lib.rs', '#[cfg(unix)]\nfn port() -> u8 { 1 }\n'),
        'target_os_pair': ('src/lib.rs', '#[cfg(target_os = "linux")]\nfn port() -> u8 { 1 }\n#[cfg(not(target_os = "linux"))]\nfn port() -> u8 { 2 }\n'),
        'feature': ('src/lib.rs', '#[cfg(feature = "web")]\nfn port() -> u8 { 1 }\n'),
        'compound': ('src/lib.rs', '#[cfg(all(unix, test))]\nfn port() -> u8 { 1 }\n'),
        'double_not': ('src/lib.rs', '#[cfg(not(not(unix)))]\nfn port() -> u8 { 1 }\n'),
        'statement': ('src/lib.rs', 'fn port() { #[cfg(unix)] let value = 1; }\n'),
        'expression': ('src/lib.rs', 'fn port() -> u8 { #[cfg(unix)] { 1 } }\n'),
        'struct': ('src/lib.rs', '#[cfg(unix)]\nstruct Port(u8);\n'),
        'production_root': ('src/lib.rs', '#![cfg(unix)]\nfn port() -> u8 { 1 }\n'),
        'unix_test_root': ('tests/test.rs', '#![cfg(unix)]\nfn port() -> u8 { 1 }\n'),
        'wasm_test_root_current_limit': ('tests/test.rs', '#![cfg(target_family = "wasm")]\nfn port() -> u8 { 1 }\n'),
        'test_file_unpaired': ('tests/test.rs', '#[cfg(unix)]\nfn port() -> u8 { 1 }\n'),
        'platform_only_module': ('src/lib.rs', '#[cfg(unix)]\nmod native {}\n'),
        'test_reexport': ('src/lib.rs', '#[cfg(test)]\npub(crate) use protocol::check_version;\nmod protocol { pub(crate) fn check_version() -> u8 { 1 } }\n'),
    }
    matrix = {}
    for name, (relative, text) in cases.items():
        tree = scratch / 'matrix' / name
        destination = tree / 'crates/agents' / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_text(text)
        (tree / 'subset.toml').write_text(config)
        matrix[name] = {'path': 'crates/agents/' + relative, 'source': text,
                        'check': run([args.checker, 'check'], cwd=tree)}
    results['checker_matrix'] = matrix

    if args.compile:
        compiled = {}
        for version in ('before', 'after'):
            source_path = HERE / f'sandbox-linux-pair.{version}.rs'
            harness = scratch / f'sandbox-{version}.rs'
            harness.write_text(source_path.read_text() + '\nfn main() { println!("{:?}", sandbox()); }\n')
            binary = scratch / f'sandbox-{version}'
            native = run(['rustc', '--edition=2024', '-A', 'unused_imports', harness, '-o', binary])
            wasm = run(['rustc', '--edition=2024', '--crate-name=sandbox_example', '-A', 'unused_imports', '--crate-type=lib',
                        '--target=wasm32-unknown-unknown', source_path, '-o', scratch / f'sandbox-{version}.rlib'])
            if native['exit'] or wasm['exit']:
                raise RuntimeError({'native': native, 'wasm': wasm})
            tests = {}
            for tools_present in ((), ('bwrap',), ('socat',), ('bwrap', 'socat')):
                directory = scratch / ('tools-' + '-'.join(tools_present))
                directory.mkdir(exist_ok=True)
                for tool in tools_present:
                    (directory / tool).write_text('fixture: is_file, not command execution\n')
                env = dict(os.environ, PATH=str(directory))
                tests['+'.join(tools_present) or 'none'] = run([binary], env=env)
            compiled[version] = {'native': native, 'wasm': wasm, 'linux_path_cases': tests}
        assert compiled['before']['linux_path_cases'] == compiled['after']['linux_path_cases']
        results['compile'] = compiled
    (HERE / 'measurements.json').write_text(json.dumps(results, indent=2) + '\n')
    for name, measured in results['candidates'].items():
        print(name, 'affected score drop:', round(measured['affected']['score_drop_percent'], 2), '%', flush=True)


if __name__ == '__main__':
    main()
