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
    ArcKind, AssertionKind, Codebook, DeclarationKind, EvidenceKind, EvidenceStatus,
    ExtractionMode, FindingKind, InvocationPhase, MentionClass, Modality, ParameterKind,
    ReviewState, StopReason, SupportRole,
};
use cpg_schema::findings::recipe::{self, AssertionKey};
use cpg_schema::findings::{
    ANALYSIS_BACKED, ASSERTION_POLICY, AssertionPolicyRow, AssertionSupportRow, AssertionsRow,
    BriefAssertionsRow, BriefDocumentsRow, BriefMembersRow, BriefsRow, EvidenceRow, FindingsRow,
    WitnessesRow, derive_status, evidence_status, section_of,
};
use cpg_schema::id::Id;
use datafusion::prelude::SessionContext;
use unicode_segmentation::UnicodeSegmentation;

use crate::analyze::{AnalysisRows, CompilerRun};
use crate::delta::to_schema;
use crate::{CoreError, sql};

/// Bumped whenever a template's wording or an extractive rule changes; part of the compiler
/// digest through the synthesis tables' contracts and this constant. 2: definition steps and
/// "or more" call sites (slice 1.4 review F1, F3). 3: the slice 1.5 review: sentences on a
/// soft-break view, the mention-sentence leg, the call form, hops said as witnessed, limits by
/// depth, requiredness cited, traversal stops cited.
pub const TEMPLATE_VERSION: i64 = 3;

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

/// `text` with each run of whitespace (a soft line break and its indentation included) as one
/// space: an extracted sentence as it reads rendered. Its evidence keeps the source bytes.
fn normalized(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The first sentence (UAX #29) of `text[start..end]`, as bytes of `text`. It is found on a view
/// in which each line break is a space of the same byte length, so a hard-wrapped sentence is one
/// sentence (slice 1.5 review O1, F2).
fn first_sentence(text: &str, start: usize, end: usize) -> Option<(usize, usize)> {
    let view: String = text
        .get(start..end)?
        .chars()
        .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
        .collect();
    let (rel, sentence) = view
        .split_sentence_bound_indices()
        .find(|(_, s)| !s.trim().is_empty())?;
    let lead = sentence.len() - sentence.trim_start().len();
    let body = sentence.trim();
    Some((start + rel + lead, start + rel + lead + body.len()))
}

/// A docstring line that ends its summary paragraph without a blank line: a section header
/// (`Args:`, `Example usage:`), a reST field, a doctest or an underline.
fn ends_summary(line: &str) -> bool {
    (line.ends_with(':') && line.split_whitespace().count() <= 3 && !line.contains('`'))
        || line.starts_with(':')
        || line.starts_with(">>>")
        || (!line.is_empty() && line.chars().all(|c| c == '-' || c == '='))
}

/// The byte span of a docstring's summary: the first sentence of its first paragraph, found in the
/// literal's own source bytes so the evidence is verbatim (DESIGN §10.3). The paragraph ends at a
/// blank line, a section header or the closing quote. `None` for an empty docstring or a form this
/// reader does not follow.
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
    let body = &lit[i..i + lit[i..].find(quote).unwrap_or(lit.len() - i)];
    let mut paragraph = 0;
    let mut at = 0;
    for line in body.split_inclusive('\n') {
        let t = line.trim();
        if t.is_empty() || ends_summary(t) {
            break;
        }
        paragraph = at + line.trim_end().len();
        at += line.len();
    }
    (paragraph > 0)
        .then(|| first_sentence(source, start + i, start + i + paragraph))
        .flatten()
}

/// A list item's marker (`- `, `* `, `+ `, `1. `), whose length is returned.
fn list_marker(line: &str) -> Option<usize> {
    let t = line.trim_start();
    let indent = line.len() - t.len();
    if t.starts_with("- ") || t.starts_with("* ") || t.starts_with("+ ") {
        return Some(indent + 2);
    }
    let digits = t.bytes().take_while(u8::is_ascii_digit).count();
    (digits > 0 && t[digits..].starts_with(". ")).then_some(indent + digits + 2)
}

/// A passage's prose paragraphs as byte ranges: runs of prose lines, skipping headings, code
/// fences and their contents, MDX tags, imports, tables, quotes and admonition markers. A list item
/// starts a paragraph of its own, its marker excluded.
pub fn paragraphs(passage: &str) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = Vec::new();
    let mut open = false;
    let mut offset = 0;
    let mut fenced = false;
    for line in passage.split_inclusive('\n') {
        let at = offset;
        offset += line.len();
        let body = line.trim();
        if body.starts_with("```") || body.starts_with("~~~") {
            fenced = !fenced;
            open = false;
            continue;
        }
        let skip = fenced
            || body.is_empty()
            || body.starts_with('#')
            || body.starts_with('<')
            || body.starts_with('>')
            || body.starts_with('|')
            || body.starts_with(":::")
            || body.starts_with("import ")
            || body.starts_with("export ")
            || body.starts_with("---");
        let end = at + line.trim_end().len();
        if skip {
            open = false;
        } else if let Some(marker) = list_marker(line) {
            out.push((at + marker, end));
            open = true;
        } else if open {
            out.last_mut().expect("an open paragraph").1 = end;
        } else {
            out.push((at + line.len() - line.trim_start().len(), end));
            open = true;
        }
    }
    out
}

/// The Outcome a passage offers for an exact mention at `mention` (passage-relative bytes): the
/// lead sentence of the mention's paragraph, only when the mention lies inside it (DESIGN §10.3;
/// slice 1.5 review F2). A sentence about something else, which merely precedes the mention, is
/// never the seed's Outcome.
pub fn mention_sentence(passage: &str, mention: (usize, usize)) -> Option<(usize, usize)> {
    let (start, end) = paragraphs(passage)
        .into_iter()
        .find(|(s, e)| *s <= mention.0 && mention.1 <= *e)?;
    let (a, z) = first_sentence(passage, start, end)?;
    (a <= mention.0 && mention.1 <= z).then_some((a, z))
}

/// Documents that record changes rather than describe behaviour, by file stem: a release note is
/// never an Outcome (slice 1.5 review F2).
pub const CHANGELOG_STEMS: &[&str] = &[
    "changelog",
    "changes",
    "history",
    "release-notes",
    "releases",
    "updates",
    "whats-new",
];

fn is_changelog(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    let stem = name.split('.').next().unwrap_or(name).to_ascii_lowercase();
    CHANGELOG_STEMS.contains(&stem.as_str())
}

/// How a seed is used, read from its declaration: the Public access template's call form (slice
/// 1.5 review F3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Form {
    Function,
    Class,
    Method,
    ClassMethod,
    StaticMethod,
    Property,
}

/// One assertion before its id: its content and supports.
struct Draft {
    kind: AssertionKind,
    text: Option<String>,
    /// Finding supports (role, id, the finding's status and kind).
    findings: Vec<(SupportRole, Id, EvidenceStatus, FindingKind)>,
    /// Evidence supports (id, kind).
    evidence: Vec<(Id, EvidenceKind)>,
}

impl Draft {
    fn new(kind: AssertionKind, text: String) -> Self {
        Self {
            kind,
            text: Some(text),
            findings: Vec::new(),
            evidence: Vec::new(),
        }
    }

    fn citing(mut self, f: &FindingsRow) -> Self {
        self.findings.push((
            SupportRole::Support,
            f.finding_id,
            f.evidence_status,
            f.finding_kind,
        ));
        self
    }
}

/// A witness hop worth saying (slice 1.5 review F3): a definition, a property access or an
/// override-open call. A plain definite call goes without saying.
fn hop_note(caller: &str, callee: &str, w: &WitnessesRow) -> Option<String> {
    let open = if w.modality == Modality::Candidate {
        ", which a subclass may override"
    } else {
        ""
    };
    match (w.arc_kind, w.phase) {
        (ArcKind::Definition, _) => Some(format!("`{caller}` defines `{callee}`")),
        (_, Some(InvocationPhase::PropertyGet)) => {
            Some(format!("`{caller}` reads the property `{callee}`{open}"))
        }
        (_, Some(InvocationPhase::PropertySet)) => {
            Some(format!("`{caller}` sets the property `{callee}`{open}"))
        }
        _ if w.modality == Modality::Candidate => {
            Some(format!("`{caller}`'s call to `{callee}` is overridable"))
        }
        _ => None,
    }
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
         WHERE n.node_id IN ({}) ORDER BY n.node_id, label",
        hex_list(named.iter().copied())
    );
    // The least label per node, whichever run's row it is (slice 1.5 review O4).
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
                d.kind, pd.kind AS parent_kind, array_to_string(d.decorators, ',') AS decorators, \
                s.text, s.byte_len \
         FROM declarations d JOIN source_files s ON s.module_node_id = d.module_node_id \
         LEFT JOIN declarations pd ON pd.node_id = d.parent_node_id \
         WHERE d.node_id IN ({}) ORDER BY d.node_id",
        hex_list(seeds.iter().copied())
    );
    struct Decl {
        module: Id,
        docstring: Option<(usize, usize)>,
        form: Form,
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
            ("kind", DataType::Int16),
            ("parent_kind", DataType::Int16),
            ("decorators", DataType::Utf8),
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
        let decorators = text(b, "decorators", i).unwrap_or_default();
        let has = |d: &str| decorators.split(',').any(|x| x == d);
        let class = Some(DeclarationKind::Class.code());
        let form = if small(b, "kind", i) == class {
            Form::Class
        } else if small(b, "parent_kind", i) != class {
            Form::Function
        } else if has("staticmethod") {
            Form::StaticMethod
        } else if has("classmethod") {
            Form::ClassMethod
        } else if has("property") || has("cached_property") {
            Form::Property
        } else {
            Form::Method
        };
        decls.insert(
            node,
            Decl {
                module,
                docstring,
                form,
                text: text(b, "text", i),
            },
        );
    }

    // Passages that mention a seed exactly: by its declaration, or by an export whose target it
    // is. A member seed's aliases extend its class's export, and a mention of the class is not a
    // mention of the member.
    let mut exports_of: BTreeMap<Id, Vec<Id>> = BTreeMap::new();
    for (b, i) in Table::read(
        ctx,
        &format!(
            "SELECT DISTINCT target_node_id, export_node_id FROM exports \
             WHERE target_node_id IN ({}) ORDER BY 1, 2",
            hex_list(seeds.iter().copied())
        ),
        &[("target_node_id", ID), ("export_node_id", ID)],
    )
    .await?
    .rows()
    {
        if let (Some(t), Some(e)) = (id(b, "target_node_id", i), id(b, "export_node_id", i)) {
            exports_of.entry(t).or_default().push(e);
        }
    }
    let mention_targets: BTreeSet<Id> = seeds
        .iter()
        .copied()
        .chain(exports_of.values().flatten().copied())
        .collect();
    let mention_sql = format!(
        "SELECT t.target_node_id, p.node_id AS passage_node_id, p.document_node_id, \
                p.start_byte, p.text, d.path, p.ordinal, \
                m.start_byte AS mention_start, m.end_byte AS mention_end \
         FROM mention_targets t JOIN mentions m ON m.fact_id = t.mention_fact_id \
         JOIN passages p ON p.node_id = m.passage_node_id \
         JOIN documents d ON d.node_id = p.document_node_id \
         WHERE m.class = {exact} AND t.target_node_id IN ({targets}) \
         ORDER BY d.path, p.ordinal, m.start_byte, t.target_node_id",
        exact = MentionClass::Exact.code(),
        targets = hex_list(mention_targets.iter().copied())
    );
    struct Passage {
        node: Id,
        document: Id,
        start: i64,
        text: String,
        /// The mention, passage-relative.
        mention: (usize, usize),
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
            ("mention_start", DataType::Int64),
            ("mention_end", DataType::Int64),
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
        if text(b, "path", i).is_some_and(|p| is_changelog(&p)) {
            continue;
        }
        let (Some(ms), Some(me)) = (int(b, "mention_start", i), int(b, "mention_end", i)) else {
            continue;
        };
        let mention = ((ms - start) as usize, (me - start) as usize);
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
                mention,
            });
        }
    }

    // The seeds' own signatures' parameters.
    let params_sql = format!(
        "SELECT p.signature_node_id, ps.node_id, ps.fact_id, ps.ordinal, ps.name, ps.kind, \
                ps.default_text, ps.annotation_text, ps.start_byte, ps.end_byte, \
                sem.required, sem.fact_id AS semantics_fact_id, d.module_node_id \
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
        semantics: Option<Id>,
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
            ("semantics_fact_id", ID),
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
            semantics: id(b, "semantics_fact_id", i),
            module,
        });
    }

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

        // Outcome (§10.3): the docstring summary, else a passage's lead sentence holding an exact
        // mention of the seed; the assertion reads the sentence rendered, its evidence keeps the
        // bytes (slice 1.5 review O1, F2).
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
            let verbatim = source[a..z].to_owned();
            let ev = add_evidence(EvidenceRow {
                snapshot_id,
                evidence_id: recipe::evidence(
                    EvidenceKind::Span.code(),
                    Some(seed),
                    Some(decl.module),
                    Some((a as i64, z as i64)),
                    Some(&verbatim),
                ),
                evidence_kind: EvidenceKind::Span,
                cited_fact_id: None,
                node_id: Some(seed),
                module_node_id: Some(decl.module),
                start_byte: Some(a as i64),
                end_byte: Some(z as i64),
                text: Some(verbatim.clone()),
            });
            outcome.text = Some(normalized(&verbatim));
            outcome.evidence.push((ev, EvidenceKind::Span));
        } else if let Some((p, (a, z))) = passages.get(&seed).and_then(|ps| {
            ps.iter()
                .find_map(|p| mention_sentence(&p.text, p.mention).map(|s| (p, s)))
        }) {
            let verbatim = p.text[a..z].to_owned();
            let (start, end) = (p.start + a as i64, p.start + z as i64);
            let ev = add_evidence(EvidenceRow {
                snapshot_id,
                evidence_id: recipe::evidence(
                    EvidenceKind::Passage.code(),
                    Some(p.node),
                    Some(p.document),
                    Some((start, end)),
                    Some(&verbatim),
                ),
                evidence_kind: EvidenceKind::Passage,
                cited_fact_id: None,
                node_id: Some(p.node),
                module_node_id: Some(p.document),
                start_byte: Some(start),
                end_byte: Some(end),
                text: Some(verbatim.clone()),
            });
            outcome.text = Some(normalized(&verbatim));
            outcome.evidence.push((ev, EvidenceKind::Passage));
        }
        drafts.push(outcome);

        // Public access: the call form the seed's declaration gives, and every access path naming
        // it (slice 1.5 review F3).
        let form = decls.get(&seed).map_or(Form::Function, |d| d.form);
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
            let (owner, name) = access_path.rsplit_once('.').unwrap_or(("", access_path));
            let member = |what: &str, verb: &str, on: &str| {
                let also = if others.is_empty() {
                    String::new()
                } else {
                    format!(", and also {}", others.join(", "))
                };
                format!(
                    "{what} of `{owner}`: {verb} `{name}` {on}. It is named `{access_path}`{also}."
                )
            };
            let also = if others.is_empty() {
                String::new()
            } else {
                format!(
                    "; the same object is also reachable as {}",
                    others.join(", ")
                )
            };
            let text = match form {
                Form::Function => format!("Call it as `{access_path}`{also}."),
                Form::Class => format!("Construct one by calling `{access_path}`{also}."),
                Form::Method => member("A method", "call", "on an instance"),
                Form::ClassMethod => {
                    member("A class method", "call", "on the class or an instance")
                }
                Form::StaticMethod => {
                    member("A static method", "call", "on the class or an instance")
                }
                Form::Property => member("A property", "read", "on an instance"),
            };
            drafts.push(Draft::new(AssertionKind::PublicAccess, text).citing(f));
        }

        // Coordinates: what the operation already delegates to, each hop said as its witness
        // shows it (slice 1.4 review F1, slice 1.5 review F3).
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
            let mut first: Vec<_> = steps.iter().copied().filter(|w| w.path == 0).collect();
            first.sort_by_key(|w| w.step);
            let t = label(target);
            let count = |site: &str| {
                let more = if f.witnesses_omitted { " or more" } else { "" };
                let plural = if paths == 1 && more.is_empty() {
                    ""
                } else {
                    "s"
                };
                format!("{paths}{more} {site}{plural}")
            };
            let direct = f.finding_kind == FindingKind::DirectDelegation;
            let text = match first.as_slice() {
                [only] => match (only.arc_kind, only.phase) {
                    (ArcKind::Definition, _) => format!(
                        "`{seed_label}` defines `{t}`, a nested callable it returns or registers."
                    ),
                    (
                        _,
                        Some(phase @ (InvocationPhase::PropertyGet | InvocationPhase::PropertySet)),
                    ) => {
                        let verb = if phase == InvocationPhase::PropertyGet {
                            "reads"
                        } else {
                            "sets"
                        };
                        if direct {
                            format!(
                                "`{seed_label}` already {verb} the property `{t}` ({}).",
                                count("site")
                            )
                        } else {
                            format!(
                                "`{seed_label}` already {verb} the property `{t}` through an \
                                 overridable attribute, so a subclass may replace it."
                            )
                        }
                    }
                    _ if direct => {
                        format!(
                            "`{seed_label}` already calls `{t}` ({}).",
                            count("call site")
                        )
                    }
                    _ if only.modality == Modality::Candidate => format!(
                        "`{seed_label}` already calls `{t}` through an overridable method, so a \
                         subclass may replace it."
                    ),
                    _ => format!("`{seed_label}` already calls `{t}`."),
                },
                hops => {
                    let via = hops[..hops.len().saturating_sub(1)]
                        .iter()
                        .map(|w| format!("`{}`", label(w.callee_node_id)))
                        .collect::<Vec<_>>()
                        .join(" and ");
                    let mut caller = seed_label.clone();
                    let mut notes = Vec::new();
                    for w in hops {
                        let callee = label(w.callee_node_id);
                        notes.extend(hop_note(&caller, &callee, w));
                        caller = callee;
                    }
                    let notes = if notes.is_empty() {
                        String::new()
                    } else {
                        format!(" ({})", notes.join("; "))
                    };
                    format!("`{seed_label}` already reaches `{t}` through {via}{notes}.")
                }
            };
            drafts.push(Draft::new(AssertionKind::Coordinates, text).citing(f));
        }

        // Parameters of the seed's own signature (the receiver aside), each citing its syntax
        // fact and, where Pysa's model has it, its semantics fact (slice 1.5 review O5).
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
            let mut evidence = vec![(ev, EvidenceKind::Fact)];
            let mut parts = vec![p.kind.map_or("parameter", Codebook::text).replace('_', " ")];
            if let Some(d) = &p.default {
                parts.push(format!("default `{d}`"));
            }
            match (p.required, p.semantics) {
                (Some(required), Some(fact)) => {
                    let word = if required { "required" } else { "optional" };
                    evidence.push((
                        add_evidence(EvidenceRow {
                            snapshot_id,
                            evidence_id: recipe::evidence(
                                EvidenceKind::Fact.code(),
                                Some(p.node),
                                Some(p.module),
                                None,
                                Some(word),
                            ),
                            evidence_kind: EvidenceKind::Fact,
                            cited_fact_id: Some(fact),
                            node_id: Some(p.node),
                            module_node_id: Some(p.module),
                            start_byte: None,
                            end_byte: None,
                            text: Some(word.to_owned()),
                        }),
                        EvidenceKind::Fact,
                    ));
                    parts.push(word.to_owned());
                }
                _ => parts.push("requiredness not observed".to_owned()),
            }
            if let Some(a) = &p.annotation {
                parts.push(format!("annotated `{a}`"));
            }
            drafts.push(Draft {
                kind: AssertionKind::Parameter,
                text: Some(format!("`{}`: {}.", p.name, parts.join("; "))),
                findings: Vec::new(),
                evidence,
            });
        }

        // Limits: where the analysis stopped, one entry per reason, and apart for what the seed
        // calls itself and what the callables it reaches call (slice 1.5 review F3).
        let mut by_reason: BTreeMap<(StopReason, bool), Vec<&&FindingsRow>> = BTreeMap::new();
        for f in &findings {
            if matches!(
                f.finding_kind,
                FindingKind::ImplementationBoundary | FindingKind::IncompleteResolution
            ) && let Some(r) = f.stop_reason
            {
                let deep = r != StopReason::UnresolvedSite && f.depth.is_some_and(|d| d > 1);
                by_reason.entry((r, deep)).or_default().push(f);
            }
        }
        for ((reason, deep), group) in &by_reason {
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
            let who = if *deep {
                format!("Callables `{seed_label}` reaches call")
            } else {
                format!("`{seed_label}` calls")
            };
            let text = match reason {
                StopReason::ExternalBoundary => format!(
                    "{who} into code outside the analyzed release, which is not analyzed \
                     further: {listed}."
                ),
                StopReason::SyntheticBoundary => format!(
                    "{who} callables with no body in source (synthesized), which are not \
                     analyzed further: {listed}."
                ),
                StopReason::SubsystemBoundary => format!(
                    "{who} release code outside the analyzed subsystem, which is not analyzed \
                     further: {listed}."
                ),
                StopReason::UnresolvedSite => format!(
                    "{} call site{} reached from `{seed_label}` {} no resolved target.",
                    group.len(),
                    if group.len() == 1 { "" } else { "s" },
                    if group.len() == 1 { "has" } else { "have" }
                ),
                other => format!("The analysis of `{seed_label}` stopped: {}.", other.text()),
            };
            let mut draft = Draft::new(AssertionKind::AnalysisBoundary, text);
            for f in group {
                draft = draft.citing(f);
            }
            drafts.push(draft);
        }
        // The depth bound or a budget, citing Pass A's traversal-stop finding (slice 1.5 review F1).
        for f in findings
            .iter()
            .filter(|f| f.finding_kind == FindingKind::TraversalStop)
        {
            let text = match f.stop_reason {
                Some(StopReason::DepthLimit) => format!(
                    "What lies more than {} steps below `{seed_label}` (calls or nested \
                     definitions) was not followed (the analysis's depth bound).",
                    f.depth.unwrap_or_default()
                ),
                Some(StopReason::VertexBudget | StopReason::EdgeBudget) => format!(
                    "The delegation analysis of `{seed_label}` stopped at its budget; more \
                     delegations may exist than are listed."
                ),
                other => format!(
                    "The analysis of `{seed_label}` stopped: {}.",
                    other.map_or("unknown", |r| r.text())
                ),
            };
            drafts.push(Draft::new(AssertionKind::AnalysisBoundary, text).citing(f));
        }

        // Assertions, ordered by section then draft order.
        let mut ordered: Vec<(usize, Draft)> = drafts.into_iter().enumerate().collect();
        ordered.sort_by_key(|(i, d)| (section_of(d.kind).code(), *i));
        let mut assertion_ids = Vec::new();
        let mut uses_analysis = false;
        for (_, d) in ordered {
            let cited: Vec<(SupportRole, EvidenceStatus)> = d
                .findings
                .iter()
                .map(|(r, _, st, _)| (*r, *st))
                .chain(
                    d.evidence
                        .iter()
                        .map(|(_, k)| (SupportRole::Support, evidence_status(*k))),
                )
                .collect();
            let status = derive_status(&cited);
            let text = if status == EvidenceStatus::Unresolved {
                None
            } else {
                d.text
            };
            let supports: Vec<(i16, Option<Id>, Option<Id>)> = d
                .findings
                .iter()
                .map(|(r, f, _, _)| (r.code(), Some(*f), None))
                .chain(
                    d.evidence
                        .iter()
                        .map(|(e, _)| (SupportRole::Support.code(), None, Some(*e))),
                )
                .collect();
            uses_analysis |= d
                .findings
                .iter()
                .any(|(_, _, _, k)| ANALYSIS_BACKED.contains(k));
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
    fn a_summary_span_is_the_first_sentence_of_the_first_paragraph() {
        let src = "def f():\n    \"\"\"\n    Run it — fast. Then stop.\n\n    More.\n    \"\"\"\n";
        let start = src.find("\"\"\"").unwrap();
        let end = src.rfind("\"\"\"").unwrap() + 3;
        let (a, z) = summary_span(src, start, end).unwrap();
        assert_eq!(&src[a..z], "Run it — fast.");
        let one = "r'''Short.'''";
        let (a, z) = summary_span(one, 0, one.len()).unwrap();
        assert_eq!(&one[a..z], "Short.");
        assert_eq!(summary_span("\"\"\"   \"\"\"", 0, 9), None);
        // Slice 1.5 review O1: a hard-wrapped summary is one sentence, its bytes verbatim.
        let wrapped =
            "\"\"\"A configuration object that conforms\n    to the format. It adds fields.\"\"\"";
        let (a, z) = summary_span(wrapped, 0, wrapped.len()).unwrap();
        assert_eq!(
            &wrapped[a..z],
            "A configuration object that conforms\n    to the format."
        );
        assert_eq!(
            normalized(&wrapped[a..z]),
            "A configuration object that conforms to the format."
        );
        // A section header ends the summary paragraph; an unterminated summary is its line.
        let google = "\"\"\"Get a setting\n    Args:\n        key: the key.\n    \"\"\"";
        let (a, z) = summary_span(google, 0, google.len()).unwrap();
        assert_eq!(&google[a..z], "Get a setting");
    }

    #[test]
    fn an_outcome_sentence_holds_its_mention() {
        let p = "## Tools\n\n<Tip>x</Tip>\n```python\nmcp.tool()\n```\nEverything above is \
                 done. Then `pkg.tool` runs.\n\nThe `pkg.tool` decorator registers\na function. \
                 More.\n\n- `pkg.run`: starts the server.\n";
        let at = |needle: &str, nth: usize| {
            let s = p.match_indices(needle).nth(nth).unwrap().0;
            (s, s + needle.len())
        };
        // The mention is past its paragraph's lead sentence: no Outcome.
        assert_eq!(mention_sentence(p, at("`pkg.tool`", 0)), None);
        // In the lead sentence, across a soft line break.
        let (a, z) = mention_sentence(p, at("`pkg.tool`", 1)).unwrap();
        assert_eq!(&p[a..z], "The `pkg.tool` decorator registers\na function.");
        // A list item is a paragraph of its own, its marker excluded.
        let (a, z) = mention_sentence(p, at("`pkg.run`", 0)).unwrap();
        assert_eq!(&p[a..z], "`pkg.run`: starts the server.");
        assert!(paragraphs("## Only a heading\n").is_empty());
        assert!(is_changelog("docs/getting-started/whats-new.mdx"));
        assert!(is_changelog("docs/changelog.mdx"));
        assert!(!is_changelog("docs/servers/tools.mdx"));
    }
}
