"""Measure the real installer after 0046, without editing Conductor.

Run on Linux/WSL: python3 reproduce.py --subset /path/to/v14/subset
Compute-only rustc/clippy-driver validation requires no Cargo or network.
"""
from pathlib import Path
import argparse
import difflib
import hashlib
import json
import os
import re
import subprocess

ROOT = Path(__file__).resolve().parents[3]
OUT = Path(__file__).resolve().parent
SCRATCH = ROOT / 'target/issue5-error-factory'
INSTALL = 'crates/agents/src/install.rs'
REV = '79568ce8a55b266fe1cf96b17dd5b2bc2dfa0c8c'
FACTORY = '''fn io_error(path: &Path) -> impl FnOnce(std::io::Error) -> InstallError {
    move |error| io(path, error)
}

'''

def run(args, cwd=SCRATCH):
    result = subprocess.run(list(map(str, args)), cwd=cwd, capture_output=True, text=True)
    return dict(command=list(map(str, args)), exit=result.returncode, stdout=result.stdout, stderr=result.stderr)

def function(source, name):
    first = re.search(r'(?m)^(?:pub )?(?:async )?fn '+re.escape(name)+r'\b', source).start()
    opening = source.index('{', first)
    depth = 0
    for pos in range(opening, len(source)):
        depth += (source[pos] == '{') - (source[pos] == '}')
        if depth == 0:
            return source[first:pos+1]+'\n'
    raise ValueError(name)

def locals_0046(source):
    first = source.index('async fn unpack(')
    last = source.index('/// Streams', first)
    part = source[first:last].replace(') -> Result<(), InstallError> {\n', ') -> Result<(), InstallError> {\n    let download_error = |error| io(download, error);\n    let dir_error = |error| io(dir, error);\n', 1)
    part = part.replace('.map_err(|error| io(download, error))', '.map_err(download_error)').replace('.map_err(|error| io(dir, error))', '.map_err(dir_error)')
    source = source[:first]+part+source[last:]
    first = source.index('async fn sha256(')
    last = source.index('async fn digest(', first)
    part = source[first:last].replace(') -> Result<String, InstallError> {\n', ') -> Result<String, InstallError> {\n    let path_error = |error| io(path, error);\n', 1)
    part = part.replace('.map_err(|error| io(path, error))', '.map_err(path_error)')
    return source[:first]+part+source[last:]

def factory_version(before):
    after, count = re.subn(r'\.map_err\(\|error\| io\(([^\n]+?), error\)\)', r'.map_err(io_error(\1))', before)
    assert count == 9, count
    for name, path in [('download_error','download'), ('dir_error','dir'), ('path_error','path')]:
        declaration = f'    let {name} = |error| io({path}, error);\n'
        assert after.count(declaration) == 1
        after = after.replace(declaration, '').replace(f'.map_err({name})', f'.map_err(io_error({path}))')
    assert after.count('.map_err(io_error(') == 16
    return after.replace('fn io(path: &Path, error: std::io::Error) -> InstallError {', FACTORY+'fn io(path: &Path, error: std::io::Error) -> InstallError {')

def patch(before, after, path):
    return ''.join(difflib.unified_diff(before.splitlines(True), after.splitlines(True), 'a/'+path, 'b/'+path, n=0))

def main():
    os.environ['PATH'] = str(Path.home()/'.cargo/bin')+os.pathsep+os.environ.get('PATH','')
    parser = argparse.ArgumentParser()
    parser.add_argument('--conductor', default='/home/jim/projects/apps/conductor')
    parser.add_argument('--consumer', default='/home/jim/projects/apps/conductor-issue5-v14')
    parser.add_argument('--subset', required=True)
    args = parser.parse_args()
    SCRATCH.mkdir(parents=True, exist_ok=True)
    pinned = subprocess.check_output(['git','-C',args.conductor,'show',REV+':'+INSTALL], text=True)
    before = locals_0046(pinned)
    consumer = (Path(args.consumer)/INSTALL).read_text()
    after = factory_version(before)
    for name, source in [('pinned',pinned), ('before',before), ('after',after), ('consumer',consumer)]:
        folder = SCRATCH/name
        target = folder/INSTALL
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(source)
        formatted = run(['rustfmt','--edition','2024',target])
        assert formatted['exit'] == 0, formatted
        if name == 'before': before = target.read_text()
        if name == 'after': after = target.read_text()
        if name == 'consumer': consumer = target.read_text()
    assert before == consumer, 'consumer installer differs from pinned+0046; reassess baseline'
    pinned = (SCRATCH/'pinned'/INSTALL).read_text()
    (OUT/'baseline-local.diff').write_text(patch(pinned,before,INSTALL))
    (OUT/'factory.diff').write_text(patch(before,after,INSTALL))
    sites = [{'line':i,'source':line.strip()} for i,line in enumerate(before.splitlines(),1) if re.search(r'\.map_err\(\|error\| io\(',line)]
    io_uses = [{'line':i,'source':line.strip()} for i,line in enumerate(before.splitlines(),1) if re.search(r'\bio\(',line)]
    results = {'conductor_commit':REV,'subset_commit':'64352fdd5935de3954ee06b4598789cb89f67a03','subset_version':14,'baseline_sha256':hashlib.sha256(before.encode()).hexdigest(),'remaining_inline_sites':sites,'all_io_textual_uses_before':io_uses,'measurements':{},'validation':{}}
    config = subprocess.check_output(['git','-C',args.conductor,'show',REV+':subset.toml'],text=True)
    config = re.sub(r'(?m)^include = .*$', 'include = ["'+INSTALL+'"]',config)
    names = ['put','remove_others','remove','unpack','fetch','drain','sha256','io']
    for name,source in [('before',before),('after',after)]:
        folder = SCRATCH/name
        (folder/'subset.toml').write_text(config)
        commands = [('check',['check']),('score-file',['score',INSTALL,'--by','file','--json']),('score-fn',['score',INSTALL,'--by','fn','--json']),('score-construct',['score',INSTALL,'--by','construct','--json']),('measure',['measure',INSTALL,'--json'])]
        metrics = {}
        for label,arguments in commands:
            result = run([args.subset,*arguments],folder)
            if label == 'check':
                metrics[label] = result
            else:
                parsed = json.loads(result['stdout'])
                metrics[label] = dict(command=result['command'],exit=result['exit'],total=parsed['total'])
                if label == 'score-fn':
                    metrics[label]['affected'] = [row for row in parsed['rows'] if any(row['name'].endswith(' '+func) for func in names+['io_error'])]
                if label == 'score-construct':
                    metrics[label]['outside'] = [row for row in parsed['rows'] if row.get('tier') == 'outside']
        results['measurements'][name] = metrics
    # Use the exact Io variant fields and original constructor; derives/messages
    # are irrelevant to borrow/error propagation and require external crates.
    harness = '''use std::path::{Path, PathBuf};
#[derive(Debug)]
enum InstallError { Io { path: String, error: std::io::Error } }
'''+function(before,'io')+'\n'+FACTORY
    # Both complete real synchronous consumer functions, with only tracing's
    # log statement removed; no fake filesystem or Result API substitutes.
    for name in ['remove','remove_others']:
        body = function(after,name)
        body = re.sub(r'(?m)^    tracing::info!.*\n','',body)
        harness += '\n'+body
    harness += (OUT/'validation-tests.rs').read_text()
    testfile = SCRATCH/'validation.rs'
    testfile.write_text(harness)
    validations = [('compile',['rustc','--edition','2024','--test','-D','warnings',str(testfile),'-o',str(SCRATCH/'validation')]), ('clippy',['clippy-driver','--edition','2024','--test','-D','warnings',str(testfile),'-o',str(SCRATCH/'validation-clippy')]), ('behavior',[str(SCRATCH/'validation'),'--test-threads=1'])]
    for label,command in validations:
        result = run(command)
        results['validation'][label] = result
        assert result['exit'] == 0,result
    for label,tail in {
        'reuse': '''fn reuse(path: &Path) { let mapping = io_error(path); let _: Result<(),_> = Err(std::io::Error::other("first")).map_err(mapping); let _: Result<(),_> = Err(std::io::Error::other("second")).map_err(mapping); }\nfn main() {}\n''',
        'escape': '''fn escape() -> impl FnOnce(std::io::Error) -> InstallError { let path = PathBuf::from("gone"); io_error(&path) }\nfn main() {}\n''',
    }.items():
        negative = SCRATCH/(label+'.rs')
        negative.write_text('use std::path::{Path,PathBuf};\n#[derive(Debug)] enum InstallError { Io { path:String,error:std::io::Error } }\n'+function(before,'io')+'\n'+FACTORY+tail)
        result = run(['rustc','--edition','2024',str(negative),'-o',str(SCRATCH/label)])
        results['validation'][label] = result
        assert result['exit'] != 0,result
    results['validation']['rustc-version'] = run(['rustc','--version'])
    (OUT/'results.json').write_text(json.dumps(results,indent=2)+'\n')
    print('remaining unresolved:',len(sites))
    for name,metrics in results['measurements'].items():
        selected = metrics['score-fn']['affected']
        print(name,'file',metrics['score-file']['total'],'affected',{'score':sum(r['score'] for r in selected),'tokens':sum(r['tokens'] for r in selected),'lines':sum(r['lines'] for r in selected)})
    print('compile/behavior passed; reuse/escape correctly rejected')

if __name__ == '__main__': main()
