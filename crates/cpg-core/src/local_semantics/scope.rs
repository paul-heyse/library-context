//! Actual source Local roots; referenced sources remain dependency namespaces.
#[cfg(test)]
use crate::consumed_rows::PreparedClosure;
use crate::{
    consumed_rows::{ClosureTable, PreparedEdges},
    workspace::CompletedInputs,
};
use lctx_model::domain::{local_semantics::LocalData, *};
use std::{any::TypeId, sync::Arc};
pub(super) struct LocalScopes {
    inputs: Vec<ValidationInput>,
    #[cfg(test)]
    edges: PreparedEdges,
    base: Arc<scope_program::CompiledScopeProgram>,
    context_primary: Arc<scope_program::CompiledScopeProgram>,
    source: usize,
    tables: Vec<ClosureTable>,
    inventory: scope_program::ScopeProgram,
    #[cfg(test)]
    primary: scope_program::ScopeProgram,
    _charge: charged::StateCharge,
}
impl LocalScopes {
    pub(super) async fn prepare(
        access: &CompletedInputs,
        session: &datafusion::prelude::SessionContext,
        model: &Arc<ValidatedModel>,
        budget: &resources::ResourceBudget,
    ) -> Result<Self, ModelError> {
        let mut inputs = LocalData::validation_inputs();
        // NativeQualification selects a generated assertion/support pair; its canonical pair vocabulary
        // is an explicit closure premise even though the pure Local kernel only reads the qualification.
        inputs.push(ValidationInput::of::<
            analysis::native::NativeAssertionPremise,
        >(&["id"]));
        inputs.sort_by_key(|input| (input.name(), input.prefix()));
        inputs.dedup_by_key(|input| (input.name(), input.prefix()));
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
        let idx = |kind: TypeId| {
            tables[..real]
                .iter()
                .position(|table| table.relation.type_id() == kind)
        };
        let artifact = idx(TypeId::of::<source::SourceArtifact>())
            .ok_or(ModelError::Schema(source::SourceArtifact::NAME))?;
        let source = tables.len();
        tables.push(tables[artifact].clone());
        let mut declarations = inputs.clone();
        declarations.push(inputs[artifact].clone());
        let relations = tables
            .iter()
            .map(|table| table.relation.clone())
            .collect::<Vec<_>>();
        let program =
            compiler_scope_program::local(declarations.clone(), &relations, real, artifact)?;
        let inventory = compiler_scope_program::local_roots(declarations.clone(), real, false)?;
        #[cfg(test)]
        let primary = compiler_scope_program::local_roots(declarations.clone(), real, true)?;
        let context = compiler_scope_program::local_context_roots(declarations, real, budget)?;
        let base = crate::scope_compilation::compile(&program, model, budget, None)?;
        let context_primary =
            crate::scope_compilation::compile(context.program(), model, budget, None)?;
        #[cfg(test)]
        let edges = crate::scope_compilation::lower_compiled(
            base.clone(),
            &tables,
            &scope_program::ScopeParameters(vec![]),
            budget,
        )?
        .prepare(session, budget)
        .await?;
        let _ = session;
        let mut charge = charged::StateCharge::new(budget, "local-source-scope-descriptors");
        charge.grow(
            inputs.capacity() * size_of::<ValidationInput>()
                + tables.capacity() * size_of::<ClosureTable>()
                + real.saturating_add(1).saturating_mul(1024)
                + 16_384
                + tables
                    .iter()
                    .map(|table| table.alias.capacity())
                    .sum::<usize>(),
        )?;
        Ok(Self {
            inputs,
            #[cfg(test)]
            edges,
            base,
            context_primary,
            source,
            tables,
            inventory,
            #[cfg(test)]
            primary,
            _charge: charge,
        })
    }
    pub(super) fn root_sql(
        &self,
        access: &CompletedInputs,
        input: Id<input::InputRevision>,
        context: Id<attribution::AnalysisContext>,
        budget: &resources::ResourceBudget,
    ) -> Result<(String, charged::StateCharge), ModelError> {
        let _ = access;
        let mut charge = charged::StateCharge::new(budget, "local-root-inventory-query");
        charge.grow(1024)?;
        let parameters = scope_program::ScopeParameters(vec![
            scope_program::ScopeValue::Nominal(*context.bytes()),
            scope_program::ScopeValue::Nominal(*input.bytes()),
        ]);
        charge.grow(
            crate::scope_compilation::lowering_allowance(
                &self.inventory,
                &self.tables,
                &parameters,
            )
            .saturating_mul(4),
        )?;
        let queries = crate::scope_compilation::select_pair_queries(
            &self.inventory,
            &self.tables,
            &parameters,
        )?;
        let domains = queries
            .into_iter()
            .map(|(_, _, sql)| format!("SELECT source_id AS id FROM ({sql}) roots"))
            .collect::<Vec<_>>();
        Ok((
            format!(
                "SELECT DISTINCT id FROM ({}) roots ORDER BY id",
                domains.join(" UNION ALL ")
            ),
            charge,
        ))
    }
    pub(super) async fn context_edges(
        &self,
        session: &datafusion::prelude::SessionContext,
        context: Id<attribution::AnalysisContext>,
        budget: &resources::ResourceBudget,
    ) -> Result<PreparedEdges, ModelError> {
        let mut plan = crate::scope_compilation::lower_compiled(
            self.base.clone(),
            &self.tables,
            &scope_program::ScopeParameters(vec![]),
            budget,
        )?;
        let parameters = scope_program::ScopeParameters(vec![scope_program::ScopeValue::Nominal(
            *context.bytes(),
        )]);
        plan.extend(crate::scope_compilation::lower_compiled(
            self.context_primary.clone(),
            &self.tables,
            &parameters,
            budget,
        )?)?;
        plan.prepare(session, budget).await
    }
    pub(super) fn root(&self, key: [u8; 16]) -> crate::consumed_rows::PreparedRoot {
        crate::consumed_rows::PreparedRoot {
            table: self.source,
            key,
            kind: crate::consumed_rows::PreparedRootKind::Virtual,
        }
    }
    pub(super) async fn union(
        &self,
        selected: &crate::consumed_rows::PreparedRootBatch,
        runtime: &crate::workspace::Workspace,
    ) -> Result<LocalData, ModelError> {
        let mut data = LocalData::new(runtime.budget());
        crate::scoped_batch::hydrate_union(
            selected,
            &self.inputs,
            runtime.budget(),
            &runtime.cancellation(),
            &mut |_, input, batch| data.visit(input.name(), batch).map(|_| ()),
        )
        .await?;
        Ok(data)
    }
    pub(super) fn partition(
        &self,
        union: &LocalData,
        selected: &crate::consumed_rows::PreparedRootBatch,
        partition: usize,
        budget: &resources::ResourceBudget,
    ) -> Result<LocalData, ModelError> {
        union.selected_copy(
            &self.inputs,
            &mut |table, key| selected.contains(partition, table, key),
            budget,
        )
    }
    #[cfg(test)]
    pub(super) async fn source(
        &self,
        id: Id<source::SourceArtifact>,
        context: Id<attribution::AnalysisContext>,
        budget: &resources::ResourceBudget,
    ) -> Result<PreparedClosure, ModelError> {
        let parameters = scope_program::ScopeParameters(vec![
            scope_program::ScopeValue::Nominal(*context.bytes()),
            scope_program::ScopeValue::Nominal(*id.bytes()),
        ]);
        let queries = crate::scope_compilation::select_pair_queries(
            &self.primary,
            &self.tables,
            &parameters,
        )?;
        let mut roots = vec![(self.source, format!("id=X'{}'", id.hex()))];
        for (_, target, sql) in queries {
            roots.push((
                target,
                format!("id IN (SELECT target_id FROM ({sql}) primary_rows)"),
            ));
        }
        self.edges.grain_roots(&roots, budget).await
    }
}
#[cfg(test)]
mod local_source_scope_controls {
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
    async fn source_context_roots_exclude_unrelated_syntax_and_foreign_context_payload() {
        let model = model().unwrap();
        let session = datafusion::prelude::SessionContext::new();
        let budget = resources::ResourceBudget::fixed(16 << 20).unwrap();
        let mut inputs = LocalData::validation_inputs();
        inputs.push(ValidationInput::of::<
            analysis::native::NativeAssertionPremise,
        >(&["id"]));
        inputs.sort_by_key(|input| (input.name(), input.prefix()));
        inputs.dedup_by_key(|input| (input.name(), input.prefix()));
        let tables: Vec<_> = inputs
            .iter()
            .enumerate()
            .map(|(index, input)| ClosureTable {
                relation: model.relation(input.name()).unwrap().clone(),
                alias: format!("local_source_{index}"),
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
        let artifact =
            source::SourceArtifact::from_bytes(nominal(1), "selected.py".into(), b"x").unwrap();
        let dependency =
            source::SourceArtifact::from_bytes(artifact.input, "dependency.py".into(), b"x")
                .unwrap();
        let foreign = source::SourceArtifact::from_bytes(
            artifact.input,
            format!("{}.py", "z".repeat(256 << 10)),
            b"x",
        )
        .unwrap();
        let read = source::Occurrence {
            source: artifact.id(),
            start: 0,
            end: 1,
            syntax_kind: source::SyntaxKind::ExprName,
            role: source::OccurrenceRole::Read,
            structural_path: vec![0],
        };
        let declaration = source::Occurrence {
            source: dependency.id(),
            syntax_kind: source::SyntaxKind::StmtClassDef,
            role: source::OccurrenceRole::Declaration,
            ..read.clone()
        };
        let sibling = source::Occurrence {
            source: dependency.id(),
            structural_path: vec![99; 65536],
            ..read.clone()
        };
        let module = source::Module {
            source: dependency.id(),
            qualified_name: "dependency".into(),
        };
        let provider_module = calls::ProviderModule::Acquired {
            module: module.id(),
        };
        let symbol = calls::ProviderSymbol {
            provider: nominal(2),
            context: nominal(3),
            module: provider_module.id(),
            native_key: "Class".into(),
            name: "Class".into(),
            kind: calls::SymbolKind::Class,
        };
        let term = types::TypeTerm::ClassObject { class: symbol.id() };
        let q = assertion::AssertionQualification {
            context: nominal(3),
            scope: source::CoverageScope::Artifact {
                artifact: artifact.id(),
            }
            .id(),
            condition: conditions::Diagram::always().id(),
            modality: attribution::Modality::Definite,
            approximation: assertion::Approximation::Exact,
            assumptions: assumptions::AssumptionSet::empty().id(),
        };
        let observation = types::TypeObservation {
            qualification: q.id(),
            subject: read.id(),
            role: types::TypeRole::Expected,
            declared: false,
            term: term.id(),
        };
        let declared = declarations::SymbolDeclaration {
            qualification: q.id(),
            symbol: symbol.id(),
            declaration: declaration.id(),
        };
        let unrelated = source::Occurrence {
            source: artifact.id(),
            syntax_kind: source::SyntaxKind::StmtPass,
            role: source::OccurrenceRole::Syntax,
            structural_path: vec![22; 65536],
            ..read.clone()
        };
        let other_context = assertion::AssertionQualification {
            context: nominal(19),
            ..q.clone()
        };
        let other_read = source::Occurrence {
            structural_path: vec![20],
            ..read.clone()
        };
        let other_symbol = calls::ProviderSymbol {
            context: other_context.context,
            native_key: "foreign".repeat(65536),
            ..symbol.clone()
        };
        let other_term = types::TypeTerm::ClassObject {
            class: other_symbol.id(),
        };
        let other_observation = types::TypeObservation {
            qualification: other_context.id(),
            subject: other_read.id(),
            term: other_term.id(),
            ..observation.clone()
        };
        replace(&session, &inputs, &tables, &[q, other_context]);
        replace(
            &session,
            &inputs,
            &tables,
            &[artifact.clone(), dependency.clone(), foreign],
        );
        replace(
            &session,
            &inputs,
            &tables,
            &[
                read.clone(),
                declaration.clone(),
                sibling,
                unrelated,
                other_read,
            ],
        );
        replace(&session, &inputs, &tables, &[module]);
        replace(&session, &inputs, &tables, &[provider_module]);
        replace(&session, &inputs, &tables, &[symbol, other_symbol]);
        replace(&session, &inputs, &tables, &[term, other_term]);
        replace(
            &session,
            &inputs,
            &tables,
            &[observation, other_observation],
        );
        replace(&session, &inputs, &tables, &[declared]);
        let scopes = LocalScopes::from_tables(inputs.clone(), tables, &session, &budget, &model)
            .await
            .unwrap();
        let tiny = resources::ResourceBudget::fixed(96 << 10).unwrap();
        let grain = scopes
            .source(artifact.id(), nominal(3), &tiny)
            .await
            .unwrap();
        let table = inputs
            .iter()
            .position(|input| input.type_id() == TypeId::of::<source::Occurrence>())
            .unwrap();
        let mut occurrences = Rows::<source::Occurrence>::new(&tiny);
        use futures::TryStreamExt;
        let mut stream = crate::sql::query(grain.session(), &grain.select(table).unwrap())
            .await
            .unwrap()
            .execute_stream()
            .await
            .unwrap();
        while let Some(batch) = stream.try_next().await.unwrap() {
            occurrences.decode(&batch).unwrap();
        }
        assert_eq!(occurrences.len(), 2);
        assert_eq!(occurrences.get(read.id()), Some(&read));
        assert_eq!(occurrences.get(declaration.id()), Some(&declaration));
        let table = inputs
            .iter()
            .position(|input| input.type_id() == TypeId::of::<calls::ProviderSymbol>())
            .unwrap();
        let mut symbols = Rows::<calls::ProviderSymbol>::new(&tiny);
        let mut stream = crate::sql::query(grain.session(), &grain.select(table).unwrap())
            .await
            .unwrap()
            .execute_stream()
            .await
            .unwrap();
        while let Some(batch) = stream.try_next().await.unwrap() {
            symbols.decode(&batch).unwrap();
        }
        assert_eq!(symbols.len(), 1);
        drop(symbols);
        drop(stream);
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
        assert_eq!(artifacts.len(), 2);
        assert_eq!(artifacts.get(dependency.id()), Some(&dependency));
        let edges = scopes
            .context_edges(&session, nominal(3), &budget)
            .await
            .unwrap();
        let roots = [scopes.root(*artifact.id().bytes()); 2];
        let selected = edges.batch(&roots, &budget).await.unwrap();
        let mut union = LocalData::new(&budget);
        let mut observation_decodes = 0;
        crate::scoped_batch::hydrate_union(
            &selected,
            &inputs,
            &budget,
            &crate::workspace::Cancellation::default(),
            &mut |_, input, batch| {
                if input.name() == types::TypeObservation::NAME {
                    observation_decodes += batch.num_rows();
                }
                union.visit(input.name(), batch).map(|_| ())
            },
        )
        .await
        .unwrap();
        assert_eq!(
            observation_decodes, 1,
            "overlapping source requests share contextual native discovery and hydration"
        );
        let first = scopes.partition(&union, &selected, 0, &budget).unwrap();
        let second = scopes.partition(&union, &selected, 1, &budget).unwrap();
        assert!(
            first
                .theory
                .type_observations
                .same(&second.theory.type_observations)
        );
        assert_eq!(first.theory.type_observations.len(), 1);
        assert_eq!(first.entry.occurrences.len(), 2);
        assert_eq!(first.entry.symbols.len(), 1);
    }
}
