# Status

_Updated 2026-09-30 under the [handoff skill](.claude/skills/handoff/SKILL.md); shared main; phase 0 accepted (scoped); phase 1 implemented (P1.13 rehearsed); phase 2 through A9._

## Design standard (2026-09-30)

- **Implemented policy:** [ADR-0093](docs/adr/0093-domain-model-review-scope.md) adopts core/template
  3.2 and profile guidance 1.3. Domain-model assessment belongs to bounded design/review periods;
  AGENTS has no standing modeling mandate. Flow tracing is optional for concrete uncertainties.
  FP-04/A2 retain model adequacy and authority over behavior. Reviewer and skill guidance align.
- **Implemented review guidance:** relevant library-catalog entries are optional context for
  alternatives; design principles govern the judgment. AGENTS and the catalog introduction align.
- `just docs-check` (2026-09-30): **failed** at agent validation: unchanged implementer guidance
  names missing `check-package`, `unit-package`, `codegen` recipes. Its ADR lint **passed**.
- `just docs` (2026-09-30): **failed** on unchanged `.claude/agents/implementer.md` missing H1.
  The earlier five stale review links remain unresolved; this build did not reach link validation.
- Generic skill validation (`uv run --no-project --offline --with pyyaml python
  /home/paul/.codex/skills/.system/skill-creator/scripts/quick_validate.py`): with
  `.claude/skills/design-review-code-intelligence`, **passed**; with `.claude/skills/design-review`,
  **failed** on unchanged `model-baseline`/`user-invocable` metadata (both rerun 2026-09-30).
- Product checks: `just test-all`, facts pilots **not_run** for this documentation scope.
  `just library-catalog` **not_run**, per operator instruction for this documentation-only change.
  Concurrent P2 work was not validated by this session; the checkpoint and receipts below remain
  attributed to earlier cutover work. Next policy work: use core 3.2 at the next scheduled review;
  documentation validation failures remain open.

## Restart checkpoint: semantic model cutover

- **Accepted target:** ADR-0085–0090; review policy ADR-0093. [Cutover plan](docs/plans/semantic-model-cutover-plan_2026-09-29.md)
  §4.1.1 owns the order, §4.2 dated receipts, §8 finding dispositions, §6 deferred obligations.
- **Phase 0** is complete; its exit review is Accept scoped. It excludes the P4 composition engine.
- **Phase 1** is implemented and focused-Tested (2026-09-29; P1.1–P1.12). The store-lifecycle and
  provider-session reviews were re-inspected as Accept scoped; C01 and C02 are closed.
- **Phase 2 (in progress; focused-Tested 2026-09-29/30):**
  - **A0–A3** (`de89800`, `0213048`, `a7c71a1`, `c87c672`): provider framework, acquisition
    classes and producer, and the typed syntax model;
  - **A4** `9dc55dc`: the `pyrefly` stage emits complete typed syntax (ADR-0089);
  - **A5** `d4e980d`: the lexical recognizer states typed lexical records;
  - **A6** `b46ede2`: the symbol model (symbols, traits, `Linearization`, display-only
    annotations, public names, docs, module resolutions). `ProviderModule` gains the bundle kind
    and a `Namespace` arm (ADR-0089 amendment);
  - **A7** `804a5c1`: call events (site plus origin steps), `ProviderCallSite`, receiver class and
    evidence, `Overrides` dispatch sets, native unresolved reasons;
  - **A8** `38ba3ea`: the remaining type terms, callable parameter lists, `TestOperand`,
    function bodies, record fields;
  - **Review of A6–A8** ([review](docs/design_review/reviews/design_review_semantic-symbols-calls-types_2026-09-29.md)):
    Revise. F01–F04 are corrected in `827135d` (format-string origins, dispatch-set meaning with
    interim policy, `referents`, receiver classification). F05–F07 are deferred with triggers;
    F08 is closed by A9b;
  - **A9a** `002d254` and **A9b** `7f4a0b5`: the `pyrefly` stage states Pysa's
    definitions, public names, docstring parameter docs and the dependency context. The harness
    oracle runs through the `Pyrefly::with_tap` session hook. The legacy definition, public-name,
    parameter-doc and context rows are deleted.
- **Parallel operator work** (the library catalog and utilization scripts, `justfile`,
  `.mcp.json`): owned by another agent. This session does not modify or commit it.

## Focused verification (2026-09-30)

Cargo commands use `python3 scripts/build_environment.py --`; per-slice receipts: plan §4.2.

| Command | Outcome |
|---|---|
| `cargo test --release -p lctx-model` (all suites) | passed at `827135d` |
| `-p cpg-extract` (all suites: typed_symbols, harness, typed_*, acquisition, capture, bundle, legacy) | passed at A9b |
| `-p cpg-core --test facts_driver` | passed at A9b |
| `-p lctx-postgres --test domain_symbols --test domain_calls --test domain_composition --test domain_types --test domain_transfer` | passed at `827135d` (domain_symbols again at A9b) |
| `-p lctx --test model_describe` | passed at `827135d` (snapshot migrated at A6, A7, A8 and the corrections) |
| `cargo check --workspace --all-targets` | passed at A9b |
| `just fmt`, `just test-all`, facts pilots | not_run: P0–P2 functional scope incomplete |

## Known open items

- **P1.13 real transition is blocked** on the PostgreSQL superuser (the session's sudo needs a
  password). The operator's commands are in the
  [evidence](docs/design_review/evidence/2026-09-29_operator-transition/README.md).
- **Routed findings (plan §8):**
  - P0 exit F03 → P4, F05 → B1;
  - store-lifecycle F08 → P3, F09 → Dc;
  - provider-session F04 → before P3/P4 stage sessions, F07 → before the first family-scoped reader;
  - input-validation F02 → P1.9/P1.10 remainder and Q; F11 → P2;
  - A6–A8 review F05 → A10/P3, F06 → A11, F07 → A11/P4.
- **P4 obligations (plan §6):** override dispatch expansion from complete MROs; a class-of receiver
  for class methods called on an object; the behavioral dependency context (modeled classes,
  runtime exceptions).
- **The legacy husk** still runs `extract`: the Pysa call join, types, flow and docs keep in-memory
  structures until A10/A11/A14/A15, and are removed at C1x.
- **Documentation checks fail** on implementer metadata/recipe references and previously reported
  stale review links (receipts above); fix at C3x/Q.
- **Deadlocking editor flychecks.** rust-analyzer's `cargo check --workspace --message-format=json`
  processes can deadlock on build-unit locks. It recurred three times on 2026-09-29/30. A focused
  build then hangs with no `rustc`; stopping the flycheck `cargo` processes (no children, several
  minutes old) releases it.

## Resume here

1. **A10, the call producer** (consult `pyrefly-ruff`). Map Pysa call graphs through
   `normalize_site`/`classify_receiver` into `ProviderCallSite`, `CallTarget` and `CallResolution`
   per call event (`CallOrigin` steps from outermost context to operation; format-string steps
   22/23). Also:
   - `Overrides` targets carry their receiver class;
   - higher-order and potential remainders stay on their own channel and modality (review note);
   - identifier and attribute callees are potential or conditional;
   - Ruff-vs-Pysa boundaries;
   - delete the legacy `pysa_calls` rows and the remaining `keys`/`variants` legacy tests;
   - resolve F05 (the pseudo-callable caller predicate).

   Answers: `pysa_variants`, `map`, `C()` with New and Init, a missing Pysa range as a boundary.
2. Then A11 (types; F06/F07) → A12–A14 (flow) → A15 → A16 → B1–B3 → Dc
   (`lctx compile --through facts`) → C1x–C3x, then Q qualification and the assembled P0–P2
   review (core 3.2, ADR-0093).
3. After the operator's P1.13 run, record its receipts and delete the transition tool.
