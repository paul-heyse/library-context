# Analyzer fidelity and observation alignment

**Proposed, 2026-10-04.** Supporting design for the [target-alignment coordinator](target-implementation-alignment-plan_2026-10-04.md). It actions TA-F01 and LL-F08 plus selected LL-F09/10 mechanics. The coordinator owns current disposition, conditional decisions and assembled acceptance.

## 1. Baseline and semantic boundary

Current pins are Pyrefly 1.4.0-dev.3 with embedded Ruff 0.0.14, independent Ruff 0.16.10/crates 0.0.16 and ty 0.0.16/salsa 0.28.5. [Pins](../pins.md) owns exact immutable fork revisions and scoped receipts. This is correction of current linked facts, not another analyzer-version migration. The [migration plan](python-analyzer-migration-plan_2026-10-03.md) retains its historical baseline and execution evidence.

**Implemented / source-inspected, 2026-10-04:** `cpg-flow` identifies Bound candidates whose definition cannot attach. `ty_flow` currently maps a missing definition to Unbound reaching evidence; inventory validation permits the kind mismatch for unattached candidates. Entry's complete-inventory/Bound safeguards refuse the inspected proof path, but published raw origin still changes meaning. The correction cannot be described as preventing a demonstrated false behavioral verdict.

An observation's native kind, source/view/context, condition, support and mapping availability are separate. Missing attachment does not authorize a synthetic definition, false Undefined state or omission of native candidate evidence. Ruff typing context is not runtime predicate truth; Pyrefly platform/version policy need not agree with runtime tuple semantics.

## 2. A1 — faithful native candidate inventory

### Target producer and contract

Capture every native use candidate before pruning or loop expansion, with its native kind, reachability condition, narrowing condition, unavailable/precision state and existing attachment/cardinality flags. Extend existing `cpg-flow::UseCandidate` rather than create another target resolver.

Extend the existing model `CandidateState`/`FlowUseCandidate` with optional reachability and narrowing `AssertionQualification` references and explicit narrowing-unavailable/precision-loss distinctions. Preserve the existing reachability-unavailable flag. Each retained qualification binds condition, scope, context, assumptions and approximation; source/run/view attribution continues through inventory, use, source-view and support. An unavailable formula has explicit state, not a guessed true/false condition. Include added evidence in candidate identity/digest and replay.

For native Bound whose target cannot attach: publish the candidate as Bound, unattached and `mapped_count=0`; retain its qualified native formulas; publish no fabricated `FlowReaching` or target-bound `FlowNarrowing` observation. The existing inventory is incomplete and emits the NativeUnavailable SubjectBoundary for failed target attachment. Formula availability remains independently characterized. Do not invent a definition ID or add a second mapped-target relation.

For real Undefined/Deleted, preserve genuine Unbound mapping. For valid attached Bound, preserve target and condition/narrowing observation semantics. Nested/LoopHeader/pruned/expanded cases retain native kinds and current cardinality/refusal rules. Retained candidate formulas do not authorize new Entry witnesses.

### Invariants, consumers and migration

Remove the validator's unattached-kind-mismatch escape. Every mapped reaching member must agree with its native candidate kind and bind the same use/source/context; Bound plus unattached requires zero mapped members. Do not apply unattached⇒zero to all kinds: a LoopHeader may retain some mapped expansions while another expansion lacks attachment. Validate candidate formula membership, scope/context, availability/precision consistency, inventory attribution, candidate/member digest and ordinal cardinality.

Keep inventory completeness about native reaching enumeration and attachment: narrowing absence/precision is a separate fidelity property, not an added condition for reaching completeness. Do not strengthen Entry simply because formulas now survive. Its exact native singleton and complete-inventory gates remain required.

Migrate `cpg-flow`, `cpg-extract::ty_flow`, model construction/validation, declared store read dependencies, raw-origin hydration/response and affected tests together. Raw-origin results expose candidate qualifications and explicit mapping availability without relabelling the candidate. The condition is native evidence under its view, not a proof of runtime feasibility.

This is a deliberate small **schema and wire migration**, including candidate evidence fields, identity/digests, generated schemas and stored/raw response readers. Review snapshots and rebuild affected generations/artifacts from pinned inputs; no old-format inventory reader. Register any changed public interpretation/identity policy through the existing ADR/owner route before publication. No whole analyzer fork upgrade is needed solely for local flow capture.

### Independent acceptance

Use native source fixtures for class/type-alias PEP 695 Bound-unattached, function type parameters, attached ordinary Bound, genuine Undefined and Deleted, Nested, pruned and LoopHeader candidates. Check source assertions independently of the mapping helper. Verify retained formulas and explicit unavailable cases, no false Unbound row, exact counts/digests and shuffled stability.

Add negative model controls for kind mismatch, wrong formula context/source, unavailable/formula conflict and wrong cardinality. Actual disposable PG round trip must serve Bound/unattached candidate metadata and formulas while preserving complete=false. Re-run Entry/model handoff refusal for incomplete inventories and current attached positives; a candidate-only condition must not establish a behavioral verdict. Compile touched flow/extractor/model/store/native crates, then focused controls; full gates belong to coordinator Q0.

## 3. A2 — consume or retire supplementary Ruff observations

The narrow fork emits Export and Branch observations that `ruff_lexical` currently drops. Choose by consumer rather than lowering every emitted event.

**Export:** use exact attributed name/range observation to supplement public-exposure evidence and explain correspondence with existing export detection. It does not establish complete exports, dynamic `__all__` evaluation or a replacement export-resolution authority. Require original range and provider/view identity; retain unmatched/ambiguous remainder.

**Branch:** retain attributed typing-context observation only where it explains a selected typing/runtime or availability difference. Do not lower it into runtime path conditions or use it as reachability proof.

The first A2 step inspects concrete export and typing-context fixtures and names the output/coverage consumer. If a channel adds useful evidence, define a minimal observation relation/association through existing provider assertions, support and normalization; bind exact source/role/context. If no current/planned consumer changes an answer, remove that unused emission/patch surface and qualify remaining fork parity. The investigation closes on either choice; it cannot silently leave unused maintenance marked implemented.

Fork changes share one editing owner and use exact current sources, minimal patch/pin updates and `pin-check`/fork policy evidence. Observational additions must not change Ruff solver/lint behavior. Version/source-family checks and artifact rebuild remain required after a changed fork revision.

Acceptance: static `__all__` observations, aliases/dynamic remainder, typing-only branches and unmatched ranges; independent emitted-versus-retained controls, no false complete export list or runtime predicate. If removed, unchanged remaining source facts/diagnostics and deleted stale patch guidance establish closure.

## 4. A3 — attributed policy comparison and coverage usefulness

Investigate one consequential joined-source comparison: `sys.version_info == (3, 14)` under Pyrefly prefix equality versus runtime five-field tuple semantics. Include a resolved import/alias versus spelling-only case and typing-only versus runtime view. Compare by original byte range and declared provider/context/view, not native AST ID or global name.

The consumer is an explanation of **which policy decided an observation** and where correspondence is unavailable. A normalized comparison may record same decision, differing decision or undecided/unattached with original supports and each policy identity. Never force provider equality, replace one analyzer's decision globally, or treat a typing decision as runtime feasibility. Add a new relation only if the actual explanation consumer needs persistent paired evidence; otherwise retain attributed observations and compose the comparison at the existing pure explanation operation.

Check Pyrefly configured custom builtins as explicit input to Ruff only where their meanings match the intended lexical lookup. A provider's full builtin universe is not another provider's universe. Unmatched declarations retain provenance and unavailable correspondence; no name-only substitution or implicit environment reads.

For local coverage, select an unattached use/scope where a more local premise explanation changes a served answer. Compare existing inventory/support with the proposed locality; distinguish provider not requested, unavailable formula, unattached source, partial enumeration and complete native evidence. No attached read in a scope is not absence. Adopt a finer relation only for a demonstrated explanation gap; otherwise retain existing explicit incomplete coverage and trigger on a named affected task.

The lexical recognizer replacement remains deferred until Ruff offers needed multi-candidate/LOAD_NAME semantics or ty exposes both-profile builtin/class-body fallback. Do not replace it with a narrower API because it has a familiar name.

Acceptance: exact source joins, aliases, unavailable/ambiguous correspondence, legitimate cross-view disagreement and no behavioral promotion. Decisions and enabled consumer changes are recorded at the coordinator before implementation.

## 5. A4 — qualified parser and naming mechanics

Replace repeated docstring prefix/quote recognition with the linked Ruff string facility only after source inspection confirms the prefix/body-offset contract. Preserve Exact/Extended/None behavior, UTF-8 byte offsets, raw/unicode/triple/single quote cases and malformed-input refusal. This is not a request to normalize away original evidence or to parse more documentation styles.

Reuse canonical token comments where the current flow comment scanner is equivalent; keep provider-local parse ownership and exact offset joins. Reuse `pep508_rs::PackageName` for validated distribution normalization and `ruff_python_stdlib` identifier helpers for their actual consumers. Check accepted and invalid boundary behavior before substitution: an existing permissive helper must not silently change malformed acquisition input into an admitted name. Delete replaced local mechanics only after all consumers move.

Acceptance: independent exact byte slices, non-ASCII/prefix/quote controls, malformed literals, PEP 503 separator/case cases and invalid names. No new parser, dependency family or semantic detector replacement is implied.

## 6. Conditional analyzer opportunities

| Package | Consumer and bounded decision |
|---|---|
| IA1 references/export | Reopen Glean only for a graded typed-non-self receiver reference gap, preserving observation versus runtime use. SCIP needs a named external client. Keep existing native/ty navigation oracles development-only; no alternate production type provider |
| IA2 public surface | Griffe may qualify public_records/PR0 surface on selected nongold fixtures. Establish independence from its shared fastmcp gold extractor; agreement cannot corroborate gold-scored items. Adopt dev-only if useful, otherwise defer |
| IA4 PEP 695 attachment coverage | Enumerate affected class/alias parameter readers and the lexical scope distinctions needed for useful attachment. A later expansion requires exact definition/source-role controls; this inquiry does not block A1's faithful unattached representation |
| IA3 documentation | Ruff sections for NumPy/Raises/Returns/Examples require a selected documentary task whose answer changes. Compare exact original ranges, multi-paragraph text and unavailable sections; weigh added fork rebase cost. No automatic harvesting or executing examples |

These are investigation/deferral routes, not an exhaustive second provider survey. Named answer gaps can justify fuller analytics later. In the absence of such a consumer the coordinator records the trigger and keeps the current mechanism.

## 7. Ownership and qualification boundary

A1 is immediately ready and independent of foundational expansion. A2/A3 first settle named consumers and evidence semantics; A4 is bounded mechanics. All source/view/condition, candidate and wire definitions have a single model writer; provider/fork changes have one supplier owner. Parallel read-only advice does not require worktrees.

T0 updates current analyzer wiring and the shared skill's project guidance through its maintenance route, separating historical migration from current facts and fork changes. Shared library skill sources remain repo-agnostic under current policy; repository wiring belongs in current owners rather than another copy of authority.

Existing Entry/Structural positive and refusal receipts stay historical. New schema/source/pin changes require matching artifacts and actual PG/wire controls, followed by coordinator Q0. Production compile/tests/oracles, migrations and activation are not_run in plan authoring; all target changes remain Proposed.
