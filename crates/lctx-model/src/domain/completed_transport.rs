//! Portable correspondence is data, never native admission. Semantic v4 remains independent
//! of exact capture identity and conservative selector-local substitution eligibility.
use super::{
    ContentHash, Key, KeySink, ModelError,
    completed::{CompletedContribution, CompletedStateIdentity, CompletedView},
    completed_identity::CompletedFamilyRoots,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const TRANSPORT_CODEC: u32 = 1;
pub const RECEIPT_RECIPE: u32 = 1;
pub type CopyLabel = u64;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(transparent)]
pub struct EligibilityIdentity(pub ContentHash);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransportInventory {
    pub rows: u64,
    pub content: ContentHash,
}

#[derive(Debug, Clone, Copy)]
pub enum TransportInventoryKind {
    Memberships,
    Aliases,
}
pub struct TransportInventoryHasher {
    rows: u64,
    sink: KeySink,
}
impl TransportInventoryHasher {
    pub fn new(kind: TransportInventoryKind) -> Self {
        let mut sink = KeySink::new("completed-transport-inventory/v1");
        sink.part(
            b"kind",
            match kind {
                TransportInventoryKind::Memberships => b"memberships",
                TransportInventoryKind::Aliases => b"aliases",
            },
        );
        Self { rows: 0, sink }
    }
    pub fn row(&mut self, canonical: &[u8]) -> Result<(), ModelError> {
        self.rows = self
            .rows
            .checked_add(1)
            .ok_or(ModelError::Schema("transport inventory count"))?;
        self.sink.part(b"row", canonical);
        Ok(())
    }
    pub fn finish(mut self) -> TransportInventory {
        self.sink.part(b"rows", &self.rows.to_le_bytes());
        TransportInventory {
            rows: self.rows,
            content: self.sink.finish(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransportSlot {
    pub view: ContentHash,
    pub logical: ContentHash,
    pub copy: CopyLabel,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransportRoot {
    pub view: CompletedView,
    pub dependencies: Vec<CompletedView>,
    pub slots: Vec<TransportSlot>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransportCopy {
    pub label: CopyLabel,
    pub descriptor: CompletedContribution,
    pub membership: TransportInventory,
    pub aliases: TransportInventory,
}

/// Includes explicit zero inventories. Payload streams must independently reproduce these values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompletedTransportEnvelope {
    pub version: u32,
    pub semantic: CompletedStateIdentity,
    pub roots: Vec<TransportRoot>,
    pub copies: Vec<TransportCopy>,
    pub families: CompletedFamilyRoots,
}

fn validate_projection(
    roots: &[TransportRoot],
    copies: &[TransportCopy],
) -> Result<(), ModelError> {
    if roots
        .windows(2)
        .any(|p| p[0].view.identity >= p[1].view.identity)
        || copies.iter().enumerate().any(|(i, c)| c.label != i as u64)
    {
        return Err(ModelError::Conflict("transport canonical root/copy order"));
    }
    let mut descriptors = BTreeMap::new();
    for copy in copies {
        let logical = copy.descriptor.identity()?;
        if let Some(previous) = descriptors.insert(logical, &copy.descriptor)
            && previous != &copy.descriptor
        {
            return Err(ModelError::Conflict(
                "transport logical descriptor collision",
            ));
        }
    }
    let mut first_use = BTreeSet::new();
    for root in roots {
        root.view.validate()?;
        if root
            .slots
            .windows(2)
            .any(|p| (p[0].view, p[0].logical) >= (p[1].view, p[1].logical))
        {
            return Err(ModelError::Conflict("transport selector slot order"));
        }
        let mut views = BTreeMap::new();
        views.insert(root.view.identity, &root.view);
        if root
            .dependencies
            .windows(2)
            .any(|p| p[0].identity >= p[1].identity)
        {
            return Err(ModelError::Conflict(
                "transport dependency descriptor order",
            ));
        }
        for dependency in &root.dependencies {
            dependency.validate()?;
            if views.insert(dependency.identity, dependency).is_some() {
                return Err(ModelError::Conflict(
                    "transport duplicate dependency descriptor",
                ));
            }
        }
        let mut slots = BTreeMap::new();
        for slot in &root.slots {
            let view = views.get(&slot.view).ok_or(ModelError::Conflict(
                "transport slot missing view descriptor",
            ))?;
            let copy = copies
                .get(usize::try_from(slot.copy).map_err(ModelError::codec)?)
                .ok_or(ModelError::Conflict("transport missing copy"))?;
            if copy.descriptor.identity()? != slot.logical
                || !view.contributions.contains(&slot.logical)
                || !copy.descriptor.outputs.contains_key(&view.relation)
            {
                return Err(ModelError::Conflict("transport exact slot descriptor"));
            }
            slots.insert((slot.view, slot.logical), copy);
            if !first_use.contains(&slot.copy) {
                if slot.copy != first_use.len() as u64 {
                    return Err(ModelError::Conflict("transport copy first-use order"));
                }
                first_use.insert(slot.copy);
            }
        }
        let mut pending = vec![root.view.identity];
        let mut reached = BTreeSet::new();
        let mut reached_slots = 0usize;
        while let Some(id) = pending.pop() {
            if !reached.insert(id) {
                continue;
            }
            let view = views
                .get(&id)
                .ok_or(ModelError::Conflict("transport missing dependency view"))?;
            for logical in &view.contributions {
                let copy = slots
                    .get(&(id, *logical))
                    .ok_or(ModelError::Conflict("transport missing selector slot"))?;
                reached_slots += 1;
                for input in &copy.descriptor.spec.inputs {
                    let selected = views.get(&input.view()).ok_or(ModelError::Conflict(
                        "transport missing dependency descriptor",
                    ))?;
                    if selected.relation != input.relation()
                        || selected.rows
                            != u64::try_from(input.rows()).map_err(ModelError::codec)?
                    {
                        return Err(ModelError::Conflict(
                            "transport dependency source correspondence",
                        ));
                    }
                    pending.push(input.view());
                }
            }
        }
        if reached_slots != root.slots.len() || reached.len() != views.len() {
            return Err(ModelError::Conflict("transport foreign selector slot"));
        }
    }
    if first_use.len() != copies.len() {
        return Err(ModelError::Conflict("transport unreachable copy"));
    }
    Ok(())
}

/// Ignores capture labels/sharing while retaining root-qualified local alias meaning.
pub fn eligibility_projection(
    roots: &[TransportRoot],
    copies: &[TransportCopy],
) -> Result<EligibilityIdentity, ModelError> {
    validate_projection(roots, copies)?;
    let mut sink = KeySink::new("completed-selector-eligibility/v1");
    // Full variable-sized descriptors enter once; slot references retain exact correspondence.
    let mut views = BTreeMap::new();
    let mut descriptors = BTreeMap::new();
    for root in roots {
        for view in std::iter::once(&root.view).chain(&root.dependencies) {
            if let Some(previous) = views.insert(view.identity, view)
                && previous != view
            {
                return Err(ModelError::Conflict(
                    "eligibility view descriptor collision",
                ));
            }
        }
    }
    for copy in copies {
        descriptors.insert(copy.descriptor.identity()?, &copy.descriptor);
    }
    sink.part(b"views", &(views.len() as u64).to_le_bytes());
    for view in views.values() {
        sink.json_part(b"view", view)?;
    }
    sink.part(b"descriptors", &(descriptors.len() as u64).to_le_bytes());
    for descriptor in descriptors.values() {
        sink.json_part(b"descriptor", descriptor)?;
    }
    sink.part(b"roots", &(roots.len() as u64).to_le_bytes());
    for root in roots {
        root.view.identity.encode(&mut sink);
        sink.part(b"slots", &(root.slots.len() as u64).to_le_bytes());
        for slot in &root.slots {
            let copy = &copies[slot.copy as usize];
            slot.view.encode(&mut sink);
            slot.logical.encode(&mut sink);
            sink.json_part(b"aliases", &copy.aliases)?;
        }
    }
    Ok(EligibilityIdentity(sink.finish()))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransportCounts {
    pub roots: u64,
    pub slots: u64,
    pub copies: u64,
    pub memberships: u64,
    pub aliases: u64,
    pub rows: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompletedTransportReceipt {
    pub receipt_recipe: u32,
    pub transport_codec: u32,
    pub semantic: CompletedStateIdentity,
    pub eligibility: EligibilityIdentity,
    pub envelope: ContentHash,
    pub inventory_counts: TransportCounts,
}
impl CompletedTransportReceipt {
    pub fn validate(&self) -> Result<(), ModelError> {
        self.semantic.validate()?;
        if self.receipt_recipe != RECEIPT_RECIPE || self.transport_codec != TRANSPORT_CODEC {
            return Err(ModelError::Conflict(
                "unsupported transport receipt recipe/codec",
            ));
        }
        Ok(())
    }
}
impl CompletedTransportEnvelope {
    pub fn validate(&self) -> Result<(), ModelError> {
        self.semantic.validate()?;
        if self.version != TRANSPORT_CODEC
            || CompletedStateIdentity::from_roots(&self.families)? != self.semantic
        {
            return Err(ModelError::Conflict("transport codec/semantic families"));
        }
        validate_projection(&self.roots, &self.copies)
    }
    pub fn eligibility_identity(&self) -> Result<EligibilityIdentity, ModelError> {
        self.validate()?;
        eligibility_projection(&self.roots, &self.copies)
    }
    pub fn receipt(&self) -> Result<CompletedTransportReceipt, ModelError> {
        self.validate()?;
        let sum = |values: Vec<u64>| {
            values.into_iter().try_fold(0u64, |a, b| {
                a.checked_add(b)
                    .ok_or(ModelError::Schema("transport count overflow"))
            })
        };
        let inventory_counts = TransportCounts {
            roots: self.roots.len() as u64,
            slots: sum(self.roots.iter().map(|r| r.slots.len() as u64).collect())?,
            copies: self.copies.len() as u64,
            memberships: sum(self.copies.iter().map(|c| c.membership.rows).collect())?,
            aliases: sum(self.copies.iter().map(|c| c.aliases.rows).collect())?,
            rows: sum(self.families.roots().iter().map(|f| f.rows).collect())?,
        };
        let mut sink = KeySink::new("completed-transport-envelope/v1");
        sink.json_part(b"envelope", self)?;
        Ok(CompletedTransportReceipt {
            receipt_recipe: RECEIPT_RECIPE,
            transport_codec: TRANSPORT_CODEC,
            semantic: self.semantic.clone(),
            eligibility: self.eligibility_identity()?,
            envelope: sink.finish(),
            inventory_counts,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactExportReceipt {
    pub manifest: ContentHash,
    pub state: CompletedTransportReceipt,
}
impl ArtifactExportReceipt {
    pub fn validate(&self) -> Result<(), ModelError> {
        self.state.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackupReceipt {
    pub recipe: u32,
    pub source: super::serving::SnapshotHandle,
    pub manifest: ContentHash,
    pub definitions: ContentHash,
    pub state: CompletedTransportReceipt,
    pub carrier: ContentHash,
}
impl BackupReceipt {
    pub fn validate(&self) -> Result<(), ModelError> {
        self.source.validate_identity()?;
        self.state.validate()?;
        if self.recipe != 1
            || self.source.eligibility != self.state.eligibility
            || self.source.semantic != self.manifest
        {
            return Err(ModelError::Conflict("backup receipt recipe/eligibility"));
        }
        Ok(())
    }
}
