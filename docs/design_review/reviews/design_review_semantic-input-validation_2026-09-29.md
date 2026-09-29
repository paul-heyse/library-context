# Typed input and stored-content validation — bounded change review

Reviewed 2026-09-29 by independent read-only reviewer Herschel against Core 3.0,
code-intelligence 1.1 and the repository binding. Change/conformance scope: `db0a52c` plus
input/source, binary streaming and invariant corrections in the working tree. Attribution,
attachment indexing, producer integration and enclosing architecture qualification are excluded.

**Outcome: bounded corrections accepted; resource qualification remains open.** This is source
inspection, not a phase exit. Reviewer commands: **not_run**. Author's focused command
`python3 scripts/build_environment.py -- cargo test --release -p lctx-model --test domain -p lctx-postgres --test generations`
**passed** on 2026-09-29: 14 domain controls and one real PostgreSQL lifecycle integration test.
Current disposition belongs to the [cutover plan §8](../../plans/semantic-model-cutover-plan_2026-09-29.md#8-findings-disposition).

## Responsibilities and expected changes

The domain owns content identity and validity; generated layouts preserve typed records; storage
streams sealed contents through declared validators and binds receipts to published content.
Adding a cross-relation invariant should change its domain declaration and tests, without domain
branches in storage. Acquisition provenance must remain distinct from content revision identity.

## Findings and correction evidence

| ID | Original defect and consequence | Correction inspected |
|---|---|---|
| F01 | High: stored artifacts were not reconciled with the input manifest; changed or missing evidence could publish | `InputManifestCheck` compares complete stored membership and bytes-derived digests, rejects duplicate paths, and handles empty inputs. SQL C collation matches Rust path ordering. Changed/missing/extra/duplicate controls refuse publication. |
| F02 | High: row bound applied after SQLx allocation; no coordinated memory envelope | Generated admission checks cap row wire size before reads, validate array shape, and COPY buffers individual rows. Conversion copies, producer batches and invariant maps still need coordinated allocation accounting and measured qualification. |
| F03 | Medium: acquisition verification, file ownership and usage roles missing | `DistributionVerification`, `ArtifactOwnership`, `ArtifactUse` preserve these relationships; original bytes and derived text/stub/package properties survive. Real PG positive covers two distributions and distinct uses. |
| F04 | Medium: occurrence spans could exceed their source bytes | `OccurrenceBounds` checks half-open byte spans against the referenced artifact. The stored out-of-range control specifically fails this validator. |
| F05 | Medium: independent FKs allowed artifact input A to cite verification for input B | `OwnershipCheck` resolves acquisition inputs and requires equality with artifact input. Real PG negative includes two valid inputs and all referenced rows; same-input multi-distribution ownership remains accepted. |

Publication checks exact relation/validator identities and content/model/physical digests, rather
than counts. A substituted validator receipt is rejected. The reviewer found no new hash/order
problem in these corrections. Immutable application access is the trust boundary; owner/superuser
hostility is not claimed to be resisted.

## Architectural judgments and gates

A1 localizes changes in the domain. A2 now encodes the inspected acquisition and span constraints.
A3 allows additional invariants through declared dependencies and the common runner. All three
are satisfied within this correction. Original manifest/span/ownership G1/G3 defects are corrected;
G5 resource qualification remains unresolved. Attribution fidelity, serving evidence closure,
evaluation isolation and final P0–2 architecture remain outside this review.
