//! Pure, deterministic rendering from the current canonical catalog. Shared by publication/import.
use super::*;
use crate::{catalog::*, evidence::*};
use std::collections::{BTreeMap, BTreeSet};
fn bad(s: impl Into<String>) -> WireError {
    WireError(s.into())
}
fn encoded<T: serde::Serialize>(value: &T) -> Result<String, WireError> {
    serde_json::to_string(value).map_err(|e| bad(e.to_string()))
}
#[derive(Default, Clone)]
pub struct RetrievalInputs {
    pub members: Vec<CatalogMembersRow>,
    pub bindings: Vec<CatalogBindingsRow>,
    pub signatures: Vec<CatalogSignaturesRow>,
    pub constructors: Vec<CatalogConstructorsRow>,
    pub parameters: Vec<CatalogParametersRow>,
    pub fields: Vec<CatalogConfigurationsRow>,
    pub evidence: Vec<CatalogEvidenceRow>,
    pub artifacts: Vec<CatalogArtifactsRow>,
    pub spans: Vec<CatalogSpansRow>,
    pub scenarios: Vec<CatalogScenariosRow>,
    pub deployments: Vec<CatalogDeploymentsRow>,
    pub associations: Vec<CatalogAssociationsRow>,
}
fn unit(
    family: Family,
    key: Id,
    title: String,
    text: String,
    subjects: Vec<Subject>,
    anchors: Vec<Anchor>,
) -> Unit {
    Unit {
        unit_id: unit_id(family, key),
        source_key: key,
        family,
        subjects,
        anchors,
        title,
        text,
    }
}
pub fn derive(input: &RetrievalInputs) -> Result<Vec<Unit>, WireError> {
    let mut canonical = input.clone();
    canonical.signatures.sort_by_key(|s| s.signature_id);
    canonical
        .fields
        .sort_by_key(|f| (f.field_id, f.source_fact_id));
    canonical.bindings.sort_by_key(|b| b.binding_id);
    canonical.evidence.sort_by_key(|e| e.evidence_id);
    let input = &canonical;
    let mut out = Vec::new();
    let mut parameters: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for p in &input.parameters {
        parameters.entry(p.signature_id).or_default().push(p);
    }
    for ps in parameters.values_mut() {
        ps.sort_by_key(|p| p.ordinal);
    }
    for member in &input.members {
        let subject = Subject::Member(PublicMemberId::from_storage(member.member_id));
        let declarations: BTreeSet<_> = input
            .bindings
            .iter()
            .filter(|b| b.member_id == member.member_id)
            .filter_map(|b| b.declaration_node_id)
            .chain(member.operation_node_id)
            .collect();
        let signatures: Vec<_> = input
            .signatures
            .iter()
            .filter(|s| {
                declarations.contains(&s.callable_node_id)
                    || input.constructors.iter().any(|c| {
                        declarations.contains(&c.class_node_id) && c.signature_id == s.signature_id
                    })
            })
            .collect();
        let mut text = format!("{} ({})", member.access_path, member.kind);
        let mut anchors = Vec::new();
        for s in signatures {
            text.push_str(&format!(
                "\n{} signature {}: {}",
                s.role, s.form, member.access_path
            ));
            for p in parameters.get(&s.signature_id).into_iter().flatten() {
                text.push_str(&format!(
                    "\n  {} [{}] annotation={} default={} required={:?}",
                    p.name.as_deref().unwrap_or("<unresolved>"),
                    p.kind.as_deref().unwrap_or("unknown"),
                    p.annotation_text
                        .as_deref()
                        .or(p.provider_annotation.as_deref())
                        .unwrap_or("unknown"),
                    p.default_text.as_deref().unwrap_or(&p.default_state),
                    p.required
                ));
            }
            if let Some(doc) = &s.docstring {
                text.push('\n');
                text.push_str(doc);
            }
            anchors.push(Anchor::Declaration {
                fact_id: s.source_fact_id,
            });
            anchors.extend(
                input
                    .evidence
                    .iter()
                    .filter(|e| e.subject_node_id == s.callable_node_id && e.role == "declares")
                    .map(|e| Anchor::Catalog {
                        evidence: EvidenceId::from_storage(e.evidence_id),
                    }),
            );
        }
        for field in input
            .fields
            .iter()
            .filter(|f| declarations.contains(&f.class_node_id))
        {
            text.push_str(&format!(
                "\nConfiguration {}: {} = {}",
                field.name,
                field.annotation_text.as_deref().unwrap_or("unknown"),
                field
                    .default_text
                    .as_deref()
                    .unwrap_or(&field.default_state)
            ));
            anchors.push(Anchor::Declaration {
                fact_id: field.source_fact_id,
            });
        }
        anchors.extend(
            input
                .bindings
                .iter()
                .filter(|b| b.member_id == member.member_id)
                .map(|b| Anchor::Declaration {
                    fact_id: b.source_fact_id,
                }),
        );
        anchors.extend(
            input
                .evidence
                .iter()
                .filter(|e| declarations.contains(&e.subject_node_id) && e.role == "declares")
                .map(|e| Anchor::Catalog {
                    evidence: EvidenceId::from_storage(e.evidence_id),
                }),
        );
        anchors.sort();
        anchors.dedup();
        out.push(unit(
            Family::ApiOptions,
            member.member_id,
            member.access_path.clone(),
            text,
            vec![subject.clone()],
            anchors,
        ));
        for evidence in input
            .evidence
            .iter()
            .filter(|e| declarations.contains(&e.subject_node_id) && e.role == "declares")
        {
            let key = IdHasher::new("retrieval-member-source")
                .id(member.member_id)
                .id(evidence.evidence_id)
                .finish_id();
            out.push(unit(
                Family::Source,
                key,
                member.access_path.clone(),
                format!("{}\n{}", member.access_path, evidence.text),
                vec![subject.clone()],
                vec![Anchor::Catalog {
                    evidence: EvidenceId::from_storage(evidence.evidence_id),
                }],
            ));
        }
    }
    let spans: BTreeMap<_, _> = input.spans.iter().map(|s| (s.span_id, s)).collect();
    let artifacts: BTreeMap<_, _> = input.artifacts.iter().map(|a| (a.artifact_id, a)).collect();
    let text = |id: Id| -> Result<String, WireError> {
        let span = spans
            .get(&id)
            .ok_or_else(|| bad("retrieval span missing"))?;
        let a = artifacts
            .get(&span.artifact_id)
            .ok_or_else(|| bad("retrieval artifact missing"))?;
        let bytes = a
            .body
            .0
            .get(span.start_byte as usize..span.end_byte as usize)
            .ok_or_else(|| bad("retrieval span range"))?;
        Ok(String::from_utf8_lossy(bytes).into_owned())
    };
    let subjects = |kind: &str, id: Id| -> Vec<Subject> {
        let mut subjects: BTreeSet<_> = input
            .associations
            .iter()
            .filter(|a| a.evidence_kind == kind && a.evidence_id == id)
            .filter_map(|a| {
                a.member_id
                    .map(|m| Subject::Member(PublicMemberId::from_storage(m)))
                    .or(a.release_id.map(Subject::Release))
            })
            .collect();
        if subjects.is_empty()
            && kind == "span"
            && let Some(a) = spans.get(&id).and_then(|s| artifacts.get(&s.artifact_id))
        {
            subjects.insert(Subject::Release(a.release_id));
        }
        subjects.into_iter().collect()
    };
    for scenario in &input.scenarios {
        let detail: ScenarioDetail =
            serde_json::from_str(&scenario.detail).map_err(|e| bad(e.to_string()))?;
        let mut rendered = format!(
            "Scenario intent={} checks={}\n",
            encoded(&detail.intent)?,
            encoded(&detail.checks)?
        );
        for span in &detail.spans {
            rendered.push_str(&text(*span)?);
            rendered.push('\n');
        }
        out.push(unit(
            Family::Scenario,
            scenario.scenario_id,
            "Usage scenario".into(),
            rendered,
            subjects("scenario", scenario.scenario_id),
            vec![Anchor::Original {
                evidence: EvidenceRef::Scenario(ScenarioId::from_storage(scenario.scenario_id)),
            }],
        ));
    }
    for deployment in &input.deployments {
        let detail: DeploymentDetail =
            serde_json::from_str(&deployment.detail).map_err(|e| bad(e.to_string()))?;
        out.push(unit(
            Family::DocumentationDeployment,
            deployment.deployment_id,
            detail.field.clone(),
            format!(
                "Declared {} {:?}\n{}",
                detail.field,
                detail.name,
                text(deployment.span_id)?
            ),
            subjects("deployment", deployment.deployment_id),
            vec![Anchor::Original {
                evidence: EvidenceRef::Deployment(DeploymentId::from_storage(
                    deployment.deployment_id,
                )),
            }],
        ));
    }
    for span in &input.spans {
        if artifacts
            .get(&span.artifact_id)
            .is_some_and(|a| a.source_kind == "document")
            || input.associations.iter().any(|a| {
                a.evidence_kind == "span"
                    && a.evidence_id == span.span_id
                    && matches!(a.role.as_str(), "documents" | "suggests")
            })
        {
            let a = artifacts
                .get(&span.artifact_id)
                .ok_or_else(|| bad("missing document artifact"))?;
            out.push(unit(
                Family::DocumentationDeployment,
                span.span_id,
                a.path.clone(),
                text(span.span_id)?,
                subjects("span", span.span_id),
                vec![Anchor::Original {
                    evidence: EvidenceRef::Span(SpanId::from_storage(span.span_id)),
                }],
            ));
        }
    }
    out.sort_by_key(|u| u.unit_id);
    if out.len() > 200_000 || out.iter().map(|u| u.text.len()).sum::<usize>() > 128 * 1024 * 1024 {
        return Err(bad("resource_refused: retrieval unit budget"));
    }
    Ok(out)
}

/// Decode declared projected tables once; no store or query service belongs to this operator.
pub fn from_projection(
    tables: &BTreeMap<String, Vec<arrow_array::RecordBatch>>,
) -> Result<RetrievalInputs, crate::serving_projection::ProjectionError> {
    use crate::serving_projection::projected_rows as read;
    Ok(RetrievalInputs {
        members: read::<CatalogMembers>(tables)?,
        bindings: read::<CatalogBindings>(tables)?,
        signatures: read::<CatalogSignatures>(tables)?,
        constructors: read::<CatalogConstructors>(tables)?,
        parameters: read::<CatalogParameters>(tables)?,
        fields: read::<CatalogConfigurations>(tables)?,
        evidence: read::<crate::catalog::CatalogEvidence>(tables)?,
        artifacts: read::<CatalogArtifacts>(tables)?,
        spans: read::<CatalogSpans>(tables)?,
        scenarios: read::<CatalogScenarios>(tables)?,
        deployments: read::<CatalogDeployments>(tables)?,
        associations: read::<CatalogAssociations>(tables)?,
    })
}
