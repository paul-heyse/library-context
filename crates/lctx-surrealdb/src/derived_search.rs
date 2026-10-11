//! One terminally checked lowering for private construction and read-only cold reconciliation.
use crate::surrealdb::types::{Object, RecordId, SurrealValue, Value, Variables};
use crate::{
    Loader, NativeReader, RecordSelection,
    ordered_rows::{OrderedRows, SortedRows},
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

/// Complete selected occurrence stream. Unit payload windows use the installed equality
/// index; dependency eligibility is evaluated before the caller's ranking/quota policy.
pub fn selected_occurrences<Context>(
    reader: &NativeReader<Context>, table: &str, predicate: &str, bindings: Variables, budget: &resources::ResourceBudget,
) -> Result<crate::reader::NativeRows, ModelError> {
    if bindings.get("input_keys").is_some() || bindings.get("window_keys").is_some() {
        indexed_occurrence_rows(reader, table, predicate, bindings, budget)
    } else { occurrence_rows(reader, table, predicate, bindings, true, budget) }
}
/// Serving nominations use existing input/window indexes. Exact dependency checks run
/// before payload hydration, without preparing the complete publication membership.
fn indexed_occurrence_rows<Context>(reader: &NativeReader<Context>, table: &str, predicate: &str, bindings: Variables, budget: &resources::ResourceBudget) -> Result<crate::reader::NativeRows, ModelError> {
    if !matches!(table, "lex_occurs" | "vec_occurs") { return Err(ModelError::Schema("indexed occurrence family")); }
    let index = if bindings.get("window_keys").is_some() { "window_occurrences" } else { "eligible_input" };
    let nominations = reader.stream_prepared(crate::prepared::PreparedQuery::new(bindings, vec![], vec![format!("SELECT id,unit_payload,dependencies FROM {table} WITH INDEX {index} WHERE ({predicate})")])?)?;
    hydrate_occurrence_nominations(reader, nominations, budget)
}
/// Table FULLTEXT nominates documents, while three independent equality indexes nominate
/// exact occurrences even when the query has no analyzer terms. Neither branch has a
/// pre-eligibility quota; unrelated publications cannot consume the selected frontier.
pub fn lexical_nomination_sql(family_table: &str) -> Result<String, ModelError> {
    if !TABLES[..4].contains(&family_table) { return Err(ModelError::Schema("lexical nomination family")); }
    Ok(format!("SELECT id FROM {family_table} WITH INDEX lexical WHERE text @OR@ $query"))
}
pub fn lexical_occurrences<Context>(reader: &NativeReader<Context>, family_table: &str, predicate: &str, bindings: Variables, budget: &resources::ResourceBudget) -> Result<crate::reader::NativeRows, ModelError> {
    let nomination_sql = lexical_nomination_sql(family_table)?;
    let owner = reader.transport_reader(); let predicate = predicate.to_owned(); let budget_copy = budget.clone();
    let nominations = crate::reader::NativeRows::owned(move |sender| async move {
        let mut documents = owner.stream_prepared(crate::prepared::PreparedQuery::new(bindings.clone(), vec![], vec![nomination_sql])?)?;
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
                let mut rows = owner.stream_prepared(crate::prepared::PreparedQuery::new(vars, vec![], vec![format!("SELECT id,unit_payload,dependencies FROM lex_occurs WITH INDEX document_occurrences WHERE in IN $documents AND ({predicate})")])?)?;
                let result = async { while let Some(row) = rows.next().await? { if sender.send(row).await.is_err() { rows.cancel_delivery(); break; } } Ok(()) }.await;
                let mut terminal = completion::Completion::default(); terminal.step("lexical document occurrences drainage", rows.drain_transport().await); completion::complete(result, terminal)?;
            }
            Ok(())
        }.await;
        let mut terminal = completion::Completion::default(); terminal.step("lexical fulltext nomination drainage", documents.drain_transport().await); completion::complete(result, terminal)?;
        for field in ["exact_name", "exact_path", "exact_option"] {
            if sender.is_closed() { return Ok(()); }
            let mut rows = owner.stream_prepared(crate::prepared::PreparedQuery::new(bindings.clone(), vec![], vec![format!("SELECT id,unit_payload,dependencies FROM lex_occurs WITH INDEX {field} WHERE {field}=$query AND ({predicate})")])?)?;
            let result = async { while let Some(row) = rows.next().await? { if sender.send(row).await.is_err() { rows.cancel_delivery(); break; } } Ok(()) }.await;
            let mut terminal = completion::Completion::default(); terminal.step("lexical exact nomination drainage", rows.drain_transport().await); completion::complete(result, terminal)?;
        }
        Ok(())
    })?;
    hydrate_occurrence_nominations(reader, nominations, budget)
}
fn nomination_dependencies(object: &Object) -> Result<Vec<RecordId>, ModelError> {
    let mut dependencies = Vec::<RecordId>::from_value(object.get("dependencies").cloned().ok_or(ModelError::Schema("occurrence dependencies"))?).map_err(ModelError::codec)?;
    if dependencies.is_empty() { return Ok(dependencies); }
    dependencies.push(RecordId::from_value(object.get("unit_payload").cloned().ok_or(ModelError::Schema("occurrence unit payload"))?).map_err(ModelError::codec)?);
    dependencies.sort(); dependencies.dedup(); Ok(dependencies)
}
fn eligible_nomination_ids(window: &BTreeMap<RecordId, Object>, membership: &crate::selection::MembershipAnswers) -> Result<Vec<RecordId>, ModelError> {
    let mut accepted = Vec::new();
    for (id, object) in window {
        let dependencies = nomination_dependencies(object)?;
        if !dependencies.is_empty() && membership.contains_all(&dependencies) { accepted.push(id.clone()); }
    }
    Ok(accepted)
}
fn hydrate_occurrence_nominations<Context>(reader: &NativeReader<Context>, mut rows: crate::reader::NativeRows, budget: &resources::ResourceBudget) -> Result<crate::reader::NativeRows, ModelError> {
    let retention = reader.transport_reader();
    let reader = reader.transport_reader(); let budget = budget.clone();
    let rows = crate::reader::NativeRows::owned(move |sender| async move {
        let mut sorted = SortedRows::with_budget(&budget)?;
        let result = async {
            loop {
                if sender.is_closed() { rows.cancel_delivery(); break; }
                let mut scratch = charged::StateCharge::new(&budget, "indexed-occurrence-window");
                let mut window = BTreeMap::new(); let mut union = BTreeSet::new(); let mut examined = 0;
                while examined < BATCH_ROWS {
                    let Some(row) = rows.next().await? else { break; }; examined += 1;
                    scratch.grow(crate::loader::native_bytes(&row).saturating_mul(8))?;
                    let object = Object::from_value(row).map_err(ModelError::codec)?;
                    let dependencies = nomination_dependencies(&object)?;
                    if dependencies.is_empty() { continue; }
                    let id = RecordId::from_value(object.get("id").cloned().ok_or(ModelError::Schema("occurrence identity"))?).map_err(ModelError::codec)?;
                    union.extend(dependencies.iter().cloned());
                    if let Some(previous) = window.insert(id, object.clone()) { if previous != object { return Err(ModelError::Conflict("occurrence nomination changed")); } }
                }
                if examined == 0 { break; }
                if window.is_empty() { continue; }
                let union = union.into_iter().collect::<Vec<_>>();
                let membership = reader.selected_membership(&union, &budget).await?;
                let accepted = eligible_nomination_ids(&window, &membership)?;
                if accepted.is_empty() { continue; }
                let mut payload = reader.stream_prepared(crate::prepared::PreparedQuery::new(Variables::from_iter([("nodes".into(), accepted.clone().into_value())]), vec![], vec!["SELECT * FROM $nodes".into()])?)?;
                let result = async {
                    let mut seen = BTreeSet::new();
                    while let Some(row) = payload.next().await? {
                        let object = Object::from_value(row.clone()).map_err(ModelError::codec)?;
                        let id = RecordId::from_value(object.get("id").cloned().ok_or(ModelError::Schema("occurrence payload identity"))?).map_err(ModelError::codec)?;
                        if !accepted.contains(&id) || !seen.insert(id.clone()) { return Err(ModelError::Conflict("occurrence hydration correspondence")); }
                        let expected = &window[&id];
                        if ["dependencies", "unit_payload"].iter().any(|field| object.get(*field) != expected.get(*field)) { return Err(ModelError::Conflict("occurrence hydration dependencies changed")); }
                        sorted.push(row)?;
                    }
                    if seen.len() != accepted.len() { return Err(ModelError::Schema("missing nominated occurrence payload")); }
                    Ok(())
                }.await;
                let mut terminal = completion::Completion::default(); terminal.step("indexed occurrence payload drainage", payload.drain_transport().await); completion::complete(result, terminal)?;
            }
            Ok(())
        }.await;
        let mut terminal = completion::Completion::default(); terminal.step("indexed occurrence nomination drainage", rows.drain_transport().await); completion::complete(result, terminal)?;
        let mut sorted = sorted.finish()?;
        while sorted.next_row()?.is_some() {}
        sorted.rewind()?;
        while let Some(row) = sorted.next_row()? { if sender.send(row).await.is_err() { break; } }
        Ok(())
    })?;
    Ok(retention.retain_rows(rows))
}
fn occurrence_rows<Context>(
    reader: &NativeReader<Context>, table: &str, predicate: &str, bindings: Variables, eligible: bool, budget: &resources::ResourceBudget,
) -> Result<crate::reader::NativeRows, ModelError> {
    if !matches!(table, "lex_occurs" | "vec_occurs") { return Err(ModelError::Schema("selected occurrence family")); }
    let client = reader.shared_client(); let cancellation = reader.read_cancellation();
    let selection = reader.selection_preparation();
    let table = table.to_owned(); let predicate = predicate.to_owned(); let budget = budget.clone();
    crate::reader::NativeRows::owned(move |sender| async move {
        let mut sorted = SortedRows::with_budget(&budget)?;
        let _input = budget.reserve("selected occurrence input", BATCH_BYTES)?;
        let selected = selection.await?;
        let mut cursor = selected.as_ref().map(|selected| selected.pointers_with_budget(&budget)).transpose()?;
        loop {
            if sender.is_closed() { return Ok(()); }
            let mut batch_charge = budget.reserve("selected occurrence pointers", BATCH_ROWS * std::mem::size_of::<RecordId>())?;
            let mut vars = bindings.clone();
            let sql = if let Some(cursor) = &mut cursor {
                let mut nodes = Vec::with_capacity(BATCH_ROWS);
                while nodes.len() < BATCH_ROWS {
                    let Some(row) = cursor.next_row()? else { break; };
                    batch_charge.try_resize(batch_charge.size().saturating_add(crate::loader::native_bytes(&row)))?;
                    let Value::Object(row) = row else { return Err(ModelError::Schema("selected occurrence pointer")); };
                    let node = RecordId::from_value(row.get("id").cloned().ok_or(ModelError::Schema("selected occurrence node"))?).map_err(ModelError::codec)?;
                    if node.table.as_str() == "entity" { nodes.push(node); }
                }
                if nodes.is_empty() { break; }
                vars.insert("nodes", nodes);
                format!("SELECT * FROM {table} WITH INDEX exact_unit_payload WHERE unit_payload IN $nodes AND ({predicate})")
            } else { format!("SELECT * FROM {table} WHERE ({predicate})") };
            if sender.is_closed() { return Ok(()); }
            let mut rows = crate::prepared::PreparedQuery::new(vars, vec![], vec![sql])?.stream_cancellable(&client, cancellation.as_ref())?;
            let result = async { while let Some(row) = rows.next().await? {
                let _scratch = budget.reserve("selected occurrence decoded scratch", crate::loader::native_bytes(&row).saturating_mul(8))?;
                if eligible { if let Some(selected) = &selected {
                    let Value::Object(object) = &row else { return Err(ModelError::Schema("selected occurrence")); };
                    let dependencies = Vec::<RecordId>::from_value(object.get("dependencies").cloned().ok_or(ModelError::Schema("selected occurrence dependencies"))?).map_err(ModelError::codec)?;
                    if dependencies.is_empty() || !selected.contains_all_with_budget(&dependencies, &budget)? { continue; }
                } }
                sorted.push(row)?;
            } Ok(()) }.await;
            let mut terminal = lctx_model::domain::completion::Completion::default(); terminal.step("selected occurrence indexed drainage", rows.drain_transport().await);
            lctx_model::domain::completion::complete(result, terminal)?;
            if cursor.is_none() { break; }
        }
        let mut rows = sorted.finish()?;
        while let Some(row) = rows.next_row()? { if sender.send(row).await.is_err() { break; } }
        Ok(())
    })
}
#[cfg(test)]
mod occurrence_budget_tests {
    use super::*;
    #[test]
    fn lexical_nomination_uses_an_indexed_family_table_without_score_or_quota_filter() {
        for table in &TABLES[..4] {
            let sql = lexical_nomination_sql(table).unwrap();
            assert!(sql.contains(&format!("FROM {table} WITH INDEX lexical")));
            assert!(sql.contains("text @OR@ $query")); assert!(!sql.contains("LIMIT")); assert!(!sql.contains("score"));
        }
        assert!(lexical_nomination_sql("$documents").is_err());
    }
    #[test]
    fn occurrence_dependencies_require_every_dependency_and_the_unit_payload() {
        let a = RecordId::new("entity", "a"); let b = RecordId::new("entity", "late-missing"); let unit = RecordId::new("entity", "unit");
        let mut row = Object::new(); row.insert("dependencies", vec![a.clone(), a.clone(), b.clone()]); row.insert("unit_payload", unit.clone());
        assert_eq!(nomination_dependencies(&row).unwrap(), vec![a, b, unit]);
        row.insert("dependencies", Vec::<RecordId>::new()); assert!(nomination_dependencies(&row).unwrap().is_empty(), "empty dependency lists are never admitted solely by their unit");
        row.insert("dependencies", vec![RecordId::new("entity", "a")]); row.remove("unit_payload"); assert!(nomination_dependencies(&row).is_err());
    }
    #[test]
    fn occurrence_window_does_not_promote_one_present_dependency_to_all_present() {
        let budget = resources::ResourceBudget::fixed(1 << 20).unwrap();
        let shared = RecordId::new("entity", "shared"); let unit = RecordId::new("entity", "unit");
        let present = crate::selection::MembershipAnswers::unrestricted(&[shared.clone(), unit.clone()], &budget).unwrap();
        let nomination = |dependencies: Vec<RecordId>, unit_payload: RecordId| { let mut object = Object::new(); object.insert("dependencies", dependencies); object.insert("unit_payload", unit_payload); object };
        let good = RecordId::new("lex_occurs", "good");
        let window = BTreeMap::from([
            (good.clone(), nomination(vec![shared.clone()], unit.clone())),
            (RecordId::new("lex_occurs", "late-missing"), nomination(vec![shared.clone(), RecordId::new("entity", "absent")], unit)),
            (RecordId::new("lex_occurs", "missing-unit"), nomination(vec![shared], RecordId::new("entity", "other-unit"))),
        ]);
        assert_eq!(eligible_nomination_ids(&window, &present).unwrap(), vec![good]);
        drop(present); assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn occurrence_preparation_refuses_unadmitted_input_before_native_access() {
        let budget = resources::ResourceBudget::fixed(BATCH_BYTES / 2).unwrap();
        let reader = NativeReader::for_views(std::sync::Arc::new(crate::surrealdb::Surreal::init()), vec![]).with_budget(&budget);
        let mut rows = selected_occurrences(&reader, "lex_occurs", "true", Variables::new(), &budget).unwrap();
        let error = rows.next().await.unwrap_err();
        assert!(matches!(error.primary(), Some(ModelError::Resource { .. })), "budget refusal must precede access to the unconnected native client: {error}");
        rows.drain_transport().await.unwrap(); drop(rows);
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
            let mut row = Object::new(); row.insert("dependencies", Vec::<RecordId>::new());
            sender.send(Value::Object(row.clone())).await.map_err(ModelError::codec)?;
            sender.send(Value::Object(row)).await.map_err(ModelError::codec)?;
            entered.send(()).map_err(|_| ModelError::Conflict("occurrence gate receiver"))?;
            gate.await.map_err(ModelError::codec)?;
            Ok(())
        }).unwrap().with_client(client.clone());
        let mut rows = hydrate_occurrence_nominations(&reader, input, &budget).unwrap();
        started.await.unwrap();
        assert!(budget.reserved() > 0, "the nomination window is admitted before cancellation");
        assert!(reader.close().await.is_err(), "the active driver retains the selected reader");
        rows.cancel_delivery();
        let mut drainage = Box::pin(rows.drain_transport());
        assert!(drainage.as_mut().now_or_never().is_none(), "delivery cancellation cannot acknowledge the gated nomination terminal");
        assert!(budget.reserved() > 0);
        assert!(std::sync::Arc::strong_count(&client) > baseline_handles, "session handles remain owned through drainage");
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
        use crate::compiler::NativeCompilerStore;
        use admission::Frontier;
        use completed::ContributionSpec;
        use stages::{Profile, ProviderOutcome};
        let config = crate::RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("stable validation runtime"),
        )).unwrap();
        assert_eq!(config.database.as_str(), "validation");
        let fixture_budget = resources::ResourceBudget::fixed(64 << 20).unwrap();
        let store = NativeCompilerStore::begin(&config, Frontier::Facts).await.unwrap();
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
        let contribution = store.begin_contribution(ContributionSpec {
            captured_binding: None, producer: "occurrence-driver-control".into(),
            profile: Profile::Catalog, model: model().unwrap().digest(),
            implementation: ContentHash::of(b"occurrence-driver-control/v1"),
            configuration: None, inputs: vec![], outputs: BTreeSet::from([Unit::NAME.into()]),
        }).await.unwrap();
        store.write_batch(&contribution, &relation, &Unit::encode(std::slice::from_ref(&unit)).unwrap()).await.unwrap();
        let views = store.complete_contribution(contribution, ProviderOutcome::Complete,
            std::slice::from_ref(&relation), &BTreeMap::new(), &fixture_budget).await.unwrap();
        let views = views.values().map(|view| view.identity).collect::<Vec<_>>();
        let client = crate::compiler::check_installation(&config).await.unwrap();
        let loader = Loader::for_attempt_views(client.clone(), store.attempt(), views.clone()).with_budget(&fixture_budget);
        loader.entity_references(&[graph::Entity::from(unit.clone())]).await.unwrap();
        let reader = NativeReader::for_views(client.clone(), views).with_budget(&fixture_budget);
        let payload = typed_payload(&unit).unwrap();
        assert!(reader.selected_membership(std::slice::from_ref(&payload), &fixture_budget).await.unwrap().contains(&payload));
        let document = RecordId::new("search_api_options", format!("occurrence-integrity-{nonce}"));
        let ids = ["dependencies", "unit", "missing"].map(|case| RecordId::new("lex_occurs", format!("occurrence-integrity-{nonce}-{case}")));
        let mut doc = Object::new(); doc.insert("id", document.clone());
        doc.insert("text", format!("occurrence integrity {nonce}")); doc.insert("digest", ContentHash::of(nonce.as_bytes()).0.to_vec());
        reader.query_native::<Value>("INSERT INTO search_api_options $rows RETURN NONE", Variables::from_iter([("rows".into(), vec![Value::Object(doc)].into_value())])).await.unwrap();
        let anchor = target_id(Target::Entity(EntityId::of(unit.id())));
        let mut occurrences = Vec::new();
        for id in &ids {
            let mut row = Object::from_value(crate::loader::json_value(serde_json::json!({
                "family": Family::ApiOptions as i16, "unit": unit.id(), "window": unit.id(),
                "part": unit.id(), "context": unit.context, "input": unit.input,
                "binding": null, "member": null, "anchor": null,
                "exact_name": "", "exact_path": "", "exact_option": "", "eligible": true,
                "occurrence_key": format!("{nonce}-{:?}", id.key),
            })).unwrap()).unwrap();
            row.insert("id", id.clone()); row.insert("in", document.clone()); row.insert("out", anchor.clone());
            row.insert("unit_node", anchor.clone()); row.insert("unit_payload", payload.clone()); row.insert("dependencies", vec![payload.clone()]);
            occurrences.push(Value::Object(row));
        }
        reader.query_native::<Value>("INSERT RELATION INTO lex_occurs $rows RETURN NONE", Variables::from_iter([("rows".into(), occurrences.into_value())])).await.unwrap();
        let nominations: Vec<Value> = reader.query_native("SELECT id,dependencies,unit_payload FROM $nodes ORDER BY id", Variables::from_iter([("nodes".into(), ids.to_vec().into_value())])).await.unwrap();
        assert_eq!(nominations.len(), 3, "actual pre-mutation nominations");
        for (id, expected) in ids.iter().zip([
            "occurrence hydration dependencies changed", "occurrence hydration dependencies changed", "missing nominated occurrence payload",
        ]) {
            let nomination = nominations.iter().find(|value| value.as_object().unwrap().get("id") == Some(&Value::RecordId(id.clone()))).unwrap().clone();
            let mut vars = Variables::new(); vars.insert("node", id.clone());
            let mutation = if id == &ids[0] { "UPDATE ONLY $node SET dependencies=[] RETURN NONE" }
                else if id == &ids[1] { vars.insert("other", RecordId::new("entity", format!("missing-unit-{nonce}"))); "UPDATE ONLY $node SET unit_payload=$other RETURN NONE" }
                else { "DELETE ONLY $node RETURN NONE" };
            reader.query_native::<Value>(mutation, vars).await.unwrap();
            let nominations = crate::reader::NativeRows::owned(move |sender| async move {
                sender.send(nomination).await.map_err(ModelError::codec)?; Ok(())
            }).unwrap();
            let budget = resources::ResourceBudget::fixed(16 << 20).unwrap();
            let mut rows = hydrate_occurrence_nominations(&reader, nominations, &budget).unwrap();
            let error = rows.next().await.unwrap_err();
            assert!(error.to_string().contains(expected), "{error}");
            assert!(rows.next().await.is_err(), "hydration failures stay sticky");
            rows.drain_transport().await.unwrap(); drop(rows);
            assert_eq!(budget.reserved(), 0, "failed hydration drains all request state");
        }
        reader.query_native::<Value>("DELETE $nodes RETURN NONE", Variables::from_iter([("nodes".into(), ids.to_vec().into_value())])).await.unwrap();
        reader.query_native::<Value>("DELETE ONLY $document RETURN NONE", Variables::from_iter([("document".into(), document.into_value())])).await.unwrap();
        drop(loader); reader.close().await.unwrap(); drop(reader);
        client.invalidate().await.unwrap(); store.abandon().await.unwrap();
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
        let mut actual = actual_search_rows(reader, &self.budget)?;
        let result = async {
            let mut indices = (0..TABLES.len()).collect::<Vec<_>>(); indices.sort_by_key(|index| TABLES[*index]);
            for index in indices { while let Some(expected) = self.finish(index)?.next_row()? {
                let row = actual.next().await?.ok_or(ModelError::Serving(lctx_model::domain::serving::FailureKind::Corrupt))?;
                if serde_json::to_vec(&expected).map_err(ModelError::codec)? != serde_json::to_vec(&row).map_err(ModelError::codec)? {
                    return Err(ModelError::Serving(lctx_model::domain::serving::FailureKind::Corrupt));
                }
            } }
            if actual.next().await?.is_some() { return Err(ModelError::Serving(lctx_model::domain::serving::FailureKind::Corrupt)); }
            Ok(())
        }.await;
        let mut completion = lctx_model::domain::completion::Completion::default(); completion.step("derived actual rows transport drainage", actual.drain_transport().await);
        lctx_model::domain::completion::complete(result, completion)
    }
}
pub fn actual_search_rows<Context>(reader: &NativeReader<Context>, budget: &resources::ResourceBudget) -> Result<crate::reader::NativeRows, ModelError> {
    // Actual-state nomination deliberately retains bad dependency lists and unexpected rows.
    // The independent cold lowering, rather than eligibility filtering, diagnoses them.
    let budget = budget.clone();
    let mut lexical = occurrence_rows(reader, "lex_occurs", "true", Variables::new(), false, &budget)?;
    let mut vectors = occurrence_rows(reader, "vec_occurs", "true", Variables::new(), false, &budget)?;
    let client = reader.shared_client(); let cancellation = reader.read_cancellation();
    crate::reader::NativeRows::owned(move |sender| async move {
        let mut actual = SortedRows::with_budget(&budget)?; let mut documents = SortedRows::with_budget(&budget)?;
        for rows in [&mut lexical, &mut vectors] {
            let result = async { while let Some(row) = rows.next().await? {
                let Value::Object(object) = &row else { return Err(ModelError::Schema("actual search occurrence")); };
                let document = RecordId::from_value(object.get("in").cloned().ok_or(ModelError::Schema("actual search document"))?).map_err(ModelError::codec)?;
                let mut pointer = Object::new(); pointer.insert("id", document); documents.push(Value::Object(pointer))?; actual.push(row)?;
            } Ok(()) }.await;
            let mut terminal = lctx_model::domain::completion::Completion::default(); terminal.step("actual search occurrence drainage", rows.drain_transport().await);
            lctx_model::domain::completion::complete(result, terminal)?;
        }
        let mut documents = documents.finish()?;
        loop {
            let mut ids = Vec::new(); while ids.len() < BATCH_ROWS { let Some(row) = documents.next_row()? else { break; };
                let Value::Object(row) = row else { return Err(ModelError::Schema("actual search document pointer")); };
                ids.push(RecordId::from_value(row.get("id").cloned().ok_or(ModelError::Schema("actual search document id"))?).map_err(ModelError::codec)?);
            }
            if ids.is_empty() { break; }
            let mut vars = Variables::new(); vars.insert("ids", ids);
            let mut rows = crate::prepared::PreparedQuery::new(vars, vec![], vec!["SELECT * FROM $ids".into()])?.stream_cancellable(&client, cancellation.as_ref())?;
            let result = async { while let Some(row) = rows.next().await? { actual.push(row)?; } Ok(()) }.await;
            let mut terminal = lctx_model::domain::completion::Completion::default(); terminal.step("actual search document drainage", rows.drain_transport().await);
            lctx_model::domain::completion::complete(result, terminal)?;
        }
        let mut actual = actual.finish()?; while let Some(row) = actual.next_row()? { if sender.send(row).await.is_err() { break; } } Ok(())
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
    reader: &NativeReader<()>,
    ids: impl IntoIterator<Item = Id<R>>,
) -> Result<Index<R>, ModelError> {
    let ids = ids
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let mut result = BTreeMap::new();
    for chunk in ids.chunks(FRONTIER_ROWS) {
        for row in reader
            .records::<R>(RecordSelection::Keys(
                chunk.iter().map(|id| *id.bytes()).collect(),
            ))
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
    reader: &NativeReader<()>,
    field: &str,
    ids: impl IntoIterator<Item = T>,
) -> Result<Index<R>, ModelError> {
    let values = ids
        .into_iter()
        .map(serde_json::to_value)
        .collect::<Result<Vec<_>, _>>()
        .map_err(ModelError::codec)?;
    let rows = reader
        .records::<R>(RecordSelection::Scope {
            field: field.into(),
            values,
        })
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
    reader: &NativeReader<()>,
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
        reader: &NativeReader<()>,
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
fn vector_id(projection: &LoweredProjection, unit: &Unit) -> Result<RecordId, ModelError> {
    Ok(RecordId::new(
        "vector",
        ContentHash::of(
            &serde_json::to_vec(&("native-vector/v2", projection, unit.input, unit.family))
                .map_err(ModelError::codec)?,
        )
        .hex(),
    ))
}
fn cohort_row(p: &LoweredProjection, unit: &Unit) -> Result<Value, ModelError> {
    let mut row = Object::new();
    row.insert("id", vector_id(p, unit)?);
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
                expected.emit("vector", cohort_row(p, unit)?)?;
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
    async fn load(reader: &NativeReader<()>, b: &Basic) -> Result<Self, ModelError> {
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
        let mut rows = reader.record_stream_candidates::<OriginalAnchor>(
            crate::prepared::PreparedQuery::new(vars, vec![], vec!["SELECT id FROM entity WITH INDEX by_scope WHERE semantic_type=$type AND scope_keys CONTAINSANY $scopes AND body.original IN $originals".into()])?, "semantic_key",
        )?;
        let mut anchors = BTreeMap::new();
        let mut original_rows = Index::new();
        while let Some(anchor) = rows.next().await? {
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
        let uses: Index<RetrievalEmbeddingUse> =
            scoped(reader, "window", b.windows.keys().copied()).await?;
        let projections = uses
            .values()
            .filter_map(|row| row.projection.map(|id| id.hex()))
            .collect::<BTreeSet<_>>();
        let selected = reader.prepare_selection().await?;
        let mut values = Vec::new();
        for projection in projections {
            let mut vars = Variables::new(); vars.insert("projection", projection);
            let mut rows = reader.query_stream("SELECT id,projection_key,library_input,family,dependencies FROM vector WITH INDEX cohort WHERE projection_key=$projection", vars, 1)?;
            let result = async { while let Some(row) = rows.next().await? {
                let object = Object::from_value(row).map_err(ModelError::codec)?;
                if let Some(selected) = &selected {
                    let dependencies = Vec::<RecordId>::from_value(object.get("dependencies").cloned().ok_or(ModelError::Schema("vector dependency"))?).map_err(ModelError::codec)?;
                    if dependencies.is_empty() || !selected.contains_all(&dependencies)? { continue; }
                }
                values.push(object);
            } Ok(()) }.await;
            let mut terminal = lctx_model::domain::completion::Completion::default(); terminal.step("vector companion cohort drainage", rows.drain_transport().await);
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
            vectors,
            anchors: supports,
            uses,
        })
    }
}
struct OptionName {
    value: String,
    dependencies: Vec<RecordId>,
}
async fn option_names(
    reader: &NativeReader<()>,
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
            row.insert("id", RecordId::new(table(unit.family), window.digest.hex()));
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
#[allow(
    clippy::too_many_arguments,
    reason = "Canonical lineage and distinct physical writers remain explicit"
)]
async fn emit_witness(
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
        &key,
        RecordId::new(table(unit.family), window.digest.hex()),
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
        if !cause.permits_storage_cleanup() { completion.remote = completion::RemoteState::Unknown; }
        completion::complete::<()>(Err(ModelError::Cause(Box::new(SearchPhaseFailure { phase, cause }))), completion).unwrap_err()
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
pub async fn reconcile_search(loader: &Loader) -> Result<(), ModelError> {
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
    phase(
        "frozen lexical statistics cold comparison",
        crate::lexical_stats::reconcile(loader).await,
    )
}
fn crate_json<T: serde::Serialize>(value: T) -> Result<Value, ModelError> {
    crate::loader::json_value(serde_json::to_value(value).map_err(ModelError::codec)?)
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
    row.insert("id", RecordId::new(table, key));
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
