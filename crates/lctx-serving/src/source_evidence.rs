//! Pure evidence lowering over one already-scoped canonical native selection.
use lctx_model::domain::{resources::ResourceBudget, serving::*, *};
use lctx_surrealdb::batches::CanonicalBatches;
use std::sync::Arc;
pub use crate::records::PacketRows;
use crate::records::Prepared;
#[derive(Debug)]
pub enum EvidenceError {
    Contract,
    Codec(String),
    ResourceRefused(&'static str),
    Model(ModelError),
}
impl From<ModelError> for EvidenceError {
    fn from(value: ModelError) -> Self {
        Self::Model(value)
    }
}
impl From<EvidenceError> for ModelError {
    fn from(value: EvidenceError) -> Self {
        match value {
            EvidenceError::Contract => Self::Schema("native evidence packet contract"),
            EvidenceError::Codec(s) => Self::Invalid(s),
            EvidenceError::ResourceRefused(s) => Self::Invalid(format!("resource_refused: {s}")),
            EvidenceError::Model(e) => e,
        }
    }
}
pub struct NativePackets<'a> {
    source: Arc<Prepared<'a>>,
    pub(crate) budget: ResourceBudget,
    pub(crate) charge: charged::StateCharge,
    claims: Arc<crate::claims::Claims>,
}
impl<'a> NativePackets<'a> {
    pub fn new(source:&'a CanonicalBatches,budget:&ResourceBudget)->Result<Self,ModelError>{
        Self::from_prepared(Arc::new(Prepared::new(source,budget)))
    }
    pub fn from_prepared(source:Arc<Prepared<'a>>)->Result<Self,ModelError>{
        let budget=source.budget().clone();let claims=source.claims()?;
        Ok(Self{source,budget:budget.clone(),charge:charged::StateCharge::new(&budget,"native-evidence-packets"),claims})
    }
    pub(crate) async fn read_ids<R:Record>(&mut self,ids:&[Id<R>])->Result<PacketRows<R>,EvidenceError>{
        Ok(self.source.rows::<R>()?.select_ids(ids)?)
    }
    pub(crate) async fn read_for<R:Record,T:Record>(&mut self,field:&str,ids:&[Id<T>])->Result<PacketRows<R>,EvidenceError>{
        Ok(self.source.rows::<R>()?.select_for(field,ids)?)
    }

    pub(crate) async fn claim_basis(
        &mut self,
        q: &assertion::AssertionQualification,
    ) -> Result<ClaimBasisPacket, EvidenceError> {
        Ok(self.claims.basis(q.id())?)
    }
}

/// Source dependencies are derived from the sole packet mapping declaration.
pub fn inputs() -> Vec<ValidationInput> {
    use serving::mappings::PacketOutput;
    EvidencePacket::binding()
        .lowered()
        .sources
        .iter()
        .map(|r| ValidationInput::of_relation(r, &["id"]))
        .collect()
}
/// Proof dependencies permit outgoing reads, not incoming ownership of every graph family.
pub fn owner_inputs() -> Vec<ValidationInput> {
    use serving::mappings::PacketOutput;
    EvidencePacket::binding()
        .mapping
        .sources
        .iter()
        .map(|r| ValidationInput::of_relation(r, &["id"]))
        .collect()
}
/// Lower all indivisible selected records before independently paging either section.
pub async fn sections(
    source: Arc<Prepared<'_>>,
    range: &OriginalRange,
    request: &Request,
    snapshot: &SnapshotHandle,
    channels: &ChannelState,
    budget: &ResourceBudget,
) -> Result<
    (
        SectionPage<FlowInventoryPacket>,
        SectionPage<SourceCharacterizationPacket>,
    ),
    ModelError,
> {
    let mut packets = NativePackets::from_prepared(source)?;
    let flows = packets
        .flow_inventory(range, usize::MAX)
        .await
        .map_err(ModelError::from)?;
    let characterizations = packets
        .source_characterization(range, usize::MAX)
        .await
        .map_err(ModelError::from)?;
    let flow_keys = flows
        .items
        .into_iter()
        .map(|p| Ok((key(derivation::RowRef::of(p.inventory))?, p)))
        .collect::<Result<Vec<_>, ModelError>>()?;
    let characterization_keys = characterizations
        .items
        .into_iter()
        .map(|p| Ok((key(derivation::RowRef::of(p.characterization))?, p)))
        .collect::<Result<Vec<_>, ModelError>>()?;
    let flow = crate::pagination::page(
        flow_keys,
        request,
        snapshot,
        channels,
        request.tool().name(),
        "flow_inventory",
        None,
        flows.availability,
    )
    .map_err(crate::records::wire)?;
    let characterization = crate::pagination::page(
        characterization_keys,
        request,
        snapshot,
        channels,
        request.tool().name(),
        "source_characterization",
        None,
        characterizations.availability,
    )
    .map_err(crate::records::wire)?;
    Ok((flow, characterization))
}

fn key(row: derivation::RowRef) -> Result<ContentHash, ModelError> {
    match graph::target_for_row(row)? {
        graph::Target::Entity(id) => Ok(id.0),
        graph::Target::Assertion(id) => Ok(id.0),
        graph::Target::External { .. } => Err(ModelError::Schema("external evidence packet key")),
    }
}
