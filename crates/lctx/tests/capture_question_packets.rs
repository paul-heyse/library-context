//! Actual compiler, Summary publication and served direct capture/source boundaries.
#[path = "fixtures/serving_support.rs"]
mod support;
use support::*;
use lctx_model::domain::{
    analysis, assumptions::AssumptionSet, attribution::{Modality, Origin, ExtractionMode, Fidelity}, calls::SignatureRole,
    captures::CaptureTiming, execution::{capture_bridge::*, enriched_records::*, summary_capture::*},
    flow::FlowDefinition, flow_capture::FlowCaptureTimingObservation,
    normalized::entities::ParameterEntity, serving::*, *,
};

#[tokio::test]
async fn direct_capture_answers_preserve_actual_sources_frames_and_characterization() {
    let mut source = b"__all__ = ['captured_entry', 'captured_literal', 'mutation', 'call_before_assignment', 'escaped', 'delayed', 'nonlocal_write', 'global_read', 'loop_capture', 'nested_scope']\n".to_vec();
    source.extend_from_slice(include_bytes!("../../../fixtures/python/stable_capture_shapes/cases.py"));
    let fixture = ServingFixture::start_profile(&source, "behavioral").await;
    let execution = fixture.service.execution().await.unwrap();
    let diagnostic = execution.query(|lease| Box::pin(async move {
        Ok((lease.read::<CapturedEntryBinding>().await?,lease.read::<CapturedValueSource>().await?,
            lease.read::<SummaryCaptureWitness>().await?,lease.read::<declarations::ParameterDeclarationSupport>().await?))
    })).await.unwrap();
    eprintln!("P4_CAPTURE_COUNTS bindings={} values={} witnesses={} declaration_supports={}",diagnostic.0.rows().len(),diagnostic.1.rows().len(),diagnostic.2.rows().len(),diagnostic.3.rows().len());
    for value in diagnostic.1.rows() {
        if let CapturedValueSource::Entry {parameter,..}=value {
            let declarations=execution.query({let parameter=*parameter;move|lease|Box::pin(async move {lease.read_for::<declarations::ParameterDeclaration,calls::SignatureParameter>("parameter",&[parameter]).await})}).await.unwrap();
            for declaration in declarations.rows() {
                for support in diagnostic.3.rows().iter().filter(|support|support.assertion==declaration.id()) {
                    eprintln!("P4_CAPTURE_DECLARATION {parameter:?} origin={:?} mode={:?} fidelity={:?}",support.origin,support.mode,support.fidelity);
                }
            }
        }
    }
    assert_eq!(diagnostic.0.rows().len(),2);
    assert_eq!(diagnostic.2.rows().len(),2);
    assert!(diagnostic.3.rows().iter().any(|support|support.fidelity==Fidelity::ReportProjection));
    drop(diagnostic);
    for (operation, is_literal) in [("demo.captured_entry", false), ("demo.captured_literal", true)] {
        let response = fixture.catalog.operation(&execution, &GetOperationRequest {
            library:Name::new("demo").unwrap(), operation:path(operation),
            sections:vec![OperationSection::Behavior],
            page:PageRequest {size:100,expanded:true,..Default::default()},
        }).await.unwrap();
        let OperationResolution::Unique {packet} = response.operation else {panic!("source operation absent: {operation}")};
        let answers=packet.behavior.items.iter().filter(|answer|!answer.captures.is_empty()).collect::<Vec<_>>();
        assert!(!answers.is_empty(),"direct modeled capture must reach the actual Behavior section: {operation}");
        for answer in answers {
            assert_eq!(answer.captures.len(),1,"one direct captured return, not invented transitive provenance");
            let capture=&answer.captures[0];
            assert_eq!(capture.claim_basis,answer.claim_basis);
            assert_eq!(capture.claim_basis.set,AssumptionSet::empty_id());
            assert!(capture.claim_basis.definitions.is_empty());
            assert!(capture.frame.under_caller_entry);
            assert!(capture.timing.characterization_only);
            assert_eq!(capture.origin.declaring,capture.frame.caller_declaration);
            assert_eq!(capture.native_capture.declaring.0.is_some(),true);
            assert_eq!(capture.native_capture.name.as_str(),"value");
            assert_eq!(capture.timing.proof.support.input,capture.input);
            assert_eq!(capture.timing.proof.support.context,capture.context);
            let (bindings, witnesses, calls, callers, callees, timings, qualifiers, summary) = execution.query({
                let capture=capture.clone(); move |lease| Box::pin(async move {
                    Ok((lease.read_ids::<CapturedEntryBinding>(&[capture.binding]).await?,
                        lease.read_ids::<SummaryCaptureWitness>(&[capture.witness]).await?,
                        lease.read_ids::<SourceExecutionInvocation>(&[capture.frame.call]).await?,
                        lease.read_ids::<BodyExecution>(&[capture.frame.caller_body]).await?,
                        lease.read_ids::<BodyExecution>(&[capture.frame.callee_body]).await?,
                        lease.read_ids::<FlowCaptureTimingObservation>(&[serde_json::from_value(serde_json::json!(capture.timing.proof.assertion.row)).unwrap()]).await?,
                        lease.read_ids::<assertion::AssertionQualification>(&[capture.timing.proof.qualification]).await?,
                        lease.read_for::<analysis::summary::AnalysisInvocation,input::InputRevision>("input",&[capture.input]).await?))
                })
            }).await.unwrap();
            // The canonical value is read via the binding's nominal source, never by name.
            let binding=&bindings.rows()[0];
            let value_id=binding.value_source;
            let value=execution.query(move |lease|Box::pin(async move {lease.read_ids::<CapturedValueSource>(&[value_id]).await})).await.unwrap();
            assert_eq!((binding.read,binding.header,binding.caller,binding.callee),
                (capture.read,capture.frame.header,capture.frame.caller,capture.frame.callee));
            assert_eq!((witnesses.rows()[0].binding,witnesses.rows()[0].call,witnesses.rows()[0].body),
                (capture.binding,capture.frame.call,capture.frame.caller_body));
            assert_eq!(calls.rows()[0].body,capture.frame.callee_body);
            assert_eq!((callers.rows()[0].declaration,callees.rows()[0].declaration),
                (capture.frame.caller_declaration,capture.frame.callee_declaration));
            assert!(summary.rows().iter().any(|i|i.id()==witnesses.rows()[0].invocation&&i.context==capture.context));
            assert_eq!((timings.rows()[0].state,timings.rows()[0].timing), (capture.timing.state,capture.timing.timing));
            assert_eq!((qualifiers.rows()[0].modality,qualifiers.rows()[0].approximation),
                (capture.timing.proof.modality,capture.timing.proof.approximation));
            if capture.timing.timing==CaptureTiming::LazySnapshot {
                assert_eq!(capture.timing.proof.modality,Modality::Candidate,"native lazy AssumeBound stays a candidate");
            }
            for lower in [&capture.origin.proof,&capture.native_capture.proof,&capture.timing.proof] {
                assert_eq!((lower.support.input,lower.support.context),(capture.input,capture.context));
                assert!(!lower.support.provider_revision.as_str().is_empty());
                assert_eq!(lower.claim_basis.set,AssumptionSet::empty_id());
                assert_eq!(lower.support.fidelity,Fidelity::NativeStructural,"source projection never relaxes native value proof admission");
            }
            for relation in ["summary_capture_witnesses","summary_capture_contributions","captured_entry_bindings",
                "source_call_headers","source_execution_invocations","body_executions","capture_supports","flow_capture_timing_supports"] {
                assert!(capture.proof.iter().any(|p|p.relation.as_str()==relation),"actual lower proof {relation}");
            }
            assert!(!capture.proof.iter().any(|p|p.relation.as_str()=="entry_value_witnesses"),"a captured read is not native own-frame access");
            match (&capture.value_source,&value.rows()[0]) {
                (CapturedValueSourcePacket::Entry {formal,parameter,declaration,signature,ordinal,name,source_correspondence},CapturedValueSource::Entry {formal:f,parameter:p,declaration:d})=>{
                    assert!(!is_literal);assert_eq!((formal,parameter,declaration),(f,p,d));assert_eq!(*ordinal,0);assert_eq!(name.0.as_ref().unwrap().as_str(),"value");
                    let (formals,signatures,origin)=execution.query({let formal=*formal;let signature=*signature;let origin=capture.origin.definition;
                        move|lease|Box::pin(async move {Ok((lease.read_ids::<ParameterEntity>(&[formal]).await?,lease.read_ids::<calls::Signature>(&[signature]).await?,lease.read_ids::<FlowDefinition>(&[origin]).await?))})}).await.unwrap();
                    assert_eq!(formals.rows()[0],ParameterEntity::Source {declaration:*declaration});
                    assert_eq!(signatures.rows()[0].role,SignatureRole::Source);
                    assert_ne!(*declaration,origin.rows()[0].occurrence,"formal container and ty Identifier retain distinct nominal roots");
                    assert!(source_correspondence.source_correspondence_only);
                    assert_eq!((source_correspondence.support.origin,source_correspondence.support.mode,source_correspondence.support.fidelity),
                        (Origin::AnalyzerAssertion,ExtractionMode::NativeTraversal,Fidelity::ReportProjection));
                    assert_eq!((source_correspondence.support.input,source_correspondence.support.context),(capture.input,capture.context));
                    assert_eq!(source_correspondence.claim_basis.set,AssumptionSet::empty_id());
                    let declared_id=source_correspondence.assertion;
                    let symbol=signatures.rows()[0].symbol;
                    let (declarations,supports,symbols)=execution.query(move|lease|Box::pin(async move {
                        Ok((lease.read_ids::<declarations::ParameterDeclaration>(&[declared_id]).await?,
                            lease.read_for::<declarations::ParameterDeclarationSupport,declarations::ParameterDeclaration>("assertion",&[declared_id]).await?,
                            lease.read_ids::<calls::ProviderSymbol>(&[symbol]).await?))
                    })).await.unwrap();
                    assert_eq!((declarations.rows()[0].declaration,declarations.rows()[0].parameter),(*declaration,*parameter));
                    assert_eq!(source_correspondence.qualification,declarations.rows()[0].qualification);
                    assert_eq!(source_correspondence.support.provider_id,symbols.rows()[0].provider);
                    let actual=supports.rows().iter().find(|s|*s.id().bytes()==source_correspondence.support.support.row).unwrap();
                    assert_eq!((actual.run,actual.fidelity,actual.origin,actual.mode),
                        (source_correspondence.support.run,source_correspondence.support.fidelity,source_correspondence.support.origin,source_correspondence.support.mode));
                },
                (CapturedValueSourcePacket::Literal {value:v,statement,literal},CapturedValueSource::Literal {literal:stored,value,statement:s})=>{
                    assert!(is_literal);assert_eq!((v,statement),(value,s));assert_eq!(literal.literal,*stored);
                    assert!(matches!(&literal.value,LiteralValue::Integer {decimal} if decimal.as_str()=="7"));
                },
                _=>panic!("served value source must match the actual model-owned source"),
            }
        }
        let mut encoded=serde_json::to_value(&packet.behavior.items[0]).unwrap();
        assert!(encoded["captures"].is_array());
        encoded.as_object_mut().unwrap().remove("captures");
        assert!(serde_json::from_value::<BehaviorPacket>(encoded).is_err(),"capture field is required even when direct provenance is empty");
    }
    for operation in ["mutation","call_before_assignment","escaped","delayed","nonlocal_write","global_read","loop_capture","nested_scope"] {
        let response=fixture.catalog.operation(&execution,&GetOperationRequest {
            library:Name::new("demo").unwrap(),operation:path(&format!("demo.{operation}")),sections:vec![OperationSection::Behavior],
            page:PageRequest {size:100,expanded:true,..Default::default()},
        }).await.unwrap();
        let OperationResolution::Unique {packet}=response.operation else {panic!("unsupported shape is still a source operation: {operation}")};
        assert!(packet.behavior.items.iter().all(|answer|answer.captures.is_empty()),"unsupported {operation} cannot acquire a capture certificate");
    }
    drop(execution);
    fixture.finish().await;
}
