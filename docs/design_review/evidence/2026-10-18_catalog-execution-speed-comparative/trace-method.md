# Reproducing the retained-trace arithmetic

**2026-10-18.** Supports the [evidence notes](README.md) and [principal review](../../reviews/design_review_catalog-execution-speed-comparative-target_2026-10-18.md).
This is a small read-only audit of the named single native invocation, not a generic causal-trace
analyzer. It emits a compact textual result and changes no files. The original analysis also
inspected JUnit, historical source and the raw line anchors recorded in README.

Run from the repository root with its selected interpreter:

```sh
uv run --no-sync python - <<'PY'
from pathlib import Path
import datetime
import re

p = Path('build/runs/20261018T044128.645Z-1d2be6/'
         'verify-evidence/b653540a2b744e58a4c8cc85df3ce91a/native.log')
begins, events, retries = {}, [], []
for line in p.read_text().splitlines():
    stamp = re.search(r'(2026-\S+Z)', line)
    if not stamp:
        continue
    t = datetime.datetime.fromisoformat(stamp[1]).timestamp()
    context = line.split('lctx_phase:')[0]
    m = re.search(r'phase="([^"]+)" phase_id=(\d+) status="([^"]+)"', line)
    if m:
        name, ident, status = m.groups()
        if status == 'begin':
            begins[ident] = t
        else:
            elapsed = float(re.search(r'elapsed_ms=([\d.]+)', line)[1]) / 1000
            events.append(dict(name=name, a=begins.pop(ident, t-elapsed),
                               b=t, dt=elapsed, context=context))
    if 'native transaction terminal' in line:
        fields = {k: v.strip('"') for k, v in
                  re.findall(r'(\w+)=("[^"]*"|[^\s:]+)',
                             line.split('lctx_phase:')[-1])}
        retries.append(fields)

def union(intervals):
    merged = []
    for a, b in sorted(intervals):
        if merged and a <= merged[-1][1]:
            merged[-1] = (merged[-1][0], max(b, merged[-1][1]))
        else:
            merged.append((a, b))
    return sum(b-a for a, b in merged)

compile_end = next(e['b'] for e in events if e['name'] == 'cn02_compile')
for label, cutoff in [('whole-helper', float('inf')), ('compile-ended', compile_end)]:
    for name in ('native_effect_apply', 'native_effect_recovery_settlement',
                 'native_immutable_window', 'native_product_retention',
                 'compiler_completion_gate'):
        es = [e for e in events if e['name'] == name and e['b'] <= cutoff]
        print(label, name, len(es), sum(e['dt'] for e in es),
              union([(e['a'], e['b']) for e in es]))

stages = []
for e in events:
    m = re.search(r'compiler_producer\{producer="([^"]+)"', e['context'])
    if m and m[1] == e['name']:
        stages.append(e)
covered = 0
for s in stages:
    intervals = [
        (max(s['a'], e['a']), min(s['b'], e['b']))
        for e in events
        if e['name'] in ('native_call_admission', 'native_call_acknowledgement',
                         'compiler_completion_gate')
        and e['a'] < s['b'] and e['b'] > s['a']
        and f'compiler_producer{{producer="{s["name"]}"' in e['context']
    ]
    covered += union(intervals)
total = sum(s['dt'] for s in stages)
print('stage sum, covered, residual:', total, covered, total-covered)
print('stage wall union:', union([(s['a'], s['b']) for s in stages]))
print('retry calls, full elapsed, retries:', len(retries),
      sum(float(r['elapsed_ms'])/1000 for r in retries),
      sum(int(r['retries']) for r in retries))
print('unclosed begins:', len(begins))
PY
```

The cutoff is **completed phases**, not clipped occupancy; README states that distinction.
Timestamp unions and emitted elapsed counters use different clock observations and may differ
slightly due to formatting/rounding. The script's stage-context matching reproduces the external
calculation; it does not infer true causality, CPU utilization, conflicting keys or unobserved
operation-page submissions. Unclosed begins remain unclosed. Do not reuse this single-invocation
parser as a multi-process identity contract or infer success from a missing terminal.
