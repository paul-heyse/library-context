# Operator database transition — completed 2026-09-30

**Consumer:** semantic-model cutover plan P1.13. This is dated execution evidence.

`cargo build --release -p lctx` passed through the normalized build environment. The disposable
PG18 rehearsal restored the operator schema dump, refused legacy installation, prepared and
switched successfully, and passed `store check` and `runs list`. The real database had no active
connections before transition. The protected administrative configuration supplied authorized
superuser access without using that role in product configuration.

`uv run --locked python scripts/postgres_transition.py plan|prepare|switch|drop-retired`
all passed on 2026-09-30 (the retired one-use tool is recoverable from Git).
The four retained service tables had zero rows and identical fingerprints before preparation,
after copying and after switching. The real archive `lctx_retired_20260930071346` was dropped
after verification. The new database carries the service baseline and generated semantic store;
legacy report/serving schemas were left in the removed archive.

The protected retained-service backup is
`build/backups/2026-09-30_retained-services/retained-services.dump` (22,812 bytes, mode 0600).
It contains the retained services only. The transition utility, its disposable legacy-shape test
and its rehearsal script were removed after their last execution.

[Structured receipt](receipt_2026-09-30.json) records the current rehearsal and real commands.
The older receipt remains dated historical evidence while this phase finding consumes the folder.
This transition does not qualify remaining producer, facts admission or product behavior.
