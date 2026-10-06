//! C1 selects an artifact's actual evidence roots before decoding rich premises.
//! Root namespaces keep a referenced artifact from becoming another source-wide root.
use crate::{consumed_rows::{ClosureTable, NominalClosure, PreparedEdges, identifier}, workspace::CompletedInputs};
use datafusion::prelude::SessionContext;
use lctx_model::domain::{catalog::evidence::build::EvidenceData, resources::ResourceBudget, source::{Occurrence, SourceArtifact}, *};
use std::any::TypeId;

pub(super) struct EvidenceScopes {
    pub inputs: Vec<ValidationInput>,
    pub edges: PreparedEdges,
    pub root: usize,
}
fn target(inputs: &[ValidationInput], source: usize, kind: TypeId) -> Result<Option<usize>, ModelError> {
    let candidates: Vec<_> = inputs.iter().enumerate().filter(|(_, input)| input.type_id()==kind).map(|(index, _)| index).collect();
    if candidates.len()==1 { return Ok(candidates.first().copied()); }
    if candidates.is_empty() { return Ok(None); }
    let prefix = if kind==TypeId::of::<assertion::AssertionQualification>() {
        Some(if inputs[source].type_id()==TypeId::of::<local_fields::FieldLocation>() { stages::PublicationBoundary::Local } else { stages::PublicationBoundary::Facts })
    } else { inputs[source].prefix() };
    let selected: Vec<_> = candidates.into_iter().filter(|index| inputs[*index].prefix()==prefix).collect();
    if selected.len()!=1 { return Err(ModelError::Conflict("C1 dependency immutable epoch")); }
    Ok(selected.first().copied())
}
fn typed<R: Record>(inputs: &[ValidationInput]) -> Result<usize, ModelError> {
    inputs.iter().position(|input| input.type_id()==TypeId::of::<R>())
        .ok_or(ModelError::Schema("C1 root relation absent"))
}
fn memberships() -> Vec<TypeId> {
    use lctx_model::domain::{catalog::*, normalized::{callables::*, entities::*, links::*, events::*, bindings::*}, documents::*, local_fields::*};
    vec![
        TypeId::of::<CatalogMember>(), TypeId::of::<CatalogCallable>(), TypeId::of::<CatalogClass>(),
        TypeId::of::<EffectiveCallableAssessment>(), TypeId::of::<SignatureVariant>(), TypeId::of::<SignatureSlot>(),
        TypeId::of::<PublicExposure>(), TypeId::of::<PublicExposureCandidate>(), TypeId::of::<SymbolEntityResolution>(),
        TypeId::of::<ReferenceEntityAssessment>(), TypeId::of::<AncestryEntityAssessment>(),
        TypeId::of::<ParameterEntity>(), TypeId::of::<ClassEntity>(), TypeId::of::<FieldEntity>(),
        TypeId::of::<NormalizedCallEvent>(), TypeId::of::<NormalizedCallAlternative>(), TypeId::of::<CallBindingAttempt>(),
        TypeId::of::<DocumentNode>(), TypeId::of::<DocumentObservation>(),
        TypeId::of::<FieldLocation>(), TypeId::of::<symbols::SymbolSequence>(),
        TypeId::of::<normalized::symbolic_fields::SourceFieldClass>(),
        TypeId::of::<normalized::symbolic_fields::SourceFieldReader>(),
    ]
}
impl EvidenceScopes {
    pub async fn prepare(access: &CompletedInputs, model: &ValidatedModel, session: &SessionContext, budget: &ResourceBudget) -> Result<Self, ModelError> {
        let inputs = EvidenceData::inputs();
        let tables: Vec<_> = inputs.iter().map(|input| Ok(ClosureTable {
            relation: model.relation(input.name()).ok_or(ModelError::Schema("C1 input model relation"))?.clone(),
            alias: access.table_for(input)?,
        })).collect::<Result<_, ModelError>>()?;
        Self::prepare_bound(inputs, tables, session, budget).await
    }
    async fn prepare_bound(inputs: Vec<ValidationInput>, tables: Vec<ClosureTable>, session: &SessionContext, budget: &ResourceBudget) -> Result<Self, ModelError> {
        let artifact = typed::<SourceArtifact>(&inputs)?;
        let occurrence = typed::<Occurrence>(&inputs)?;
        let mut bindings = tables.clone();
        let root = bindings.len(); bindings.push(tables[artifact].clone());
        let root_occurrence = bindings.len(); bindings.push(tables[occurrence].clone());
        let mut plan = NominalClosure::new(bindings)?;
        plan.pairs(root, artifact, format!("SELECT id AS source_id,id AS target_id FROM {}", identifier(&tables[artifact].alias)))?;
        plan.pairs(root_occurrence, occurrence, format!("SELECT id AS source_id,id AS target_id FROM {}", identifier(&tables[occurrence].alias)))?;
        plan.pairs(root, root_occurrence, format!("SELECT source AS source_id,id AS target_id FROM {}", identifier(&tables[occurrence].alias)))?;
        let memberships = memberships();
        for (source, table) in tables.iter().enumerate() {
            for field in table.relation.fields() {
                let Some((kind, _)) = field.target() else { continue; };
                let Some(to) = target(&inputs, source, kind)? else { continue; };
                let values = if field.list() { format!("UNNEST({})", identifier(field.name())) } else { identifier(field.name()) };
                plan.pairs(source, to, format!("SELECT id AS source_id,{values} AS target_id FROM {}", identifier(&table.alias)))?;
                if field.list() { continue; }
                // All locations in the selected artifact are roots. Forward references use the
                // ordinary namespace and cannot pull every fact from a dependency's artifact.
                let selected_root = if kind==TypeId::of::<SourceArtifact>() { Some(root) }
                    else if kind==TypeId::of::<Occurrence>() { Some(root_occurrence) } else { None };
                if let Some(selected_root) = selected_root {
                    plan.pairs(selected_root, source, format!("SELECT {} AS source_id,id AS target_id FROM {}", identifier(field.name()), identifier(&table.alias)))?;
                }
                if memberships.contains(&kind) {
                    plan.pairs(to, source, format!("SELECT {} AS source_id,id AS target_id FROM {}", identifier(field.name()), identifier(&table.alias)))?;
                }
            }
        }
        // A document and its materialized fences form one original-source grain.
        let derived = typed::<input::DerivedArtifact>(&inputs)?;
        plan.pairs(root, root, format!("SELECT pythoncodeblock_document AS source_id,pythoncodeblock_artifact AS target_id FROM {} WHERE pythoncodeblock_document IS NOT NULL", identifier(&tables[derived].alias)))?;
        // Root public slots use the actual module ownership, never every module on one input.
        let modules = typed::<source::Module>(&inputs)?;
        let members = typed::<catalog::CatalogMember>(&inputs)?;
        plan.pairs(root, members, format!("SELECT m.source AS source_id,c.id AS target_id FROM {} m JOIN {} c ON c.access=m.id", identifier(&tables[modules].alias), identifier(&tables[members].alias)))?;
        // Fixed C0 parent membership and native support pairs are actual inverse memberships.
        let owners = [catalog::CatalogMemberInvocation::NAME,
            diagnostics::RuffDiagnosticSupport::NAME, diagnostics::PyreflyDiagnosticSupport::NAME,
            diagnostics::NativeParameterDefinitionSupport::NAME, calls::ProviderCallSiteSupport::NAME,
            analysis::native::NativeAssertionPremise::NAME];
        for (source, table) in tables.iter().enumerate().filter(|(_, table)| owners.contains(&table.relation.name())) {
            for field in table.relation.fields() {
                let Some((kind, _)) = field.target() else { continue; };
                if field.list() { continue; }
                if table.relation.name()==analysis::native::NativeAssertionPremise::NAME || field.name()=="assertion" || field.name()=="member" {
                    if let Some(to)=target(&inputs, source, kind)? {
                        plan.pairs(to, source, format!("SELECT {} AS source_id,id AS target_id FROM {}", identifier(field.name()), identifier(&table.alias)))?;
                    }
                }
            }
        }
        let coverage = typed::<attribution::ProviderCoverage>(&inputs)?;
        let coverage_scope = typed::<source::CoverageScope>(&inputs)?;
        plan.own(coverage, "scope", coverage_scope)?;
        let nodes = typed::<documents::DocumentNode>(&inputs)?;
        for field in tables[nodes].relation.fields().iter().filter(|field| field.target().is_some_and(|(kind,_)| kind==TypeId::of::<assertion::Evidence>())) {
            let owner = target(&inputs,nodes,TypeId::of::<assertion::Evidence>())?.ok_or(ModelError::Schema("C1 document span owner"))?;
            plan.own(nodes,field.name(),owner)?;
        }
        for (source, table) in tables.iter().enumerate() {
            let owner_field = if table.relation.type_id()==TypeId::of::<syntax::DeclarationObservation>() { Some("declaration") }
                else if table.relation.type_id()==TypeId::of::<syntax::ParameterSyntaxObservation>() { Some("function") } else { None };
            if let Some(field)=owner_field { plan.own(source,field,occurrence)?; }
        }
        let edges = plan.prepare(session, budget).await?;
        Ok(Self { inputs, edges, root })
    }
}

#[cfg(test)]
mod controls {
    use super::*;
    use datafusion::datasource::MemTable;
    use futures::TryStreamExt;
    use std::sync::Arc;
    use lctx_model::domain::{attribution::*, input::*, source::*};
    fn nominal<T>(byte:u8)->Id<T> {
        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<_,serde::de::value::Error>::new([byte;16].into_iter())).unwrap()
    }
    fn install<R:Record>(session:&SessionContext, tables:&[ClosureTable], inputs:&[ValidationInput], rows:&[R]) {
        let index=typed::<R>(inputs).unwrap();
        let batch=R::encode(rows).unwrap();
        session.deregister_table(tables[index].alias.as_str()).unwrap();
        session.register_table(tables[index].alias.as_str(),Arc::new(MemTable::try_new(batch.schema(),vec![vec![batch]]).unwrap())).unwrap();
    }
    fn fixture()->(SessionContext,Vec<ValidationInput>,Vec<ClosureTable>) {
        let model=model().unwrap(); let inputs=EvidenceData::inputs();let session=SessionContext::new();
        let tables=inputs.iter().enumerate().map(|(index,input)| {
            let relation=model.relation(input.name()).unwrap().clone();
            let alias=format!("evidence_fixture_{index}");
            let batch=arrow_array::RecordBatch::new_empty(relation.schema().clone());
            session.register_table(alias.as_str(),Arc::new(MemTable::try_new(batch.schema(),vec![vec![batch]]).unwrap())).unwrap();
            ClosureTable {relation,alias}
        }).collect(); (session,inputs,tables)
    }
    async fn load(scope:&crate::consumed_rows::PreparedClosure, inputs:&[ValidationInput], budget:&ResourceBudget)->EvidenceData {
        let mut data=EvidenceData::new(budget);
        for (index,input) in inputs.iter().enumerate() {
            let mut rows=crate::sql::query(scope.session(),&scope.select(index).unwrap()).await.unwrap().execute_stream().await.unwrap();
            while let Some(batch)=rows.try_next().await.unwrap() {data.visit_input(input,&batch).unwrap();}
        }
        data
    }
    fn predicate<R:Record>(id:Id<R>)->String {format!("id=X'{}'",id.bytes().iter().map(|byte|format!("{byte:02x}")).collect::<String>())}
    #[tokio::test]
    async fn artifact_grain_union_preserves_the_independent_scenario_oracle() {
        let budget=ResourceBudget::fixed(8<<20).unwrap();let (session,inputs,tables)=fixture();
        let a=SourceArtifact::from_bytes(nominal(1),"a.py".into(),b"x").unwrap();
        let b=SourceArtifact::from_bytes(nominal(1),"b.py".into(),b"y").unwrap();
        let artifacts=vec![a.clone(),b.clone()];
        let scopes:Vec<_>=artifacts.iter().map(|artifact|CoverageScope::Artifact {artifact:artifact.id()}).collect();
        let uses:Vec<_>=artifacts.iter().map(|artifact|ArtifactUse {artifact:artifact.id(),input:artifact.input,role:SourceRole::Example}).collect();
        let coverage:Vec<_>=scopes.iter().map(|scope|ProviderCoverage {scope:scope.id(),provider:Some(nominal(3)),context:nominal(2),family:FactFamily::Syntax,run:Some(nominal(4)),status:CoverageStatus::Failed,reason:Some(obligation::ObligationKind::SyntaxError),diagnostic:None}).collect();
        install(&session,&tables,&inputs,&artifacts);install(&session,&tables,&inputs,&scopes);
        install(&session,&tables,&inputs,&uses);install(&session,&tables,&inputs,&coverage);
        let mut all=EvidenceData::new(&budget);
        for row in &artifacts {all.core.artifacts.insert(row.clone()).unwrap();}
        for row in &uses {all.facts.uses.insert(row.clone()).unwrap();}
        for row in &coverage {all.core.native_coverage.insert(row.clone()).unwrap();}
        let expected=lctx_model::domain::catalog::evidence::build::build(&all,&budget).unwrap();drop(all);
        let prepared=EvidenceScopes::prepare_bound(inputs.clone(),tables,&session,&budget).await.unwrap();
        let mut actual=lctx_model::domain::catalog::evidence::build::EvidenceOutput::new(&budget);
        for artifact in &artifacts {
            let scope=prepared.edges.grain(prepared.root,&predicate(artifact.id()),&budget).await.unwrap();
            let data=load(&scope,&inputs,&budget).await;assert_eq!(data.core.artifacts.len(),1);assert_eq!(data.facts.uses.len(),1);
            let rows=lctx_model::domain::catalog::evidence::build::build(&data,&budget).unwrap();
            macro_rules! merge {($($field:ident:$ty:ty,)*)=>{$(for row in rows.$field.iter() {actual.$field.insert(row.clone()).unwrap();})*};}
            lctx_model::catalog_evidence_outputs!(merge);
        }
        actual.matches(&expected).unwrap();assert_eq!(actual.scenarios.len(),2);
        drop(actual);drop(expected);drop(prepared);assert_eq!(budget.reserved(),0);
    }
    #[tokio::test]
    async fn materialized_fence_is_in_the_original_grain_and_other_rich_labels_are_excluded() {
        let budget=ResourceBudget::fixed(8<<20).unwrap();let (session,inputs,tables)=fixture();
        let doc=SourceArtifact::from_bytes(nominal(1),"guide.md".into(),b"fence").unwrap();
        let fence=SourceArtifact::from_bytes(nominal(1),"_lctx/fence.py".into(),b"x").unwrap();
        let other=SourceArtifact::from_bytes(nominal(1),format!("{}.py","unrelated".repeat(100_000)),b"z").unwrap();
        install(&session,&tables,&inputs,&[doc.clone(),fence.clone(),other.clone()]);
        install(&session,&tables,&inputs,&[DerivedArtifact::PythonCodeBlock {artifact:fence.id(),document:doc.id(),ordinal:0,fence_start:0,fence_end:5}]);
        let occurrence=|artifact:&SourceArtifact|Occurrence {source:artifact.id(),start:0,end:1,syntax_kind:SyntaxKind::ExprName,role:OccurrenceRole::Syntax,structural_path:vec![0]};
        install(&session,&tables,&inputs,&[occurrence(&fence),occurrence(&other)]);
        let prepared=EvidenceScopes::prepare_bound(inputs.clone(),tables,&session,&budget).await.unwrap();
        let scope=prepared.edges.grain(prepared.root,&predicate(doc.id()),&budget).await.unwrap();
        let data=load(&scope,&inputs,&budget).await;
        assert_eq!(data.core.artifacts.len(),2);assert!(data.core.artifacts.get(other.id()).is_none());
        assert_eq!(data.core.occurrences.len(),1);assert_eq!(data.core.occurrences.iter().next().unwrap().source,fence.id());
        drop(data);drop(scope);drop(prepared);assert_eq!(budget.reserved(),0);
    }
}
