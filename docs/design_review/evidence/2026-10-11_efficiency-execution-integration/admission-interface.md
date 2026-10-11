# Admission interface source check, 2026-10-11

Read-only investigation against the integrated dirty source after main `45e816a3`.
Current consumers: coordinator §4, N11 and D02. No compilation, native execution or
semantic acceptance was performed. This records a prerequisite defect, not a new plan.

`CheckedAdmissionPremises::full` currently requires every Admission-purpose invariant
and every publication check. That proves a different contract from ordinary artifact
admission. It must not grant `admitted=true` merely because all these factories ran.
Root consequently closed both legacy `mark_attempt_admitted` and the new
`mark_completed_admitted` transition before any native query/effect. Ordinary and
restored artifact admission remain incomplete in this source checkpoint.

Observed seams:

- `lctx-model::domain::{model,checked_admission}` need model-owned exact applicability
  over selected inventory, profile and frontier. Preserve dynamically scoped
  `derivation_acyclic` definitions; their inputs can differ from the global factory.
- Completed input, declaration-backed NotRequested input, and an obligation discharged
  by an actually complete-empty applicability domain are distinct evidence. Missing
  requested input is an error. Ordered roles, requested/resolved publication boundaries
  and explicitly detached resolution cannot collapse.
- `AdmissionRuntime::validate_scope` has scoped branches and empty-root handling whose
  current `Result<()>` and memo hash are not opaque executed receipts. Root/domain
  exhaustiveness, selected-owner proof and checked EOF still need their own owners.
- `AdmissionRuntime::admit_frontier` requires no publication completeness check for
  Facts/Normalized, analysis completeness for Analysis, and analysis plus catalog
  completeness for Catalog. `Workspace::validate_publications` explicitly serves
  fixture replay and has no production source callers. Requiring every publication
  factory would introduce diagnostic reconstruction into ordinary admission.
- Facts/producer coverage, reference closure, `admit_entity`, `admit_assertion`, graph
  derivation admission and manifest/outcome checks are additional production gates.
  Native canonical-byte/backing/hold audits do not replace them.
- Both `artifact::admit` and `artifact::verify_restored` still terminate through the
  deliberately refusing legacy marker. Restore must earn fresh destination evidence;
  portable source receipts grant no destination authority.

The production dependency is physical provider drain/freeze and exact inventory → earned
physical completion/selected holds → resolved applicability → executed facts/producer,
reference, whole-input/scoped and frontier evidence → canonical graph and manifest/outcome
admission → opaque complete artifact owner → independently checked native persistence.
No callback result, serialized DTO, diagnostic receipt, missing-input empty relation or
generic wrapper is an acceptable substitute. These obligations remain open in coordinator §4.
