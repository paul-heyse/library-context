//! Summary's finite whole-frame frontier has one physically selected predecessor closure.
//! Referenced artifacts and occurrences are dependencies, never new publication roots.
use crate::{
    consumed_rows::{ClosureTable, PreparedClosure},
    normalize::call_scope::{CallScopeMode, CallScopes},
    workspace::CompletedInputs,
};
use futures::future::BoxFuture;
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
        Self::from_tables(inputs, tables, session, budget, model).await
    }
    async fn from_tables(
        inputs: Vec<ValidationInput>,
        mut tables: Vec<ClosureTable>,
        session: &datafusion::prelude::SessionContext,
        budget: &resources::ResourceBudget,
        model: &ValidatedModel,
    ) -> Result<Self, ModelError> {
        let mut _construction = compiler_scope_program::reserve_construction(
            inputs.len().saturating_add(6),
            tables
                .iter()
                .map(|table| table.relation.fields().len())
                .sum(),
            0,
            budget,
        )?;
        let binding_bytes = tables
            .iter()
            .map(|table| table.alias.capacity())
            .sum::<usize>()
            .saturating_add(
                tables
                    .iter()
                    .map(|table| table.alias.capacity())
                    .max()
                    .unwrap_or(0)
                    .saturating_mul(6),
            )
            .saturating_mul(4);
        let construction_bytes = _construction.size().saturating_add(binding_bytes);
        _construction.try_resize(construction_bytes)?;
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
        let actual_inputs = inputs.clone();
        let calls = CallScopes::from_tables_with(
            inputs,
            tables,
            session,
            budget,
            CallScopeMode {
                binding: true,
                admission: false,
            },
            model,
            |plan, tables| {
                // Virtual ports repeat their exact physical descriptor; aliases are only a binding
                // map here. Scope meaning and epoch selection belong to the model program.
                let mut declarations = actual_inputs.clone();
                for table in &tables[real..] {
                    let original = tables[..real]
                        .iter()
                        .position(|original| original.alias == table.alias)
                        .ok_or(ModelError::Conflict("Summary virtual physical binding"))?;
                    declarations.push(actual_inputs[original].clone());
                }
                let relations = tables
                    .iter()
                    .map(|table| table.relation.clone())
                    .collect::<Vec<_>>();
                let program = compiler_scope_program::summary(
                    declarations,
                    &relations,
                    real,
                    frame,
                    source,
                    &parents,
                    tables.len() - 1,
                )?;
                plan.extend(crate::scope_compilation::lower_compiled(
                    crate::scope_compilation::compile(&program, model, budget, None)?,
                    tables,
                    &scope_program::ScopeParameters(vec![]),
                    budget,
                )?)?;
                // The extended plan retains independently charged compiled and physical owners.
                drop(program);
                drop(relations);
                drop(actual_inputs);
                drop(parents);
                drop(_construction);
                Ok(())
            },
        )
        .await?;
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
    pub(super) fn load<'a>(
        &'a self,
        access: &'a CompletedInputs,
        grain: &'a PreparedClosure,
        budget: &'a resources::ResourceBudget,
    ) -> BoxFuture<'a, Result<SummaryData, ModelError>> {
        Box::pin(async move {
            let mut data = SummaryData::new(budget);
            let mut readers: Vec<Reader> = Vec::new();
            macro_rules! read {($($field:ident:$ty:ty,)*)=>{$(readers.push(reader::<$ty>());)*};}
            lctx_model::normalized_binding_inputs!(read);
            lctx_model::normalized_binding_outputs!(read);
            lctx_model::entry_value_inputs!(read);
            lctx_model::summary_path_inputs!(read);
            lctx_model::summary_owned_inputs!(read);
            lctx_model::summary_vocabulary!(read);
            lctx_model::summary_evidence_inputs!(read);
            for read in readers {
                read(self, access, grain, &mut data).await?;
            }
            Ok(data)
        })
    }
}
// Inventories retain their order and overlap; each adapter visits every matching prefix.
type Reader = for<'a> fn(
    &'a SummaryScopes,
    &'a CompletedInputs,
    &'a PreparedClosure,
    &'a mut SummaryData,
) -> BoxFuture<'a, Result<(), ModelError>>;

fn reader<R: Record>() -> Reader {
    if TypeId::of::<R>() == TypeId::of::<source::Occurrence>() {
        |scopes, access, grain, data| {
            read_record::<source::Occurrence>(
                scopes,
                access,
                grain,
                data,
                |data, _, permit, batch| data.occurrences.visit(permit, batch),
            )
        }
    } else if TypeId::of::<R>() == TypeId::of::<source::SourceArtifact>() {
        |scopes, access, grain, data| {
            read_record::<source::SourceArtifact>(
                scopes,
                access,
                grain,
                data,
                |data, _, permit, batch| data.artifacts.visit(permit, batch),
            )
        }
    } else if TypeId::of::<R>() == TypeId::of::<calls::ProviderSymbol>() {
        |scopes, access, grain, data| {
            read_record::<calls::ProviderSymbol>(
                scopes,
                access,
                grain,
                data,
                |data, _, permit, batch| data.symbols.visit(permit, batch),
            )
        }
    } else {
        |scopes, access, grain, data| {
            read_record::<R>(scopes, access, grain, data, |data, input, _, batch| {
                data.visit_actual(input, batch)
            })
        }
    }
}

fn read_record<'a, R: Record>(
    scopes: &'a SummaryScopes,
    access: &'a CompletedInputs,
    grain: &'a PreparedClosure,
    data: &'a mut SummaryData,
    visit: fn(
        &mut SummaryData,
        &ValidationInput,
        &analysis::sources::CompletedInput<R>,
        &arrow_array::RecordBatch,
    ) -> Result<(), ModelError>,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        for (table, input) in scopes
            .calls
            .inputs()
            .iter()
            .enumerate()
            .filter(|(_, input)| input.type_id() == TypeId::of::<R>())
        {
            let Some(sql) = SummaryScopes::select_actual(grain, table, input)? else {
                continue;
            };
            let permit = access.read_at::<R>(input.prefix())?;
            crate::consumed_rows::stream_query_at(
                &permit,
                input,
                access,
                grain.session(),
                &sql,
                |permit, batch| visit(data, input, permit, batch),
            )
            .await?;
        }
        Ok(())
    })
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
        let scopes = SummaryScopes::from_tables(inputs.clone(), tables, &session, &budget, &model)
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
