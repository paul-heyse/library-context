# Snapshot model, structural sharing and cross-version diff

**Role.** Supporting analysis for the principal review
[design_review_surrealdb-graph-store_2026-10-05](design_review_surrealdb-graph-store_2026-10-05.md).
It answers brief decision 3, "should the generation be redefined, whatever the store?", and the
operator's follow-up question on typing as graph metadata (§7), at bounded depth. It carries no
overall verdict and no disposition ledger. Findings cited here (F05, F06, F07, F12) are owned by
the principal review.

**Baseline.** Repository `f66eb15a` (code identical to `1158ebe2` for the cited paths),
2026-10-05. Consumed context:
- Phase A lanes [`a3b-snapshot-representations.md`](../evidence/2026-10-05_surrealdb-pivot-lanes/a3b-snapshot-representations.md) and [`a5-obligation-map.md`](../evidence/2026-10-05_surrealdb-pivot-lanes/a5-obligation-map.md);
- Phase B `docs/design_review/evidence/2026-10-05_snapshot-diff/` (W2) and
  `docs/design_review/evidence/2026-10-05_surrealdb-native-realization/` (W1);
- source read by this reviewer: `crates/lctx-model/src/domain/model.rs:738-756` (`hash_rows`),
  `crates/lctx-model/build.rs` (model contract capture), `scripts/producer_fingerprint.rs:37-60`,
  `crates/cpg-extract/src/bundle.rs:28-48` (`build_digest`), `crates/lctx-model/src/domain/memory.rs`;
- owners: DESIGN §15.3, §15.11, `storage-and-publication.md` §6.1–§6.3, ADR-0086, ADR-0105, ADR-0126.

Evidence labels follow core principles §D. No timing is used.

---

## 1. Separate the intent from today's mechanism

The snapshot intent the brief states, and that §15.11/§6 and CI-13 require, has six parts. The
current mechanism realizes each one as follows; the third column is what a redefinition must keep.

| Intent | Current realization (Implemented) | What any redefinition must still provide |
|---|---|---|
| Complete, closed, attributable state for pinned inputs and producers | One schema per attempt; frontier admission; receipts bind model, schedule, coverage and content digests | A root that names input, producer, model and validation-definition identities plus a per-relation membership and digest |
| Never visible partially or before validation | Staging schema, grants issued only at publication | Readers can resolve only validated roots |
| Immutable once published | Privilege: writers revoked at seal; readers have SELECT only; ADR-0126 trusts this "under legal service operations" | No legal service operation can change what a published root contains |
| Pinnable by readers | Advisory-lock lease checked against model/physical digests | A pin that retirement or GC must honour |
| Reproducible | Rebuild from pinned inputs; content identities | Unchanged |
| Selection separate from publication | Control-schema selection pointer | Unchanged |

Nothing in this list requires full copies, a schema per generation, or privilege phases. Nothing
in it requires structural sharing either. The question is whether sharing, diffs or a different
membership representation would *deliver* something the product needs, at what model cost.

## 2. What structural sharing would actually share today

Two current identity choices decide the answer before any store is chosen.

1. **Provider identity covers the whole workspace.** `build_digest` hashes
   `LCTX_PRODUCER_SOURCE_DIGEST`, the lockfile and both analyzer patches
   (`cpg-extract/src/bundle.rs:30-45`). The source digest walks every crate's `src`, `sql`,
   `migrations`, `models` and `build.rs`, plus `specs/` and `third_party/`
   (`scripts/producer_fingerprint.rs:37-60`). Every support is keyed by its run, and the run by
   its provider (A5 fact 3). So any edit anywhere in `crates/`, including `lctx-postgres`, or a
   lockfile move, gives every attributed observation, support and assertion a new id.
   **Implemented, source-inspected 2026-10-05.**
2. **Source identity includes the input revision.** `SourceArtifact` is keyed by
   (input revision, path, content hash); `Occurrence` by (source, span, kind, role, path);
   `Source` callable/class/parameter entities by their declaration occurrence (W2 probe 2,
   static). A new library version therefore re-identifies every source, occurrence and source
   entity, including those in unchanged files.

Consequence (Interface-checked reasoning; the real two-generation measurement is pending):
- across **binaries**, sharing of provider-attributed facts is about zero;
- across **library versions**, sharing of source-anchored facts is about zero;
- sharing is complete only for the same binary on the same inputs, which is a recompile nobody
  needs.

W1's 99% sharing and W2's 99.1% logical sharing come from synthetic perturbations that keep ids
stable where the real key chain would not. They prove the *mechanisms* (identity records,
payload records, membership, diff) are correct; they do not establish a sharing ratio for this
model. The pending real-generation diff would confirm or contradict this reading; a high real
sharing ratio across binaries would mean the key chain above is not what governs ids, and would
reopen this section.

## 3. Membership representations, measured qualitatively

W2 ran four representations against an independent DuckDB reference on 612k model-shaped rows
(2026-10-05, synthetic v1/v2, all `passed` for exactness):

| Representation | Store result | Consequence for this model |
|---|---|---|
| M0 full copies | PG `EXCEPT`/full outer join exact | Today's shape; simplest; no lifecycle mutation |
| M1 explicit membership rows | PG and SurrealDB exact; PG size after `VACUUM FULL` 92 MB content + 151 MB membership vs **168 MB for two full copies** | Does not save storage for narrow, id-dominated rows |
| M2 validity intervals | PG `int4range` and SurrealDB `lo`/`hi`, and W1's `spans`, exact; 110 MB | Saves storage; requires a linear history; **mutates a lifecycle column on shared rows that published snapshots read** |
| M3/M4 prolly/layered (Dolt, TerminusDB) | Exact diffs; Dolt rewrote ~40% of chunks for ~1% scattered churn; TerminusDB holds each branch in memory; Dolt commits FK-violating rows under bulk mode | Native Git semantics, but no graph or relational advantage that the product needs, and new integrity traps |

The M2 point matters for the guarantee. In W1, content fields are `READONLY`, but the loader
routinely rewrites `spans` on records that a published snapshot reads; abort reopens spans
closed at S. A published snapshot's view is therefore immutable **by loader protocol**, not by
any store-enforced property. That is weaker than ADR-0126's "no legal service operation can
mutate an acknowledged frame". It is compensable only by a per-snapshot membership digest that
readers verify, which costs a scan. This change would have to be made explicit if M2 were chosen.

## 4. Validation does not become incremental by sharing

W2 probe 3 classified all 119 checks (106 invariants + 13 publication checks) by reading
`visit`/`finish` (static, 2026-10-05):

| Class | Count | Delta-checkable |
|---|---|---|
| R forward reference / row predicate | 21 | Yes forward; deletions need reverse indexes |
| K key-local parent groups | 16 | Yes per parent key |
| F multi-hop forward closure | 20 | Mostly, with reverse indexes |
| G global / closed-world | 11 | No |
| D derivation replay (`output == f(inputs)`) | 51 | No; cost equals rerunning the stage |

The 51 replays dominate and set the validation cost. Sharing removes copy and storage, not
validation. Incremental validation would need partitioned derivations memoised by
(definition digest, partition-input digest): incremental view maintenance or salsa-style
memoisation. No candidate store supplies that; §14.11 already keeps Salsa behind a measured
trigger. This also bears on the principal review's F01: the replays run in `finish()` while a
transaction is open, and on real data they are long.

## 5. Model changes worth making whatever the store

These follow from the product and the scenarios, not from a store choice. Each is a model
decision through ADR; none needs SurrealDB, Dolt or TerminusDB.

### 5.1 Cross-version correspondence keys (principal F06)

The product is version-pinned; the plausible next capability is "what changed in this API
between releases". Today the diff can only be by identity, where a changed parameter list is a
removed plus an added `Signature` (key includes the parameters digest, `calls.rs:296-312`) and an
edited file re-identifies everything in it.

Proposed model addition:
- a derived, per-generation relation `ApiCorrespondence` (name illustrative) mapping public
  members to a declared correspondence key: (access-module qualified name, public name, member
  kind, stub flag); parameters keyed by (member correspondence key, parameter name, kind);
- ambiguity is explicit and many-to-many (overloads, conditional redefinitions, property
  accessors, `.py`/`.pyi` pairs), never resolved by choosing one;
- the key is derived from existing facts (`PublicNameObservation`, module `qualified_name`,
  `ProviderSymbol.native_key`), with no cross-provider spelling join (the model forbids it).

The comparison is then a keyed full outer join over two pinned generations, classified as
added / removed / changed / unchanged / ambiguous, with each side's evidence. That join is native
in PostgreSQL and DataFusion; in SurrealDB it is two keyed reads merged in the client (no JOIN).

Two contract questions belong to the ADR, not to a store:
- ADR-0105 forbids linked external generations, and CI-13 requires a serving process to hold one
  generation. A comparison is either a compile-time derived comparison generation whose pinned
  inputs are two published generations (keeps CI-13; needs ADR-0105's linked-generation clause
  revisited for read-only inputs), or a serving process that pins an ordered pair and names both
  (needs CI-13's "one generation" restated as "one named, pinned snapshot set").
- What "changed" means per field (signature, default, documented option, evidence) must be
  declared, so a moved span alone is not a semantic change.

Remedy maturity: Proposed. W2 static review found no declared key today; key uniqueness on real
data (provider, `native_key`, role, variant, form; module `qualified_name`) is pending and can run
on the facts layer that now exists.

*Coordinator note, 2026-10-05, after the review was written (W2 real facts-layer run, principal
§10.2).* `native_key` is an analyzer-positional index (`F:n`, `CF:class:n`), neither unique nor
stable across versions. It cannot serve as, or feed, the correspondence key. The public access path
(module, name) was unique (4,720). A syntax-containment qualified path was unique for classes
(1,995/1,995) and nearly unique for callables (13,862/13,950; the collisions are deliberate
same-name redefinitions, which the explicit many-to-many ambiguity above already covers). This
supports the access-path key and removes `native_key` from its derivation. Cross-version stability
still needs a second compiled release.

### 5.2 Identity scope (principal F05)

Narrow two identities:
- **Model contract digest.** `build.rs` captures every `.rs` byte of `lctx-model` and the macros
  plus `Cargo.lock`; `store install` refuses on mismatch (A5 fact 4), so a comment edit or a
  kernel bugfix resets the operator store. The contract the store depends on is the declaration
  and validation-definition encoding, which ADR-0124 already frames explicitly. Keying the store
  contract on that encoding would make a store reset follow schema and definition changes only.
- **Provider build identity.** Derive it from the provider's actual crate closure, analyzer
  patches and the lockfile entries of that closure, not from the whole workspace and `specs/`.

Challenge case (a legitimate change the narrowing must not hide): an edit to a model codec or
normalization that changes the bytes a provider emits must still change provider identity. The
provider's crate closure includes `lctx-model`, so a codec edit stays inside it; a `lctx-postgres`
or `lctx-mcp` edit does not. CI-10 still records the conservative fingerprint as run context; it
need not be an identity input. Remedy maturity: Proposed; G6 risk if under-narrowed, so the ADR
needs a revealing control (edit a codec, expect new provider ids; edit `lctx-postgres`, expect
identical fact ids).

### 5.3 Set or Merkle relation digest (principal F07)

`hash_rows` (`model.rs:738-756`) is a sequential hash of (id, payload digest) in strictly
increasing id order. A Merkle tree over sorted ids, or a multiset hash over per-row digests,
gives the same equality and order-independence with O(Δ·log N) or O(Δ) update and lets two
generations prove a relation identical without rescanning. Its consumers would be relation-level
receipt reuse across generations (ADR-0126 bindings keyed by definition and input digests) and
the comparison in §5.1. Prefer the Merkle form: additive multiset hashes are cheaper but have
weaker collision properties. Low priority until a reuse or comparison consumer exists.

### 5.4 What not to change now

- **Do not redefine the generation as structurally shared.** Until 5.2 lands, sharing is about
  zero; even after it, M1 does not save storage for these row shapes and M2 trades a published
  immutability property for space. Revisit when (a) identity narrowing is implemented and (b) a
  measured real two-generation sharing ratio is high **and** storage or rebuild cost is a named
  problem (operator memory: robustness over disk space).
- **Do not adopt VERSION time travel** (SurrealDB): wrong results with indexed predicates under
  VERSION, knob ineffective (W1 2026-10-05), global retention, timestamp addressing.
- **Do not adopt Dolt or TerminusDB** as the snapshot store: they deliver Git semantics the
  product does not need beyond §5.1's keyed diff, and add integrity traps (Dolt bulk-mode FK
  commits) or resident-memory risk (TerminusDB).

## 6. A mechanism-neutral statement of the generation

Whatever store follows the principal review's F12 decision, the contract can be stated without
naming PostgreSQL schemas or privileges:

> A generation is a validated **root**: a manifest naming input, producer, model-contract and
> validation-definition identities, and for each declared relation its membership digest and
> count. A root becomes readable only by publication after validation; no legal service operation
> changes a published root's membership or content; readers pin a root and retirement honours
> pins; selection is a separate pointer.

This wording keeps every guarantee in §1 and lets either the current per-schema realization, an
in-PostgreSQL simplification, or write-once content-addressed segments implement it. It is a
proposed amendment to §15.11 and ADR-0086's realization clauses, decided together with F12.

## 7. Typing as graph metadata: the operator's representation hypothesis

**The hypothesis.** Typed relations are a mechanism for keeping actions aligned and contract-safe,
not a functional requirement. If information and analysis lived in one graph, typing could be
graph metadata (node and edge kinds with property schemas), and the graph framework would satisfy
the alignment need with fewer relations and less code.

**Baseline (A7, static, 2026-10-05).** 878 declared relations: 81 entity, 83 binary, 79 n-ary;
153 observation/support/attribution (75 supports share one 7-field shape: assertion, run,
surface, evidence, origin, mode, fidelity); 335 lifecycle/proof/receipt/coverage (150 generated
by `owner_table!` as 15 owners × 10 suffixes); 59 vocabulary/value/tagged unions; 8 blobs; 80
projection/analytics. At most 47 relations are natural binary edges; 79 are functional
(`record<>`-link shaped); n-ary and discriminated relations need edge records with identity in
any store. The relation-level reference graph is a DAG apart from four small SCCs. W1's
SurrealDB schema used only `TYPE NORMAL` tables with `REFERENCE` fields: no `TYPE RELATION`.

### 7.1 Would a node/edge-kind model meet the principles with fewer relations and less code?

**Fewer *kinds*: no, not by moving to a graph.** In SurrealDB a node or edge kind with a property
schema *is* a SCHEMAFULL table; W1 needed 1,203 of them. A kind is the same concept as a relation;
calling it graph metadata does not reduce the count. A7's shape census shows why: only ~5% of the
relations are binary edges, and 63% are attribution, lifecycle, vocabulary or blob records whose
references qualify a node or edge rather than connect domain nodes. A graph store models these as
records with links, exactly as W1 did.

**Fewer *physical* relations: yes, on any store, by representation.** Two families are uniform:
- the 75 supports differ only in the nominal target of `assertion`;
- the 150 lifecycle tables differ only in their owner.

Each could be one physical relation with a discriminator (support: `(observation kind, id)`;
lifecycle: `owner`). Because `Id` encoding includes the nominal type discriminator (§15.3), a
reference sum can lower to (tag, id) rather than one nullable column per arm, which also removes
most of the 1,239 nullable id fields (A7). That would cut physical objects by roughly a quarter.

**Less authored code: little.** Supports are generated by one `#[assertion(support = …)]`
attribute per observation family, and lifecycle families by one macro. The authored code is
already the generic form; the 878 count is the lowering's output, not hand-written declarations.

**What the 878 actually are: authority units.** Each relation is a unit of single-writer
authority (stage table: "every ordinary output has exactly one writer"), receipts and content
digests, read grants, vocabulary prefixes and nominal reference targets (ADR-0105, ADR-0108,
ADR-0126). Supports are written by different provider stages; lifecycle families by different
owners. Merging them physically requires the authority to move to (relation, discriminator)
partitions. The logical count stays; only storage merges.

Against the principles:
- **DP-01, DP-02, CI-02.** Met if the Rust declaration stays the authority and consumers see
  typed views (`Support<O>` over a discriminated table). Violated if consumers receive generic
  records and dispatch on kind strings (core §G "Everything is generic").
- **DP-03, CI-04.** Subtype membership of a (tag, id) reference becomes a runtime check rather
  than a nominal `Id<T>` column; the model already does this for 28 subtype references, so the
  mechanism exists, but compile-time target typing weakens to runtime checks for these families.
- **CI-03.** Unaffected: every row keeps its content key; discriminators separate parallel facts.

### 7.2 Where the authority lives

The analyses stay in Rust (no in-database fixpoints, BDD algebra or analytics; A2, A3, W3). They
need nominal types (`Id<T>`, typed rows, sum arms) at compile time. Therefore:
- **One declaration can generate both** the store schema (PostgreSQL DDL today; SurrealQL in W1's
  `gen_schema.py`) and the Rust access. That is today's architecture (§B2); the store schema is a
  lowering, whether tables or graph kinds.
- **Making graph metadata the authority** would either create a second authority (Rust types
  maintained beside graph kinds) or invert generation (Rust generated from the graph schema). The
  inversion fails because SurrealQL cannot express the content-identity recipe (BLAKE3 typed key,
  W1: "not expressible"), the 231 invariants and 39 publication checks (opaque Rust), derivation
  rules, stage ownership, projection roles, or sum types beyond flat predicates (W1 parser-depth
  trap). Graph metadata alone would degrade to convention for everything it cannot express.
- **Store-side enforcement** (SCHEMAFULL, `ASSERT`, `record<t>`, ENFORCED relations; or PG CHECKs)
  is defence in depth and fail-fast at write time. It cannot be the authority: `record<t>` does not
  check existence on write, `OPTION IMPORT` skips `ASSERT`, ENFORCED is deferred under import, and
  the model's validators must run over stored contents regardless (W1 p2.seed results).

### 7.3 Representation versus store

| Reduction | Available on PostgreSQL | Available on segments | What graph-native records add |
|---|---|---|---|
| One support relation per family group, typed views in Rust | Yes; per-table privileges no longer separate writers, so authority needs partition-level enforcement (collides with F12's privilege model) | Yes; files per authority unit are cheap, so merging is unnecessary | Nothing specific |
| One lifecycle relation per suffix, owner discriminator | Same as above | Same | Nothing specific |
| (tag, id) lowering for reference sums | Yes (two columns; ids are type-discriminated) | Yes | `record<a\|b\|c>` is native, with `<~(t FIELD f)` reverse lookup without declaring an index |
| Payload/identity split for sharing | Yes (W2 M1) | Yes | Nothing specific; W1 did it with ordinary tables |
| Reverse-reference navigation | Needs lookup indexes per field (`ddl.rs:391-406`) | Needs built indexes | Native `<~`, which is the one representational convenience SurrealDB adds |

### 7.4 Second-order effects

Fewer physical relations would reduce per-generation DDL, indexes and CHECKs, the number of
receipts, frames and digests (not rows scanned), store reset time and the fixed per-generation
cost (Proposed). Validator count is unchanged: invariants are semantic rules, not per-table
checks. Serving hydration is unchanged in hop count, because hops follow logical paths. The risks:
- weaker compile-time target typing for merged families (runtime subtype checks);
- discriminator dispatch code in consumers, unless typed views are generated from the declaration;
- consumers reinterpreting kinds if the generic form leaks past the store adapter;
- partition-level writer, receipt and read authority, which is new machinery on PostgreSQL.

**Assessment.** The operator is right that typing serves alignment and that the store's typing is
a lowering, not the authority. The authority must remain one Rust declaration because the analyses
stay in Rust; graph metadata cannot carry what the model declares. The attainable reduction is a
**lowering decision** (logical relation ≠ physical relation) available on any store, and it is
worth making only together with the F12 decision: under R1 (PostgreSQL) it costs partition-level
authority; under R2 (segments) it is mostly unnecessary, because per-unit files have no DDL, grant
or index cost. Graph-native records add native polymorphic links and reverse navigation, which do
not offset the reasons decision 1 rejects SurrealDB.

Route back: [principal review](design_review_surrealdb-graph-store_2026-10-05.md), §2.4, §7 (F05–F07, F12), §8.2 and §11.
