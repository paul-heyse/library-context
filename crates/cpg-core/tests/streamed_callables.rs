//! Actual store-free callable scopes. A finite whole-operation oracle checks semantics;
//! no frontend, diagnostic admission replay, store or operator behavior is claimed.
use cpg_core::workspace::{Workspace,WorkspaceOptions};
use std::sync::Arc;
use lctx_model::domain::{
    assertion::*,
    attribution::*,
    calls::*,
    conditions::Diagram,
    input::*,
    normalized::{Rows, callable_normalization::*, callables::*, entities::*},
    resources::ResourceBudget,
    source::*,
    symbols::*,
    syntax::*,
    types::*,
    *,
};
fn fixture() -> (
    CallableData,
    ResourceBudget,
    ProviderSymbol,
    AssertionQualification,
) {
    let budget = ResourceBudget::fixed(32 << 20).unwrap();
    let mut data = CallableData::new(&budget);
    let bytes = b"def f(value=4):\n    return value\n";
    let input = InputRevision::from_entries(vec![ManifestEntry {
        path: "pure.py".into(),
        content: ContentHash::of(bytes),
        byte_len: bytes.len() as i64,
    }])
    .unwrap();
    let source = SourceArtifact::from_bytes(input.id(), "pure.py".into(), bytes).unwrap();
    let scope = CoverageScope::Artifact {
        artifact: source.id(),
    };
    data.scopes.insert(scope.clone()).unwrap();
    let context = AnalysisContext {
        python_version: "3.14".into(),
        python_platform: "pure".into(),
        search_path: vec![],
        site_package_path: vec![],
        config_digest: input.manifest,
        environment_digest: input.manifest,
        lock_digest: None,
    };
    let provider = Provider {
        tool: "pure-callable".into(),
        revision: "1".into(),
        build_digest: input.manifest,
    };
    let (run, _) = ProviderRun::new(
        provider.id(),
        context.id(),
        input.id(),
        input.manifest,
        [
            FactFamily::Syntax,
            FactFamily::Signatures,
            FactFamily::Types,
        ],
    )
    .unwrap();
    data.coverage
        .insert(ProviderCoverage {
            scope: scope.id(),
            provider: Some(provider.id()),
            context: context.id(),
            family: FactFamily::Syntax,
            run: Some(run.id()),
            status: CoverageStatus::CompleteUnderStatedModel,
            reason: None,
            diagnostic: None,
        })
        .unwrap();
    let (condition, _) = Diagram::always().records();
    let q = AssertionQualification {
        assumptions: lctx_model::domain::assumptions::AssumptionSet::empty_id(),
        context: context.id(),
        scope: scope.id(),
        condition: condition.id(),
        modality: Modality::Definite,
        approximation: Approximation::Exact,
    };
    data.qualifications.insert(q.clone()).unwrap();
    let function = Occurrence {
        source: source.id(),
        start: 0,
        end: bytes.len() as i64,
        syntax_kind: SyntaxKind::StmtFunctionDef,
        role: OccurrenceRole::Declaration,
        structural_path: vec![0, 0],
    };
    data.occurrences.insert(function.clone()).unwrap();
    let name = Occurrence {
        start: 4,
        end: 5,
        syntax_kind: SyntaxKind::Identifier,
        role: OccurrenceRole::Syntax,
        structural_path: vec![0, 0, 0],
        ..function.clone()
    };
    data.occurrences.insert(name.clone()).unwrap();
    data.declarations
        .insert(DeclarationObservation {
            qualification: q.id(),
            declaration: function.id(),
            name: name.id(),
            kind: DeclarationKind::Function,
            parent: None,
            overload: false,
            docstring: None,
        })
        .unwrap();
    let module = Module {
        source: source.id(),
        qualified_name: "pure".into(),
    };
    let provider_module = ProviderModule::Acquired {
        module: module.id(),
    };
    let symbol = ProviderSymbol {
        provider: provider.id(),
        context: context.id(),
        module: provider_module.id(),
        native_key: "f".into(),
        name: "f".into(),
        kind: SymbolKind::Function,
    };
    let callable = CallableEntity::Source {
        declaration: function.id(),
        kind: CallableKind::Function,
    };
    data.callables.insert(callable.clone()).unwrap();
    let entity = EntityRef::Callable {
        callable: callable.id(),
    };
    data.refs.insert(entity.clone()).unwrap();
    data.resolutions
        .insert(SymbolEntityResolution {
            symbol: symbol.id(),
            context: context.id(),
            policy: normalized::policy_revision(),
            status: ResolutionStatus::Resolved,
            entity: Some(entity.id()),
            reason: EntityReason::DeclarationAgreement,
        })
        .unwrap();
    data.traits
        .insert(FunctionTraitObservation {
            qualification: q.id(),
            symbol: symbol.id(),
            overload: false,
            staticmethod: false,
            classmethod: false,
            property_getter: false,
            property_setter: false,
            stub: false,
            origin: FunctionOrigin::DefStatement,
            defining_class: None,
            overrides: None,
        })
        .unwrap();
    data.bodies
        .insert(FunctionBodyObservation {
            qualification: q.id(),
            declaration: function.id(),
            body: FunctionBodyKind::Other,
            abstract_method: false,
            in_protocol_class: false,
            in_type_checking_block: false,
            overload: false,
        })
        .unwrap();
    let shape = ParameterShape {
        name: Some("value".into()),
        kind: ParameterKind::PositionalOrKeyword,
        required: false,
    };
    data.shapes.insert(shape.clone()).unwrap();
    let (signature, parameters) = Signature::new(
        &q,
        lctx_model::domain::calls::SignatureRole::Source,
        None,
        symbol.id(),
        0,
        SignatureForm::List,
        &[shape],
    )
    .unwrap();
    data.signatures.insert(signature).unwrap();
    for parameter in parameters {
        data.parameters.insert(parameter).unwrap();
    }
    (data, budget, symbol, q)
}

fn id<T>(n:u8)->Id<T> {serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<_,serde::de::value::Error>::new([n;16].into_iter())).unwrap()}
async fn run(data:&CallableData,memory:usize,batch_rows:usize)->Arc<Workspace> {
    let model=Arc::new(model().unwrap());let workspace=Workspace::new(model.clone(),WorkspaceOptions {memory_bytes:memory,batch_rows,partitions:1}).unwrap();
    let facts=workspace.output("callable-fixture",stages::Profile::Catalog,ContentHash::of(b"fixture"),workspace.inputs("callable-fixture",stages::Profile::Catalog,[]).unwrap());
    macro_rules! emit {($($field:ident:$ty:ty,)*)=>{$(facts.declare::<$ty>().unwrap();for row in data.$field.iter() {facts.push(row.clone()).await.unwrap();})*};}lctx_model::normalized_callable_inputs!(emit);
    facts.finish(stages::ProviderOutcome::Complete).await.unwrap();workspace.freeze_inputs(stages::PublicationBoundary::Facts).unwrap();
    let declaration=stage(stages::Profile::Catalog);assert_eq!(declaration.effect,stages::Effect::Pure);
    let access=workspace.stage_inputs(&declaration,stages::Profile::Catalog).unwrap();let output=workspace.producer(&declaration,stages::Profile::Catalog,access.clone());
    let expected=normalize(data,workspace.budget()).unwrap();
    cpg_core::normalize::callables(access,output,&workspace,&model).await.unwrap();
    let mut actual=CallableOutput::new(workspace.budget());
    macro_rules! read {($($field:ident:$ty:ty,)*)=>{$(for batch in workspace.completed::<$ty>().unwrap().batches().unwrap() {actual.$field.decode(&batch.unwrap()).unwrap();})*};}lctx_model::normalized_callable_outputs!(read);
    actual.matches(&expected).unwrap();drop(actual);drop(expected);
    check_admission(&workspace,|_,_|{}).await.unwrap();
    assert!(workspace.budget().reserved()<1<<20);workspace
}
async fn check_admission(workspace:&Workspace,replace:impl FnOnce(&datafusion::prelude::SessionContext,&[cpg_core::consumed_rows::ClosureTable]))->Result<(),ModelError> {
    let model=model()?;let invariant=invariants().into_iter().find(|invariant|invariant.purpose==InvariantPurpose::Admission).unwrap();
    let inputs=workspace.inputs("callable-admission-fixture",stages::Profile::Catalog,invariant.inputs.iter().map(ValidationInput::name))?;
    let session=inputs.session(workspace).await?;
    let tables=invariant.inputs.iter().map(|input| {let relation=model.relation(input.name()).unwrap().clone();let alias=inputs.table_for(&ValidationInput::of_relation(&relation,&["id"]))?;Ok(cpg_core::consumed_rows::ClosureTable {relation,alias})}).collect::<Result<Vec<_>,ModelError>>()?;
    replace(&session,&tables);
    cpg_core::normalize::validate_callables(&invariant,tables,&session,workspace.budget(),&cpg_core::workspace::Cancellation::default()).await
}
fn replace_rows<R:Record>(session:&datafusion::prelude::SessionContext,tables:&[cpg_core::consumed_rows::ClosureTable],rows:&[R]) {
    let table=tables.iter().find(|table|table.relation.type_id()==std::any::TypeId::of::<R>()).unwrap();let batch=R::encode(rows).unwrap();
    session.deregister_table(table.alias.as_str()).unwrap();session.register_table(table.alias.as_str(),Arc::new(datafusion::datasource::MemTable::try_new(batch.schema(),vec![vec![batch]]).unwrap())).unwrap();
}
fn signatures(data:&mut CallableData,budget:&ResourceBudget,symbol:&ProviderSymbol,q:&AssertionQualification) {
    let source_shapes=[ParameterShape {name:Some("required".into()),kind:ParameterKind::PositionalOnly,required:true},ParameterShape {name:Some("args".into()),kind:ParameterKind::VarPositional,required:false}];
    for row in &source_shapes {data.shapes.insert(row.clone()).unwrap();}
    let (signature,parameters)=Signature::new(q,SignatureRole::Source,None,symbol.id(),1,SignatureForm::List,&source_shapes).unwrap();data.signatures.insert(signature).unwrap();for row in parameters {data.parameters.insert(row).unwrap();}
    let returns=TypeTerm::None;data.type_terms.insert(returns.clone()).unwrap();
    let term=TypeTerm::Callable {function:Some(symbol.id()),form:CallableForm::List,parameters:id(50),param_spec:None,returns:returns.id()};data.type_terms.insert(term.clone()).unwrap();
    for variant in 0..2 {
        let shapes=[ParameterShape {name:Some(format!("native{variant}")),kind:ParameterKind::PositionalOrKeyword,required:false}];data.shapes.insert(shapes[0].clone()).unwrap();
        let (signature,parameters)=Signature::new(q,SignatureRole::EffectiveTyped,Some(term.id()),symbol.id(),variant,SignatureForm::List,&shapes).unwrap();data.signatures.insert(signature.clone()).unwrap();
        for row in parameters {
            let subject=SignatureTypeSubject::Parameter {parameter:row.id()};data.signature_type_subjects.insert(subject.clone()).unwrap();data.signature_types.insert(SignatureTypeObservation {qualification:q.id(),subject:subject.id(),term:returns.id(),scope:q.scope}).unwrap();
            let callable=data.callables.iter().next().unwrap().id();let entity=ParameterEntity::NativeSlot {callable,signature:signature.id(),parameter:row.id()};data.parameter_links.insert(ParameterEntityLink {parameter:row.id(),entity:entity.id(),declaration:None}).unwrap();data.parameters.insert(row).unwrap();
        }
        let subject=SignatureTypeSubject::Return {signature:signature.id()};data.signature_type_subjects.insert(subject.clone()).unwrap();data.signature_types.insert(SignatureTypeObservation {qualification:q.id(),subject:subject.id(),term:returns.id(),scope:q.scope}).unwrap();
        let native=NativeSignatureObservation {qualification:q.id(),signature:signature.id(),scope:q.scope,term:term.id(),family:None,implementation:Some(symbol.id()),metadata_origin:Some(symbol.id()),deprecation:CallableDeprecation::NotDeprecated,deprecation_message:Some(" raw \0 message ".into()),receiver:NativeReceiver::Unbound,complete:true};data.native_signatures.insert(native.clone()).unwrap();
        data.native_signature_supports.insert(NativeSignatureSupport {assertion:native.id(),run:id(80),surface:id(81),evidence:id(82),origin:Origin::AnalyzerAssertion,mode:ExtractionMode::NativeTraversal,fidelity:Fidelity::NativeStructural}).unwrap();
    }
    let root=data.occurrences.iter().find(|row|row.syntax_kind==SyntaxKind::StmtFunctionDef).unwrap().clone();
    let site=Occurrence {start:10,end:11,syntax_kind:SyntaxKind::ExprCall,role:OccurrenceRole::Call,structural_path:vec![0,1],..root.clone()};let arguments=Occurrence {start:11,end:12,syntax_kind:SyntaxKind::Arguments,role:OccurrenceRole::Syntax,structural_path:vec![0,1,0],..root};data.occurrences.insert(site.clone()).unwrap();data.occurrences.insert(arguments.clone()).unwrap();
    let inputs=[OverloadCandidateInput {term:term.id(),origin:Some(symbol.id()),generic:false,receiver_basis_required:false},OverloadCandidateInput {term:term.id(),origin:Some(symbol.id()),generic:true,receiver_basis_required:false},OverloadCandidateInput {term:term.id(),origin:Some(symbol.id()),generic:false,receiver_basis_required:true},OverloadCandidateInput {term:term.id(),origin:None,generic:false,receiver_basis_required:false}];
    let (trace,candidates)=NativeOverloadObservation::new(q.id(),q.scope,site.id(),arguments.id(),OverloadSelection::ClosestOnly,0,&inputs).unwrap();data.overload_traces.insert(trace.clone()).unwrap();for candidate in candidates {data.overload_candidates.insert(candidate).unwrap();}
    data.overload_supports.insert(NativeOverloadSupport {assertion:trace.id(),run:id(80),surface:id(81),evidence:id(82),origin:Origin::AnalyzerAssertion,mode:ExtractionMode::NativeTraversal,fidelity:Fidelity::NativeStructural}).unwrap();
    let output=normalize(data,budget).unwrap();assert_eq!(output.overload_assessments.len(),4);assert_eq!(output.overload_variant_candidates.len(),6);
    let defaults=output.slots.iter().map(|row|row.default.code()).collect::<std::collections::BTreeSet<_>>();assert_eq!(defaults,std::collections::BTreeSet::from([0,1,2,3]));
}
#[tokio::test]
async fn complete_native_overload_and_slot_groups_match_oracle_across_batch_sizes() {
    let (mut data,budget,symbol,q)=fixture();signatures(&mut data,&budget,&symbol,&q);
    let first=run(&data,8<<20,1024).await;let second=run(&data,8<<20,1).await;
    macro_rules! compare {($($field:ident:$ty:ty,)*)=>{$(assert_eq!(first.completed::<$ty>().unwrap().content(),second.completed::<$ty>().unwrap().content());)*};}lctx_model::normalized_callable_outputs!(compare);
    assert_eq!(first.completed::<SignatureVariant>().unwrap().rows(),4);assert_eq!(first.completed::<SignatureSlotType>().unwrap().rows(),2);assert_eq!(first.completed::<SignatureReturnType>().unwrap().rows(),2);
}
#[tokio::test]
async fn ordinary_owned_body_payloads_do_not_enter_tiny_callable_metadata_scopes() {
    let (mut data,_,_,_)=fixture();let root=data.occurrences.iter().find(|row|row.syntax_kind==SyntaxKind::StmtFunctionDef).unwrap().clone();
    let selected_entity=data.refs.iter().next().unwrap().id();
    let unrelated=Occurrence {source:id(77),..root.clone()};data.occurrences.insert(unrelated.clone()).unwrap();
    let unrelated_callable=CallableEntity::Source {declaration:unrelated.id(),kind:CallableKind::Function};data.callables.insert(unrelated_callable.clone()).unwrap();
    let unrelated_entity=EntityRef::Callable {callable:unrelated_callable.id()};data.refs.insert(unrelated_entity.clone()).unwrap();
    // Ordinary bodies from this source and a separate source remain outside metadata grains.
    // The latter has no coverage/context universe, so it introduces no synthetic assessment.
    for ordinal in 0..40 {
        let (owner,entity)=if ordinal<20 {(&root,selected_entity)} else {(&unrelated,unrelated_entity.id())};
        let mut path=vec![0,0,ordinal];path.extend(std::iter::repeat_n(0,50_000));
        let body=Occurrence {start:10,end:11,syntax_kind:SyntaxKind::StmtPass,role:OccurrenceRole::Syntax,structural_path:path,..owner.clone()};
        data.owners.insert(OccurrenceOwnership {occurrence:body.id(),owner:owner.id(),entity}).unwrap();data.occurrences.insert(body).unwrap();
    }
    // A real generator premise must still enter the selected scope.
    let yielding=Occurrence {start:10,end:11,syntax_kind:SyntaxKind::ExprYield,role:OccurrenceRole::Syntax,structural_path:vec![0,0,41],..root.clone()};
    data.owners.insert(OccurrenceOwnership {occurrence:yielding.id(),owner:root.id(),entity:selected_entity}).unwrap();data.occurrences.insert(yielding).unwrap();
    let workspace=run(&data,2<<20,1).await;assert_eq!(workspace.completed::<EffectiveCallableAssessment>().unwrap().rows(),1);
    let assessments=workspace.completed::<EffectiveCallableAssessment>().unwrap().batches().unwrap().map(|batch|EffectiveCallableAssessment::decode(&batch.unwrap()).unwrap()).flatten().collect::<Vec<_>>();
    assert_eq!(assessments[0].generator,Some(true));
}
#[tokio::test]
async fn full_descriptor_alternatives_and_qualified_uncertainty_survive_scope_selection() {
    use lctx_model::domain::{lexical::*,normalized::links::*};
    for uncertainty in [false,true] {
        let (mut data,budget,_,q)=fixture();let root=data.occurrences.iter().find(|row|row.syntax_kind==SyntaxKind::StmtFunctionDef).unwrap().clone();
        let bare=Occurrence {start:2,end:3,syntax_kind:SyntaxKind::ExprName,role:OccurrenceRole::Read,structural_path:vec![0,0,1],..root.clone()};data.occurrences.insert(bare.clone()).unwrap();
        data.decorators.insert(DeclarationDecorator {qualification:q.id(),declaration:root.id(),decorator:bare.id(),ordinal:0}).unwrap();
        let reference=ReferenceObservation {qualification:q.id(),read:bare.id(),scope:id(42),parent:root.id(),field:SyntaxField::Body,name:"staticmethod".into()};data.references.insert(reference.clone()).unwrap();
        let assessment=ReferenceEntityAssessment {reference:reference.id(),status:ResolutionStatus::Resolved,reason:LinkReason::ExplicitIdentity};data.reference_assessments.insert(assessment.clone()).unwrap();
        for index in 0..2 {
            let target=LexicalTarget::Builtin {name:if index==0 || uncertainty {"staticmethod"} else {"classmethod"}.into(),variable:false};data.lexical_targets.insert(target.clone()).unwrap();
            let other=AssertionQualification {modality:Modality::Candidate,..q.clone()};if uncertainty {data.qualifications.insert(other.clone()).unwrap();}
            let raw=LexicalResolution {qualification:if uncertainty && index==1 {other.id()} else {q.id()},read:bare.id(),target:target.id(),captured:index==1};data.lexical_resolutions.insert(raw.clone()).unwrap();let target=ReferenceEntityTarget::Builtin {target:target.id()};data.reference_targets.insert(target.clone()).unwrap();data.reference_candidates.insert(ReferenceEntityCandidate {assessment:assessment.id(),resolution:raw.id(),target:target.id()}).unwrap();
        }
        let expected=normalize(&data,&budget).unwrap();assert!(!expected.assessments.iter().next().unwrap().body_admitted);
        run(&data,8<<20,1).await;
    }
}
#[tokio::test]
async fn all_same_symbol_resolution_context_alternatives_keep_existing_ordered_choice() {
    let (mut data,_,_,_)=fixture();let row=data.resolutions.iter().next().unwrap().clone();data.resolutions.insert(SymbolEntityResolution {context:id(99),..row}).unwrap();
    run(&data,8<<20,1).await;
}
#[tokio::test]
async fn empty_catalog_callable_owner_finishes_all_declared_outputs() {
    let budget=ResourceBudget::fixed(1<<20).unwrap();let data=CallableData::new(&budget);let workspace=run(&data,2<<20,1).await;
    macro_rules! empty {($($field:ident:$ty:ty,)*)=>{$(assert_eq!(workspace.completed::<$ty>().unwrap().rows(),0);)*};}lctx_model::normalized_callable_outputs!(empty);
}

#[tokio::test]
async fn scoped_owner_admission_refuses_changed_claim_missing_membership_and_unsupported_roots() {
    let (mut data,_,_,_)=fixture();let unsupported=CallableEntity::External {symbol:id(96)};data.callables.insert(unsupported.clone()).unwrap();
    let workspace=run(&data,4<<20,1).await;
    let actual=workspace.completed::<EffectiveCallableAssessment>().unwrap().batches().unwrap().flat_map(|batch|EffectiveCallableAssessment::decode(&batch.unwrap()).unwrap()).collect::<Vec<_>>();
    let mut changed=actual.clone();changed[0].generator=Some(true);
    assert!(check_admission(&workspace,|session,tables|replace_rows(session,tables,&changed)).await.is_err());
    assert!(check_admission(&workspace,|session,tables|replace_rows::<CallableEntity>(session,tables,&[])).await.is_err());
    assert!(check_admission(&workspace,|session,tables|replace_rows::<EffectiveCallablePremise>(session,tables,&[])).await.is_err());
    let mut extra=actual.clone();let mut row=actual[0].clone();row.callable=unsupported.id();extra.push(row);
    assert!(check_admission(&workspace,|session,tables|replace_rows(session,tables,&extra)).await.is_err());
    // A standalone structurally valid premise cannot vanish because it has no evidence owner.
    let mut premises=workspace.completed::<EffectiveCallablePremise>().unwrap().batches().unwrap().flat_map(|batch|EffectiveCallablePremise::decode(&batch.unwrap()).unwrap()).collect::<Vec<_>>();
    premises.push(EffectiveCallablePremise::Resolution {resolution:id(97)});
    assert!(check_admission(&workspace,|session,tables|replace_rows(session,tables,&premises)).await.is_err());
}
#[tokio::test]
async fn decorator_container_keeps_all_direct_children_before_recognition() {
    use lctx_model::domain::{lexical::*,normalized::links::*};
    for multiple in [false,true] {
        let (mut data,budget,_,q)=fixture();
        let native=data.traits.iter().next().unwrap().clone();data.traits=Rows::new(&budget);data.traits.insert(FunctionTraitObservation {staticmethod:true,..native}).unwrap();
        let root=data.occurrences.iter().find(|row|row.syntax_kind==SyntaxKind::StmtFunctionDef).unwrap().clone();
        let decorator=Occurrence {start:1,end:4,syntax_kind:SyntaxKind::Decorator,role:OccurrenceRole::Syntax,structural_path:vec![0,0,2],..root.clone()};data.occurrences.insert(decorator.clone()).unwrap();
        let bare=Occurrence {start:2,end:3,syntax_kind:SyntaxKind::ExprName,role:OccurrenceRole::Read,structural_path:vec![0,0,2,0],..root.clone()};data.occurrences.insert(bare.clone()).unwrap();
        data.placements.insert(SyntaxPlacement {qualification:q.id(),occurrence:bare.id(),parent:Some(decorator.id()),field:SyntaxField::Body,ordinal:0}).unwrap();
        if multiple {let other=Occurrence {structural_path:vec![0,0,2,1],..bare.clone()};data.occurrences.insert(other.clone()).unwrap();data.placements.insert(SyntaxPlacement {qualification:q.id(),occurrence:other.id(),parent:Some(decorator.id()),field:SyntaxField::Body,ordinal:1}).unwrap();}
        data.decorators.insert(DeclarationDecorator {qualification:q.id(),declaration:root.id(),decorator:decorator.id(),ordinal:0}).unwrap();
        let reference=ReferenceObservation {qualification:q.id(),read:bare.id(),scope:id(42),parent:root.id(),field:SyntaxField::Body,name:"staticmethod".into()};data.references.insert(reference.clone()).unwrap();
        let assessment=ReferenceEntityAssessment {reference:reference.id(),status:ResolutionStatus::Resolved,reason:LinkReason::ExplicitIdentity};data.reference_assessments.insert(assessment.clone()).unwrap();
        let target=LexicalTarget::Builtin {name:"staticmethod".into(),variable:false};data.lexical_targets.insert(target.clone()).unwrap();let raw=LexicalResolution {qualification:q.id(),read:bare.id(),target:target.id(),captured:false};data.lexical_resolutions.insert(raw.clone()).unwrap();let target=ReferenceEntityTarget::Builtin {target:target.id()};data.reference_targets.insert(target.clone()).unwrap();data.reference_candidates.insert(ReferenceEntityCandidate {assessment:assessment.id(),resolution:raw.id(),target:target.id()}).unwrap();
        let expected=normalize(&data,&budget).unwrap();assert_eq!(expected.assessments.iter().next().unwrap().body_admitted,!multiple);
        run(&data,4<<20,1).await;
    }
}
