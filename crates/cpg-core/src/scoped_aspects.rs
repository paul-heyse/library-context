//! Callable metadata selects typed owner namespaces before rich decoding.
use crate::{
    consumed_rows::{ClosureTable, NominalClosure, PreparedEdges, identifier},
    scoped_admission::{column, field_target, root_predicate},
    workspace::Cancellation,
};
use datafusion::prelude::SessionContext;
use futures::TryStreamExt;
use lctx_model::domain::{
    normalized::callable_aspects::{self, AspectData, AspectKernel, AspectOutput, AspectScope},
    resources::ResourceBudget,
    *,
};
use std::any::TypeId;

fn typed<R: Record>(inputs: &[ValidationInput]) -> Result<usize, ModelError> {
    let mut choices = inputs
        .iter()
        .enumerate()
        .filter(|(_, input)| input.type_id() == TypeId::of::<R>());
    let (index, _) = choices
        .next()
        .ok_or(ModelError::Schema("aspect scope relation absent"))?;
    if choices.next().is_some() {
        return Err(ModelError::Conflict("aspect scope immutable binding"));
    }
    Ok(index)
}
fn bound(inputs: &[ValidationInput], input: &ValidationInput) -> Option<usize> {
    inputs.iter().position(|candidate| {
        candidate.type_id() == input.type_id() && candidate.prefix() == input.prefix()
    })
}
pub(crate) struct AspectScopes {
    pub inputs: Vec<ValidationInput>,
    pub roots: [usize; 3],
    pub root_tables: [String; 3],
    pub admission_roots: Vec<(usize, String)>,
    pub edges: PreparedEdges,
}
impl AspectScopes {
    pub async fn prepare(
        inputs: Vec<ValidationInput>,
        tables: Vec<ClosureTable>,
        scope: &AspectScope,
        model: &ValidatedModel,
        session: &SessionContext,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        use lctx_model::domain::{
            normalized::{callables::*, entities::*},
            source::Occurrence,
            syntax::*,
        };
        let real_roots: Vec<_> = scope
            .roots
            .iter()
            .map(|input| {
                bound(&inputs, input).ok_or(ModelError::Conflict("aspect root immutable epoch"))
            })
            .collect::<Result<_, _>>()?;
        let occurrence = typed::<Occurrence>(&inputs)?;
        let qualification = typed::<assertion::AssertionQualification>(&inputs)?;
        let mut bindings = tables.clone();
        let roots: [usize; 3] = std::array::from_fn(|index| {
            let root = bindings.len();
            bindings.push(tables[real_roots[index]].clone());
            root
        });
        let body = bindings.len();
        bindings.push(tables[occurrence].clone());
        let mut plan = NominalClosure::new(bindings)?;
        for (virtual_root, real_root) in roots.iter().zip(&real_roots) {
            plan.pairs(
                *virtual_root,
                *real_root,
                format!(
                    "SELECT id AS source_id,id AS target_id FROM {}",
                    identifier(&tables[*real_root].alias)
                ),
            )?;
        }
        plan.pairs(
            body,
            occurrence,
            format!(
                "SELECT id AS source_id,id AS target_id FROM {}",
                identifier(&tables[occurrence].alias)
            ),
        )?;
        let table = |id: usize| identifier(&tables[id].alias);
        let occurrence_table = table(occurrence);
        let assessment = real_roots[0];
        let field = real_roots[1];
        let declaration = real_roots[2];
        let callable = typed::<CallableEntity>(&inputs)?;
        let field_syntax = typed::<ClassFieldSyntaxObservation>(&inputs)?;
        let member = typed::<EffectiveDecoratorMember>(&inputs)?;
        let decorator = typed::<DeclarationDecorator>(&inputs)?;
        let span = |owners: String| {
            format!(
                "SELECT owner.root_id AS source_id,child.id AS target_id FROM ({owners}) owner JOIN {occurrence_table} parent ON parent.id=owner.occurrence JOIN {occurrence_table} child ON child.source=parent.source AND child.start>=parent.start AND child.\"end\"<=parent.\"end\" AND array_slice(child.structural_path,1,CAST(array_length(parent.structural_path) AS BIGINT))=parent.structural_path"
            )
        };
        plan.pairs(roots[0],body,span(format!("SELECT a.id AS root_id,c.source_declaration AS occurrence FROM {} a JOIN {} c ON c.id=a.callable UNION SELECT m.assessment AS root_id,d.decorator AS occurrence FROM {} m JOIN {} d ON d.id=m.observation",table(assessment),table(callable),table(member),table(decorator))))?;
        plan.pairs(roots[1],body,span(format!("SELECT f.id AS root_id,s.value AS occurrence FROM {} f JOIN {} s ON s.id=f.declaration UNION SELECT f.id AS root_id,s.target AS occurrence FROM {} f JOIN {} s ON s.id=f.declaration UNION SELECT f.id AS root_id,s.annotation AS occurrence FROM {} f JOIN {} s ON s.id=f.declaration",table(field),table(field_syntax),table(field),table(field_syntax),table(field),table(field_syntax))))?;
        plan.pairs(
            roots[2],
            body,
            span(format!(
                "SELECT id AS root_id,declaration AS occurrence FROM {} WHERE kind={}",
                table(declaration),
                DeclarationKind::Class as i16
            )),
        )?;
        // A class requires its actual field initializer metadata, including fields that have no
        // rich subtree of their own. Supporting class references never enter the virtual root.
        plan.pairs(roots[2],field,format!("SELECT d.id AS source_id,f.id AS target_id FROM {} d JOIN {} s ON s.class=d.declaration JOIN {} f ON f.declaration=s.id WHERE d.kind={}",table(declaration),table(field_syntax),table(field),DeclarationKind::Class as i16))?;
        for (source, row) in tables.iter().enumerate() {
            for field in row.relation.fields() {
                let Some((kind, _)) = field.target() else {
                    continue;
                };
                let Some(target) = field_target(&inputs, source, kind)? else {
                    continue;
                };
                if field.list() {
                    plan.pairs(
                        source,
                        target,
                        format!(
                            "SELECT id AS source_id,UNNEST({}) AS target_id FROM {}",
                            identifier(field.name()),
                            table(source)
                        ),
                    )?;
                } else {
                    plan.follow(source, field.name(), target)?;
                    if kind == TypeId::of::<Occurrence>() {
                        plan.pairs(
                            body,
                            source,
                            format!(
                                "SELECT {} AS source_id,id AS target_id FROM {}",
                                identifier(field.name()),
                                table(source)
                            ),
                        )?;
                    }
                }
            }
        }
        for (member, field, owner) in &scope.memberships {
            if let (Some(member), Some(owner)) = (bound(&inputs, member), bound(&inputs, owner)) {
                plan.own(member, field, owner)?;
            }
        }
        // Native support memberships come from the authoritative typed SupportScope contract.
        // Discovery allocates no rich state and runs once per prepared input set.
        for invariant in model.invariants() {
            let check = (invariant.create)(budget);
            if let Some(support) = check.support_scope()
                && let (Some(member), Some(owner)) = (
                    inputs
                        .iter()
                        .position(|input| input.type_id() == support.support.type_id()),
                    inputs
                        .iter()
                        .position(|input| input.type_id() == support.assertion.type_id()),
                ) {
                    plan.own(member, "assertion", owner)?;
            }
        }
        if let Ok(coverage) = typed::<attribution::ProviderCoverage>(&inputs) {
            plan.pairs(qualification,coverage,format!("SELECT q.id AS source_id,c.id AS target_id FROM {} q JOIN {} c ON c.scope=q.scope AND c.context=q.context",table(qualification),table(coverage)))?;
        }
        // Advertised rows root their real source namespace when it exists. If it does not,
        // the row still runs the necessary predicate and is refused; it cannot vanish from work.
        use lctx_model::domain::normalized::symbolic_fields::*;
        let optional = |kind| inputs.iter().position(|input| input.type_id() == kind);
        if let Some(index) = optional(TypeId::of::<callable_aspects::CallableAspect>()) {
            plan.pairs(
                index,
                roots[0],
                format!(
                    "SELECT id AS source_id,assessment AS target_id FROM {}",
                    table(index)
                ),
            )?;
        }
        if let Some(index) = optional(TypeId::of::<callable_aspects::FieldDefaultAssessment>()) {
            plan.pairs(
                index,
                roots[1],
                format!(
                    "SELECT id AS source_id,declaration AS target_id FROM {}",
                    table(index)
                ),
            )?;
        }
        for kind in [
            TypeId::of::<SourceFieldClass>(),
            TypeId::of::<SourceFieldStore>(),
            TypeId::of::<SourceFieldReader>(),
        ] {
            if let Some(index) = optional(kind) {
                plan.pairs(index,roots[2],format!("SELECT a.id AS source_id,d.id AS target_id FROM {} a JOIN {} d ON d.declaration=a.class WHERE d.kind={}",table(index),table(declaration),DeclarationKind::Class as i16))?;
            }
        }
        if let (Some(index), Some(class)) = (
            optional(TypeId::of::<SourceFieldAssociation>()),
            optional(TypeId::of::<SourceFieldClass>()),
        ) {
            plan.pairs(index,roots[2],format!("SELECT a.id AS source_id,d.id AS target_id FROM {} a JOIN {} c ON c.id=a.class JOIN {} d ON d.declaration=c.class WHERE d.kind={}",table(index),table(class),table(declaration),DeclarationKind::Class as i16))?;
        }
        if let (Some(index), Some(association), Some(class)) = (
            optional(TypeId::of::<SourceFieldReaderLink>()),
            optional(TypeId::of::<SourceFieldAssociation>()),
            optional(TypeId::of::<SourceFieldClass>()),
        ) {
            plan.pairs(index,roots[2],format!("SELECT l.id AS source_id,d.id AS target_id FROM {} l JOIN {} a ON a.id=l.association JOIN {} c ON c.id=a.class JOIN {} d ON d.declaration=c.class WHERE d.kind={}",table(index),table(association),table(class),table(declaration),DeclarationKind::Class as i16))?;
        }
        let admission_roots = scope
            .admission_roots
            .iter()
            .filter_map(|input| {
                bound(&inputs, input).map(|index| (index, tables[index].alias.clone()))
            })
            .collect();
        let roots_tables = std::array::from_fn(|index| tables[real_roots[index]].alias.clone());
        Ok(Self {
            inputs,
            roots,
            root_tables: roots_tables,
            admission_roots,
            edges: plan.prepare(session, budget).await?,
        })
    }
}

pub(crate) async fn load_data(
    scoped: &crate::consumed_rows::PreparedClosure,
    inputs: &[ValidationInput],
    budget: &ResourceBudget,
) -> Result<(AspectData, AspectOutput), ModelError> {
    let mut data = AspectData::new(budget);
    let mut out = AspectOutput::new(budget);
    for (index, input) in inputs.iter().enumerate() {
        let order = input
            .order()
            .iter()
            .map(|field| identifier(field))
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT * FROM ({}) selected{}",
            scoped.select(index)?,
            if order.is_empty() {
                String::new()
            } else {
                format!(" ORDER BY {order}")
            }
        );
        let mut rows = crate::sql::query(scoped.session(), &sql)
            .await
            .map_err(ModelError::codec)?
            .execute_stream()
            .await
            .map_err(ModelError::codec)?;
        while let Some(batch) = rows.try_next().await.map_err(ModelError::codec)? {
            if !data.visit(input.name(), &batch)? && !out.visit(input.name(), &batch)? {
                return Err(ModelError::Schema("aspect scoped input not declared"));
            }
        }
    }
    Ok((data, out))
}
pub(crate) fn kernel(index: usize, id: [u8; 16]) -> Result<AspectKernel, ModelError> {
    let nominal = serde_json::to_value(id).map_err(ModelError::codec)?;
    Ok(match index {
        0 => AspectKernel::Assessment(serde_json::from_value(nominal).map_err(ModelError::codec)?),
        1 => AspectKernel::Field(serde_json::from_value(nominal).map_err(ModelError::codec)?),
        2 => AspectKernel::Class(serde_json::from_value(nominal).map_err(ModelError::codec)?),
        _ => return Err(ModelError::Schema("aspect kernel root")),
    })
}
pub(crate) async fn validate_aspects(
    invariant: &Invariant,
    scope: &AspectScope,
    tables: Vec<ClosureTable>,
    model: &ValidatedModel,
    session: &SessionContext,
    budget: &ResourceBudget,
    cancellation: &Cancellation,
) -> Result<(), ModelError> {
    let prepared = AspectScopes::prepare(
        invariant.inputs.clone(),
        tables,
        scope,
        model,
        session,
        budget,
    )
    .await?;
    let roots = prepared
        .roots
        .iter()
        .enumerate()
        .map(|(index, root)| (*root, prepared.root_tables[index].clone(), index == 2))
        .chain(
            prepared
                .admission_roots
                .iter()
                .map(|(root, table)| (*root, table.clone(), false)),
        )
        .collect::<Vec<_>>();
    for (root, table, class_only) in roots {
        let filter = if class_only {
            format!(" WHERE kind={}", syntax::DeclarationKind::Class as i16)
        } else {
            String::new()
        };
        let sql = format!("SELECT id FROM {}{filter} ORDER BY id", identifier(&table));
        let mut roots = crate::sql::query(session, &sql)
            .await
            .map_err(ModelError::codec)?
            .execute_stream()
            .await
            .map_err(ModelError::codec)?;
        while let Some(batch) = roots.try_next().await.map_err(ModelError::codec)? {
            for row in 0..batch.num_rows() {
                cancellation.check()?;
                let id =
                    column(&batch, "id", row)?.ok_or(ModelError::Schema("aspect owner root ID"))?;
                let scoped = prepared
                    .edges
                    .grain(root, &root_predicate(&[id]), budget)
                    .await?;
                let mut check = (invariant.create)(budget);
                for (input_index, input) in invariant.inputs.iter().enumerate() {
                    let mut rows =
                        crate::sql::query(scoped.session(), &scoped.select(input_index)?)
                            .await
                            .map_err(ModelError::codec)?
                            .execute_stream()
                            .await
                            .map_err(ModelError::codec)?;
                    while let Some(batch) = rows.try_next().await.map_err(ModelError::codec)? {
                        cancellation.check()?;
                        check.visit_input(input, &batch)?;
                    }
                }
                check.finish()?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod controls {
    use super::*;
    use datafusion::{datasource::MemTable, prelude::SessionConfig};
    use lctx_model::domain::{
        assertion::{Approximation, AssertionQualification},
        attribution::Modality,
        normalized::{callables::*, entities::*, symbolic_fields::*},
        source::{Occurrence, OccurrenceRole, SyntaxKind, SyntaxObservation},
        syntax::{ClassFieldSyntaxObservation, DeclarationDecorator},
    };
    use std::sync::Arc;
    fn nominal<R>(byte: u8) -> Id<R> {
        serde_json::from_value(serde_json::to_value([byte; 16]).unwrap()).unwrap()
    }
    struct Fixture {
        model: ValidatedModel,
        invariant: Invariant,
        scope: AspectScope,
        tables: Vec<ClosureTable>,
        session: SessionContext,
        budget: ResourceBudget,
        data: AspectData,
        out: AspectOutput,
    }
    impl Fixture {
        fn put<R: Record>(&self, rows: &[R]) {
            let table = self
                .tables
                .iter()
                .find(|table| table.relation.type_id() == TypeId::of::<R>())
                .unwrap();
            self.session.deregister_table(table.alias.as_str()).unwrap();
            let batch = R::encode(rows).unwrap();
            self.session
                .register_table(
                    table.alias.as_str(),
                    Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap()),
                )
                .unwrap();
        }
        fn new() -> Self {
            let model = lctx_model::domain::model().unwrap();
            let invariant = model
                .invariant("normalized_callable_metadata_admission")
                .unwrap()
                .clone();
            let budget = ResourceBudget::fixed(128 << 20).unwrap();
            let scope = (invariant.create)(&budget).aspect_scope().unwrap();
            let session =
                SessionContext::new_with_config(SessionConfig::new().with_target_partitions(1));
            let tables = invariant
                .inputs
                .iter()
                .enumerate()
                .map(|(index, input)| ClosureTable {
                    relation: model.relation(input.name()).unwrap().clone(),
                    alias: format!("aspect_input_{index}"),
                })
                .collect::<Vec<_>>();
            for table in &tables {
                let batch = arrow_array::RecordBatch::new_empty(table.relation.schema().clone());
                session
                    .register_table(
                        table.alias.as_str(),
                        Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap()),
                    )
                    .unwrap();
            }
            let mut data = AspectData::new(&budget);
            let occurrence = |start, end, kind, path| Occurrence {
                source: nominal(1),
                start,
                end,
                syntax_kind: kind,
                role: OccurrenceRole::Syntax,
                structural_path: path,
            };
            let owner = occurrence(0, 10, SyntaxKind::ExprName, vec![0]);
            let target = occurrence(1, 2, SyntaxKind::ExprName, vec![0, 1]);
            let value = occurrence(3, 4, SyntaxKind::ExprName, vec![0, 2]);
            let sibling = occurrence(20, 30, SyntaxKind::ExprName, vec![1]);
            let q = AssertionQualification {
                context: nominal(2),
                scope: nominal(3),
                condition: conditions::Diagram::always().id(),
                modality: Modality::Definite,
                approximation: Approximation::Exact,
                assumptions: nominal(4),
            };
            data.qualifications.insert(q.clone()).unwrap();
            for row in [&owner, &target, &value, &sibling] {
                data.occurrences.insert(row.clone()).unwrap();
            }
            // A real rich row in the same source/context must be excluded before model decoding.
            data.spellings
                .insert(SyntaxObservation {
                    qualification: q.id(),
                    occurrence: sibling.id(),
                    spelling: "x".repeat(2 << 20),
                })
                .unwrap();
            for (byte, value) in [(5, Some(value.id())), (6, None)] {
                let syntax = ClassFieldSyntaxObservation {
                    qualification: q.id(),
                    class: owner.id(),
                    target: target.id(),
                    annotation: Some(target.id()),
                    value,
                };
                data.field_syntax.insert(syntax.clone()).unwrap();
                data.fields
                    .insert(FieldDeclarationLink {
                        field: nominal(byte),
                        declaration: syntax.id(),
                        binding: nominal(byte),
                    })
                    .unwrap();
            }
            let callable = CallableEntity::Source {
                declaration: owner.id(),
                kind: CallableKind::Function,
            };
            data.callable_entities.insert(callable.clone()).unwrap();
            let decorator = DeclarationDecorator {
                qualification: q.id(),
                declaration: owner.id(),
                decorator: value.id(),
                ordinal: 0,
            };
            data.decorators.insert(decorator.clone()).unwrap();
            for byte in [7, 8] {
                let assessment = EffectiveCallableAssessment {
                    callable: callable.id(),
                    context: q.context,
                    decorators: ContentHash::of(&[byte]),
                    policy: ContentHash::of(b"control"),
                    identity: Knowledge::Unknown,
                    identity_reason: CallableReason::UnsupportedDecorator,
                    signatures: Knowledge::Unknown,
                    signature_reason: CallableReason::MissingSignature,
                    descriptor: Knowledge::Unknown,
                    descriptor_kind: None,
                    descriptor_reason: CallableReason::MissingTraits,
                    body: Knowledge::Unknown,
                    body_admitted: false,
                    body_reason: CallableReason::BodyExcluded,
                    asynchronous: None,
                    generator: None,
                };
                data.assessments.insert(assessment.clone()).unwrap();
                data.members
                    .insert(EffectiveDecoratorMember {
                        assessment: assessment.id(),
                        observation: decorator.id(),
                        source_ordinal: 0,
                        application_ordinal: 0,
                    })
                    .unwrap();
            }
            let out = callable_aspects::normalize(&data, &budget).unwrap();
            let fixture = Self {
                model,
                invariant,
                scope,
                tables,
                session,
                budget,
                data,
                out,
            };
            macro_rules! put {($($field:ident:$ty:ty,)*)=>{$(fixture.put(&fixture.data.$field.iter().cloned().collect::<Vec<_>>());)*};}
            lctx_model::callable_aspect_inputs!(put);
            macro_rules! put {($($field:ident:$ty:ty,)*)=>{$(fixture.put(&fixture.out.$field.iter().cloned().collect::<Vec<_>>());)*};}
            lctx_model::callable_aspect_outputs!(put);
            fixture
        }
        async fn validate(&self) -> Result<(), ModelError> {
            validate_aspects(
                &self.invariant,
                &self.scope,
                self.tables.clone(),
                &self.model,
                &self.session,
                &self.budget,
                &Cancellation::default(),
            )
            .await
        }
    }
    #[tokio::test]
    async fn physical_owner_kernels_match_actual_oracle_and_exclude_unrelated_rich_rows() {
        let fixture = Fixture::new();
        let baseline = fixture.budget.reserved();
        let prepared = AspectScopes::prepare(
            fixture.invariant.inputs.clone(),
            fixture.tables.clone(),
            &fixture.scope,
            &fixture.model,
            &fixture.session,
            &fixture.budget,
        )
        .await
        .unwrap();
        let mut actual = AspectOutput::new(&fixture.budget);
        for (index, ids) in [
            (
                0,
                fixture
                    .data
                    .assessments
                    .iter()
                    .map(|row| *row.id().bytes())
                    .collect::<Vec<_>>(),
            ),
            (
                1,
                fixture
                    .data
                    .fields
                    .iter()
                    .map(|row| *row.id().bytes())
                    .collect::<Vec<_>>(),
            ),
        ] {
            for id in ids {
                let grain = prepared
                    .edges
                    .grain(
                        prepared.roots[index],
                        &root_predicate(&[id]),
                        &fixture.budget,
                    )
                    .await
                    .unwrap();
                let (data, prior) = load_data(&grain, &prepared.inputs, &fixture.budget)
                    .await
                    .unwrap();
                assert!(data.spellings.is_empty());
                assert!(data.occurrences.len() < fixture.data.occurrences.len());
                let rows = callable_aspects::normalize_scope(
                    &data,
                    kernel(index, id).unwrap(),
                    &fixture.budget,
                )
                .unwrap();
                macro_rules! merge {($($field:ident:$ty:ty,)*)=>{$(for row in rows.$field.iter(){actual.$field.insert(row.clone()).unwrap();})*};}
                lctx_model::callable_aspect_outputs!(merge);
                drop(prior);
                drop(data);
                drop(grain);
            }
        }
        actual.matches(&fixture.out).unwrap();
        drop(actual);
        drop(prepared);
        assert_eq!(fixture.budget.reserved(), baseline);
        fixture.validate().await.unwrap();
        assert_eq!(fixture.budget.reserved(), baseline);
    }
    #[tokio::test]
    async fn redirected_existing_default_foreign_aspect_and_unsupported_class_are_refused() {
        let fixture = Fixture::new();
        let baseline = fixture.budget.reserved();
        fixture.validate().await.unwrap();
        let mut fields = fixture.out.fields.iter().cloned().collect::<Vec<_>>();
        let absent = callable_aspects::FieldDefault::Absent {}.id();
        fields
            .iter_mut()
            .find(|row| row.default != absent)
            .unwrap()
            .default = absent;
        fixture.put(&fields);
        assert!(fixture.validate().await.is_err());
        assert_eq!(fixture.budget.reserved(), baseline);
        fixture.put(&fixture.out.fields.iter().cloned().collect::<Vec<_>>());
        let mut aspects = fixture.out.aspects.iter().cloned().collect::<Vec<_>>();
        let other = fixture
            .data
            .assessments
            .iter()
            .find(|row| row.id() != aspects[0].assessment)
            .unwrap()
            .id();
        aspects[0].assessment = other;
        fixture.put(&aspects);
        assert!(fixture.validate().await.is_err());
        assert_eq!(fixture.budget.reserved(), baseline);
        fixture.put(&fixture.out.aspects.iter().cloned().collect::<Vec<_>>());
        let class = SourceFieldClass {
            class: fixture
                .data
                .callable_entities
                .iter()
                .find_map(|row| match row {
                    CallableEntity::Source { declaration, .. } => Some(*declaration),
                    _ => None,
                })
                .unwrap(),
            qualification: fixture.data.qualifications.iter().next().unwrap().id(),
            traits: nominal(20),
            support: nominal(21),
            supported_record: true,
            reason: None,
            inventory: ContentHash::of(b"invented"),
        };
        fixture.put(&[class]);
        assert!(fixture.validate().await.is_err());
        assert_eq!(fixture.budget.reserved(), baseline);
    }
}
