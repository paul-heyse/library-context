# Design review: increment 1, slice 2 (Stage C/D derivations, generated validators, publication) (compact)

**Date:** 2026-09-22 · **Depth:** compact · **Mode:** code plus DESIGN.md and ADR-0008. This is the
end of a slice that adds derived tables and validators, so ADR-0001 owes a `compact` review.
**Reviewer:** `design-reviewer` subagent (fresh context) · **Author:** the session that wrote
`3448a50` and `68120ae`
**Prior review:** `design_review_inc1-slice1-schema-extract-delta_2026-09-22.md`. Its §7 Q5
conditions (a)–(d) for ADR-0008 are judged in §7.2. This review's findings are F1–F5 and O1–O9.

## 1. Decision and scope

**Target.**
- `3448a50`:
  - `cpg-schema`: `derived.rs` (six derived tables and their SQL) and `rules.rs` (the rule
    generator and `REFERENCES`);
  - `tables.rs` additions: `snapshots`, `public_names.origin_module_node_id` and
    `pysa_functions.signature_count`;
  - `cpg-core`: `snapshot.rs`, `derive.rs`, `validate.rs`, `attempt.rs` and `lib.rs`;
  - the tests in `cpg-core/tests/compile.rs` (with 3 snapshots) and `cpg-schema/tests/contracts.rs`;
  - the amendments to DESIGN §3.2, §3.4.1, §4.1, §4.3, §6 and §8, and to ADR-0008.
- `68120ae` is docs only: it lifts the line budget and restores §4.2 detail. It carries no
  in-scope semantics, and I do not recommend trimming anything for length.

**Observable outcome claimed.**
- One compile attempt writes the 17 raw tables and derives 6 tables from them in Delta.
- It then runs 118 generated rules: 23 `key`, 38 `ref`, 13 `fact`, 12 `fact-payload`,
  30 `codebook` and 2 `coverage`.
- It publishes with one `snapshots` append.
- A validation failure publishes nothing, and readers resolve versions through `snapshots`.

**Supported scope.**
- Stage C/D for `exports`, `signatures` and `calls`.
- Declared out of scope (§3.2 L363): Pysa rows at non-call sites (property accesses, identifiers,
  artificial and format-string sites) stay raw until increment 2.
- **Proposed:** endpoint kinds and the brief rules.

### Method and coverage

**Read in full:**
- `derived.rs`, `rules.rs` and `tables.rs`;
- all of `cpg-core/src` (`snapshot`, `derive`, `validate`, `attempt`, `lib`, `delta`, `sql`);
- `compile.rs` and its three snapshots, the derivations snapshot and the codebook registry
  snapshot;
- the `contracts.rs` diff and the extractor diff (`public.rs`, `lib.rs` L380–500,
  `pysa_map.rs` `map_definitions` and `push_call_callees`);
- DESIGN §B6, §B7, §3.1–§3.7, §4.1, §4.2.3, §4.3, §6 and §8;
- ADR-0008, the slice-1 review and STATUS.

**Read at the grain cited:** pinned Pyrefly `b9f2857`:
- `report/pysa/function.rs` L387–397 and L541–558 (overload export rule), L847–890 (`name_location`);
- `call_graph.rs` L610–745 (`CallCallees`) and L1211–1225 (`add_callees`);
- `commands/coverage/collect.rs` L302–330 (`trace_export_origin`);
- `export/exports.rs` L86–100 and `export/definitions.rs` L98–115.

**Checks run in this session:**

| Check | Command | Outcome | Observation |
|---|---|---|---|
| Full suite | `just test-all` | passed | nextest 46/46, pytest 18/18, ast-grep scan + rule tests 4/4, adr lint (12), lint-agents, fixtures-check 17 files, family ok, cargo-deny ok, pyrefly-fork ok |
| Probe harness | `target/debug/lctx-extract` (current at HEAD: nextest rebuilt nothing), then the derivation and rule SQL copied verbatim from `contracts__derivations_snapshot.snap` / `contracts__rules_snapshot.snap` and replayed in Python DataFusion 54.0.0 (`uv run --no-project --with datafusion --with pyarrow --with pandas`) | ran | The repo pins 55.1.0. On `pysa_keys` the replay reproduces the committed exports and signatures rows, and every rule passes |
| P1 `pysa_keys` | as above | ran | F5 |
| P2 scratch package `rv` | as above. It holds a dataclass constructor call, a class-field `staticmethod`, an `lru_cache` rebinding, version-conditional overloads in a `.pyi`, and a version-conditional redefinition in a `.py`, under context Python 3.14 | ran | F1, F2, F3 |
| P3 FastMCP **4.0.5** | as above. This is the dev-dependency install, not the 4.0.3 pilot: 257 modules against a 35-package `.py`/`.pyi` subset of site-packages | ran | extraction 28.9 s (debug build); 0 rule violations; counts in F1–F3, O4, O6, O7 |

**Not inspected or not attacked (asserted only):**
- DataFusion 55.1 vs 54 differences beyond the reproduced `pysa_keys` rows;
- the ambiguous-success branch of `publish` (`attempt.rs` L208), which only ADR-0009's P3 probe
  exercised;
- concurrent attempts on one root;
- Rust derive time;
- the 4.0.3 pilot, since Stage A doesn't exist yet;
- `codebook.rs` (I read the registry snapshot instead) and the `table!` macro (read in the slice-1
  review).

## 2. Authority and lifecycle (compressed)

- **Derived contracts: one authority each.** Each `table!` declaration sits next to its
  `Derived::sql()`. `derivations_snapshot` and `rules_snapshot` pin them, and `compiler_digest`
  hashes them.
- **Rules come from the contracts.** Keys, codebook columns and fact tables are read off the
  schemas. `REFERENCES` is the only hand-written list (O8).
- **One producer per table holds.**
  - `derive` writes only derived tables, and the extractor only raw ones.
  - The registries follow §3.2 L348–350.
- **Derived rows carry keys and source `fact_id`s,** except for three aggregated copies (O1).
- **Two definitions of one fact.** "Call site X has no regular Pysa record" is computed twice (F3):
  - by the extractor, into `boundaries` and `coverage`;
  - by SQL, into `resolutions.status`/`reason`.
- **Two rules for which `def` a name binds** (F2):
  - Pyrefly's binding and successor chain decides Pysa's functions;
  - derived SQL decides by byte order.
- **The compiler, which produces every derived table and `snapshots`, has no recorded identity**
  (F4).

## 3–4. Contracts and derivation (merged)

**The path that executes** (`attempt.rs` L151–197):
1. `write_raw` writes every raw table exactly once. It refuses a schema mismatch or a foreign
   `snapshot_id` (L79–84).
2. It builds a session at the written versions, filtered to the snapshot (`snapshot.rs` L45–73).
3. It runs each derivation in dependency order (`derive.rs` L27–47, `attempt.rs` L121–134):
   - the query goes through the read-only helper;
   - the result is cast strictly to the declared schema (`RecordBatch::try_new` enforces
     non-null);
   - the rows are sorted canonically;
   - `DeltaTable::write` stores them, and the table is registered again at its new version.
4. It runs every rule (`validate.rs` L19–39).
5. It appends to `snapshots` (L201–212).

A failed rule returns `CoreError::Invalid` before step 5.

**What a null means in each derived table (the absence lattice):**

| Derived table | Null node means | In-row reason |
|---|---|---|
| `provider_node_map` | no name span (Pysa `ClassField`: synthesized members), or no declaration at that span | **none** (F1) |
| `exports` | origin not a `def`/`class` of the release. FastMCP 4.0.5: 311 release variables, 17 non-release origins | none; the raw row explains it |
| `signatures` | not applicable; `function_key` is null when Pysa gives no callable | `missing_evidence`, `provider_disagreement` |
| `parameters` | not applicable | `provider_disagreement` |
| `resolutions` | not applicable; the site keeps a `status` | `outside_provider_model` or `missing_evidence` when `not_attempted` |
| `call_targets` | a target outside the release, **or** a release target with no declaration node | **none** (F1) |

## 5. Journey: a version-conditional redefinition (P2)

**The input.** `rv/compat.py` defines `_new` and `_old`, then:

```python
if sys.version_info >= (3, 11):

    def load(x):
        return _new(x)
else:

    def load(x):
        return _old(x)
```

The context is Python 3.14, and `rv.compat.load` is public.

**What each stage produces:**
- **Pysa** exports `F:2`, which maps to the live `def` at byte 105. Pyrefly's binding pass drops
  the `else` branch (§4.2.4).
- **`exports`** ranks by `is_overload, start_byte DESC` (`derived.rs` L81). So it seeds
  `rv.compat.load` from the dead `def` at byte 151.
- **`signatures`** gives that `def` `missing_evidence`.
- **`resolutions`** gives its only call, `_old(x)`, `not_attempted` with `missing_evidence`.
- **`coverage`** marks `calls` `partial` for the module, and a boundary agrees. `signatures`
  coverage stays `complete_under_stated_model`.

**Consequence.** Pass A (§9.1) would build the public symbol's brief from code that can't run
under the stated context. It would report the symbol's calls as unanalysed, although Pyrefly
resolved the live definition completely. This is F2.

## 6. Acceptance gates

| Gate | Result | Evidence or scope rationale | Required action |
|---|---|---|---|
| **G1** Authority | **fail** (narrow, latent) | "No regular Pysa record at this call" is decided by two independent predicates: `lib.rs` L426–431 + `pysa_map.rs` L635–636 and `derived.rs` L279–284/L295. No rule reconciles them at publication, and the one test compares counts on one fixture, where `missing_evidence` is 0 = 0. They agree today (FastMCP 4.0.5: 188/188 per site). Everything else has one authority (§2) | F3 |
| **G2** Semantic fidelity | **fail** | Demonstrated on probes: (1) a release target with no declaration (a synthesized `__init__`) is indistinguishable from an external target: 253 `call_targets` rows on FastMCP 4.0.5 (F1). (2) The seed and overload roll-up contradict the analyzer's context: a dead-branch seed and a misattributed `missing_evidence` (F2). Unresolved: two seeds for one access path under a `.py`/`.pyi` pair (F5) | F1, F2, F5 |
| **G3** Validity | **pass** | Every derived batch is strictly cast to the declared schema, with non-null and CHECK enforced on write. The rules run before publication. Each rule kind rejects an injected violation (`every_rule_kind_rejects_its_violation`, **Tested**). The Stage-C key is checked after downstream use but before publication, which is safe (O3). Gaps in the rules' own coverage are O2 | O2 (regression control) |
| **G4** Hidden behaviour | **pass** | Validators read only, through `sql::query` with DDL, DML and statements off (`sql.rs` L9–20). Sessions pin versions and filter the snapshot. `resolve` reads the latest `snapshots` by design, since that table is append-only and the authority. No crate names `.claude` or skill paths (grep) | — |
| **G5** Consistency and recovery | **pass** | Publication follows validation (`attempt.rs` L168–191). A validation failure publishes nothing, and A's reader still sees only A's rows (`an_attempt_publishes_every_table_and_readers_see_only_published_rows`). A rejected append is classified as unpublished (`a_failed_snapshots_append_is_classified_by_rereading`). Reusing a `snapshot_id` should fail closed on the key rules (reasoned from the SQL, not executed). The remaining gap is O5 | O5 |
| **G6** Transformation and reuse | **unresolved** | `content_digest` compares reruns (§3.4.1). Its compiler part hashes the SQL, contracts, rules and a fixed `0.1.0`, but not the query engine or the derive and cast code, and it is stored only inside the hash (`attempt.rs` L31–72). Its consumer (§9.8) doesn't exist yet; what "compiler identity" covers is still undecided | F4 |
| **G7** Truthful capability claims | **fail** (narrow) | ADR-0008 L57–59 and §4.1 L603–604 promise unmapped rows "with a null node and a reason column", but `provider_node_map` and `call_targets` have no reason column (F1). Smaller wording overclaims: O1, O3, O7 | F1, O1 |

## 7. Findings

### 7.1 Findings

Ordered by severity: demonstrated wrong or ambiguous output first, then authority, then identity.

| ID | Finding | Principle IDs | Concrete evidence or gap | Consequence | Proposed correction | Verification |
|---|---|---|---|---|---|---|
| **F1** | A call to a release function that has no declaration node is indistinguishable from a call outside the release. `provider_node_map` and `call_targets` leave the node null with no reason. | DM-08, DM-42, DM-59 · G2, G7 | **Where the null comes from:** Pysa gives `ClassField` functions (synthesized members) no `name_location` (`function.rs` L867–872), so the span join (`derived.rs` L47–53) leaves them null. **What the schemas lack:** `ProviderNodeMap` (L28–43) and `CallTargets` (L328–338) declare no `reason`. `call_targets` LEFT JOINs the map (L350–352), so both cases come out as null/null. **What the ADR promises:** ADR-0008 L57–59 says "with a null node and a `reason` column". **P2:** `Point(1, 2)` for a release `@dataclass` gives an `init` target `@rv/impl.py CF:0:2 __init__` with `target_node_id` null; the resolution is `resolved` and complete, with no reason. **P3 (FastMCP 4.0.5):** 489 of 2,734 Pysa functions have no span, and all are unmapped. 253 of the 3,744 `call_targets` rows whose `target_module` is a release file (6.8%) have no node. The top keys are `CF:*:* __init__`, phase `init` | In §5's invocation projection and in Pass A, those 253 arcs fall under the unknown-target policy exactly like `builtins.print`. "Constructs a release dataclass" then reads as "calls outside the release", and a brief can say a delegation chain ends at an external call | Add `reason` to `provider_node_map` and `call_targets`. A Pysa function with no name span gets a reason: append a `boundary_reason` value for provider-synthesized members, or map it through `defining_class` → `class_ancestry` span → the class declaration with that reason. A release target with no node inherits the map's reason. Document null/null as "not a release function" in §3.2. Surface: 2 columns, 1 codebook append, 2 SQL edits (a schema migration) | **Test:** a dataclass constructor call in a fixture, plus a rule that no `call_targets` row with a `@` target has a null node and a null reason. That rule fails today: 253 rows on FastMCP 4.0.5 and 1 on P2 |
| **F2** | The `exports` seed and the overload roll-up pick a `def` by byte order, re-implementing Pyrefly's binding choice without the context's reachability. | DM-24, DM-02, DM-08 · G2 | **Seed:** `ranked` orders by `is_overload, start_byte DESC, node_id` (`derived.rs` L78–82). **Roll-up:** `impl_start`/`last_stub` use byte order (L129–146), and a missing key becomes `missing_evidence` (L169). **Pyrefly's rule is different:** it exports over binding successors (`function.rs` L395–396), and its binding pass drops statically decided branches (§4.2.4). **P2 `compat.py`:** §5. **P2 `ov.pyi`** has two `sys.version_info` overload pairs. Pysa's `F:1` (2 signatures) maps to the live second stub, but SQL rolls all four stubs up to the last (dead) stub. All four get `missing_evidence`, and no `signatures` row references `F:1`. **P3:** 0 seeds with `missing_evidence`; 4 same-name redefinitions, all mapped | A public symbol is seeded from dead code, and its calls read as unresolved while the live `def` is fully resolved. For conditional overloads, Pysa's signatures are dropped and the reason says "no evidence" when the evidence exists and our roll-up disagrees | Make Stage C the tie-breaker. Rank `exports` candidates by (`is_overload`, has a `provider_node_map` key DESC, `start_byte` DESC, `node_id`). In `signatures`, when the byte-order callable has no key but a same-name `def` in the module does, roll up to the keyed `def` or report `provider_disagreement`. State the rule in §3.4.1, including `TYPE_CHECKING`: Pyrefly keeps the `if` branch while the runtime binds the `else`, which is a real choice between the typed facade and the runtime body | **Test:** add the P2 `compat.py` and `ov.pyi` shapes to a fixture. Add a rule that every mapped key with `signature_count > 0` is referenced by a `signatures` row. It fails on P2 and passes on FastMCP 4.0.5 and the three fixtures |
| **F3** | One fact, "this call site has no regular Pysa record", has two independent definitions and no reconciliation at publication. | DM-02, DM-23 · G1 | **Extractor:** a regular `Call` record at the range, whatever it emits (`pysa_map.rs` L635–636, `lib.rs` L426–431), drives `boundaries` and `calls` coverage. **Derivation:** at least one emitted row with `higher_order_index IS NULL` (`derived.rs` L279–284, L295, L306–307) drives `resolutions`. **Links:** none in `rules.rs`. The only link is `compile.rs` L173–186, per-reason counts on `pysa_variants`, where `missing_evidence` is 0 = 0. **Agreement today:** 188/188 per site on FastMCP 4.0.5. **Where they would diverge:** a `Call` record that emits nothing non-higher-order (`CallCallees::empty()` has `Unresolved::False`, `call_graph.rs` L614–621, and `add_callees` L1211–1225 does not filter it; I did not show this reachable). More likely, increment 2 widens the SQL to artificial sites without touching the extractor. **A related gap:** derived-only gaps never reach `coverage`. In P2, `compat.py` has `signatures` coverage `complete` next to a `missing_evidence` signature, while §3.7 L534–535 tells analytics to read `boundaries` | A snapshot can publish with `calls` coverage `complete` for a module whose resolutions say `not_attempted`, or the reverse. A pass reading §3.7's channel and one reading `resolutions` then disagree about whether a call was analysed | Add one rule: per call site, a non-null `resolutions.reason` ⇔ a `calls` boundary with that subject and the same reason (an anti-join each way). Amend §3.7: derived reason columns carry Stage C/D gaps, and `coverage` describes the producer's run | **Test:** `every_rule_kind_rejects_its_violation` gains a case that drops one `calls` boundary and its `facts` row, and the new rule fires |
| **F4** | The compiler, which produces every derived table and `snapshots`, has no recorded identity. Its digest omits the query engine and the derive code, and it survives only inside `content_digest`. | DM-31, DM-32, DM-46, DM-48 · G6 (unresolved) | **What the digest hashes:** `compiler_digest` (`attempt.rs` L31–44) takes `CARGO_PKG_VERSION`, which is fixed at `0.1.0`, plus the SQL, contracts and rules. **What it omits:** the DataFusion, Arrow and delta-rs versions, and `derive.rs`/`to_declared`/`canonical_sort`. **Where it goes:** it is folded into `content_digest` (L64–72), and `snapshots` has no column for it (`tables.rs` L444–466). **What DESIGN says:** §3.4.1 L444–445 says the compiler "has its own run" (Proposed), but no `runs`/`producers` row exists for it. §B6 L200 says "every assertion is a `facts` row", yet `resolutions.status`, `provider_node_map.node_id` and `signatures.reason` are not | After a DataFusion bump or a change to the strict cast, a rerun gets the same `content_digest` while its derived tables can differ. §3.4.1's "compares reruns" and the §9.8 ablation joins would then call it unchanged. A published snapshot also can't say which compiler produced its derived rows | Store `compiler_digest` on `snapshots`, or write the compiler's `producers`/`runs` rows (§3.4.1). Hash the locked engine versions and a hand-bumped compiler output version into it, with `just deps` checking them against `Cargo.lock` (the slice-1 F4 pattern). Before acceptance, add a scope sentence to ADR-0008 and put §B6 in its `design:` list: `facts` rows are extracted (later analytic) assertions, and a derived join row is traced by its cited `fact_id`s plus its snapshot's compiler identity | **`just deps`** (constant = `Cargo.lock`), plus a unit test that the digest changes with any derivation string |
| **F5** | `exports` can give one access path two seeds, and what two rows mean is undeclared. | DM-09, DM-08 · G2 (unresolved) | **Schema:** the key is `[snapshot_id, access_path, public_fact_id]` (`derived.rs` L65). **Snapshot:** `derived_tables_on_the_keys_fixture` has `keys.dual.f` and `keys.dual.g` twice each. **P1:** one row seeds `dual.py`'s `def`, the other `dual.pyi`'s, because `public_names` has a row per file (`public.rs` L62–68). **DESIGN:** §3.2 L361 says "the seed declaration" (singular), and STUB_FOR (§3.4) is Proposed. **P3:** 0 such paths | Pass A gets two seeds for one public symbol, with nothing saying which is the interface and which the implementation. The result is duplicate findings or an arbitrary pick, and the stub seed has no calls | Decide before Pass A. Either seed from the source and link the stub (STUB_FOR as a derived table), or keep both rows with a role. State the cardinality in §3.2 | **Test** on `pysa_keys`, asserting the chosen rule |

**Observations** (no gate impact on their own; each fix is one line or one test):
- **O1. Three derived columns are raw values.**
  - `signatures.form` is MIN over `parameter_semantics.form`, defaulted to `list` (`derived.rs`
    L180–187).
  - `signatures.function_key` (L160, L185).
  - `resolutions.unresolved_reason` is a MIN (L291, L303).

  All three are rebuildable, so none is a second authority. Still, §3.2 L354's "never a copy of a
  raw payload column (**Implemented**)" and ADR-0008 L60–61 overstate the rule. Reword it to
  "keys, and aggregates the join decides".
- **O2. Gaps in the rules' own coverage.**
  - **(a) Two rule kinds never see an injected violation:** `fact-payload:*` and
    `coverage:declared-family` (`compile.rs` L283–302). The `"fact:"` prefix only matches the
    raw → `facts` direction.
  - **(b) `coverage:complete` can pass vacuously.** It joins `source_files` to `runs` on
    `release_id` (`rules.rs` L232), and no `ref` rule links the two.
  - **(c) Composite provider-local references are not checked:** `parameter_semantics` and
    `pysa_calls.caller_key` → `pysa_functions`.
  - **(d) `facts.model_id` is "validated against `producers`"** (`tables.rs` L33, §3.5 L484–485),
    but no rule does it.
- **O3.** §4.1 L597 says Stage C uses "Rust identity logic" (there is none) and checks keys
  "unique before use". In fact the `key:provider_node_map` rule runs after `signatures` and
  `call_targets` have consumed the map (`attempt.rs` L161–168). That is safe, because nothing
  publishes, but the sentence should say "before publication".
- **O4.** The reverse Stage-C direction is unchecked. When two Pysa keys map to one declaration,
  `signatures` keeps `MIN(function_key)` (`derived.rs` L159–161). There were 0 such
  declarations on FastMCP 4.0.5 and the fixtures. A uniqueness rule on non-null
  `provider_node_map.node_id` would make it fail closed.
- **O5.** `publish` is `pub` and nothing guards a second call for the same `snapshot_id`. If one
  happens, `resolve`'s `ORDER BY table_name` (`snapshot.rs` L88) is no longer total, and the
  `BTreeMap` keeps whichever row came last. `compile` never calls it twice. The
  ambiguous-success branch (`attempt.rs` L208) is untested in the repo.
- **O6.** Higher-order unresolved remainders are raw-only: 19 on FastMCP 4.0.5.
  `resolutions` filters out `higher_order_index`, and `call_targets` drops `unresolved`
  (`derived.rs` L284, L349). §3.2 L363's raw-only list doesn't name them.
- **O7.** §4.3 L805's "not needed at pilot scale" was never measured in Rust. P3's row counts
  support it: 33,012 `pysa_calls`, 15,772 `call_targets` and 93,101 `facts`. Cite them, or label
  the line Proposed.
- **O8.** `REFERENCES` (`rules.rs` L43–94) is hand-written. When §3.2's family → node/edge
  mapping lands, generate one from the other, so the two don't become separate authorities.
- **O9.** ADR-0008's frontmatter says `evidence: Proposed`, while its Consequences cite
  **Tested** evidence.

### 7.2 ADR-0008: the slice-1 conditions

| Condition | Holds? | Evidence |
|---|---|---|
| **(a)** Coverage-completeness and fact-reference validators, shared, run on fixtures | **met** | `coverage:complete`, `fact:*` and `fact-payload:*` are generated in `cpg_schema::rules` and run by `cpg_core::validate` inside `compile`, the publication path. No test-only copy exists. `compile` runs on `pysa_variants`, `unicode_bom` and `pysa_keys`. Each kind rejects an injected violation (**Tested**). Residual gaps: O2(a) and O2(b) |
| **(b)** Stage-C keys unique per snapshot, by a uniqueness query | **met** (forward direction) | `key:provider_node_map` groups by `(snapshot_id, module_node_id, function_key)`. A duplicate Pysa key, or a name span matching two declarations, fails validation. It passes on the three fixtures and on P3. Residual: it runs before publication rather than before use (O3), and the reverse direction is unchecked (O4) |
| **(c)** The family → node/edge mapping, or the ADR says it waits | **met** (by deferral) | ADR-0008 L66–67 and §8 L1009 say the mapping and endpoint rule land with the first projection. §3.2 L343–345 is still in the present tense in a Proposed section. See O8 |
| **(d)** Each provenance table names its producer | **met** for the provenance tables | §3.2 L348–350 and ADR-0008 L63–65: the registries are appended by each producer, and `source_files` is the extractor's until Stage A. Residual: the producer of the derived tables has no recorded identity (F4) |

**Verdict: ADR-0008 can be accepted now.** Accepted ADRs are immutable, so accept it in the
commit that also:
1. fixes F1, or narrows its sentence "with a null node and a `reason` column";
2. adds F4's §B6 scope sentence, with §B6 in `design:`;
3. sets `evidence:` to Tested and names the tests.

F2, F3 and F5 don't touch its Decision text.

### 7.3 Applicability and verdicts

**Applicability.**
- **Bore on this scope:**
  - group 1 (F3);
  - group 2 (F1, F5, O2);
  - group 3: identity and publication (F4, G5);
  - group 5: derivation contracts (F1–F3, O1);
  - group 6 (publication);
  - group 7 (F4);
  - group 10: lineage (F4);
  - group 11: generated rules and oracles (DM-52, DM-53, O2);
  - group 12 (DM-58; §8).
- **Bore little:**
  - group 4: the only declarations are `table!` and a one-method `Derived` trait;
  - group 8: no performance claim beyond O7;
  - group 9: the adapter changed by two raw columns; DM-42 applies in F1.

**Verdicts.**
- **Satisfied:**
  - DM-52: rules generated from the contracts;
  - DM-53: each rule kind rejects an injected violation, and the validators are shared;
  - DM-20: validators read only;
  - DM-14, DM-29, DM-30: publication after validation, nothing published on failure, readers
    through `snapshots` (all **Tested**);
  - DM-23 for lineage: derived rows cite `fact_id`s and are rebuilt from Delta by construction;
  - DM-07 at the derived-write boundary.
- **Violated:** DM-08 and DM-42 (F1); DM-24 (F2); DM-02 (F3); DM-59 (F1's claim, O1).
- **Unresolved:** DM-31, DM-32 and DM-48 for the compiler identity (F4); DM-09 for the
  cardinality of `exports` (F5).

## 8. Alternatives (compressed)

| Alternative | Assessment |
|---|---|
| **Current:** in-row `reason` on derived rows, with `boundaries` kept as the extractor's | Keeps one producer per table. The cost is a second channel that consumers must read, plus a duplicated predicate for calls (F3). Keep it and add F3's rule |
| **Simpler:** `resolutions` carries `status` only; an unmatched call's reason is the extractor's boundary, joined on `subject_node_id` | Viable for calls, and it removes the duplicated reason. `status` still needs F3's rule, and `signatures`/`parameters` need an in-row reason anyway, because no extractor boundary exists for them. Not preferred |
| **Heavier:** a compiler-written `derivation_boundaries` table | Another table and producer with no consumer. Rejected (DM-58) |
| **Current:** collect, then `DeltaTable::write` | Justified. The largest derived table on P3 is 15,772 rows |
| **Streaming** with `with_input_plan` (S6) | Not needed until a pilot measurement says otherwise (O7) |
| **Current:** a rule generator (240 lines, 118 rules) plus a hand-written `REFERENCES` | Justified. Adding a table adds its key, codebook and fact rules automatically. `compile` is the consumer, and the snapshot pins the output |
| **Simpler:** a hand-written list of validators | It would re-list every key and codebook column per table (second-authority risk). Rejected |

**No over-construction found.**

## 9. Top verification gaps

| Claim | Label now | Gap | Oracle |
|---|---|---|---|
| Unmapped derived rows carry a reason | Implemented for 4 of 6 tables | F1 | dataclass fixture + a `@`-target rule |
| The seed and the callable follow the analyzer's binding | Implemented (byte order) | F2 | P2 fixture + the orphan-key rule |
| `boundaries` and `resolutions` agree | Tested on one fixture by count; `missing_evidence` is vacuous | F3 | a generated per-site rule |
| `content_digest` compares reruns | Implemented, incomplete | F4 | `just deps` + a digest unit test |
| Each rule rejects its violation | Tested per kind, not per direction | O2(a) | a case each for `fact-payload` and `coverage:declared-family` |

## 10. Exceptions

None are claimed. F1–F5 are recorded as violations or unresolved decisions.

## 11. Decision

**Decision: Revise (narrow).**

**Reason.** The publication spine holds, each piece Tested and each piece the one that would
break without the others:
- derivations read only Delta, at pinned versions, filtered to the snapshot;
- every derived batch passes the same strict local check as a raw one;
- 118 rules generated from the contracts run on the publication path and reject each injected
  kind;
- one `snapshots` append follows validation, and nothing publishes on failure;
- a rejected append is classified by re-reading.

ADR-0008's four conditions hold.

Three gates fail, each narrowly and on claimed behaviour:
- **G2:** F1 is on pilot-shaped input, 253 arcs; F2 is shown on a probe;
- **G1:** F3 is latent;
- **G7:** F1's claim.

G6 is unresolved (F4). None needs a new spike or an architecture change:
- F1 is a reason column and a codebook append;
- F2 is a ranking change and a rule;
- F3 is one rule;
- F4 is one stored digest and a `just deps` check.

| Priority | Change | Principle IDs | Acceptance evidence | Regression protection |
|---|---|---|---|---|
| 1 | `reason` on `provider_node_map`/`call_targets`; synthesized members given a reason or mapped to their class (F1) | DM-08, DM-42 | the fixture test and the `@`-target rule pass | test + rule (a schema migration: snapshot diff) |
| 1 | Seed and roll-up tie-broken by Stage-C keys; §3.4.1 states the rule (F2) | DM-24, DM-02 | the P2 shapes seed the live `def` | test + orphan-key rule |
| 2 | Per-site `resolutions` ⇔ `boundaries` rule; §3.7 amended (F3) | DM-02, DM-23 | an injected mismatch is rejected | test |
| 2 | `compiler_digest` stored and tied to `Cargo.lock`; §B6 scope sentence in ADR-0008 (F4) | DM-31, DM-46 | `just deps` fails on a mismatched engine pin | `just` recipe + unit test |
| 3 | Decide the cardinality for `.py`/`.pyi` seeds (F5) | DM-09 | `pysa_keys` asserts the rule | test |
| 3 | O1–O9 | — | one line each | as noted |

**ADR-0008:** accept it in the commit that closes F1 (or narrows its sentence) and adds F4's §B6
scope sentence (§7.2).

### Deferred

| Item | Why deferred | Reopen when |
|---|---|---|
| Running the pipeline on the FastMCP 4.0.3 pilot in a test | Stage A doesn't exist. P3 used 4.0.5 and a site-packages subset | Stage A lands, or the increment-1 `deep` review |
| Showing whether an empty regular `CallCallees` is reachable (F3's divergence case) | F3's rule catches it either way | F3's rule fires on a real package |
| Carried from slice 1: O3–O5, O7 and the ADR-0012 review's sidecar and cost items | No trigger fired: slice-1 O4 said "slice 2's validators", and no validator reads a boundary's model | their stated triggers |

**Probe fixtures** (they live in the reviewer's scratchpad, so they are described here for reuse):
- **`rv/impl.py`:**
  - `@dataclass class Point: x: int; y: int`;
  - `def _helper(v)`;
  - `class Holder: fn = staticmethod(_helper)`;
  - `def make(x)`;
  - `def wrapped(x)` followed by `wrapped = functools.lru_cache(wrapped)`.
- **`rv/__init__.py`:** re-exports those names. It defines `api(x)`, which calls `Point(1, 2)`,
  `Holder().fn(1)` and `make(x)`.
- **`rv/compat.py`:** as in §5.
- **`rv/ov.pyi`:** under `if sys.version_info >= (3, 11):`, two `@overload def f` stubs (`int`,
  `str`); under `else:`, two more (`int`, `bytes`).

## Disposition (author, 2026-09-22)

Every finding is fixed or deferred below. Verified with `just test-all` on 2026-09-22: passed (nextest
48/48, pytest 18/18, ast-grep rule tests 4/4, adr lint, lint-agents, fixtures, family, cargo-deny,
fork check). The schema changes are a migration with no stored data: `provider_node_map.reason`,
`call_targets.reason`, `snapshots.compiler_digest`, and two `boundary_reason` codes appended
(13 `no_source_declaration`, 14 `unreachable_in_context`). The review's probe shapes are now the
`fixtures/python/derive_cases` fixture.

| ID | Outcome | Change | Oracle |
|---|---|---|---|
| F1 | fixed | `provider_node_map` and `call_targets` carry `reason`. A Pysa function with no name span is `no_source_declaration`, and no `def` at its span is `provider_disagreement`. A release target inherits the map's reason, or `missing_evidence` if Pysa has no such key. A null node with a null reason is declared to mean "outside the release" (§3.2). The node stays null rather than mapping to the class: the class is reachable through `pysa_functions.defining_class` | `semantic:release-target-explained` (fails when injected); `derive_cases` asserts `Point(1, 2)`'s `__init__` is reason 13 |
| F2 | fixed | Stage C breaks the ties. `exports` ranks `is_overload`, then keyed first, then `start_byte DESC`. A stub rolls up to the first later implementation **or keyed** `def` (else the last stub). An unkeyed callable with a keyed same-name `def` is `unreachable_in_context` (new code 14), else `missing_evidence`. §3.4.1 states the rule, including `TYPE_CHECKING` | `semantic:pysa-signatures-placed`; `derive_cases` asserts the live `load` seed, 3 unreachable signatures and 2 placed `ov.f` signatures |
| F3 | fixed | `semantic:resolution-has-boundary` and `semantic:boundary-has-resolution` match the reason per call site, both ways. §3.7 says the two channels describe one fact each | a new rule-kind case drops the calls boundary and its fact, and the rule fires |
| F4 | fixed | `cpg-core/build.rs` reads the locked engines from `Cargo.lock` (DataFusion, Arrow, Parquet, object_store, delta-rs core and kernel, with sources), so there is no hand-kept constant and no drift check is needed. `compiler_digest` hashes them plus a hand-bumped `COMPILER_OUTPUT_VERSION`, every derivation, contract and rule, and is stored on every `snapshots` row. ADR-0008 and §B6 gain the scope sentence | unit test `the_compiler_digest_follows_each_input` (each input moves it; the engines include `datafusion 55.1.0` and the delta-rs git source) |
| F5 | fixed (declared) | One `exports` row per `public_names` row: a pair gives two rows, each seeding its own file's declaration, told apart by `source_files.is_stub`. Pass A seeds from the source row. STUB_FOR stays Proposed | `derived_tables_on_the_keys_fixture` asserts the two rows and their files |
| O1 | fixed | §3.2 and ADR-0008 now say "a mapping, a status, a reason, or an aggregate of raw values" | prose |
| O2 | fixed | (a) new rule-kind cases for `fact-payload:*` and `coverage:declared-family`. (b) references `runs.release_id` ↔ `source_files.release_id`, so coverage cannot pass vacuously. (c) `semantic:parameter-semantics-function`; `pysa_calls.caller_key` stays unchecked because `MTL`/`CTL` callers have no `pysa_functions` row. (d) `semantic:model-id-producer` | rules snapshot; rule-kind cases |
| O3 | fixed | §4.1 C: a DataFusion name-span join, checked unique and injective before publication | prose |
| O4 | fixed | `semantic:stage-c-injective` | rules snapshot (passes on the four fixtures) |
| O5 | fixed (guard) | `publish` refuses a `snapshot_id` already in `snapshots` (`CoreError::AlreadyPublished`). The ambiguous-success branch stays untested in the repo; the spike P3 covers it | Deferred below |
| O6 | fixed | §3.2 names higher-order unresolved remainders as raw-only | prose |
| O7 | fixed | §4.3 cites the probe's counts as Measured (review probe, FastMCP 4.0.5) | prose |
| O8 | deferred | §8 now says the references and the node/edge mapping are generated one from the other once the mapping lands | below |
| O9 | fixed | ADR-0008 is `accepted` with `evidence: Tested` and names its tests | adr lint |

**ADR-0008 is accepted** in the commit that closes F1 and adds the §B6 scope (review §7.2).

| Deferred item | Why deferred | Reopen when |
|---|---|---|
| O5: a repo test of an ambiguous `snapshots` append that committed | It needs a fault injected after the commit; spike P3 showed the classification | the publication path changes |
| O8: generate `REFERENCES` and the family → node/edge mapping from one declaration | The mapping has no reader until the first projection | the first projection (§5) lands |
| Calls inside a `def` Pyrefly never binds read `missing_evidence`, not `unreachable_in_context` | The extractor's boundary and `resolutions` must agree (F3), and Pass A does not traverse unbound `def`s, since no seed or target reaches them | a pass reads those resolutions |
| `pysa_calls.caller_key` → `pysa_functions` | `MTL`/`CTL` callers have no function row; ownership comes from `call_syntax` | a consumer reads `caller_key` |
