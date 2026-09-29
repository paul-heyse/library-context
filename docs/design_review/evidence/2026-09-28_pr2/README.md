# Salsa catalog experiments

Evidence-only input to the [conditional adoption owner](../../../plans/behavioral-model-forward-plan_2026-09-24.md#7-deferred-each-with-a-trigger). No production engine or cache is selected.

## Library experiments

`salsa_catalog_probe.rs` is a Cargo example over an already published pinned fact snapshot. It
compares clean and prepared derivation with Salsa 0.28.2, including the same full-output comparison
encoding/hash work in timed queries. Initial, unchanged, narrowed, expanded, restored and absent-root
cases retain every evidence field. The Debug encoding is probe-local, never a publication format.
The experiment does not qualify source/environment changes, production persistence or a speed claim.

`persistence-probe` is an isolated workspace. It pins the complete Salsa macro family to 0.28.2;
a floating macro helper patch did not compile against that family. Persistence preserves the original
serialization string inside a version/source envelope. Reordering ingredient JSON through a generic
map caused a restore panic and is not supported by this probe. Owner-envelope incompatibility is
refused before Salsa deserialization. This is a disposable DTO experiment, not serialization of ty
or production catalog state. Ascent remains the S4 recursive-summary comparison.

**Tested and Measured, 2026-09-28:** all eight roots cases matched clean and prepared output.
The [roots receipt](raw/salsa-roots.json) records 31 µs index preparation, unchanged memo reads
at 0–1 µs clock resolution, and changed-root execution at 170–1861 µs for this small fixture.
A redundant setter reran the catalog query; equal output allowed the dependent digest to validate.
These single-process observations are not an end-to-end speed comparison or a scaling result.
The [persistence receipt](raw/salsa-persistence.json) records a 478-byte envelope, no persisted-query
execution after restore, one transient-query execution and rejection of changed producer/source
identity. Ordered serialized ingredient data must remain intact.
