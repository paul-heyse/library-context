//! Summary's finite whole-frame frontier has one physically selected predecessor closure.
//! Referenced artifacts and occurrences are dependencies, never new publication roots.
use crate::{
    consumed_rows::{ClosureTable, PreparedClosure, identifier},
    normalize::call_scope::CallScopes,
    workspace::CompletedInputs,
};
use lctx_model::domain::{execution::summary_production::SummaryData, *};
use std::{any::TypeId, sync::Arc};
pub(super) struct SummaryScopes {
    calls: CallScopes,
    frame: usize,
}
impl SummaryScopes {
    pub(super) async fn prepare(
        access: &CompletedInputs,
        session: &datafusion::prelude::SessionContext,
        model: &Arc<ValidatedModel>,
        budget: &resources::ResourceBudget,
    ) -> Result<Self, ModelError> {
        let inputs: Vec<_> = SummaryData::inputs()
            .into_iter()
            .filter(|input| {
                ![
                    projection::ProjectionSourceAssessment::NAME,
                    projection::ProjectionSnapshot::NAME,
                    projection::ProjectionSnapshotChunk::NAME,
                ]
                .contains(&input.name())
            })
            .collect();
        let tables = inputs
            .iter()
            .map(|input| {
                Ok(ClosureTable {
                    relation: model
                        .relation(input.name())
                        .ok_or(ModelError::Schema(input.name()))?
                        .clone(),
                    alias: access.table_for(input)?,
                })
            })
            .collect::<Result<Vec<_>, ModelError>>()?;
        Self::from_tables(inputs, tables, session, budget).await
    }
    async fn from_tables(
        inputs: Vec<ValidationInput>,
        mut tables: Vec<ClosureTable>,
        session: &datafusion::prelude::SessionContext,
        budget: &resources::ResourceBudget,
    ) -> Result<Self, ModelError> {
        let real = tables.len();
        let idx = |kind: TypeId| {
            tables[..real]
                .iter()
                .position(|table| table.relation.type_id() == kind)
        };
        let source = idx(TypeId::of::<attribution::ProviderRun>())
            .ok_or(ModelError::Schema(attribution::ProviderRun::NAME))?;
        let parent_sources = [
            TypeId::of::<analysis::local::AnalysisInvocation>(),
            TypeId::of::<analysis::model::AnalysisInvocation>(),
            TypeId::of::<analysis::enriched_execution::AnalysisInvocation>(),
            TypeId::of::<analysis::source_call::AnalysisInvocation>(),
        ]
        .into_iter()
        .map(|kind| {
            Ok((
                kind,
                idx(kind)
                    .ok_or_else(|| ModelError::Invalid("Summary predecessor root absent".into()))?,
            ))
        })
        .collect::<Result<Vec<_>, ModelError>>()?;
        let frame = tables.len();
        tables.push(tables[source].clone());
        let mut parents = Vec::new();
        for (kind, original) in parent_sources {
            let root = tables.len();
            tables.push(tables[original].clone());
            parents.push((kind, original, root));
        }
        let native: std::collections::BTreeSet<_> =
            normalized::binding_normalization::BindingData::validation_inputs()
                .into_iter()
                .chain(conditions::entry::EntryData::facts_inputs())
                .chain(execution::summary_path::PathData::inputs())
                .map(|input| input.type_id())
                .collect();
        let calls=CallScopes::from_tables_with(inputs.clone(),tables,session,budget,true,false,|plan,tables|{
   let idx=|kind:TypeId|tables[..real].iter().position(|table|table.relation.type_id()==kind);
   let owner_idx=|kind:TypeId|parents.iter().find(|(parent,_,_)|*parent==kind).map(|(_,_,root)|*root).or_else(||idx(kind));
   let alias=|kind:TypeId|idx(kind).map(|i|identifier(&tables[i].alias));
   // Native identity uses Facts; predecessor semantic diagrams use the exact completed Model
   // vocabulary. Both namespaces are explicit even when their nominal keys coincide.
   for(from,table)in tables[..real].iter().enumerate(){
    let epoch=if stages::is_vocabulary(table.relation.name()){inputs[from].prefix()}else if native.contains(&table.relation.type_id()){Some(stages::PublicationBoundary::Facts)}else{Some(stages::PublicationBoundary::Model)};
    for field in table.relation.fields().iter().filter(|field|!field.list()){
     if let Some((kind,_))=field.target(){
      let target=tables[..real].iter().enumerate().find(|(i,t)|t.relation.type_id()==kind && (!stages::is_vocabulary(t.relation.name())||inputs[*i].prefix()==epoch)).map(|(i,_)|i);
      if let Some(to)=target{plan.follow(from,field.name(),to)?;}
     }
    }
   }
   macro_rules! own{($member:ty,$field:literal,$owner:ty)=>{{if let(Some(member),Some(owner))=(idx(TypeId::of::<$member>()),owner_idx(TypeId::of::<$owner>())){plan.own(member,$field,owner)?;}}};}
   macro_rules! owned_fields{($member:ty,$owner:ty)=>{{if let(Some(member),Some(owner))=(idx(TypeId::of::<$member>()),idx(TypeId::of::<$owner>())){for field in tables[member].relation.fields().iter().filter(|field|!field.list()&&field.target().map(|(kind,_)|kind)==Some(TypeId::of::<$owner>())){plan.own(member,field.name(),owner)?;}}}};}
   let frames=identifier(&tables[frame].alias);
   for kind in[TypeId::of::<analysis::MethodParameters>(),TypeId::of::<analysis::AnalysisDefinition>()]{if let Some(to)=idx(kind){plan.pairs(frame,to,format!("SELECT f.id AS source_id,t.id AS target_id FROM {frames} f CROSS JOIN {} t",identifier(&tables[to].alias)))?;}}
   plan.pairs(frame,source,format!("SELECT f.id AS source_id,t.id AS target_id FROM {frames} f JOIN {} t ON t.input=f.input AND t.context=f.context",identifier(&tables[source].alias)))?;
   for(_,original,root)in &parents{
    let rows=identifier(&tables[*root].alias);plan.pairs(frame,*root,format!("SELECT f.id AS source_id,t.id AS target_id FROM {frames} f JOIN {rows} t ON t.input=f.input AND t.context=f.context"))?;
    plan.pairs(*root,*original,format!("SELECT id AS source_id,id AS target_id FROM {rows}"))?;
   }
   // Every actual advertised predecessor member of the selected frame remains a root, including
   // absent/ambiguous call headers and all restrictions. Dependencies cannot expand other frames.
   own!(local_semantics::LocalContribution,"invocation",analysis::local::AnalysisInvocation);
   own!(local_semantics::LocalGuardContribution,"invocation",analysis::local::AnalysisInvocation);
   own!(local_symbolic::SymbolicFieldStore,"invocation",analysis::local::AnalysisInvocation);
   own!(atom_decision::AtomRestriction,"invocation",analysis::local::AnalysisInvocation);
   own!(analysis::local::AnalysisDerivation,"invocation",analysis::local::AnalysisInvocation);
   own!(analysis::local::AnalysisOutcome,"invocation",analysis::local::AnalysisInvocation);
   own!(analysis::model::AnalysisDerivation,"invocation",analysis::model::AnalysisInvocation);
   own!(analysis::model::AnalysisOutcome,"invocation",analysis::model::AnalysisInvocation);
   own!(execution::protocol_interpretation::ConditionalTerminalFrontier,"invocation",analysis::model::AnalysisInvocation);
   own!(execution::protocol_interpretation::NormalContinuationRestriction,"frontier",execution::protocol_interpretation::ConditionalTerminalFrontier);
   own!(execution::capture_bridge::CapturedEntryBinding,"invocation",analysis::enriched_execution::AnalysisInvocation);
   own!(execution::enriched_records::SourceExecutionInvocation,"invocation",analysis::enriched_execution::AnalysisInvocation);
   own!(execution::enriched_records::BodyExecution,"invocation",analysis::enriched_execution::AnalysisInvocation);
   own!(analysis::enriched_execution::AnalysisOutcome,"invocation",analysis::enriched_execution::AnalysisInvocation);
   own!(analysis::source_call::AnalysisOutcome,"invocation",analysis::source_call::AnalysisInvocation);
   owned_fields!(analysis::local::SupportSource,analysis::local::AnalysisDerivation);
   owned_fields!(analysis::model::SupportSource,analysis::model::AnalysisDerivation);
   own!(transfer::local::TransferAlternative,"transfer",transfer::local::TransferKey);
   own!(transfer::local::TransferSupport,"source",analysis::local::SupportSource);
   own!(transfer::model::TransferSupport,"source",analysis::model::SupportSource);
   own!(transfer::local::TransferSupport,"assertion",transfer::local::TransferAlternative);
   own!(transfer::model::TransferSupport,"assertion",transfer::model::TransferAlternative);
   own!(execution::source_call_records::SourceCallHeader,"attempt",normalized::bindings::CallBindingAttempt);
   // Event roots use the CallScopes virtual namespace to include the entire candidate bag.
   let occurrences=alias(TypeId::of::<source::Occurrence>()).ok_or(ModelError::Schema(source::Occurrence::NAME))?;
   let artifacts=alias(TypeId::of::<source::SourceArtifact>()).ok_or(ModelError::Schema(source::SourceArtifact::NAME))?;
   let qualifications=alias(TypeId::of::<assertion::AssertionQualification>()).ok_or(ModelError::Schema(assertion::AssertionQualification::NAME))?;
   let event=tables.len()-1;let events=identifier(&tables[event].alias);
   plan.pairs(frame,event,format!("SELECT f.id AS source_id,e.id AS target_id FROM {frames} f JOIN {events} e ON e.context=f.context JOIN {occurrences} o ON o.id=e.site JOIN {artifacts} a ON a.id=o.source AND a.input=f.input"))?;
   // Complete native continuation and symbolic association domains of this actual frame. This
   // roots semantic observations, not every artifact, occurrence, provider symbol or evidence row.
   for(kind,subject)in[(TypeId::of::<flow::FlowValueObservation>(),"sink"),(TypeId::of::<normalized::symbolic_fields::SourceFieldClass>(),"class"),(TypeId::of::<normalized::symbolic_fields::SourceFieldStore>(),"target"),(TypeId::of::<normalized::symbolic_fields::SourceFieldReader>(),"access")]{if let Some(to)=idx(kind){plan.pairs(frame,to,format!("SELECT f.id AS source_id,r.id AS target_id FROM {frames} f JOIN {} r ON TRUE JOIN {qualifications} q ON q.id=r.qualification AND q.context=f.context JOIN {occurrences} o ON o.id=r.{subject} JOIN {artifacts} a ON a.id=o.source AND a.input=f.input",identifier(&tables[to].alias)))?;}}
   own!(normalized::symbolic_fields::SourceFieldAssociation,"class",normalized::symbolic_fields::SourceFieldClass);
   own!(normalized::symbolic_fields::SourceFieldReaderLink,"association",normalized::symbolic_fields::SourceFieldAssociation);
   own!(normalized::symbolic_fields::SourceFieldReaderLink,"reader",normalized::symbolic_fields::SourceFieldReader);
   own!(flow::FlowValuePathObservation,"value",flow::FlowValueObservation);
   own!(flow::FlowCallStep,"path",flow::FlowCallPath);
   // Symbolic association and completed capture matching compare source span/kind across
   // independently attributed occurrence roles. Keep every matching candidate before uniqueness.
   if let Some(occurrence_index)=idx(TypeId::of::<source::Occurrence>()){
    plan.pairs(occurrence_index,occurrence_index,format!("SELECT o.id AS source_id,c.id AS target_id FROM {occurrences} o JOIN {occurrences} c ON c.source=o.source AND c.start=o.start AND c.end=o.end AND c.syntax_kind=o.syntax_kind"))?;
   }
   own!(flow::FlowUse,"occurrence",source::Occurrence);
   own!(flow::FlowUseObservation,"use_",flow::FlowUse);
   own!(flow::FlowReachingObservation,"use_",flow::FlowUse);
   own!(flow_inventory::FlowUseInventoryObservation,"use_",flow::FlowUse);
   own!(flow_inventory::FlowUseCandidate,"inventory",flow_inventory::FlowUseInventoryObservation);
   own!(flow_inventory::FlowUseInventoryMember,"inventory",flow_inventory::FlowUseInventoryObservation);
   own!(flow::FlowDefinitionObservation,"definition",flow::FlowDefinition);
   own!(flow::FlowRegionObservation,"scope",lexical::LexicalScope);
   own!(syntax::SyntaxPlacement,"occurrence",source::Occurrence);
   own!(normalized::entities::ParameterEntityLink,"entity",normalized::entities::ParameterEntity);
   own!(declarations::ParameterDeclaration,"parameter",calls::SignatureParameter);
   own!(declarations::SymbolDeclaration,"symbol",calls::ProviderSymbol);
   owned_fields!(normalized::entities::CallableEntity,source::Occurrence);
   owned_fields!(normalized::entities::CallableEntity,calls::ProviderSymbol);
   owned_fields!(normalized::entities::EntityRef,normalized::entities::CallableEntity);
   own!(types::TypeSequenceMember,"sequence",types::TypeSequence);
   // Set memberships are independent in each selected epoch; condition nodes and finite path
   // segments already have forward nominal edges to their exact roots/children.
   for epoch in[stages::PublicationBoundary::Facts,stages::PublicationBoundary::Model]{
    let at=|kind:TypeId|tables[..real].iter().enumerate().position(|(i,t)|t.relation.type_id()==kind&&inputs[i].prefix()==Some(epoch));
    if let(Some(member),Some(owner))=(at(TypeId::of::<assumptions::AssumptionSetMember>()),at(TypeId::of::<assumptions::AssumptionSet>())){plan.own(member,"set",owner)?;}
   }
   // All independently attributed supports of a selected assertion participate; there is no
   // generic reverse source/input edge that would turn evidence into a whole-input collector.
   for(member,table)in tables[..real].iter().enumerate(){if let Some(field)=table.relation.fields().iter().find(|field|field.name()=="assertion")&&let Some((kind,_))=field.target()&&let Some(owner)=idx(kind){plan.own(member,field.name(),owner)?;}}
   if let Some(premises)=idx(TypeId::of::<analysis::native::NativeAssertionPremise>()){
    for field in tables[premises].relation.fields().iter().filter(|field|!field.list()){
     if let Some((kind,_))=field.target()&&let Some(owner)=idx(kind){plan.own(premises,field.name(),owner)?;}
    }
    if let Some(native)=idx(TypeId::of::<analysis::native::NativeQualification>()){plan.own(native,"premise",premises)?;}
   }
   if let(Some(values),Some(placements))=(idx(TypeId::of::<source::Occurrence>()),idx(TypeId::of::<syntax::SyntaxPlacement>())){plan.pairs(values,placements,format!("SELECT o.id AS source_id,p.id AS target_id FROM {occurrences} o JOIN {} p ON p.parent=o.id WHERE o.syntax_kind IN ({},{},{})",identifier(&tables[placements].alias),source::SyntaxKind::StmtReturn.code(),source::SyntaxKind::StmtRaise.code(),source::SyntaxKind::StmtTry.code()))?;}
   Ok(())
  }).await?;
        Ok(Self { calls, frame })
    }
    pub(super) async fn frame(
        &self,
        run: Id<attribution::ProviderRun>,
        budget: &resources::ResourceBudget,
    ) -> Result<PreparedClosure, ModelError> {
        self.calls
            .edges()
            .grain(self.frame, &format!("id=X'{}'", run.hex()), budget)
            .await
    }
    fn select_actual(
        grain: &PreparedClosure,
        table: usize,
        input: &ValidationInput,
    ) -> Result<Option<String>, ModelError> {
        if !SummaryData::consumes_actual(input) {
            return Ok(None);
        }
        let sql = grain.select(table)?;
        let columns = if input.type_id() == TypeId::of::<source::Occurrence>() {
            Some(source::properties::OccurrenceIndex::columns())
        } else if input.type_id() == TypeId::of::<source::SourceArtifact>() {
            Some(source::properties::ArtifactIndex::columns())
        } else if input.type_id() == TypeId::of::<calls::ProviderSymbol>() {
            Some(source::properties::SymbolIndex::columns())
        } else {
            None
        };
        Ok(Some(columns.map_or(sql.clone(), |columns| {
            format!("SELECT {columns} FROM ({sql}) source_properties")
        })))
    }
    pub(super) async fn load(
        &self,
        access: &CompletedInputs,
        grain: &PreparedClosure,
        budget: &resources::ResourceBudget,
    ) -> Result<SummaryData, ModelError> {
        let mut data = SummaryData::new(budget);
        macro_rules! read{($($field:ident:$ty:ty,)*)=>{$({for(table,input)in self.calls.inputs().iter().enumerate().filter(|(_,input)|input.type_id()==TypeId::of::<$ty>()){
   let Some(sql)=Self::select_actual(grain,table,input)?else{continue;};
   if input.type_id()==TypeId::of::<source::Occurrence>(){
    let permit=access.read_at::<source::Occurrence>(input.prefix())?;
    crate::consumed_rows::stream_query_at(&permit,input,grain.session(),&sql,|permit,batch|data.occurrences.visit(permit,batch)).await?;
   }else if input.type_id()==TypeId::of::<source::SourceArtifact>(){
    let permit=access.read_at::<source::SourceArtifact>(input.prefix())?;crate::consumed_rows::stream_query_at(&permit,input,grain.session(),&sql,|permit,batch|data.artifacts.visit(permit,batch)).await?;
   }else if input.type_id()==TypeId::of::<calls::ProviderSymbol>(){
    let permit=access.read_at::<calls::ProviderSymbol>(input.prefix())?;crate::consumed_rows::stream_query_at(&permit,input,grain.session(),&sql,|permit,batch|data.symbols.visit(permit,batch)).await?;
   }else{
    let permit=access.read_at::<$ty>(input.prefix())?;crate::consumed_rows::stream_query_at(&permit,input,grain.session(),&sql,|_,batch|data.visit_actual(input,batch)).await?;
   }
  }})*};}
        lctx_model::normalized_binding_inputs!(read);
        lctx_model::normalized_binding_outputs!(read);
        lctx_model::entry_value_inputs!(read);
        lctx_model::summary_path_inputs!(read);
        lctx_model::summary_owned_inputs!(read);
        lctx_model::summary_vocabulary!(read);
        lctx_model::summary_evidence_inputs!(read);
        Ok(data)
    }
}
#[cfg(test)]
mod summary_scope_controls {
    use super::*;
    use futures::TryStreamExt;
    use lctx_model::domain::normalized::Rows;
    fn nominal<R>(n: u8) -> Id<R> {
        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
            _,
            serde::de::value::Error,
        >::new([n; 16].into_iter()))
        .unwrap()
    }
    #[tokio::test]
    async fn frame_keeps_model_epoch_and_native_pair_evidence_without_unrelated_payload() {
        let model = model().unwrap();
        let session = datafusion::prelude::SessionContext::new();
        let budget = resources::ResourceBudget::fixed(16 << 20).unwrap();
        let inputs: Vec<_> = SummaryData::inputs()
            .into_iter()
            .filter(|input| {
                ![
                    projection::ProjectionSourceAssessment::NAME,
                    projection::ProjectionSnapshot::NAME,
                    projection::ProjectionSnapshotChunk::NAME,
                ]
                .contains(&input.name())
            })
            .collect();
        let tables: Vec<_> = inputs
            .iter()
            .enumerate()
            .map(|(i, input)| ClosureTable {
                relation: model.relation(input.name()).unwrap().clone(),
                alias: format!("summary_scope_{i}"),
            })
            .collect();
        for table in &tables {
            session
                .register_batch(
                    &table.alias,
                    arrow_array::RecordBatch::new_empty(table.relation.schema().clone()),
                )
                .unwrap();
        }
        fn replace<R: Record>(
            session: &datafusion::prelude::SessionContext,
            inputs: &[ValidationInput],
            tables: &[ClosureTable],
            epoch: Option<stages::PublicationBoundary>,
            rows: &[R],
        ) {
            for (i, _input) in inputs.iter().enumerate().filter(|(_, input)| {
                input.type_id() == TypeId::of::<R>() && input.prefix() == epoch
            }) {
                session.deregister_table(&tables[i].alias).unwrap();
                session
                    .register_batch(&tables[i].alias, <R as Record>::encode(rows).unwrap())
                    .unwrap();
            }
        }
        let source = source::SourceArtifact::from_bytes(
            nominal(1),
            format!("{}.py", "p".repeat(6 << 20)),
            b"x",
        )
        .unwrap();
        let unrelated = source::SourceArtifact::from_bytes(
            source.input,
            format!("{}.py", "z".repeat(256 << 10)),
            b"y",
        )
        .unwrap();
        let foreign =
            source::SourceArtifact::from_bytes(nominal(2), "foreign.py".into(), b"x").unwrap();
        let run = attribution::ProviderRun {
            input: source.input,
            context: nominal(3),
            provider: nominal(4),
            configuration: ContentHash::of(b"run"),
            requested_families: ContentHash::of(b"families"),
        };
        let occurrence = source::Occurrence {
            source: source.id(),
            start: 0,
            end: 1,
            syntax_kind: source::SyntaxKind::ExprName,
            role: source::OccurrenceRole::Read,
            structural_path: vec![1; 2 << 20],
        };
        let foreign_occurrence = source::Occurrence {
            source: foreign.id(),
            structural_path: vec![2; 65536],
            ..occurrence.clone()
        };
        let q = assertion::AssertionQualification {
            context: run.context,
            scope: nominal(5),
            condition: conditions::Condition {
                root: conditions::ConditionNode::True.id(),
            }
            .id(),
            modality: attribution::Modality::Definite,
            approximation: assertion::Approximation::Exact,
            assumptions: assumptions::AssumptionSet::empty().id(),
        };
        let derived = assertion::AssertionQualification {
            condition: conditions::Condition {
                root: conditions::ConditionNode::False.id(),
            }
            .id(),
            ..q.clone()
        };
        let value = flow::FlowValueObservation {
            qualification: q.id(),
            use_: nominal(6),
            sink: occurrence.id(),
            kind: flow::FlowSinkKind::Return,
            transfer: transfer::TransferKind::Identity,
            through_call: false,
        };
        let other_value = flow::FlowValueObservation {
            sink: foreign_occurrence.id(),
            ..value.clone()
        };
        let premise = analysis::native::NativeAssertionPremise::Value {
            assertion: value.id(),
            support: nominal(7),
        };
        let other_premise = analysis::native::NativeAssertionPremise::Value {
            assertion: other_value.id(),
            support: nominal(8),
        };
        let native = analysis::native::NativeQualification {
            premise: premise.id(),
            qualification: q.id(),
            family: attribution::FactFamily::Flow,
            fidelity: attribution::Fidelity::NativeStructural,
            status: analysis::policy::EvidenceStatus::StructurallyObserved,
        };
        let other_native = analysis::native::NativeQualification {
            premise: other_premise.id(),
            ..native.clone()
        };
        let (local, _) =
            analysis::local::AnalysisInvocation::new(run.input, run.context, nominal(9), None, []);
        let (foreign_local, _) =
            analysis::local::AnalysisInvocation::new(nominal(2), run.context, nominal(9), None, []);
        let dependency_decision = atom_decision::AtomDecision {
            invocation: foreign_local.id(),
            leaf: nominal(13),
            support: nominal(14),
            qualification: derived.id(),
            theory: None,
            outcome: atom_decision::AtomOutcome::Mixed,
            reason: None,
        };
        let restriction = atom_decision::AtomRestriction {
            invocation: local.id(),
            decision: dependency_decision.id(),
            original: nominal(11),
            original_support: nominal(12),
            qualification: derived.id(),
            status: analysis::policy::EvidenceStatus::StructurallyObserved,
        };
        let foreign_restriction = atom_decision::AtomRestriction {
            invocation: foreign_local.id(),
            qualification: nominal(15),
            ..restriction.clone()
        };
        replace(
            &session,
            &inputs,
            &tables,
            None,
            &[source.clone(), unrelated, foreign],
        );
        replace(&session, &inputs, &tables, None, std::slice::from_ref(&run));
        replace(
            &session,
            &inputs,
            &tables,
            None,
            &[occurrence.clone(), foreign_occurrence],
        );
        replace(&session, &inputs, &tables, None, &[value, other_value]);
        replace(&session, &inputs, &tables, None, &[premise, other_premise]);
        replace(
            &session,
            &inputs,
            &tables,
            None,
            &[native.clone(), other_native],
        );
        replace(&session, &inputs, &tables, None, &[local, foreign_local]);
        replace(
            &session,
            &inputs,
            &tables,
            None,
            &[restriction, foreign_restriction],
        );
        replace(&session, &inputs, &tables, None, &[dependency_decision]);
        replace(
            &session,
            &inputs,
            &tables,
            Some(stages::PublicationBoundary::Facts),
            std::slice::from_ref(&q),
        );
        replace(
            &session,
            &inputs,
            &tables,
            Some(stages::PublicationBoundary::Model),
            &[q, derived.clone()],
        );
        replace(
            &session,
            &inputs,
            &tables,
            Some(stages::PublicationBoundary::Model),
            &[conditions::Condition {
                root: conditions::ConditionNode::False.id(),
            }],
        );
        replace(
            &session,
            &inputs,
            &tables,
            Some(stages::PublicationBoundary::Model),
            &[conditions::ConditionNode::False],
        );
        replace(
            &session,
            &inputs,
            &tables,
            Some(stages::PublicationBoundary::Model),
            &[value::Literal::Integer {
                decimal: "9".repeat(256 << 10),
            }],
        );
        let scopes = SummaryScopes::from_tables(inputs.clone(), tables, &session, &budget)
            .await
            .unwrap();
        let tiny = resources::ResourceBudget::fixed(96 << 10).unwrap();
        let grain = scopes.frame(run.id(), &tiny).await.unwrap();
        async fn selected<R: Record>(
            grain: &PreparedClosure,
            inputs: &[ValidationInput],
            epoch: Option<stages::PublicationBoundary>,
            budget: &resources::ResourceBudget,
        ) -> Rows<R> {
            let table = inputs
                .iter()
                .position(|input| input.type_id() == TypeId::of::<R>() && input.prefix() == epoch)
                .unwrap();
            let mut rows = Rows::new(budget);
            let mut stream = crate::sql::query(grain.session(), &grain.select(table).unwrap())
                .await
                .unwrap()
                .execute_stream()
                .await
                .unwrap();
            while let Some(batch) = stream.try_next().await.unwrap() {
                rows.decode(&batch).unwrap();
            }
            rows
        }
        let occurrence_table = inputs
            .iter()
            .position(|input| input.type_id() == TypeId::of::<source::Occurrence>())
            .unwrap();
        let permit = analysis::sources::CompletedInput::<source::Occurrence>::new(
            "summary-property-control",
            ContentHash::of(b"contract"),
            ContentHash::of(b"implementation"),
            ContentHash::of(b"stream"),
            2,
        )
        .unwrap();
        let mut properties = source::properties::OccurrenceIndex::new(&tiny);
        let mut stream = crate::sql::query(
            grain.session(),
            &SummaryScopes::select_actual(&grain, occurrence_table, &inputs[occurrence_table])
                .unwrap()
                .unwrap(),
        )
        .await
        .unwrap()
        .execute_stream()
        .await
        .unwrap();
        while let Some(batch) = stream.try_next().await.unwrap() {
            properties.visit(&permit, &batch).unwrap();
        }
        assert_eq!(
            properties.get(occurrence.id()).unwrap().id(),
            occurrence.id()
        );
        assert_eq!(
            properties.get(occurrence.id()).unwrap().syntax_kind(),
            occurrence.syntax_kind
        );
        drop(properties);
        drop(stream);
        assert!(
            SummaryScopes::select_actual(
                &grain,
                inputs
                    .iter()
                    .position(
                        |input| input.type_id() == TypeId::of::<types::FunctionBodyObservation>()
                    )
                    .unwrap(),
                &ValidationInput::of::<types::FunctionBodyObservation>(&["id"])
            )
            .unwrap()
            .is_none()
        );
        let table = inputs
            .iter()
            .position(|input| input.type_id() == TypeId::of::<source::SourceArtifact>())
            .unwrap();
        let permit = analysis::sources::CompletedInput::<source::SourceArtifact>::new(
            "summary-property-control",
            ContentHash::of(b"contract"),
            ContentHash::of(b"implementation"),
            ContentHash::of(b"artifact-stream"),
            3,
        )
        .unwrap();
        let mut artifacts = source::properties::ArtifactIndex::new(&tiny);
        let mut stream = crate::sql::query(
            grain.session(),
            &SummaryScopes::select_actual(&grain, table, &inputs[table])
                .unwrap()
                .unwrap(),
        )
        .await
        .unwrap()
        .execute_stream()
        .await
        .unwrap();
        while let Some(batch) = stream.try_next().await.unwrap() {
            artifacts.visit(&permit, &batch).unwrap();
        }
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts.get(source.id()), Some(source.input));
        drop(artifacts);
        drop(stream);
        let quals = selected::<assertion::AssertionQualification>(
            &grain,
            &inputs,
            Some(stages::PublicationBoundary::Model),
            &tiny,
        )
        .await;
        assert_eq!(quals.get(derived.id()), Some(&derived));
        drop(quals);
        let natives =
            selected::<analysis::native::NativeQualification>(&grain, &inputs, None, &tiny).await;
        assert_eq!(natives.len(), 1);
        assert_eq!(natives.get(native.id()), Some(&native));
        drop(natives);
        let restrictions =
            selected::<atom_decision::AtomRestriction>(&grain, &inputs, None, &tiny).await;
        assert_eq!(restrictions.len(), 1);
        drop(restrictions);
        let literals = selected::<value::Literal>(
            &grain,
            &inputs,
            Some(stages::PublicationBoundary::Model),
            &tiny,
        )
        .await;
        assert_eq!(literals.len(), 0);
        drop(literals);
    }
}
