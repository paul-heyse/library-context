//! Retrieval rendering is a pure consumer of published canonical facts. Embedding follows it.
use crate::{CoreError, sql};
use cpg_schema::{Id, IdHasher, Table, catalog::*, evidence::*, retrieval::*, wire::*};
use datafusion::prelude::SessionContext;
use std::collections::BTreeMap;

pub use cpg_schema::retrieval::catalog::RetrievalInputs;
pub fn derive(input: &RetrievalInputs) -> Result<Vec<Unit>, CoreError> {
    cpg_schema::retrieval::catalog::derive(input).map_err(|e| bad(e.to_string()))
}
async fn rows<T: Table>(ctx: &SessionContext) -> Result<Vec<T::Row>, CoreError>
where
    T::Row: cpg_schema::query::QueryRow,
{
    sql::fetch(
        ctx,
        &cpg_schema::query::Relation {
            name: T::NAME,
            deps: T::DEPS,
            sql: format!("SELECT * FROM {}", T::NAME),
        },
        sql::Params::new(),
    )
    .await
}
pub async fn load(ctx: &SessionContext) -> Result<RetrievalInputs, CoreError> {
    Ok(RetrievalInputs {
        members: rows::<CatalogMembers>(ctx).await?,
        bindings: rows::<CatalogBindings>(ctx).await?,
        signatures: rows::<CatalogSignatures>(ctx).await?,
        constructors: rows::<CatalogConstructors>(ctx).await?,
        parameters: rows::<CatalogParameters>(ctx).await?,
        fields: rows::<CatalogConfigurations>(ctx).await?,
        evidence: rows::<cpg_schema::catalog::CatalogEvidence>(ctx).await?,
        artifacts: rows::<CatalogArtifacts>(ctx).await?,
        spans: rows::<CatalogSpans>(ctx).await?,
        scenarios: rows::<CatalogScenarios>(ctx).await?,
        deployments: rows::<CatalogDeployments>(ctx).await?,
        associations: rows::<CatalogAssociations>(ctx).await?,
    })
}
fn bad(s: impl Into<String>) -> CoreError {
    CoreError::Bundle(s.into())
}
fn encoded<T: serde::Serialize>(value: &T) -> Result<String, CoreError> {
    serde_json::to_string(value).map_err(|e| bad(e.to_string()))
}

pub struct Artifacts {
    pub batches: BTreeMap<String, arrow_array::RecordBatch>,
    pub spec: Option<crate::embed::Spec>,
}
fn binary<'a>(
    values: impl Iterator<Item = Option<&'a [u8]>>,
    width: i32,
) -> Result<arrow_array::ArrayRef, CoreError> {
    use arrow_array::builder::FixedSizeBinaryBuilder;
    let mut b = FixedSizeBinaryBuilder::new(width);
    for value in values {
        match value {
            Some(v) => b.append_value(v)?,
            None => b.append_null(),
        }
    }
    Ok(std::sync::Arc::new(b.finish()))
}
/// Build independent immutable artifacts, with admission before any embedding request.
pub async fn materialize(
    snapshot: Id,
    units: &[Unit],
    embedder: Option<&dyn crate::embed::Embedder>,
    cache: Option<crate::postgres::Store>,
) -> Result<Artifacts, CoreError> {
    use arrow_array::{ArrayRef, RecordBatch, StringArray};
    use std::sync::Arc;
    let fragments = units
        .iter()
        .map(|u| cpg_schema::retrieval::fragments(u, 4096))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| bad(e.to_string()))?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    let spec = embedder.map(|e| e.spec().clone());
    if let Some(spec) = &spec {
        spec.validate().map_err(CoreError::Embed)?;
    }
    let dimensions = spec.as_ref().map_or(0, |s| s.dimensions as i32);
    let files = cpg_schema::retrieval::files(dimensions);
    let strings = |values: Vec<String>| -> ArrayRef { Arc::new(StringArray::from(values)) };
    let make = |name: &str, columns: Vec<ArrayRef>| -> Result<RecordBatch, CoreError> {
        Ok(RecordBatch::try_new(
            files
                .iter()
                .find(|f| f.name == name)
                .expect("finite retrieval file")
                .schema
                .clone(),
            columns,
        )?)
    };
    let mut batches = BTreeMap::new();
    batches.insert(
        "retrieval_units".into(),
        make(
            "retrieval_units",
            vec![
                binary(
                    units
                        .iter()
                        .map(|u| Some(u.unit_id.storage().0))
                        .collect::<Vec<_>>()
                        .iter()
                        .map(|id| id.as_ref().map(|a| a.as_slice())),
                    16,
                )?,
                strings(units.iter().map(|u| u.family.name().into()).collect()),
                strings(units.iter().map(encoded).collect::<Result<_, _>>()?),
            ],
        )?,
    );
    let subjects: Vec<_> = units
        .iter()
        .flat_map(|u| u.subjects.iter().map(|s| (u.unit_id.storage(), s)))
        .collect();
    batches.insert(
        "retrieval_subjects".into(),
        make(
            "retrieval_subjects",
            vec![
                binary(subjects.iter().map(|(id, _)| Some(id.0.as_slice())), 16)?,
                binary(
                    subjects
                        .iter()
                        .map(|(_, s)| match s {
                            Subject::Member(m) => Some(m.storage().0),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .iter()
                        .map(|i| i.as_ref().map(|i| i.as_slice())),
                    16,
                )?,
                binary(
                    subjects.iter().map(|(_, s)| match s {
                        Subject::Release(id) => Some(id.0.as_slice()),
                        _ => None,
                    }),
                    16,
                )?,
            ],
        )?,
    );
    let mut session = crate::embed::Session::new(snapshot, cache, 256 * 1024 * 1024);
    let mut status = vec!["not_requested".to_owned(); fragments.len()];
    let mut admitted = Vec::new();
    if let Some(embedder) = embedder {
        for (index, f) in fragments.iter().enumerate() {
            let request = embedder.spec().document_text(&f.text);
            match embedder.count_tokens(&request).await {
                Ok(n) if n <= embedder.spec().max_document_tokens as usize => {
                    admitted.push(index);
                    status[index] = "embedded".into();
                }
                Ok(_) => status[index] = "token_refused".into(),
                Err(_) => status[index] = "token_unavailable".into(),
            }
        }
    }
    let vectors = if let Some(embedder) = embedder {
        match session
            .texts(
                embedder,
                &admitted
                    .iter()
                    .map(|i| fragments[*i].text.clone())
                    .collect::<Vec<_>>(),
                crate::embed::Usage::Operation,
            )
            .await
        {
            Ok(vectors) => vectors,
            Err(CoreError::EmbeddingService(_)) => {
                for i in &admitted {
                    status[*i] = "embedding_failed".into();
                }
                admitted.clear();
                vec![]
            }
            Err(error) => return Err(error),
        }
    } else {
        vec![]
    };
    batches.insert(
        "retrieval_fragments".into(),
        make(
            "retrieval_fragments",
            vec![
                binary(
                    fragments.iter().map(|f| Some(f.fragment_id.0.as_slice())),
                    16,
                )?,
                binary(
                    fragments
                        .iter()
                        .map(|f| Some(f.unit_id.storage().0))
                        .collect::<Vec<_>>()
                        .iter()
                        .map(|i| i.as_ref().map(|i| i.as_slice())),
                    16,
                )?,
                strings(fragments.iter().map(|f| f.family.name().into()).collect()),
                strings(fragments.iter().map(|f| f.text.clone()).collect()),
                binary(
                    fragments
                        .iter()
                        .map(|f| Some(f.content_digest.0.as_slice())),
                    32,
                )?,
                strings(status),
            ],
        )?,
    );
    let mut vb = arrow_array::builder::FixedSizeListBuilder::new(
        arrow_array::builder::Float32Builder::new(),
        dimensions,
    )
    .with_field(arrow_schema::Field::new(
        "item",
        arrow_schema::DataType::Float32,
        false,
    ));
    for vector in &vectors {
        vb.values().append_slice(vector);
        vb.append(true);
    }
    let hashes: Vec<_> = admitted
        .iter()
        .map(|i| {
            crate::embed::input_hash(
                &spec
                    .as_ref()
                    .expect("admitted spec")
                    .document_text(&fragments[*i].text),
            )
        })
        .collect();
    batches.insert(
        "retrieval_vectors".into(),
        make(
            "retrieval_vectors",
            vec![
                binary(
                    admitted
                        .iter()
                        .map(|i| Some(fragments[*i].fragment_id.0.as_slice())),
                    16,
                )?,
                binary(hashes.iter().map(|h| Some(h.0.as_slice())), 32)?,
                Arc::new(vb.finish()),
            ],
        )?,
    );
    let receipt = RetrievalReceipt {
        snapshot_id: SnapshotId::from_storage(snapshot),
        input_digest: IdHasher::new("retrieval-inputs-v1")
            .str(&encoded(&units)?)
            .finish_digest(),
        view_revision: VIEW_REVISION,
        render_revision: RENDER_REVISION,
        spec_hash: spec.as_ref().map(crate::embed::Spec::hash),
    };
    batches.insert(
        "retrieval_receipt".into(),
        make("retrieval_receipt", vec![strings(vec![encoded(&receipt)?])])?,
    );
    // Canonical sorting also makes subject null ordering explicit.
    for (name, batch) in &mut batches {
        let file = files
            .iter()
            .find(|f| f.name == name)
            .expect("retrieval file");
        *batch = cpg_schema::table::canonical_sort(batch, file.key)?;
    }
    Ok(Artifacts { batches, spec })
}

/// Re-embed briefs in the same exact specification as units. Analytical receipts stay canonical.
pub async fn prepare(
    ctx: &SessionContext,
    snapshot: Id,
    embedder: Option<&dyn crate::embed::Embedder>,
    cache: Option<crate::postgres::Store>,
) -> Result<Artifacts, CoreError> {
    let units = derive(&load(ctx).await?)?;
    let mut artifacts = materialize(snapshot, &units, embedder, cache.clone()).await?;
    if embedder.is_none() {
        cpg_schema::query_row! {struct CanonicalSpec {spec:String}}
        let specs: Vec<CanonicalSpec> = sql::fetch(
            ctx,
            &cpg_schema::query::Relation {
                name: "retrieval_canonical_spec",
                deps: &["embedding_specs"],
                sql: "SELECT spec FROM embedding_specs ORDER BY spec_hash".into(),
            },
            sql::Params::new(),
        )
        .await?;
        if specs.len() > 1 {
            return Err(bad("mixed canonical embedding specifications"));
        }
        if let Some(row) = specs.first() {
            let spec = crate::embed::Spec::parse(&row.spec).map_err(bad)?;
            let files = cpg_schema::retrieval::files(spec.dimensions as i32);
            artifacts.batches.insert(
                "retrieval_vectors".into(),
                arrow_array::RecordBatch::new_empty(
                    files
                        .iter()
                        .find(|f| f.name == "retrieval_vectors")
                        .expect("finite file")
                        .schema
                        .clone(),
                ),
            );
            let receipt = RetrievalReceipt {
                snapshot_id: SnapshotId::from_storage(snapshot),
                input_digest: IdHasher::new("retrieval-inputs-v1")
                    .str(&encoded(&units)?)
                    .finish_digest(),
                view_revision: VIEW_REVISION,
                render_revision: RENDER_REVISION,
                spec_hash: Some(spec.hash()),
            };
            artifacts.batches.insert(
                "retrieval_receipt".into(),
                arrow_array::RecordBatch::try_new(
                    files
                        .iter()
                        .find(|f| f.name == "retrieval_receipt")
                        .expect("finite file")
                        .schema
                        .clone(),
                    vec![std::sync::Arc::new(arrow_array::StringArray::from(vec![
                        encoded(&receipt)?,
                    ]))],
                )?,
            );
            artifacts.spec = Some(spec);
        }
    }
    if let Some(embedder) = embedder {
        use arrow_array::{ArrayRef, RecordBatch, StringArray};
        use std::sync::Arc;
        cpg_schema::query_row! {struct BriefText {brief_id:Id,chunk:i64,text:String}}
        let texts: Vec<BriefText> = sql::fetch(
            ctx,
            &cpg_schema::query::Relation {
                name: "retrieval_brief_text",
                deps: &["brief_documents"],
                sql: "SELECT brief_id,chunk,text FROM brief_documents ORDER BY brief_id,chunk"
                    .into(),
            },
            sql::Params::new(),
        )
        .await?;
        let mut session = crate::embed::Session::new(snapshot, cache, 256 * 1024 * 1024);
        let vectors = session
            .texts(
                embedder,
                &texts.iter().map(|t| t.text.clone()).collect::<Vec<_>>(),
                crate::embed::Usage::Brief,
            )
            .await?;
        let spec = embedder.spec();
        let dimensions = spec.dimensions as i32;
        let files = cpg_schema::bundle::files(dimensions);
        let mut vb = arrow_array::builder::FixedSizeListBuilder::new(
            arrow_array::builder::Float32Builder::new(),
            dimensions,
        )
        .with_field(arrow_schema::Field::new(
            "item",
            arrow_schema::DataType::Float32,
            false,
        ));
        for v in vectors {
            vb.values().append_slice(&v);
            vb.append(true);
        }
        let hashes: Vec<_> = texts
            .iter()
            .map(|t| crate::embed::input_hash(&spec.document_text(&t.text)))
            .collect();
        let columns: Vec<ArrayRef> = vec![
            binary(texts.iter().map(|t| Some(t.brief_id.0.as_slice())), 16)?,
            Arc::new(arrow_array::Int64Array::from(
                texts.iter().map(|t| t.chunk).collect::<Vec<_>>(),
            )),
            binary(hashes.iter().map(|h| Some(h.0.as_slice())), 32)?,
            Arc::new(vb.finish()),
        ];
        artifacts.batches.insert(
            "vectors".into(),
            RecordBatch::try_new(
                files
                    .iter()
                    .find(|f| f.name == "vectors")
                    .expect("brief vector file")
                    .schema
                    .clone(),
                columns,
            )?,
        );
        artifacts.batches.insert(
            "embedding_spec".into(),
            RecordBatch::try_new(
                files
                    .iter()
                    .find(|f| f.name == "embedding_spec")
                    .expect("spec file")
                    .schema
                    .clone(),
                vec![
                    binary(std::iter::once(Some(spec.hash().0.as_slice())), 32)?,
                    Arc::new(StringArray::from(vec![spec.canonical_json()])),
                ],
            )?,
        );
    }
    Ok(artifacts)
}

/// Immutable replay receipt outside Delta. Existing generation publication remains the publisher.
pub fn save(
    artifacts: &Artifacts,
    root: &std::path::Path,
    snapshot: Id,
) -> Result<std::path::PathBuf, CoreError> {
    use sha2::{Digest as _, Sha256};
    let path = root.join("retrieval").join(snapshot.hex());
    let encoded_batches = artifacts
        .batches
        .iter()
        .map(|(name, batch)| Ok((name.clone(), crate::bundle::ipc_bytes(batch)?)))
        .collect::<Result<BTreeMap<_, _>, CoreError>>()?;
    let files: BTreeMap<_, _> = encoded_batches
        .iter()
        .map(|(name, bytes)| (name.clone(), format!("{:x}", Sha256::digest(bytes))))
        .collect();
    let receipt = encoded(
        &serde_json::json!({"format":2,"snapshot_id":snapshot,"spec":artifacts.spec,"files":files}),
    )?;
    let key = format!("{:x}", Sha256::digest(receipt.as_bytes()));
    let dir = path.join(&key);
    fs_err::create_dir_all(&dir)?;
    for (name, bytes) in encoded_batches {
        immutable_write(&dir.join(format!("{name}.arrow")), &bytes)?;
    }
    immutable_write(&dir.join("ARTIFACTS.json"), receipt.as_bytes())?;
    // Select only a completely written artifact set. Immutable files remain independent of this pointer.
    let temporary = path.join(format!(
        "CURRENT-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(key.as_bytes())?;
        file.sync_all()?;
    }
    fs_err::rename(temporary, path.join("CURRENT"))?;
    Ok(dir)
}
pub fn restore(root: &std::path::Path, snapshot: Id) -> Result<Option<Artifacts>, CoreError> {
    use sha2::{Digest as _, Sha256};
    let path = root.join("retrieval").join(snapshot.hex());
    if !path.exists() {
        return Ok(None);
    }
    let key = String::from_utf8(bounded_read(&path.join("CURRENT"), 64)?)
        .map_err(|_| bad("retrieval selection encoding"))?;
    if key.len() != 64
        || !key
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(bad("retrieval selection identity"));
    }
    let dir = path.join(&key);
    if !fs_err::symlink_metadata(&dir)?.is_dir() {
        return Err(bad("retrieval artifact directory"));
    }
    let receipt_bytes = bounded_read(&dir.join("ARTIFACTS.json"), 64 * 1024)?;
    let receipt: serde_json::Value =
        serde_json::from_slice(&receipt_bytes).map_err(|e| bad(e.to_string()))?;
    if receipt["format"] != 2
        || receipt["snapshot_id"] != snapshot.hex()
        || format!("{:x}", Sha256::digest(&receipt_bytes)) != key
    {
        return Err(bad("retrieval replay receipt mismatch"));
    }
    let spec: Option<crate::embed::Spec> =
        serde_json::from_value(receipt["spec"].clone()).map_err(|e| bad(e.to_string()))?;
    if let Some(spec) = &spec {
        spec.validate().map_err(bad)?;
    }
    let declared = cpg_schema::bundle::files(spec.as_ref().map_or(0, |s| s.dimensions as i32));
    let mut batches = BTreeMap::new();
    for (name, hash) in receipt["files"]
        .as_object()
        .ok_or_else(|| bad("retrieval replay files"))?
    {
        if !matches!(
            name.as_str(),
            "retrieval_units"
                | "retrieval_subjects"
                | "retrieval_fragments"
                | "retrieval_vectors"
                | "retrieval_receipt"
                | "vectors"
                | "embedding_spec"
        ) {
            return Err(bad("unexpected retrieval replay artifact"));
        }
        let bytes = bounded_read(&dir.join(format!("{name}.arrow")), 512 * 1024 * 1024)?;
        if hash.as_str() != Some(format!("{:x}", Sha256::digest(&bytes)).as_str()) {
            return Err(bad("retrieval replay digest mismatch"));
        }
        let mut reader = arrow_ipc::reader::FileReader::try_new(std::io::Cursor::new(bytes), None)?;
        let batch = reader.next().ok_or_else(|| bad("missing replay batch"))??;
        if reader.next().is_some()
            || declared
                .iter()
                .find(|f| f.name == name)
                .is_none_or(|f| f.schema != batch.schema())
        {
            return Err(bad("retrieval replay schema mismatch"));
        }
        batches.insert(name.clone(), batch);
    }
    if cpg_schema::retrieval::files(spec.as_ref().map_or(0, |s| s.dimensions as i32))
        .iter()
        .any(|f| !batches.contains_key(f.name))
    {
        return Err(bad("retrieval replay closure missing"));
    }
    Ok(Some(Artifacts { batches, spec }))
}

fn bounded_read(path: &std::path::Path, limit: u64) -> Result<Vec<u8>, CoreError> {
    let meta = fs_err::symlink_metadata(path)?;
    if !meta.is_file() || meta.len() > limit {
        return Err(bad("resource_refused: replay file kind/size"));
    }
    Ok(fs_err::read(path)?)
}
static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
fn immutable_write(path: &std::path::Path, bytes: &[u8]) -> Result<(), CoreError> {
    use std::io::Write;
    if path.exists() {
        if bounded_read(path, 512 * 1024 * 1024)? == bytes {
            return Ok(());
        }
        return Err(bad("immutable retrieval artifact differs"));
    }
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let temporary = path.with_extension(format!("tmp-{}-{n}", std::process::id()));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let result = (|| -> Result<(), CoreError> {
        file.write_all(bytes)?;
        file.sync_all()?;
        match std::fs::hard_link(&temporary, path) {
            Ok(()) => Ok(()),
            Err(e)
                if e.kind() == std::io::ErrorKind::AlreadyExists
                    && bounded_read(path, 512 * 1024 * 1024)? == bytes =>
            {
                Ok(())
            }
            Err(e) => Err(e.into()),
        }
    })();
    fs_err::remove_file(temporary)?;
    result
}
