//! Canonical declaration-domain completeness. No text reason is used as a semantic switch.
use crate::{
    CoreError,
    catalog::{CatalogFacts, Contracts},
    evidence::EvidenceInputs,
};
use cpg_schema::{
    Id, IdHasher,
    codebook::{CoverageStatus, FactFamily},
    selection::catalog::*,
    wire::*,
};
use std::collections::BTreeSet;

pub fn derive(
    facts: &CatalogFacts,
    evidence: &EvidenceInputs,
    catalog: &Contracts,
    snapshot: Id,
) -> Result<Vec<CatalogSelectionDomainsRow>, CoreError> {
    let mut rows = Vec::new();
    let sources = associated_sources(
        &catalog.contextual.associations,
        &catalog.contextual.scenarios,
        &catalog.contextual.deployments,
    )
    .map_err(|e| CoreError::Analysis(e.to_string()))?;
    for member in &catalog.members {
        let mid = PublicMemberId::from_storage(member.member_id);
        let mut all_bindings: Vec<_> = catalog
            .bindings
            .iter()
            .filter(|b| b.member_id == member.member_id && b.role != "shadowed_source")
            .collect();
        all_bindings.sort_by_key(|b| b.binding_id);
        let provider_resolved = all_bindings
            .iter()
            .filter(|b| b.role == "provider_public_observation")
            .all(|b| {
                let candidates: Vec<_> = facts
                    .candidates
                    .iter()
                    .filter(|c| c.public_fact_id == b.source_fact_id)
                    .collect();
                !candidates.is_empty() && candidates.iter().all(|c| c.declaration_node_id.is_some())
            });
        let bindings: Vec<_> = all_bindings
            .iter()
            .copied()
            .filter(|b| b.role != "provider_public_observation")
            .collect();
        let declaration_ids: BTreeSet<_> = bindings
            .iter()
            .filter_map(|b| b.declaration_node_id)
            .chain(member.operation_node_id)
            .collect();
        let access_module = facts
            .names
            .iter()
            .filter(|n| n.access_path == member.access_path)
            .map(|n| n.access_module.as_str())
            .min();
        let modules: BTreeSet<_> = facts
            .declarations
            .iter()
            .filter(|d| declaration_ids.contains(&d.node_id))
            .map(|d| d.module_node_id)
            .chain(
                facts
                    .source
                    .iter()
                    .filter(|f| Some(f.module_name.as_str()) == access_module)
                    .map(|f| f.module_node_id),
            )
            .collect();
        let files: Vec<_> = facts
            .source
            .iter()
            .filter(|f| modules.contains(&f.module_node_id))
            .collect();
        let module = access_module
            .map(str::to_owned)
            .or_else(|| files.iter().map(|f| f.module_name.clone()).min());
        let class_owner = catalog
            .members
            .iter()
            .find(|m| m.kind == "class" && m.access_path == member.owner_path)
            .map(|m| m.access_path.clone());
        let release_ids: BTreeSet<_> = files.iter().map(|f| f.release_id).collect();
        let mut releases = facts
            .releases
            .iter()
            .filter(|r| release_ids.contains(&r.release_id))
            .map(|r| ReleaseScope {
                release_id: r.release_id,
                distributions: r.distributions.clone(),
            })
            .collect::<Vec<_>>();
        releases.sort_by_key(|r| r.release_id);
        let source_evidence = fact_evidence(
            bindings
                .iter()
                .map(|b| b.source_fact_id)
                .chain(files.iter().map(|f| f.fact_id)),
        );
        let source_complete = !modules.is_empty()
            && modules.iter().all(|m| {
                let coverage: Vec<_> = evidence
                    .coverage
                    .iter()
                    .filter(|c| c.scope_node_id == *m && c.fact_family == FactFamily::Exports)
                    .collect();
                !coverage.is_empty()
                    && coverage
                        .iter()
                        .all(|c| c.status == CoverageStatus::CompleteUnderStatedModel)
            });
        let signature_coverage = !modules.is_empty()
            && modules.iter().all(|m| {
                let rows: Vec<_> = evidence
                    .coverage
                    .iter()
                    .filter(|c| c.scope_node_id == *m && c.fact_family == FactFamily::Signatures)
                    .collect();
                !rows.is_empty()
                    && rows
                        .iter()
                        .all(|c| c.status == CoverageStatus::CompleteUnderStatedModel)
            });
        let preserved = provider_resolved
            && !bindings.is_empty()
            && bindings.iter().all(|b| {
                b.declaration_node_id.is_some_and(|node| {
                    catalog
                        .surfaces
                        .iter()
                        .filter(|s| s.declaration_node_id == node)
                        .all(|s| s.admission == "body_preserved")
                })
            });
        let member_context = ApplicableContext {
            context: SelectionContext::Member { member: mid },
            complete: source_complete,
            evidence: source_evidence.clone(),
        };
        let mut signatures = Vec::new();
        for binding in &bindings {
            for sig in catalog.signatures.iter().filter(|s| {
                Some(s.callable_node_id) == binding.declaration_node_id
                    || catalog.constructors.iter().any(|c| {
                        Some(c.class_node_id) == binding.declaration_node_id
                            && c.signature_id == s.signature_id
                    })
            }) {
                signatures.push(ApplicableContext {
                    context: SelectionContext::Signature {
                        member: mid,
                        binding: BindingId::from_storage(binding.binding_id),
                        signature: SignatureId::from_storage(sig.signature_id),
                    },
                    complete: source_complete
                        && signature_coverage
                        && preserved
                        && matches!(sig.form.as_str(), "source_list" | "list"),
                    evidence: vec![
                        WitnessEvidence::Fact {
                            id: binding.source_fact_id,
                        },
                        WitnessEvidence::Fact {
                            id: sig.source_fact_id,
                        },
                    ],
                });
            }
        }
        let signatures_complete=!signatures.is_empty()&&signatures.iter().all(|c|c.complete)&&bindings.iter().all(|b|signatures.iter().any(|c|matches!(&c.context,SelectionContext::Signature{binding,..} if binding.storage()==b.binding_id)));
        let mut field_contexts = std::collections::BTreeMap::<_, Vec<Id>>::new();
        for field in &catalog.configurations {
            if declaration_ids.contains(&field.class_node_id) {
                field_contexts
                    .entry((field.class_node_id, ConfigurationScopeKind::Object))
                    .or_default()
                    .push(field.source_fact_id);
            }
            for link in catalog
                .field_links
                .iter()
                .filter(|l| l.field_id == field.field_id)
            {
                let scope = if link
                    .reader_node_id
                    .is_some_and(|n| declaration_ids.contains(&n))
                {
                    Some(ConfigurationScopeKind::Object)
                } else if catalog.signatures.iter().any(|s| {
                    s.signature_id == link.signature_id
                        && declaration_ids.contains(&s.callable_node_id)
                }) {
                    Some(ConfigurationScopeKind::PerCall)
                } else {
                    None
                };
                if let Some(scope) = scope {
                    field_contexts
                        .entry((field.class_node_id, scope))
                        .or_default()
                        .extend([field.source_fact_id, link.source_fact_id]);
                }
            }
        }
        let fields = field_contexts
            .into_iter()
            .map(|((owner, scope), evidence)| ApplicableContext {
                context: SelectionContext::Configuration {
                    member: mid,
                    owner,
                    scope,
                },
                complete: false,
                evidence: fact_evidence(evidence),
            })
            .collect();
        let scenarios: Vec<_> = catalog
            .contextual
            .associations
            .iter()
            .filter(|a| a.member_id == Some(member.member_id) && a.evidence_kind == "scenario")
            .map(|a| ApplicableContext {
                context: SelectionContext::Scenario {
                    member: mid,
                    scenario: ScenarioId::from_storage(a.evidence_id),
                },
                complete: false,
                evidence: vec![WitnessEvidence::Original {
                    evidence: cpg_schema::evidence::EvidenceRef::Scenario(
                        ScenarioId::from_storage(a.evidence_id),
                    ),
                }],
            })
            .collect();
        let source_contexts = sources
            .iter()
            .filter(|((owner, _), _)| *owner == member.member_id)
            .map(|((_, span), references)| ApplicableContext {
                context: SelectionContext::Source {
                    member: mid,
                    span: SpanId::from_storage(*span),
                },
                complete: false,
                evidence: references
                    .iter()
                    .cloned()
                    .map(|evidence| WitnessEvidence::Original { evidence })
                    .collect(),
            })
            .collect();
        let release_contexts: Vec<_> = release_ids
            .into_iter()
            .map(|release| ApplicableContext {
                context: SelectionContext::Release { release },
                complete: false,
                evidence: source_evidence.clone(),
            })
            .collect();
        let mut detail = DomainDetail {
            module,
            class_owner,
            releases,
            domains: vec![
                ScopedDomain {
                    domain: SelectionDomain::PublicExposures,
                    corpus_complete: source_complete,
                    analyzer_complete: source_complete,
                    contexts: vec![member_context],
                },
                ScopedDomain {
                    domain: SelectionDomain::SignatureVariants,
                    corpus_complete: source_complete,
                    analyzer_complete: signatures_complete,
                    contexts: signatures,
                },
                ScopedDomain {
                    domain: SelectionDomain::ConfigurationFields,
                    corpus_complete: source_complete,
                    analyzer_complete: false,
                    contexts: fields,
                },
                ScopedDomain {
                    domain: SelectionDomain::Relationships,
                    corpus_complete: false,
                    analyzer_complete: false,
                    contexts: vec![],
                },
                ScopedDomain {
                    domain: SelectionDomain::Scenarios,
                    corpus_complete: false,
                    analyzer_complete: false,
                    contexts: dedup_contexts(scenarios),
                },
                ScopedDomain {
                    domain: SelectionDomain::SourceArtifacts,
                    corpus_complete: false,
                    analyzer_complete: false,
                    contexts: dedup_contexts(source_contexts),
                },
                ScopedDomain {
                    domain: SelectionDomain::ReleaseDeclarations,
                    corpus_complete: false,
                    analyzer_complete: false,
                    contexts: release_contexts,
                },
            ],
        };
        // SQL scans and the derivation's input relations are unordered. Nested collections
        // must be canonical before their bytes participate in identity or reconstruction.
        for domain in &mut detail.domains {
            domain.contexts.sort_by(|a, b| a.context.cmp(&b.context));
        }
        let detail =
            serde_json::to_string(&detail).map_err(|e| CoreError::Analysis(e.to_string()))?;
        rows.push(CatalogSelectionDomainsRow {
            snapshot_id: snapshot,
            member_id: member.member_id,
            domain_id: IdHasher::new("selection-domain-v1")
                .id(member.member_id)
                .str(&detail)
                .finish_id(),
            detail,
        });
    }
    rows.sort_by_key(|r| r.member_id);
    Ok(rows)
}

fn fact_evidence(ids: impl IntoIterator<Item = Id>) -> Vec<WitnessEvidence> {
    ids.into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|id| WitnessEvidence::Fact { id })
        .collect()
}

fn dedup_contexts(contexts: Vec<ApplicableContext>) -> Vec<ApplicableContext> {
    contexts
        .into_iter()
        .map(|c| (c.context.clone(), c))
        .collect::<std::collections::BTreeMap<_, _>>()
        .into_values()
        .collect()
}
