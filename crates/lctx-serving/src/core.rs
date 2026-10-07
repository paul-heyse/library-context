//! Indivisible mandatory operation packets from exact scoped canonical records.
use crate::{
    claims::Claims,
    records::{need, rows, wire, Prepared,PacketRows},
};
use lctx_model::domain::{resources, serving::mappings::PacketOutput};
use lctx_model::domain::{
    normalized::{callables::*, entities::*},
    serving::*,
    *,
};
use lctx_surrealdb::{NativeReader, batches::CanonicalBatches, reader::target_id};
use std::collections::BTreeSet;

// The core emits native type presentations, not structural TypeTerm trees. Only IDs
// actually referenced by its signatures/defaults/claim premises need readable inventory.
#[derive(Default)]
struct InventoryRoots {
    terms: BTreeSet<(Id<attribution::AnalysisContext>, Id<types::TypeTerm>)>,
    literals: BTreeSet<[u8; 16]>,
}
impl InventoryRoots {
    fn literal_default(&mut self, value: &DefaultValue) {
        if let DefaultValue::Literal { literal } = value {
            self.literals.insert(*literal.bytes());
        }
    }
    fn basis(&mut self, basis: &ClaimBasisPacket) {
        for definition in &basis.definitions {
            if let ClaimAssumptionPacket::TypeConformance { term, support, .. } = definition {
                self.terms.insert((support.context, *term));
            }
        }
    }
    fn core(core: &OperationCore) -> Self {
        let mut roots = Self::default();
        for signature in &core.signatures {
            roots.terms.extend(
                signature
                    .return_types
                    .iter()
                    .map(|id| (signature.analysis, *id)),
            );
            for typing in &signature.typing {
                roots.terms.insert((signature.analysis, typing.term));
                roots.basis(&typing.claim_basis);
            }
            for parameter in signature
                .parameters
                .iter()
                .chain(&signature.effective_parameters)
            {
                roots
                    .terms
                    .extend(parameter.types.iter().map(|id| (signature.analysis, *id)));
                roots.literal_default(&parameter.default);
            }
        }
        for option in &core.options {
            roots.literal_default(&option.default);
        }
        for default in &core.interpretation.defaults {
            roots.literal_default(&default.value);
        }
        for qualification in &core.interpretation.qualifications {
            roots.basis(&qualification.claim_basis);
        }
        roots
    }
    fn type_literals(&mut self, terms: &PacketRows<types::TypeTerm>) -> Result<(), ModelError> {
        for (_, id) in &self.terms {
            let term = need(terms, *id)?;
            for reference in term
                .references()
                .iter()
                .filter(|r| r.target == value::Literal::NAME)
            {
                self.literals.insert(reference.key);
            }
        }
        Ok(())
    }
    fn literal_values(
        &self,
        literals: &PacketRows<value::Literal>,
    ) -> Result<Vec<LiteralPacket>, ModelError> {
        self.literals
            .iter()
            .map(|key| {
                let row = literals.get_key(key)?
                    .ok_or(ModelError::Schema("core literal dependency"))?;
                LiteralPacket::from_canonical(row)
            })
            .collect()
    }
    fn presentation(
        &self,
        presentation: &types::TypePresentation,
        analysis: Id<attribution::AnalysisContext>,
    ) -> bool {
        self.terms.contains(&(analysis, presentation.term))
    }
}
fn inventory(
    source: &Prepared<'_>,
    core: &mut OperationCore,
    claims: &Claims,
) -> Result<(), ModelError> {
    let mut roots = InventoryRoots::core(core);
    let terms = rows::<types::TypeTerm>(source)?;
    let literals = rows::<value::Literal>(source)?;
    let presentations = rows::<types::TypePresentation>(source)?;
    let mut selected = BTreeSet::new();
    loop {
        let before = selected.len();
        for presentation in &presentations {
            if selected.contains(&presentation.id()) {
                continue;
            }
            if !roots
                .terms
                .iter()
                .any(|(_, term)| *term == presentation.term)
            {
                continue;
            }
            let qualification = claims
                .qualifications
                .get(&presentation.qualification)
                .ok_or(ModelError::Schema("type presentation qualification"))?;
            if !roots.presentation(presentation, qualification.context) {
                continue;
            }
            let basis = claims.basis(presentation.qualification)?;
            roots.basis(&basis);
            core.type_presentations
                .push(TypePresentationPacket::from_canonical(presentation, basis).map_err(wire)?);
            selected.insert(presentation.id());
        }
        if selected.len() == before {
            break;
        }
    }
    roots.type_literals(&terms)?;
    core.literal_values = roots.literal_values(&literals)?;
    core.type_presentations.sort_by_key(|p| p.presentation);
    Ok(())
}

pub async fn hydrate(
    reader: &NativeReader,
    member: &catalog::CatalogMember,
    budget: &resources::ResourceBudget,
) -> Result<CanonicalBatches, ModelError> {
    let inputs = OperationCore::binding()
        .lowered()
        .sources
        .iter()
        .map(|r| ValidationInput::of_relation(r, &["id"]))
        .collect::<Vec<_>>();
    crate::scope::hydrate(
        reader,
        vec![target_id(graph::Target::Entity(graph::EntityId::of(
            member.id(),
        )))],
        &inputs,
        budget,
    )
    .await
}
pub fn packet(
    source: &Prepared<'_>,
    member: &catalog::CatalogMember,
    domains: &[LibraryDomainPacket],
    limits: &ResourceLimits,
    budget: &resources::ResourceBudget,
) -> Result<OperationCore, ModelError> {
    let mut charge = charged::StateCharge::new(budget, "native-operation-core-packet");
    let claims = source.claims()?;
    let modules = rows::<source::Module>(source)?;
    let module = need(&modules, member.access)?;
    let releases = domains
        .iter()
        .flat_map(|d| &d.captures)
        .filter(|c| c.release.input == member.input)
        .map(|c| c.release.clone())
        .collect::<Vec<_>>();
    let release = releases
        .first()
        .cloned()
        .ok_or(ModelError::Schema("operation release capture"))?;
    // A member belongs to one captured input; multiple release declarations for that same
    // capture remain explicit in candidate responses and cannot pick an arbitrary core release.
    if releases.iter().any(|r| r != &release) {
        return Err(ModelError::Conflict("operation release capture ambiguity"));
    }
    let exposures = rows::<catalog::CatalogExposure>(source)?.select_for("member",&[member.id()])?;
    let exposure_ids = exposures
        .iter()
        .map(Record::id)
        .collect::<std::collections::BTreeSet<_>>();
    let candidates = rows::<catalog::CatalogCandidate>(source)?.select_for("exposure",&exposure_ids.iter().copied().collect::<Vec<_>>())?;
    let callables = rows::<catalog::CatalogCallable>(source)?.select_for("member",&[member.id()])?;
    let callable_ids = callables
        .iter()
        .map(Record::id)
        .collect::<std::collections::BTreeSet<_>>();
    let invocations = rows::<catalog::CatalogInvocation>(source)?.select_for("callable",&callable_ids.iter().copied().collect::<Vec<_>>())?;
    let assessments = rows::<EffectiveCallableAssessment>(source)?;
    let variants = rows::<SignatureVariant>(source)?;
    let signatures = rows::<calls::Signature>(source)?;
    let parameters = rows::<calls::SignatureParameter>(source)?;
    let shapes = rows::<calls::ParameterShape>(source)?;
    let slots = rows::<SignatureSlot>(source)?;
    let links = rows::<ParameterEntityLink>(source)?;
    let formals = rows::<ParameterEntity>(source)?;
    let subjects = rows::<types::SignatureTypeSubject>(source)?;
    let signature_types = rows::<types::SignatureTypeObservation>(source)?;
    let source_types = rows::<types::TypeObservation>(source)?;
    let type_supports = rows::<types::TypeSupport>(source)?;
    let signature_type_supports = rows::<types::SignatureTypeSupport>(source)?;
    let native_signatures = rows::<types::NativeSignatureObservation>(source)?;
    let options = rows::<catalog::CatalogOption>(source)?.select_for("member",&[member.id()])?;
    let option_subjects = rows::<catalog::CatalogOptionSubject>(source)?;
    let defaults = rows::<catalog::CatalogDefault>(source)?;
    let default_evidence = rows::<catalog::CatalogOptionEvidence>(source)?;
    let default_syntax = rows::<syntax::ParameterSyntaxObservation>(source)?;
    let mut subjects_checked=false;
    let mut invocation_packets = Vec::new();
    let mut signature_packets = Vec::new();
    for invocation in &invocations {
        let callable = need(&callables, invocation.callable)?;
        let assessment = need(&assessments, callable.assessment)?;
        let variant = need(&variants, invocation.variant)?;
        let signature = need(&signatures, variant.signature)?;
        invocation_packets.push(InvocationPacket {
            callable: callable.id(),
            invocation: invocation.id(),
            assessment: assessment.id(),
            analysis: assessment.context,
            knowledge: assessment.signatures,
            form: Nullable(assessment.descriptor_kind.map(|kind| match kind {
                DescriptorKind::Function => selection::InvocationForm::Function,
                DescriptorKind::InstanceMethod => selection::InvocationForm::Method,
                DescriptorKind::ClassMethod => selection::InvocationForm::Class,
                DescriptorKind::StaticMethod => selection::InvocationForm::Static,
                DescriptorKind::Property => selection::InvocationForm::Property,
            })),
        });
        let selected_parameters=parameters.select_for("signature",&[signature.id()])?;
        let mut signature_parameters=selected_parameters.iter().collect::<Vec<_>>();
        signature_parameters.sort_by_key(|p| p.ordinal);
        let mut parameter_packets = Vec::new();
        let mut typing = Vec::new();
        let mut return_types = Vec::new();
        let mut return_evidence = Vec::new();
        for parameter in signature_parameters {
            let shape = need(&shapes, parameter.shape)?;
            let selected_slots=slots.select_for("parameter",&[parameter.id()])?;
            let slot=selected_slots.iter().find(|slot|slot.variant==variant.id());
            let parameter_links=links.select_for("parameter",&[parameter.id()])?;
            let parameter_formals = parameter_links
                .iter()
                .map(|l| l.entity)
                .collect::<Vec<_>>();
            let mut terms = Vec::new();
            let mut evidence = Vec::new();
            if !subjects_checked{for observation in &signature_types{need(&subjects,observation.subject)?;}subjects_checked=true;}
            let parameter_subjects=subjects.select_for("parameter",&[parameter.id()])?;
            let parameter_observations=signature_types.select_for("subject",&parameter_subjects.iter().map(Record::id).collect::<Vec<_>>())?;
            for observation in &parameter_observations {
                let q = claims
                    .qualifications
                    .get(&observation.qualification)
                    .ok_or(ModelError::Schema("signature typing qualification"))?;
                if q.context != variant.context {
                    continue;
                }
                let mut proof = vec![ProofReference::from_canonical(derivation::RowRef::of(
                    observation.id(),
                ))?];
                proof.extend(
                    signature_type_supports.select_for("assertion",&[observation.id()])?
                        .iter()
                        .map(|s| ProofReference::from_canonical(derivation::RowRef::of(s.id())))
                        .collect::<Result<Vec<_>, _>>()?,
                );
                terms.push(observation.term);
                evidence.extend(proof.iter().copied());
                typing.push(SignatureTypingPacket {
                    origin: SignatureTypingOrigin::NativeObserved {
                        observation: observation.id(),
                        subject: observation.subject,
                    },
                    term: observation.term,
                    qualification: observation.qualification,
                    claim_basis: claims.basis(observation.qualification)?,
                    proof,
                });
            }
            for formal in &parameter_formals {
                if let ParameterEntity::Source { declaration } = need(&formals, *formal)? {
                    let selected_types=source_types.select_for("subject",&[*declaration])?;
                    for observation in selected_types.iter().filter(|observation|observation.role==types::TypeRole::Parameter) {
                        let q = claims
                            .qualifications
                            .get(&observation.qualification)
                            .ok_or(ModelError::Schema("source typing qualification"))?;
                        if q.context != variant.context {
                            continue;
                        }
                        let mut proof = vec![ProofReference::from_canonical(
                            derivation::RowRef::of(observation.id()),
                        )?];
                        proof.extend(
                            type_supports.select_for("assertion",&[observation.id()])?
                        .iter()
                                .map(|s| {
                                    ProofReference::from_canonical(derivation::RowRef::of(s.id()))
                                })
                                .collect::<Result<Vec<_>, _>>()?,
                        );
                        terms.push(observation.term);
                        evidence.extend(proof.iter().copied());
                        typing.push(SignatureTypingPacket {
                            origin: SignatureTypingOrigin::SourceDeclared {
                                observation: observation.id(),
                                subject: observation.subject,
                            },
                            term: observation.term,
                            qualification: observation.qualification,
                            claim_basis: claims.basis(observation.qualification)?,
                            proof,
                        });
                    }
                }
            }
            terms.sort();
            terms.dedup();
            evidence.sort();
            evidence.dedup();
            let mut available_defaults = Vec::new();
            for option in &options {
                let subject = need(&option_subjects, option.subject)?;
                if matches!(subject,catalog::CatalogOptionSubject::Parameter{slot:s} if slot.is_some_and(|slot|slot.id()==*s))
                    || matches!(subject,catalog::CatalogOptionSubject::SourceParameter{parameter:p} if parameter_formals.contains(p))
                {
                    if let catalog::CatalogOptionEvidence::Parameter { syntax, .. }
                    | catalog::CatalogOptionEvidence::SourceParameter { syntax, .. } =
                        need(&default_evidence, option.evidence)?
                    {
                        let declaration = need(&default_syntax, *syntax)?;
                        if claims
                            .qualifications
                            .get(&declaration.qualification)
                            .is_none_or(|q| q.context != variant.context)
                        {
                            continue;
                        }
                    }
                    available_defaults.push(DefaultValue::from_canonical(need(
                        &defaults,
                        option.default,
                    )?));
                }
            }
            let default = available_defaults
                .first()
                .cloned()
                .unwrap_or(if shape.required {
                    DefaultValue::Absent {}
                } else {
                    DefaultValue::Unavailable {}
                });
            let default = if available_defaults.iter().all(|d| d == &default) {
                default
            } else {
                DefaultValue::Unknown {}
            };
            parameter_packets.push(ParameterPacket {
                parameter: parameter.id(),
                slot: Nullable(slot.map(Record::id)),
                formals: parameter_formals,
                ordinal: parameter.ordinal,
                name: Nullable(
                    shape
                        .name
                        .as_ref()
                        .map(|n| Name::new(n.as_str()))
                        .transpose()
                        .map_err(wire)?,
                ),
                kind: shape.kind,
                required: shape.required,
                types: terms,
                type_evidence: evidence,
                default,
            });
        }
        if !subjects_checked{for observation in &signature_types{need(&subjects,observation.subject)?;}subjects_checked=true;}
        let return_subjects=subjects.select_for("signature",&[signature.id()])?;
        let return_observations=signature_types.select_for("subject",&return_subjects.iter().map(Record::id).collect::<Vec<_>>())?;
        for observation in &return_observations {
            let q = claims
                .qualifications
                .get(&observation.qualification)
                .ok_or(ModelError::Schema("return typing qualification"))?;
            if q.context != variant.context {
                continue;
            }
            let mut proof = vec![ProofReference::from_canonical(derivation::RowRef::of(
                observation.id(),
            ))?];
            proof.extend(
                signature_type_supports.select_for("assertion",&[observation.id()])?
                        .iter()
                    .map(|s| ProofReference::from_canonical(derivation::RowRef::of(s.id())))
                    .collect::<Result<Vec<_>, _>>()?,
            );
            return_types.push(observation.term);
            return_evidence.extend(proof.iter().copied());
            typing.push(SignatureTypingPacket {
                origin: SignatureTypingOrigin::NativeObserved {
                    observation: observation.id(),
                    subject: observation.subject,
                },
                term: observation.term,
                qualification: observation.qualification,
                claim_basis: claims.basis(observation.qualification)?,
                proof,
            });
        }
        if let Some(callable) = variant.callable {
            let callable_rows = rows::<CallableEntity>(source)?;
            if let CallableEntity::Source { declaration, .. } = need(&callable_rows, callable)? {
                let selected_types=source_types.select_for("subject",&[*declaration])?;
                for observation in selected_types.iter().filter(|observation|observation.role==types::TypeRole::Return) {
                    let q = claims
                        .qualifications
                        .get(&observation.qualification)
                        .ok_or(ModelError::Schema("source return qualification"))?;
                    if q.context != variant.context {
                        continue;
                    }
                    let mut proof = vec![ProofReference::from_canonical(derivation::RowRef::of(
                        observation.id(),
                    ))?];
                    proof.extend(
                        type_supports.select_for("assertion",&[observation.id()])?
                        .iter()
                            .map(|s| ProofReference::from_canonical(derivation::RowRef::of(s.id())))
                            .collect::<Result<Vec<_>, _>>()?,
                    );
                    return_types.push(observation.term);
                    return_evidence.extend(proof.iter().copied());
                    typing.push(SignatureTypingPacket {
                        origin: SignatureTypingOrigin::SourceDeclared {
                            observation: observation.id(),
                            subject: observation.subject,
                        },
                        term: observation.term,
                        qualification: observation.qualification,
                        claim_basis: claims.basis(observation.qualification)?,
                        proof,
                    });
                }
            }
        }
        return_types.sort();
        return_types.dedup();
        return_evidence.sort();
        return_evidence.dedup();
        let effective_parameters = match variant.adjustment {
            SignatureAdjustment::None => parameter_packets.clone(),
            SignatureAdjustment::BindClassReceiver
            | SignatureAdjustment::BindInstanceReceiver
            | SignatureAdjustment::PropertyAccess => {
                parameter_packets.iter().skip(1).cloned().collect()
            }
            SignatureAdjustment::Unknown => Vec::new(),
        };
        let complete = assessment.signatures == Knowledge::Known
            && signature.form != calls::SignatureForm::NativeUnavailable
            && variant.adjustment != SignatureAdjustment::Unknown
            && variant
                .native
                .map(|id| need(&native_signatures, id).map(|n| n.complete))
                .transpose()?
                .unwrap_or(true);
        signature_packets.push(SignaturePacket {
            signature: signature.id(),
            role: variant.role,
            native: Nullable(variant.native),
            variant: variant.id(),
            analysis: variant.context,
            form: signature.form,
            adjustment: variant.adjustment,
            parameters: parameter_packets,
            effective_parameters,
            return_types,
            return_evidence,
            typing,
            complete,
        });
    }
    invocation_packets.sort_by_key(|p| p.invocation);
    signature_packets.sort_by_key(|p| p.variant);
    let knowledge_values = callables
        .iter()
        .map(|c| need(&assessments, c.assessment).map(|a| a.signatures))
        .collect::<Result<Vec<_>, _>>()?;
    let knowledge = if knowledge_values.contains(&Knowledge::Conflicting) {
        Knowledge::Conflicting
    } else if !knowledge_values.is_empty()
        && knowledge_values.iter().all(|k| *k == Knowledge::Known)
    {
        Knowledge::Known
    } else {
        Knowledge::Unknown
    };
    let option_packets = options
        .iter()
        .map(|o| {
            Ok(OptionPacket {
                option: o.id(),
                subject: o.subject,
                evidence: o.evidence,
                default: DefaultValue::from_canonical(need(&defaults, o.default)?),
            })
        })
        .collect::<Result<Vec<_>, ModelError>>()?;
    let mut path = module
        .qualified_name
        .split('.')
        .map(Name::new)
        .collect::<Result<Vec<_>, _>>()
        .map_err(wire)?;
    path.extend(
        member
            .path
            .iter()
            .map(Name::new)
            .collect::<Result<Vec<_>, _>>()
            .map_err(wire)?,
    );
    let mut packet = OperationCore {
        member: member.id(),
        name: Name::new(member.name.clone()).map_err(wire)?,
        release,
        access: AccessProvenance {
            module: member.access,
            path,
            exposures: exposures.iter().map(Record::id).collect(),
            candidates: candidates.iter().map(Record::id).collect(),
            basis: Nullable(
                callables
                    .first()
                    .map(|c| c.basis)
                    .filter(|b| callables.iter().all(|c| &c.basis == b)),
            ),
        },
        invocations: invocation_packets,
        signatures: signature_packets,
        signature_knowledge: knowledge,
        options: option_packets,
        literal_values: vec![],
        type_presentations: vec![],
        interpretation: InterpretationClosure {
            contexts: vec![],
            defaults: vec![],
            qualifications: vec![],
            availability: Availability::NotRequested {},
        },
        limits: PacketLimits {
            maximum_page_rows: limits.maximum_page_rows,
            maximum_response_bytes: limits.default_response_bytes,
            signature_indivisible: true,
        },
    };
    packet.interpretation = crate::defaults::closure(source, &packet)?;
    inventory(source, &mut packet, &claims)?;
    charge.grow(
        serde_json::to_vec(&packet)
            .map_err(ModelError::codec)?
            .len(),
    )?;
    Ok(packet)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn packet_rows<R:Record>(rows:Vec<R>)->PacketRows<R>{PacketRows::new(rows,&resources::ResourceBudget::fixed(1<<20).unwrap()).unwrap()}
    fn id<T>(n: u8) -> Id<T> {
        serde_json::from_value(serde_json::json!(vec![n; 16])).unwrap()
    }
    fn core(term: Id<types::TypeTerm>, default: Id<value::Literal>) -> OperationCore {
        // An independent small public DTO. Its roots are an int-like literal type and a
        // declared parameter default; broad hydration also has another operation's values.
        serde_json::from_value(serde_json::json!({
            "member":id::<catalog::CatalogMember>(1),"name":"alpha",
            "release":{"input":id::<input::InputRevision>(2),"release":id::<input::Release>(3),"distribution":"mini","version":"1"},
            "access":{"module":id::<source::Module>(4),"path":["alpha","alpha"],"exposures":[],"candidates":[],"basis":null},
            "invocations":[],"signature_knowledge":0,"options":[],"literal_values":[],"type_presentations":[],
            "signatures":[{"signature":id::<calls::Signature>(5),"role":0,"native":null,"variant":id::<SignatureVariant>(6),"analysis":id::<attribution::AnalysisContext>(7),"form":0,"adjustment":0,
                "parameters":[{"parameter":id::<calls::SignatureParameter>(8),"slot":null,"formals":[],"ordinal":0,"name":"red","kind":1,"required":false,"types":[term],"type_evidence":[],"default":{"kind":"literal","literal":default}}],
                "effective_parameters":[],"return_types":[],"return_evidence":[],"typing":[],"complete":true}],
            "interpretation":{"contexts":[],"defaults":[],"qualifications":[],"availability":{"status":"not_requested"}},
            "limits":{"maximum_page_rows":100,"maximum_response_bytes":32768,"signature_indivisible":true}
        })).unwrap()
    }
    #[test]
    fn emitted_core_inventory_excludes_unrelated_values_and_foreign_presentations() {
        let typed = value::Literal::Integer {
            decimal: "3".into(),
        };
        let default = value::Literal::Integer {
            decimal: "2".into(),
        };
        let unrelated = value::Literal::String {
            value: "unrelated".repeat(4096).into(),
        };
        let term = types::TypeTerm::Literal { value: typed.id() };
        let unrelated_term = types::TypeTerm::Literal {
            value: unrelated.id(),
        };
        let packet = core(term.id(), default.id());
        let mut roots = InventoryRoots::core(&packet);
        roots
            .type_literals(&packet_rows(vec![term.clone(), unrelated_term.clone()]))
            .unwrap();
        let values = roots
            .literal_values(&packet_rows(vec![typed.clone(), default.clone(), unrelated]))
            .unwrap();
        assert_eq!(
            values.iter().map(|v| v.literal).collect::<BTreeSet<_>>(),
            BTreeSet::from([typed.id(), default.id()])
        );
        let display = types::TypePresentation {
            qualification: id(9),
            scope: id(10),
            term: term.id(),
            display: "Literal[3]".into(),
            detail: None,
        };
        assert!(roots.presentation(&display, id(7)));
        assert!(
            !roots.presentation(&display, id(11)),
            "shared type identity is not shared analysis"
        );
        let sibling = types::TypePresentation {
            term: unrelated_term.id(),
            display: "sibling".repeat(4096),
            ..display
        };
        assert!(!roots.presentation(&sibling, id(7)));
        // This regression cannot authorize eliding or changing the source signature/default.
        assert_eq!(packet.signatures[0].parameters[0].types, vec![term.id()]);
        assert_eq!(
            packet.signatures[0].parameters[0].default,
            DefaultValue::Literal {
                literal: default.id()
            }
        );
    }
    #[test]
    fn emitted_core_inventory_refuses_missing_required_type_and_literal() {
        let literal = value::Literal::Integer {
            decimal: "2".into(),
        };
        let term = types::TypeTerm::Literal {
            value: literal.id(),
        };
        let packet = core(term.id(), literal.id());
        let mut roots = InventoryRoots::core(&packet);
        assert!(roots.type_literals(&packet_rows(vec![])).is_err());
        roots.type_literals(&packet_rows(vec![term])).unwrap();
        assert!(roots.literal_values(&packet_rows(vec![])).is_err());
        // Presentation absence remains permitted: the helper does not invent a native display.
        assert!(packet.type_presentations.is_empty());
    }
}
