# Expanded analyzer facts: evidence boundary

**Static review evidence, 2026-10-03.** Current consumer baseline `main` at `700476d9`,
with preserved uncommitted serving qualification/packet changes. Current consumer inspection
includes that tree without testing or modifying those changes. Provider analysis assumes
the selected future pins; production pins remain unchanged.

Question: which additional provider payloads and first interpretation operations improve
library-context's catalog and bounded analysis, including independent latest Ruff?
The accountable consumer is the
[expanded-target review](../../reviews/design_review_code-facts-expanded-target_2026-10-03.md).
The earlier review retains its own IDs and evidence. This folder is evidence, not a new
architectural authority, mutable finding register or implementation plan.

## Contents

- [Supply contracts](supply-contracts.md): independently read consequential pinned interfaces,
  access/population/lifetime limits, corrections and candidate payloads.
- [Coverage inventory](coverage-inventory.md) and [JSON](coverage-inventory.json): all 340
  indexed facts, 137 concepts and 26 areas, competing providers and survey limits.
- [Consumer ledger](consumer-ledger.md): coordinator selection/deferral routes for every concept
  and all 41 candidate provider payloads. Detailed current consumer defects and first operations
  are in the principal review, rather than a duplicate source map or disposition register.
- [Inventory verifier](verify_inventory.py): reproduces bounded consistency checks on these
  artifacts. It does not prove every provider semantic guarantee or rerun source acquisition.

## Sources, versions and method

Pinned primary sources: Pyrefly `80cec3f57364bc11d4a39a419f6894a8eabcaa00`
(1.4.0-dev.3, embedded Ruff 0.0.14); standalone Ruff/ty
`3265ed1f944c98bb4c04d632fbefb1257cdb583d` (Ruff 0.16.10 / crates 0.0.16,
ty core 0.0.16, salsa family 0.28.5). Exact source links are retained per inventory record;
the supply-contract file cites source-written limits at the relevant grain.

Inspection ran in the operator's local Linux workspace. Runtime machine performance is
irrelevant to these static findings; no analyzer runtime, benchmark or qualification campaign
ran. Local pinned source trees are under
`/home/paul/.cache/rust-skill-acquire/src/{pyrefly/80cec3f5,ruff/3265ed1f}`.
They are reproducible caches, not committed evidence. The live skill's fact-index input SHA256
is recorded in the JSON. No skill project-usage overlay was imported as current code truth.
Context7 resolve/query results were current-main orientation only; exact pin source governs
all version-specific claims. No gold or heldout content was read or used as compiler input.

## Checks and outcomes

| Command / inspection, 2026-10-03 | Outcome and boundary |
|---|---|
| `git -C /home/paul/.cache/rust-skill-acquire/src/pyrefly/80cec3f5 rev-parse HEAD` | **passed**, selected commit |
| `git -C /home/paul/.cache/rust-skill-acquire/src/ruff/3265ed1f rev-parse HEAD` | **passed**, selected commit |
| Worker `python3 /tmp/lctx_supply_inventory.py` | **passed**, 26/137/340 inventory, 488 source files present; source existence is not semantic verification |
| `uv run --no-sync python docs/design_review/evidence/2026-10-03_code-facts-expanded-target/verify_inventory.py` | **passed**, finite inventory and consumer-route consistency |
| `sed -n` and `rg -n` at principal-review/provider-contract source ranges | **passed**, bounded static evidence; no runtime behavior or measured benefit claimed |
| `just docs-check` | **passed**, 273 canonical pages, zero offline link errors; publication only |
| Compile, runtime probes, `just test-all`, `just hygiene`, real-library pilots/activation | **not_run**, documentation-only review; production adoption/qualification not authorized in this scope |

The provider researcher verified the finite source map and selected contracts, not all 340
semantic claims. The coordinator inspected decisive current consumers: parameter/formal/type
identity, theory truth production/consumption, imports, typed handlers, signature extraction,
selection, incidence and type-layer membership, Summary/S0 evidence propagation and architectural
routes. Broader method/output proposals remain Proposed. A fresh principal subagent was unavailable
because of the agent thread limit; the coordinator owns the combined architectural judgment.

No binary/raw runtime outputs, virtual environments, targets or generated stores were produced.
