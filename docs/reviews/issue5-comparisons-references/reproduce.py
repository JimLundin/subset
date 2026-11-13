from pathlib import Path
import json
import os
import re
import subprocess
import difflib

ROOT = Path(__file__).resolve().parents[3]
SOURCE = ROOT / 'target/issue5-review/conductor'
OUT = ROOT / 'docs/reviews/issue5-comparisons-references'
CHECKER = Path(os.environ.get('SUBSET', str(ROOT / 'target/debug/subset')))
REV = '79568ce8a55b266fe1cf96b17dd5b2bc2dfa0c8c'

def item(text, name):
    match = re.search(r'(?m)^([ ]*)(?:pub(?:\([^)]*\))? )?(?:async )?fn ' + re.escape(name) + r'\b', text)
    start = match.start()
    brace = text.index('{', match.end())
    depth = 0
    for pos in range(brace, len(text)):
        depth += (text[pos] == '{') - (text[pos] == '}')
        if depth == 0:
            return text[start:pos+1] + '\n'
    raise ValueError(name)

paths = [
    'crates/daemon/src/log.rs',
    'crates/daemon/src/session.rs',
    'crates/daemon/src/registry.rs',
    'crates/proto/src/frame.rs',
    'crates/agents/src/claude/protocol.rs',
    'crates/client/src/answers.rs',
    'crates/agents/src/codex/config.rs',
]
before = {p: (SOURCE / p).read_text() for p in paths}
after = dict(before)
old = '''            LogMsg::Append {
                session,
                kind: EventKind::AssistantDelta { text },
            } => {
                let _ = self.live.send(Event {
                    seq: self.head(),
                    session,
                    kind: EventKind::AssistantDelta { text },
                });
            }
            LogMsg::Append {
                session,
                kind: EventKind::ReasoningDelta { text },
            } => {
                let _ = self.live.send(Event {
                    seq: self.head(),
                    session,
                    kind: EventKind::ReasoningDelta { text },
                });
            }'''
new = '''            LogMsg::Append {
                session,
                kind: kind @ (EventKind::AssistantDelta { .. } | EventKind::ReasoningDelta { .. }),
            } => {
                let _ = self.live.send(Event {
                    seq: self.head(),
                    session,
                    kind,
                });
            }'''
assert old in before[paths[0]]
after[paths[0]] = before[paths[0]].replace(old, new)
after[paths[1]] = before[paths[1]].replace('.find(|offered| offered == &&effort)', '.find(|&&offered| offered == effort)')
old = '''                    Some((provider, handle)) => self
                        .ready(provider.to_owned())
                        .map(|(launch, _)| {
                            (handle.clone(), launch, self.models(provider.to_owned()))
                        })
                        .map_err(|reason| Rejection::ProviderUnavailable { reason }),'''
new = '''                    Some((provider, handle)) => {
                        let &provider = provider;
                        self.ready(provider)
                            .map(|(launch, _)| (handle.clone(), launch, self.models(provider)))
                            .map_err(|reason| Rejection::ProviderUnavailable { reason })
                    }'''
assert old in before[paths[2]]
after[paths[2]] = before[paths[2]].replace(old, new)
after[paths[3]] = before[paths[3]].replace('use std::cmp::Ordering;\n\n', '').replace(
    'match u32::try_from(body.len()).map(|len| (len, len.cmp(&MAX_FRAME_LEN))) {',
    'match u32::try_from(body.len()) {'
).replace('Ok((len, Ordering::Less | Ordering::Equal))', 'Ok(len @ ..=MAX_FRAME_LEN)').replace(
    'Ok((_, Ordering::Greater)) | Err(_)', 'Ok(_) | Err(_)'
).replace('match len.cmp(&MAX_FRAME_LEN) {', 'match len {').replace(
    'Ordering::Less | Ordering::Equal =>', '..=MAX_FRAME_LEN =>'
).replace('Ordering::Greater =>', '_ =>')

OUT.mkdir(parents=True, exist_ok=True)
(OUT / 'revision.txt').write_text(f'Conductor {REV}\nsubset checker v14 pinned by Conductor: 64352fdd5935de3954ee06b4598789cb89f67a03\nsubset.toml version 14\n')
config = (SOURCE / 'subset.toml').read_text()
config = re.sub(r'(?m)^include = .*$', 'include = ' + json.dumps(paths[:4]), config)
for variant, files in [('before', before), ('after', after)]:
    folder = OUT / variant
    folder.mkdir(exist_ok=True)
    (folder / 'subset.toml').write_text(config)
    for path, contents in files.items():
        if variant == 'after' and path not in paths[:4]:
            continue
        dest = folder / path
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_text(contents)
    fragments = {
        'log-on': item(files[paths[0]], 'on'),
        'session-choose': item(files[paths[1]], 'choose'),
        'registry-on': item(files[paths[2]], 'on'),
        'frame-functions': ('use std::cmp::Ordering;\n\n' if variant == 'before' else '') + 'pub(crate) const MAX_FRAME_LEN: u32 = 16_777_216;\n\n' + item(files[paths[3]], 'encode') + '\n' + item(files[paths[3]], 'body_len'),
    }
    for name, contents in fragments.items():
        dest = folder / 'affected' / (name + '.rs')
        dest.parent.mkdir(exist_ok=True)
        dest.write_text(contents)
    for label, args in [
        ('check', ['check']),
        ('score-files', ['score', *paths[:4], '--by', 'file', '--json']),
        ('score-functions', ['score', *paths[:4], '--by', 'fn', '--json']),
        ('measure-files', ['measure', *paths[:4], '--json']),
        ('score-affected', ['score', 'affected', '--by', 'file', '--json']),
        ('measure-affected', ['measure', 'affected', '--json']),
    ]:
        result = subprocess.run([str(CHECKER), *args], cwd=folder, text=True, capture_output=True)
        (OUT / f'{variant}-{label}.txt').write_text('$ subset ' + ' '.join(args) + '\n' + result.stdout + result.stderr + f'Exit: {result.returncode}\n')

diff = ''.join(''.join(difflib.unified_diff(before[p].splitlines(True), after[p].splitlines(True), 'a/'+p, 'b/'+p, n=0)) for p in paths[:4])
(OUT / 'supported-rewrites.diff').write_text(diff)
print(diff)
