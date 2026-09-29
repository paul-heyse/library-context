//! Coarse rebuilds over validated publications. Delta remains the only canonical authority.
use crate::{
    CoreError,
    analyze::Analysis,
    attempt::{Published, compiler_digest, content_digest_with},
    catalog::CompileInputs,
};
use arrow_array::RecordBatch;
use cpg_schema::{
    Digest, Id, IdHasher, Table,
    query::QueryRow,
    table::{canonical_sort, rebind_snapshot},
    tables::{Producers, Runs, SnapshotsRow},
};
use datafusion::prelude::SessionContext;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::{collections::BTreeSet, path::Path};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Facts,
    ContractNormalization,
    ContextualAssociation,
    BehavioralEnrichment,
    CatalogFinalization,
    Retrieval,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Reused,
    Recomputed,
    NotRequested,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub stage: Stage,
    pub dependencies: Vec<Stage>,
    pub key: Digest,
    pub outcome: Outcome,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    pub format: u32,
    pub source: Id,
    pub snapshot: Id,
    pub compiler: Digest,
    pub steps: Vec<Step>,
}

async fn batch<T: Table>(ctx: &SessionContext) -> Result<RecordBatch, CoreError> {
    let batches = ctx.table(T::NAME).await?.collect().await?;
    let declared = batches
        .iter()
        .map(crate::delta::to_declared::<T>)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(canonical_sort(
        &arrow_select::concat::concat_batches(&T::schema(), &declared)?,
        T::key(),
    )?)
}
/// Every canonical raw relation participates, including empty memberships and coverage rows.
async fn raw(
    ctx: &SessionContext,
    source: Id,
    target: Id,
) -> Result<Vec<(&'static str, RecordBatch)>, CoreError> {
    let mut rows = Vec::new();
    macro_rules! load {($($t:ty),+)=>{$(rows.push((<$t as Table>::NAME,rebind_snapshot::<$t>(&batch::<$t>(ctx).await?,source,target)?));)+};}
    cpg_schema::for_each_table!(load);
    let producers = rows
        .iter()
        .find(|r| r.0 == Producers::NAME)
        .ok_or(CoreError::MissingTable(Producers::NAME))?;
    let mut producers = <Producers as Table>::Row::read_batch(&producers.1)?;
    let compiler_ids = producers
        .iter()
        .filter(|p| p.tool == "lctx-compiler")
        .map(|p| p.producer_id)
        .collect::<BTreeSet<_>>();
    producers.retain(|p| !compiler_ids.contains(&p.producer_id));
    for (name, b) in &mut rows {
        if *name == Producers::NAME {
            *b = Producers::to_sorted_batch(&producers)?;
        }
        if *name == Runs::NAME {
            let mut runs = <Runs as Table>::Row::read_batch(b)?;
            runs.retain(|r| !compiler_ids.contains(&r.producer_id));
            *b = Runs::to_sorted_batch(&runs)?;
        }
    }
    Ok(rows)
}
fn input_digest(
    raw: &[(&str, RecordBatch)],
    snapshot: Id,
    inputs: &CompileInputs,
    analysis: Option<&Analysis>,
) -> Result<Digest, CoreError> {
    let mut hash = Sha256::new();
    // Arrow IPC has canonical schemas/order; only the enclosing snapshot identity is excluded.
    struct Sink<'a>(&'a mut Sha256);
    impl std::io::Write for Sink<'_> {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.update(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    for (name, b) in raw {
        hash.update(name.as_bytes());
        let mut neutral = None;
        macro_rules! bind {($($t:ty),+)=>{$(if *name==<$t as Table>::NAME {neutral=Some(rebind_snapshot::<$t>(b,snapshot,Id::ZERO)?);})+};}
        cpg_schema::for_each_table!(bind);
        let b = neutral.ok_or_else(|| CoreError::Analysis("undeclared rebuild input".into()))?;
        let mut writer = arrow_ipc::writer::StreamWriter::try_new(Sink(&mut hash), &b.schema())?;
        writer.write(&b)?;
        writer.finish()?;
    }
    let mut key = IdHasher::new("coarse-catalog-input-v1");
    key.str(&format!("{:x}", hash.finalize()))
        .digest_field(inputs.digest(analysis))
        .digest_field(crate::attempt::semantic_digest());
    if let Some(e) = &inputs.embedder {
        key.digest_field(e.spec().hash());
    }
    Ok(key.finish_digest())
}

/// Fresh publication from a pinned fact snapshot. Reuse is conservative at the canonical
/// compilation boundary: any semantic producer/config/spec change recomputes affected output.
/// A full clean compile is available as the equality oracle; no cache can admit invalid inputs.
pub async fn catalog(
    root: &Path,
    source: Id,
    target: Id,
    inputs: &CompileInputs,
    analysis: Option<&Analysis>,
    clean: bool,
) -> Result<(Published, Receipt), CoreError> {
    let (_, ctx) = crate::snapshot::published(root, source)
        .await?
        .ok_or_else(|| CoreError::Analysis("rebuild source is not published".into()))?;
    crate::validate::rebuild_source(&ctx).await?;
    let compilation = batch::<cpg_schema::catalog::CatalogCompilation>(&ctx).await?;
    let compilation = cpg_schema::catalog::CatalogCompilationRow::read_batch(&compilation)?;
    let [original] = compilation.as_slice() else {
        return Err(CoreError::Analysis(
            "rebuild requires one catalog compilation".into(),
        ));
    };
    if inputs.profile.behavioral() && original.profile != "behavioral" {
        return Err(CoreError::Analysis(
            "behavioral rebuild requires captured flow facts; run a full behavioral compile".into(),
        ));
    }
    let raw = raw(&ctx, source, target).await?;
    let key = input_digest(&raw, target, inputs, analysis)?;
    let producers = <Producers as Table>::Row::read_batch(&batch::<Producers>(&ctx).await?)?;
    let producer_matches = producers
        .iter()
        .any(|p| p.tool == "lctx-compiler" && p.build_digest == crate::attempt::semantic_digest());
    let specs = cpg_schema::findings::EmbeddingSpecsRow::read_batch(
        &batch::<cpg_schema::findings::EmbeddingSpecs>(&ctx).await?,
    )?;
    let requested_spec = inputs.embedder.as_ref().map(|e| e.spec().hash());
    let spec_matches = match requested_spec {
        Some(spec) => specs.len() == 1 && specs[0].spec_hash == spec,
        None => specs.is_empty(),
    };
    let reuse = !clean
        && producer_matches
        && original.input_digest == inputs.digest(analysis)
        && spec_matches;
    let outcome = if reuse {
        Outcome::Reused
    } else {
        Outcome::Recomputed
    };
    let mut steps = vec![Step {
        stage: Stage::Facts,
        dependencies: vec![],
        key,
        outcome: Outcome::Reused,
    }];
    let published = if reuse {
        let public = cpg_schema::findings::PublicPathsRow::read_batch(
            &batch::<cpg_schema::findings::PublicPaths>(&ctx).await?,
        )?;
        let (mut contracts, catalog_steps) = crate::stage_cache::contracts(
            root,
            &ctx,
            source,
            target,
            &inputs.public_roots,
            &public,
            false,
        )
        .await?;
        steps.extend(catalog_steps);
        crate::stage_cache::finish_members(&ctx, &mut contracts).await?;
        copy_publication(root, &ctx, source, target, &contracts).await?
    } else {
        // Old derived output may embody the policy being replaced. Only captured raw facts are
        // admitted here; the ordinary compiler fully validates them and every new derived row.
        crate::attempt::compile_catalog_mode(root, target, raw, inputs, analysis, clean).await?
    };
    if !reuse {
        steps.extend(published.catalog_stages.clone());
    }
    steps.push(Step {
        stage: Stage::BehavioralEnrichment,
        dependencies: vec![Stage::Facts, Stage::ContractNormalization],
        key,
        outcome: if inputs.profile.behavioral() {
            outcome
        } else {
            Outcome::NotRequested
        },
    });
    steps.push(Step {
        stage: Stage::CatalogFinalization,
        dependencies: vec![
            Stage::ContractNormalization,
            Stage::ContextualAssociation,
            Stage::BehavioralEnrichment,
        ],
        key,
        outcome: Outcome::Recomputed,
    });
    let receipt = Receipt {
        format: 1,
        source,
        snapshot: target,
        compiler: compiler_digest(),
        steps,
    };
    Ok((published, receipt))
}
async fn copy_publication(
    root: &Path,
    ctx: &SessionContext,
    source: Id,
    target: Id,
    contracts: &crate::catalog::Contracts,
) -> Result<Published, CoreError> {
    let mut versions = crate::snapshot::Versions::new();
    let mut rows = Vec::new();
    let mut stages = cpg_schema::metrics::Stages::default();
    let mut catalog_batches = std::collections::BTreeMap::new();
    macro_rules! contract {
        ($table:ty,$field:ident) => {
            catalog_batches.insert(
                <$table as Table>::NAME,
                <$table as Table>::to_sorted_batch(&contracts.$field)?,
            );
        };
    }
    contract!(cpg_schema::catalog::CatalogMembers, members);
    contract!(cpg_schema::catalog::CatalogConstructors, constructors);
    contract!(cpg_schema::catalog::CatalogBindings, bindings);
    contract!(cpg_schema::catalog::CatalogSignatures, signatures);
    contract!(cpg_schema::catalog::CatalogParameters, parameters);
    contract!(cpg_schema::catalog::CatalogEvidence, evidence);
    contract!(cpg_schema::catalog::CatalogTypes, types);
    contract!(cpg_schema::catalog::CatalogTypeArgs, type_args);
    contract!(
        cpg_schema::catalog::CatalogTypeObservations,
        type_observations
    );
    contract!(cpg_schema::catalog::CatalogSurfaces, surfaces);
    contract!(cpg_schema::catalog::CatalogConfigurations, configurations);
    contract!(cpg_schema::catalog::CatalogFieldLinks, field_links);
    contract!(
        cpg_schema::selection::catalog::CatalogSelectionDomains,
        domains
    );
    macro_rules! contextual {
        ($table:ty,$field:ident) => {
            catalog_batches.insert(
                <$table as Table>::NAME,
                <$table as Table>::to_sorted_batch(&contracts.contextual.$field)?,
            );
        };
    }
    contextual!(cpg_schema::evidence::CatalogArtifacts, artifacts);
    contextual!(cpg_schema::evidence::CatalogSpans, spans);
    contextual!(cpg_schema::evidence::CatalogScenarios, scenarios);
    contextual!(cpg_schema::evidence::CatalogDeployments, deployments);
    contextual!(cpg_schema::evidence::CatalogAssociations, associations);
    macro_rules! copy {($($t:ty),+)=>{$(
        let name=<$t as Table>::NAME;
        let b=if let Some(b)=catalog_batches.get(name){b.clone()}else{rebind_snapshot::<$t>(&batch::<$t>(ctx).await?,source,target)?};
        let v=crate::attempt::write::<$t>(root,&b,target).await?;
        versions.insert(<$t as Table>::NAME.to_owned(),v);rows.push((<$t as Table>::NAME,b.num_rows() as i64));
    )+};}
    cpg_schema::for_each_table!(copy);
    cpg_schema::for_each_derived_table!(copy);
    cpg_schema::for_each_analysis_table!(copy);
    let new = crate::snapshot::session(root, target, &versions).await?;
    let (violations, _) = crate::validate::validate_costed(&new).await?;
    if !violations.is_empty() {
        return Err(CoreError::Invalid(violations));
    }
    let runs = <Runs as Table>::Row::read_batch(&batch::<Runs>(&new).await?)?
        .into_iter()
        .map(|r| r.run_id)
        .collect::<Vec<_>>();
    let specs = cpg_schema::findings::EmbeddingSpecsRow::read_batch(
        &batch::<cpg_schema::findings::EmbeddingSpecs>(&new).await?,
    )?;
    let values = cpg_schema::embedding::UsedEmbeddingsRow::read_batch(
        &batch::<cpg_schema::embedding::UsedEmbeddings>(&new).await?,
    )?;
    let embedded = specs.first().map(|s| {
        (
            s.spec_hash,
            cpg_schema::embedding::receipt_digest(s.spec_hash, &values),
        )
    });
    let digest = content_digest_with(&runs, embedded);
    let declarations = rows
        .iter()
        .map(|(name, count)| {
            Ok(SnapshotsRow {
                snapshot_id: target,
                content_digest: digest,
                table_name: (*name).into(),
                table_version: versions[*name] as i64,
                schema_digest: crate::attempt::schema_digest_of(name)?,
                compiler_digest: compiler_digest(),
                row_count: *count,
            })
        })
        .collect::<Result<Vec<_>, CoreError>>()?;
    crate::attempt::publish(root, target, &declarations).await?;
    stages.mark("reused validated canonical compilation");
    Ok(Published {
        snapshot_id: target,
        content_digest: digest,
        versions,
        rows,
        stages: stages.stages,
        catalog_stages: vec![],
    })
}

pub async fn configuration(
    root: &Path,
    snapshot: Id,
) -> Result<cpg_schema::catalog::CatalogCompilationRow, CoreError> {
    let (_, ctx) = crate::snapshot::published(root, snapshot)
        .await?
        .ok_or_else(|| CoreError::Analysis("source snapshot not published".into()))?;
    let rows = cpg_schema::catalog::CatalogCompilationRow::read_batch(
        &batch::<cpg_schema::catalog::CatalogCompilation>(&ctx).await?,
    )?;
    let [row] = rows.as_slice() else {
        return Err(CoreError::Analysis(
            "source requires one compilation contract".into(),
        ));
    };
    Ok(row.clone())
}
/// Retrieval-only rebuild uses the existing immutable artifact publisher and never selects it.
pub async fn retrieval(
    root: &Path,
    snapshot: Id,
    out: &Path,
    embedder: Option<&dyn crate::embed::Embedder>,
    cache: Option<crate::postgres::Store>,
) -> Result<(crate::bundle::Generation, Receipt), CoreError> {
    let (_, ctx) = crate::snapshot::published(root, snapshot)
        .await?
        .ok_or_else(|| CoreError::Analysis("source snapshot not published".into()))?;
    let (violations, _) = crate::validate::validate_costed(&ctx).await?;
    if !violations.is_empty() {
        return Err(CoreError::Invalid(violations));
    }
    let generation =
        crate::bundle::bundle_with_embedding(root, snapshot, out, embedder, cache).await?;
    let key = IdHasher::new("retrieval-realization-v1")
        .str(&generation.key)
        .finish_digest();
    let receipt = Receipt {
        format: 1,
        source: snapshot,
        snapshot,
        compiler: compiler_digest(),
        steps: vec![Step {
            stage: Stage::Retrieval,
            dependencies: vec![Stage::CatalogFinalization],
            key,
            outcome: Outcome::Recomputed,
        }],
    };
    Ok((generation, receipt))
}
