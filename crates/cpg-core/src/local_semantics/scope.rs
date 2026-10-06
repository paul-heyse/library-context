//! Actual source Local roots; referenced sources remain dependency namespaces.
use crate::{
    consumed_rows::{ClosureTable, NominalClosure, PreparedClosure, PreparedEdges, identifier},
    workspace::CompletedInputs,
};
use lctx_model::domain::{local_semantics::LocalData, *};
use std::{any::TypeId, sync::Arc};
pub(super) struct LocalScopes {
    inputs: Vec<ValidationInput>,
    edges: PreparedEdges,
    source: usize,
    tables: Vec<ClosureTable>,
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
        let artifact = idx(TypeId::of::<source::SourceArtifact>())
            .ok_or(ModelError::Schema(source::SourceArtifact::NAME))?;
        let source = tables.len();
        tables.push(tables[artifact].clone());
        let idx = |kind: TypeId| {
            tables[..real]
                .iter()
                .position(|table| table.relation.type_id() == kind)
        };
        let alias = |kind: TypeId| idx(kind).map(|index| identifier(&tables[index].alias));
        let mut plan = NominalClosure::new(tables.clone())?;
        for (from, table) in tables[..real].iter().enumerate() {
            for field in table.relation.fields().iter().filter(|field| !field.list()) {
                if let Some((kind, _)) = field.target()
                    && let Some(to) = idx(kind)
                {
                    plan.follow(from, field.name(), to)?;
                }
            }
        }
        let artifacts = identifier(&tables[source].alias);
        plan.pairs(
            source,
            artifact,
            format!("SELECT id AS source_id,id AS target_id FROM {artifacts}"),
        )?;
        let occurrences = idx(TypeId::of::<source::Occurrence>())
            .ok_or(ModelError::Schema(source::Occurrence::NAME))?;
        let occurrences_alias = identifier(&tables[occurrences].alias);

        macro_rules! own {
            ($member:ty,$field:literal,$owner:ty) => {
                if let (Some(member), Some(owner)) =
                    (idx(TypeId::of::<$member>()), idx(TypeId::of::<$owner>()))
                {
                    plan.own(member, $field, owner)?;
                }
            };
        }
        // Exact native and normalized candidate bags required by the Entry/Theory/Field kernels.
        own!(
            normalized::entities::OccurrenceOwnership,
            "occurrence",
            source::Occurrence
        );
        own!(flow::FlowUse, "occurrence", source::Occurrence);
        own!(flow::FlowUseObservation, "use_", flow::FlowUse);
        own!(flow::FlowReachingObservation, "use_", flow::FlowUse);
        own!(
            flow_inventory::FlowUseInventoryObservation,
            "use_",
            flow::FlowUse
        );
        own!(
            flow_inventory::FlowUseCandidate,
            "inventory",
            flow_inventory::FlowUseInventoryObservation
        );
        own!(
            flow_inventory::FlowUseInventoryMember,
            "inventory",
            flow_inventory::FlowUseInventoryObservation
        );
        own!(
            flow::FlowDefinitionObservation,
            "definition",
            flow::FlowDefinition
        );
        own!(flow::FlowValueObservation, "sink", source::Occurrence);
        own!(flow::FlowTestLeafObservation, "test", source::Occurrence);
        own!(
            flow::FlowAttributeLoadObservation,
            "occurrence",
            source::Occurrence
        );
        own!(flow::FlowRegionObservation, "scope", lexical::LexicalScope);
        own!(syntax::SyntaxPlacement, "occurrence", source::Occurrence);
        own!(
            normalized::entities::ParameterEntityLink,
            "entity",
            normalized::entities::ParameterEntity
        );
        own!(
            normalized::entities::ParameterEntityLink,
            "parameter",
            calls::SignatureParameter
        );
        own!(
            declarations::SymbolDeclaration,
            "symbol",
            calls::ProviderSymbol
        );
        own!(
            declarations::ParameterDeclaration,
            "parameter",
            calls::SignatureParameter
        );
        own!(
            declarations::ParameterDeclaration,
            "declaration",
            source::Occurrence
        );
        own!(
            syntax::ParameterSyntaxObservation,
            "parameter",
            source::Occurrence
        );
        for (member, table) in tables[..real].iter().enumerate().filter(|(_, table)| {
            table.relation.type_id() == TypeId::of::<normalized::entities::ParameterEntity>()
        }) {
            for field in table.relation.fields().iter().filter(|field| {
                !field.list()
                    && field.target().map(|(kind, _)| kind)
                        == Some(TypeId::of::<source::Occurrence>())
            }) {
                plan.own(member, field.name(), occurrences)?;
            }
        }
        // Only actual scalar/receiver/formal containers own these child memberships. Ordinary
        // enclosing statements and referenced modules do not expand their entire syntax subtree.
        if let Some(placements) = idx(TypeId::of::<syntax::SyntaxPlacement>()) {
            plan.pairs(occurrences,placements,format!("SELECT o.id AS source_id,p.id AS target_id FROM {occurrences_alias} o JOIN {} p ON p.parent=o.id WHERE o.syntax_kind IN ({},{},{},{})",identifier(&tables[placements].alias),source::SyntaxKind::ParameterWithDefault.code(),source::SyntaxKind::ExprAttribute.code(),source::SyntaxKind::ExprCompare.code(),source::SyntaxKind::ExprCall.code()))?;
        }
        own!(
            symbols::ClassAncestryObservation,
            "class",
            calls::ProviderSymbol
        );
        own!(
            symbols::SymbolSequenceMember,
            "sequence",
            symbols::SymbolSequence
        );
        own!(types::TypeSequenceMember, "sequence", types::TypeSequence);
        own!(value::LiteralSetMember, "set", value::LiteralSet);
        own!(types::TypeObservation, "subject", source::Occurrence);
        own!(types::TypeQueryObservation, "subject", source::Occurrence);
        own!(
            normalized::entities::SymbolEntityResolution,
            "symbol",
            calls::ProviderSymbol
        );
        own!(
            normalized::entities::FieldEntity,
            "class",
            normalized::entities::ClassEntity
        );
        own!(
            normalized::entities::FieldDeclarationLink,
            "field",
            normalized::entities::FieldEntity
        );
        own!(
            normalized::symbolic_fields::SourceFieldStore,
            "target",
            source::Occurrence
        );
        own!(lexical::LexicalResolution, "read", source::Occurrence);
        own!(calls::CallSyntax, "site", source::Occurrence);
        own!(calls::CallArgument, "call", calls::CallSyntax);
        own!(
            normalized::links::TestOperandTypeAssessment,
            "leaf",
            flow::FlowTestLeafObservation
        );
        own!(
            normalized::links::TestOperandTypeLink,
            "assessment",
            normalized::links::TestOperandTypeAssessment
        );
        own!(
            normalized::links::TestOperandCoverage,
            "assessment",
            normalized::links::TestOperandTypeAssessment
        );
        for (member, table) in tables[..real].iter().enumerate() {
            if let Some(field) = table
                .relation
                .fields()
                .iter()
                .find(|field| field.name() == "assertion")
                && let Some((kind, _)) = field.target()
                && let Some(owner) = idx(kind)
            {
                plan.own(member, field.name(), owner)?;
            }
        }
        if let Some(native) = idx(TypeId::of::<analysis::native::NativeAssertionPremise>()) {
            for field in tables[native]
                .relation
                .fields()
                .iter()
                .filter(|field| !field.list())
            {
                if let Some((kind, _)) = field.target()
                    && let Some(owner) = idx(kind)
                {
                    plan.own(native, field.name(), owner)?;
                }
            }
            if let Some(qualifications) = idx(TypeId::of::<analysis::native::NativeQualification>())
            {
                plan.own(qualifications, "premise", native)?;
            }
        }
        if let (Some(qualifications), Some(coverage)) = (
            alias(TypeId::of::<assertion::AssertionQualification>()),
            idx(TypeId::of::<attribution::ProviderCoverage>()),
        ) {
            plan.pairs(idx(TypeId::of::<assertion::AssertionQualification>()).unwrap(),coverage,format!("SELECT q.id AS source_id,c.id AS target_id FROM {qualifications} q JOIN {} c ON c.scope=q.scope AND c.context=q.context WHERE c.family IN ({},{},{})",identifier(&tables[coverage].alias),attribution::FactFamily::Flow.code(),attribution::FactFamily::Signatures.code(),attribution::FactFamily::Syntax.code()))?;
        }
        if let (Some(coverage), Some(scopes), Some(modules)) = (
            idx(TypeId::of::<attribution::ProviderCoverage>()),
            alias(TypeId::of::<source::CoverageScope>()),
            alias(TypeId::of::<source::Module>()),
        ) {
            plan.pairs(source,coverage,format!("SELECT a.id AS source_id,c.id AS target_id FROM {artifacts} a LEFT JOIN {modules} m ON m.source=a.id JOIN {scopes} s ON s.input_input=a.input OR s.artifact_artifact=a.id OR s.module_module=m.id JOIN {} c ON c.scope=s.id WHERE c.family IN ({},{},{},{})",identifier(&tables[coverage].alias),attribution::FactFamily::Flow.code(),attribution::FactFamily::Signatures.code(),attribution::FactFamily::Syntax.code(),attribution::FactFamily::Types.code()))?;
        }
        // Field receiver reads and guard operands use independently attributed read occurrences at the
        // exact syntax coordinate, including all candidates so ambiguity cannot become uniqueness.
        plan.pairs(occurrences,occurrences,format!("SELECT o.id AS source_id,r.id AS target_id FROM {occurrences_alias} o JOIN {occurrences_alias} r ON r.source=o.source AND r.start=o.start AND r.end=o.end AND r.syntax_kind=o.syntax_kind AND r.structural_path=o.structural_path WHERE r.role={}",source::OccurrenceRole::Read.code()))?;
        let edges = plan.prepare(session, budget).await?;
        let mut charge = charged::StateCharge::new(budget, "local-source-scope-descriptors");
        charge.grow(
            inputs.capacity() * size_of::<ValidationInput>()
                + tables.capacity() * size_of::<ClosureTable>()
                + tables
                    .iter()
                    .map(|table| table.alias.capacity())
                    .sum::<usize>(),
        )?;
        Ok(Self {
            inputs,
            edges,
            source,
            tables,
            _charge: charge,
        })
    }
    pub(super) fn root_sql(
        &self,
        access: &CompletedInputs,
        input: Id<input::InputRevision>,
        context: Id<attribution::AnalysisContext>,
    ) -> Result<String, ModelError> {
        let table = |kind: TypeId| -> Result<String, ModelError> {
            let input = self
                .inputs
                .iter()
                .find(|input| input.type_id() == kind)
                .ok_or_else(|| {
                    ModelError::Invalid("Local source coordinate input absent".into())
                })?;
            Ok(identifier(&access.table_for(input)?))
        };
        let qualifications = table(TypeId::of::<assertion::AssertionQualification>())?;
        let occurrences = table(TypeId::of::<source::Occurrence>())?;
        let artifacts = table(TypeId::of::<source::SourceArtifact>())?;
        let mut domains = Vec::new();
        for (kind, field) in [
            (TypeId::of::<flow::FlowValueObservation>(), "sink"),
            (TypeId::of::<flow::FlowTestLeafObservation>(), "test"),
            (
                TypeId::of::<flow::FlowAttributeLoadObservation>(),
                "occurrence",
            ),
            (TypeId::of::<types::TypeObservation>(), "subject"),
            (
                TypeId::of::<normalized::symbolic_fields::SourceFieldStore>(),
                "target",
            ),
        ] {
            let rows = table(kind)?;
            domains.push(format!("SELECT r.{field} AS occurrence,q.context FROM {rows} r JOIN {qualifications} q ON q.id=r.qualification"));
        }
        let use_observations = table(TypeId::of::<flow::FlowUseObservation>())?;
        let uses = table(TypeId::of::<flow::FlowUse>())?;
        domains.push(format!("SELECT u.occurrence,q.context FROM {use_observations} r JOIN {uses} u ON u.id=r.use_ JOIN {qualifications} q ON q.id=r.qualification"));
        Ok(format!(
            "SELECT DISTINCT a.id FROM ({}) roots JOIN {occurrences} o ON o.id=roots.occurrence JOIN {artifacts} a ON a.id=o.source WHERE roots.context=X'{}' AND a.input=X'{}' ORDER BY a.id",
            domains.join(" UNION ALL "),
            context.hex(),
            input.hex()
        ))
    }
    pub(super) async fn source(
        &self,
        id: Id<source::SourceArtifact>,
        context: Id<attribution::AnalysisContext>,
        budget: &resources::ResourceBudget,
    ) -> Result<PreparedClosure, ModelError> {
        let index = |kind: TypeId| {
            self.inputs
                .iter()
                .position(|input| input.type_id() == kind)
                .ok_or_else(|| ModelError::Invalid("Local primary source family absent".into()))
        };
        let occurrences =
            identifier(&self.tables[index(TypeId::of::<source::Occurrence>())?].alias);
        let qualifications = identifier(
            &self.tables[index(TypeId::of::<assertion::AssertionQualification>())?].alias,
        );
        let coordinate = format!("SELECT id FROM {occurrences} WHERE source=X'{}'", id.hex());
        let qualified = format!(
            "SELECT id FROM {qualifications} WHERE context=X'{}'",
            context.hex()
        );
        let mut roots = vec![(self.source, format!("id=X'{}'", id.hex()))];
        for (kind, field) in [
            (TypeId::of::<flow::FlowValueObservation>(), "sink"),
            (TypeId::of::<flow::FlowTestLeafObservation>(), "test"),
            (
                TypeId::of::<flow::FlowAttributeLoadObservation>(),
                "occurrence",
            ),
            (TypeId::of::<types::TypeObservation>(), "subject"),
            (
                TypeId::of::<normalized::symbolic_fields::SourceFieldStore>(),
                "target",
            ),
        ] {
            roots.push((
                index(kind)?,
                format!("{field} IN ({coordinate}) AND qualification IN ({qualified})"),
            ));
        }
        let uses = identifier(&self.tables[index(TypeId::of::<flow::FlowUse>())?].alias);
        roots.push((index(TypeId::of::<flow::FlowUseObservation>())?,format!("use_ IN (SELECT id FROM {uses} WHERE occurrence IN ({coordinate})) AND qualification IN ({qualified})")));
        self.edges.grain_roots(&roots, budget).await
    }
    pub(super) async fn load(
        &self,
        access: &CompletedInputs,
        grain: &PreparedClosure,
        budget: &resources::ResourceBudget,
    ) -> Result<LocalData, ModelError> {
        let mut data = LocalData::new(budget);
        macro_rules! read{($($field:ident:$ty:ty,)*)=>{$({for(table,input)in self.inputs.iter().enumerate().filter(|(_,input)|input.type_id()==TypeId::of::<$ty>()){
   let permit=access.read_at::<$ty>(input.prefix())?;crate::consumed_rows::stream_query_at(&permit,input,grain.session(),&grain.select(table)?,|_,batch|data.visit(input.name(),batch).map(|_|())).await?;
  }})*};}
        lctx_model::entry_value_inputs!(read);
        lctx_model::local_semantic_inputs!(read);
        lctx_model::local_theory_inputs!(read);
        lctx_model::local_field_inputs!(read);
        Ok(data)
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
        let scopes = LocalScopes::from_tables(inputs.clone(), tables, &session, &budget)
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
    }
}
