//! Typed document producer over markdown-rs offsets and captured derived blocks.
use lctx_model::domain::{calls::SymbolKind, documents::*, source::*, syntax::DeclarationKind, *};
use markdown::mdast::{AttributeContent, AttributeValue, Node};
use std::collections::{BTreeMap, BTreeSet, HashMap};
struct PublicName {
    access_path: String,
    origin_module: Option<String>,
    origin_name: Option<String>,
    origin_symbol_kind: Option<SymbolKind>,
    name: String,
}
struct Declaration {
    kind: DeclarationKind,
    name: String,
    node_id: Id<Occurrence>,
    parent_node_id: Option<Id<Occurrence>>,
    qualified_name: String,
}
#[derive(Clone, Copy)]
enum AttributeKind {
    Literal,
    Expression,
    Bare,
    Spread,
}
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
    fn new(public: &[PublicName], declarations: &[Declaration]) -> Self {
        let origin = |p: &PublicName| match (&p.origin_module, &p.origin_name) {
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
        let classes: HashMap<Id<Occurrence>, &str> = declarations
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
    // The documentary recognizer intentionally nominates ASCII words (keywords included),
    // rather than expanding its task policy to every Python Unicode identifier.
    !text.is_empty() && text.split('.').all(|segment| segment.is_ascii()
        && (ruff_python_stdlib::identifiers::is_identifier(segment)
            || ruff_python_stdlib::keyword::is_keyword(segment)))
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
        // `C.m`, or `p.C.m` where `p.C` is this library's class: another library's `C.m` (a
        // migration guide's `httpx.Client.get`) is no mention of ours (C5 review F6).
        let prefix = &segs[..segs.len() - 1];
        let ours = prefix.len() == 1 || v.paths.contains_key(&prefix.join("."));
        if ours && let Some(members) = v.members.get(&tail) {
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
    /// MDX JSX components, in pre-order (the holistic assessment's A3).
    components: Vec<Component>,
}

/// One MDX JSX element as the mdast gives it.
struct Component {
    start: usize,
    end: usize,
    parent: Option<usize>,
    depth: i64,
    name: Option<String>,
    form: ComponentForm,
    /// First child's start to last child's end; none when self-closing.
    inner: Option<(usize, usize)>,
    /// The first direct paragraph child.
    lead: Option<(usize, usize)>,
    /// Name, value, kind.
    attributes: Vec<(Option<String>, Option<String>, AttributeKind)>,
}

fn attribute(a: &AttributeContent) -> (Option<String>, Option<String>, AttributeKind) {
    match a {
        AttributeContent::Property(p) => match &p.value {
            None => (Some(p.name.clone()), None, AttributeKind::Bare),
            Some(AttributeValue::Literal(v)) => (
                Some(p.name.clone()),
                Some(v.clone()),
                AttributeKind::Literal,
            ),
            Some(AttributeValue::Expression(e)) => (
                Some(p.name.clone()),
                Some(e.value.clone()),
                AttributeKind::Expression,
            ),
        },
        AttributeContent::Expression(e) => (None, Some(e.value.clone()), AttributeKind::Spread),
    }
}

fn collect(n: &Node, text: &str, v: &Vocabulary, out: &mut Collected) {
    collect_in(n, text, v, out, None, 0);
}

/// The walk, inside the component `parent` (at `depth` components deep).
fn collect_in(
    n: &Node,
    text: &str,
    v: &Vocabulary,
    out: &mut Collected,
    parent: Option<usize>,
    depth: i64,
) {
    let element = match n {
        Node::MdxJsxFlowElement(e) => {
            Some((&e.name, &e.attributes, &e.children, ComponentForm::Flow))
        }
        Node::MdxJsxTextElement(e) => {
            Some((&e.name, &e.attributes, &e.children, ComponentForm::Text))
        }
        _ => None,
    };
    if let (Some((name, attributes, children, form)), Some((s, e))) = (element, span(n)) {
        let spans: Vec<(usize, usize)> = children.iter().filter_map(span).collect();
        let inner = spans.first().zip(spans.last()).map(|(a, z)| (a.0, z.1));
        let lead = children
            .iter()
            .find(|c| matches!(c, Node::Paragraph(_)))
            .and_then(span);
        let index = out.components.len();
        out.components.push(Component {
            start: s,
            end: e,
            parent,
            depth,
            name: name.clone(),
            form,
            inner,
            lead,
            attributes: attributes.iter().map(attribute).collect(),
        });
        for c in children {
            collect_in(c, text, v, out, Some(index), depth + 1);
        }
        return;
    }
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
        collect_in(c, text, v, out, parent, depth);
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
/// its own under the capture's reserved `_lctx/blocks/`, named from the document's path so it is a
/// valid module name.
pub(crate) fn block_module_path(document: &str, ordinal: i64) -> String {
    let name: String = document
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    // The digest keeps the name injective: `a-b.mdx` and `a_b.mdx` read alike (C5 review F7).
    let digest = lctx_model::domain::ContentHash::of(document.as_bytes()).hex();
    format!("_lctx/blocks/d_{name}_{}/block_{ordinal}.py", &digest[..8])
}

/// A document's Python code blocks, with the ordinals `document` gives them and each block's byte
/// span in the document: what acquisition derives into `_lctx/` (C5b). A document that does not
/// parse has none.
pub(crate) fn python_blocks(bytes: &[u8]) -> Vec<(i64, usize, usize, String)> {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return Vec::new();
    };
    let mut options = markdown::ParseOptions::mdx();
    options.constructs.frontmatter = true;
    let Ok(tree) = markdown::to_mdast(text, &options) else {
        return Vec::new();
    };
    if !within_tree_bounds(&tree) {
        return Vec::new();
    }
    let mut found = Collected::default();
    collect(&tree, text, &Vocabulary::default(), &mut found);
    found
        .code
        .into_iter()
        .enumerate()
        .filter(|(_, (_, _, language, _, _))| is_python(language_of(language)))
        .map(|(i, (start, end, _, _, code))| (i as i64, start, end, code))
        .collect()
}

use crate::{
    acquisition::{AcquiredInput, Acquisition},
    assembly,
    bundle::{Declared, ProviderStage, StageContext},
};
use lctx_model::domain::{
    assertion::*,
    attribution::*,
    calls::ProviderModule,
    charged::StateCharge,
    conditions::Diagram,
    input::SourceRole,
    stages::{Effect, Profile, ProviderOutcome, RelationUse, Stage, StageSink},
    symbols::*,
    syntax::*,
};
pub const DOCUMENTS: &str = "documents";
pub struct Documents;
pub fn provider() -> Provider {
    Provider {
        tool: "markdown-rs".into(),
        revision: "1.0.0".into(),
        build_digest: crate::bundle::build_digest(&[include_str!("document_parser.rs")]),
    }
}
macro_rules! document_types {
    ($apply:ident) => {
        $apply!(
            DocumentNode,
            DocumentAttributeValue,
            DocumentObservation,
            DocumentSupport,
            PassageObservation,
            PassageSupport,
            CodeBlockObservation,
            CodeBlockSupport,
            DocumentLinkObservation,
            DocumentLinkSupport,
            DocumentMentionObservation,
            DocumentMentionSupport,
            DocumentComponentObservation,
            DocumentComponentSupport,
            DocumentAttributeObservation,
            DocumentAttributeSupport
        )
    };
}
impl Declared for Documents {
    fn declaration(&self, _: Profile) -> Stage {
        macro_rules! uses {($($ty:ty),+)=>{vec![$(RelationUse::of::<$ty>()),+]}}
        let provider = provider();
        Stage {
            name: DOCUMENTS,
            inputs: uses!(
                Module,
                Occurrence,
                SyntaxObservation,
                DeclarationObservation,
                PublicNameObservation,
                ExportOrigin,
                ProviderModule
            ),
            outputs: document_types!(uses),
            contributes: assembly::vocabulary(),
            coverage: vec![FactFamily::Docs]
                .into_iter()
                .map(|family| lctx_model::domain::stages::FamilyCoverage {
                    family,
                    provider: provider.id(),
                })
                .collect(),
            profiles: vec![Profile::Catalog, Profile::Behavioral],
            effect: Effect::Extraction,
            code: provider.build_digest,
            configuration: ContentHash::of(
                b"mdx-frontmatter; source=64MiB; nodes=1000000; depth=256",
            ),
        }
    }
}
impl Vocabulary {
    fn typed<S: StageSink + 'static>(
        context: &mut StageContext<S>,
    ) -> Result<(Self, StateCharge), ModelError> {
        let mut charge = StateCharge::new(context.budget(), "documents_vocabulary");
        fn rows<R: Record, S: StageSink + 'static>(
            context: &mut StageContext<S>,
            charge: &mut StateCharge,
        ) -> Result<Vec<R>, ModelError> {
            let mut rows = vec![];
            for batch in context.handoff::<R>()? {
                for row in batch.rows() {
                    charge.grow((size_of::<R>() + row.heap_bytes() + 256).saturating_mul(4))?;
                    rows.push(row.clone());
                }
            }
            Ok(rows)
        }
        let modules: BTreeMap<_, _> = rows::<Module, _>(context, &mut charge)?
            .into_iter()
            .map(|r| (r.id(), r))
            .collect();
        let provider_modules: BTreeMap<_, _> = rows::<ProviderModule, _>(context, &mut charge)?
            .into_iter()
            .map(|r| (r.id(), r))
            .collect();
        let origins: BTreeMap<_, _> = rows::<ExportOrigin, _>(context, &mut charge)?
            .into_iter()
            .map(|r| (r.id(), r))
            .collect();
        let public = rows::<PublicNameObservation, _>(context, &mut charge)?
            .into_iter()
            .map(|p| {
                let access = modules.get(&p.access).ok_or_else(|| {
                    ModelError::Invalid("document vocabulary access module absent".into())
                })?;
                let (origin_module, origin_name, kind) = match origins.get(&p.origin) {
                    Some(ExportOrigin::Traced { module, name, kind }) => (
                        Some(
                            match provider_modules.get(module).ok_or_else(|| {
                                ModelError::Invalid(
                                    "document vocabulary origin module absent".into(),
                                )
                            })? {
                                ProviderModule::Acquired { module } => modules
                                    .get(module)
                                    .ok_or_else(|| {
                                        ModelError::Invalid(
                                            "document vocabulary acquired module absent".into(),
                                        )
                                    })?
                                    .qualified_name
                                    .clone(),
                                ProviderModule::Bundled { name, .. }
                                | ProviderModule::Namespace { name, .. }
                                | ProviderModule::Unresolved { name, .. } => name.clone(),
                            },
                        ),
                        Some(name.clone()),
                        *kind,
                    ),
                    Some(ExportOrigin::Untraced) => (None, None, None),
                    None => {
                        return Err(ModelError::Invalid(
                            "document vocabulary origin absent".into(),
                        ));
                    }
                };
                let origin_symbol_kind = kind.and_then(|k| match k {
                    ExportKind::Function => Some(SymbolKind::Function),
                    ExportKind::Method => Some(SymbolKind::Method),
                    ExportKind::Class => Some(SymbolKind::Class),
                    ExportKind::Module => Some(SymbolKind::Module),
                    _ => None,
                });
                Ok(PublicName {
                    access_path: format!("{}.{}", access.qualified_name, p.name),
                    name: p.name,
                    origin_module,
                    origin_name,
                    origin_symbol_kind,
                })
            })
            .collect::<Result<Vec<_>, ModelError>>()?;
        let names: BTreeMap<_, _> = rows::<SyntaxObservation, _>(context, &mut charge)?
            .into_iter()
            .map(|s| (s.occurrence, s.spelling))
            .collect();
        let occurrences: BTreeMap<_, _> = rows::<Occurrence, _>(context, &mut charge)?
            .into_iter()
            .map(|o| (o.id(), o))
            .collect();
        let declarations = rows::<DeclarationObservation, _>(context, &mut charge)?;
        let mut qualified = BTreeMap::new();
        // Admit intermediate and vocabulary copies before joining or indexing them.
        let public_bytes = public
            .iter()
            .map(|p| {
                p.access_path.len()
                    + p.name.len()
                    + p.origin_name.as_ref().map_or(0, String::len)
                    + p.origin_module.as_ref().map_or(0, String::len)
                    + 512
            })
            .sum::<usize>();
        charge.grow(public_bytes.saturating_mul(16))?;
        for declaration in &declarations {
            let occurrence = occurrences
                .get(&declaration.declaration)
                .ok_or_else(|| ModelError::Invalid("document declaration source absent".into()))?;
            let module = modules
                .values()
                .find(|m| m.source == occurrence.source)
                .ok_or_else(|| ModelError::Invalid("document declaration module absent".into()))?;
            let mut chain = vec![];
            let mut next = Some(declaration);
            let mut depth = 0;
            while let Some(d) = next {
                depth += 1;
                if depth > 256 {
                    return Err(ModelError::Invalid(
                        "document declaration nesting limit".into(),
                    ));
                }
                let name = names.get(&d.name).ok_or_else(|| {
                    ModelError::Invalid("document declaration name absent".into())
                })?;
                charge.grow((name.len() + module.qualified_name.len() + 256).saturating_mul(16))?;
                chain.push(name.clone());
                next = d
                    .parent
                    .and_then(|id| declarations.iter().find(|d| d.declaration == id));
            }
            chain.reverse();
            qualified.insert(
                declaration.declaration,
                format!("{}.{}", module.qualified_name, chain.join(".")),
            );
        }
        let declarations = declarations
            .iter()
            .map(|d| Declaration {
                kind: d.kind,
                name: names[&d.name].clone(),
                node_id: d.declaration,
                parent_node_id: d.parent,
                qualified_name: qualified[&d.declaration].clone(),
            })
            .collect::<Vec<_>>();
        let vocabulary = Self::new(&public, &declarations);

        Ok((vocabulary, charge))
    }
}
macro_rules! supported {
    ($context:expr,$run:expr,$surface:expr,$evidence:expr,$ty:ident,$support:ident,$row:expr,$mode:expr,$origin:expr,$fidelity:expr) => {{
        let row: $ty = $row;
        $context.emit($support {
            assertion: row.id(),
            run: $run.id(),
            surface: $surface.id(),
            evidence: $evidence.id(),
            origin: $origin,
            mode: $mode,
            fidelity: $fidelity,
        })?;
        $context.emit(row)?;
    }};
}
impl<S: StageSink + 'static> ProviderStage<S> for Documents {
    fn run(&mut self, context: &mut StageContext<S>) -> Result<ProviderOutcome, ModelError> {
        macro_rules! declare {($($ty:ty),+)=>{$(context.declare::<$ty>()?;)+}}
        document_types!(declare);
        let (vocabulary, _charge) = Vocabulary::typed(context)?;
        let provider = provider();
        context.contribute(provider.clone())?;
        let captured = context.captured();
        let mut partial = false;
        for input in captured.inputs() {
            let library = match input.acquisition() {
                Acquisition::Corpus { library, .. } => captured.inputs().get(*library),
                _ => None,
            };
            let analysis =
                crate::pyrefly_stage::analysis_context(input, library, captured.config())?;
            let (run, families) = ProviderRun::new(
                provider.id(),
                analysis.id(),
                input.captured().revision().id(),
                analysis.config_digest,
                [FactFamily::Docs],
            )?;
            let surface = ProviderSurface {
                provider: provider.id(),
                family: FactFamily::Docs,
                name: "native mdast and declared API vocabulary".into(),
            };
            context.contribute(analysis.clone())?;
            context.contribute(run.clone())?;
            for family in families {
                context.contribute(family)?;
            }
            context.contribute(surface.clone())?;
            let documents: BTreeSet<_> = input
                .uses()
                .iter()
                .filter(|u| u.role == SourceRole::Document)
                .map(|u| u.artifact)
                .collect();
            for artifact in input
                .captured()
                .artifacts()
                .iter()
                .filter(|a| documents.contains(&a.id()))
            {
                partial |= document(
                    context,
                    input,
                    artifact,
                    &vocabulary,
                    &analysis,
                    &run,
                    &surface,
                )?;
            }
        }
        Ok(if partial {
            ProviderOutcome::Partial
        } else {
            ProviderOutcome::Complete
        })
    }
}
fn source_span<S: StageSink + 'static>(
    context: &mut StageContext<S>,
    source: &SourceArtifact,
    start: usize,
    end: usize,
) -> Result<(Evidence, EvidenceSourceSpanId), ModelError> {
    let evidence = Evidence::SourceSpan {
        source: source.id(),
        start: start as i64,
        end: end as i64,
    };
    let span = EvidenceSourceSpanId::of(&evidence)?;
    context.contribute(evidence.clone())?;
    Ok((evidence, span))
}
fn document<S: StageSink + 'static>(
    context: &mut StageContext<S>,
    input: &AcquiredInput,
    source: &SourceArtifact,
    vocabulary: &Vocabulary,
    analysis: &AnalysisContext,
    run: &ProviderRun,
    surface: &ProviderSurface,
) -> Result<bool, ModelError> {
    let scope = CoverageScope::Artifact {
        artifact: source.id(),
    };
    context.contribute(scope.clone())?;
    let (condition, nodes) = Diagram::always().records();
    context.contribute(condition.clone())?;
    for node in nodes {
        context.contribute(node)?;
    }
    let q = AssertionQualification {
        assumptions: lctx_model::domain::assumptions::AssumptionSet::empty_id(),
        context: analysis.id(),
        scope: scope.id(),
        condition: condition.id(),
        modality: Modality::Definite,
        approximation: Approximation::Exact,
    };
    context.contribute(q.clone())?;
    let (whole, _) = source_span(context, source, 0, source.byte_len as usize)?;
    if source.byte_len > 64 << 20 {
        supported!(
            context,
            run,
            surface,
            whole,
            DocumentObservation,
            DocumentSupport,
            DocumentObservation {
                qualification: q.id(),
                source: source.id(),
                title: None,
                parsed: false
            },
            ExtractionMode::NativeTraversal,
            Origin::InputContext,
            Fidelity::Raw
        );
        context.contribute(ProviderCoverage {
            scope: scope.id(),
            provider: Some(run.provider),
            context: analysis.id(),
            family: FactFamily::Docs,
            run: Some(run.id()),
            status: CoverageStatus::Unavailable,
            reason: Some(ObligationKind::ResourceRefused),
            diagnostic: Some("document exceeds 64 MiB source limit".into()),
        })?;
        return Ok(true);
    }
    let _source = context.budget().reserve(
        "document_source_and_native_collections",
        (source.byte_len as usize)
            .saturating_mul(32)
            .saturating_add(4096),
    )?;
    let bytes =
        std::fs::read(input.captured().root().join(&source.path)).map_err(ModelError::codec)?;
    if ContentHash::of(&bytes) != source.content {
        return Err(ModelError::Invalid(
            "document source differs from capture".into(),
        ));
    }
    let mut options = markdown::ParseOptions::mdx();
    options.constructs.frontmatter = true;
    let parsed = std::str::from_utf8(&bytes)
        .map_err(|e| e.to_string())
        .and_then(|text| markdown::to_mdast(text, &options).map_err(|e| e.to_string()));
    let tree = match parsed {
        Ok(tree) => tree,
        Err(error) => {
            let reason = if std::str::from_utf8(&bytes).is_err() {
                ObligationKind::UndecodableSource
            } else {
                ObligationKind::OutsideProviderModel
            };
            supported!(
                context,
                run,
                surface,
                whole,
                DocumentObservation,
                DocumentSupport,
                DocumentObservation {
                    qualification: q.id(),
                    source: source.id(),
                    title: None,
                    parsed: false
                },
                ExtractionMode::NativeTraversal,
                Origin::InputContext,
                Fidelity::Raw
            );
            let error: String = error.chars().take(1024).collect();
            context.contribute(ProviderCoverage {
                scope: scope.id(),
                provider: Some(run.provider),
                context: analysis.id(),
                family: FactFamily::Docs,
                run: Some(run.id()),
                status: CoverageStatus::Partial,
                reason: Some(reason),
                diagnostic: Some(error),
            })?;
            return Ok(true);
        }
    };
    // Native parser trees are provider-owned. Check structural traversal bounds before retained
    // collections or recursive presentation helpers are built.
    if !within_tree_bounds(&tree) {
        supported!(
            context,
            run,
            surface,
            whole,
            DocumentObservation,
            DocumentSupport,
            DocumentObservation {
                qualification: q.id(),
                source: source.id(),
                title: None,
                parsed: false
            },
            ExtractionMode::NativeTraversal,
            Origin::InputContext,
            Fidelity::Raw
        );
        context.contribute(ProviderCoverage {
            scope: scope.id(),
            provider: Some(run.provider),
            context: analysis.id(),
            family: FactFamily::Docs,
            run: Some(run.id()),
            status: CoverageStatus::Partial,
            reason: Some(ObligationKind::BudgetReached),
            diagnostic: Some("document traversal bound reached".into()),
        })?;
        return Ok(true);
    }
    let text = std::str::from_utf8(&bytes).map_err(ModelError::codec)?;
    let yaml = tree.children().into_iter().flatten().find_map(|n| {
        if let Node::Yaml(y) = n {
            Some((&y.value, span(n).map_or(0, |s| s.1)))
        } else {
            None
        }
    });
    supported!(
        context,
        run,
        surface,
        whole,
        DocumentObservation,
        DocumentSupport,
        DocumentObservation {
            qualification: q.id(),
            source: source.id(),
            title: yaml.and_then(|(y, _)| title(y)),
            parsed: true
        },
        ExtractionMode::NativeTraversal,
        Origin::InputContext,
        Fidelity::Raw
    );
    let body_start = yaml.map_or(0, |(_, end)| end);
    let mut bounds = vec![];
    let mut heading_stack: Vec<(u8, String)> = vec![];
    for child in tree.children().into_iter().flatten() {
        if let (Node::Heading(h), Some((start, _))) = (child, span(child)) {
            let heading = plain(child);
            while heading_stack
                .last()
                .is_some_and(|(depth, _)| *depth >= h.depth)
            {
                heading_stack.pop();
            }
            let path = heading_stack
                .iter()
                .map(|(_, heading)| heading.clone())
                .collect::<Vec<_>>();
            heading_stack.push((h.depth, heading.clone()));
            bounds.push((start, h.depth as i64, Some(heading), path));
        }
    }
    let first = bounds.first().map_or(text.len(), |b| b.0);
    if text
        .get(body_start..first)
        .is_some_and(|text| !text.trim().is_empty())
    {
        bounds.insert(0, (body_start, 0, None, vec![]));
    }
    let starts: Vec<_> = bounds.iter().map(|b| b.0).collect();
    let mut passages = vec![];
    for (ordinal, (start, level, heading, heading_path)) in bounds.into_iter().enumerate() {
        let end = starts.get(ordinal + 1).copied().unwrap_or(text.len());
        let (evidence, span) = source_span(context, source, start, end)?;
        let node = DocumentNode::Passage {
            span,
            ordinal: ordinal as i64,
        };
        let passage = DocumentNodePassageId::of(&node)?;
        context.emit(node)?;
        passages.push(passage);
        supported!(
            context,
            run,
            surface,
            evidence,
            PassageObservation,
            PassageSupport,
            PassageObservation {
                qualification: q.id(),
                passage,
                level,
                heading,
                heading_path,
                text: text[start..end].into()
            },
            ExtractionMode::NativeTraversal,
            Origin::SourceObservation,
            Fidelity::NativeStructural
        );
    }
    let passage_of = |start: usize| {
        starts
            .partition_point(|s| *s <= start)
            .checked_sub(1)
            .map(|i| passages[i])
    };
    let mut found = Collected::default();
    collect(&tree, text, vocabulary, &mut found);
    for (ordinal, (start, end, language, meta, code)) in found.code.into_iter().enumerate() {
        let Some(passage) = passage_of(start) else {
            continue;
        };
        let (evidence, span) = source_span(context, source, start, end)?;
        let node = DocumentNode::CodeBlock {
            span,
            ordinal: ordinal as i64,
        };
        let block = DocumentNodeCodeBlockId::of(&node)?;
        context.emit(node)?;
        let content = ContentHash::of(code.as_bytes());
        let module_path = is_python(language_of(&language))
            .then(|| block_module_path(&source.path, ordinal as i64));
        let materialized = if let Some(path) = &module_path {
            let artifact = input
                .captured()
                .artifacts()
                .iter()
                .find(|a| a.path == *path)
                .ok_or_else(|| {
                    ModelError::Invalid(
                        "document Python block was not captured before providers ran".into(),
                    )
                })?;
            if artifact.content != content || artifact.byte_len != code.len() as i64 {
                return Err(ModelError::Invalid(
                    "materialized document block differs from parsed code".into(),
                ));
            }
            Some(artifact.id())
        } else {
            None
        };
        supported!(
            context,
            run,
            surface,
            evidence,
            CodeBlockObservation,
            CodeBlockSupport,
            CodeBlockObservation {
                qualification: q.id(),
                block,
                passage,
                language,
                meta,
                code,
                content,
                module_path,
                materialized
            },
            ExtractionMode::NativeTraversal,
            Origin::SourceObservation,
            Fidelity::NativeStructural
        );
    }
    for (ordinal, (start, end, url, title, text)) in found.links.into_iter().enumerate() {
        let Some(passage) = passage_of(start) else {
            continue;
        };
        let (evidence, span) = source_span(context, source, start, end)?;
        let node = DocumentNode::Link {
            span,
            ordinal: ordinal as i64,
        };
        let link = DocumentNodeLinkId::of(&node)?;
        context.emit(node)?;
        supported!(
            context,
            run,
            surface,
            evidence,
            DocumentLinkObservation,
            DocumentLinkSupport,
            DocumentLinkObservation {
                qualification: q.id(),
                link,
                passage,
                url,
                title,
                text
            },
            ExtractionMode::NativeTraversal,
            Origin::SourceObservation,
            Fidelity::NativeStructural
        );
    }
    let mut per_span = BTreeMap::new();
    for (_, _, _, _, _, start, end) in &found.found {
        *per_span.entry((*start, *end)).or_insert(0) += 1;
    }
    for (class, source_kind, form, access_path, qualified_name, start, end) in found.found {
        let Some(passage) = passage_of(start) else {
            continue;
        };
        let (evidence, span) = source_span(context, source, start, end)?;
        let node = DocumentNode::Mention { span };
        let mention = DocumentNodeMentionId::of(&node)?;
        context.emit(node)?;
        let q = if class == MentionClass::Exact && per_span[&(start, end)] == 1 {
            q.clone()
        } else {
            AssertionQualification {
                modality: Modality::Candidate,
                ..q.clone()
            }
        };
        context.contribute(q.clone())?;
        supported!(
            context,
            run,
            surface,
            evidence,
            DocumentMentionObservation,
            DocumentMentionSupport,
            DocumentMentionObservation {
                qualification: q.id(),
                mention,
                passage,
                class,
                source: source_kind,
                form,
                access_path,
                qualified_name
            },
            ExtractionMode::Recognizer,
            Origin::DerivedAnalysis,
            Fidelity::NormalizedStructural
        );
    }
    let mut components = vec![];
    for (ordinal, c) in found.components.into_iter().enumerate() {
        let Some(passage) = passage_of(c.start) else {
            return Err(ModelError::Invalid(
                "component outside document passages".into(),
            ));
        };
        let (evidence, span) = source_span(context, source, c.start, c.end)?;
        let node = DocumentNode::Component {
            span,
            ordinal: ordinal as i64,
        };
        let component = DocumentNodeComponentId::of(&node)?;
        context.emit(node)?;
        let parent = c.parent.map(|index| components[index]);
        components.push(component);
        let inner = match c.inner {
            Some((start, end)) => Some(source_span(context, source, start, end)?.1),
            None => None,
        };
        let lead = match c.lead {
            Some((start, end)) => Some(source_span(context, source, start, end)?.1),
            None => None,
        };
        supported!(
            context,
            run,
            surface,
            evidence,
            DocumentComponentObservation,
            DocumentComponentSupport,
            DocumentComponentObservation {
                qualification: q.id(),
                component,
                passage,
                parent,
                depth: c.depth,
                name: c.name,
                form: c.form,
                inner,
                lead
            },
            ExtractionMode::NativeTraversal,
            Origin::SourceObservation,
            Fidelity::NativeStructural
        );
        for (ordinal, (name, value, kind)) in c.attributes.into_iter().enumerate() {
            let value = match kind {
                AttributeKind::Literal => DocumentAttributeValue::Literal {
                    name: name.unwrap(),
                    value: value.unwrap(),
                },
                AttributeKind::Expression => DocumentAttributeValue::Expression {
                    name: name.unwrap(),
                    source: value.unwrap(),
                },
                AttributeKind::Bare => DocumentAttributeValue::Bare {
                    name: name.unwrap(),
                },
                AttributeKind::Spread => DocumentAttributeValue::Spread {
                    source: value.unwrap(),
                },
            };
            context.emit(value.clone())?;
            supported!(
                context,
                run,
                surface,
                evidence,
                DocumentAttributeObservation,
                DocumentAttributeSupport,
                DocumentAttributeObservation {
                    qualification: q.id(),
                    component,
                    ordinal: ordinal as i64,
                    value: value.id()
                },
                ExtractionMode::NativeTraversal,
                Origin::SourceObservation,
                Fidelity::NativeStructural
            );
        }
    }
    context.contribute(ProviderCoverage {
        scope: scope.id(),
        provider: Some(run.provider),
        context: analysis.id(),
        family: FactFamily::Docs,
        run: Some(run.id()),
        status: CoverageStatus::CompleteUnderStatedModel,
        reason: None,
        diagnostic: None,
    })?;
    Ok(false)
}

fn within_tree_bounds(tree: &Node) -> bool {
    let mut pending = vec![(tree, 0usize)];
    let mut nodes = 0;
    while let Some((node, depth)) = pending.pop() {
        nodes += 1;
        if depth > 256 || nodes > 1_000_000 {
            return false;
        }
        for child in node.children().into_iter().flatten() {
            pending.push((child, depth + 1));
        }
    }
    true
}
