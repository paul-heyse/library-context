//! The `docs` family (CPG slice C5, DESIGN §3.2): a corpus release's documents parsed by
//! markdown-rs (MDX constructs and frontmatter; its offsets are bytes, probe P5), their passages,
//! code blocks and links, and our recognizer's mentions of the library's API.
//!
//! A passage is a top-level heading's section: it runs to the next top-level heading, so a
//! document's passages partition it, and the text before the first heading is passage 0. Code
//! blocks, links and mentions are found at any depth (MDX components hold most code blocks) and
//! belong to the passage their start falls in.
//!
//! Mentions come in two classes that are never merged (DESIGN §3.2 `docs`):
//! - `exact`: inline code (or a dotted token in prose) that is a public access path, a public
//!   name's origin path, or a public class's member (`FastMCP.tool`);
//! - `lexical`: inline code that is a bare public name, or a distinctive bare public name in prose
//!   (`FastMCP`, `get_context`: an interior capital or an underscore), one `candidate` row per
//!   access path it could be.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use cpg_schema::codebook::{
    BoundaryReason, CoverageStatus, DeclarationKind, ExtractionMode, Fidelity, MentionClass,
    MentionSource, Modality, Origin, SymbolKind,
};
use cpg_schema::id::{Id, content_digest, recipe};
use cpg_schema::tables::{
    CodeBlocks, CodeBlocksRow, DeclarationsRow, DocLinks, DocLinksRow, Documents, DocumentsRow,
    Mentions, MentionsRow, Passages, PassagesRow, PublicNamesRow,
};
use markdown::mdast::Node;

use crate::facts::{FactSink, Provenance, Surface, fact_row};

/// The library's API names a mention may match: from the library run's public names and
/// declarations. Re-exports of one origin collapse to its shortest access path, so a name the
/// package re-exports three times is one candidate, not three.
#[derive(Default)]
pub(crate) struct Vocabulary {
    /// Each public access path → itself; each other origin path (`module.name`) → its shortest
    /// access path.
    paths: BTreeMap<String, BTreeSet<String>>,
    /// Each public name of a class, function, method or module → the shortest access path of each
    /// origin with that name. A public variable is not a bare-name mention (every module's
    /// `logger` is one).
    by_name: BTreeMap<String, BTreeSet<String>>,
    /// `Class.member` of a public class → the members' qualified names.
    members: BTreeMap<String, BTreeSet<String>>,
}

impl Vocabulary {
    pub(crate) fn new(public: &[PublicNamesRow], declarations: &[DeclarationsRow]) -> Self {
        let origin = |p: &PublicNamesRow| match (&p.origin_module, &p.origin_name) {
            (Some(m), Some(n)) => format!("{m}.{n}"),
            _ => p.access_path.clone(),
        };
        let mut shortest: BTreeMap<String, &str> = BTreeMap::new();
        for p in public {
            let best = shortest.entry(origin(p)).or_insert(&p.access_path);
            if (p.access_path.len(), p.access_path.as_str()) < (best.len(), *best) {
                *best = &p.access_path;
            }
        }
        let access: BTreeSet<&str> = public.iter().map(|p| p.access_path.as_str()).collect();
        let mut v = Vocabulary::default();
        for p in public {
            let o = origin(p);
            let canonical = shortest[&o].to_owned();
            v.paths
                .entry(p.access_path.clone())
                .or_default()
                .insert(p.access_path.clone());
            // An access path names itself, even when it is also an origin path.
            if !access.contains(o.as_str()) {
                v.paths.entry(o).or_default().insert(canonical.clone());
            }
            let named = matches!(
                p.origin_symbol_kind,
                Some(
                    SymbolKind::Class
                        | SymbolKind::Function
                        | SymbolKind::Method
                        | SymbolKind::Module
                )
            );
            if named {
                v.by_name
                    .entry(p.name.clone())
                    .or_default()
                    .insert(canonical);
            }
        }
        let classes: HashMap<Id, &str> = declarations
            .iter()
            .filter(|d| d.kind == DeclarationKind::Class && v.by_name.contains_key(&d.name))
            .map(|d| (d.node_id, d.name.as_str()))
            .collect();
        for d in declarations {
            if let Some(class) = d.parent_node_id.and_then(|p| classes.get(&p)) {
                v.members
                    .entry(format!("{class}.{}", d.name))
                    .or_default()
                    .insert(d.qualified_name.clone());
            }
        }
        v
    }
}

#[derive(Default)]
pub(crate) struct DocsOut {
    pub documents: Vec<DocumentsRow>,
    pub passages: Vec<PassagesRow>,
    pub code_blocks: Vec<CodeBlocksRow>,
    pub links: Vec<DocLinksRow>,
    pub mentions: Vec<MentionsRow>,
}

/// A document's coverage: complete, or unavailable with the reason and the parser's message.
pub(crate) struct Covered {
    pub document: Id,
    pub status: CoverageStatus,
    pub reason: Option<BoundaryReason>,
    pub detail: Option<String>,
}

fn provenance(surface: Surface, mode: ExtractionMode, origin: Origin) -> Provenance {
    Provenance {
        surface,
        mode,
        origin,
        modality: Modality::Definite,
        fidelity: Fidelity::NativeStructural,
    }
}

/// The frontmatter's `title:` value, as written (quotes removed).
fn title(yaml: &str) -> Option<String> {
    yaml.lines().find_map(|l| {
        let v = l.strip_prefix("title:")?.trim();
        Some(v.trim_matches(|c| c == '"' || c == '\'').to_owned())
    })
}

/// The plain text of a node's descendants.
fn plain(n: &Node) -> String {
    match n {
        Node::Text(t) => t.value.clone(),
        Node::InlineCode(c) => c.value.clone(),
        other => other
            .children()
            .map(|cs| cs.iter().map(plain).collect::<String>())
            .unwrap_or_default(),
    }
}

fn span(n: &Node) -> Option<(usize, usize)> {
    n.position().map(|p| (p.start.offset, p.end.offset))
}

/// A dotted identifier, as the recognizer reads one.
fn is_dotted(text: &str) -> bool {
    !text.is_empty()
        && text.split('.').all(|seg| {
            let mut chars = seg.chars();
            chars
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        })
}

/// A bare name distinctive enough to count in prose: an underscore inside it, or at least two
/// capitals and a lower-case letter (`FastMCP`, `ToolError`; not `Context`, `run`, `MCP`).
fn distinctive(name: &str) -> bool {
    let inner = name.trim_matches('_');
    let upper = name.chars().filter(char::is_ascii_uppercase).count();
    let lower = name.chars().filter(char::is_ascii_lowercase).count();
    (inner.contains('_') && inner.len() > 1) || (upper >= 2 && lower >= 1)
}

/// One mention before its fact: class, source, form, target, span.
type Found = (
    MentionClass,
    MentionSource,
    String,
    Option<String>,
    Option<String>,
    usize,
    usize,
);

/// What a code or prose text names, by the rules in the module doc.
fn recognize(
    v: &Vocabulary,
    text: &str,
    source: MentionSource,
    form: &str,
    at: (usize, usize),
    out: &mut Vec<Found>,
) {
    let exact = |out: &mut Vec<Found>, path: Option<String>, qualified: Option<String>| {
        out.push((
            MentionClass::Exact,
            source,
            form.to_owned(),
            path,
            qualified,
            at.0,
            at.1,
        ));
    };
    if let Some(paths) = v.paths.get(text) {
        for p in paths {
            exact(out, Some(p.clone()), None);
        }
        return;
    }
    if text.contains('.') {
        let segs: Vec<&str> = text.split('.').collect();
        let tail = segs[segs.len() - 2..].join(".");
        if let Some(members) = v.members.get(&tail) {
            for q in members {
                exact(out, None, Some(q.clone()));
            }
        }
        return;
    }
    let bare_counts = match source {
        MentionSource::InlineCode => true,
        MentionSource::Prose => distinctive(text),
    };
    if bare_counts && let Some(paths) = v.by_name.get(text) {
        for p in paths {
            out.push((
                MentionClass::Lexical,
                source,
                form.to_owned(),
                Some(p.clone()),
                None,
                at.0,
                at.1,
            ));
        }
    }
}

/// Every dotted identifier token of a prose slice, with its byte span in the document.
fn tokens(slice: &str, base: usize) -> Vec<(usize, usize)> {
    let bytes = slice.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        let starts = (c.is_ascii_alphabetic() || c == b'_')
            && (i == 0 || !(bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_'));
        if !starts {
            i += 1;
            continue;
        }
        let start = i;
        while i < bytes.len()
            && (bytes[i].is_ascii_alphanumeric()
                || bytes[i] == b'_'
                || (bytes[i] == b'.'
                    && i + 1 < bytes.len()
                    && (bytes[i + 1].is_ascii_alphabetic() || bytes[i + 1] == b'_')))
        {
            i += 1;
        }
        out.push((base + start, base + i));
    }
    out
}

/// What a walk of the tree collects, each at its start offset.
#[derive(Default)]
#[allow(
    clippy::type_complexity,
    reason = "each tuple is one mdast node's fields, used once"
)]
struct Collected {
    /// Start, end, language, meta, code.
    code: Vec<(usize, usize, Option<String>, Option<String>, String)>,
    /// Start, end, URL, title, text.
    links: Vec<(usize, usize, String, Option<String>, String)>,
    found: Vec<Found>,
}

fn collect(n: &Node, text: &str, v: &Vocabulary, out: &mut Collected) {
    match n {
        Node::Code(c) => {
            if let Some((s, e)) = span(n) {
                out.code
                    .push((s, e, c.lang.clone(), c.meta.clone(), c.value.clone()));
            }
            return;
        }
        Node::InlineCode(c) => {
            if let Some((s, e)) = span(n) {
                let mut t = c.value.trim().trim_start_matches('@');
                if let Some(i) = t.find('(') {
                    t = &t[..i];
                }
                if is_dotted(t) {
                    let form = text.get(s..e).unwrap_or(&c.value);
                    recognize(
                        v,
                        t,
                        MentionSource::InlineCode,
                        form,
                        (s, e),
                        &mut out.found,
                    );
                }
            }
            return;
        }
        Node::Link(l) => {
            if let Some((s, e)) = span(n) {
                out.links
                    .push((s, e, l.url.clone(), l.title.clone(), plain(n)));
            }
        }
        Node::Text(_) => {
            if let Some((s, e)) = span(n) {
                let slice = text.get(s..e).unwrap_or("");
                for (ts, te) in tokens(slice, s) {
                    let token = &text[ts..te];
                    recognize(
                        v,
                        token,
                        MentionSource::Prose,
                        token,
                        (ts, te),
                        &mut out.found,
                    );
                }
            }
            return;
        }
        _ => {}
    }
    for c in n.children().into_iter().flatten() {
        collect(c, text, v, out);
    }
}

fn language_of(language: &Option<String>) -> &str {
    language.as_deref().unwrap_or("")
}

/// A fenced block the usage run compiles (C5b): tagged Python.
fn is_python(language: &str) -> bool {
    matches!(language, "python" | "py" | "python3")
}

/// Where a document's Python code block `ordinal` is materialized, release-relative: a module of
/// its own under `_lctx_blocks/`, named from the document's path so it is a valid module name.
pub(crate) fn block_module_path(document: &str, ordinal: i64) -> String {
    let name: String = document
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    format!("_lctx_blocks/d_{name}/block_{ordinal}.py")
}

/// A document's Python code blocks, with the ordinals `document` gives them: what the usage run
/// materializes before it starts (C5b). A document that does not parse has none.
pub(crate) fn python_blocks(bytes: &[u8]) -> Vec<(i64, String)> {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return Vec::new();
    };
    let mut options = markdown::ParseOptions::mdx();
    options.constructs.frontmatter = true;
    let Ok(tree) = markdown::to_mdast(text, &options) else {
        return Vec::new();
    };
    let mut found = Collected::default();
    collect(&tree, text, &Vocabulary::default(), &mut found);
    found
        .code
        .into_iter()
        .enumerate()
        .filter(|(_, (_, _, language, _, _))| is_python(language_of(language)))
        .map(|(i, (_, _, _, _, code))| (i as i64, code))
        .collect()
}

/// One document: its row, and when it parses its passages, code blocks, links and mentions.
pub(crate) fn document(
    sink: &mut FactSink,
    release_id: Id,
    path: &str,
    bytes: &[u8],
    v: &Vocabulary,
    out: &mut DocsOut,
) -> Covered {
    let node_id = recipe::document(release_id, path);
    let text = std::str::from_utf8(bytes).ok();
    let mut options = markdown::ParseOptions::mdx();
    options.constructs.frontmatter = true;
    let parsed = text.map(|t| markdown::to_mdast(t, &options));
    let tree = match &parsed {
        Some(Ok(tree)) => Some(tree),
        Some(Err(_)) | None => None,
    };
    let yaml = tree.and_then(|t| {
        t.children()?.iter().find_map(|c| match c {
            Node::Yaml(y) => Some((y.value.clone(), span(c).map_or(0, |s| s.1))),
            _ => None,
        })
    });
    out.documents.push(fact_row!(
        sink,
        Documents,
        Provenance {
            fidelity: Fidelity::Raw,
            ..provenance(
                Surface::Source,
                ExtractionMode::NativeTraversal,
                Origin::InputContext
            )
        },
        DocumentsRow {
            snapshot_id: Id::ZERO,
            fact_id: Id::ZERO,
            node_id,
            release_id,
            path: path.to_owned(),
            content_digest: content_digest(bytes),
            byte_len: bytes.len() as i64,
            title: yaml.as_ref().and_then(|(y, _)| title(y)),
            parsed: tree.is_some(),
        }
    ));
    let (Some(text), Some(tree)) = (text, tree) else {
        let (reason, detail) = match &parsed {
            None => (BoundaryReason::UndecodableSource, "not UTF-8".to_owned()),
            Some(Err(e)) => (BoundaryReason::OutsideProviderModel, e.to_string()),
            Some(Ok(_)) => unreachable!("a tree parses"),
        };
        return Covered {
            document: node_id,
            status: CoverageStatus::Unavailable,
            reason: Some(reason),
            detail: Some(detail),
        };
    };

    // Passages: the preamble after the frontmatter, then one per top-level heading.
    let body_start = yaml.as_ref().map_or(0, |(_, end)| *end);
    let mut bounds: Vec<(usize, i64, Option<String>, Vec<String>)> = Vec::new();
    let mut path_stack: Vec<(u8, String)> = Vec::new();
    for c in tree.children().into_iter().flatten() {
        if let (Node::Heading(h), Some((s, _))) = (c, span(c)) {
            let heading = plain(c);
            while path_stack.last().is_some_and(|(d, _)| *d >= h.depth) {
                path_stack.pop();
            }
            let heading_path = path_stack.iter().map(|(_, t)| t.clone()).collect();
            path_stack.push((h.depth, heading.clone()));
            bounds.push((s, i64::from(h.depth), Some(heading), heading_path));
        }
    }
    let first = bounds.first().map_or(text.len(), |b| b.0);
    if text
        .get(body_start..first)
        .is_some_and(|pre| !pre.trim().is_empty())
    {
        bounds.insert(0, (body_start, 0, None, Vec::new()));
    }
    let starts: Vec<usize> = bounds.iter().map(|b| b.0).collect();
    let mut passage_ids = Vec::new();
    for (i, (start, level, heading, heading_path)) in bounds.into_iter().enumerate() {
        let end = starts.get(i + 1).copied().unwrap_or(text.len());
        let id = recipe::passage(node_id, i as i64);
        passage_ids.push(id);
        out.passages.push(fact_row!(
            sink,
            Passages,
            provenance(
                Surface::MarkdownRs,
                ExtractionMode::NativeTraversal,
                Origin::SourceObservation
            ),
            PassagesRow {
                snapshot_id: Id::ZERO,
                fact_id: Id::ZERO,
                node_id: id,
                document_node_id: node_id,
                ordinal: i as i64,
                level,
                heading,
                heading_path,
                start_byte: start as i64,
                end_byte: end as i64,
                text: text[start..end].to_owned(),
            }
        ));
    }
    // What lies before every passage (the frontmatter) belongs to none.
    let passage_of = |at: usize| -> Option<Id> {
        let i = starts.partition_point(|&s| s <= at);
        i.checked_sub(1).map(|i| passage_ids[i])
    };

    let mut found = Collected::default();
    collect(tree, text, v, &mut found);
    for (ordinal, (s, e, language, meta, code)) in found.code.into_iter().enumerate() {
        let code_language = language.clone();
        let Some(passage) = passage_of(s) else {
            continue;
        };
        out.code_blocks.push(fact_row!(
            sink,
            CodeBlocks,
            provenance(
                Surface::MarkdownRs,
                ExtractionMode::NativeTraversal,
                Origin::SourceObservation
            ),
            CodeBlocksRow {
                snapshot_id: Id::ZERO,
                fact_id: Id::ZERO,
                node_id: recipe::code_block(node_id, ordinal as i64),
                document_node_id: node_id,
                passage_node_id: passage,
                ordinal: ordinal as i64,
                language,
                meta,
                start_byte: s as i64,
                end_byte: e as i64,
                content_digest: content_digest(code.as_bytes()),
                module_path: is_python(language_of(&code_language))
                    .then(|| block_module_path(path, ordinal as i64)),
                code,
            }
        ));
    }
    let mut per_passage: HashMap<Id, i64> = HashMap::new();
    for (s, e, url, link_title, link_text) in found.links {
        let Some(passage) = passage_of(s) else {
            continue;
        };
        let ordinal = per_passage.entry(passage).or_insert(0);
        out.links.push(fact_row!(
            sink,
            DocLinks,
            provenance(
                Surface::MarkdownRs,
                ExtractionMode::NativeTraversal,
                Origin::SourceObservation
            ),
            DocLinksRow {
                snapshot_id: Id::ZERO,
                fact_id: Id::ZERO,
                passage_node_id: passage,
                ordinal: *ordinal,
                url,
                title: link_title,
                text: link_text,
                start_byte: s as i64,
                end_byte: e as i64,
            }
        ));
        *ordinal += 1;
    }
    // A span naming several targets names each as a candidate.
    let mut per_span: HashMap<(usize, usize), usize> = HashMap::new();
    for f in &found.found {
        *per_span.entry((f.5, f.6)).or_insert(0) += 1;
    }
    for (class, source, form, access_path, qualified_name, s, e) in found.found {
        let Some(passage) = passage_of(s) else {
            continue;
        };
        let modality = match class {
            MentionClass::Exact if per_span[&(s, e)] == 1 => Modality::Definite,
            MentionClass::Exact | MentionClass::Lexical => Modality::Candidate,
        };
        out.mentions.push(fact_row!(
            sink,
            Mentions,
            Provenance {
                surface: Surface::Docs,
                mode: ExtractionMode::Recognizer,
                origin: Origin::DerivedAnalysis,
                modality,
                fidelity: Fidelity::NormalizedStructural,
            },
            MentionsRow {
                snapshot_id: Id::ZERO,
                fact_id: Id::ZERO,
                passage_node_id: passage,
                class,
                source,
                form,
                access_path,
                qualified_name,
                start_byte: s as i64,
                end_byte: e as i64,
            }
        ));
    }
    Covered {
        document: node_id,
        status: CoverageStatus::CompleteUnderStatedModel,
        reason: None,
        detail: None,
    }
}
