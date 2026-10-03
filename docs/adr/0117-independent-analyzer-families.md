---
id: ADR-0117
title: Independent latest Ruff owns canonical syntax alongside native Pyrefly typing and ty flow
status: accepted
date: 2026-10-03
supersedes: [ADR-0046]
superseded-by: null
design: [§1.2, §1.4, §B1, §B8, §3.2, §3.4, §3.4.1, §3.5, §3.6, §4.0, §4.1, §4.2, §11, §12, §13]
evidence: Interface-checked
revisit: A source/view attachment cannot distinguish role or node kind; an observational fork changes ordinary inference or lint results; a provider cannot co-resolve or repeatedly panics; verified analyzer-readable inputs change answers without changing context identity; or pinned uv acquisition stops verifying the input closure.
---

## Context

The operator selected independent latest Ruff rather than constraining it to Pyrefly's embedded
Ruff line. The expanded code-facts review identified populated contextual semantics and native
query observations that cannot be reached by the old shared-AST/visibility-only contract.
The [coordinator](../plans/code-facts-expansion-plan_2026-10-03.md) owns implementation and finding
closure. This decision replaces ADR-0046, retaining its acquisition, context identity, in-process
execution, panic and oracle clauses. The shared-parse ceiling and approximate patch line-count
trigger are rejected. Model-owned interpretation and question-specific admission remain required.

## Options

1. **Retain Pyrefly's embedded Ruff as the only syntax line.** Simplest dependency graph, but
   prevents independent Ruff improvements and populated contextual observation; rejected by the
   operator's chosen scope.
2. **Run three CLIs and decode reports.** Avoids private Rust APIs, but duplicates sessions,
   loses native types/roles, and adds report-format authorities and IPC; rejected.
3. **Independent canonical Ruff plus native Pyrefly and ty adapters** (chosen). It costs explicit
   source correspondence and narrow fork maintenance, while retaining upstream analysis and one
   semantic model. Nominal Rust AST types never cross provider family boundaries.

## Decision

**Libraries are data.** Every analyzed library, the pilot included, is a directory
`libraries/<name>/` with three committed files:
- `pyproject.toml`: a virtual uv project with exactly one pinned requirement, an exact
  `requires-python`, and `[tool.lctx] release`, the first-party distributions whose code is
  compiled. Everything else installed is dependency context, analyzed only as far as imports
  reach. `[tool.lctx.source]` names the upstream repository, tag and full 40-hex commit for docs,
  examples and tests; the tag must name the locked version. Corpus selection semantics are
  ADR-0018's; example and test roles are ADR-0015's.
- `.python-version`: the exact interpreter.
- `uv.lock`: the reviewed acquisition lock.

**The pilot and the gold reference.** The pilot is FastMCP 4.0.5 in `libraries/fastmcp`, installed
with the `fastmcp` skill's install line (`fastmcp[anthropic,openai,gemini,azure,apps,code-mode,tasks]==4.0.5`),
release `fastmcp`, `fastmcp-slim` and `fastmcp-tasks`. `scripts/check_gold.py` (`just gold`) fails
when the skill's install line or its resolved release versions differ from `libraries/fastmcp`, so
the gold reference and the analysis stay on one version. The skill is only ever an evaluation
reference, never a compiler input.

**Acquisition** is `lctx acquire <name>`: `uv sync --project libraries/<name> --frozen
--no-install-project --no-config --python <.python-version> --link-mode copy` into a gitignored,
rebuildable `build/envs/<name>`, with every other `UV_*` variable and `VIRTUAL_ENV` removed.
`--reinstall` is the remedy when Stage A finds a changed file. `lctx library init` locks without
`--no-config`, because the library's own `[tool.uv]` applies there and the lock is reviewed. When
a source is declared, `lctx acquire` also fetches the tree hermetically (no `GIT_*` variables,
system or global configuration, prompts or ambient attributes) as a shallow fetch of the one
commit, checked by `rev-parse HEAD` every time.

**Stage A: one equivalence, the analyzer-readable bytes.** The analyzer-readable files are `.py`,
`.pyi` and `py.typed`, the files Pyrefly's module finder reads; `.pth` files are never read.
Stage A reads the definition, the lock, `pyvenv.cfg` and every `*.dist-info`, with no network and
no interpreter, and refuses the attempt unless every installed distribution is the version the
lock names, the interpreter is `.python-version`'s, every distribution's analyzer-readable `RECORD`
entries match their sha256, and every release distribution has locked artifact hashes. The
release's modules are exactly the `.py`/`.pyi` entries of its distributions' `RECORD`s; nothing
walks a directory.

**Input and run identity** (encoding and content ids: ADR-0047).
- `release_id` hashes the release distributions' names and versions and the sorted (path, sha256)
  of their verified analyzer-readable entries: what is analyzed, not the lock entry. A source tree
  hashes its label. A corpus release hashes `repository@commit`, the library's `release_id`, and
  every selected file's path, content and role.
- `context_id` covers the Python version and platform, the ordered search and site-package paths
  relative to their roots, the digest of every configuration an analyzer receives, the
  **environment digest** (the installed distributions' dist-info names and every analyzer-readable
  file's site-relative path and content; `RECORD` lines outside site-packages never enter it) and
  the **lock digest**. Where a checkout or environment sits never changes an identity; what the
  analyzer can read must.
- `producer_id` covers the tool, its revision (for the extractor: the fork revision and patch
  digest, the ruff line, and the flow provider with its runtime-view version) and an adapter build
  digest bumped whenever mapping output changes.
- `run_id` covers release, context, producer, the sorted enabled families and the producer's own
  configuration digest.

**Runs in an attempt.** `releases`, `distributions` and `source_files` are written once per
extractor run, which carries Stage A's output; later producers reference `release_id` and never
append. An attempt holds several extractor runs only over distinct releases: the library, and the
corpus built from its upstream tree. The corpus run analyzes in the library's environment and
lists the same `distributions`, but has its own context, because its search path puts the tree
ahead of site-packages so that a module both runs import resolves to one file. What both runs
assert about one thing is one node, from its first fact.

**The front ends (accepted target, implementation tracked in M0–M3).**

- Latest independent Ruff owns canonical syntax, occurrences and contextual semantic observation.
  Ruff and ty share one exact fork/source revision. Pyrefly retains its own embedded Ruff family
  strictly inside its native adapter. The dependency decision is ADR-0118; exact pins are in
  `docs/pins.md`.
- One narrow aggregate patch on the exact upstream Pyrefly tag exposes existing collectors,
  no-write Pysa reporting, borrowed session queries, and query-scoped native terminal/exit
  observations. Native normal/exceptional results and applicability diagnostics are observed
  before upstream collapse; ordinary inference/hover/overload tracing remains unchanged.
- One narrow aggregate Ruff patch exposes an optional owned observer from the private Checker
  runner and sticky ty precision-loss provenance. Existing lint/inference algorithms, fallback
  thresholds and ordinary no-observer behavior remain upstream's. Observer cancellation and
  resource refusal report explicit Partial boundaries; the parser's own call is not interruptible.
- Patches may observe existing computations and expose owned provenance. Changing upstream
  inference, lint or algebra semantics requires another decision. Each fork remains tag plus
  one reviewed patch, immutable revision, checked digest, and paired native parity controls.
  Patch size is recorded as maintenance cost rather than an arbitrary rejection threshold.
- The driver constructs configuration from pinned inputs: Python version/platform, ordered
  search and site-package paths, selected rules and preview/target settings. No interpreter
  query, ambient configuration discovery or fallback environment is an input. Unsupported
  configuration is characterized explicitly rather than falsely reported as active behavior.
  The existing inline Pyrefly State/Require::Everything and no-write reporter stay installed
  while dependencies solve lazily; configured unsafe ambient variables are refused.
- Owned correspondence checks source snapshot/view, range, node kind, role, context and uniqueness.
  Equal byte ranges alone cannot equate a container with a formal or a synthetic slot. Synthetic
  native variants have optional source attachment and never invented locations or AST casts.
- ty supplies runtime-view use-def, timing, predicates and precision observations. Its
  same-length TYPE_CHECKING token view is explicit and joined to canonical source through the
  same correspondence contract. ty inference is an offline oracle, not a second production
  typing authority. Catalog Flow remains NotRequested.
- Public names and origins remain provider-attributed Pyrefly observations. Known entries may
  establish paths under partial export coverage; absence then cannot establish non-public status.
  Binding/source history and screening observations cannot independently prove runtime execution.
- Panics abort the attempt without catch_unwind; native Pyrefly types are encoded while the
  transaction is live. The matching Pyrefly CLI remains a parity oracle only.

**One production CLI:** `lctx` owns library init/acquire/compile/query and generation operations.
Library upgrades review the pinned uv lock and rebuild from inputs. No compatibility reader or
parallel semantic authority survives a completed migration.

## Consequences

The selected inputs can expand without tying Ruff's release schedule to Pyrefly. Correspondence,
configuration and observation parity now become explicit integration obligations. The existing
uv acquisition/input identity behavior is retained; earlier receipts keep their original scope.

**Interface-checked, 2026-10-03:** exact upstream sources and native-query/Checker contracts were
inspected during review and M0. Isolated compile/runtime controls establish only their named
seams; this ADR does not qualify production consumers. M1–M3, B0 and scheduled first operations
remain open in the coordinator until actual producer→consumer→output evidence and Q0 gates pass.
Real-library reconstruction and serving activation remain separately authorized follow-up.
