//! C0 public slots retain their complete alias, class-path and constructor dependencies.
//! A referenced module or occurrence does not become a new public-slot root.
use crate::{
    consumed_rows::{ClosureTable, PreparedEdges},
    workspace::CompletedInputs,
};
use datafusion::prelude::SessionContext;
use lctx_model::domain::{catalog::build::CatalogData, resources::ResourceBudget, *};
use std::any::TypeId;

pub(super) struct CatalogScopes {
    pub inputs: Vec<ValidationInput>,
    pub edges: PreparedEdges,
    pub root: usize,
}
fn typed<R: Record>(inputs: &[ValidationInput]) -> Result<usize, ModelError> {
    inputs
        .iter()
        .position(|input| input.type_id() == TypeId::of::<R>())
        .ok_or(ModelError::Schema("C0 scoped relation absent"))
}

impl CatalogScopes {
    pub async fn prepare(
        access: &CompletedInputs,
        model: &ValidatedModel,
        session: &SessionContext,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        let inputs = CatalogData::validation_inputs();
        let tables = inputs
            .iter()
            .map(|input| {
                Ok(ClosureTable {
                    relation: model
                        .relation(input.name())
                        .ok_or(ModelError::Schema("C0 input declaration"))?
                        .clone(),
                    alias: access.table_for(input)?,
                })
            })
            .collect::<Result<Vec<_>, ModelError>>()?;
        Self::prepare_bound(inputs, tables, model, session, budget).await
    }
    async fn prepare_bound(
        inputs: Vec<ValidationInput>,
        tables: Vec<ClosureTable>,
        model: &ValidatedModel,
        session: &SessionContext,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        let names = typed::<symbols::PublicNameObservation>(&inputs)?;
        let root = tables.len();
        let mut bindings = tables.clone();
        bindings.push(tables[names].clone());
        let relations = tables
            .iter()
            .map(|table| table.relation.clone())
            .collect::<Vec<_>>();
        let program = catalog_scope_program::core(inputs.clone(), &relations, model, budget)?;
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
        })
    }
}

#[cfg(test)]
mod controls {
    use super::*;
    use datafusion::datasource::MemTable;
    use futures::TryStreamExt;
    use lctx_model::domain::{
        assertion::*, attribution::*, normalized::entities::*, source::*, symbols::*,
    };
    use std::sync::Arc;
    fn nominal<R>(byte: u8) -> Id<R> {
        serde_json::from_value(serde_json::to_value([byte; 16]).unwrap()).unwrap()
    }
    fn fixture() -> (SessionContext, Vec<ValidationInput>, Vec<ClosureTable>) {
        let model = model().unwrap();
        let inputs = CatalogData::validation_inputs();
        let session = SessionContext::new();
        let tables = inputs
            .iter()
            .enumerate()
            .map(|(index, input)| {
                let relation = model.relation(input.name()).unwrap().clone();
                let alias = format!("catalog_fixture_{index}");
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
    async fn load(
        scope: &crate::consumed_rows::PreparedClosure,
        inputs: &[ValidationInput],
        budget: &ResourceBudget,
    ) -> CatalogData {
        let mut data = CatalogData::new(budget);
        for (index, input) in inputs.iter().enumerate() {
            let mut rows = crate::sql::query(scope.session(), &scope.select(index).unwrap())
                .await
                .unwrap()
                .execute_stream()
                .await
                .unwrap();
            while let Some(batch) = rows.try_next().await.unwrap() {
                data.visit(input.name(), &batch).unwrap();
            }
        }
        data
    }
    #[tokio::test]
    async fn public_slot_union_matches_whole_builder_and_excludes_unrelated_rich_labels() {
        let (session, inputs, tables) = fixture();
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let artifact = SourceArtifact::from_bytes(nominal(1), "api.py".into(), b"x").unwrap();
        let unrelated =
            SourceArtifact::from_bytes(nominal(1), "unrelated".repeat(130000), b"x").unwrap();
        let module = Module {
            source: artifact.id(),
            qualified_name: "api".into(),
        };
        let origin = ExportOrigin::Untraced;
        let q = AssertionQualification {
            context: nominal(2),
            scope: nominal(3),
            condition: conditions::Diagram::always().id(),
            modality: Modality::Definite,
            approximation: Approximation::Exact,
            assumptions: assumptions::AssumptionSet::empty_id(),
        };
        let names: Vec<_> = ["first", "second"]
            .into_iter()
            .map(|name| PublicNameObservation {
                qualification: q.id(),
                access: module.id(),
                name: name.into(),
                via_dunder_all: false,
                origin: origin.id(),
            })
            .collect();
        let access = module.id();
        let origin_id = origin.id();
        let exposures: Vec<_> = names
            .iter()
            .flat_map(|name| {
                [nominal(2), nominal(4)]
                    .into_iter()
                    .map(move |context| PublicExposure {
                        access,
                        context,
                        observation: name.id(),
                        origin: origin_id,
                        enumeration: None,
                        publicity: PublicPathKnowledge::Unknown,
                        status: ResolutionStatus::Unresolved,
                        reason: EntityReason::MissingDeclaration,
                    })
            })
            .collect();
        install(&session, &tables, &inputs, &[artifact.clone(), unrelated]);
        install(&session, &tables, &inputs, std::slice::from_ref(&module));
        install(&session, &tables, &inputs, std::slice::from_ref(&origin));
        install(&session, &tables, &inputs, std::slice::from_ref(&q));
        install(&session, &tables, &inputs, &names);
        install(&session, &tables, &inputs, &exposures);
        let mut all = CatalogData::new(&budget);
        all.artifacts.insert(artifact).unwrap();
        all.modules.insert(module).unwrap();
        all.export_origins.insert(origin).unwrap();
        all.qualifications.insert(q).unwrap();
        for row in &names {
            all.names.insert(row.clone()).unwrap();
        }
        for row in &exposures {
            all.exposures.insert(row.clone()).unwrap();
        }
        let expected = catalog::build::build(&all, &budget).unwrap();
        drop(all);
        let prepared = CatalogScopes::prepare_bound(
            inputs.clone(),
            tables,
            &lctx_model::domain::model().unwrap(),
            &session,
            &budget,
        )
        .await
        .unwrap();
        let mut actual = catalog::build::CatalogOutput::new(&budget);
        for name in &names {
            let predicate = format!(
                "id=X'{}'",
                name.id()
                    .bytes()
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>()
            );
            let scope = prepared
                .edges
                .grain(prepared.root, &predicate, &budget)
                .await
                .unwrap();
            let data = load(&scope, &inputs, &budget).await;
            assert_eq!(data.artifacts.len(), 1);
            assert_eq!(data.names.len(), 1);
            assert_eq!(data.exposures.len(), 2);
            let rows = catalog::build::build(&data, &budget).unwrap();
            macro_rules! merge{($($field:ident:$ty:ty,)*)=>{$(for row in rows.$field.iter(){actual.$field.insert(row.clone()).unwrap();})*};}
            lctx_model::catalog_outputs!(merge);
        }
        actual.matches(&expected).unwrap();
        assert_eq!(actual.members.len(), 2);
        assert_eq!(actual.exposures.len(), 4);
        let keys = names
            .iter()
            .map(|name| {
                format!(
                    "X'{}'",
                    name.id()
                        .bytes()
                        .iter()
                        .map(|byte| format!("{byte:02x}"))
                        .collect::<String>()
                )
            })
            .collect::<Vec<_>>();
        let scope = prepared
            .edges
            .grain(
                prepared.root,
                &format!("id IN ({})", keys.join(",")),
                &budget,
            )
            .await
            .unwrap();
        let data = load(&scope, &inputs, &budget).await;
        let grouped = catalog::build::build(&data, &budget).unwrap();
        grouped.matches(&expected).unwrap();
        drop(grouped);
        drop(data);
        drop(scope);
        drop(actual);
        drop(expected);
        drop(prepared);
        assert_eq!(budget.reserved(), 0);
    }
}
