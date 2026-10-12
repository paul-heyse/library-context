# Development-worker negative inputs

These are current small test inputs, never generated runtime outputs. The host
copies only each project's `tests/` directory into a newly owned worker project;
Rust prepares the authoritative schema there. Credentials and target selection
come exclusively from the host-issued campaign binding.

- `invalid-toml`: refuses during input parsing, before worker connection and JSON report.
- `assertion-failure`: produces an actual failed case and JSON report after schema setup.
- `authentication-failure`: an explicitly absent actor and dummy password fail actual remote signin without changing campaign credentials.
- `cancellation`: executes pinned SurrealQL `SLEEP 15s` for parent-owned cancellation.
  A live worker alone does not prove that SQL has begun; retain the actual phase
  observation and keep remote finality unknown until checked whole-target drainage.

Run through `just surreal-dev test --installation ID --project PATH` with the
explicit lifecycle campaign configuration. Cancellation and disposal belong to
the host owners. No fixture contains live credentials or a service endpoint; the authentication-refusal input contains only an explicitly absent user and nonsecret invalid password.
