//! Bounded validation over owned immutable frames. Receipts acknowledge complete conclusions;
//! read grants refer to them, rather than constituting a second semantic authority.
use super::{Error, GenerationId, GenerationStore, visit_named};
use lctx_model::domain::{ContentHash, Invariant, KeySink, PublicationInvariant, ValidationInput,
    admission::FrontierAdmission, resources::{Reservation, ResourceBudget}, stages::{CompletedRelation, Profile, RelationReceipt}};
use sqlx::PgConnection;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub(super) enum ProofContext {
    Invariant { frames: Vec<(String, String)> },
    Publication { frames: Vec<(String, String)>, sources: Vec<lctx_model::domain::analysis::sources::SourceSnapshot>, profile: String },
    Nominal { inputs: ContentHash },
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ValidationStats {
    pub resolved_frames: u64,
    pub physical_frames: u64,
    pub row_scans: u64,
    pub check_executions: u64,
    pub proof_hits: u64,
}

type EnvelopeRow = (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>, String);

struct CheckpointAuthority {
    frontier: &'static str,
    contract: ContentHash,
    model: ContentHash,
    schedule: ContentHash,
    coverage: ContentHash,
    content: ContentHash,
    profile: Profile,
    covered: BTreeSet<&'static str>,
}

pub(super) struct Session<'s> {
    model: &'s lctx_model::domain::ValidatedModel,
    generation: GenerationId,
    budget: &'s ResourceBudget,
    envelope: ContentHash,
    physical: ContentHash,
    frames: BTreeMap<&'static str, BTreeMap<String, RelationReceipt>>,
    checkpoint: Option<CheckpointAuthority>,
    charge: Box<dyn Reservation>,
    pub stats: ValidationStats,
}

impl<'s> Session<'s> {
    pub async fn new(store: &'s GenerationStore, tx: &mut PgConnection, generation: GenerationId,
        budget: &'s ResourceBudget) -> Result<Self, Error> {
        Self::for_model(&store.model, tx, generation, budget).await
    }

    pub async fn for_model(model_owner: &'s lctx_model::domain::ValidatedModel, tx: &mut PgConnection, generation: GenerationId,
        budget: &'s ResourceBudget) -> Result<Self, Error> {
        let charge = budget.reserve("validation-session", 4096)?;
        let (installation, model, physical, producer, schedule, profile):
            EnvelopeRow = sqlx::query_as(
            "SELECT i.identity,g.model_digest,g.physical_digest,g.producer_digest,g.schedule_digest,g.profile FROM lctx_model_store.installation i CROSS JOIN lctx_model_store.generations g WHERE i.singleton AND g.id=$1")
            .bind(generation.0.to_vec()).fetch_one(&mut *tx).await?;
        if model != model_owner.digest().0 { return Err(Error::Contract); }
        let physical = ContentHash(physical.try_into().map_err(|_| Error::Contract)?);
        let mut sink = KeySink::new("validation-envelope/v1");
        sink.part(b"installation", &installation);
        sink.part(b"generation", &generation.0);
        sink.part(b"model", &model);
        sink.part(b"physical", &physical.0);
        sink.part(b"producer", &producer);
        sink.part(b"schedule", &schedule);
        sink.part(b"profile", profile.as_bytes());
        Ok(Self { model: model_owner, generation, budget, envelope: sink.finish(), physical,
            frames: BTreeMap::new(), checkpoint: None, charge, stats: ValidationStats::default() })
    }

    /// Only the effect owner's admitted checkpoint authorizes its covered immutable frames.
    /// Completed sources and producer acknowledgements remain independently required.
    pub fn bind_checkpoint(&mut self, store: &GenerationStore, admission: &FrontierAdmission) -> Result<(), Error> {
        if admission.model() != self.model.digest() || store.model.digest() != self.model.digest()
            || self.checkpoint.is_some() { return Err(Error::Contract); }
        let covered = &store.scope(admission.frontier())?.relations;
        let bytes = covered.len().checked_mul(128).and_then(|bytes| bytes.checked_add(512))
            .ok_or(Error::Contract)?;
        self.charge.try_resize(self.charge.size().checked_add(bytes).ok_or(Error::Contract)?)?;
        self.checkpoint = Some(CheckpointAuthority {
            frontier: admission.frontier().name(), contract: admission.contract(), model: admission.model(),
            schedule: admission.schedule(), coverage: admission.coverage(), content: admission.content(),
            profile: admission.profile(), covered: covered.clone(),
        });
        Ok(())
    }

    /// The caller holds installation/generation authority and mutation-excluding candidate
    /// locks. An absent acknowledgement is allowed only for the final candidate being closed.
    pub async fn frame(&mut self, tx: &mut PgConnection, input: &ValidationInput,
        physical: &str, candidate: bool) -> Result<RelationReceipt, Error> {
        self.prepare_frame(tx, input, physical, candidate).await?.ok_or(Error::Contract)
    }

    async fn prepare_frame(&mut self, tx: &mut PgConnection, input: &ValidationInput,
        physical: &str, hash_missing: bool) -> Result<Option<RelationReceipt>, Error> {
        if let Some(receipt) = self.frames.get(input.name()).and_then(|versions| versions.get(physical)) { return Ok(Some(*receipt)); }
        let mut expected: Option<(i64, Vec<u8>)> = if lctx_model::domain::stages::is_vocabulary(input.name()) {
            if physical == input.name() {
                sqlx::query_as("SELECT r.row_count,r.content_digest FROM lctx_model_store.epoch_receipts r JOIN lctx_model_store.publication_groups p USING(generation_id,epoch) WHERE r.generation_id=$1 AND r.relation_name=$2 AND p.closed ORDER BY r.epoch DESC LIMIT 1")
                    .bind(self.generation.0.to_vec()).bind(input.name()).fetch_optional(&mut *tx).await?
            } else {
                let ordinal = physical.strip_prefix("__v").and_then(|v| v.split_once('_'))
                    .filter(|(_, name)| *name == input.name()).and_then(|(n, _)| n.parse::<i16>().ok())
                    .ok_or(Error::Contract)?;
                sqlx::query_as("SELECT r.row_count,r.content_digest FROM lctx_model_store.epoch_receipts r JOIN lctx_model_store.publication_groups p USING(generation_id,epoch) WHERE r.generation_id=$1 AND r.relation_name=$2 AND r.epoch=$3 AND p.closed")
                    .bind(self.generation.0.to_vec()).bind(input.name()).bind(ordinal).fetch_optional(&mut *tx).await?
            }
        } else {
            sqlx::query_as("SELECT row_count,content_digest FROM lctx_model_store.stage_receipts WHERE generation_id=$1 AND relation_name=$2")
                .bind(self.generation.0.to_vec()).bind(input.name()).fetch_optional(&mut *tx).await?
        };
        if expected.is_none() && physical == input.name() {
            // Finite ordinary vocabulary models have no publication groups. They still
            // freeze through the same direct-output acknowledgement; no prefix is invented.
            let grouped: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM lctx_model_store.publication_groups WHERE generation_id=$1)")
                .bind(self.generation.0.to_vec()).fetch_one(&mut *tx).await?;
            if !grouped {
                expected = sqlx::query_as("SELECT row_count,content_digest FROM lctx_model_store.stage_receipts WHERE generation_id=$1 AND relation_name=$2")
                    .bind(self.generation.0.to_vec()).bind(input.name()).fetch_optional(&mut *tx).await?;
            }
        }
        if expected.is_none() && physical == input.name() {
            // Final validation freezes every held relation, including explicitly empty
            // unrequested families. Its final receipt is owned acknowledgement too;
            // this does not manufacture producer stage/source authority.
            expected = sqlx::query_as("SELECT r.row_count,r.content_digest FROM lctx_model_store.receipts r JOIN lctx_model_store.generations g ON g.id=r.generation_id WHERE r.generation_id=$1 AND r.relation_name=$2 AND g.state IN ('validated','published')")
                .bind(self.generation.0.to_vec()).bind(input.name()).fetch_optional(&mut *tx).await?;
        }
        if expected.is_none() && physical == input.name()
            && !lctx_model::domain::stages::is_vocabulary(input.name())
            && let Some(checkpoint) = &self.checkpoint
            && checkpoint.covered.contains(input.name()) {
            let _lookup = self.budget.reserve("checkpoint-frame-read", 512)?;
            expected = sqlx::query_as("SELECT r.row_count,r.content_digest FROM lctx_model_store.checkpoint_frame_receipts r JOIN lctx_model_store.checkpoints c USING(generation_id,frontier) JOIN lctx_model_store.generations g ON g.id=r.generation_id WHERE r.generation_id=$1 AND r.frontier=$2 AND r.relation_name=$3 AND r.physical_frame=$4 AND c.contract_digest=$5 AND c.model_digest=$6 AND c.schedule_digest=$7 AND c.coverage_digest=$8 AND c.content_digest=$9 AND g.state='staging' AND g.model_digest=c.model_digest AND g.schedule_digest=c.schedule_digest AND g.physical_digest=$10 AND g.profile=$11")
                .bind(self.generation.0.to_vec()).bind(checkpoint.frontier).bind(input.name()).bind(physical)
                .bind(checkpoint.contract.0.to_vec()).bind(checkpoint.model.0.to_vec()).bind(checkpoint.schedule.0.to_vec())
                .bind(checkpoint.coverage.0.to_vec()).bind(checkpoint.content.0.to_vec()).bind(self.physical.0.to_vec())
                .bind(checkpoint.profile.name()).fetch_optional(&mut *tx).await?;
        }
        let receipt = match expected {
            Some((rows, content)) => RelationReceipt { rows: u64::try_from(rows).map_err(|_| Error::Contract)?,
                content: ContentHash(content.try_into().map_err(|_| Error::Contract)?) },
            None if hash_missing => {
                let relation = self.model.relation(input.name()).ok_or(Error::Contract)?;
                self.stats.physical_frames += 1;
                self.stats.row_scans += 1;
                super::vocabulary::receipt(tx, self.generation, relation, physical, self.budget).await?
            },
            None => return Ok(None),
        };
        self.retain_frame(input, physical, receipt)?;
        Ok(Some(receipt))
    }

    fn retain_frame(&mut self, input: &ValidationInput, physical: &str, receipt: RelationReceipt) -> Result<(), Error> {
        self.charge.try_resize(self.charge.size().checked_add(512 + physical.len())
            .ok_or(Error::Contract)?)?;
        self.frames.entry(input.name()).or_default().insert(physical.to_owned(), receipt);
        self.stats.resolved_frames += 1;
        Ok(())
    }

    pub async fn binding(&mut self, tx: &mut PgConnection, definition: ContentHash,
        inputs: &[ValidationInput], physical: &[String], candidate: bool,
        context: Option<ContentHash>) -> Result<ContentHash, Error> {
        if inputs.len() != physical.len() { return Err(Error::Contract); }
        let mut sink = KeySink::new("validation-binding/v1");
        sink.part(b"envelope", &self.envelope.0);
        sink.part(b"definition", &definition.0);
        if let Some(context) = context { sink.part(b"context", &context.0); }
        for (input, physical) in inputs.iter().zip(physical) {
            let receipt = self.frame(tx, input, physical, candidate).await?;
            sink.part(b"relation", input.name().as_bytes());
            let logical = if physical == input.name() && lctx_model::domain::stages::is_vocabulary(input.name()) {
                let epoch: Option<i16> = sqlx::query_scalar("SELECT max(r.epoch) FROM lctx_model_store.epoch_receipts r JOIN lctx_model_store.publication_groups p USING(generation_id,epoch) WHERE r.generation_id=$1 AND r.relation_name=$2 AND p.closed")
                    .bind(self.generation.0.to_vec()).bind(input.name()).fetch_one(&mut *tx).await?;
                epoch.map_or_else(|| physical.clone(), |epoch| format!("__v{epoch}_{}", input.name()))
            } else { physical.clone() };
            sink.part(b"physical-frame", logical.as_bytes());
            sink.part(b"rows", &receipt.rows.to_le_bytes());
            sink.part(b"content", &receipt.content.0);
        }
        Ok(sink.finish())
    }

    pub fn metadata_binding(&self, definition: ContentHash, inputs: ContentHash) -> ContentHash {
        let mut sink = KeySink::new("validation-metadata-binding/v1");
        sink.part(b"envelope", &self.envelope.0);
        sink.part(b"definition", &definition.0);
        sink.part(b"inputs", &inputs.0);
        sink.finish()
    }

    pub async fn has(&mut self, tx: &mut PgConnection, binding: ContentHash,
        definition: ContentHash) -> Result<bool, Error> {
        let hit: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM lctx_model_store.validation_receipts WHERE generation_id=$1 AND binding_digest=$2 AND definition_digest=$3 AND model_digest=$4 AND physical_digest=$5)")
            .bind(self.generation.0.to_vec()).bind(binding.0.to_vec()).bind(definition.0.to_vec())
            .bind(self.model.digest().0.to_vec()).bind(self.physical.0.to_vec()).fetch_one(&mut *tx).await?;
        if hit { self.stats.proof_hits += 1; }
        Ok(hit)
    }

    pub async fn acknowledge(&self, tx: &mut PgConnection, name: &str,
        definition: ContentHash, binding: ContentHash, context: &ProofContext) -> Result<(), Error> {
        let context = serde_json::to_value(context).map_err(|e| Error::Codec(e.to_string()))?;
        sqlx::query("INSERT INTO lctx_model_store.validation_receipts(generation_id,validator_name,definition_digest,model_digest,physical_digest,binding_digest,proof_context) VALUES($1,$2,$3,$4,$5,$6,$7) ON CONFLICT(generation_id,binding_digest) DO NOTHING")
            .bind(self.generation.0.to_vec()).bind(name).bind(definition.0.to_vec())
            .bind(self.model.digest().0.to_vec()).bind(self.physical.0.to_vec()).bind(binding.0.to_vec())
            .bind(context).execute(&mut *tx).await?;
        Ok(())
    }

    async fn proof_frames(&self, tx: &mut PgConnection, inputs: &[ValidationInput], frames: &[String]) -> Result<Vec<(String, String)>, Error> {
        let mut stored = Vec::new();
        for (input, physical) in inputs.iter().zip(frames) {
            let logical = if physical == input.name() && lctx_model::domain::stages::is_vocabulary(input.name()) {
                let epoch: Option<i16> = sqlx::query_scalar("SELECT max(r.epoch) FROM lctx_model_store.epoch_receipts r JOIN lctx_model_store.publication_groups p USING(generation_id,epoch) WHERE r.generation_id=$1 AND r.relation_name=$2 AND p.closed")
                    .bind(self.generation.0.to_vec()).bind(input.name()).fetch_one(&mut *tx).await?;
                epoch.map_or_else(|| physical.clone(), |epoch| format!("__v{epoch}_{}", input.name()))
            } else { physical.clone() };
            stored.push((input.name().to_owned(), logical));
        }
        Ok(stored)
    }

    async fn acknowledge_invariant(&self, tx: &mut PgConnection, invariant: &Invariant,
        frames: &[String], binding: ContentHash) -> Result<(), Error> {
        let _wire = self.budget.reserve("validation-receipt-wire", 1024 + frames.len().saturating_mul(1024))?;
        let frames = self.proof_frames(tx, &invariant.inputs, frames).await?;
        self.acknowledge(tx, invariant.name, invariant.digest(), binding,
            &ProofContext::Invariant { frames }).await
    }

    pub async fn invariant(&mut self, tx: &mut PgConnection, invariant: &Invariant,
        physical: Vec<String>, candidate: bool) -> Result<ContentHash, Error> {
        let binding = self.execute_invariant(tx, invariant, &physical, candidate).await?;
        self.acknowledge_invariant(tx, invariant, &physical, binding).await?;
        Ok(binding)
    }

    async fn execute_invariant(&mut self, tx: &mut PgConnection, invariant: &Invariant,
        physical: &[String], candidate: bool) -> Result<ContentHash, Error> {
        let binding = self.binding(tx, invariant.digest(), &invariant.inputs, physical, candidate, None).await?;
        if self.has(tx, binding, invariant.digest()).await? { return Ok(binding); }
        let mut check = (invariant.create)(self.budget);
        self.stats.check_executions += 1;
        for (input, physical) in invariant.inputs.iter().zip(physical.iter()) {
            let relation = self.model.relation(input.name()).ok_or(Error::Contract)?;
            self.stats.row_scans += 1;
            visit_named(tx, self.generation, relation, physical, input.order(), self.budget,
                |batch| { check.visit_input(input, &batch)?; Ok(()) }).await?;
        }
        check.finish()?;
        Ok(binding)
    }

    /// Share only the next ordered input of each checker. This respects checker-local
    /// dependency order without retaining or sorting an entire relation.
    pub async fn invariants(&mut self, tx: &mut PgConnection,
        requests: &[(&Invariant, Vec<String>)], candidate: bool) -> Result<(), Error> {
        let _metadata = self.budget.reserve("validation-plan", requests.len().saturating_mul(512))?;
        let mut pending = Vec::new();
        for (index, (invariant, frames)) in requests.iter().enumerate() {
            if requests[..index].iter().any(|(other, existing)|
                other.digest() == invariant.digest() && existing == frames) { continue; }
            if invariant.inputs.len() != frames.len() { return Err(Error::Contract); }
            let mut complete = true;
            for (input, physical) in invariant.inputs.iter().zip(frames) {
                if self.prepare_frame(tx, input, physical, false).await?.is_none() {
                    if !candidate { return Err(Error::Contract); }
                    complete = false;
                }
            }
            let binding = if complete {
                let binding = self.binding(tx, invariant.digest(), &invariant.inputs, frames, false, None).await?;
                if self.has(tx, binding, invariant.digest()).await? { continue; }
                Some(binding)
            } else { None };
            pending.push((*invariant, frames, binding));
        }
        let result = self.shared_checks(tx, &pending).await;
        match result {
            Err(Error::Model(lctx_model::domain::ModelError::Resource { .. })) if pending.len() > 1 => {
                // All shared states have dropped. The bounded serial fallback admits each
                // check separately; resource failure is never recorded as a conclusion.
                let mut conclusions = Vec::new();
                for (invariant, frames, _) in &pending {
                    conclusions.push(self.execute_invariant(tx, invariant, frames, candidate).await?);
                }
                for ((invariant, frames, _), binding) in pending.iter().zip(conclusions) {
                    self.acknowledge_invariant(tx, invariant, frames, binding).await?;
                }
                Ok(())
            }
            other => other,
        }
    }

    async fn shared_checks(&mut self, tx: &mut PgConnection,
        pending: &[(&Invariant, &Vec<String>, Option<ContentHash>)]) -> Result<(), Error> {
        let mut checks: Vec<_> = pending.iter().map(|(i, _, _)| (i.create)(self.budget)).collect();
        let mut positions = vec![0; pending.len()];
        self.stats.check_executions += pending.len() as u64;
        while let Some(first) = (0..pending.len()).find(|&n| positions[n] < pending[n].0.inputs.len()) {
            let (definition, frames, _) = pending[first];
            let index = positions[first];
            let input = &definition.inputs[index];
            let physical = &frames[index];
            let compatible: Vec<_> = (0..pending.len()).filter(|&n| {
                let (other, frames, _) = pending[n];
                let index = positions[n];
                index < other.inputs.len() && frames[index] == *physical
                    && other.inputs[index].name() == input.name()
                    && other.inputs[index].order() == input.order()
            }).collect();
            let relation = self.model.relation(input.name()).ok_or(Error::Contract)?;
            // A new candidate's canonical content and compatible semantic checkers
            // consume the same bounded ID-ordered stream. Other orders need their own scan.
            let needs_digest = input.order() == ["id"]
                && self.frames.get(input.name()).and_then(|versions| versions.get(physical)).is_none();
            let _digest_charge = needs_digest.then(|| self.budget.reserve("validation-content", 512)).transpose()?;
            let mut content = needs_digest.then(|| relation.content());
            self.stats.row_scans += 1;
            visit_named(tx, self.generation, relation, physical, input.order(), self.budget, |batch| {
                if let Some(content) = &mut content { relation.hash_rows(&batch, content)?; }
                for &n in &compatible {
                    checks[n].visit_input(&pending[n].0.inputs[positions[n]], &batch)?;
                }
                Ok(())
            }).await?;
            if let Some(content) = content {
                let (rows, content) = content.finish();
                self.retain_frame(input, physical, RelationReceipt { rows, content })?;
                self.stats.physical_frames += 1;
            }
            for n in compatible { positions[n] += 1; }
        }
        // Complete every state before acknowledging any result in this execution group.
        for check in checks { check.finish()?; }
        for (definition, frames, binding) in pending {
            let binding = match binding {
                Some(binding) => *binding,
                None => self.binding(tx, definition.digest(), &definition.inputs, frames, true, None).await?,
            };
            self.acknowledge_invariant(tx, definition, frames, binding).await?;
        }
        Ok(())
    }

    /// Publication checks share actual source snapshots and compatible next-input streams.
    /// Pure checks may use this same session's receipts, but execute in a separate group.
    pub async fn publications(&mut self, tx: &mut PgConnection,
        requests: &[(&PublicationInvariant, Vec<String>)], sources: &[CompletedRelation],
        profile: Profile) -> Result<(), Error> {
        if requests.is_empty() { return Ok(()); }
        let source_bytes = sources.iter().try_fold(0usize, |bytes, source| {
            bytes.checked_add(512)?.checked_add(source.relation().len())?
                .checked_add(source.producer().len())?.checked_add(source.relation().len())?
                .checked_add(source.prefix().map_or(0, |prefix| prefix.name().len()))
        }).ok_or(Error::Contract)?;
        let _source_charge = self.budget.reserve("publication-source-context", source_bytes)?;
        let snapshots = sources.iter().map(lctx_model::domain::analysis::sources::SourceSnapshot::from_source)
            .collect::<Result<Vec<_>, _>>()?;
        let context = publication_context(&snapshots, profile);
        let _metadata = self.budget.reserve("publication-validation-plan",
            requests.len().checked_mul(512).ok_or(Error::Contract)?)?;
        let mut pending = Vec::with_capacity(requests.len());
        for (index, (definition, frames)) in requests.iter().enumerate() {
            if definition.inputs.len() != frames.len() { return Err(Error::Contract); }
            if requests[..index].iter().any(|(other, existing)|
                other.digest() == definition.digest() && existing == frames) { continue; }
            let mut complete = true;
            for (input, physical) in definition.inputs.iter().zip(frames) {
                if self.prepare_frame(tx, input, physical, false).await?.is_none() { complete = false; }
            }
            let binding = if complete {
                let binding = self.binding(tx, definition.digest(), &definition.inputs, frames, false, Some(context)).await?;
                if self.has(tx, binding, definition.digest()).await? { continue; }
                Some(binding)
            } else { None };
            pending.push((*definition, frames, binding));
        }
        let conclusions = match self.shared_publication_checks(tx, &pending, &snapshots, profile, context).await {
            Err(Error::Model(lctx_model::domain::ModelError::Resource { .. })) if pending.len() > 1 => {
                // Shared checker state is dropped before serial admission. Keep every
                // conclusion local until the whole fallback group has finished successfully.
                let mut conclusions = Vec::with_capacity(pending.len());
                for request in &pending {
                    let bindings = self.shared_publication_checks(tx, std::slice::from_ref(request),
                        &snapshots, profile, context).await?;
                    conclusions.push(bindings[0]);
                }
                conclusions
            },
            other => other?,
        };
        for ((definition, frames, _), binding) in pending.iter().zip(conclusions) {
            self.acknowledge_publication(tx, definition, frames, binding, &snapshots, profile).await?;
        }
        Ok(())
    }

    async fn shared_publication_checks(&mut self, tx: &mut PgConnection,
        pending: &[(&PublicationInvariant, &Vec<String>, Option<ContentHash>)],
        snapshots: &[lctx_model::domain::analysis::sources::SourceSnapshot],
        profile: Profile, context: ContentHash) -> Result<Vec<ContentHash>, Error> {
        // Charge vectors, indices and conclusion slots before constructing any checker.
        // Each checker additionally owns the reservations for its retained semantic state.
        let _execution = self.budget.reserve("publication-validation-execution",
            pending.len().checked_mul(128).ok_or(Error::Contract)?)?;
        let mut checks: Vec<_> = pending.iter().map(|(i, _, _)| (i.create)(self.budget)).collect();
        let mut positions = vec![0; pending.len()];
        self.stats.check_executions += pending.len() as u64;
        while let Some(first) = (0..pending.len()).find(|&n| positions[n] < pending[n].0.inputs.len()) {
            let (definition, frames, _) = pending[first];
            let index = positions[first];
            let input = &definition.inputs[index];
            let physical = &frames[index];
            let compatible: Vec<_> = (0..pending.len()).filter(|&n| {
                let (other, frames, _) = pending[n];
                let index = positions[n];
                index < other.inputs.len() && frames[index] == *physical
                    && other.inputs[index].name() == input.name()
                    && other.inputs[index].order() == input.order()
            }).collect();
            let relation = self.model.relation(input.name()).ok_or(Error::Contract)?;
            let needs_digest = input.order() == ["id"]
                && self.frames.get(input.name()).and_then(|versions| versions.get(physical)).is_none();
            let _digest_charge = needs_digest.then(|| self.budget.reserve("validation-content", 512)).transpose()?;
            let mut content = needs_digest.then(|| relation.content());
            self.stats.row_scans += 1;
            visit_named(tx, self.generation, relation, physical, input.order(), self.budget, |batch| {
                if let Some(content) = &mut content { relation.hash_rows(&batch, content)?; }
                for &n in &compatible {
                    checks[n].visit_input(&pending[n].0.inputs[positions[n]], &batch)?;
                }
                Ok(())
            }).await?;
            if let Some(content) = content {
                let (rows, content) = content.finish();
                self.retain_frame(input, physical, RelationReceipt { rows, content })?;
                self.stats.physical_frames += 1;
            }
            for n in compatible { positions[n] += 1; }
        }
        for check in checks { check.finish(snapshots, profile)?; }
        let mut conclusions = Vec::with_capacity(pending.len());
        for (definition, frames, binding) in pending {
            conclusions.push(match binding {
                Some(binding) => *binding,
                None => self.binding(tx, definition.digest(), &definition.inputs, frames, true, Some(context)).await?,
            });
        }
        Ok(conclusions)
    }

    async fn acknowledge_publication(&self, tx: &mut PgConnection, definition: &PublicationInvariant,
        physical: &[String], binding: ContentHash,
        snapshots: &[lctx_model::domain::analysis::sources::SourceSnapshot], profile: Profile) -> Result<(), Error> {
        use lctx_model::domain::HeapSize;
        let source_bytes = snapshots.iter().try_fold(0usize, |bytes, source|
            bytes.checked_add(4096)?.checked_add(source.heap_bytes().checked_mul(4)?))
            .ok_or(Error::Contract)?;
        let bytes = physical.iter().try_fold(1024usize, |bytes, frame|
            bytes.checked_add(1024)?.checked_add(frame.len()))
            .and_then(|bytes| bytes.checked_add(source_bytes))
            .ok_or(Error::Contract)?;
        let _wire = self.budget.reserve("publication-receipt-wire", bytes)?;
        let frames = self.proof_frames(tx, &definition.inputs, physical).await?;
        self.acknowledge(tx, definition.name, definition.digest(), binding,
            &ProofContext::Publication { frames, sources: snapshots.to_vec(), profile: profile.name().to_owned() }).await
    }

}

pub(super) fn publication_context(sources: &[lctx_model::domain::analysis::sources::SourceSnapshot], profile: Profile) -> ContentHash {
    let mut context = KeySink::new("publication-authority/v2");
    context.part(b"profile", profile.name().as_bytes());
    let mut ordered: Vec<_> = sources.iter().collect();
    ordered.sort_by_key(|source| (source.relation(), source.producer()));
    for source in ordered {
        context.part(b"relation", source.relation().as_bytes());
        context.part(b"producer", source.producer().as_bytes());
        context.part(b"model", &source.model().0);
        context.part(b"schedule", &source.schedule().0);
        context.part(b"has-prefix", &[u8::from(source.prefix().is_some())]);
        if let Some(prefix) = source.prefix() { context.part(b"prefix", prefix.as_bytes()); }
        context.part(b"frame", source.physical().as_bytes());
        context.part(b"rows", &source.rows().to_le_bytes());
        context.part(b"content", &source.content().0);
    }
    context.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use lctx_model::{Domain, domain::{Batch, InvariantCheck, ModelError, Record, Relation,
        ValidatedModel, ValidationDefinitions}};
    use crate::testing::{DisposableDatabase, Harness};
    use std::sync::Arc;

    fn references() -> Vec<&'static str> { vec!["first_count", "second_count"] }
    #[derive(Debug, Clone, PartialEq, Eq, Domain)]
    #[model(name="proof_items", semantic_source=include_bytes!("validation_session.rs"), invariant_refs=references)]
    struct Item { #[model(key)] name: String }

    struct Count { rows: usize, expected: usize }
    impl InvariantCheck for Count {
        fn visit(&mut self, _: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
            self.rows += batch.num_rows(); Ok(())
        }
        fn finish(self: Box<Self>) -> Result<(), ModelError> {
            if self.rows == self.expected { Ok(()) }
            else { Err(ModelError::Invalid("independent authored row count differs".into())) }
        }
    }
    struct ChargedCount { count: Count, _charge: Option<Box<dyn Reservation>>, refusal: Option<ModelError> }
    impl InvariantCheck for ChargedCount {
        fn visit(&mut self, name: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
            if let Some(error) = self.refusal.take() { return Err(error); }
            self.count.visit(name, batch)
        }
        fn finish(self: Box<Self>) -> Result<(), ModelError> { Box::new(self.count).finish() }
    }
    fn charged_definition(name: &'static str, expected: usize) -> Invariant {
        let mut check = definition(name, expected); check.revision = 3;
        check.create = Arc::new(move |budget| {
            let (charge, refusal) = match budget.reserve("independent-charged-count", 10000) {
                Ok(charge) => (Some(charge), None), Err(error) => (None, Some(error)),
            };
            Box::new(ChargedCount { count: Count { rows: 0, expected }, _charge: charge, refusal })
        }); check
    }

    fn definition(name: &'static str, expected: usize) -> Invariant {
        Invariant { name, revision: 1, inputs: vec![ValidationInput::of::<Item>(&["id"])],
            create: Arc::new(move |_| Box::new(Count { rows: 0, expected })) }
    }

    #[tokio::test]
    async fn compatible_checks_share_streams_complete_proofs_reuse_and_revision_misses() {
        let db = DisposableDatabase::start().await;
        let model = Arc::new(ValidatedModel::validate(vec![Relation::of::<Item>()],
            ValidationDefinitions { invariants: vec![definition("first_count", 1), definition("second_count", 1)],
                publication_checks: vec![] }).unwrap());
        let store = GenerationStore::install(db.owner.clone(), model.clone()).await.unwrap();
        let budget = ResourceBudget::fixed(64 << 20).unwrap();
        let mut harness = Harness::begin(&store, db.writer.clone(), Profile::Catalog, budget.clone()).await.unwrap();
        harness.copy(&Batch::new(&model, vec![Item { name: "one".into() }], &budget).unwrap(), &budget).await.unwrap();
        harness.seal().await.unwrap();
        let generation = harness.generation();
        super::super::transaction(&store.owner, async |tx| {
            store.lock_installation(tx).await?;
            super::super::lock(tx, generation, true).await?;
            let acknowledged: Option<(i64, Vec<u8>)> = sqlx::query_as("DELETE FROM lctx_model_store.stage_receipts WHERE generation_id=$1 AND relation_name='proof_items' RETURNING row_count,content_digest")
                .bind(generation.0.to_vec()).fetch_optional(&mut *tx).await?;
            assert!(acknowledged.is_some());
            let requests: Vec<_> = model.invariants().iter().map(|i| (i, vec!["proof_items".into()])).collect();
            let mut session = Session::new(&store, tx, generation, &budget).await?;
            session.invariants(tx, &requests, true).await?;
            assert_eq!(session.stats.physical_frames, 1);
            assert_eq!(session.stats.row_scans, 1, "canonical digest shares the two compatible checkers' stream");
            // This control deliberately rolls the candidate conclusion back. The following
            // transaction must establish it again; no uncommitted success survives.
            Err::<(), Error>(Error::Contract)
        }).await.unwrap_err();
        super::super::transaction(&store.owner, async |tx| {
            store.lock_installation(tx).await?;
            super::super::lock(tx, generation, true).await?;
            let requests: Vec<_> = model.invariants().iter().map(|i| (i, vec!["proof_items".into()])).collect();
            let mut session = Session::new(&store, tx, generation, &budget).await?;
            let mut duplicated = requests.clone(); duplicated.push(requests[0].clone());
            session.invariants(tx, &duplicated, false).await?;
            assert_eq!(session.stats.resolved_frames, 1);
            assert_eq!(session.stats.row_scans, 1);
            assert_eq!(session.stats.check_executions, 2);
            assert_eq!(session.stats.proof_hits, 0);
            Ok(())
        }).await.unwrap();
        super::super::transaction(&store.owner, async |tx| {
            let requests: Vec<_> = model.invariants().iter().map(|i| (i, vec!["proof_items".into()])).collect();
            let mut session = Session::new(&store, tx, generation, &budget).await?;
            session.invariants(tx, &requests, false).await?;
            assert_eq!(session.stats.proof_hits, 2);
            assert_eq!(session.stats.row_scans, 0);
            assert_eq!(session.stats.check_executions, 0);
            let original = &model.invariants()[0];
            let old = session.binding(tx, original.digest(), &original.inputs, &requests[0].1, false, None).await?;
            let mut revised = original.clone(); revised.revision += 1;
            let new = session.binding(tx, revised.digest(), &revised.inputs, &requests[0].1, false, None).await?;
            assert_ne!(old, new);
            assert!(!session.has(tx, new, revised.digest()).await?);
            for (update, restore) in [
                ("UPDATE lctx_model_store.generations SET profile='behavioral' WHERE id=$1", "UPDATE lctx_model_store.generations SET profile='catalog' WHERE id=$1"),
                ("UPDATE lctx_model_store.generations SET producer_digest=decode(repeat('ab',32),'hex') WHERE id=$1", ""),
            ] {
                let producer: Vec<u8> = sqlx::query_scalar("SELECT producer_digest FROM lctx_model_store.generations WHERE id=$1").bind(generation.0.to_vec()).fetch_one(&mut *tx).await?;
                sqlx::query(update).bind(generation.0.to_vec()).execute(&mut *tx).await?;
                let mut rebound = Session::new(&store, tx, generation, &budget).await?;
                let changed = rebound.binding(tx, original.digest(), &original.inputs, &requests[0].1, false, None).await?;
                assert_ne!(old, changed);
                assert!(!rebound.has(tx, changed, original.digest()).await?);
                if restore.is_empty() {
                    sqlx::query("UPDATE lctx_model_store.generations SET producer_digest=$2 WHERE id=$1").bind(generation.0.to_vec()).bind(producer).execute(&mut *tx).await?;
                } else { sqlx::query(restore).bind(generation.0.to_vec()).execute(&mut *tx).await?; }
            }
            let installation: Vec<u8> = sqlx::query_scalar("SELECT identity FROM lctx_model_store.installation WHERE singleton").fetch_one(&mut *tx).await?;
            sqlx::query("UPDATE lctx_model_store.installation SET identity=decode(repeat('cd',16),'hex') WHERE singleton").execute(&mut *tx).await?;
            let mut reinstalled = Session::new(&store, tx, generation, &budget).await?;
            let changed = reinstalled.binding(tx, original.digest(), &original.inputs, &requests[0].1, false, None).await?;
            assert_ne!(old, changed);
            assert!(!reinstalled.has(tx, changed, original.digest()).await?);
            drop(reinstalled);
            sqlx::query("UPDATE lctx_model_store.installation SET identity=$1 WHERE singleton").bind(installation).execute(&mut *tx).await?;
            let mut failing = definition("second_count", 2); failing.revision = 2;
            assert!(session.invariants(tx, &[( &revised, vec!["proof_items".into()]),
                (&failing, vec!["proof_items".into()])], false).await.is_err());
            assert!(!session.has(tx, new, revised.digest()).await?, "a sibling finish failure acknowledges no result");
            let fallback_budget = ResourceBudget::fixed(24576).unwrap();
            let first = charged_definition("first_count", 1);
            let later_failure = charged_definition("second_count", 2);
            let mut fallback = Session::new(&store, tx, generation, &fallback_budget).await?;
            assert!(fallback.invariants(tx, &[(&first, vec!["proof_items".into()]),
                (&later_failure, vec!["proof_items".into()])], false).await.is_err());
            let first_binding = fallback.binding(tx, first.digest(), &first.inputs, &requests[0].1, false, None).await?;
            assert!(!fallback.has(tx, first_binding, first.digest()).await?, "serial fallback also acknowledges no sibling before complete group success");
            drop(fallback);
            assert_eq!(fallback_budget.reserved(), 0);
            let tiny = ResourceBudget::fixed(1).unwrap();
            assert!(Session::new(&store, tx, generation, &tiny).await.is_err());
            assert_eq!(tiny.reserved(), 0);
            let plan_budget = ResourceBudget::fixed(4600).unwrap();
            let mut constrained = Session::new(&store, tx, generation, &plan_budget).await?;
            assert!(constrained.invariants(tx, &requests, false).await.is_err());
            drop(constrained);
            assert_eq!(plan_budget.reserved(), 0, "failed plan admission releases the session");
            Ok(())
        }).await.unwrap();
        harness.validate(&budget).await.unwrap();
        harness.publish().await.unwrap();
        store.audit(generation, lctx_model::domain::admission::Frontier::Conformance, None, &budget).await.unwrap();
        sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE {}.proof_items SET name='privileged damage'", generation.schema())))
            .execute(&db.superuser).await.unwrap();
        assert!(store.audit(generation, lctx_model::domain::admission::Frontier::Conformance, None, &budget).await.is_err());
        sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE {}.proof_items SET name='one'", generation.schema())))
            .execute(&db.superuser).await.unwrap();
        let saved: Vec<(String, Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>, serde_json::Value)> = sqlx::query_as("SELECT validator_name,definition_digest,model_digest,physical_digest,binding_digest,proof_context FROM lctx_model_store.validation_receipts WHERE generation_id=$1")
            .bind(generation.0.to_vec()).fetch_all(&db.superuser).await.unwrap();
        sqlx::query("DELETE FROM lctx_model_store.validation_receipts WHERE generation_id=$1").bind(generation.0.to_vec()).execute(&db.superuser).await.unwrap();
        assert!(store.audit(generation, lctx_model::domain::admission::Frontier::Conformance, None, &budget).await.is_err(), "deleting complete conclusions is an audit discrepancy");
        for (name, definition, model, physical, binding, context) in saved {
            sqlx::query("INSERT INTO lctx_model_store.validation_receipts(generation_id,validator_name,definition_digest,model_digest,physical_digest,binding_digest,proof_context) VALUES($1,$2,$3,$4,$5,$6,$7)")
                .bind(generation.0.to_vec()).bind(name).bind(definition).bind(model).bind(physical).bind(binding).bind(context).execute(&db.superuser).await.unwrap();
        }
        store.audit(generation, lctx_model::domain::admission::Frontier::Conformance, None, &budget).await.unwrap();
        sqlx::query("UPDATE lctx_model_store.validation_receipts SET binding_digest=decode(repeat('ef',32),'hex') WHERE generation_id=$1 AND validator_name='first_count'")
            .bind(generation.0.to_vec()).execute(&db.superuser).await.unwrap();
        assert!(store.audit(generation, lctx_model::domain::admission::Frontier::Conformance, None, &budget).await.is_err(), "substituted exact binding is refused");
        store.retire(generation).await.unwrap();
    }
    #[tokio::test]
    async fn committed_checkpoint_owns_unproduced_ordinary_frames_and_excludes_late_writers() {
        use crate::testing::fixtures::Facts;
        use lctx_model::domain::{admission::Frontier, flow::FlowTestLeafObservation, value::Literal};
        use std::time::Duration;
        let db = DisposableDatabase::start().await;
        let fixture = Facts::new();
        let store = GenerationStore::install(db.owner.clone(), fixture.model.clone()).await.unwrap();
        let schedule = fixture.schedule();
        let preflight = fixture.contract().checkpoint_preflight(&schedule).unwrap();
        let (attempt, receipt) = fixture.written(&store, db.writer.clone(), true).await;
        let generation = attempt.generation();
        let budget = ResourceBudget::fixed(256 << 20).unwrap();
        let input = ValidationInput::of::<FlowTestLeafObservation>(&["id"]);
        let frames = vec![FlowTestLeafObservation::NAME.to_owned()];
        let definition = Invariant { name: "checkpoint_empty_flow_question", revision: 1,
            inputs: vec![input.clone()], create: Arc::new(|_| Box::new(Count { rows: 0, expected: 0 })) };
        let sources: i64 = sqlx::query_scalar("SELECT count(*) FROM lctx_model_store.stage_receipts WHERE generation_id=$1 AND relation_name=$2")
            .bind(generation.0.to_vec()).bind(input.name()).fetch_one(&db.superuser).await.unwrap();
        assert_eq!(sources, 0, "Catalog never fabricates a Flow producer");
        super::super::transaction(&store.owner, async |tx| {
            let mut session = Session::new(&store, tx, generation, &budget).await?;
            assert!(matches!(session.frame(tx, &input, input.name(), false).await, Err(Error::Contract)),
                "table existence is not acknowledgement");
            Ok(())
        }).await.unwrap();

        // An admission returned inside a transaction is unusable after its rollback.
        let mut tx = store.owner.begin().await.unwrap();
        let rolled_back = store.checkpoint_step(&mut tx, generation, &budget, &preflight, &receipt).await.unwrap();
        tx.rollback().await.unwrap();
        super::super::transaction(&store.owner, async |tx| {
            let mut session = Session::new(&store, tx, generation, &budget).await?;
            session.bind_checkpoint(&store, &rolled_back)?;
            assert!(matches!(session.frame(tx, &input, input.name(), false).await, Err(Error::Contract)));
            let count: i64 = sqlx::query_scalar("SELECT count(*) FROM lctx_model_store.checkpoint_frame_receipts WHERE generation_id=$1")
                .bind(generation.0.to_vec()).fetch_one(&mut *tx).await?;
            assert_eq!(count, 0, "rollback exposes neither parent nor frame conclusions");
            Ok(())
        }).await.unwrap();

        // A real importer statement holds RowExclusiveLock even for an empty write. Closing
        // this unproduced frame must drain that transaction and exclude a later queued writer.
        let physical = format!("{}.{}", generation.schema(), input.name());
        let write = format!("INSERT INTO {physical}(id,qualification,test,atom,operand) SELECT decode(repeat('01',16),'hex'),decode(repeat('02',16),'hex'),decode(repeat('03',16),'hex'),decode(repeat('04',16),'hex'),NULL WHERE false");
        let mut prior = db.writer.begin().await.unwrap();
        sqlx::query(sqlx::AssertSqlSafe(write.clone())).execute(&mut *prior).await.unwrap();
        let admission = {
            let closing = super::super::transaction(&store.owner, async |tx|
                store.checkpoint_step(tx, generation, &budget, &preflight, &receipt).await);
            tokio::pin!(closing);
            let wait_for_lock = async {
                loop {
                    let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_locks WHERE relation=to_regclass($1) AND mode='AccessExclusiveLock' AND NOT granted)")
                        .bind(&physical).fetch_one(&db.superuser).await.unwrap();
                    if waiting { break; }
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
            };
            tokio::select! {
                completed = &mut closing => panic!("checkpoint did not drain the writer: {completed:?}"),
                waited = tokio::time::timeout(Duration::from_secs(5), wait_for_lock) =>
                    assert!(waited.is_ok(), "checkpoint queues its frame lock before the later writer"),
            }
            let writer = db.writer.clone();
            let mut queued = tokio::spawn(async move {
                sqlx::query(sqlx::AssertSqlSafe(write)).execute(&writer).await
            });
            assert!(tokio::time::timeout(Duration::from_millis(100), &mut queued).await.is_err());
            let queued_write: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_locks WHERE relation=to_regclass($1) AND mode='RowExclusiveLock' AND NOT granted)")
                .bind(&physical).fetch_one(&db.superuser).await.unwrap();
            assert!(queued_write, "the later importer is actually queued behind checkpoint closure");
            prior.commit().await.unwrap();
            let admission = closing.await.unwrap();
            let refusal = queued.await.unwrap().unwrap_err();
            assert_eq!(refusal.as_database_error().and_then(|error| error.code()).as_deref(), Some("42501"),
                "queued write rechecks revoked importer permission");
            admission
        };
        assert_eq!(admission.frontier(), Frontier::Facts);
        let frame: (i64, String) = sqlx::query_as("SELECT row_count,physical_frame FROM lctx_model_store.checkpoint_frame_receipts WHERE generation_id=$1 AND frontier='facts' AND relation_name=$2")
            .bind(generation.0.to_vec()).bind(input.name()).fetch_one(&db.superuser).await.unwrap();
        assert_eq!(frame, (0, input.name().to_owned()));
        let vocabulary: i64 = sqlx::query_scalar("SELECT count(*) FROM lctx_model_store.checkpoint_frame_receipts WHERE generation_id=$1 AND relation_name=$2")
            .bind(generation.0.to_vec()).bind(Literal::NAME).fetch_one(&db.superuser).await.unwrap();
        assert_eq!(vocabulary, 0, "growing vocabulary stays owned by exact epoch acknowledgements");
        let expected = super::super::transaction(&store.owner, async |tx| {
            let mut unbound = Session::new(&store, tx, generation, &budget).await?;
            assert!(unbound.frame(tx, &input, input.name(), false).await.is_err(),
                "a committed checkpoint still needs explicit covered authority");
            let mut session = Session::new(&store, tx, generation, &budget).await?;
            session.bind_checkpoint(&store, &admission)?;
            let binding = session.binding(tx, definition.digest(), &definition.inputs, &frames, false, None).await?;
            assert_eq!(session.stats.row_scans, 0, "acknowledged content resolves without rehash");
            session.invariant(tx, &definition, frames.clone(), false).await?;
            assert_eq!(session.stats.row_scans, 1, "a new semantic question still executes");
            Ok(binding)
        }).await.unwrap();
        super::super::transaction(&store.owner, async |tx| {
            let mut session = Session::new(&store, tx, generation, &budget).await?;
            session.bind_checkpoint(&store, &admission)?;
            let binding = session.invariant(tx, &definition, frames.clone(), false).await?;
            assert_eq!(binding, expected, "the declared Flow premise keeps its exact binding");
            assert_eq!(session.stats.proof_hits, 1);
            assert_eq!(session.stats.row_scans, 0);
            for column in ["contract_digest", "model_digest", "schedule_digest", "coverage_digest", "content_digest"] {
                // Scoped privileged injection challenges identity lookup; rollback restores it.
                sqlx::query("SAVEPOINT mismatched_checkpoint").execute(&mut *tx).await?;
                sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE lctx_model_store.checkpoints SET {column}=decode(repeat('ef',32),'hex') WHERE generation_id=$1 AND frontier='facts'")))
                    .bind(generation.0.to_vec()).execute(&mut *tx).await?;
                let mut mismatched = Session::new(&store, tx, generation, &budget).await?;
                mismatched.bind_checkpoint(&store, &admission)?;
                assert!(matches!(mismatched.frame(tx, &input, input.name(), false).await, Err(Error::Contract)),
                    "{column} mismatch cannot authorize a checkpoint frame");
                sqlx::query("ROLLBACK TO SAVEPOINT mismatched_checkpoint").execute(&mut *tx).await?;
            }
            let mut outside = Session::new(&store, tx, generation, &budget).await?;
            outside.bind_checkpoint(&store, &admission)?;
            assert!(outside.frame(tx, &input, "__v0_flow_test_leaf_observations", false).await.is_err(),
                "checkpoint receipts never invent a different physical frame");
            Ok(())
        }).await.unwrap();
        let sources: i64 = sqlx::query_scalar("SELECT count(*) FROM lctx_model_store.stage_receipts WHERE generation_id=$1 AND relation_name=$2")
            .bind(generation.0.to_vec()).bind(input.name()).fetch_one(&db.superuser).await.unwrap();
        assert_eq!(sources, 0, "checkpoint acknowledgement creates no source capability");
        attempt.abort().await.unwrap();
        let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM lctx_model_store.checkpoint_frame_receipts WHERE generation_id=$1")
            .bind(generation.0.to_vec()).fetch_one(&db.superuser).await.unwrap();
        assert_eq!(remaining, 0, "generation cleanup removes checkpoint frame conclusions");
        drop(admission);
        drop(rolled_back);
        assert_eq!(budget.reserved(), 0);
    }

    struct PublicationCount {
        count: Count,
        _charge: Option<Box<dyn Reservation>>,
        refusal: Option<ModelError>,
    }
    impl lctx_model::domain::PublicationCheck for PublicationCount {
        fn visit(&mut self, name: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
            if let Some(error) = self.refusal.take() { return Err(error); }
            self.count.visit(name, batch)
        }
        fn finish(self: Box<Self>, sources: &[lctx_model::domain::analysis::sources::SourceSnapshot],
            profile: Profile) -> Result<(), ModelError> {
            if sources.len() != 1 || sources[0].relation() != Item::NAME || sources[0].rows() != 1
                || profile != Profile::Catalog {
                return Err(ModelError::Invalid("publication differs from authored source/profile answer".into()));
            }
            Box::new(self.count).finish()
        }
    }
    fn publication_definition(name: &'static str, expected: usize, state_bytes: usize) -> PublicationInvariant {
        PublicationInvariant { name, revision: 1, inputs: vec![ValidationInput::of::<Item>(&["id"])],
            create: Arc::new(move |budget| {
                let (charge, refusal) = match budget.reserve("independent-publication-count", state_bytes) {
                    Ok(charge) => (Some(charge), None), Err(error) => (None, Some(error)),
                };
                Box::new(PublicationCount { count: Count { rows: 0, expected }, _charge: charge, refusal })
            }) }
    }

    #[tokio::test]
    async fn publication_group_shares_candidate_and_source_streams_and_stages_fallback_results() {
        use lctx_model::domain::{input::Package, stages::*};
        let db = DisposableDatabase::start().await;
        let first_check = publication_definition("first_publication", 1, 0);
        let second_check = publication_definition("second_publication", 1, 0);
        let model = Arc::new(ValidatedModel::validate(vec![Relation::of::<Item>(), Relation::of::<Package>()],
            ValidationDefinitions { invariants: vec![definition("first_count", 1), definition("second_count", 1)],
                publication_checks: vec![first_check.clone(), second_check.clone()] }).unwrap());
        let store = GenerationStore::install(db.owner.clone(), model.clone()).await.unwrap();
        let stage = |name, inputs, outputs| Stage { name, inputs, outputs,
            contributes: vec![], coverage: vec![], profiles: vec![Profile::Catalog], effect: Effect::Pure,
            code: ContentHash::of(b"publication group fixture"), configuration: ContentHash::of(b"authored source count") };
        let schedule = Schedule::build(&model, vec![
            stage("source", vec![], vec![RelationUse::of::<Item>()]),
            stage("consumer", vec![RelationUse::stored::<Item>()], vec![RelationUse::of::<Package>()]),
        ], &[], Profile::Catalog).unwrap();
        let budget = ResourceBudget::fixed(64 << 20).unwrap();
        let mut execution = schedule.execute();
        let attempt = store.begin_conformance(db.writer.clone(), &mut execution, budget.clone()).await.unwrap();
        let generation = attempt.generation();
        let mut source = execution.begin("source").unwrap();
        let batch = Batch::new(&model, vec![Item { name: "one".into() }], &budget).unwrap();
        source.write::<Item, _>(async |permit| attempt.copy(permit, &batch).await).await.unwrap();
        source.complete(&attempt, ProviderOutcome::Complete).await.unwrap();
        let mut consumer = execution.begin("consumer").unwrap();
        let sources = consumer.completed_sources().unwrap();
        assert_eq!(sources.len(), 1, "fixture supplies a real acknowledged source");
        let batch = Batch::<Package>::new(&model, vec![], &budget).unwrap();
        consumer.write::<Package, _>(async |permit| attempt.copy(permit, &batch).await).await.unwrap();
        consumer.complete(&attempt, ProviderOutcome::Complete).await.unwrap();
        let requests = vec![(&first_check, vec![Item::NAME.into()]), (&second_check, vec![Item::NAME.into()])];
        super::super::transaction(&store.owner, async |tx| {
            store.lock_installation(tx).await?; super::super::lock(tx, generation, true).await?;
            sqlx::query("DELETE FROM lctx_model_store.stage_receipts WHERE generation_id=$1 AND relation_name='proof_items'")
                .bind(generation.0.to_vec()).execute(&mut *tx).await?;
            let mut session = Session::new(&store, tx, generation, &budget).await?;
            session.publications(tx, &requests, &sources, Profile::Catalog).await?;
            assert_eq!(session.stats.row_scans, 1, "candidate digest and both publication checks share ID order");
            assert_eq!(session.stats.physical_frames, 1);
            assert_eq!(session.stats.check_executions, 2);
            Err::<(), Error>(Error::Contract)
        }).await.unwrap_err();
        super::super::transaction(&store.owner, async |tx| {
            store.lock_installation(tx).await?; super::super::lock(tx, generation, true).await?;
            let mut session = Session::new(&store, tx, generation, &budget).await?;
            let mut duplicated = requests.clone(); duplicated.push(requests[0].clone());
            session.publications(tx, &duplicated, &sources, Profile::Catalog).await?;
            assert_eq!(session.stats.row_scans, 1);
            assert_eq!(session.stats.check_executions, 2);
            assert_eq!(session.stats.proof_hits, 0, "rolled-back conclusions cannot answer");
            Ok(())
        }).await.unwrap();
        super::super::transaction(&store.owner, async |tx| {
            store.lock_installation(tx).await?; super::super::lock(tx, generation, true).await?;
            let mut session = Session::new(&store, tx, generation, &budget).await?;
            session.publications(tx, &requests, &sources, Profile::Catalog).await?;
            assert_eq!(session.stats.proof_hits, 2);
            assert_eq!(session.stats.row_scans, 0);
            assert_eq!(session.stats.check_executions, 0);
            let snapshots = sources.iter().map(lctx_model::domain::analysis::sources::SourceSnapshot::from_source)
                .collect::<Result<Vec<_>, _>>()?;
            let context = publication_context(&snapshots, Profile::Catalog);
            let old = session.binding(tx, first_check.digest(), &first_check.inputs, &requests[0].1, false, Some(context)).await?;
            let mut revised = first_check.clone(); revised.revision += 1;
            let revised_binding = session.binding(tx, revised.digest(), &revised.inputs, &requests[0].1, false, Some(context)).await?;
            assert_ne!(old, revised_binding);
            assert!(!session.has(tx, revised_binding, revised.digest()).await?);
            for changed_context in [publication_context(&[], Profile::Catalog), publication_context(&snapshots, Profile::Behavioral)] {
                let changed = session.binding(tx, first_check.digest(), &first_check.inputs, &requests[0].1, false, Some(changed_context)).await?;
                assert_ne!(old, changed);
                assert!(!session.has(tx, changed, first_check.digest()).await?);
            }
            let mut failing = second_check.clone(); failing.revision += 1;
            failing.create = publication_definition("second_publication", 2, 0).create;
            assert!(session.publications(tx, &[(&revised, requests[0].1.clone()), (&failing, requests[1].1.clone())], &sources, Profile::Catalog).await.is_err());
            assert!(!session.has(tx, revised_binding, revised.digest()).await?, "finish failure publishes no sibling conclusion");
            let mut incompatible = first_check.clone(); incompatible.revision = 3;
            incompatible.inputs = vec![ValidationInput::of::<Item>(&["name"])];
            let before = session.stats.row_scans;
            session.publications(tx, &[(&revised, requests[0].1.clone()), (&incompatible, requests[0].1.clone())], &sources, Profile::Catalog).await?;
            assert_eq!(session.stats.row_scans - before, 2, "different total orders remain separate streams");
            let fallback_budget = ResourceBudget::fixed(24576).unwrap();
            let mut charged_first = publication_definition("first_publication", 1, 10000); charged_first.revision = 4;
            let mut charged_second = publication_definition("second_publication", 1, 10000); charged_second.revision = 4;
            let mut fallback = Session::new(&store, tx, generation, &fallback_budget).await?;
            fallback.publications(tx, &[(&charged_first, requests[0].1.clone()), (&charged_second, requests[1].1.clone())], &sources, Profile::Catalog).await?;
            assert!(fallback.stats.check_executions > 2, "aggregate state refusal uses serial admission");
            drop(fallback);
            assert_eq!(fallback_budget.reserved(), 0);
            charged_first.revision = 5; charged_second.revision = 5;
            charged_second.create = publication_definition("second_publication", 2, 10000).create;
            let mut fallback = Session::new(&store, tx, generation, &fallback_budget).await?;
            assert!(fallback.publications(tx, &[(&charged_first, requests[0].1.clone()), (&charged_second, requests[1].1.clone())], &sources, Profile::Catalog).await.is_err());
            let unacknowledged = fallback.binding(tx, charged_first.digest(), &charged_first.inputs, &requests[0].1, false, Some(context)).await?;
            assert!(!fallback.has(tx, unacknowledged, charged_first.digest()).await?, "serial fallback stages the earlier result until every sibling finishes");
            drop(fallback);
            assert_eq!(fallback_budget.reserved(), 0);
            let plan_budget = ResourceBudget::fixed(4600).unwrap();
            let mut constrained = Session::new(&store, tx, generation, &plan_budget).await?;
            assert!(constrained.publications(tx, &requests, &sources, Profile::Catalog).await.is_err());
            drop(constrained);
            assert_eq!(plan_budget.reserved(), 0);
            assert!(session.publications(tx, &[(&first_check, vec![])], &sources, Profile::Catalog).await.is_err(), "incomplete premise mapping refuses");
            Ok(())
        }).await.unwrap();
        // The fixture's intentionally revised/ad hoc proofs are controls, not an audit claim.
        attempt.seal(execution.finish().unwrap()).await.unwrap().validate().await.unwrap().publish().await.unwrap();
        store.retire(generation).await.unwrap();
    }

    #[test]
    fn publication_context_discriminates_every_source_authority_field() {
        use lctx_model::domain::analysis::sources::SourceSnapshot;
        let original = serde_json::json!({ "relation": "proof_items", "producer": "source",
            "model": ContentHash([1; 32]), "schedule": ContentHash([2; 32]),
            "content": ContentHash([3; 32]), "rows": 1, "physical": "proof_items", "prefix": null });
        let snapshot: SourceSnapshot = serde_json::from_value(original.clone()).unwrap();
        let baseline = publication_context(&[snapshot], Profile::Catalog);
        for (field, replacement) in [
            ("relation", serde_json::json!("other")), ("producer", serde_json::json!("other")),
            ("model", serde_json::json!(ContentHash([4; 32]))), ("schedule", serde_json::json!(ContentHash([4; 32]))),
            ("content", serde_json::json!(ContentHash([4; 32]))), ("rows", serde_json::json!(2)),
            ("physical", serde_json::json!("__v0_proof_items")), ("prefix", serde_json::json!("Facts")),
        ] {
            let mut changed = original.clone(); changed[field] = replacement;
            let snapshot: SourceSnapshot = serde_json::from_value(changed).unwrap();
            assert_ne!(baseline, publication_context(&[snapshot], Profile::Catalog), "{field} must rebind publication authority");
        }
    }

    #[tokio::test]
    async fn final_receipts_cover_unproduced_empty_vocabulary_without_a_prefix() {
        use lctx_model::domain::{input::Package, value::{Literal, LiteralSet, LiteralSetMember}, stages::*};
        let db = DisposableDatabase::start().await;
        let model = Arc::new(ValidatedModel::declared(vec![Relation::of::<Package>(),
            Relation::of::<Literal>(), Relation::of::<LiteralSet>(), Relation::of::<LiteralSetMember>()]).unwrap());
        let store = GenerationStore::install(db.owner.clone(), model.clone()).await.unwrap();
        let schedule = Schedule::build_with_publications(&model, vec![Stage { name: "package",
            inputs: vec![], outputs: vec![RelationUse::of::<Package>()], contributes: vec![], coverage: vec![],
            profiles: vec![Profile::Catalog], effect: Effect::Pure, code: ContentHash::of(b"empty held vocabulary"),
            configuration: ContentHash::of(b"final acknowledgement") }], &[], Profile::Catalog,
            vec![PublicationGroup::new(PublicationBoundary::Facts, vec!["package"])]).unwrap();
        let budget = ResourceBudget::fixed(64 << 20).unwrap();
        let mut execution = schedule.execute();
        let attempt = store.begin_conformance(db.writer.clone(), &mut execution, budget.clone()).await.unwrap();
        let generation = attempt.generation();
        let mut stage = execution.begin("package").unwrap();
        let batch = Batch::new(&model, vec![Package { name: "authored-seed".into() }], &budget).unwrap();
        stage.write::<Package, _>(async |permit| attempt.copy(permit, &batch).await).await.unwrap();
        stage.complete(&attempt, ProviderOutcome::Complete).await.unwrap();
        attempt.seal(execution.finish().unwrap()).await.unwrap().validate().await.unwrap().publish().await.unwrap();
        let report = store.audit(generation, lctx_model::domain::admission::Frontier::Conformance, None, &budget).await.unwrap();
        assert_eq!(report.relations, 4);
        assert!(report.semantic_checks > 0);
        store.retire(generation).await.unwrap();
    }

    #[tokio::test]
    async fn acknowledged_unprefixed_question_retains_its_exact_frame_after_legal_append() {
        use lctx_model::domain::{input::Package, value::Literal, stages::*};
        let db = DisposableDatabase::start().await;
        let definition = Invariant { name: "one_literal_question", revision: 1,
            inputs: vec![ValidationInput::of::<Literal>(&["id"])],
            create: Arc::new(|_| Box::new(Count { rows: 0, expected: 1 })) };
        let model = Arc::new(ValidatedModel::validate(vec![Relation::of::<Literal>(), Relation::of::<Package>()],
            ValidationDefinitions { invariants: vec![definition.clone()], publication_checks: vec![] }).unwrap());
        let store = GenerationStore::install(db.owner.clone(), model.clone()).await.unwrap();
        let stage = |name, inputs, outputs| Stage { name, inputs, outputs,
            contributes: vec![], coverage: vec![], profiles: vec![Profile::Catalog], effect: Effect::Pure,
            code: ContentHash::of(b"prefix question fixture"), configuration: ContentHash::of(b"authored count") };
        let schedule = Schedule::build_with_publications(&model, vec![
            stage("first", vec![], vec![RelationUse::of::<Literal>()]),
            stage("later", vec![], vec![RelationUse::of::<Literal>()]),
            stage("end", vec![RelationUse::stored::<Literal>().at_epoch(PublicationBoundary::Dispatch)], vec![RelationUse::of::<Package>()]),
        ], &[], Profile::Catalog, vec![PublicationGroup::new(PublicationBoundary::Facts, vec!["first"]),
            PublicationGroup::new(PublicationBoundary::Dispatch, vec!["later"])]).unwrap();
        let budget = ResourceBudget::fixed(64 << 20).unwrap();
        let mut execution = schedule.execute();
        let attempt = store.begin_conformance(db.writer.clone(), &mut execution, budget.clone()).await.unwrap();
        let generation = attempt.generation();
        let mut first = execution.begin("first").unwrap();
        let batch = Batch::new(&model, vec![Literal::None], &budget).unwrap();
        first.write::<Literal, _>(async |permit| attempt.copy(permit, &batch).await).await.unwrap();
        first.complete(&attempt, ProviderOutcome::Complete).await.unwrap();
        let old = super::super::transaction(&store.owner, async |tx| {
            store.lock_installation(tx).await?; super::super::lock(tx, generation, true).await?;
            let mut session = Session::new(&store, tx, generation, &budget).await?;
            session.invariant(tx, &definition, vec![Literal::NAME.into()], false).await
        }).await.unwrap();
        let mut later = execution.begin("later").unwrap();
        let batch = Batch::new(&model, vec![Literal::Bool { value: true }], &budget).unwrap();
        later.write::<Literal, _>(async |permit| attempt.copy(permit, &batch).await).await.unwrap();
        later.complete(&attempt, ProviderOutcome::Complete).await.unwrap();
        super::super::transaction(&store.owner, async |tx| {
            let mut session = Session::new(&store, tx, generation, &budget).await?;
            let current = session.binding(tx, definition.digest(), &definition.inputs, &[Literal::NAME.into()], false, None).await?;
            assert_ne!(old, current);
            assert!(!session.has(tx, current, definition.digest()).await?);
            let original = schedule.prefix_for(PublicationBoundary::Facts)?.view(Literal::NAME);
            let earlier = session.binding(tx, definition.digest(), &definition.inputs, &[original], false, None).await?;
            assert_eq!(old, earlier);
            assert!(session.has(tx, earlier, definition.digest()).await?);
            assert_eq!(session.stats.row_scans, 0);
            Ok(())
        }).await.unwrap();
        let mut end = execution.begin("end").unwrap();
        let batch = Batch::<Package>::new(&model, vec![], &budget).unwrap();
        end.write::<Package, _>(async |permit| attempt.copy(permit, &batch).await).await.unwrap();
        end.complete(&attempt, ProviderOutcome::Complete).await.unwrap();
        attempt.seal(execution.finish().unwrap()).await.unwrap().validate().await.unwrap().publish().await.unwrap();
        let audit = store.audit(generation, lctx_model::domain::admission::Frontier::Conformance, None, &budget).await.unwrap();
        assert_eq!(audit.semantic_checks, 1, "the acknowledged earlier question is independently replayed at its original frame");
        store.retire(generation).await.unwrap();
    }

}
