# Followup flow-model probes — 2026-09-25

Focused evidence for the followup to the library-fit/reasoning review. These are
review probes, not production changes or integrated acceptance. Binaries and
temporary input packages are created outside the repository and removed on exit.

The accountable reviewer reran both probes with `uv run --no-sync python -B` on 2026-09-25;
both exited 0. Exact outputs are [reach.stdout.txt](raw/reach.stdout.txt) and
[literal.stdout.txt](raw/literal.stdout.txt). These runs reuse the same release artifacts below;
the literal probe is a cached-release provider execution with the relevant current source
separately inspected, not a fresh workspace build.

## 1. F05: a cycle-head result depends on the first queried member

Command: `uv run python docs/design_review/evidence/2026-09-25_library-fit-followup-flow/reach_probe.py`

Outcome: **passed** on 2026-09-25; the assertions reproduce the existing defect.
The script compiles the unchanged current `Model` implementation and its local
source types from `crates/cpg-core/src/flow_model.rs`. It uses real `cpg-schema`
IDs, codebooks and bounded BDD conditions. Only the SQL input-row structures are
narrowed to their accessed fields; graph input is synthetic and conditions are
all true. It does not claim a Python-to-serving reproduction.

Observed source SHA256:
`9e776e6f13c9f2c7418529de4d736b0e979a7b0f96a9f91b9f529077bb6a36c2`.
Schema artifact: `libcpg_schema-da6fcf7dbf76d7a8.rlib`.

The graph equations are `A = p OR Call(B)` and `B = A`; an acyclic control uses
`B = p`. Both variants should belong to the least fixed point at A.

```text
cyclic A queried first: [Identity]; cached after B: [Identity]
cyclic B queried first, then A: [Identity, Call]
acyclic control A: [Identity, Call]
passed: query-order-dependent loss of a loop-carried transfer reproduced
```

The cycle cut returns no sources (`flow_model.rs:709–710`), and the cycle head's
single-pass result is memoized (`:820–823`). Simple origin reachability and
reachability of `(origin, transfer, condition)` are different contracts: a cycle
can add a weaker transfer for an already reachable origin.

Impact is bounded to what was shown. Contributions feed the direct summary seed
and unresolved-boundary relations (`cpg-schema/src/behavior.rs:2085–2124,
2386–2425`). Losing a sibling changes the evidence available to those consumers.
The probe does **not** establish a false served positive: verdicts describe may
behavior, and surviving finite paths and the preceding-call checks can still be
sound. The original review's generic allegation that this affects negative
premises is not demonstrated: parameter unread premises use lexical references,
and field/global premises use attribute names and receiver reachability rather
than contribution row counts (`flow_model.rs:2140–2288`).

## 2. Qualified and aliased literal attribute reads disappear from negative premises

Command: `uv run python docs/design_review/evidence/2026-09-25_library-fit-followup-flow/literal_probe.py`

Outcome: **passed** on 2026-09-25; assertions reproduce incorrect premises using
real `cpg-extract`, its actual Pyrefly/ty providers, DataFusion derivation and
`cpg_core::flow_model::run`. No mocked provider was used. Raw and derived batches
are registered in an in-memory session; this is not a Delta/publication test.

Cached release artifacts used:

- `libcpg_core-70ba415ca109996c.rlib`
- `libcpg_extract-2bb8801014af7246.rlib`
- `libcpg_schema-da6fcf7dbf76d7a8.rlib`
- `libtokio-64c9f841b2b6928c.rlib`
- `libtempfile-b903c75c355e3ebd.rlib`

The script resolves core's Tokio/schema dependencies through Cargo fingerprints
so its runtime and DataFusion use the same Tokio artifact. It requires existing
compatible release artifacts; it does not rebuild the workspace.

The input's singleton has four fields read in four ways:

| Read | Field premise `holds` | Singleton-global premise `holds` |
|---|---|---|
| `self.attribute_control` | false | false |
| `getattr(self, "literal_only")` | false | false |
| `builtins.getattr(self, "qualified_only")` | **true** | **true** |
| `read_attribute(self, "aliased_only")`, with `from builtins import getattr as read_attribute` | **true** | **true** |

All reported boundary reasons were `None`; there were zero dynamic-access rows.
The bare-getattr control disproves the broader suspicion that *all* literal
getattr reads are missed. The confirmed defect is qualification/aliasing.

The split is visible in code: `cpg-flow/src/lib.rs:1220–1230` recognizes a literal
getattr/hasattr by callee text; `cpg-core/src/flow_model.rs:1942–2002` recognizes
dynamic operations from the builtin on the whole callee span and skips recognized
literal names. The negative-premise validator tests only `flow_attribute_loads`
(`cpg-schema/src/rules.rs:2164–2174`) and shares the omission. The bundle exports
these premises as `place_claims` (`cpg-core/src/bundle.rs:274–281`); the Python
consumer renders `holds=true` as no read anywhere in the release
(`python/lctx_mcp/src/lctx_mcp/operations.py:366–379`). That serving trace was
inspected, not executed by this probe.

Correction belongs at the resolved-access contract between extraction and the
flow model: retain whether a known builtin access reads a constant field or uses
a computed name, irrespective of qualification/alias spelling. Derive attribute
loads and dynamic boundaries from that one classification. The original F09's
literal-node repair is useful but does not close this gap. Test bare, qualified,
aliased and shadowed controls, plus computed-name boundaries and a real generated
singleton claim.

## Limits and initial harness repairs

No full `just test-all`, `just pilot`, Delta publication, generated-bundle load,
Ascent adoption spike, provider-version comparison or performance measurement was
run. The literal probe's first compile used a nonexistent `Table::name` function;
it was corrected to `Table::NAME`. Its first executable chose an unrelated cached
Tokio artifact and had no matching reactor; dependency selection was corrected.
Its next run correctly rejected the initial broad plain-getattr hypothesis; the
final input includes the bare control and the confirmed qualified/aliased cases.
