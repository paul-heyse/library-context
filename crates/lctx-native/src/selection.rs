//! Complete indexed selection with disk-backed identity sets and bounded exact payload reads.
//! Selection is preparation; independent readback and cold admission retain their own checks.
use crate::{
    ordered_rows::{PreparedPointRows, PreparedRows, SortedRows},
    reader::NativeRows,
};
use lctx_model::domain::charged::StateCharge;
use lctx_model::domain::{
    ContentHash, ModelError,
    completion::{Completion, complete},
    resources::{MAX_ROW_BYTES, Reservation, ResourceBudget},
};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    sync::Arc,
};
use surrealdb::{
    Surreal,
    engine::remote::grpc::Client,
    types::{Object, RecordId, RecordIdKey, SurrealValue, Value, Variables},
};

const WINDOW: usize = 128;
impl crate::derived_search::SearchCanonicalRead for SelectedPayloads {
    fn search_candidates<R: lctx_model::domain::Record + serde::de::DeserializeOwned>(
        &self,
        query: crate::prepared::PreparedQuery,
    ) -> Result<crate::reader::CanonicalRecords<R>, ModelError> {
        let table = crate::schema::ScopeTable::for_relation(R::NAME)?.name();
        Ok(crate::reader::CanonicalRecords::from_physical_rows(
            self.candidate_rows(query, table, "semantic_key", &self.budget)?,
        ))
    }
}
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
        self.answers
            .binary_search_by(|answer| answer.requested.cmp(node))
            .ok()
            .is_some_and(|index| self.answers[index].selected)
    }
    pub fn contains_all(&self, nodes: &[RecordId]) -> bool {
        nodes.iter().all(|node| self.contains(node))
    }
    pub fn unrestricted(
        requested: &[RecordId],
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        let mut charge = StateCharge::new(budget, "selected-membership-answers");
        let mut answers = BTreeMap::new();
        for node in requested {
            if !answers.contains_key(node) {
                charge.grow(
                    node_bytes(node)
                        .saturating_mul(2)
                        .saturating_add(std::mem::size_of::<NodeMembership>() + 32),
                )?;
                answers.insert(
                    node.clone(),
                    NodeMembership {
                        requested: node.clone(),
                        selected: true,
                        witnesses: vec![],
                    },
                );
            }
        }
        Ok(Self {
            answers: answers.into_values().collect(),
            _charge: charge,
        })
    }
}
pub fn node_bytes(node: &RecordId) -> usize {
    std::mem::size_of::<RecordId>().saturating_add(crate::ordered_rows::record_id_bytes(node))
}

struct CanonicalSelection {
    views: Vec<ContentHash>,
    owner_views: BTreeMap<RecordId, BTreeMap<String, BTreeSet<ContentHash>>>,
    alias_membership: BTreeMap<ContentHash, PreparedPointRows>,
    _charge: StateCharge,
}
struct ExplicitOwners {
    rows: Vec<RecordId>,
    _charge: StateCharge,
}
struct OwnedAliases {
    topology: crate::alias_topology::AliasTopology,
    incidence: crate::ordered_rows::PreparedSourceRows,
}
fn seed_grant(view: ContentHash, node: &RecordId) -> Value {
    let view = RecordId::new("compiler_view", view.hex());
    let mut row = Object::new();
    row.insert(
        "id",
        RecordId::new(
            "selection_seed",
            RecordIdKey::Array(
                vec![Value::RecordId(view.clone()), Value::RecordId(node.clone())].into(),
            ),
        ),
    );
    row.insert("view", view);
    row.insert("node", node.clone());
    Value::Object(row)
}
pub struct SelectedPayloadRead<'a> {
    pub table: &'a str,
    pub predicate: &'a str,
    pub bindings: Variables,
    pub preparation: Vec<String>,
    pub order: &'a str,
    pub limit: Option<usize>,
}
#[derive(Clone)]
pub struct SelectedPayloads {
    exact: Option<Arc<CanonicalSelection>>,
    explicit_owners: Option<Arc<ExplicitOwners>>,
    derivation: Option<ContentHash>,
    source: SelectionSource,
    nodes: PreparedRows,
    membership: PreparedPointRows,
    aliases: PreparedRows,
    topology: Option<crate::alias_topology::AliasTopology>,
    budget: ResourceBudget,
}
#[cfg(test)]
type FixtureSelectionRead = dyn Fn(crate::prepared::PreparedQuery, &ResourceBudget) -> Result<NativeRows, ModelError>
    + Send
    + Sync;
#[derive(Clone)]
enum SelectionSource {
    Borrowed(
        Box<SelectionSource>,
        Arc<crate::reader::BorrowedInstalledGuard>,
    ),
    Owned(Arc<SelectionSource>, Arc<dyn SelectionRetention>),
    Published(Arc<SelectionSource>, Arc<dyn SelectionRetention>),
    #[cfg(test)]
    Fixture(Arc<FixtureSelectionRead>),
    Session(Arc<Surreal<Client>>),
    Transaction(Arc<surrealdb::method::Transaction<Client>>),
    Cancellable(Box<SelectionSource>, Arc<std::sync::atomic::AtomicBool>),
}
// Cache outputs expose the ability to retain/read an owner, not that owner's
// recursive ReaderScope -> selection-cache type graph. Concrete adapters below
// keep the original reader/publication and its allocation pool alive unchanged.
trait SelectionRetention: Send + Sync {
    fn check_live(&self) -> Result<(), ModelError>;
    fn attach_scope(&self, rows: NativeRows) -> Result<NativeRows, ModelError>;
}
struct ReaderSelectionRetention {
    reader: crate::NativeReader<()>,
    _grant: Box<dyn Reservation>,
}
impl SelectionRetention for ReaderSelectionRetention {
    fn check_live(&self) -> Result<(), ModelError> {
        self.reader.ensure_scope_live()
    }
    fn attach_scope(&self, rows: NativeRows) -> Result<NativeRows, ModelError> {
        self.reader.attach_scope(rows)
    }
}
struct PublishedSelectionRetention {
    completed: crate::completed_state::PublishedCompletedState,
    _grant: Box<dyn Reservation>,
}
impl SelectionRetention for PublishedSelectionRetention {
    fn check_live(&self) -> Result<(), ModelError> {
        self.completed.retained_reader().ensure_scope_live()
    }
    fn attach_scope(&self, rows: NativeRows) -> Result<NativeRows, ModelError> {
        self.completed.retained_reader().attach_scope(rows)
    }
}
fn selection_retention_grant<T>(
    budget: &ResourceBudget,
    owner: &'static str,
) -> Result<Box<dyn Reservation>, ModelError> {
    // Both retention payloads and their inner sources are shared by clones.
    // Admit each Arc's counters, alignment and state before either allocation.
    let bytes = std::mem::size_of::<T>()
        .checked_add(2 * std::mem::size_of::<usize>())
        .and_then(|bytes| bytes.checked_add(std::mem::align_of::<T>() - 1))
        .and_then(|bytes| bytes.checked_add(std::mem::size_of::<SelectionSource>()))
        .and_then(|bytes| bytes.checked_add(2 * std::mem::size_of::<usize>()))
        .and_then(|bytes| bytes.checked_add(std::mem::align_of::<SelectionSource>() - 1))
        .ok_or(ModelError::Schema("selected owner retention size"))?;
    budget.reserve(owner, bytes)
}

enum CandidateNominations {
    Query(crate::prepared::PreparedQuery),
    Relation(String),
}
impl SelectionSource {
    fn check_live(&self) -> Result<(), ModelError> {
        match self {
            Self::Borrowed(source, guard) => {
                guard.ensure_live()?;
                source.check_live()
            }
            Self::Owned(source, owner) | Self::Published(source, owner) => {
                owner.check_live()?;
                source.check_live()
            }
            Self::Cancellable(source, flag) => {
                crate::prepared::check_read_cancellation(Some(flag))?;
                source.check_live()
            }
            _ => Ok(()),
        }
    }
    fn stream(
        &self,
        query: crate::prepared::PreparedQuery,
        budget: &ResourceBudget,
    ) -> Result<NativeRows, ModelError> {
        match self {
            Self::Borrowed(source, guard) => {
                guard.ensure_live()?;
                source.stream(query, budget)
            }
            Self::Owned(source, owner) | Self::Published(source, owner) => {
                owner.check_live()?;
                owner.attach_scope(source.stream(query, budget)?)
            }
            #[cfg(test)]
            Self::Fixture(stream) => stream(query, budget),
            Self::Cancellable(source, cancelled) => {
                if cancelled.load(std::sync::atomic::Ordering::Acquire) {
                    return Err(ModelError::Cause(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Interrupted,
                        "selected read delivery cancelled",
                    ))));
                }
                source.stream(query, budget)
            }
            Self::Session(client) => query.stream(client, budget),
            Self::Transaction(transaction) => query.stream_transaction(transaction, budget),
        }
    }
}
fn id(row: &Value) -> Result<RecordId, ModelError> {
    let Value::Object(row) = row else {
        return Err(ModelError::Schema("selection object"));
    };
    RecordId::from_value(
        row.get("id")
            .cloned()
            .ok_or(ModelError::Schema("selection identity"))?,
    )
    .map_err(ModelError::codec)
}
fn membership_fields(row: &Value) -> Result<(&RecordId, &RecordId, &str, &str, &str), ModelError> {
    let row = row
        .as_object()
        .ok_or(ModelError::Schema("canonical selected membership object"))?;
    let (
        Some(Value::RecordId(owner)),
        Some(Value::RecordId(node)),
        Some(Value::String(relation)),
        Some(Value::String(key)),
        Some(Value::String(content)),
    ) = (
        row.get("contribution"),
        row.get("node"),
        row.get("relation"),
        row.get("semantic_key"),
        row.get("content"),
    )
    else {
        return Err(ModelError::Schema("canonical selected membership fields"));
    };
    Ok((
        owner,
        node,
        relation.as_str(),
        key.as_str(),
        content.as_str(),
    ))
}
fn pointer(node: RecordId) -> Value {
    let mut row = Object::new();
    row.insert("id", node);
    Value::Object(row)
}
// Only these schema-owned ordering fields are permitted. Hex preserves UTF-8 byte
// order; a terminator below every hex digit preserves prefix order and tuple boundaries.
fn order_string(value: &str, out: &mut String) {
    out.push_str(&hex::encode(value.as_bytes()));
    out.push('!');
}
fn order_record(value: &RecordId, out: &mut String) -> Result<(), ModelError> {
    let RecordIdKey::String(key) = &value.key else {
        return Err(ModelError::Schema("selected string record identity"));
    };
    order_string(value.table.as_str(), out);
    order_string(key, out);
    Ok(())
}
fn order_key(row: &Value, fields: &[String]) -> Result<String, ModelError> {
    let Value::Object(object) = row else {
        return Err(ModelError::Schema("selected payload row"));
    };
    let mut key = String::new();
    for field in fields {
        let value = if let Some(body_field) = field.strip_prefix("body.") {
            object
                .get("body")
                .and_then(Value::as_object)
                .and_then(|body| body.get(body_field))
        } else {
            object.get(field)
        }
        .ok_or(ModelError::Schema("selected ordering field"))?;
        match (field.as_str(), value) {
            ("id" | "anchor", Value::RecordId(value)) => order_record(value, &mut key)?,
            ("semantic_key", Value::String(value)) => order_string(value, &mut key),
            ("body.projection" | "body.window", Value::Array(values)) => {
                // Canonical Id fields lower from FixedSizeBinary(16) to native
                // integer arrays. Fixed-width byte hex preserves their tuple order.
                if values.len() != 16 {
                    return Err(ModelError::Schema("selected ordering nominal identity"));
                }
                let mut bytes = [0u8; 16];
                for (byte, value) in bytes.iter_mut().zip(values.iter()) {
                    let Value::Number(surrealdb::types::Number::Int(value)) = value else {
                        return Err(ModelError::Schema("selected ordering nominal byte"));
                    };
                    *byte = u8::try_from(*value)
                        .map_err(|_| ModelError::Schema("selected ordering nominal byte"))?;
                }
                key.push_str(&hex::encode(bytes));
                key.push('!');
            }
            _ => return Err(ModelError::Schema("selected ordering field type")),
        }
    }
    order_record(&id(row)?, &mut key)?;
    Ok(key)
}
async fn consume(
    source: &SelectionSource,
    budget: &ResourceBudget,
    sql: String,
    bindings: Variables,
    mut accept: impl FnMut(Value) -> Result<(), ModelError>,
) -> Result<(), ModelError> {
    let mut rows = source
        .stream(
            crate::prepared::PreparedQuery::new(bindings, vec![], vec![sql])?,
            budget,
        )?
        .with_row_bytes(MAX_ROW_BYTES);
    let result = async {
        while let Some(row) = rows.next().await? {
            source.check_live()?;
            accept(row)?;
        }
        source.check_live()?;
        Ok(())
    }
    .await;
    let mut terminal = Completion::default();
    terminal.step(
        "selected indexed query drainage",
        rows.drain_transport().await,
    );
    complete(result, terminal)
}
impl SelectedPayloads {
    /// Earn session preparation from the protected publication's exact inventory. The
    /// cache receives mechanical data only; its caller attaches the reader owner after
    /// the shared future resolves, avoiding a scope→cache→scope ownership cycle.
    pub(crate) async fn for_published_session(
        client: Arc<Surreal<Client>>,
        completed: &crate::completed_state::PublishedCompletedState,
        requested: &[ContentHash],
        cancelled: Option<Arc<std::sync::atomic::AtomicBool>>,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        let references = selected_references(completed.inventory(), requested, budget)?;
        let source = SelectionSource::Session(client);
        let source = if let Some(flag) = cancelled {
            SelectionSource::Cancellable(Box::new(source), flag)
        } else {
            source
        };
        Self::prepare_completed_selectors(source, &references.0, budget).await
    }
    pub(crate) fn with_reader_owner(
        mut self,
        reader: crate::NativeReader<()>,
    ) -> Result<Self, ModelError> {
        let grant = selection_retention_grant::<ReaderSelectionRetention>(
            &self.budget,
            "selected reader owner retention",
        )?;
        let owner = Arc::new(ReaderSelectionRetention {
            reader,
            _grant: grant,
        });
        self.source = SelectionSource::Owned(Arc::new(self.source), owner);
        Ok(self)
    }
    /// A snapshot independently compares the protected marker and every selected
    /// completion/mapping/hold/incarnation. The external capability keeps its publication
    /// pin alive through cancellation, selected payload delivery and actual EOF.
    pub async fn for_published_transaction(
        transaction: Arc<surrealdb::method::Transaction<Client>>,
        completed: &crate::completed_state::PublishedCompletedState,
        requested: &[ContentHash],
        cancelled: Arc<std::sync::atomic::AtomicBool>,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        completed.retained_reader().ensure_scope_live()?;
        let source = SelectionSource::Cancellable(
            Box::new(SelectionSource::Transaction(transaction)),
            cancelled,
        );
        let references = selected_references(completed.inventory(), requested, budget)?;
        check_publication_snapshot(&source, completed, budget).await?;
        let mut result = Self::prepare_completed_selectors(source, &references.0, budget).await?;
        let grant = selection_retention_grant::<PublishedSelectionRetention>(
            budget,
            "selected publication owner retention",
        )?;
        let owner = Arc::new(PublishedSelectionRetention {
            completed: completed.clone(),
            _grant: grant,
        });
        result.source = SelectionSource::Published(Arc::new(result.source), owner);
        Ok(result)
    }

    /// Mechanical preparation over exact compound selectors. These descriptors do
    /// not grant admission; callers retain the completed-state/pin authority.
    pub(crate) async fn for_completed_selectors(
        client: Arc<Surreal<Client>>,
        references: &[crate::completed_state::InstalledViewRef],
        cancelled: Option<Arc<std::sync::atomic::AtomicBool>>,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        let source = SelectionSource::Session(client);
        let source = if let Some(flag) = cancelled {
            SelectionSource::Cancellable(Box::new(source), flag)
        } else {
            source
        };
        Self::prepare_completed_selectors(source, references, budget).await
    }
    pub(crate) async fn for_installed_session(
        client: Arc<Surreal<Client>>,
        caps: &[Arc<crate::installed_view::HeldInstalledView>],
        cancelled: Option<Arc<std::sync::atomic::AtomicBool>>,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        // The receiving reader already earned its own exact contributor pin. Borrow
        // only immutable selectors here: the source capability may have since closed.
        let source = SelectionSource::Session(client);
        let source = if let Some(flag) = cancelled {
            SelectionSource::Cancellable(Box::new(source), flag)
        } else {
            source
        };
        let _selector_charge = budget.reserve(
            "selected installed descriptor references",
            caps.len().saturating_mul(std::mem::size_of::<(
                &lctx_model::domain::completed::CompletedView,
                &[crate::installed_view::ContributorRealization],
            )>()),
        )?;
        let selectors = caps
            .iter()
            .map(|cap| (&cap.completion().view, cap.completion().mappings.as_slice()))
            .collect::<Vec<_>>();
        Self::prepare_selectors(source, &selectors, budget).await
    }
    /// Internal selection retains actual source pins without a reader-scope cycle.
    pub(crate) async fn for_borrowed_installed_session(
        client: Arc<Surreal<Client>>,
        caps: &[Arc<crate::installed_view::HeldInstalledView>],
        guard: Arc<crate::reader::BorrowedInstalledGuard>,
        cancelled: Option<Arc<std::sync::atomic::AtomicBool>>,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        guard.ensure_live()?;
        let source =
            SelectionSource::Borrowed(Box::new(SelectionSource::Session(client)), guard.clone());
        let source = if let Some(flag) = cancelled {
            SelectionSource::Cancellable(Box::new(source), flag)
        } else {
            source
        };
        let _selector_charge = budget.reserve(
            "borrowed installed descriptor references",
            caps.len().saturating_mul(std::mem::size_of::<(
                &lctx_model::domain::completed::CompletedView,
                &[crate::installed_view::ContributorRealization],
            )>()),
        )?;
        let selectors = caps
            .iter()
            .map(|cap| (&cap.completion().view, cap.completion().mappings.as_slice()))
            .collect::<Vec<_>>();
        let result = Self::prepare_selectors(source, &selectors, budget).await;
        let result = match result {
            Ok(selected) => {
                guard.ensure_current(budget).await?;
                Ok(selected)
            }
            Err(error) => Err(error),
        };
        if result
            .as_ref()
            .err()
            .is_some_and(|error| !error.permits_storage_cleanup())
        {
            guard.retain_unknown();
        }
        result
    }
    async fn prepare_completed_selectors(
        source: SelectionSource,
        references: &[crate::completed_state::InstalledViewRef],
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        for reference in references {
            reference.validate()?;
        }
        let _selector_charge = budget.reserve(
            "selected retained descriptor references",
            references.len().saturating_mul(std::mem::size_of::<(
                &lctx_model::domain::completed::CompletedView,
                &[crate::installed_view::ContributorRealization],
            )>()),
        )?;
        let selectors = references
            .iter()
            .map(|reference| {
                (
                    &reference.selector.view,
                    reference.selector.mappings.as_slice(),
                )
            })
            .collect::<Vec<_>>();
        Self::prepare_selectors(source, &selectors, budget).await
    }
    async fn prepare_selectors(
        source: SelectionSource,
        selectors: &[(
            &lctx_model::domain::completed::CompletedView,
            &[crate::installed_view::ContributorRealization],
        )],
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        Self::prepare_selectors_execution(source, selectors, budget)?.await
    }
    // Session, transaction and installed-view consumers share one complete exact
    // selection preparation. Erase its native/alias execution at that common seam.
    fn prepare_selectors_execution<'a>(
        source: SelectionSource,
        selectors: &'a [(
            &'a lctx_model::domain::completed::CompletedView,
            &'a [crate::installed_view::ContributorRealization],
        )],
        budget: &'a ResourceBudget,
    ) -> Result<futures::future::BoxFuture<'a, Result<Self, ModelError>>, ModelError> {
        let body = async move {
            let mut charge = StateCharge::new(budget, "selected canonical contributor scopes");
            let mut exact = CanonicalSelection {
                views: Vec::new(),
                owner_views: BTreeMap::new(),
                alias_membership: BTreeMap::new(),
                _charge: StateCharge::new(budget, "selected canonical contributor correspondence"),
            };
            let mut owners = SortedRows::with_budget(budget)?;
            for (view, mappings) in selectors {
                view.validate()?;
                if exact.views.contains(&view.identity) {
                    return Err(ModelError::Conflict("selected logical view duplicate"));
                }
                charge.grow(std::mem::size_of::<ContentHash>())?;
                exact.views.push(view.identity);
                for logical in &view.contributions {
                    let mapping = mappings
                        .iter()
                        .find(|mapping| {
                            mapping.view == view.identity && mapping.logical == *logical
                        })
                        .ok_or(ModelError::Conflict(
                            "selected canonical contributor mapping missing",
                        ))?;
                    if mapping.incarnation <= 0 {
                        return Err(ModelError::Conflict(
                            "selected canonical contributor incarnation",
                        ));
                    }
                    let owner = RecordId::new("compiler_contribution", mapping.physical.hex());
                    exact._charge.grow(
                        node_bytes(&owner)
                            .saturating_mul(2)
                            .saturating_add(view.relation.len())
                            .saturating_add(512),
                    )?;
                    exact
                        .owner_views
                        .entry(owner)
                        .or_default()
                        .entry(view.relation.clone())
                        .or_default()
                        .insert(view.identity);
                }
                // Aliases across dependency contributors can carry selected endpoint paths.
                for mapping in *mappings {
                    owners.push(pointer(RecordId::new(
                        "compiler_contribution",
                        mapping.physical.hex(),
                    )))?;
                }
            }
            exact._charge.grow(
                exact
                    .views
                    .len()
                    .saturating_mul(std::mem::size_of::<ContentHash>()),
            )?;
            let mut seeds = SortedRows::with_budget(budget)?;
            charge.grow(
                exact
                    .owner_views
                    .keys()
                    .map(|owner| node_bytes(owner).saturating_mul(3))
                    .fold(0usize, usize::saturating_add),
            )?;
            let selected_owners = exact.owner_views.keys().cloned().collect::<Vec<_>>();
            for owners in selected_owners.chunks(WINDOW) {
                let mut vars = Variables::new();
                vars.insert("contributors", owners.to_vec());
                consume(&source, budget,"SELECT * FROM compiler_membership WITH INDEX contribution_rows WHERE contribution IN $contributors".into(),vars,|row|{
                crate::compiler_blocks::validate_locator(&row)?;
                let (owner,node,relation,_,_)=membership_fields(&row)?;
                if !owners.contains(owner){return Err(ModelError::Conflict("canonical selected foreign contributor"));}
                if let Some(views)=exact.owner_views.get(owner).and_then(|relations|relations.get(relation)) {
                    for view in views {seeds.push(seed_grant(*view,node))?;}
                }
                Ok(())
            }).await?;
            }
            let owned =
                Self::owned_alias_topology(&source, owners.finish()?.into_prepared(), budget)
                    .await?;
            let seeds = seeds
                .finish()?
                .into_prepared()
                .source_index("view", budget, || source.check_live())?;
            let mut node_runs = Vec::new();
            let mut alias_runs = Vec::new();
            charge.grow(
                selectors
                    .len()
                    .saturating_mul(2 * std::mem::size_of::<PreparedRows>()),
            )?;
            for (view, mappings) in selectors {
                let mut eligible = SortedRows::with_budget(budget)?;
                let mut closure_owners = SortedRows::with_budget(budget)?;
                for mapping in *mappings {
                    closure_owners.push(pointer(RecordId::new(
                        "compiler_contribution",
                        mapping.physical.hex(),
                    )))?;
                }
                let mut closure_owners = closure_owners.finish()?;
                while let Some(owner) = closure_owners.next_row()? {
                    let mut incidence = owned.incidence.rows_for(&id(&owner)?, budget)?;
                    while let Some(row) = incidence.next_row()? {
                        source.check_live()?;
                        let Some(Value::RecordId(alias)) =
                            row.as_object().and_then(|row| row.get("object"))
                        else {
                            return Err(ModelError::Schema("selector alias incidence target"));
                        };
                        eligible.push(pointer(alias.clone()))?;
                    }
                }
                let eligible = eligible.finish()?.into_prepared().point_index(budget)?;
                let mut view_seeds = SortedRows::with_budget(budget)?;
                let mut seed_rows =
                    seeds.rows_for(&RecordId::new("compiler_view", view.identity.hex()), budget)?;
                while let Some(row) = seed_rows.next_row()? {
                    source.check_live()?;
                    let Some(Value::RecordId(node)) =
                        row.as_object().and_then(|row| row.get("node"))
                    else {
                        return Err(ModelError::Schema("selector seed grant"));
                    };
                    view_seeds.push(pointer(node.clone()))?;
                }
                let (nodes, aliases) = owned.topology.expand_selected(
                    view_seeds.finish()?.into_prepared(),
                    &eligible,
                    budget,
                    || source.check_live(),
                )?;
                node_runs.push(nodes);
                alias_runs.push(aliases);
                exact
                    ._charge
                    .grow(std::mem::size_of::<PreparedPointRows>().saturating_add(128))?;
                exact.alias_membership.insert(view.identity, eligible);
            }
            let nodes = PreparedRows::union_prepared(&node_runs, budget, || source.check_live())?;
            let aliases =
                PreparedRows::union_prepared(&alias_runs, budget, || source.check_live())?;
            let membership = nodes.point_index(budget)?;
            Ok(Self {
                exact: Some(Arc::new(exact)),
                explicit_owners: None,
                derivation: None,
                source,
                nodes,
                membership,
                aliases,
                topology: Some(owned.topology),
                budget: budget.clone(),
            })
        };
        crate::continuation::admit(
            budget,
            "native exact selection preparation continuation",
            body,
        )
    }

    async fn owned_alias_topology(
        source: &SelectionSource,
        owners: PreparedRows,
        budget: &ResourceBudget,
    ) -> Result<OwnedAliases, ModelError> {
        let mut alias_ids = SortedRows::with_budget(budget)?;
        let mut incidence = SortedRows::with_budget(budget)?;
        let mut owners = owners.cursor_with_budget(budget)?;
        loop {
            let mut roots = Vec::new();
            let mut window = StateCharge::new(budget, "selected alias contributor window");
            while roots.len() < WINDOW {
                let Some(row) = owners.next_row()? else {
                    break;
                };
                let owner = id(&row)?;
                window.grow(node_bytes(&owner).saturating_mul(3))?;
                roots.push(owner);
            }
            if roots.is_empty() {
                break;
            }
            let mut vars = Variables::new();
            vars.insert("contributors", roots.clone());
            consume(source, budget,"SELECT * FROM native_hold WITH INDEX owner_holds WHERE owner IN $contributors AND record::table(object)='compiler_alias'".into(),vars,|row|{
                let fields=row.as_object().ok_or(ModelError::Schema("selected alias hold object"))?;
                let (Some(Value::RecordId(owner)),Some(Value::RecordId(object)))=(fields.get("owner"),fields.get("object")) else {return Err(ModelError::Schema("selected alias hold incidence"));};
                if fields.len()!=3 || !roots.contains(owner) || id(&row)?!=crate::control::hold_record_id(owner,object)? {return Err(ModelError::Conflict("selected alias hold foreign or noncanonical"));}
                if object.table.as_str()!="compiler_alias"{return Err(ModelError::Conflict("selected alias foreign target family"));}
                alias_ids.push(pointer(object.clone()))?;incidence.push(row)?;Ok(())
            }).await?;
        }
        let mut aliases = SortedRows::with_budget(budget)?;
        let mut alias_ids = alias_ids.finish()?;
        loop {
            let mut window = StateCharge::new(budget, "selected alias payload window");
            let mut ids = Vec::new();
            while ids.len() < WINDOW {
                let Some(row) = alias_ids.next_row()? else {
                    break;
                };
                let id = id(&row)?;
                window.grow(node_bytes(&id).saturating_mul(3))?;
                ids.push(id);
            }
            if ids.is_empty() {
                break;
            }
            let mut vars = Variables::new();
            vars.insert("ids", ids.clone());
            let mut seen = BTreeSet::new();
            consume(source, budget, "SELECT * FROM $ids".into(), vars, |row| {
                let id = id(&row)?;
                if !ids.contains(&id) || !seen.insert(id) {
                    return Err(ModelError::Conflict(
                        "selected alias foreign or duplicate payload",
                    ));
                }
                aliases.push(row)
            })
            .await?;
            if seen.len() != ids.len() {
                return Err(ModelError::Conflict("selected owned alias missing"));
            }
        }
        let topology = crate::alias_topology::AliasTopology::new(
            aliases.finish()?.into_prepared(),
            budget,
            || source.check_live(),
        )?;
        let incidence =
            incidence
                .finish()?
                .into_prepared()
                .source_index("owner", budget, || source.check_live())?;
        Ok(OwnedAliases {
            topology,
            incidence,
        })
    }
    #[cfg(test)]
    fn expand_owned_aliases(
        nodes: PreparedRows,
        owned: PreparedRows,
        budget: &ResourceBudget,
    ) -> Result<(PreparedRows, PreparedRows), ModelError> {
        crate::alias_topology::AliasTopology::new(owned, budget, || Ok(()))?.expand(
            nodes,
            false,
            budget,
            || Ok(()),
        )
    }
    pub fn with_derivation(mut self, derivation: ContentHash) -> Self {
        self.derivation = Some(derivation);
        self
    }
    fn bind_role_scope(&self, vars: &mut Variables) -> Result<ContentHash, ModelError> {
        let derivation = self.derivation.ok_or(ModelError::Conflict(
            "derived traversal requires exact derivation",
        ))?;
        vars.insert("lctx_derivation", derivation.hex());
        Ok(derivation)
    }
    fn check_role_scope(row: &Value, derivation: ContentHash) -> Result<(), ModelError> {
        let object = row
            .as_object()
            .ok_or(ModelError::Schema("derived traversal object"))?;
        if object.get("derivation") != Some(&Value::String(derivation.hex())) {
            return Err(ModelError::Conflict(
                "derived traversal returned foreign namespace",
            ));
        }
        Ok(())
    }
    pub fn with_read_cancellation(
        mut self,
        flag: Option<Arc<std::sync::atomic::AtomicBool>>,
    ) -> Self {
        if let Some(flag) = flag {
            self.source = SelectionSource::Cancellable(Box::new(self.source), flag);
        }
        self
    }
    /// Every root is an equality-indexed stream. No root/membership array is constructed.
    pub async fn for_views(
        client: Arc<Surreal<Client>>,
        views: &[ContentHash],
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        let _ = (client, views, budget);
        Err(ModelError::Conflict(
            "canonical selection requires protected exact installed views",
        ))
    }
    pub async fn for_cancellable_views(
        client: Arc<Surreal<Client>>,
        views: &[ContentHash],
        flag: Option<Arc<std::sync::atomic::AtomicBool>>,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        let _ = (client, views, flag, budget);
        Err(ModelError::Conflict(
            "canonical selection requires protected exact installed views",
        ))
    }
    pub async fn for_cancellable_owners(
        client: Arc<Surreal<Client>>,
        owners: &[ContentHash],
        flag: Option<Arc<std::sync::atomic::AtomicBool>>,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        let source = SelectionSource::Session(client);
        let source = if let Some(flag) = flag {
            SelectionSource::Cancellable(Box::new(source), flag)
        } else {
            source
        };
        Self::prepare(source, owners, true, budget).await
    }
    pub async fn for_transaction_owners(
        transaction: Arc<surrealdb::method::Transaction<Client>>,
        owners: &[ContentHash],
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        Self::prepare(
            SelectionSource::Transaction(transaction),
            owners,
            true,
            budget,
        )
        .await
    }
    pub async fn for_transaction_views(
        transaction: Arc<surrealdb::method::Transaction<Client>>,
        views: &[ContentHash],
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        let _ = (transaction, views, budget);
        Err(ModelError::Conflict(
            "canonical selection requires protected exact installed views",
        ))
    }
    pub async fn for_cancellable_transaction(
        transaction: Arc<surrealdb::method::Transaction<Client>>,
        roots: &[ContentHash],
        owners: bool,
        cancelled: Arc<std::sync::atomic::AtomicBool>,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        Self::prepare(
            SelectionSource::Cancellable(
                Box::new(SelectionSource::Transaction(transaction)),
                cancelled,
            ),
            roots,
            owners,
            budget,
        )
        .await
    }
    async fn prepare(
        source: SelectionSource,
        roots: &[ContentHash],
        owners: bool,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        if !owners {
            return Err(ModelError::Conflict(
                "canonical selection requires protected exact installed views",
            ));
        }
        let mut nodes = SortedRows::with_budget(budget)?;
        let mut has_entities = false;
        for roots in roots.chunks(WINDOW) {
            let mut charge = StateCharge::new(budget, "selected-root-window");
            let table = "compiler_contribution";
            charge.grow(roots.len().saturating_mul(
                std::mem::size_of::<RecordId>()
                    + 2 * std::mem::size_of::<Value>()
                    + 2 * (64 + table.len()),
            ))?;
            let mut vars = Variables::new();
            vars.insert(
                "roots",
                roots
                    .iter()
                    .map(|root| RecordId::new(table, root.hex()))
                    .collect::<Vec<_>>(),
            );
            let sql = "SELECT node FROM compiler_membership WITH INDEX contribution_rows WHERE contribution IN $roots";
            consume(&source, budget, sql.into(), vars, |value| {
                let Value::Object(row) = value else {
                    return Err(ModelError::Schema("selected membership pointer"));
                };
                let node = RecordId::from_value(
                    row.get("node")
                        .cloned()
                        .ok_or(ModelError::Schema("selected membership node"))?,
                )
                .map_err(ModelError::codec)?;
                has_entities |= node.table.as_str() == "entity";
                nodes.push(pointer(node))
            })
            .await?;
        }
        let mut owners = SortedRows::with_budget(budget)?;
        if has_entities {
            for root in roots {
                source.check_live()?;
                owners.push(pointer(RecordId::new("compiler_contribution", root.hex())))?;
            }
        }
        let owned =
            Self::owned_alias_topology(&source, owners.finish()?.into_prepared(), budget).await?;
        let (nodes, aliases) =
            owned
                .topology
                .expand(nodes.finish()?.into_prepared(), false, budget, || {
                    source.check_live()
                })?;
        let membership = nodes.point_index(budget)?;
        let mut charge = StateCharge::new(budget, "selected explicit contributor identities");
        charge.grow(std::mem::size_of::<ExplicitOwners>() + 2 * std::mem::size_of::<usize>())?;
        let mut physical = Vec::new();
        for root in roots {
            let id = RecordId::new("compiler_contribution", root.hex());
            charge.grow(node_bytes(&id).saturating_add(std::mem::size_of::<RecordId>()))?;
            physical.push(id);
        }
        physical.sort();
        physical.dedup();
        let explicit_owners = Some(Arc::new(ExplicitOwners {
            rows: physical,
            _charge: charge,
        }));
        Ok(Self {
            exact: None,
            explicit_owners,
            derivation: None,
            source,
            nodes,
            membership,
            aliases,
            topology: Some(owned.topology),
            budget: budget.clone(),
        })
    }
    /// Immutable borrowers own a fresh cursor; the shared selection never stores mutable position.
    pub fn pointers(&self) -> Result<crate::ordered_rows::OrderedRows, ModelError> {
        self.nodes.cursor_with_budget(&self.budget)
    }
    pub fn pointers_with_budget(
        &self,
        budget: &ResourceBudget,
    ) -> Result<crate::ordered_rows::OrderedRows, ModelError> {
        self.nodes.cursor_with_budget(budget)
    }
    pub fn aliases(&self) -> Result<crate::ordered_rows::OrderedRows, ModelError> {
        self.aliases.cursor_with_budget(&self.budget)
    }
    /// Reuse the immutable disk index; each probe reads logarithmically many keys.
    pub fn contains_all(&self, requested: &[RecordId]) -> Result<bool, ModelError> {
        self.contains_all_with_budget(requested, &self.budget)
    }
    pub fn contains_all_with_budget(
        &self,
        requested: &[RecordId],
        budget: &ResourceBudget,
    ) -> Result<bool, ModelError> {
        let mut cursor = self.membership.cursor(budget)?;
        for wanted in requested {
            if !cursor.contains(wanted)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
    /// Filter a finite candidate window without rescanning the selected universe.
    pub fn filter_members(&self, requested: &[RecordId]) -> Result<Vec<RecordId>, ModelError> {
        self.filter_members_with_budget(requested, &self.budget)
    }
    pub fn filter_members_with_budget(
        &self,
        requested: &[RecordId],
        budget: &ResourceBudget,
    ) -> Result<Vec<RecordId>, ModelError> {
        let mut cursor = self.membership.cursor(budget)?;
        let mut accepted = Vec::new();
        for wanted in requested {
            if cursor.contains(wanted)? {
                accepted.push(wanted.clone());
            }
        }
        accepted.sort();
        accepted.dedup();
        Ok(accepted)
    }
    async fn compiler_rows(
        &self,
        nodes: &[RecordId],
        budget: &ResourceBudget,
    ) -> Result<(Vec<Value>, StateCharge), ModelError> {
        if nodes
            .iter()
            .any(|id| id.table.as_str() != "compiler_record")
        {
            return Err(ModelError::Schema("selected compiler logical identity"));
        }
        self.source.check_live()?;
        let (owners, _owners) = if self.exact.is_some() {
            self.canonical_owners(None, budget)?
        } else {
            let actual = self.explicit_owners.as_ref().ok_or(ModelError::Conflict(
                "selected compiler contributor authority absent",
            ))?;
            let mut charge = StateCharge::new(budget, "selected compiler contributor handles");
            charge.grow(
                actual
                    .rows
                    .iter()
                    .map(|row| node_bytes(row).saturating_add(std::mem::size_of::<RecordId>()))
                    .sum(),
            )?;
            (actual.rows.clone(), charge)
        };
        let mut metadata = StateCharge::new(budget, "selected compiler locator correspondence");
        let mut locators = BTreeMap::<RecordId, Value>::new();
        for window in owners.chunks(WINDOW) {
            let mut bindings = Variables::new();
            bindings.insert("contributors", window.to_vec());
            bindings.insert("nodes", nodes.to_vec());
            let mut seen_members = BTreeSet::new();
            let mut response_charge =
                StateCharge::new(budget, "selected compiler locator response identities");
            consume(&self.source, budget,
                "SELECT * FROM compiler_membership WITH INDEX contribution_nodes WHERE contribution IN $contributors AND node IN $nodes".into(),
                bindings, |row| {
                    crate::compiler_blocks::validate_locator(&row)?;
                    let address = id(&row)?;
                    response_charge.grow(node_bytes(&address).saturating_add(128))?;
                    if !seen_members.insert(address) { return Err(ModelError::Conflict("selected compiler duplicate locator")); }
                    let (owner, node, relation, _, _) = membership_fields(&row)?;
                    if !window.contains(owner) || !nodes.contains(node) {
                        return Err(ModelError::Conflict("selected compiler locator foreign correspondence"));
                    }
                    if self.exact.is_some() { self.owner_witnesses(owner, relation)?; }
                    let size = crate::loader::native_bytes(&row).saturating_mul(8).saturating_add(512);
                    if let Some(previous) = locators.get(node) {
                        let prior = previous.as_object().ok_or(ModelError::Schema("selected compiler prior locator"))?;
                        let actual = row.as_object().ok_or(ModelError::Schema("selected compiler locator"))?;
                        if ["relation", "semantic_key", "content", "scope_keys"].iter().any(|field| prior.get(*field) != actual.get(*field)) {
                            return Err(ModelError::Conflict("selected compiler nominal variants"));
                        }
                        if id(previous)? <= id(&row)? { return Ok(()); }
                        metadata.grow(size)?;
                        metadata.release(crate::loader::native_bytes(previous).saturating_mul(8).saturating_add(512));
                    } else {
                        metadata.grow(size.saturating_add(node_bytes(node)).saturating_add(128))?;
                    }
                    locators.insert(node.clone(), row);
                    Ok(())
                }).await?;
        }
        if nodes.iter().any(|id| !locators.contains_key(id)) {
            return Err(ModelError::Conflict(
                "selected compiler logical member missing",
            ));
        }
        let mut blocks = BTreeMap::<RecordId, Vec<RecordId>>::new();
        for (node, locator) in &locators {
            let location = crate::compiler_blocks::PhysicalLocation::from_locator(locator)?;
            if location.record().table.as_str() != "compiler_block" {
                return Err(ModelError::Schema("selected nongraph physical location"));
            }
            metadata.grow(
                node_bytes(location.record())
                    .saturating_add(node_bytes(node))
                    .saturating_add(256),
            )?;
            blocks
                .entry(location.record().clone())
                .or_default()
                .push(node.clone());
        }
        let mut output_charge = StateCharge::new(budget, "selected compiler logical row delivery");
        output_charge.grow(nodes.len().saturating_mul(std::mem::size_of::<Value>()))?;
        let mut output = Vec::with_capacity(nodes.len());
        let mut seen = BTreeSet::new();
        let mut bindings = Variables::new();
        bindings.insert("blocks", blocks.keys().cloned().collect::<Vec<_>>());
        consume(
            &self.source,
            budget,
            "SELECT * FROM $blocks".into(),
            bindings,
            |block| {
                let physical = id(&block)?;
                let selected = blocks
                    .get(&physical)
                    .ok_or(ModelError::Conflict("selected compiler foreign block"))?;
                if !seen.insert(physical) {
                    return Err(ModelError::Conflict("selected compiler duplicate block"));
                }
                crate::compiler_blocks::validate_block(&block, budget)?;
                for node in selected {
                    let row = crate::compiler_blocks::selected_row(&block, &locators[node])?;
                    output_charge.grow(
                        crate::loader::native_bytes(row)
                            .saturating_mul(8)
                            .saturating_add(512),
                    )?;
                    output.push(row.clone());
                }
                Ok(())
            },
        )
        .await?;
        if seen.len() != blocks.len() || output.len() != locators.len() {
            return Err(ModelError::Conflict(
                "selected compiler physical block missing",
            ));
        }
        self.source.check_live()?;
        Ok((output, output_charge))
    }
    /// Point reads are finite; predicates run before any output limit. Complete global ordering
    /// and deduplication happen in the charged spill owner, never in a closure-wide server array.
    pub fn rows(
        &self,
        table: &str,
        predicate: &str,
        bindings: Variables,
        preparation: Vec<String>,
        order: &str,
        limit: Option<usize>,
    ) -> Result<NativeRows, ModelError> {
        self.rows_with_budget(
            SelectedPayloadRead {
                table,
                predicate,
                bindings,
                preparation,
                order,
                limit,
            },
            &self.budget,
        )
    }
    pub fn rows_with_budget(
        &self,
        demand: SelectedPayloadRead<'_>,
        budget: &ResourceBudget,
    ) -> Result<NativeRows, ModelError> {
        let SelectedPayloadRead {
            table,
            predicate,
            bindings,
            preparation,
            order,
            limit,
        } = demand;
        if !matches!(table, "entity" | "assertion" | "compiler_record") {
            return Err(ModelError::Schema("selected payload table"));
        }
        let order = order
            .split(',')
            .map(str::trim)
            .map(str::to_owned)
            .collect::<Vec<_>>();
        if order.iter().any(|field| {
            !matches!(
                field.as_str(),
                "id" | "anchor" | "semantic_key" | "body.projection" | "body.window"
            )
        }) {
            return Err(ModelError::Schema("selected payload ordering"));
        }
        let selected = self.clone();
        let table = table.to_owned();
        let predicate = predicate.to_owned();
        let budget = budget.clone();
        NativeRows::owned(budget.clone(), move |sender| async move {
            let mut sorted = SortedRows::with_budget_and_row_bytes(&budget, MAX_ROW_BYTES)?;
            let mut cursor = selected.pointers_with_budget(&budget)?;
            loop {
                if sender.is_closed() {
                    return Ok(());
                }
                let mut batch = Vec::new();
                while batch.len() < WINDOW {
                    let Some(row) = cursor.next_row()? else {
                        break;
                    };
                    let node = id(&row)?;
                    if node.table.as_str() == table {
                        batch.push(node);
                    }
                }
                if batch.is_empty() {
                    break;
                }
                let mut vars = bindings.clone();
                let _payload_charge = if table == "compiler_record" {
                    let (rows, charge) = selected.compiler_rows(&batch, &budget).await?;
                    vars.insert("nodes", rows);
                    Some(charge)
                } else {
                    vars.insert("nodes", batch);
                    None
                };
                let query = crate::prepared::PreparedQuery::new(
                    vars,
                    preparation.clone(),
                    vec![format!("SELECT * FROM $nodes WHERE ({predicate})")],
                )?;
                let mut rows = selected
                    .source
                    .stream(query, &budget)?
                    .with_row_bytes(MAX_ROW_BYTES);
                let result = async {
                    while let Some(row) = rows.next().await? {
                        let key = order_key(&row, &order)?;
                        let mut wrapped = Object::new();
                        wrapped.insert("id", RecordId::new("selected_order", key));
                        wrapped.insert("row", row);
                        sorted.push(Value::Object(wrapped))?;
                    }
                    Ok(())
                }
                .await;
                let mut completion = Completion::default();
                completion.step(
                    "selected payload batch drainage",
                    rows.drain_transport().await,
                );
                complete(result, completion)?;
            }
            let mut rows = sorted.finish()?;
            let mut emitted = 0usize;
            while let Some(row) = rows.next_row()? {
                if limit.is_some_and(|limit| emitted >= limit) {
                    break;
                }
                let Value::Object(mut row) = row else {
                    return Err(ModelError::Schema("selected ordered wrapper"));
                };
                let row = row
                    .remove("row")
                    .ok_or(ModelError::Schema("selected ordered payload"))?;
                if !sender.send(row).await? {
                    break;
                }
                emitted += 1;
            }
            Ok(())
        })
    }
    /// Sparse membership starts from nominated physical identities. Reverse aliases
    /// visit only their relevant ancestry; each ancestor window uses exact indexed view
    /// membership. No complete view or payload universe is prepared on this path.
    async fn validate_nominal_membership(
        source: &SelectionSource,
        views: &[ContentHash],
        direct: &BTreeMap<RecordId, BTreeMap<ContentHash, (String, String, String)>>,
        budget: &ResourceBudget,
        exact: Option<&Self>,
    ) -> Result<(), ModelError> {
        let mut charge = StateCharge::new(budget, "selected-nominal-membership");
        let mut expected: BTreeMap<&str, BTreeMap<&str, (&RecordId, &str)>> = BTreeMap::new();
        for (node, witnesses) in direct {
            for (relation, key, content) in witnesses.values() {
                if let Some(previous) = expected
                    .get(relation.as_str())
                    .and_then(|keys| keys.get(key.as_str()))
                {
                    if previous != &(node, content.as_str()) {
                        return Err(ModelError::Conflict("selected nominal membership variants"));
                    }
                } else {
                    charge.grow(160)?;
                    expected
                        .entry(relation)
                        .or_default()
                        .insert(key, (node, content));
                }
            }
        }
        let exact = exact.ok_or(ModelError::Conflict(
            "canonical membership requires exact selected contributors",
        ))?;
        // Probe each witnessed nominal key over the union of exact contributors.
        // Returned owners are mapped back to the selected views locally.
        for (relation, expected_keys) in &expected {
            let (owners, _owners_charge) = exact.canonical_owners(Some(relation), budget)?;
            charge.grow(
                expected_keys
                    .len()
                    .saturating_mul(std::mem::size_of::<&str>()),
            )?;
            let keys = expected_keys.keys().copied().collect::<Vec<_>>();
            for nominated in keys.chunks(WINDOW) {
                let mut window = StateCharge::new(budget, "selected nominal membership window");
                window.grow(
                    nominated
                        .iter()
                        .map(|key| {
                            key.len()
                                .saturating_mul(8)
                                .saturating_add(256usize.saturating_mul(views.len()))
                        })
                        .fold(0usize, usize::saturating_add),
                )?;
                let mut witnessed = BTreeSet::new();
                for owners in owners.chunks(WINDOW) {
                    let mut vars = Variables::new();
                    vars.insert("contributors", owners.to_vec());
                    vars.insert("relation", (*relation).to_owned());
                    vars.insert(
                        "keys",
                        nominated
                            .iter()
                            .map(|key| (*key).to_owned())
                            .collect::<Vec<_>>(),
                    );
                    let mut seen = BTreeSet::new();
                    consume(source, budget,"SELECT * FROM compiler_membership WITH INDEX contribution_rows WHERE contribution IN $contributors AND relation=$relation AND semantic_key IN $keys".into(),vars,|row|{
                        let _scratch=budget.reserve("selected nominal membership row",crate::loader::native_bytes(&row).saturating_mul(4))?;
                        crate::compiler_blocks::validate_locator(&row)?;
                        let (owner,node,actual_relation,key,content)=membership_fields(&row)?;
                        window.grow(node_bytes(owner).saturating_add(key.len()).saturating_add(256))?;
                        if !owners.contains(owner) || actual_relation!=*relation || !nominated.contains(&key) || !seen.insert((owner.clone(),key.to_owned())) {
                            return Err(ModelError::Conflict("canonical nominal membership foreign or duplicate"));
                        }
                        let Some((expected_node,expected_content))=expected_keys.get(key) else{return Err(ModelError::Conflict("canonical nominal membership unrequested key"));};
                        if node!=*expected_node || content!=*expected_content{return Err(ModelError::Conflict("selected nominal membership variants"));}
                        for view in exact.owner_witnesses(owner,actual_relation)? {
                            if views.contains(view){witnessed.insert((*view,key.to_owned()));}
                        }
                        Ok(())
                    }).await?;
                }
                for key in nominated {
                    let (node, _) = expected_keys[key];
                    if let Some(direct) = direct.get(node) {
                        for view in direct.keys() {
                            if !witnessed.contains(&(*view, (*key).to_owned())) {
                                return Err(ModelError::Conflict(
                                    "nominal membership witness disappeared",
                                ));
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
    async fn sparse_membership(
        source: &SelectionSource,
        views: &[ContentHash],
        requested: &[RecordId],
        budget: &ResourceBudget,
    ) -> Result<MembershipAnswers, ModelError> {
        Self::sparse_membership_in(source, views, requested, budget, None).await
    }
    pub async fn membership_answers(
        &self,
        requested: &[RecordId],
        budget: &ResourceBudget,
    ) -> Result<MembershipAnswers, ModelError> {
        let exact = self.exact.as_ref().ok_or(ModelError::Conflict(
            "exact contributor membership required",
        ))?;
        let mut charge = StateCharge::new(budget, "membership requested exact views");
        charge.grow(
            exact
                .views
                .len()
                .saturating_mul(std::mem::size_of::<ContentHash>()),
        )?;
        let views = exact.views.clone();
        Self::sparse_membership_in(&self.source, &views, requested, budget, Some(self)).await
    }
    fn canonical_owners(
        &self,
        relation: Option<&str>,
        budget: &ResourceBudget,
    ) -> Result<(Vec<RecordId>, StateCharge), ModelError> {
        let exact = self.exact.as_ref().ok_or(ModelError::Conflict(
            "canonical membership requires exact selected contributors",
        ))?;
        let mut charge = StateCharge::new(budget, "canonical selected contributor query window");
        let mut owners = Vec::new();
        for (owner, relations) in &exact.owner_views {
            if relation.is_none_or(|relation| relations.contains_key(relation)) {
                charge.grow(node_bytes(owner).saturating_mul(3).saturating_add(64))?;
                owners.push(owner.clone());
            }
        }
        Ok((owners, charge))
    }
    fn owner_witnesses(
        &self,
        owner: &RecordId,
        relation: &str,
    ) -> Result<&BTreeSet<ContentHash>, ModelError> {
        self.exact
            .as_ref()
            .and_then(|exact| exact.owner_views.get(owner))
            .and_then(|relations| relations.get(relation))
            .ok_or(ModelError::Conflict(
                "canonical membership outside selected contributor relation",
            ))
    }
    async fn sparse_membership_in(
        source: &SelectionSource,
        views: &[ContentHash],
        requested: &[RecordId],
        budget: &ResourceBudget,
        exact: Option<&Self>,
    ) -> Result<MembershipAnswers, ModelError> {
        if exact.is_none() {
            return Err(ModelError::Conflict(
                "canonical membership requires exact selected contributors",
            ));
        }
        let mut graph_charge = StateCharge::new(budget, "selected-membership-ancestry");
        let mut graph: BTreeMap<RecordId, BTreeMap<RecordId, BTreeSet<ContentHash>>> =
            BTreeMap::new();
        let mut direct: BTreeMap<RecordId, BTreeMap<ContentHash, (String, String, String)>> =
            BTreeMap::new();
        let scope = exact
            .and_then(|exact| exact.exact.as_ref())
            .ok_or(ModelError::Conflict("selected alias eligibility scope"))?;
        let mut eligibility = Vec::new();
        graph_charge.grow(
            scope
                .alias_membership
                .len()
                .saturating_mul(std::mem::size_of::<(
                    ContentHash,
                    crate::ordered_rows::PointRows,
                )>()),
        )?;
        for view in views {
            let aliases = scope
                .alias_membership
                .get(view)
                .ok_or(ModelError::Conflict("selector alias eligibility missing"))?;
            eligibility.push((*view, aliases.cursor(budget)?));
        }
        let mut requested_nodes = BTreeSet::new();
        let mut frontier = VecDeque::new();
        for node in requested {
            if !graph.contains_key(node) {
                graph_charge.grow(node_bytes(node).saturating_mul(4).saturating_add(192))?;
                graph.insert(node.clone(), BTreeMap::new());
                frontier.push_back(node.clone());
                requested_nodes.insert(node.clone());
            }
        }
        // Each physical ancestor is read once for the whole requested window. Queries
        // stay finite even when several requests share an ancestry or form a cycle.
        while !frontier.is_empty() && !views.is_empty() {
            let mut window = StateCharge::new(budget, "selected-membership-window");
            let mut nodes = Vec::new();
            while nodes.len() < WINDOW {
                let Some(node) = frontier.pop_front() else {
                    break;
                };
                window.grow(node_bytes(&node).saturating_mul(3))?;
                nodes.push(node);
            }
            let selected = exact.ok_or(ModelError::Conflict(
                "canonical selected contributor scope missing",
            ))?;
            let (owners, _owners_charge) = selected.canonical_owners(None, budget)?;
            for owners in owners.chunks(WINDOW) {
                let mut vars = Variables::new();
                vars.insert("contributors", owners.to_vec());
                vars.insert("nodes", nodes.clone());
                let mut seen = BTreeSet::new();
                consume(source, budget,"SELECT * FROM compiler_membership WITH INDEX contribution_nodes WHERE contribution IN $contributors AND node IN $nodes".into(),vars,|row| {
                    let _scratch=budget.reserve("selected membership row",crate::loader::native_bytes(&row).saturating_mul(4))?;
                    crate::compiler_blocks::validate_locator(&row)?;
                    let (owner,node,relation,key,content)=membership_fields(&row)?;
                    window.grow(node_bytes(&id(&row)?).saturating_add(256))?;
                    if !owners.contains(owner) || !nodes.contains(node) || !seen.insert(id(&row)?) {return Err(ModelError::Conflict("canonical sparse membership foreign or duplicate"));}
                    let Some(witnesses)=selected.exact.as_ref().and_then(|scope|scope.owner_views.get(owner)).and_then(|relations|relations.get(relation)) else {return Ok(());};
                    let nominal=(relation.to_owned(),key.to_owned(),content.to_owned());
                    if direct.get(node).is_some_and(|members|members.values().any(|previous|previous!=&nominal)){return Err(ModelError::Conflict("membership physical/nominal witness changed"));}
                    for view in witnesses.iter().filter(|view|views.contains(view)) {
                        if let Some(previous)=direct.get(node).and_then(|members|members.get(view)) {
                            if previous!=&nominal{return Err(ModelError::Conflict("membership physical/nominal witness changed"));}
                        } else {
                            graph_charge.grow(node_bytes(node).saturating_add(256).saturating_add(nominal.0.len()).saturating_add(nominal.1.len()).saturating_add(nominal.2.len()))?;
                            direct.entry(node.clone()).or_default().insert(*view,nominal.clone());
                        }
                    }
                    Ok(())
                }).await?;
            }
            nodes.retain(|node| node.table.as_str() == "entity");
            if nodes.is_empty() {
                continue;
            }
            let mut accept = |row: Value| -> Result<(), ModelError> {
                let _scratch = budget.reserve(
                    "selected-membership-alias-row",
                    crate::loader::native_bytes(&row).saturating_mul(4),
                )?;
                let alias = id(&row)?;
                let mut allowed = BTreeSet::new();
                for (view, eligible) in &mut eligibility {
                    if eligible.contains(&alias)? {
                        graph_charge.grow(96)?;
                        allowed.insert(*view);
                    }
                }
                if allowed.is_empty() {
                    return Ok(());
                }
                let Value::Object(object) = row else {
                    return Err(ModelError::Schema("sparse alias object"));
                };
                let ancestor = RecordId::from_value(
                    object
                        .get("source")
                        .cloned()
                        .ok_or(ModelError::Schema("sparse alias source"))?,
                )
                .map_err(ModelError::codec)?;
                let target = RecordId::from_value(
                    object
                        .get("target")
                        .cloned()
                        .ok_or(ModelError::Schema("sparse alias target"))?,
                )
                .map_err(ModelError::codec)?;
                if ancestor.table.as_str() != "entity" || !nodes.contains(&target) {
                    return Err(ModelError::Conflict("sparse alias ancestry"));
                }
                if !graph.contains_key(&ancestor) {
                    graph_charge
                        .grow(node_bytes(&ancestor).saturating_mul(3).saturating_add(160))?;
                    graph.insert(ancestor.clone(), BTreeMap::new());
                    frontier.push_back(ancestor.clone());
                }
                let parents = graph
                    .get_mut(&target)
                    .ok_or(ModelError::Conflict("sparse alias frontier"))?;
                if !parents.contains_key(&ancestor) {
                    graph_charge.grow(node_bytes(&ancestor).saturating_add(128))?;
                }
                parents.entry(ancestor).or_default().extend(allowed);
                Ok(())
            };
            if let Some(exact) = exact {
                let topology = exact
                    .topology
                    .as_ref()
                    .ok_or(ModelError::Conflict("exact alias topology missing"))?;
                for target in &nodes {
                    source.check_live()?;
                    let mut aliases = topology.incoming(target, budget)?;
                    let mut selected_sources = exact.membership.cursor(budget)?;
                    while let Some(row) = aliases.next_row()? {
                        source.check_live()?;
                        let Some(Value::RecordId(origin)) =
                            row.as_object().and_then(|row| row.get("source"))
                        else {
                            return Err(ModelError::Schema("prepared reverse alias source"));
                        };
                        if selected_sources.contains(origin)? {
                            accept(row)?;
                        }
                    }
                }
            }
        }
        let mut qualified: BTreeMap<RecordId, BTreeSet<ContentHash>> = BTreeMap::new();
        let mut charge = StateCharge::new(budget, "selected-membership-answers");
        let mut answers = Vec::new();
        for node in &requested_nodes {
            charge.grow(node_bytes(node).saturating_add(std::mem::size_of::<NodeMembership>()))?;
            let mut witnesses = Vec::new();
            let mut scratch = StateCharge::new(budget, "selected-membership-paths");
            let mut paths = BTreeMap::new();
            let mut pending = VecDeque::new();
            for view in views {
                scratch.grow(node_bytes(node).saturating_mul(3).saturating_add(160))?;
                paths.insert((*view, node.clone()), vec![node.clone()]);
                pending.push_back((*view, node.clone()));
            }
            while let Some((view, member)) = pending.pop_front() {
                let path = &paths[&(view, member.clone())];
                if let Some(nominal) = direct.get(&member).and_then(|members| members.get(&view)) {
                    if !qualified
                        .get(&member)
                        .is_some_and(|views| views.contains(&view))
                    {
                        graph_charge
                            .grow(node_bytes(&member).saturating_mul(2).saturating_add(128))?;
                        qualified.entry(member.clone()).or_default().insert(view);
                    }
                    charge.grow(
                        nominal
                            .0
                            .len()
                            .saturating_add(nominal.1.len())
                            .saturating_add(nominal.2.len())
                            .saturating_add(std::mem::size_of::<MembershipWitness>())
                            .saturating_add(node_bytes(&member))
                            .saturating_add(
                                path.iter()
                                    .map(node_bytes)
                                    .fold(0usize, usize::saturating_add),
                            ),
                    )?;
                    witnesses.push(MembershipWitness {
                        view,
                        member: member.clone(),
                        relation: nominal.0.clone(),
                        semantic_key: nominal.1.clone(),
                        content: nominal.2.clone(),
                        ancestry: path.clone(),
                    });
                }
                if let Some(parents) = graph.get(&member) {
                    for (parent, allowed) in parents {
                        if !allowed.contains(&view) || paths.contains_key(&(view, parent.clone())) {
                            continue;
                        }
                        let path = &paths[&(view, member.clone())];
                        scratch.grow(
                            node_bytes(parent)
                                .saturating_mul(3)
                                .saturating_add(160)
                                .saturating_add(
                                    path.iter()
                                        .map(node_bytes)
                                        .fold(0usize, usize::saturating_add),
                                ),
                        )?;
                        let mut path = path.clone();
                        path.push(parent.clone());
                        paths.insert((view, parent.clone()), path);
                        pending.push_back((view, parent.clone()));
                    }
                }
            }
            answers.push(NodeMembership {
                requested: node.clone(),
                selected: !witnesses.is_empty(),
                witnesses,
            });
        }
        direct.retain(|member, witnesses| {
            witnesses.retain(|view, _| {
                qualified
                    .get(member)
                    .is_some_and(|views| views.contains(view))
            });
            !witnesses.is_empty()
        });
        Self::validate_nominal_membership(source, views, &direct, budget, exact).await?;
        Ok(MembershipAnswers {
            answers,
            _charge: charge,
        })
    }
    pub async fn view_membership(
        client: Arc<Surreal<Client>>,
        views: &[ContentHash],
        cancelled: Option<Arc<std::sync::atomic::AtomicBool>>,
        requested: &[RecordId],
        budget: &ResourceBudget,
    ) -> Result<MembershipAnswers, ModelError> {
        let source = SelectionSource::Session(client);
        let source = if let Some(flag) = cancelled {
            SelectionSource::Cancellable(Box::new(source), flag)
        } else {
            source
        };
        Self::sparse_membership(&source, views, requested, budget).await
    }
    pub fn sparse_candidate_rows(
        client: Arc<Surreal<Client>>,
        views: Vec<ContentHash>,
        cancelled: Option<Arc<std::sync::atomic::AtomicBool>>,
        candidates: crate::prepared::PreparedQuery,
        table: &str,
        order: &str,
        budget: &ResourceBudget,
    ) -> Result<NativeRows, ModelError> {
        let source = SelectionSource::Session(client);
        let source = if let Some(flag) = cancelled {
            SelectionSource::Cancellable(Box::new(source), flag)
        } else {
            source
        };
        Self::candidate_rows_inner(source, None, views, candidates, table, order, budget)
    }
    pub fn sparse_relation_rows(
        client: Arc<Surreal<Client>>,
        views: Vec<ContentHash>,
        cancelled: Option<Arc<std::sync::atomic::AtomicBool>>,
        relation: &str,
        table: &str,
        budget: &ResourceBudget,
    ) -> Result<NativeRows, ModelError> {
        let source = SelectionSource::Session(client);
        let source = if let Some(flag) = cancelled {
            SelectionSource::Cancellable(Box::new(source), flag)
        } else {
            source
        };
        Self::candidate_rows_from(
            source,
            None,
            views,
            CandidateNominations::Relation(relation.to_owned()),
            table,
            "semantic_key",
            budget,
        )
    }
    /// Nominate from exact selected relation membership, not the global payload
    /// family. Only model-governed endpoint sources can widen this input scope.
    fn relation_nominations(
        source: SelectionSource,
        views: Vec<ContentHash>,
        relation: String,
        budget: ResourceBudget,
        exact: Option<Self>,
    ) -> Result<NativeRows, ModelError> {
        let exact = exact.ok_or(ModelError::Conflict(
            "canonical relation requires exact selected contributors",
        ))?;
        NativeRows::owned(budget.clone(), move |sender| async move {
            let mut nominated = SortedRows::with_budget(&budget)?;
            let mut seed_grants = SortedRows::with_budget(&budget)?;
            let source_relations =
                lctx_model::domain::graph::Entity::canonical_place_endpoint_relations()
                    .iter()
                    .filter_map(|(source, target)| (*target == relation).then_some(*source))
                    .collect::<BTreeSet<_>>();
            for selected_relation in
                std::iter::once(relation.as_str()).chain(source_relations.iter().copied())
            {
                let (owners, _owners_charge) =
                    exact.canonical_owners(Some(selected_relation), &budget)?;
                for owners in owners.chunks(WINDOW) {
                    if sender.is_closed() {
                        return Ok(());
                    }
                    let mut vars = Variables::new();
                    vars.insert("contributors", owners.to_vec());
                    vars.insert("relation", selected_relation.to_owned());
                    consume(&source, &budget,"SELECT * FROM compiler_membership WITH INDEX contribution_rows WHERE contribution IN $contributors AND relation=$relation".into(),vars,|row|{
                        crate::compiler_blocks::validate_locator(&row)?;
                        let (owner,node,actual_relation,_,_)=membership_fields(&row)?;
                        if !owners.contains(owner) || actual_relation!=selected_relation {return Err(ModelError::Conflict("canonical relation foreign membership"));}
                        if !exact.owner_witnesses(owner,actual_relation)?.iter().any(|view|views.contains(view)){return Ok(());}
                        if selected_relation==relation {nominated.push(pointer(node.clone()))}
                        else if node.table.as_str()!="entity" {Err(ModelError::Conflict("canonical endpoint source family"))}
                        else {for view in exact.owner_witnesses(owner,actual_relation)?.iter().filter(|view|views.contains(view)) {seed_grants.push(seed_grant(*view,node))?;}Ok(())}
                    }).await?;
                }
            }
            if !source_relations.is_empty() && !sender.is_closed() {
                // Reuse the cycle-safe disk-backed forward traversal on just these
                // selected source relations. No complete view is prepared here.
                let seeds =
                    seed_grants
                        .finish()?
                        .into_prepared()
                        .source_index("view", &budget, || source.check_live())?;
                let topology = exact
                    .topology
                    .as_ref()
                    .ok_or(ModelError::Conflict("exact alias topology missing"))?;
                let scope = exact
                    .exact
                    .as_ref()
                    .ok_or(ModelError::Conflict("exact relation selector scope"))?;
                let mut run_charge = StateCharge::new(&budget, "relation selector closure runs");
                run_charge.grow(
                    views
                        .len()
                        .saturating_mul(std::mem::size_of::<PreparedRows>()),
                )?;
                let mut expanded_runs = Vec::new();
                for view in &views {
                    let eligible = scope
                        .alias_membership
                        .get(view)
                        .ok_or(ModelError::Conflict("relation selector alias scope"))?;
                    let mut view_seeds = SortedRows::with_budget(&budget)?;
                    let mut rows =
                        seeds.rows_for(&RecordId::new("compiler_view", view.hex()), &budget)?;
                    while let Some(row) = rows.next_row()? {
                        source.check_live()?;
                        let Some(Value::RecordId(node)) =
                            row.as_object().and_then(|row| row.get("node"))
                        else {
                            return Err(ModelError::Schema("relation selector seed"));
                        };
                        view_seeds.push(pointer(node.clone()))?;
                    }
                    let (expanded, _) = topology.expand_selected(
                        view_seeds.finish()?.into_prepared(),
                        eligible,
                        &budget,
                        || source.check_live(),
                    )?;
                    expanded_runs.push(expanded);
                }
                let expanded =
                    PreparedRows::union_prepared(&expanded_runs, &budget, || source.check_live())?;
                let mut cursor = expanded.cursor_with_budget(&budget)?;
                loop {
                    if sender.is_closed() {
                        return Ok(());
                    }
                    let mut window = StateCharge::new(&budget, "relation-alias-window");
                    let mut nodes = Vec::new();
                    while nodes.len() < WINDOW {
                        let Some(row) = cursor.next_row()? else {
                            break;
                        };
                        let node = id(&row)?;
                        window.grow(node_bytes(&node).saturating_mul(3))?;
                        nodes.push(node);
                    }
                    if nodes.is_empty() {
                        break;
                    }
                    let mut vars = Variables::new();
                    vars.insert("nodes", nodes.clone());
                    vars.insert("relation", relation.clone());
                    // Exact reached IDs retain intermediates for chain traversal;
                    // only endpoints of the requested relation become candidates.
                    consume(
                        &source,
                        &budget,
                        "SELECT id FROM $nodes WHERE semantic_type=$relation".into(),
                        vars,
                        |row| {
                            let node = id(&row)?;
                            if !nodes.contains(&node) {
                                return Err(ModelError::Conflict(
                                    "canonical endpoint nomination correspondence",
                                ));
                            }
                            nominated.push(pointer(node))
                        },
                    )
                    .await?;
                }
            }
            let mut nominated = nominated.finish()?;
            while let Some(row) = nominated.next_row()? {
                if !sender.send(row).await? {
                    break;
                }
            }
            Ok(())
        })
    }
    /// Compact indexed nominations are intersected with the exact selected identity set
    /// before any canonical/body payload is fetched. Sorting owns request scratch only.
    pub fn relation_candidates(
        &self,
        relation: &str,
        table: &str,
        budget: &ResourceBudget,
    ) -> Result<NativeRows, ModelError> {
        let exact = self
            .exact
            .as_ref()
            .ok_or(ModelError::Conflict("exact contributor relation required"))?;
        let mut charge = StateCharge::new(budget, "relation exact view identifiers");
        charge.grow(
            exact
                .views
                .len()
                .saturating_mul(std::mem::size_of::<ContentHash>()),
        )?;
        // The driver may be refused before invocation. Capturing this owner as
        // a whole keeps the cloned vector before its charge in every drop path.
        struct RelationViews {
            views: Vec<ContentHash>,
            charge: StateCharge,
        }
        impl RelationViews {
            fn into_parts(self) -> (Vec<ContentHash>, StateCharge) {
                (self.views, self.charge)
            }
        }
        let input = RelationViews {
            views: exact.views.clone(),
            charge,
        };
        // The future owns this vector and its admission until all nominations drain.
        let selected = self.clone();
        let relation = relation.to_owned();
        let table = table.to_owned();
        let budget = budget.clone();
        NativeRows::owned(budget.clone(), move |sender| {
            let (views, charge) = input.into_parts();
            crate::continuation::retain(
                async move {
                    let mut rows = Self::candidate_rows_from(
                        selected.source.clone(),
                        Some(selected),
                        views,
                        CandidateNominations::Relation(relation),
                        &table,
                        "semantic_key",
                        &budget,
                    )?;
                    let result = async {
                        while let Some(row) = rows.next().await? {
                            if !sender.send(row).await? {
                                rows.cancel_delivery();
                                break;
                            }
                        }
                        Ok(())
                    }
                    .await;
                    let mut completion = Completion::default();
                    completion.step(
                        "exact relation candidates drainage",
                        rows.drain_transport().await,
                    );
                    complete(result, completion)
                },
                charge,
            )
        })
    }
    pub fn candidate_rows(
        &self,
        candidates: crate::prepared::PreparedQuery,
        table: &str,
        order: &str,
        budget: &ResourceBudget,
    ) -> Result<NativeRows, ModelError> {
        Self::candidate_rows_inner(
            self.source.clone(),
            Some(self.clone()),
            vec![],
            candidates,
            table,
            order,
            budget,
        )
    }
    fn candidate_rows_inner(
        source: SelectionSource,
        selected: Option<Self>,
        views: Vec<ContentHash>,
        candidates: crate::prepared::PreparedQuery,
        table: &str,
        order: &str,
        budget: &ResourceBudget,
    ) -> Result<NativeRows, ModelError> {
        Self::candidate_rows_from(
            source,
            selected,
            views,
            CandidateNominations::Query(candidates),
            table,
            order,
            budget,
        )
    }
    fn candidate_rows_from(
        source: SelectionSource,
        selected: Option<Self>,
        views: Vec<ContentHash>,
        candidates: CandidateNominations,
        table: &str,
        order: &str,
        budget: &ResourceBudget,
    ) -> Result<NativeRows, ModelError> {
        if selected.is_none() {
            return Err(ModelError::Conflict(
                "canonical candidates require prepared exact selection",
            ));
        }
        if !matches!(table, "entity" | "assertion" | "compiler_record") {
            return Err(ModelError::Schema("selected candidate table"));
        }
        let order = order
            .split(',')
            .map(str::trim)
            .map(str::to_owned)
            .collect::<Vec<_>>();
        if order.iter().any(|field| {
            !matches!(
                field.as_str(),
                "id" | "anchor" | "semantic_key" | "body.projection" | "body.window"
            )
        }) {
            return Err(ModelError::Schema("selected candidate ordering"));
        }
        let table = table.to_owned();
        let budget = budget.clone();
        NativeRows::owned(budget.clone(), move |sender| async move {
            let mut candidates = match candidates {
                CandidateNominations::Query(query) => source.stream(query, &budget)?,
                CandidateNominations::Relation(relation) => Self::relation_nominations(
                    source.clone(),
                    views.clone(),
                    relation,
                    budget.clone(),
                    selected.clone(),
                )?,
            }
            .with_row_bytes(MAX_ROW_BYTES);
            let result = async {
                let mut sorted = SortedRows::with_budget_and_row_bytes(&budget, MAX_ROW_BYTES)?;
                loop {
                    if sender.is_closed() {
                        candidates.cancel_delivery();
                        break;
                    }
                    let mut window = lctx_model::domain::charged::StateCharge::new(
                        &budget,
                        "selected-candidate-window",
                    );
                    let mut nominated = Vec::with_capacity(WINDOW);
                    let mut examined = 0usize;
                    while examined < WINDOW {
                        let Some(row) = candidates.next().await? else {
                            break;
                        };
                        examined += 1;
                        window.grow(
                            crate::loader::native_bytes(&row)
                                .saturating_add(std::mem::size_of::<RecordId>()),
                        )?;
                        let node = id(&row)?;
                        if node.table.as_str() == table {
                            nominated.push(node);
                        }
                    }
                    if examined == 0 {
                        break;
                    }
                    if nominated.is_empty() {
                        continue;
                    }
                    let nodes = match &selected {
                        Some(selected) => {
                            if selected.exact.is_some() {
                                selected
                                    .membership_answers(&nominated, &budget)
                                    .await?
                                    .answers
                                    .into_iter()
                                    .filter(|answer| answer.selected)
                                    .map(|answer| answer.requested)
                                    .collect()
                            } else {
                                selected.filter_members_with_budget(&nominated, &budget)?
                            }
                        }
                        None => Self::sparse_membership(&source, &views, &nominated, &budget)
                            .await?
                            .answers
                            .into_iter()
                            .filter(|answer| answer.selected)
                            .map(|answer| answer.requested)
                            .collect(),
                    };
                    if nodes.is_empty() {
                        continue;
                    }
                    let mut vars = Variables::new();
                    let _payload_charge = if table == "compiler_record" {
                        let authority = selected
                            .as_ref()
                            .ok_or(ModelError::Conflict("selected compiler source absent"))?;
                        let (rows, charge) = authority.compiler_rows(&nodes, &budget).await?;
                        vars.insert("nodes", rows);
                        Some(charge)
                    } else {
                        vars.insert("nodes", nodes.clone());
                        None
                    };
                    let mut payloads = source
                        .stream(
                            crate::prepared::PreparedQuery::new(
                                vars,
                                vec![],
                                vec!["SELECT * FROM $nodes".into()],
                            )?,
                            &budget,
                        )?
                        .with_row_bytes(MAX_ROW_BYTES);
                    let result = async {
                        let mut seen = BTreeSet::new();
                        while let Some(row) = payloads.next().await? {
                            let actual = id(&row)?;
                            if !nodes.contains(&actual) || !seen.insert(actual) {
                                return Err(ModelError::Conflict(
                                    "candidate hydration correspondence",
                                ));
                            }
                            let key = if order.as_slice() == ["semantic_key"] {
                                let Value::Object(object) = &row else {
                                    return Err(ModelError::Schema("candidate semantic row"));
                                };
                                let Some(Value::String(key)) = object.get("semantic_key") else {
                                    return Err(ModelError::Schema("candidate semantic key"));
                                };
                                key.clone()
                            } else {
                                order_key(&row, &order)?
                            };
                            let mut wrapped = Object::new();
                            wrapped.insert("id", RecordId::new("selected_order", key));
                            wrapped.insert("row", row);
                            sorted.push(Value::Object(wrapped))?;
                        }
                        if seen.len() != nodes.len() {
                            return Err(ModelError::Schema("missing selected candidate payload"));
                        }
                        Ok(())
                    }
                    .await;
                    let mut terminal = Completion::default();
                    terminal.step(
                        "selected candidate payload drainage",
                        payloads.drain_transport().await,
                    );
                    complete(result, terminal)?;
                }
                let mut sorted = sorted.finish()?;
                // Validate every relevant row before exposing a limited prefix. Merge
                // conflicts can occur after an apparently valid first result.
                while sorted.next_row()?.is_some() {}
                sorted.rewind()?;
                while let Some(row) = sorted.next_row()? {
                    let Value::Object(mut row) = row else {
                        return Err(ModelError::Schema("selected candidate wrapper"));
                    };
                    if sender
                        .send(
                            row.remove("row")
                                .ok_or(ModelError::Schema("selected candidate payload"))?,
                        )
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                Ok(())
            }
            .await;
            let mut terminal = Completion::default();
            terminal.step(
                "selected candidate nomination drainage",
                candidates.drain_transport().await,
            );
            complete(result, terminal)
        })
    }
    /// Raw source roles plus their actual external endpoints. These are integrity claims,
    /// never nominations constrained by canonical expected roles.
    pub fn recovery_roles(&self) -> Result<NativeRows, ModelError> {
        self.recovery_derived(true)
    }
    /// Exact selected occurrence lineages plus actual referenced documents/vectors.
    /// Unexpected eligible rows remain claims for independent cold comparison.
    pub fn recovery_search(&self) -> Result<NativeRows, ModelError> {
        self.recovery_derived(false)
    }
    fn recovery_derived(&self, roles: bool) -> Result<NativeRows, ModelError> {
        let selected = self.clone();
        NativeRows::owned(selected.budget.clone(), move |sender| async move {
            if roles {
                selected.bind_role_scope(&mut Variables::new())?;
            }
            let mut actual = SortedRows::with_budget(&selected.budget)?;
            let mut endpoints = SortedRows::with_budget(&selected.budget)?;
            for table in crate::recovery::recovery_tables(if roles {
                lctx_model::domain::recovery_closure::RecoveryFamily::GraphRoles
            } else {
                lctx_model::domain::recovery_closure::RecoveryFamily::SearchOccurrences
            }) {
                let mut cursor = selected.pointers()?;
                loop {
                    if sender.is_closed() {
                        return Ok(());
                    }
                    let mut search_window = (!roles).then(|| {
                        StateCharge::new(&selected.budget, "recovery-search-pointer-window")
                    });
                    let mut nodes = Vec::new();
                    while nodes.len() < WINDOW {
                        let Some(row) = cursor.next_row()? else {
                            break;
                        };
                        let node = id(&row)?;
                        if let Some(charge) = &mut search_window {
                            charge.grow(node_bytes(&node).saturating_mul(2))?;
                        }
                        nodes.push(node);
                    }
                    if nodes.is_empty() {
                        break;
                    }
                    let mut vars = Variables::new();
                    let derivation = if roles {
                        Some(selected.bind_role_scope(&mut vars)?)
                    } else {
                        None
                    };
                    let namespace = if roles {
                        " AND derivation = $lctx_derivation"
                    } else {
                        ""
                    };
                    vars.insert("nodes", nodes);
                    let (index, field) = if roles {
                        ("outgoing", "in")
                    } else {
                        ("exact_unit_payload", "unit_payload")
                    };
                    let mut membership = if roles {
                        None
                    } else {
                        Some(selected.membership.cursor(&selected.budget)?)
                    };
                    let mut search_claims = if roles {
                        None
                    } else {
                        Some(SortedRows::with_budget(&selected.budget)?)
                    };
                    consume(
                        &selected.source, &selected.budget,
                        format!("SELECT * FROM {table} WITH INDEX {index} WHERE {field} IN $nodes{namespace}"),
                        vars,
                        |row| {
                            if let Some(derivation)=derivation {Self::check_role_scope(&row,derivation)?;}
                            let Value::Object(object) = &row else {
                                return Err(ModelError::Schema("recovery derived object"));
                            };
                            if let Some(membership) = &mut membership {
                                let _scratch = selected.budget.reserve(
                                    "recovery search dependency scratch",
                                    crate::loader::native_bytes(&row).saturating_mul(8),
                                )?;
                                let dependencies =
                                    crate::derived_search::nomination_dependencies(object)?;
                                if dependencies.is_empty() {
                                    return Ok(());
                                }
                                for dependency in dependencies {
                                    if !membership.contains(&dependency)? {
                                        return Ok(());
                                    }
                                }
                            }
                            if let Some(claims) = &mut search_claims {
                                return claims.push(row);
                            }
                            let endpoint = RecordId::from_value(
                                object
                                    .get(if roles { "out" } else { "in" })
                                    .cloned()
                                    .ok_or(ModelError::Schema("recovery derived endpoint"))?,
                            )
                            .map_err(ModelError::codec)?;
                            if !roles || endpoint.table.as_str() == "external" {
                                endpoints.push(pointer(endpoint))?;
                            }
                            actual.push(row)
                        },
                    )
                    .await?;
                    if let Some(claims) = search_claims {
                        let mut claims = claims.finish()?;
                        loop {
                            if sender.is_closed() {
                                return Ok(());
                            }
                            let mut charge = StateCharge::new(
                                &selected.budget,
                                "recovery-search-context-window",
                            );
                            let mut window = Vec::new();
                            while window.len() < WINDOW {
                                let Some(row) = claims.next_row()? else {
                                    break;
                                };
                                charge.grow(crate::loader::native_bytes(&row).saturating_mul(4))?;
                                window.push(row);
                            }
                            if window.is_empty() {
                                break;
                            }
                            for row in crate::derived_search::eligible_context_rows(
                                &selected,
                                window,
                                &selected.budget,
                            )
                            .await?
                            {
                                let object = row
                                    .as_object()
                                    .ok_or(ModelError::Schema("recovery search context object"))?;
                                let endpoint = RecordId::from_value(
                                    object
                                        .get("in")
                                        .cloned()
                                        .ok_or(ModelError::Schema("recovery derived endpoint"))?,
                                )
                                .map_err(ModelError::codec)?;
                                endpoints.push(pointer(endpoint))?;
                                actual.push(row)?;
                            }
                        }
                    }
                }
            }
            let mut endpoints = endpoints.finish()?;
            loop {
                if sender.is_closed() {
                    return Ok(());
                }
                let mut ids = Vec::new();
                while ids.len() < WINDOW {
                    let Some(row) = endpoints.next_row()? else {
                        break;
                    };
                    ids.push(id(&row)?);
                }
                if ids.is_empty() {
                    break;
                }
                let mut vars = Variables::new();
                vars.insert("ids", ids);
                consume(
                    &selected.source,
                    &selected.budget,
                    "SELECT * FROM $ids".into(),
                    vars,
                    |row| actual.push(row),
                )
                .await?;
            }
            let mut actual = actual.finish()?;
            while let Some(row) = actual.next_row()? {
                if !sender.send(row).await? {
                    break;
                }
            }
            Ok(())
        })
    }
    /// Enumerate every actual outgoing edge, including unexpected roles/fields, from exact
    /// selected source windows. Expected rows never constrain this integrity source.
    pub fn outgoing(&self, table: &str) -> Result<NativeRows, ModelError> {
        if !matches!(table, "participant" | "reference" | "compiler_alias") {
            return Err(ModelError::Schema("selected outgoing family"));
        }
        let selected = self.clone();
        let table = table.to_owned();
        NativeRows::owned(selected.budget.clone(), move |sender| async move {
            if table == "compiler_alias" {
                let mut aliases = selected.aliases.cursor_with_budget(&selected.budget)?;
                while let Some(row) = aliases.next_row()? {
                    selected.source.check_live()?;
                    if !sender.send(row).await? {
                        break;
                    }
                }
                selected.source.check_live()?;
                return Ok(());
            }
            selected.bind_role_scope(&mut Variables::new())?;
            let mut sorted = SortedRows::with_budget(&selected.budget)?;
            let mut cursor = selected.pointers()?;
            loop {
                if sender.is_closed() {
                    return Ok(());
                }
                let mut batch = Vec::new();
                while batch.len() < WINDOW {
                    let Some(row) = cursor.next_row()? else {
                        break;
                    };
                    batch.push(id(&row)?);
                }
                if batch.is_empty() {
                    break;
                }
                let mut vars = Variables::new();
                let derivation = selected.bind_role_scope(&mut vars)?;
                vars.insert("nodes", batch);
                consume(
                    &selected.source, &selected.budget,
                    format!("SELECT * FROM {table} WITH INDEX outgoing WHERE in IN $nodes AND derivation = $lctx_derivation"),
                    vars,
                    |row| {
                        Self::check_role_scope(&row, derivation)?;
                        sorted.push(row)
                    },
                )
                .await?;
            }
            let mut rows = sorted.finish()?;
            while let Some(row) = rows.next_row()? {
                if !sender.send(row).await? {
                    break;
                }
            }
            Ok(())
        })
    }
}

fn selected_references(
    inventory: &crate::completed_state::InstalledViewInventory,
    requested: &[ContentHash],
    budget: &ResourceBudget,
) -> Result<(Vec<crate::completed_state::InstalledViewRef>, StateCharge), ModelError> {
    if requested.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(ModelError::Conflict(
            "published exact requested view order/duplicate",
        ));
    }
    let mut charge = StateCharge::new(budget, "published exact selected view references");
    charge.grow(
        requested
            .len()
            .saturating_mul(std::mem::size_of::<crate::completed_state::InstalledViewRef>()),
    )?;
    let mut references = Vec::with_capacity(requested.len());
    for view in requested {
        let reference = inventory.view(*view)?;
        charge.grow(
            serde_json::to_vec(reference)
                .map_err(ModelError::codec)?
                .len()
                .saturating_mul(8)
                .saturating_add(1024),
        )?;
        references.push(reference.clone());
    }
    Ok((references, charge))
}
async fn snapshot_row(
    source: &SelectionSource,
    id: RecordId,
    expected: &Value,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let mut vars = Variables::new();
    vars.insert("record", id);
    let mut found = false;
    consume(
        source,
        budget,
        "SELECT * FROM $record".into(),
        vars,
        |row| {
            if found || row != expected {
                return Err(ModelError::Conflict(
                    "published transaction exact witness bytes",
                ));
            }
            found = true;
            Ok(())
        },
    )
    .await?;
    if !found {
        return Err(ModelError::Conflict(
            "published transaction witness missing",
        ));
    }
    Ok(())
}
async fn snapshot_incarnation(
    source: &SelectionSource,
    id: &RecordId,
    incarnation: i64,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let mut vars = Variables::new();
    vars.insert(
        "guard",
        crate::control::guard_id(&Value::RecordId(id.clone()))?,
    );
    let mut found = false;
    consume(
        source,
        budget,
        "SELECT phase,incarnation FROM $guard".into(),
        vars,
        |row| {
            if found
                || row.as_object().and_then(|row| row.get("phase"))
                    != Some(&Value::String("active".into()))
                || row.as_object().and_then(|row| row.get("incarnation"))
                    != Some(&incarnation.into_value())
            {
                return Err(ModelError::Conflict(
                    "published transaction exact witness incarnation",
                ));
            }
            found = true;
            Ok(())
        },
    )
    .await?;
    if !found {
        return Err(ModelError::Conflict(
            "published transaction witness guard missing",
        ));
    }
    Ok(())
}
async fn snapshot_holds(
    source: &SelectionSource,
    pairs: &[(RecordId, RecordId)],
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    for pairs in pairs.chunks(WINDOW) {
        let mut charge = StateCharge::new(budget, "snapshot exact hold correspondence");
        let mut expected = BTreeMap::new();
        for (owner, object) in pairs {
            charge.grow(
                node_bytes(owner)
                    .saturating_add(node_bytes(object))
                    .saturating_mul(6)
                    .saturating_add(512),
            )?;
            expected.insert(
                crate::control::hold_record_id(owner, object)?,
                (owner, object),
            );
        }
        let mut vars = Variables::new();
        vars.insert("holds", expected.keys().cloned().collect::<Vec<_>>());
        let mut seen = BTreeSet::new();
        consume(source, budget, "SELECT * FROM $holds".into(), vars, |row| {
            let address = id(&row)?;
            let Some((owner, object)) = expected.get(&address) else {
                return Err(ModelError::Conflict(
                    "snapshot foreign or noncanonical hold",
                ));
            };
            let fields = row
                .as_object()
                .ok_or(ModelError::Schema("snapshot hold object"))?;
            if !seen.insert(address)
                || fields.len() != 3
                || fields.get("owner") != Some(&Value::RecordId((*owner).clone()))
                || fields.get("object") != Some(&Value::RecordId((*object).clone()))
            {
                return Err(ModelError::Conflict(
                    "snapshot hold duplicate or mismatched incidence",
                ));
            }
            Ok(())
        })
        .await?;
        if seen.len() != expected.len() {
            return Err(ModelError::Conflict("snapshot required hold missing"));
        }
    }
    Ok(())
}
async fn snapshot_hold(
    source: &SelectionSource,
    owner: &RecordId,
    object: &RecordId,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    snapshot_holds(source, &[(owner.clone(), object.clone())], budget).await
}
async fn check_publication_snapshot(
    source: &SelectionSource,
    completed: &crate::completed_state::PublishedCompletedState,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let marker = completed.marker();
    let data = completed.completion();
    let identity = data.identity()?;
    let Some(Value::RecordId(publication)) = marker.get("id") else {
        return Err(ModelError::Schema("published transaction marker id"));
    };
    snapshot_row(
        source,
        publication.clone(),
        &Value::Object(marker.clone()),
        budget,
    )
    .await?;
    snapshot_row(source, data.record()?, &data.value(true)?, budget).await?;
    snapshot_incarnation(source, &data.record()?, completed.incarnation(), budget).await?;
    let admission = completed.admission();
    snapshot_row(source, admission.record()?, &admission.value()?, budget).await?;
    snapshot_incarnation(
        source,
        &admission.record()?,
        completed.admission_incarnation(),
        budget,
    )
    .await?;
    snapshot_hold(source, &admission.record()?, &data.record()?, budget).await?;
    let Some(Value::RecordId(root)) = marker.get("staging_root") else {
        return Err(ModelError::Schema("published transaction root"));
    };
    let Some(Value::Number(surrealdb::types::Number::Int(root_inc))) =
        marker.get("staging_incarnation")
    else {
        return Err(ModelError::Schema("published transaction root incarnation"));
    };
    snapshot_incarnation(source, root, *root_inc, budget).await?;
    snapshot_hold(source, publication, root, budget).await?;
    snapshot_hold(source, root, &data.record()?, budget).await?;
    snapshot_hold(source, root, &admission.record()?, budget).await?;
    let mut vars = Variables::new();
    vars.insert("generation", data.generation.hex());
    vars.insert("epoch", data.validity_epoch);
    let mut found = false;
    consume(source, budget,"SELECT id FROM native_installation:current WHERE generation=$generation AND validity_epoch=$epoch".into(),vars,|_|{if found{return Err(ModelError::Conflict("snapshot installation duplicate"));}found=true;Ok(())}).await?;
    if !found {
        return Err(ModelError::Conflict(
            "published transaction current validity epoch",
        ));
    }
    let mut hasher = crate::completed_state::RealizationHasher::default();
    let mut seen = 0usize;
    let mut vars = Variables::new();
    vars.insert("completion", data.record()?);
    consume(source, budget,"SELECT * FROM native_completed_view WITH INDEX completion_view WHERE completion=$completion ORDER BY logical_view,id".into(),vars,|row|{
        let reference=crate::completed_state::InstalledViewRef::from_row(&row,identity)?;hasher.row(&reference)?;
        if completed.inventory().references().get(seen)!=Some(&reference){return Err(ModelError::Conflict("published transaction exact complete inventory"));}seen+=1;Ok(())
    }).await?;
    if seen != completed.inventory().references().len()
        || hasher.finish() != data.realization_inventory
    {
        return Err(ModelError::Conflict(
            "published transaction inventory actual EOF",
        ));
    }
    let mut contributor_hasher = crate::completed_state::ContributorHasher::default();
    let mut copy_count = 0usize;
    let mut vars = Variables::new();
    vars.insert("completion", data.record()?);
    consume(source, budget,"SELECT * FROM native_completed_contributor WITH INDEX completion_contributor WHERE completion=$completion ORDER BY physical,id".into(),vars,|row|{
        let copy=crate::completed_state::ContributorCopy::from_row(&row,identity)?;
        if completed.inventory().copies().get(&copy.physical)!=Some(&copy){return Err(ModelError::Conflict("snapshot compound contributor full inventory"));}
        contributor_hasher.row(&copy)?;copy_count+=1;Ok(())
    }).await?;
    if copy_count != completed.inventory().copies().len()
        || contributor_hasher.finish() != data.contributor_inventory
    {
        return Err(ModelError::Conflict(
            "snapshot compound contributor EOF/digest",
        ));
    }
    let mut hold_charge = StateCharge::new(budget, "snapshot compound exact hold premises");
    let mut pairs = Vec::new();
    for reference in completed.inventory().references() {
        let child = reference.record(identity);
        hold_charge.grow(node_bytes(&child).saturating_mul(4).saturating_add(512))?;
        pairs.push((data.record()?, child));
    }
    hold_charge.grow(
        completed
            .inventory()
            .copies()
            .len()
            .saturating_mul(std::mem::size_of::<&crate::completed_state::ContributorCopy>()),
    )?;
    let physical = completed.inventory().copies().values().collect::<Vec<_>>();
    for copies in physical.chunks(WINDOW) {
        let mut observed = BTreeMap::new();
        let mut guards = BTreeMap::new();
        let mut charge = StateCharge::new(budget, "snapshot contributor physical correspondence");
        for copy in copies {
            let owner = RecordId::new("compiler_contribution", copy.physical.hex());
            charge.grow(node_bytes(&owner).saturating_mul(10).saturating_add(1024))?;
            observed.insert(owner.clone(), *copy);
            guards.insert(
                crate::control::guard_id(&Value::RecordId(owner.clone()))?,
                copy.incarnation,
            );
            hold_charge.grow(node_bytes(&owner).saturating_mul(8).saturating_add(1024))?;
            pairs.push((data.record()?, copy.record(identity)));
            pairs.push((data.record()?, owner));
        }
        let mut vars = Variables::new();
        vars.insert("contributors", observed.keys().cloned().collect::<Vec<_>>());
        let mut seen = BTreeSet::new();
        consume(
            source,
            budget,
            "SELECT * FROM $contributors".into(),
            vars,
            |row| {
                let owner = id(&row)?;
                let copy = observed
                    .get(&owner)
                    .ok_or(ModelError::Conflict("snapshot foreign contributor"))?;
                if !seen.insert(owner) {
                    return Err(ModelError::Conflict("snapshot duplicate contributor"));
                }
                let _scratch = budget.reserve(
                    "snapshot selected contributor decode",
                    crate::loader::native_bytes(&row)
                        .saturating_mul(32)
                        .saturating_add(65536),
                )?;
                crate::completed_rows::validate_state_row("compiler_contribution", &row)?;
                if crate::installed_view::audit::descriptor::<
                    lctx_model::domain::completed::CompletedContribution,
                >(&row)?
                    != copy.descriptor
                {
                    return Err(ModelError::Conflict(
                        "snapshot selected contributor full descriptor",
                    ));
                }
                Ok(())
            },
        )
        .await?;
        if seen.len() != observed.len() {
            return Err(ModelError::Conflict("snapshot contributor missing"));
        }
        let mut vars = Variables::new();
        vars.insert("guards", guards.keys().cloned().collect::<Vec<_>>());
        let mut seen = BTreeSet::new();
        consume(
            source,
            budget,
            "SELECT id,phase,incarnation FROM $guards".into(),
            vars,
            |row| {
                let guard = id(&row)?;
                let expected = guards
                    .get(&guard)
                    .ok_or(ModelError::Conflict("snapshot foreign contributor guard"))?;
                let fields = row
                    .as_object()
                    .ok_or(ModelError::Schema("snapshot guard object"))?;
                if !seen.insert(guard)
                    || fields.get("phase") != Some(&Value::String("active".into()))
                    || fields.get("incarnation") != Some(&(*expected).into_value())
                {
                    return Err(ModelError::Conflict(
                        "snapshot contributor guard duplicate/inactive/incarnation",
                    ));
                }
                Ok(())
            },
        )
        .await?;
        if seen.len() != guards.len() {
            return Err(ModelError::Conflict("snapshot contributor guard missing"));
        }
    }
    snapshot_holds(source, &pairs, budget).await?;
    let mut bindings = Vec::new();
    let mut vars = Variables::new();
    vars.insert("completion", data.record()?);
    let mut charge = StateCharge::new(budget, "snapshot exact canonical binding inventory");
    consume(source, budget,"SELECT object AS id FROM native_hold WITH INDEX owner_holds WHERE owner=$completion AND record::table(object)='compiler_binding_descriptor' ORDER BY object".into(),vars,|row|{
        let address=id(&row)?;charge.grow(node_bytes(&address).saturating_mul(2).saturating_add(128))?;bindings.push(address);Ok(())
    }).await?;
    if bindings.len() != completed.inventory().bindings().len() {
        return Err(ModelError::Conflict("snapshot complete binding hold EOF"));
    }
    for binding in completed.inventory().bindings() {
        let id = crate::binding_descriptor::record(binding)?;
        if !bindings.contains(&id) {
            return Err(ModelError::Conflict(
                "snapshot missing canonical binding hold",
            ));
        }
        snapshot_row(
            source,
            id.clone(),
            &crate::binding_descriptor::value(binding)?,
            budget,
        )
        .await?;
        snapshot_incarnation(
            source,
            &id,
            completed
                .inventory()
                .binding_incarnation(binding.descriptor_identity()?)?,
            budget,
        )
        .await?;
        snapshot_hold(
            source,
            &id,
            &RecordId::new("compiler_view", binding.view.identity.hex()),
            budget,
        )
        .await?;
    }
    // The marker comparison is repeated under the same snapshot; external capability
    // liveness is checked again before any prepared selection escapes this constructor.
    snapshot_row(
        source,
        publication.clone(),
        &Value::Object(marker.clone()),
        budget,
    )
    .await?;
    completed.retained_reader().ensure_scope_live()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_owned_alias_preparation_keeps_selected_branch_multiplicity_and_ignores_unreachable_other_outputs()
     {
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        let mut seeds = SortedRows::with_budget(&budget).unwrap();
        seeds
            .push(pointer(RecordId::new("entity", "selected")))
            .unwrap();
        let mut aliases = SortedRows::with_budget(&budget).unwrap();
        for (ordinal, (source, target)) in [
            ("selected", "a"),
            ("selected", "b"),
            ("a", "c"),
            ("c", "a"),
            ("unselected-sibling", "foreign"),
        ]
        .into_iter()
        .enumerate()
        {
            let mut row = Object::new();
            row.insert("id", RecordId::new("compiler_alias", ordinal as i64));
            row.insert("source", RecordId::new("entity", source));
            row.insert("target", RecordId::new("entity", target));
            aliases.push(Value::Object(row)).unwrap();
        }
        let (nodes, aliases) = SelectedPayloads::expand_owned_aliases(
            seeds.finish().unwrap().into_prepared(),
            aliases.finish().unwrap().into_prepared(),
            &budget,
        )
        .unwrap();
        let mut cursor = nodes.cursor_with_budget(&budget).unwrap();
        let mut actual = BTreeSet::new();
        while let Some(row) = cursor.next_row().unwrap() {
            actual.insert(id(&row).unwrap());
        }
        assert_eq!(
            actual,
            ["selected", "a", "b", "c"]
                .map(|key| RecordId::new("entity", key))
                .into(),
            "every reachable branch remains visible for nominal ambiguity checks"
        );
        let mut cursor = aliases.cursor_with_budget(&budget).unwrap();
        let mut count = 0;
        while cursor.next_row().unwrap().is_some() {
            count += 1;
        }
        assert_eq!(
            count, 4,
            "reachable cycle terminates and aliases from unselected sibling outputs remain outside demand"
        );
    }

    fn role_selection(
        source: SelectionSource,
        nodes: &[RecordId],
        budget: ResourceBudget,
    ) -> SelectedPayloads {
        let mut rows = SortedRows::with_budget(&budget).unwrap();
        for node in nodes {
            rows.push(pointer(node.clone())).unwrap();
        }
        let nodes = rows.finish().unwrap().into_prepared();
        let membership = nodes.point_index(&budget).unwrap();
        SelectedPayloads {
            exact: None,
            explicit_owners: None,
            derivation: None,
            source,
            nodes,
            membership,
            aliases: SortedRows::with_budget(&budget)
                .unwrap()
                .finish()
                .unwrap()
                .into_prepared(),
            topology: None,
            budget,
        }
    }
    fn test_key(key: &str) -> String {
        hex::encode(&ContentHash::of(key.as_bytes()).0[..16])
    }
    fn test_membership(
        view: ContentHash,
        node: RecordId,
        relation: &str,
        key: &str,
        content: &str,
    ) -> Value {
        let key = test_key(key);
        let mut row = Object::new();
        row.insert(
            "id",
            crate::completed_rows::membership_id(
                view,
                relation,
                &crate::completed_rows::decode_nominal_key(&key).unwrap(),
            ),
        );
        row.insert(
            "contribution",
            RecordId::new("compiler_contribution", view.hex()),
        );
        if node.table.as_str() == "compiler_record" {
            row.insert("storage", "block");
            row.insert(
                "block",
                RecordId::new(
                    "compiler_block",
                    ContentHash::of(b"mechanical selected block").hex(),
                ),
            );
            row.insert("slot", 0i64);
        } else {
            row.insert("storage", "graph");
        }
        row.insert("scope_keys", Vec::<Value>::new());
        row.insert("scope_lookup", Vec::<Value>::new());
        row.insert("node", node);
        row.insert("relation", relation.to_owned());
        row.insert("semantic_key", key);
        row.insert("content", content.to_owned());
        Value::Object(row)
    }
    // Mechanical exact-contributor controls do not construct a native capability.
    fn exact_selection(
        source: SelectionSource,
        views: &[ContentHash],
        relation: &str,
        aliases: &[(RecordId, RecordId)],
        budget: &ResourceBudget,
    ) -> SelectedPayloads {
        let mut selected = role_selection(source, &[], budget.clone());
        let mut charge = StateCharge::new(budget, "mechanical exact test contributors");
        let mut owner_views = BTreeMap::new();
        for view in views {
            charge.grow(1024).unwrap();
            owner_views.insert(
                RecordId::new("compiler_contribution", view.hex()),
                BTreeMap::from([(relation.to_owned(), BTreeSet::from([*view]))]),
            );
        }
        selected.exact = Some(Arc::new(CanonicalSelection {
            views: views.to_vec(),
            owner_views,
            alias_membership: BTreeMap::new(),
            _charge: charge,
        }));
        let mut rows = SortedRows::with_budget(budget).unwrap();
        for (index, (source, target)) in aliases.iter().enumerate() {
            let mut row = Object::new();
            row.insert("id", RecordId::new("compiler_alias", index.to_string()));
            row.insert("source", source.clone());
            row.insert("target", target.clone());
            rows.push(Value::Object(row)).unwrap();
        }
        selected.aliases = rows.finish().unwrap().into_prepared();
        let mut nodes = SortedRows::with_budget(budget).unwrap();
        for (source, target) in aliases {
            nodes.push(pointer(source.clone())).unwrap();
            nodes.push(pointer(target.clone())).unwrap();
        }
        selected.nodes = nodes.finish().unwrap().into_prepared();
        selected.membership = selected.nodes.point_index(budget).unwrap();
        selected.topology = Some(
            crate::alias_topology::AliasTopology::new(selected.aliases.clone(), budget, || Ok(()))
                .unwrap(),
        );
        let mut exact = selected.exact.take().unwrap();
        for view in views {
            Arc::get_mut(&mut exact)
                .unwrap()
                .alias_membership
                .insert(*view, selected.aliases.point_index(budget).unwrap());
        }
        selected.exact = Some(exact);
        selected
    }
    fn selector_fixture_source(
        members: Vec<Value>,
        owned_aliases: Vec<(ContentHash, Value)>,
    ) -> SelectionSource {
        let aliases = owned_aliases
            .iter()
            .map(|(_, row)| (id(row).unwrap(), row.clone()))
            .collect::<BTreeMap<_, _>>();
        let holds = owned_aliases
            .into_iter()
            .map(|(owner, row)| {
                let owner = RecordId::new("compiler_contribution", owner.hex());
                let alias = id(&row).unwrap();
                let mut hold = Object::new();
                hold.insert(
                    "id",
                    crate::control::hold_record_id(&owner, &alias).unwrap(),
                );
                hold.insert("owner", owner);
                hold.insert("object", alias);
                Value::Object(hold)
            })
            .collect::<Vec<_>>();
        SelectionSource::Fixture(Arc::new(move |query, budget| {
            let (sql, vars) = query.into_request();
            assert!(!sql.contains("FROM compiler_alias"));
            let rows = if sql.contains("FROM compiler_membership") {
                let owners =
                    Vec::<RecordId>::from_value(vars.get("contributors").unwrap().clone()).unwrap();
                let nodes = vars
                    .get("nodes")
                    .map(|nodes| Vec::<RecordId>::from_value(nodes.clone()).unwrap());
                let keys = vars
                    .get("keys")
                    .map(|keys| Vec::<String>::from_value(keys.clone()).unwrap());
                members
                    .iter()
                    .filter(|row| {
                        let (owner, node, relation, key, _) = membership_fields(row).unwrap();
                        owners.contains(owner)
                            && nodes.as_ref().is_none_or(|nodes| nodes.contains(node))
                            && vars
                                .get("relation")
                                .is_none_or(|value| value == Value::String(relation.into()))
                            && keys
                                .as_ref()
                                .is_none_or(|keys| keys.iter().any(|candidate| candidate == key))
                    })
                    .cloned()
                    .collect::<Vec<_>>()
            } else if sql.contains("FROM native_hold") {
                let owners =
                    Vec::<RecordId>::from_value(vars.get("contributors").unwrap().clone()).unwrap();
                holds.iter().filter(|row|matches!(row.as_object().unwrap().get("owner"),Some(Value::RecordId(owner)) if owners.contains(owner))).cloned().collect()
            } else {
                assert_eq!(sql, "SELECT * FROM $ids;");
                let ids = Vec::<RecordId>::from_value(vars.get("ids").unwrap().clone()).unwrap();
                ids.iter().map(|id| aliases[id].clone()).collect()
            };
            NativeRows::owned(budget.clone(), move |sender| async move {
                for row in rows {
                    if !sender.send(row).await? {
                        break;
                    }
                }
                Ok(())
            })
        }))
    }
    fn fixture_alias(name: &str, source: &str, target: &str) -> Value {
        let mut row = Object::new();
        row.insert("id", RecordId::new("compiler_alias", name));
        row.insert("source", RecordId::new("entity", source));
        row.insert("target", RecordId::new("entity", target));
        Value::Object(row)
    }
    #[tokio::test]
    async fn combined_selectors_keep_alias_witnesses_inside_each_contributor_closure() {
        let budget = ResourceBudget::fixed(32 << 20).unwrap();
        let a = ContentHash::of(b"physical A");
        let b = ContentHash::of(b"physical B");
        let va = lctx_model::domain::completed::CompletedView::new(
            "places".into(),
            BTreeSet::from([a]),
            1,
        )
        .unwrap();
        let vb = lctx_model::domain::completed::CompletedView::new(
            "places".into(),
            BTreeSet::from([b]),
            1,
        )
        .unwrap();
        let ma = [crate::installed_view::ContributorRealization {
            view: va.identity,
            logical: a,
            physical: a,
            incarnation: 1,
        }];
        let mb = [crate::installed_view::ContributorRealization {
            view: vb.identity,
            logical: b,
            physical: b,
            incarnation: 1,
        }];
        let x = RecordId::new("entity", "X");
        let y = RecordId::new("entity", "Y");
        let source = selector_fixture_source(
            vec![
                test_membership(a, x.clone(), "places", "X", "immutable"),
                test_membership(b, x.clone(), "places", "X", "immutable"),
            ],
            vec![(b, fixture_alias("B edge", "X", "Y"))],
        );
        let only_a = SelectedPayloads::prepare_selectors(source.clone(), &[(&va, &ma)], &budget)
            .await
            .unwrap();
        let absent = only_a
            .membership_answers(std::slice::from_ref(&y), &budget)
            .await
            .unwrap();
        assert!(!absent.contains(&y));
        drop(absent);
        drop(only_a);
        let combined =
            SelectedPayloads::prepare_selectors(source, &[(&va, &ma), (&vb, &mb)], &budget)
                .await
                .unwrap();
        let answers = combined
            .membership_answers(std::slice::from_ref(&y), &budget)
            .await
            .unwrap();
        assert!(answers.contains(&y));
        assert_eq!(answers.answers[0].witnesses.len(), 1);
        assert_eq!(answers.answers[0].witnesses[0].view, vb.identity);
        assert_eq!(answers.answers[0].witnesses[0].ancestry, vec![y, x]);
        drop(answers);
        drop(combined);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn aliases_from_different_selector_closures_cannot_assemble_a_new_chain() {
        let budget = ResourceBudget::fixed(32 << 20).unwrap();
        let a = ContentHash::of(b"chain A");
        let b = ContentHash::of(b"chain B");
        let va = lctx_model::domain::completed::CompletedView::new(
            "places".into(),
            BTreeSet::from([a]),
            1,
        )
        .unwrap();
        let vb = lctx_model::domain::completed::CompletedView::new(
            "places".into(),
            BTreeSet::from([b]),
            1,
        )
        .unwrap();
        let ma = [crate::installed_view::ContributorRealization {
            view: va.identity,
            logical: a,
            physical: a,
            incarnation: 1,
        }];
        let mb = [crate::installed_view::ContributorRealization {
            view: vb.identity,
            logical: b,
            physical: b,
            incarnation: 1,
        }];
        let source = selector_fixture_source(
            vec![
                test_membership(a, RecordId::new("entity", "X"), "places", "X", "immutable"),
                test_membership(
                    b,
                    RecordId::new("entity", "Z"),
                    "places",
                    "X",
                    "different canonical content",
                ),
                test_membership(b, RecordId::new("entity", "M"), "units", "M", "immutable"),
            ],
            vec![
                (a, fixture_alias("A prefix", "X", "M")),
                (b, fixture_alias("B suffix", "M", "Y")),
            ],
        );
        let combined =
            SelectedPayloads::prepare_selectors(source, &[(&va, &ma), (&vb, &mb)], &budget)
                .await
                .unwrap();
        let y = RecordId::new("entity", "Y");
        assert!(!combined.contains_all(std::slice::from_ref(&y)).unwrap());
        let answers = combined
            .membership_answers(std::slice::from_ref(&y), &budget)
            .await
            .unwrap();
        assert!(!answers.contains(&y));
        assert!(answers.answers[0].witnesses.is_empty());
        let mut aliases = combined.aliases().unwrap();
        let mut actual = Vec::new();
        while let Some(row) = aliases.next_row().unwrap() {
            actual.push(id(&row).unwrap());
        }
        assert_eq!(actual, vec![RecordId::new("compiler_alias", "A prefix")]);
        drop(aliases);
        drop(answers);
        drop(combined);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn logical_root_factories_refuse_without_opening_native_queries() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let client = Arc::new(Surreal::init());
        for views in [vec![], vec![ContentHash::of(b"unowned logical root")]] {
            assert!(
                SelectedPayloads::for_views(client.clone(), &views, &budget)
                    .await
                    .is_err()
            );
            assert!(
                SelectedPayloads::for_cancellable_views(client.clone(), &views, None, &budget)
                    .await
                    .is_err()
            );
        }
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn duplicate_logical_selectors_refuse_before_any_inventory_query() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let logical = ContentHash::of(b"contributor");
        let view = lctx_model::domain::completed::CompletedView::new(
            "places".into(),
            BTreeSet::from([logical]),
            1,
        )
        .unwrap();
        let mappings = vec![crate::installed_view::ContributorRealization {
            view: view.identity,
            logical,
            physical: logical,
            incarnation: 1,
        }];
        let source = SelectionSource::Fixture(Arc::new(|_, _budget| {
            panic!("duplicate exact input must refuse before query")
        }));
        assert!(
            SelectedPayloads::prepare_selectors(
                source,
                &[(&view, &mappings), (&view, &mappings)],
                &budget
            )
            .await
            .is_err()
        );
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn exact_membership_batches_contributors_and_refuses_unseen_nominal_variant() {
        let one = ContentHash::of(b"logical one");
        let two = ContentHash::of(b"logical two");
        let direct = RecordId::new("entity", "source");
        let target = RecordId::new("entity", "owned target");
        let variant = RecordId::new("entity", "unseen variant");
        let nominal_queries = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        for conflict in [false, true] {
            let budget = ResourceBudget::fixed(4 << 20).unwrap();
            let source_node = direct.clone();
            let variant = variant.clone();
            let observed = nominal_queries.clone();
            let source = SelectionSource::Fixture(Arc::new(move |query, budget| {
                let (sql, vars) = query.into_request();
                assert!(!sql.contains("compiler_alias"));
                assert!(!sql.contains("compiler_view_member"));
                assert!(vars.get("view").is_none() && vars.get("realization").is_none());
                let owners =
                    Vec::<RecordId>::from_value(vars.get("contributors").unwrap().clone()).unwrap();
                assert_eq!(owners.len(), 2, "views share one contributor query window");
                let mut rows = Vec::new();
                if sql.contains("contribution_nodes") {
                    let requested =
                        Vec::<RecordId>::from_value(vars.get("nodes").unwrap().clone()).unwrap();
                    if requested.contains(&source_node) {
                        rows.push(test_membership(
                            one,
                            source_node.clone(),
                            "places",
                            "same nominal key",
                            "same canonical content",
                        ));
                    }
                } else if sql.contains("contribution_rows") {
                    observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    rows.push(test_membership(
                        one,
                        source_node.clone(),
                        "places",
                        "same nominal key",
                        "same canonical content",
                    ));
                    if conflict {
                        rows.push(test_membership(
                            two,
                            variant.clone(),
                            "places",
                            "same nominal key",
                            "same canonical content",
                        ));
                    }
                } else {
                    panic!("unexpected canonical membership query: {sql}");
                }
                NativeRows::owned(budget.clone(), move |sender| async move {
                    for row in rows {
                        if !sender.send(row).await? {
                            break;
                        }
                    }
                    Ok(())
                })
            }));
            let selected = exact_selection(
                source,
                &[one, two],
                "places",
                &[(direct.clone(), target.clone())],
                &budget,
            );
            let result = selected
                .membership_answers(std::slice::from_ref(&target), &budget)
                .await;
            if conflict {
                assert!(
                    result.is_err(),
                    "off-nomination selected nominal variant must refuse"
                );
            } else {
                let answer = result.unwrap();
                assert!(answer.contains(&target));
                assert_eq!(answer.answers[0].witnesses.len(), 1);
                assert_eq!(
                    answer.answers[0].witnesses[0].ancestry,
                    vec![target.clone(), direct.clone()]
                );
            }
        }
        assert_eq!(
            nominal_queries.load(std::sync::atomic::Ordering::SeqCst),
            2,
            "one nominal query for all exact contributors per case"
        );
    }

    #[tokio::test]
    async fn derived_outgoing_scopes_before_enumeration_and_keeps_unexpected_selected_roles() {
        let derivation = ContentHash::of(b"selected implementation");
        let node = RecordId::new("entity", "selected");
        let selected_node = node.clone();
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed = calls.clone();
        let source = SelectionSource::Fixture(Arc::new(move |query, budget| {
            let (sql, vars) = query.into_request();
            assert!(
                sql.contains("AND derivation = $lctx_derivation"),
                "namespace must constrain actual nomination: {sql}"
            );
            assert_eq!(
                vars.get("lctx_derivation"),
                Some(&Value::String(derivation.hex()))
            );
            assert_eq!(
                Vec::<RecordId>::from_value(vars.get("nodes").unwrap().clone()).unwrap(),
                vec![selected_node.clone()]
            );
            observed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let table = if sql.contains("FROM participant ") {
                "participant"
            } else {
                "reference"
            };
            let rows = ["expected", "unexpected-selected-role"].map(|field| {
                let mut row = Object::new();
                row.insert("id", RecordId::new(table, field));
                row.insert("in", selected_node.clone());
                row.insert("out", RecordId::new("external", field));
                row.insert("field", field);
                row.insert("derivation", derivation.hex());
                Value::Object(row)
            });
            NativeRows::owned(budget.clone(), move |sender| async move {
                for row in rows {
                    if !sender.send(row).await? {
                        break;
                    }
                }
                Ok(())
            })
        }));
        let selected = role_selection(source, &[node], ResourceBudget::fixed(8 << 20).unwrap())
            .with_derivation(derivation);
        for table in ["participant", "reference"] {
            let mut rows = selected.outgoing(table).unwrap();
            let mut fields = BTreeSet::new();
            while let Some(row) = rows.next().await.unwrap() {
                fields.insert(
                    String::from_value(row.as_object().unwrap().get("field").unwrap().clone())
                        .unwrap(),
                );
            }
            rows.drain_transport().await.unwrap();
            assert_eq!(
                fields,
                BTreeSet::from(["expected".to_owned(), "unexpected-selected-role".to_owned()]),
                "actual unexpected roles remain independent integrity claims"
            );
        }
        assert_eq!(calls.load(std::sync::atomic::Ordering::Relaxed), 2);
    }
    #[tokio::test]
    async fn unqualified_derived_demand_refuses_even_empty_selection_before_native_access() {
        let source = SelectionSource::Fixture(Arc::new(|_, _budget| {
            panic!("unqualified derived demand must refuse before native access")
        }));
        let selected = role_selection(source, &[], ResourceBudget::fixed(8 << 20).unwrap());
        for mut rows in [
            selected.outgoing("reference").unwrap(),
            selected.recovery_roles().unwrap(),
        ] {
            let error = rows.next().await.unwrap_err();
            assert!(error.to_string().contains("exact derivation"), "{error}");
            let _ = rows.drain_transport().await;
        }
        let mut canonical = selected.outgoing("compiler_alias").unwrap();
        assert!(canonical.next().await.unwrap().is_none());
        canonical.drain_transport().await.unwrap();
    }
    #[test]
    fn role_namespace_readback_refuses_foreign_rows() {
        let mut row = Object::new();
        row.insert("derivation", ContentHash::of(b"foreign").hex());
        assert!(
            SelectedPayloads::check_role_scope(&Value::Object(row), ContentHash::of(b"selected"))
                .is_err()
        );
    }

    #[tokio::test]
    async fn root_windows_preserve_indexed_exact_union_across_chunk_boundaries() {
        for owners in [true] {
            for count in [0usize, 1, 128, 129, 257] {
                let budget = ResourceBudget::fixed(8 << 20).unwrap();
                let roots = (0..count)
                    .map(|ordinal| ContentHash::of(format!("selected-root-{ordinal}").as_bytes()))
                    .collect::<Vec<_>>();
                let table = if owners {
                    "compiler_contribution"
                } else {
                    "compiler_view"
                };
                let mut members = BTreeMap::new();
                for (ordinal, root) in roots.iter().enumerate() {
                    members.insert(
                        RecordId::new(table, root.hex()),
                        vec![
                            RecordId::new(
                                "compiler_record",
                                format!("member-{:03}", ordinal % 193),
                            ),
                            RecordId::new("compiler_record", "shared"),
                        ],
                    );
                }
                // The independent singleton union includes overlaps and deliberately scrambled results.
                let expected = members
                    .values()
                    .flatten()
                    .cloned()
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>();
                let calls = Arc::new(std::sync::Mutex::new(Vec::new()));
                let observed = calls.clone();
                let source = SelectionSource::Fixture(Arc::new(move |query, budget| {
                    let (sql, vars) = query.into_request();
                    assert!(
                        sql.contains("WITH INDEX contribution_rows WHERE contribution IN $roots")
                    );
                    let roots =
                        Vec::<RecordId>::from_value(vars.get("roots").unwrap().clone()).unwrap();
                    assert!(!roots.is_empty() && roots.len() <= WINDOW);
                    observed.lock().unwrap().push(roots.clone());
                    let values = roots
                        .iter()
                        .rev()
                        .flat_map(|root| members[root].iter().rev())
                        .cloned()
                        .map(|node| {
                            let mut row = Object::new();
                            row.insert("node", node);
                            Value::Object(row)
                        })
                        .collect::<Vec<_>>();
                    NativeRows::owned(budget.clone(), move |sender| async move {
                        for row in values {
                            if !sender.send(row).await? {
                                break;
                            }
                        }
                        Ok(())
                    })
                }));
                let selected = SelectedPayloads::prepare(source, &roots, owners, &budget)
                    .await
                    .unwrap();
                let mut cursor = selected.pointers().unwrap();
                let mut actual = Vec::new();
                while let Some(row) = cursor.next_row().unwrap() {
                    actual.push(id(&row).unwrap());
                }
                assert_eq!(actual, expected);
                let calls = calls.lock().unwrap();
                assert_eq!(calls.len(), count.div_ceil(WINDOW));
                assert_eq!(
                    calls.iter().flatten().cloned().collect::<Vec<_>>(),
                    roots
                        .iter()
                        .map(|root| RecordId::new(table, root.hex()))
                        .collect::<Vec<_>>()
                );
                drop(cursor);
                drop(selected);
                assert_eq!(budget.reserved(), 0);
            }
        }
    }
    #[tokio::test]
    async fn raw_capture_uses_only_exact_owner_alias_incidence_even_when_sources_are_shared() {
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let root = ContentHash::of(b"raw capture owner");
        let owner = RecordId::new("compiler_contribution", root.hex());
        let source_owner = owner.clone();
        let seed = RecordId::new("entity", "seed");
        let bridge = RecordId::new("entity", "bridge");
        let target = RecordId::new("entity", "target");
        let mut aliases = BTreeMap::new();
        for (name, source, target) in [
            ("first", seed.clone(), bridge.clone()),
            ("second", bridge.clone(), target.clone()),
            (
                "foreign",
                seed.clone(),
                RecordId::new("entity", "foreign target"),
            ),
        ] {
            let mut row = Object::new();
            row.insert("id", RecordId::new("compiler_alias", name));
            row.insert("source", source);
            row.insert("target", target);
            aliases.insert(name.to_owned(), Value::Object(row));
        }
        let original_seed = seed.clone();
        let reads = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed = reads.clone();
        let source = SelectionSource::Fixture(Arc::new(move |query, budget| {
            let (sql, vars) = query.into_request();
            assert!(
                !sql.contains("FROM compiler_alias"),
                "raw capture cannot discover globally by a shared source: {sql}"
            );
            let rows = if sql.contains("FROM compiler_membership") {
                let mut row = Object::new();
                row.insert("node", seed.clone());
                vec![Value::Object(row)]
            } else if sql.contains("FROM native_hold") {
                assert_eq!(
                    Vec::<RecordId>::from_value(vars.get("contributors").unwrap().clone()).unwrap(),
                    vec![source_owner.clone()]
                );
                ["first", "second"]
                    .into_iter()
                    .map(|name| {
                        let object = RecordId::new("compiler_alias", name);
                        let mut row = Object::new();
                        row.insert(
                            "id",
                            crate::control::hold_record_id(&source_owner, &object).unwrap(),
                        );
                        row.insert("owner", source_owner.clone());
                        row.insert("object", object);
                        Value::Object(row)
                    })
                    .collect()
            } else {
                assert_eq!(sql, "SELECT * FROM $ids;");
                let ids = Vec::<RecordId>::from_value(vars.get("ids").unwrap().clone()).unwrap();
                assert_eq!(ids.len(), 2);
                observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                aliases
                    .values()
                    .filter(|row| ids.contains(&id(row).unwrap()))
                    .cloned()
                    .collect()
            };
            NativeRows::owned(budget.clone(), move |sender| async move {
                for row in rows {
                    if !sender.send(row).await? {
                        break;
                    }
                }
                Ok(())
            })
        }));
        let selected = SelectedPayloads::prepare(source, &[root], true, &budget)
            .await
            .unwrap();
        assert!(
            selected
                .contains_all(&[original_seed, bridge, target])
                .unwrap()
        );
        assert!(
            !selected
                .contains_all(&[RecordId::new("entity", "foreign target")])
                .unwrap()
        );
        let mut aliases = selected.outgoing("compiler_alias").unwrap();
        let mut actual = BTreeSet::new();
        while let Some(row) = aliases.next().await.unwrap() {
            actual.insert(id(&row).unwrap());
        }
        aliases.drain_transport().await.unwrap();
        assert_eq!(
            actual,
            ["first", "second"]
                .map(|name| RecordId::new("compiler_alias", name))
                .into()
        );
        assert_eq!(
            reads.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "owned alias payloads are hydrated once and outgoing reuses them"
        );
        drop(aliases);
        drop(selected);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn root_windows_refuse_unadmitted_binding_before_query() {
        let budget = ResourceBudget::fixed(16 << 10).unwrap();
        let roots = (0..WINDOW)
            .map(|ordinal| ContentHash::of(format!("root-{ordinal}").as_bytes()))
            .collect::<Vec<_>>();
        let source = SelectionSource::Fixture(Arc::new(|_, _budget| {
            panic!("unadmitted root window opened a query")
        }));
        let error = SelectedPayloads::prepare(source, &roots, true, &budget)
            .await
            .err()
            .unwrap();
        assert!(
            error.to_string().contains("selected-root-window"),
            "{error:?}"
        );
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn root_windows_drain_consumer_failure_and_preserve_terminal_failure_and_cancellation() {
        use std::sync::atomic::{AtomicBool, Ordering};
        for terminal_failure in [false, true] {
            let budget = ResourceBudget::fixed(8 << 20).unwrap();
            let drained = Arc::new(AtomicBool::new(false));
            let observed = drained.clone();
            let source = SelectionSource::Fixture(Arc::new(move |_, budget| {
                let observed = observed.clone();
                NativeRows::owned(budget.clone(), move |sender| async move {
                    for row in [
                        Value::Null,
                        pointer(RecordId::new("compiler_record", "remaining")),
                    ] {
                        if !sender.send(row).await? {
                            break;
                        }
                    }
                    observed.store(true, Ordering::Release);
                    if terminal_failure {
                        Err(ModelError::Schema("root-window-terminal-failure"))
                    } else {
                        Ok(())
                    }
                })
            }));
            let error =
                SelectedPayloads::prepare(source, &[ContentHash::of(b"root")], true, &budget)
                    .await
                    .err()
                    .unwrap();
            assert!(drained.load(Ordering::Acquire));
            assert!(format!("{error:?}").contains("selected membership pointer"));
            if terminal_failure {
                assert!(format!("{error:?}").contains("root-window-terminal-failure"));
            }
            assert_eq!(budget.reserved(), 0);
        }
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let source = SelectionSource::Cancellable(
            Box::new(SelectionSource::Fixture(Arc::new(|_, _budget| {
                panic!("cancelled root window opened a query")
            }))),
            Arc::new(AtomicBool::new(true)),
        );
        assert!(
            SelectedPayloads::prepare(source, &[ContentHash::of(b"root")], true, &budget)
                .await
                .is_err()
        );
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn selected_order_preserves_schema_field_tuple_order() {
        let strings = ["", "a", "a!", "a\"", "a'", "a|", "aa", "b", "é", "🍎"];
        let mut rows = Vec::new();
        for (n, text) in strings.iter().enumerate() {
            for table in ["anchor", "anchor!", "anchor2"] {
                let mut row = Object::new();
                row.insert("id", RecordId::new("entity", format!("id{n}")));
                row.insert("anchor", RecordId::new(table, *text));
                row.insert("semantic_key", *text);
                rows.push(Value::Object(row));
            }
        }
        for fields in [
            vec!["id"],
            vec!["anchor", "id"],
            vec!["semantic_key", "anchor", "id"],
        ] {
            let fields = fields.into_iter().map(str::to_owned).collect::<Vec<_>>();
            for left in &rows {
                for right in &rows {
                    let native = |value: &Value| {
                        let Value::Object(row) = value else {
                            unreachable!()
                        };
                        fields
                            .iter()
                            .map(|field| row.get(field).unwrap().clone())
                            .chain([Value::RecordId(id(value).unwrap())])
                            .collect::<Vec<_>>()
                    };
                    assert_eq!(
                        order_key(left, &fields)
                            .unwrap()
                            .cmp(&order_key(right, &fields).unwrap()),
                        native(left).cmp(&native(right))
                    );
                }
            }
        }
        let mut row = Object::new();
        row.insert("id", RecordId::new("entity", 1i64));
        assert!(order_key(&Value::Object(row), &["id".into()]).is_err());
    }
    #[test]
    fn selected_order_preserves_fixed_nominal_projection_and_window_tuples() {
        let mut low_tail = [0u8; 16];
        low_tail[15] = 255;
        let identities = [
            [0u8; 16], low_tail, [1; 16], [15; 16], [16; 16], [128; 16], [255; 16],
        ];
        let fields = ["body.projection", "body.window", "semantic_key"].map(str::to_owned);
        let mut rows = Vec::new();
        for projection in identities {
            for window in identities {
                for semantic_key in ["a", "a!"] {
                    let mut body = Object::new();
                    body.insert("projection", projection.to_vec());
                    body.insert("window", window.to_vec());
                    let mut row = Object::new();
                    row.insert("body", body);
                    row.insert("semantic_key", semantic_key);
                    row.insert(
                        "id",
                        RecordId::new("entity", format!("row-{:04}", rows.len())),
                    );
                    rows.push((projection, window, semantic_key, Value::Object(row)));
                }
            }
        }
        for (lp, lw, lk, left) in &rows {
            for (rp, rw, rk, right) in &rows {
                let expected =
                    (lp, lw, lk, id(left).unwrap()).cmp(&(rp, rw, rk, id(right).unwrap()));
                assert_eq!(
                    order_key(left, &fields)
                        .unwrap()
                        .cmp(&order_key(right, &fields).unwrap()),
                    expected
                );
            }
        }
        let wrong = [
            Value::Null,
            Value::String(hex::encode([0u8; 16])),
            vec![0u8; 15].into_value(),
            vec![0u8; 17].into_value(),
            vec![-1i64; 16].into_value(),
            vec![256i64; 16].into_value(),
            vec![0.0f64; 16].into_value(),
        ];
        for field in ["projection", "window"] {
            for value in &wrong {
                let mut row = rows[0].3.as_object().unwrap().clone();
                let mut body = row.get("body").unwrap().as_object().unwrap().clone();
                body.insert(field, value.clone());
                row.insert("body", body);
                assert!(
                    order_key(&Value::Object(row), &fields).is_err(),
                    "invalid {field} order value: {value:?}"
                );
            }
        }
    }
    #[test]
    fn finite_member_probe_preserves_aliases_and_absence() {
        let budget = ResourceBudget::fixed(1024 * 1024).unwrap();
        let mut nodes = SortedRows::with_budget(&budget).unwrap();
        for key in ["a", "c", "z"] {
            nodes.push(pointer(RecordId::new("entity", key))).unwrap();
        }
        let nodes = nodes.finish().unwrap().into_prepared();
        let membership = nodes.point_index(&budget).unwrap();
        let selected = SelectedPayloads {
            exact: None,
            explicit_owners: None,
            derivation: None,
            membership,
            source: SelectionSource::Session(Arc::new(Surreal::init())),
            nodes,
            aliases: SortedRows::with_budget(&budget)
                .unwrap()
                .finish()
                .unwrap()
                .into_prepared(),
            topology: None,
            budget,
        };
        assert_eq!(
            selected
                .filter_members(&[
                    RecordId::new("entity", "z"),
                    RecordId::new("entity", "b"),
                    RecordId::new("entity", "a"),
                    RecordId::new("entity", "z")
                ])
                .unwrap(),
            vec![RecordId::new("entity", "a"), RecordId::new("entity", "z")]
        );
        assert!(selected.filter_members(&[]).unwrap().is_empty());
    }
    #[tokio::test]
    async fn relation_nominations_follow_selected_membership_independently_of_foreign_payload_growth()
     {
        use lctx_model::domain::{
            Record, graph::Entity, input::Package, normalized::entities::EntityRef, value::Place,
        };
        for alias in [false, true] {
            for admitted in [false, true] {
                let mut baseline = None;
                for foreign_count in [0, 4096] {
                    let budget = ResourceBudget::fixed(8 << 20).unwrap();
                    let view = ContentHash::of(b"relation-selected-view");
                    let relation = if alias {
                        EntityRef::NAME
                    } else {
                        Package::NAME
                    };
                    let source_relation = if alias { Place::NAME } else { Package::NAME };
                    if alias {
                        assert!(
                            Entity::canonical_place_endpoint_relations()
                                .contains(&(source_relation, relation))
                        );
                    }
                    let member = RecordId::new("entity", "chosen");
                    let endpoint = RecordId::new("entity", "endpoint");
                    let middle = RecordId::new("entity", "intermediate");
                    let requested = if alias {
                        endpoint.clone()
                    } else {
                        member.clone()
                    };
                    let mut payloads = BTreeMap::new();
                    for (node, row_relation) in [
                        (member.clone(), source_relation),
                        (middle.clone(), Place::NAME),
                        (endpoint.clone(), EntityRef::NAME),
                    ] {
                        let mut row = Object::new();
                        row.insert("id", node.clone());
                        row.insert("semantic_type", row_relation);
                        row.insert("semantic_key", format!("{:?}", node.key));
                        row.insert("content", "immutable");
                        payloads.insert(node, Value::Object(row));
                    }
                    for ordinal in 0..foreign_count {
                        let node = RecordId::new("entity", format!("foreign-{ordinal:04}"));
                        let mut row = Object::new();
                        row.insert("id", node.clone());
                        row.insert("semantic_type", relation);
                        row.insert("semantic_key", format!("foreign-{ordinal:04}"));
                        row.insert("content", "foreign");
                        payloads.insert(node, Value::Object(row));
                    }
                    let edges = if alias {
                        vec![
                            (member.clone(), middle.clone()),
                            (middle.clone(), endpoint.clone()),
                            (endpoint.clone(), member.clone()),
                        ]
                    } else {
                        vec![]
                    };
                    let owned_edges = edges.clone();
                    let calls = Arc::new(std::sync::Mutex::new(Vec::new()));
                    let observed = calls.clone();
                    let hydrated = Arc::new(std::sync::Mutex::new(Vec::new()));
                    let observed_hydrated = hydrated.clone();
                    let source = SelectionSource::Fixture(Arc::new(move |query, budget| {
                        let (sql, vars) = query.into_request();
                        let nodes = vars
                            .get("nodes")
                            .or_else(|| vars.get("sources"))
                            .map(|value| Vec::<RecordId>::from_value(value.clone()).unwrap())
                            .unwrap_or_default();
                        observed.lock().unwrap().push((sql.clone(), nodes.clone()));
                        let values = if sql.starts_with("SELECT * FROM compiler_membership") {
                            assert_eq!(
                                Vec::<RecordId>::from_value(
                                    vars.get("contributors").unwrap().clone()
                                )
                                .unwrap(),
                                vec![RecordId::new("compiler_contribution", view.hex())]
                            );
                            let hit = if vars.get("nodes").is_some() {
                                nodes.contains(&member)
                            } else if let Some(keys) = vars.get("keys") {
                                Vec::<String>::from_value(keys.clone())
                                    .unwrap()
                                    .contains(&test_key(&format!("{:?}", member.key)))
                            } else {
                                true
                            };
                            let relation_matches = vars
                                .get("relation")
                                .is_none_or(|value| value == Value::String(source_relation.into()));
                            if admitted && hit && relation_matches {
                                vec![test_membership(
                                    view,
                                    member.clone(),
                                    source_relation,
                                    &format!("{:?}", member.key),
                                    "immutable",
                                )]
                            } else {
                                vec![]
                            }
                        } else if sql == "SELECT id FROM $nodes WHERE semantic_type=$relation;" {
                            nodes
                                .iter()
                                .filter(|node| {
                                    payloads
                                        .get(*node)
                                        .and_then(Value::as_object)
                                        .and_then(|row| row.get("semantic_type"))
                                        == vars.get("relation")
                                })
                                .map(|node| pointer(node.clone()))
                                .collect()
                        } else {
                            assert_eq!(
                                sql, "SELECT * FROM $nodes;",
                                "relation reads cannot enumerate the global payload universe"
                            );
                            observed_hydrated
                                .lock()
                                .unwrap()
                                .extend(nodes.iter().cloned());
                            nodes.iter().map(|node| payloads[node].clone()).collect()
                        };
                        NativeRows::owned(budget.clone(), move |sender| async move {
                            for row in values {
                                if !sender.send(row).await? {
                                    break;
                                }
                            }
                            Ok(())
                        })
                    }));
                    let selected = exact_selection(
                        source.clone(),
                        &[view],
                        source_relation,
                        &owned_edges,
                        &budget,
                    );
                    let mut rows = SelectedPayloads::candidate_rows_from(
                        source,
                        Some(selected),
                        vec![view],
                        CandidateNominations::Relation(relation.into()),
                        "entity",
                        "semantic_key",
                        &budget,
                    )
                    .unwrap();
                    let mut actual = Vec::new();
                    while let Some(row) = rows.next().await.unwrap() {
                        actual.push(id(&row).unwrap());
                    }
                    rows.drain_transport().await.unwrap();
                    drop(rows);
                    assert_eq!(
                        actual,
                        if admitted {
                            vec![requested.clone()]
                        } else {
                            vec![]
                        }
                    );
                    assert_eq!(*hydrated.lock().unwrap(), actual);
                    let work = calls.lock().unwrap().clone();
                    assert!(
                        work.iter()
                            .flat_map(|(_, nodes)| nodes)
                            .all(|node| !format!("{:?}", node.key).contains("foreign-"))
                    );
                    if let Some(baseline) = &baseline {
                        assert_eq!(
                            &work, baseline,
                            "foreign same-relation growth changes neither nominations nor exact membership/query work"
                        );
                    } else {
                        baseline = Some(work);
                    }
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
        for n in 0..512 {
            nodes
                .push(pointer(RecordId::new("entity", format!("selected-{n:04}"))))
                .unwrap();
        }
        let nodes = nodes.finish().unwrap().into_prepared();
        let membership = nodes.point_index(&retained).unwrap();
        let payload_reads = Arc::new(AtomicUsize::new(0));
        let observed = payload_reads.clone();
        let mut selected = SelectedPayloads {
            exact: None,
            explicit_owners: None,
            derivation: None,
            nodes,
            membership,
            aliases: SortedRows::with_budget(&retained)
                .unwrap()
                .finish()
                .unwrap()
                .into_prepared(),
            topology: None,
            budget: retained.clone(),
            source: SelectionSource::Fixture(Arc::new(move |query, budget| {
                let (sql, vars) = query.into_request();
                let values = if sql == "NOMINATE;" {
                    vec![
                        pointer(RecordId::new("entity", "selected-0001")),
                        pointer(RecordId::new("entity", "outside-view")),
                    ]
                } else if sql
                    .starts_with("SELECT * FROM compiler_membership WITH INDEX contribution_nodes")
                {
                    let nodes =
                        Vec::<RecordId>::from_value(vars.get("nodes").unwrap().clone()).unwrap();
                    assert!(nodes.len() <= WINDOW);
                    nodes
                        .into_iter()
                        .filter(|node| node == &RecordId::new("entity", "selected-0001"))
                        .map(|node| {
                            test_membership(
                                ContentHash::of(b"exact-view"),
                                node,
                                "packages",
                                "key",
                                "hash",
                            )
                        })
                        .collect()
                } else if sql
                    .starts_with("SELECT * FROM compiler_membership WITH INDEX contribution_rows")
                {
                    vec![test_membership(
                        ContentHash::of(b"exact-view"),
                        RecordId::new("entity", "selected-0001"),
                        "packages",
                        "key",
                        "hash",
                    )]
                } else {
                    assert_eq!(
                        sql, "SELECT * FROM $nodes;",
                        "sparse candidates must never open a complete selected payload scan"
                    );
                    let nodes =
                        Vec::<RecordId>::from_value(vars.get("nodes").unwrap().clone()).unwrap();
                    assert_eq!(nodes, vec![RecordId::new("entity", "selected-0001")]);
                    observed.fetch_add(nodes.len(), Ordering::SeqCst);
                    let mut row = Object::new();
                    row.insert("id", nodes[0].clone());
                    row.insert("semantic_key", "key");
                    vec![Value::Object(row)]
                };
                NativeRows::owned(budget.clone(), move |sender| async move {
                    for row in values {
                        if !sender.send(row).await? {
                            break;
                        }
                    }
                    Ok(())
                })
            })),
        };
        let mut exact = exact_selection(
            selected.source.clone(),
            &[ContentHash::of(b"exact-view")],
            "packages",
            &[],
            &retained,
        );
        selected.exact = exact.exact.take();
        selected.topology = exact.topology.take();
        drop(exact);
        let baseline = retained.reserved();
        for _ in 0..3 {
            let query = crate::prepared::PreparedQuery::new(
                Variables::new(),
                vec![],
                vec!["NOMINATE".into()],
            )
            .unwrap();
            let mut rows = SelectedPayloads::candidate_rows_inner(
                selected.source.clone(),
                Some(selected.clone()),
                vec![ContentHash::of(b"exact-view")],
                query,
                "entity",
                "semantic_key",
                &request,
            )
            .unwrap();
            assert!(rows.next().await.unwrap().is_some());
            assert!(rows.next().await.unwrap().is_none());
            rows.drain_transport().await.unwrap();
            drop(rows);
            assert_eq!(request.reserved(), 0);
            assert_eq!(retained.reserved(), baseline);
        }
        assert_eq!(
            payload_reads.load(Ordering::SeqCst),
            3,
            "512 selected identities do not cause 512 payload hydrations per point request"
        );
        let refused = ResourceBudget::scoped(&retained, 1).unwrap();
        let query =
            crate::prepared::PreparedQuery::new(Variables::new(), vec![], vec!["NOMINATE".into()])
                .unwrap();
        let mut rows = SelectedPayloads::candidate_rows_inner(
            selected.source.clone(),
            Some(selected.clone()),
            vec![ContentHash::of(b"exact-view")],
            query,
            "entity",
            "id",
            &refused,
        )
        .unwrap();
        assert!(rows.next().await.is_err());
        rows.drain_transport().await.unwrap();
        drop(rows);
        assert_eq!(
            payload_reads.load(Ordering::SeqCst),
            3,
            "scratch refusal happens before payload hydration"
        );
        assert_eq!(refused.reserved(), 0);
        let borrower = selected.clone();
        drop(selected);
        assert_eq!(retained.reserved(), baseline);
        drop(borrower);
        assert_eq!(retained.reserved(), 0);
    }

    #[tokio::test]
    async fn sparse_membership_batches_shared_cyclic_ancestry_and_retains_per_node_witnesses() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let one = ContentHash::of(b"view-one");
        let two = ContentHash::of(b"view-two");
        let reverse = Arc::new(AtomicUsize::new(0));
        let calls = reverse.clone();
        let source = SelectionSource::Fixture(Arc::new(move |query, budget| {
            let (sql, vars) = query.into_request();
            assert!(
                sql.starts_with("SELECT * FROM compiler_membership"),
                "reverse aliases use the prepared target index: {sql}"
            );
            let nodes = if let Some(nodes) = vars.get("nodes") {
                Vec::<RecordId>::from_value(nodes.clone()).unwrap()
            } else {
                let keys = Vec::<String>::from_value(vars.get("keys").unwrap().clone()).unwrap();
                ["c", "d"]
                    .into_iter()
                    .map(|key| RecordId::new("entity", key))
                    .filter(|node| keys.contains(&test_key(&format!("{:?}", node.key))))
                    .collect()
            };
            let owners =
                Vec::<RecordId>::from_value(vars.get("contributors").unwrap().clone()).unwrap();
            let values = [(one, "d"), (two, "c")]
                .into_iter()
                .filter(|(view, _)| {
                    owners.contains(&RecordId::new("compiler_contribution", view.hex()))
                })
                .filter_map(|(view, key)| {
                    let node = RecordId::new("entity", key);
                    nodes.contains(&node).then(|| {
                        test_membership(
                            view,
                            node.clone(),
                            "places",
                            &format!("{:?}", node.key),
                            "immutable",
                        )
                    })
                })
                .collect::<Vec<_>>();
            let _ = &calls;
            NativeRows::owned(budget.clone(), move |sender| async move {
                for row in values {
                    if !sender.send(row).await? {
                        break;
                    }
                }
                Ok(())
            })
        }));
        let requested = ["a", "b", "missing", "a"].map(|key| RecordId::new("entity", key));
        let aliases = [("c", "a"), ("c", "b"), ("d", "c"), ("a", "d")].map(|(source, target)| {
            (
                RecordId::new("entity", source),
                RecordId::new("entity", target),
            )
        });
        let selected = exact_selection(source, &[one, two], "places", &aliases, &budget);
        let answers = selected
            .membership_answers(&requested, &budget)
            .await
            .unwrap();
        assert_eq!(answers.answers.len(), 3);
        assert!(answers.contains_all(&requested[..2]));
        assert!(!answers.contains(&requested[2]));
        for answer in &answers.answers[..2] {
            assert_eq!(answer.witnesses.len(), 2);
            for witness in &answer.witnesses {
                assert_eq!(witness.ancestry.first(), Some(&answer.requested));
                assert_eq!(witness.ancestry.last(), Some(&witness.member));
                assert_eq!(
                    witness.ancestry.iter().collect::<BTreeSet<_>>().len(),
                    witness.ancestry.len(),
                    "cycles cannot duplicate an ancestry node"
                );
                assert_eq!(witness.relation, "places");
            }
        }
        assert_eq!(
            reverse.load(Ordering::SeqCst),
            0,
            "shared cyclic ancestry uses only the retained exact alias inventory"
        );
        assert!(budget.reserved() > 0);
        drop(answers);
        drop(selected);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn relevant_nominal_conflict_is_rejected_before_any_prefix_is_exposed() {
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let mut nodes = SortedRows::with_budget(&budget).unwrap();
        for key in ["a", "z"] {
            nodes.push(pointer(RecordId::new("entity", key))).unwrap();
        }
        let nodes = nodes.finish().unwrap().into_prepared();
        let membership = nodes.point_index(&budget).unwrap();
        let source = SelectionSource::Fixture(Arc::new(|query, budget| {
            let (sql, _) = query.into_request();
            let values = if sql == "NOMINATE;" {
                ["a", "z"]
                    .into_iter()
                    .map(|key| pointer(RecordId::new("entity", key)))
                    .collect()
            } else {
                assert_eq!(sql, "SELECT * FROM $nodes;");
                ["a", "z"]
                    .into_iter()
                    .map(|key| {
                        let mut row = Object::new();
                        row.insert("id", RecordId::new("entity", key));
                        row.insert("semantic_key", "same-nominal-id");
                        row.insert("content", key);
                        Value::Object(row)
                    })
                    .collect::<Vec<_>>()
            };
            NativeRows::owned(budget.clone(), move |sender| async move {
                for row in values {
                    if !sender.send(row).await? {
                        break;
                    }
                }
                Ok(())
            })
        }));
        let selected = SelectedPayloads {
            exact: None,
            explicit_owners: None,
            derivation: None,
            source,
            nodes,
            membership,
            aliases: SortedRows::with_budget(&budget)
                .unwrap()
                .finish()
                .unwrap()
                .into_prepared(),
            topology: None,
            budget: budget.clone(),
        };
        let query =
            crate::prepared::PreparedQuery::new(Variables::new(), vec![], vec!["NOMINATE".into()])
                .unwrap();
        let mut rows = selected
            .candidate_rows(query, "entity", "semantic_key", &budget)
            .unwrap();
        assert!(
            rows.next().await.is_err(),
            "a valid first payload must not hide a later conflicting nominal variant"
        );
        rows.drain_transport().await.unwrap();
        drop(rows);
        drop(selected);
        assert_eq!(budget.reserved(), 0);
    }

    #[tokio::test]
    async fn singleton_membership_rejects_un_nominated_nominal_revision_but_accepts_shared_witness()
    {
        let one = ContentHash::of(b"nominal-one");
        let two = ContentHash::of(b"nominal-two");
        for competing in [false, true] {
            let budget = ResourceBudget::fixed(8 << 20).unwrap();
            let source = SelectionSource::Fixture(Arc::new(move |query, budget| {
                let (sql, vars) = query.into_request();
                assert!(sql.starts_with("SELECT * FROM compiler_membership"));
                let owners =
                    Vec::<RecordId>::from_value(vars.get("contributors").unwrap().clone()).unwrap();
                let values = [one, two]
                    .into_iter()
                    .filter(|view| {
                        owners.contains(&RecordId::new("compiler_contribution", view.hex()))
                    })
                    .filter_map(|view| {
                        let second = view == two;
                        let key = if second && competing {
                            "revision-two"
                        } else {
                            "revision-one"
                        };
                        let node = RecordId::new("entity", key);
                        let hit = if let Some(nodes) = vars.get("nodes") {
                            Vec::<RecordId>::from_value(nodes.clone())
                                .unwrap()
                                .contains(&node)
                        } else {
                            assert!(sql.contains("WITH INDEX contribution_rows"));
                            assert_eq!(
                                vars.get("relation"),
                                Some(&Value::String("source_artifacts".into()))
                            );
                            assert_eq!(
                                Vec::<String>::from_value(vars.get("keys").unwrap().clone())
                                    .unwrap(),
                                vec![test_key("same-nominal")]
                            );
                            true
                        };
                        hit.then(|| {
                            test_membership(
                                view,
                                node,
                                "source_artifacts",
                                "same-nominal",
                                if second && competing {
                                    "content-two"
                                } else {
                                    "content-one"
                                },
                            )
                        })
                    })
                    .collect::<Vec<_>>();
                NativeRows::owned(budget.clone(), move |sender| async move {
                    for row in values {
                        if !sender.send(row).await? {
                            break;
                        }
                    }
                    Ok(())
                })
            }));
            let selected = exact_selection(source, &[one, two], "source_artifacts", &[], &budget);
            let result = selected
                .membership_answers(&[RecordId::new("entity", "revision-one")], &budget)
                .await;
            if competing {
                assert!(
                    result.is_err(),
                    "singleton physical nominations cannot hide another selected revision"
                );
            } else {
                let answers = result.unwrap();
                assert_eq!(answers.answers.len(), 1);
                assert_eq!(answers.answers[0].witnesses.len(), 2);
                assert!(answers.answers[0].selected);
                drop(answers);
            }
            drop(selected);
            assert_eq!(budget.reserved(), 0);
        }
    }
    #[tokio::test]
    async fn selected_compiler_block_hydrates_only_exact_slots_and_refuses_incomplete_or_foreign_sources()
     {
        use lctx_model::domain::{self as d, Record};
        let budget = ResourceBudget::fixed(32 << 20).unwrap();
        let row = |ordinal| {
            let typed = d::analytics::QualityStep {
                run: serde_json::from_value(serde_json::to_value([7u8; 16]).unwrap()).unwrap(),
                ordinal,
                value: d::FiniteF64::new(0.25).unwrap(),
            };
            let view = crate::codec::views(std::slice::from_ref(&typed))
                .unwrap()
                .remove(0);
            let canonical = serde_json::to_vec(&view.body).unwrap();
            let content = ContentHash::of(&canonical);
            crate::adapter::physical_row(
                crate::loader::payload_id(
                    "compiler_record",
                    d::analytics::QualityStep::NAME,
                    typed.id().bytes(),
                    content,
                )
                .unwrap(),
                content,
                canonical,
                None,
                view,
            )
            .unwrap()
        };
        let first = row(0);
        let second = row(1);
        let owner = ContentHash::of(b"selected actual block source");
        let packet = crate::compiler_blocks::PreparedBlocks::prepare(
            owner,
            vec![first.clone(), second],
            &budget,
        )
        .unwrap();
        let node = id(&first).unwrap();
        let locator = packet
            .locators()
            .iter()
            .find(|row| {
                row.as_object().unwrap().get("node") == Some(&Value::RecordId(node.clone()))
            })
            .unwrap()
            .clone();
        for mode in 0..9 {
            let block = packet.blocks()[0].clone();
            let locator = locator.clone();
            let extra = packet
                .locators()
                .iter()
                .find(|row| {
                    row.as_object().unwrap().get("node") != Some(&Value::RecordId(node.clone()))
                })
                .unwrap()
                .clone();
            let calls = Arc::new(std::sync::Mutex::new(Vec::new()));
            let observed = calls.clone();
            let expected_node = node.clone();
            let source = SelectionSource::Fixture(Arc::new(move |query, budget| {
                let (sql, bindings) = query.into_request();
                observed.lock().unwrap().push(sql.clone());
                let values = if sql
                    .starts_with("SELECT * FROM compiler_membership WITH INDEX contribution_nodes")
                {
                    assert_eq!(
                        Vec::<RecordId>::from_value(bindings.get("nodes").unwrap().clone())
                            .unwrap(),
                        vec![expected_node.clone()]
                    );
                    match mode {
                        1 => vec![locator.clone(), extra.clone()],
                        3 => vec![],
                        6 => vec![locator.clone(), locator.clone()],
                        _ => vec![locator.clone()],
                    }
                } else {
                    assert_eq!(
                        sql, "SELECT * FROM $blocks;",
                        "virtual logical IDs must never be dereferenced"
                    );
                    assert!(
                        Vec::<RecordId>::from_value(bindings.get("blocks").unwrap().clone())
                            .unwrap()
                            .iter()
                            .all(|id| id.table.as_str() == "compiler_block")
                    );
                    let mut block = block.clone();
                    if mode == 2 {
                        let Value::Object(ref mut object) = block else {
                            unreachable!()
                        };
                        object.insert("content", ContentHash::of(b"corrupt").hex());
                    }
                    match mode {
                        4 => vec![block.clone(), block],
                        5 => vec![],
                        _ => vec![block],
                    }
                };
                let terminal_failure = (mode == 7 && sql.starts_with("SELECT * FROM $blocks"))
                    || (mode == 8 && sql.starts_with("SELECT * FROM compiler_membership"));
                NativeRows::owned(budget.clone(), move |sender| async move {
                    for row in values {
                        if !sender.send(row).await? {
                            break;
                        }
                    }
                    if terminal_failure {
                        return Err(ModelError::Conflict(
                            "mechanical selected source late failure",
                        ));
                    }
                    Ok(())
                })
            }));
            let selected = exact_selection(
                source,
                &[owner],
                d::analytics::QualityStep::NAME,
                &[],
                &budget,
            );
            let result = selected
                .compiler_rows(std::slice::from_ref(&node), &budget)
                .await;
            if mode == 0 {
                let (rows, _charge) = result.unwrap();
                assert_eq!(
                    rows,
                    vec![first.clone()],
                    "unselected container siblings confer no membership"
                );
            } else {
                assert!(
                    result.is_err(),
                    "mode {mode} must not publish a successful selected prefix"
                );
            }
            assert_eq!(
                calls.lock().unwrap().len(),
                if mode == 1 || mode == 3 || mode == 6 || mode == 8 {
                    1
                } else {
                    2
                }
            );
        }
    }
}
