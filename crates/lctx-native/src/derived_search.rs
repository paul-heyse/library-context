//! One terminally checked lowering for private construction and read-only cold reconciliation.
use crate::surrealdb::types::{Object, RecordId, SurrealValue, Value, Variables};
use crate::{
    Loader, NativeReader, RecordSelection,
    ordered_rows::{OrderedRows, PreparedRows, SortedRows},
    prepared::scope_string,
    reader::target_id,
};
use lctx_model::domain::{
    embedding::{
        EmbeddingSpec,
        projection::{ProjectedValue, ProjectionDefinition},
        value::FullValue,
    },
    graph::{EntityId, Target},
    retrieval::{
        ContentPart, Family, Origin, OriginalAnchor, PartPurpose, SearchWindow, Subject, Unit,
        WindowBinding, WindowPart, WindowSourceMap, consumption::RetrievalEmbeddingUse,
    },
    *,
};
use std::collections::{BTreeMap, BTreeSet};
const TABLES: [&str; 7] = [
    "search_api_options",
    "search_documentation_deployment",
    "search_scenario",
    "search_source",
    "vector",
    "lex_occurs",
    "vec_occurs",
];
const BATCH_ROWS: usize = 128;
const BATCH_BYTES: usize = 1024 * 1024;
fn table(family: Family) -> &'static str {
    match family {
        Family::ApiOptions => TABLES[0],
        Family::DocumentationDeployment => TABLES[1],
        Family::Scenario => TABLES[2],
        Family::Source => TABLES[3],
    }
}
fn reader(loader: &Loader) -> Result<NativeReader<()>, ModelError> {
    Ok(loader.reader())
}
fn original_anchor_nomination_sql() -> Result<String, ModelError> {
    let table = crate::schema::ScopeTable::for_relation(OriginalAnchor::NAME)?.name();
    Ok(format!(
        "SELECT id FROM {table} WITH INDEX by_scope WHERE semantic_type=$type AND scope_keys CONTAINSANY $scopes AND body.original IN $originals"
    ))
}
/// Canonical context reads retain the caller's sparse reader or prepared selection.
/// Recovery may use a transaction; it must not create a separate session or selection.
pub(crate) trait SearchCanonicalRead: Sync {
    fn search_candidates<R: Record + serde::de::DeserializeOwned>(
        &self,
        query: crate::prepared::PreparedQuery,
    ) -> Result<crate::reader::CanonicalRecords<R>, ModelError>;
    fn charge_record<R: Record>(&self, _row: &R) -> Result<(), ModelError> {
        Ok(())
    }
}
fn context_query<R: Record>(
    selection: RecordSelection,
) -> Result<Option<crate::prepared::PreparedQuery>, ModelError> {
    let mut bindings = Variables::new();
    bindings.insert("type", R::NAME.to_owned());
    let mut preparation = Vec::new();
    let (index, predicate) = match selection {
        RecordSelection::Keys(keys) => {
            if keys.is_empty() {
                return Ok(None);
            }
            bindings.insert("keys", keys.iter().map(hex::encode).collect::<Vec<_>>());
            (
                " WITH INDEX semantic_key",
                "semantic_key IN $keys".to_owned(),
            )
        }
        RecordSelection::Scope { field, values } => {
            if values.is_empty() {
                return Ok(None);
            }
            if !R::fields().iter().any(|f| f.name() == field) {
                return Err(ModelError::Invalid("undeclared native scope field".into()));
            }
            let sparse = crate::schema::atomic_scope_field(R::NAME, field.as_str())
                && !values.iter().any(serde_json::Value::is_null);
            bindings.insert(
                "values",
                crate::loader::json_value(serde_json::Value::Array(values))?,
            );
            if sparse {
                (
                    " WITH INDEX by_scope",
                    crate::prepared::prepare_scope(
                        &mut preparation,
                        "record_scope",
                        "$type",
                        &field,
                        "$values",
                    ),
                )
            } else {
                ("", format!("body.`{field}` IN $values"))
            }
        }
        _ => return Err(ModelError::Schema("search canonical context selection")),
    };
    let table = crate::schema::ScopeTable::for_relation(R::NAME)?.name();
    Ok(Some(crate::prepared::PreparedQuery::new(
        bindings,
        preparation,
        vec![format!(
            "SELECT id FROM {table}{index} WHERE semantic_type=$type AND ({predicate})"
        )],
    )?))
}
async fn collect_context<R: Record + serde::de::DeserializeOwned>(
    source: &impl SearchCanonicalRead,
    mut rows: crate::reader::CanonicalRecords<R>,
) -> Result<Vec<R>, ModelError> {
    let result = async {
        let mut values = Vec::new();
        while let Some(row) = rows.next().await? {
            source.charge_record(&row)?;
            values.push(row);
        }
        Ok(values)
    }
    .await;
    let mut terminal = completion::Completion::default();
    terminal.step(
        "search canonical context drainage",
        rows.drain_transport().await,
    );
    completion::complete(result, terminal)
}
async fn context_records<R: Record + serde::de::DeserializeOwned>(
    source: &impl SearchCanonicalRead,
    selection: RecordSelection,
) -> Result<Vec<R>, ModelError> {
    let Some(query) = context_query::<R>(selection)? else {
        return Ok(vec![]);
    };
    collect_context(source, source.search_candidates(query)?).await
}
impl SearchCanonicalRead for NativeReader<()> {
    fn search_candidates<R: Record + serde::de::DeserializeOwned>(
        &self,
        query: crate::prepared::PreparedQuery,
    ) -> Result<crate::reader::CanonicalRecords<R>, ModelError> {
        self.record_stream_candidates(query, "semantic_key")
    }
}
struct ChargedContextRead<'a, S> {
    source: &'a S,
    charge: std::sync::Mutex<charged::StateCharge>,
}
impl<S: SearchCanonicalRead> SearchCanonicalRead for ChargedContextRead<'_, S> {
    fn search_candidates<R: Record + serde::de::DeserializeOwned>(
        &self,
        query: crate::prepared::PreparedQuery,
    ) -> Result<crate::reader::CanonicalRecords<R>, ModelError> {
        self.source.search_candidates(query)
    }
    fn charge_record<R: Record>(&self, row: &R) -> Result<(), ModelError> {
        let bytes = row
            .heap_bytes()
            .saturating_mul(2)
            .saturating_add(std::mem::size_of::<R>().saturating_mul(2))
            .saturating_add(128);
        self.charge
            .lock()
            .map_err(|_| ModelError::Conflict("search context charge"))?
            .grow(bytes)
    }
}
/// Positive selected dependencies alone cannot establish binding absence or complete
/// additive companion context. Keep extra claimed dependencies and physical fields for
/// independent cold comparison; only stale canonical witness contexts are excluded.
pub(crate) async fn eligible_context_rows(
    source: &impl SearchCanonicalRead,
    rows: Vec<Value>,
    budget: &resources::ResourceBudget,
) -> Result<Vec<Value>, ModelError> {
    if rows.is_empty() {
        return Ok(rows);
    }
    let source = ChargedContextRead {
        source,
        charge: std::sync::Mutex::new(charged::StateCharge::new(
            budget,
            "occurrence-witness-context",
        )),
    };
    let _decoded = budget.reserve("occurrence context decode", BATCH_BYTES)?;
    let windows = rows
        .iter()
        .map(|row| {
            let Value::Object(row) = row else {
                return Err(ModelError::Schema("occurrence context object"));
            };
            context_nominal(row, "window")
        })
        .collect::<Result<Vec<Id<SearchWindow>>, _>>()?;
    let basic = Basic::load(&source, keyed(&source, windows).await?).await?;
    let companions = Companions::witness_context(&source, &basic).await?;
    let mut accepted = Vec::new();
    for row in rows {
        let object = row
            .as_object()
            .ok_or(ModelError::Schema("occurrence context object"))?;
        let window: Id<SearchWindow> = context_nominal(object, "window")?;
        let part: Id<ContentPart> = context_nominal(object, "part")?;
        let binding = context_binding(object)?;
        let window = need(&basic.windows, window)?;
        if !basic.primary(window.id()).any(|p| p.id() == part) {
            continue;
        }
        let part = need(&basic.parts, part)?;
        let selected_binding = match binding {
            Some(binding) => match companions.bindings.get(&binding) {
                Some(binding) if binding.window == window.id() && binding.part == part.id() => {
                    Some(binding)
                }
                _ => continue,
            },
            None => {
                if companions
                    .bindings
                    .values()
                    .any(|binding| binding.window == window.id() && binding.part == part.id())
                {
                    continue;
                }
                None
            }
        };
        let (_, required) = witness_context(
            need(&basic.units, window.unit)?,
            window,
            part,
            selected_binding,
            &companions,
            &basic,
        )?;
        let claimed = nomination_dependencies(object)?;
        if required
            .iter()
            .all(|dependency| claimed.contains(dependency))
        {
            accepted.push(row);
        }
    }
    Ok(accepted)
}
fn context_nominal<T: serde::de::DeserializeOwned + serde::Serialize + 'static>(
    object: &Object,
    field: &str,
) -> Result<T, ModelError> {
    Ok(crate::surrealdb::types::SerdeWrapper::<T>::from_value(
        object
            .get(field)
            .cloned()
            .ok_or(ModelError::Schema("occurrence context nominal"))?,
    )
    .map_err(ModelError::codec)?
    .0)
}
fn context_binding(object: &Object) -> Result<Option<Id<WindowBinding>>, ModelError> {
    // Occurrence construction writes JSON null, whereas SerdeWrapper<Option<T>>
    // treats only native NONE as absent. Missing fields remain invalid claims.
    match object.get("binding") {
        Some(Value::Null) => Ok(None),
        _ => context_nominal(object, "binding").map(Some),
    }
}

/// Complete selected occurrence stream. Unit payload windows use the installed equality
/// index; dependency eligibility is evaluated before the caller's ranking/quota policy.
pub fn selected_occurrences<Context>(
    reader: &NativeReader<Context>,
    table: &str,
    predicate: &str,
    bindings: Variables,
    budget: &resources::ResourceBudget,
) -> Result<crate::reader::NativeRows, ModelError> {
    if bindings.get("input_keys").is_some() || bindings.get("window_keys").is_some() {
        indexed_occurrence_rows(reader, table, predicate, bindings, budget)
    } else {
        occurrence_rows(reader, table, predicate, bindings, true, budget)
    }
}
/// Serving nominations use existing input/window indexes. Exact dependency checks run
/// before payload hydration, without preparing the complete publication membership.
fn indexed_occurrence_rows<Context>(
    reader: &NativeReader<Context>,
    table: &str,
    predicate: &str,
    bindings: Variables,
    budget: &resources::ResourceBudget,
) -> Result<crate::reader::NativeRows, ModelError> {
    if !matches!(table, "lex_occurs" | "vec_occurs") {
        return Err(ModelError::Schema("indexed occurrence family"));
    }
    let index = if bindings.get("window_keys").is_some() {
        "window_occurrences"
    } else {
        "eligible_input"
    };
    reader.derivation()?;
    let predicate = format!("derivation=$lctx_derivation AND ({predicate})");
    let nominations = reader.stream_prepared(crate::prepared::PreparedQuery::new(bindings, vec![], vec![format!("SELECT id,derivation,unit_payload,dependencies FROM {table} WITH INDEX {index} WHERE ({predicate})")])?)?;
    hydrate_occurrence_nominations(reader, nominations, budget)
}
/// Table FULLTEXT nominates documents, while three independent equality indexes nominate
/// exact occurrences even when the query has no analyzer terms. Neither branch has a
/// pre-eligibility quota; unrelated publications cannot consume the selected frontier.
pub fn lexical_nomination_sql(family_table: &str) -> Result<String, ModelError> {
    if !TABLES[..4].contains(&family_table) {
        return Err(ModelError::Schema("lexical nomination family"));
    }
    Ok(format!(
        "SELECT id FROM {family_table} WITH INDEX lexical WHERE derivation=$lctx_derivation AND text @OR@ $query"
    ))
}
pub fn lexical_occurrences<Context>(
    reader: &NativeReader<Context>,
    family_table: &str,
    predicate: &str,
    bindings: Variables,
    budget: &resources::ResourceBudget,
) -> Result<crate::reader::NativeRows, ModelError> {
    reader.derivation()?;
    let nomination_sql = lexical_nomination_sql(family_table)?;
    let owner = reader.transport_reader();
    let predicate = format!("derivation=$lctx_derivation AND ({predicate})");
    let budget_copy = budget.clone();
    let nominations = crate::reader::NativeRows::owned(move |sender| async move {
        let mut documents = owner.stream_prepared(crate::prepared::PreparedQuery::new(
            bindings.clone(),
            vec![],
            vec![nomination_sql],
        )?)?;
        let result = async {
            loop {
                if sender.is_closed() { documents.cancel_delivery(); break; }
                let mut scratch = charged::StateCharge::new(&budget_copy, "lexical-document-nominations");
                let mut ids = Vec::new();
                while ids.len() < BATCH_ROWS {
                    let Some(row) = documents.next().await? else { break; };
                    scratch.grow(crate::loader::native_bytes(&row).saturating_mul(3))?;
                    let object = Object::from_value(row).map_err(ModelError::codec)?;
                    ids.push(RecordId::from_value(object.get("id").cloned().ok_or(ModelError::Schema("lexical document nomination"))?).map_err(ModelError::codec)?);
                }
                if ids.is_empty() { break; }
                let mut vars = bindings.clone(); vars.insert("documents", ids);
                let mut rows = owner.stream_prepared(crate::prepared::PreparedQuery::new(vars, vec![], vec![format!("SELECT id,derivation,unit_payload,dependencies FROM lex_occurs WITH INDEX document_occurrences WHERE in IN $documents AND ({predicate})")])?)?;
                let result = async { while let Some(row) = rows.next().await? { if sender.send(row).await.is_err() { rows.cancel_delivery(); break; } } Ok(()) }.await;
                let mut terminal = completion::Completion::default(); terminal.step("lexical document occurrences drainage", rows.drain_transport().await); completion::complete(result, terminal)?;
            }
            Ok(())
        }.await;
        let mut terminal = completion::Completion::default();
        terminal.step(
            "lexical fulltext nomination drainage",
            documents.drain_transport().await,
        );
        completion::complete(result, terminal)?;
        for field in ["exact_name", "exact_path", "exact_option"] {
            if sender.is_closed() {
                return Ok(());
            }
            let mut rows = owner.stream_prepared(crate::prepared::PreparedQuery::new(bindings.clone(), vec![], vec![format!("SELECT id,derivation,unit_payload,dependencies FROM lex_occurs WITH INDEX {field} WHERE {field}=$query AND ({predicate})")])?)?;
            let result = async {
                while let Some(row) = rows.next().await? {
                    if sender.send(row).await.is_err() {
                        rows.cancel_delivery();
                        break;
                    }
                }
                Ok(())
            }
            .await;
            let mut terminal = completion::Completion::default();
            terminal.step(
                "lexical exact nomination drainage",
                rows.drain_transport().await,
            );
            completion::complete(result, terminal)?;
        }
        Ok(())
    })?;
    hydrate_occurrence_nominations(reader, nominations, budget)
}
pub(crate) fn nomination_dependencies(object: &Object) -> Result<Vec<RecordId>, ModelError> {
    let mut dependencies = Vec::<RecordId>::from_value(
        object
            .get("dependencies")
            .cloned()
            .ok_or(ModelError::Schema("occurrence dependencies"))?,
    )
    .map_err(ModelError::codec)?;
    if dependencies.is_empty() {
        return Ok(dependencies);
    }
    dependencies.push(
        RecordId::from_value(
            object
                .get("unit_payload")
                .cloned()
                .ok_or(ModelError::Schema("occurrence unit payload"))?,
        )
        .map_err(ModelError::codec)?,
    );
    dependencies.sort();
    dependencies.dedup();
    Ok(dependencies)
}
fn eligible_nomination_ids(
    window: &BTreeMap<RecordId, Object>,
    membership: &crate::selection::MembershipAnswers,
) -> Result<Vec<RecordId>, ModelError> {
    let mut accepted = Vec::new();
    for (id, object) in window {
        let dependencies = nomination_dependencies(object)?;
        if !dependencies.is_empty() && membership.contains_all(&dependencies) {
            accepted.push(id.clone());
        }
    }
    Ok(accepted)
}
fn hydrate_occurrence_nominations<Context>(
    reader: &NativeReader<Context>,
    mut rows: crate::reader::NativeRows,
    budget: &resources::ResourceBudget,
) -> Result<crate::reader::NativeRows, ModelError> {
    let namespace = reader.derivation()?;
    let retention = reader.transport_reader();
    let reader = reader.transport_reader().with_request_budget(budget);
    let budget = budget.clone();
    let rows = crate::reader::NativeRows::owned(move |sender| async move {
        let mut sorted = SortedRows::with_budget(&budget)?;
        let result = async {
            loop {
                if sender.is_closed() {
                    rows.cancel_delivery();
                    break;
                }
                let mut scratch = charged::StateCharge::new(&budget, "indexed-occurrence-window");
                let mut window = BTreeMap::new();
                let mut union = BTreeSet::new();
                let mut examined = 0;
                while examined < BATCH_ROWS {
                    let Some(row) = rows.next().await? else {
                        break;
                    };
                    examined += 1;
                    scratch.grow(crate::loader::native_bytes(&row).saturating_mul(8))?;
                    let object = Object::from_value(row).map_err(ModelError::codec)?;
                    crate::derivation::require_namespace(&object, namespace)?;
                    let dependencies = nomination_dependencies(&object)?;
                    if dependencies.is_empty() {
                        continue;
                    }
                    let id = RecordId::from_value(
                        object
                            .get("id")
                            .cloned()
                            .ok_or(ModelError::Schema("occurrence identity"))?,
                    )
                    .map_err(ModelError::codec)?;
                    union.extend(dependencies.iter().cloned());
                    if let Some(previous) = window.insert(id, object.clone())
                        && previous != object
                    {
                        return Err(ModelError::Conflict("occurrence nomination changed"));
                    }
                }
                if examined == 0 {
                    break;
                }
                if window.is_empty() {
                    continue;
                }
                let union = union.into_iter().collect::<Vec<_>>();
                let membership = reader.selected_membership(&union, &budget).await?;
                let accepted = eligible_nomination_ids(&window, &membership)?;
                if accepted.is_empty() {
                    continue;
                }
                let mut hydrated = Vec::new();
                let mut payload = reader.stream_prepared(crate::prepared::PreparedQuery::new(
                    Variables::from_iter([("nodes".into(), accepted.clone().into_value())]),
                    vec![],
                    vec!["SELECT * FROM $nodes".into()],
                )?)?;
                let result = async {
                    let mut seen = BTreeSet::new();
                    while let Some(row) = payload.next().await? {
                        let object = Object::from_value(row.clone()).map_err(ModelError::codec)?;
                        crate::derivation::require_namespace(&object, namespace)?;
                        let id = RecordId::from_value(
                            object
                                .get("id")
                                .cloned()
                                .ok_or(ModelError::Schema("occurrence payload identity"))?,
                        )
                        .map_err(ModelError::codec)?;
                        if !accepted.contains(&id) || !seen.insert(id.clone()) {
                            return Err(ModelError::Conflict(
                                "occurrence hydration correspondence",
                            ));
                        }
                        let expected = &window[&id];
                        if ["derivation", "dependencies", "unit_payload"]
                            .iter()
                            .any(|field| object.get(*field) != expected.get(*field))
                        {
                            return Err(ModelError::Conflict(
                                "occurrence hydration dependencies changed",
                            ));
                        }
                        scratch.grow(crate::loader::native_bytes(&row).saturating_mul(4))?;
                        hydrated.push(row);
                    }
                    if seen.len() != accepted.len() {
                        return Err(ModelError::Schema("missing nominated occurrence payload"));
                    }
                    Ok(())
                }
                .await;
                let mut terminal = completion::Completion::default();
                terminal.step(
                    "indexed occurrence payload drainage",
                    payload.drain_transport().await,
                );
                completion::complete(result, terminal)?;
                for row in eligible_context_rows(&reader, hydrated, &budget).await? {
                    sorted.push(row)?;
                }
            }
            Ok(())
        }
        .await;
        let mut terminal = completion::Completion::default();
        terminal.step(
            "indexed occurrence nomination drainage",
            rows.drain_transport().await,
        );
        completion::complete(result, terminal)?;
        let mut sorted = sorted.finish()?;
        while sorted.next_row()?.is_some() {}
        sorted.rewind()?;
        while let Some(row) = sorted.next_row()? {
            if sender.send(row).await.is_err() {
                break;
            }
        }
        Ok(())
    })?;
    Ok(retention.retain_rows(rows))
}
fn occurrence_rows<Context>(
    reader: &NativeReader<Context>,
    table: &str,
    predicate: &str,
    bindings: Variables,
    eligible: bool,
    budget: &resources::ResourceBudget,
) -> Result<crate::reader::NativeRows, ModelError> {
    if !matches!(table, "lex_occurs" | "vec_occurs") {
        return Err(ModelError::Schema("selected occurrence family"));
    }
    let client = reader.shared_client();
    let cancellation = reader.read_cancellation();
    let context_reader = reader.transport_reader().with_request_budget(budget);
    let selection = reader.selection_preparation();
    let table = table.to_owned();
    let namespace = reader.derivation()?;
    let mut exact_bindings = reader.view_bindings();
    exact_bindings.extend(bindings);
    let bindings = exact_bindings;
    let predicate = format!("derivation=$lctx_derivation AND ({predicate})");
    let budget = budget.clone();
    crate::reader::NativeRows::owned(move |sender| async move {
        let mut sorted = SortedRows::with_budget(&budget)?;
        let _input = budget.reserve("selected occurrence input", BATCH_BYTES)?;
        let selected = selection.await?;
        let mut cursor = selected
            .as_ref()
            .map(|selected| selected.pointers_with_budget(&budget))
            .transpose()?;
        loop {
            if sender.is_closed() {
                return Ok(());
            }
            let mut batch_charge = budget.reserve(
                "selected occurrence pointers",
                BATCH_ROWS * std::mem::size_of::<RecordId>(),
            )?;
            let mut vars = bindings.clone();
            let sql = if let Some(cursor) = &mut cursor {
                let mut nodes = Vec::with_capacity(BATCH_ROWS);
                while nodes.len() < BATCH_ROWS {
                    let Some(row) = cursor.next_row()? else {
                        break;
                    };
                    batch_charge.try_resize(
                        batch_charge
                            .size()
                            .saturating_add(crate::loader::native_bytes(&row)),
                    )?;
                    let Value::Object(row) = row else {
                        return Err(ModelError::Schema("selected occurrence pointer"));
                    };
                    let node = RecordId::from_value(
                        row.get("id")
                            .cloned()
                            .ok_or(ModelError::Schema("selected occurrence node"))?,
                    )
                    .map_err(ModelError::codec)?;
                    if node.table.as_str() == "entity" {
                        nodes.push(node);
                    }
                }
                if nodes.is_empty() {
                    break;
                }
                vars.insert("nodes", nodes);
                format!(
                    "SELECT * FROM {table} WITH INDEX exact_unit_payload WHERE unit_payload IN $nodes AND ({predicate})"
                )
            } else {
                format!("SELECT * FROM {table} WHERE ({predicate})")
            };
            if sender.is_closed() {
                return Ok(());
            }
            let mut rows = crate::prepared::PreparedQuery::new(vars, vec![], vec![sql])?
                .stream_cancellable(&client, cancellation.as_ref())?;
            let result = async {
                let mut context = Vec::new();
                let mut context_charge =
                    charged::StateCharge::new(&budget, "selected-occurrence-context-window");
                while let Some(row) = rows.next().await? {
                    let _scratch = budget.reserve(
                        "selected occurrence decoded scratch",
                        crate::loader::native_bytes(&row).saturating_mul(8),
                    )?;
                    let Value::Object(scoped) = &row else {
                        return Err(ModelError::Schema("selected occurrence object"));
                    };
                    crate::derivation::require_namespace(scoped, namespace)?;
                    if eligible && let Some(selected) = &selected {
                        let Value::Object(object) = &row else {
                            return Err(ModelError::Schema("selected occurrence"));
                        };
                        let dependencies = Vec::<RecordId>::from_value(
                            object
                                .get("dependencies")
                                .cloned()
                                .ok_or(ModelError::Schema("selected occurrence dependencies"))?,
                        )
                        .map_err(ModelError::codec)?;
                        if dependencies.is_empty()
                            || !selected.contains_all_with_budget(&dependencies, &budget)?
                        {
                            continue;
                        }
                    }
                    context_charge.grow(crate::loader::native_bytes(&row).saturating_mul(4))?;
                    context.push(row);
                    if context.len() == BATCH_ROWS {
                        for row in eligible_context_rows(
                            &context_reader,
                            std::mem::take(&mut context),
                            &budget,
                        )
                        .await?
                        {
                            sorted.push(row)?;
                        }
                        context_charge = charged::StateCharge::new(
                            &budget,
                            "selected-occurrence-context-window",
                        );
                    }
                }
                for row in eligible_context_rows(&context_reader, context, &budget).await? {
                    sorted.push(row)?;
                }
                Ok(())
            }
            .await;
            let mut terminal = lctx_model::domain::completion::Completion::default();
            terminal.step(
                "selected occurrence indexed drainage",
                rows.drain_transport().await,
            );
            lctx_model::domain::completion::complete(result, terminal)?;
            if cursor.is_none() {
                break;
            }
        }
        let mut rows = sorted.finish()?;
        while let Some(row) = rows.next_row()? {
            if sender.send(row).await.is_err() {
                break;
            }
        }
        Ok(())
    })
}
#[cfg(test)]
mod occurrence_budget_tests {
    use super::*;
    #[test]
    fn occurrence_binding_decodes_explicit_null_and_strict_nominal_identity() {
        let mut row = Object::new();
        row.insert("binding", Value::Null);
        assert_eq!(context_binding(&row).unwrap(), None);
        let binding: Id<WindowBinding> =
            serde_json::from_value(serde_json::to_value([7u8; 16]).unwrap()).unwrap();
        row.insert("binding", crate_json(binding).unwrap());
        assert_eq!(context_binding(&row).unwrap(), Some(binding));
        row.insert("binding", Value::None);
        assert!(context_binding(&row).is_err());
        row.insert("binding", vec![7i64; 15]);
        assert!(context_binding(&row).is_err());
        row.insert("binding", vec![256i64; 16]);
        assert!(context_binding(&row).is_err());
        row.remove("binding");
        assert!(context_binding(&row).is_err());
    }
    #[test]
    fn lexical_nomination_uses_an_indexed_family_table_without_score_or_quota_filter() {
        for table in &TABLES[..4] {
            let sql = lexical_nomination_sql(table).unwrap();
            assert!(sql.contains(&format!("FROM {table} WITH INDEX lexical")));
            assert!(sql.contains("text @OR@ $query"));
            assert!(!sql.contains("LIMIT"));
            assert!(!sql.contains("score"));
        }
        assert!(lexical_nomination_sql("$documents").is_err());
    }
    #[test]
    fn occurrence_dependencies_require_every_dependency_and_the_unit_payload() {
        let a = RecordId::new("entity", "a");
        let b = RecordId::new("entity", "late-missing");
        let unit = RecordId::new("entity", "unit");
        let mut row = Object::new();
        row.insert("dependencies", vec![a.clone(), a.clone(), b.clone()]);
        row.insert("unit_payload", unit.clone());
        assert_eq!(nomination_dependencies(&row).unwrap(), vec![a, b, unit]);
        row.insert("dependencies", Vec::<RecordId>::new());
        assert!(
            nomination_dependencies(&row).unwrap().is_empty(),
            "empty dependency lists are never admitted solely by their unit"
        );
        row.insert("dependencies", vec![RecordId::new("entity", "a")]);
        row.remove("unit_payload");
        assert!(nomination_dependencies(&row).is_err());
    }
    #[test]
    fn occurrence_window_does_not_promote_one_present_dependency_to_all_present() {
        let budget = resources::ResourceBudget::fixed(1 << 20).unwrap();
        let shared = RecordId::new("entity", "shared");
        let unit = RecordId::new("entity", "unit");
        let present = crate::selection::MembershipAnswers::unrestricted(
            &[shared.clone(), unit.clone()],
            &budget,
        )
        .unwrap();
        let nomination = |dependencies: Vec<RecordId>, unit_payload: RecordId| {
            let mut object = Object::new();
            object.insert("dependencies", dependencies);
            object.insert("unit_payload", unit_payload);
            object
        };
        let good = RecordId::new("lex_occurs", "good");
        let window = BTreeMap::from([
            (good.clone(), nomination(vec![shared.clone()], unit.clone())),
            (
                RecordId::new("lex_occurs", "late-missing"),
                nomination(
                    vec![shared.clone(), RecordId::new("entity", "absent")],
                    unit,
                ),
            ),
            (
                RecordId::new("lex_occurs", "missing-unit"),
                nomination(vec![shared], RecordId::new("entity", "other-unit")),
            ),
        ]);
        assert_eq!(
            eligible_nomination_ids(&window, &present).unwrap(),
            vec![good]
        );
        drop(present);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn occurrence_preparation_refuses_unadmitted_input_before_native_access() {
        let budget = resources::ResourceBudget::fixed(BATCH_BYTES / 2).unwrap();
        let reader = NativeReader::for_views(
            std::sync::Arc::new(crate::surrealdb::Surreal::init()),
            vec![],
        )
        .with_budget(&budget);
        let mut rows =
            selected_occurrences(&reader, "lex_occurs", "true", Variables::new(), &budget).unwrap();
        let error = rows.next().await.unwrap_err();
        assert!(
            matches!(error.primary(), Some(ModelError::Resource { .. })),
            "budget refusal must precede access to the unconnected native client: {error}"
        );
        rows.drain_transport().await.unwrap();
        drop(rows);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn occurrence_driver_cancel_retains_session_and_charges_until_nomination_drainage() {
        use futures::FutureExt;
        let budget = resources::ResourceBudget::fixed(4 << 20).unwrap();
        let client = std::sync::Arc::new(crate::surrealdb::Surreal::init());
        let reader = NativeReader::for_views(client.clone(), vec![]).with_budget(&budget);
        let baseline_handles = std::sync::Arc::strong_count(&client);
        let (entered, started) = tokio::sync::oneshot::channel();
        let (release, gate) = tokio::sync::oneshot::channel();
        let input = crate::reader::NativeRows::owned(move |sender| async move {
            // Empty dependencies deliberately avoid native access. The second send
            // establishes that the actual driver has consumed and charged the first.
            let mut row = Object::new();
            row.insert("dependencies", Vec::<RecordId>::new());
            sender
                .send(Value::Object(row.clone()))
                .await
                .map_err(ModelError::codec)?;
            sender
                .send(Value::Object(row))
                .await
                .map_err(ModelError::codec)?;
            entered
                .send(())
                .map_err(|_| ModelError::Conflict("occurrence gate receiver"))?;
            gate.await.map_err(ModelError::codec)?;
            Ok(())
        })
        .unwrap()
        .with_client(client.clone());
        let mut rows = hydrate_occurrence_nominations(&reader, input, &budget).unwrap();
        started.await.unwrap();
        assert!(
            budget.reserved() > 0,
            "the nomination window is admitted before cancellation"
        );
        assert!(
            reader.close().await.is_err(),
            "the active driver retains the selected reader"
        );
        rows.cancel_delivery();
        let mut drainage = Box::pin(rows.drain_transport());
        assert!(
            drainage.as_mut().now_or_never().is_none(),
            "delivery cancellation cannot acknowledge the gated nomination terminal"
        );
        assert!(budget.reserved() > 0);
        assert!(
            std::sync::Arc::strong_count(&client) > baseline_handles,
            "session handles remain owned through drainage"
        );
        assert!(reader.close().await.is_err());
        release.send(()).unwrap();
        drainage.await.unwrap();
        drop(rows);
        reader.close().await.unwrap();
        assert_eq!(budget.reserved(), 0);
        assert_eq!(std::sync::Arc::strong_count(&client), baseline_handles);
    }

    #[tokio::test]
    #[ignore = "requires the installed stable validation service through just fixture"]
    async fn occurrence_driver_refuses_changed_dependencies_unit_and_missing_payload() {
        use admission::Frontier;
        use completed::ContributionSpec;
        use lctx_compiler_store::compiler::NativeCompilerStore;
        use stages::{Profile, ProviderOutcome};
        let config = lctx_compiler_store::RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("stable validation runtime"),
        ))
        .unwrap();
        assert_eq!(config.database.as_str(), "validation");
        let fixture_budget = resources::ResourceBudget::fixed(64 << 20).unwrap();
        let store = NativeCompilerStore::begin(&config, Frontier::Facts)
            .await
            .unwrap();
        let nonce = store.attempt().hex();
        // A partial typed domain fixture, not publication admission. Only this
        // completed Unit is selected; every mutation below owns a nonce occurrence.
        let key = serde_json::json!(&store.attempt().0[..16]);
        let unit = Unit {
            input: serde_json::from_value(key.clone()).unwrap(),
            context: serde_json::from_value(key.clone()).unwrap(),
            family: Family::ApiOptions,
            origin: serde_json::from_value(key.clone()).unwrap(),
            corpus: serde_json::from_value(key).unwrap(),
            title: format!("occurrence integrity {nonce}").into(),
        };
        let relation = Relation::of::<Unit>();
        let contribution = store
            .begin_contribution(ContributionSpec {
                captured_binding: None,
                producer: "occurrence-driver-control".into(),
                profile: lctx_model::domain::completed::OutputProfile::Specific(Profile::Catalog),
                model: model().unwrap().digest(),
                implementation: ContentHash::of(b"occurrence-driver-control/v1"),
                configuration: None,
                inputs: vec![],
                outputs: BTreeSet::from([Unit::NAME.into()]),
            })
            .await
            .unwrap();
        store
            .write_batch(
                &contribution,
                &relation,
                &Unit::encode(std::slice::from_ref(&unit)).unwrap(),
                &fixture_budget,
            )
            .await
            .unwrap();
        let views = store
            .complete_contribution(
                contribution,
                ProviderOutcome::Complete,
                std::slice::from_ref(&relation),
                &BTreeMap::new(),
                &fixture_budget,
            )
            .await
            .unwrap();
        let descriptors = views.values().cloned().collect::<Vec<_>>();
        let views = descriptors
            .iter()
            .map(|view| view.identity)
            .collect::<Vec<_>>();
        let client = lctx_compiler_store::reader::connect(
            &config.endpoint,
            &config.writer_credentials(),
            store.namespace().as_str(),
            store.database().as_str(),
        )
        .await
        .unwrap();
        let derivation = ContentHash::of(
            format!("occurrence-driver-control/{}", store.attempt().hex()).as_bytes(),
        );
        let loader = Loader::for_attempt_views(client.clone(), store.attempt(), views.clone())
            .with_derivation(derivation)
            .with_budget(&fixture_budget);
        loader
            .entity_references(&[graph::Entity::from(unit.clone())])
            .await
            .unwrap();
        // The dev dependency owns a separately compiled native crate. Its metadata
        // supplies only exact mappings; this crate independently earns its own
        // capability by auditing the actual persisted closure and memberships.
        let installation_audit =
            crate::installed_view::InstallationAudit::new(client.clone(), &fixture_budget)
                .await
                .unwrap();
        let mut installed = Vec::with_capacity(descriptors.len());
        for descriptor in &descriptors {
            let source = store
                .installed_view(descriptor, &fixture_budget)
                .await
                .unwrap();
            let mappings = source
                .completion()
                .mappings
                .iter()
                .map(|mapping| crate::installed_view::ContributorRealization {
                    view: mapping.view,
                    logical: mapping.logical,
                    physical: mapping.physical,
                    incarnation: mapping.incarnation,
                })
                .collect::<Vec<_>>();
            installed.push(std::sync::Arc::new(
                crate::installed_view::select_live_view_scoped(
                    client.clone(),
                    store.attempt(),
                    descriptor,
                    &mappings,
                    &fixture_budget,
                    &installation_audit,
                )
                .await
                .unwrap(),
            ));
        }
        let reader =
            crate::NativeReader::for_installed_views(client.clone(), &installed, &fixture_budget)
                .await
                .unwrap()
                .with_derivation(derivation);
        let payload = typed_payload(&unit).unwrap();
        assert!(
            reader
                .selected_membership(std::slice::from_ref(&payload), &fixture_budget)
                .await
                .unwrap()
                .contains(&payload)
        );
        let document = RecordId::new(
            "search_api_options",
            format!("occurrence-integrity-{nonce}"),
        );
        let ids = ["dependencies", "unit", "missing"].map(|case| {
            RecordId::new("lex_occurs", format!("occurrence-integrity-{nonce}-{case}"))
        });
        let mut doc = Object::new();
        doc.insert("id", document.clone());
        doc.insert("text", format!("occurrence integrity {nonce}"));
        doc.insert("digest", ContentHash::of(nonce.as_bytes()).0.to_vec());
        reader
            .query_native::<Value>(
                "INSERT INTO search_api_options $rows RETURN NONE",
                Variables::from_iter([("rows".into(), vec![Value::Object(doc)].into_value())]),
            )
            .await
            .unwrap();
        let anchor = target_id(Target::Entity(EntityId::of(unit.id())));
        let mut occurrences = Vec::new();
        for id in &ids {
            let mut row = Object::from_value(
                crate::loader::json_value(serde_json::json!({
                    "family": Family::ApiOptions as i16, "unit": unit.id(), "window": unit.id(),
                    "part": unit.id(), "context": unit.context, "input": unit.input,
                    "binding": null, "member": null, "anchor": null,
                    "exact_name": "", "exact_path": "", "exact_option": "", "eligible": true,
                    "occurrence_key": format!("{nonce}-{:?}", id.key),
                }))
                .unwrap(),
            )
            .unwrap();
            row.insert("id", id.clone());
            row.insert("in", document.clone());
            row.insert("out", anchor.clone());
            row.insert("unit_node", anchor.clone());
            row.insert("unit_payload", payload.clone());
            row.insert("dependencies", vec![payload.clone()]);
            occurrences.push(Value::Object(row));
        }
        reader
            .query_native::<Value>(
                "INSERT RELATION INTO lex_occurs $rows RETURN NONE",
                Variables::from_iter([("rows".into(), occurrences.into_value())]),
            )
            .await
            .unwrap();
        let nominations: Vec<Value> = reader
            .query_native(
                "SELECT id,dependencies,unit_payload FROM $nodes ORDER BY id",
                Variables::from_iter([("nodes".into(), ids.to_vec().into_value())]),
            )
            .await
            .unwrap();
        assert_eq!(nominations.len(), 3, "actual pre-mutation nominations");
        for (id, expected) in ids.iter().zip([
            "occurrence hydration dependencies changed",
            "occurrence hydration dependencies changed",
            "missing nominated occurrence payload",
        ]) {
            let nomination = nominations
                .iter()
                .find(|value| {
                    value.as_object().unwrap().get("id") == Some(&Value::RecordId(id.clone()))
                })
                .unwrap()
                .clone();
            let mut vars = Variables::new();
            vars.insert("node", id.clone());
            let mutation = if id == &ids[0] {
                "UPDATE ONLY $node SET dependencies=[] RETURN NONE"
            } else if id == &ids[1] {
                vars.insert(
                    "other",
                    RecordId::new("entity", format!("missing-unit-{nonce}")),
                );
                "UPDATE ONLY $node SET unit_payload=$other RETURN NONE"
            } else {
                "DELETE ONLY $node RETURN NONE"
            };
            reader.query_native::<Value>(mutation, vars).await.unwrap();
            let nominations = crate::reader::NativeRows::owned(move |sender| async move {
                sender.send(nomination).await.map_err(ModelError::codec)?;
                Ok(())
            })
            .unwrap();
            let budget = resources::ResourceBudget::fixed(16 << 20).unwrap();
            let mut rows = hydrate_occurrence_nominations(&reader, nominations, &budget).unwrap();
            let error = rows.next().await.unwrap_err();
            assert!(error.to_string().contains(expected), "{error}");
            assert!(rows.next().await.is_err(), "hydration failures stay sticky");
            rows.drain_transport().await.unwrap();
            drop(rows);
            assert_eq!(
                budget.reserved(),
                0,
                "failed hydration drains all request state"
            );
        }
        reader
            .query_native::<Value>(
                "DELETE $nodes RETURN NONE",
                Variables::from_iter([("nodes".into(), ids.to_vec().into_value())]),
            )
            .await
            .unwrap();
        reader
            .query_native::<Value>(
                "DELETE ONLY $document RETURN NONE",
                Variables::from_iter([("document".into(), document.into_value())]),
            )
            .await
            .unwrap();
        drop(loader);
        reader.close().await.unwrap();
        drop(reader);
        installation_audit.close_drained().await.unwrap();
        client.invalidate().await.unwrap();
        store.abandon().await.unwrap();
    }
}
fn typed_payload<R: Record>(row: &R) -> Result<RecordId, ModelError> {
    let relation = Relation::of::<R>();
    let batch = R::encode(std::slice::from_ref(row))?;
    match crate::adapter::select(R::NAME)?
        .graph(&batch)?
        .into_iter()
        .next()
        .flatten()
    {
        Some(crate::adapter::GraphRow::Entity(row)) => crate::loader::entity_payload_id(&row),
        Some(crate::adapter::GraphRow::Assertion(row)) => crate::loader::assertion_payload_id(&row),
        None => {
            let body = crate::codec::batch_bodies(&relation, &batch)?
                .into_iter()
                .next()
                .ok_or(ModelError::Schema("derived dependency body"))?;
            crate::loader::payload_id(
                "compiler_record",
                R::NAME,
                row.id().bytes(),
                ContentHash::of(&serde_json::to_vec(&body).map_err(ModelError::codec)?),
            )
        }
    }
}
struct Expected {
    budget: resources::ResourceBudget,
    pending: Vec<Option<SortedRows>>,
    ordered: Vec<Option<OrderedRows>>,
}
impl Expected {
    fn new(budget: &resources::ResourceBudget) -> Result<Self, ModelError> {
        Ok(Self {
            budget: budget.clone(),
            pending: (0..7)
                .map(|_| SortedRows::with_budget(budget).map(Some))
                .collect::<Result<_, _>>()?,
            ordered: (0..7).map(|_| None).collect(),
        })
    }
    fn emit(&mut self, table: &str, row: Value) -> Result<(), ModelError> {
        let index = TABLES
            .iter()
            .position(|candidate| *candidate == table)
            .ok_or(ModelError::Schema("derived table"))?;
        self.pending[index]
            .as_mut()
            .ok_or(ModelError::Schema("finished derived family"))?
            .push(row)
    }
    fn finish(&mut self, index: usize) -> Result<&mut OrderedRows, ModelError> {
        if self.ordered[index].is_none() {
            self.ordered[index] = Some(
                self.pending[index]
                    .take()
                    .ok_or(ModelError::Schema("derived ordering"))?
                    .finish()?,
            );
        }
        Ok(self.ordered[index].as_mut().expect("ordered family"))
    }
    async fn reconcile(&mut self, reader: &NativeReader<()>) -> Result<(), ModelError> {
        let budget = self.budget.clone();
        self.reconcile_actual(|| actual_search_rows(reader, &budget)).await
    }
    async fn reconcile_actual(&mut self, actual: impl FnOnce() -> Result<crate::reader::NativeRows, ModelError>) -> Result<(), ModelError> {
        // Every comparison starts at the complete run, including after construction.
        // Cursor errors also drain the actual transport before returning.
        (0..TABLES.len()).try_for_each(|index| self.finish(index).map(drop))?;
        let runs = self.ordered.iter().map(|rows| rows.as_ref().expect("finished search family").prepared());
        let mut actual = retain_expected_until_drained(runs, &self.budget, actual)?;
        let result = async {
            for index in 0..TABLES.len() { self.finish(index)?.rewind()?; }
            let mut indices = (0..TABLES.len()).collect::<Vec<_>>(); indices.sort_by_key(|index| TABLES[*index]);
            for index in indices { while let Some(expected) = self.finish(index)?.next_row()? {
                let Some(row) = actual.next().await? else {
                    tracing::error!(expected_id = ?expected.as_object().and_then(|row| row.get("id")), mismatch = "missing", "derived search reconciliation refused");
                    return Err(ModelError::Serving(lctx_model::domain::serving::FailureKind::Corrupt));
                };
                if serde_json::to_vec(&expected).map_err(ModelError::codec)? != serde_json::to_vec(&row).map_err(ModelError::codec)? {
                    let mut changed_fields = Vec::new();
                    if let (Some(expected), Some(actual)) = (expected.as_object(), row.as_object()) {
                        let fields = expected.keys().chain(actual.keys()).collect::<BTreeSet<_>>();
                        for field in fields {
                            if expected.get(field).is_some() != actual.get(field).is_some() || serde_json::to_vec(&expected.get(field)).map_err(ModelError::codec)? != serde_json::to_vec(&actual.get(field)).map_err(ModelError::codec)? { changed_fields.push(field.to_owned()); }
                        }
                    }
                    tracing::error!(expected_id = ?expected.as_object().and_then(|row| row.get("id")), actual_id = ?row.as_object().and_then(|row| row.get("id")), ?changed_fields, mismatch = "different", "derived search reconciliation refused");
                    return Err(ModelError::Serving(lctx_model::domain::serving::FailureKind::Corrupt));
                }
            } }
            if let Some(row) = actual.next().await? {
                tracing::error!(actual_id = ?row.as_object().and_then(|row| row.get("id")), mismatch = "extra", "derived search reconciliation refused");
                return Err(ModelError::Serving(lctx_model::domain::serving::FailureKind::Corrupt));
            }
            Ok(())
        }.await;
        let mut completion = lctx_model::domain::completion::Completion::default();
        completion.step(
            "derived actual rows transport drainage",
            actual.drain_transport().await,
        );
        lctx_model::domain::completion::complete(result, completion)
    }
    fn into_prepared(mut self) -> Result<[PreparedRows; 7], ModelError> {
        for index in 0..TABLES.len() {
            self.finish(index)?;
        }
        self.ordered.into_iter().map(|rows| {
            rows.ok_or(ModelError::Schema("prepared search family")).map(OrderedRows::into_prepared)
        }).collect::<Result<Vec<_>, _>>()?.try_into()
            .map_err(|_| ModelError::Schema("prepared search family count"))
    }
    fn from_prepared(rows: &[PreparedRows; 7], budget: &resources::ResourceBudget) -> Result<Self, ModelError> {
        Ok(Self {
            budget: budget.clone(),
            pending: (0..7).map(|_| None).collect(),
            ordered: rows.iter().map(|rows| rows.cursor().map(Some)).collect::<Result<_, _>>()?,
        })
    }
}
/// Receiver cancellation does not release a comparison's expected backing before
/// its already admitted actual reads reach their independently checked terminal.
pub(crate) fn retain_expected_until_drained(
    runs: impl ExactSizeIterator<Item = PreparedRows>,
    budget: &resources::ResourceBudget,
    actual: impl FnOnce() -> Result<crate::reader::NativeRows, ModelError>,
) -> Result<crate::reader::NativeRows, ModelError> {
    // No actual stream is admitted until its expected carrier is charged and owned.
    // Refusal is synchronous; there is no cancellation window around a refusal drain.
    let charge = budget.reserve("search comparison retained expectations", runs.len().saturating_mul(std::mem::size_of::<PreparedRows>()).saturating_add(256))?;
    let runs = runs.collect::<Vec<_>>();
    let mut actual = actual()?;
    crate::reader::NativeRows::owned(move |sender| async move {
        let result = async {
            while let Some(row) = actual.next().await? {
                if sender.send(row).await.is_err() { break; }
            }
            Ok(())
        }.await;
        let mut terminal = completion::Completion::default();
        terminal.step("search expectation borrower actual drainage", actual.drain_transport().await);
        let result = completion::complete(result, terminal);
        drop(runs);
        drop(charge);
        result
    })
}
pub fn actual_search_rows<Context>(
    reader: &NativeReader<Context>,
    budget: &resources::ResourceBudget,
) -> Result<crate::reader::NativeRows, ModelError> {
    // Shared unit/window payloads can have companions from other exact views. Admit
    // only complete selected lineages; independent cold lowering still refuses missing
    // expected occurrences, changed payloads, and unexpected eligible occurrences.
    let namespace = reader.derivation()?;
    let budget = budget.clone();
    let mut lexical =
        selected_occurrences(reader, "lex_occurs", "true", Variables::new(), &budget)?;
    let mut vectors =
        selected_occurrences(reader, "vec_occurs", "true", Variables::new(), &budget)?;
    let client = reader.shared_client();
    let cancellation = reader.read_cancellation();
    crate::reader::NativeRows::owned(move |sender| async move {
        let mut actual = SortedRows::with_budget(&budget)?;
        let mut documents = SortedRows::with_budget(&budget)?;
        for rows in [&mut lexical, &mut vectors] {
            let result = async {
                while let Some(row) = rows.next().await? {
                    let Value::Object(object) = &row else {
                        return Err(ModelError::Schema("actual search occurrence"));
                    };
                    let document = RecordId::from_value(
                        object
                            .get("in")
                            .cloned()
                            .ok_or(ModelError::Schema("actual search document"))?,
                    )
                    .map_err(ModelError::codec)?;
                    let mut pointer = Object::new();
                    pointer.insert("id", document);
                    documents.push(Value::Object(pointer))?;
                    crate::derivation::require_namespace(object, namespace)?;
                    actual.push(row)?;
                }
                Ok(())
            }
            .await;
            let mut terminal = lctx_model::domain::completion::Completion::default();
            terminal.step(
                "actual search occurrence drainage",
                rows.drain_transport().await,
            );
            lctx_model::domain::completion::complete(result, terminal)?;
        }
        let mut documents = documents.finish()?;
        loop {
            let mut ids = Vec::new();
            while ids.len() < BATCH_ROWS {
                let Some(row) = documents.next_row()? else {
                    break;
                };
                let Value::Object(row) = row else {
                    return Err(ModelError::Schema("actual search document pointer"));
                };
                ids.push(
                    RecordId::from_value(
                        row.get("id")
                            .cloned()
                            .ok_or(ModelError::Schema("actual search document id"))?,
                    )
                    .map_err(ModelError::codec)?,
                );
            }
            if ids.is_empty() {
                break;
            }
            let mut vars = Variables::new();
            vars.insert("ids", ids);
            let mut rows = crate::prepared::PreparedQuery::new(
                vars,
                vec![],
                vec!["SELECT * FROM $ids".into()],
            )?
            .stream_cancellable(&client, cancellation.as_ref())?;
            let result = async {
                while let Some(row) = rows.next().await? {
                    let Value::Object(object) = &row else {
                        return Err(ModelError::Schema("actual search companion object"));
                    };
                    crate::derivation::require_namespace(object, namespace)?;
                    actual.push(row)?;
                }
                Ok(())
            }
            .await;
            let mut terminal = lctx_model::domain::completion::Completion::default();
            terminal.step(
                "actual search document drainage",
                rows.drain_transport().await,
            );
            lctx_model::domain::completion::complete(result, terminal)?;
        }
        let mut actual = actual.finish()?;
        while let Some(row) = actual.next_row()? {
            if sender.send(row).await.is_err() {
                break;
            }
        }
        Ok(())
    })
}

struct Batch<'a> {
    loader: Option<&'a Loader>,
    table: &'static str,
    rows: Vec<Value>,
    bytes: usize,
}
impl<'a> Batch<'a> {
    fn new(loader: Option<&'a Loader>, table: &'static str) -> Self {
        Self {
            loader,
            table,
            rows: Vec::new(),
            bytes: 0,
        }
    }
    async fn emit(&mut self, row: Value) -> Result<(), ModelError> {
        if self.loader.is_none() {
            return Ok(());
        }
        let bytes = serde_json::to_vec(&row).map_err(ModelError::codec)?.len();
        if bytes > BATCH_BYTES {
            return Err(ModelError::Limit {
                owner: "native-search-write",
                limit: "value bytes",
                observed: bytes,
                bound: BATCH_BYTES,
            });
        }
        if self.rows.len() >= BATCH_ROWS || self.bytes.saturating_add(bytes) > BATCH_BYTES {
            self.flush().await?;
        }
        self.bytes += bytes;
        self.rows.push(row);
        Ok(())
    }
    async fn flush(&mut self) -> Result<(), ModelError> {
        if self.rows.is_empty() {
            return Ok(());
        }
        let loader = self
            .loader
            .ok_or(ModelError::Schema("derived write owner"))?;
        loader
            .insert(
                self.table,
                std::mem::take(&mut self.rows),
                self.table.ends_with("occurs"),
            )
            .await?;
        self.bytes = 0;
        Ok(())
    }
}
const FRONTIER_ROWS: usize = 64;
type Index<R> = BTreeMap<Id<R>, R>;
fn need<R: Record>(index: &Index<R>, id: Id<R>) -> Result<&R, ModelError> {
    index
        .get(&id)
        .ok_or(ModelError::Schema("canonical search companion"))
}
async fn keyed<R: Record + serde::de::DeserializeOwned>(
    reader: &impl SearchCanonicalRead,
    ids: impl IntoIterator<Item = Id<R>>,
) -> Result<Index<R>, ModelError> {
    let ids = ids
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let mut result = BTreeMap::new();
    for chunk in ids.chunks(FRONTIER_ROWS) {
        for row in context_records::<R>(
            reader,
            RecordSelection::Keys(chunk.iter().map(|id| *id.bytes()).collect()),
        )
        .await?
        {
            if !chunk.contains(&row.id()) || result.insert(row.id(), row).is_some() {
                return Err(ModelError::Conflict("canonical companion identity"));
            }
        }
    }
    if result.len() != ids.len() {
        return Err(ModelError::Schema("canonical search companion"));
    }
    Ok(result)
}
async fn scoped<R: Record + serde::de::DeserializeOwned, T: serde::Serialize>(
    reader: &impl SearchCanonicalRead,
    field: &str,
    ids: impl IntoIterator<Item = T>,
) -> Result<Index<R>, ModelError> {
    let values = ids
        .into_iter()
        .map(serde_json::to_value)
        .collect::<Result<Vec<_>, _>>()
        .map_err(ModelError::codec)?;
    let rows = context_records::<R>(
        reader,
        RecordSelection::Scope {
            field: field.into(),
            values,
        },
    )
    .await?;
    let mut result = BTreeMap::new();
    for row in rows {
        if result.insert(row.id(), row).is_some() {
            return Err(ModelError::Conflict("canonical scoped companion"));
        }
    }
    Ok(result)
}
async fn fill<R: Record + serde::de::DeserializeOwned>(
    reader: &impl SearchCanonicalRead,
    index: &mut Index<R>,
    ids: impl IntoIterator<Item = Id<R>>,
) -> Result<(), ModelError> {
    let missing = ids
        .into_iter()
        .filter(|id| !index.contains_key(id))
        .collect::<BTreeSet<_>>();
    index.extend(keyed(reader, missing).await?);
    Ok(())
}
struct Basic {
    windows: Index<SearchWindow>,
    units: Index<Unit>,
    links: Index<WindowPart>,
    parts: Index<ContentPart>,
}
impl Basic {
    async fn load(
        reader: &impl SearchCanonicalRead,
        windows: Index<SearchWindow>,
    ) -> Result<Self, ModelError> {
        let units = keyed(reader, windows.values().map(|w| w.unit)).await?;
        let links = scoped::<WindowPart, _>(reader, "window", windows.keys().copied()).await?;
        let parts = keyed(reader, links.values().map(|link| link.part)).await?;
        for link in links.values() {
            if need(&windows, link.window)?.unit != need(&parts, link.part)?.unit {
                return Err(ModelError::Conflict("window part unit"));
            }
        }
        Ok(Self {
            windows,
            units,
            links,
            parts,
        })
    }
    fn primary(&self, window: Id<SearchWindow>) -> impl Iterator<Item = &ContentPart> {
        self.links
            .values()
            .filter(move |link| link.window == window)
            .filter_map(|link| self.parts.get(&link.part))
            .filter(|part| part.purpose == PartPurpose::Primary)
    }
}
#[derive(serde::Serialize)]
struct LoweredProjection {
    dependencies: Vec<RecordId>,
    id: Id<ProjectedValue>,
    value: Id<FullValue>,
    encoder: Id<EmbeddingSpec>,
    encoder_hash: ContentHash,
    policy: Id<ProjectionDefinition>,
    input: ContentHash,
    tokens: i64,
    embedding: Vec<f32>,
}
fn vector_id(
    projection: &LoweredProjection,
    unit: &Unit,
    namespace: ContentHash,
) -> Result<RecordId, ModelError> {
    Ok(crate::derivation::address(
        namespace,
        "vector",
        &serde_json::to_vec(&("native-vector/v2", projection, unit.input, unit.family))
            .map_err(ModelError::codec)?,
    ))
}
fn cohort_row(
    p: &LoweredProjection,
    unit: &Unit,
    namespace: ContentHash,
) -> Result<Value, ModelError> {
    let mut row = Object::new();
    row.insert("id", vector_id(p, unit, namespace)?);
    row.insert("derivation", namespace.hex());
    row.insert("dependencies", p.dependencies.clone());
    row.insert("encoder_hash", p.encoder_hash.hex());
    row.insert("policy_key", p.policy.hex());
    row.insert("library_input", scope_string(&crate_json(unit.input)?));
    row.insert("family", unit.family as i16);
    row.insert("full_key", p.value.hex());
    row.insert("projection_key", p.id.hex());
    row.insert("embedding", p.embedding.clone());
    Ok(Value::Object(row))
}
async fn lower_vectors(
    reader: &NativeReader<()>,
    expected: &mut Expected,
) -> Result<(), ModelError> {
    let mut uses = reader.record_stream::<RetrievalEmbeddingUse>(
        "body.availability=0",
        Variables::new(),
        "body.projection,body.window,semantic_key",
    )?;
    let mut encoders = Index::new();
    let mut policies = Index::new();
    let mut last: Option<LoweredProjection> = None;
    loop {
        let mut batch = Vec::new();
        while batch.len() < FRONTIER_ROWS {
            let Some(row) = uses.next().await? else { break };
            batch.push(row);
        }
        if batch.is_empty() {
            break;
        }
        let windows = keyed(reader, batch.iter().map(|row| row.window)).await?;
        let basic = Basic::load(reader, windows).await?;
        let ids = batch
            .iter()
            .map(|row| {
                row.projection
                    .ok_or(ModelError::Schema("available projection"))
            })
            .collect::<Result<BTreeSet<_>, _>>()?;
        let new = keyed::<ProjectedValue>(
            reader,
            ids.into_iter()
                .filter(|id| last.as_ref().is_none_or(|last| last.id != *id)),
        )
        .await?;
        let full = keyed(reader, new.values().map(|p| p.value)).await?;
        fill(reader, &mut encoders, full.values().map(|v| v.encoder)).await?;
        fill(reader, &mut policies, new.values().map(|p| p.definition)).await?;
        let mut prepared = BTreeMap::new();
        for p in new.values() {
            let full = need(&full, p.value)?;
            let encoder = need(&encoders, full.encoder)?;
            let policy = need(&policies, p.definition)?;
            full.verify_encoder(encoder)?;
            p.verify(full, policy)?;
            if full.dimensions != 4096 || p.dimensions != 1024 {
                return Err(ModelError::Schema("native full4096/projection1024"));
            }
            prepared.insert(
                p.id(),
                LoweredProjection {
                    dependencies: vec![
                        typed_payload(p)?,
                        typed_payload(full)?,
                        typed_payload(encoder)?,
                        typed_payload(policy)?,
                    ],
                    id: p.id(),
                    value: full.id(),
                    encoder: full.encoder,
                    encoder_hash: encoder.service_hash,
                    policy: policy.id(),
                    input: full.input,
                    tokens: full.tokens,
                    embedding: p.values()?,
                },
            );
        }
        for consumed in batch {
            let id = consumed
                .projection
                .ok_or(ModelError::Schema("available projection"))?;
            if last.as_ref().is_none_or(|old| old.id != id) {
                if last.as_ref().is_some_and(|old| old.id > id) {
                    return Err(ModelError::Conflict("canonical projection order"));
                }
                last = Some(
                    prepared
                        .remove(&id)
                        .ok_or(ModelError::Schema("prepared canonical projection"))?,
                );
            }
            let p = last.as_ref().expect("prepared projection");
            let window = need(&basic.windows, consumed.window)?;
            let unit = need(&basic.units, window.unit)?;
            if consumed.value != Some(p.value)
                || consumed.specification != p.encoder
                || consumed.input != p.input
                || consumed.admitted_tokens != Some(p.tokens)
                || window.encoded_digest != p.input
            {
                return Err(ModelError::Conflict(
                    "retrieval canonical winner references",
                ));
            }
            if basic.primary(window.id()).next().is_some() {
                expected.emit("vector", cohort_row(p, unit, reader.derivation()?)?)?;
            }
        }
    }
    Ok(())
}
async fn window_batch(
    stream: &mut crate::reader::CanonicalRecords<SearchWindow>,
) -> Result<Index<SearchWindow>, ModelError> {
    let mut windows = BTreeMap::new();
    while windows.len() < FRONTIER_ROWS {
        let Some(window) = stream.next().await? else {
            break;
        };
        windows.insert(window.id(), window);
    }
    Ok(windows)
}
struct Companions {
    bindings: Index<WindowBinding>,
    subjects: Index<Subject>,
    origins: Index<Origin>,
    options: Index<catalog::CatalogOption>,
    members: Index<catalog::CatalogMember>,
    artifacts: Index<source::SourceArtifact>,
    names: BTreeMap<Id<catalog::CatalogOption>, OptionName>,
    original_rows: Index<OriginalAnchor>,
    maps: Index<WindowSourceMap>,
    vectors: BTreeMap<(String, String, i16), (RecordId, Vec<RecordId>)>,
    anchors: BTreeMap<(Id<SearchWindow>, Id<ContentPart>), Id<OriginalAnchor>>,
    uses: Index<RetrievalEmbeddingUse>,
}
impl Companions {
    async fn witness_context(
        reader: &impl SearchCanonicalRead,
        b: &Basic,
    ) -> Result<Self, ModelError> {
        let bindings =
            scoped::<WindowBinding, _>(reader, "window", b.windows.keys().copied()).await?;
        let subjects = keyed(reader, bindings.values().map(|r| r.subject)).await?;
        let origins = keyed(reader, b.units.values().map(|u| u.origin)).await?;
        let options = keyed(
            reader,
            subjects.values().filter_map(|s| {
                if let Subject::Option { option } = s {
                    Some(*option)
                } else {
                    None
                }
            }),
        )
        .await?;
        let mut member_ids = subjects
            .values()
            .filter_map(|s| {
                if let Subject::Member { member } = s {
                    Some(*member)
                } else {
                    None
                }
            })
            .collect::<BTreeSet<_>>();
        member_ids.extend(options.values().map(|o| o.member));
        member_ids.extend(origins.values().filter_map(|o| {
            if let Origin::Definition { member, .. } = o {
                Some(*member)
            } else {
                None
            }
        }));
        let members = keyed(reader, member_ids).await?;
        let artifacts = keyed(
            reader,
            subjects.values().filter_map(|s| {
                if let Subject::Source { artifact } = s {
                    Some(*artifact)
                } else {
                    None
                }
            }),
        )
        .await?;
        let names = option_names(reader, &options).await?;
        let maps =
            scoped::<WindowSourceMap, _>(reader, "window", b.windows.keys().copied()).await?;
        let originals = maps
            .values()
            .filter(|map| {
                map.part.is_some_and(|part| {
                    b.parts
                        .get(&part)
                        .is_some_and(|p| p.purpose == PartPurpose::Primary)
                })
            })
            .filter_map(|map| map.original)
            .collect::<BTreeSet<_>>();
        let mut vars = Variables::new();
        vars.insert("originals", crate_json(&originals)?);
        let scopes = b
            .units
            .keys()
            .map(|id| {
                Ok(format!(
                    "{}|unit|{}",
                    OriginalAnchor::NAME,
                    scope_string(&crate_json(id)?)
                ))
            })
            .collect::<Result<Vec<_>, ModelError>>()?;
        vars.insert("scopes", scopes);
        vars.insert("type", OriginalAnchor::NAME.to_owned());
        let rows = collect_context(
            reader,
            reader.search_candidates::<OriginalAnchor>(crate::prepared::PreparedQuery::new(
                vars,
                vec![],
                vec![original_anchor_nomination_sql()?],
            )?)?,
        )
        .await?;
        let mut anchors = BTreeMap::new();
        let mut original_rows = Index::new();
        for anchor in rows {
            original_rows.insert(anchor.id(), anchor.clone());
            anchors
                .entry((anchor.unit, anchor.original))
                .and_modify(|id: &mut Id<OriginalAnchor>| *id = (*id).min(anchor.id()))
                .or_insert(anchor.id());
        }
        // Map each actual part/window to a supporting anchor, without a unit-anchor cross product.
        let mut supports = BTreeMap::new();
        for map in maps.values() {
            if let (Some(part), Some(original)) = (map.part, map.original) {
                let unit = need(&b.parts, part)?.unit;
                if let Some(anchor) = anchors.get(&(unit, original)) {
                    supports
                        .entry((map.window, part))
                        .and_modify(|id: &mut Id<OriginalAnchor>| *id = (*id).min(*anchor))
                        .or_insert(*anchor);
                }
            }
        }
        Ok(Self {
            bindings,
            subjects,
            origins,
            options,
            members,
            artifacts,
            names,
            original_rows,
            maps,
            anchors: supports,
            vectors: BTreeMap::new(),
            uses: Index::new(),
        })
    }
    async fn load(reader: &NativeReader<()>, b: &Basic) -> Result<Self, ModelError> {
        let mut context = Self::witness_context(reader, b).await?;
        let uses: Index<RetrievalEmbeddingUse> =
            scoped(reader, "window", b.windows.keys().copied()).await?;
        let projections = uses
            .values()
            .filter_map(|row| row.projection.map(|id| id.hex()))
            .collect::<BTreeSet<_>>();
        let selected = reader.prepare_selection().await?;
        let mut values = Vec::new();
        for projection in projections {
            let mut vars = Variables::new();
            vars.insert("projection", projection);
            let mut rows = reader.query_stream("SELECT id,projection_key,library_input,family,dependencies FROM vector WITH INDEX cohort WHERE derivation=$lctx_derivation AND projection_key=$projection", vars, 1)?;
            let result = async {
                while let Some(row) = rows.next().await? {
                    let object = Object::from_value(row).map_err(ModelError::codec)?;
                    if let Some(selected) = &selected {
                        let dependencies = Vec::<RecordId>::from_value(
                            object
                                .get("dependencies")
                                .cloned()
                                .ok_or(ModelError::Schema("vector dependency"))?,
                        )
                        .map_err(ModelError::codec)?;
                        if dependencies.is_empty() || !selected.contains_all(&dependencies)? {
                            continue;
                        }
                    }
                    values.push(object);
                }
                Ok(())
            }
            .await;
            let mut terminal = lctx_model::domain::completion::Completion::default();
            terminal.step(
                "vector companion cohort drainage",
                rows.drain_transport().await,
            );
            lctx_model::domain::completion::complete(result, terminal)?;
        }
        let mut vectors = BTreeMap::new();
        for row in values {
            let (
                Some(Value::RecordId(id)),
                Some(Value::String(projection)),
                Some(Value::String(input)),
                Some(Value::Number(surrealdb::types::Number::Int(family))),
                Some(Value::Array(dependencies)),
            ) = (
                row.get("id"),
                row.get("projection_key"),
                row.get("library_input"),
                row.get("family"),
                row.get("dependencies"),
            )
            else {
                return Err(ModelError::Schema("exact vector cohort"));
            };
            let dependencies = dependencies
                .iter()
                .map(|value| {
                    if let Value::RecordId(id) = value {
                        Ok(id.clone())
                    } else {
                        Err(ModelError::Schema("vector dependency"))
                    }
                })
                .collect::<Result<Vec<_>, _>>()?;
            if vectors
                .insert(
                    (projection.clone(), input.clone(), *family as i16),
                    (id.clone(), dependencies),
                )
                .is_some()
            {
                return Err(ModelError::Conflict("competing exact vector cohort"));
            }
        }

        context.vectors = vectors;
        context.uses = uses;
        Ok(context)
    }
}
struct OptionName {
    value: String,
    dependencies: Vec<RecordId>,
}
async fn option_names(
    reader: &impl SearchCanonicalRead,
    options: &Index<catalog::CatalogOption>,
) -> Result<BTreeMap<Id<catalog::CatalogOption>, OptionName>, ModelError> {
    use catalog::CatalogOptionSubject as Subject;
    use normalized::{
        callables::SignatureSlot,
        entities::{FieldEntity, ParameterEntity, ParameterEntityLink},
    };
    let subjects = keyed(reader, options.values().map(|o| o.subject)).await?;
    let fields = keyed::<FieldEntity>(
        reader,
        subjects.values().filter_map(|s| {
            if let Subject::Field { field } = s {
                Some(*field)
            } else {
                None
            }
        }),
    )
    .await?;
    let slots = keyed::<SignatureSlot>(
        reader,
        subjects.values().filter_map(|s| {
            if let Subject::Parameter { slot } = s {
                Some(*slot)
            } else {
                None
            }
        }),
    )
    .await?;
    let entities = keyed::<ParameterEntity>(
        reader,
        subjects.values().filter_map(|s| {
            if let Subject::SourceParameter { parameter } = s {
                Some(*parameter)
            } else {
                None
            }
        }),
    )
    .await?;
    let links =
        scoped::<ParameterEntityLink, _>(reader, "entity", entities.keys().copied()).await?;
    let mut ids = slots.values().map(|s| s.parameter).collect::<BTreeSet<_>>();
    ids.extend(links.values().map(|l| l.parameter));
    ids.extend(entities.values().filter_map(|e| {
        if let ParameterEntity::NativeSlot { parameter, .. } = e {
            Some(*parameter)
        } else {
            None
        }
    }));
    let parameters = keyed::<calls::SignatureParameter>(reader, ids).await?;
    let shapes =
        keyed::<calls::ParameterShape>(reader, parameters.values().map(|p| p.shape)).await?;
    let name = |id| -> Result<String, ModelError> {
        Ok(need(&shapes, need(&parameters, id)?.shape)?
            .name
            .as_ref()
            .map(|s| s.as_str().to_owned())
            .unwrap_or_default())
    };
    let mut result = BTreeMap::new();
    for option in options.values() {
        let mut dependencies = vec![typed_payload(need(&subjects, option.subject)?)?];
        let mut parameter_name = |id| -> Result<String, ModelError> {
            let parameter = need(&parameters, id)?;
            let shape = need(&shapes, parameter.shape)?;
            dependencies.push(typed_payload(parameter)?);
            dependencies.push(typed_payload(shape)?);
            name(id)
        };
        let value = match need(&subjects, option.subject)? {
            Subject::Field { field } => {
                let field = need(&fields, *field)?;
                dependencies.push(typed_payload(field)?);
                field.name.as_str().to_owned()
            }
            Subject::Parameter { slot } => {
                let slot = need(&slots, *slot)?;
                let parameter = slot.parameter;
                let slot_payload = typed_payload(slot)?;
                let value = parameter_name(parameter)?;
                dependencies.push(slot_payload);
                value
            }
            Subject::SourceParameter { parameter } => match need(&entities, *parameter)? {
                ParameterEntity::NativeSlot { parameter: id, .. } => {
                    let value = parameter_name(*id)?;
                    dependencies.push(typed_payload(need(&entities, *parameter)?)?);
                    value
                }
                ParameterEntity::Source { .. } => {
                    let mut names = BTreeSet::new();
                    for link in links.values().filter(|link| link.entity == *parameter) {
                        let n = parameter_name(link.parameter)?;
                        // Dependency accumulation happens after the closure borrow ends.
                        if !n.is_empty() {
                            names.insert(n);
                        }
                    }
                    if names.len() > 1 {
                        return Err(ModelError::Conflict(
                            "source option has competing declared names",
                        ));
                    }
                    names.into_iter().next().unwrap_or_default()
                }
            },
        };
        if let Subject::SourceParameter { parameter } = need(&subjects, option.subject)? {
            dependencies.push(typed_payload(need(&entities, *parameter)?)?);
            for link in links.values().filter(|link| link.entity == *parameter) {
                dependencies.push(typed_payload(link)?);
            }
        }
        dependencies.sort();
        dependencies.dedup();
        result.insert(
            option.id(),
            OptionName {
                value,
                dependencies,
            },
        );
    }
    Ok(result)
}
async fn lower(
    reader: &NativeReader<()>,
    expected: &mut Expected,
    loader: Option<&Loader>,
) -> Result<(), ModelError> {
    lower_vectors(reader, expected).await?;
    let mut windows =
        reader.record_stream::<SearchWindow>("true", Variables::new(), "semantic_key")?;
    loop {
        let batch = window_batch(&mut windows).await?;
        if batch.is_empty() {
            break;
        }
        let b = Basic::load(reader, batch).await?;
        for window in b.windows.values() {
            if b.primary(window.id()).next().is_none() {
                continue;
            }
            let unit = need(&b.units, window.unit)?;
            let mut row = Object::new();
            row.insert(
                "id",
                crate::derivation::address(
                    reader.derivation()?,
                    table(unit.family),
                    &window.digest.0,
                ),
            );
            row.insert("derivation", reader.derivation()?.hex());
            row.insert("text", window.text.as_str().to_owned());
            row.insert("digest", crate_json(window.digest)?);
            row.insert(
                "scope_digest",
                scope_string(row.get("digest").expect("digest")),
            );
            expected.emit(table(unit.family), Value::Object(row))?;
        }
    }
    for (index, table) in TABLES[..5].iter().enumerate() {
        let rows = expected.finish(index)?;
        if loader.is_some() {
            let mut batch = Batch::new(loader, table);
            while let Some(row) = rows.next_row()? {
                batch.emit(row).await?;
            }
            batch.flush().await?;
            rows.rewind()?;
        }
    }
    let mut lexical = Batch::new(loader, "lex_occurs");
    let mut vectors = Batch::new(loader, "vec_occurs");
    let mut windows =
        reader.record_stream::<SearchWindow>("true", Variables::new(), "semantic_key")?;
    loop {
        let batch = window_batch(&mut windows).await?;
        if batch.is_empty() {
            break;
        }
        let b = Basic::load(reader, batch).await?;
        let c = Companions::load(reader, &b).await?;
        for window in b.windows.values() {
            let unit = need(&b.units, window.unit)?;
            for part in b.primary(window.id()) {
                let bindings = c
                    .bindings
                    .values()
                    .filter(|binding| binding.window == window.id() && binding.part == part.id())
                    .collect::<Vec<_>>();
                if bindings.is_empty() {
                    emit_witness(
                        reader.derivation()?,
                        expected,
                        &mut lexical,
                        &mut vectors,
                        unit,
                        window,
                        part,
                        None,
                        &c,
                        &b,
                    )
                    .await?;
                } else {
                    for binding in bindings {
                        emit_witness(
                            reader.derivation()?,
                            expected,
                            &mut lexical,
                            &mut vectors,
                            unit,
                            window,
                            part,
                            Some(binding),
                            &c,
                            &b,
                        )
                        .await?;
                    }
                }
            }
        }
    }
    lexical.flush().await?;
    vectors.flush().await?;
    Ok(())
}
fn witness_context<'a>(
    unit: &'a Unit,
    window: &'a SearchWindow,
    part: &'a ContentPart,
    binding: Option<&WindowBinding>,
    c: &Companions,
    b: &Basic,
) -> Result<(Witness<'a>, Vec<RecordId>), ModelError> {
    let mut option_key = String::new();
    let mut source_path = String::new();
    let member = if let Some(binding) = binding {
        match need(&c.subjects, binding.subject)? {
            Subject::Member { member } => Some(*member),
            Subject::Option { option } => {
                option_key = c
                    .names
                    .get(option)
                    .ok_or(ModelError::Schema("option name"))?
                    .value
                    .clone();
                Some(need(&c.options, *option)?.member)
            }
            Subject::Definition { entity } => match need(&c.origins, unit.origin)? {
                Origin::Definition {
                    member,
                    entity: owner,
                } if owner == entity => Some(*member),
                _ => None,
            },
            Subject::Source { artifact } => {
                source_path = need(&c.artifacts, *artifact)?.path.clone();
                None
            }
            _ => None,
        }
    } else {
        None
    };
    let (name, path) = if let Some(id) = member {
        let member = need(&c.members, id)?;
        if member.input != unit.input {
            return Err(ModelError::Conflict("foreign primary member input"));
        }
        (
            member.path.last().cloned().unwrap_or_default(),
            member.name.clone(),
        )
    } else {
        (String::new(), source_path)
    };
    let anchor = c.anchors.get(&(window.id(), part.id())).copied();
    let mut dependencies = vec![
        typed_payload(unit)?,
        typed_payload(window)?,
        typed_payload(part)?,
        typed_payload(need(&c.origins, unit.origin)?)?,
    ];
    for link in b
        .links
        .values()
        .filter(|link| link.window == window.id() && link.part == part.id())
    {
        dependencies.push(typed_payload(link)?);
    }
    if let Some(binding) = binding {
        dependencies.push(typed_payload(binding)?);
        let subject = need(&c.subjects, binding.subject)?;
        dependencies.push(typed_payload(subject)?);
        match subject {
            Subject::Option { option } => {
                dependencies.push(typed_payload(need(&c.options, *option)?)?);
                dependencies.extend(
                    c.names
                        .get(option)
                        .ok_or(ModelError::Schema("option name dependencies"))?
                        .dependencies
                        .clone(),
                );
            }
            Subject::Source { artifact } => {
                dependencies.push(typed_payload(need(&c.artifacts, *artifact)?)?)
            }
            _ => {}
        }
    }
    if let Some(member) = member {
        dependencies.push(typed_payload(need(&c.members, member)?)?);
    }
    if let Some(anchor) = anchor {
        dependencies.push(typed_payload(need(&c.original_rows, anchor)?)?);
        for map in c
            .maps
            .values()
            .filter(|map| map.window == window.id() && map.part == Some(part.id()))
        {
            dependencies.push(typed_payload(map)?);
        }
    }
    dependencies.sort();
    dependencies.dedup();
    let witness = Witness {
        unit,
        window,
        part,
        binding: binding.map(Record::id),
        member,
        anchor,
        name,
        path,
        option_key,
    };
    Ok((witness, dependencies))
}
#[allow(
    clippy::too_many_arguments,
    reason = "Canonical lineage and distinct physical writers remain explicit"
)]
async fn emit_witness(
    namespace: ContentHash,
    expected: &mut Expected,
    lexical: &mut Batch<'_>,
    vectors: &mut Batch<'_>,
    unit: &Unit,
    window: &SearchWindow,
    part: &ContentPart,
    binding: Option<&WindowBinding>,
    c: &Companions,
    b: &Basic,
) -> Result<(), ModelError> {
    let (witness, dependencies) = witness_context(unit, window, part, binding, c, b)?;
    let mut identity = KeySink::new("native-search-occurrence/v3");
    identity.part(
        b"dependencies",
        &serde_json::to_vec(&dependencies).map_err(ModelError::codec)?,
    );
    identity.part(
        b"names",
        &serde_json::to_vec(&(&witness.name, &witness.path, &witness.option_key))
            .map_err(ModelError::codec)?,
    );
    unit.id().encode(&mut identity);
    window.id().encode(&mut identity);
    part.id().encode(&mut identity);
    witness.binding.encode(&mut identity);
    let key = identity.finish().hex();
    let row = occurrence(
        "lex_occurs",
        namespace,
        &key,
        crate::derivation::address(namespace, table(unit.family), &window.digest.0),
        &witness,
        &dependencies,
    )?;
    expected.emit("lex_occurs", row.clone())?;
    lexical.emit(row).await?;
    for consumed in c.uses.values().filter(|use_| {
        use_.window == window.id()
            && use_.availability == embedding::analytic::VectorAvailability::Available
    }) {
        let projection = consumed
            .projection
            .ok_or(ModelError::Schema("available projection"))?;
        let (vector, vector_dependencies) = c
            .vectors
            .get(&(
                projection.hex(),
                scope_string(&crate_json(unit.input)?),
                unit.family as i16,
            ))
            .ok_or(ModelError::Schema("selected exact vector cohort"))?;
        let mut vector_dependencies = vector_dependencies.clone();
        vector_dependencies.extend(dependencies.clone());
        vector_dependencies.push(typed_payload(consumed)?);
        vector_dependencies.sort();
        vector_dependencies.dedup();
        let vector_key = ContentHash::of(
            &serde_json::to_vec(&(&key, &vector_dependencies, vector))
                .map_err(ModelError::codec)?,
        )
        .hex();
        let row = occurrence(
            "vec_occurs",
            namespace,
            &vector_key,
            vector.clone(),
            &witness,
            &vector_dependencies,
        )?;
        expected.emit("vec_occurs", row.clone())?;
        vectors.emit(row).await?;
    }
    Ok(())
}
#[derive(Debug)]
struct SearchPhaseFailure {
    phase: &'static str,
    cause: ModelError,
}
impl std::fmt::Display for SearchPhaseFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.phase, self.cause)
    }
}
impl std::error::Error for SearchPhaseFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.cause)
    }
}
fn phase<T>(phase: &'static str, result: Result<T, ModelError>) -> Result<T, ModelError> {
    result.map_err(|cause| {
        let mut completion = completion::Completion::default();
        if !cause.permits_storage_cleanup() {
            completion.remote = completion::RemoteState::Unknown;
        }
        completion::complete::<()>(
            Err(ModelError::Cause(Box::new(SearchPhaseFailure {
                phase,
                cause,
            }))),
            completion,
        )
        .unwrap_err()
    })
}
pub async fn materialize_search(loader: &Loader) -> Result<(), ModelError> {
    let reader = reader(loader)?;
    let budget = loader.read_budget()?;
    let mut expected = Expected::new(&budget)?;
    phase(
        "derived search canonical lowering",
        lower(&reader, &mut expected, Some(loader)).await,
    )?;
    phase(
        "derived search actual-row reconciliation",
        expected.reconcile(&reader).await,
    )?;
    phase(
        "frozen lexical statistics construction",
        crate::lexical_stats::materialize(loader).await,
    )
}
/// Opaque, in-process preparation belonging to one actual staging Loader. It is
/// neither serializable nor constructible from a realization or completed descriptor.
/// The publisher retains this alongside its admitted input and session through seal.
pub struct PreparedSearch {
    owner: std::sync::Arc<()>,
    client: std::sync::Arc<crate::surrealdb::Surreal<crate::surrealdb::engine::remote::grpc::Client>>,
    staging: std::sync::Arc<crate::control::staging::DerivedStaging>,
    source_pin: std::sync::Arc<crate::control::ReaderPin>,
    source_client: std::sync::Arc<crate::surrealdb::Surreal<crate::surrealdb::engine::remote::grpc::Client>>,
    budget: resources::ResourceBudget,
    realization: ContentHash,
    scope: ContentHash,
    search: [PreparedRows; 7],
    lexical: crate::lexical_stats::Expected,
    _charge: Box<dyn resources::Reservation>,
}
impl PreparedSearch {
    fn check(&self, loader: &Loader) -> Result<(), ModelError> {
        loader.check_read_admission()?;
        let reader = loader.reader();
        reader.ensure_scope_live()?;
        self.source_pin.ensure_live()?;
        if !std::sync::Arc::ptr_eq(&self.owner, loader.search_owner())
            || !std::sync::Arc::ptr_eq(&self.client, &loader.shared_client())
            || !std::sync::Arc::ptr_eq(&self.source_client, &reader.shared_client())
            || !reader.scope_pin().is_some_and(|pin| std::sync::Arc::ptr_eq(&self.source_pin, &pin))
            || !loader.staging().is_some_and(|staging| std::sync::Arc::ptr_eq(staging, &self.staging))
            || !self.budget.shares_pool(&loader.read_budget()?)
            || self.realization != loader.derivation()?
            || self.scope != crate::lexical_stats::scope_identity(&loader.reader())?
        {
            return Err(ModelError::Conflict("prepared search protected owner/context changed"));
        }
        Ok(())
    }
    pub(crate) async fn reconcile_owned(&self, loader: &Loader, mut inventory: Option<&mut SortedRows>) -> Result<(), ModelError> {
        self.check(loader)?;
        let mut expected = Expected::from_prepared(&self.search, &self.budget)?;
        phase("prepared search fresh actual-row reconciliation", expected.reconcile(&reader(loader)?).await)?;
        if let Some(inventory) = inventory.as_deref_mut() {
            for prepared in &self.search {
                let mut rows = prepared.cursor()?;
                while let Some(row) = rows.next_row()? {
                    loader.check_read_admission()?;
                    crate::control::staging::remember_expected_child(inventory, &row)?;
                }
            }
        }
        phase("prepared lexical fresh actual-row reconciliation", crate::lexical_stats::reconcile_prepared(loader, &self.lexical, inventory).await)?;
        self.check(loader)
    }
}

/// The ordinary publication route retains exactly the lowering that successfully
/// wrote and compared all eleven families. Cold reconciliation never calls this.
pub async fn materialize_search_prepared(loader: &Loader) -> Result<PreparedSearch, ModelError> {
    let staging = loader.staging().ok_or(ModelError::Conflict("prepared search requires actual staging owner"))?.clone();
    let reader = reader(loader)?;
    reader.require_installed_scope()?;
    let source_pin = reader.scope_pin().ok_or(ModelError::Conflict("prepared search requires actual completed reader pin"))?;
    let budget = loader.read_budget()?;
    let charge = budget.reserve("publication retained search expectations", std::mem::size_of::<PreparedSearch>())?;
    let mut expected = Expected::new(&budget)?;
    phase("derived search canonical lowering", lower(&reader, &mut expected, Some(loader)).await)?;
    phase("derived search actual-row reconciliation", expected.reconcile(&reader).await)?;
    let lexical = phase("frozen lexical statistics construction", crate::lexical_stats::materialize_prepared(loader).await)?;
    let prepared = PreparedSearch {
        owner: loader.search_owner().clone(),
        client: loader.shared_client(),
        staging,
        source_pin,
        source_client: reader.shared_client(),
        budget,
        realization: loader.derivation()?,
        scope: crate::lexical_stats::scope_identity(&reader)?,
        search: expected.into_prepared()?,
        lexical,
        _charge: charge,
    };
    prepared.check(loader)?;
    Ok(prepared)
}
pub async fn reconcile_search(loader: &Loader) -> Result<(), ModelError> {
    reconcile_search_owned(loader, None).await
}
pub(crate) async fn reconcile_search_owned(
    loader: &Loader,
    mut inventory: Option<&mut SortedRows>,
) -> Result<(), ModelError> {
    let reader = reader(loader)?;
    let budget = loader.read_budget()?;
    let mut expected = Expected::new(&budget)?;
    phase(
        "derived search independent cold lowering",
        lower(&reader, &mut expected, None).await,
    )?;
    phase(
        "derived search cold actual-row reconciliation",
        expected.reconcile(&reader).await,
    )?;
    if let Some(inventory) = inventory.as_deref_mut() {
        for index in 0..TABLES.len() {
            let rows = expected.finish(index)?;
            rows.rewind()?;
            while let Some(row) = rows.next_row()? {
                crate::control::staging::remember_expected_child(inventory, &row)?;
            }
        }
    }
    phase(
        "frozen lexical statistics cold comparison",
        crate::lexical_stats::reconcile_owned(loader, inventory).await,
    )
}
fn crate_json<T: serde::Serialize>(value: T) -> Result<Value, ModelError> {
    crate::loader::json_value(serde_json::to_value(value).map_err(ModelError::codec)?)
}
#[cfg(test)]
mod prepared_expectation_tests {
    use super::*;

    fn row(table: &str, key: &str, text: &str) -> Value {
        let mut row = Object::new();
        row.insert("id", RecordId::new(table, key));
        row.insert("text", text.to_owned());
        Value::Object(row)
    }
    fn input(values: Vec<Value>, late_failure: bool) -> crate::reader::NativeRows {
        crate::reader::NativeRows::owned(move |sender| async move {
            for value in values {
                if sender.send(value).await.is_err() { break; }
            }
            if late_failure { return Err(ModelError::Conflict("late actual-search terminal failure")); }
            Ok(())
        }).unwrap()
    }
    fn prepared(budget: &resources::ResourceBudget) -> [PreparedRows; 7] {
        let mut expected = Expected::new(budget).unwrap();
        expected.emit("search_source", row("search_source", "a", "original")).unwrap();
        expected.emit("search_source", row("search_source", "b", "last")).unwrap();
        // Materialization has consumed this cursor before the preparation is retained.
        let cursor = expected.finish(3).unwrap();
        while cursor.next_row().unwrap().is_some() {}
        expected.into_prepared().unwrap()
    }
    #[tokio::test]
    async fn retained_search_compares_complete_fresh_cursors_and_refuses_mutations() {
        let budget = resources::ResourceBudget::fixed(8 << 20).unwrap();
        let runs = prepared(&budget);
        let original = vec![row("search_source", "a", "original"), row("search_source", "b", "last")];
        let mut expected = Expected::from_prepared(&runs, &budget).unwrap();
        for _ in 0..2 {
            expected.reconcile_actual(|| Ok(input(original.clone(), false))).await.unwrap();
        }
        for values in [
            vec![original[0].clone()],
            vec![original[0].clone(), original[1].clone(), row("search_source", "c", "extra")],
            vec![original[0].clone(), original[0].clone(), original[1].clone()],
            vec![row("search_source", "a", "changed"), original[1].clone()],
            vec![row("search_source", "foreign", "original"), original[1].clone()],
        ] {
            assert!(expected.reconcile_actual(|| Ok(input(values, false))).await.is_err());
            expected.reconcile_actual(|| Ok(input(original.clone(), false))).await.unwrap();
        }
        assert!(expected.reconcile_actual(|| Ok(input(original.clone(), true))).await.is_err());
        expected.reconcile_actual(|| Ok(input(original, false))).await.unwrap();
        // Inventory receives independent full runs after every successful/failed comparison.
        let mut inventory = SortedRows::with_budget(&budget).unwrap();
        for run in &runs {
            let mut cursor = run.cursor().unwrap();
            while let Some(value) = cursor.next_row().unwrap() {
                crate::control::staging::remember_expected_child(&mut inventory, &value).unwrap();
            }
        }
        let mut inventory = inventory.finish().unwrap();
        assert!(inventory.next_row().unwrap().is_some());
        assert!(inventory.next_row().unwrap().is_some());
        assert!(inventory.next_row().unwrap().is_none());
        drop(inventory); drop(expected); drop(runs);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn retained_search_waits_for_actual_terminal_after_mismatch() {
        use futures::FutureExt;
        let budget = resources::ResourceBudget::fixed(8 << 20).unwrap();
        let runs = prepared(&budget);
        let mut expected = Expected::from_prepared(&runs, &budget).unwrap();
        let (release, gate) = tokio::sync::oneshot::channel();
        let actual = || crate::reader::NativeRows::owned(move |sender| async move {
            sender.send(row("search_source", "a", "changed")).await.map_err(ModelError::codec)?;
            gate.await.map_err(ModelError::codec)?;
            Err(ModelError::Conflict("late failed drainage"))
        });
        let mut compared = Box::pin(expected.reconcile_actual(actual));
        assert!(compared.as_mut().now_or_never().is_none());
        drop(runs);
        assert!(budget.reserved() > 0, "last comparison borrower retains prepared backing");
        release.send(()).unwrap();
        let error = compared.await.unwrap_err();
        assert!(error.to_string().contains("late failed drainage"));
        drop(expected);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn cancelled_comparison_retains_spilled_expectations_through_actual_drainage() {
        use futures::FutureExt;
        let budget = resources::ResourceBudget::fixed(64 << 20).unwrap();
        let mut sort = SortedRows::with_budget(&budget).unwrap();
        let text = "payload".repeat(8192);
        for index in 0..32 { sort.push(row("search_source", &format!("{index:03}"), &text)).unwrap(); }
        let run = sort.finish().unwrap().into_prepared();
        let (release, gate) = tokio::sync::oneshot::channel();
        let actual = || crate::reader::NativeRows::owned(move |_sender| async move {
            gate.await.map_err(ModelError::codec)?;
            Ok(())
        });
        let mut actual = retain_expected_until_drained(std::iter::once(run.clone()), &budget, actual).unwrap();
        actual.cancel_delivery();
        drop(run);
        let mut drain = Box::pin(actual.drain_transport());
        assert!(drain.as_mut().now_or_never().is_none());
        assert!(budget.reserved() > 0, "cancelled delivery retains the spilled run and its owner charge");
        release.send(()).unwrap();
        drain.await.unwrap();
        drop(actual);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn exhausted_expectation_borrow_refuses_before_gated_actual_admission() {
        use futures::FutureExt;
        use std::sync::atomic::{AtomicBool, Ordering};
        let budget = resources::ResourceBudget::fixed(8 << 20).unwrap();
        let runs = prepared(&budget);
        let mut expected = Expected::from_prepared(&runs, &budget).unwrap();
        let hold = budget.reserve("exhausted comparison admission control", budget.limit() - budget.reserved()).unwrap();
        let admitted = std::sync::Arc::new(AtomicBool::new(false));
        let actual_admitted = admitted.clone();
        let (release, gate) = tokio::sync::oneshot::channel::<()>();
        let factory = move || {
            actual_admitted.store(true, Ordering::Release);
            crate::reader::NativeRows::owned(move |_sender| async move {
                gate.await.map_err(ModelError::codec)?;
                Ok(())
            })
        };
        let mut compared = Box::pin(expected.reconcile_actual(factory));
        let error = compared.as_mut().now_or_never().expect("refusal has no pending actual drain").unwrap_err();
        assert!(matches!(error.primary(), Some(ModelError::Resource { .. })));
        drop(compared); drop(expected); drop(runs);
        assert!(!admitted.load(Ordering::Acquire));
        assert!(release.is_closed(), "the unadmitted gated factory was dropped, with no detached worker");
        assert_eq!(budget.reserved(), hold.size());
        drop(hold);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn dropped_comparison_and_last_owner_retain_backing_until_gated_terminal_with_exhausted_pool() {
        use futures::FutureExt;
        let budget = resources::ResourceBudget::fixed(8 << 20).unwrap();
        let runs = prepared(&budget);
        let mut expected = Expected::from_prepared(&runs, &budget).unwrap();
        let (release, gate) = tokio::sync::oneshot::channel::<()>();
        let factory = move || crate::reader::NativeRows::owned(move |_sender| async move {
            gate.await.map_err(ModelError::codec)?;
            Ok(())
        });
        let mut compared = Box::pin(expected.reconcile_actual(factory));
        assert!(compared.as_mut().now_or_never().is_none());
        let hold = budget.reserve("exhausted cancelled comparison control", budget.limit() - budget.reserved()).unwrap();
        drop(compared); drop(expected); drop(runs);
        assert!(budget.reserved() > hold.size(), "the admitted driver owns backing after cancellation and the last publication owner drops");
        release.send(()).unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while budget.reserved() != hold.size() { tokio::task::yield_now().await; }
        }).await.expect("detached actual drainage releases expected backing");
        drop(hold);
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn loader_nominal_identity_survives_moves_but_never_foreign_construction() {
        let client = std::sync::Arc::new(crate::surrealdb::Surreal::init());
        let loader = Loader::for_views(client.clone(), vec![]);
        let identity = loader.search_owner().clone();
        let loader = loader.with_derivation(ContentHash::of(b"realization"));
        assert!(std::sync::Arc::ptr_eq(&identity, loader.search_owner()));
        let foreign = Loader::for_views(client, vec![]).with_derivation(loader.derivation().unwrap());
        assert!(!std::sync::Arc::ptr_eq(&identity, foreign.search_owner()));
    }
}
struct Witness<'a> {
    unit: &'a Unit,
    window: &'a SearchWindow,
    part: &'a ContentPart,
    binding: Option<Id<WindowBinding>>,
    member: Option<Id<catalog::CatalogMember>>,
    anchor: Option<Id<OriginalAnchor>>,
    name: String,
    path: String,
    option_key: String,
}
fn occurrence(
    table: &str,
    namespace: ContentHash,
    key: &str,
    input: RecordId,
    w: &Witness<'_>,
    dependencies: &[RecordId],
) -> Result<Value, ModelError> {
    let out = w
        .member
        .map(|m| target_id(Target::Entity(EntityId::of(m))))
        .unwrap_or_else(|| target_id(Target::Entity(EntityId::of(w.unit.id()))));
    let mut row = Object::new();
    row.insert(
        "id",
        crate::derivation::address(namespace, table, key.as_bytes()),
    );
    row.insert("derivation", namespace.hex());
    row.insert("dependencies", dependencies.to_vec());
    row.insert("unit_payload", typed_payload(w.unit)?);
    row.insert("in", input);
    row.insert("out", out);
    row.insert("family", w.unit.family as i16);
    row.insert("unit", crate_json(w.unit.id())?);
    row.insert(
        "unit_node",
        target_id(Target::Entity(EntityId::of(w.unit.id()))),
    );
    row.insert("window", crate_json(w.window.id())?);
    row.insert("part", crate_json(w.part.id())?);
    row.insert("binding", crate_json(w.binding)?);
    row.insert("context", crate_json(w.unit.context)?);
    row.insert("member", crate_json(w.member)?);
    row.insert("anchor", crate_json(w.anchor)?);
    row.insert("input", crate_json(w.unit.input)?);
    row.insert("eligible", true);
    row.insert("exact_name", w.name.clone());
    row.insert("exact_path", w.path.clone());
    row.insert("exact_option", w.option_key.clone());
    row.insert(
        "occurrence_key",
        format!(
            "{}|{:02}|{}|{}|{}|{}|{}",
            w.member
                .map(|id| format!("0{}", id.hex()))
                .unwrap_or_else(|| format!("1{}", w.unit.id().hex())),
            w.unit.family as i16,
            w.unit.id().hex(),
            w.window.id().hex(),
            w.part.id().hex(),
            w.unit.context.hex(),
            key
        ),
    );
    for field in ["input", "member", "context", "window"] {
        row.insert(
            format!("scope_{field}"),
            scope_string(row.get(field).expect("occurrence field")),
        );
    }
    Ok(Value::Object(row))
}
