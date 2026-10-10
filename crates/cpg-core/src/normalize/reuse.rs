//! Reconstruct normalization verification owners from retained typed rows and current inputs.
//! Shared owner predicates mint the fresh application; cached bytes never carry authority.
use super::*;
use crate::{consumed_rows::ClosureTable, workspace::ProductCandidate};
use datafusion::datasource::MemTable;
#[cfg(test)]
use lctx_surrealdb::surrealdb::types::Value;

struct CachedInputs {
    invariant: Invariant,
    tables: Vec<ClosureTable>,
    session: datafusion::prelude::SessionContext,
    // Every decoded Arrow batch remains charged through predicate execution. The session owns
    // only bounded transfer batches, never a deserialized semantic capability.
    _candidate: Arc<ProductCandidate>,
}
async fn inputs(
    access: &CompletedInputs,
    runtime: &Workspace,
    mut invariant: Invariant,
    outputs: Vec<Relation>,
    candidate: &Arc<ProductCandidate>,
) -> Result<CachedInputs, ModelError> {
    invariant.inputs.retain(|input| access.table_for(input).is_ok()
        || (input.prefix().is_none() && outputs.iter().any(|output| output.name() == input.name())));
    let session = access.predicate_session(runtime).await?;
    let mut tables = Vec::new();
    for input in &invariant.inputs {
        let relation = runtime.model().relation(input.name())
            .ok_or(ModelError::Schema("cached normalization relation"))?.clone();
        let alias = if let Ok(alias) = access.table_for(input) { alias } else {
            let batches = candidate.batches(input.name())
                .ok_or(ModelError::Conflict("cached normalization complete output domain"))?
                .to_vec();
            let alias = format!("cached_normalization_{}", relation.name());
            let provider = MemTable::try_new(relation.schema().clone(), vec![batches]).map_err(ModelError::codec)?;
            session.register_table(&alias, Arc::new(provider)).map_err(ModelError::codec)?;
            alias
        };
        tables.push(ClosureTable { relation, alias });
    }
    Ok(CachedInputs { invariant, tables, session, _candidate: candidate.clone() })
}

pub(super) async fn receivers(
    access: &CompletedInputs, runtime: &Workspace, candidate: &Arc<ProductCandidate>,
) -> Result<normalized::receiver::VerifiedReceivers, ModelError> {
    let invariant = normalized::receiver::invariants().into_iter()
        .find(|invariant| invariant.purpose == InvariantPurpose::Admission)
        .ok_or(ModelError::Schema("receiver product admission"))?;
    let cached = inputs(access, runtime, invariant, normalized::receiver::relations(), candidate).await?;
    admission::prepare_receivers(&cached.invariant, cached.tables, &cached.session, runtime.budget(),
        &runtime.cancellation(), runtime.model(), true).await?
        .ok_or(ModelError::Conflict("receiver product application absent"))
}
pub(super) async fn events(
    access: &CompletedInputs, runtime: &Workspace, candidate: &Arc<ProductCandidate>,
) -> Result<normalized::event_normalization::VerifiedEvents, ModelError> {
    let invariant = normalized::event_normalization::invariants().into_iter()
        .find(|invariant| invariant.purpose == InvariantPurpose::Admission)
        .ok_or(ModelError::Schema("event product admission"))?;
    let cached = inputs(access, runtime, invariant, normalized::events::relations(), candidate).await?;
    admission::prepare_events(&cached.invariant, cached.tables, &cached.session, runtime.budget(),
        &runtime.cancellation(), runtime.model(), true).await?
        .ok_or(ModelError::Conflict("event product application absent"))
}
pub(super) async fn bindings(
    access: &CompletedInputs, runtime: &Workspace, candidate: &Arc<ProductCandidate>,
) -> Result<normalized::binding_normalization::VerifiedBindings, ModelError> {
    let invariant = normalized::binding_normalization::invariants().into_iter()
        .find(|invariant| invariant.purpose == InvariantPurpose::Admission)
        .ok_or(ModelError::Schema("binding product admission"))?;
    let cached = inputs(access, runtime, invariant, normalized::bindings::relations(), candidate).await?;
    let mut owner = admission::prepare_bindings(&cached.invariant, cached.tables, &cached.session, runtime.budget(),
        &runtime.cancellation(), runtime.model(), true).await?
        .ok_or(ModelError::Conflict("binding product application absent"))?;
    super::prepare_enumeration_authority(access, &cached.session, &mut owner, runtime).await?;
    Ok(owner)
}

#[cfg(test)]
mod controls {
    use super::*;
    use lctx_model::domain::{admission::Frontier, compilation_product::*, normalized::receiver, };
    fn section<R: Record>(rows: &[R]) -> ProductSection {
        let batch = R::encode(rows).unwrap();
        let mut bodies = lctx_surrealdb::codec::batch_bodies(&Relation::of::<R>(), &batch).unwrap();
        for (row, body) in rows.iter().zip(&mut bodies) {
            let Value::Object(fields) = body else { panic!("canonical test body") };
            fields.insert("id", hex::encode(row.id().bytes()));
        }
        ProductSection { name: R::NAME.into(), rows: rows.len() as u64, bytes: serde_json::to_vec(&bodies).unwrap() }
    }
    #[tokio::test]
    async fn normalization_cached_receiver_predicate_rejection_precedes_ingress() {
        let config = lctx_surrealdb::RuntimeConfig::read(std::path::Path::new(
            &std::env::var("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned native fixture"))).unwrap();
        let native = lctx_surrealdb::compiler::NativeCompilerStore::begin(&config, Frontier::Facts).await.unwrap();
        let model = Arc::new(lctx_model::domain::model().unwrap());
        let workspace = Workspace::new(model.clone(), crate::workspace::WorkspaceOptions {
            memory_bytes: 64 << 20, ..Default::default()
        }, native).unwrap();
        let stage = receiver::stage(Profile::Catalog);
        let facts = workspace.output("normalization-cache-premises", Profile::Catalog, ContentHash::of(b"fixture/v1"),
            workspace.inputs("normalization-cache-premises", Profile::Catalog, []).unwrap(),
            stage.inputs.iter().map(|input| input.name()));
        let mut available = std::collections::BTreeMap::<&'static str, Declaration>::new();
        macro_rules! declarations {($($field:ident:$ty:ty $(=> $family:ident)?,)*) => {$(available.insert(<$ty>::NAME, declare::<$ty> as Declaration);)*};}
        lctx_model::normalized_entity_inputs!(declarations);
        lctx_model::normalized_entity_outputs!(declarations);
        lctx_model::normalized_relation_inputs!(declarations);
        lctx_model::normalized_relation_outputs!(declarations);
        lctx_model::normalized_callable_inputs!(declarations);
        lctx_model::normalized_callable_outputs!(declarations);
        lctx_model::normalized_receiver_inputs!(declarations);
        let declarations = stage.inputs.iter().map(|input| *available.get(input.name())
            .unwrap_or_else(|| panic!("typed fixture declaration missing: {}", input.name())))
            .collect::<Vec<_>>();
        declare_ordered(&facts, &declarations).await.unwrap();
        facts.finish(ProviderOutcome::Complete).await.unwrap();
        workspace.freeze_inputs_async(PublicationBoundary::Facts).await.unwrap();
        let access = workspace.stage_inputs(&stage, Profile::Catalog).unwrap();
        let output = workspace.producer(&stage, Profile::Catalog, access.clone());
        let request = output.product_request().unwrap();
        let bad = receiver::ReceiverAssessment::Unknown {
            target: super::super::callable_scope::nominal(&[242; 16]).unwrap(),
            policy: ContentHash::of(b"fixture-policy"), reason: receiver::ReceiverReason::MissingSyntax,
            members: ContentHash::of(b"fixture-members"),
        };
        let mut sections = vec![section(&[bad]), section::<receiver::ReceiverEvidence>(&[]), section::<receiver::ReceiverPremise>(&[])];
        sections.sort_by(|left, right| left.name.cmp(&right.name));
        let bad = PortableProduct { request: request.clone(), outcome: ProductOutcome::Complete, sections };
        // The candidate is perfectly typed/canonical; only current receiver semantics reject it.
        crate::workspace::validate_product_rows(&bad, &model, workspace.budget(), resources::TRANSFER_ROWS).unwrap();
        let candidate=Arc::new(crate::workspace::decode_product_rows(&bad,&model,workspace.budget(),resources::TRANSFER_ROWS).unwrap());
        assert!(super::receivers(&access,&workspace,&candidate).await.is_err(),"typed rows alone do not establish receiver authority");
        drop(candidate);
        super::super::receivers_produced(access, output, &workspace, &model).await.unwrap();
        assert_eq!(workspace.relation(receiver::ReceiverAssessment::NAME).unwrap().rows(), 0,
            "a rejected candidate must leave no ingress rows before fresh empty production");
        workspace.native().abandon().await.unwrap();
    }
}
