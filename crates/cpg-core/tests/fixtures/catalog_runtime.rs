//! Actual earlier Local and SourceCall producers for contextual catalog controls.
use lctx_model::domain::{stages::*, *};
pub fn relations() -> Vec<Relation> {
    let mut r = Vec::new();
    r.extend(analysis::local::relations());
    r.extend(transfer::local::relations());
    r.extend(local_semantics::relations());
    r.extend(local_theory::relations());
    r.extend(local_fields::relations());
    macro_rules! outputs {($($f:ident:$ty:ty,)*)=>{$(r.push(Relation::of::<$ty>());)*};}
    lctx_model::local_semantic_outputs!(outputs);
    r.extend(analysis::base_evaluation::relations());
    r.extend(execution::records::relations());
    r.extend(execution::production::relations());
    r.extend(analysis::base_completion::relations());
    r.extend(execution::completion_records::relations());
    r.extend(execution::completion_production::relations());
    r.extend(execution::body_records::relations());
    r.extend(analysis::source_call::relations());
    r.extend(execution::source_call_records::relations());
    r
}
pub fn definitions() -> Vec<(analysis::MethodParameters, analysis::AnalysisDefinition)> {
    vec![
        local_semantics::definition(),
        execution::configuration::base_evaluation(),
        execution::configuration::base_completion(),
        execution::configuration::source_calls(),
    ]
}
pub fn stages(profile: Profile, model: &ValidatedModel) -> Vec<Stage> {
    vec![
        local_semantics::stage(profile, &local_semantics::definition().1, model),
        execution::production::stage(
            profile,
            &execution::configuration::base_evaluation().1,
            model,
        )
        .unwrap(),
        execution::completion_production::stage(
            profile,
            &execution::configuration::base_completion().1,
            model,
        )
        .unwrap(),
        execution::source_call::stage(profile, &execution::configuration::source_calls().1, model)
            .unwrap(),
    ]
}
pub fn schedule(model: &ValidatedModel, stages: Vec<Stage>, profile: Profile) -> Schedule {
    let facts = stages
        .iter()
        .filter(|s| s.name != "analyze_local" && s.outputs.iter().any(|r| is_vocabulary(r.name())))
        .map(|s| s.name)
        .collect();
    Schedule::build_with_publications(
        model,
        stages,
        &[],
        profile,
        vec![
            PublicationGroup::new(PublicationBoundary::Facts, facts),
            PublicationGroup::new(PublicationBoundary::Local, vec!["analyze_local"]),
        ],
    )
    .unwrap()
}
pub async fn run(
    name: &str,
    access: StageAccess<'_, '_>,
    attempt: &lctx_postgres::generations::GenerationAttempt,
    config: &lctx_postgres::roles::RoleConfig,
    runtime: &cpg_core::model_runtime::AttemptRuntime,
    model: &std::sync::Arc<ValidatedModel>,
) -> Result<(), ModelError> {
    match name {
        "analyze_local" => {
            cpg_core::local_semantics::run(
                access,
                attempt,
                config,
                runtime,
                model,
                &local_semantics::definition().1,
            )
            .await
        }
        "evaluate_base" => {
            cpg_core::semantic_execution::evaluate_base(
                access,
                attempt,
                config,
                runtime,
                model,
                &execution::configuration::base_evaluation().1,
            )
            .await
        }
        "complete_base" => {
            cpg_core::semantic_execution::complete_base(
                access,
                attempt,
                config,
                runtime,
                model,
                &execution::configuration::base_completion().1,
            )
            .await
        }
        "prepare_source_calls" => {
            cpg_core::semantic_execution::prepare_source_calls(
                access,
                attempt,
                config,
                runtime,
                model,
                &execution::configuration::source_calls().1,
            )
            .await
        }
        _ => Err(ModelError::Invalid("not a catalog runtime parent".into())),
    }
}
/// Fault-inject only the captured in-memory view. The store remains immutable and is sealed
/// normally afterward; both assertions use the same production replay kernel as publication.
pub async fn replay_controls(
    access: &StageAccess<'_, '_>,
    attempt: &lctx_postgres::generations::GenerationAttempt,
    config: &lctx_postgres::roles::RoleConfig,
    runtime: &cpg_core::model_runtime::AttemptRuntime,
    model: &std::sync::Arc<ValidatedModel>,
    profile: Profile,
) -> Result<(), ModelError> {
    use catalog::evidence as c1;
    use cpg_core::generation_read::{AttemptSession, ProviderOptions};
    use futures::TryStreamExt;
    use normalized::Rows;
    let reader = AttemptSession::open(
        config,
        attempt,
        access,
        model.clone(),
        ProviderOptions::default(),
    )
    .await
    .map_err(ModelError::codec)?;
    let session = runtime.session(access);
    let mut d = selection::build::Data::new(runtime.budget());
    macro_rules! read{($($f:ident:$ty:ty,)*)=>{$({let permit=access.read::<$ty>()?;session.register(&permit,reader.table(&permit).map_err(ModelError::codec)?)?;let query=session.query(&format!("SELECT * FROM \"{}\"",<$ty>::NAME)).await.map_err(ModelError::codec)?;let mut stream=query.execute_stream().await.map_err(ModelError::codec)?;while let Some(batch)=stream.try_next().await.map_err(ModelError::codec)?{d.visit(<$ty>::NAME,&batch)?;}})*};}
    lctx_model::catalog_inputs!(read);
    lctx_model::catalog_outputs!(read);
    lctx_model::catalog_evidence_inputs!(read);
    lctx_model::catalog_runtime_inputs!(read);
    lctx_model::catalog_evidence_outputs!(read);
    lctx_model::catalog_selection_inputs!(read);
    drop(session);
    reader.close().await.map_err(ModelError::codec)?;
    let b = runtime.budget();
    let original = c1::build::build(&d.source, b)?;
    d.evidence.matches(&original)?;
    if profile == Profile::Behavioral {
        let link = d
            .evidence
            .field_locations
            .iter()
            .next()
            .expect("actual Local location reaches C1")
            .clone();
        let access = d.evidence.accesses.get(link.assessment).unwrap();
        let option = d.source.catalog.options.get(access.option).unwrap();
        let q = d
            .source
            .core
            .qualifications
            .get(access.qualification)
            .unwrap();
        let field = d.source.core.fields.get(access.field).unwrap();
        let owner = d.source.core.ownership.get(access.owner).unwrap();
        let output = selection::build::build(&d, b)?;
        assert!(
            output
                .witnesses
                .get(selection::Witness::ReceiverLocation { link: link.id() }.id())
                .is_some()
        );
        for kind in [
            selection::FieldRelationship::ExactReader,
            selection::FieldRelationship::ExactStorage,
        ] {
            let classified = selection::evaluate::Prepared::new(&d, &output, b)?.classify(
                option.member,
                q.context,
                &selection::Requirement {
                    predicate: selection::Predicate::ConfigurationRelationship {
                        name: field.name.as_str().into(),
                        kind,
                        target: selection::FieldTarget::Declaration {
                            entity: owner.entity,
                        },
                    },
                    quantifier: selection::Quantifier::AnyApplicable,
                },
                b,
            )?;
            assert_eq!(classified.outcome, selection::Outcome::Unresolved);
        }
        let _reservation = b.reserve(
            "catalog-runtime-forgery-control",
            d.evidence.field_locations.len() * std::mem::size_of::<c1::FieldLocationLink>(),
        )?;
        let rows = d
            .evidence
            .field_locations
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        for corruption in 0..3 {
            d.evidence.field_locations = Rows::new(b);
            for row in &rows {
                let mut changed = row.clone();
                if row.id() == link.id() {
                    match corruption {
                        0 => continue,
                        1 => changed.phase = calls::CallPhase::Init,
                        _ => changed.candidate =
                            serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
                                _,
                                serde::de::value::Error,
                            >::new(
                                [199u8; 16].into_iter()
                            ))
                            .unwrap(),
                    };
                }
                d.evidence.field_locations.insert(changed)?;
            }
            assert!(d.evidence.matches(&original).is_err());
        }
    }
    Ok(())
}
