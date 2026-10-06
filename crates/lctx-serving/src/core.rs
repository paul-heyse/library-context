//! Indivisible mandatory operation packets from exact scoped canonical records.
use lctx_model::domain::{*, serving::*, normalized::{callables::*,entities::*}};
use lctx_model::domain::{resources, assertion::Support,serving::mappings::PacketOutput};
use lctx_surrealdb::{NativeReader,batches::CanonicalBatches,reader::target_id};
use crate::{records::{rows,need,wire},claims::Claims};

pub async fn hydrate(reader:&NativeReader,member:&catalog::CatalogMember,budget:&resources::ResourceBudget)->Result<CanonicalBatches,ModelError>{
    let inputs=OperationCore::binding().mapping.sources.iter().map(|r|ValidationInput::of_relation(r,&["id"])).collect::<Vec<_>>();
    crate::scope::hydrate(reader,vec![target_id(graph::Target::Entity(graph::EntityId::of(member.id())))],&inputs,budget).await
}
pub fn packet(source:&CanonicalBatches,member:&catalog::CatalogMember,domains:&[LibraryDomainPacket],limits:&ResourceLimits,budget:&resources::ResourceBudget)->Result<OperationCore,ModelError>{
    let mut charge=charged::StateCharge::new(budget,"native-operation-core-packet");
    let claims=Claims::new(source,budget)?;
    let modules=rows::<source::Module>(source)?;
    let module=need(&modules,member.access)?;
    let releases=domains.iter().flat_map(|d|&d.captures).filter(|c|c.release.input==member.input).map(|c|c.release.clone()).collect::<Vec<_>>();
    let release=releases.first().cloned().ok_or(ModelError::Schema("operation release capture"))?;
    // A member belongs to one captured input; multiple release declarations for that same
    // capture remain explicit in candidate responses and cannot pick an arbitrary core release.
    if releases.iter().any(|r|r!=&release){return Err(ModelError::Conflict("operation release capture ambiguity"))}
    let exposures=rows::<catalog::CatalogExposure>(source)?.into_iter().filter(|r|r.member==member.id()).collect::<Vec<_>>();
    let exposure_ids=exposures.iter().map(Record::id).collect::<std::collections::BTreeSet<_>>();
    let candidates=rows::<catalog::CatalogCandidate>(source)?.into_iter().filter(|r|exposure_ids.contains(&r.exposure)).collect::<Vec<_>>();
    let callables=rows::<catalog::CatalogCallable>(source)?.into_iter().filter(|r|r.member==member.id()).collect::<Vec<_>>();
    let callable_ids=callables.iter().map(Record::id).collect::<std::collections::BTreeSet<_>>();
    let invocations=rows::<catalog::CatalogInvocation>(source)?.into_iter().filter(|r|callable_ids.contains(&r.callable)).collect::<Vec<_>>();
    let assessments=rows::<EffectiveCallableAssessment>(source)?;
    let variants=rows::<SignatureVariant>(source)?;
    let signatures=rows::<calls::Signature>(source)?;
    let parameters=rows::<calls::SignatureParameter>(source)?;
    let shapes=rows::<calls::ParameterShape>(source)?;
    let slots=rows::<SignatureSlot>(source)?;
    let links=rows::<ParameterEntityLink>(source)?;
    let formals=rows::<ParameterEntity>(source)?;
    let subjects=rows::<types::SignatureTypeSubject>(source)?;
    let signature_types=rows::<types::SignatureTypeObservation>(source)?;
    let source_types=rows::<types::TypeObservation>(source)?;
    let type_supports=rows::<types::TypeSupport>(source)?;
    let signature_type_supports=rows::<types::SignatureTypeSupport>(source)?;
    let native_signatures=rows::<types::NativeSignatureObservation>(source)?;
    let options=rows::<catalog::CatalogOption>(source)?.into_iter().filter(|r|r.member==member.id()).collect::<Vec<_>>();
    let option_subjects=rows::<catalog::CatalogOptionSubject>(source)?;
    let defaults=rows::<catalog::CatalogDefault>(source)?;
    let mut invocation_packets=Vec::new();let mut signature_packets=Vec::new();
    for invocation in &invocations {
        let callable=need(&callables,invocation.callable)?;
        let assessment=need(&assessments,callable.assessment)?;
        let variant=need(&variants,invocation.variant)?;
        let signature=need(&signatures,variant.signature)?;
        invocation_packets.push(InvocationPacket{callable:callable.id(),invocation:invocation.id(),assessment:assessment.id(),analysis:assessment.context,knowledge:assessment.signatures,form:Nullable(assessment.descriptor_kind.map(|kind|match kind{DescriptorKind::Function=>selection::InvocationForm::Function,DescriptorKind::InstanceMethod=>selection::InvocationForm::Method,DescriptorKind::ClassMethod=>selection::InvocationForm::Class,DescriptorKind::StaticMethod=>selection::InvocationForm::Static,DescriptorKind::Property=>selection::InvocationForm::Property}))});
        let mut signature_parameters=parameters.iter().filter(|p|p.signature==signature.id()).collect::<Vec<_>>();
        signature_parameters.sort_by_key(|p|p.ordinal);
        let mut parameter_packets=Vec::new();let mut typing=Vec::new();let mut return_types=Vec::new();let mut return_evidence=Vec::new();
        for parameter in signature_parameters {
            let shape=need(&shapes,parameter.shape)?;
            let slot=slots.iter().find(|s|s.parameter==parameter.id() && s.variant==variant.id());
            let parameter_formals=links.iter().filter(|l|l.parameter==parameter.id()).map(|l|l.entity).collect::<Vec<_>>();
            let mut terms=Vec::new();let mut evidence=Vec::new();
            for observation in &signature_types {
                if !matches!(need(&subjects,observation.subject)?,types::SignatureTypeSubject::Parameter{parameter:p} if *p==parameter.id()){continue}
                let q=claims.qualifications.get(&observation.qualification).ok_or(ModelError::Schema("signature typing qualification"))?;
                if q.context!=variant.context{continue}
                let mut proof=vec![ProofReference::from_canonical(derivation::RowRef::of(observation.id()))?];
                proof.extend(signature_type_supports.iter().filter(|s|s.assertion()==observation.id()).map(|s|ProofReference::from_canonical(derivation::RowRef::of(s.id()))).collect::<Result<Vec<_>,_>>()?);
                terms.push(observation.term);evidence.extend(proof.iter().copied());
                typing.push(SignatureTypingPacket{origin:SignatureTypingOrigin::NativeObserved{observation:observation.id(),subject:observation.subject},term:observation.term,qualification:observation.qualification,claim_basis:claims.basis(observation.qualification)?,proof});
            }
            for formal in &parameter_formals {
                if let ParameterEntity::Source{declaration}=need(&formals,*formal)? {
                    for observation in source_types.iter().filter(|o|o.subject==*declaration && o.role==types::TypeRole::Parameter) {
                        let q=claims.qualifications.get(&observation.qualification).ok_or(ModelError::Schema("source typing qualification"))?;
                        if q.context!=variant.context{continue}
                        let mut proof=vec![ProofReference::from_canonical(derivation::RowRef::of(observation.id()))?];
                        proof.extend(type_supports.iter().filter(|s|s.assertion()==observation.id()).map(|s|ProofReference::from_canonical(derivation::RowRef::of(s.id()))).collect::<Result<Vec<_>,_>>()?);
                        terms.push(observation.term);evidence.extend(proof.iter().copied());
                        typing.push(SignatureTypingPacket{origin:SignatureTypingOrigin::SourceDeclared{observation:observation.id(),subject:observation.subject},term:observation.term,qualification:observation.qualification,claim_basis:claims.basis(observation.qualification)?,proof});
                    }
                }
            }
            terms.sort();terms.dedup();evidence.sort();evidence.dedup();
            let mut available_defaults=Vec::new();
            for option in &options {
                let subject=need(&option_subjects,option.subject)?;
                if matches!(subject,catalog::CatalogOptionSubject::Parameter{slot:s} if slot.is_some_and(|slot|slot.id()==*s)) || matches!(subject,catalog::CatalogOptionSubject::SourceParameter{parameter:p} if parameter_formals.contains(p)) {available_defaults.push(DefaultValue::from_canonical(need(&defaults,option.default)?));}
            }
            let default=available_defaults.first().cloned().unwrap_or(if shape.required{DefaultValue::Absent{}}else{DefaultValue::Unavailable{}});
            let default=if available_defaults.iter().all(|d|d==&default){default}else{DefaultValue::Unknown{}};
            parameter_packets.push(ParameterPacket{parameter:parameter.id(),slot:Nullable(slot.map(Record::id)),formals:parameter_formals,ordinal:parameter.ordinal,name:Nullable(shape.name.as_ref().map(|n|Name::new(n.as_str())).transpose().map_err(wire)?),kind:shape.kind,required:shape.required,types:terms,type_evidence:evidence,default});
        }
        for observation in &signature_types {
            if !matches!(need(&subjects,observation.subject)?,types::SignatureTypeSubject::Return{signature:s} if *s==signature.id()){continue}
            let q=claims.qualifications.get(&observation.qualification).ok_or(ModelError::Schema("return typing qualification"))?;
            if q.context!=variant.context{continue}
            let mut proof=vec![ProofReference::from_canonical(derivation::RowRef::of(observation.id()))?];
            proof.extend(signature_type_supports.iter().filter(|s|s.assertion()==observation.id()).map(|s|ProofReference::from_canonical(derivation::RowRef::of(s.id()))).collect::<Result<Vec<_>,_>>()?);
            return_types.push(observation.term);return_evidence.extend(proof.iter().copied());
            typing.push(SignatureTypingPacket{origin:SignatureTypingOrigin::NativeObserved{observation:observation.id(),subject:observation.subject},term:observation.term,qualification:observation.qualification,claim_basis:claims.basis(observation.qualification)?,proof});
        }
        if let Some(callable)=variant.callable {
            let callable_rows=rows::<CallableEntity>(source)?;
            if let CallableEntity::Source{declaration,..}=need(&callable_rows,callable)? {
                for observation in source_types.iter().filter(|o|o.subject==*declaration && o.role==types::TypeRole::Return) {
                    let q=claims.qualifications.get(&observation.qualification).ok_or(ModelError::Schema("source return qualification"))?;
                    if q.context!=variant.context{continue}
                    let mut proof=vec![ProofReference::from_canonical(derivation::RowRef::of(observation.id()))?];
                    proof.extend(type_supports.iter().filter(|s|s.assertion()==observation.id()).map(|s|ProofReference::from_canonical(derivation::RowRef::of(s.id()))).collect::<Result<Vec<_>,_>>()?);
                    return_types.push(observation.term);return_evidence.extend(proof.iter().copied());
                    typing.push(SignatureTypingPacket{origin:SignatureTypingOrigin::SourceDeclared{observation:observation.id(),subject:observation.subject},term:observation.term,qualification:observation.qualification,claim_basis:claims.basis(observation.qualification)?,proof});
                }
            }
        }
        return_types.sort();return_types.dedup();return_evidence.sort();return_evidence.dedup();
        let effective_parameters=match variant.adjustment{SignatureAdjustment::None=>parameter_packets.clone(),SignatureAdjustment::BindClassReceiver|SignatureAdjustment::BindInstanceReceiver|SignatureAdjustment::PropertyAccess=>parameter_packets.iter().skip(1).cloned().collect(),SignatureAdjustment::Unknown=>Vec::new()};
        let complete=assessment.signatures==Knowledge::Known && signature.form!=calls::SignatureForm::NativeUnavailable && variant.adjustment!=SignatureAdjustment::Unknown && variant.native.map(|id|need(&native_signatures,id).map(|n|n.complete)).transpose()?.unwrap_or(true);
        signature_packets.push(SignaturePacket{signature:signature.id(),role:variant.role,native:Nullable(variant.native),variant:variant.id(),analysis:variant.context,form:signature.form,adjustment:variant.adjustment,parameters:parameter_packets,effective_parameters,return_types,return_evidence,typing,complete});
    }
    invocation_packets.sort_by_key(|p|p.invocation);signature_packets.sort_by_key(|p|p.variant);
    let knowledge_values=callables.iter().map(|c|need(&assessments,c.assessment).map(|a|a.signatures)).collect::<Result<Vec<_>,_>>()?;
    let knowledge=if knowledge_values.contains(&Knowledge::Conflicting){Knowledge::Conflicting}else if !knowledge_values.is_empty() && knowledge_values.iter().all(|k|*k==Knowledge::Known){Knowledge::Known}else{Knowledge::Unknown};
    let option_packets=options.iter().map(|o|Ok(OptionPacket{option:o.id(),subject:o.subject,evidence:o.evidence,default:DefaultValue::from_canonical(need(&defaults,o.default)?)})).collect::<Result<Vec<_>,ModelError>>()?;
    let literals=rows::<value::Literal>(source)?.iter().map(LiteralPacket::from_canonical).collect::<Result<Vec<_>,_>>()?;
    let presentations=rows::<types::TypePresentation>(source)?.iter().map(|r|TypePresentationPacket::from_canonical(r,claims.basis(r.qualification)?).map_err(wire)).collect::<Result<Vec<_>,_>>()?;
    let mut path=module.qualified_name.split('.').map(Name::new).collect::<Result<Vec<_>,_>>().map_err(wire)?;path.extend(member.path.iter().map(Name::new).collect::<Result<Vec<_>,_>>().map_err(wire)?);
    let packet=OperationCore{member:member.id(),name:Name::new(member.name.clone()).map_err(wire)?,release,access:AccessProvenance{module:member.access,path,exposures:exposures.iter().map(Record::id).collect(),candidates:candidates.iter().map(Record::id).collect(),basis:Nullable(callables.first().map(|c|c.basis).filter(|b|callables.iter().all(|c|&c.basis==b)))},invocations:invocation_packets,signatures:signature_packets,signature_knowledge:knowledge,options:option_packets,literal_values:literals,type_presentations:presentations,limits:PacketLimits{maximum_page_rows:limits.maximum_page_rows,maximum_response_bytes:limits.default_response_bytes,signature_indivisible:true}};
    charge.grow(serde_json::to_vec(&packet).map_err(ModelError::codec)?.len())?;
    Ok(packet)
}
