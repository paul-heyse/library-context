---
id: ADR-0162
title: Share the ordinary O2 dev and test family with explicit release acceptance
status: accepted
date: 2026-10-12
supersedes: [ADR-0137]
superseded-by: null
design: [§1.2, §4, §15]
evidence: Interface-checked
revisit: Current affected functional controls expose a profile-dependent defect, a relevant producer dependency is omitted, or a concrete remaining build cost warrants investigation.
---

## Context

The operator approved efficiency RC14 and D03: qualify a single ordinary dev/test family in
this track, independently of the earlier compilation-cost plan. Current development already uses
O2 incremental compilation. A second O1 candidate introduces another artifact family without a
present functional requirement. First-principles execution fit and current revealing controls are
sufficient to choose a profile; no historical benchmark is required and no speedup is claimed.

This record replaces ADR-0137's profile choice and carries its unaffected development decisions.
Exact dependency pins remain with ADR-0132, assurance with ADR-0126, environment/maintenance/worktree
ownership with ADR-0134, and current persistence and service ownership with ADR-0145. The
[efficiency coordinator](../plans/efficiency-alignment-plan_2026-10-11.md#4-package-sequence-and-sole-execution-checkpoint)
owns functional evidence and the default switch. Existing plans retain their independent scope.

## Options

1. Share the existing O2 incremental dev/test family, with assertions and overflow checks on;
   keep release as an explicit acceptance family. This reuses the ordinary dependency policy and
   gives every consumer one deliberate ordinary profile (selected).
2. Qualify a separate O1 ordinary family. It may reduce optimizer work, but creates another
   standing artifact family without evidence of a necessary behavior or workload advantage.
3. Keep release-only local verification. It preserves the current default but couples ordinary
   iteration to the production assertion policy and retains separate dev/release artifacts.
4. Add scheduling, resource caps or an alternate resolver. These do not remove the diagnosed
   coupling and duplicate existing tool owners.

## Decision

- Use O2 incremental dev and test with debug=0, assertions and overflow checks enabled. Imported
  dependencies, including non-workspace path dependencies, use O3 non-incremental compilation.
  Release remains O2 incremental with its distinct production assertion/overflow policy.
  After this track's current affected functional controls pass, ordinary verification selects
  `test` and the temporary `local-test-candidate` profile retires. Until then the verifier remains
  release. No existing plan's qualification is inferred from this switch.
- Resolve commands, artifact paths, environment bindings, receipts and reruns from the same
  selected Cargo profile. Test/dev artifacts use Cargo's debug directory. Every evaluator
  consumer owns its selected-profile producer rather than silently borrowing a release binary.
  Recorded reruns retain their recorded profile; an unavailable retired profile refuses with
  a route to a newly selected run, never silently substituting another profile.
- Production acceptance, required oracle/extension leaves and assembled qualification remain
  explicitly release-resolved. Fixture and profiling wrappers preserve caller selections;
  production service installation continues to use its explicit release installer.
- Retain the pinned `nightly-2026-09-29` toolchain, minimum language version and exact Hakari pin
  in their executable owners. Keep Cargo workspace feature unification, fine-grain locking,
  sccache and Clang/mold. Compiler frontend/backend workers, Cargo's jobserver and test runners
  use available parallelism; correct concurrency failures in production or test code.
- Shared intermediates remain `{cargo-cache-home}/build/library-context`; final binaries and
  extensions remain checkout-local `target/`. Preserve warm caches and existing captures;
  never routinely `cargo clean` shared intermediates. Existing shared sccache capacity is
  unchanged. Isolated benchmark producers own both artifact directories; their comparison
  variants prescribe no ordinary setting.
- Retain the allocative never-type backport at
  `9711293c6de502d50583cafb12e4a7b764094d3a` through the existing vendored patch until a compatible
  selected release replaces it. Dependency versions change through their current pin owner.
- Repository launchers normalize inherited target exports through `build_environment.py`;
  bare Cargo reads repository configuration. `LCTX_CARGO_TARGET_DIR` is the explicit override.
  Stable paths preserve cache identity; environment repair uses its existing owner.
- Hakari materializes the feature union only at `lctx`. Lower libraries/native bindings stay
  outside it. Cargo owns resolution; existing generation/dependency commands own generated
  edges. Separate analyzer families remain in their owning adapters.
- Provider fingerprints include relevant normal/build dependencies and declared assets, with
  conservative target/optional/patch roots and root manifest/lock sensitivity. Dev-only and
  unrelated downstream roots are excluded. Compiler output composes stage implementation and
  rooted compiler/ingestion identity in the existing contribution implementation hash. Raw
  supplier identity, stage code, exact source attribution and semantic compatibility remain
  distinct; imported completed specifications retain captured hashes.
- Typed adapters retain record, declaration and coverage checks. Native task/stream mechanics
  run after admission, and accepted writes own complete charged batches until terminality.
  Coherent provider topic groups share a substantial driver only under exact immutable input
  identity; semantic variants and lifecycle tests retain fresh execution.
- Keep ordinary work on shared main and use worktrees for independent/conflicting mutable
  state. DESIGN owns architecture, ADRs rationale, and AGENTS commands. Process remains light:
  add a hook, gate, register or document type only for an actual consumer.
- The stable service owner governs configuration and lifecycle. Retire ADR-0137's disposable
  fixture allocation/threshold clause: current verification borrows logical attempts on an
  installed stable service; no implicit service creation or fixed resource cap is introduced.
  Operator databases and adoption remain outside validation effects.

## Consequences

Current pure model and client controls support the selected family; native and Python/native
integration qualification and the coherent default switch remain with D03. Decision acceptance
is not a test pass. Profile/toolchain/flags/target differences may still require separate artifacts,
and sharing a directory does not make test-target and ordinary library objects identical.

Nightly instability remains accepted. Feature unification can mask missing standalone feature
requirements; independently publishing a crate still needs its own qualification. No elapsed-time
benefit, historical-plan closure or whole-product qualification is claimed. Current-only cutover
retires obsolete runtime configuration; retained receipts remain attributable observations.
