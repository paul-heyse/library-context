//! Explicit read-only recomputation. Normal consumers trust acknowledged owned frames.
use super::{Error, GenerationId, GenerationStore, lock, visit_named};
use super::validation_session::{ProofContext, Session};
use lctx_model::domain::{ContentHash, ModelError, ValidationInput, admission::Frontier,
    resources::ResourceBudget, stages::{PublicationBoundary, Profile, is_vocabulary}};
use sqlx::{Connection, Row};
use futures::TryStreamExt;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy)]
pub struct AuditReport { pub relations: usize, pub semantic_checks: usize }

impl GenerationStore {
    /// Recompute bodies, exact proof bindings and required semantics without issuing grants,
    /// consuming cached success, repairing data or acknowledging new conclusions.
    pub async fn audit(&self, generation: GenerationId, frontier: Frontier,
        prefix: Option<PublicationBoundary>, budget: &ResourceBudget) -> Result<AuditReport, Error> {
        let mut connection = self.owner.acquire().await?;
        let mut tx = connection.begin_with("BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY").await?;
        self.lock_installation(&mut tx).await?;
        lock(&mut tx, generation, true).await?;
        let registered = self.registered(&mut tx, generation).await?;
        if !matches!(registered.state.as_str(), "validated" | "published") { return Err(Error::State); }
        let requested = frontier.descriptor().relations(&self.model)?;
        if !requested.is_subset(&self.scope(registered.frontier)?.relations) { return Err(Error::Contract); }
        // The enclosing metadata inventory is challenged even for a smaller body scope.
        // This detects registry-only corruption without hydrating unrelated relations.
        let _inventory = budget.reserve("audit-receipt-inventory", 4096)?;
        let scope = self.scope(registered.frontier)?;
        let mut expected_names = scope.relations.iter();
        let mut aggregate = lctx_model::domain::KeySink::new("generation-content");
        {
            let mut receipts = sqlx::query("SELECT relation_name,content_digest FROM lctx_model_store.receipts WHERE generation_id=$1 ORDER BY relation_name COLLATE \"C\"")
                .bind(generation.0.to_vec()).fetch(&mut *tx);
            while let Some(receipt) = receipts.try_next().await? {
                let name: String = receipt.try_get(0)?;
                let digest: Vec<u8> = receipt.try_get(1)?;
                if expected_names.next().copied() != Some(name.as_str()) || digest.len() != 32 { return Err(Error::Contract); }
                aggregate.part(name.as_bytes(), &digest);
            }
        }
        let stored: Vec<u8> = sqlx::query_scalar("SELECT content_digest FROM lctx_model_store.generations WHERE id=$1")
            .bind(generation.0.to_vec()).fetch_one(&mut *tx).await?;
        if expected_names.next().is_some() || stored != aggregate.finish().0 {
            return Err(invalid("audit aggregate content mismatch".into()));
        }
        let order = super::vocabulary::publication_order(&mut tx, generation).await?;
        let selected = prefix.map(|p| order.resolve(p)).transpose()?;
        if let Some(bound) = selected {
            let closed: bool = sqlx::query_scalar("SELECT closed FROM lctx_model_store.publication_groups WHERE generation_id=$1 AND epoch=$2")
                .bind(generation.0.to_vec()).bind(i16::try_from(bound.ordinal()).map_err(|_| Error::Contract)?)
                .fetch_one(&mut *tx).await?;
            if !closed { return Err(Error::Contract); }
        }
        let _metadata = budget.reserve("generation-audit", requested.len().saturating_mul(512))?;
        let mut report = AuditReport { relations: 0, semantic_checks: 0 };
        let mut session = Session::new(self, &mut tx, generation, budget).await?;
        for name in &requested {
            let relation = self.model.relation(name).ok_or(Error::Contract)?;
            let physical = selected.map_or_else(|| (*name).to_owned(), |p| super::vocabulary::physical(name, p));
            let expected: Option<(i64, Vec<u8>)> = if is_vocabulary(name) && selected.is_some() {
                sqlx::query_as("SELECT row_count,content_digest FROM lctx_model_store.epoch_receipts WHERE generation_id=$1 AND epoch=$2 AND relation_name=$3")
                    .bind(generation.0.to_vec()).bind(i16::try_from(selected.ok_or(Error::Contract)?.ordinal()).map_err(|_| Error::Contract)?)
                    .bind(name).fetch_optional(&mut *tx).await?
            } else {
                sqlx::query_as("SELECT row_count,content_digest FROM lctx_model_store.receipts WHERE generation_id=$1 AND relation_name=$2")
                    .bind(generation.0.to_vec()).bind(name).fetch_optional(&mut *tx).await?
            };
            let expected = expected.ok_or(Error::Contract)?;
            let actual = super::vocabulary::receipt(&mut tx, generation, relation, &physical, budget).await?;
            if u64::try_from(expected.0).ok() != Some(actual.rows) || expected.1 != actual.content.0 {
                return Err(invalid(format!("audit content mismatch: {name}")));
            }
            // Independent nominal challenge, including imported rows and selected prefix views.
            for field in relation.fields() {
                let Some((_, target)) = field.target() else { continue; };
                let target_relation = self.model.relation(target).ok_or(Error::Contract)?;
                let target_frame = selected.map_or_else(|| target.to_owned(), |p| super::vocabulary::physical(target, p));
                let subtype = field.subtype().map_or(String::new(), |code|
                    format!(" AND b.{}={code}", super::quoted(target_relation.sum().expect("validated subtype").tag)));
                let bad: bool = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                    "SELECT EXISTS(SELECT 1 FROM {} a WHERE a.{} IS NOT NULL AND NOT EXISTS(SELECT 1 FROM {} b WHERE b.id=a.{}{subtype}))",
                    super::qualified(generation, &physical), super::quoted(field.name()),
                    super::qualified(generation, &target_frame), super::quoted(field.name()))))
                    .fetch_one(&mut *tx).await?;
                if bad { return Err(invalid(format!("audit nominal reference mismatch: {name}.{}", field.name()))); }
            }
            report.relations += 1;
        }
        let profile: String = sqlx::query_scalar("SELECT profile FROM lctx_model_store.generations WHERE id=$1")
            .bind(generation.0.to_vec()).fetch_one(&mut *tx).await?;
        let actual_profile = Profile::ALL.into_iter().find(|p| p.name() == profile).ok_or(Error::Contract)?;
        let mut publications = BTreeSet::new();
        let mut previous = Vec::<u8>::new();
        // One bounded receipt at a time. Inspect size before decoding JSON so a damaged receipt
        // cannot allocate an unadmitted replay context. The read snapshot is stable throughout.
        loop {
            let claim = sqlx::query("SELECT binding_digest,octet_length(proof_context::text) FROM lctx_model_store.validation_receipts WHERE generation_id=$1 AND binding_digest>$2 ORDER BY binding_digest LIMIT 1")
                .bind(generation.0.to_vec()).bind(&previous).fetch_optional(&mut *tx).await?;
            let Some(claim) = claim else { break; };
            let key: Vec<u8> = claim.try_get(0)?;
            let bytes: i32 = claim.try_get(1)?;
            let _context = budget.reserve("audit-proof-context", usize::try_from(bytes).map_err(|_| Error::Contract)?.checked_mul(8).and_then(|n| n.checked_add(4096)).ok_or(Error::Contract)?)?;
            let row = sqlx::query("SELECT validator_name,definition_digest,model_digest,physical_digest,proof_context FROM lctx_model_store.validation_receipts WHERE generation_id=$1 AND binding_digest=$2")
                .bind(generation.0.to_vec()).bind(&key).fetch_one(&mut *tx).await?;
            let name: String = row.try_get(0)?;
            let definition: Vec<u8> = row.try_get(1)?;
            if row.try_get::<Vec<u8>, _>(2)? != self.model.digest().0
                || row.try_get::<Vec<u8>, _>(3)? != self.scope(registered.frontier)?.physical.0 { return Err(Error::Contract); }
            let context: ProofContext = serde_json::from_value(row.try_get::<serde_json::Value, _>(4)?)
                .map_err(|e| Error::Codec(e.to_string()))?;
            let binding = ContentHash(key.clone().try_into().map_err(|_| Error::Contract)?);
            match context {
                ProofContext::Invariant { frames } => {
                    let names: BTreeSet<_> = frames.iter().map(|(name, _)| name.as_str()).collect();
                    let generated;
                    let check = if name == "derivation_acyclic" {
                        generated = self.model.generated_invariant_for_scope(&names)?.ok_or(Error::Contract)?;
                        &generated
                    } else { self.model.invariant(&name)? };
                    if definition != check.digest().0 { return Err(Error::Contract); }
                    let physical = exact_frames(&check.inputs, &frames, &order)?;
                    let expected = session.binding(&mut tx, check.digest(), &check.inputs, &physical, false, None).await?;
                    if expected != binding { return Err(invalid(format!("audit binding mismatch: {name}"))); }
                    if names.is_subset(&requested) {
                        let mut state = (check.create)(budget);
                        for (input, frame) in check.inputs.iter().zip(&physical) {
                            audit_frame(self, &mut session, &mut tx, generation, input, frame, budget).await?;
                            visit_named(&mut tx, generation, self.model.relation(input.name()).ok_or(Error::Contract)?, frame, input.order(), budget,
                                |batch| { state.visit_input(input, &batch)?; Ok(()) }).await?;
                        }
                        state.finish()?; report.semantic_checks += 1;
                    }
                }
                ProofContext::Publication { frames, sources, profile: claimed_profile } => {
                    let check = self.model.publication_check(&name)?;
                    if definition != check.digest().0 || claimed_profile != profile { return Err(Error::Contract); }
                    let physical = exact_frames(&check.inputs, &frames, &order)?;
                    let context = super::validation_session::publication_context(&sources, actual_profile);
                    if session.binding(&mut tx, check.digest(), &check.inputs, &physical, false, Some(context)).await? != binding {
                        return Err(invalid(format!("audit publication binding mismatch: {name}")));
                    }
                    for source in &sources {
                        if source.model() != self.model.digest() { return Err(Error::Contract); }
                        let stored: Option<(i64, Vec<u8>, Vec<u8>)> = sqlx::query_as("SELECT row_count,content_digest,schedule_digest FROM lctx_model_store.stage_receipts WHERE generation_id=$1 AND stage_name=$2 AND relation_name=$3")
                            .bind(generation.0.to_vec()).bind(source.producer()).bind(source.relation()).fetch_optional(&mut *tx).await?;
                        if stored.is_none_or(|(rows, content, schedule)| rows != source.rows() || content != source.content().0 || schedule != source.schedule().0) { return Err(Error::Contract); }
                        let relation = self.model.relation(source.relation()).ok_or(Error::Contract)?;
                        let prefix = source.prefix().map(|name| {
                            let boundary = PublicationBoundary::from_name(name).ok_or(Error::Contract)?;
                            Ok::<_, Error>(order.resolve(boundary)?)
                        }).transpose()?;
                        if let Some(prefix) = prefix {
                            let closed: bool = sqlx::query_scalar("SELECT closed FROM lctx_model_store.publication_groups WHERE generation_id=$1 AND epoch=$2")
                                .bind(generation.0.to_vec()).bind(i16::try_from(prefix.ordinal()).map_err(|_| Error::Contract)?)
                                .fetch_one(&mut *tx).await?;
                            if !closed { return Err(Error::Contract); }
                        }
                        let physical = prefix.map_or_else(|| source.relation().to_owned(), |p| super::vocabulary::physical(source.relation(), p));
                        if source.physical() != physical { return Err(Error::Contract); }
                        let input = ValidationInput::of_relation(relation, &["id"]);
                        let acknowledged = session.frame(&mut tx, &input, &physical, false).await?;
                        if i64::try_from(acknowledged.rows).ok() != Some(source.rows()) || acknowledged.content != source.content() { return Err(Error::Contract); }
                        audit_frame(self, &mut session, &mut tx, generation, &input, &physical, budget).await?;
                    }
                    if check.inputs.iter().all(|i| requested.contains(i.name())) {
                        let mut state = (check.create)(budget);
                        for (input, frame) in check.inputs.iter().zip(&physical) {
                            audit_frame(self, &mut session, &mut tx, generation, input, frame, budget).await?;
                            visit_named(&mut tx, generation, self.model.relation(input.name()).ok_or(Error::Contract)?, frame, input.order(), budget,
                                |batch| { state.visit_input(input, &batch)?; Ok(()) }).await?;
                        }
                        state.finish(&sources, actual_profile)?;
                        publications.insert(check.name); report.semantic_checks += 1;
                    }
                }
                ProofContext::Nominal { inputs } => {
                    let expected = ContentHash::of(b"model-owned-nominal-reference-closure/v1");
                    if name != "nominal_references" || definition != expected.0 || session.metadata_binding(expected, inputs) != binding { return Err(Error::Contract); }
                }
            }
            previous = key;
        }
        // A deleted or substituted complete conclusion is a discrepancy even when recomputed
        // rows happen to remain valid. Strict consumers require this exact inventory too.
        for invariant in frontier.descriptor().invariants(&self.model)? {
            let mut frames = Vec::new();
            for input in &invariant.inputs {
                let fallback = selected.map_or_else(|| input.name().to_owned(), |p| super::vocabulary::physical(input.name(), p));
                frames.push(super::validation_views::physical(&mut tx, generation, input,
                    self.model.relation(input.name()).ok_or(Error::Contract)?, &fallback,
                    super::validation_views::Scope { upper: selected, candidate: None }, budget).await?);
            }
            let binding = session.binding(&mut tx, invariant.digest(), &invariant.inputs, &frames, false, None).await?;
            if !session.has(&mut tx, binding, invariant.digest()).await? { return Err(invalid(format!("audit required conclusion missing: {}", invariant.name))); }
        }
        for name in &requested {
            for id in self.model.relation(name).ok_or(Error::Contract)?.publication_refs() {
                if !publications.contains(id) { return Err(invalid(format!("audit publication conclusion missing: {id}"))); }
            }
        }
        tx.commit().await.map_err(Error::Commit)?;
        Ok(report)
    }
}

fn exact_frames(inputs: &[ValidationInput], frames: &[(String, String)], order: &lctx_model::domain::stages::PublicationOrder) -> Result<Vec<String>, Error> {
    if inputs.len() != frames.len() { return Err(Error::Contract); }
    for (input, (name, frame)) in inputs.iter().zip(frames) {
        if input.name() != name { return Err(Error::Contract); }
        if !is_vocabulary(name) {
            if frame != name { return Err(Error::Contract); }
        } else if let Some(boundary) = input.prefix() {
            if *frame != order.resolve(boundary)?.view(name) { return Err(Error::Contract); }
        } else if frame == name {
            // A finite ordinary model can freeze vocabulary without publication groups.
            // Grouped vocabulary proofs always retain their exact closed ordinal.
            if order.decode(0).is_ok() { return Err(Error::Contract); }
        } else {
            let ordinal = frame.strip_prefix("__v").and_then(|v| v.split_once('_'))
                .filter(|(_, relation)| *relation == name).and_then(|(n, _)| n.parse::<u16>().ok())
                .ok_or(Error::Contract)?;
            if *frame != order.decode(ordinal)?.view(name) { return Err(Error::Contract); }
        }
    }
    Ok(frames.iter().map(|(_, frame)| frame.clone()).collect())
}
async fn audit_frame(store: &GenerationStore, session: &mut Session<'_>, tx: &mut sqlx::PgConnection,
    generation: GenerationId, input: &ValidationInput, physical: &str, budget: &ResourceBudget) -> Result<(), Error> {
    let expected = session.frame(tx, input, physical, false).await?;
    let actual = super::vocabulary::receipt(tx, generation, store.model.relation(input.name()).ok_or(Error::Contract)?, physical, budget).await?;
    if expected != actual { return Err(invalid(format!("audit proof frame mismatch: {}", input.name()))); }
    Ok(())
}
fn invalid(message: String) -> Error { Error::Model(ModelError::Invalid(message)) }

#[cfg(test)]
mod tests {
    use super::*;
    use lctx_model::domain::{source::SourceArtifact, value::Literal, stages::PublicationOrder};

    #[test]
    fn replay_context_cannot_substitute_an_ordinary_or_declared_prefix_frame() {
        let order = PublicationOrder::registered(ContentHash::of(b"audit-frame-control"), &[
            (0, PublicationBoundary::Facts), (1, PublicationBoundary::Dispatch),
        ]).unwrap();
        let ordinary = ValidationInput::of::<SourceArtifact>(&["id"]);
        assert!(exact_frames(std::slice::from_ref(&ordinary), &[(ordinary.name().into(), "other_table".into())], &order).is_err());
        assert!(exact_frames(std::slice::from_ref(&ordinary), &[(ordinary.name().into(), ordinary.name().into())], &order).is_ok());
        let vocabulary = ValidationInput::of::<Literal>(&["id"]).at_epoch(PublicationBoundary::Facts);
        assert!(exact_frames(std::slice::from_ref(&vocabulary), &[(vocabulary.name().into(), order.decode(1).unwrap().view(vocabulary.name()))], &order).is_err());
        assert!(exact_frames(std::slice::from_ref(&vocabulary), &[(vocabulary.name().into(), order.decode(0).unwrap().view(vocabulary.name()))], &order).is_ok());
        let unprefixed = ValidationInput::of::<Literal>(&["id"]);
        assert!(exact_frames(std::slice::from_ref(&unprefixed), &[(unprefixed.name().into(), order.decode(0).unwrap().view(unprefixed.name()))], &order).is_ok());
        assert!(exact_frames(std::slice::from_ref(&unprefixed), &[(unprefixed.name().into(), unprefixed.name().into())], &order).is_err());
    }
}
