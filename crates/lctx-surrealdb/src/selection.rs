//! Complete indexed selection with disk-backed identity sets and bounded exact payload reads.
//! Selection is preparation; independent readback and cold admission retain their own checks.
use crate::{ordered_rows::{PreparedPointRows, PreparedRows, SortedRows}, reader::NativeRows};
use lctx_model::domain::{ContentHash, ModelError, completion::{Completion, complete}, resources::{ResourceBudget, MAX_ROW_BYTES}};
use std::{sync::Arc, collections::{BTreeMap, BTreeSet, VecDeque}};
use lctx_model::domain::charged::StateCharge;
use surrealdb::{Surreal, engine::remote::grpc::Client, types::{Object, RecordId, RecordIdKey, SurrealValue, Value, Variables}};

const WINDOW: usize = 128;
/// One exact selected-view witness. `ancestry` starts at the requested physical node
/// and follows reverse aliases to the directly admitted member; cycles never repeat.
#[derive(Debug)]
pub struct MembershipWitness {
    pub view: ContentHash,
    pub member: RecordId,
    pub relation: String,
    pub semantic_key: String,
    pub content: String,
    pub ancestry: Vec<RecordId>,
}
#[derive(Debug)]
pub struct NodeMembership {
    pub requested: RecordId,
    pub selected: bool,
    pub witnesses: Vec<MembershipWitness>,
}
/// Request-local results retain their allocation authority through the last consumer.
pub struct MembershipAnswers {
    pub answers: Vec<NodeMembership>,
    _charge: StateCharge,
}
impl MembershipAnswers {
    pub fn contains(&self, node: &RecordId) -> bool {
        self.answers.binary_search_by(|answer| answer.requested.cmp(node)).ok().is_some_and(|index| self.answers[index].selected)
    }
    pub fn contains_all(&self, nodes: &[RecordId]) -> bool { nodes.iter().all(|node| self.contains(node)) }
    pub(crate) fn unrestricted(requested: &[RecordId], budget: &ResourceBudget) -> Result<Self, ModelError> {
        let mut charge = StateCharge::new(budget, "selected-membership-answers");
        let mut answers = BTreeMap::new();
        for node in requested {
            if !answers.contains_key(node) {
                charge.grow(node_bytes(node).saturating_mul(2).saturating_add(std::mem::size_of::<NodeMembership>() + 32))?;
                answers.insert(node.clone(), NodeMembership { requested: node.clone(), selected: true, witnesses: vec![] });
            }
        }
        Ok(Self { answers: answers.into_values().collect(), _charge: charge })
    }
}
pub(crate) fn node_bytes(node: &RecordId) -> usize {
    let key = match &node.key { RecordIdKey::String(key) => key.len(), RecordIdKey::Number(_) | RecordIdKey::Uuid(_) => 0, _ => MAX_ROW_BYTES };
    std::mem::size_of::<RecordId>().saturating_add(node.table.as_str().len()).saturating_add(key)
}
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
enum CandidateNominations {
    Query(crate::prepared::PreparedQuery),
    Relation(String),
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
        Self::expand(source, nodes.finish()?.into_prepared(), budget.clone()).await
    }
    async fn expand(source: SelectionSource, all: PreparedRows, budget: ResourceBudget) -> Result<Self, ModelError> {
        let (nodes, aliases) = Self::expand_nodes(&source, all, &budget, true).await?;
        let membership = nodes.point_index(&budget)?;
        Ok(Self { source, nodes, membership, aliases, budget })
    }
    async fn expand_nodes(source: &SelectionSource, mut all: PreparedRows, budget: &ResourceBudget, retain_aliases: bool) -> Result<(PreparedRows, PreparedRows), ModelError> {
        let mut frontier = all.clone();
        let mut aliases = SortedRows::with_budget(budget)?;
        loop {
            let mut cursor = frontier.cursor_with_budget(budget)?;
            let mut candidates = SortedRows::with_budget(budget)?;
            loop {
                let mut batch = Vec::new();
                while batch.len() < WINDOW {
                    let Some(row) = cursor.next_row()? else { break; };
                    let node = id(&row)?;
                    if node.table.as_str() == "entity" { batch.push(node); }
                }
                if batch.is_empty() { break; }
                let mut vars = Variables::new(); vars.insert("sources", batch);
                let sql = "SELECT * FROM compiler_alias WITH INDEX alias_source WHERE source IN $sources";
                consume(source, sql.into(), vars, |row| {
                    let Value::Object(object) = &row else { return Err(ModelError::Schema("selected alias")); };
                    let target = RecordId::from_value(object.get("target").cloned().ok_or(ModelError::Schema("selected alias target"))?).map_err(ModelError::codec)?;
                    if target.table.as_str() != "entity" { return Err(ModelError::Conflict("selected alias family")); }
                    candidates.push(pointer(target))?;
                    if retain_aliases { aliases.push(row)?; } Ok(())
                }).await?;
            }
            // Merge on disk. The next frontier contains only unseen targets, so cycles finish.
            let mut previous = all.cursor_with_budget(budget)?;
            let mut old = previous.next_row()?;
            let mut candidates = candidates.finish()?;
            let mut merged = SortedRows::with_budget(budget)?;
            let mut next = SortedRows::with_budget(budget)?;
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
        Ok((all, aliases.finish()?.into_prepared()))
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
    async fn validate_nominal_membership(
        source: &SelectionSource, views: &[ContentHash], direct: &BTreeMap<RecordId, BTreeMap<ContentHash, (String, String, String)>>, budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        let mut charge = StateCharge::new(budget, "selected-nominal-membership");
        let mut expected: BTreeMap<&str, BTreeMap<&str, (&RecordId, &str)>> = BTreeMap::new();
        for (node, witnesses) in direct {
            for (relation, key, content) in witnesses.values() {
                if let Some(previous) = expected.get(relation.as_str()).and_then(|keys| keys.get(key.as_str())) {
                    if previous != &(node, content.as_str()) { return Err(ModelError::Conflict("selected nominal membership variants")); }
                } else {
                    charge.grow(160)?;
                    expected.entry(relation).or_default().insert(key, (node, content));
                }
            }
        }
        // A physical point hit alone cannot establish an unambiguous nominal answer:
        // another selected view may admit an un-nominated revision of that same key.
        // Probe only witnessed keys, using the exact view/relation index prefix.
        for (relation, expected_keys) in &expected {
            for view in views {
                let mut keys = expected_keys.keys();
                loop {
                    let mut window = StateCharge::new(budget, "selected-nominal-membership-window");
                    let mut nominated = Vec::new();
                    while nominated.len() < WINDOW {
                        let Some(key) = keys.next() else { break; };
                        window.grow(key.len().saturating_mul(5).saturating_add(256))?;
                        nominated.push((*key).to_owned());
                    }
                    if nominated.is_empty() { break; }
                    let mut vars = Variables::new(); vars.insert("view", RecordId::new("compiler_view", view.hex())); vars.insert("relation", (*relation).to_owned()); vars.insert("keys", nominated.clone());
                    let mut seen = BTreeSet::new();
                    consume(source, "SELECT node AS id,relation,semantic_key,content FROM compiler_view_member WITH INDEX view_key WHERE view=$view AND relation=$relation AND semantic_key IN $keys".into(), vars, |row| {
                        let _scratch = budget.reserve("selected-nominal-membership-row", crate::loader::native_bytes(&row).saturating_mul(4))?;
                        let node = id(&row)?;
                        let Value::Object(object) = row else { return Err(ModelError::Schema("nominal membership witness object")); };
                        let text = |name| String::from_value(object.get(name).cloned().ok_or(ModelError::Schema("nominal membership witness field"))?).map_err(ModelError::codec);
                        let actual_relation = text("relation")?; let key = text("semantic_key")?; let content = text("content")?;
                        if actual_relation != *relation || !nominated.contains(&key) { return Err(ModelError::Conflict("nominal membership returned foreign key")); }
                        let Some((expected_node, expected_content)) = expected_keys.get(key.as_str()) else { return Err(ModelError::Conflict("nominal membership witness key")); };
                        if &node != *expected_node || content != *expected_content { return Err(ModelError::Conflict("selected nominal membership variants")); }
                        if !seen.insert(key) { return Err(ModelError::Conflict("duplicate nominal membership witness")); }
                        Ok(())
                    }).await?;
                    for key in &nominated {
                        let (node, _) = expected_keys[key.as_str()];
                        if direct.get(node).is_some_and(|witnesses| witnesses.contains_key(view)) && !seen.contains(key) { return Err(ModelError::Conflict("nominal membership witness disappeared")); }
                    }
                }
            }
        }
        Ok(())
    }
    async fn sparse_membership(source: &SelectionSource, views: &[ContentHash], requested: &[RecordId], budget: &ResourceBudget) -> Result<MembershipAnswers, ModelError> {
        let mut graph_charge = StateCharge::new(budget, "selected-membership-ancestry");
        let mut graph: BTreeMap<RecordId, BTreeSet<RecordId>> = BTreeMap::new();
        let mut direct: BTreeMap<RecordId, BTreeMap<ContentHash, (String, String, String)>> = BTreeMap::new();
        let mut requested_nodes = BTreeSet::new();
        let mut frontier = VecDeque::new();
        for node in requested {
            if !graph.contains_key(node) {
                graph_charge.grow(node_bytes(node).saturating_mul(4).saturating_add(192))?;
                graph.insert(node.clone(), BTreeSet::new()); frontier.push_back(node.clone()); requested_nodes.insert(node.clone());
            }
        }
        // Each physical ancestor is read once for the whole requested window. Queries
        // stay finite even when several requests share an ancestry or form a cycle.
        while !frontier.is_empty() && !views.is_empty() {
            let mut window = StateCharge::new(budget, "selected-membership-window");
            let mut nodes = Vec::new();
            while nodes.len() < WINDOW {
                let Some(node) = frontier.pop_front() else { break; };
                window.grow(node_bytes(&node).saturating_mul(3))?; nodes.push(node);
            }
            for view in views {
                let mut vars = Variables::new(); vars.insert("view", RecordId::new("compiler_view", view.hex())); vars.insert("nodes", nodes.clone());
                consume(source, "SELECT node AS id,relation,semantic_key,content FROM compiler_view_member WITH INDEX view_nodes WHERE view=$view AND node IN $nodes".into(), vars, |row| {
                    let _scratch = budget.reserve("selected-membership-row", crate::loader::native_bytes(&row).saturating_mul(4))?;
                    let node = id(&row)?;
                    if !nodes.contains(&node) { return Err(ModelError::Conflict("sparse membership returned foreign pointer")); }
                    let Value::Object(object) = row else { return Err(ModelError::Schema("membership witness object")); };
                    let text = |name| String::from_value(object.get(name).cloned().ok_or(ModelError::Schema("membership witness field"))?).map_err(ModelError::codec);
                    let nominal = (text("relation")?, text("semantic_key")?, text("content")?);
                    if direct.get(&node).is_some_and(|members| members.values().any(|previous| previous != &nominal)) { return Err(ModelError::Conflict("membership physical/nominal witness changed")); }
                    if let Some(previous) = direct.get(&node).and_then(|members| members.get(view)) {
                        if previous != &nominal { return Err(ModelError::Conflict("membership physical/nominal witness changed")); }
                    } else {
                        graph_charge.grow(node_bytes(&node).saturating_add(256).saturating_add(nominal.0.len()).saturating_add(nominal.1.len()).saturating_add(nominal.2.len()))?;
                        direct.entry(node).or_default().insert(*view, nominal);
                    }
                    Ok(())
                }).await?;
            }
            nodes.retain(|node| node.table.as_str() == "entity");
            if nodes.is_empty() { continue; }
            let mut vars = Variables::new(); vars.insert("sources", nodes.clone());
            consume(source, "SELECT source,target,id FROM compiler_alias WITH INDEX alias_target WHERE target IN $sources".into(), vars, |row| {
                let _scratch = budget.reserve("selected-membership-alias-row", crate::loader::native_bytes(&row).saturating_mul(4))?;
                let Value::Object(object) = row else { return Err(ModelError::Schema("sparse alias object")); };
                let ancestor = RecordId::from_value(object.get("source").cloned().ok_or(ModelError::Schema("sparse alias source"))?).map_err(ModelError::codec)?;
                let target = RecordId::from_value(object.get("target").cloned().ok_or(ModelError::Schema("sparse alias target"))?).map_err(ModelError::codec)?;
                if ancestor.table.as_str() != "entity" || !nodes.contains(&target) { return Err(ModelError::Conflict("sparse alias ancestry")); }
                if !graph.contains_key(&ancestor) {
                    graph_charge.grow(node_bytes(&ancestor).saturating_mul(3).saturating_add(160))?;
                    graph.insert(ancestor.clone(), BTreeSet::new()); frontier.push_back(ancestor.clone());
                }
                let parents = graph.get_mut(&target).ok_or(ModelError::Conflict("sparse alias frontier"))?;
                if !parents.contains(&ancestor) { graph_charge.grow(node_bytes(&ancestor).saturating_add(32))?; parents.insert(ancestor); }
                Ok(())
            }).await?;
        }
        Self::validate_nominal_membership(source, views, &direct, budget).await?;
        let mut charge = StateCharge::new(budget, "selected-membership-answers");
        let mut answers = Vec::new();
        for node in &requested_nodes {
            charge.grow(node_bytes(node).saturating_add(std::mem::size_of::<NodeMembership>()))?;
            let mut witnesses = Vec::new();
            let mut scratch = StateCharge::new(budget, "selected-membership-paths");
            scratch.grow(node_bytes(node).saturating_mul(3).saturating_add(96))?;
            let mut paths = BTreeMap::from([(node.clone(), vec![node.clone()])]);
            let mut pending = VecDeque::from([node.clone()]);
            while let Some(member) = pending.pop_front() {
                let path = &paths[&member];
                if let Some(member_views) = direct.get(&member) {
                    for (view, nominal) in member_views {
                        charge.grow(nominal.0.len().saturating_add(nominal.1.len()).saturating_add(nominal.2.len()).saturating_add(std::mem::size_of::<MembershipWitness>().saturating_add(node_bytes(&member)).saturating_add(path.iter().map(node_bytes).sum::<usize>())))?;
                        witnesses.push(MembershipWitness { view: *view, member: member.clone(), relation: nominal.0.clone(), semantic_key: nominal.1.clone(), content: nominal.2.clone(), ancestry: path.clone() });
                    }
                }
                if let Some(parents) = graph.get(&member) {
                    for parent in parents {
                        if !paths.contains_key(parent) {
                            let path = &paths[&member];
                            scratch.grow(node_bytes(parent).saturating_mul(3).saturating_add(96).saturating_add(path.iter().map(node_bytes).sum::<usize>()))?;
                            let mut path = path.clone(); path.push(parent.clone()); paths.insert(parent.clone(), path); pending.push_back(parent.clone());
                        }
                    }
                }
            }
            answers.push(NodeMembership { requested: node.clone(), selected: !witnesses.is_empty(), witnesses });
        }
        Ok(MembershipAnswers { answers, _charge: charge })
    }
    pub(crate) async fn view_membership(client: Arc<Surreal<Client>>, views: &[ContentHash], cancelled: Option<Arc<std::sync::atomic::AtomicBool>>, requested: &[RecordId], budget: &ResourceBudget) -> Result<MembershipAnswers, ModelError> {
        let source = SelectionSource::Session(client);
        let source = if let Some(flag) = cancelled { SelectionSource::Cancellable(Box::new(source), flag) } else { source };
        Self::sparse_membership(&source, views, requested, budget).await
    }
    pub(crate) fn sparse_candidate_rows(client: Arc<Surreal<Client>>, views: Vec<ContentHash>, cancelled: Option<Arc<std::sync::atomic::AtomicBool>>, candidates: crate::prepared::PreparedQuery, table: &str, order: &str, budget: &ResourceBudget) -> Result<NativeRows, ModelError> {
        let source = SelectionSource::Session(client);
        let source = if let Some(flag) = cancelled { SelectionSource::Cancellable(Box::new(source), flag) } else { source };
        Self::candidate_rows_inner(source, None, views, candidates, table, order, budget)
    }
    pub(crate) fn sparse_relation_rows(client: Arc<Surreal<Client>>, views: Vec<ContentHash>, cancelled: Option<Arc<std::sync::atomic::AtomicBool>>, relation: &str, table: &str, budget: &ResourceBudget) -> Result<NativeRows, ModelError> {
        let source = SelectionSource::Session(client);
        let source = if let Some(flag) = cancelled { SelectionSource::Cancellable(Box::new(source), flag) } else { source };
        Self::candidate_rows_from(source, None, views, CandidateNominations::Relation(relation.to_owned()), table, "semantic_key", budget)
    }
    /// Nominate from exact selected relation membership, not the global payload
    /// family. Only model-governed endpoint sources can widen this input scope.
    fn relation_nominations(source: SelectionSource, views: Vec<ContentHash>, relation: String, budget: ResourceBudget) -> Result<NativeRows, ModelError> {
        NativeRows::owned(move |sender| async move {
            let mut nominated = SortedRows::with_budget(&budget)?;
            let mut seeds = SortedRows::with_budget(&budget)?;
            let source_relations = lctx_model::domain::graph::Entity::canonical_place_endpoint_relations().iter()
                .filter_map(|(source, target)| (*target == relation).then_some(*source)).collect::<BTreeSet<_>>();
            for view in &views {
                for selected_relation in std::iter::once(relation.as_str()).chain(source_relations.iter().copied()) {
                    if sender.is_closed() { return Ok(()); }
                    let mut vars = Variables::new(); vars.insert("view", RecordId::new("compiler_view", view.hex())); vars.insert("relation", selected_relation.to_owned());
                    consume(&source, "SELECT node AS id FROM compiler_view_member WITH INDEX view_key WHERE view=$view AND relation=$relation".into(), vars, |row| {
                        let node = id(&row)?;
                        if selected_relation == relation { nominated.push(pointer(node)) }
                        else if node.table.as_str() != "entity" { Err(ModelError::Conflict("canonical endpoint source family")) }
                        else { seeds.push(pointer(node)) }
                    }).await?;
                }
            }
            if !source_relations.is_empty() && !sender.is_closed() {
                // Reuse the cycle-safe disk-backed forward traversal on just these
                // selected source relations. No complete view is prepared here.
                let (expanded, _) = Self::expand_nodes(&source, seeds.finish()?.into_prepared(), &budget, false).await?;
                let mut cursor = expanded.cursor_with_budget(&budget)?;
                loop {
                    if sender.is_closed() { return Ok(()); }
                    let mut window = StateCharge::new(&budget, "relation-alias-window");
                    let mut nodes = Vec::new();
                    while nodes.len() < WINDOW {
                        let Some(row) = cursor.next_row()? else { break; };
                        let node = id(&row)?; window.grow(node_bytes(&node).saturating_mul(3))?; nodes.push(node);
                    }
                    if nodes.is_empty() { break; }
                    let mut vars = Variables::new(); vars.insert("nodes", nodes.clone()); vars.insert("relation", relation.clone());
                    // Exact reached IDs retain intermediates for chain traversal;
                    // only endpoints of the requested relation become candidates.
                    consume(&source, "SELECT id FROM $nodes WHERE semantic_type=$relation".into(), vars, |row| {
                        let node = id(&row)?;
                        if !nodes.contains(&node) { return Err(ModelError::Conflict("canonical endpoint nomination correspondence")); }
                        nominated.push(pointer(node))
                    }).await?;
                }
            }
            let mut nominated = nominated.finish()?;
            while let Some(row) = nominated.next_row()? {
                if sender.send(row).await.is_err() { break; }
            }
            Ok(())
        })
    }
    /// Compact indexed nominations are intersected with the exact selected identity set
    /// before any canonical/body payload is fetched. Sorting owns request scratch only.
    pub fn candidate_rows(&self, candidates: crate::prepared::PreparedQuery, table: &str,
        order: &str, budget: &ResourceBudget) -> Result<NativeRows, ModelError> {
        Self::candidate_rows_inner(self.source.clone(), Some(self.clone()), vec![], candidates, table, order, budget)
    }
    fn candidate_rows_inner(source: SelectionSource, selected: Option<Self>, views: Vec<ContentHash>, candidates: crate::prepared::PreparedQuery, table: &str, order: &str, budget: &ResourceBudget) -> Result<NativeRows, ModelError> {
        Self::candidate_rows_from(source, selected, views, CandidateNominations::Query(candidates), table, order, budget)
    }
    fn candidate_rows_from(source: SelectionSource, selected: Option<Self>, views: Vec<ContentHash>, candidates: CandidateNominations, table: &str, order: &str, budget: &ResourceBudget) -> Result<NativeRows, ModelError> {
        if !matches!(table, "entity" | "assertion" | "compiler_record") { return Err(ModelError::Schema("selected candidate table")); }
        let order = order.split(',').map(str::trim).map(str::to_owned).collect::<Vec<_>>();
        if order.iter().any(|field| !matches!(field.as_str(), "id" | "anchor" | "semantic_key" | "body.projection" | "body.window")) { return Err(ModelError::Schema("selected candidate ordering")); }
        let table = table.to_owned(); let budget = budget.clone();
        NativeRows::owned(move |sender| async move {
            let mut candidates = match candidates {
                CandidateNominations::Query(query) => source.stream(query)?,
                CandidateNominations::Relation(relation) => Self::relation_nominations(source.clone(), views.clone(), relation, budget.clone())?,
            }.with_row_bytes(MAX_ROW_BYTES);
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
                    let nodes = match &selected { Some(selected) => selected.filter_members_with_budget(&nominated, &budget)?, None => Self::sparse_membership(&source, &views, &nominated, &budget).await?.answers.into_iter().filter(|answer| answer.selected).map(|answer| answer.requested).collect() };
                    if nodes.is_empty() { continue; }
                    let vars = Variables::from_iter([("nodes".into(), nodes.clone().into_value())]);
                    let mut payloads = source.stream(crate::prepared::PreparedQuery::new(vars, vec![], vec!["SELECT * FROM $nodes".into()])?)?.with_row_bytes(MAX_ROW_BYTES);
                    let result = async {
                        let mut seen = BTreeSet::new();
                        while let Some(row) = payloads.next().await? {
                            let actual = id(&row)?;
                            if !nodes.contains(&actual) || !seen.insert(actual) { return Err(ModelError::Conflict("candidate hydration correspondence")); }
                            let key = if order.as_slice() == ["semantic_key"] {
                                let Value::Object(object) = &row else { return Err(ModelError::Schema("candidate semantic row")); };
                                let Some(Value::String(key)) = object.get("semantic_key") else { return Err(ModelError::Schema("candidate semantic key")); }; key.clone()
                            } else { order_key(&row, &order)? };
                            let mut wrapped = Object::new(); wrapped.insert("id", RecordId::new("selected_order", key)); wrapped.insert("row", row); sorted.push(Value::Object(wrapped))?;
                        }
                        if seen.len() != nodes.len() { return Err(ModelError::Schema("missing selected candidate payload")); }
                        Ok(())
                    }.await;
                    let mut terminal = Completion::default(); terminal.step("selected candidate payload drainage", payloads.drain_transport().await); complete(result, terminal)?;
                }
                let mut sorted = sorted.finish()?;
                // Validate every relevant row before exposing a limited prefix. Merge
                // conflicts can occur after an apparently valid first result.
                while sorted.next_row()?.is_some() {}
                sorted.rewind()?;
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
    async fn relation_nominations_follow_selected_membership_independently_of_foreign_payload_growth() {
        use lctx_model::domain::{Record, graph::Entity, input::Package, normalized::entities::EntityRef, value::Place};
        for alias in [false, true] {
            for admitted in [false, true] {
                let mut baseline = None;
                for foreign_count in [0, 4096] {
                    let budget = ResourceBudget::fixed(8 << 20).unwrap();
                    let view = ContentHash::of(b"relation-selected-view");
                    let relation = if alias { EntityRef::NAME } else { Package::NAME };
                    let source_relation = if alias { Place::NAME } else { Package::NAME };
                    if alias { assert!(Entity::canonical_place_endpoint_relations().contains(&(source_relation, relation))); }
                    let member = RecordId::new("entity", "chosen");
                    let endpoint = RecordId::new("entity", "endpoint");
                    let middle = RecordId::new("entity", "intermediate");
                    let requested = if alias { endpoint.clone() } else { member.clone() };
                    let mut payloads = BTreeMap::new();
                    for (node, row_relation) in [(member.clone(), source_relation), (middle.clone(), Place::NAME), (endpoint.clone(), EntityRef::NAME)] {
                        let mut row = Object::new(); row.insert("id", node.clone()); row.insert("semantic_type", row_relation); row.insert("semantic_key", format!("{:?}", node.key)); row.insert("content", "immutable"); payloads.insert(node, Value::Object(row));
                    }
                    for ordinal in 0..foreign_count {
                        let node = RecordId::new("entity", format!("foreign-{ordinal:04}"));
                        let mut row = Object::new(); row.insert("id", node.clone()); row.insert("semantic_type", relation); row.insert("semantic_key", format!("foreign-{ordinal:04}")); row.insert("content", "foreign"); payloads.insert(node, Value::Object(row));
                    }
                    let edges = if alias { vec![(member.clone(), middle.clone()), (middle.clone(), endpoint.clone()), (endpoint.clone(), member.clone())] } else { vec![] };
                    let calls = Arc::new(std::sync::Mutex::new(Vec::new())); let observed = calls.clone();
                    let hydrated = Arc::new(std::sync::Mutex::new(Vec::new())); let observed_hydrated = hydrated.clone();
                    let source = SelectionSource::Fixture(Arc::new(move |query| {
                        let (sql, vars) = query.into_request();
                        let nodes = vars.get("nodes").or_else(|| vars.get("sources")).map(|value| Vec::<RecordId>::from_value(value.clone()).unwrap()).unwrap_or_default();
                        observed.lock().unwrap().push((sql.clone(), nodes.clone()));
                        let values = if sql == "SELECT node AS id FROM compiler_view_member WITH INDEX view_key WHERE view=$view AND relation=$relation;" {
                            assert_eq!(vars.get("view"), Some(&RecordId::new("compiler_view", view.hex()).into_value()));
                            if admitted && vars.get("relation") == Some(&Value::String(source_relation.into())) { vec![pointer(member.clone())] } else { vec![] }
                        } else if sql.starts_with("SELECT node AS id,relation,semantic_key,content FROM compiler_view_member") {
                            assert_eq!(vars.get("view"), Some(&RecordId::new("compiler_view", view.hex()).into_value()));
                            let hit = if vars.get("nodes").is_some() { nodes.contains(&member) } else {
                                assert!(sql.contains("WITH INDEX view_key"));
                                assert_eq!(vars.get("relation"), Some(&Value::String(source_relation.into())));
                                Vec::<String>::from_value(vars.get("keys").unwrap().clone()).unwrap().contains(&format!("{:?}", member.key))
                            };
                            if admitted && hit { let mut row = Object::new(); row.insert("id", member.clone()); row.insert("relation", source_relation); row.insert("semantic_key", format!("{:?}", member.key)); row.insert("content", "immutable"); vec![Value::Object(row)] } else { vec![] }
                        } else if sql.starts_with("SELECT * FROM compiler_alias WITH INDEX alias_source") || sql.starts_with("SELECT source,target,id FROM compiler_alias WITH INDEX alias_target") {
                            let forward = sql.contains("WITH INDEX alias_source");
                            edges.iter().filter(|(source, target)| nodes.contains(if forward { source } else { target })).map(|(source, target)| {
            let mut row = Object::new(); row.insert("id", RecordId::new("compiler_alias", format!("{source:?}-{target:?}"))); row.insert("source", source.clone()); row.insert("target", target.clone()); Value::Object(row)
                            }).collect()
                        } else if sql == "SELECT id FROM $nodes WHERE semantic_type=$relation;" {
                            nodes.iter().filter(|node| payloads.get(*node).and_then(Value::as_object).and_then(|row| row.get("semantic_type")) == vars.get("relation")).map(|node| pointer(node.clone())).collect()
                        } else {
                            assert_eq!(sql, "SELECT * FROM $nodes;", "relation reads cannot enumerate the global payload universe");
                            observed_hydrated.lock().unwrap().extend(nodes.iter().cloned());
                            nodes.iter().map(|node| payloads[node].clone()).collect()
                        };
                        NativeRows::owned(move |sender| async move { for row in values { if sender.send(row).await.is_err() { break; } } Ok(()) })
                    }));
                    let mut rows = SelectedPayloads::candidate_rows_from(source, None, vec![view], CandidateNominations::Relation(relation.into()), "entity", "semantic_key", &budget).unwrap();
                    let mut actual = Vec::new(); while let Some(row) = rows.next().await.unwrap() { actual.push(id(&row).unwrap()); }
                    rows.drain_transport().await.unwrap(); drop(rows);
                    assert_eq!(actual, if admitted { vec![requested.clone()] } else { vec![] });
                    assert_eq!(*hydrated.lock().unwrap(), actual);
                    let work = calls.lock().unwrap().clone();
                    assert!(work.iter().flat_map(|(_, nodes)| nodes).all(|node| !format!("{:?}", node.key).contains("foreign-")));
                    if let Some(baseline) = &baseline { assert_eq!(&work, baseline, "foreign same-relation growth changes neither nominations nor exact membership/query work"); } else { baseline = Some(work); }
                    assert_eq!(budget.reserved(), 0);
                }
            }
        }
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
                } else if sql.starts_with("SELECT node AS id,relation,semantic_key,content FROM compiler_view_member WITH INDEX view_nodes") {
                    let nodes = Vec::<RecordId>::from_value(vars.get("nodes").unwrap().clone()).unwrap();
                    assert!(nodes.len() <= WINDOW); nodes.into_iter().filter(|node| node == &RecordId::new("entity", "selected-0001")).map(|node| { let mut row = Object::new(); row.insert("id", node); row.insert("relation", "packages"); row.insert("semantic_key", "key"); row.insert("content", "hash"); Value::Object(row) }).collect()
                } else if sql.starts_with("SELECT node AS id,relation,semantic_key,content FROM compiler_view_member WITH INDEX view_key") {
                    let mut row = Object::new(); row.insert("id", RecordId::new("entity", "selected-0001")); row.insert("relation", "packages"); row.insert("semantic_key", "key"); row.insert("content", "hash"); vec![Value::Object(row)]
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

    #[tokio::test]
    async fn sparse_membership_batches_shared_cyclic_ancestry_and_retains_per_node_witnesses() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let one = ContentHash::of(b"view-one"); let two = ContentHash::of(b"view-two");
        let reverse = Arc::new(AtomicUsize::new(0)); let calls = reverse.clone();
        let source = SelectionSource::Fixture(Arc::new(move |query| {
            let (sql, vars) = query.into_request();
            let values = if sql.starts_with("SELECT node AS id,relation,semantic_key,content FROM compiler_view_member") {
                let nodes = if let Some(nodes) = vars.get("nodes") { Vec::<RecordId>::from_value(nodes.clone()).unwrap() } else {
                    let keys = Vec::<String>::from_value(vars.get("keys").unwrap().clone()).unwrap();
                    ["c", "d"].into_iter().map(|key| RecordId::new("entity", key)).filter(|node| keys.contains(&format!("{:?}", node.key))).collect()
                };
                let view = RecordId::from_value(vars.get("view").unwrap().clone()).unwrap();
                nodes.into_iter().filter(|node| (node == &RecordId::new("entity", "d") && view == RecordId::new("compiler_view", one.hex())) || (node == &RecordId::new("entity", "c") && view == RecordId::new("compiler_view", two.hex()))).map(|node| {
                    let mut row = Object::new(); row.insert("semantic_key", format!("{:?}", node.key)); row.insert("id", node); row.insert("relation", "places"); row.insert("content", "immutable"); Value::Object(row)
                }).collect::<Vec<_>>()
            } else {
                assert!(sql.starts_with("SELECT source,target,id FROM compiler_alias WITH INDEX alias_target")); calls.fetch_add(1, Ordering::SeqCst);
                let nodes = Vec::<RecordId>::from_value(vars.get("sources").unwrap().clone()).unwrap();
                [("c", "a"), ("c", "b"), ("d", "c"), ("a", "d")].into_iter().filter(|(_, target)| nodes.contains(&RecordId::new("entity", *target))).map(|(source, target)| {
                    let mut row = Object::new(); row.insert("id", RecordId::new("compiler_alias", format!("{source}-{target}"))); row.insert("source", RecordId::new("entity", source)); row.insert("target", RecordId::new("entity", target)); Value::Object(row)
                }).collect()
            };
            NativeRows::owned(move |sender| async move { for row in values { if sender.send(row).await.is_err() { break; } } Ok(()) })
        }));
        let requested = ["a", "b", "missing", "a"].map(|key| RecordId::new("entity", key));
        let answers = SelectedPayloads::sparse_membership(&source, &[one, two], &requested, &budget).await.unwrap();
        assert_eq!(answers.answers.len(), 3); assert!(answers.contains_all(&requested[..2])); assert!(!answers.contains(&requested[2]));
        for answer in &answers.answers[..2] {
            assert_eq!(answer.witnesses.len(), 2);
            for witness in &answer.witnesses {
                assert_eq!(witness.ancestry.first(), Some(&answer.requested)); assert_eq!(witness.ancestry.last(), Some(&witness.member));
                assert_eq!(witness.ancestry.iter().collect::<BTreeSet<_>>().len(), witness.ancestry.len(), "cycles cannot duplicate an ancestry node");
                assert_eq!(witness.relation, "places");
            }
        }
        assert_eq!(reverse.load(Ordering::SeqCst), 3, "shared ancestor c and d are read once, independent of requested-node count");
        assert!(budget.reserved() > 0); drop(answers); assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn relevant_nominal_conflict_is_rejected_before_any_prefix_is_exposed() {
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let mut nodes = SortedRows::with_budget(&budget).unwrap();
        for key in ["a", "z"] { nodes.push(pointer(RecordId::new("entity", key))).unwrap(); }
        let nodes = nodes.finish().unwrap().into_prepared(); let membership = nodes.point_index(&budget).unwrap();
        let source = SelectionSource::Fixture(Arc::new(|query| {
            let (sql, _) = query.into_request();
            let values = if sql == "NOMINATE;" { ["a", "z"].into_iter().map(|key| pointer(RecordId::new("entity", key))).collect() } else {
                assert_eq!(sql, "SELECT * FROM $nodes;");
                ["a", "z"].into_iter().map(|key| { let mut row = Object::new(); row.insert("id", RecordId::new("entity", key)); row.insert("semantic_key", "same-nominal-id"); row.insert("content", key); Value::Object(row) }).collect::<Vec<_>>()
            };
            NativeRows::owned(move |sender| async move { for row in values { if sender.send(row).await.is_err() { break; } } Ok(()) })
        }));
        let selected = SelectedPayloads { source, nodes, membership, aliases: SortedRows::with_budget(&budget).unwrap().finish().unwrap().into_prepared(), budget: budget.clone() };
        let query = crate::prepared::PreparedQuery::new(Variables::new(), vec![], vec!["NOMINATE".into()]).unwrap();
        let mut rows = selected.candidate_rows(query, "entity", "semantic_key", &budget).unwrap();
        assert!(rows.next().await.is_err(), "a valid first payload must not hide a later conflicting nominal variant");
        rows.drain_transport().await.unwrap(); drop(rows); drop(selected); assert_eq!(budget.reserved(), 0);
    }

    #[tokio::test]
    async fn singleton_membership_rejects_un_nominated_nominal_revision_but_accepts_shared_witness() {
        let one = ContentHash::of(b"nominal-one"); let two = ContentHash::of(b"nominal-two");
        for competing in [false, true] {
            let budget = ResourceBudget::fixed(8 << 20).unwrap();
            let source = SelectionSource::Fixture(Arc::new(move |query| {
                let (sql, vars) = query.into_request();
                let values = if sql.starts_with("SELECT node AS id,relation,semantic_key,content FROM compiler_view_member") {
                    let view = RecordId::from_value(vars.get("view").unwrap().clone()).unwrap();
                    let second = view == RecordId::new("compiler_view", two.hex());
                    let key = if second && competing { "revision-two" } else { "revision-one" };
                    let node = RecordId::new("entity", key);
                    let hit = if let Some(nodes) = vars.get("nodes") {
                        Vec::<RecordId>::from_value(nodes.clone()).unwrap().contains(&node)
                    } else {
                        assert!(sql.contains("WITH INDEX view_key"));
                        assert_eq!(vars.get("relation"), Some(&Value::String("source_artifacts".into())));
                        assert_eq!(Vec::<String>::from_value(vars.get("keys").unwrap().clone()).unwrap(), vec!["same-nominal"]);
                        true
                    };
                    if hit { let mut row = Object::new(); row.insert("id", node); row.insert("relation", "source_artifacts"); row.insert("semantic_key", "same-nominal"); row.insert("content", if second && competing { "content-two" } else { "content-one" }); vec![Value::Object(row)] } else { vec![] }
                } else {
                    assert!(sql.starts_with("SELECT source,target,id FROM compiler_alias WITH INDEX alias_target")); vec![]
                };
                NativeRows::owned(move |sender| async move { for row in values { if sender.send(row).await.is_err() { break; } } Ok(()) })
            }));
            let result = SelectedPayloads::sparse_membership(&source, &[one, two], &[RecordId::new("entity", "revision-one")], &budget).await;
            if competing { assert!(result.is_err(), "singleton physical nominations cannot hide another selected revision"); }
            else { let answers = result.unwrap(); assert_eq!(answers.answers.len(), 1); assert_eq!(answers.answers[0].witnesses.len(), 2); assert!(answers.answers[0].selected); drop(answers); }
            assert_eq!(budget.reserved(), 0);
        }
    }

}
