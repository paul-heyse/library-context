//! Current-only disposable pure-stage artifacts. Keys bind complete scans, not lookup hits.
use crate::{
    CoreError,
    catalog::Contracts,
    rebuild::{Outcome, Stage, Step},
};
use arrow_array::RecordBatch;
use cpg_schema::{Digest, Id, IdHasher, Table, query::QueryRow};
use datafusion::prelude::SessionContext;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::{collections::BTreeMap, path::Path};

pub(crate) async fn key(
    ctx: &SessionContext,
    source: Id,
    deps: &[&str],
    parameters: &str,
    code: Digest,
) -> Result<Digest, CoreError> {
    let mut hash = Sha256::new();
    for name in deps
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>()
    {
        let input = ctx.table(name).await?.collect().await?;
        let mut normalized = None;
        macro_rules! bind {($($t:ty),+)=>{$(if name==<$t as Table>::NAME {
            let rows=input.iter().map(crate::arrow_types::to_declared::<$t>).collect::<Result<Vec<_>,_>>()?;
            let b=arrow_select::concat::concat_batches(&<$t as Table>::schema(),&rows)?;
            let b=cpg_schema::table::canonical_sort(&b,<$t as Table>::key())?;
            normalized=Some(cpg_schema::table::rebind_snapshot::<$t>(&b,source,Id::ZERO)?);
        })+};}
        cpg_schema::for_each_table!(bind);
        cpg_schema::for_each_derived_table!(bind);
        cpg_schema::for_each_analysis_table!(bind);
        let b = normalized
            .ok_or_else(|| CoreError::Analysis(format!("undeclared stage input {name}")))?;
        let mut bytes = Vec::new();
        let mut writer = arrow_ipc::writer::StreamWriter::try_new(&mut bytes, &b.schema())?;
        writer.write(&b)?;
        writer.finish()?;
        drop(writer);
        hash.update(name.as_bytes());
        hash.update((bytes.len() as u64).to_le_bytes());
        hash.update(bytes);
    }
    Ok(IdHasher::new("pure-stage-v1")
        .str(&format!("{:x}", hash.finalize()))
        .str(parameters)
        .digest_field(code)
        .finish_digest())
}
type Batches = BTreeMap<String, RecordBatch>;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    format: u32,
    key: Digest,
    files: BTreeMap<String, String>,
}
fn read(root: &Path, name: &str, key: Digest) -> Option<Batches> {
    let dir = root.join("rebuild-cache").join(name);
    let manifest = dir.join("CURRENT.json");
    if std::fs::metadata(&manifest).ok()?.len() > 64 * 1024 {
        return None;
    }
    let e: Envelope = serde_json::from_slice(&std::fs::read(manifest).ok()?).ok()?;
    if e.format != 1 || e.key != key || e.files.len() > 32 {
        return None;
    }
    let mut out = BTreeMap::new();
    let mut total = 0;
    for (table, hash) in e.files {
        if !table.bytes().all(|b| b.is_ascii_lowercase() || b == b'_') {
            return None;
        }
        let path = dir.join(format!("{table}.arrow"));
        total += std::fs::metadata(&path).ok()?.len();
        if total > 512 * 1024 * 1024 {
            return None;
        }
        let bytes = std::fs::read(path).ok()?;
        if hash != format!("{:x}", Sha256::digest(&bytes)) {
            return None;
        }
        let mut reader =
            arrow_ipc::reader::FileReader::try_new(std::io::Cursor::new(bytes), None).ok()?;
        let batch = reader.next()?.ok()?;
        if reader.next().is_some() {
            return None;
        }
        out.insert(table, batch);
    }
    Some(out)
}
fn write(root: &Path, name: &str, key: Digest, value: &Batches) -> Result<(), CoreError> {
    let dir = root.join("rebuild-cache").join(name);
    std::fs::create_dir_all(&dir)?;
    static SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let mut files = BTreeMap::new();
    for (table, batch) in value {
        let mut bytes = Vec::new();
        let mut writer = arrow_ipc::writer::FileWriter::try_new(&mut bytes, &batch.schema())?;
        writer.write(batch)?;
        writer.finish()?;
        drop(writer);
        files.insert(table.clone(), format!("{:x}", Sha256::digest(&bytes)));
        let tmp = dir.join(format!(".{table}.{}.{sequence}.tmp", std::process::id()));
        std::fs::write(&tmp, bytes)?;
        std::fs::rename(tmp, dir.join(format!("{table}.arrow")))?;
    }
    let e = Envelope {
        format: 1,
        key,
        files,
    };
    let tmp = dir.join(format!(".CURRENT.{}.{sequence}.tmp", std::process::id()));
    std::fs::write(
        &tmp,
        serde_json::to_vec(&e).map_err(|e| CoreError::Analysis(e.to_string()))?,
    )?;
    std::fs::rename(tmp, dir.join("CURRENT.json"))?;
    Ok(())
}
macro_rules! normalization_tables {
    ($f:ident) => {
        $f!(cpg_schema::catalog::CatalogMembers, members);
        $f!(cpg_schema::catalog::CatalogConstructors, constructors);
        $f!(cpg_schema::catalog::CatalogBindings, bindings);
        $f!(cpg_schema::catalog::CatalogSignatures, signatures);
        $f!(cpg_schema::catalog::CatalogParameters, parameters);
        $f!(cpg_schema::catalog::CatalogEvidence, evidence);
        $f!(cpg_schema::catalog::CatalogTypes, types);
        $f!(cpg_schema::catalog::CatalogTypeArgs, type_args);
        $f!(
            cpg_schema::catalog::CatalogTypeObservations,
            type_observations
        );
        $f!(cpg_schema::catalog::CatalogSurfaces, surfaces);
        $f!(cpg_schema::catalog::CatalogConfigurations, configurations);
        $f!(cpg_schema::catalog::CatalogFieldLinks, field_links);
    };
}
macro_rules! contextual_tables {
    ($f:ident) => {
        $f!(cpg_schema::evidence::CatalogArtifacts, artifacts);
        $f!(cpg_schema::evidence::CatalogSpans, spans);
        $f!(cpg_schema::evidence::CatalogScenarios, scenarios);
        $f!(cpg_schema::evidence::CatalogDeployments, deployments);
        $f!(cpg_schema::evidence::CatalogAssociations, associations);
    };
}
fn output(contracts: &Contracts, snapshot: Id, association: bool) -> Result<Batches, CoreError> {
    let mut out = BTreeMap::new();
    macro_rules! save {
        ($table:ty,$field:ident) => {
            out.insert(
                <$table as Table>::NAME.into(),
                cpg_schema::table::rebind_snapshot::<$table>(
                    &<$table as Table>::to_sorted_batch(&contracts.$field)?,
                    snapshot,
                    Id::ZERO,
                )?,
            );
        };
    }
    macro_rules! contextual {
        ($table:ty,$field:ident) => {
            out.insert(
                <$table as Table>::NAME.into(),
                cpg_schema::table::rebind_snapshot::<$table>(
                    &<$table as Table>::to_sorted_batch(&contracts.contextual.$field)?,
                    snapshot,
                    Id::ZERO,
                )?,
            );
        };
    }
    if association {
        contextual_tables!(contextual);
        save!(
            cpg_schema::selection::catalog::CatalogSelectionDomains,
            domains
        );
    } else {
        normalization_tables!(save);
    }
    Ok(out)
}
fn rows<T: Table>(batches: &Batches, snapshot: Id) -> Result<Vec<T::Row>, CoreError>
where
    T::Row: QueryRow,
{
    let b = batches
        .get(T::NAME)
        .ok_or(CoreError::MissingTable(T::NAME))?;
    Ok(T::Row::read_batch(
        &cpg_schema::table::rebind_snapshot::<T>(b, Id::ZERO, snapshot)?,
    )?)
}
fn decoded(batches: &Batches, snapshot: Id, association: bool) -> Result<Contracts, CoreError> {
    let mut out = Contracts::default();
    let mut count = 0;
    macro_rules! load {
        ($table:ty,$field:ident) => {
            out.$field = rows::<$table>(batches, snapshot)?;
            count += 1;
        };
    }
    macro_rules! contextual {
        ($table:ty,$field:ident) => {
            out.contextual.$field = rows::<$table>(batches, snapshot)?;
            count += 1;
        };
    }
    if association {
        contextual_tables!(contextual);
        load!(
            cpg_schema::selection::catalog::CatalogSelectionDomains,
            domains
        );
    } else {
        normalization_tables!(load);
    }
    if count != batches.len() {
        return Err(CoreError::Analysis(
            "unexpected cached stage relation".into(),
        ));
    }
    Ok(out)
}
fn save(
    root: &Path,
    name: &str,
    key: Digest,
    contracts: &Contracts,
    snapshot: Id,
    association: bool,
) {
    if let Err(error) =
        output(contracts, snapshot, association).and_then(|out| write(root, name, key, &out))
    {
        tracing::warn!(%error,stage=name,"disposable stage cache unavailable");
    }
}

pub(crate) async fn contracts(
    root: &Path,
    ctx: &SessionContext,
    source: Id,
    target: Id,
    roots: &[String],
    public: &[cpg_schema::findings::PublicPathsRow],
    clean: bool,
) -> Result<(Contracts, Vec<Step>), CoreError> {
    let facts = crate::catalog::load_facts(ctx).await?;
    let mut public_parameters = cpg_schema::findings::PublicPaths::to_sorted_batch(public)?;
    public_parameters = cpg_schema::table::rebind_snapshot::<cpg_schema::findings::PublicPaths>(
        &public_parameters,
        source,
        Id::ZERO,
    )?;
    let mut bytes = Vec::new();
    let mut writer =
        arrow_ipc::writer::StreamWriter::try_new(&mut bytes, &public_parameters.schema())?;
    writer.write(&public_parameters)?;
    writer.finish()?;
    drop(writer);
    let parameters = format!(
        "{}:{:x}",
        serde_json::to_string(roots).map_err(|e| CoreError::Analysis(e.to_string()))?,
        Sha256::digest(bytes)
    );
    let normalization = key(
        ctx,
        source,
        &crate::catalog::input_dependencies(),
        &parameters,
        crate::attempt::semantic_digest(),
    )
    .await?;
    let cached = if clean {
        None
    } else {
        read(root, "normalization", normalization).and_then(|b| decoded(&b, target, false).ok())
    };
    // Disposable bytes are never semantic authority. Admission uses the same pure owner as
    // publication validation, including a validly encoded but incorrect cached result.
    let expected = crate::catalog::derive_contracts(&facts, target, roots, public)?;
    let expected_output = output(&expected, target, false)?;
    let reused = cached
        .as_ref()
        .is_some_and(|c| output(c, target, false).is_ok_and(|b| b == expected_output));
    let mut contracts = if reused {
        cached.expect("admitted cache")
    } else {
        expected
    };
    if !reused {
        save(
            root,
            "normalization",
            normalization,
            &contracts,
            target,
            false,
        );
    }
    let code = IdHasher::new("association-transform-v1")
        .digest_field(crate::attempt::semantic_digest())
        .str(env!("LCTX_ASSOCIATION_SOURCE_DIGEST"))
        .finish_digest();
    let association = key(
        ctx,
        source,
        &crate::evidence::input_dependencies(),
        &normalization.hex(),
        code,
    )
    .await?;
    let cached = if clean {
        None
    } else {
        read(root, "association", association).and_then(|b| decoded(&b, target, true).ok())
    };
    let evidence = crate::evidence::load(ctx).await?;
    let expected_context =
        crate::evidence::PreparedEvidence::new(&facts, &evidence).derive(target, &contracts)?;
    contracts.contextual = expected_context;
    let expected_domains = crate::catalog_domains::derive(&facts, &evidence, &contracts, target)?;
    contracts.domains = expected_domains;
    let expected_output = output(&contracts, target, true)?;
    let association_reused = cached
        .as_ref()
        .is_some_and(|c| output(c, target, true).is_ok_and(|b| b == expected_output));
    if association_reused {
        let cached = cached.expect("admitted cache");
        contracts.contextual = cached.contextual;
        contracts.domains = cached.domains;
    } else {
        save(root, "association", association, &contracts, target, true);
    }
    Ok((
        contracts,
        vec![
            Step {
                stage: Stage::ContractNormalization,
                dependencies: vec![Stage::Facts],
                key: normalization,
                outcome: if reused {
                    Outcome::Reused
                } else {
                    Outcome::Recomputed
                },
            },
            Step {
                stage: Stage::ContextualAssociation,
                dependencies: vec![Stage::Facts, Stage::ContractNormalization],
                key: association,
                outcome: if association_reused {
                    Outcome::Reused
                } else {
                    Outcome::Recomputed
                },
            },
        ],
    ))
}
/// Rebind final brief availability from its unchanged semantic producer, never invent it.
pub(crate) async fn finish_members(
    ctx: &SessionContext,
    contracts: &mut Contracts,
) -> Result<(), CoreError> {
    let batches = ctx.table("catalog_members").await?.collect().await?;
    let mut members = BTreeMap::new();
    for batch in batches {
        for row in
            cpg_schema::catalog::CatalogMembersRow::read_batch(&crate::arrow_types::to_declared::<
                cpg_schema::catalog::CatalogMembers,
            >(&batch)?)?
        {
            members.insert(row.member_id, (row.brief_status, row.brief_reason));
        }
    }
    for member in &mut contracts.members {
        if let Some((status, reason)) = members.get(&member.member_id) {
            member.brief_status = status.clone();
            member.brief_reason = reason.clone();
        }
    }
    Ok(())
}
