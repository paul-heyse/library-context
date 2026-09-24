# CrossHair 0.0.110 on Python 3.14 (P4), 2026-09-24

**Run.** `uv venv --python 3.14 .venv && uv pip install --python .venv/bin/python crosshair-tool==0.0.110`,
then `crosshair diffbehavior ops.eq_true ops.is_true --max_uninteresting_iterations 5` and the same
for `ops.eq_none ops.is_none`.

**Output (Python 3.14.7):**
```
Given: (x=1),
  ops.eq_true : returns True
  ops.is_true : returns False
Given: (x=<ops.Box object ...>),
  ops.eq_none : returns True, after execution x=<ops.Box object ...>
  ops.is_none : returns False, after execution x=<ops.Box object ...>
```

**Conclusion.** It runs on 3.14 and finds the operator differences the translator's canonical
forms erased. It executes code, so it runs isolated. Cited by the assessment (E8, E22) and forward
plan Stage 3.1.
