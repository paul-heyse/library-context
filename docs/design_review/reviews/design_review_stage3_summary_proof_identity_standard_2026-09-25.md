# Stage 3 finite summary proof identity — standard design review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | ADR-0034, DESIGN §B5/§B6/§3.4.1/§9.9, `summary_flows` identity and `summary_flow_steps` |
| Standard | Core 2.0; code-intelligence profile 1.0; library-context binding |
| Tier · purpose | Design · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept the bounded ordered proof-path identity for the direct summary base |

The raw-return-fact key could not distinguish later modeled call chains with the same endpoints. This change gives each finite path a canonical ID over callable, input/output paths, transfer kind, condition, exit and ordered typed evidence. I inspected the ID recipe, Arrow contracts, codebook, publication writer, generated references and shared validator, plus the focused direct fixture. I did not inspect an implemented modeled-call or SCC proof variant: neither exists yet. No serving or integrated pilot result is claimed.

## 2. Authority and identity map

| Fact or concept | Authority and fidelity | Revision and identity | Consumer |
|---|---|---|---|
| Raw parameter-origin return | `flow_values`, ty analysis conditions and exit site facts | Provider run, source fact IDs and BDD condition ID | Direct summary seed |
| Step kind | Append-only `summary_flow_step_kind` codebook | Code and label; currently only `raw_identity` | Proof encoder and reference rule |
| Finite summary path | Derived `summary_flows` and ordered `summary_flow_steps` | Snapshot plus content-derived `summary_id`; compiler version 58 | Future L3 composition and FORMAT 7 |

The step's `evidence_id` is kind-scoped: `raw_identity` references `flow_values.fact_id`. Its condition references `analysis_conditions`. It is not a free-text path. A changed step order or evidence fact changes `summary_id`, while an input row-order shuffle does not.

## 3. Contracts and invariants

| Invariant | Enforcement point | Failure behaviour |
|---|---|---|
| Summary ID incorporates semantic endpoints, condition, exit and ordered typed steps | `id::recipe::summary_flow` called by the producer; shared summary equality validator | Reject a forged or missing summary row |
| Each current direct summary has exactly one ordinal-zero raw-identity step | `direct_flow_steps` and shared step equality validator | Reject missing, altered, duplicate or extra steps |
| Step evidence, condition and parent summary exist in the same snapshot | Generated reference rules and snapshot-scoped validation | Reject publication |
| Only implemented kinds can appear | Append-only codebook and generated codebook rule | Reject unknown kind; future kinds require source validation |

The schema and rule snapshots were reviewed; `summary_flows` changed key, and `summary_flow_steps` plus one codebook were added. Old compiler output is not silently reused under version 58. A bounded or missing BDD root remains an unknown summary with its boundary reason.

## 4. Derivation and execution

| Stage / question | Method and dependencies | Exactness, budget, evidence and effects |
|---|---|---|
| Seed: can a local parameter reach a direct body return? | Existing DataFusion `summary_flow_seeds` over raw contributions, return syntax/region and no crossed call | Candidate source fact and condition retained; no source execution |
| Decide path condition | Shared bounded BDD hydration in `cpg-core::summaries` | False drops the path; true/conditional admitted; missing/capped root stays unknown |
| Encode finite proof | Typed ID recipe hashes an ordered sequence; one `raw_identity` step is stored | No path enumeration or new graph; direct case has depth zero and one step |
| Publish | Arrow tables through the current Delta attempt, then shared reconstruction and generated rules | One snapshot boundary; a doctored step is rejected |

There is no topology in this slice. Petgraph SCC work is a later consumer; it must keep canonical IDs separate from graph-local indices and cap path growth. End-to-end cost is unmeasured.

## 5. Journeys

A direct `return value` in a synchronous function yields one parameter-origin fact, one condition-backed summary and one step citing that fact. A second proof with a different evidence fact or order gets another ID even if it reaches the same output. A shadowed/model call, nested return, generator or finalizer still has no positive direct summary. A new step variant is an append-only codebook and schema/validator extension, not a new display-string convention. A missing step fails the shared publication validator; an interrupted attempt has no published snapshot. A future FORMAT 7 reader must validate the same ID recipe before serving its proof.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass | Raw facts own evidence; summary and steps are rebuilt projections. | — |
| G2 Fidelity | Pass, scoped | Step kind, evidence ID and condition remain typed and separate; no modeled-call claim. | Add a checked relation for every future kind. |
| G3 Validity | Pass | Key/reference/codebook rules plus shared full source equality reject invalid rows. | — |
| G4 Hidden behaviour | Pass | Hashing and BDD evaluation read persisted facts; no runtime import or ambient input. | — |
| G5 Consistency | Pass | Version 58 and one validated Delta snapshot carry both tables. | — |
| G6 Transformation | Pass, scoped | Ordered evidence and endpoints enter the canonical ID; row order is excluded. | Verify future SCC path canonicalization. |
| G7 Claims | Pass, scoped | Only the direct raw identity path is implemented and tested. | Keep model/SCC claims Proposed. |
| G8 Library leverage | Pass | Existing BLAKE3 ID mechanism, Arrow schema, DataFusion and BDD kernel supply generic operations. | — |
| CI-G1 Fidelity | Pass | Raw flow fact is not relabelled as an interprocedural completion. | — |
| CI-G2 Evidence closure | Not applicable | No served summary proof exists yet. | Validate in FORMAT 7. |
| CI-G3 Evaluation integrity | Pass | Gold is not part of the derivation or test fixture. | — |

## 7. Findings and principle verdicts

| ID | Finding | Principles · gate | Evidence and consequence | Correction |
|---|---|---|---|---|
| F01, deferred | A modeled-call step needs more than one raw `evidence_id`. | DP-02, DP-21, CI-11 · G2 | Reusing `raw_identity` for a model call would lose call, target and rule provenance. | Append a dedicated kind and typed source relation with shared validation before the first positive model summary. |
| F02, deferred | Ordered path steps may not express a future genuinely multi-parent proof without duplicated prefixes. | DP-08, DP-16 · G6 | A join with two independent premises may require a proof DAG. | Revisit ADR-0034 on the first such consumer; do not encode parent lists in text. |

F01–F02 are outside the direct identity guarantee. DP-01/02/03/04/05/07/08/09/11/13/14/15/18/19/21/22/23/24 and CI-01/02/03/04/06/07/08/10/12 are satisfied in the supported scope. Graph projections, heuristics and serving (CI-05/09/11/13) are not asserted here. No SHOULD exception is requested.

## 8. Library-leverage ledger

| Capability | Bespoke scope | Library or built-in | Fit |
|---|---|---|---|
| Canonical identity | Domain tuple and proof order | Existing BLAKE3 `IdHasher` with length-prefix encoding | Stable for equal inputs; no custom hashing algorithm. |
| Source selection and checks | Domain predicates and witness meanings | DataFusion, Arrow and generated rules | Reuses existing relational/typed machinery. |
| Condition semantics | None added | Pinned BDD kernel | Keeps structural roots; no DNF re-parser. |

## 9. Alternatives

| Alternative | Locality and meaning | Decision |
|---|---|---|
| Raw-return-fact key, witnesses derived later | Smallest current table; collapses future parallel proofs | Reject. |
| Nullable single-call tuple on `summary_flows` | Handles first call, but redefines schema at every path depth | Reject. |
| Generic untyped JSON/text proof blob | Few columns, but loses typed source references and shared validation | Reject. |
| Canonical ID and ordered typed step relation | One source path identity with append-only kinds and bounded rows | Select. |

The existing BLAKE3/Arrow/DataFusion mechanisms are the library-owned and simplest viable implementation of this domain contract. Petgraph remains reserved for actual SCC topology.

## 10. Verification plan

| Claim or risk | Label | Focused evidence | Remaining |
|---|---|---|---|
| Step order and output path affect identity | Tested, 2026-09-25 | `summary_identity_retains_the_ordered_evidence_path` | Future modeled variants not_run. |
| Direct summary carries one matching raw step; omission rejects | Tested, 2026-09-25 | `pinned_identity_models_require_and_publish_their_real_formals`, including step table tamper | SCC and serve-time proofs not_run. |
| Schema/codebook/rules and local static quality | Tested, 2026-09-25 | Focused release Nextest snapshot/reference selections and Clippy after reviewed insta diffs | `just fmt`, `just test-all` and `just pilot` not_run by operator direction. |

## 11. Authority changes and exceptions

ADR-0034 amends DESIGN §B5, §B6, §3.4.1 and §9.9. It changes the summary path's derived identity, not the authority of provider facts or the Delta publication protocol. Binding conflicts K1–K3 do not affect this choice. No exception record is needed.

## 12. Decision

**Accept the direct finite proof identity.** It preserves parallel path identity and typed raw evidence without claiming any unimplemented model-call, SCC or served behavior. F01 is the next functional dependency; F02 is a deliberate revisit trigger. Integrated acceptance remains `not_run` until the complete Stage 3 scope lands.
