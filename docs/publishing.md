# Publishing documentation

**Implemented, 2026-09-25** (ADR-0041; lifecycle ADR-0042). Local qualification of the publisher
(2026-09-25): bootstrap, focused publisher tests, complete builds at `/` and `/library-context/`,
browser checks of scope defaults and prefixed assets, and a clean-checkout build without product
dependencies all **passed**; remote CI is **not_run** (the workflow has not run remotely).

## Commands and dependencies

`just bootstrap-docs` installs documentation binaries through cargo-binstall and prepares the
isolated pytest environment. Prerequisites are Git, uv, just and cargo-binstall; bootstrap may use
the network. Python comes from `.python-version`, pytest from `uv.lock`, and documentation tools
from [site.toml](site.toml). No product package, Rust workspace build or analysis store is needed.

- `just docs`: render, index and check offline links/fragments, then replace `build/docs/site/`.
- `just docs-check`: ADR and agent checks plus the same complete publication.
- `just docs-test`: focused script regressions through isolated pytest.
- `just docs-serve`: build, then serve the final artifact at http://127.0.0.1:8000.

Ordinary commands are offline after bootstrap. Preview has no watcher: stop, rebuild and serve
again after edits. Missing tools/cache report failure. The last successful site survives a failed
build. Run one publishing command at a time; concurrent publishers and process-crash recovery are
outside this personal preview contract. Delete ignored `build/docs/` to reset generated output.

## Authoring and transport

Collections and current-work selections live in site.toml. Add a document under its collection;
use a scalar title or an H1 outside fenced examples. Do not edit generated navigation. The active
standard manifest and ADR lifecycle control scope; current-work selection is a reading priority,
not a status table. The site publishes only the retained working set (ADR-0042): a retired
document leaves the tree and its collection, never moves to an archive collection.

Published Markdown links become local HTML. Omitted tracked files/directories link to GitHub at
the build revision. A dirty preview says those links show committed content. Optional ignored
capability indexes are visibly local references. Ordinary linked images are copied; evidence raw
files are not recursively copied. An evidence download must be an explicit asset selection.
The renderer rejects missing intended targets, including historical local fragments.

A live section move retains its ID, governing decisions and old heading as a short relocation
pointer with `<!-- relocated-section -->` immediately after it; DESIGN's pointers also keep each
former subsection fragment in a compact list. The authoritative owner carries the prose. The
shared resolver ignores fenced examples and relocation pointers, checks unique ownership and
supplies the generated section directory. A retired document gets no stub.

## Search and artifacts

The bundled Pagefind Component UI defaults to Current on each opening. Select Reference or
Everything explicitly; the publisher offers History only when a published page has that scope
(for example a superseded record still in the tree). Navigation, print pages and landing aliases are excluded from indexing.
Search assets are local and URLs resolve from the module location, supporting root or subpath
hosting. To check a deployment prefix, run `uv run --no-project --offline python scripts/docs.py
build --base-url /library-context/` and mount the artifact at that path; normal previews use `/`.

The docs-only workflow uses these same commands and uploads the entire site as an artifact.
There is no public deployment. Remote CI results are separate from local verification.
Mechanical checks establish publication integrity; architectural review remains human/agent judgment.
