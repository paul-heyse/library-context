//! Compact selection for assertion support; the model's support predicate owns admission.
//! Immutable nominal edges are prepared once, then rich rows live only for one bounded grain.
use crate::{
    consumed_rows::{ClosureTable, NominalClosure, PreparedClosure, identifier},
    workspace::Cancellation,
};
use arrow_array::{Array, FixedSizeBinaryArray};
use datafusion::prelude::SessionContext;
use futures::TryStreamExt;
use lctx_model::domain::{
    assertion::{AssertionQualification, SupportScope},
    attribution::{ProviderRun, RunFamily},
    input::{CorpusLibrary, InputDistribution, InputRevision},
    resources::ResourceBudget,
    source::{CoverageScope, SourceArtifact},
    *,
};
use std::any::TypeId;

const ROOT_ROWS: usize = 128;
type Grain = (Option<[u8; 16]>, Option<[u8; 16]>);

fn typed<R: Record>(tables: &[ClosureTable]) -> Option<usize> {
    let mut choices = tables
        .iter()
        .enumerate()
        .filter(|(_, table)| table.relation.type_id() == TypeId::of::<R>());
    let (index, _) = choices.next()?;
    if choices.next().is_some() {
        return None;
    }
    Some(index)
}
pub(crate) fn declared(
    tables: &[ClosureTable],
    inputs: &[ValidationInput],
    input: &ValidationInput,
) -> Result<usize, ModelError> {
    inputs
        .iter()
        .position(|candidate| {
            candidate.type_id() == input.type_id() && candidate.prefix() == input.prefix()
        })
        .filter(|index| tables[*index].relation.type_id() == input.type_id())
        .ok_or(ModelError::Conflict("support input immutable binding"))
}
pub(crate) fn field_target(
    inputs: &[ValidationInput],
    source: usize,
    target: TypeId,
) -> Result<Option<usize>, ModelError> {
    let candidates: Vec<_> = inputs
        .iter()
        .enumerate()
        .filter(|(_, input)| input.type_id() == target)
        .map(|(index, _)| index)
        .collect();
    if candidates.is_empty() {
        return Ok(None);
    }
    if let [only] = candidates.as_slice() {
        return Ok(Some(*only));
    }
    let matching: Vec<_> = candidates
        .into_iter()
        .filter(|index| inputs[*index].prefix() == inputs[source].prefix())
        .collect();
    match matching.as_slice() {
        [] => Err(ModelError::Conflict("support dependency epoch absent")),
        [only] => Ok(Some(*only)),
        _ => Err(ModelError::Conflict("ambiguous support dependency epoch")),
    }
}
fn own<M: Record, O: Record>(
    plan: &mut NominalClosure,
    tables: &[ClosureTable],
    inputs: &[ValidationInput],
    field: &str,
) -> Result<(), ModelError> {
    for (member, _) in tables
        .iter()
        .enumerate()
        .filter(|(_, table)| table.relation.type_id() == TypeId::of::<M>())
    {
        if let Some(owner) = field_target(inputs, member, TypeId::of::<O>())? {
            plan.own(member, field, owner)?;
        }
    }
    Ok(())
}
fn plan(
    tables: &[ClosureTable],
    inputs: &[ValidationInput],
    scope: &SupportScope,
) -> Result<NominalClosure, ModelError> {
    let mut plan = NominalClosure::new(tables.to_vec())?;
    // Forward dependencies are mechanical nominal declarations, including qualifications,
    // source/evidence/subject/condition lineage and the selected derived support owner frame.
    for (source, table) in tables.iter().enumerate() {
        for field in table.relation.fields() {
            let Some((target, _)) = field.target() else {
                continue;
            };
            let Some(target) = field_target(inputs, source, target)? else {
                continue;
            };
            if field.list() {
                plan.pairs(
                    source,
                    target,
                    format!(
                        "SELECT id AS source_id, UNNEST({}) AS target_id FROM {}",
                        identifier(field.name()),
                        identifier(&table.alias)
                    ),
                )?;
            } else {
                plan.follow(source, field.name(), target)?;
            }
        }
    }
    let assertion = declared(tables, inputs, &scope.assertion)?;
    let support = declared(tables, inputs, &scope.support)?;
    plan.own(support, "assertion", assertion)?;
    // These are actual owner memberships. Arbitrary incoming references must not grow a grain.
    own::<flow::FlowCallStep, flow::FlowCallPath>(&mut plan, tables, inputs, "path")?;
    own::<types::TypeSequenceMember, types::TypeSequence>(&mut plan, tables, inputs, "sequence")?;
    own::<types::TypedDictField, types::TypedDictFieldList>(&mut plan, tables, inputs, "list")?;
    own::<types::CallableParameter, types::CallableParameterList>(
        &mut plan, tables, inputs, "list",
    )?;
    if let (Some(run), Some(family)) = (typed::<ProviderRun>(tables), typed::<RunFamily>(tables)) {
        plan.pairs(
            run,
            family,
            format!(
                "SELECT run AS source_id,id AS target_id FROM {} WHERE family={}",
                identifier(&tables[family].alias),
                scope.family as i16
            ),
        )?;
    }
    Ok(plan)
}

pub(crate) fn column(
    batch: &arrow_array::RecordBatch,
    name: &str,
    row: usize,
) -> Result<Option<[u8; 16]>, ModelError> {
    let values = batch
        .column_by_name(name)
        .and_then(|column| column.as_any().downcast_ref::<FixedSizeBinaryArray>())
        .ok_or(ModelError::Schema("support grain nominal projection"))?;
    if values.is_null(row) {
        return Ok(None);
    }
    Ok(Some(values.value(row).try_into().map_err(|_| {
        ModelError::Schema("support grain nominal width")
    })?))
}
fn selected<R: Record>(
    closure: &PreparedClosure,
    tables: &[ClosureTable],
) -> Result<String, ModelError> {
    let index = typed::<R>(tables).ok_or(ModelError::Conflict("support ownership input epoch"))?;
    closure.select(index)
}
fn union(parts: Vec<String>) -> String {
    parts.join(" UNION ")
}

/// Ownership needs pairs, rather than all input neighbors. The selected artifacts and scope
/// supply the library/release side; selected native/derived invocations supply the corpus side.
fn ownership(
    closure: &PreparedClosure,
    tables: &[ClosureTable],
    inputs: &[ValidationInput],
    scope: &SupportScope,
) -> Result<(String, String), ModelError> {
    let sources = selected::<SourceArtifact>(closure, tables)?;
    let scopes = selected::<CoverageScope>(closure, tables)?;
    let mut invocation_inputs = vec![format!(
        "SELECT input FROM ({}) AS native_runs",
        selected::<ProviderRun>(closure, tables)?
    )];
    for input in &scope.source_inputs {
        let index = declared(tables, inputs, input)?;
        for field in tables[index].relation.fields().iter().filter(|field| {
            field
                .target()
                .is_some_and(|(target, _)| target == TypeId::of::<InputRevision>())
                && !field.list()
        }) {
            invocation_inputs.push(format!(
                "SELECT {} AS input FROM ({}) AS derived_frames",
                identifier(field.name()),
                closure.select(index)?
            ));
        }
    }
    invocation_inputs.push(format!(
        "SELECT input_input AS input FROM ({scopes}) AS input_scopes WHERE input_input IS NOT NULL"
    ));
    let owners = union(invocation_inputs);
    let release_ids = format!(
        "SELECT release_release AS release FROM ({scopes}) AS release_scopes WHERE release_release IS NOT NULL"
    );
    let artifact_inputs = format!("SELECT input FROM ({sources}) AS artifacts");
    let distributions =
        typed::<InputDistribution>(tables).ok_or(ModelError::Schema("support distributions"))?;
    let corpus =
        typed::<CorpusLibrary>(tables).ok_or(ModelError::Schema("support corpus membership"))?;
    let dist_table = identifier(&tables[distributions].alias);
    let corpus_table = identifier(&tables[corpus].alias);
    let needed_libraries = format!(
        "{artifact_inputs} UNION SELECT input FROM {dist_table} WHERE release IN ({release_ids})"
    );
    let selected_corpus = format!(
        "SELECT * FROM {corpus_table} WHERE corpus IN ({owners}) AND library IN ({needed_libraries})"
    );
    let needed_inputs = format!(
        "{artifact_inputs} UNION {owners} UNION SELECT library AS input FROM ({selected_corpus}) AS release_members"
    );
    let selected_distributions = format!(
        "SELECT * FROM {dist_table} WHERE release IN ({release_ids}) AND input IN ({needed_inputs})"
    );
    Ok((selected_corpus, selected_distributions))
}

async fn validate_grain(
    invariant: &Invariant,
    scope: &SupportScope,
    tables: &[ClosureTable],
    closure: &PreparedClosure,
    budget: &ResourceBudget,
    cancellation: &Cancellation,
) -> Result<(), ModelError> {
    let (corpus, distributions) = ownership(closure, tables, &invariant.inputs, scope)?;
    let mut check = (invariant.create)(budget);
    for (index, input) in invariant.inputs.iter().enumerate() {
        let select = if input.type_id() == TypeId::of::<CorpusLibrary>() {
            corpus.clone()
        } else if input.type_id() == TypeId::of::<InputDistribution>() {
            distributions.clone()
        } else {
            closure.select(index)?
        };
        let order = input
            .order()
            .iter()
            .map(|field| identifier(field))
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT * FROM ({select}) AS selected_rows{}",
            if order.is_empty() {
                String::new()
            } else {
                format!(" ORDER BY {order}")
            }
        );
        let mut rows = crate::sql::query(closure.session(), &sql)
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
    check.finish()
}

pub(crate) async fn validate_support(
    invariant: &Invariant,
    scope: &SupportScope,
    tables: Vec<ClosureTable>,
    session: &SessionContext,
    budget: &ResourceBudget,
    cancellation: &Cancellation,
) -> Result<(), ModelError> {
    let root = declared(&tables, &invariant.inputs, &scope.assertion)?;
    let qualification = typed::<AssertionQualification>(&tables)
        .ok_or(ModelError::Schema("support qualification"))?;
    let prepared = plan(&tables, &invariant.inputs, scope)?
        .prepare(session, budget)
        .await?;
    // LEFT JOIN keeps unsupported assertions and assertions with an absent qualification in the
    // root domain. The independent global reference pass still refuses every missing reference.
    let sql = format!(
        "SELECT a.id,q.scope,q.context FROM {} AS a LEFT JOIN {} AS q ON a.qualification=q.id ORDER BY q.scope,q.context,a.id",
        identifier(&tables[root].alias),
        identifier(&tables[qualification].alias)
    );
    let mut roots = crate::sql::query(session, &sql)
        .await
        .map_err(ModelError::codec)?
        .execute_stream()
        .await
        .map_err(ModelError::codec)?;
    let _root_charge = budget.reserve("support-admission-roots", ROOT_ROWS * 128)?;
    let mut ids = Vec::with_capacity(ROOT_ROWS);
    let mut grain: Option<Grain> = None;
    while let Some(batch) = roots.try_next().await.map_err(ModelError::codec)? {
        cancellation.check()?;
        for row in 0..batch.num_rows() {
            let next = (
                column(&batch, "scope", row)?,
                column(&batch, "context", row)?,
            );
            if !ids.is_empty() && (grain != Some(next) || ids.len() == ROOT_ROWS) {
                let predicate = root_predicate(&ids);
                let closure = prepared.grain(root, &predicate, budget).await?;
                validate_grain(invariant, scope, &tables, &closure, budget, cancellation).await?;
                ids.clear();
            }
            grain = Some(next);
            ids.push(column(&batch, "id", row)?.ok_or(ModelError::Schema("support assertion ID"))?);
        }
    }
    if !ids.is_empty() {
        let predicate = root_predicate(&ids);
        let closure = prepared.grain(root, &predicate, budget).await?;
        validate_grain(invariant, scope, &tables, &closure, budget, cancellation).await?;
    }
    Ok(())
}
pub(crate) fn root_predicate(ids: &[[u8; 16]]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut predicate = String::with_capacity(9 + ids.len() * 37);
    predicate.push_str("id IN (");
    for (index, id) in ids.iter().enumerate() {
        if index != 0 {
            predicate.push(',');
        }
        predicate.push_str("X'");
        for byte in id {
            predicate.push(HEX[usize::from(byte >> 4)] as char);
            predicate.push(HEX[usize::from(byte & 15)] as char);
        }
        predicate.push('\'');
    }
    predicate.push(')');
    predicate
}

#[cfg(test)]
mod controls {
    use super::*;
    use datafusion::{datasource::MemTable, prelude::SessionConfig};
    use lctx_model::domain::{
        assertion::{Approximation, Evidence, ProviderSurface},
        attribution::{ExtractionMode, FactFamily, Fidelity, Modality, Origin, Provider},
        conditions::{Condition, ConditionNode},
        source::{Occurrence, OccurrenceRole, SyntaxKind, SyntaxObservation, SyntaxSupport},
    };
    use std::sync::Arc;

    fn nominal<R>(byte: u8) -> Id<R> {
        serde_json::from_value(serde_json::to_value([byte; 16]).unwrap()).unwrap()
    }
    struct Fixture {
        invariant: Invariant,
        scope: SupportScope,
        tables: Vec<ClosureTable>,
        session: SessionContext,
        budget: ResourceBudget,
        assertion: SyntaxObservation,
        support: SyntaxSupport,
        artifact: SourceArtifact,
        run: ProviderRun,
    }
    impl Fixture {
        fn put<R: Record>(&self, rows: &[R]) {
            let index = typed::<R>(&self.tables).unwrap();
            let alias = &self.tables[index].alias;
            self.session.deregister_table(alias.as_str()).unwrap();
            let batch = R::encode(rows).unwrap();
            self.session
                .register_table(
                    alias.as_str(),
                    Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap()),
                )
                .unwrap();
        }
        fn new() -> Self {
            let model = lctx_model::domain::model().unwrap();
            let invariant = assertion::support_invariants::<SyntaxObservation, SyntaxSupport>()
                .pop()
                .unwrap();
            let budget = ResourceBudget::fixed(64 << 20).unwrap();
            let scope = (invariant.create)(&budget).support_scope().unwrap();
            let session =
                SessionContext::new_with_config(SessionConfig::new().with_target_partitions(2));
            let tables: Vec<_> = invariant
                .inputs
                .iter()
                .enumerate()
                .map(|(index, input)| ClosureTable {
                    relation: model.relation(input.name()).unwrap().clone(),
                    alias: format!("scope_input_{index}"),
                })
                .collect();
            for table in &tables {
                let batch = arrow_array::RecordBatch::new_empty(table.relation.schema().clone());
                session
                    .register_table(
                        table.alias.as_str(),
                        Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap()),
                    )
                    .unwrap();
            }
            let artifact =
                SourceArtifact::from_bytes(nominal(1), "selected.py".into(), b"x").unwrap();
            let occurrence = Occurrence {
                source: artifact.id(),
                start: 0,
                end: 1,
                syntax_kind: SyntaxKind::ExprName,
                role: OccurrenceRole::Syntax,
                structural_path: vec![],
            };
            let coverage = CoverageScope::Artifact {
                artifact: artifact.id(),
            };
            let condition = Condition {
                root: ConditionNode::True.id(),
            };
            let qualification = AssertionQualification {
                context: nominal(2),
                scope: coverage.id(),
                condition: condition.id(),
                modality: Modality::Definite,
                approximation: Approximation::Exact,
                assumptions: nominal(3),
            };
            let run = ProviderRun {
                provider: nominal::<Provider>(4),
                context: qualification.context,
                input: nominal(9),
                configuration: ContentHash::of(b"config"),
                requested_families: ContentHash::of(b"families"),
            };
            let surface = ProviderSurface {
                provider: run.provider,
                family: FactFamily::Syntax,
                name: "syntax".into(),
            };
            let evidence = Evidence::Occurrence {
                occurrence: occurrence.id(),
            };
            let assertion = SyntaxObservation {
                qualification: qualification.id(),
                occurrence: occurrence.id(),
                spelling: "x".into(),
            };
            let support = SyntaxSupport {
                assertion: assertion.id(),
                run: run.id(),
                surface: surface.id(),
                evidence: evidence.id(),
                origin: Origin::SourceObservation,
                mode: ExtractionMode::NativeTraversal,
                fidelity: Fidelity::NativeStructural,
            };
            let fixture = Self {
                invariant,
                scope,
                tables,
                session,
                budget,
                assertion,
                support,
                artifact,
                run,
            };
            fixture.put(std::slice::from_ref(&fixture.artifact));
            fixture.put(&[occurrence]);
            fixture.put(&[coverage]);
            fixture.put(&[ConditionNode::True]);
            fixture.put(&[condition]);
            fixture.put(&[qualification]);
            fixture.put(std::slice::from_ref(&fixture.run));
            fixture.put(&[surface]);
            fixture.put(&[evidence]);
            fixture.put(std::slice::from_ref(&fixture.assertion));
            fixture.put(std::slice::from_ref(&fixture.support));
            fixture.put(&[RunFamily {
                run: fixture.run.id(),
                family: FactFamily::Syntax,
            }]);
            fixture.put(&[CorpusLibrary {
                corpus: fixture.run.input,
                library: fixture.artifact.input,
            }]);
            fixture
        }
        async fn validate(&self) -> Result<(), ModelError> {
            validate_support(
                &self.invariant,
                &self.scope,
                self.tables.clone(),
                &self.session,
                &self.budget,
                &Cancellation::default(),
            )
            .await
        }
    }
    #[tokio::test]
    async fn unsupported_roots_and_wrong_run_family_still_refuse() {
        let fixture = Fixture::new();
        fixture.validate().await.unwrap();
        fixture.put::<SyntaxSupport>(&[]);
        let error = fixture.validate().await.unwrap_err();
        assert!(
            error
                .to_string()
                .contains("assertion has no attributed support"),
            "{error}"
        );
        fixture.put(std::slice::from_ref(&fixture.support));
        fixture.put(&[RunFamily {
            run: fixture.run.id(),
            family: FactFamily::Calls,
        }]);
        assert!(fixture.validate().await.is_err());
        assert_eq!(fixture.budget.reserved(), 0);
    }
    #[tokio::test]
    async fn ownership_selects_actual_artifacts_not_all_input_neighbors() {
        let fixture = Fixture::new();
        let mut memberships = vec![CorpusLibrary {
            corpus: fixture.run.input,
            library: fixture.artifact.input,
        }];
        memberships.extend((10..=200).map(|byte| CorpusLibrary {
            corpus: fixture.run.input,
            library: nominal(byte),
        }));
        fixture.put(&memberships);
        let root = declared(
            &fixture.tables,
            &fixture.invariant.inputs,
            &fixture.scope.assertion,
        )
        .unwrap();
        let edges = plan(&fixture.tables, &fixture.invariant.inputs, &fixture.scope)
            .unwrap()
            .prepare(&fixture.session, &fixture.budget)
            .await
            .unwrap();
        let closure = edges
            .grain(
                root,
                &root_predicate(&[*fixture.assertion.id().bytes()]),
                &fixture.budget,
            )
            .await
            .unwrap();
        let (corpus, _) = ownership(
            &closure,
            &fixture.tables,
            &fixture.invariant.inputs,
            &fixture.scope,
        )
        .unwrap();
        let batches = crate::sql::query(closure.session(), &corpus)
            .await
            .unwrap()
            .collect()
            .await
            .unwrap();
        let rows = batches
            .iter()
            .flat_map(|batch| CorpusLibrary::decode(batch).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(rows, vec![memberships[0].clone()]);
        drop(closure);
        drop(edges);
        fixture.validate().await.unwrap();
        fixture.put::<CorpusLibrary>(&[]);
        assert!(fixture.validate().await.is_err());
        assert_eq!(fixture.budget.reserved(), 0);
    }
    #[tokio::test]
    async fn absent_qualification_is_not_removed_by_grain_selection() {
        let fixture = Fixture::new();
        fixture.put::<AssertionQualification>(&[]);
        let error = fixture.validate().await.unwrap_err();
        assert!(
            error
                .to_string()
                .contains("assertion qualification missing"),
            "{error}"
        );
        assert_eq!(fixture.budget.reserved(), 0);
    }
    #[tokio::test]
    async fn rich_support_state_is_released_between_bounded_grains() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let mut fixture = Fixture::new();
        let assertions: Vec<_> = (0..ROOT_ROWS * 2 + 1)
            .map(|index| SyntaxObservation {
                spelling: format!("spelling-{index}"),
                ..fixture.assertion.clone()
            })
            .collect();
        let supports: Vec<_> = assertions
            .iter()
            .map(|row| SyntaxSupport {
                assertion: row.id(),
                ..fixture.support.clone()
            })
            .collect();
        fixture.put(&assertions);
        fixture.put(&supports);
        let checks = Arc::new(AtomicUsize::new(0));
        let count = checks.clone();
        let original = fixture.invariant.create.clone();
        fixture.invariant.create = Arc::new(move |budget| {
            count.fetch_add(1, Ordering::Relaxed);
            original(budget)
        });
        fixture.validate().await.unwrap();
        assert_eq!(checks.load(Ordering::Relaxed), 3);
        assert_eq!(fixture.budget.reserved(), 0);
        fixture.put(&supports[..supports.len() - 1]);
        let error = fixture.validate().await.unwrap_err();
        assert!(
            error
                .to_string()
                .contains("assertion has no attributed support"),
            "{error}"
        );
        assert_eq!(fixture.budget.reserved(), 0);
    }
    #[test]
    fn dependency_epochs_are_selected_explicitly() {
        use stages::PublicationBoundary;
        let inputs = vec![
            ValidationInput::of::<types::TypeSequenceMember>(&["id"])
                .at_epoch(PublicationBoundary::Facts),
            ValidationInput::of::<types::TypeTerm>(&["id"]),
            ValidationInput::of::<types::TypeTerm>(&["id"]).at_epoch(PublicationBoundary::Facts),
        ];
        assert_eq!(
            field_target(&inputs, 0, TypeId::of::<types::TypeTerm>()).unwrap(),
            Some(2)
        );
        let mut absent = inputs;
        absent[0] = ValidationInput::of::<types::TypeSequenceMember>(&["id"])
            .at_epoch(PublicationBoundary::Dispatch);
        assert!(field_target(&absent, 0, TypeId::of::<types::TypeTerm>()).is_err());
    }
}
