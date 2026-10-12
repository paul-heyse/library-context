# SurrealDB lifecycle and integration review evidence

**Source investigation, 2026-10-12.** Consumer: the [principal target review](../../reviews/design_review_surrealdb-lifecycle-target_2026-10-12.md).

## Question and baseline

Which lifecycle taxonomy, ownership boundaries, native mechanisms and development tools best
serve library-context's pinned API/evidence catalog and credible growth? The two external
white papers are hypotheses to examine, not architectural authorities:
[simulation state management](../../../white_paper_on_surreal_db_in_simulation_state_management.md)
and [code intelligence](../../../surrealdb_white_paper_code_intelligence.md).

The inspected clean baseline is `ad1c1020e6ce39fd14c7ff5e506d0197deff894a` on `main`.
The earlier preparation map used `0ad8bf59a72f1685477d94855c1f93ea6762d7b4`;
the intervening commit adds only the two papers. No source change separates those maps.
The core standard is 3.3, heuristics 1.0 and code-intelligence profile 1.5.

During review the user selected SurrealKit 1.0 beta, including its Rust library, as the target
tooling baseline. The concrete inspected version is 1.0.0-beta.1; this does not change the
separately qualified SurrealDB SDK/server version. [Library contracts](library-contracts.md)
retain the capability research and version-specific limitations.

## Method and evidence boundary

The coordinator inspected product/storage owners, current STATUS and coordinator receipts,
host service/fixture/storage composition, configuration, SDK session/terminal source and
server recipe/provenance. A code-mapper traced native lifecycle contracts; a library researcher
checked pinned source and current primary documentation; a fresh design reviewer independently
inspected the assembled architecture and formed the principal judgment. Their assignments
were read-only. The coordinator reconciled material claims and counterexamples before publication.

Navigation used `git rev-parse HEAD`, `git status --short`, `git diff --stat 0ad8bf59..HEAD`,
`rg`, `rg --files`, `sed` and `cat`. Context7 supplied documentation leads; primary documentation
and versioned source qualified capability claims. These commands establish inspection, not
executed product behavior. Supporting source paths and primary links are retained at the grain
used in the principal document and capability notes.

No product builds, tests, database queries, service installation/restart, fixture borrowing,
recovery, migration, selection, retirement or real-library qualification ran for this review.
No benchmarks or quantitative speed claims were produced. Static evidence was sufficient for
the reported structural diagnoses; unresolved runtime semantics remain decision-linked gaps.
Existing shared, dedicated campaign and operator installations were untouched. Historical
test receipts remain dated historical observations, not fresh outcomes or whole-plan acceptance.

Publication validation, 2026-10-12: a focused `python3` link/ID check **passed** for all three new
documents (56 local file targets; unique F01–F05/RC01–RC06 anchors) and the 120-line STATUS bound.
`git diff --check` **passed**. These are document checks, not product qualification.

## Cross-owner facts inspected by the coordinator

| Boundary | Inspected source | What this establishes |
|---|---|---|
| Host borrow versus maintenance | `scripts/surrealdb_service.py:941-1096` | Shared borrower protection precedes generation proof; exclusive maintenance closes admission, waits for attachments, invokes native drainage and retains failure markers. |
| Attachment cleanup versus native outcome | `scripts/surrealdb_fixture.py:209-255,404-416,875-907,1199-1242` | Native owner receipts and command cleanup are classified separately; unresolved native effects cannot be released merely because a child exited. |
| Explicit discarded scope versus original outcome | `scripts/surrealdb_fixture.py:273-401` | An explicit discard/regeneration receipt can settle local obligations while preserving the original native outcome as Unknown. |
| Retained publication versus attachment | `scripts/surrealdb_fixture.py:943-1085` | Named retained retirement has its own campaign-scoped journal and native job; ordinary attachment release does not retire publication content. |
| Host executable versus database content | `scripts/storage_service.py:1-5,175-216` | Executable dependency retention is separate from native content lifetime and protects unresolved handoffs. |
| Transport versus SDK session | `third_party/surrealdb/src/lib.rs:338-357` | Cloning shares `Inner` but issues a new session identity; it does not preserve the original transaction's session. |
| Exact transaction finalization | `third_party/surrealdb/src/method/transaction.rs:40-50` | Borrowed cancellation and invalidation retain the actual owning session; failed cancellation is not acknowledged by invalidation. |
| Application terminal versus physical stream | `third_party/surrealdb/src/engine/remote/grpc.rs:44-90` | The patched adapter polls through physical EOF and preserves late typed errors. Source existence does not qualify a composed runtime journey. |
| Supported binding versus database semantics | `crates/lctx-native/src/config.rs:55-85` | Product configuration currently restricts managed loopback gRPC and main/validation. Restriction alone is not a defect without a relevant consumer or change scenario. |
| Build recipe versus installed server | `scripts/surrealdb_server.py:42-108`; `third_party/surrealdb-server/grpc-drain-provenance.json` | Current recipes can include server execution drainage in addition to timeout repair. That does not establish which generation an existing installation runs. |

## Interpretation constraints

The schema installation marker, catalog format, executable definition epoch, service generation,
publication realization and client build identity are distinct. They must not be collapsed into
one version or treated as evidence of each other's compatibility.

Missing native receipts are not automatically a demonstrated safety defect: inspected compiler,
cache, product and retirement owners register durable Started receipts before their native
requests. This is contrary evidence to a superficial reading of the host's `not_observed` branch;
it is not a repository-wide proof about arbitrary direct control callers.

Native references, events, database disposal and schema tooling must be evaluated as composed
protocols. No individual feature establishes remote work finality, semantic admission,
multi-call evidence closure or safe reclamation of shared content by itself.
