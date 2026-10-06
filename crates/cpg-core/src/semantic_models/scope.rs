//! Explicit publication roots over one immutable externally prepared nominal edge projection.
use crate::{
    consumed_rows::{ClosureTable, PreparedClosure, identifier},
    normalize::call_scope::CallScopes,
    workspace::CompletedInputs,
};
use lctx_model::domain::{
    execution::model_production::{ModelData, ProductionScope, SelectedCatalog},
    *,
};
use std::{any::TypeId, sync::Arc};
pub(super) struct ModelScopes {
    catalog: Id<models::ModelCatalog>,
    calls: CallScopes,
    frame: usize,
    targets: Vec<(Id<models::AuthoredModel>, usize)>,
    contexts: usize,
    terminals: usize,
    exits: usize,
}
fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}
pub(super) fn hex<R: Record>(id: Id<R>) -> String {
    format!(
        "X'{}'",
        id.bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    )
}
impl ModelScopes {
    pub(super) async fn prepare(
        access: &CompletedInputs,
        session: &datafusion::prelude::SessionContext,
        model: &Arc<ValidatedModel>,
        catalog: &SelectedCatalog,
        budget: &resources::ResourceBudget,
    ) -> Result<Self, ModelError> {
        let inputs = ModelData::inputs();
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
        Self::from_tables(inputs, tables, session, catalog, budget).await
    }
    async fn from_tables(
        inputs: Vec<ValidationInput>,
        mut tables: Vec<ClosureTable>,
        session: &datafusion::prelude::SessionContext,
        catalog: &SelectedCatalog,
        budget: &resources::ResourceBudget,
    ) -> Result<Self, ModelError> {
        let real = tables.len();
        let index = |kind: TypeId| {
            tables[..real]
                .iter()
                .position(|t| t.relation.type_id() == kind)
                .ok_or_else(|| ModelError::Invalid("Model scope root absent".into()))
        };
        let frame_source = index(TypeId::of::<attribution::ProviderRun>())?;
        let context_source = index(TypeId::of::<execution::context_execution::ContextExecution>())?;
        let terminal_source = index(TypeId::of::<protocols::NativeTerminalObservation>())?;
        let exit_source = index(TypeId::of::<protocols::NativeExitObservation>())?;
        let frame = tables.len();
        tables.push(tables[frame_source].clone());
        let contexts = tables.len();
        tables.push(tables[context_source].clone());
        let terminals = tables.len();
        tables.push(tables[terminal_source].clone());
        let exits = tables.len();
        tables.push(tables[exit_source].clone());
        let mut targets = Vec::new();
        for compiled in catalog.catalog().models() {
            let root = tables.len();
            tables.push(tables[frame_source].clone());
            targets.push((compiled.declaration().id(), root));
        }
        let calls=CallScopes::from_tables_with(inputs,tables,session,budget,true,false,|plan,tables|{
   let idx=|kind:TypeId|tables[..real].iter().position(|t|t.relation.type_id()==kind);
   let alias=|kind:TypeId|idx(kind).map(|i|identifier(&tables[i].alias));
   macro_rules! own{($member:ty,$field:literal,$owner:ty)=>{if let(Some(member),Some(owner))=(idx(TypeId::of::<$member>()),idx(TypeId::of::<$owner>())){plan.own(member,$field,owner)?;}};}
   own!(types::TypeObservation,"subject",source::Occurrence);
   own!(types::TypeSequenceMember,"sequence",types::TypeSequence);
   own!(class_metadata::ClassMemberObservation,"class",calls::ProviderSymbol);
   own!(class_metadata::ClassMetadataObservation,"class",calls::ProviderSymbol);
   own!(symbols::FunctionTraitObservation,"defining_class",calls::ProviderSymbol);
   own!(calls::CallOriginStep,"origin",calls::CallOrigin);
   own!(input::ArtifactOwnership,"artifact",source::SourceArtifact);
   own!(input::ArtifactUse,"artifact",source::SourceArtifact);
   own!(input::InputAcquisition,"input",input::InputRevision);
   own!(input::EnvironmentFingerprint,"acquisition",input::InputAcquisition);
   own!(execution::context_execution::ContextItem,"execution",execution::context_execution::ContextExecution);
   own!(execution::context_execution::ContextMember,"execution",execution::context_execution::ContextExecution);
   own!(execution::context_binding::BindingMember,"binding",execution::context_binding::ContextEntryBinding);
   own!(execution::modeled_call::ModeledCallEvaluation,"attempt",normalized::bindings::CallBindingAttempt);
   own!(execution::modeled_call::ModeledCallArgument,"call",execution::modeled_call::ModeledCallEvaluation);
   own!(execution::modeled_call::ModeledCallNative,"call",execution::modeled_call::ModeledCallEvaluation);
   own!(execution::records::EvaluationMember,"evaluation",execution::records::ExpressionEvaluation);
   own!(execution::records::EvaluationOperand,"evaluation",execution::records::ExpressionEvaluation);
   own!(conditions::entry::EntryValueWitness,"access_source",conditions::entry::EntryAccessSource);
   // These source-memberships are model-owned finite families, not arbitrary incoming references.
   if let(Some(native),Some(premises))=(idx(TypeId::of::<analysis::native::NativeQualification>()),idx(TypeId::of::<analysis::native::NativeAssertionPremise>())){plan.own(native,"premise",premises)?;}
   if let Some(premises)=idx(TypeId::of::<analysis::native::NativeAssertionPremise>()){
    for field in tables[premises].relation.fields().iter().filter(|f|!f.list()){
     if let Some((kind,_))=field.target() && let Some(assertion)=idx(kind){plan.own(premises,field.name(),assertion)?;}
    }
   }
   macro_rules! pair{($from:ty,$to:ty,$sql:expr)=>{if let(Some(from),Some(to))=(idx(TypeId::of::<$from>()),idx(TypeId::of::<$to>())){plan.pairs(from,to,$sql)?;}};}
   // Exact stored access issuers and complete context bindings are selected by their publication item.
   if let(Some(symbols),Some(observations),Some(quals))=(alias(TypeId::of::<calls::ProviderSymbol>()),idx(TypeId::of::<symbols::SymbolObservation>()),alias(TypeId::of::<assertion::AssertionQualification>())){
    plan.pairs(idx(TypeId::of::<calls::ProviderSymbol>()).unwrap(),observations,format!("SELECT s.id AS source_id,o.id AS target_id FROM {symbols} s JOIN {} o ON o.symbol=s.id JOIN {quals} q ON q.id=o.qualification AND q.context=s.context",identifier(&tables[observations].alias)))?;
   }
   // Native supports are reverse ownership of their exact assertion, never same-input neighbors.
   for (member,table)in tables[..real].iter().enumerate(){if let Some(field)=table.relation.fields().iter().find(|f|f.name()=="assertion") && let Some((kind,_))=field.target() && let Some(owner)=idx(kind){plan.own(member,field.name(),owner)?;}}
   if let(Some(items),Some(bindings))=(alias(TypeId::of::<execution::context_execution::ContextItem>()),alias(TypeId::of::<execution::context_binding::ContextEntryBinding>())){
    pair!(execution::context_execution::ContextItem,execution::context_binding::ContextEntryBinding,format!("SELECT i.id AS source_id,b.id AS target_id FROM {items} i JOIN {bindings} b ON b.item=i.item AND b.site=i.site"));
   }
   if let(Some(bindings),Some(evaluations),Some(base),Some(quals))=(alias(TypeId::of::<execution::context_binding::ContextEntryBinding>()),idx(TypeId::of::<execution::records::ExpressionEvaluation>()),alias(TypeId::of::<analysis::base_evaluation::AnalysisInvocation>()),alias(TypeId::of::<assertion::AssertionQualification>())){
    plan.pairs(idx(TypeId::of::<execution::context_binding::ContextEntryBinding>()).unwrap(),evaluations,format!("SELECT b.id AS source_id,e.id AS target_id FROM {bindings} b JOIN {quals} q ON q.id=b.qualification JOIN {} e ON e.expression=b.entry_actual AND e.owner=b.owner JOIN {base} f ON f.id=e.invocation AND f.context=q.context",identifier(&tables[evaluations].alias)))?;
   }
   let frames=identifier(&tables[frame].alias);
   // Configurations and exact predecessor frames are compact and shared by every owner grain.
   for kind in [TypeId::of::<analysis::MethodParameters>(),TypeId::of::<analysis::AnalysisDefinition>()]{
    if let Some(target)=idx(kind){plan.pairs(frame,target,format!("SELECT f.id AS source_id,t.id AS target_id FROM {frames} f CROSS JOIN {} t",identifier(&tables[target].alias)))?;}
   }
   if let Some(target)=idx(TypeId::of::<models::ModelCatalog>()){
    plan.pairs(frame,target,format!("SELECT f.id AS source_id,t.id AS target_id FROM {frames} f JOIN {} t ON t.id={}",identifier(&tables[target].alias),hex(catalog.catalog().declaration().id())))?;
   }
   for kind in [TypeId::of::<attribution::ProviderRun>(),TypeId::of::<analysis::enriched_execution::AnalysisInvocation>(),TypeId::of::<analysis::source_call::AnalysisInvocation>(),TypeId::of::<analysis::local::AnalysisInvocation>()]{
    if let Some(target)=idx(kind){plan.pairs(frame,target,format!("SELECT f.id AS source_id,t.id AS target_id FROM {frames} f JOIN {} t ON t.input=f.input AND t.context=f.context",identifier(&tables[target].alias)))?;}
   }
   for(root,source)in[(contexts,context_source),(terminals,terminal_source),(exits,exit_source)]{
    let rows=identifier(&tables[root].alias);plan.pairs(root,source,format!("SELECT id AS source_id,id AS target_id FROM {rows}"))?;
    if root==contexts{let enriched=alias(TypeId::of::<analysis::enriched_execution::AnalysisInvocation>()).unwrap();plan.pairs(root,frame,format!("SELECT r.id AS source_id,f.id AS target_id FROM {rows} r JOIN {enriched} p ON p.id=r.invocation JOIN {frames} f ON f.input=p.input AND f.context=p.context"))?;}
    else{let quals=alias(TypeId::of::<assertion::AssertionQualification>()).unwrap();plan.pairs(root,frame,format!("SELECT r.id AS source_id,f.id AS target_id FROM {rows} r JOIN {quals} q ON q.id=r.qualification JOIN {} o ON o.id=r.subject JOIN {} a ON a.id=o.source JOIN {frames} f ON f.context=q.context AND f.input=a.input",alias(TypeId::of::<source::Occurrence>()).unwrap(),alias(TypeId::of::<source::SourceArtifact>()).unwrap()))?;}
   }
   let events=idx(TypeId::of::<normalized::events::NormalizedCallEvent>()).unwrap();
   let event_rows=identifier(&tables[events].alias);
   // The virtual Event root used by CallScopes closes every alternative, then adds exact frame/config.
   let event_root=tables.len()-1;
   plan.pairs(event_root,frame,format!("SELECT e.id AS source_id,f.id AS target_id FROM {event_rows} e JOIN {} o ON o.id=e.site JOIN {} a ON a.id=o.source JOIN {frames} f ON f.input=a.input AND f.context=e.context",alias(TypeId::of::<source::Occurrence>()).unwrap(),alias(TypeId::of::<source::SourceArtifact>()).unwrap()))?;
   if let Some(term)=idx(TypeId::of::<protocols::NativeTerminalObservation>()){
    let term_rows=identifier(&tables[term].alias);let quals=alias(TypeId::of::<assertion::AssertionQualification>()).unwrap();
    plan.pairs(event_root,term,format!("SELECT e.id AS source_id,t.id AS target_id FROM {event_rows} e JOIN {term_rows} t ON t.subject=e.site JOIN {quals} q ON q.id=t.qualification AND q.context=e.context WHERE e.origin={}",hex(calls::CallOrigin::explicit())))?;
   }
   for (compiled,(_,root))in catalog.catalog().models().iter().zip(targets.iter()){
    let rows=identifier(&tables[*root].alias);plan.pairs(*root,frame,format!("SELECT id AS source_id,id AS target_id FROM {rows}"))?;
    let(module,callable)=match &compiled.model().target{models::Target::Stdlib{module,callable,..}|models::Target::Dependency{module,callable,..}|models::Target::Release{module,callable}=>(module,callable)};
    if let(Some(symbols),Some(modules),Some(acquired))=(idx(TypeId::of::<calls::ProviderSymbol>()),alias(TypeId::of::<calls::ProviderModule>()),alias(TypeId::of::<source::Module>())){
     let symbol_rows=identifier(&tables[symbols].alias);
     plan.pairs(*root,symbols,format!("SELECT r.id AS source_id,s.id AS target_id FROM {rows} r JOIN {symbol_rows} s ON s.context=r.context JOIN {modules} pm ON pm.id=s.module LEFT JOIN {acquired} m ON m.id=pm.acquired_module WHERE s.name={} AND (m.qualified_name={} OR pm.bundled_name={})",quote(callable.rsplit('.').next().unwrap_or(callable)),quote(module),quote(module)))?;
    }
    // One actual Python analysis root proves domain applicability; no whole input artifact fanout.
    if let(Some(artifacts),Some(uses))=(idx(TypeId::of::<source::SourceArtifact>()),alias(TypeId::of::<input::ArtifactUse>())){
     let artifact_rows=identifier(&tables[artifacts].alias);
     plan.pairs(*root,artifacts,format!("SELECT r.id AS source_id,p.id AS target_id FROM {rows} r JOIN (SELECT a.input,MIN(a.id) AS id FROM {artifact_rows} a JOIN {uses} u ON u.artifact=a.id AND u.input=a.input WHERE (a.path LIKE '%.py' OR a.path LIKE '%.pyi') AND u.role IN ({},{},{},{}) GROUP BY a.input) p ON p.input=r.input",input::SourceRole::Release.code(),input::SourceRole::Example.code(),input::SourceRole::Test.code(),input::SourceRole::DocBlock.code()))?;
    }
   }
   Ok(())
  }).await?;
        Ok(Self {
            catalog: catalog.catalog().declaration().id(),
            calls,
            frame,
            targets,
            contexts,
            terminals,
            exits,
        })
    }
    pub(super) async fn frame(
        &self,
        run: Id<attribution::ProviderRun>,
        budget: &resources::ResourceBudget,
    ) -> Result<PreparedClosure, ModelError> {
        self.calls
            .edges()
            .grain(self.frame, &format!("id={}", hex(run)), budget)
            .await
    }
    pub(super) async fn selected(
        &self,
        scope: ProductionScope,
        run: Id<attribution::ProviderRun>,
        budget: &resources::ResourceBudget,
    ) -> Result<PreparedClosure, ModelError> {
        let (root, key) = match scope {
            ProductionScope::Target(model) => (
                self.targets
                    .iter()
                    .find(|(id, _)| *id == model)
                    .ok_or(ModelError::Schema(models::AuthoredModel::NAME))?
                    .1,
                hex(run),
            ),
            ProductionScope::Context(id) => (self.contexts, hex(id)),
            ProductionScope::Terminal(id) => (self.terminals, hex(id)),
            ProductionScope::Exit(id) => (self.exits, hex(id)),
            ProductionScope::Frame => return self.frame(run, budget).await,
            ProductionScope::Event(id) => return self.calls.grain(id, budget).await,
            _ => return Err(ModelError::Invalid("Model requires selected root".into())),
        };
        self.calls
            .edges()
            .grain(root, &format!("id={key}"), budget)
            .await
    }
    fn select(
        &self,
        grain: &PreparedClosure,
        table: usize,
        input: &ValidationInput,
    ) -> Result<String, ModelError> {
        let sql = grain.select(table)?;
        // Compact configuration dependencies may reference another catalog. They remain declared
        // inputs, but only the captured selected source language belongs to this model kernel.
        Ok(if input.type_id() == TypeId::of::<models::ModelCatalog>() {
            format!(
                "SELECT * FROM ({sql}) captured_catalog WHERE id={}",
                hex(self.catalog)
            )
        } else {
            sql
        })
    }
    pub(super) async fn load(
        &self,
        access: &CompletedInputs,
        grain: &PreparedClosure,
        budget: &resources::ResourceBudget,
    ) -> Result<ModelData, ModelError> {
        let mut data = ModelData::new(budget);
        macro_rules! read{($($field:ident:$ty:ty,)*)=>{$({for(table,input)in self.calls.inputs().iter().enumerate().filter(|(_,input)|input.type_id()==TypeId::of::<$ty>()){
   let permit=access.read_at::<$ty>(input.prefix())?;crate::consumed_rows::stream_query_at(&permit,input,grain.session(),&self.select(grain,table,input)?,|_,batch|data.visit_input(input,batch)).await?;
  }})*};}
        lctx_model::normalized_binding_inputs!(read);
        lctx_model::normalized_binding_outputs!(read);
        lctx_model::model_pin_inputs!(read);
        lctx_model::execution_evaluation_inputs!(read);
        lctx_model::entry_value_inputs!(read);
        read! {catalogs:models::ModelCatalog,parameters:analysis::MethodParameters,definitions:analysis::AnalysisDefinition,enriched:analysis::enriched_execution::AnalysisInvocation,source_calls:analysis::source_call::AnalysisInvocation,local:analysis::local::AnalysisInvocation,runs:attribution::ProviderRun,members:class_metadata::ClassMemberObservation,metadata:class_metadata::ClassMetadataObservation,origins:calls::CallOrigin,origin_steps:calls::CallOriginStep,terminals:protocols::NativeTerminalObservation,exits:protocols::NativeExitObservation,literals:value::Literal,
        entries:conditions::entry::EntryValueWitness,entry_sources:conditions::entry::EntryAccessSource,evaluations:execution::records::ExpressionEvaluation,sources:execution::records::EvaluationSource,members:execution::records::EvaluationMember,operands:execution::records::EvaluationOperand,base:analysis::base_evaluation::AnalysisInvocation,bodies:execution::body_records::SourceBodyCompletion,body_frames:analysis::base_completion::AnalysisInvocation,headers:execution::source_call_records::SourceCallHeader,header_members:execution::source_call_records::HeaderMember,boundaries:execution::source_call_records::SourceCallBoundary,runs:execution::source_call_records::SourceCallRun,invocations:execution::source_call_records::SourceInvocation,releases:execution::source_call_records::SourceFrameRelease,arguments:execution::source_call_records::SourceFrameArgument,syntax:syntax::ParameterSyntaxObservation,outcomes:execution::source_call_records::SourceCallOutcome,boundaries:execution::source_call_records::InvocationBoundary,outcomes:analysis::source_call::AnalysisOutcome,modeled:execution::modeled_call::ModeledCallEvaluation,modeled_args:execution::modeled_call::ModeledCallArgument,modeled_native:execution::modeled_call::ModeledCallNative,bindings:execution::context_binding::ContextEntryBinding,binding_sources:execution::context_binding::BindingSource,binding_members:execution::context_binding::BindingMember,contexts:execution::context_execution::ContextExecution,items:execution::context_execution::ContextItem,sources:execution::context_execution::ContextSource,members:execution::context_execution::ContextMember,outcomes:execution::enriched_records::ExecutionOutcome,}
        Ok(data)
    }
}

#[cfg(test)]
mod model_scope_controls {
    use super::*;
    use lctx_model::domain::normalized::Rows;
    fn nominal<R>(value: u8) -> Id<R> {
        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
            _,
            serde::de::value::Error,
        >::new([value; 16].into_iter()))
        .unwrap()
    }
    #[tokio::test]
    async fn target_closure_preserves_absence_and_same_context_candidates_without_foreign_payload()
    {
        let model = model().unwrap();
        let session = datafusion::prelude::SessionContext::new();
        let budget = resources::ResourceBudget::fixed(16 << 20).unwrap();
        let catalog = models::Catalog::parse(
            "external.toml",
            include_str!("../../../lctx-model/models/external.toml"),
        )
        .unwrap();
        let parsed = SelectedCatalog::read(catalog.declaration(), &budget).unwrap();
        let inputs = ModelData::inputs();
        let tables: Vec<_> = inputs
            .iter()
            .enumerate()
            .map(|(i, input)| ClosureTable {
                relation: model.relation(input.name()).unwrap().clone(),
                alias: format!("model_scope_{i}"),
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
            rows: &[R],
        ) {
            for (index, _) in inputs
                .iter()
                .enumerate()
                .filter(|(_, input)| input.type_id() == TypeId::of::<R>())
            {
                session.deregister_table(&tables[index].alias).unwrap();
                session
                    .register_batch(&tables[index].alias, R::encode(rows).unwrap())
                    .unwrap();
            }
        }
        let selected =
            source::SourceArtifact::from_bytes(nominal(1), "selected.py".into(), b"x").unwrap();
        let foreign = source::SourceArtifact::from_bytes(
            nominal(1),
            format!("{}.py", "z".repeat(256 << 10)),
            b"x",
        )
        .unwrap();
        let run = attribution::ProviderRun {
            provider: nominal(2),
            context: nominal(3),
            input: selected.input,
            configuration: ContentHash::of(b"run"),
            requested_families: ContentHash::of(b"families"),
        };
        let module = calls::ProviderModule::Bundled {
            provider: run.provider,
            bundle: calls::ModuleBundle::Typeshed,
            name: "typing".into(),
        };
        let symbol = calls::ProviderSymbol {
            provider: run.provider,
            context: run.context,
            module: module.id(),
            native_key: "cast".into(),
            name: "cast".into(),
            kind: calls::SymbolKind::Function,
        };
        let other = calls::ProviderSymbol {
            context: nominal(4),
            native_key: "foreign".repeat(65536),
            ..symbol.clone()
        };
        replace(&session, &inputs, &tables, &[selected.clone(), foreign]);
        replace(
            &session,
            &inputs,
            &tables,
            &[input::ArtifactUse {
                artifact: selected.id(),
                input: selected.input,
                role: input::SourceRole::Release,
            }],
        );
        replace(&session, &inputs, &tables, std::slice::from_ref(&run));
        replace(&session, &inputs, &tables, &[module]);
        replace(&session, &inputs, &tables, &[symbol.clone(), other]);
        let selected_catalog = catalog.declaration();
        let source = format!("{}\n#{}", selected_catalog.source, "unused".repeat(1 << 20));
        let unused_catalog = models::ModelCatalog {
            source_name: "unused.toml".into(),
            content: ContentHash::of(source.as_bytes()),
            source,
            ..selected_catalog.clone()
        };
        let unused_parameters = analysis::MethodParameters {
            depth: None,
            proof_steps: None,
            work: None,
            members: None,
            seed: None,
            iterations: None,
            threshold: None,
            resolution: None,
            damping: None,
            model_catalog: Some(unused_catalog.id()),
        };
        replace(
            &session,
            &inputs,
            &tables,
            &[selected_catalog.clone(), unused_catalog],
        );
        replace(&session, &inputs, &tables, &[unused_parameters]);
        let scopes = ModelScopes::from_tables(inputs.clone(), tables, &session, &parsed, &budget)
            .await
            .unwrap();
        let tiny = resources::ResourceBudget::fixed(96 << 10).unwrap();
        let cast=parsed.catalog().models().iter().find(|compiled|matches!(&compiled.model().target,models::Target::Stdlib{callable,..}if callable=="cast")).unwrap();
        let grain = scopes
            .selected(
                ProductionScope::Target(cast.declaration().id()),
                run.id(),
                &tiny,
            )
            .await
            .unwrap();
        let catalog_table = inputs
            .iter()
            .position(|input| input.type_id() == TypeId::of::<models::ModelCatalog>())
            .unwrap();
        let mut catalogs = Rows::<models::ModelCatalog>::new(&tiny);
        let mut stream = crate::sql::query(
            grain.session(),
            &scopes
                .select(&grain, catalog_table, &inputs[catalog_table])
                .unwrap(),
        )
        .await
        .unwrap()
        .execute_stream()
        .await
        .unwrap();
        while let Some(batch) = stream.try_next().await.unwrap() {
            catalogs.decode(&batch).unwrap();
        }
        assert_eq!(catalogs.len(), 1);
        assert_eq!(catalogs.get(selected_catalog.id()), Some(selected_catalog));
        drop(catalogs);
        drop(stream);
        let mut selected_symbols = Rows::<calls::ProviderSymbol>::new(&tiny);
        let table = inputs
            .iter()
            .position(|input| input.type_id() == TypeId::of::<calls::ProviderSymbol>())
            .unwrap();
        let mut stream = crate::sql::query(grain.session(), &grain.select(table).unwrap())
            .await
            .unwrap()
            .execute_stream()
            .await
            .unwrap();
        use futures::TryStreamExt;
        while let Some(batch) = stream.try_next().await.unwrap() {
            selected_symbols.decode(&batch).unwrap();
        }
        assert_eq!(selected_symbols.len(), 1);
        assert_eq!(selected_symbols.get(symbol.id()), Some(&symbol));
        drop(selected_symbols);
        drop(stream);
        drop(grain);
        let absent=parsed.catalog().models().iter().find(|compiled|matches!(&compiled.model().target,models::Target::Stdlib{callable,..}if callable=="assert_type")).unwrap();
        let grain = scopes
            .selected(
                ProductionScope::Target(absent.declaration().id()),
                run.id(),
                &tiny,
            )
            .await
            .unwrap();
        let mut stream = crate::sql::query(grain.session(), &grain.select(table).unwrap())
            .await
            .unwrap()
            .execute_stream()
            .await
            .unwrap();
        while let Some(batch) = stream.try_next().await.unwrap() {
            assert_eq!(batch.num_rows(), 0);
        }
        let table = inputs
            .iter()
            .position(|input| input.type_id() == TypeId::of::<source::SourceArtifact>())
            .unwrap();
        let mut artifacts = Rows::<source::SourceArtifact>::new(&tiny);
        let mut stream = crate::sql::query(grain.session(), &grain.select(table).unwrap())
            .await
            .unwrap()
            .execute_stream()
            .await
            .unwrap();
        while let Some(batch) = stream.try_next().await.unwrap() {
            artifacts.decode(&batch).unwrap();
        }
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts.get(selected.id()), Some(&selected));
    }
}
