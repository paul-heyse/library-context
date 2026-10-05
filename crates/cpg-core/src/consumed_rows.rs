//! Invocation-local consumed declarations and typed source-bound batch streaming.
use crate::{generation_read::AttemptSession, model_runtime::StageSession};
use futures::TryStreamExt;
use lctx_model::domain::{charged, resources::ResourceBudget, stages::*, *};
use std::{any::TypeId, collections::BTreeMap};

/// Resolve each owned declaration once and coalesce aliases only when the acknowledged source
/// is identical. The same relation at distinct immutable epochs remains two consumed sources.
pub struct ConsumedInputs {
    declarations: Vec<ValidationInput>,
    dispatched: charged::ChargedSet<usize>,
    sources: BTreeMap<(&'static str, Option<u16>), CompletedRelation>,
    charge: charged::StateCharge,
}
impl ConsumedInputs {
    pub fn new(
        mut declarations: Vec<ValidationInput>,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        declarations.sort_by_key(|i| (i.name(), i.prefix(), i.order().to_vec()));
        declarations.dedup_by(|a, b| {
            a.name() == b.name() && a.prefix() == b.prefix() && a.order() == b.order()
        });
        let mut charge = charged::StateCharge::new(budget, "consumed-inputs");
        charge.grow(
            declarations
                .capacity()
                .saturating_mul(size_of::<ValidationInput>())
                + declarations
                    .iter()
                    .map(|i| i.order().len().saturating_mul(size_of::<&str>()))
                    .sum::<usize>(),
        )?;
        Ok(Self {
            declarations,
            dispatched: Default::default(),
            sources: BTreeMap::new(),
            charge,
        })
    }
    /// Typed macros provide decoder reachability; they never select the semantic input inventory.
    pub fn next<'a, R: Record>(
        &mut self,
        access: &'a StageAccess<'_, '_>,
    ) -> Result<Option<(ValidationInput, ReadPermit<'a, R>)>, ModelError> {
        loop {
            let Some((index, input)) = self.declarations.iter().enumerate().find(|(index, i)| {
                i.type_id() == TypeId::of::<R>() && !self.dispatched.contains(index)
            }) else {
                return Ok(None);
            };
            let mut input = input.clone();
            self.dispatched.insert(&mut self.charge, index)?;
            let permit = match input.prefix() {
                Some(epoch) => access.read_at_epoch::<R>(epoch)?,
                None => access.read::<R>()?,
            };
            let source = permit.source().ok_or_else(|| {
                ModelError::Invalid("consumed rows require an acknowledged source".into())
            })?;
            let key = (R::NAME, source.prefix_ordinal().map(PrefixOrdinal::ordinal));
            if let Some(previous) = self.sources.get(&key) {
                if previous != source {
                    return Err(ModelError::Invalid(
                        "consumed input changes acknowledged source within an epoch".into(),
                    ));
                }
                continue;
            }
            self.charge
                .grow(size_of::<CompletedRelation>() + size_of::<(&str, Option<u16>)>() + 64)?;
            self.sources.insert(key, source.clone());
            if is_vocabulary(input.name()) {
                input = input.at_epoch(source.prefix().ok_or_else(|| {
                    ModelError::Invalid("consumed vocabulary has no immutable epoch".into())
                })?);
            }
            return Ok(Some((input, permit)));
        }
    }
    /// Refuse a typed dispatch that omitted an owned consumed declaration.
    pub fn finish(self, stage: &str) -> Result<(), ModelError> {
        if self.dispatched.len() != self.declarations.len() {
            return Err(ModelError::Invalid(format!(
                "stage {stage} consumed input has no typed loader: {}",
                self.declarations
                    .iter()
                    .enumerate()
                    .find(|(i, _)| !self.dispatched.contains(i))
                    .expect("undispatched input")
                    .1
                    .name()
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
pub(crate) fn assert_decoder_reachability(
    declarations: Vec<ValidationInput>,
    decoders: &std::collections::BTreeSet<TypeId>,
) {
    let missing = declarations
        .iter()
        .filter(|input| !decoders.contains(&input.type_id()))
        .map(|input| (input.name(), input.prefix()))
        .collect::<Vec<_>>();
    assert!(
        missing.is_empty(),
        "model-owned consumed sources lack decoder reachability: {missing:?}"
    );
}

/// Register an existing typed source in its stage session and stream to an explicit consumer.
/// On errors or cancellation the stream drops through the provider's existing drain contract.
/// Callers isolate sessions for distinct epochs because StageSession registers by relation name.
pub async fn stream<R: Record>(
    permit: &ReadPermit<'_, R>,
    reader: &AttemptSession,
    session: &StageSession,
    mut consume: impl FnMut(&ReadPermit<'_, R>, &arrow_array::RecordBatch) -> Result<(), ModelError>,
) -> Result<(), ModelError> {
    session.register(permit, reader.table(permit).map_err(ModelError::codec)?)?;
    let query = session
        .query(&format!("SELECT * FROM \"{}\"", R::NAME))
        .await
        .map_err(ModelError::codec)?;
    let mut batches = query.execute_stream().await.map_err(ModelError::codec)?;
    while let Some(batch) = batches.try_next().await.map_err(ModelError::codec)? {
        consume(permit, &batch)?;
        // Ready batches must still offer cancellation and fair scheduling between consumers.
        tokio::task::yield_now().await;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lctx_model::domain::{input::Package, memory::MemoryGeneration, value::Literal};
    #[test]
    fn missing_decoder_names_the_stage_without_changing_error_class() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let inputs =
            ConsumedInputs::new(vec![ValidationInput::of::<Literal>(&["id"])], &budget).unwrap();
        let ModelError::Invalid(message) = inputs.finish("synthesize").unwrap_err() else {
            panic!("missing decoder changed error class")
        };
        assert_eq!(
            message,
            "stage synthesize consumed input has no typed loader: literal_values"
        );
    }
    #[tokio::test]
    async fn two_acknowledged_epochs_remain_distinct_while_same_source_aliases_coalesce() {
        let model =
            ValidatedModel::declared(vec![Relation::of::<Literal>(), Relation::of::<Package>()])
                .unwrap();
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let stage = |name, inputs, outputs| Stage {
            name,
            inputs,
            outputs,
            contributes: vec![],
            coverage: vec![],
            profiles: vec![Profile::Catalog],
            effect: Effect::Pure,
            code: ContentHash::of(b"consumption control"),
            configuration: ContentHash::of(b"fixture"),
        };
        let schedule = Schedule::build_with_publications(
            &model,
            vec![
                stage("facts", vec![], vec![RelationUse::of::<Literal>()]),
                stage("local", vec![], vec![RelationUse::of::<Literal>()]),
                stage(
                    "reader",
                    vec![RelationUse::stored::<Literal>().at_epoch(PublicationBoundary::Local)],
                    vec![RelationUse::of::<Package>()],
                ),
            ],
            &[],
            Profile::Catalog,
            vec![
                PublicationGroup::new(PublicationBoundary::Facts, vec!["facts"]),
                PublicationGroup::new(PublicationBoundary::Local, vec!["local"]),
            ],
        )
        .unwrap();
        let mut execution = schedule.execute();
        let sink = MemoryGeneration::bind(&model, &budget, &mut execution).unwrap();
        for (name, row) in [
            ("facts", Literal::None),
            ("local", Literal::Bool { value: true }),
        ] {
            let batch = Batch::new(&model, vec![row], &budget).unwrap();
            let mut access = execution.begin(name).unwrap();
            access
                .write::<Literal, _>(async |permit| sink.copy(permit, &batch).await)
                .await
                .unwrap();
            access
                .complete(&sink, ProviderOutcome::Complete)
                .await
                .unwrap();
        }
        let access = execution.begin("reader").unwrap();
        let mut inputs = ConsumedInputs::new(
            vec![
                ValidationInput::of::<Literal>(&["id"]).at_epoch(PublicationBoundary::Facts),
                ValidationInput::of::<Literal>(&["id"]).at_epoch(PublicationBoundary::Local),
                ValidationInput::of::<Literal>(&["kind", "id"])
                    .at_epoch(PublicationBoundary::Local),
            ],
            &budget,
        )
        .unwrap();
        let mut observed = std::collections::BTreeMap::new();
        while let Some((input, permit)) = inputs.next::<Literal>(&access).unwrap() {
            let source = permit.source().unwrap();
            observed.insert(input.prefix().unwrap(), source.receipt().rows);
        }
        inputs.finish("reader").unwrap();
        assert_eq!(observed.len(), 2);
        assert_eq!(observed[&PublicationBoundary::Facts], 1);
        assert_eq!(observed[&PublicationBoundary::Local], 2);
        assert!(
            access
                .read_at_epoch::<Literal>(PublicationBoundary::Structural)
                .is_err()
        );
    }
}
