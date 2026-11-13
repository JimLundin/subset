from pathlib import Path
import hashlib
import json
import os
import signal
import subprocess

root = Path('/home/jim/projects/apps/conductor-issue5-v14')
target = root / 'crates/agents/src/install.rs'
audit = root / '.audit'
backup = audit / '0059-original-install.rs'
candidate = Path('/mnt/c/Users/Jim.Lundin/Projects/subset/target/worktrees/issue5-patterns/target/issue5-error-factory/after/crates/agents/src/install.rs')
expected = '12d32fa7cac9e78ce7510d1ee5f09f0e60b674a81ca8f6022efdf0c4332ab78c'

def digest(data):
    return hashlib.sha256(data).hexdigest()

def diff():
    return subprocess.check_output(['git', 'diff', '--binary', 'HEAD'], cwd=root)

original = target.read_bytes()
original_diff = diff()
assert digest(original) == expected, ('baseline mismatch; no mutation', digest(original), expected)
assert candidate.is_file(), candidate
replacement = candidate.read_bytes()
backup.write_bytes(original)
(audit / '0059-original.diff').write_bytes(original_diff)
record = {
    'python_pid': os.getpid(),
    'cargo_pid': None,
    'root': str(root),
    'target': str(target),
    'backup': str(backup),
    'candidate': str(candidate),
    'original_install_sha256': digest(original),
    'original_diff_sha256': digest(original_diff),
    'candidate_install_sha256': digest(replacement),
    'phase': 'backed_up_before_mutation',
}

def checkpoint():
    (audit / '0059-process.json').write_text(json.dumps(record, indent=2) + '\n')

def interrupted(signum, frame):
    raise InterruptedError(f'interrupted by signal {signum}')

signal.signal(signal.SIGTERM, interrupted)
signal.signal(signal.SIGINT, interrupted)
checkpoint()
print(json.dumps(record), flush=True)
process = None
try:
    target.write_bytes(replacement)
    record['phase'] = 'candidate_installed'
    checkpoint()
    environment = dict(os.environ)
    environment['PATH'] = '/home/jim/.local/bin:/home/jim/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin'
    environment['CARGO_BUILD_JOBS'] = '1'
    environment.pop('CARGO_TARGET_DIR', None)
    with (audit / '0059-cargo-check.log').open('wb') as output:
        process = subprocess.Popen(
            ['cargo', 'check', '-p', 'agents', '--all-targets'],
            cwd=root,
            env=environment,
            stdout=output,
            stderr=subprocess.STDOUT,
            start_new_session=True,
        )
        record['cargo_pid'] = process.pid
        record['phase'] = 'checking_candidate'
        checkpoint()
        print(f'cargo PID {process.pid}; backup {backup}', flush=True)
        record['cargo_exit_code'] = process.wait()
except BaseException as error:
    record['error'] = repr(error)
    raise
finally:
    try:
        if process is not None and process.poll() is None:
            try:
                os.killpg(process.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait()
    finally:
        target.write_bytes(original)
        record['restored_install_sha256'] = digest(target.read_bytes())
        record['restored_diff_sha256'] = digest(diff())
        record['install_restored'] = record['restored_install_sha256'] == record['original_install_sha256']
        record['full_diff_restored'] = record['restored_diff_sha256'] == record['original_diff_sha256']
        record['phase'] = 'restored'
        checkpoint()
        (audit / '0059-result.json').write_text(json.dumps(record, indent=2) + '\n')
        print(json.dumps(record), flush=True)
        assert record['install_restored'], record
        assert record['full_diff_restored'], record
