//! C1 selects an artifact's actual evidence roots before decoding rich premises.
//! Root namespaces keep a referenced artifact from becoming another source-wide root.
use crate::{
    consumed_rows::{ClosureTable, PreparedEdges},
    workspace::CompletedInputs,
};
use datafusion::prelude::SessionContext;
use lctx_model::domain::{
    catalog::evidence::build::EvidenceData,
    resources::ResourceBudget,
    source::{Occurrence, SourceArtifact},
    *,
};
use std::any::TypeId;

pub(super) struct EvidenceScopes {
    pub inputs: Vec<ValidationInput>,
    pub edges: PreparedEdges,
    pub root: usize,
    _charge: charged::StateCharge,
}
fn typed<R: Record>(inputs: &[ValidationInput]) -> Result<usize, ModelError> {
    inputs
        .iter()
        .position(|input| input.type_id() == TypeId::of::<R>())
        .ok_or(ModelError::Schema("C1 root relation absent"))
}
impl EvidenceScopes {
    pub async fn prepare(
        access: &CompletedInputs,
        model: &ValidatedModel,
        session: &SessionContext,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        let inputs = EvidenceData::inputs();
        let tables: Vec<_> = inputs
            .iter()
            .map(|input| {
                Ok(ClosureTable {
                    relation: model
                        .relation(input.name())
                        .ok_or(ModelError::Schema("C1 input model relation"))?
                        .clone(),
                    alias: access.table_for(input)?,
                })
            })
            .collect::<Result<_, ModelError>>()?;
        Self::prepare_bound(inputs, tables, session, budget, model).await
    }
    async fn prepare_bound(
        inputs: Vec<ValidationInput>,
        tables: Vec<ClosureTable>,
        session: &SessionContext,
        budget: &ResourceBudget,
        model: &ValidatedModel,
    ) -> Result<Self, ModelError> {
        let mut charge = charged::StateCharge::new(budget, "C1-scope-descriptors");
        charge.grow(inputs.capacity() * 512)?;
        let artifact = typed::<SourceArtifact>(&inputs)?;
        let occurrence = typed::<Occurrence>(&inputs)?;
        let root = tables.len();
        let mut bindings = tables.clone();
        bindings.extend([tables[artifact].clone(), tables[occurrence].clone()]);
        let relations = tables
            .iter()
            .map(|table| table.relation.clone())
            .collect::<Vec<_>>();
        let program = catalog_scope_program::evidence(inputs.clone(), &relations, budget)?;
        let plan = crate::scope_compilation::lower_compiled(
            crate::scope_compilation::compile(program.program(), model, budget, None)?,
            &bindings,
            &scope_program::ScopeParameters(vec![]),
            budget,
        )?;
        let edges = plan.prepare(session, budget).await?;
        Ok(Self {
            inputs,
            edges,
            root,
            _charge: charge,
        })
    }
}

#[cfg(test)]
mod controls {
    use super::*;
    use datafusion::datasource::MemTable;
    use futures::TryStreamExt;
    use lctx_model::domain::{attribution::*, input::*, source::*};
    use std::sync::Arc;
    fn nominal<T>(byte: u8) -> Id<T> {
        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
            _,
            serde::de::value::Error,
        >::new([byte; 16].into_iter()))
        .unwrap()
    }
    fn install<R: Record>(
        session: &SessionContext,
        tables: &[ClosureTable],
        inputs: &[ValidationInput],
        rows: &[R],
    ) {
        let index = typed::<R>(inputs).unwrap();
        let batch = R::encode(rows).unwrap();
        session
            .deregister_table(tables[index].alias.as_str())
            .unwrap();
        session
            .register_table(
                tables[index].alias.as_str(),
                Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap()),
            )
            .unwrap();
    }
    fn fixture() -> (SessionContext, Vec<ValidationInput>, Vec<ClosureTable>) {
        let model = model().unwrap();
        let inputs = EvidenceData::inputs();
        let session = SessionContext::new();
        let tables = inputs
            .iter()
            .enumerate()
            .map(|(index, input)| {
                let relation = model.relation(input.name()).unwrap().clone();
                let alias = format!("evidence_fixture_{index}");
                let batch = arrow_array::RecordBatch::new_empty(relation.schema().clone());
                session
                    .register_table(
                        alias.as_str(),
                        Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap()),
                    )
                    .unwrap();
                ClosureTable { relation, alias }
            })
            .collect();
        (session, inputs, tables)
    }
    async fn load(
        scope: &crate::consumed_rows::PreparedClosure,
        inputs: &[ValidationInput],
        budget: &ResourceBudget,
    ) -> EvidenceData {
        let mut data = EvidenceData::new(budget);
        for (index, input) in inputs.iter().enumerate() {
            let mut rows = crate::sql::query(scope.session(), &scope.select(index).unwrap())
                .await
                .unwrap()
                .execute_stream()
                .await
                .unwrap();
            while let Some(batch) = rows.try_next().await.unwrap() {
                data.visit_input(input, &batch).unwrap();
            }
        }
        data
    }
    fn predicate<R: Record>(id: Id<R>) -> String {
        format!(
            "id=X'{}'",
            id.bytes()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        )
    }
    #[tokio::test]
    async fn artifact_batch_decodes_overlapping_union_once_and_preserves_independent_scenario_oracle()
     {
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let (session, inputs, tables) = fixture();
        let a = SourceArtifact::from_bytes(nominal(1), "a.py".into(), b"x").unwrap();
        let b = SourceArtifact::from_bytes(nominal(1), "b.py".into(), b"y").unwrap();
        let artifacts = vec![a.clone(), b.clone()];
        let scopes: Vec<_> = artifacts
            .iter()
            .map(|artifact| CoverageScope::Artifact {
                artifact: artifact.id(),
            })
            .collect();
        let uses: Vec<_> = artifacts
            .iter()
            .map(|artifact| ArtifactUse {
                artifact: artifact.id(),
                input: artifact.input,
                role: SourceRole::Example,
            })
            .collect();
        let coverage: Vec<_> = scopes
            .iter()
            .map(|scope| ProviderCoverage {
                scope: scope.id(),
                provider: Some(nominal(3)),
                context: nominal(2),
                family: FactFamily::Syntax,
                run: Some(nominal(4)),
                status: CoverageStatus::Failed,
                reason: Some(obligation::ObligationKind::SyntaxError),
                diagnostic: None,
            })
            .collect();
        install(&session, &tables, &inputs, &artifacts);
        install(&session, &tables, &inputs, &scopes);
        install(&session, &tables, &inputs, &uses);
        install(&session, &tables, &inputs, &coverage);
        let mut all = EvidenceData::new(&budget);
        for row in &artifacts {
            all.core.artifacts.insert(row.clone()).unwrap();
        }
        for row in &uses {
            all.facts.uses.insert(row.clone()).unwrap();
        }
        for row in &coverage {
            all.core.native_coverage.insert(row.clone()).unwrap();
        }
        let expected = lctx_model::domain::catalog::evidence::build::build(&all, &budget).unwrap();
        drop(all);
        let prepared = EvidenceScopes::prepare_bound(
            inputs.clone(),
            tables,
            &session,
            &budget,
            &model().unwrap(),
        )
        .await
        .unwrap();
        let roots = [a.id(), b.id(), a.id()].map(|id| crate::consumed_rows::PreparedRoot {
            table: prepared.root,
            key: *id.bytes(),
            kind: crate::consumed_rows::PreparedRootKind::Virtual,
        });
        let selected = prepared.edges.batch(&roots, &budget).await.unwrap();
        let mut union = EvidenceData::new(&budget);
        let mut decoded_artifacts = 0;
        crate::scoped_batch::hydrate_union(
            &selected,
            &inputs,
            &budget,
            &crate::workspace::Cancellation::default(),
            &mut |_, input, batch| {
                if input.name() == SourceArtifact::NAME {
                    decoded_artifacts += batch.num_rows();
                }
                union.visit_input(input, batch).map(|_| ())
            },
        )
        .await
        .unwrap();
        assert_eq!(decoded_artifacts, 2);
        let mut actual = lctx_model::domain::catalog::evidence::build::EvidenceOutput::new(&budget);
        for partition in 0..roots.len() {
            let data = union
                .selected_copy(
                    &inputs,
                    &mut |table, key| selected.contains(partition, table, key),
                    &budget,
                )
                .unwrap();
            assert_eq!(data.core.artifacts.len(), 1);
            assert_eq!(data.facts.uses.len(), 1);
            let rows = lctx_model::domain::catalog::evidence::build::build(&data, &budget).unwrap();
            macro_rules! merge {($($field:ident:$ty:ty,)*)=>{$(for row in rows.$field.iter() {actual.$field.insert(row.clone()).unwrap();})*};}
            lctx_model::catalog_evidence_outputs!(merge);
        }
        actual.matches(&expected).unwrap();
        assert_eq!(actual.scenarios.len(), 2);
        drop(actual);
        drop(expected);
        drop(union);
        drop(selected);
        drop(prepared);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn materialized_fence_is_in_the_original_grain_and_other_rich_labels_are_excluded() {
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let (session, inputs, tables) = fixture();
        let doc = SourceArtifact::from_bytes(nominal(1), "guide.md".into(), b"fence").unwrap();
        let fence = SourceArtifact::from_bytes(nominal(1), "_lctx/fence.py".into(), b"x").unwrap();
        let other = SourceArtifact::from_bytes(
            nominal(1),
            format!("{}.py", "unrelated".repeat(100_000)),
            b"z",
        )
        .unwrap();
        install(
            &session,
            &tables,
            &inputs,
            &[doc.clone(), fence.clone(), other.clone()],
        );
        install(
            &session,
            &tables,
            &inputs,
            &[DerivedArtifact::PythonCodeBlock {
                artifact: fence.id(),
                document: doc.id(),
                ordinal: 0,
                fence_start: 0,
                fence_end: 5,
            }],
        );
        let occurrence = |artifact: &SourceArtifact| Occurrence {
            source: artifact.id(),
            start: 0,
            end: 1,
            syntax_kind: SyntaxKind::ExprName,
            role: OccurrenceRole::Syntax,
            structural_path: vec![0],
        };
        install(
            &session,
            &tables,
            &inputs,
            &[occurrence(&fence), occurrence(&other)],
        );
        let prepared = EvidenceScopes::prepare_bound(
            inputs.clone(),
            tables,
            &session,
            &budget,
            &model().unwrap(),
        )
        .await
        .unwrap();
        let scope = prepared
            .edges
            .grain(prepared.root, &predicate(doc.id()), &budget)
            .await
            .unwrap();
        let data = load(&scope, &inputs, &budget).await;
        assert_eq!(data.core.artifacts.len(), 2);
        assert!(data.core.artifacts.get(other.id()).is_none());
        assert_eq!(data.core.occurrences.len(), 1);
        assert_eq!(
            data.core.occurrences.iter().next().unwrap().source,
            fence.id()
        );
        drop(data);
        drop(scope);
        drop(prepared);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn actual_source_reads_and_bindings_preserve_setup_without_opening_shared_scope_neighbors()
     {
        use lctx_model::domain::{
            assertion::{Approximation, AssertionQualification},
            catalog::evidence::{SetupDependency, build},
            lexical::*,
            normalized::{
                entities::ResolutionStatus,
                links::{LinkReason, ReferenceEntityAssessment},
            },
        };
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let (session, inputs, tables) = fixture();
        let source = SourceArtifact::from_bytes(
            nominal(1),
            "example.py".into(),
            b"missing()
",
        )
        .unwrap();
        let other = SourceArtifact::from_bytes(
            nominal(1),
            "other.py".into(),
            b"neighbor
",
        )
        .unwrap();
        let occurrences: Vec<_> = [&source, &other]
            .into_iter()
            .map(|source| Occurrence {
                source: source.id(),
                start: 0,
                end: 7,
                syntax_kind: SyntaxKind::ExprName,
                role: OccurrenceRole::Syntax,
                structural_path: vec![0],
            })
            .collect();
        let lexical = LexicalScope {
            owner: occurrences[0].id(),
            kind: LexicalScopeKind::Module,
        };
        let qualifications: Vec<_> = [&source, &other]
            .into_iter()
            .map(|source| AssertionQualification {
                assumptions: assumptions::AssumptionSet::empty_id(),
                context: nominal(2),
                scope: CoverageScope::Artifact {
                    artifact: source.id(),
                }
                .id(),
                condition: conditions::Diagram::always().id(),
                modality: Modality::Definite,
                approximation: Approximation::Exact,
            })
            .collect();
        let references: Vec<_> = occurrences
            .iter()
            .zip(&qualifications)
            .map(|(site, q)| ReferenceObservation {
                qualification: q.id(),
                read: site.id(),
                scope: lexical.id(),
                parent: site.id(),
                field: SyntaxField::Callee,
                name: "missing".into(),
            })
            .collect();
        let assessments: Vec<_> = references
            .iter()
            .map(|reference| ReferenceEntityAssessment {
                reference: reference.id(),
                status: ResolutionStatus::Unresolved,
                reason: LinkReason::MissingResolution,
            })
            .collect();
        let events: Vec<_> = occurrences
            .iter()
            .map(|site| BindingEvent {
                site: site.id(),
                name: "bound".into(),
            })
            .collect();
        let bindings: Vec<_> = events
            .iter()
            .zip(&qualifications)
            .map(|(event, q)| BindingObservation {
                qualification: q.id(),
                event: event.id(),
                scope: lexical.id(),
                kind: BindingEventKind::Assignment,
                ordinal: 0,
                value: None,
                static_branch: None,
                static_polarity: None,
            })
            .collect();
        let usage = ArtifactUse {
            artifact: source.id(),
            input: source.input,
            role: SourceRole::Example,
        };
        let coverage_scope = CoverageScope::Artifact {
            artifact: source.id(),
        };
        let coverage = ProviderCoverage {
            scope: coverage_scope.id(),
            provider: Some(nominal(3)),
            context: nominal(2),
            family: FactFamily::Syntax,
            run: Some(nominal(4)),
            status: CoverageStatus::Failed,
            reason: Some(obligation::ObligationKind::SyntaxError),
            diagnostic: None,
        };
        install(&session, &tables, &inputs, &[source.clone(), other]);
        install(&session, &tables, &inputs, &occurrences);
        install(&session, &tables, &inputs, std::slice::from_ref(&lexical));
        install(&session, &tables, &inputs, &qualifications);
        install(&session, &tables, &inputs, &references);
        install(&session, &tables, &inputs, &assessments);
        install(&session, &tables, &inputs, &events);
        install(&session, &tables, &inputs, &bindings);
        install(&session, &tables, &inputs, std::slice::from_ref(&usage));
        install(&session, &tables, &inputs, &[coverage_scope]);
        install(&session, &tables, &inputs, std::slice::from_ref(&coverage));
        let prepared = EvidenceScopes::prepare_bound(
            inputs.clone(),
            tables,
            &session,
            &budget,
            &model().unwrap(),
        )
        .await
        .unwrap();
        let scope = prepared
            .edges
            .grain(prepared.root, &predicate(source.id()), &budget)
            .await
            .unwrap();
        let actual = load(&scope, &inputs, &budget).await;
        assert_eq!(actual.core.reference_assessments.len(), 1);
        assert_eq!(
            actual
                .core
                .reference_assessments
                .iter()
                .next()
                .unwrap()
                .id(),
            assessments[0].id()
        );
        assert_eq!(actual.core.bindings.len(), 1);
        assert_eq!(
            actual.core.bindings.iter().next().unwrap().id(),
            bindings[0].id()
        );
        let mut expected = EvidenceData::new(&budget);
        expected.core.artifacts.insert(source).unwrap();
        expected.facts.uses.insert(usage).unwrap();
        expected.core.native_coverage.insert(coverage).unwrap();
        expected.core.lexical_scopes.insert(lexical).unwrap();
        expected
            .core
            .occurrences
            .insert(occurrences[0].clone())
            .unwrap();
        expected
            .core
            .qualifications
            .insert(qualifications[0].clone())
            .unwrap();
        expected
            .core
            .references
            .insert(references[0].clone())
            .unwrap();
        expected
            .core
            .reference_assessments
            .insert(assessments[0].clone())
            .unwrap();
        expected
            .core
            .binding_events
            .insert(events[0].clone())
            .unwrap();
        expected.core.bindings.insert(bindings[0].clone()).unwrap();
        let oracle = build::build(&expected, &budget).unwrap();
        let rows = build::build(&actual, &budget).unwrap();
        rows.matches(&oracle).unwrap();
        assert!(rows.setup.iter().any(|row| *row
            == SetupDependency::UnresolvedReference {
                assessment: assessments[0].id()
            }));
        drop(rows);
        drop(oracle);
        drop(actual);
        drop(expected);
        drop(scope);
        drop(prepared);
        assert_eq!(budget.reserved(), 0);
    }
}
