# Follow-up: actual kernel decisions, support and catalog hydration

Date: 2026-09-25. Reviewed source: `92eca2e8244a82f574be48e3124855d14f6b8241`.

`probe.rs` compiles the unchanged repository `condition_kernel`, `condition`, `id` and
`codebook` modules. It uses the existing release-profile dependency artifacts for pinned
biodivine-lib-bdd 0.6.3, blake3 1.8.6 and serde_json 1.0.151. `run.py` prints source SHA-256s,
dependency artifact names and the pinned rustc version. Only the small harness is compiled,
using Clang/mold and a temporary executable; no Cargo config, target directory, source module
or production build is changed. A missing cached dependency is a named prerequisite.

```sh
uv run --no-sync python -B docs/design_review/evidence/2026-09-25_library-fit-followup-kernel/run.py > docs/design_review/evidence/2026-09-25_library-fit-followup-kernel/raw/output.txt 2>&1
```

Outcome: **passed**, exit 0. This is a successful diagnostic reproduction, not a conformance
pass for the defects. See [output.txt](raw/output.txt) and [probe.rs](probe.rs).

**Tested, 2026-09-25:**

- Folding 128 contradictory atoms leaves the live false diagram with support 128. Hydration
  yields the same semantic ID with support 0. Adding a new atom is `AtomLimit` in the live
  instance and succeeds in the hydrated instance. This confirms source-review F02.
- Two admitted 512-node operands have a 130,562-node conjunction. The current kernel returns
  `NodeLimit` for compatibility; upstream `check_binary_op` returns `(true, 130560)`.
- An admitted 8,192-node diagram and its complement produce `WorkPreflight`; the upstream
  bounded decision returns `(false, 8190)`. A task-limit-10 control returns `None`.
  These close the source review's oversized-operand evidence gap for F01. Admission goes
  through the real validated node-hydration boundary; the fixture export itself is not a
  replacement implementation of the kernel.
- A valid catalog with 256 distinct roots and 320 shared nonterminal rows hydrates into 17,152
  independently owned BDD nodes, including each diagram's terminals. This demonstrates
  expansion beyond stored-row count. It does not measure bytes, peak RSS, startup time, OOM,
  or pilot incidence. The new review's aggregate hydration-budget finding is source-grounded
  plus this bounded count observation.

An initial harness run used illegal upstream variable-name punctuation and failed before its
decision cases. The harness now uses neutral local names and separately exports the canonical
atom encodings. Production code was unaffected. The retained output is the corrected complete run.

`just test-all`, `just pilot`, actual native-generation startup at scale and performance
comparison against OxiDD: **not_run**. This review establishes no integrated qualification.
