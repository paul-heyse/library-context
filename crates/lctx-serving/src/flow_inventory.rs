//! Native per-use enumeration within an immutable original-source grant. Full inventory
//! hydration and model replay precede any packet or byte omission.
use super::source_evidence::{EvidenceError as Error, NativePackets, PacketRows};
use lctx_model::domain::{
    assertion::*, attribution::*, flow::*, flow_inventory::*, serving::*, source::Occurrence, *,
};
fn required<R: Record>(batch: &PacketRows<R>, id: Id<R>) -> Result<&R, Error> {
    batch.rows().iter().find(|r| r.id() == id).ok_or_else(|| {
        Error::Model(ModelError::Invalid(format!(
            "required native evidence row missing: {}",
            R::NAME
        )))
    })
}
fn name(value: &str) -> Result<Name, Error> {
    Name::new(value).map_err(|e| Error::Codec(e.to_string()))
}
impl NativePackets<'_> {
    pub(crate) async fn flow_inventory(
        &mut self,
        grant: &OriginalRange,
        maximum: usize,
    ) -> Result<SectionPage<FlowInventoryPacket>, Error> {
        let occurrences = self
            .read_for::<Occurrence, source::SourceArtifact>("source", &[grant.artifact])
            .await?;
        let _work = self.budget.reserve(
            "flow-inventory-grant-index",
            occurrences.rows().len().saturating_add(1) * 128,
        )?;
        let selected = occurrences
            .rows()
            .iter()
            .filter(|o| {
                o.start >= 0
                    && grant.start <= o.start as u64
                    && o.end >= o.start
                    && o.end as u64 <= grant.end
            })
            .map(Record::id)
            .collect::<Vec<_>>();
        let uses = self
            .read_for::<FlowUse, Occurrence>("occurrence", &selected)
            .await?;
        let _uses_index = self.budget.reserve(
            "flow-inventory-use-index",
            uses.rows().len().saturating_add(1) * 64,
        )?;
        let use_ids = uses.rows().iter().map(Record::id).collect::<Vec<_>>();
        let inventories = self
            .read_for::<FlowUseInventoryObservation, FlowUse>("use_", &use_ids)
            .await?;
        let mut selected = Vec::new();
        let _inventory_work = self.budget.reserve(
            "flow-inventory-grant-selection",
            inventories.rows().len().saturating_add(1) * 128,
        )?;
        for inventory in inventories.rows() {
            let qualifications = self
                .read_ids::<AssertionQualification>(&[inventory.qualification])
                .await?;
            let qualification = required(&qualifications, inventory.qualification)?;
            if qualification.context == grant.context {
                selected.push(inventory);
            }
        }
        selected.sort_by_key(|i| {
            let use_ = uses
                .rows()
                .iter()
                .find(|u| u.id() == i.use_)
                .expect("inventory use supplied");
            let o = occurrences
                .rows()
                .iter()
                .find(|o| o.id() == use_.occurrence)
                .expect("grant occurrence supplied");
            (o.start, o.end, i.id())
        });
        let total = selected.len();
        let mut items = Vec::new();
        let mut partial = false;
        for inventory in selected.into_iter().take(maximum) {
            let candidates = self
                .read_for::<FlowUseCandidate, FlowUseInventoryObservation>(
                    "inventory",
                    &[inventory.id()],
                )
                .await?;
            let members = self
                .read_for::<FlowUseInventoryMember, FlowUseInventoryObservation>(
                    "inventory",
                    &[inventory.id()],
                )
                .await?;
            let _packet = self.budget.reserve(
                "flow-inventory-packet",
                (candidates.rows().len() + members.rows().len() + 1).saturating_mul(2048),
            )?;
            let explanation = explain(
                inventory.use_,
                inventory,
                candidates.rows(),
                members.rows(),
                &self.budget,
            )?;
            let qualifications = self
                .read_ids::<AssertionQualification>(&[inventory.qualification])
                .await?;
            let qualification = required(&qualifications, inventory.qualification)?;
            let use_ = required(&uses, inventory.use_)?;
            let occurrence = required(&occurrences, use_.occurrence)?;
            let supports = self
                .read_for::<FlowUseInventorySupport, FlowUseInventoryObservation>(
                    "assertion",
                    &[inventory.id()],
                )
                .await?;
            let _support_work = self.budget.reserve(
                "flow-inventory-support-proof",
                supports.rows().len().saturating_add(1) * 512,
            )?;
            if supports.rows().is_empty() {
                return Err(Error::Contract);
            }
            let witnesses = self
                .read_for::<conditions::entry::EntryValueWitness, Occurrence>(
                    "access",
                    &[occurrence.id()],
                )
                .await?;
            let _singleton_work = self.budget.reserve(
                "flow-inventory-singleton-packets",
                witnesses.rows().len().saturating_add(1) * 8192,
            )?;
            let runs = supports.rows().iter().map(|s| s.run).collect::<Vec<_>>();
            let singleton = explanation.entry_outcomes(
                use_,
                grant.context,
                &runs,
                witnesses.rows(),
                &self.budget,
            )?;
            let entry_value_reason = Nullable(singleton.reason());
            let mut outcomes = Vec::new();
            for witness in &singleton.witnesses {
                let inventory_support = required(&supports, witness.inventory_support)?;
                if inventory_support.assertion != inventory.id()
                    || inventory_support.run != witness.run
                {
                    return Err(Error::Contract);
                }
                let source_rows = self
                    .read_ids::<conditions::entry::EntryAccessSource>(&[witness.access_source])
                    .await?;
                let mut qualifications = Vec::new();
                let uses = self
                    .read_ids::<FlowUseObservation>(&[witness.use_observation])
                    .await?;
                let use_ = required(&uses, witness.use_observation)?;
                if use_.use_ != inventory.use_ {
                    return Err(Error::Contract);
                }
                qualifications.push(use_.qualification);
                match required(&source_rows, witness.access_source)? {
                    conditions::entry::EntryAccessSource::Use { observation, .. } => {
                        let rows = self.read_ids::<FlowUseObservation>(&[*observation]).await?;
                        qualifications.push(required(&rows, *observation)?.qualification);
                    }
                    conditions::entry::EntryAccessSource::Value {
                        observation,
                        region,
                        ..
                    } => {
                        let rows = self
                            .read_ids::<FlowValueObservation>(&[*observation])
                            .await?;
                        qualifications.push(required(&rows, *observation)?.qualification);
                        let rows = self.read_ids::<FlowRegionObservation>(&[*region]).await?;
                        qualifications.push(required(&rows, *region)?.qualification);
                    }
                    conditions::entry::EntryAccessSource::Guard {
                        observation,
                        region,
                        ..
                    } => {
                        let rows = self
                            .read_ids::<FlowTestLeafObservation>(&[*observation])
                            .await?;
                        qualifications.push(required(&rows, *observation)?.qualification);
                        let rows = self.read_ids::<FlowRegionObservation>(&[*region]).await?;
                        qualifications.push(required(&rows, *region)?.qualification);
                    }
                }
                qualifications.sort();
                qualifications.dedup();
                let rows = self
                    .read_ids::<AssertionQualification>(&qualifications)
                    .await?;
                let mut premises = Vec::new();
                for q in rows.rows() {
                    if q.context != grant.context {
                        return Err(Error::Contract);
                    }
                    premises.push(StoredEntryPremise {
                        qualification: q.id(),
                        condition: q.condition,
                        claim_basis: self.claim_basis(q).await?,
                    });
                }
                let contributions=self.read_for::<local_semantics::LocalContribution,conditions::entry::EntryValueWitness>("entry",&[witness.id()]).await?;
                let _contribution_work = self.budget.reserve(
                    "flow-inventory-contribution-packets",
                    contributions.rows().len().saturating_add(1) * 8192,
                )?;
                let mut transferred = Vec::new();
                for contribution in contributions.rows() {
                    use analysis::support::DerivedEvidence;
                    let rows = self
                        .read_ids::<AssertionQualification>(&[contribution.qualification])
                        .await?;
                    let q = required(&rows, contribution.qualification)?;
                    if q.context != grant.context {
                        return Err(Error::Contract);
                    }
                    transferred.push(StoredEntryContribution {
                        contribution: contribution.id(),
                        qualification: q.id(),
                        condition: q.condition,
                        claim_basis: self.claim_basis(q).await?,
                        status: contribution.source_facts().status,
                    });
                }
                outcomes.push(StoredEntryOutcome {
                    witness: witness.id(),
                    formal: witness.formal,
                    owner: witness.owner,
                    run: witness.run,
                    access_source: witness.access_source,
                    coverage: witness.coverage,
                    premises,
                    contributions: transferred,
                    proof: [
                        derivation::RowRef::of(witness.id()),
                        derivation::RowRef::of(witness.access_source),
                        derivation::RowRef::of(witness.use_support),
                        derivation::RowRef::of(witness.inventory),
                        derivation::RowRef::of(witness.inventory_support),
                        derivation::RowRef::of(witness.reaching_support),
                        derivation::RowRef::of(witness.definition_support),
                        derivation::RowRef::of(witness.declaration_support),
                        derivation::RowRef::of(witness.owner_support),
                    ]
                    .into_iter()
                    .map(ProofReference::from_canonical)
                    .collect::<Result<Vec<_>, _>>()?,
                });
            }
            let entry_outcomes = SectionPage {
                items: outcomes,
                availability: Availability::Available {},
                continuation: Optional::default(),
                omitted: 0,
                truncated: false,
            };
            let view_rows = self
                .read_ids::<FlowSourceViewObservation>(&[inventory.view])
                .await?;
            let view = required(&view_rows, inventory.view)?;
            if view.source != grant.artifact || view.original_content != grant.digest {
                return Err(Error::Contract);
            }
            let view_supports = self
                .read_for::<FlowSourceViewSupport, FlowSourceViewObservation>(
                    "assertion",
                    &[view.id()],
                )
                .await?;
            let _view_work = self.budget.reserve(
                "flow-inventory-view-proof",
                view_supports.rows().len().saturating_add(1) * 512,
            )?;
            let coverage = self
                .read_for::<ProviderCoverage, source::CoverageScope>(
                    "scope",
                    &[qualification.scope],
                )
                .await?;
            let _coverage_work = self.budget.reserve(
                "flow-inventory-coverage-index",
                coverage.rows().len().saturating_add(1) * 64,
            )?;
            let coverage = coverage
                .rows()
                .iter()
                .filter(|c| {
                    c.family == FactFamily::Flow
                        && c.context == grant.context
                        && supports.rows().iter().any(|s| c.run == Some(s.run))
                })
                .collect::<Vec<_>>();
            partial |= !inventory.complete
                || coverage.is_empty()
                || coverage
                    .iter()
                    .any(|c| c.status != CoverageStatus::CompleteUnderStatedModel);
            let mut mapped = Vec::new();
            let mut proof = vec![
                derivation::RowRef::of(inventory.id()),
                derivation::RowRef::of(use_.id()),
                derivation::RowRef::of(occurrence.id()),
                derivation::RowRef::of(inventory.qualification),
            ];
            proof.extend(
                supports
                    .rows()
                    .iter()
                    .map(|s| derivation::RowRef::of(s.id())),
            );
            for member in explanation.members {
                let observations = self
                    .read_ids::<FlowReachingObservation>(&[member.reaching])
                    .await?;
                let reaching = required(&observations, member.reaching)?;
                let support_rows = self
                    .read_ids::<FlowReachingSupport>(&[member.support])
                    .await?;
                let support = required(&support_rows, member.support)?;
                if reaching.use_ != inventory.use_ || support.assertion != reaching.id() {
                    return Err(Error::Contract);
                }
                let targets = self
                    .read_ids::<ReachingDefinition>(&[reaching.target])
                    .await?;
                let target = match required(&targets, reaching.target)? {
                    ReachingDefinition::Bound { definition } => {
                        let definitions = self.read_ids::<FlowDefinition>(&[*definition]).await?;
                        let row = required(&definitions, *definition)?;
                        FlowOriginTarget::Bound {
                            definition: *definition,
                            occurrence: row.occurrence,
                        }
                    }
                    ReachingDefinition::Unbound => FlowOriginTarget::Unbound {},
                    ReachingDefinition::Nested => FlowOriginTarget::Nested {},
                };
                let q_rows = self
                    .read_ids::<AssertionQualification>(&[reaching.qualification])
                    .await?;
                let q = required(&q_rows, reaching.qualification)?;
                if q.context != grant.context {
                    return Err(Error::Contract);
                }
                mapped.push(FlowInventoryReaching {
                    member: member.id(),
                    ordinal: member.ordinal,
                    reaching: reaching.id(),
                    support: support.id(),
                    qualification: q.id(),
                    condition: q.condition,
                    target,
                    loop_carried: reaching.loop_carried,
                });
            }
            let mut candidates = Vec::new();
            for c in explanation.candidates {
                let mut formulas = Vec::new();
                for id in [c.reachability, c.narrowing] {
                    let formula = if let Some(id) = id {
                        let rows = self.read_ids::<AssertionQualification>(&[id]).await?;
                        let q = required(&rows, id)?;
                        if q.context != grant.context
                            || q.scope != qualification.scope
                            || q.assumptions != qualification.assumptions
                        {
                            return Err(Error::Contract);
                        }
                        proof.push(derivation::RowRef::of(q.id()));
                        Some(FlowCandidateFormula {
                            qualification: q.id(),
                            condition: q.condition,
                            scope: q.scope,
                            context: q.context,
                            modality: q.modality,
                            approximation: q.approximation,
                            claim_basis: self.claim_basis(q).await?,
                        })
                    } else {
                        None
                    };
                    formulas.push(formula);
                }
                let narrowing = formulas.pop().expect("two native formulas");
                let reachability = formulas.pop().expect("two native formulas");
                candidates.push(FlowInventoryCandidate {
                    candidate: c.id(),
                    ordinal: c.ordinal,
                    kind: c.kind,
                    pruned: c.pruned,
                    loop_expanded: c.loop_expanded,
                    unattached: c.unattached,
                    reachability: Nullable(reachability),
                    narrowing: Nullable(narrowing),
                    narrowing_unavailable: c.narrowing_unavailable,
                    narrowing_precision_lost: c.narrowing_precision_lost,
                    condition_unavailable: c.condition_unavailable,
                    reachability_lost: c.reachability_lost,
                    mapped_count: c.mapped_count,
                });
            }
            let mut view_proof = vec![ProofReference::from_canonical(derivation::RowRef::of(
                view.id(),
            ))?];
            view_proof.extend(
                view_supports
                    .rows()
                    .iter()
                    .map(|s| ProofReference::from_canonical(derivation::RowRef::of(s.id())))
                    .collect::<Result<Vec<_>, _>>()?,
            );
            items.push(FlowInventoryPacket {
                inventory: inventory.id(),
                use_: use_.id(),
                occurrence: occurrence.id(),
                artifact: occurrence.source,
                start: occurrence.start as u64,
                end: occurrence.end as u64,
                context: grant.context,
                qualification: qualification.id(),
                condition: qualification.condition,
                native_count: inventory.native_count as u64,
                mapped_count: inventory.mapped_count as u64,
                complete: inventory.complete,
                candidates,
                members: mapped,
                view: FlowInventoryView {
                    view: view.id(),
                    original_content: view.original_content,
                    view_content: view.view_content,
                    renamed_type_checking: view.renamed_type_checking,
                    coverage: coverage.iter().map(|c| c.id()).collect(),
                    proof: view_proof,
                },
                entry_value_reason,
                entry_outcomes,
                proof: proof
                    .into_iter()
                    .map(ProofReference::from_canonical)
                    .collect::<Result<Vec<_>, _>>()?,
            });
            // Retained packet accounting outlives all hydrated batches and explanation indexes.
            self.charge.grow(
                serde_json::to_vec(items.last().expect("packet inserted"))
                    .map_err(|e| Error::Codec(e.to_string()))?
                    .len(),
            )?;
        }
        let omitted = total - items.len();
        Ok(SectionPage {
            availability: if total == 0 {
                Availability::Unavailable {
                    reason: name(
                        "no native inventory in granted range; absence is not established",
                    )?,
                }
            } else if omitted > 0 {
                Availability::Partial {
                    reason: name("whole native inventory result count bound reached")?,
                }
            } else if partial {
                Availability::Partial {
                    reason: name("native inventory or provider coverage is partial")?,
                }
            } else {
                Availability::Available {}
            },
            items,
            continuation: Optional::default(),
            omitted: omitted as u64,
            truncated: omitted > 0,
        })
    }
}
