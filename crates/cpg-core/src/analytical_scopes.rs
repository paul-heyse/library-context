//! Named analytical frames reuse compact edges; rich evidence belongs to one frame lifetime.
use crate::{
    consumed_rows::{ClosureTable, PreparedClosure, PreparedEdges},
    workspace::CompletedInputs,
};
use futures::future::BoxFuture;
pub(super) use lctx_model::domain::compiler_scope_program::FrameKind as Kind;
use lctx_model::domain::*;
use std::{any::TypeId, sync::Arc};
pub(super) struct FrameScopes {
    inputs: Vec<ValidationInput>,
    edges: PreparedEdges,
    root: usize,
    _charge: charged::StateCharge,
}
impl FrameScopes {
    pub(super) async fn prepare(
        access: &CompletedInputs,
        session: &datafusion::prelude::SessionContext,
        model: &Arc<ValidatedModel>,
        inputs: Vec<ValidationInput>,
        kind: Kind,
        budget: &resources::ResourceBudget,
    ) -> Result<Self, ModelError> {
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
        Self::bound(inputs, tables, session, kind, budget, model).await
    }
    async fn bound(
        inputs: Vec<ValidationInput>,
        mut tables: Vec<ClosureTable>,
        session: &datafusion::prelude::SessionContext,
        kind: Kind,
        budget: &resources::ResourceBudget,
        model: &ValidatedModel,
    ) -> Result<Self, ModelError> {
        let mut _construction = compiler_scope_program::reserve_construction(
            inputs.len().saturating_add(1),
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
                    .saturating_mul(1),
            )
            .saturating_mul(4);
        let construction_bytes = _construction.size().saturating_add(binding_bytes);
        _construction.try_resize(construction_bytes)?;
        let real = tables.len();
        let owner = match kind {
            Kind::Structural => TypeId::of::<analysis::catalog_core::Invocation>(),
            Kind::Analytic => TypeId::of::<structural::StructuralFrame>(),
        };
        let source = tables
            .iter()
            .position(|table| table.relation.type_id() == owner)
            .ok_or(ModelError::Schema("analytical frame root"))?;
        let root = real;
        tables.push(tables[source].clone());
        let mut declarations = inputs.clone();
        declarations.push(inputs[source].clone());
        let relations = tables
            .iter()
            .map(|table| table.relation.clone())
            .collect::<Vec<_>>();
        let program =
            compiler_scope_program::analytical(declarations, &relations, real, source, kind)?;
        let plan = crate::scope_compilation::lower_compiled(
            crate::scope_compilation::compile(&program, model, budget, None)?,
            &tables,
            &scope_program::ScopeParameters(vec![]),
            budget,
        )?;
        let mut charge = charged::StateCharge::new(budget, "analytical-frame-descriptors");
        charge.grow(
            inputs.capacity() * size_of::<ValidationInput>()
                + tables.capacity() * size_of::<ClosureTable>()
                + tables
                    .iter()
                    .map(|table| table.alias.capacity())
                    .sum::<usize>(),
        )?;
        let edges = plan.prepare(session, budget).await?;
        Ok(Self {
            inputs,
            edges,
            root,
            _charge: charge,
        })
    }
    pub(super) async fn grain<R: Record>(
        &self,
        id: Id<R>,
        budget: &resources::ResourceBudget,
    ) -> Result<PreparedClosure, ModelError> {
        self.edges
            .grain(
                self.root,
                &crate::scoped_admission::root_predicate(&[*id.bytes()]),
                budget,
            )
            .await
    }
    pub(super) fn read<'a, R: Record>(
        &'a self,
        access: &'a CompletedInputs,
        grain: &'a PreparedClosure,
        visit: impl FnMut(&ValidationInput, &arrow_array::RecordBatch) -> Result<(), ModelError>
        + Send
        + 'a,
    ) -> BoxFuture<'a, Result<(), ModelError>> {
        self.read_matching(
            access,
            grain,
            TypeId::of::<R>(),
            read_declaration::<R>,
            Box::new(visit),
        )
    }

    fn read_matching<'a>(
        &'a self,
        access: &'a CompletedInputs,
        grain: &'a PreparedClosure,
        kind: TypeId,
        read: DeclarationReader,
        mut visit: FrameVisitor<'a>,
    ) -> BoxFuture<'a, Result<(), ModelError>> {
        Box::pin(async move {
            for (table, input) in self
                .inputs
                .iter()
                .enumerate()
                .filter(|(_, input)| input.type_id() == kind)
            {
                read(access, input, grain, table, &mut visit).await?;
            }
            Ok(())
        })
    }
}

type FrameVisitor<'a> = Box<
    dyn FnMut(&ValidationInput, &arrow_array::RecordBatch) -> Result<(), ModelError> + Send + 'a,
>;
type DeclarationReader = for<'a, 'visit> fn(
    &'a CompletedInputs,
    &'a ValidationInput,
    &'a PreparedClosure,
    usize,
    &'a mut FrameVisitor<'visit>,
) -> BoxFuture<'a, Result<(), ModelError>>;

fn read_declaration<'a, R: Record>(
    access: &'a CompletedInputs,
    input: &'a ValidationInput,
    grain: &'a PreparedClosure,
    table: usize,
    visit: &'a mut FrameVisitor<'_>,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        let permit = access.read_at::<R>(input.prefix())?;
        crate::consumed_rows::stream_query_at(
            &permit,
            input,
            access,
            grain.session(),
            &grain.select(table)?,
            |_, batch| visit(input, batch),
        )
        .await
    })
}

#[cfg(test)]
mod controls {
    use super::*;
    use crate::consumed_rows::NominalClosure;
    use datafusion::prelude::SessionContext;
    use futures::TryStreamExt;
    use lctx_model::domain::{normalized::entities::*, source::*};
    fn id<R>(n: u8) -> Id<R> {
        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
            _,
            serde::de::value::Error,
        >::new([n; 16].into_iter()))
        .unwrap()
    }
    #[tokio::test]
    async fn structural_frame_keeps_complete_callable_roots_without_unrelated_source_payload() {
        let session = SessionContext::new();
        let budget = resources::ResourceBudget::fixed(4 << 20).unwrap();
        let selected = SourceArtifact::from_bytes(
            id(1),
            "selected.py".into(),
            b"def f(): pass\ndef g(): pass",
        )
        .unwrap();
        let foreign =
            SourceArtifact::from_bytes(id(2), format!("{}.py", "z".repeat(256 << 10)), b"other")
                .unwrap();
        let occurrence = |source, start| Occurrence {
            source,
            start,
            end: start + 1,
            syntax_kind: SyntaxKind::StmtFunctionDef,
            role: OccurrenceRole::Declaration,
            structural_path: vec![start as i32],
        };
        let first = occurrence(selected.id(), 0);
        let second = occurrence(selected.id(), 14);
        let other = occurrence(foreign.id(), 0);
        let unused = Occurrence {
            syntax_kind: SyntaxKind::StmtPass,
            role: OccurrenceRole::Syntax,
            structural_path: vec![4; 65536],
            ..first.clone()
        };
        let callables =
            [first.clone(), second.clone(), other.clone()].map(|row| CallableEntity::Source {
                declaration: row.id(),
                kind: CallableKind::Function,
            });
        let refs = callables
            .iter()
            .map(|row| EntityRef::Callable { callable: row.id() })
            .collect::<Vec<_>>();
        let frame =
            analysis::catalog_core::Invocation::new(selected.input, id(3), id(4), None, []).0;
        let inputs = vec![
            ValidationInput::of::<SourceArtifact>(&["id"]),
            ValidationInput::of::<Occurrence>(&["id"]),
            ValidationInput::of::<CallableEntity>(&["id"]),
            ValidationInput::of::<EntityRef>(&["id"]),
            ValidationInput::of::<analysis::catalog_core::Invocation>(&["id"]),
        ];
        let model = model().unwrap();
        let tables = inputs
            .iter()
            .map(|input| ClosureTable {
                relation: model.relation(input.name()).unwrap().clone(),
                alias: input.name().into(),
            })
            .collect::<Vec<_>>();
        for table in &tables {
            session
                .register_batch(
                    &table.alias,
                    arrow_array::RecordBatch::new_empty(table.relation.schema().clone()),
                )
                .unwrap();
        }
        fn put<R: Record>(session: &SessionContext, rows: &[R]) {
            session.deregister_table(R::NAME).unwrap();
            session
                .register_batch(R::NAME, <R as Record>::encode(rows).unwrap())
                .unwrap();
        }
        put(&session, &[selected, foreign]);
        put(
            &session,
            &[first.clone(), second.clone(), other.clone(), unused.clone()],
        );
        put(&session, &callables);
        put(&session, &refs);
        put(&session, std::slice::from_ref(&frame));
        let prepared = FrameScopes::bound(
            inputs.clone(),
            tables,
            &session,
            Kind::Structural,
            &budget,
            &model,
        )
        .await
        .unwrap();
        let tiny = resources::ResourceBudget::fixed(96 << 10).unwrap();
        let grain = prepared.grain(frame.id(), &tiny).await.unwrap();
        let table = inputs
            .iter()
            .position(|input| input.type_id() == TypeId::of::<Occurrence>())
            .unwrap();
        let mut stream = crate::sql::query(grain.session(), &grain.select(table).unwrap())
            .await
            .unwrap()
            .execute_stream()
            .await
            .unwrap();
        let mut rows = normalized::Rows::<Occurrence>::new(&tiny);
        while let Some(batch) = stream.try_next().await.unwrap() {
            rows.decode(&batch).unwrap();
        }
        assert_eq!(rows.len(), 2);
        assert_eq!(rows.get(first.id()), Some(&first));
        assert_eq!(rows.get(second.id()), Some(&second));
        assert!(rows.get(other.id()).is_none());
        assert!(rows.get(unused.id()).is_none());
        drop(rows);
        drop(stream);
        drop(grain);
        assert_eq!(tiny.reserved(), 0);
        drop(prepared);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn frame_reader_preserves_exact_prefix_order_failure_and_unpolled_charge() {
        use crate::workspace::{Workspace, WorkspaceOptions};
        use stages::{PublicationBoundary, RelationUse};
        use std::sync::atomic::{AtomicBool, Ordering};
        let workspace = Workspace::new(
            Arc::new(model().unwrap()),
            WorkspaceOptions::default(),
            crate::test_native::store(),
        )
        .unwrap();
        let place = |name: &str| {
            let input = input::InputRevision {
                manifest: ContentHash::of(b"frame-reader-prefixes"),
            };
            let source =
                SourceArtifact::from_bytes(input.id(), "frame.py".into(), b"x=1\n").unwrap();
            let module = source::Module {
                source: source.id(),
                qualified_name: "frame".into(),
            };
            value::Place {
                root: value::PlaceRoot::Global {
                    module: module.id(),
                    name: name.into(),
                }
                .id(),
                path: value::AccessPath::empty().id(),
            }
        };
        let first = place("facts");
        let second = place("model");
        for (producer, row, epoch) in [
            (
                "frame-reader-facts",
                first.clone(),
                PublicationBoundary::Facts,
            ),
            (
                "frame-reader-model",
                second.clone(),
                PublicationBoundary::Model,
            ),
        ] {
            let output = workspace.output(
                producer,
                stages::Profile::Catalog,
                ContentHash::of(producer.as_bytes()),
                workspace
                    .inputs(producer, stages::Profile::Catalog, [])
                    .unwrap(),
                [value::Place::NAME],
            );
            output.declare_async::<value::Place>().await.unwrap();
            output.push(row).await.unwrap();
            output
                .finish(stages::ProviderOutcome::Complete)
                .await
                .unwrap();
            workspace.freeze_inputs_async(epoch).await.unwrap();
        }
        let declaration = stages::Stage {
            captured_binding: None,
            name: "frame-reader-control",
            inputs: vec![
                RelationUse::completed::<value::Place>().at_epoch(PublicationBoundary::Facts),
                RelationUse::completed::<value::Place>().at_epoch(PublicationBoundary::Model),
            ],
            outputs: vec![],
            contributes: vec![],
            coverage: vec![],
            profiles: stages::Profile::ALL.to_vec(),
            effect: stages::Effect::Pure,
            code: ContentHash::of(b"frame-reader-control"),
            configuration: ContentHash::of(b"frame-reader-configuration"),
        };
        let access = workspace
            .stage_inputs(&declaration, stages::Profile::Catalog)
            .unwrap();
        let session = access.session(&workspace).await.unwrap();
        let inputs = [PublicationBoundary::Facts, PublicationBoundary::Model]
            .map(|prefix| ValidationInput::of::<value::Place>(&["id"]).at_epoch(prefix))
            .to_vec();
        let tables = inputs
            .iter()
            .map(|input| ClosureTable {
                relation: Relation::of::<value::Place>(),
                alias: access.table_for(input).unwrap(),
            })
            .collect();
        let budget = resources::ResourceBudget::fixed(16 << 20).unwrap();
        let edges = NominalClosure::new(tables)
            .unwrap()
            .prepare(&session, &budget)
            .await
            .unwrap();
        let grain = edges
            .grain_roots(&[(0, "TRUE".into()), (1, "TRUE".into())], &budget)
            .await
            .unwrap();
        let scopes = FrameScopes {
            inputs,
            edges,
            root: 0,
            _charge: charged::StateCharge::new(&budget, "frame-reader-control"),
        };
        let mut observed = Vec::new();
        scopes
            .read::<value::Place>(&access, &grain, |input, batch| {
                for row in <value::Place as Record>::decode(batch)? {
                    observed.push((input.prefix(), row));
                }
                Ok(())
            })
            .await
            .unwrap();
        let facts: Vec<_> = observed
            .iter()
            .filter(|(epoch, _)| *epoch == Some(PublicationBoundary::Facts))
            .map(|(_, row)| row.clone())
            .collect();
        let model: Vec<_> = observed
            .iter()
            .filter(|(epoch, _)| *epoch == Some(PublicationBoundary::Model))
            .map(|(_, row)| row.clone())
            .collect();
        assert_eq!(facts, vec![first.clone()]);
        assert_eq!(model.len(), 2);
        assert!(model.contains(&first) && model.contains(&second));
        assert_eq!(
            observed.first().unwrap().0,
            Some(PublicationBoundary::Facts)
        );
        assert!(
            observed[1..]
                .iter()
                .all(|(epoch, _)| *epoch == Some(PublicationBoundary::Model))
        );
        let mut visited = Vec::new();
        let failure = scopes
            .read::<value::Place>(&access, &grain, |input, _| {
                visited.push(input.prefix());
                Err(ModelError::Conflict("frame visitor refusal"))
            })
            .await;
        assert!(matches!(
            failure,
            Err(ModelError::Conflict("frame visitor refusal"))
        ));
        assert_eq!(visited, vec![Some(PublicationBoundary::Facts)]);
        let before = budget.reserved();
        let charge = budget.reserve("unpolled-frame-visitor", 1024).unwrap();
        let visited = Arc::new(AtomicBool::new(false));
        let seen = visited.clone();
        let pending = scopes.read::<value::Place>(&access, &grain, move |_, _| {
            let _held = &charge;
            seen.store(true, Ordering::Relaxed);
            Ok(())
        });
        assert_eq!(budget.reserved(), before + 1024);
        assert!(!visited.load(Ordering::Relaxed));
        drop(pending);
        assert_eq!(budget.reserved(), before);
        assert!(!visited.load(Ordering::Relaxed));
        drop(grain);
        drop(scopes);
        assert_eq!(budget.reserved(), 0);
        workspace.drain().await.unwrap();
    }
}
