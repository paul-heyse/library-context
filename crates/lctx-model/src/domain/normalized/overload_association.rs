//! Original native member correspondence. No shape match, new solver, or body admission.
use super::{
    Rows,
    callable_normalization::{CallableData, CallableDataView, CallableOutput},
    callables::*,
    entities::ResolutionStatus,
};
use crate::domain::assertion::Support;
use crate::domain::{
    attribution::AnalysisContext,
    calls::SignatureRole,
    charged::{ChargedMap, StateCharge},
    resources::ResourceBudget,
    types::*,
    *,
};
use crate::{Domain, DomainCode};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum OverloadAssociationReason {
    IdentityAgreement = 0,
    MissingOrigin = 1,
    ReceiverBasisMissing = 2,
    SpecializationMissing = 3,
    NativeDeclarationUnavailable = 4,
    MultipleIdentityMatches = 5,
    NativeDeclarationIncomplete = 6,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "overload_variant_assessments")]
pub struct OverloadVariantAssessment {
    #[model(key)]
    pub candidate: Id<NativeOverloadCandidate>,
    #[model(key)]
    pub context: Id<AnalysisContext>,
    #[model(key)]
    pub policy: ContentHash,
    pub status: ResolutionStatus,
    pub reason: OverloadAssociationReason,
    pub variant: Option<Id<SignatureVariant>>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "overload_variant_candidates")]
pub struct OverloadVariantCandidate {
    #[model(key)]
    pub assessment: Id<OverloadVariantAssessment>,
    #[model(key)]
    pub variant: Id<SignatureVariant>,
    pub native: Id<NativeSignatureObservation>,
    #[model(key)]
    pub native_support: Id<NativeSignatureSupport>,
    #[model(key)]
    pub trace_support: Id<NativeOverloadSupport>,
}
fn invalid(s: &str) -> ModelError {
    ModelError::Invalid(s.into())
}
type DeclarationKey = (Id<calls::ProviderSymbol>, Id<AnalysisContext>);
type NativeDeclaration<'a> = (&'a SignatureVariant, &'a NativeSignatureObservation);
/// Candidate association has no selected-call or applicability authority. Selection is a separate
/// attributed fact. Missing receiver/solver basis can coexist with a retained original source ID.
pub fn associate(
    data: &CallableData,
    output: &mut CallableOutput,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    associate_view(&data.view(), output, budget)
}
pub fn associate_view(
    data: &CallableDataView<'_>,
    output: &mut CallableOutput,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let mut charge = StateCharge::new(budget, "native-overload-association");
    let mut declarations: ChargedMap<DeclarationKey, Vec<NativeDeclaration<'_>>> =
        Default::default();
    for variant in output.variants.iter() {
        if variant.role != SignatureRole::EffectiveTyped {
            continue;
        }
        let Some(native) = variant.native.and_then(|id| data.native_signatures.get(id)) else {
            continue;
        };
        let Some(origin) = native.metadata_origin else {
            continue;
        };
        let qualification = data
            .qualifications
            .get(native.qualification)
            .ok_or_else(|| invalid("native signature qualification missing"))?;
        if qualification.context != variant.context {
            return Err(invalid("native declaration context mismatch"));
        }
        declarations.update(&mut charge, (origin, variant.context), |values| {
            values.push((variant, native))
        })?;
    }
    let mut trace_supports: ChargedMap<Id<NativeOverloadObservation>, Vec<&NativeOverloadSupport>> =
        Default::default();
    for support in data.overload_supports.iter() {
        trace_supports.update(&mut charge, support.assertion(), |v| v.push(support))?;
    }
    let mut declaration_supports: ChargedMap<
        Id<NativeSignatureObservation>,
        Vec<&NativeSignatureSupport>,
    > = Default::default();
    for support in data.native_signature_supports.iter() {
        declaration_supports.update(&mut charge, support.assertion(), |v| v.push(support))?;
    }
    let mut traces: ChargedMap<Id<NativeOverloadObservation>, Vec<&NativeOverloadCandidate>> =
        Default::default();
    for candidate in data.overload_candidates.iter() {
        traces.update(&mut charge, candidate.trace, |v| v.push(candidate))?;
    }
    // Reserve the result before allocation. Rows retains its own transferred values afterwards.
    charge.grow(data.overload_candidates.len().saturating_mul(1024))?;
    let mut assessments = Rows::new(budget);
    let mut members = Rows::new(budget);
    for trace in data.overload_traces.iter() {
        let q = data
            .qualifications
            .get(trace.qualification)
            .ok_or_else(|| invalid("native trace qualification missing"))?;
        let mut candidates = traces.get(&trace.id()).cloned().unwrap_or_default();
        candidates.sort_by_key(|c| c.ordinal);
        charge.grow(candidates.len().saturating_mul(512))?;
        trace.verify_candidates(&candidates.iter().map(|c| (*c).clone()).collect::<Vec<_>>())?;
        for candidate in candidates {
            let observed = candidate
                .origin
                .and_then(|origin| declarations.get(&(origin, q.context)))
                .map(Vec::as_slice)
                .unwrap_or_default();
            charge.grow(observed.len().saturating_mul(1024))?;
            let trace_basis = trace_supports
                .get(&trace.id())
                .map(Vec::as_slice)
                .unwrap_or_default();
            if trace_basis.is_empty() {
                return Err(invalid("native trace has no attributed run basis"));
            }
            let relevant = observed
                .iter()
                .filter(|(_, native)| {
                    declaration_supports
                        .get(&native.id())
                        .into_iter()
                        .flatten()
                        .any(|ns| {
                            trace_basis.iter().any(|ts| {
                                let Some(n) = ns.attribution() else {
                                    return false;
                                };
                                let Some(t) = ts.attribution() else {
                                    return false;
                                };
                                n.run == t.run && n.surface == t.surface
                            })
                        })
                })
                .collect::<Vec<_>>();
            let eligible = relevant
                .iter()
                .filter(|(v, n)| {
                    n.complete
                        && n.receiver == NativeReceiver::Unbound
                        && v.adjustment == SignatureAdjustment::None
                        && v.callable.is_some()
                        && data
                            .resolutions
                            .get(v.resolution)
                            .is_some_and(|r| r.status == ResolutionStatus::Resolved)
                        && matches!(data.type_terms.get(n.term), Some(TypeTerm::Callable { .. }))
                })
                .collect::<Vec<_>>();
            let (status, reason, variant) = if candidate.origin.is_none() {
                (
                    ResolutionStatus::Unresolved,
                    OverloadAssociationReason::MissingOrigin,
                    None,
                )
            } else if candidate.receiver_basis_required {
                (
                    ResolutionStatus::Unresolved,
                    OverloadAssociationReason::ReceiverBasisMissing,
                    None,
                )
            } else if candidate.generic
                || relevant.iter().any(|(_, n)| {
                    matches!(data.type_terms.get(n.term), Some(TypeTerm::Generic { .. }))
                })
            {
                (
                    ResolutionStatus::Unresolved,
                    OverloadAssociationReason::SpecializationMissing,
                    None,
                )
            } else if eligible.len() == 1 {
                (
                    ResolutionStatus::Resolved,
                    OverloadAssociationReason::IdentityAgreement,
                    Some(eligible[0].0.id()),
                )
            } else if eligible.len() > 1 {
                (
                    ResolutionStatus::Ambiguous,
                    OverloadAssociationReason::MultipleIdentityMatches,
                    None,
                )
            } else if relevant.is_empty() {
                (
                    ResolutionStatus::Unresolved,
                    OverloadAssociationReason::NativeDeclarationUnavailable,
                    None,
                )
            } else {
                (
                    ResolutionStatus::Unresolved,
                    OverloadAssociationReason::NativeDeclarationIncomplete,
                    None,
                )
            };
            let assessment = OverloadVariantAssessment {
                candidate: candidate.id(),
                context: q.context,
                policy: definition(),
                status,
                reason,
                variant,
            };
            for (variant, native) in &relevant {
                for ns in declaration_supports.get(&native.id()).into_iter().flatten() {
                    for ts in trace_basis {
                        if let (Some(n), Some(t)) = (ns.attribution(), ts.attribution())
                            && n.run == t.run
                            && n.surface == t.surface
                        {
                            members.insert(OverloadVariantCandidate {
                                assessment: assessment.id(),
                                variant: variant.id(),
                                native: native.id(),
                                native_support: ns.id(),
                                trace_support: ts.id(),
                            })?;
                        }
                    }
                }
            }
            assessments.insert(assessment)?;
        }
    }
    for row in assessments.iter() {
        output.overload_assessments.insert(row.clone())?;
    }
    for row in members.iter() {
        output.overload_variant_candidates.insert(row.clone())?;
    }
    Ok(())
}
pub fn definition() -> ContentHash {
    ContentHash::of(b"normalized/overload_association/semantic-policy/v1")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        assertion::{Approximation, AssertionQualification},
        attribution::Modality,
        calls::{Signature, SignatureForm},
        normalized::entities::{CallableEntity, EntityReason, EntityRef, SymbolEntityResolution},
        source::CoverageScope,
    };
    fn id<T>(n: u8) -> Id<T> {
        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
            _,
            serde::de::value::Error,
        >::new([n; 16].into_iter()))
        .unwrap()
    }
    #[test]
    fn original_members_associate_without_shapes_and_keep_missing_bases() {
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let mut data = CallableData::new(&budget);
        let mut output = CallableOutput::new(&budget);
        let q = AssertionQualification {
            context: id(1),
            scope: CoverageScope::Artifact { artifact: id(2) }.id(),
            condition: conditions::Diagram::always().id(),
            modality: Modality::Definite,
            approximation: Approximation::Exact,
            assumptions: assumptions::AssumptionSet::empty_id(),
        };
        data.qualifications.insert(q.clone()).unwrap();
        let returns = TypeTerm::None;
        data.type_terms.insert(returns.clone()).unwrap();
        let term = TypeTerm::Callable {
            function: None,
            form: CallableForm::List,
            parameters: id(50),
            param_spec: None,
            returns: returns.id(),
        };
        data.type_terms.insert(term.clone()).unwrap();
        for n in [10, 11] {
            let callable = CallableEntity::External { symbol: id(n) };
            let entity = EntityRef::Callable {
                callable: callable.id(),
            };
            let resolution = SymbolEntityResolution {
                symbol: id(n),
                context: q.context,
                policy: super::definition(),
                status: ResolutionStatus::Resolved,
                entity: Some(entity.id()),
                reason: EntityReason::ProviderExternal,
            };
            data.resolutions.insert(resolution.clone()).unwrap();
            let native_term = TypeTerm::Callable {
                function: Some(id(n)),
                form: CallableForm::List,
                parameters: id(50),
                param_spec: None,
                returns: returns.id(),
            };
            data.type_terms.insert(native_term.clone()).unwrap();
            let (signature, _) = Signature::new(
                &q,
                SignatureRole::EffectiveTyped,
                Some(native_term.id()),
                id(n),
                0,
                SignatureForm::List,
                &[],
            )
            .unwrap();
            data.signatures.insert(signature.clone()).unwrap();
            let native = NativeSignatureObservation {
                qualification: q.id(),
                signature: signature.id(),
                scope: q.scope,
                term: native_term.id(),
                family: None,
                implementation: Some(id(n)),
                metadata_origin: Some(id(n)),
                deprecation: CallableDeprecation::NotDeprecated,
                deprecation_message: None,
                receiver: NativeReceiver::Unbound,
                complete: true,
            };
            data.native_signatures.insert(native.clone()).unwrap();
            data.native_signature_supports
                .insert(NativeSignatureSupport {
                    assertion: native.id(),
                    run: id(80),
                    surface: id(81),
                    evidence: id(82),
                    origin: attribution::Origin::AnalyzerAssertion,
                    mode: attribution::ExtractionMode::NativeTraversal,
                    fidelity: attribution::Fidelity::NativeStructural,
                })
                .unwrap();
            output
                .variants
                .insert(SignatureVariant {
                    signature: signature.id(),
                    role: SignatureRole::EffectiveTyped,
                    native: Some(native.id()),
                    context: q.context,
                    resolution: resolution.id(),
                    callable: Some(callable.id()),
                    assessment: None,
                    adjustment: SignatureAdjustment::None,
                })
                .unwrap();
        }
        let inputs = [
            OverloadCandidateInput {
                term: term.id(),
                origin: Some(id(10)),
                generic: false,
                receiver_basis_required: false,
            },
            OverloadCandidateInput {
                term: term.id(),
                origin: Some(id(11)),
                generic: false,
                receiver_basis_required: false,
            },
            OverloadCandidateInput {
                term: term.id(),
                origin: Some(id(10)),
                generic: true,
                receiver_basis_required: false,
            },
            OverloadCandidateInput {
                term: term.id(),
                origin: Some(id(10)),
                generic: false,
                receiver_basis_required: true,
            },
            OverloadCandidateInput {
                term: term.id(),
                origin: None,
                generic: false,
                receiver_basis_required: false,
            },
        ];
        let (trace, candidates) = NativeOverloadObservation::new(
            q.id(),
            q.scope,
            id(3),
            id(4),
            OverloadSelection::ClosestOnly,
            0,
            &inputs,
        )
        .unwrap();
        data.overload_traces.insert(trace.clone()).unwrap();
        data.overload_supports
            .insert(NativeOverloadSupport {
                assertion: trace.id(),
                run: id(80),
                surface: id(81),
                evidence: id(82),
                origin: attribution::Origin::AnalyzerAssertion,
                mode: attribution::ExtractionMode::NativeTraversal,
                fidelity: attribution::Fidelity::NativeStructural,
            })
            .unwrap();
        for c in &candidates {
            data.overload_candidates.insert(c.clone()).unwrap();
        }
        associate(&data, &mut output, &budget).unwrap();
        let rows = candidates
            .iter()
            .map(|c| {
                output
                    .overload_assessments
                    .iter()
                    .find(|r| r.candidate == c.id())
                    .unwrap()
            })
            .collect::<Vec<_>>();
        assert_eq!(rows[0].status, ResolutionStatus::Resolved);
        assert_eq!(rows[1].status, ResolutionStatus::Resolved);
        assert_ne!(
            rows[0].variant, rows[1].variant,
            "identical terms cannot collapse distinct original members"
        );
        assert_eq!(
            rows[2].reason,
            OverloadAssociationReason::SpecializationMissing
        );
        assert_eq!(
            rows[3].reason,
            OverloadAssociationReason::ReceiverBasisMissing
        );
        assert_eq!(rows[4].reason, OverloadAssociationReason::MissingOrigin);
        assert_eq!(
            trace.selection,
            OverloadSelection::ClosestOnly,
            "association is not selected-call authority"
        );
        let mut tampered = candidates.clone();
        tampered[0].origin = Some(id(11));
        assert!(trace.verify_candidates(&tampered).is_err());
        assert!(trace.verify_candidates(&candidates[..4]).is_err());
        let mut reordered = candidates.clone();
        reordered.swap(0, 1);
        assert!(trace.verify_candidates(&reordered).is_err());
        let tiny = ResourceBudget::fixed(1).unwrap();
        let mut empty = CallableOutput::new(&tiny);
        assert!(matches!(
            associate(&data, &mut empty, &tiny),
            Err(ModelError::Resource { .. })
        ));
        drop(output);
        drop(data);
        assert_eq!(budget.reserved(), 0);
    }
}
