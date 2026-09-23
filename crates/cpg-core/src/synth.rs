//! Stage F (DESIGN §4.1, §10; ADR-0005, ADR-0019): assertions and briefs from findings and
//! evidence, by deterministic templates and extractive selection. There is no generative model.
//!
//! - **Outcome** (§10.3), the first source that applies: the seed's docstring summary line; else
//!   the lead sentence of a passage that mentions the seed **exactly**; else `unresolved`. Both are
//!   verbatim bytes of their source, with their span (review F8).
//! - **Public access** from Pass A's `public_alias`, **coordinates** from its delegations,
//!   **analysis boundaries** from its boundaries, unresolved sites and truncation, **parameters**
//!   from the extracted signature.
//! - An assertion's status is derived from its supports, never chosen (§10.2); an `unresolved`
//!   slot has no text.

use std::collections::{BTreeMap, BTreeSet};

use arrow_array::{Array, BooleanArray, FixedSizeBinaryArray, Int16Array, Int64Array, StringArray};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use cpg_schema::codebook::{
    AssertionKind, Codebook, EvidenceKind, EvidenceStatus, ExtractionMode, FindingKind,
    MentionClass, Modality, ParameterKind, ReviewState, StopReason, SupportRole,
};
use cpg_schema::findings::recipe::{self, AssertionKey};
use cpg_schema::findings::{
    ASSERTION_POLICY, AssertionPolicyRow, AssertionSupportRow, AssertionsRow, BriefAssertionsRow,
    BriefDocumentsRow, BriefMembersRow, BriefsRow, EvidenceRow, FindingsRow, section_of,
};
use cpg_schema::id::Id;
use datafusion::prelude::SessionContext;
use unicode_segmentation::UnicodeSegmentation;

use crate::analyze::{AnalysisRows, CompilerRun};
use crate::delta::to_schema;
use crate::{CoreError, sql};

/// Bumped whenever a template's wording or an extractive rule changes; part of the compiler
/// digest through the synthesis tables' contracts and this constant.
pub const TEMPLATE_VERSION: i64 = 1;

/// The §11.1 cap on a brief document: 2,048 tokens. The embedder counts tokens with the served
/// model's tokenizer (slice 1.6); here a declared proxy of four bytes per token. In increment 1 an
/// over-cap brief fails the compile (applicable cases, which split it, arrive in 2.5).
pub const DOCUMENT_BYTE_CAP: usize = 4 * 2048;

/// The rows Stage F produces.
#[derive(Debug, Default)]
pub struct SynthRows {
    pub evidence: Vec<EvidenceRow>,
    pub assertions: Vec<AssertionsRow>,
    pub supports: Vec<AssertionSupportRow>,
    pub briefs: Vec<BriefsRow>,
    pub brief_assertions: Vec<BriefAssertionsRow>,
    pub brief_members: Vec<BriefMembersRow>,
    pub brief_documents: Vec<BriefDocumentsRow>,
    pub policy: Vec<AssertionPolicyRow>,
}

fn schema(fields: &[(&str, DataType)]) -> SchemaRef {
    std::sync::Arc::new(Schema::new(
        fields
            .iter()
            .map(|(n, t)| Field::new(*n, t.clone(), true))
            .collect::<Vec<_>>(),
    ))
}

const ID: DataType = DataType::FixedSizeBinary(16);

/// One query's rows, cast to its declared columns.
struct Table {
    batches: Vec<arrow_array::RecordBatch>,
}

impl Table {
    async fn read(
        ctx: &SessionContext,
        query: &str,
        fields: &[(&str, DataType)],
    ) -> Result<Self, CoreError> {
        let s = schema(fields);
        let batches = sql::query(ctx, query)
            .await?
            .collect()
            .await?
            .iter()
            .map(|b| to_schema(b, &s))
            .collect::<Result<_, _>>()?;
        Ok(Self { batches })
    }

    fn rows(&self) -> impl Iterator<Item = (&arrow_array::RecordBatch, usize)> {
        self.batches
            .iter()
            .flat_map(|b| (0..b.num_rows()).map(move |i| (b, i)))
    }
}

fn col<'a, T: 'static>(b: &'a arrow_array::RecordBatch, name: &str) -> &'a T {
    b.column_by_name(name)
        .and_then(|c| c.as_any().downcast_ref::<T>())
        .expect("a declared column")
}

fn id(b: &arrow_array::RecordBatch, name: &str, i: usize) -> Option<Id> {
    let a = col::<FixedSizeBinaryArray>(b, name);
    (!a.is_null(i)).then(|| Id(<[u8; 16]>::try_from(a.value(i)).expect("16 bytes")))
}

fn text(b: &arrow_array::RecordBatch, name: &str, i: usize) -> Option<String> {
    let a = col::<StringArray>(b, name);
    (!a.is_null(i)).then(|| a.value(i).to_owned())
}

fn int(b: &arrow_array::RecordBatch, name: &str, i: usize) -> Option<i64> {
    let a = col::<Int64Array>(b, name);
    (!a.is_null(i)).then(|| a.value(i))
}

fn small(b: &arrow_array::RecordBatch, name: &str, i: usize) -> Option<i16> {
    let a = col::<Int16Array>(b, name);
    (!a.is_null(i)).then(|| a.value(i))
}

fn flag(b: &arrow_array::RecordBatch, name: &str, i: usize) -> Option<bool> {
    let a = col::<BooleanArray>(b, name);
    (!a.is_null(i)).then(|| a.value(i))
}

fn hex_list(ids: impl IntoIterator<Item = Id>) -> String {
    let list: Vec<String> = ids.into_iter().map(|i| format!("X'{}'", i.hex())).collect();
    if list.is_empty() {
        "NULL".to_owned()
    } else {
        list.join(", ")
    }
}

/// The byte span of a docstring's summary line (the first non-blank line of its literal), found
/// in the literal's own source bytes so the evidence is verbatim. `None` for an empty docstring or
/// a form this reader does not follow.
pub fn summary_span(source: &str, start: usize, end: usize) -> Option<(usize, usize)> {
    let lit = source.get(start..end)?;
    let bytes = lit.as_bytes();
    let mut i = bytes.iter().take_while(|b| b"rRuUbBfF".contains(b)).count();
    let quote = match &lit[i..] {
        s if s.starts_with("\"\"\"") => "\"\"\"",
        s if s.starts_with("'''") => "'''",
        s if s.starts_with('"') => "\"",
        s if s.starts_with('\'') => "'",
        _ => return None,
    };
    i += quote.len();
    i += lit[i..].len() - lit[i..].trim_start().len();
    let rest = &lit[i..];
    let stop = rest
        .find('\n')
        .unwrap_or(rest.len())
        .min(rest.find(quote).unwrap_or(rest.len()));
    let line = rest[..stop].trim_end();
    (!line.is_empty()).then(|| (start + i, start + i + line.len()))
}

/// The byte span of a passage's lead sentence (DESIGN §10.3): the first sentence (UAX #29) of its
/// first prose paragraph, skipping headings, code fences and their contents, MDX tags, imports,
/// tables and admonition markers.
pub fn lead_sentence(passage: &str) -> Option<(usize, usize)> {
    let mut offset = 0;
    let mut fenced = false;
    let mut paragraph: Option<(usize, usize)> = None;
    for line in passage.split_inclusive('\n') {
        let at = offset;
        offset += line.len();
        let body = line.trim();
        if body.starts_with("```") || body.starts_with("~~~") {
            fenced = !fenced;
            if paragraph.is_some() {
                break;
            }
            continue;
        }
        let skip = fenced
            || body.is_empty()
            || body.starts_with('#')
            || body.starts_with('<')
            || body.starts_with('|')
            || body.starts_with(":::")
            || body.starts_with("import ")
            || body.starts_with("export ")
            || body.starts_with("---");
        match (&mut paragraph, skip) {
            (None, true) => {}
            (Some(_), true) => break,
            (None, false) => {
                let lead = line.len() - line.trim_start().len();
                paragraph = Some((at + lead, at + line.trim_end().len()));
            }
            (Some(p), false) => p.1 = at + line.trim_end().len(),
        }
    }
    let (start, end) = paragraph?;
    let (rel, sentence) = passage[start..end].split_sentence_bound_indices().next()?;
    let sentence = sentence.trim_end();
    (!sentence.is_empty()).then(|| (start + rel, start + rel + sentence.len()))
}

/// §10.2's derivation: an unresolved supporting finding makes the assertion unresolved, a
/// statistical one statistical; otherwise the strongest status its evidence supports.
fn derive_status(
    kind: AssertionKind,
    findings: &[EvidenceStatus],
    has_evidence: bool,
) -> EvidenceStatus {
    if findings.contains(&EvidenceStatus::Unresolved) {
        EvidenceStatus::Unresolved
    } else if findings.contains(&EvidenceStatus::StatisticallyDerived) {
        EvidenceStatus::StatisticallyDerived
    } else if kind == AssertionKind::Outcome {
        if has_evidence {
            EvidenceStatus::Documented
        } else {
            EvidenceStatus::Unresolved
        }
    } else {
        EvidenceStatus::StructurallyObserved
    }
}

/// One assertion before its id: its content and supports.
struct Draft {
    kind: AssertionKind,
    text: Option<String>,
    /// Finding supports (role, id, the finding's status).
    findings: Vec<(SupportRole, Id, EvidenceStatus)>,
    evidence: Vec<Id>,
}

/// The kind policy as published rows (DESIGN §10.2).
pub fn policy_rows(snapshot_id: Id) -> Vec<AssertionPolicyRow> {
    ASSERTION_POLICY
        .iter()
        .flat_map(|(kind, section, statuses)| {
            statuses.iter().map(move |status| AssertionPolicyRow {
                snapshot_id,
                assertion_kind: *kind,
                brief_section: *section,
                evidence_status: *status,
            })
        })
        .collect()
}

/// Stage F for one attempt.
pub async fn run(
    ctx: &SessionContext,
    snapshot_id: Id,
    compiler: CompilerRun,
    found: &AnalysisRows,
) -> Result<SynthRows, CoreError> {
    let mut out = SynthRows {
        policy: policy_rows(snapshot_id),
        ..SynthRows::default()
    };
    if found.seeds.is_empty() {
        return Ok(out);
    }
    let seeds: Vec<Id> = found.seeds.iter().map(|(_, n)| *n).collect();

    // Labels for every node a finding names.
    let named: BTreeSet<Id> = found
        .findings
        .iter()
        .flat_map(|f| [Some(f.subject_node_id), f.related_node_id])
        .flatten()
        .chain(found.witnesses.iter().map(|w| w.callee_node_id))
        .collect();
    let labels_sql = format!(
        "SELECT n.node_id, COALESCE(d.qualified_name, \
                cd.module_name || '.' || cd.qualified_name, \
                COALESCE(dc.qualified_name, pf.module_name) || '.' || pf.name) AS label \
         FROM nodes n \
         LEFT JOIN declarations d ON d.node_id = n.node_id \
         LEFT JOIN context_definitions cd ON cd.symbol_node_id = n.node_id \
         LEFT JOIN synthetic_callables sc ON sc.node_id = n.node_id \
         LEFT JOIN pysa_functions pf ON pf.module_node_id = sc.module_node_id \
           AND pf.function_key = sc.function_key \
         LEFT JOIN provider_class_map pc ON pc.module_node_id = sc.module_node_id \
           AND pc.class_key = pf.defining_class_key \
         LEFT JOIN declarations dc ON dc.node_id = pc.node_id \
         WHERE n.node_id IN ({}) ORDER BY n.node_id",
        hex_list(named.iter().copied())
    );
    let mut labels: BTreeMap<Id, String> = BTreeMap::new();
    for (b, i) in Table::read(
        ctx,
        &labels_sql,
        &[("node_id", ID), ("label", DataType::Utf8)],
    )
    .await?
    .rows()
    {
        if let (Some(n), Some(l)) = (id(b, "node_id", i), text(b, "label", i)) {
            labels.entry(n).or_insert(l);
        }
    }
    let label = |n: Id| labels.get(&n).cloned().unwrap_or_else(|| n.hex());

    // The seeds' declarations, docstrings and module texts.
    let decl_sql = format!(
        "SELECT d.node_id, d.module_node_id, d.docstring_start_byte, d.docstring_end_byte, \
                s.text, s.byte_len \
         FROM declarations d JOIN source_files s ON s.module_node_id = d.module_node_id \
         WHERE d.node_id IN ({}) ORDER BY d.node_id",
        hex_list(seeds.iter().copied())
    );
    struct Decl {
        module: Id,
        docstring: Option<(usize, usize)>,
        text: Option<String>,
    }
    let mut decls: BTreeMap<Id, Decl> = BTreeMap::new();
    for (b, i) in Table::read(
        ctx,
        &decl_sql,
        &[
            ("node_id", ID),
            ("module_node_id", ID),
            ("docstring_start_byte", DataType::Int64),
            ("docstring_end_byte", DataType::Int64),
            ("text", DataType::Utf8),
            ("byte_len", DataType::Int64),
        ],
    )
    .await?
    .rows()
    {
        let (Some(node), Some(module)) = (id(b, "node_id", i), id(b, "module_node_id", i)) else {
            continue;
        };
        let docstring = int(b, "docstring_start_byte", i)
            .zip(int(b, "docstring_end_byte", i))
            .map(|(s, e)| (s as usize, e as usize));
        decls.insert(
            node,
            Decl {
                module,
                docstring,
                text: text(b, "text", i),
            },
        );
    }

    // Passages that mention a seed exactly (by its declaration or an export naming it).
    let exports_of: BTreeMap<Id, Vec<Id>> = found
        .findings
        .iter()
        .filter(|f| f.finding_kind == FindingKind::PublicAlias)
        .map(|f| {
            let nodes = found
                .members
                .iter()
                .filter(|m| m.finding_id == f.finding_id)
                .filter_map(|m| m.node_id)
                .collect();
            (f.subject_node_id, nodes)
        })
        .collect();
    let mention_targets: BTreeSet<Id> = seeds
        .iter()
        .copied()
        .chain(exports_of.values().flatten().copied())
        .collect();
    let mention_sql = format!(
        "SELECT t.target_node_id, p.node_id AS passage_node_id, p.document_node_id, \
                p.start_byte, p.text, d.path, p.ordinal \
         FROM mention_targets t JOIN mentions m ON m.fact_id = t.mention_fact_id \
         JOIN passages p ON p.node_id = m.passage_node_id \
         JOIN documents d ON d.node_id = p.document_node_id \
         WHERE m.class = {exact} AND t.target_node_id IN ({targets}) \
         ORDER BY d.path, p.ordinal, t.target_node_id",
        exact = MentionClass::Exact.code(),
        targets = hex_list(mention_targets.iter().copied())
    );
    struct Passage {
        node: Id,
        document: Id,
        start: i64,
        text: String,
    }
    let mut passages: BTreeMap<Id, Vec<Passage>> = BTreeMap::new();
    for (b, i) in Table::read(
        ctx,
        &mention_sql,
        &[
            ("target_node_id", ID),
            ("passage_node_id", ID),
            ("document_node_id", ID),
            ("start_byte", DataType::Int64),
            ("text", DataType::Utf8),
            ("path", DataType::Utf8),
            ("ordinal", DataType::Int64),
        ],
    )
    .await?
    .rows()
    {
        let (Some(target), Some(node), Some(document), Some(start), Some(body)) = (
            id(b, "target_node_id", i),
            id(b, "passage_node_id", i),
            id(b, "document_node_id", i),
            int(b, "start_byte", i),
            text(b, "text", i),
        ) else {
            continue;
        };
        // An export's mention counts for the seed it names.
        let seed = seeds
            .iter()
            .copied()
            .find(|s| *s == target || exports_of.get(s).is_some_and(|e| e.contains(&target)));
        if let Some(seed) = seed {
            passages.entry(seed).or_default().push(Passage {
                node,
                document,
                start,
                text: body,
            });
        }
    }

    // The seeds' own signatures' parameters.
    let params_sql = format!(
        "SELECT p.signature_node_id, ps.node_id, ps.fact_id, ps.ordinal, ps.name, ps.kind, \
                ps.default_text, ps.annotation_text, ps.start_byte, ps.end_byte, \
                sem.required, d.module_node_id \
         FROM parameters p JOIN parameter_syntax ps ON ps.fact_id = p.syntax_fact_id \
         LEFT JOIN parameter_semantics sem ON sem.fact_id = p.semantics_fact_id \
         JOIN declarations d ON d.node_id = p.signature_node_id \
         WHERE p.signature_node_id IN ({}) ORDER BY p.signature_node_id, ps.ordinal",
        hex_list(seeds.iter().copied())
    );
    struct Param {
        node: Id,
        fact: Id,
        ordinal: i64,
        name: String,
        kind: Option<ParameterKind>,
        default: Option<String>,
        annotation: Option<String>,
        span: (usize, usize),
        required: Option<bool>,
        module: Id,
    }
    let mut params: BTreeMap<Id, Vec<Param>> = BTreeMap::new();
    for (b, i) in Table::read(
        ctx,
        &params_sql,
        &[
            ("signature_node_id", ID),
            ("node_id", ID),
            ("fact_id", ID),
            ("ordinal", DataType::Int64),
            ("name", DataType::Utf8),
            ("kind", DataType::Int16),
            ("default_text", DataType::Utf8),
            ("annotation_text", DataType::Utf8),
            ("start_byte", DataType::Int64),
            ("end_byte", DataType::Int64),
            ("required", DataType::Boolean),
            ("module_node_id", ID),
        ],
    )
    .await?
    .rows()
    {
        let (Some(sig), Some(node), Some(fact), Some(module)) = (
            id(b, "signature_node_id", i),
            id(b, "node_id", i),
            id(b, "fact_id", i),
            id(b, "module_node_id", i),
        ) else {
            continue;
        };
        params.entry(sig).or_default().push(Param {
            node,
            fact,
            ordinal: int(b, "ordinal", i).unwrap_or_default(),
            name: text(b, "name", i).unwrap_or_default(),
            kind: small(b, "kind", i).and_then(ParameterKind::from_code),
            default: text(b, "default_text", i),
            annotation: text(b, "annotation_text", i),
            span: (
                int(b, "start_byte", i).unwrap_or_default() as usize,
                int(b, "end_byte", i).unwrap_or_default() as usize,
            ),
            required: flag(b, "required", i),
            module,
        });
    }

    let invocation_of: BTreeMap<Id, &cpg_schema::findings::AnalysisInvocationsRow> = found
        .invocations
        .iter()
        .filter_map(|v| v.subject_node_id.map(|s| (s, v)))
        .collect();
    let mut evidence: BTreeMap<Id, EvidenceRow> = BTreeMap::new();
    let mut add_evidence = |row: EvidenceRow| -> Id {
        let id = row.evidence_id;
        evidence.entry(id).or_insert(row);
        id
    };

    for (access_path, seed) in &found.seeds {
        let seed = *seed;
        let seed_label = access_path.clone();
        let findings: Vec<&FindingsRow> = found
            .findings
            .iter()
            .filter(|f| f.subject_node_id == seed)
            .collect();
        let mut drafts: Vec<Draft> = Vec::new();

        // Outcome.
        let mut outcome = Draft {
            kind: AssertionKind::Outcome,
            text: None,
            findings: Vec::new(),
            evidence: Vec::new(),
        };
        if let Some(decl) = decls.get(&seed)
            && let (Some((s, e)), Some(source)) = (decl.docstring, decl.text.as_deref())
            && let Some((a, z)) = summary_span(source, s, e)
        {
            let line = source[a..z].to_owned();
            let ev = add_evidence(EvidenceRow {
                snapshot_id,
                evidence_id: recipe::evidence(
                    EvidenceKind::Span.code(),
                    Some(seed),
                    Some(decl.module),
                    Some((a as i64, z as i64)),
                    Some(&line),
                ),
                evidence_kind: EvidenceKind::Span,
                cited_fact_id: None,
                node_id: Some(seed),
                module_node_id: Some(decl.module),
                start_byte: Some(a as i64),
                end_byte: Some(z as i64),
                text: Some(line.clone()),
            });
            outcome.text = Some(line);
            outcome.evidence.push(ev);
        } else if let Some(p) = passages.get(&seed).and_then(|ps| {
            ps.iter()
                .find_map(|p| lead_sentence(&p.text).map(|s| (p, s)))
        }) {
            let (p, (a, z)) = p;
            let sentence = p.text[a..z].to_owned();
            let (start, end) = (p.start + a as i64, p.start + z as i64);
            let ev = add_evidence(EvidenceRow {
                snapshot_id,
                evidence_id: recipe::evidence(
                    EvidenceKind::Passage.code(),
                    Some(p.node),
                    Some(p.document),
                    Some((start, end)),
                    Some(&sentence),
                ),
                evidence_kind: EvidenceKind::Passage,
                cited_fact_id: None,
                node_id: Some(p.node),
                module_node_id: Some(p.document),
                start_byte: Some(start),
                end_byte: Some(end),
                text: Some(sentence.clone()),
            });
            outcome.text = Some(sentence);
            outcome.evidence.push(ev);
        }
        drafts.push(outcome);

        // Public access.
        let mut aliases: Vec<(Id, String)> = Vec::new();
        for f in findings
            .iter()
            .filter(|f| f.finding_kind == FindingKind::PublicAlias)
        {
            for m in found
                .members
                .iter()
                .filter(|m| m.finding_id == f.finding_id)
            {
                if let (Some(n), Some(l)) = (m.node_id, m.label.clone()) {
                    aliases.push((n, l));
                }
            }
            let others: Vec<String> = aliases
                .iter()
                .map(|(_, l)| l.clone())
                .filter(|l| l != access_path)
                .map(|l| format!("`{l}`"))
                .collect();
            let also = if others.is_empty() {
                String::new()
            } else {
                format!(
                    "; the same object is also reachable as {}",
                    others.join(", ")
                )
            };
            drafts.push(Draft {
                kind: AssertionKind::PublicAccess,
                text: Some(format!("Call it as `{access_path}`{also}.")),
                findings: vec![(SupportRole::Support, f.finding_id, f.evidence_status)],
                evidence: Vec::new(),
            });
        }

        // Coordinates: what the operation already delegates to.
        let mut delegations: Vec<&&FindingsRow> = findings
            .iter()
            .filter(|f| {
                matches!(
                    f.finding_kind,
                    FindingKind::DirectDelegation | FindingKind::BoundedDelegationPath
                )
            })
            .collect();
        delegations.sort_by_key(|f| (f.related_node_id.map(label), f.finding_id));
        for f in delegations {
            let Some(target) = f.related_node_id else {
                continue;
            };
            let steps: Vec<_> = found
                .witnesses
                .iter()
                .filter(|w| w.finding_id == f.finding_id)
                .collect();
            let paths = steps.iter().map(|w| w.path).collect::<BTreeSet<_>>().len();
            let candidate = steps.iter().any(|w| w.modality == Modality::Candidate);
            let via = steps
                .iter()
                .find(|w| w.path == 0 && w.step == 0 && w.callee_node_id != target)
                .map(|w| label(w.callee_node_id));
            let text = match (f.finding_kind, via) {
                (FindingKind::DirectDelegation, _) => format!(
                    "`{seed_label}` already calls `{}` ({paths} call site{}).",
                    label(target),
                    if paths == 1 { "" } else { "s" }
                ),
                (_, Some(via)) => format!(
                    "`{seed_label}` already reaches `{}` through `{via}`{}.",
                    label(target),
                    if candidate {
                        ", an overridable call"
                    } else {
                        ""
                    }
                ),
                (_, None) => format!(
                    "`{seed_label}` already calls `{}`{}.",
                    label(target),
                    if candidate {
                        " through an overridable method, so a subclass may replace it"
                    } else {
                        ""
                    }
                ),
            };
            drafts.push(Draft {
                kind: AssertionKind::Coordinates,
                text: Some(text),
                findings: vec![(SupportRole::Support, f.finding_id, f.evidence_status)],
                evidence: Vec::new(),
            });
        }

        // Parameters of the seed's own signature (the receiver aside).
        for p in params.get(&seed).map(Vec::as_slice).unwrap_or_default() {
            if p.ordinal == 0 && (p.name == "self" || p.name == "cls") {
                continue;
            }
            let Some(decl) = decls.get(&seed) else {
                continue;
            };
            let Some(span_text) = decl.text.as_deref().and_then(|t| t.get(p.span.0..p.span.1))
            else {
                continue;
            };
            let ev = add_evidence(EvidenceRow {
                snapshot_id,
                evidence_id: recipe::evidence(
                    EvidenceKind::Fact.code(),
                    Some(p.node),
                    Some(p.module),
                    Some((p.span.0 as i64, p.span.1 as i64)),
                    Some(span_text),
                ),
                evidence_kind: EvidenceKind::Fact,
                cited_fact_id: Some(p.fact),
                node_id: Some(p.node),
                module_node_id: Some(p.module),
                start_byte: Some(p.span.0 as i64),
                end_byte: Some(p.span.1 as i64),
                text: Some(span_text.to_owned()),
            });
            let mut parts = vec![p.kind.map_or("parameter", Codebook::text).replace('_', " ")];
            if let Some(d) = &p.default {
                parts.push(format!("default `{d}`"));
            }
            match p.required {
                Some(true) => parts.push("required".to_owned()),
                Some(false) => parts.push("optional".to_owned()),
                None => {}
            }
            if let Some(a) = &p.annotation {
                parts.push(format!("annotated `{a}`"));
            }
            drafts.push(Draft {
                kind: AssertionKind::Parameter,
                text: Some(format!("`{}`: {}.", p.name, parts.join("; "))),
                findings: Vec::new(),
                evidence: vec![ev],
            });
        }

        // Limits: where the analysis stopped.
        let mut by_reason: BTreeMap<StopReason, Vec<&&FindingsRow>> = BTreeMap::new();
        for f in &findings {
            if matches!(
                f.finding_kind,
                FindingKind::ImplementationBoundary | FindingKind::IncompleteResolution
            ) && let Some(r) = f.stop_reason
            {
                by_reason.entry(r).or_default().push(f);
            }
        }
        for (reason, group) in &by_reason {
            let names: BTreeSet<String> = group
                .iter()
                .filter_map(|f| f.related_node_id)
                .map(label)
                .collect();
            let listed = names
                .iter()
                .map(|n| format!("`{n}`"))
                .collect::<Vec<_>>()
                .join(", ");
            let text = match reason {
                StopReason::ExternalBoundary => format!(
                    "`{seed_label}` calls into code outside the analyzed release, which is not \
                     analyzed further: {listed}."
                ),
                StopReason::SyntheticBoundary => format!(
                    "`{seed_label}` calls callables with no body in source (synthesized), which \
                     are not analyzed further: {listed}."
                ),
                StopReason::SubsystemBoundary => format!(
                    "`{seed_label}` calls release code outside the analyzed subsystem, which is \
                     not analyzed further: {listed}."
                ),
                StopReason::UnresolvedSite => format!(
                    "{} call site{} reached from `{seed_label}` {} no resolved target.",
                    group.len(),
                    if group.len() == 1 { "" } else { "s" },
                    if group.len() == 1 { "has" } else { "have" }
                ),
                other => format!("The analysis of `{seed_label}` stopped: {}.", other.text()),
            };
            drafts.push(Draft {
                kind: AssertionKind::AnalysisBoundary,
                text: Some(text),
                findings: group
                    .iter()
                    .map(|f| (SupportRole::Support, f.finding_id, f.evidence_status))
                    .collect(),
                evidence: Vec::new(),
            });
        }
        if let Some(inv) = invocation_of.get(&seed) {
            let stop = match inv.stop_reason {
                Some(StopReason::DepthLimit) => Some(format!(
                    "Calls more than {} steps below `{seed_label}` were not followed (the \
                     analysis's depth bound).",
                    serde_json::from_str::<serde_json::Value>(&inv.parameters)
                        .ok()
                        .and_then(|v| v.get("max_depth").and_then(serde_json::Value::as_u64))
                        .unwrap_or_default()
                )),
                Some(StopReason::VertexBudget | StopReason::EdgeBudget) => Some(format!(
                    "The delegation analysis of `{seed_label}` stopped at its budget; more \
                     delegations may exist than are listed."
                )),
                _ => None,
            };
            if let Some(text) = stop {
                drafts.push(Draft {
                    kind: AssertionKind::AnalysisBoundary,
                    text: Some(text),
                    findings: Vec::new(),
                    evidence: Vec::new(),
                });
            }
        }

        // Assertions, ordered by section then draft order.
        let mut ordered: Vec<(usize, Draft)> = drafts.into_iter().enumerate().collect();
        ordered.sort_by_key(|(i, d)| (section_of(d.kind).code(), *i));
        let mut assertion_ids = Vec::new();
        let mut uses_analysis = false;
        for (_, d) in ordered {
            let statuses: Vec<EvidenceStatus> = d.findings.iter().map(|f| f.2).collect();
            let status = derive_status(d.kind, &statuses, !d.evidence.is_empty());
            let text = if status == EvidenceStatus::Unresolved {
                None
            } else {
                d.text
            };
            let supports: Vec<(i16, Option<Id>, Option<Id>)> = d
                .findings
                .iter()
                .map(|(r, f, _)| (r.code(), Some(*f), None))
                .chain(
                    d.evidence
                        .iter()
                        .map(|e| (SupportRole::Support.code(), None, Some(*e))),
                )
                .collect();
            uses_analysis |= !d.findings.is_empty();
            let assertion_id = AssertionKey {
                kind: d.kind.code(),
                subject: seed,
                applicable_case: None,
                status: status.code(),
                text: text.as_deref(),
                conditions: None,
                limitations: None,
                supports: &supports,
            }
            .id();
            for (ordinal, (role, f, e)) in supports.iter().enumerate() {
                out.supports.push(AssertionSupportRow {
                    snapshot_id,
                    assertion_id,
                    role: SupportRole::from_code(*role).expect("a support role"),
                    ordinal: ordinal as i64,
                    finding_id: *f,
                    evidence_id: *e,
                });
            }
            if !out
                .assertions
                .iter()
                .any(|a| a.assertion_id == assertion_id)
            {
                out.assertions.push(AssertionsRow {
                    snapshot_id,
                    assertion_id,
                    run_id: compiler.run_id,
                    model_id: compiler.model("synthesis"),
                    extraction_mode: ExtractionMode::TemplateSynthesis,
                    assertion_kind: d.kind,
                    subject_node_id: seed,
                    applicable_case: None,
                    evidence_status: status,
                    text,
                    conditions: None,
                    limitations: None,
                    template_version: TEMPLATE_VERSION,
                });
            }
            assertion_ids.push(assertion_id);
        }
        // A support row is keyed by its assertion; an identical assertion in two briefs shares
        // one set of rows.
        out.supports
            .sort_by_key(|s| (s.assertion_id, s.role, s.ordinal));
        out.supports
            .dedup_by_key(|s| (s.assertion_id, s.role.code(), s.ordinal));

        let brief_id = recipe::brief(seed, None, &assertion_ids);
        for (ordinal, a) in assertion_ids.iter().enumerate() {
            out.brief_assertions.push(BriefAssertionsRow {
                snapshot_id,
                brief_id,
                ordinal: ordinal as i64,
                assertion_id: *a,
            });
        }
        for (export, path) in &aliases {
            out.brief_members.push(BriefMembersRow {
                snapshot_id,
                brief_id,
                access_path: path.clone(),
                export_node_id: *export,
                declaration_node_id: seed,
            });
        }
        out.briefs.push(BriefsRow {
            snapshot_id,
            brief_id,
            run_id: compiler.run_id,
            model_id: compiler.model("synthesis"),
            seed_node_id: seed,
            access_path: access_path.clone(),
            title: access_path.clone(),
            applicable_case: None,
            documentation_only: !uses_analysis,
            review_state: ReviewState::Unreviewed,
        });

        // The embedding projection (§11.1): outcome, public APIs, controls, limits.
        let assertion = |a: &Id| out.assertions.iter().find(|x| x.assertion_id == *a);
        let texts = |kind: AssertionKind| -> Vec<String> {
            assertion_ids
                .iter()
                .filter_map(assertion)
                .filter(|a| a.assertion_kind == kind)
                .filter_map(|a| a.text.clone())
                .collect()
        };
        let outcome = texts(AssertionKind::Outcome)
            .into_iter()
            .next()
            .unwrap_or_else(|| "(unresolved)".to_owned());
        let apis = aliases
            .iter()
            .map(|(_, l)| l.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let controls = params
            .get(&seed)
            .map(|ps| {
                ps.iter()
                    .filter(|p| !(p.ordinal == 0 && (p.name == "self" || p.name == "cls")))
                    .map(|p| p.name.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default();
        let limits = texts(AssertionKind::AnalysisBoundary).join(" ");
        let document = format!(
            "Outcome: {outcome}\nPublic APIs: {apis}\nBuilt-in controls: {controls}\n\
             Conditions and limitations: {limits}"
        );
        if document.len() > DOCUMENT_BYTE_CAP {
            return Err(CoreError::Analysis(format!(
                "the brief for {access_path} is {} bytes, over the {DOCUMENT_BYTE_CAP}-byte cap \
                 (2,048 tokens); splitting by applicable case arrives in increment 2",
                document.len()
            )));
        }
        out.brief_documents.push(BriefDocumentsRow {
            snapshot_id,
            brief_id,
            chunk: 0,
            text: document,
            input_hash: None,
        });
    }
    out.evidence = evidence.into_values().collect();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_summary_span_is_the_first_line_of_the_literal() {
        let src = "def f():\n    \"\"\"\n    Run it — fast.\n\n    More.\n    \"\"\"\n";
        let start = src.find("\"\"\"").unwrap();
        let end = src.rfind("\"\"\"").unwrap() + 3;
        let (a, z) = summary_span(src, start, end).unwrap();
        assert_eq!(&src[a..z], "Run it — fast.");
        let one = "r'''Short.'''";
        let (a, z) = summary_span(one, 0, one.len()).unwrap();
        assert_eq!(&one[a..z], "Short.");
        assert_eq!(summary_span("\"\"\"   \"\"\"", 0, 9), None);
    }

    #[test]
    fn a_lead_sentence_skips_headings_code_and_markup() {
        let p = "## Tools\n\n<Tip>x</Tip>\n```python\nmcp.tool()\n```\nTools let an LLM act. They \
                 are functions.\nMore text.\n";
        let (a, z) = lead_sentence(p).unwrap();
        assert_eq!(&p[a..z], "Tools let an LLM act.");
        assert_eq!(lead_sentence("## Only a heading\n"), None);
    }

    #[test]
    fn status_follows_supports_never_a_choice() {
        use EvidenceStatus::*;
        assert_eq!(
            derive_status(AssertionKind::Coordinates, &[StructurallyObserved], false),
            StructurallyObserved
        );
        assert_eq!(
            derive_status(
                AssertionKind::Coordinates,
                &[StructurallyObserved, StatisticallyDerived],
                false
            ),
            StatisticallyDerived
        );
        assert_eq!(
            derive_status(
                AssertionKind::Coordinates,
                &[StatisticallyDerived, Unresolved],
                false
            ),
            Unresolved
        );
        assert_eq!(derive_status(AssertionKind::Outcome, &[], true), Documented);
        assert_eq!(
            derive_status(AssertionKind::Outcome, &[], false),
            Unresolved
        );
    }
}
