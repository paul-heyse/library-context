//! Callable metadata selects typed owner namespaces before rich decoding.
use crate::{
    consumed_rows::{ClosureTable, PreparedEdges, identifier},
    scoped_admission::column,
    workspace::Cancellation,
};
use datafusion::prelude::SessionContext;
use futures::TryStreamExt;
use lctx_model::domain::{
    normalized::callable_aspects::{self, AspectData, AspectKernel, AspectOutput, AspectScope},
    resources::ResourceBudget,
    *,
};

pub(crate) struct AspectScopes {
    pub inputs: Vec<ValidationInput>,
    pub roots: [usize; 3],
    pub root_tables: [String; 3],
    pub admission_roots: Vec<(usize, String)>,
    pub edges: PreparedEdges,
    inventories: [std::sync::Arc<scope_program::CompiledScopeProgram>; 3],
    bindings: Vec<ClosureTable>,
    _charge: charged::StateCharge,
    _program: std::sync::Arc<lctx_model::domain::scope_program::CompiledScopeProgram>,
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
        Self::prepare_in(inputs, tables, scope, model, session, budget, None).await
    }
    pub async fn prepare_in(
        inputs: Vec<ValidationInput>,
        tables: Vec<ClosureTable>,
        scope: &AspectScope,
        model: &ValidatedModel,
        session: &SessionContext,
        budget: &ResourceBudget,
        programs: Option<&std::sync::Mutex<lctx_model::domain::scope_program::ScopeInterner>>,
    ) -> Result<Self, ModelError> {
        let mut charge = charged::StateCharge::new(budget, "aspect-prepared-bindings");
        charge.grow(inputs.len() * 2048 + 4096)?;
        let inventories = (0..3)
            .map(|index| {
                let program =
                    normalized::aspect_program::root_inventory(scope, inputs.clone(), index)?;
                crate::scope_compilation::compile(&program, model, budget, programs)
            })
            .collect::<Result<Vec<_>, ModelError>>()?
            .try_into()
            .map_err(|_| ModelError::Schema("aspect root inventory count"))?;
        let semantic =
            lctx_model::domain::normalized::aspect_program::build(scope, inputs, model, budget)?;
        let roots = semantic.roots;
        let root_tables =
            std::array::from_fn(|i| tables[semantic.program.ports[roots[i]].input].alias.clone());
        let admission_roots = semantic
            .admission_roots
            .iter()
            .map(|&p| (p, tables[semantic.program.ports[p].input].alias.clone()))
            .collect();
        let inputs = semantic.program.inputs.clone();
        let compiled = if let Some(programs) = programs {
            programs
                .lock()
                .map_err(|_| ModelError::Conflict("scope interner poisoned"))?
                .intern(semantic.program, model)?
        } else {
            lctx_model::domain::scope_program::ScopeInterner::new(budget)?
                .intern(semantic.program, model)?
        };
        let plan = crate::scope_compilation::lower_compiled(
            compiled.clone(),
            &tables,
            &scope_program::ScopeParameters(vec![]),
            budget,
        )?;
        Ok(Self {
            inputs,
            roots,
            root_tables,
            admission_roots,
            edges: plan.prepare(session, budget).await?,
            inventories,
            bindings: tables,
            _charge: charge,
            _program: compiled,
        })
    }
    pub(crate) fn inventory_sql(
        &self,
        index: usize,
        budget: &ResourceBudget,
    ) -> Result<(String, Box<dyn resources::Reservation>), ModelError> {
        let program = self
            .inventories
            .get(index)
            .ok_or(ModelError::Schema("aspect root inventory"))?;
        let charge = budget.reserve(
            "aspect-root-query",
            crate::scope_compilation::lowering_allowance(
                program.program(),
                &self.bindings,
                &scope_program::ScopeParameters(vec![]),
            ),
        )?;
        let queries = crate::scope_compilation::select_pair_queries(
            program.program(),
            &self.bindings,
            &scope_program::ScopeParameters(vec![]),
        )?;
        let query = queries
            .first()
            .ok_or(ModelError::Schema("aspect root query"))?;
        Ok((
            format!(
                "SELECT DISTINCT target_id AS id FROM ({}) roots ORDER BY id",
                query.2
            ),
            charge,
        ))
    }
}

pub(crate) async fn load_batch(
    batch: &crate::consumed_rows::PreparedRootBatch,
    inputs: &[ValidationInput],
    budget: &ResourceBudget,
    cancellation: &Cancellation,
) -> Result<(AspectData, AspectOutput), ModelError> {
    let mut data = AspectData::new(budget);
    let mut out = AspectOutput::new(budget);
    crate::scoped_batch::hydrate_union(
        batch,
        inputs,
        budget,
        cancellation,
        &mut |_, input, rows| {
            if !data.visit(input.name(), rows)? && !out.visit(input.name(), rows)? {
                return Err(ModelError::Schema("aspect batch input undeclared"));
            }
            Ok(())
        },
    )
    .await?;
    Ok((data, out))
}
macro_rules! selected_data {($($field:ident:$ty:ty,)*)=>{
    pub(crate) struct AspectSelection<'a> {$( $field:crate::scoped_batch::SelectedRows<'a,$ty>,)*}
    impl<'a> AspectSelection<'a> {
        pub(crate) fn new(batch:&crate::consumed_rows::PreparedRootBatch,partition:usize,inputs:&[ValidationInput],data:&'a AspectData,budget:&ResourceBudget)->Result<Self,ModelError>{
            Ok(Self{$($field:crate::scoped_batch::SelectedRows::new(batch,partition,inputs.iter().position(|i|i.type_id()==std::any::TypeId::of::<$ty>()).ok_or(ModelError::Schema("aspect selected input absent"))?,&data.$field,budget)?,)*})
        }
        pub(crate) fn view(&self)->Result<callable_aspects::AspectDataView<'_>,ModelError>{Ok(callable_aspects::AspectDataView{$($field:self.$field.view()?,)*})}
    }
};}
lctx_model::callable_aspect_inputs!(selected_data);
macro_rules! selected_output {($($field:ident:$ty:ty,)*)=>{
    pub(crate) struct OutputSelection<'a> {rows:&'a AspectOutput,$($field:Option<crate::scoped_batch::SelectedRows<'a,$ty>>,)*}
    impl<'a> OutputSelection<'a> {
        pub(crate) fn new(batch:&crate::consumed_rows::PreparedRootBatch,partition:usize,inputs:&[ValidationInput],rows:&'a AspectOutput,budget:&ResourceBudget)->Result<Self,ModelError>{
            Ok(Self{rows,$($field:inputs.iter().position(|i|i.type_id()==std::any::TypeId::of::<$ty>()).map(|table|crate::scoped_batch::SelectedRows::new(batch,partition,table,&rows.$field,budget)).transpose()?,)*})
        }
        pub(crate) fn view(&self)->Result<callable_aspects::AspectOutputView<'_>,ModelError>{Ok(callable_aspects::AspectOutputView{$($field:match &self.$field {Some(selected)=>selected.view()?,None=>self.rows.$field.view()},)*})}
    }
};}
lctx_model::callable_aspect_outputs!(selected_output);
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
        .map(|(index, root)| (*root, prepared.root_tables[index].clone(), Some(index)))
        .chain(
            prepared
                .admission_roots
                .iter()
                .map(|(root, table)| (*root, table.clone(), None)),
        )
        .collect::<Vec<_>>();
    for (root, table, inventory) in roots {
        let (sql, _query) = if let Some(index) = inventory {
            prepared.inventory_sql(index, budget)?
        } else {
            (
                format!("SELECT id FROM {} ORDER BY id", identifier(&table)),
                budget.reserve("aspect-advertised-root-query", table.len() + 1024)?,
            )
        };
        let mut roots = crate::sql::query(session, &sql)
            .await
            .map_err(ModelError::codec)?
            .execute_stream()
            .await
            .map_err(ModelError::codec)?;
        while let Some(batch) = roots.try_next().await.map_err(ModelError::codec)? {
            for first in (0..batch.num_rows()).step_by(32) {
                let stop = (first + 32).min(batch.num_rows());
                let _roots =
                    budget.reserve("aspect-admission-batch-roots", (stop - first) * 128)?;
                let requested = (first..stop)
                    .map(|row| {
                        Ok(crate::consumed_rows::PreparedRoot {
                            table: root,
                            key: column(&batch, "id", row)?
                                .ok_or(ModelError::Schema("aspect owner root ID"))?,
                            kind: if prepared.roots.contains(&root) {
                                crate::consumed_rows::PreparedRootKind::Virtual
                            } else {
                                crate::consumed_rows::PreparedRootKind::Physical
                            },
                        })
                    })
                    .collect::<Result<Vec<_>, ModelError>>()?;
                let selected = prepared
                    .edges
                    .batch_with_cancellation(&requested, budget, cancellation)
                    .await?;
                let (data, out) =
                    load_batch(&selected, &prepared.inputs, budget, cancellation).await?;
                for partition in 0..requested.len() {
                    cancellation.check()?;
                    let owner = AspectSelection::new(
                        &selected,
                        partition,
                        &prepared.inputs,
                        &data,
                        budget,
                    )?;
                    let advertised =
                        OutputSelection::new(&selected, partition, &prepared.inputs, &out, budget)?;
                    crate::stage_runtime::borrowed_cpu("callable_aspect_admission", || {
                        callable_aspects::admit_view(&owner.view()?, &advertised.view()?, budget)
                    })?;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod controls {
    use super::*;
    use datafusion::datasource::MemTable;
    use lctx_model::domain::{
        assertion::{Approximation, AssertionQualification},
        attribution::Modality,
        normalized::{callables::*, entities::*, symbolic_fields::*},
        source::{Occurrence, OccurrenceRole, SyntaxKind, SyntaxObservation},
        syntax::{ClassFieldSyntaxObservation, DeclarationDecorator},
    };
    use std::any::TypeId;
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
            let session = SessionContext::new();
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
    async fn batched_owner_kernels_match_actual_oracle_and_exclude_unrelated_rich_rows() {
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
        let mut requests = Vec::new();
        let mut owners = Vec::new();
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
                requests.push(crate::consumed_rows::PreparedRoot {
                    table: prepared.roots[index],
                    key: id,
                    kind: crate::consumed_rows::PreparedRootKind::Virtual,
                });
                owners.push((index, id));
            }
        }
        requests.push(requests[0]);
        owners.push(owners[0]);
        let selected = prepared
            .edges
            .batch_with_cancellation(&requests, &fixture.budget, &Cancellation::default())
            .await
            .unwrap();
        let (data, prior) = load_batch(
            &selected,
            &prepared.inputs,
            &fixture.budget,
            &Cancellation::default(),
        )
        .await
        .unwrap();
        assert!(data.spellings.is_empty());
        assert!(data.occurrences.len() < fixture.data.occurrences.len());
        let mut partition_occurrences = 0;
        for (partition, (index, id)) in owners.into_iter().enumerate() {
            let selected_data = AspectSelection::new(
                &selected,
                partition,
                &prepared.inputs,
                &data,
                &fixture.budget,
            )
            .unwrap();
            let view = selected_data.view().unwrap();
            partition_occurrences += view.occurrences.len();
            let rows = callable_aspects::normalize_scope_view(
                &view,
                kernel(index, id).unwrap(),
                &fixture.budget,
            )
            .unwrap();
            macro_rules! merge {($($field:ident:$ty:ty,)*)=>{$(for row in rows.$field.iter(){actual.$field.insert_borrowed(row).unwrap();})*};}
            lctx_model::callable_aspect_outputs!(merge);
        }
        assert!(
            data.occurrences.len() < partition_occurrences,
            "shared physical body demand is hydrated once across root partitions"
        );
        drop(prior);
        drop(data);
        drop(selected);
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
