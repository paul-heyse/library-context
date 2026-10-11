# Efficiency-alignment audit evidence

**Owner and consumer.** The root audit coordinator owns this evidence folder for the
[independent comparative efficiency-alignment audit](../../reviews/design_review_efficiency-alignment-audit_2026-10-11.md)
dated 2026-10-11.
The original review and its evidence remain unchanged. This folder is retained Git/LFS
review evidence, not a disposable run or a native service output.

**Authorized method.** Static source, contracts, pinned library evidence and existing
receipts, plus light service-free probes only. No product build/test, fixture, database
query, service operation, environment synchronization or real-library compile is authorized
by this audit. Repository documentation checks are separate publication checks.

**Baseline acquisition.** `raw/baseline.json` records the committed revision, timestamp,
file hashes and concurrently edited files captured before inquiry. Committed source is
read with `git show <recorded-commit>:<path>`. `raw/overlay/` preserves exact captured dirty
file bytes with a `.snapshot` suffix; `raw/baseline.patch` permits comparison with the commit.
These raw captures use the existing Git LFS evidence owner. No worktree, environment or
service is created. Later shared-main changes are reported separately.

## Baseline and concurrent work

- Original source: `88f33bc0dd8bc67ef9291115c3a487b5f54bcb57`.
- Audit committed source and original review commit:
  `2b05c6c239fb6569472751faf126f5e39f822853`.
- Frozen working overlay captured at **2026-10-11 11:08:39 UTC**: 12 already modified
  paths, 1,209,106 bytes. The manifest records each exact file and SHA-256; the diff is
  retained as `raw/baseline.patch`. Captured files were reread to check stability, and
  HEAD was checked before capture publication. This is a checked capture, not an atomic
  filesystem snapshot.
- The two commits differ only in the original review/evidence documents. Audit source
  conclusions therefore correct or extend the original baseline; they are not fixes credited
  to later committed implementation.
- The frozen source overlay adds current-context/absence witnesses for derived serving rows,
  charged selected-root/state batching and affected controls. It does not close the global
  effect, copying attachment, provider reuse or repeated seal diagnoses. The audit treats
  these source changes separately and does not adopt their external test receipts as new
  audit qualification.
- During inquiry, concurrent STATUS and persisted-graph-plan receipts changed. Later small edits
  added one exact control's timeout allowance, coarse manifest/native-capture phase observations
  and first-owner recovery dependency traversal with matching owner prose.
  `raw/publication-context.json`, its patch and late snapshots record them separately. These do not
  change the baseline verdict. Final comparison is recorded in the publication receipt. Neither
  the shared service nor its logs/configuration/registrations were operated by this audit.

## Method and independence

The approved plan used `plan-design-review`, then `design-review` with its code-intelligence
profile. Criteria were core 3.3, heuristics 1.0 and code-intelligence 1.5, plus the functional
API/evidence-product target. Existing ADRs, pins, implementation plans and operating preferences
were design subjects whose proposed changes remain explicit decisions. Execution restrictions
continued to bind this session.

The root read the original principal, handoff, method/reconciliation ledger, inquiry briefs,
operation-map qualifications and probe limits. The source pass followed production owners and
adjacent consumers, including counterexamples to proposed remedies. Historical measurements
were attributed and were not remeasured. The known V02 operation-map error was not propagated:
TyFlow is behavioral-only, not part of catalog extraction.

Three supporting inquiries ran with read-only scopes. Native and compiler inquiries read the
original findings first. The fresh-coverage reviewer formed its four principal candidates before
seeing the original review/handoff, then compared overlaps. The root separately covered serving
and development tooling, integrated evidence and reread decisive corrections. A **fresh principal
design-reviewer**, not a supporting investigator, reread the decisive source and formed the
combined judgment. Root retained publication and integration responsibility.

Supporting reports below are coordinator-integrated records of returned assessments, not
verbatim transcripts. The additional F09 batching correction and F38 reachability clarification
are identified in their reports. They are evidence; only the principal assigns audit findings and
the combined decision. No product remediation or implementation-plan disposition was performed.

## Evidence index and complete comparison coverage

| Record | Scope and consumer |
|---|---|
| [Native contracts](native-contracts.md) | Complete O-F01–O-F15 and native O-F17 comparison; O-RC01/03/06/07/09/13/16/17; cold lifecycle, upgrade and two additional retention findings |
| [Compiler and reuse](compiler-and-reuse.md) | Complete O-F16–O-F34 comparison; O-RC02/04/05/08/18; provider/context keys, receipts, selective reads, coverage and finite kernels |
| [Serving and development](serving-and-development.md) | Complete O-F35–O-F50 comparison; O-RC10/11/12/14/15; ranking, preparation, admission, sorting, tokenizer, build/evidence tooling |
| [Fresh coverage](fresh-coverage.md) | Independent-before-exposure Summary, automatic synthesis, Ruff correspondence and profile-status findings; smaller opportunities and inspected strengths |
| [Principal](../../reviews/design_review_efficiency-alignment-audit_2026-10-11.md) | Combined comparative judgment, calibrated gates, stable audit findings, priorities/prerequisites and decision routes |
| `raw/baseline.json`, `raw/baseline.patch`, `raw/overlay/` | Exact source/overlay attribution, original review hash and preservation boundary |
| `raw/publication-context.json`, `.patch`, `raw/late-overlay/` | Later concurrent timeout/phase-observation/recovery-traversal edits, not a replacement baseline or qualification |
| `raw/publication.json` | Final original-preservation, comparison coverage and documentation-check results |

The three comparison matrices contain a row for **every O-F01–O-F50 and O-RC01–O-RC18**.
O-F17 intentionally appears in both native and compiler reports because its physical and
consumer contracts meet there. Original IDs remain unchanged; audit-local IDs never renumber them.

## Evidence strength and commands

Primary static command families were `git show 2b05c6c:<path>`, `git grep -n ... 2b05c6c -- ...`,
`git diff 88f33bc0 2b05c6c -- <source paths>`, `rg --files`, scoped `rg`, and standalone standard-library
Python for capture/hashes. Source searches were followed by positive owner/caller reads where an
absence claim mattered. Empty search results were not treated as library capability absence.

Context7 was used for relevant library/tool documentation; exact source settled fork/version
contracts where current docs could not. Notably tokenizers' current documentation was not transferred
over the locked 0.23.2 API. No new dependency, version or shared skill was changed.

Static evidence settled the asserted work shapes and supplied countercases. No new runtime probe
was needed to support a timing or engine-plan claim because this audit makes none. Native plan
choices, no-regression cost claims and concurrency behavior remain **unresolved** where source
cannot establish them. Product verification is **not_run**, as agreed. Documentation checks verify
publication only; their actual outcomes belong to the publication receipt.

## Breadth and limits

Coverage was deliberately extended into Summary/synthesis/Ruff correspondence, binding/product
lifecycle, selected backup/restore/upgrade, evaluation worker reuse, vLLM scheduling/cancellation,
profile status, freshness, archive and worktree ownership. This was a targeted owner-and-consumer
pass, not an exhaustive inspection of every function. Complete transfer/consequence semantics,
all lexical statistics/lowering internals, full Python middleware/resource flows, server-side gRPC
scheduling, every migration body and deployed resource behavior remain unqualified. Docs publication
tooling was excluded as a review subject, though its checker publishes this audit.

No real FastMCP compile, representative corpus sizes, actual SurrealDB plans, host retry share,
peak live memory, throughput, latency benefit or complete product gate pass was established.
Protected gold/heldout inputs were not opened. The audit states where the original is right and
where uncertainty remains; additional coverage does not retrospectively make either review exhaustive.
