//! Stage G (DESIGN §6.4; ADR-0017, ADR-0019): a serving generation, read from a published
//! snapshot and normalized, so that rebuilding it from the store gives byte-identical files.
//!
//! - **Queries.** Each served file is one query over the published session, sorted by its
//!   declared key (`cpg_schema::bundle`). Codebook values are served as their text.
//! - **Normalization.** Every column is cast to its declared type and rebuilt through a builder.
//!   View types, scan metadata and the bytes under null slots never reach a file.
//! - **Files.** One record batch per file, in the Arrow IPC file format: V5 metadata, 64-byte
//!   alignment, uncompressed.
//! - **`MANIFEST.json`**, with sorted keys:
//!   - the snapshot and its content and compiler digests;
//!   - each file's sha256, rows and serving schema digest;
//!   - the spec hash;
//!   - a coverage summary.
//!
//! The generation key is the first 16 hex digits of the SHA-256 of the manifest without its key,
//! so two generations share a key only when their files and provenance are the same.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use arrow_array::builder::{
    BooleanBuilder, FixedSizeBinaryBuilder, FixedSizeListBuilder, Float32Builder, Int64Builder,
    StringBuilder,
};
use arrow_array::cast::AsArray;
use arrow_array::types::{Float32Type, Int64Type};
use arrow_array::{Array, ArrayRef, RecordBatch};
use arrow_cast::{CastOptions, cast_with_options};
use arrow_ipc::MetadataVersion;
use arrow_ipc::reader::FileReader;
use arrow_ipc::writer::{FileWriter, IpcWriteOptions};
use arrow_schema::{DataType, Field, FieldRef, SchemaRef};
use cpg_schema::bundle::{ServingFile, files, schema_digest};
use cpg_schema::codebook::{
    AssertionKind, BoundaryReason, Codebook, CoverageStatus, EvidenceKind, EvidenceStatus,
    FactFamily, FindingKind, ReviewState, ScopeKind, SupportRole,
};
use cpg_schema::findings::{ASSERTION_POLICY, SLOT_SECTIONS};
use cpg_schema::id::Id;
use datafusion::prelude::SessionContext;
use serde_json::{Map, Value, json};
use sha2::{Digest as _, Sha256};

use crate::{CoreError, sql};

/// The manifest's format version: bumped when a served file, its schema or the manifest changes.
pub const FORMAT: u64 = 1;

/// A built generation: its key, directory and manifest.
#[derive(Debug, Clone)]
pub struct Generation {
    pub key: String,
    pub dir: PathBuf,
    pub manifest: Value,
}

fn bad(message: impl Into<String>) -> CoreError {
    CoreError::Bundle(message.into())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn sha256(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

/// A codebook column as its text.
fn text_of<C: Codebook>(expr: &str) -> String {
    let arms: String = C::all()
        .iter()
        .map(|c| format!(" WHEN {} THEN '{}'", c.code(), c.text()))
        .collect();
    format!("CASE {expr}{arms} END")
}

/// An assertion kind's section, as text, from the kind policy.
fn section_of(expr: &str) -> String {
    let arms: String = ASSERTION_POLICY
        .iter()
        .map(|(k, s, _)| format!(" WHEN {} THEN '{}'", k.code(), s.text()))
        .collect();
    format!("CASE {expr}{arms} END")
}

/// Each served file's query, sorted by its key; `lexical_text` is computed after the others.
fn query(name: &str) -> Option<String> {
    Some(match name {
        "briefs" => format!(
            "SELECT b.brief_id, b.seed_node_id, b.access_path, b.title, b.applicable_case, \
                    b.documentation_only, {review} AS review_state, o.text AS outcome, \
                    COALESCE({status}, 'unresolved') AS outcome_status \
             FROM briefs b LEFT JOIN ( \
               SELECT ba.brief_id, a.text, a.evidence_status FROM brief_assertions ba \
               JOIN assertions a ON a.assertion_id = ba.assertion_id \
               WHERE a.assertion_kind = {outcome}) o ON o.brief_id = b.brief_id \
             ORDER BY b.brief_id",
            review = text_of::<ReviewState>("b.review_state"),
            status = text_of::<EvidenceStatus>("o.evidence_status"),
            outcome = AssertionKind::Outcome.code(),
        ),
        "assertions" => format!(
            "SELECT ba.brief_id, ba.ordinal, a.assertion_id, {kind} AS kind, {section} AS section, \
                    {status} AS status, a.text, a.applicable_case, a.conditions, a.limitations, \
                    a.template_version \
             FROM brief_assertions ba JOIN assertions a ON a.assertion_id = ba.assertion_id \
             ORDER BY ba.brief_id, ba.ordinal",
            kind = text_of::<AssertionKind>("a.assertion_kind"),
            section = section_of("a.assertion_kind"),
            status = text_of::<EvidenceStatus>("a.evidence_status"),
        ),
        "supports" => format!(
            "SELECT s.assertion_id, {role} AS role, s.ordinal, s.finding_id, \
                    {kind} AS finding_kind, s.evidence_id \
             FROM assertion_support s LEFT JOIN findings f ON f.finding_id = s.finding_id \
             ORDER BY s.assertion_id, role, s.ordinal",
            role = text_of::<SupportRole>("s.role"),
            kind = text_of::<FindingKind>("f.finding_kind"),
        ),
        "evidence" => format!(
            "SELECT e.evidence_id, {kind} AS kind, e.node_id, COALESCE(sf.path, d.path) AS path, \
                    e.start_byte, e.end_byte, e.text \
             FROM evidence e \
             LEFT JOIN (SELECT module_node_id, min(path) AS path FROM ({files}) \
                        GROUP BY module_node_id) sf ON sf.module_node_id = e.module_node_id \
             LEFT JOIN (SELECT node_id, min(path) AS path FROM documents GROUP BY node_id) d \
               ON d.node_id = e.module_node_id \
             ORDER BY e.evidence_id",
            kind = text_of::<EvidenceKind>("e.evidence_kind"),
            files = cpg_schema::flows::display_files_sql(),
        ),
        // FORMAT 1 serves the members it always served: the seed's aliases, its `public_alias`
        // finding's members. FORMAT 2 serves every public spelling (the holistic assessment's A1).
        "brief_members" => format!(
            "SELECT m.brief_id, m.access_path, m.export_node_id, m.declaration_node_id \
             FROM ({aliased}) m ORDER BY m.brief_id, m.access_path",
            aliased = format_1_members()
        ),
        "symbol_map" => format!(
            "SELECT DISTINCT access_path AS symbol, brief_id FROM ({aliased}) m \
             ORDER BY symbol, brief_id",
            aliased = format_1_members()
        ),
        "embedding_spec" => {
            "SELECT spec_hash, spec FROM embedding_specs ORDER BY spec_hash".to_owned()
        }
        "vectors" => "SELECT d.brief_id, d.chunk, d.input_hash, c.vector FROM brief_documents d \
                      JOIN embedding_cache c \
                        ON c.spec_hash = d.spec_hash AND c.input_hash = d.input_hash \
                      ORDER BY d.brief_id, d.chunk"
            .to_owned(),
        _ => return None,
    })
}

const STRICT: CastOptions<'static> = CastOptions {
    safe: false,
    format_options: arrow_cast::display::FormatOptions::new(),
};

/// A column cast toward its declared type: a fixed-size binary through `Binary`, a fixed-size
/// list with its child left nullable (the builder below declares it).
fn cast_to(a: &ArrayRef, field: &Field) -> Result<ArrayRef, CoreError> {
    Ok(match field.data_type() {
        DataType::FixedSizeBinary(_) => {
            let binary = cast_with_options(a, &DataType::Binary, &STRICT)?;
            cast_with_options(&binary, field.data_type(), &STRICT)?
        }
        DataType::FixedSizeList(child, n) => {
            let loose = DataType::FixedSizeList(
                Arc::new(Field::new(child.name(), child.data_type().clone(), true)),
                *n,
            );
            cast_with_options(a, &loose, &STRICT)?
        }
        other => cast_with_options(a, other, &STRICT)?,
    })
}

/// A column rebuilt through a builder of its declared type: fresh buffers, null slots zeroed.
fn rebuild(a: &ArrayRef, field: &FieldRef) -> Result<ArrayRef, CoreError> {
    let a = cast_to(a, field)?;
    let n = a.len();
    let null = |i: usize| a.is_null(i);
    let built: ArrayRef = match field.data_type() {
        DataType::FixedSizeBinary(width) => {
            let v = a.as_fixed_size_binary();
            let mut b = FixedSizeBinaryBuilder::with_capacity(n, *width);
            for i in 0..n {
                if null(i) {
                    b.append_null();
                } else {
                    b.append_value(v.value(i))?;
                }
            }
            Arc::new(b.finish())
        }
        DataType::Utf8 => {
            let v = a.as_string::<i32>();
            let mut b = StringBuilder::with_capacity(n, v.value_data().len());
            for i in 0..n {
                if null(i) {
                    b.append_null();
                } else {
                    b.append_value(v.value(i));
                }
            }
            Arc::new(b.finish())
        }
        DataType::Boolean => {
            let v = a.as_boolean();
            let mut b = BooleanBuilder::with_capacity(n);
            for i in 0..n {
                b.append_option((!null(i)).then(|| v.value(i)));
            }
            Arc::new(b.finish())
        }
        DataType::Int64 => {
            let v = a.as_primitive::<Int64Type>();
            let mut b = Int64Builder::with_capacity(n);
            for i in 0..n {
                b.append_option((!null(i)).then(|| v.value(i)));
            }
            Arc::new(b.finish())
        }
        DataType::FixedSizeList(child, size) => {
            let v = a.as_fixed_size_list();
            let mut b = FixedSizeListBuilder::with_capacity(
                Float32Builder::with_capacity(n * *size as usize),
                *size,
                n,
            )
            .with_field(child.clone());
            for i in 0..n {
                if null(i) {
                    return Err(bad(format!("{}: a null vector", field.name())));
                }
                let item = v.value(i);
                let floats = item.as_primitive::<Float32Type>();
                if floats.null_count() > 0 {
                    return Err(bad(format!("{}: a null vector component", field.name())));
                }
                b.values().append_slice(floats.values());
                b.append(true);
            }
            Arc::new(b.finish())
        }
        other => return Err(bad(format!("{other} is not a served column type"))),
    };
    if !field.is_nullable() && built.null_count() > 0 {
        return Err(bad(format!("{}: nulls in a non-null column", field.name())));
    }
    Ok(built)
}

/// A query's rows as one normalized batch of `schema`.
async fn normalized(
    ctx: &SessionContext,
    statement: &str,
    schema: &SchemaRef,
) -> Result<RecordBatch, CoreError> {
    let batches = sql::query(ctx, statement).await?.collect().await?;
    let mut columns: Vec<ArrayRef> = Vec::with_capacity(schema.fields().len());
    for field in schema.fields() {
        let parts: Vec<ArrayRef> = batches
            .iter()
            .map(|b| {
                b.column_by_name(field.name())
                    .cloned()
                    .ok_or_else(|| bad(format!("the query lacks {}", field.name())))
            })
            .collect::<Result<_, _>>()?;
        let whole: ArrayRef = if parts.is_empty() {
            arrow_array::new_empty_array(field.data_type())
        } else {
            let refs: Vec<&dyn Array> = parts.iter().map(|p| p.as_ref()).collect();
            arrow_select::concat::concat(&refs)?
        };
        columns.push(rebuild(&whole, field)?);
    }
    Ok(RecordBatch::try_new(schema.clone(), columns)?)
}

/// A public name split for lexical search: the path, its segments, and each segment's words (at
/// underscores and case changes), each once, in first-seen order.
fn name_words(path: &str, out: &mut Vec<String>) {
    let mut push = |w: &str| {
        if !w.is_empty() && !out.iter().any(|x| x == w) {
            out.push(w.to_owned());
        }
    };
    push(path);
    for segment in path.split('.') {
        push(segment);
        let parts: Vec<&str> = segment.split('_').collect();
        for part in &parts {
            if parts.len() > 1 {
                push(part);
            }
            let chars: Vec<char> = part.chars().collect();
            let mut start = 0;
            for i in 1..chars.len() {
                let (a, b) = (chars[i - 1], chars[i]);
                let next_lower = chars.get(i + 1).is_some_and(|c| c.is_lowercase());
                if (a.is_lowercase() && b.is_uppercase())
                    || (a.is_uppercase() && b.is_uppercase() && next_lower)
                    || (a.is_alphabetic() != b.is_alphabetic())
                {
                    push(&chars[start..i].iter().collect::<String>());
                    start = i;
                }
            }
            if start > 0 {
                push(&chars[start..].iter().collect::<String>());
            }
        }
    }
}

/// The members bundle `FORMAT` 1 serves: each brief's members that its seed's `public_alias`
/// finding names (the seed's aliases, as before the holistic assessment's A1).
fn format_1_members() -> String {
    format!(
        "SELECT m.* FROM brief_members m JOIN briefs b ON b.brief_id = m.brief_id \
         WHERE EXISTS (SELECT 1 FROM findings f \
           JOIN finding_members fm ON fm.finding_id = f.finding_id \
           WHERE f.subject_node_id = b.seed_node_id AND f.finding_kind = {public_alias} \
             AND fm.label = m.access_path)",
        public_alias = FindingKind::PublicAlias.code()
    )
}

/// `lexical_text`: each brief's documents, then the words of its public names (§11.2).
async fn lexical(ctx: &SessionContext, schema: &SchemaRef) -> Result<RecordBatch, CoreError> {
    let docs = normalized(
        ctx,
        &format!(
            "SELECT d.brief_id, d.chunk, d.text, m.access_path FROM brief_documents d \
             LEFT JOIN ({aliased}) m ON m.brief_id = d.brief_id \
             ORDER BY d.brief_id, d.chunk, m.access_path",
            aliased = format_1_members()
        ),
        &Arc::new(arrow_schema::Schema::new(vec![
            Field::new("brief_id", DataType::FixedSizeBinary(16), false),
            Field::new("chunk", DataType::Int64, false),
            Field::new("text", DataType::Utf8, false),
            Field::new("access_path", DataType::Utf8, true),
        ])),
    )
    .await?;
    let ids = docs.column(0).as_fixed_size_binary();
    let chunks = docs.column(1).as_primitive::<Int64Type>();
    let texts = docs.column(2).as_string::<i32>();
    let paths = docs.column(3).as_string::<i32>();
    /// A brief's document chunks and the words of its public names.
    type Lexical = (Vec<(i64, String)>, Vec<String>);
    let mut per: BTreeMap<Vec<u8>, Lexical> = BTreeMap::new();
    for i in 0..docs.num_rows() {
        let entry = per.entry(ids.value(i).to_vec()).or_default();
        if !entry.0.iter().any(|(c, _)| *c == chunks.value(i)) {
            entry.0.push((chunks.value(i), texts.value(i).to_owned()));
        }
        if !paths.is_null(i) {
            name_words(paths.value(i), &mut entry.1);
        }
    }
    let mut id_b = FixedSizeBinaryBuilder::with_capacity(per.len(), 16);
    let mut text_b = StringBuilder::new();
    for (id, (chunks, words)) in &per {
        id_b.append_value(id)?;
        let body: Vec<&str> = chunks.iter().map(|(_, t)| t.as_str()).collect();
        text_b.append_value(format!("{}\n{}", body.join("\n"), words.join(" ")));
    }
    Ok(RecordBatch::try_new(
        schema.clone(),
        vec![Arc::new(id_b.finish()), Arc::new(text_b.finish())],
    )?)
}

/// One batch as an Arrow IPC file's bytes (V5, 64-byte alignment, uncompressed).
fn ipc_bytes(batch: &RecordBatch) -> Result<Vec<u8>, CoreError> {
    let options = IpcWriteOptions::try_new(64, false, MetadataVersion::V5)?;
    let mut out = Vec::new();
    {
        let mut writer = FileWriter::try_new_with_options(&mut out, &batch.schema(), options)?;
        writer.write(batch)?;
        writer.finish()?;
    }
    Ok(out)
}

/// `value` with every object's keys sorted (serde_json keeps insertion order in this build).
fn sorted(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let ordered: BTreeMap<String, Value> =
                map.into_iter().map(|(k, v)| (k, sorted(v))).collect();
            Value::Object(ordered.into_iter().collect::<Map<String, Value>>())
        }
        Value::Array(items) => Value::Array(items.into_iter().map(sorted).collect()),
        other => other,
    }
}

/// The generation key of a manifest (its `generation` field left out).
fn key_of(manifest: &Value) -> Result<String, CoreError> {
    let mut body = manifest.clone();
    if let Value::Object(map) = &mut body {
        map.remove("generation");
    }
    let canonical = serde_json::to_string(&sorted(body)).map_err(|e| bad(e.to_string()))?;
    Ok(sha256(canonical.as_bytes())[..16].to_owned())
}

/// `(label…, count)` rows of a grouping query whose label columns are text.
async fn counted(
    ctx: &SessionContext,
    statement: &str,
) -> Result<Vec<(Vec<String>, i64)>, CoreError> {
    let mut out = Vec::new();
    for b in sql::query(ctx, statement).await?.collect().await? {
        let n = b.num_columns();
        let labels: Vec<ArrayRef> = (0..n - 1)
            .map(|c| cast_with_options(b.column(c), &DataType::Utf8, &STRICT))
            .collect::<Result<_, _>>()?;
        let counts = cast_with_options(b.column(n - 1), &DataType::Int64, &STRICT)?;
        let counts = counts.as_primitive::<Int64Type>();
        for i in 0..b.num_rows() {
            let row = labels
                .iter()
                .map(|l| {
                    let l = l.as_string::<i32>();
                    if l.is_null(i) {
                        "-".to_owned()
                    } else {
                        l.value(i).to_owned()
                    }
                })
                .collect();
            out.push((row, counts.value(i)));
        }
    }
    Ok(out)
}

/// Nested objects of counts, one level per label.
fn nest(rows: Vec<(Vec<String>, i64)>) -> Value {
    fn insert(map: &mut Map<String, Value>, labels: &[String], n: i64) {
        match labels {
            [] => {}
            [last] => {
                map.insert(last.clone(), json!(n));
            }
            [first, rest @ ..] => {
                let child = map
                    .entry(first.clone())
                    .or_insert_with(|| Value::Object(Map::new()));
                if let Value::Object(child) = child {
                    insert(child, rest, n);
                }
            }
        }
    }
    let mut root = Map::new();
    for (labels, n) in rows {
        insert(&mut root, &labels, n);
    }
    Value::Object(root)
}

/// The coverage summary (§6.4): coverage by scope, family and status; boundaries by reason;
/// analysis invocations by completion; briefs by review state; unresolved slots by section (the
/// §B11 gap metric).
async fn coverage(ctx: &SessionContext) -> Result<Value, CoreError> {
    let unresolved = EvidenceStatus::Unresolved.code();
    Ok(json!({
        "coverage": nest(counted(ctx, &format!(
            "SELECT {}, {}, {}, count(*) FROM coverage GROUP BY 1, 2, 3 ORDER BY 1, 2, 3",
            text_of::<ScopeKind>("scope_kind"),
            text_of::<FactFamily>("fact_family"),
            text_of::<CoverageStatus>("status"),
        )).await?),
        "boundaries": nest(counted(ctx, &format!(
            "SELECT {}, count(*) FROM boundaries GROUP BY 1 ORDER BY 1",
            text_of::<BoundaryReason>("reason"),
        )).await?),
        "invocations": nest(counted(ctx, &format!(
            "SELECT {}, count(*) FROM analysis_invocations GROUP BY 1 ORDER BY 1",
            text_of::<CoverageStatus>("completion"),
        )).await?),
        "briefs": nest(counted(ctx, &format!(
            "SELECT {}, CASE WHEN documentation_only THEN 'documentation_only' \
                    ELSE 'analysis_backed' END, count(*) FROM briefs GROUP BY 1, 2 ORDER BY 1, 2",
            text_of::<ReviewState>("review_state"),
        )).await?),
        "unresolved_slots": nest(counted(ctx, &format!(
            "SELECT {}, count(*) FROM assertions WHERE evidence_status = {unresolved} \
             GROUP BY 1 ORDER BY 1",
            section_of("assertion_kind"),
        )).await?),
        // Slot sections a brief has no assertion in (increment-1 deep review F4).
        "absent_slots": nest(counted(ctx, &format!(
            "SELECT s.section, count(*) FROM briefs b CROSS JOIN (VALUES {slots}) AS s(section) \
             LEFT ANTI JOIN (SELECT ba.brief_id, {section} AS section FROM brief_assertions ba \
                             JOIN assertions a ON a.assertion_id = ba.assertion_id) p \
               ON p.brief_id = b.brief_id AND p.section = s.section \
             GROUP BY 1 ORDER BY 1",
            slots = SLOT_SECTIONS
                .iter()
                .map(|s| format!("('{}')", s.text()))
                .collect::<Vec<_>>()
                .join(", "),
            section = section_of("a.assertion_kind"),
        )).await?),
        "slot_sections": SLOT_SECTIONS.iter().map(|s| s.text()).collect::<Vec<_>>(),
    }))
}

/// Build a generation from a published session into `out/<key>/`. An existing generation of the
/// same key must hold the same bytes; it is then left as it is.
pub async fn build(ctx: &SessionContext, out: &Path) -> Result<Generation, CoreError> {
    let snapshot = counted(
        ctx,
        "SELECT encode(CAST(snapshot_id AS BYTEA), 'hex'), \
                encode(CAST(content_digest AS BYTEA), 'hex'), \
                encode(CAST(compiler_digest AS BYTEA), 'hex'), count(*) \
         FROM snapshots GROUP BY 1, 2, 3",
    )
    .await?;
    // The library the generation serves: an acquired library's name, else a source tree's label.
    let release = counted(
        ctx,
        "SELECT COALESCE(min(library), min(label)), COALESCE(min(requirement), ''), count(*) \
         FROM releases",
    )
    .await?;
    let [(release, _)] = release.as_slice() else {
        return Err(bad("the snapshot has no release"));
    };
    let [(ids, _)] = snapshot.as_slice() else {
        return Err(bad(format!(
            "the session holds {} snapshot identities, not one",
            snapshot.len()
        )));
    };
    // One spec, or none (lexical only); never two vector spaces in one generation.
    let specs = counted(
        ctx,
        "SELECT encode(CAST(spec_hash AS BYTEA), 'hex'), spec, count(*) FROM embedding_specs \
         GROUP BY 1, 2 ORDER BY 1",
    )
    .await?;
    let documents = counted(
        ctx,
        "SELECT encode(CAST(spec_hash AS BYTEA), 'hex'), count(*) FROM brief_documents \
         WHERE spec_hash IS NOT NULL GROUP BY 1 ORDER BY 1",
    )
    .await?;
    if specs.len() > 1 || documents.len() > specs.len() {
        return Err(bad(format!(
            "mixed embedding specs: {} declared, {} used by documents",
            specs.len(),
            documents.len()
        )));
    }
    let (spec_hash, dimensions) = match specs.first() {
        Some((labels, _)) => {
            let spec: crate::embed::Spec =
                serde_json::from_str(&labels[1]).map_err(|e| bad(format!("the spec: {e}")))?;
            if documents.first().is_some_and(|(d, _)| d[0] != labels[0]) {
                return Err(bad("the documents' spec is not the snapshot's"));
            }
            (Some(labels[0].clone()), spec.dimensions as i32)
        }
        None => (None, 0),
    };

    let served: Vec<ServingFile> = files(dimensions);
    let mut built: Vec<(&'static str, Vec<u8>, usize, String)> = Vec::new();
    for file in &served {
        let batch = match query(file.name) {
            Some(q) => normalized(ctx, &q, &file.schema).await?,
            None => lexical(ctx, &file.schema).await?,
        };
        let digest = schema_digest(&file.schema).map_err(bad)?;
        built.push((file.name, ipc_bytes(&batch)?, batch.num_rows(), digest));
    }
    let embedded = counted(
        ctx,
        "SELECT 'documents', count(*) FROM brief_documents WHERE input_hash IS NOT NULL",
    )
    .await?;
    let vectors = built.iter().find(|b| b.0 == "vectors").map_or(0, |b| b.2);
    if embedded.first().map_or(0, |e| e.1) as usize != vectors {
        return Err(bad(format!(
            "{vectors} vectors for {} embedded documents",
            embedded.first().map_or(0, |e| e.1)
        )));
    }

    let mut entries = Map::new();
    for (name, bytes, rows, digest) in &built {
        entries.insert(
            (*name).to_owned(),
            json!({
                "file": format!("{name}.arrow"),
                "rows": rows,
                "sha256": sha256(bytes),
                "schema_digest": digest,
            }),
        );
    }
    let mut manifest = json!({
        "format": FORMAT,
        "library": release[0],
        "requirement": release[1],
        "snapshot_id": ids[0],
        "content_digest": ids[1],
        "compiler_digest": ids[2],
        "spec_hash": spec_hash,
        "files": Value::Object(entries),
        "summary": coverage(ctx).await?,
    });
    let key = key_of(&manifest)?;
    manifest["generation"] = json!(key);
    let manifest = sorted(manifest);
    let text = serde_json::to_string_pretty(&manifest).map_err(|e| bad(e.to_string()))? + "\n";

    let dir = out.join(&key);
    fs_err::create_dir_all(out)?;
    let staging = out.join(format!(".building-{key}-{}", std::process::id()));
    if staging.exists() {
        fs_err::remove_dir_all(&staging)?;
    }
    fs_err::create_dir_all(&staging)?;
    for (name, bytes, _, _) in &built {
        fs_err::write(staging.join(format!("{name}.arrow")), bytes)?;
    }
    fs_err::write(staging.join("MANIFEST.json"), &text)?;
    if dir.exists() {
        for entry in fs_err::read_dir(&staging)? {
            let path = entry?.path();
            let name = path.file_name().expect("a file name");
            if fs_err::read(&path)? != fs_err::read(dir.join(name))? {
                fs_err::remove_dir_all(&staging)?;
                return Err(bad(format!(
                    "generation {key} exists with other bytes in {}",
                    name.to_string_lossy()
                )));
            }
        }
        fs_err::remove_dir_all(&staging)?;
    } else {
        fs_err::rename(&staging, &dir)?;
    }
    Ok(Generation { key, dir, manifest })
}

/// Build the generation of a published snapshot.
pub async fn bundle(root: &Path, snapshot_id: Id, out: &Path) -> Result<Generation, CoreError> {
    let (_, ctx) = crate::snapshot::published(root, snapshot_id)
        .await?
        .ok_or_else(|| bad(format!("snapshot {} is not published", snapshot_id.hex())))?;
    build(&ctx, out).await
}

/// Check a generation against its manifest: each file's sha256, row count and serving schema
/// digest, and the key, which must also name the directory.
pub fn verify(dir: &Path) -> Result<Value, CoreError> {
    let manifest: Value = serde_json::from_str(&fs_err::read_to_string(dir.join("MANIFEST.json"))?)
        .map_err(|e| bad(format!("MANIFEST.json: {e}")))?;
    let files = manifest["files"]
        .as_object()
        .ok_or_else(|| bad("MANIFEST.json lists no files"))?;
    for (name, entry) in files {
        let file = entry["file"]
            .as_str()
            .ok_or_else(|| bad(format!("{name}: no file")))?;
        let bytes = fs_err::read(dir.join(file))?;
        if Some(sha256(&bytes).as_str()) != entry["sha256"].as_str() {
            return Err(bad(format!("{file}: its sha256 differs from the manifest")));
        }
        let reader = FileReader::try_new(std::io::Cursor::new(bytes), None)?;
        let digest = schema_digest(&reader.schema()).map_err(bad)?;
        if Some(digest.as_str()) != entry["schema_digest"].as_str() {
            return Err(bad(format!(
                "{file}: its schema digest differs from the manifest"
            )));
        }
        let rows: usize = reader
            .map(|b| b.map(|b| b.num_rows()))
            .sum::<Result<usize, _>>()?;
        if Some(rows as u64) != entry["rows"].as_u64() {
            return Err(bad(format!("{file}: {rows} rows, not the manifest's")));
        }
    }
    let key = key_of(&manifest)?;
    let named = dir.file_name().map(|n| n.to_string_lossy().into_owned());
    if manifest["generation"].as_str() != Some(key.as_str()) || named.as_deref() != Some(&key) {
        return Err(bad(format!(
            "the generation key is {key}, not the manifest's or the directory's"
        )));
    }
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_names_split_into_words() {
        let mut words = Vec::new();
        name_words("fastmcp.FastMCP.custom_route", &mut words);
        assert_eq!(
            words,
            [
                "fastmcp.FastMCP.custom_route",
                "fastmcp",
                "FastMCP",
                "Fast",
                "MCP",
                "custom_route",
                "custom",
                "route"
            ]
        );
    }
}
