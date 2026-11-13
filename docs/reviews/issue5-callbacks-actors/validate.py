"""Compile/test the actual from_fn rewrite and same harness on its before.

Requires the pinned project materialized by measure.py --archive.
This script is deliberately separate so source measurement does not start
heavy builds. Every build uses the owning project's isolated target/.
"""
from pathlib import Path
import argparse
import json
import os
import subprocess

ROOT = Path(__file__).resolve().parents[3]
EVIDENCE = Path(__file__).resolve().parent
PROJECT = ROOT / 'target/issue5-callbacks/conductor'
PATHS = ['crates/agents/src/install.rs', 'crates/agents/src/claude/session.rs',
         'crates/app/src/session.rs', 'crates/daemon/src/session.rs']


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--phase', choices=['before', 'after'], default='after')
    parser.add_argument('--jobs', type=int, default=1)
    args = parser.parse_args()
    assert (PROJECT / 'Cargo.toml').exists(), 'Run measure.py --archive first'
    for path in PATHS:
        phase = 'from-fn' if args.phase == 'after' and path == PATHS[1] else 'before'
        (PROJECT / path).write_text((EVIDENCE / phase / path).read_text())
    source = PROJECT / PATHS[1]
    source.write_text(source.read_text() + '\n' + (EVIDENCE / 'validation-tests.rs').read_text())
    environment = dict(os.environ)
    environment['PATH'] = os.pathsep.join([str(Path.home() / '.cargo/bin'), str(Path.home() / '.local/bin'), environment['PATH']])
    environment['CARGO_BUILD_JOBS'] = str(args.jobs)
    environment['CARGO_TARGET_DIR'] = str(PROJECT / 'target')
    commands = [
        ['cargo', 'check', '-p', 'agents', '--all-targets'],
        ['cargo', 'test', '-p', 'agents', '--lib', 'issue5_from_fn_review_tests'],
        ['cargo', 'test', '-p', 'agents', '--test', 'replay', 'fails_with'],
    ]
    results = {'phase': args.phase, 'jobs': args.jobs, 'source_commit': '79568ce8a55b266fe1cf96b17dd5b2bc2dfa0c8c', 'commands': []}
    process_record = ROOT / 'target/issue5-callbacks/validation-process.json'
    process_record.write_text(json.dumps({'pid': os.getpid(), 'script': str(Path(__file__).resolve()), 'project': str(PROJECT), 'phase': args.phase}, indent=2) + '\n')
    print('Validation process PID:', os.getpid(), flush=True)
    destination = EVIDENCE / ('validation-' + args.phase + '.json')
    for index, command in enumerate(commands):
        log = ROOT / 'target/issue5-callbacks' / f'validation-{args.phase}-{index}.log'
        print(' '.join(command), flush=True)
        with log.open('w') as output:
            result = subprocess.run(command, cwd=PROJECT, env=environment, text=True, stdout=output, stderr=subprocess.STDOUT)
        results['commands'].append({'command': command, 'exit': result.returncode, 'output': log.read_text()})
        destination.write_text(json.dumps(results, indent=2) + '\n')
        if result.returncode != 0:
            print(log, flush=True)
            raise SystemExit(result.returncode)
    print(destination, flush=True)


if __name__ == '__main__':
    main()
