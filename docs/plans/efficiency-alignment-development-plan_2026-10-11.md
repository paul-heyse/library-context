# Efficiency alignment: development loop and evidence tooling

**Proposed, 2026-10-11.** Companion to the new
[coordinator](efficiency-alignment-plan_2026-10-11.md), the sole progress and finding-disposition
owner. Sources: original O-F41–O-F43/O-F45–O-F50 and §7.10; audit A-F09/A-F12/A-F13/A-F17.
No existing plan was reviewed or superseded. Product implementation/control requirements remain
Planned. The 2026-10-11 design-decision investigation ran isolated pure Cargo probes; it performed
no product implementation, shared-service action or repository-level quality check. See
[development/serving decisions](../design_review/evidence/2026-10-11_efficiency-design-decisions/development-serving.md).

## 1. Target and boundaries

Make a focused change build and execute its actual dependency and test scope. Share immutable provider
observations within a meaningful test case, use one qualified ordinary dev/test profile, carry service
identity through a protected generation, and complete lifecycle operations through exact enrolled
objects. Status reads metadata; reports decode samples. Evidence retains what was observed at the
stated time. Faster reporting must not silently strengthen that evidence into executed-binary proof.

Reuse current verification, fixture, runs, storage and profiling owners. No new mandatory runner,
scheduler, host admission service, fixture cache or global registry is needed. Preserve normal available
compiler/test concurrency, logical isolation on the installed shared service, captured receipts,
protected caches and independent child-exit/cleanup facts. RC12/RC13 retain these rules.

The baseline Cargo dev profile is O2 incremental with debug=0; imported packages are O3 non-incremental.
Test inherits dev except debug=0. Release is O2 incremental with the release assertion/overflow policy.
An additional O1 local-test candidate exists. This track independently qualifies a shared ordinary
dev/test route; it does not read or wait for the old BC3 plan. No timings currently establish which
profile minimizes elapsed time. Functional equivalence and one deliberate default are the exit here.

## 2. Packages

<a id="D01"></a>
### D01 — Share immutable provider observations at meaningful test grain

**Sources:** O-F45; A-F12. Start with execution-channel, local-semantics and entry-value-witness tests
that compile the same fixture/provider/profile/configuration. Group related assertions around one
owned compilation within a test process. Keep assertion names diagnostic; grouping changes nextest
case grain and must not report subassertions as independently executed cases.

Fresh execution remains necessary for input/profile variants, ordering/determinism, cancellation,
abandonment and tests whose behavior is compilation itself. Sharing requires complete immutable
fixture/provider/settings/profile identity and valid output lifetime. Do not create a cross-process
persistent fixture cache merely to preserve an unnecessarily fine case split. Use C02/C07's complete
capture/attribution principles when choosing what is actually common.

**Closure:** a revealing common fixture compiles once for its grouped observations; a differing semantic
variant compiles separately; failures name the failed assertion; no mutable fixture or lifetime leaks
between cases; ordinary affected provider controls retain their original semantic assertions.

<a id="D02"></a>
### D02 — Align crate dependency and producer fingerprint closures

**Sources:** O-F46; A-F12. Consume C01's lower semantic-provider contracts, then migrate actual consumers.
Move embedding/tokenizer contracts below compiler orchestration. Separate shared native codec/control/
configuration contracts from compiler-store/reader implementations where an actual consumer otherwise
links unrelated behavior. Do not split the roughly 196k-line model simply because of its size.

The current fingerprint walker follows declared normal/build local dependencies and full source
directories. Feature-gating an unused adapter does not by itself narrow it. Establish responsible
identity boundaries alongside dependency changes; retain model, codec, ingestion and N08 derivation
implementation in their consumers' fingerprints. Avoid an under-keyed build cache disguised as a
smaller dependency graph.

Retain the DataFusion Parquet feature: the bounded dependency inquiry below found a supported
public session capability that requires it. Prefer existing lower types over a new contracts crate
for every name.

**Decision selected, Interface-checked 2026-10-11:** move `Embedder`/`EmbedFuture` to the existing
model embedding owner with `ModelError`; adapt compiler errors at its boundary. Consume C01's
`domain::producer_contract` fact inventory and capture descriptors, retaining provider executables
in extraction. Select a physically lower `lctx-compiler-store` native admission boundary for the
publisher's admitted/export/restored capabilities, exact completed views and native handle; private
validated construction preserves semantic admission. Restore invokes that admission without analyzer
orchestration. Move publisher budget defaults to their resource/config owner. Keep shared codec,
control/installed identity, canonical ingestion, transport and selected semantic admission below it;
serving-only preparation/presentation/read adapters remain above the producer closure. Split compiler
store/provider orchestration out of the current omnibus native crate, rather than merely feature-gating.
All normal/build manifests and consumers move together. Shared selected-read changes affecting admission
remain responsible semantic changes; only physically separated serving-only edits may leave compiler
identity unchanged. Retain conservative full source/runtime-asset fingerprint bytes until that physical
boundary exists, including model, ingestion, codec and N08 derivation implementation.

**Parquet inquiry resolved:** retain the current feature. Literal product Parquet calls were not found
in the inspected crate/script/Python scope, but `CompletedInputs::session` exposes unrestricted public
`SessionContext`; DataFusion 55.1.0 registers `ParquetFormatFactory` under that feature. Removing it
would silently narrow this capability. Exact transitive feature output is retained in the decision
evidence. Its external sort spills Arrow IPC, so spill is not a Parquet requirement. This negative
removal decision neither closes dependency/fingerprint migration nor authorizes a support-policy change.

**Closure:** actual compiler dependency inspection and focused builds show publisher/embed/serving
consumers avoid unrelated analyzers; reader-only edits leave producer identity unchanged; responsible
provider/ingestion/derivation edits change it. Consumer migration, manifests and fingerprints move as
one slice. No dependency version bump or wholesale lock resolution is implied.

<a id="D03"></a>
### D03 — Qualify one ordinary dev/test profile in this track

**Sources:** O-F47/RC14; A-F12. Select the current O2 incremental dev/test family as the initial shared
candidate, retaining test debug assertions and overflow checks, debug=0, imported O3 non-incremental
code and pinned toolchain/linker settings. This avoids introducing yet another profile while establishing
that ordinary tests can use the dev family's artifacts. Release remains a distinct production acceptance
profile; `cargo check` is not a second code-generation profile.

Run this track's affected functional controls in the selected ordinary profile, including invariant,
assertion/overflow-sensitive, native contract and Python/native integration behavior where affected.
Use current ordinary controls and owners without importing the old BC3 plan's status or prerequisites.
Investigate any discrepancy as a code/test contract issue; do not disable assertions merely to pass.
Default-switch consumers include verify/fixture/rerun/profile tooling and current documentation.

After functional qualification, switch ordinary defaults together, retain explicit release acceptance,
and retire obsolete candidate configuration only after checking actual current consumers. This is a
configuration/consumer cutover, not authorization to purge existing build caches. If evidence requires
a different optimization level, choose one shared dev/test family and rerun the affected controls;
record the reason here through the coordinator before switching.

**Decision selected, Interface-checked 2026-10-11:** keep O2 incremental dev/test with explicit
assertions/overflow on and debug=0, imported O3 non-incremental and current toolchain/linker. The isolated
exact-profile probe passed dev build/test/release and demonstrated reuse of the same imported dev artifact
by tests, assertions/overflow enabled for test and separate release policy; it establishes no product
qualification or elapsed-time improvement. Functional controls in this track precede switching
`verify.DEFAULT_CARGO_PROFILE` to `test`, fixture/artifact-path consumers, profile docs and recorded rerun
handling together. Retain production-only native Python/oracle leaves and qualifier guards as explicit
release until their own consumer cutover. Remove `local-test-candidate` only after current consumers
migrate; a shared directory does not make test-target and ordinary library objects identical.

**Closure:** ordinary commands select the qualified family consistently, recorded reruns preserve it,
release remains selectable, and affected controls pass. Artifact reuse is supported by actual profile/
dependency configuration; faster elapsed time remains unmeasured. This package has no dependency on
completion of an existing plan and does not claim that plan's findings closed.

<a id="D04"></a>
### D04 — Carry service identity through one protected fixture generation

**Sources:** service-proof part of O-F48; A-F13. Acquire/borrow the installed service generation before
administrative checks and carry its checked identity across the fixture boundary and required databases.
Do not reopen and re-prove an unchanged protected generation for every subcommand. Service replacement
invalidates the witness. A global stat/path cache is not protection against replacement.

Integrate the existing fixture/service lifetime owners; preserve explicit readiness refusal, logical
attempt isolation and maintenance authority. This package never installs or resets a service implicitly.

**Decision selected, Interface-checked 2026-10-11:** the existing borrow owner returns a
`CheckedServiceGeneration` only after acquiring protection and checking daemon process identity,
immutable executable generation, installed service/schema generation and exact endpoint/database scope.
Carry it through verify prerequisites, fixture attachment and child setup under the same borrower lease.
Readiness outside a borrow remains observational. This replaces repeated administrative proof only within
the protected generation; a changed generation/admission boundary invalidates it. Integrate
`surrealdb_service.borrow`/`Installation.check` and `surrealdb_fixture.substrate_readiness`/
`Attachment.create`; preserve exclusive maintenance and unknown attachment holds. No service action
ran in this lane's design investigation.

**Closure:** a borrow reuses checked premises within its protected generation; replacement or altered
identity refuses/refreshes through the owner; failed readiness remains blocked with its actual repair;
borrower release neither retires the service nor mutates operator databases.

<a id="D05"></a>
### D05 — Preserve completion-time executable observation while avoiding repeated work

**Sources:** executable-evidence part of O-F48/RC15; A-F13. Retain the current policy: evidence records
the executable observed at completion, with its exact time and scope. It does not prove the bytes that
were loaded at process start, nor complete source provenance. Reuse an observation only while its
executable generation is protected under an owner that makes the reuse valid.

Export and later report commands consume the original observation. They must not hash a mutable path
later and substitute that digest for the earlier one. If a stronger executed-artifact claim becomes
necessary, design the launch/identity capture explicitly and obtain the corresponding rule decision;
this plan does not add that requirement by accident.

**Decision selected, Interface-checked 2026-10-11:** capture a completion-time executable descriptor,
reading/hashing once with before/after `fstat` and pathname-binding checks, and retain its original time,
descriptor identity, digest and observation policy in the receipt. A descriptor protects rename/unlink,
not in-place writes. Rehash each unprotected mutable Cargo output at completion; stat equality alone is
not reuse authority. Digest reuse requires an actual immutable producer-owned executable generation
held through observation. Keep source `context_after`, service descriptor and the completion artifact
digest distinct. Executed-byte identity and complete source-closure provenance remain unknown.
Reports/exports consume
the retained observation without reopening a later path. Replacement/mutation/race/missing/immutable-reuse
controls remain functional work.

**Closure:** a later executable replacement cannot alter retained receipt identity; protected repeated
completion observations reuse safely; missing capture remains unknown. Reports state the original
policy and limit. This is independent of D04's service identity witness.

<a id="D06"></a>
### D06 — Narrow recorded reruns to exact Cargo target identity

**Sources:** O-F49; A-F13. Preserve package ID plus Cargo target kind/name, profile, features, exact
case filters, required prerequisites/environment and fixture bindings. Replace broad selectors only
when the recorded target identifies a narrower build. Deliberately handle library/proc-macro, integration
test, binary, example and benchmark targets; refuse ambiguous or unsupported reconstruction.

Print and execute the same intended selection. A test-name filter alone often narrows execution but
not compilation. Use stable exact filtering and existing metadata; experimental nextest regex/filter
features are not a prerequisite. Keep failure/result inspection independent of rerun execution.

**Closure:** representative target kinds reconstruct their exact build/test scope; selected tests do
not disappear through filter mistakes; feature/profile/environment distinctions survive; ambiguity
refuses with a useful reason; printed reruns have no execution side effect.

<a id="D07"></a>
### D07 — Complete lifecycle work through exact enrolled objects

**Sources:** O-F50; A-F13. Runs, cancellation, fixture recovery and retention use the owner's exact
enrolled IDs and indexed object paths. Avoid scanning and validating unrelated host records while
completing a known owner. Preserve that owner's local lifecycle errors, references, cleanup evidence
and durable release obligations. Main run receipts already precede the global sidecar scan; describe
the correction accurately rather than claiming receipt publication is currently blocked by it.

Global inventory/retirement still detects foreign corruption where its scope requires it. A local
completion shortcut must not silently hide errors from a later global audit. No filename, size or age
becomes deletion authority, and this package does not authorize a collection campaign.

**Closure:** unrelated corrupt registration does not prevent a known run's local completion; corruption
in its own objects still refuses; global inventory remains honest; interrupted local release recovers;
shared/cited/retained objects survive. Preserve storage admission until child cleanup confirms release.

<a id="D08"></a>
### D08 — Make profile status metadata-only and decode on report demand

**Sources:** A-F17. Status reads run metadata and reports capture/child/cleanup/analysis state without
decoding unchanged sampled stacks. Explicit report/view operations own decoding and cache the result
by exact input bytes, pinned reader identity and settings. A changed capture or reader invalidates
analysis; unavailable tools do not make capture failure indistinguishable from analysis not_run.

**Closure:** repeated status does not invoke the decoder; changed inputs invalidate a prior report;
explicit report/view produces the requested analysis; partial captures and decoder errors retain honest
states and logs. Reuse existing run/output ownership rather than a new analysis service.

<a id="D09"></a>
### D09 — Select detailed diagnostics and bound freshness capture safely

**Sources:** O-F43; A-F12; smaller freshness observation. Put per-root events behind explicit diagnostic
selection while preserving coarse phase, terminal and drain observations. Current phase logging added
by concurrent work is a foundation to retain; turning down logs cannot erase failure localization.

Implement the metadata-only freshness capture selected below for supported Cargo/Hakari inputs.
Its equivalence controls cover automatic and explicit targets, build scripts, workspace/cfg
manifests, file existence/path escapes and concurrent additions/removals. If those declarations cannot
be captured faithfully, retain the current conservative capture; record the unresolved optimization.
Do not create an incomplete freshness key to save reading source.

**Decision selected, Interface-checked 2026-10-11:** for supported Cargo/Hakari layouts, retain all
file/directory shape and manifest/lock/toolchain/config/Hakari declaration bytes, with empty source-body
placeholders. Cargo owns discovery, including explicit arbitrary suffixes and empty/glob directories.
Reobserve membership and declaration bytes before/after; concurrent change makes the result stale or
inconclusive. Preserve relevant non-source declaration/config bytes. Unsupported symlink/escaping/config
layouts retain conservative full capture, or its honest refusal; no incomplete key can report clean.
The independent actual Cargo metadata probe passed 19 cases with full metadata/error records, including
valid and invalid discovery and explicit refusal boundaries. A subsequent actual current-checkout
full-body/placeholder comparison over 988 captured inputs exercised whole resolved Cargo metadata,
Hakari `generate --diff`/`manage-deps --dry-run`, and negative generated-dependency/CLI-dependency drift;
all five complete outputs/exit results agreed with stable inputs. Positive commands returned 0 and
intentional drift/edit cases returned 1. Preserve its recorded explicit effective
Cargo flags and final implementation's ancestor/Cargo-home config, declared exclusions and lock
normalization; an ambient duplicate-flag blocked run remains retained separately. This metadata-only optimization
never applies to producer semantic fingerprints or their full runtime-asset/source bytes. Per-root
logging is explicitly selected while ordinary phase/terminal/drain observations remain visible.

**Closure:** ordinary output is bounded by phases rather than roots; diagnostic selection exposes
needed root detail; terminal/drain errors remain visible. Freshness declares changed inputs stale and
preserves unknown/heuristic distinctions; only the equivalent capture optimization may replace it.

<a id="D10"></a>
### D10 — Integrate direct test-process cancellation with actual fixture cleanup

**Sources:** direct-test part of O-F42; A-F09. Use S09's common cancellation/drain primitives but wire
the actual direct nextest/test process entrypoint independently. The outer verification process-group
signal does not prove native transaction drainage or fixture cleanup. Stop accepting work, join/drain
owned work or transfer it to explicit recovery ownership, then release the fixture under its existing
owner. Hard stop or lost ownership remains unknown/protected rather than falsely clean.

Resolve S09's PD02 remote completion question before claiming remote drainage. Keep normal test
parallelism, current request deadlines and shared-service logical isolation; no per-attempt database,
fixed job cap or forced service reset is a substitute for a lifetime contract.

**Decision selected, Interface-checked 2026-10-11:** install the cooperative cancellation owner
inside the actual direct nextest/test process before native admission and fixture use; retain its guards,
streams and completion channel until owned drainage settles. `verify` wrapper signalling and
`surrealdb_fixture.run_attached` establish separate process facts, not this owner. Use S09's selected
minimal server run-transfer target and current recovery-owned unknown disposition when physical remote
terminal proof is lost. Preserve strong `NativeRows` owned transport drainage. Accepted effects remain
journal/control-reconcilable before guard release; hard stop/lost runtime leaves protected unknown
attachments. Independently exercise direct-nextest, CLI and wrapper entrypoints during functional work.

**Closure:** cancellation of a direct fixture-backed process and the verify wrapper have truthful,
separate child-exit/native-finality/cleanup results; accepted effects remain reconcilable; unrelated
borrowers survive; cleanup failures preserve attachments. Pure cancellation controls cannot by themselves
establish real service drainage.

## 3. Ordering and verification boundary

D01/D04–D09 contain useful independent slices. C01 supplies D02's semantic boundary; physical migrations
should land before final D03 qualification so the default applies to the intended dependency closure.
D03 design and pure controls can proceed earlier. D10 shares transport work with S09 but is not closed
by CLI signal handling. Coordinate edits to verify/rerun/run receipts rather than introducing competing
owners or status files.

At implementation time run revealing affected controls and compile checks through the existing owners,
then one deliberately selected integrated functional boundary. The coordinator owns final acceptance.
No repository-wide quality work is created by this documentation task. Performance attribution, host
memory/load causal analysis and real-library campaigns remain external follow-ups; process-local
64-GiB reservations are neither observed allocation nor a host-wide guarantee. RC12 stays in force.
