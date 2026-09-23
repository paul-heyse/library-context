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
    ArcKind, AssertionKind, BriefSection, Codebook, DeclarationKind, EvidenceKind, EvidenceStatus,
    ExtractionMode, FindingKind, InvocationPhase, MemberRole, MentionClass, Modality,
    ParameterKind, ReviewState, StopReason, SupportRole,
};
use cpg_schema::findings::recipe::{self, AssertionKey};
use cpg_schema::findings::{
    ANALYSIS_BACKED, ASSERTION_POLICY, AssertionPolicyRow, AssertionSupportRow, AssertionsRow,
    BriefAssertionsRow, BriefDocumentsRow, BriefMembersRow, BriefsRow, EvidenceRow,
    FindingMembersRow, FindingsRow, WitnessesRow, derive_status, evidence_status, section_of,
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
/// depth, requiredness cited, traversal stops cited. 4: the brief document leaves out the
/// analysis's own boundaries (slice 1.9). 5: documented parameters, controls, transformed
/// controls and restrictions (slice 2.1). 6: call sites count call arcs only, "may call" over an
/// override-open final arc, a definition claims nothing more (increment-1 deep review F5). 7: usage
/// patterns and handoffs, and the pattern's code in the brief document (slice 2.2). 8: one
/// path-qualifier rule for Pass B's templates, and unfollowed controls (slice 2.1 review F1, F4).
/// 9: a usage pattern cites only a handoff it shows (slice 2.2 review F3).
pub const TEMPLATE_VERSION: i64 = 9;

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

/// `text` as a sentence: with a final stop when it has none.
fn sentence(text: &str) -> String {
    let t = text.trim_end();
    if t.ends_with(['.', '!', '?', ':']) {
        t.to_owned()
    } else {
        format!("{t}.")
    }
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
        .chain(
            found
                .witnesses
                .iter()
                .flat_map(|w| [w.caller_node_id, w.callee_node_id]),
        )
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
                sem.required, sem.fact_id AS semantics_fact_id, d.module_node_id, \
                pdoc.text AS doc_text, pdoc.start_byte AS doc_start, pdoc.end_byte AS doc_end, \
                rcv.parameter_node_id IS NOT NULL AS receiver \
         FROM parameters p JOIN parameter_syntax ps ON ps.fact_id = p.syntax_fact_id \
         LEFT JOIN parameter_semantics sem ON sem.fact_id = p.semantics_fact_id \
         LEFT JOIN parameter_docs pdoc ON pdoc.function_node_id = p.signature_node_id \
           AND pdoc.name = ps.name \
         LEFT JOIN ({receivers}) rcv ON rcv.parameter_node_id = ps.node_id \
         JOIN declarations d ON d.node_id = p.signature_node_id \
         WHERE p.signature_node_id IN ({seeds}) ORDER BY p.signature_node_id, ps.ordinal",
        receivers = cpg_schema::flows::receivers_sql(),
        seeds = hex_list(seeds.iter().copied())
    );
    struct Param {
        node: Id,
        fact: Id,
        name: String,
        kind: Option<ParameterKind>,
        default: Option<String>,
        annotation: Option<String>,
        span: (usize, usize),
        required: Option<bool>,
        semantics: Option<Id>,
        module: Id,
        /// The docstring's description of it (slice 2.1): normalized text and verbatim span.
        doc: Option<(String, usize, usize)>,
        /// The method's receiver, by the declaration's kind (slice 2.1 review F8).
        receiver: bool,
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
            ("doc_text", DataType::Utf8),
            ("doc_start", DataType::Int64),
            ("doc_end", DataType::Int64),
            ("receiver", DataType::Boolean),
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
            doc: text(b, "doc_text", i)
                .zip(int(b, "doc_start", i))
                .zip(int(b, "doc_end", i))
                .map(|((t, a), z)| (t, a as usize, z as usize)),
            receiver: flag(b, "receiver", i).unwrap_or(false),
        });
    }

    // Pass B (§9.2): the formals its findings name, and its guards' test and raise spans.
    let pass_b: Vec<&FindingsRow> = found
        .findings
        .iter()
        .filter(|f| {
            matches!(
                f.finding_kind,
                FindingKind::Forwarding
                    | FindingKind::TransformedArgument
                    | FindingKind::ConditionalRaise
                    | FindingKind::UnfollowedArgument
            )
        })
        .collect();
    let mut formal_names: BTreeMap<Id, String> = BTreeMap::new();
    for (b, i) in Table::read(
        ctx,
        &format!(
            "SELECT node_id, name FROM parameter_syntax WHERE node_id IN ({}) ORDER BY node_id",
            hex_list(pass_b.iter().filter_map(|f| f.related_node_id))
        ),
        &[("node_id", ID), ("name", DataType::Utf8)],
    )
    .await?
    .rows()
    {
        if let (Some(n), Some(name)) = (id(b, "node_id", i), text(b, "name", i)) {
            formal_names.entry(n).or_insert(name);
        }
    }
    struct Code {
        fact: Id,
        module: Id,
        span: (usize, usize),
        text: String,
    }
    let mut code: BTreeMap<Id, Code> = BTreeMap::new();
    let guard_nodes = pass_b
        .iter()
        .filter(|f| f.finding_kind == FindingKind::ConditionalRaise)
        .flat_map(|f| [f.related_node_id, f.condition_node_id])
        .flatten();
    for (b, i) in Table::read(
        ctx,
        &format!(
            "SELECT sn.node_id, sn.fact_id, sn.module_node_id, sn.start_byte, sn.end_byte, s.text \
             FROM syntax_nodes sn JOIN source_files s ON s.module_node_id = sn.module_node_id \
             WHERE sn.node_id IN ({}) ORDER BY sn.node_id",
            hex_list(guard_nodes)
        ),
        &[
            ("node_id", ID),
            ("fact_id", ID),
            ("module_node_id", ID),
            ("start_byte", DataType::Int64),
            ("end_byte", DataType::Int64),
            ("text", DataType::Utf8),
        ],
    )
    .await?
    .rows()
    {
        let (Some(n), Some(fact), Some(module), Some(a), Some(z), Some(source)) = (
            id(b, "node_id", i),
            id(b, "fact_id", i),
            id(b, "module_node_id", i),
            int(b, "start_byte", i),
            int(b, "end_byte", i),
            text(b, "text", i),
        ) else {
            continue;
        };
        if let Some(slice) = source.get(a as usize..z as usize) {
            code.entry(n).or_insert(Code {
                fact,
                module,
                span: (a as usize, z as usize),
                text: slice.to_owned(),
            });
        }
    }

    // Pass C and §10.5: each seed's handoffs, the formals they name, and its usage pattern.
    let handoff_formals: BTreeSet<Id> = found
        .members
        .iter()
        .filter(|m| m.role == MemberRole::Formal)
        .filter_map(|m| m.node_id)
        .collect();
    let mut formal_function: BTreeMap<Id, Id> = BTreeMap::new();
    for (b, i) in Table::read(
        ctx,
        &format!(
            "SELECT node_id, function_node_id FROM parameter_syntax WHERE node_id IN ({}) \
             ORDER BY node_id",
            hex_list(handoff_formals.iter().copied())
        ),
        &[("node_id", ID), ("function_node_id", ID)],
    )
    .await?
    .rows()
    {
        if let (Some(n), Some(f)) = (id(b, "node_id", i), id(b, "function_node_id", i)) {
            formal_function.entry(n).or_insert(f);
        }
    }
    // Each seed's handoff occurrences as (producer site, consumer site): a pattern that shows one
    // whole is preferred within its role (slice 2.2 review F3).
    let mut preferred: BTreeMap<Id, Vec<(Id, Id)>> = BTreeMap::new();
    for f in found
        .findings
        .iter()
        .filter(|f| f.finding_kind == FindingKind::Handoff)
    {
        let mut sites: Vec<&FindingMembersRow> = found
            .members
            .iter()
            .filter(|m| {
                m.finding_id == f.finding_id
                    && matches!(m.role, MemberRole::ConsumerSite | MemberRole::ProducerSite)
            })
            .collect();
        sites.sort_by_key(|m| m.ordinal);
        for pair in sites.chunks(2) {
            if let [p, c] = pair
                && p.role == MemberRole::ProducerSite
                && c.role == MemberRole::ConsumerSite
                && let (Some(p), Some(c)) = (p.node_id, c.node_id)
            {
                preferred.entry(f.subject_node_id).or_default().push((p, c));
            }
        }
    }
    let patterns = crate::usage::patterns(ctx, &seeds, &preferred).await?;

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
            // Call sites are the witness paths whose final arc is a call; a definition is none
            // (increment-1 deep review F5).
            let finals: BTreeMap<i64, &&WitnessesRow> =
                steps.iter().fold(BTreeMap::new(), |mut m, w| {
                    let e = m.entry(w.path).or_insert(w);
                    if w.step > e.step {
                        *e = w;
                    }
                    m
                });
            let paths = finals
                .values()
                .filter(|w| w.arc_kind == ArcKind::Call)
                .count();
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
                    (ArcKind::Definition, _) => {
                        format!("`{seed_label}` defines the nested callable `{t}`.")
                    }
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
            if p.receiver {
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
            // The docstring's own description of the parameter (slice 2.1), verbatim as evidence.
            let described = p.doc.as_ref().and_then(|(doc, a, z)| {
                let verbatim = decl.text.as_deref()?.get(*a..*z)?.to_owned();
                Some((normalized(doc), *a, *z, verbatim))
            });
            let text = match described {
                Some((doc, a, z, verbatim)) => {
                    evidence.push((
                        add_evidence(EvidenceRow {
                            snapshot_id,
                            evidence_id: recipe::evidence(
                                EvidenceKind::Span.code(),
                                Some(p.node),
                                Some(p.module),
                                Some((a as i64, z as i64)),
                                Some(&verbatim),
                            ),
                            evidence_kind: EvidenceKind::Span,
                            cited_fact_id: None,
                            node_id: Some(p.node),
                            module_node_id: Some(p.module),
                            start_byte: Some(a as i64),
                            end_byte: Some(z as i64),
                            text: Some(verbatim),
                        }),
                        EvidenceKind::Span,
                    ));
                    format!("`{}` ({}): {}", p.name, parts.join("; "), sentence(&doc))
                }
                None => format!("`{}`: {}.", p.name, parts.join("; ")),
            };
            drafts.push(Draft {
                kind: AssertionKind::Parameter,
                text: Some(text),
                findings: Vec::new(),
                evidence,
            });
        }

        // Pass B (§9.2): where each parameter is passed on, the literals the operation fixes, and
        // the branches in which the implementation raises.
        let member = |f: &FindingsRow, role: MemberRole| {
            found
                .members
                .iter()
                .filter(|m| m.finding_id == f.finding_id && m.role == role)
                .find_map(|m| m.label.clone())
                .unwrap_or_default()
        };
        let path_of = |f: &FindingsRow| {
            let mut steps: Vec<_> = found
                .witnesses
                .iter()
                .filter(|w| w.finding_id == f.finding_id && w.path == 0)
                .collect();
            steps.sort_by_key(|w| w.step);
            steps
        };
        let formal_of = |f: &FindingsRow| {
            f.related_node_id
                .and_then(|n| formal_names.get(&n))
                .cloned()
                .unwrap_or_default()
        };
        let ordinal_of = |name: &str| {
            params
                .get(&seed)
                .and_then(|ps| ps.iter().position(|p| p.name == name))
                .unwrap_or(usize::MAX)
        };
        // One path-qualifier rule for every Pass B template (slice 2.1 review F1): each
        // override-open hop named, and each call its caller makes only on some paths.
        let qualified = |f: &FindingsRow| {
            let conditional: BTreeSet<Id> = found
                .members
                .iter()
                .filter(|m| m.finding_id == f.finding_id && m.role == MemberRole::ConditionalCall)
                .filter_map(|m| m.node_id)
                .collect();
            let mut notes = Vec::new();
            for w in path_of(f) {
                let caller = if w.caller_node_id == seed {
                    seed_label.clone()
                } else {
                    label(w.caller_node_id)
                };
                let callee = label(w.callee_node_id);
                if w.modality == Modality::Candidate {
                    notes.push(format!("`{caller}`'s call to `{callee}` is overridable"));
                }
                if conditional.contains(&w.call_site_node_id) {
                    notes.push(format!("`{caller}` calls `{callee}` only on some paths"));
                }
            }
            if notes.is_empty() {
                String::new()
            } else {
                format!(" ({})", notes.join("; "))
            }
        };
        let mut forwarded: BTreeMap<(usize, String), Vec<&&FindingsRow>> = BTreeMap::new();
        for f in findings
            .iter()
            .filter(|f| f.finding_kind == FindingKind::Forwarding)
        {
            let p = member(f, MemberRole::SourceParameter);
            forwarded.entry((ordinal_of(&p), p)).or_default().push(f);
        }
        for ((_, p), mut group) in forwarded {
            group.sort_by_key(|f| {
                let steps = path_of(f);
                (
                    f.depth,
                    steps.last().map(|w| label(w.callee_node_id)),
                    formal_of(f),
                )
            });
            let items: Vec<String> = group
                .iter()
                .map(|f| {
                    let steps = path_of(f);
                    let target = steps
                        .last()
                        .map(|w| label(w.callee_node_id))
                        .unwrap_or_default();
                    let via = if steps.len() > 1 {
                        format!(" through `{}`", label(steps[0].callee_node_id))
                    } else {
                        String::new()
                    };
                    format!("`{target}` as `{}`{via}{}", formal_of(f), qualified(f))
                })
                .collect();
            let mut draft = Draft::new(
                AssertionKind::Control,
                format!("`{p}` is passed on to {}.", items.join("; ")),
            );
            for f in group {
                draft = draft.citing(f);
            }
            drafts.push(draft);
        }
        let mut fixed: Vec<&&FindingsRow> = findings
            .iter()
            .filter(|f| f.finding_kind == FindingKind::TransformedArgument)
            .collect();
        fixed.sort_by_key(|f| {
            let steps = path_of(f);
            (
                steps.first().map(|w| label(w.callee_node_id)),
                formal_of(f),
                member(f, MemberRole::Value),
            )
        });
        for f in fixed {
            let target = path_of(f)
                .first()
                .map(|w| label(w.callee_node_id))
                .unwrap_or_default();
            drafts.push(
                Draft::new(
                    AssertionKind::TransformedControl,
                    format!(
                        "`{seed_label}` calls `{target}` with `{}` fixed to `{}`{}.",
                        formal_of(f),
                        member(f, MemberRole::Value),
                        qualified(f)
                    ),
                )
                .citing(f),
            );
        }
        for f in findings
            .iter()
            .filter(|f| f.finding_kind == FindingKind::ConditionalRaise)
        {
            let (Some(test), Some(raise)) = (
                f.condition_node_id.and_then(|n| code.get(&n)),
                f.related_node_id.and_then(|n| code.get(&n)),
            ) else {
                continue;
            };
            let raised = normalized(&raise.text);
            let raised = raised
                .split('(')
                .next()
                .unwrap_or(&raised)
                .trim()
                .to_owned();
            let test_text = normalized(&test.text);
            let p = member(f, MemberRole::SourceParameter);
            let steps = path_of(f);
            let text = match steps.last() {
                None => format!(
                    "The implementation raises (`{raised}`) when `{test_text}`, a check on `{p}`."
                ),
                Some(w) => format!(
                    "`{}` raises (`{raised}`) when `{test_text}`; its `{}` receives `{p}`{}.",
                    label(w.callee_node_id),
                    member(f, MemberRole::Formal),
                    qualified(f)
                ),
            };
            let mut draft = Draft::new(AssertionKind::Restriction, text).citing(f);
            for (node, c) in [(f.condition_node_id, test), (f.related_node_id, raise)] {
                draft.evidence.push((
                    add_evidence(EvidenceRow {
                        snapshot_id,
                        evidence_id: recipe::evidence(
                            EvidenceKind::Fact.code(),
                            node,
                            Some(c.module),
                            Some((c.span.0 as i64, c.span.1 as i64)),
                            Some(&c.text),
                        ),
                        evidence_kind: EvidenceKind::Fact,
                        cited_fact_id: Some(c.fact),
                        node_id: node,
                        module_node_id: Some(c.module),
                        start_byte: Some(c.span.0 as i64),
                        end_byte: Some(c.span.1 as i64),
                        text: Some(c.text.clone()),
                    }),
                    EvidenceKind::Fact,
                ));
            }
            drafts.push(draft);
        }

        // Values Pass B does not follow (slice 2.1 review F4): one Limits line per parameter, so
        // "not listed as passed on" is never read as "not passed on".
        let mut unfollowed: BTreeMap<(usize, String), Vec<&&FindingsRow>> = BTreeMap::new();
        for f in findings
            .iter()
            .filter(|f| f.finding_kind == FindingKind::UnfollowedArgument)
        {
            let p = member(f, MemberRole::SourceParameter);
            unfollowed.entry((ordinal_of(&p), p)).or_default().push(f);
        }
        for ((_, p), mut group) in unfollowed {
            group.sort_by_key(|f| {
                let steps = path_of(f);
                (
                    f.depth,
                    steps.last().map(|w| label(w.callee_node_id)),
                    member(f, MemberRole::Reason),
                )
            });
            let mut items: Vec<String> = Vec::new();
            for f in &group {
                let steps = path_of(f);
                let target = steps
                    .last()
                    .map(|w| label(w.callee_node_id))
                    .unwrap_or_default();
                let via = if steps.len() > 1 {
                    format!(" through `{}`", label(steps[0].callee_node_id))
                } else {
                    String::new()
                };
                let why = match member(f, MemberRole::Reason).as_str() {
                    "rebound" => "after it is rebound",
                    "unmapped" => "unpacked, or where no single parameter takes it",
                    _ => "inside an expression",
                };
                let item = format!("`{target}`{via} ({why})");
                if !items.contains(&item) {
                    items.push(item);
                }
            }
            let mut draft = Draft::new(
                AssertionKind::UnfollowedControl,
                format!(
                    "`{p}` also reaches {}; the analysis does not follow it there.",
                    items.join("; ")
                ),
            );
            for f in group {
                draft = draft.citing(f);
            }
            drafts.push(draft);
        }

        // Usage pattern (§10.5): official code using the operation, with its setup, verbatim.
        if let Some(pattern) = patterns.get(&seed) {
            let mut draft = Draft::new(
                AssertionKind::UsagePattern,
                format!("From `{}`:\n```python\n{}\n```", pattern.path, pattern.code),
            );
            for st in &pattern.statements {
                draft.evidence.push((
                    add_evidence(EvidenceRow {
                        snapshot_id,
                        evidence_id: recipe::evidence(
                            EvidenceKind::Example.code(),
                            Some(st.node),
                            Some(st.module),
                            Some((st.span.0 as i64, st.span.1 as i64)),
                            Some(&st.text),
                        ),
                        evidence_kind: EvidenceKind::Example,
                        cited_fact_id: None,
                        node_id: Some(st.node),
                        module_node_id: Some(st.module),
                        start_byte: Some(st.span.0 as i64),
                        end_byte: Some(st.span.1 as i64),
                        text: Some(st.text.clone()),
                    }),
                    EvidenceKind::Example,
                ));
            }
            // The handoffs it shows: one whose recorded occurrence, producer and consumer site
            // both, lies inside the pattern (slice 2.2 review F3).
            for f in findings.iter().filter(|f| {
                if f.finding_kind != FindingKind::Handoff {
                    return false;
                }
                let mut sites: Vec<&FindingMembersRow> = found
                    .members
                    .iter()
                    .filter(|m| {
                        m.finding_id == f.finding_id
                            && matches!(m.role, MemberRole::ProducerSite | MemberRole::ConsumerSite)
                    })
                    .collect();
                sites.sort_by_key(|m| m.ordinal);
                sites.chunks(2).any(|pair| {
                    pair.len() == 2
                        && pair
                            .iter()
                            .all(|m| m.node_id.is_some_and(|n| pattern.sites.contains(&n)))
                })
            }) {
                draft = draft.citing(f);
            }
            drafts.push(draft);
        }
        // Handoffs (Pass C): the most frequent three, the seed consuming or producing.
        let mut handoffs: Vec<&&FindingsRow> = findings
            .iter()
            .filter(|f| f.finding_kind == FindingKind::Handoff)
            .collect();
        handoffs.sort_by(|a, b| {
            b.score
                .unwrap_or_default()
                .total_cmp(&a.score.unwrap_or_default())
                .then_with(|| {
                    a.related_node_id
                        .map(label)
                        .cmp(&b.related_node_id.map(label))
                })
        });
        for f in handoffs.into_iter().take(3) {
            let Some(other) = f.related_node_id.map(label) else {
                continue;
            };
            let formal = found
                .members
                .iter()
                .find(|m| m.finding_id == f.finding_id && m.role == MemberRole::Formal);
            let consumes = formal
                .and_then(|m| m.node_id)
                .and_then(|n| formal_function.get(&n))
                == Some(&seed);
            let formal_name = formal.and_then(|m| m.label.clone()).unwrap_or_default();
            let example = found
                .members
                .iter()
                .find(|m| m.finding_id == f.finding_id && m.role == MemberRole::ConsumerSite)
                .and_then(|m| m.label.clone())
                .unwrap_or_default();
            let n = f.score.unwrap_or_default() as i64;
            let occurrences = format!(
                "{n} occurrence{}, e.g. in `{example}`",
                if n == 1 { "" } else { "s" }
            );
            let text = if consumes {
                let what = match other.strip_suffix(".__init__") {
                    Some(class) => format!("a `{class}` instance"),
                    None => format!("what `{other}` returns"),
                };
                format!(
                    "Official usage passes {what} straight to `{seed_label}` as `{formal_name}` \
                     ({occurrences})."
                )
            } else {
                format!(
                    "Official usage passes what `{seed_label}` returns straight to `{other}` as \
                     `{formal_name}` ({occurrences})."
                )
            };
            drafts.push(Draft::new(AssertionKind::Handoff, text).citing(f));
        }

        // Limits: where the analysis stopped, one entry per reason, and apart for what the seed
        // calls itself and what the callables it reaches call (slice 1.5 review F3).
        // A boundary whose final arc is override-open is one the code may call (increment-1 deep
        // review F5).
        let may = |f: &FindingsRow| {
            found
                .witnesses
                .iter()
                .filter(|w| w.finding_id == f.finding_id && w.path == 0)
                .max_by_key(|w| w.step)
                .is_some_and(|w| w.modality == Modality::Candidate)
        };
        let mut by_reason: BTreeMap<(StopReason, bool, bool), Vec<&&FindingsRow>> = BTreeMap::new();
        for f in &findings {
            if matches!(
                f.finding_kind,
                FindingKind::ImplementationBoundary | FindingKind::IncompleteResolution
            ) && let Some(r) = f.stop_reason
            {
                let deep = r != StopReason::UnresolvedSite && f.depth.is_some_and(|d| d > 1);
                let may = r != StopReason::UnresolvedSite && may(f);
                by_reason.entry((r, deep, may)).or_default().push(f);
            }
        }
        for ((reason, deep, may), group) in &by_reason {
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
            let verb = if *may { "may call" } else { "calls" };
            let who = if *deep {
                format!(
                    "Callables `{seed_label}` reaches {}",
                    if *may { "may call" } else { "call" }
                )
            } else {
                format!("`{seed_label}` {verb}")
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
                    .filter(|p| !p.receiver)
                    .map(|p| p.name.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default();
        // The capability's own limits, never the analysis's scope: an `analysis_boundary` or an
        // `unfollowed_control` (what the analysis did not follow) stays in the brief and out of
        // the document (slice 1.5 review O7, measured in slice 1.9; deviation log D14).
        let limits: Vec<String> = assertion_ids
            .iter()
            .filter_map(assertion)
            .filter(|a| {
                section_of(a.assertion_kind) == BriefSection::Limits
                    && !matches!(
                        a.assertion_kind,
                        AssertionKind::AnalysisBoundary | AssertionKind::UnfollowedControl
                    )
            })
            .filter_map(|a| a.text.clone())
            .collect();
        let mut document =
            format!("Outcome: {outcome}\nPublic APIs: {apis}\nBuilt-in controls: {controls}");
        // The usage description (§11.1): the pattern's code.
        if let Some(pattern) = patterns.get(&seed) {
            document += &format!("\nUsage:\n{}", pattern.code);
        }
        if !limits.is_empty() {
            document += &format!("\nConditions and limitations: {}", limits.join(" "));
        }
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
            spec_hash: None,
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
