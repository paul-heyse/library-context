# PostgreSQL expansion — design evidence

**2026-09-27.** Evidence for the
[expanded architecture review](../../reviews/design_review_postgresql-expanded-architecture_2026-09-27.md).
This folder is a bounded investigation, not the implementation plan or a product acceptance packet.

| Evidence | Outcome and claim boundary |
|---|---|
| [Bridge compatibility and COPY/provider probes](bridges/README.md) | **passed:** candidate resolution with one DataFusion 55.1 / Arrow 59.3 / ADBC 0.24 family; combined selected-provider/federation/delta-rs compilation; real PostgreSQL 18.6 binary COPY and representative provider queries |
| [Embedding boundary and earlier geometry observations](embedding/README.md) | **Interface-checked:** pinned vLLM dimension admission and normalization, pgvector type/index fit. The operator selected 1024 as standard; no additional dimension-quality validation is required |
| Expanded serving/ANN/native ADBC operation | **not_run:** new production integration is outside the review and has not been implemented |
| `just test-all`; `just pilot` | **not_run:** design-only scope; previous deployed PostgreSQL acceptance remains in the [original packet](../2026-09-27_postgresql/README.md) |

## Research and source precedence

Context7 was the first documentation route for PostgreSQL transaction snapshots, Qwen MRL,
the DataFusion PostgreSQL/ADBC providers, and the PyO3 async boundary. Exact
revision manifests, installed vLLM 0.30.0 sources, the pinned DataFusion capability skill and
upstream source were used to resolve version-specific behavior. The child evidence pages and
review cite the primary sources and identify executed behavior separately.

The probes live in their own Cargo workspace and use
`CARGO_TARGET_DIR=/home/paul/.cache/lctx-pg-expansion-target`; they do not alter root dependency pins
or use the inherited target directory of another repository. Disposable PG containers use the
existing pinned `postgres:18.6-bookworm@sha256:3725f4e2499eef5134592b3b4ab79a543ed7f8e533b05b5b637af926630f6650`
image. No operator database, role, extension, launch configuration or running GPU service changed.

Raw outputs are under the existing Git LFS `raw/` policy. Source, independent lockfile and notes are
ordinary text. The bridge README records commands, selected feature sets and limitations: optional
package resolution is broader than the compiled subset, and compilation is broader than the executed
PostgreSQL-provider subset. Native ADBC loading and client-facing protocol servers were not tested.

## Documentation checks

**2026-09-27:** `just docs-check` **failed** because the unchanged external input
`docs/external-review-postgresl-options.md` lacks a title/H1; its ADR/agent checks passed.
The same publisher with only that input excluded **passed**: 150 canonical pages and zero
offline-link errors. Reproduction (no product environment needed):

```bash
uv run --no-project --offline --no-python-downloads python - <<'PY'
from pathlib import Path
import sys
sys.path.insert(0, str(Path('scripts').resolve()))
import docs
settings = docs.config(docs.ROOT)
settings['publication']['exclude'].append('docs/external-review-postgresl-options.md')
docs.build(docs.ROOT, settings)
PY
```

`uv run ruff check` and `uv run ruff format --check` over the two probe Python sources **passed**.
These formatting/source checks do not extend the behavior claims of the executed probes.
