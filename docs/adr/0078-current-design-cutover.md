---
id: ADR-0078
title: Validate the current design and replace obsolete runtime state
status: accepted
date: 2026-09-28
supersedes: [ADR-0068, ADR-0070, ADR-0072]
superseded-by: null
design: [§B13, §B14, §6.4, §6.5, §11, §11.1, §11.2, §11.4, §14.3, §14.4, §14.13]
evidence: Implemented
revisit: A real external consumer requires a compatibility window or historical data that cannot be reproduced from pinned inputs.
---

## Context

The operator clarified on 2026-09-28 that this project is in its design phase. After validating a
change, execution must pivot entirely to the new design. Preserving obsolete generations, old
runtimes, rollback dumps and compatibility readers has no current consumer. This decision replaces
ADR-0068, ADR-0070 and ADR-0072, preserving their current publication, query and compile-profile contracts
below while replacing their historical-retention, old facet-wire and fixed-view retrieval obligations.
PR4 selection and retrieval details
remain owned by ADR-0077.

## Options

1. Keep every generation, matching runtime, backup and old-format reader. Rejected: this creates
   compatibility and operational work without a product consumer.
2. Validate a fresh current build, stop old readers, switch completely, and delete superseded
   runtime state (chosen). Pinned acquisition inputs make regeneration the migration strategy.
3. Delete the working state before checking its replacement. Rejected: validation must precede
   the cutover, as instructed by the operator.

## Decision

**Accepted target; implementation and qualification tracked in PR4.** Current formats are the only
supported formats. Schema changes may rebuild the project's application database, Delta store and
serving artifacts from pinned inputs. Fresh-schema and current-runtime reconstruction are the
acceptance paths. No mixed-version recovery, old-runtime packaging, preservation of previous ready
generations or historical-record compatibility is required. Version identifiers still detect
incorrect inputs; rejection does not imply a reader or migration obligation for obsolete data.

Once the replacement is validated, quiesce project readers/writers, install the current runtime,
replace the project's persisted state, validate the selected current generation, and delete the
superseded stores, generations, runtime copies, rollback backups and compatibility-only code.
Temporary baseline measurements survive only while used by the current qualification; retain
current conclusions rather than an archive of obsolete product state. Git owns source history.
Unrelated workspace data and source inputs are outside this cleanup.

A backup or cold reconstruction test, when useful, exercises the current design only. Current
publication receipts, source provenance, attempted computations and comparison evidence remain
part of the contracts that consume them; they do not require retaining obsolete runtime epochs.
Durable caches can be rebuilt. No compatibility adapter is justified solely by a previous public
shape; migrate callers to the current typed contract, preserving still-required functionality.

**Compile profiles and identity.** Catalog is the default. Behavioral compilation builds the same
catalog with explicitly selected enrichment and briefs. Mandatory inputs own public roots,
embedding/cache inputs and compiler provenance; optional analysis owns its models and parameters.
Scope uses exact-root-or-dot-descendant membership over final exposures and retains supporting
owners outside the enumeration. Frozen behavioral evaluation and isolated confirmation inputs
remain unchanged.

One schema-owned capability inventory drives publication, import and serving. Not-requested,
produced-empty, unresolved and corrupt output remain distinct. Native value paths require their
whole artifact closure. Optional selection cannot ignore a selected compiler failure; brief
admissibility remains strict. Public member IDs denote exposures independently of improving
binding knowledge. Declarations, overloads, shadowed definitions and provider observations remain
attributed alternatives; ambiguous lookups return choices. Operation IDs never become member IDs.

**Publication and storage.** Delta remains canonical; Arrow declarations own schemas, SQLx/pgpq
owns PostgreSQL effects and codecs, and PostgreSQL is a rebuildable serving projection. Standard
Qwen/1024 embeddings and generation pinning remain. Projection context includes the library,
requirement and complete coverage summary. Import uses generation-scoped advisory locks, committed
row/batch receipts and a frozen transport recipe separate from content identity. Transient
interruption is distinct from terminal validation failure. Shared validators read stored rows back
at the write barrier. Only complete validated content, artifacts and the exact profile become
ready. Runtime DDL remains limited to migration-owned functions.

**Effect and transport boundaries retained from ADR-0068.** `lctx-postgres` owns SQLx effects,
configuration, migrations, COPY, cache/operations and repositories; `cpg-core` constructs canonical
Delta outputs and projections. `lctx_storage` owns async effects over one process Tokio runtime and
lifespan-owned pools. `lctx_semantics` remains pure; this does not accept proposed ADR-0025. FastMCP
pins one generation/profile and never runs the compiler or reads Delta. Database loss is explicit
unavailability. SQLx/pgpq/pgvector and the owned immutable-revision DataFusion provider remain the
selected stack at the pins page's versions; ADBC, Python-owned SQL and SQL-wire serving remain
conditional. This avoids separate readiness/query owners and an unevidenced canonical-store pivot.

Separate migration/application/importer/serving credentials enforce ready-only serving, frozen
content and generation-qualified foreign keys. Bounded COPY uses staging before validated inserts
because row security forbids direct COPY into protected tables. Declared Arrow codecs own schema,
metadata and null meaning; PostgreSQL catalog types cannot redefine them. Native IPC is bounded
and digest checked. Both client families enforce trust, timeouts and combined connection budgets.
Runtime roles cannot alter schemas or ready content. Cache winners are insert-only, read back in a
subsequent READ COMMITTED statement and frozen per attempt. Canonical reconstruction never reads
the mutable cache. Current operational events remain attributed observations; reconciled discovery
indexes do not publish snapshots or establish semantic claims.

Read-only, idempotent, closed-world MCP annotations, masked unexpected errors and stdio transport
remain. Shared Rust requests and generated schemas replace the former semantic Pydantic/`where`
contract. Expected domain validation has one typed route; external embedding availability alone
permits a disclosed lexical-only result. No stdout at import/lifespan, implicit file fallback,
response-cache/response-limiting middleware or unpinned FastMCP installer is introduced. Existing
brief lookup/resource share deterministic hydration; `get_operation`, typed find/search and evidence
expansion retain their current responsibilities. Future registry lookup/explanation remains Proposed.

**Embedding and brief policy retained.** A separate pinned Qwen3-Embedding-8B/vLLM service serves
1024 Float32 cosine vectors; the format-2 spec owns MRL prefix selection before L2 normalization and
both clients send dimensions. Canonical JSON SHA-256 binds model/tokenizer/service revisions,
dtype/pooling, templates/instruction, dimensions, normalization and tokenizer token cap. Documents
have no prefix; every embedded document must pass tokenizer admission, independently of byte
chunking. Rust reqwest and Python httpx2 share request/rejection fixtures; live cosine agreement
>=0.9995 is a separate conformance control. Fake fixtures establish mechanics only.

Cache keys remain full spec plus input hash, regardless of retrieval family. Attempt-owned exact
values feed E0 and briefs and canonical used-vector receipts; addressable retrieval has its own
complete immutable receipt under ADR-0077. No service/cache failure silently selects another
backend. Brief BM25 retains Lucene scoring and lowercased letter/digit tokenization, own-public-name
tokens once, discriminating query words and abstention; best-chunk exact cosine, RRF60/1-based ranks,
brief-ID ties and any-public-symbol promotion remain. ADR-0077 governs member/family fusion and the
new shared instruction. Gold scores never select policy changes. Frozen semantic questions and
analytics parameters remain untouched.

**Retrieval and admission.** Exact remains selected. PostgreSQL owns cosine scores and stable
ordering; independent float64 controls use absolute 1e-5 without fuzzy ties. BM25/discriminating
terms and NumPy RRF remain Python-owned; shared Rust contracts validate final assembly. ADR-0077
owns addressable member/family/channel ranks, promotion and contextual eligibility. Missing query
embeddings permit explicit lexical-only output; database failure never selects another backend.

ANN is unadmitted for the changed retrieval units. Future adoption must qualify the current
content and engine, use disjoint calibration/confirmation requests, compare each active family and
fused result, and demonstrate benefit over exact. Existing criteria remain 99% recall per stratum,
rank-stage p95 <=250 ms and at least 20% p95 improvement in both paired runs. Small/selective queries
and brief search use exact. Candidate underfill, rescoring and fallback need independent bounds.
Physical admission binds cluster/database identity, index definition/validity/filenode, engine and
server/extension versions. Restore or rebuild never inherits admission; shared read locks and
exclusive owned maintenance protect an admitted realization. No historical qualification receipt
can activate a changed index.

**Serving and operations.** Native evaluation remains pure, releases the GIL and runs in bounded
CPU slots without a database lease. Cursors bind generation, request and ordering; native cursors
advance by examined work. Existing pools, bounded diagnostics and PostgreSQL metrics remain; finite
maintenance permits 300 seconds with a 305-second drain, 256 MiB memory and two maintenance workers.
Editable Python and cached Cargo remain the deployment environment, without a wheel gate.

Federation retains its finite expression policy at scans and complete optimizer subtrees. Declared
schemas own Arrow meaning. Mutable operational report inputs are captured in one bounded read-only
repeatable-read transaction and joined to explicit immutable inputs. ADBC remains conditional on a
measured consumer need. Source/catalog facts and still-selected behavioral capabilities are not
removed merely because an older runtime also used them.

## Consequences

The current design has one contract and one runtime epoch. Tests cover current reconstruction,
interrupted imports, corruption, pinning and actual serving; they no longer certify historical
readers or mixed-format restore. PR4's full code/docs gates and both real profiles remain required
before cleanup and operator cutover. This decision records authorization, not successful execution.
The forward plan owns progress and finding disposition. General comparative product admission,
ANN benefit and retained semantic research remain separately unqualified.
