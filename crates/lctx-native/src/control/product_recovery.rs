//! Exact terminal product recovery. This earns protected physical reads only;
//! compiler storage separately executes current producer and semantic admission.
use super::*;
use crate::installed_view::{self, HeldInstalledView};
use lctx_model::domain::{
    charged::StateCharge,
    completed::{CompletedContribution, CompletedView},
    completion::{Completion, complete},
    resources::ResourceBudget,
};
use std::collections::{BTreeMap, BTreeSet};
use surrealdb::types::RecordIdKey;

/// Private fields retain the actual product and exact native namespace owners.
/// Neither this type nor a deserialized completion establishes semantic eligibility.
pub struct HeldTerminalProduct {
    client: Arc<Surreal<Client>>,
    pin: Option<Arc<ReaderPin>>,
    product: ExpectedRecord,
    contribution: ExpectedRecord,
    descriptor: CompletedContribution,
    generation: ContentHash,
    validity_epoch: i64,
    views: BTreeMap<ContentHash, Arc<HeldInstalledView>>,
    contributors: BTreeMap<ContentHash, CompletedContribution>,
    factory_lease: FactoryLease,
    closed: bool,
    cancellation: Arc<std::sync::atomic::AtomicBool>,
    retry: RetryCancellation,
    _charge: StateCharge,
}
impl Drop for HeldTerminalProduct {
    fn drop(&mut self) {
        if !self.closed
            && let Some(pin) = &self.pin
        {
            pin.retain_unknown();
        }
    }
}
// Factory and method cancellation must preserve the pin before a public cap or
// caller-owned drainage guard exists. A terminal logical failure may release;
// cancellation, panic and an unconfirmed result cannot.
struct RecoveryPinTerminal {
    pin: Arc<ReaderPin>,
    confirmed: bool,
}
fn live_borrowers(message: &'static str) -> ModelError {
    let completion = Completion {
        local: lctx_model::domain::completion::LocalState::Outstanding,
        ..Completion::default()
    };
    complete::<()>(Err(ModelError::Conflict(message)), completion).unwrap_err()
}
impl RecoveryPinTerminal {
    fn new(pin: Arc<ReaderPin>) -> Self {
        Self {
            pin,
            confirmed: false,
        }
    }
    fn observe<T>(&mut self, result: &Result<T, ModelError>) {
        self.confirmed = result
            .as_ref()
            .err()
            .is_none_or(|error| error.permits_storage_cleanup());
    }
}
impl Drop for RecoveryPinTerminal {
    fn drop(&mut self) {
        if !self.confirmed {
            self.pin.retain_unknown();
        }
    }
}

impl HeldTerminalProduct {
    fn pin(&self) -> &Arc<ReaderPin> {
        self.pin
            .as_ref()
            .expect("terminal owner pin exists outside atomic close transfer")
    }
    /// Consume the source owner only after every borrowed cap/reader and accepted
    /// query has drained. A failed or interrupted close preserves exact ownership.
    pub async fn close(self: Arc<Self>) -> Result<(), ModelError> {
        let factory = self.factory_lease.factory().clone();
        self.close_inner()
            .await
            .map_err(|error| factory.handback(error))
    }
    async fn close_inner(self: Arc<Self>) -> Result<(), ModelError> {
        let mut owner = match Arc::try_unwrap(self) {
            Ok(owner) => owner,
            Err(owner) => {
                owner.retain_unknown();
                return Err(live_borrowers(
                    "terminal product still has live source owners",
                ));
            }
        };
        // Atomic consumption prevents a Weak upgrade racing a count-then-clear
        // check. A retained cap refuses close before any pin/session release.
        for view in std::mem::take(&mut owner.views).into_values() {
            if Arc::try_unwrap(view).is_err() {
                owner.retain_unknown();
                return Err(live_borrowers(
                    "terminal product still has live cap borrowers",
                ));
            }
        }
        let pin = owner.pin.take().expect("terminal source pin transfer");
        match Arc::try_unwrap(pin) {
            Ok(pin) => owner.pin = Some(Arc::new(pin)),
            Err(pin) => {
                owner.pin = Some(pin);
                owner.retain_unknown();
                return Err(live_borrowers(
                    "terminal product still has live reader borrowers",
                ));
            }
        }
        if let Err(error) = owner.pin().release().await {
            owner.retain_unknown();
            let completion = Completion {
                local: lctx_model::domain::completion::LocalState::Outstanding,
                ..Completion::default()
            };
            return complete(Err(error), completion);
        }
        owner.factory_lease.close_session().await?;
        owner.closed = true;
        // Only actual joined pin and session completion may finish this source's
        // independent process receipt. Drop/Unknown leaves it started.
        owner.factory_lease.complete().await
    }
    pub fn descriptor(&self) -> &CompletedContribution {
        &self.descriptor
    }
    pub fn contribution(&self) -> &ExpectedRecord {
        &self.contribution
    }
    pub fn views(&self) -> &BTreeMap<ContentHash, Arc<HeldInstalledView>> {
        &self.views
    }
    pub fn contributors(&self) -> &BTreeMap<ContentHash, CompletedContribution> {
        &self.contributors
    }
    pub fn generation(&self) -> ContentHash {
        self.generation
    }
    /// Borrow the actual forked source scope. This does not transfer its pin,
    /// session or finalization authority to a consuming read adapter.
    pub fn recovery_scope(&self) -> &Arc<RecoveryScope> {
        self.factory_lease.recovery_scope()
    }
    pub fn retain_unknown(&self) {
        self.pin().retain_unknown();
    }
    pub fn reader(&self, budget: &ResourceBudget) -> crate::NativeReader<()> {
        crate::NativeReader::private_pinned(self.client.clone(), self.pin().clone())
            .with_budget(budget)
            .with_read_cancellation(self.cancellation.clone())
    }
    /// Compare current captured source metadata with actual backing retained by
    /// this product's explicitly selected physical contributors. A missing row
    /// is not a completed-empty assertion; callers may use only returned matches
    /// as temporary reference targets and must reject missing referenced IDs.
    pub async fn verify_captured_row<R: lctx_model::domain::Record>(
        &self,
        row: &R,
        budget: &ResourceBudget,
    ) -> Result<bool, ModelError> {
        use crate::adapter::GraphRow;
        if !matches!(
            R::NAME,
            "packages"
                | "releases"
                | "input_revisions"
                | "input_origins"
                | "input_acquisitions"
                | "corpus_libraries"
                | "input_distributions"
                | "distribution_verifications"
                | "environment_fingerprints"
                | "source_artifacts"
                | "artifact_ownership"
                | "unowned_artifacts"
                | "derived_artifacts"
                | "artifact_uses"
        ) {
            return Err(ModelError::Conflict("terminal capture companion family"));
        }
        row.validate()?;
        let mut charge = StateCharge::new(budget, "terminal captured canonical comparison");
        charge.grow(
            std::mem::size_of::<R>()
                .saturating_add(lctx_model::domain::HeapSize::heap_bytes(row))
                .saturating_mul(32)
                .saturating_add(65536),
        )?;
        let adapter = crate::adapter::select(R::NAME)?;
        let batch = R::encode(std::slice::from_ref(row))?;
        let view = crate::codec::views_with_adapter(std::slice::from_ref(row), adapter)?
            .pop()
            .ok_or(ModelError::Schema("terminal captured body"))?;
        let graph = adapter
            .graph(&batch)?
            .pop()
            .ok_or(ModelError::Schema("terminal captured graph decode"))?;
        let (id, content, canonical, kind) = match graph {
            Some(GraphRow::Entity(entity)) => (
                crate::loader::entity_payload_id(&entity)?,
                entity.content(),
                serde_json::to_vec(&entity).map_err(ModelError::codec)?,
                Some((entity.kind() as i64, entity.subtype())),
            ),
            Some(GraphRow::Assertion(assertion)) => (
                crate::loader::assertion_payload_id(&assertion)?,
                assertion.content(),
                serde_json::to_vec(&assertion).map_err(ModelError::codec)?,
                Some((assertion.kind as i64, None)),
            ),
            None => {
                let canonical = serde_json::to_vec(&view.body).map_err(ModelError::codec)?;
                let content = ContentHash::of(&canonical);
                (
                    crate::loader::payload_id(
                        "compiler_record",
                        R::NAME,
                        row.id().bytes(),
                        content,
                    )?,
                    content,
                    canonical,
                    None,
                )
            }
        };
        let expected = crate::adapter::physical_row(id.clone(), content, canonical, kind, view)?;
        charge.grow(crate::loader::native_bytes(&expected).saturating_mul(4))?;
        let mut owners = BTreeSet::new();
        for view in self.views.values() {
            for mapping in &view.completion().mappings {
                charge.grow(512)?;
                owners.insert(RecordId::new(
                    "compiler_contribution",
                    mapping.physical.hex(),
                ));
            }
        }
        let mut pin_terminal = RecoveryPinTerminal::new(self.pin().clone());
        let reader = self.reader(budget);
        let result=async {
            self.pin().ensure_live()?;
            let mut after=Value::None;let mut found=false;
            loop {
                let mut variables=Variables::new();variables.insert("capture_owners",owners.iter().cloned().collect::<Vec<_>>());
                variables.insert("capture_relation",R::NAME);variables.insert("capture_key",hex::encode(row.id().bytes()));variables.insert("capture_after",after.clone());
                let rows:Vec<Object>=reader.query_native("SELECT id,object AS address,object.* AS stored FROM native_hold WITH INDEX owner_holds WHERE owner IN $capture_owners AND object.semantic_type=$capture_relation AND object.semantic_key=$capture_key AND ($capture_after=NONE OR id>$capture_after) ORDER BY id LIMIT 1",variables).await?;
                let [actual]=rows.as_slice()else{if !rows.is_empty(){return Err(ModelError::Conflict("terminal captured bounded backing page"));}break;};
                after=actual.get("id").cloned().ok_or(ModelError::Schema("terminal captured backing cursor"))?;
                if actual.get("address")!=Some(&Value::RecordId(id.clone())) || actual.get("stored")!=Some(&expected){return Err(ModelError::Conflict("terminal captured native payload differs"));}
                with_retry_cancellation(self.retry.clone(),self.pin().protect(id.clone())).await?;
                let protected=held::inspect_expected_read(&reader,id.clone(),budget).await?.ok_or(ModelError::Conflict("terminal captured native backing disappeared"))?;
                if protected.row()!=expected{return Err(ModelError::Conflict("terminal captured protected bytes changed"));}
                found=true;
            }
            Ok(found)
        }.await;
        let mut terminal = Completion::default();
        terminal.step(
            "terminal captured backing reader close",
            reader.close().await,
        );
        let result = complete(result, terminal);
        pin_terminal.observe(&result);
        result
    }
    /// Repeat actual guard/bytes and installation checks; an old original counter
    /// or a matching logical contributor in another namespace cannot satisfy them.
    pub async fn ensure_current(&self, budget: &ResourceBudget) -> Result<(), ModelError> {
        let mut pin_terminal = RecoveryPinTerminal::new(self.pin().clone());
        let reader = self.reader(budget);
        let result = async {
            self.pin().ensure_live()?;
            for original in [&self.product, &self.contribution] {
                let actual =
                    held::inspect_expected_read(&reader, original.record().clone(), budget)
                        .await?
                        .ok_or(ModelError::Conflict(
                            "terminal product held record disappeared",
                        ))?;
                if actual.incarnation() != original.incarnation() || actual.row() != original.row()
                {
                    return Err(ModelError::Conflict(
                        "terminal product held bytes/incarnation changed",
                    ));
                }
            }
            check_installation(&reader, self.generation, self.validity_epoch).await
        }
        .await;
        let mut terminal = Completion::default();
        terminal.step(
            "terminal product current reader close",
            reader.close().await,
        );
        let result = complete(result, terminal);
        pin_terminal.observe(&result);
        result
    }
}
fn terminal_source(source: &Object, generation: ContentHash) -> Result<(), ModelError> {
    if authorization::hash_field(source, "generation")? != generation
        || authorization::int_field(source, "epoch")? <= 0
        || !matches!(source.get("state"),Some(Value::String(state)) if matches!(state.as_str(),"closed"|"abandoned"|"maintenance_fenced"|"frozen"))
    {
        return Err(ModelError::Conflict(
            "retained product producer is not terminal/current",
        ));
    }
    Ok(())
}
async fn installation(reader: &crate::NativeReader<()>) -> Result<(ContentHash, i64), ModelError> {
    let rows: Vec<Object> = reader
        .query_native(
            "SELECT generation,validity_epoch FROM native_installation:current",
            Variables::new(),
        )
        .await?;
    let [row] = rows.as_slice() else {
        return Err(ModelError::Conflict(
            "terminal product installation cardinality",
        ));
    };
    let epoch = authorization::int_field(row, "validity_epoch")?;
    if epoch <= 0 {
        return Err(ModelError::Conflict("terminal product installation epoch"));
    }
    Ok((authorization::hash_field(row, "generation")?, epoch))
}
async fn check_installation(
    reader: &crate::NativeReader<()>,
    generation: ContentHash,
    epoch: i64,
) -> Result<(), ModelError> {
    if installation(reader).await? != (generation, epoch) {
        return Err(ModelError::Conflict(
            "terminal product installation changed",
        ));
    }
    Ok(())
}

/// The supplied address is discovery data. This constructor independently protects
/// and reads the actual product, singleton output witnesses and full selected closure.
pub async fn hold_terminal_product(
    client: Arc<Surreal<Client>>,
    product: RecordId,
    request: ContentHash,
    budget: &ResourceBudget,
) -> Result<HeldTerminalProduct, ModelError> {
    if product.table.as_str() != "native_product" {
        return Err(ModelError::Conflict("terminal product family"));
    }
    let retry = RetryCancellation::default();
    let cancellation = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let parent = capture_recovery_scope().ok_or(ModelError::Conflict(
        "terminal product requires source recovery owner",
    ))?;
    if !Arc::ptr_eq(parent.client(), &client) {
        return Err(ModelError::Conflict(
            "terminal product source session differs",
        ));
    }
    let factory = parent.factory()?;
    let (stop, reads) = (retry.clone(), cancellation.clone());
    let signals = Arc::new(crate::signal::SignalRegistration::with_receipt(
        "native terminal product source",
        move || {
            stop.cancel();
            reads.store(true, std::sync::atomic::Ordering::Release);
        },
        factory.binding().clone(),
    )?);
    // SDK clone creates a distinct authenticated session. Receiver abandonment
    // cannot invalidate this source owner while a scanner/candidate retains it.
    let (recovery, _signals, lease) =
        parent.fork_session_with_signals(budget, retry.clone(), signals)?;
    let client = recovery.client().clone();
    let pin = match with_recovery_scope(recovery.clone(), ReaderPin::acquire(client.clone(), &[]))
        .await
    {
        Ok(pin) => pin,
        Err(error) => {
            if !error.permits_storage_cleanup() {
                return Err(factory.handback(error));
            }
            let mut terminal = Completion::default();
            let closed = lease.close_session().await;
            if closed.is_ok() {
                terminal.step(
                    "terminal failed acquisition source receipt",
                    lease.complete().await,
                );
            }
            terminal.step("terminal acquisition session close", closed);
            return complete(Err(error), terminal).map_err(|error| factory.handback(error));
        }
    };
    let mut pin_terminal = RecoveryPinTerminal::new(pin.clone());
    let reader = crate::NativeReader::private_pinned(client.clone(), pin.clone())
        .with_budget(budget)
        .with_read_cancellation(cancellation.clone());
    let result=with_recovery_scope(recovery.clone(),async {
        pin.protect(product.clone()).await?;
        let product=held::inspect_expected_read(&reader,product,budget).await?
            .ok_or(ModelError::Conflict("terminal product disappeared before protection"))?;
        let row=product.row().as_object().ok_or(ModelError::Schema("terminal product row"))?;
        if row.get("request")!=Some(&Value::String(request.hex())) || row.get("released")!=Some(&Value::Bool(false)) {
            return Err(ModelError::Conflict("terminal product request/retention changed"));
        }
        let Some(Value::RecordId(address))=row.get("contribution")else{return Err(ModelError::Schema("terminal product contributor"));};
        if address.table.as_str()!="compiler_contribution"{return Err(ModelError::Schema("terminal product contributor family"));}
        pin.protect(address.clone()).await?;
        let contribution=held::inspect_expected_read(&reader,address.clone(),budget).await?
            .ok_or(ModelError::Conflict("terminal product contributor missing"))?;
        crate::completed_rows::validate_state_row("compiler_contribution",contribution.row())?;
        let mut charge=StateCharge::new(budget,"terminal product exact descriptors");
        charge.grow(crate::loader::native_bytes(contribution.row()).saturating_mul(32).saturating_add(65536))?;
        let descriptor:CompletedContribution=installed_view::audit::descriptor(contribution.row())?;
        let logical=descriptor.identity()?;
        let stored=contribution.row().as_object().ok_or(ModelError::Schema("terminal contribution row"))?;
        if stored.get("completed")!=Some(&Value::Bool(true)) || stored.get("logical")!=Some(&Value::String(logical.hex())) {
            return Err(ModelError::Conflict("terminal product contributor not complete"));
        }
        let Some(Value::RecordId(source))=stored.get("attempt")else{return Err(ModelError::Schema("terminal contributor original attempt"));};
        let (generation,validity_epoch)=installation(&reader).await?;
        let mut vars=Variables::new();vars.insert("source",source.clone());
        let sources:Vec<Object>=reader.query_native("SELECT * FROM $source",vars).await?;
        let [source_row]=sources.as_slice()else{return Err(ModelError::Conflict("terminal product original owner cardinality"));};
        terminal_source(source_row,generation)?;
        let RecordIdKey::String(source_key)=&source.key else{return Err(ModelError::Schema("terminal product source identity"));};
        if product.record()!=&RecordId::new("native_product",format!("{}_{}",request.hex(),source_key)) {
            return Err(ModelError::Conflict("terminal product original owner address"));
        }
        crate::completed_state::require_hold(&reader,product.record(),contribution.record()).await?;
        let mut variables=Variables::new();variables.insert("product",product.record().clone());
        let mut stream=reader.stream_prepared(crate::prepared::PreparedQuery::new(variables,vec![],vec!["SELECT VALUE object FROM native_hold WITH INDEX owner_holds WHERE owner=$product AND record::table(object)='native_completed_state' ORDER BY id".into()])?)?;
        let result=async{let mut parent=None;while let Some(value)=stream.next().await?{let Value::RecordId(id)=value else{return Err(ModelError::Schema("terminal compound boundary pointer"));};if parent.replace(id).is_some(){return Err(ModelError::Conflict("terminal product multiple compound boundaries"));}}parent.ok_or(ModelError::Conflict("terminal product compound boundary missing"))}.await;
        let mut terminal=Completion::default();terminal.step("terminal compound pointer EOF/drain",stream.drain_transport().await);let parent=complete(result,terminal)?;
        let expected=held::inspect_expected_read(&reader,parent.clone(),budget).await?.ok_or(ModelError::Conflict("terminal compound parent missing"))?;
        let data=crate::completed_state::FrozenCompletion::from_row(expected.row(),true)?;
        if data.record()?!=parent || data.generation!=generation || data.validity_epoch!=validity_epoch || data.attempt.hex()!=*source_key{return Err(ModelError::Conflict("terminal compound source/context"));}
        super::live_view::transfer_expected(&pin,&expected,budget).await?;
        crate::completed_state::require_hold(&reader,product.record(),&parent).await?;
        let inventory=crate::completed_state::inspect_inventory(&reader,&data,budget).await?;
        let audit=installed_view::InstallationAudit::new(client.clone(),budget).await?;
        let acquired=async{
            let mut inspections=installed_view::audit::ClosureInspectionScope::new(&audit,budget)?;
            let mut views=BTreeMap::new();let mut contributors=BTreeMap::new();let mut families=installed_view::audit::LogicalFamilyInputs::new(budget)?;let mut outputs=BTreeSet::new();
            for reference in inventory.references(){
                let installed=&reference.selector;
                let closure=installed_view::audit::inspect_closure_scoped(&reader,&installed.view,&installed.mappings,budget,&audit,&mut inspections).await?;
                let full=inventory.bindings().is_empty() || inventory.bindings().iter().any(|binding|closure.logical.views().contains_key(&binding.view.identity));
                let aliases=installed_view::audit::assure_boundary_closure(&reader,&closure,budget,&audit,&mut inspections,full).await?;
                if full {families.include_closure(&reader,&closure,&aliases,budget,&audit)?;}
                let (_,content)=installed_view::audit::selected_prepared_members_with_content(&closure,&installed.view,budget,&audit).await?;
                if content!=installed.membership_content{return Err(ModelError::Conflict("terminal compound selector complete membership"));}
                if let Some(output)=descriptor.outputs.get(&installed.view.relation){
                    let singleton=CompletedView::new(installed.view.relation.clone(),BTreeSet::from([logical]),output.rows)?;
                    if installed.view==singleton
                        && (!installed.mappings.iter().any(|mapping|mapping.view==installed.view.identity && mapping.logical==logical && RecordId::new("compiler_contribution",mapping.physical.hex())==*contribution.record() && mapping.incarnation==contribution.incarnation()) || !outputs.insert(installed.view.relation.clone())){return Err(ModelError::Conflict("terminal exact singleton output selector"));}
                }
                for (id,owner) in closure.logical.contributions(){
                    if let Some(previous)=contributors.get(id){if previous!=owner{return Err(ModelError::Conflict("terminal full contributor descriptor collision"));}}else{
                        charge.grow(std::mem::size_of::<(ContentHash,CompletedContribution)>().saturating_add(installed_view::audit::contribution_heap(owner)).saturating_add(64))?;
                        contributors.insert(*id,owner.clone());
                    }
                }
                let expected=held::inspect_expected_read(&reader,installed.record()?,budget).await?.ok_or(ModelError::Conflict("terminal selected view disappeared"))?;
                if expected.row()!=installed.value()?{return Err(ModelError::Conflict("terminal selector changed"));}
                let cap=installed_view::recovery_view(client.clone(),pin.clone(),installed.clone(),expected,inventory.clone(),budget)?;
                charge.grow(4096)?;views.insert(installed.view.identity,Arc::new(cap));
            }
            if outputs!=descriptor.outputs.keys().cloned().collect(){return Err(ModelError::Conflict("terminal complete singleton output selector inventory"));}
            let roots=families.finish_scoped(&reader,inventory.bindings(),budget,&audit).await?;
            if roots!=data.roots{return Err(ModelError::Conflict("terminal compound independently reconstructed six-family state"));}
            Ok((views,contributors))
        }.await;
        if acquired.as_ref().err().is_some_and(|error|!error.permits_storage_cleanup()){audit.retain_unknown();}
        let mut terminal=Completion::default();terminal.step("terminal independent compound audit close",audit.close().await);let (views,contributors)=complete(acquired,terminal)?;
        for expected in [&product,&contribution] {
            let actual=held::inspect_expected_read(&reader,expected.record().clone(),budget).await?.ok_or(ModelError::Conflict("terminal acquired record disappeared"))?;
            if actual.incarnation()!=expected.incarnation() || actual.row()!=expected.row(){return Err(ModelError::Conflict("terminal acquired record changed"));}
        }
        check_installation(&reader,generation,validity_epoch).await?;
        Ok(HeldTerminalProduct{client:client.clone(),pin:Some(pin.clone()),product,contribution,descriptor,
            generation,validity_epoch,views,contributors,factory_lease:lease.clone(),cancellation:cancellation.clone(),retry:retry.clone(),closed:false,_charge:charge})
    }).await;
    let mut terminal = Completion::default();
    terminal.step(
        "terminal product acquisition reader close",
        reader.close().await,
    );
    let result = complete(result, terminal);
    drop(reader);
    if result
        .as_ref()
        .err()
        .is_some_and(ModelError::permits_storage_cleanup)
    {
        let mut terminal = Completion::default();
        let released = pin.release().await;
        if released.is_err() {
            pin.retain_unknown();
            terminal.local = lctx_model::domain::completion::LocalState::Outstanding;
        }
        let can_close = released.is_ok();
        terminal.step("failed terminal acquisition pin release", released);
        if can_close {
            let closed = lease.close_session().await;
            if closed.is_ok() {
                terminal.step(
                    "terminal rejected acquisition source receipt",
                    lease.complete().await,
                );
            }
            terminal.step("failed terminal acquisition session close", closed);
        }
        let result = complete(result, terminal);
        pin_terminal.observe(&result);
        result.map_err(|error| factory.handback(error))
    } else {
        pin_terminal.observe(&result);
        result.map_err(|error| factory.handback(error))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn recovery_factory_guard_retains_unknown_before_cap_publication() {
        let pin = ReaderPin::test_detached(false);
        drop(RecoveryPinTerminal::new(pin.clone()));
        assert!(pin.test_uncertain());
        let confirmed = ReaderPin::test_detached(false);
        let mut guard = RecoveryPinTerminal::new(confirmed.clone());
        guard.observe(&Ok::<_, ModelError>(()));
        drop(guard);
        assert!(!confirmed.test_uncertain());
        confirmed.retain_unknown();
    }
    #[test]
    fn terminal_product_requires_actual_terminal_current_source() {
        let generation = ContentHash::of(b"installed current generation");
        let mut row = Object::new();
        row.insert("generation", generation.hex());
        row.insert("epoch", 1_i64);
        for state in ["closed", "abandoned", "maintenance_fenced", "frozen"] {
            row.insert("state", state);
            terminal_source(&row, generation).unwrap();
        }
        for state in ["open", "closing", "preparing", "failed", ""] {
            row.insert("state", state);
            assert!(terminal_source(&row, generation).is_err());
        }
        row.insert("state", "closed");
        row.insert("epoch", 0_i64);
        assert!(terminal_source(&row, generation).is_err());
        row.insert("epoch", 1_i64);
        assert!(terminal_source(&row, ContentHash::of(b"foreign generation")).is_err());
    }
}
