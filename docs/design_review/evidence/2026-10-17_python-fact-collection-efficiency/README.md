# Python fact-collection efficiency: evidence scope

This source investigation supports the [principal target review](../../reviews/design_review_python-fact-collection-efficiency-target_2026-10-17.md).
It is evidence, not architectural authority or an execution plan.

## Baseline, questions and method

Inspected on 2026-10-17: clean repository source
`842e1f4e07a1a2bc70286c8f3991cf2ffb82f4c3`. The question is whether fundamental
Pyrefly/Ruff/ty collection can produce the same attributed facts with less repeated analysis,
traversal, conversion, retained state and coordination. Normal checking is a comparison of
mechanisms, not an equivalent output contract or a latency promise.

The operator selected source plus focused database-free probes where a consequential uncertainty
warrants one. This review uses static source, types, current owners, exact fork differences,
Context7 discovery and directly fetched immutable upstream sources. No product test, build,
analyzer benchmark or runtime probe was needed to establish the structural mechanisms reported.
Those operations are **not_run** in this review. No quantitative speed, memory or timeout-remedy
claim is established. Existing tests are inspected closure routes, not fresh passes.

Two bounded source inquiries examined Pyrefly and Ruff/ty; a fresh design reviewer independently
inspected the decisive source and settled the integrated judgment. Preparation hypotheses were
corrected where source contradicted them. In particular, imported Pyrefly modules do not all
default to Everything, the non-writing reporter does not duplicate automatic report export,
and parser-mediated token recovery cannot be replaced blindly with bare lexing.

The [library comparison](library-comparison.md) records normal-check mechanisms, version
applicability, existing capabilities and limitations of proposed alternatives. The principal
review owns findings, relationships, priorities, rule impacts, independent judgments and limits.
There is no second finding-disposition register here.

## Source identities and reproducibility

| Source | Inspected identity |
|---|---|
| Linked Pyrefly | `63cda076956013cd0bd1d0d05c785f747fe6adc0` |
| Its upstream parent | `80cec3f57364bc11d4a39a419f6894a8eabcaa00` |
| Current upstream GitHub main | `4e602d5f490e7723392904efbe606d1b559d311c` |
| Independent Ruff/ty fork | `9080ee7a82a4ec7359ba2ade60839ef1108a2968` |
| Ruff/ty upstream parent | `3265ed1f944c98bb4c04d632fbefb1257cdb583d` |
| Pyrefly embedded Ruff | Registry 0.0.14; distinct nominal types from canonical Ruff 0.0.16 |
| Salsa family | 0.28.5 |

`git ls-remote https://github.com/facebook/pyrefly.git refs/heads/main` returned the main SHA
above during planning and again during execution. `git diff --stat` between the exact Pyrefly
parent and fork reports 13 changed files, 348 insertions and 30 deletions. This bounds the source
delta; it does not bound runtime impact. The observation APIs touch solver and state code and
are inspected as executing operations.

The linked sources are already present under Cargo's pinned Git checkouts. Current GitHub source
was read in memory from `https://raw.githubusercontent.com/facebook/pyrefly/<SHA>/<path>`;
no separate checkout, generated source tree or raw capture was created. Source line references
use actual raw-file lines, not browser-normalized line numbers. The immutable links and hashes
below identify the retrieved bytes:

| Main source path | SHA-256 |
|---|---|
| `ARCHITECTURE.md` | `b849309c188caa7fbd7cdfe5c18510c2e6cfbb53161839c3bd8a03de626b33ab` |
| `pyrefly/lib/commands/check.rs` | `4f3b1497a44350f1e1c1e3b30aa56646cbfe193f3ee59914d83e1e44fe3b4da3` |
| `pyrefly/lib/state/state.rs` | `756adb3b1988ef8b4a1e05cdb1122481ff8f953bca03dcaf857d1802fd911e00` |
| `pyrefly/lib/state/require.rs` | `11f209ff1783a53f6324df68385fe255e3d754c403ddfb29fe3aba3b8f364404` |
| `pyrefly/lib/report/pysa.rs` | `d93e8f4193363d76ff864a23a3a9bd612f347c0ab4fa3c71656862d44748e797` |
| `pyrefly/lib/report/pysa/context.rs` | `6f45eee68ff6cc32637b8c2f0c1e3c8efbb4ed3fdb59de7aca58d4d965e21e3b` |
| `pyrefly/lib/export/symbols.rs` | `81fd12d325c361b3cbd6f43e0e29edc39d2dbe546e4978a3a1a0a6fce7cb1d74` |

Context7 `/facebook/pyrefly` supplied discovery leads, checked against these exact sources.
Its mixed-version snippets do not establish availability at a pinned revision. Latest-main
discovery does not authorize an upgrade or transfer its semantics to our fork.

## Protected boundaries

Shared SurrealDB targets, the unresolved attachment `a5bd9a10-6a7f-437a-8245-275f9837f0f2`,
old failures/Unknowns, retained captures and native caches are unchanged. No recovery,
regeneration, acquisition, publication, real-library qualification, embedding, submitted MCP
runtime or paused implementation execution ran. The existing runtime-efficiency coordinator
retains its own acceptance and native ownership obligations.

This folder contains small authored source analysis only. Git owns these documents; no venv,
binary, target directory or raw run output is introduced. Its named consumer is the principal
review and any later plan that explicitly consumes its findings.
