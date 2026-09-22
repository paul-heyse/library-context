# Design review: ADR-0012, Pyrefly and Ruff linked in-process (standard)

**Date:** 2026-09-22 · **Depth:** standard · **Mode:** document plus spike code. ADR-0012 changes
§B1, §B2 and §B8, so a `standard` review is owed (ADR-0001).
**Reviewer:** `design-reviewer` subagent (fresh context) · **Author:** the session that ran spike
`spike/pyrefly-inproc` and wrote commit `ea248fc`
**Prior review:** `design_review_capability-compiler-design_2026-09-22.md` (standard, Not Accept).
Its F4 (one definition of "public") and F10 (Pyrefly's ambient inputs) bear on this change.

## 1. Decision and scope

**Proposal.** ADR-0012 (`proposed`, `evidence: Tested`) supersedes ADR-0006. It makes four
changes:
- Pyrefly 1.3.1 is linked in-process from a fork: tag `3e3177d0` plus
  `third_party/pyrefly-1.3.1.patch`, which is fork commit `b9f28575`.
- The Ruff crates are pinned to `=0.0.11`, so the Ruff walk runs over Pyrefly's own parse.
- Pyrefly's Pysa collectors and public-name helpers replace the JSON report decoders, and
  Delta built-ins (CHECK constraints, `appendOnly`) take over part of storage-boundary
  validation.
- The Pyrefly CLI stays only as a parity-test oracle.

**Target.**
- ADR-0012 itself.
- The DESIGN amendments in `ea248fc`: §B1, §B2, §B8; §3.2–§3.5, §4.0, §4.1; §4.2.1–§4.2.6;
  §4.3; §6.1, §6.2, §8 and §13.
- The other changed files: `docs/pins.md` (Analyzers), the ADDENDUM §B1/§B8 rows, DISPOSITION,
  and `third_party/pyrefly-1.3.1.patch`.
- The evidence: worktree `/home/paul/lc-spike-pyrefly` at `d00bab5`.

**Status of the claims.** DESIGN labels §B1, §B8 and §4.2 **Tested**, and S7 in §4.2.1
**Measured**. §4.3 is **Interface-checked**, with some rows marked Tested. No production code
exists: `crates/` holds only `cpg-schema`, which is a doc-comment `lib.rs` and a smoke test. So
"Tested" here means "exercised by the spike", and that is what this review checks, line by line.

**Observable outcome.** One analysis run, one parse and one byte coordinate system. No decoder for
undocumented JSON. An upstream enum change fails our build instead of degrading silently.

**Baseline.** ADR-0006:
- Ruff 0.0.13 in-process;
- the Pyrefly CLI as a subprocess, run twice (`check --report-pysa` and `coverage report`);
- the JSON decoded in Rust;
- two parses.

**Supported scope.** Increment-1 extraction for the `exports`, `signatures` and `calls` families
on FastMCP 4.0.3, plus the Delta write and read path in §4.3.

**Constraints.** One operator. The operator asked for in-process linking (ADR-0012 Context), and
this review takes that direction as given. It judges the contract, not the preference.

### Method and coverage

**Read in full:**
- ADR-0012 and ADR-0006;
- the commit diff of DESIGN.md and of every other file in `ea248fc`;
- DESIGN §1, §B6–§B8, §3–§4.3, the §5 header, §6, §8 and §9.1;
- the charter, directive, template, ADDENDUM and REVIEW_REFERENCE;
- the prior review's §1, §6, F4, F10 and §11;
- `analysis/SPIKE_RESULTS.md`;
- the spike code: `crates/spike-pyrefly/src/main.rs` (all 834 lines), `tests/s6_delta.rs` and
  `analysis/scripts/parity.py`;
- the run outputs: `summary.json` for runs a–e and `fixture-1`, `fastmcp-cli/parity.json`,
  `coverage-public.json` and `context.json`.

**Read at the grain cited: the patched clone `vendor/pyrefly`** (HEAD `b9f2857` on `3e3177d`):
- `crates/pyrefly_util/src/{panic,lock,thread_pool,trace}.rs`;
- `pyrefly/lib/export/{exports,definitions}.rs` (`__all__` handling);
- `pyrefly/lib/commands/coverage/collect.rs` (`compute_public_fqns`);
- `pyrefly/lib/report/pysa.rs`, `report/pysa/call_graph.rs`, `report/pysa/types.rs`;
- `pyrefly/lib/state/load.rs`;
- `pyrefly/lib/alt/answers_solver.rs` (cycle placeholders, `PYREFLY_FIXPOINT_DETAILS`);
- `crates/pyrefly_config` (the `ConfigFile` serde-skip fields, `search_path()`, `PYTHONPATH`);
- an `env::var` sweep of `pyrefly/lib` and `crates/`.

**Read at the grain cited: delta-rs at `58f07cd`.** `crates/core/src/writer/` and
`operations/mod.rs`, for constraint handling.

**Checks run in this session:**

| Check | Command | Outcome | Observation |
|---|---|---|---|
| S6 Delta probes | `cargo test -p spike-pyrefly --test s6_delta -- --nocapture` (worktree) | passed (3/3) | CHECK is enforced on `write(batches)` and on `with_input_plan`. `appendOnly` rejects a delete. `INSERT INTO` committed one CHECK-violating row; the test prints this but doesn't assert it. Ids read back as `BinaryView`. The stored constraint text is normalized: `origin >= 0 AND origin <= 4` |
| S4/S5 recomputation | a reviewer script over `runs/fastmcp-a/*.arrow` and `fastmcp-cli/coverage-public.json` (`uv run --no-project --with pyarrow`) | passed | 188 unmatched calls, **all** inside parameter, return or `AnnAssign` annotations; the spike recorded only 15 examples. No Pysa regular call range lacks a `call_syntax` row. 13,104 ranges, none under two callers. CLI `--public-only`: 2,790 symbols, of which 976 match exactly and 1,814 only by a public parent prefix; 0 unexplained |
| Determinism record | `summary.json` of runs a–e | passed | The five table digests and the context digest are identical across reruns, shuffle, perturbed ambient variables and 4 threads |
| Patch identity | `git -C vendor/pyrefly format-patch -1 HEAD --stdout`, piped to `sha256sum` | passed | `b7b82828…b767`, equal to `third_party/pyrefly-1.3.1.patch` and to pins.md |
| Reviewer probe: `__all__` forms, parse error, non-UTF-8 | the spike release binary on a 5-module package (§9) | ran | exit 0 in every case; results in F3 and F5 |
| Reviewer probe: tree location | the spike binary on `lcfix` at two absolute paths | ran | tables identical, context digest different (F9) |
| Reviewer probe: file effects | `strace -f -e trace=openat,creat,mkdir,mkdirat,rename,renameat,unlink,unlinkat,execve` on the `lcfix` run | passed | No write outside `--out`, and one `execve`. No config, ignore or `typings` file is opened. The only other reads are `/sys/fs`, `/proc/self` and libc |
| ADR metadata | `uv run --no-sync python scripts/adr.py lint` | passed | 12 records |
| Agent docs | `uv run --no-sync python scripts/check_agents.py` | passed | — |
| `just check`, `just test-all` | — | not_run | The working tree has uncommitted `pyproject.toml`/`uv.lock` changes that add `fastmcp` and `vllm` (operator work outside this ADR), and `uv run` would sync them. No ADR-0012 code is in the main workspace, so nextest would exercise only `cpg-schema` |

**Asserted only, not inspected or attacked:**
- the S1 build and the `cargo deny` result (not rebuilt);
- the S7 timings (not re-measured);
- memory co-residence of Pyrefly state with DataFusion;
- the provenance of the spike venv, which was built with `uv` rather than from an acquisition
  lock as §4.0 requires;
- blake3 id derivation (§3.4.1, unchanged);
- the `lsp-types` git source;
- the §B2 dependency change.

The panic analysis in F1 rests on Pyrefly's documented contract, not on a reproduced failure.

## 2. Authority and lifecycle (reconstructed)

| Concept or fact | Authority | Other representations | Reconciliation | Gap |
|---|---|---|---|---|
| What "public" means | Pyrefly `compute_public_fqns` + `trace_export_origin` at `b9f28575` | our copy of its name-selection rule (spike `main.rs` L359–L386), producing (access path, origin) pairs | the flattened pairs must equal `compute_public_fqns`, or the run fails (L391–L396) | the copy and the authority share the `__all__` fallback, so the check can't see it (F3) |
| Call targets | Pyrefly's Pysa collectors | raw `pysa_calls`; derived `call_targets` through the full-range join | CLI parity, which runs the same collectors | the mapping from variant to phase and modality is undecided (F2) |
| Syntax facts | the Ruff 0.0.11 walk over `Transaction::get_ast` | — | one parse, so none is needed | — |
| Codebook ranges | `cpg-schema` codebooks (append-only) | the Delta `delta.constraints.*` entries, written once at table creation | none | F4 |
| What our fork changes | fork rev `b9f28575`, the one that is built | `third_party/pyrefly-1.3.1.patch`, with its sha256 in pins.md | none mechanical; equal today (checked by hand) | F11 |
| Analyzer configuration | the constructed `ConfigFile` | the parity test's hand-written `pyrefly.toml` | asserted to be "equivalent" | F10 (observation) |
| Refused ambient variables | a list derived by reading 1.3.1 | — | no re-derivation step at upgrade | F11 |
| Coverage status per (module, family) | our driver | — | — | detection of `unavailable`/`partial` is undecided (F5) |
| `fidelity` values | the codebook names (§3.5) | — | — | no definitions anywhere (F6) |

**Deliberately opaque.** The Pyrefly solver and Pysa's call model. The model's known limit, calls
inside annotations, is declared as `outside_provider_model` (§3.5). My recomputation backs that
classification for all 188 FastMCP cases.

**Identity behaviour.** Pysa's `ModuleId` is dropped, because a parallel counter assigns it
(§4.2.3). But dependency modules are keyed by "(module name, path)" with no canonical path form,
and the context digest embeds absolute paths (F9).

## 3. Contracts and invariants

| Invariant | Enforcement | Failure behaviour | Evidence |
|---|---|---|---|
| A Pysa location and its byte range are exact inverses | `LineIndex::offset(.., Utf8)` per module | — | **Tested** (S5: 29,279 of 29,279) |
| The full-range join key is unique per module | the Stage C uniqueness check (§4.1) | a `boundaries` row | **Tested** on FastMCP (reviewer recomputation: 13,104 ranges, none under two callers) |
| Unmatched syntax calls are annotation calls | a classification rule | `outside_provider_model` | **Tested** on FastMCP (188 of 188, recomputed). Nothing says what happens to an unmatched call *outside* an annotation (F2) |
| The public pairs flatten to `compute_public_fqns` | an assertion in every run | the run fails | **Implemented** in the spike; blind to fallbacks both sides share (F3) |
| The config is explicit: no discovery, no interpreter | a constructed `ConfigFile`, `new_constant`, `skip_interpreter_query` | errors from `configure()` abort | **Tested** (S2, plus the reviewer trace) |
| Ambient knobs are refused | an env scan before analysis | exit 3 | **Tested** (S2). The list is complete for pyrefly's library code at `b9f28575` (**Interface-checked**, env sweep) |
| Tables are byte-identical across reruns and handle orders | total sort keys; one thread | — | **Tested** on FastMCP; the evidence doesn't discriminate for import cycles (F8) |
| A per-row CHECK holds at the Delta boundary | `add_constraint` + `DeltaTable::write` | the write is rejected | **Tested** (S6, re-run). The copy is not reconciled (F4); there are bypass paths (F7) |
| Ids survive the Delta round trip | the two-step `cast_with_options` with `safe: false` | a cast error | **Tested** (S6, re-run) |
| A Pyrefly panic never publishes | abort the attempt; a per-module `catch_unwind` | `failed` coverage | **Proposed**; the per-module half is unsafe (F1) |

**The absence lattice at module grain.** The design names four statuses:
- `complete_under_stated_model`;
- `partial` (a parse error; syntax families only);
- `unavailable` (a non-UTF-8 file);
- `failed` (a panic).

The probe shows three collapses:
- `unavailable` has no detector: an undecodable module produces zero rows and exit 0 (F5).
- `partial` does not reach the semantic families built from the recovered tree (F5).
- An unresolvable `__all__` collapses into "no `__all__`" (F3).

## 4. Derivation and execution (Stage B)

| Stage | Inputs | Output | Effects | Provenance |
|---|---|---|---|---|
| Configure | analysis tree, site-packages, Python version and platform | `ConfigFile`; the context digest | none (trace) | `contexts`: a digest of the serialized config, the resolved paths and a `Debug` rendering of sys info (F9) |
| `run` | sorted handles, `Require::Everything`, a no-write `PysaReporter` | Pyrefly `State`, held in memory | a rayon pool with 1 worker and a 10 MB stack (`thread_pool.rs` L23, L84–L99) | producer: fork rev, patch digest, ruff version (§4.0) |
| Extract | per module: AST, `ModuleInfo`, collectors, public helpers | raw batches | lazy solves of dependency modules on the **caller's** thread (§4.2.1 step 5) | `model_id` per surface |
| Build and write | batches against the `cpg-schema` `SchemaRef` | raw Delta tables | Delta commits | commit metadata `lctx.snapshot_id` (audit only) |

**Provider limitations, as far as the design declares them:**
- calls inside annotations: declared;
- `__all__` limited to statically known literal entries: **not declared** (F3);
- recovery artefacts from parse errors: **not declared** (F5);
- state after a panic unsupported: **not declared** (F1).

**Boundary contracts.** From Pysa struct to Arrow, the struct equals the CLI's JSON (S4), so the
in-memory path loses nothing relative to the report. What the Arrow mapping keeps is undecided,
both per variant (F2) and per type field (F6).

## 5. Representative journeys

**Ordinary extension: cross-references (§13).** `pub mod report` already exposes `report::glean`.
A future `references` family is therefore one raw table, one mapper and a fixture test, with no
patch change unless a helper is `pub(crate)`. That is good locality, and it is where the
in-process choice pays off.

**Meaningful change: Pyrefly 1.3.1 → 1.4.x (§4.2.6).** The steps are:
1. Rebase the patch and move the ruff pin.
2. Fix mapper compile errors. Exhaustive matches force codebook appends.
3. Run the parity and determinism oracles.

What the steps miss:
- An appended `pysa_unresolved_reason` code is rejected by the existing table's stored CHECK,
  and no step migrates it (F4).
- The list of refused variables is not re-derived, so a new `PYREFLY_*` knob would pass
  silently (F11).
- The trigger measures the size of the patch, not the cost of the port (F11).

**Boundary: struct → Arrow → Delta → read.**
- Ids come back as `BinaryView` and need the two-step cast (Tested).
- A parameter annotation kept only as `PysaType.string` (spike `main.rs` L704–L736) is
  `display_only`. Yet §4.2.3 labels every Pysa-model fact `report_projection` (F6).

**Interruption or failure.**
- **A panic inside `run`.** The process aborts before any `snapshots` row exists. Readers resolve
  through `snapshots` and filter by `snapshot_id` (§6.1–§6.2), so nothing is published. This is
  **sound**. One wording fix: "Nothing partial is written" (L701) is inexact, because Stage A rows
  may already be written. What holds is "nothing is published".
- **A panic during one module's extraction.** It is caught, the module is marked `failed`, and
  extraction continues on a transaction that Pyrefly declares unsupported after a panic (F1).
- **A stack overflow in a lazy solve during extraction.** This is a process abort, not a panic, so
  nothing is published. But the stack size of the calling thread is undeclared (F8).
- **A crash between `create` (v0) and `add_constraint` (v1).** The table persists without its
  CHECKs, and no open-time check notices (F4).
- **An undecodable module.** It is silently empty (F5).

## 6. Acceptance gates

| Gate | Result | Evidence or scope rationale | Required action |
|---|---|---|---|
| **G1** Authority | **fail** (narrow) | The CHECK constraints in each table's Delta log are an independently mutable copy of the `cpg-schema` codebook ranges. `add_constraint`, `drop_constraints` and `set_tbl_properties` all exist, and the text is stored in normalized form. Nothing reconciles the copy, while §8 L920 claims "they do not add a second definition". "Public" has a single authority, and our copy of its selection rule is reconciled in every run. The patch file is a derived description of the fork rev; it is equal today but unchecked (F11) | F4 |
| **G2** Semantic fidelity | **fail**, and **unresolved** | **Fail.** An unresolvable `__all__` falls back to the local-definitions policy with `via_dunder_all = false`, the same representation as "no `__all__`"; `sub.__all__` entries are silently dropped (reviewer probe). **Unresolved.** What `Target::Overrides` (11,708 of 34,204 FastMCP rows), `Define`, `Return` and `ArtificialAttributeAccess` become; coverage for recovered and undecodable modules; the meaning of the fidelity values. The pilot does not trigger the failure (all 60 FastMCP `__all__` are literal lists), but it does hit the `Overrides` gap | F2, F3, F5, F6 |
| **G3** Validity | **pass** | Every invariant in §3 has a rejection path: errors from `configure()`, the env refusal, the public-set assertion, `RecordBatch::try_new`, the §8 local validators before every write, and CHECK on `DeltaTable::write`. The write-path rule has no mechanical oracle (F7), but local validation stands in front of it | F7 (regression control) |
| **G4** Hidden behaviour | **pass** | **S2:** no `execve`, and identical output under a perturbed `PATH`, `VIRTUAL_ENV`, `PYTHONPATH`, `CONDA_PREFIX` and working directory. **This session's trace:** no writes, no config, ignore or `typings` reads, no walk-up. **The env sweep at `b9f28575`:** the only library reads that change behaviour are `PYREFLY_STACK_SIZE` (worker stack), `PYREFLY_FIXPOINT_DETAILS` (diagnostic detail), `PYSA_DUMP` and `PYSA_DUMP_CALL_GRAPH` (debug dumps), and all four are refused. None of them changes facts. `PYTHONPATH` is read only on the interpreter-query path, which is skipped, and `PYREFLY_LOG` only in the CLI's tracing layer. Pysa's use of the working directory (`absolutize_source_path`, `report/pysa.rs` L347) is neutralized by absolute paths. The prior review's F10 is resolved | keep the list current (F11) |
| **G5** Consistency and recovery | **fail** (narrow) | Aborting the whole attempt is sound. The per-module `catch_unwind` (L702) is not: it publishes a snapshot whose remaining modules were extracted after a panic, from a transaction whose locks and SCC state Pyrefly declares unsupported after a panic (`pyrefly_util/src/panic.rs` L62–L64, `lock.rs` L12) | F1 |
| **G6** Transformation and reuse | **unresolved** | Producer identity carries the fork rev, the patch digest and the ruff version. Open: absolute paths make identical reruns look changed and may enter dependency keys (F9). The one-thread contract's evidence can't discriminate, and its thread topology and stack are undeclared (F8) | F8, F9 |
| **G7** Truthful capability claims | **fail** | The §4.2 table says, under a Tested label, that Pyrefly's public-name helpers **define "public"**. On an unresolvable `__all__` that operation silently falls back to different semantics. Several Tested lines have no test behind them (F10) | F3, F10 |

Each failure is narrow. Each is corrected by a sentence or a table row plus one test. None of
them touches the core choice: in-process linking, one parse, a patched fork, and the CLI as an
oracle.

### The six questions

| # | Question | Answer | Where |
|---|---|---|---|
| 1 | Does the in-process design keep G4 and G5 sound? | **G4: yes.** The explicit config and the refusal list close every output-changing ambient input in pyrefly's library code at the pin, and the trace shows no file effects. **G5: half.** Aborting the attempt on a panic in `run` is sound, because publication is only the `snapshots` append. Catching a panic per module and continuing is not: Pyrefly's own contract is "exit on panic", and extraction runs solver code lazily | G4, G5, F1, F8 |
| 2 | Is dropping Ruff's `__all__` corroboration acceptable? | **As a second authority, yes.** DM-02 wants one definition of "public", and the prior review's F4 asked for exactly that. The ADR's stated reason is imprecise, though. With one parse the *parse* is shared, but a Ruff-side reading of `__all__` would still be our own interpretation, independent of Pyrefly's. **What is lost is a detector, not a corroborator.** Pyrefly's `__all__` is limited to literal entries and falls back silently. Both remaining checks, the `compute_public_fqns` equality and CLI parity, run that same code, so they can't see it (DM-53; charter §G, "the outputs match"). Restore a coverage signal, not a disagreement rule | F3 |
| 3 | Is `fidelity = report_projection` honest for in-memory Pysa structs? | **Yes, for information content.** S4 shows the struct *is* the report; "in memory" describes the transport, not the fidelity. It overclaims in two cases: a column that keeps only `PysaType.string`, as the spike does, is `display_only`; and one row mixes exact enums with projected types. No fidelity value is defined anywhere, so the rule can't be applied consistently to `pyrefly-public` or `ruff-ast` facts | F6 |
| 4 | Are the Delta CHECKs a second authority? Is "writes only through `DeltaTable::write`" enforceable? | **Yes, as written they are a second authority** (G1 fails). They are generated once, and the table keeps the copy across codebook appends and crashes. **Yes, the rule is enforceable.** It needs an ast-grep rule that bans `write_table`, `insert_into`, `RecordBatchWriter`, `JsonWriter`, `SaveMode::Ignore`, and a raw `.sql(` outside one session helper. It also needs a test that the helper's `SQLOptions` reject DML. The bypass surface is wider than `INSERT INTO`: `deltalake::writer` has no constraint handling at `58f07cd` | F4, F7 |
| 5 | Is the one-thread contract justified? | **Justified in mechanism, and cheap:** 5.4 s against 3.1 s at 4 threads on FastMCP (S7, Measured). Cycle answers use per-thread placeholders, published by whichever thread completes the SCC first (`answers_solver.rs` L3350). **But its evidence doesn't discriminate:** 4 threads also gave identical output. And as specified it isn't really one thread. `NumThreads(1)` runs `run` on a rayon worker with a 10 MB stack, while extraction solves lazily on the caller's thread, whose stack is undeclared. `ThreadCount::Inline` exists and would run everything on one thread | F8 |
| 6 | Is the fork-plus-patch maintenance risk stated falsifiably? | **Partly.** "Over ~60 changed lines" is measurable. "Logic change" is undefined, and the current patch already adds a branch. `just adr revisit` reports the trigger as `manual`. The trigger measures the patch, not the port against private APIs, which is where the real cost lies. The only named fallback, Option 4's sidecar, keeps that port cost. The fork is unpublished, and nothing checks that the fork equals the tag plus the patch | F11 |

## 7. Findings

Ordered by severity: correctness and authority first, then extension and cost.

| ID | Finding | Principle IDs | Concrete evidence or gap | Consequence | Proposed correction | Verification |
|---|---|---|---|---|---|---|
| **F1** | §4.2.5 catches a panic in one module's extraction and continues. But extraction runs Pyrefly solver code, and Pyrefly declares its state unsupported after any panic. | DM-30, DM-43, DM-29 · G5, G2 | **DESIGN L702–L703:** "A panic in one module's extraction is caught with `catch_unwind`. It becomes `coverage = failed` …". §4.2.1 step 5 says dependency modules "solve lazily" during extraction. **Pyrefly's contract.** `pyrefly_util/src/panic.rs` L62–L64: "We want to exit immediately because otherwise we'd have to ensure all our thread/lock code properly bubbled up the panic, without producing a deadlock, or another panic". `lock.rs` L12: "If we have a panic, we immediately terminate the program, so we should never encounter a poisoned lock". **The spike** (`main.rs` L94–L98, `worker.join().expect(..)`) has no `catch_unwind`, so the isolation is untested. | A panic while extracting module M, inside a lazily solved dependency SCC, is caught, and M is marked `failed`. Every later module N is then extracted from a transaction whose SCC answers were never published and whose locks may be poisoned. The outcome is one of three: a cascade of panics (loud); a hang, since no timeout is declared; or N's answers computed against leftover placeholders and published as `complete_under_stated_model`. | Any panic in code that touches Pyrefly (`run`, the collectors, the public-name helpers, lazy solves) aborts the attempt. The publication protocol already makes that invisible to readers. Keep `coverage = failed` for non-panic failures such as load and parse errors (F5). If isolation is ever needed, Option 4's process boundary provides it. Change "Nothing partial is written" to "nothing is published". | **None exists.** ast-grep rule: `catch_unwind` is forbidden in the extractor crate. Test: a `cfg(test)` fault hook panics inside one module's collector call, and the test asserts the attempt ends with no `snapshots` row. |
| **F2** | The §4.2.3 mapping table leaves undecided the Pysa variants that carry distinct meaning. The most important is `Target::Overrides`, which is a dispatch set, not one callee. | DM-42, DM-24, DM-34 · G2 | **What DESIGN L656–L665 maps:** `call_targets`, `init`/`new`, `higher_order_parameters`, `if_called`, property getters and setters, `ArtificialCall` and format strings, and `Unresolved::True`. **What it leaves out:** `Target::Overrides` ("All overrides of the given method", `call_graph.rs` L287); `ExpressionCallees::Define` and `Return` (return shims); `ExpressionIdentifier::ArtificialAttributeAccess`; and `implicit_receiver` and `implicit_dunder_call`. **The FastMCP counts** (run a, reviewer count): 11,708 of 34,204 `pysa_calls` rows carry an `Overrides` target, 150 are `define`, and 83 are artificial attribute accesses. **Other gaps.** §3.6 defines `candidate` only as "one of a set", and `DispatchGroup` (§3.1) has no producer. L663 makes every unmatched call an `outside_provider_model` row, with no rule for an unmatched call outside an annotation. The exhaustive-match rule (§3.5) catches only *new* variants. | Implementer A maps a lone `Overrides(f)` target to a definite edge to `f`. Pass A (§9.1) then emits `direct_delegation` from the seed to `f`, although runtime dispatch reaches any override, including user subclasses outside the release. That relabels a dispatch set as one call (ADDENDUM Q5). Implementer B emits `candidate` with `candidate_set_complete_under_model = false`. The same facts give different briefs. Separately, a real Pysa miss outside an annotation would be recorded as a declared model limit. | Give each variant one table row: phase, modality, origin and completeness flag. Add explicit "not carried, because …" rows. Decide `Overrides` (a candidate with an open remainder, or a `DispatchGroup`) and `Define` (the `decorator` phase?). Classify unmatched calls by context: inside an annotation → `outside_provider_model`; otherwise a validator failure or `missing_evidence`. | **None exists.** Test: a fixture with a base method and an override, a decorated `def`, a return shim and an attribute-access dunder, plus an insta snapshot of the mapped `pysa_calls` and `call_targets` rows. |
| **F3** | When `__all__` is not a literal, the public set degrades silently. Pyrefly falls back to another policy or drops entries, every remaining check shares that code, and no coverage signal is emitted. | DM-08, DM-42, DM-53, DM-43 · G2, G7 | **The rule.** DESIGN L673–L679: take `explicit_dunder_all_names` "if present", otherwise local definitions plus explicit re-exports; the Ruff corroboration is dropped. **What Pyrefly does.** `export/exports.rs` L349 returns `None` unless `DunderAllKind::Specified`, so an `Unresolvable` `__all__` reads as absent. L363 (`DunderAllEntry::Module(..) => {}`) skips `sub.__all__` and `*sub.__all__` entries. `commands/coverage/collect.rs` L345 applies the same rule, and the spike copies it (`main.rs` L363–L377). **Reviewer probe** (spike binary, exit 0, the assertion passed): (a) `__all__ = sub.__all__ + ["top"]` gives only `rprobe.top`; `rprobe.alpha` and `rprobe.beta` are missing. (b) `__all__ = _names()` lists `alpha_call`, which the runtime `__all__` excludes, with `via_dunder_all = false`, the same as a module with no `__all__`. **The unused signal.** Pyrefly offers `unresolvable_dunder_all_range()` (`exports.rs` L556) and `ErrorKind::UnresolvableDunderAll` (`binding/bindings.rs` L724); the driver reads neither. **The pilot** is not affected: all 60 FastMCP `__all__` are literal (grep). | On the next library that builds `__all__` from its submodules, top-level public paths vanish. §8's "every public symbol in a brief exists in `exports`" then rejects the brief that names the real path, or Pass A seeds from the submodule alias. Nothing records that the public set is partial, so absence reads as "not public". | Keep Pyrefly as the only definition of "public" and add a **coverage detector**. It fires when `unresolvable_dunder_all_range()` is `Some`, or when the module's `__all__` statement is anything other than a literal list or tuple of strings; Ruff already walks that statement into `export_syntax`. It sets the module's `exports` coverage to `partial` and emits a `boundaries` row, with reason `outside_provider_model` or an appended value. Also correct the rationale at L679. | **None exists.** Test: a fixture package with the probe's three forms (literal, `sub.__all__ + [...]`, a call), with hand-written expected public sets and coverage rows. The hand-written expectations are the independent oracle DM-53 asks for, which parity can't be. |
| **F4** | The Delta CHECK constraints are a persisted copy of the `cpg-schema` codebook ranges, frozen at table creation and never reconciled. "Not a second definition" holds only until the first codebook append. | DM-02, DM-23, DM-51 · G1 | **The claims.** §8 L918–L921: "generated from the same `cpg-schema` declarations … they do not add a second definition". §4.3 L737: create, then `add_constraint`. **S6 re-run.** The table configuration holds `delta.constraints.origin_code: "origin >= 0 AND origin <= 4"`, normalized from `BETWEEN 0 AND 4`. It was committed as v1, separately from create at v0 (`s6_delta.rs` L38–L56, L169). **Mutation paths.** `drop_constraints()` and `set_tbl_properties()` exist (delta-rs `operations/mod.rs` L159, L258). **Growth.** §3.5 L462 says `pysa_unresolved_reason` is "appended when the pin moves". Neither §6.3 nor §4.2.6 has a constraint step. | **(a)** A Pyrefly bump appends reason code 14. The first write of it to the existing `pysa_calls` table fails the stored `<= 13` CHECK, and the upgrade stalls until someone edits constraints by hand, outside any declared path. **(b)** A crash between v0 and v1 leaves a table without CHECKs. Later attempts reuse it without noticing, so the storage-boundary claim in §6.1 and §8 is silently false for that table. | Two ways to close G1. **(i)** Keep in CHECKs only invariants that never change (span order, non-negative offsets). Leave codebook membership to the §8 local validators, which read the current codebook. When an attempt opens its tables, verify each table's `delta.constraints.*` and `delta.appendOnly` against the generated set, in delta-rs's normalized form, and abort on a mismatch. **(ii)** Drop the CHECKs and rely on §8. Option (i) keeps defence in depth for the cost of one open-time check. | **None exists.** Test: create a table, then drop one constraint (or stop after v0), and assert that the open helper refuses it. Plus a codebook-append fixture proving that writes of the new code succeed. |
| **F5** | `partial` and `unavailable` for recovered and undecodable modules have no named detection mechanism. `partial` also leaves out the semantic families built from the recovered tree. | DM-08, DM-47 · G2 | **DESIGN L639–L641:** "that module's syntax families are `partial` … Pyrefly loads a non-UTF-8 file as an empty module; it is `unavailable`". **Pyrefly.** In `state/load.rs` L119 a load error becomes empty source plus a self-error of kind `MissingImport` (L173). The spike reads no Pyrefly errors at all. **Reviewer probe.** A latin-1 `latin1.py` produced zero rows in all five tables, with exit 0. `broken.py` (`return y +`) produced `public_names`, `parameter_semantics` and an `artificial_call` (a binary operator) with `UnexpectedPyreflyTarget`: an artefact of error recovery. | An undecodable module reads as a module with no API, `complete_under_stated_model` and empty. That is exactly the "missing output as negative evidence" §3.5 forbids. Recovery artefacts reach `pysa_calls` looking like genuine unresolved calls. | Detect `unavailable` with our own UTF-8 check on the acquired bytes, which acquisition already holds, rather than by string-matching a `MissingImport` message (DM-47). Detect parse errors from Pyrefly's per-module load errors. Then mark **all** families of that module `partial` (or suppress rows inside the error ranges), and emit a `boundaries` row. | **None exists.** Test: `fixtures/python/_invalid/` with a latin-1 module and a truncated expression, asserting the coverage rows for each family. |
| **F6** | `report_projection` is honest for Pysa types by information content. But the fidelity values have no definitions, the label is per row while a row mixes exact and projected fields, and an annotation kept as a string would be mislabelled. | DM-06, DM-42 · G2 | **No definitions.** §3.5 L454 lists only the names; `cpg-schema` has none; IP L562–L570 lists the names without meanings. **The rule.** DESIGN L680–L683: "Pysa-model facts are `report_projection` even though they stay in memory". **The struct.** `PysaType` is `string` + `scalar_type_properties` + `class_names` (`report/pysa/types.rs` L68–L78). The spike keeps only `annotation.string` (`main.rs` L704–L736). A `parameter_semantics` row carries the exact `kind` and `required` next to the annotation. | One implementer stores only the display string under `report_projection`. An analytic that trusts `report_projection` to carry class names, such as a type-based control recognizer in Pass B, finds none and treats that as absence. `public_names` and `ruff-ast` facts get whatever fidelity each implementer guesses. | Add one sentence per fidelity value in §3.5, saying what structure it guarantees. State which `PysaType` fields `parameter_semantics.annotation` keeps. Add a rule: a fact's fidelity is that of its weakest semantic field, or the annotation becomes its own fact. | **None exists.** Test (the one in REVIEW_REFERENCE §5): `fidelity = display_only` whenever class names and scalar properties are absent. Plus an insta snapshot of the codebook with its definitions. |
| **F7** | "Writes go through `DeltaTable::write` only" has no mechanical oracle, and the bypass surface is wider than `INSERT INTO`. | DM-07, DM-60 · G3 (regression control) | **Prose only:** §4.3 L747 and §6.1. **S6 re-run:** "rows violating CHECK after INSERT INTO: 1", printed but not asserted (`s6_delta.rs` L114–L159). **A second bypass:** delta-rs `crates/core/src/writer/` (`RecordBatchWriter`, `JsonWriter`) has no reference to constraints or data validation at `58f07cd` (Interface-checked, not run). **No rules:** `rules/` is empty. | A later slice streams a derived table through `RecordBatchWriter`, or uses `ctx.sql("INSERT INTO …")` for convenience. The CHECKs are skipped, and only §8 local validation stands, and only if that path also calls it. | Route all SQL through one session helper that uses `sql_with_options` with DDL, DML and statements disabled; §4.3 already says so for Derive. Add ast-grep rules forbidding `$X.write_table($$$)`, `$X.insert_into($$$)`, `RecordBatchWriter`, `JsonWriter`, `SaveMode::Ignore`, and `$CTX.sql($$$)` outside the helper. Turn the S6 bypass observation into an asserting test pinned to the delta-rs rev, so an upstream fix gets noticed. | **None exists.** ast-grep rules in `rules/`, with fixtures in `rule-tests/`. Test: `INSERT INTO` through the helper fails at plan time. |
| **F8** | The one-thread contract is justified, but its evidence can't tell "one thread matters" apart from "FastMCP has no order-sensitive cycles". The thread topology it relies on is also undeclared. | DM-40, DM-35, DM-28 · G6 | **DESIGN L608–L610.** **Order dependence is real:** `answers_solver.rs` L3350 uses per-thread cycle placeholders, published when the SCC completes, so above one thread the result depends on order. **The evidence:** runs a–e have identical digests, including run e at `--threads 4`. **The topology.** `thread_pool.rs` L84–L99: `NumThreads(1)` builds a rayon pool with a 10 MB stack (L23), and `run` executes there. Extraction's lazy solves run on the caller's thread (512 MB in the spike, `main.rs` L94–L96). `ThreadCount::Inline` (L34) would run everything on the caller's thread. **Identity:** the thread count is a spike flag and is not part of `run_id`. | **(a)** Production calls the driver from a thread with a default stack. A deep lazy solve during extraction overflows it, which is SIGSEGV, not a panic, so a whole compile dies on a library the spike handled. **(b)** Someone raises the thread count for speed. The FastMCP tests stay green, and a library with import cycles publishes order-dependent types under an unchanged `run_id`. | Specify `ThreadCount::Inline` on a driver-owned thread with a declared stack size, or keep `NumThreads(1)` and declare the caller's stack. Say the thread count is fixed, or put it in the producer config digest. Make the contract falsifiable with a cycle fixture. | **None exists.** Test: two modules that import each other, with unannotated globals that depend on each other. At the contract setting, output must be byte-identical across shuffled orders and separate processes. Run at N threads, the same fixture shows whether the contract is necessary. |
| **F9** | Absolute locations enter the context digest, and perhaps the dependency-module keys, with no canonical form. Identical analyses therefore get different identities. | DM-15, DM-32, DM-11 · G6 | **DESIGN.** L547–L549: "digest of the canonical serialization of the configured `ConfigFile`, plus the resolved search path, site-package path and sys info". L668: "keyed by (module name, path)". **The spike's `context.json`** for run a has `lctx.search_path: ["/home/paul/lc-spike-pyrefly/analysis/tree"]`, and `lctx.sys_info` is a `Debug` string of an interned struct. **Reviewer probe:** `lcfix` at two absolute paths gives byte-identical tables but different context digests, and the Pysa definition structs embed the absolute path. | Rerunning the same release from another checkout or a tempdir yields a new `context_id`, hence a new `run_id` and `content_digest`. §3.4.1's "compares reruns" then reports a change where there is none, and §9.8's ablation diffs break across machines. If the "path" in dependency keys is absolute, dependency target ids differ per machine, contrary to `node_id` being "stable across snapshots and runs". | Record search and site paths relative to declared roots (the release root and the analysis-venv root), together with content digests of those roots. Serialize sys info from its fields, not `Debug`. Key dependency modules by (module name, site-relative path). | **None exists.** Test: the probe as a test. Run the extractor on one fixture at two absolute locations, and assert that `context_id` and every id are equal. |
| **F10** | Section-level **Tested** labels cover claims the spike did not test. | DM-59 · G7 | **§B8's label (L217)** covers "git dependency on the fork … pinned by revision"; the spike used `path = "../../vendor/pyrefly/pyrefly"`, and pins.md says the fork is unpublished. It also covers "panic handling". **The §4.2 header (L579, "Tested unless marked")** covers: §4.2.5's per-module `catch_unwind`, which the spike lacks; the §4.2.3 mapping table, while the spike emits strings and maps nothing to codebooks; §4.2.2's `partial`/`unavailable`, which the spike never exercises; and "A test runs the pinned Pyrefly CLI", which is a spike script (the nextest test is slice-1 work). **"The public set explains the CLI's `--public-only` report" (L710)** is undefined, and `parity.py` doesn't check it; my recomputation shows "explains" means exact (976) or a public parent prefix (1,814). **§4.3:** "non-null are enforced" (L738) has no null case in `s6_delta.rs`, and "CreateBuilder rejects `delta.constraints.*`" (L737) is a comment (L49), not an assertion. | A later session trusts **Tested**, skips writing the test, and ships the untested path. F1 is the dangerous instance. | Relabel these lines **Proposed**, or **Interface-checked** where the source was read. Define "explains" as an exact or public-parent-prefix match. Call the parity test a "harness-equivalence" oracle: it shares the collectors with the CLI, so it checks our driver (config, reporter lifecycle, lazy solving), not Pysa's correctness. | **No mechanical oracle exists.** Prose: AGENTS.md already says design claims carry a charter §D label. |
| **F11** | The fork-maintenance trigger is only partly falsifiable. It measures the patch rather than the port, and it routes to a fallback that doesn't reduce the port. | DM-59, DM-48 · — | **The trigger.** ADR-0012's `revisit:` says "a patch with a logic change or over ~60 changed lines". Yet the current patch adds a branch (`if !self.write_files { return; }`) that the ADR calls "no logic", and `just adr revisit` prints the trigger as `manual`. **The port.** The spike imports 29 items from `pyrefly::` internals (`main.rs` L14–L33), and a 33-line patch doesn't bound any of them. Option 4 would keep the same imports. **Missing controls.** §4.2.6 has no step to re-derive the refused env list. The fork is unpublished (pins.md). Nothing checks that the fork rev equals the tag plus the patch (checked by hand in this session). | A 1.4.x upstream refactor of `report::pysa` leaves the patch at 30 lines while the mapper port takes days. No trigger fires, and the only named fallback costs the same. Meanwhile a fresh clone can't build until the fork is pushed. | **Define "logic change":** anything beyond visibility changes, borrow-only accessors, and fields whose default reproduces upstream behaviour. **Add a port-cost trigger:** for example, lines changed per bump in the extractor's Pyrefly-facing module, above a stated N. Route it to **Option 1** (CLI + JSON), and keep panics and co-resolution failures routed to Option 4. **Fix the gaps:** add "re-derive the `env::var` reads" to §4.2.6, and either publish the fork or build from tag plus patch. | **None exists.** A `just` recipe, for example inside `pin-check`, that regenerates `git format-patch` from the fork rev and compares its sha256 with pins.md. The same recipe greps the pinned source for `env::var` and diffs the result against the refused list. |

**Observation O1** (no gate impact). Two small consistency gaps; the correction is one line each:
- §3.3 was amended (the BinaryView line is now Tested), but it has no `> Decision:` line and is
  missing from ADR-0012's `design:` list.
- §6's header still says "Pending the ADR-0009 Delta probe", although S6 ran its CHECK and cast
  parts. The Binary-statistics part remains open.

`just adr lint` catches neither.

**Applicability.** Several groups bore on this scope:
- group 9, providers and boundaries (F1, F2, F3, F7);
- group 2, absence and types (F3, F5, F6);
- group 10, reproducibility and diagnostics (F5, F9, F11);
- group 6, effects and failure (G4, F1);
- group 7, dependencies and concurrency (F8, F9, G4);
- group 1, authority (F4);
- group 3, identity (F9);
- group 11, verification and migration (F3, F4, F7);
- group 12, proportionality and falsifiability (F10, F11, §8).

Three bore little:
- Group 4: the change introduces no templates or declarative composition. The CHECK
  "generation" is the only artifact derived from a declaration, and it is handled under G1.
- Group 5: only DM-24 (in F2). No rewrites or lowerings change.
- Group 8: DM-37 is satisfied, since in-process linking removes IPC. DM-39 is satisfied too:
  S7 is labelled Measured, with conditions and an extraction-only scope. No layout changes.

**Verdicts for the applicable principles:**
- **Satisfied:** DM-04 (Pysa's model limit is declared and recomputed); DM-28 (G4); DM-31
  (producer identity); DM-39; DM-41 (the coordinate conversion is mechanical and an exact
  inverse).
- **Violated:** DM-02 (F4); DM-08 (F3, F5); DM-30 (F1); DM-43 (F1, F3); DM-53 (F3); DM-59 (F10).
- **Unresolved:** DM-06 (F6); DM-15 and DM-32 (F9); DM-24 and DM-42 (F2); DM-40 (F8); DM-48
  (F11); DM-60 (F7).

## 8. Alternatives and architectural leverage

| Alternative | Semantic duplication and extension locality | Correctness and operational risks | Implementation and maintenance cost | Performance evidence | Why selected or rejected |
|---|---|---|---|---|---|
| Baseline: ADR-0006 (CLI + JSON) | Two parses with coordinate reconciliation; a decoder schema for undocumented JSON; "public" from a second command | The process boundary isolates panics. Report-format drift is silent unless the decoders are strict | Decoders and line tables; two analyzer runs | none measured | Rejected on the spike's evidence: one parse and exact coordinates are real gains (S5, recomputed) |
| Proposed: ADR-0012 as written | One parse; the collectors reused; our copy of the public rule, reconciled per run | F1 (the per-module catch), F4 (the CHECK copy) | the patch plus a private-API port at every bump | S7: 5.4 s and ~918 MB (FastMCP, 257 modules, 1 thread, release) | A sound core with two mechanisms that add risk |
| **Simpler variant of the proposal** | The same as proposed, minus three things: the per-module `catch_unwind` (abort on any panic); the codebook-range CHECKs (keep immutable CHECKs with an open-time verify, or none); and the rayon pool (use `Inline`) | Closes G1 and G5. One thread and one declared stack | Less code than the proposal | S7 still applies. `Inline` removes a thread handoff (unmeasured) | **Recommended.** Each removal takes out a mechanism whose failure mode is worse than the risk it addresses |
| Option 4: a sidecar (patched fork, Arrow IPC) | Adds an IPC schema per raw table, which becomes a second place the columns are declared unless it is generated from `cpg-schema` | The process isolates panics; the private-API port is the same | An IPC protocol plus process management | none | Keep it as the named fallback for panics and co-resolution only (F11) |

**Abstractions justified by current needs, and what would break without them:**
- **The constructed `ConfigFile` with `new_constant`.** Without it, an upward `pyproject.toml` or
  the interpreter on `PATH` changes Pysa targets under an unchanged `context_id`, which was the
  prior review's F10.
- **The env refusal.** Without it, `PYREFLY_FIXPOINT_DETAILS` or `PYSA_DUMP*` add debug output to
  a run, and `PYREFLY_STACK_SIZE` changes which libraries complete, with no trace in `run_id`.
- **Exhaustive matches.** Without them, a 15th `UnresolvedReason` maps to a wildcard and becomes an
  indistinguishable reason.
- **The public-set assertion.** Without it, our copy of the selection rule drifts from
  `compute_public_fqns` at a pin bump. The assertion is necessary, but not sufficient (F3).

**Not justified:**
- per-module panic isolation: it has no consumer, and it is unsafe (F1);
- codebook-range CHECKs: they duplicate §8 and carry a migration burden (F4).

**What remains ordinary code:** the Ruff walker, the mappers and `IdHasher`. None needs a
declarative layer.

## 9. Verification and measurement plan

| Claim or risk | Evidence label now | Test / analysis | Conditions and expected result | Current result or remaining gap |
|---|---|---|---|---|
| One parse, exact coordinates | Tested | S5 + reviewer recomputation | 29,279 exact; 188 of 188 are annotation calls | held |
| No ambient inputs | Tested | S2 + reviewer `strace` | identical output; no file effects | held. The trace ran on the fixture; FastMCP was traced for `execve` only |
| The env refusal list is complete | Interface-checked (env sweep) | a `just` recipe grepping `env::var` in the pinned source against the list | equal | F11: none exists |
| A panic never publishes | Proposed | a fault-hook test + an ast-grep `catch_unwind` rule | no `snapshots` row | F1: none exists |
| Pysa variant mapping | Proposed | a fixture + an insta snapshot | reviewed rows per variant | F2: none exists |
| Public-set completeness | Tested (literal `__all__` only) | an `__all__` fixture | the expected sets; `partial` plus a boundary row for non-literal forms | F3: the probe shows a silent fallback |
| Undecodable and recovered modules | Proposed | an `_invalid/` fixture | coverage rows per family | F5: the probe shows a silently empty module |
| Fidelity honesty | Proposed | a fidelity test | `display_only` when structure is absent | F6: none exists |
| The CHECK copy matches `cpg-schema` | Proposed | an open-time verify test | a mismatch aborts | F4: none exists |
| Writes only through `DeltaTable::write` | Tested (bypass observed) | ast-grep rules + an `SQLOptions` test | DML rejected at plan time | F7: none exists |
| Determinism under import cycles | Proposed | a cycle fixture | byte-identical at the contract setting | F8: none exists |
| Location-independent ids | Proposed | a two-location test | equal `context_id` and ids | F9: the probe shows they differ |
| The fork equals the tag plus the patch | Tested (by hand, 2026-09-22) | a `just` recipe | sha256 equal | F11: none exists |
| Extraction cost | Measured | S7 | 5.4 s cold, ~918 MB peak, 257 modules, 1 thread, release build | End-to-end cost (with Delta writes and DataFusion) is not measured |

**Reviewer probe fixture.** It lives in the reviewer's scratchpad, not the repo. It is listed
here so it can become the F3 and F5 tests. The package is `rprobe`:
- `__init__.py`: `from . import sub`, `from .sub import *`, `from .dyn import dyn_public`,
  `__all__ = sub.__all__ + ["top"]`, `def top()`;
- `sub/__init__.py`: `__all__ = ["alpha", "beta"]`, with `def alpha()` and `def beta()`;
- `dyn.py`: `def _names(): return ["dyn_public", "dyn_other"]`, `__all__ = _names()`, with
  `def dyn_public()`, `def dyn_other()` and `def alpha_call()`;
- `broken.py`: `def helper(y): return y +`, between two valid functions;
- `latin1.py`: `def latin(): return "café"`, encoded in latin-1.

Run it with `spike-pyrefly --tree <dir> --site analysis/empty-site --out <out>`.

**Cost accounting.** S7 covers extraction only. The Pyrefly `State` (~918 MB peak on FastMCP)
stays alive for as long as the driver holds it, and DESIGN does not say when it is dropped
relative to Stages C–G (Deferred below).

## 10. Exceptions and unresolved decisions

The design claims no SHOULD deviation. F1–F11 are recorded as violations or unresolved decisions,
not as exceptions.

## 11. Decision

**Decision: Not Accept as written. ADR-0012 stays `proposed`.**

**Reason.** The core pivot is sound and well evidenced: in-process linking, one parse, the
collectors, the constructed config and the CLI as an oracle. Where I re-checked S2–S6, they hold:
coordinates, ambient inputs, patch identity and the Delta probes. Judged on its own evidence, that
core would pass.

What fails are four amendments that ADR-0012 introduced:
- per-module panic catching (G5);
- CHECK constraints as an unreconciled copy (G1);
- the silent `__all__` fallback (G2, G7);
- Tested labels without tests behind them (G7).

G2 and G6 also have in-scope decisions not yet made, the meaning of `Overrides` above all. Under
the calibration, an unresolved in-scope gate means Not Accept. None of this needs a new spike.
Each item is a sentence or a table row in DESIGN plus one focused test. Because ADR-0012 is still
`proposed`, it can be amended in place.

**The author has to decide:**
1. **The panic policy:** abort on any panic in code that touches Pyrefly (F1).
2. **The Pysa variant table,** including the modality of `Overrides` and the rule for unmatched
   calls (F2).
3. **The public-set completeness signal** (F3).
4. **The CHECK constraints:** immutable ones only, with a verify-at-open check, or none (F4).

After those, the remaining corrections are sentence-level: F5, F6, F8, F9, F10 and F11. The F7
rules land with slice 1's first Delta write.

| Priority | Change | Principle IDs | Acceptance evidence | Regression protection |
|---|---|---|---|---|
| 1 | Abort on any Pyrefly-touching panic; "nothing is published" (F1) | DM-30, DM-43 | §4.2.5 lines; §B8 bullet | ast-grep `catch_unwind` rule; fault-hook test |
| 1 | Complete the Pysa variant table and the unmatched-call rule (F2) | DM-42, DM-24, DM-34 | §4.2.3 table rows | fixture + insta snapshot |
| 1 | Public-set coverage detector; correct the rationale (F3) | DM-08, DM-53 | §4.2.3 bullets; §3.7 reason | `__all__` fixture test |
| 1 | CHECKs narrowed to immutable rules and verified at open, or dropped (F4) | DM-02, DM-23 | §4.3 and §8 lines; §6.3 step | open-time verify test; codebook-append test |
| 2 | Coverage detection for undecodable and recovered modules, across all families (F5) | DM-08, DM-47 | §4.2.2 line | `_invalid/` fixture test |
| 2 | Fidelity definitions; which `PysaType` fields are kept (F6) | DM-06, DM-42 | §3.5 row; §4.2.3 bullet | fidelity test |
| 2 | Write-path rules and the SQL helper (F7) | DM-07, DM-60 | §4.3 "Never" list | ast-grep rules; `SQLOptions` test |
| 2 | `Inline` on a declared stack; fixed thread count; cycle fixture (F8) | DM-40, DM-35 | §4.2.1 step 3 | cycle determinism test |
| 2 | Root-relative paths in `contexts` and dependency keys (F9) | DM-15, DM-32 | §4.0, §4.2.3 lines | two-location test |
| 3 | Relabel untested Tested lines; define "explains"; name the parity oracle (F10) | DM-59 | DESIGN labels | prose (no mechanical oracle) |
| 3 | Falsifiable revisit trigger routed to Option 1; fork identity and env-list recipe (F11); O1 | DM-59, DM-48 | ADR-0012 `revisit:` and Options; §4.2.6 step | `just` recipe |

**Continuity with the prior review.**
- **F10 (ambient Pyrefly): resolved.**
- **F4 ("public" has two definitions): resolved as to the single definition.** Its "corroboration"
  half is superseded by F3 here: a coverage detector, not a disagreement rule.

### Deferred

| Item | Why deferred | Reopen when |
|---|---|---|
| Sidecar isolation (Option 4) | No panic was observed on FastMCP, and abort-on-panic is safe for readers | the first Pyrefly panic on an in-scope library, or a pin that won't co-resolve with the §7 family |
| Dropping the Pyrefly `State` before Stage C | ~918 MB on FastMCP is affordable on this machine | a library whose peak RSS exceeds the machine budget |
| End-to-end cost (extraction + Delta + DataFusion) | S7 is honestly scoped to extraction | the increment-1 `deep` review |

## 12. Disposition (author, 2026-09-22)

The review's recommended "simpler variant" (§8) was adopted. ADR-0012 was amended in place; it is
still `proposed`. Each design correction is in DESIGN.md. Each oracle is carried into increment 1,
slice 1, as ADR-0012's Consequences list.

| Finding | Disposition | Where | Oracle (slice 1) |
|---|---|---|---|
| F1 | Any panic in Pyrefly-touching code aborts the attempt; no `catch_unwind` | §4.2.5, §B8 | ast-grep rule against `catch_unwind` in the extractor; fault-hook test |
| F2 | Every Pysa variant has a row. `Overrides` is a candidate with an open set. `Define`, `Return`, `FormatString` and others are "not carried". Unmatched calls are classified by annotation context | §4.2.3, §3.6 | variant fixture + insta snapshot |
| F3 | Completeness detector: `unresolvable_dunder_all_range()` or a non-literal `__all__` → `exports` coverage `partial` + boundary | §4.2.3 | `__all__` fixture with hand-written expected sets |
| F4 | Option (i): only immutable CHECKs, verified when a table is opened; codebook membership stays in §8; a CHECK change is a migration | §4.3, §6.3, §8 | open-time verify test; codebook-append test |
| F5 | Own UTF-8 check gives `unavailable`; parse errors read from Pyrefly errors; all families `partial` | §4.2.2 | `_invalid/` fixture |
| F6 | Fidelity values defined; weakest-field rule; `parameter_semantics` keeps string, class names and scalar properties | §3.5, §4.2.3 | fidelity test |
| F7 | One SQL helper; low-level writers banned; the bypass becomes an asserting test | §4.3 | ast-grep rules; `SQLOptions` test |
| F8 | `ThreadCount::Inline` on a driver-owned thread with a declared stack; the setting is part of `producer_id`. The spike re-ran FastMCP with `Inline`: identical output, 4.1 s, ~765 MB | §4.2.1 | import-cycle fixture across shuffled orders and processes |
| F9 | Context paths relative to declared roots, plus root digests; sys info from fields; dependency keys site-relative | §4.0, §4.2.3 | two-location test |
| F10 | Labels corrected: §B8 split; §4.2 header ("Tested only where a spike result is cited"); "explained" defined; the harness-equivalence oracle named; §4.3 observations downgraded | §B8, §4.2, §4.3 | prose (none mechanical) |
| F11 | Logic change defined; port-cost trigger routed to Option 1; fork-identity and env-list step added | ADR-0012 `revisit:`, Options; §4.2.6 | `just` recipe |
| O1 | §3.3 has a `Decision:` line and is in `design:`; the §6 header states which ADR-0009 probe parts remain | §3.3, §6 | — |
