# Catalog execution speed: integrated private compiler review

**2026-10-18 · design / target · Accept at Implemented strength.** Core/template3.3,
efficiency heuristics1.0 and code-intelligence profile1.5, under the repository binding.
Baseline`d8a842fc` plus the SC00–SC04 implementation under ADR-0191. Independent implementation
and design reviewers assessed source; the coordinator reconciled their findings and corrections.
The [coordinator §6](../../plans/catalog-execution-speed-plan_2026-10-18.md#6-sole-disposition-and-current-checkpoint)
owns current dispositions and actual verification, including any subsequent failures. This dated
assessment supplies no native, whole-plan or measured-performance qualification.

## Scope and judgment

The inspected scope comprises private completed backing, shared resources and spill, coherent
producer and input-set visibility, dependency-ready preparation, compact demand, pure semantic
properties, genuine retained-product reads, requested-frontier lowering and independent native
readback, CLI allocation and composed finalization. Required native boundary slices are included;
the remaining SN/AA/AX/AD packages, full FastMCP benefit campaign, embedding inference and submitted
MCP runtime are outside this judgment. Reviewers ran no builds, tests or services.

The selected responsibility split is sound. Model declarations own canonical rows, meanings,
exact consumed inputs and explicit outcomes. Private sources own immutable working content;
actual native owners independently earn persistence, selected-view and publication authority.
Ordinary CLI compilation allocates its target after private execution. A configured cache read
has its own genuine held source and finality; copying its fully checked rows does not manufacture
a receiver attempt or an installed-view capability.

Memory-first execution uses the existing shared64GiB default and ordinary available parallelism.
Arrow clones and slices retain the original backing reservation through`Bytes::from_owner`.
Compact canonical indexes preserve full row equality for duplicate IDs. Fragmented canonical
windows use bounded Arrow interleave instead of emitting a native write per tiny fragment.
Contingency IPC is an internal representation, never an admission certificate or mandatory export.

The pressure route uses the same actual pool: one nonrecursive callback, an unwrapped base budget,
pre-admitted encoding workspace, weak source/index registrations and nonblocking reclamation locks.
Active borrowers retain their charges after representation replacement and may legitimately prevent
immediate reclamation. No replacement engine, stage allowance, worker cap or general backend
framework is warranted. Pinned DataFusion55.1.0, Arrow59.3.0 and bytes1.12.1 supply the execution,
IPC, interleave and backing mechanisms used here.

Pure composition occurs outside visibility exclusion, rebasing only changed predecessors before
atomic whole-producer publication. Frozen input sets preserve exact semantic epochs independently
of mutable current unions. Ready preparation runs as owned work; unrelated setup need not finish
before another ready stage can prepare. Pending initialization retains terminal results and actual
joins even when all delivery waiters disappear.

Early demand preserves Catalog's existing compact behavioral domains and extends compact production
to wholly disabled analytic/embedding owners before rich preparation. Authored settings, complete
domains, unknown coverage and NotRequested remain meaningful. Native completion and independent
receiving reconstruct logical content from actual stored typed rows; physical envelope hashes keep
their separate storage role. Completed-state5, logical contribution identity3, source identity2 and
native schema11 explicitly mark this cutover, without compatibility readers.

## Findings corrected during integration

The first independent pass identified six implementation defects. The subsequent design pass
identified two additional boundary defects. The corrections below were inspected as source;
their revealing controls still require the root's coordinated verification checkpoint.

| ID | Diagnosis and correction inspected | Responsible owner / revealing control |
|---|---|---|
| SC-R01 | Validation took visible then exact registries in the reverse order from completion. Snapshot under the completion gate using owners→exact→visible→frozen order, releasing all locks before validation. | Workspace / queued validation and completion at the same atomic boundary |
| SC-R02 | Canonical interleaving across original segments fragmented reads into tiny batches. Bounded gather now coalesces fragmented windows while useful contiguous runs retain zero-copy reads. | Private source / interleaved multisegment canonical output and transfer shape |
| SC-R03 | Writer-only spill after a failed8KiB index fallback could not release payload usefully at exhaustion. Actual allocation/growth pressure now invokes pre-admitted source reclamation. | Shared budget and private source / ordinary reservation triggers IPC without an explicit spill call |
| SC-R04 | Same-model batches from a foreign pool could become resident outside the operation allowance. Actual budget grants now retain pool identity and private ingress refuses foreign grants. | Model reservation and private writer / same-model foreign-pool ingress |
| SC-R05 | Independent ready upper stages still awaited launch preparation serially. Owned preparation events now progress independently, with coordinator-only integration and unchanged genuine dependencies. | Compiler / held earlier preparation cannot prevent a ready peer completing |
| SC-R06 | A public arbitrary mutable provider could receive memoized validity from its unchanged pointer. Private bindings now retain a sealed immutable source, derive their provider, and verify the actual source/pool correspondence. | Private validation / source substitution and foreign-pool refusal |
| SC-R07 | Equal-count different-content sources could receive the same public property identity under caller-authored snapshots. Property identities now include actual sealed content/count; typed EOF compares full content. Compact contributor grants require actual singleton output/descriptor agreement and exact canonical-union agreement. | Private validation / equal metadata with different actual content and altered compact contributor output |
| SC-R08 | Captured private DataFusion plans retained backing but bypassed operation cancellation/execution ownership. Compiler exposure now retains the actual execution owner and checks cancellation at alias resolution, scan and stream polling. Generic immutable storage stays independent. | Completed inputs / captured plan and stream after cancellation, optimized-away query and last-owner release |
| SC-R09 | Private and native predicate memos were separate: sharing the native memo did not transport actual private successes. A charged operation-local ledger now retains opaque whole-input proofs and distinct scoped/empty-domain terminal results. Producing admission adopts an exact predicate only after complete actual native/private typed-row equality under current held capabilities, including genuine uniform qualification dependencies. | Private validation and native scoped admission / context and failed-prefix controls, batch-independent actual equality, and counted whole/scoped native transfer controls |

SC-R07 does not describe forged native authority: native lowering and independent incoming admission
already check their real content. It corrects false private exact-source attribution. SC-R08 preserves
backing after registry release while denying further compiler reads after cancellation. Neither fix
adds a synthetic authority flag or a replacement native owner.

SC-R09 was identified during the native functional follow-up. Independent source re-review accepts
the corrected transfer, including effective uniform context, extra qualification capabilities and
their revalidation, at Implemented strength. Explicitly uniform-independent scoped predicates may
omit unused compact premises; whole proofs retain their actual complete uniform context. Every
unmatched final obligation still executes, and unattached incoming admission receives no private
evidence. The first native transfer control remains unexecuted at the operator-requested closeout;
coordinator §6 distinguishes the latest compiled/pure state from that remaining native obligation.

## Scenarios and preservation constraints

A new analysis over existing facts should change its model-owned declaration and kernel without
adding native lifecycle work to private handoffs. A backing substitution must preserve exact rows,
canonical order, content and borrower charges. Overlapping producers may rebase stale pure preparation;
uncertain accepted native effects must retain their original recovery duty. The inspected boundaries
provide these routes after the corrections above.

Preserve explicit completed-empty versus missing versus NotRequested, all supported fact families,
ordered roles and epochs, actual complete EOF, original evidence and unknown domains. A private
snapshot, serialized eligibility definition or matching digest does not grant installed authority.
Requested outputs still require genuine materialization, actual native ownership and independent
admission; export/import remains explicitly requested transport. Ordinary publication commits an
unselected manifest, recording its identity before later acknowledgment/finalization can fail.

Compared with persisted private handoff, the selected route removes unnecessary writes, bindings
and rereads. Compared with all-RAM execution, contingency spill provides a pressure route without
making disk an ordinary stage. Remaining index, equality and assurance machinery has a concrete
semantic consumer. No historical timing subtraction, latency forecast or minimum benchmark campaign
is needed to accept that structural argument.

## Gates and evidence boundary

A1–A4 are satisfied within the inspected scope at **Implemented / source-inspected** strength.
The initial G3/G5/G6 failures associated with SC-R07/08 were independently rechecked after correction;
no remaining material defect was identified in those changes. G1/G2/G4/G7/G8 pass within this scope.
CI-G1/CI-G2 pass for the inspected fidelity/evidence-preservation transformations; CI-G3 is not
applicable because evaluation inputs and gold routing did not change. No unexamined serving claim
is certified. No additional rule impact is needed beyond the approved cutover and ADR-0191.

Focused functional controls, full compile/lint and applicable qualification remain implementation
acceptance obligations owned by coordinator §6. Earlier pre-integration passes retain their exact
scope. Builds/tests/services by the independent reviewers were **not_run**. This review makes no
Tested, Measured, end-to-end speedup, operator-adoption or enclosing-subsystem acceptance claim.
