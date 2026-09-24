# Earlier probes (2026-09-22 to 2026-09-24), sources only

Recovered from the session scratchpad before it was cleared. Build outputs and virtualenvs are
omitted; each crate or script rebuilds from its sources. Dates are approximate; the reviews and
deviation logs of that period cite what each found.

| Folder | What it probed |
|---|---|
| `pyrefly-probe` | Pyrefly linked in-process (ADR-0012) |
| `ruffprobe` | Ruff 0.0.11 range parity with our syntax facts (`ours.txt`, `parity.py`, `residue.py`) |
| `df-probe` | DataFusion 55.1 features used by derivation and validation |
| `schema-probe`, `serving-probe`, `stagef-probe` | Arrow schemas, the serving bundle, Stage F |
| `graph-probe` | Dependency-graph lock checks |
| `odis-probe` | odis as an FCA oracle (a dev-only candidate) |
| `ascent_probe` | ascent 0.8.1 (the Datalog spike behind its trigger) |
| `lanceprobe` | LanceDB determinism (a deferred store) |
| `slice1.5-review_probe.rs` | The slice 1.5 review's probes |
