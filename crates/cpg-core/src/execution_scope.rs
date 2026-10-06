//! Ordered completion syntax dependencies, separated from ancestor/owner metadata.
use crate::{
    consumed_rows::{
        ClosureTable, NominalClosure, PreparedClosure, PreparedEdges, identifier, stream_query_at,
    },
    workspace::CompletedInputs,
};
use lctx_model::domain::{
    analysis::native::*,
    execution::{
        completion_production::CompletedEvaluations, evaluation::EvaluationData, records::*,
    },
    normalized::entities::*,
    source::*,
    syntax::*,
    *,
};
use std::{any::TypeId, sync::Arc};

pub(super) fn nominal<R>(bytes: &[u8]) -> Result<Id<R>, ModelError> {
    serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
        _,
        serde::de::value::Error,
    >::new(bytes.iter().copied()))
    .map_err(ModelError::codec)
}
pub(super) fn predicate<R>(id: Id<R>) -> String {
    crate::scoped_admission::root_predicate(&[*id.bytes()])
}
/// A compact external edge index shared by one immutable execution input set. Rich rows are
/// decoded only after selecting a statement tree or one ordered callable body.
pub(super) struct CompletionScopes {
    inputs: Vec<ValidationInput>,
    tables: Vec<ClosureTable>,
    edges: PreparedEdges,
    dependency: usize,
    body: usize,
    _charge: charged::StateCharge,
}
impl CompletionScopes {
    pub(super) async fn prepare(
        access: &CompletedInputs,
        session: &datafusion::prelude::SessionContext,
        model: &Arc<ValidatedModel>,
        budget: &resources::ResourceBudget,
    ) -> Result<Self, ModelError> {
        // Actual predecessor receipts already own entry values. Hydration needs their published
        // member inventory, not another native entry producer replay or its rich fact universe.
        let mut inputs = EvaluationData::validation_inputs();
        inputs.extend([
            ValidationInput::of::<analysis::base_evaluation::AnalysisInvocation>(&["id"]),
            ValidationInput::of::<analysis::AnalysisDefinition>(&["id"]),
            ValidationInput::of::<ExpressionEvaluation>(&["id"]),
            ValidationInput::of::<EvaluationSource>(&["id"]),
            ValidationInput::of::<EvaluationMember>(&["id"]),
            ValidationInput::of::<EvaluationOperand>(&["id"]),
        ]);
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
        Self::prepare_bound(inputs, tables, session, budget).await
    }
    async fn prepare_bound(
        inputs: Vec<ValidationInput>,
        mut tables: Vec<ClosureTable>,
        session: &datafusion::prelude::SessionContext,
        budget: &resources::ResourceBudget,
    ) -> Result<Self, ModelError> {
        let index = |kind: TypeId| {
            inputs
                .iter()
                .position(|input| input.type_id() == kind)
                .ok_or(ModelError::Schema("completion closure input"))
        };
        let occurrence = index(TypeId::of::<Occurrence>())?;
        let owner = index(TypeId::of::<EntityRef>())?;
        let dependency = tables.len();
        tables.push(tables[occurrence].clone());
        let body = tables.len();
        tables.push(tables[index(TypeId::of::<CallableEntity>())?].clone());
        let child_placement = tables.len();
        tables.push(tables[index(TypeId::of::<SyntaxPlacement>())?].clone());
        let mut plan = NominalClosure::new(tables.clone())?;
        for (source, table) in tables.iter().take(inputs.len()).enumerate() {
            for field in table.relation.fields() {
                let Some((target, _)) = field.target() else {
                    continue;
                };
                let Some(target) = crate::scoped_admission::field_target(&inputs, source, target)?
                else {
                    continue;
                };
                if field.list() {
                    plan.pairs(
                        source,
                        target,
                        format!(
                            "SELECT id AS source_id,UNNEST({}) AS target_id FROM {}",
                            identifier(field.name()),
                            identifier(&table.alias)
                        ),
                    )?;
                } else {
                    plan.follow(source, field.name(), target)?;
                }
                // Native pair alternatives remain complete. Only assertion/support membership
                // reverses here; a referenced qualification/run/source never imports its universe.
                if field.name() == "assertion"
                    || field.name().ends_with("_assertion")
                    || (table.relation.type_id() == TypeId::of::<NativeQualification>()
                        && field.name() == "premise")
                {
                    plan.own(source, field.name(), target)?;
                }
            }
        }
        macro_rules! own {
            ($member:ty,$field:literal,$owner:ty) => {
                plan.own(
                    index(TypeId::of::<$member>())?,
                    $field,
                    index(TypeId::of::<$owner>())?,
                )?
            };
        }
        own!(input::ArtifactUse, "artifact", SourceArtifact);
        own!(SyntaxPlacement, "occurrence", Occurrence);
        own!(OccurrenceOwnership, "occurrence", Occurrence);
        own!(DeclarationObservation, "declaration", Occurrence);
        own!(ExpressionEvaluation, "expression", Occurrence);
        own!(EvaluationMember, "evaluation", ExpressionEvaluation);
        own!(EvaluationOperand, "evaluation", ExpressionEvaluation);
        let occurrences = identifier(&tables[occurrence].alias);
        let placements = identifier(&tables[index(TypeId::of::<SyntaxPlacement>())?].alias);
        let refs = identifier(&tables[owner].alias);
        let callables = identifier(&tables[index(TypeId::of::<CallableEntity>())?].alias);
        let ownership = identifier(&tables[index(TypeId::of::<OccurrenceOwnership>())?].alias);
        let details = identifier(&tables[index(TypeId::of::<SyntaxDetailObservation>())?].alias);
        let spellings = identifier(&tables[index(TypeId::of::<SyntaxObservation>())?].alias);
        let events = identifier(&tables[index(TypeId::of::<lexical::BindingEvent>())?].alias);
        let bindings =
            identifier(&tables[index(TypeId::of::<lexical::BindingObservation>())?].alias);
        let references =
            identifier(&tables[index(TypeId::of::<lexical::ReferenceObservation>())?].alias);
        plan.pairs(occurrence,index(TypeId::of::<SyntaxDetailObservation>())?,format!("SELECT o.id AS source_id,d.id AS target_id FROM {occurrences} o JOIN {details} d ON d.occurrence=o.id WHERE o.syntax_kind IN ({},{})",SyntaxKind::StmtTry.code(),SyntaxKind::ExceptHandlerExceptHandler.code()))?;
        plan.pairs(occurrence,index(TypeId::of::<SyntaxObservation>())?,format!("SELECT o.id AS source_id,s.id AS target_id FROM {occurrences} o JOIN {spellings} s ON s.occurrence=o.id WHERE o.syntax_kind={}",SyntaxKind::Identifier.code()))?;
        own!(lexical::BindingEvent, "site", Occurrence);
        own!(lexical::BindingObservation, "event", lexical::BindingEvent);
        own!(lexical::ReferenceObservation, "read", Occurrence);
        // A bound handler's cleanup checks all same-scope/name bindings, plus every native
        // occurrence/reference within its span, even an observation without a syntax placement.
        plan.pairs(index(TypeId::of::<lexical::BindingObservation>())?,index(TypeId::of::<lexical::BindingObservation>())?,format!("SELECT a.id AS source_id,b.id AS target_id FROM {bindings} a JOIN {events} ae ON ae.id=a.event JOIN {bindings} b ON b.scope=a.scope JOIN {events} be ON be.id=b.event WHERE be.name=ae.name"))?;
        plan.pairs(occurrence,index(TypeId::of::<lexical::BindingEvent>())?,format!("SELECT h.id AS source_id,e.id AS target_id FROM {occurrences} h JOIN {occurrences} o ON o.source=h.source AND o.start>=h.start AND o.end<=h.end JOIN {events} e ON e.site=o.id WHERE h.syntax_kind={}",SyntaxKind::ExceptHandlerExceptHandler.code()))?;
        plan.pairs(occurrence,index(TypeId::of::<lexical::ReferenceObservation>())?,format!("SELECT h.id AS source_id,r.id AS target_id FROM {occurrences} h JOIN {occurrences} o ON o.source=h.source AND o.start>=h.start AND o.end<=h.end JOIN {references} r ON r.read=o.id WHERE h.syntax_kind={}",SyntaxKind::ExceptHandlerExceptHandler.code()))?;
        // Exception matching consumes the full finite builtin class domain in each selected
        // context, including competing providers; a sealed Class value carries its exact ID.
        let qualifications =
            identifier(&tables[index(TypeId::of::<assertion::AssertionQualification>())?].alias);
        let symbols = identifier(&tables[index(TypeId::of::<calls::ProviderSymbol>())?].alias);
        let modules = identifier(&tables[index(TypeId::of::<calls::ProviderModule>())?].alias);
        let names = std::iter::once("BaseException")
            .chain(
                execution::ExactRuntimeException::ALL
                    .iter()
                    .map(|kind| kind.class().1),
            )
            .map(|name| format!("'{name}'"))
            .collect::<Vec<_>>()
            .join(",");
        plan.pairs(index(TypeId::of::<assertion::AssertionQualification>())?,index(TypeId::of::<calls::ProviderSymbol>())?,format!("SELECT q.id AS source_id,s.id AS target_id FROM {qualifications} q JOIN {symbols} s ON s.context=q.context JOIN {modules} m ON m.id=s.module WHERE s.kind={} AND s.name IN ({names}) AND m.bundled_name='builtins' AND m.bundled_bundle={}",calls::SymbolKind::Class.code(),calls::ModuleBundle::Typeshed.code()))?;
        plan.pairs(
            dependency,
            occurrence,
            format!("SELECT id AS source_id,id AS target_id FROM {occurrences}"),
        )?;
        // Ancestors use the ordinary metadata namespace. Only explicit dependency nodes may
        // expand children, and a nested function's Body belongs to a separate callable kernel.
        plan.pairs(dependency,child_placement,format!("SELECT o.id AS source_id,p.id AS target_id FROM {occurrences} o JOIN {placements} p ON p.parent=o.id WHERE o.syntax_kind NOT IN ({},{},{}) OR p.field!={}",SyntaxKind::StmtFunctionDef.code(),SyntaxKind::StmtClassDef.code(),SyntaxKind::ExprLambda.code(),lexical::SyntaxField::Body.code()))?;
        plan.pairs(
            child_placement,
            index(TypeId::of::<SyntaxPlacement>())?,
            format!("SELECT id AS source_id,id AS target_id FROM {placements}"),
        )?;
        plan.pairs(
            child_placement,
            dependency,
            format!("SELECT p.id AS source_id,p.occurrence AS target_id FROM {placements} p"),
        )?;
        // A callable-body root includes its exact direct Body suite. Forward metadata of another
        // callable cannot become a body root because this private namespace has no incoming edge.
        plan.pairs(
            body,
            index(TypeId::of::<CallableEntity>())?,
            format!("SELECT id AS source_id,id AS target_id FROM {callables}"),
        )?;
        plan.pairs(body,owner,format!("SELECT c.id AS source_id,r.id AS target_id FROM {callables} c JOIN {refs} r ON r.callable_callable=c.id"))?;
        // complete_body observes every immediate declaration placement before selecting its
        // Body suite. Keep that complete native metadata domain in the ordinary namespace;
        // only Body placements below enter recursive dependency traversal.
        plan.pairs(
            body,
            index(TypeId::of::<SyntaxPlacement>())?,
            format!("SELECT c.id AS source_id,p.id AS target_id FROM {callables} c JOIN {placements} p ON p.parent=c.source_declaration"),
        )?;
        plan.pairs(body,child_placement,format!("SELECT c.id AS source_id,p.id AS target_id FROM {callables} c JOIN {placements} p ON p.parent=c.source_declaration WHERE p.field={}",lexical::SyntaxField::Body.code()))?;
        // Generator uncertainty is a complete owner predicate, independent of entered syntax.
        plan.pairs(owner,index(TypeId::of::<OccurrenceOwnership>())?,format!("SELECT r.id AS source_id,m.id AS target_id FROM {refs} r JOIN {ownership} m ON m.entity=r.id JOIN {occurrences} o ON o.id=m.occurrence WHERE o.syntax_kind IN ({},{})",SyntaxKind::ExprYield.code(),SyntaxKind::ExprYieldFrom.code()))?;
        let mut charge = charged::StateCharge::new(budget, "completion-scope-descriptors");
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
            tables,
            edges,
            dependency,
            body,
            _charge: charge,
        })
    }
    pub(super) fn table<R: Record>(&self) -> Result<String, ModelError> {
        Ok(identifier(
            &self.tables[self
                .inputs
                .iter()
                .position(|input| input.type_id() == TypeId::of::<R>())
                .ok_or(ModelError::Schema("completion root table"))?]
            .alias,
        ))
    }
    pub(super) fn roots(
        &self,
        input: Id<input::InputRevision>,
        body: bool,
    ) -> Result<String, ModelError> {
        let artifacts = self.table::<SourceArtifact>()?;
        let uses = self.table::<input::ArtifactUse>()?;
        let input = predicate(input).replacen("id IN", "a.input IN", 1);
        let selected = format!(
            "{input} AND (a.path LIKE '%.py' OR a.path LIKE '%.pyi') AND EXISTS (SELECT 1 FROM {uses} u WHERE u.artifact=a.id AND u.role IN ({},{},{},{}))",
            input::SourceRole::Release.code(),
            input::SourceRole::Example.code(),
            input::SourceRole::Test.code(),
            input::SourceRole::DocBlock.code()
        );
        let occurrences = self.table::<Occurrence>()?;
        if body {
            let callables = self.table::<CallableEntity>()?;
            Ok(format!(
                "SELECT c.id FROM {callables} c JOIN {occurrences} o ON o.id=c.source_declaration JOIN {artifacts} a ON a.id=o.source WHERE {selected} ORDER BY c.id"
            ))
        } else {
            let kinds = [
                SyntaxKind::StmtFunctionDef,
                SyntaxKind::StmtClassDef,
                SyntaxKind::StmtReturn,
                SyntaxKind::StmtDelete,
                SyntaxKind::StmtTypeAlias,
                SyntaxKind::StmtAssign,
                SyntaxKind::StmtAugAssign,
                SyntaxKind::StmtAnnAssign,
                SyntaxKind::StmtFor,
                SyntaxKind::StmtWhile,
                SyntaxKind::StmtIf,
                SyntaxKind::StmtWith,
                SyntaxKind::StmtMatch,
                SyntaxKind::StmtRaise,
                SyntaxKind::StmtTry,
                SyntaxKind::StmtAssert,
                SyntaxKind::StmtImport,
                SyntaxKind::StmtImportFrom,
                SyntaxKind::StmtGlobal,
                SyntaxKind::StmtNonlocal,
                SyntaxKind::StmtExpr,
                SyntaxKind::StmtPass,
                SyntaxKind::StmtBreak,
                SyntaxKind::StmtContinue,
                SyntaxKind::StmtIpyEscapeCommand,
            ]
            .iter()
            .map(|kind| kind.code().to_string())
            .collect::<Vec<_>>()
            .join(",");
            Ok(format!(
                "SELECT o.id FROM {occurrences} o JOIN {artifacts} a ON a.id=o.source WHERE {selected} AND o.syntax_kind IN ({kinds}) ORDER BY o.id"
            ))
        }
    }
    pub(super) async fn statement(
        &self,
        id: Id<Occurrence>,
        budget: &resources::ResourceBudget,
    ) -> Result<PreparedClosure, ModelError> {
        self.edges
            .grain(self.dependency, &predicate(id), budget)
            .await
    }
    pub(super) async fn body(
        &self,
        id: Id<CallableEntity>,
        budget: &resources::ResourceBudget,
    ) -> Result<PreparedClosure, ModelError> {
        self.edges.grain(self.body, &predicate(id), budget).await
    }
    pub(super) async fn data(
        &self,
        access: &CompletedInputs,
        scope: &PreparedClosure,
        budget: &resources::ResourceBudget,
    ) -> Result<CompletedEvaluations, ModelError> {
        let mut data = CompletedEvaluations::new(budget);
        for (table, input) in self.inputs.iter().enumerate() {
            let sql = scope.select(table)?;
            macro_rules! read {($($field:ident:$ty:ty,)*)=>{$(if input.type_id()==TypeId::of::<$ty>() {let permit=access.read_at::<$ty>(input.prefix())?;stream_query_at(&permit,input,scope.session(),&sql,|_,batch|data.visit_input(input,batch)).await?;})*};}
            lctx_model::execution_evaluation_inputs!(read);
            macro_rules! records {($($ty:ty),*)=>{$(if input.type_id()==TypeId::of::<$ty>() {let permit=access.read_at::<$ty>(input.prefix())?;stream_query_at(&permit,input,scope.session(),&sql,|_,batch|data.visit_input(input,batch)).await?;})*};}
            records!(
                analysis::base_evaluation::AnalysisInvocation,
                analysis::AnalysisDefinition,
                ExpressionEvaluation,
                EvaluationSource,
                EvaluationMember,
                EvaluationOperand
            );
        }
        Ok(data)
    }
}

#[cfg(test)]
mod controls {
    use super::*;
    use assertion::*;
    use datafusion::{datasource::MemTable, prelude::SessionContext};
    use futures::TryStreamExt;
    fn id<R>(n: u8) -> Id<R> {
        nominal(&[n; 16]).unwrap()
    }
    fn register<R: Record>(session: &SessionContext, rows: &[R]) {
        let batch = R::encode(rows).unwrap();
        session.deregister_table(R::NAME).unwrap();
        session
            .register_table(
                R::NAME,
                Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap()),
            )
            .unwrap();
    }
    async fn selected<R: Record>(scope: &PreparedClosure, prepared: &CompletionScopes) -> Vec<R> {
        let table = prepared
            .inputs
            .iter()
            .position(|input| input.type_id() == TypeId::of::<R>())
            .unwrap();
        let mut stream = crate::sql::query(scope.session(), &scope.select(table).unwrap())
            .await
            .unwrap()
            .execute_stream()
            .await
            .unwrap();
        let mut rows = Vec::new();
        while let Some(batch) = stream.try_next().await.unwrap() {
            rows.extend(R::decode(&batch).unwrap());
        }
        rows
    }
    async fn fixture(
        session: &SessionContext,
        budget: &resources::ResourceBudget,
    ) -> (CompletionScopes, Occurrence, CallableEntity, Occurrence) {
        let artifact = SourceArtifact::from_bytes(
            id(1),
            "body.py".into(),
            b"def f():\n pass\ndef g():\n pass\n",
        )
        .unwrap();
        let scope = CoverageScope::Artifact {
            artifact: artifact.id(),
        };
        let q = AssertionQualification {
            context: id(2),
            scope: scope.id(),
            assumptions: assumptions::AssumptionSet::empty_id(),
            condition: conditions::Diagram::always().id(),
            modality: attribution::Modality::Definite,
            approximation: Approximation::Exact,
        };
        let mut occurrences = Vec::new();
        let mut callables = Vec::new();
        let mut refs = Vec::new();
        let mut owners = Vec::new();
        let mut placements = Vec::new();
        let mut declarations = Vec::new();
        for (index, offset) in [0, 14].into_iter().enumerate() {
            let declaration = Occurrence {
                source: artifact.id(),
                start: offset,
                end: offset + 13,
                syntax_kind: SyntaxKind::StmtFunctionDef,
                role: OccurrenceRole::Declaration,
                structural_path: vec![index as i32],
            };
            let name = Occurrence {
                start: offset + 4,
                end: offset + 5,
                syntax_kind: SyntaxKind::ExprName,
                role: OccurrenceRole::Syntax,
                structural_path: vec![index as i32, 0],
                ..declaration.clone()
            };
            let statement = Occurrence {
                start: offset + 10,
                end: offset + 13,
                syntax_kind: SyntaxKind::StmtPass,
                role: OccurrenceRole::Syntax,
                structural_path: if index == 0 {
                    vec![0, 1]
                } else {
                    vec![1; 2_000_000]
                },
                ..declaration.clone()
            };
            let callable = CallableEntity::Source {
                declaration: declaration.id(),
                kind: CallableKind::Function,
            };
            let reference = EntityRef::Callable {
                callable: callable.id(),
            };
            owners.push(OccurrenceOwnership {
                occurrence: statement.id(),
                owner: declaration.id(),
                entity: reference.id(),
            });
            // An immediate non-Body child is observed by the body owner as metadata.
            placements.push(SyntaxPlacement {
                qualification: q.id(),
                occurrence: name.id(),
                parent: Some(declaration.id()),
                field: lexical::SyntaxField::Child,
                ordinal: 0,
            });
            placements.push(SyntaxPlacement {
                qualification: q.id(),
                occurrence: statement.id(),
                parent: Some(declaration.id()),
                field: lexical::SyntaxField::Body,
                ordinal: 0,
            });
            placements.push(SyntaxPlacement {
                qualification: q.id(),
                occurrence: declaration.id(),
                parent: None,
                field: lexical::SyntaxField::Body,
                ordinal: index as i64,
            });
            declarations.push(DeclarationObservation {
                qualification: q.id(),
                declaration: declaration.id(),
                name: name.id(),
                kind: DeclarationKind::Function,
                parent: None,
                overload: false,
                docstring: None,
            });
            occurrences.extend([declaration, name, statement]);
            callables.push(callable);
            refs.push(reference);
        }
        let first_statement = occurrences[2].clone();
        let rich = occurrences[5].clone();
        let first_callable = callables[0].clone();
        macro_rules! empty {($($field:ident:$ty:ty,)*)=>{$(register::<$ty>(session,&[]);)*};}
        lctx_model::execution_evaluation_inputs!(empty);
        macro_rules! records {($($ty:ty),*)=>{$(register::<$ty>(session,&[]);)*};}
        records!(
            analysis::base_evaluation::AnalysisInvocation,
            analysis::AnalysisDefinition,
            ExpressionEvaluation,
            EvaluationSource,
            EvaluationMember,
            EvaluationOperand
        );
        register(session, std::slice::from_ref(&artifact));
        register(
            session,
            &[input::ArtifactUse {
                artifact: artifact.id(),
                input: artifact.input,
                role: input::SourceRole::Release,
            }],
        );
        register(session, &[scope]);
        register(session, &[q]);
        register(session, &occurrences);
        register(session, &callables);
        register(session, &refs);
        register(session, &owners);
        register(session, &placements);
        register(session, &declarations);
        drop(occurrences);
        drop(callables);
        drop(refs);
        drop(owners);
        drop(placements);
        drop(declarations);
        let mut inputs = EvaluationData::validation_inputs();
        inputs.extend([
            ValidationInput::of::<analysis::base_evaluation::AnalysisInvocation>(&["id"]),
            ValidationInput::of::<analysis::AnalysisDefinition>(&["id"]),
            ValidationInput::of::<ExpressionEvaluation>(&["id"]),
            ValidationInput::of::<EvaluationSource>(&["id"]),
            ValidationInput::of::<EvaluationMember>(&["id"]),
            ValidationInput::of::<EvaluationOperand>(&["id"]),
        ]);
        inputs.sort_by_key(|input| (input.name(), input.prefix()));
        inputs.dedup_by_key(|input| (input.name(), input.prefix()));
        let model = model().unwrap();
        let tables = inputs
            .iter()
            .map(|input| ClosureTable {
                relation: model.relation(input.name()).unwrap().clone(),
                alias: input.name().into(),
            })
            .collect();
        (
            CompletionScopes::prepare_bound(inputs, tables, session, budget)
                .await
                .unwrap(),
            first_statement,
            first_callable,
            rich,
        )
    }
    #[tokio::test]
    async fn statement_metadata_ancestors_do_not_expand_their_body_or_unrelated_rich_source() {
        let session = SessionContext::new();
        let budget = resources::ResourceBudget::fixed(4 << 20).unwrap();
        let (prepared, statement, _, rich) = fixture(&session, &budget).await;
        let retained = budget.reserved();
        let scope = prepared.statement(statement.id(), &budget).await.unwrap();
        let occurrences = selected::<Occurrence>(&scope, &prepared).await;
        assert!(occurrences.iter().any(|row| row.id() == statement.id()));
        assert!(!occurrences.iter().any(|row| row.id() == rich.id()));
        assert!(occurrences.iter().all(|row| row.structural_path.len() < 4));
        drop(occurrences);
        drop(scope);
        assert_eq!(budget.reserved(), retained);
        drop(prepared);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn body_root_retains_complete_direct_suite_without_importing_another_callable_body() {
        let session = SessionContext::new();
        let budget = resources::ResourceBudget::fixed(4 << 20).unwrap();
        let (prepared, statement, callable, rich) = fixture(&session, &budget).await;
        let retained = budget.reserved();
        let scope = prepared.body(callable.id(), &budget).await.unwrap();
        let occurrences = selected::<Occurrence>(&scope, &prepared).await;
        assert!(occurrences.iter().any(|row| row.id() == statement.id()));
        assert!(!occurrences.iter().any(|row| row.id() == rich.id()));
        let placements = selected::<SyntaxPlacement>(&scope, &prepared).await;
        assert_eq!(
            placements
                .iter()
                .filter(|row| row.parent
                    == Some(match callable {
                        CallableEntity::Source { declaration, .. } => declaration,
                        _ => unreachable!(),
                    }))
                .count(),
            2
        );
        assert!(
            placements
                .iter()
                .any(|row| row.field == lexical::SyntaxField::Child)
        );
        assert!(
            placements
                .iter()
                .any(|row| row.field == lexical::SyntaxField::Body)
        );
        drop(placements);
        drop(occurrences);
        drop(scope);
        assert_eq!(budget.reserved(), retained);
        drop(prepared);
        assert_eq!(budget.reserved(), 0);
    }
}
