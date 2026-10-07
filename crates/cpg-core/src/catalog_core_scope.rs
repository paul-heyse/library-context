//! C0 public slots retain their complete alias, class-path and constructor dependencies.
//! A referenced module or occurrence does not become a new public-slot root.
use crate::{
    consumed_rows::{ClosureTable, NominalClosure, PreparedEdges, identifier},
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

fn membership(source: TypeId, field: &str, kind: TypeId) -> bool {
    use normalized::{callable_aspects::*, callables::*, entities::*, links::*};
    let pair = |member: TypeId, owner: &str| source == member && field == owner;
    pair(TypeId::of::<PublicExposureCandidate>(), "exposure")
        || pair(TypeId::of::<SymbolEntityCandidate>(), "resolution")
        || pair(TypeId::of::<EffectiveCallableAssessment>(), "callable")
        || pair(TypeId::of::<SignatureVariant>(), "callable")
        || pair(TypeId::of::<SignatureSlot>(), "variant")
        || pair(TypeId::of::<SignatureSlotEntity>(), "slot")
        || pair(TypeId::of::<SignatureSlotType>(), "slot")
        || pair(TypeId::of::<SignatureReturnType>(), "variant")
        || pair(TypeId::of::<CallableAspect>(), "assessment")
        || pair(TypeId::of::<FieldEntity>(), "class")
        || pair(TypeId::of::<FieldEntityLink>(), "field")
        || pair(TypeId::of::<FieldDeclarationLink>(), "field")
        || pair(TypeId::of::<FieldDefaultAssessment>(), "declaration")
        || pair(TypeId::of::<ParameterEntityLink>(), "entity")
        || pair(
            TypeId::of::<syntax::ParameterSyntaxObservation>(),
            "function",
        )
        || pair(
            TypeId::of::<syntax::ParameterSyntaxObservation>(),
            "parameter",
        )
        || pair(
            TypeId::of::<syntax::DeclarationObservation>(),
            "declaration",
        )
        || pair(TypeId::of::<syntax::DeclarationObservation>(), "parent")
        || pair(TypeId::of::<OccurrenceOwnership>(), "occurrence")
        || pair(TypeId::of::<lexical::BindingEvent>(), "site")
        || pair(TypeId::of::<lexical::BindingObservation>(), "event")
        || pair(TypeId::of::<lexical::ReferenceObservation>(), "read")
        || pair(TypeId::of::<ReferenceEntityAssessment>(), "reference")
        || pair(TypeId::of::<ReferenceEntityCandidate>(), "assessment")
        || pair(TypeId::of::<AncestryEntityAssessment>(), "class")
        || pair(TypeId::of::<AncestryEntityMember>(), "assessment")
        || pair(TypeId::of::<SymbolEntityResolution>(), "entity")
        || pair(TypeId::of::<SymbolEntityResolution>(), "symbol")
        || pair(TypeId::of::<symbols::FunctionTraitObservation>(), "symbol")
        || pair(
            TypeId::of::<symbols::FunctionTraitObservation>(),
            "defining_class",
        )
        || pair(TypeId::of::<symbols::ClassTraitObservation>(), "symbol")
        || pair(
            TypeId::of::<class_metadata::ClassMetadataObservation>(),
            "class",
        )
        || pair(
            TypeId::of::<class_metadata::ClassMemberObservation>(),
            "class",
        )
        || (kind == TypeId::of::<source::Occurrence>()
            && [
                TypeId::of::<CallableEntity>(),
                TypeId::of::<ClassEntity>(),
                TypeId::of::<ParameterEntity>(),
            ]
            .contains(&source))
        || (source == TypeId::of::<EntityRef>()
            && [
                TypeId::of::<CallableEntity>(),
                TypeId::of::<ClassEntity>(),
                TypeId::of::<ParameterEntity>(),
            ]
            .contains(&kind))
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
        use normalized::entities::*;
        let names = typed::<symbols::PublicNameObservation>(&inputs)?;
        let exposures = typed::<PublicExposure>(&inputs)?;
        let mut bindings = tables.clone();
        let root = bindings.len();
        bindings.push(tables[names].clone());
        let mut plan = NominalClosure::new(bindings)?;
        plan.pairs(
            root,
            names,
            format!(
                "SELECT id AS source_id,id AS target_id FROM {}",
                identifier(&tables[names].alias)
            ),
        )?;
        plan.pairs(
            root,
            exposures,
            format!(
                "SELECT observation AS source_id,id AS target_id FROM {}",
                identifier(&tables[exposures].alias)
            ),
        )?;
        let field_bindings = CatalogData::scoped_field_bindings(model, &inputs)?;
        for (source, table) in tables.iter().enumerate() {
            for (field_index, field) in table.relation.fields().iter().enumerate() {
                let Some((kind, _)) = field.target() else {
                    continue;
                };
                let Some(to) = field_bindings[source][field_index] else {
                    continue;
                };
                let values = if field.list() {
                    format!("UNNEST({})", identifier(field.name()))
                } else {
                    identifier(field.name())
                };
                plan.pairs(
                    source,
                    to,
                    format!(
                        "SELECT id AS source_id,{values} AS target_id FROM {}",
                        identifier(&table.alias)
                    ),
                )?;
                if !field.list() && membership(table.relation.type_id(), field.name(), kind) {
                    plan.pairs(
                        to,
                        source,
                        format!(
                            "SELECT {} AS source_id,id AS target_id FROM {}",
                            identifier(field.name()),
                            identifier(&table.alias)
                        ),
                    )?;
                }
            }
        }
        // Alias slots select assignments by their exact module access and exported spelling.
        // Module ownership never pulls every unrelated assignment into a public slot.
        let scopes = typed::<lexical::LexicalScope>(&inputs)?;
        let bindings = typed::<lexical::BindingObservation>(&inputs)?;
        let events = typed::<lexical::BindingEvent>(&inputs)?;
        let ownership = typed::<OccurrenceOwnership>(&inputs)?;
        let refs = typed::<EntityRef>(&inputs)?;
        let origins = typed::<symbols::ExportOrigin>(&inputs)?;
        let provider_modules = typed::<calls::ProviderModule>(&inputs)?;
        let table = |index: usize| identifier(&tables[index].alias);
        plan.pairs(root,bindings,format!("SELECT n.id AS source_id,b.id AS target_id FROM {} n JOIN {} e ON e.observation=n.id LEFT JOIN {} origin ON origin.id=n.origin LEFT JOIN {} pm ON pm.id=origin.traced_module JOIN {} r ON r.module_module=COALESCE(pm.acquired_module,e.access) JOIN {} own ON own.entity=r.id JOIN {} s ON s.owner=own.owner AND s.kind={} JOIN {} ev ON ev.site=own.occurrence AND ev.name=COALESCE(origin.traced_name,n.name) JOIN {} b ON b.event=ev.id AND b.scope=s.id WHERE (origin.traced_module IS NULL OR pm.acquired_module IS NOT NULL) AND b.kind={} AND b.static_branch IS NULL",table(names),table(exposures),table(origins),table(provider_modules),table(refs),table(ownership),table(scopes),lexical::LexicalScopeKind::Module as i16,table(events),table(bindings),lexical::BindingEventKind::Assignment as i16))?;
        // A class's complete binding domain includes non-declaration rebinding evidence.
        let occurrences = typed::<source::Occurrence>(&inputs)?;
        plan.pairs(
            occurrences,
            scopes,
            format!(
                "SELECT owner AS source_id,id AS target_id FROM {} WHERE kind={}",
                table(scopes),
                lexical::LexicalScopeKind::Class as i16
            ),
        )?;
        plan.pairs(scopes,bindings,format!("SELECT s.id AS source_id,b.id AS target_id FROM {} s JOIN {} b ON b.scope=s.id WHERE s.kind={}",table(scopes),table(bindings),lexical::LexicalScopeKind::Class as i16))?;
        // Formal parameter placements are necessary; function body placements are not.
        let placements = typed::<syntax::SyntaxPlacement>(&inputs)?;
        plan.pairs(occurrences,placements,format!("SELECT p.parent AS source_id,p.id AS target_id FROM {} p JOIN {} o ON o.id=p.occurrence WHERE o.syntax_kind={}",table(placements),table(occurrences),source::SyntaxKind::Parameter as i16))?;
        // Signature completeness uses exact scope/context coverage rather than every run row.
        let qualifications = typed::<assertion::AssertionQualification>(&inputs)?;
        let coverage = typed::<attribution::ProviderCoverage>(&inputs)?;
        plan.pairs(qualifications,coverage,format!("SELECT q.id AS source_id,c.id AS target_id FROM {} q JOIN {} c ON c.scope=q.scope AND c.context=q.context AND c.family={}",table(qualifications),table(coverage),attribution::FactFamily::Signatures as i16))?;
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
