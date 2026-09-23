//! Stage F's usage patterns (DESIGN §10.5): for each seed, a verbatim statement subset of the
//! official usage code that uses it, with the setup it needs.
//!
//! - **Candidates** are the seed's call and decorator sites in the usage modules: official
//!   examples first, then doc blocks, then tests, each by module path and position. A site of one
//!   of the seed's handoffs (Pass C) comes first within its role.
//! - **A pattern** is the statement holding the site, in a module or function body (never inside
//!   a `with`, `if`, loop or `try`, whose header the pattern would lose), plus, transitively, every
//!   statement of the same block before it that binds a name the pattern reads, and every import
//!   binding one. A candidate reading a name bound anywhere else (an enclosing function's
//!   parameter, another block) or bound nowhere (neither a binding nor a builtin) is not
//!   self-contained and is refused (slice 2.2 review F1).
//! - **Publication.** Each statement is cut from its line start and dedented by its own
//!   indentation. The assembled code must parse (Ruff), and every name it loads, annotations
//!   included, must be bound in it, imported by it or a builtin of the analyzed Python; otherwise
//!   the candidate is refused (§10.4). The smallest pattern of the best role wins. Its evidence is
//!   each statement's verbatim span.

use std::collections::{BTreeMap, BTreeSet};

use arrow_array::{Array, FixedSizeBinaryArray, Int16Array, Int64Array, RecordBatch, StringArray};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use cpg_schema::codebook::{
    BindingKind, Codebook, EdgeKind, Modality, SourceRole, SyntaxField, SyntaxKind,
};
use cpg_schema::id::Id;
use datafusion::prelude::SessionContext;

use crate::delta::to_schema;
use crate::{CoreError, sql};

/// How many candidate sites per seed are tried, in candidate order.
pub const MAX_CANDIDATES: usize = 12;
/// The largest pattern, in statements.
pub const MAX_STATEMENTS: usize = 12;

/// One statement of a pattern: its node, module and verbatim span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternStatement {
    pub node: Id,
    pub module: Id,
    pub span: (usize, usize),
    pub text: String,
}

/// A seed's usage pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pattern {
    pub role: SourceRole,
    pub path: String,
    /// The site it was built around.
    pub site: Id,
    pub statements: Vec<PatternStatement>,
    /// The code as published: each statement dedented, in source order.
    pub code: String,
    /// Every call site its statements contain: the handoffs it shows (review F3).
    pub sites: BTreeSet<Id>,
}

const ID: DataType = DataType::FixedSizeBinary(16);

fn schema(fields: &[(&str, DataType)]) -> SchemaRef {
    std::sync::Arc::new(Schema::new(
        fields
            .iter()
            .map(|(n, t)| Field::new(*n, t.clone(), true))
            .collect::<Vec<_>>(),
    ))
}

async fn rows(
    ctx: &SessionContext,
    query: &str,
    fields: &[(&str, DataType)],
) -> Result<Vec<RecordBatch>, CoreError> {
    let s = schema(fields);
    sql::query(ctx, query)
        .await?
        .collect()
        .await?
        .iter()
        .map(|b| to_schema(b, &s))
        .collect()
}

fn id(b: &RecordBatch, name: &str, i: usize) -> Option<Id> {
    let a = b
        .column_by_name(name)?
        .as_any()
        .downcast_ref::<FixedSizeBinaryArray>()?;
    (!a.is_null(i)).then(|| Id(<[u8; 16]>::try_from(a.value(i)).expect("16 bytes")))
}

fn int(b: &RecordBatch, name: &str, i: usize) -> Option<i64> {
    let a = b
        .column_by_name(name)?
        .as_any()
        .downcast_ref::<Int64Array>()?;
    (!a.is_null(i)).then(|| a.value(i))
}

fn small(b: &RecordBatch, name: &str, i: usize) -> Option<i16> {
    let a = b
        .column_by_name(name)?
        .as_any()
        .downcast_ref::<Int16Array>()?;
    (!a.is_null(i)).then(|| a.value(i))
}

fn text(b: &RecordBatch, name: &str, i: usize) -> Option<String> {
    let a = b
        .column_by_name(name)?
        .as_any()
        .downcast_ref::<StringArray>()?;
    (!a.is_null(i)).then(|| a.value(i).to_owned())
}

fn hex_list(ids: impl IntoIterator<Item = Id>) -> String {
    let list: Vec<String> = ids.into_iter().map(|i| format!("X'{}'", i.hex())).collect();
    if list.is_empty() {
        "NULL".to_owned()
    } else {
        list.join(", ")
    }
}

#[derive(Debug, Clone)]
struct Node {
    parent: Option<Id>,
    kind: i16,
    field: i16,
    ordinal: i64,
    start: usize,
    end: usize,
}

/// What a read resolves to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Read {
    Binding(Id),
    Builtin,
    /// Neither: a name no scope binds (a continuation block's, say; C5 O2).
    Unresolved,
}

#[derive(Debug, Clone)]
struct Binding {
    kind: BindingKind,
    site: Id,
    /// The bound name's span: how an import's statement is found (its alias has no node).
    span: (usize, usize),
}

/// One module's syntax, bindings, name reads and text.
#[derive(Debug, Default)]
struct Module {
    nodes: BTreeMap<Id, Node>,
    bindings: BTreeMap<Id, Binding>,
    /// Each read name's span and what it resolves to.
    reads: Vec<(usize, usize, Read)>,
    /// Each call site's span.
    calls: Vec<(Id, usize, usize)>,
    text: String,
}

impl Module {
    /// The innermost statement whose span holds `span` (for a binding whose site is no node).
    fn statement_at(&self, span: (usize, usize), statements: &BTreeSet<i16>) -> Option<Id> {
        self.nodes
            .iter()
            .filter(|(_, n)| statements.contains(&n.kind) && n.start <= span.0 && span.1 <= n.end)
            .min_by_key(|(id, n)| (n.end - n.start, **id))
            .map(|(id, _)| *id)
    }

    /// The statement holding a node: its nearest statement ancestor, the node included.
    fn statement(&self, node: Id, statements: &BTreeSet<i16>) -> Option<Id> {
        let mut at = node;
        for _ in 0..64 {
            let n = self.nodes.get(&at)?;
            if statements.contains(&n.kind) {
                return Some(at);
            }
            at = n.parent?;
        }
        None
    }
}

/// The candidate sites of every seed, in candidate order.
async fn candidates(
    ctx: &SessionContext,
    seeds: &[Id],
) -> Result<BTreeMap<Id, Vec<(Id, Id, SourceRole, String)>>, CoreError> {
    let statement = format!(
        "SELECT DISTINCT e.dst_node_id AS seed, e.src_node_id AS site, sn.module_node_id, \
                sf.role, sf.path, sn.start_byte, \
                CASE sf.role WHEN {example} THEN 0 WHEN {doc} THEN 1 ELSE 2 END AS rank \
         FROM edges e JOIN syntax_nodes sn ON sn.node_id = e.src_node_id \
         JOIN ({usage}) sf ON sf.module_node_id = sn.module_node_id \
         JOIN facts f ON f.fact_id = e.evidence_fact_id \
         WHERE e.edge_kind IN ({call_target}, {site_target}) AND e.dst_node_id IN ({seeds}) \
           AND f.modality IN ({definite}, {candidate}) \
         ORDER BY seed, rank, sf.path, sn.start_byte, site",
        usage = cpg_schema::flows::usage_files_sql(),
        example = SourceRole::Example.code(),
        doc = SourceRole::DocBlock.code(),
        call_target = EdgeKind::CallTarget.code(),
        site_target = EdgeKind::SiteTarget.code(),
        definite = Modality::Definite.code(),
        candidate = Modality::Candidate.code(),
        seeds = hex_list(seeds.iter().copied()),
    );
    let mut out: BTreeMap<Id, Vec<(Id, Id, SourceRole, String)>> = BTreeMap::new();
    for b in rows(
        ctx,
        &statement,
        &[
            ("seed", ID),
            ("site", ID),
            ("module_node_id", ID),
            ("role", DataType::Int16),
            ("path", DataType::Utf8),
            ("start_byte", DataType::Int64),
            ("rank", DataType::Int64),
        ],
    )
    .await?
    {
        for i in 0..b.num_rows() {
            let (Some(seed), Some(site), Some(module), Some(role), Some(path)) = (
                id(&b, "seed", i),
                id(&b, "site", i),
                id(&b, "module_node_id", i),
                small(&b, "role", i).and_then(SourceRole::from_code),
                text(&b, "path", i),
            ) else {
                continue;
            };
            out.entry(seed)
                .or_default()
                .push((site, module, role, path));
        }
    }
    Ok(out)
}

async fn modules(
    ctx: &SessionContext,
    ids: &BTreeSet<Id>,
) -> Result<BTreeMap<Id, Module>, CoreError> {
    let list = hex_list(ids.iter().copied());
    let mut out: BTreeMap<Id, Module> = BTreeMap::new();
    for b in rows(
        ctx,
        &format!(
            "SELECT module_node_id, node_id, parent_node_id, kind, field, ordinal, start_byte, \
                    end_byte FROM syntax_nodes WHERE module_node_id IN ({list})"
        ),
        &[
            ("module_node_id", ID),
            ("node_id", ID),
            ("parent_node_id", ID),
            ("kind", DataType::Int16),
            ("field", DataType::Int16),
            ("ordinal", DataType::Int64),
            ("start_byte", DataType::Int64),
            ("end_byte", DataType::Int64),
        ],
    )
    .await?
    {
        for i in 0..b.num_rows() {
            let (Some(m), Some(n)) = (id(&b, "module_node_id", i), id(&b, "node_id", i)) else {
                continue;
            };
            out.entry(m).or_default().nodes.insert(
                n,
                Node {
                    parent: id(&b, "parent_node_id", i),
                    kind: small(&b, "kind", i).unwrap_or(-1),
                    field: small(&b, "field", i).unwrap_or(-1),
                    ordinal: int(&b, "ordinal", i).unwrap_or(0),
                    start: int(&b, "start_byte", i).unwrap_or(0) as usize,
                    end: int(&b, "end_byte", i).unwrap_or(0) as usize,
                },
            );
        }
    }
    for b in rows(
        ctx,
        &format!(
            "SELECT module_node_id, node_id, kind, site_node_id, start_byte, end_byte \
             FROM bindings WHERE module_node_id IN ({list})"
        ),
        &[
            ("module_node_id", ID),
            ("node_id", ID),
            ("kind", DataType::Int16),
            ("site_node_id", ID),
            ("start_byte", DataType::Int64),
            ("end_byte", DataType::Int64),
        ],
    )
    .await?
    {
        for i in 0..b.num_rows() {
            let (Some(m), Some(n), Some(kind), Some(site)) = (
                id(&b, "module_node_id", i),
                id(&b, "node_id", i),
                small(&b, "kind", i).and_then(BindingKind::from_code),
                id(&b, "site_node_id", i),
            ) else {
                continue;
            };
            out.entry(m).or_default().bindings.insert(
                n,
                Binding {
                    kind,
                    site,
                    span: (
                        int(&b, "start_byte", i).unwrap_or(0) as usize,
                        int(&b, "end_byte", i).unwrap_or(0) as usize,
                    ),
                },
            );
        }
    }
    for b in rows(
        ctx,
        &format!(
            "SELECT rf.module_node_id, rf.start_byte, rf.end_byte, rr.binding_id, \
                    rr.builtin_name \
             FROM references rf JOIN reference_resolutions rr ON rr.reference_id = rf.node_id \
             WHERE rf.module_node_id IN ({list})"
        ),
        &[
            ("module_node_id", ID),
            ("start_byte", DataType::Int64),
            ("end_byte", DataType::Int64),
            ("binding_id", ID),
            ("builtin_name", DataType::Utf8),
        ],
    )
    .await?
    {
        for i in 0..b.num_rows() {
            let Some(m) = id(&b, "module_node_id", i) else {
                continue;
            };
            let read = match (id(&b, "binding_id", i), text(&b, "builtin_name", i)) {
                (Some(binding), _) => Read::Binding(binding),
                (None, Some(_)) => Read::Builtin,
                (None, None) => Read::Unresolved,
            };
            out.entry(m).or_default().reads.push((
                int(&b, "start_byte", i).unwrap_or(0) as usize,
                int(&b, "end_byte", i).unwrap_or(0) as usize,
                read,
            ));
        }
    }
    for b in rows(
        ctx,
        &format!(
            "SELECT module_node_id, node_id, start_byte, end_byte FROM call_syntax \
             WHERE module_node_id IN ({list})"
        ),
        &[
            ("module_node_id", ID),
            ("node_id", ID),
            ("start_byte", DataType::Int64),
            ("end_byte", DataType::Int64),
        ],
    )
    .await?
    {
        for i in 0..b.num_rows() {
            if let (Some(m), Some(n)) = (id(&b, "module_node_id", i), id(&b, "node_id", i)) {
                out.entry(m).or_default().calls.push((
                    n,
                    int(&b, "start_byte", i).unwrap_or(0) as usize,
                    int(&b, "end_byte", i).unwrap_or(0) as usize,
                ));
            }
        }
    }
    for b in rows(
        ctx,
        &format!("SELECT module_node_id, text FROM source_files WHERE module_node_id IN ({list})"),
        &[("module_node_id", ID), ("text", DataType::Utf8)],
    )
    .await?
    {
        for i in 0..b.num_rows() {
            if let (Some(m), Some(t)) = (id(&b, "module_node_id", i), text(&b, "text", i)) {
                out.entry(m).or_default().text = t;
            }
        }
    }
    Ok(out)
}

/// A statement cut from its line start and dedented by its own indentation; `None` when another
/// statement shares its first line.
fn dedented(source: &str, start: usize, end: usize) -> Option<String> {
    let line_start = source.get(..start)?.rfind('\n').map_or(0, |i| i + 1);
    let lead = source.get(line_start..start)?;
    if !lead.chars().all(|c| c == ' ' || c == '\t') {
        return None;
    }
    let width = lead.len();
    let body = source.get(line_start..end)?;
    Some(
        body.split('\n')
            .map(|line| {
                let spaces = line.len() - line.trim_start_matches([' ', '\t']).len();
                &line[spaces.min(width)..]
            })
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

/// The pattern around one site, or `None` when it is not self-contained or does not parse.
fn build(module: &Module, module_id: Id, site: Id, statements: &BTreeSet<i16>) -> Option<Vec<Id>> {
    let root = module.statement(site, statements)?;
    let block = {
        let n = &module.nodes[&root];
        (n.parent, n.field)
    };
    // A module or function body only: a header the pattern would drop (`with pytest.raises`, an
    // `if`, a loop, a `try`) changes what the statement means (slice 2.2 review F1).
    if let Some(parent) = block.0.and_then(|p| module.nodes.get(&p))
        && !(parent.kind == SyntaxKind::StmtFunctionDef.code()
            && block.1 == SyntaxField::Body.code())
    {
        return None;
    }
    let root_start = module.nodes[&root].start;
    let mut chosen: BTreeSet<Id> = BTreeSet::from([root]);
    let mut queue = vec![root];
    while let Some(statement) = queue.pop() {
        let s = &module.nodes[&statement];
        for &(a, z, binding) in &module.reads {
            if a < s.start || z > s.end {
                continue;
            }
            let binding = match binding {
                Read::Binding(b) => b,
                Read::Builtin => continue,
                Read::Unresolved => return None,
            };
            let b = module.bindings.get(&binding)?;
            if b.kind == BindingKind::Implicit {
                continue; // a module global such as `__name__`
            }
            let site = module.nodes.get(&b.site);
            // Bound inside a chosen statement (a def's own parameters and locals).
            if let Some(n) = site
                && chosen.iter().any(|c| {
                    let c = &module.nodes[c];
                    c.start <= n.start && n.end <= c.end
                })
            {
                continue;
            }
            let owner = match site {
                Some(_) => module.statement(b.site, statements)?,
                None if matches!(b.kind, BindingKind::Import | BindingKind::FromImport) => {
                    module.statement_at(b.span, statements)?
                }
                None => return None,
            };
            let o = &module.nodes[&owner];
            let import = matches!(b.kind, BindingKind::Import | BindingKind::FromImport);
            let same_block = (o.parent, o.field) == block && o.start < root_start;
            if !(import || same_block) {
                return None;
            }
            if chosen.insert(owner) {
                queue.push(owner);
            }
            if chosen.len() > MAX_STATEMENTS {
                return None;
            }
        }
    }
    let _ = module_id;
    let mut ordered: Vec<Id> = chosen.into_iter().collect();
    ordered.sort_by_key(|n| (module.nodes[n].start, module.nodes[n].ordinal));
    Some(ordered)
}

/// Each seed's usage pattern, if one qualifies. `preferred` lists, per seed, its handoff
/// occurrences as (producer site, consumer site): their sites are tried first within their role,
/// and a pattern holding a whole occurrence wins within its role (slice 2.2 review F3).
pub async fn patterns(
    ctx: &SessionContext,
    seeds: &[Id],
    preferred: &BTreeMap<Id, Vec<(Id, Id)>>,
) -> Result<BTreeMap<Id, Pattern>, CoreError> {
    let statement_kinds: BTreeSet<i16> = SyntaxKind::all()
        .iter()
        .filter(|k| k.text().starts_with("stmt_"))
        .map(|k| k.code())
        .collect();
    let mut chosen: BTreeMap<Id, Vec<(Id, Id, SourceRole, String)>> = BTreeMap::new();
    for (seed, mut sites) in candidates(ctx, seeds).await? {
        let rank = |r: SourceRole| crate_rank(r);
        let first: BTreeSet<Id> = preferred
            .get(&seed)
            .into_iter()
            .flatten()
            .flat_map(|(p, c)| [*p, *c])
            .collect();
        sites.sort_by_key(|(site, _, role, _)| (rank(*role), !first.contains(site)));
        sites.truncate(MAX_CANDIDATES);
        chosen.insert(seed, sites);
    }
    let wanted: BTreeSet<Id> = chosen.values().flatten().map(|c| c.1).collect();
    let modules = modules(ctx, &wanted).await?;
    let minor = python_minor(ctx).await?;
    let mut out = BTreeMap::new();
    for (seed, sites) in chosen {
        let occurrences: &[(Id, Id)] = preferred.get(&seed).map_or(&[], Vec::as_slice);
        // (role, shows no whole handoff, size): the best role first, a pattern showing a handoff
        // within it, then the smallest.
        let mut best: Option<((u8, bool, usize), Pattern)> = None;
        for (site, module_id, role, path) in sites {
            let Some(module) = modules.get(&module_id) else {
                continue;
            };
            let Some(nodes) = build(module, module_id, site, &statement_kinds) else {
                continue;
            };
            let mut statements = Vec::new();
            let mut parts = Vec::new();
            let mut ok = true;
            for n in &nodes {
                let node = &module.nodes[n];
                let (Some(verbatim), Some(code)) = (
                    module.text.get(node.start..node.end),
                    dedented(&module.text, node.start, node.end),
                ) else {
                    ok = false;
                    break;
                };
                statements.push(PatternStatement {
                    node: *n,
                    module: module_id,
                    span: (node.start, node.end),
                    text: verbatim.to_owned(),
                });
                parts.push(code);
            }
            if !ok {
                continue;
            }
            let code = parts.join("\n");
            if free_names(&code, minor).is_none_or(|free| !free.is_empty()) {
                continue;
            }
            let sites: BTreeSet<Id> = module
                .calls
                .iter()
                .filter(|(_, a, z)| {
                    statements
                        .iter()
                        .any(|st| st.span.0 <= *a && *z <= st.span.1)
                })
                .map(|(n, _, _)| *n)
                .collect();
            let shows = occurrences
                .iter()
                .any(|(p, c)| sites.contains(p) && sites.contains(c));
            let key = (crate_rank(role), !shows, code.len());
            if best.as_ref().is_none_or(|b| key < b.0) {
                best = Some((
                    key,
                    Pattern {
                        role,
                        path,
                        site,
                        statements,
                        code,
                        sites,
                    },
                ));
            }
        }
        if let Some((_, p)) = best {
            out.insert(seed, p);
        }
    }
    Ok(out)
}

/// The names `code` loads, annotations included, that nothing in it binds or imports and that
/// are not builtins of Python 3.`minor` (slice 2.2 review F1). Scopes are not told apart: a name
/// bound anywhere in the code counts as bound. `None` when it does not parse; a star import makes
/// every name possibly bound, so nothing is free.
pub fn free_names(code: &str, minor: u8) -> Option<BTreeSet<String>> {
    use ruff_python_ast::visitor::{
        Visitor, walk_except_handler, walk_expr, walk_parameter, walk_pattern, walk_stmt,
    };
    use ruff_python_ast::{ExceptHandler, Expr, ExprContext, Parameter, Pattern, Stmt};

    #[derive(Default)]
    struct Names {
        loaded: BTreeSet<String>,
        bound: BTreeSet<String>,
        star: bool,
    }
    impl<'a> Visitor<'a> for Names {
        fn visit_stmt(&mut self, stmt: &'a Stmt) {
            match stmt {
                Stmt::FunctionDef(f) => {
                    self.bound.insert(f.name.to_string());
                }
                Stmt::ClassDef(c) => {
                    self.bound.insert(c.name.to_string());
                }
                Stmt::Import(i) => {
                    for a in &i.names {
                        let bound = a.asname.as_ref().map_or_else(
                            || a.name.split('.').next().unwrap_or_default().to_owned(),
                            ToString::to_string,
                        );
                        self.bound.insert(bound);
                    }
                }
                Stmt::ImportFrom(i) => {
                    for a in &i.names {
                        if a.name.as_str() == "*" {
                            self.star = true;
                        }
                        self.bound
                            .insert(a.asname.as_ref().unwrap_or(&a.name).to_string());
                    }
                }
                _ => {}
            }
            walk_stmt(self, stmt);
        }
        fn visit_expr(&mut self, expr: &'a Expr) {
            if let Expr::Name(n) = expr {
                let name = n.id.to_string();
                if matches!(n.ctx, ExprContext::Load) {
                    self.loaded.insert(name);
                } else {
                    self.bound.insert(name);
                }
            }
            walk_expr(self, expr);
        }
        fn visit_parameter(&mut self, parameter: &'a Parameter) {
            self.bound.insert(parameter.name.to_string());
            walk_parameter(self, parameter);
        }
        fn visit_except_handler(&mut self, handler: &'a ExceptHandler) {
            let ExceptHandler::ExceptHandler(h) = handler;
            if let Some(name) = &h.name {
                self.bound.insert(name.to_string());
            }
            walk_except_handler(self, handler);
        }
        fn visit_pattern(&mut self, pattern: &'a Pattern) {
            match pattern {
                Pattern::MatchAs(p) => self.bound.extend(p.name.iter().map(ToString::to_string)),
                Pattern::MatchStar(p) => self.bound.extend(p.name.iter().map(ToString::to_string)),
                Pattern::MatchMapping(p) => {
                    self.bound.extend(p.rest.iter().map(ToString::to_string));
                }
                _ => {}
            }
            walk_pattern(self, pattern);
        }
    }
    let parsed = ruff_python_parser::parse_module(code).ok()?;
    let mut names = Names::default();
    for stmt in &parsed.syntax().body {
        names.visit_stmt(stmt);
    }
    if names.star {
        return Some(BTreeSet::new());
    }
    let magic: BTreeSet<&str> = ruff_python_stdlib::builtins::python_magic_globals(minor).collect();
    Some(
        names
            .loaded
            .into_iter()
            .filter(|n| {
                !names.bound.contains(n)
                    && !magic.contains(n.as_str())
                    && !ruff_python_stdlib::builtins::is_python_builtin(n, minor, false)
            })
            .collect(),
    )
}

/// The analyzed Python's least minor version (the contexts' `3.x`), for the builtins.
async fn python_minor(ctx: &SessionContext) -> Result<u8, CoreError> {
    let mut least: Option<u8> = None;
    for b in rows(
        ctx,
        "SELECT DISTINCT python_version FROM contexts",
        &[("python_version", DataType::Utf8)],
    )
    .await?
    {
        for i in 0..b.num_rows() {
            if let Some(minor) = text(&b, "python_version", i)
                .and_then(|v| v.split('.').nth(1).and_then(|m| m.parse::<u8>().ok()))
            {
                least = Some(least.map_or(minor, |l| l.min(minor)));
            }
        }
    }
    Ok(least.unwrap_or(10))
}

/// Official examples first, then doc blocks, then tests (Pass C's order).
fn crate_rank(role: SourceRole) -> u8 {
    lctx_analytics::pass_c::role_rank(role)
}

#[cfg(test)]
mod tests {
    use super::{dedented, free_names};

    /// Slice 2.2 review F1: an annotation's name counts, a builtin does not, an import binds.
    #[test]
    fn free_names_include_annotations_and_skip_builtins() {
        let dropped = "@mcp.custom_route(\"/health\")\nasync def health(request: Request):\n    return len([])";
        let free = free_names(dropped, 14).unwrap();
        assert_eq!(free.into_iter().collect::<Vec<_>>(), ["Request", "mcp"]);
        let whole = "from starlette.requests import Request\nfrom fastmcp import FastMCP\nmcp = FastMCP()\n@mcp.custom_route(\"/health\")\nasync def health(request: Request):\n    return print(__name__)";
        assert!(free_names(whole, 14).unwrap().is_empty());
        assert!(free_names("from x import *\nf(y)", 14).unwrap().is_empty());
        assert_eq!(free_names("def (", 14), None);
    }

    #[test]
    fn a_statement_is_dedented_by_its_own_indentation() {
        let src = "def f():\n    x = g(\n        1,\n    )\n    y = 2; z = 3\n";
        let start = src.find("x =").unwrap();
        let end = src.find(")\n").unwrap() + 1;
        assert_eq!(dedented(src, start, end).unwrap(), "x = g(\n    1,\n)");
        let z = src.find("z =").unwrap();
        assert_eq!(dedented(src, z, z + 5), None, "shares its line");
    }
}
