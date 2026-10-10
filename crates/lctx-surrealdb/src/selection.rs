//! Complete indexed selection with disk-backed identity sets and bounded exact payload reads.
//! Selection is preparation; independent readback and cold admission retain their own checks.
use crate::{ordered_rows::{PreparedPointRows, PreparedRows, SortedRows}, reader::NativeRows};
use lctx_model::domain::{ContentHash, ModelError, completion::{Completion, complete}, resources::{ResourceBudget, MAX_ROW_BYTES}};
use std::sync::Arc;
use surrealdb::{Surreal, engine::remote::grpc::Client, types::{Object, RecordId, RecordIdKey, SurrealValue, Value, Variables}};

const WINDOW: usize = 128;
#[derive(Clone)]
pub struct SelectedPayloads {
    source: SelectionSource,
    nodes: PreparedRows,
    membership: PreparedPointRows,
    aliases: PreparedRows,
    budget: ResourceBudget,
}
#[derive(Clone)]
enum SelectionSource {
    #[cfg(test)]
    Fixture(Arc<dyn Fn(crate::prepared::PreparedQuery) -> Result<NativeRows, ModelError> + Send + Sync>),
    Session(Arc<Surreal<Client>>),
    Transaction(Arc<surrealdb::method::Transaction<Client>>),
    Cancellable(Box<SelectionSource>, Arc<std::sync::atomic::AtomicBool>),
}
impl SelectionSource {
    fn stream(&self, query: crate::prepared::PreparedQuery) -> Result<NativeRows, ModelError> {
        match self {
            #[cfg(test)]
            Self::Fixture(stream) => stream(query),
            Self::Cancellable(source, cancelled) => {
                if cancelled.load(std::sync::atomic::Ordering::Acquire) { return Err(ModelError::Cause(Box::new(std::io::Error::new(std::io::ErrorKind::Interrupted, "selected read delivery cancelled")))); }
                source.stream(query)
            }
            Self::Session(client) => query.stream(client),
            Self::Transaction(transaction) => {
                query.stream_transaction(transaction)
            }
        }
    }
}
fn id(row: &Value) -> Result<RecordId, ModelError> {
    let Value::Object(row) = row else { return Err(ModelError::Schema("selection object")); };
    RecordId::from_value(row.get("id").cloned().ok_or(ModelError::Schema("selection identity"))?).map_err(ModelError::codec)
}
fn pointer(node: RecordId) -> Value {
    let mut row = Object::new(); row.insert("id", node); Value::Object(row)
}
// Only these schema-owned ordering fields are permitted. Hex preserves UTF-8 byte
// order; a terminator below every hex digit preserves prefix order and tuple boundaries.
fn order_string(value: &str, out: &mut String) { out.push_str(&hex::encode(value.as_bytes())); out.push('!'); }
fn order_record(value: &RecordId, out: &mut String) -> Result<(), ModelError> {
    let RecordIdKey::String(key) = &value.key else { return Err(ModelError::Schema("selected string record identity")); };
    order_string(value.table.as_str(), out); order_string(key, out); Ok(())
}
fn order_key(row: &Value, fields: &[String]) -> Result<String, ModelError> {
    let Value::Object(object) = row else { return Err(ModelError::Schema("selected payload row")); };
    let mut key = String::new();
    for field in fields {
        let value = if let Some(body_field) = field.strip_prefix("body.") { object.get("body").and_then(Value::as_object).and_then(|body| body.get(body_field)) } else { object.get(field) }.ok_or(ModelError::Schema("selected ordering field"))?;
        match (field.as_str(), value) {
            ("id" | "anchor", Value::RecordId(value)) => order_record(value, &mut key)?,
            ("semantic_key" | "body.projection" | "body.window", Value::String(value)) => order_string(value, &mut key),
            _ => return Err(ModelError::Schema("selected ordering field type")),
        }
    }
    order_record(&id(row)?, &mut key)?; Ok(key)
}
async fn consume(
    source: &SelectionSource, sql: String, bindings: Variables,
    mut accept: impl FnMut(Value) -> Result<(), ModelError>,
) -> Result<(), ModelError> {
    let mut rows = source.stream(crate::prepared::PreparedQuery::new(bindings, vec![], vec![sql])?)?
        .with_row_bytes(MAX_ROW_BYTES);
    let result = async { while let Some(row) = rows.next().await? { accept(row)?; } Ok(()) }.await;
    let mut terminal = Completion::default();
    terminal.step("selected indexed query drainage", rows.drain_transport().await);
    complete(result, terminal)
}
impl SelectedPayloads {
    pub(crate) fn with_read_cancellation(mut self, flag: Option<Arc<std::sync::atomic::AtomicBool>>) -> Self {
        if let Some(flag) = flag { self.source = SelectionSource::Cancellable(Box::new(self.source), flag); } self
    }
    /// Every root is an equality-indexed stream. No root/membership array is constructed.
    pub async fn for_views(client: Arc<Surreal<Client>>, views: &[ContentHash], budget: &ResourceBudget) -> Result<Self, ModelError> {
        Self::prepare(SelectionSource::Session(client), views, false, budget).await
    }
    pub(crate) async fn for_cancellable_views(client: Arc<Surreal<Client>>, views: &[ContentHash], flag: Option<Arc<std::sync::atomic::AtomicBool>>, budget: &ResourceBudget) -> Result<Self, ModelError> {
        let source = SelectionSource::Session(client);
        let source = if let Some(flag) = flag { SelectionSource::Cancellable(Box::new(source), flag) } else { source };
        Self::prepare(source, views, false, budget).await
    }
    pub(crate) async fn for_cancellable_owners(client: Arc<Surreal<Client>>, owners: &[ContentHash], flag: Option<Arc<std::sync::atomic::AtomicBool>>, budget: &ResourceBudget) -> Result<Self, ModelError> {
        let source = SelectionSource::Session(client);
        let source = if let Some(flag) = flag { SelectionSource::Cancellable(Box::new(source), flag) } else { source };
        Self::prepare(source, owners, true, budget).await
    }
    pub async fn for_transaction_owners(transaction: Arc<surrealdb::method::Transaction<Client>>, owners: &[ContentHash], budget: &ResourceBudget) -> Result<Self, ModelError> {
        Self::prepare(SelectionSource::Transaction(transaction), owners, true, budget).await
    }
    pub async fn for_transaction_views(transaction: Arc<surrealdb::method::Transaction<Client>>, views: &[ContentHash], budget: &ResourceBudget) -> Result<Self, ModelError> {
        Self::prepare(SelectionSource::Transaction(transaction), views, false, budget).await
    }
    pub async fn for_cancellable_transaction(transaction: Arc<surrealdb::method::Transaction<Client>>, roots: &[ContentHash], owners: bool, cancelled: Arc<std::sync::atomic::AtomicBool>, budget: &ResourceBudget) -> Result<Self, ModelError> {
        Self::prepare(SelectionSource::Cancellable(Box::new(SelectionSource::Transaction(transaction)), cancelled), roots, owners, budget).await
    }
    async fn prepare(source: SelectionSource, roots: &[ContentHash], owners: bool, budget: &ResourceBudget) -> Result<Self, ModelError> {
        let mut nodes = SortedRows::with_budget(&budget)?;
        for root in roots {
            let mut vars = Variables::new();
            vars.insert("root", RecordId::new(if owners {"compiler_contribution"} else {"compiler_view"}, root.hex()));
            let sql = if owners {
                "SELECT node FROM compiler_membership WITH INDEX contribution_rows WHERE contribution=$root"
            } else { "SELECT node FROM compiler_view_member WITH INDEX view_nodes WHERE view=$root" };
            consume(&source, sql.into(), vars, |value| {
                let Value::Object(row) = value else { return Err(ModelError::Schema("selected membership pointer")); };
                let node = RecordId::from_value(row.get("node").cloned().ok_or(ModelError::Schema("selected membership node"))?).map_err(ModelError::codec)?;
                nodes.push(pointer(node))
            }).await?;
        }
        Self::expand(source, nodes.finish()?.into_prepared(), budget.clone(), false).await
    }
    async fn expand(source: SelectionSource, mut all: PreparedRows, budget: ResourceBudget, reverse: bool) -> Result<Self, ModelError> {
        let mut frontier = all.clone();
        let mut aliases = SortedRows::with_budget(&budget)?;
        loop {
            let mut cursor = frontier.cursor_with_budget(&budget)?;
            let mut candidates = SortedRows::with_budget(&budget)?;
            loop {
                let mut batch = Vec::new();
                while batch.len() < WINDOW {
                    let Some(row) = cursor.next_row()? else { break; };
                    let node = id(&row)?;
                    if node.table.as_str() == "entity" { batch.push(node); }
                }
                if batch.is_empty() { break; }
                let mut vars = Variables::new(); vars.insert("sources", batch);
                let sql = if reverse { "SELECT source,target,id FROM compiler_alias WITH INDEX alias_target WHERE target IN $sources" } else { "SELECT * FROM compiler_alias WITH INDEX alias_source WHERE source IN $sources" };
                consume(&source, sql.into(), vars, |row| {
                    let Value::Object(object) = &row else { return Err(ModelError::Schema("selected alias")); };
                    let target = RecordId::from_value(object.get(if reverse { "source" } else { "target" }).cloned().ok_or(ModelError::Schema("selected alias target"))?).map_err(ModelError::codec)?;
                    if target.table.as_str() != "entity" { return Err(ModelError::Conflict("selected alias family")); }
                    candidates.push(pointer(target))?; aliases.push(row)
                }).await?;
            }
            // Merge on disk. The next frontier contains only unseen targets, so cycles finish.
            let mut previous = all.cursor_with_budget(&budget)?;
            let mut old = previous.next_row()?;
            let mut candidates = candidates.finish()?;
            let mut merged = SortedRows::with_budget(&budget)?;
            let mut next = SortedRows::with_budget(&budget)?;
            let mut changed = false;
            while let Some(candidate) = candidates.next_row()? {
                let key = id(&candidate)?;
                while old.as_ref().map(id).transpose()?.is_some_and(|old| old < key) {
                    merged.push(old.take().expect("old row"))?; old = previous.next_row()?;
                }
                if old.as_ref().map(id).transpose()?.as_ref() != Some(&key) {
                    changed = true; next.push(candidate.clone())?;
                }
                merged.push(candidate)?;
            }
            while let Some(row) = old.take() { merged.push(row)?; old = previous.next_row()?; }
            all = merged.finish()?.into_prepared();
            if !changed { break; }
            frontier = next.finish()?.into_prepared();
        }
        let membership = all.point_index(&budget)?;
        Ok(Self { source, nodes: all, membership, aliases: aliases.finish()?.into_prepared(), budget })
    }
    /// Immutable borrowers own a fresh cursor; the shared selection never stores mutable position.
    pub fn pointers(&self) -> Result<crate::ordered_rows::OrderedRows, ModelError> { self.nodes.cursor_with_budget(&self.budget) }
    pub(crate) fn pointers_with_budget(&self, budget: &ResourceBudget) -> Result<crate::ordered_rows::OrderedRows, ModelError> { self.nodes.cursor_with_budget(budget) }
    pub fn aliases(&self) -> Result<crate::ordered_rows::OrderedRows, ModelError> { self.aliases.cursor_with_budget(&self.budget) }
    /// Reuse the immutable disk index; each probe reads logarithmically many keys.
    pub fn contains_all(&self, requested: &[RecordId]) -> Result<bool, ModelError> {
        self.contains_all_with_budget(requested, &self.budget)
    }
    pub(crate) fn contains_all_with_budget(&self, requested: &[RecordId], budget: &ResourceBudget) -> Result<bool, ModelError> {
        let mut cursor = self.membership.cursor(budget)?;
        for wanted in requested { if !cursor.contains(wanted)? { return Ok(false); } }
        Ok(true)
    }
    /// Filter a finite candidate window without rescanning the selected universe.
    pub fn filter_members(&self, requested: &[RecordId]) -> Result<Vec<RecordId>, ModelError> {
        self.filter_members_with_budget(requested, &self.budget)
    }
    pub fn filter_members_with_budget(&self, requested: &[RecordId], budget: &ResourceBudget) -> Result<Vec<RecordId>, ModelError> {
        let mut cursor = self.membership.cursor(budget)?;
        let mut accepted = Vec::new();
        for wanted in requested { if cursor.contains(wanted)? { accepted.push(wanted.clone()); } }
        accepted.sort(); accepted.dedup(); Ok(accepted)
    }
    /// Point reads are finite; predicates run before any output limit. Complete global ordering
    /// and deduplication happen in the charged spill owner, never in a closure-wide server array.
    pub fn rows(&self, table: &str, predicate: &str, bindings: Variables, preparation: Vec<String>, order: &str, limit: Option<usize>) -> Result<NativeRows, ModelError> {
        self.rows_with_budget(table, predicate, bindings, preparation, order, limit, &self.budget)
    }
    pub(crate) fn rows_with_budget(&self, table: &str, predicate: &str, bindings: Variables, preparation: Vec<String>, order: &str, limit: Option<usize>, budget: &ResourceBudget) -> Result<NativeRows, ModelError> {
        if !matches!(table, "entity" | "assertion" | "compiler_record") { return Err(ModelError::Schema("selected payload table")); }
        let order = order.split(',').map(str::trim).map(str::to_owned).collect::<Vec<_>>();
        if order.iter().any(|field| !matches!(field.as_str(), "id" | "anchor" | "semantic_key" | "body.projection" | "body.window")) { return Err(ModelError::Schema("selected payload ordering")); }
        let selected = self.clone(); let table = table.to_owned(); let predicate = predicate.to_owned(); let budget = budget.clone();
        NativeRows::owned(move |sender| async move {
            let mut sorted = SortedRows::with_budget_and_row_bytes(&budget, MAX_ROW_BYTES)?;
            let mut cursor = selected.pointers_with_budget(&budget)?;
            loop {
                if sender.is_closed() { return Ok(()); }
                let mut batch = Vec::new();
                while batch.len() < WINDOW {
                    let Some(row) = cursor.next_row()? else { break; };
                    let node = id(&row)?; if node.table.as_str() == table { batch.push(node); }
                }
                if batch.is_empty() { break; }
                let mut vars = bindings.clone(); vars.insert("nodes", batch);
                let query = crate::prepared::PreparedQuery::new(vars, preparation.clone(), vec![format!("SELECT * FROM $nodes WHERE ({predicate})")])?;
                let mut rows = selected.source.stream(query)?.with_row_bytes(MAX_ROW_BYTES);
                let result = async { while let Some(row) = rows.next().await? {
                    let key = order_key(&row, &order)?;
                    let mut wrapped = Object::new(); wrapped.insert("id", RecordId::new("selected_order", key)); wrapped.insert("row", row); sorted.push(Value::Object(wrapped))?;
                } Ok(()) }.await;
                let mut completion = Completion::default(); completion.step("selected payload batch drainage", rows.drain_transport().await); complete(result, completion)?;
            }
            let mut rows = sorted.finish()?; let mut emitted = 0usize;
            while let Some(row) = rows.next_row()? {
                if limit.is_some_and(|limit| emitted >= limit) { break; }
                let Value::Object(mut row) = row else { return Err(ModelError::Schema("selected ordered wrapper")); };
                let row = row.remove("row").ok_or(ModelError::Schema("selected ordered payload"))?;
                if sender.send(row).await.is_err() { break; } emitted += 1;
            }
            Ok(())
        })
    }
    /// Sparse membership starts from nominated physical identities. Reverse aliases
    /// visit only their relevant ancestry; each ancestor window uses exact indexed view
    /// membership. No complete view or payload universe is prepared on this path.
    async fn sparse_filter_members(source: &SelectionSource, views: &[ContentHash], requested: &[RecordId], budget: &ResourceBudget) -> Result<Vec<RecordId>, ModelError> {
        async fn direct(source: &SelectionSource, views: &[ContentHash], nodes: &[RecordId]) -> Result<bool, ModelError> {
            for view in views {
                let mut vars = Variables::new(); vars.insert("view", RecordId::new("compiler_view", view.hex())); vars.insert("nodes", nodes.to_vec());
                let mut found = false;
                consume(source, "SELECT node AS id FROM compiler_view_member WITH INDEX view_nodes WHERE view=$view AND node IN $nodes".into(), vars, |row| {
                    if !nodes.contains(&id(&row)?) { return Err(ModelError::Conflict("sparse membership returned foreign pointer")); } found = true; Ok(())
                }).await?;
                if found { return Ok(true); }
            } Ok(false)
        }
        let mut accepted = Vec::new();
        for node in requested {
            if direct(source, views, std::slice::from_ref(node)).await? { accepted.push(node.clone()); continue; }
            if node.table.as_str() != "entity" || views.is_empty() { continue; }
            let mut seed = SortedRows::with_budget(budget)?; seed.push(pointer(node.clone()))?;
            let ancestors = Self::expand(source.clone(), seed.finish()?.into_prepared(), budget.clone(), true).await?;
            let mut cursor = ancestors.pointers_with_budget(budget)?;
            loop {
                let mut nodes = Vec::new(); while nodes.len() < WINDOW { let Some(row) = cursor.next_row()? else { break; }; nodes.push(id(&row)?); }
                if nodes.is_empty() { break; }
                if direct(source, views, &nodes).await? { accepted.push(node.clone()); break; }
            }
        }
        accepted.sort(); accepted.dedup(); Ok(accepted)
    }
    pub(crate) async fn filter_view_candidates(client: Arc<Surreal<Client>>, views: &[ContentHash], cancelled: Option<Arc<std::sync::atomic::AtomicBool>>, requested: &[RecordId], budget: &ResourceBudget) -> Result<Vec<RecordId>, ModelError> {
        let source = SelectionSource::Session(client);
        let source = if let Some(flag) = cancelled { SelectionSource::Cancellable(Box::new(source), flag) } else { source };
        Self::sparse_filter_members(&source, views, requested, budget).await
    }
    pub(crate) fn sparse_candidate_rows(client: Arc<Surreal<Client>>, views: Vec<ContentHash>, cancelled: Option<Arc<std::sync::atomic::AtomicBool>>, candidates: crate::prepared::PreparedQuery, table: &str, order: &str, budget: &ResourceBudget) -> Result<NativeRows, ModelError> {
        let source = SelectionSource::Session(client);
        let source = if let Some(flag) = cancelled { SelectionSource::Cancellable(Box::new(source), flag) } else { source };
        Self::candidate_rows_inner(source, None, views, candidates, table, order, budget)
    }
    /// Compact indexed nominations are intersected with the exact selected identity set
    /// before any canonical/body payload is fetched. Sorting owns request scratch only.
    pub fn candidate_rows(&self, candidates: crate::prepared::PreparedQuery, table: &str,
        order: &str, budget: &ResourceBudget) -> Result<NativeRows, ModelError> {
        Self::candidate_rows_inner(self.source.clone(), Some(self.clone()), vec![], candidates, table, order, budget)
    }
    fn candidate_rows_inner(source: SelectionSource, selected: Option<Self>, views: Vec<ContentHash>, candidates: crate::prepared::PreparedQuery, table: &str, order: &str, budget: &ResourceBudget) -> Result<NativeRows, ModelError> {
        if !matches!(table, "entity" | "assertion" | "compiler_record") { return Err(ModelError::Schema("selected candidate table")); }
        let order = order.split(',').map(str::trim).map(str::to_owned).collect::<Vec<_>>();
        if order.iter().any(|field| !matches!(field.as_str(), "id" | "anchor" | "semantic_key" | "body.projection" | "body.window")) { return Err(ModelError::Schema("selected candidate ordering")); }
        let table = table.to_owned(); let budget = budget.clone();
        NativeRows::owned(move |sender| async move {
            let mut candidates = source.stream(candidates)?.with_row_bytes(MAX_ROW_BYTES);
            let result = async {
                let mut sorted = SortedRows::with_budget_and_row_bytes(&budget, MAX_ROW_BYTES)?;
                loop {
                    if sender.is_closed() { candidates.cancel_delivery(); break; }
                    let mut window = lctx_model::domain::charged::StateCharge::new(&budget, "selected-candidate-window");
                    let mut nominated = Vec::with_capacity(WINDOW);
                    let mut examined = 0usize;
                    while examined < WINDOW {
                        let Some(row) = candidates.next().await? else { break; }; examined += 1;
                        window.grow(crate::loader::native_bytes(&row).saturating_add(std::mem::size_of::<RecordId>()))?;
                        let node = id(&row)?;
                        if node.table.as_str() == table { nominated.push(node); }
                    }
                    if examined == 0 { break; }
                    if nominated.is_empty() { continue; }
                    let nodes = match &selected { Some(selected) => selected.filter_members_with_budget(&nominated, &budget)?, None => Self::sparse_filter_members(&source, &views, &nominated, &budget).await? };
                    if nodes.is_empty() { continue; }
                    let vars = Variables::from_iter([("nodes".into(), nodes.into_value())]);
                    let mut payloads = source.stream(crate::prepared::PreparedQuery::new(vars, vec![], vec!["SELECT * FROM $nodes".into()])?)?.with_row_bytes(MAX_ROW_BYTES);
                    let result = async {
                        while let Some(row) = payloads.next().await? {
                            let key = if order.as_slice() == ["semantic_key"] {
                                let Value::Object(object) = &row else { return Err(ModelError::Schema("candidate semantic row")); };
                                let Some(Value::String(key)) = object.get("semantic_key") else { return Err(ModelError::Schema("candidate semantic key")); }; key.clone()
                            } else { order_key(&row, &order)? };
                            let mut wrapped = Object::new(); wrapped.insert("id", RecordId::new("selected_order", key)); wrapped.insert("row", row); sorted.push(Value::Object(wrapped))?;
                        } Ok(())
                    }.await;
                    let mut terminal = Completion::default(); terminal.step("selected candidate payload drainage", payloads.drain_transport().await); complete(result, terminal)?;
                }
                let mut sorted = sorted.finish()?;
                while let Some(row) = sorted.next_row()? {
                    let Value::Object(mut row) = row else { return Err(ModelError::Schema("selected candidate wrapper")); };
                    if sender.send(row.remove("row").ok_or(ModelError::Schema("selected candidate payload"))?).await.is_err() { break; }
                }
                Ok(())
            }.await;
            let mut terminal = Completion::default(); terminal.step("selected candidate nomination drainage", candidates.drain_transport().await); complete(result, terminal)
        })
    }
    /// Raw source roles plus their actual external endpoints. These are integrity claims,
    /// never nominations constrained by canonical expected roles.
    pub fn recovery_roles(&self) -> Result<NativeRows, ModelError> { self.recovery_derived(true) }
    /// Raw source occurrences plus actual referenced documents/vectors, including invalid
    /// dependency lists and unexpected eligible rows for independent cold comparison.
    pub fn recovery_search(&self) -> Result<NativeRows, ModelError> { self.recovery_derived(false) }
    fn recovery_derived(&self, roles: bool) -> Result<NativeRows, ModelError> {
        let selected = self.clone();
        NativeRows::owned(move |sender| async move {
            let mut actual = SortedRows::with_budget(&selected.budget)?; let mut endpoints = SortedRows::with_budget(&selected.budget)?;
            for table in crate::compiler::recovery_tables(if roles { lctx_model::domain::recovery_closure::RecoveryFamily::GraphRoles } else { lctx_model::domain::recovery_closure::RecoveryFamily::SearchOccurrences }) {
                let mut cursor = selected.pointers()?;
                loop {
                    if sender.is_closed() { return Ok(()); }
                    let mut nodes = Vec::new(); while nodes.len() < WINDOW { let Some(row) = cursor.next_row()? else { break; }; nodes.push(id(&row)?); }
                    if nodes.is_empty() { break; }
                    let mut vars = Variables::new(); vars.insert("nodes", nodes);
                    let (index, field) = if roles { ("outgoing", "in") } else { ("exact_unit_payload", "unit_payload") };
                    consume(&selected.source, format!("SELECT * FROM {table} WITH INDEX {index} WHERE {field} IN $nodes"), vars, |row| {
                        let Value::Object(object) = &row else { return Err(ModelError::Schema("recovery derived object")); };
                        let endpoint = RecordId::from_value(object.get(if roles {"out"} else {"in"}).cloned().ok_or(ModelError::Schema("recovery derived endpoint"))?).map_err(ModelError::codec)?;
                        if !roles || endpoint.table.as_str() == "external" { endpoints.push(pointer(endpoint))?; }
                        actual.push(row)
                    }).await?;
                }
            }
            let mut endpoints = endpoints.finish()?;
            loop {
                if sender.is_closed() { return Ok(()); }
                let mut ids = Vec::new(); while ids.len() < WINDOW { let Some(row) = endpoints.next_row()? else { break; }; ids.push(id(&row)?); }
                if ids.is_empty() { break; }
                let mut vars = Variables::new(); vars.insert("ids", ids);
                consume(&selected.source, "SELECT * FROM $ids".into(), vars, |row| actual.push(row)).await?;
            }
            let mut actual = actual.finish()?; while let Some(row) = actual.next_row()? { if sender.send(row).await.is_err() { break; } } Ok(())
        })
    }
    /// Enumerate every actual outgoing edge, including unexpected roles/fields, from exact
    /// selected source windows. Expected rows never constrain this integrity source.
    pub fn outgoing(&self, table: &str) -> Result<NativeRows, ModelError> {
        if !matches!(table, "participant" | "reference" | "compiler_alias") { return Err(ModelError::Schema("selected outgoing family")); }
        let selected = self.clone(); let table = table.to_owned();
        NativeRows::owned(move |sender| async move {
            let mut sorted = SortedRows::with_budget(&selected.budget)?; let mut cursor = selected.pointers()?;
            loop {
                if sender.is_closed() { return Ok(()); }
                let mut batch = Vec::new(); while batch.len() < WINDOW { let Some(row) = cursor.next_row()? else { break; }; batch.push(id(&row)?); }
                if batch.is_empty() { break; }
                let mut vars = Variables::new(); vars.insert("nodes", batch);
                let (index, field) = if table == "compiler_alias" {("alias_source", "source")} else {("outgoing", "in")};
                consume(&selected.source, format!("SELECT * FROM {table} WITH INDEX {index} WHERE {field} IN $nodes"), vars, |row| sorted.push(row)).await?;
            }
            let mut rows = sorted.finish()?; while let Some(row) = rows.next_row()? { if sender.send(row).await.is_err() { break; } } Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selected_order_preserves_schema_field_tuple_order() {
        let strings = ["", "a", "a!", "a\"", "a'", "a|", "aa", "b", "é", "🍎"];
        let mut rows = Vec::new();
        for (n, text) in strings.iter().enumerate() {
            for table in ["anchor", "anchor!", "anchor2"] {
                let mut row = Object::new(); row.insert("id", RecordId::new("entity", format!("id{n}")));
                row.insert("anchor", RecordId::new(table, *text)); row.insert("semantic_key", *text); rows.push(Value::Object(row));
            }
        }
        for fields in [vec!["id"], vec!["anchor", "id"], vec!["semantic_key", "anchor", "id"]] {
            let fields = fields.into_iter().map(str::to_owned).collect::<Vec<_>>();
            for left in &rows { for right in &rows {
                let native = |value: &Value| { let Value::Object(row) = value else { unreachable!() }; fields.iter().map(|field| row.get(field).unwrap().clone()).chain([Value::RecordId(id(value).unwrap())]).collect::<Vec<_>>() };
                assert_eq!(order_key(left, &fields).unwrap().cmp(&order_key(right, &fields).unwrap()), native(left).cmp(&native(right)));
            } }
        }
        let mut row = Object::new(); row.insert("id", RecordId::new("entity", 1i64));
        assert!(order_key(&Value::Object(row), &["id".into()]).is_err());
    }
    #[test]
    fn finite_member_probe_preserves_aliases_and_absence() {
        let budget = ResourceBudget::fixed(1024 * 1024).unwrap();
        let mut nodes = SortedRows::with_budget(&budget).unwrap();
        for key in ["a", "c", "z"] { nodes.push(pointer(RecordId::new("entity", key))).unwrap(); }
        let nodes = nodes.finish().unwrap().into_prepared();
        let membership = nodes.point_index(&budget).unwrap();
        let selected = SelectedPayloads { membership, source: SelectionSource::Session(Arc::new(Surreal::init())), nodes, aliases: SortedRows::with_budget(&budget).unwrap().finish().unwrap().into_prepared(), budget };
        assert_eq!(selected.filter_members(&[RecordId::new("entity", "z"), RecordId::new("entity", "b"), RecordId::new("entity", "a"), RecordId::new("entity", "z")]).unwrap(), vec![RecordId::new("entity", "a"), RecordId::new("entity", "z")]);
        assert!(selected.filter_members(&[]).unwrap().is_empty());
    }
    #[tokio::test]
    async fn sparse_candidate_driver_never_enumerates_payload_universe_and_composes_budgets() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let retained = ResourceBudget::fixed(8 << 20).unwrap();
        let request = ResourceBudget::scoped(&retained, 1 << 20).unwrap();
        let mut nodes = SortedRows::with_budget(&retained).unwrap();
        for n in 0..512 { nodes.push(pointer(RecordId::new("entity", format!("selected-{n:04}")))).unwrap(); }
        let nodes = nodes.finish().unwrap().into_prepared();
        let membership = nodes.point_index(&retained).unwrap();
        let payload_reads = Arc::new(AtomicUsize::new(0)); let observed = payload_reads.clone();
        let selected = SelectedPayloads {
            nodes, membership, aliases: SortedRows::with_budget(&retained).unwrap().finish().unwrap().into_prepared(), budget: retained.clone(),
            source: SelectionSource::Fixture(Arc::new(move |query| {
                let (sql, vars) = query.into_request();
                let values = if sql == "NOMINATE;" {
                    vec![pointer(RecordId::new("entity", "selected-0001")), pointer(RecordId::new("entity", "outside-view"))]
                } else if sql.starts_with("SELECT node AS id FROM compiler_view_member WITH INDEX view_nodes") {
                    let nodes = Vec::<RecordId>::from_value(vars.get("nodes").unwrap().clone()).unwrap();
                    assert!(nodes.len() <= WINDOW); nodes.into_iter().filter(|node| node == &RecordId::new("entity", "selected-0001")).map(pointer).collect()
                } else if sql.starts_with("SELECT source,target,id FROM compiler_alias WITH INDEX alias_target") { vec![]
                } else {
                    assert_eq!(sql, "SELECT * FROM $nodes;", "sparse candidates must never open a complete selected payload scan");
                    let nodes = Vec::<RecordId>::from_value(vars.get("nodes").unwrap().clone()).unwrap();
                    assert_eq!(nodes, vec![RecordId::new("entity", "selected-0001")]);
                    observed.fetch_add(nodes.len(), Ordering::SeqCst);
                    let mut row = Object::new(); row.insert("id", nodes[0].clone()); row.insert("semantic_key", "key");
                    vec![Value::Object(row)]
                };
                NativeRows::owned(move |sender| async move { for row in values { if sender.send(row).await.is_err() { break; } } Ok(()) })
            })),
        };
        let baseline = retained.reserved();
        for _ in 0..3 {
            let query = crate::prepared::PreparedQuery::new(Variables::new(), vec![], vec!["NOMINATE".into()]).unwrap();
            let mut rows = SelectedPayloads::candidate_rows_inner(selected.source.clone(), None, vec![ContentHash::of(b"exact-view")], query, "entity", "semantic_key", &request).unwrap();
            assert!(rows.next().await.unwrap().is_some()); assert!(rows.next().await.unwrap().is_none()); rows.drain_transport().await.unwrap(); drop(rows);
            assert_eq!(request.reserved(), 0); assert_eq!(retained.reserved(), baseline);
        }
        assert_eq!(payload_reads.load(Ordering::SeqCst), 3, "512 selected identities do not cause 512 payload hydrations per point request");
        let refused = ResourceBudget::scoped(&retained, 1).unwrap();
        let query = crate::prepared::PreparedQuery::new(Variables::new(), vec![], vec!["NOMINATE".into()]).unwrap();
        let mut rows = SelectedPayloads::candidate_rows_inner(selected.source.clone(), None, vec![ContentHash::of(b"exact-view")], query, "entity", "id", &refused).unwrap();
        assert!(rows.next().await.is_err()); rows.drain_transport().await.unwrap(); drop(rows);
        assert_eq!(payload_reads.load(Ordering::SeqCst), 3, "scratch refusal happens before payload hydration");
        assert_eq!(refused.reserved(), 0);
        let borrower = selected.clone(); drop(selected); assert_eq!(retained.reserved(), baseline);
        drop(borrower); assert_eq!(retained.reserved(), 0);
    }

}
