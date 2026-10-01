//! Independent C1/C2 source-link and runtime-witness corruption controls.
use lctx_model::domain::{stages::*, *};
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
    let source_link=d.evidence.source_field_links.iter().next()
        .expect("admitted native record yields actual C1 source links").clone();
    let _source_charge=b.reserve("catalog-source-link-corruption-control",
        d.evidence.source_field_links.len()*std::mem::size_of::<c1::SourceFieldLink>())?;
    let source_rows=d.evidence.source_field_links.iter().cloned().collect::<Vec<_>>();
    let sibling=d.evidence.source_field_links.iter().find(|r|r.parameter_option!=source_link.parameter_option)
        .expect("independent sibling field provides nonleakage twin").parameter_option;
    for corruption in 0..3 {
        d.evidence.source_field_links=Rows::new(b);
        for row in &source_rows {
            let mut changed=row.clone();
            if row.id()==source_link.id() {
                match corruption {
                    0=>continue,
                    1=>changed.runtime_value=normalized::callables::Knowledge::Known,
                    _=>changed.parameter_option=sibling,
                }
            }
            d.evidence.source_field_links.insert(changed)?;
        }
        assert!(d.evidence.matches(&original).is_err(),"C1 source association corruption {corruption} must refuse");
    }
    d.evidence.source_field_links=Rows::new(b);
    for row in source_rows{d.evidence.source_field_links.insert(row)?;}
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
