//! Stage 2.6 of the behavior model (ADR-0022; DESIGN §3.9): what the flow IR says about the
//! release, as analysis tables the behavior scan and the tools read.
//!
//! - **Parameter reach.** A use's sources are the parameters whose value reaches it: through its
//!   reaching definitions, then each definition's value sources, to a parameter's definition, each
//!   step **identity** or derived, under the conjunction of the conditions met on the way
//!   (reachability at each use, the selecting condition inside each value). A lambda's or
//!   comprehension's read of an enclosing function's parameter is followed through our
//!   resolution, flow-insensitively (`captured`).
//! - **`value_flows`**: per sink (a call argument, a `return`, a `raise`, a `yield`, a stored field
//!   or dict entry) the parameters reaching it.
//! - **`field_accesses`**: every `x.f` read or written, by the field's name.
//! - **`ambient_reads`**: reads of a module-global singleton's fields, at the resolved key
//!   (`Global[module.name]`, ADR-0022 §Places), with the read phase.
//! - **`dynamic_accesses`**: the getattr family and its kin, and what they reach under the model.
//! - **`negative_premises`**: per parameter, field and setting, whether "never read" can be
//!   refuted under the model.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::rc::Rc;

use cpg_schema::behavior::{
    AmbientReadsRow, DynamicAccessesRow, FieldAccessesRow, NegativePremisesRow, RaiseSitesRow,
    SingletonsRow, ValueFlowsRow,
};
use cpg_schema::codebook::{
    BindingKind, BoundaryReason, Codebook, DeclarationKind, DynamicKind, EdgeKind, FlowSink,
    LexicalScopeKind, PremiseKind, ReadPhase, SourceRole, SyntaxKind,
};
use cpg_schema::condition::Condition;
use cpg_schema::id::{Id, IdHasher};
use datafusion::prelude::SessionContext;

use crate::CoreError;
use crate::sql;

cpg_schema::query_row! {
    struct UseRow {
        use_id: Id,
        module_node_id: Id,
        place: String,
        scope_kind: LexicalScopeKind,
        start_byte: i64,
        end_byte: i64,
        function_node_id: Option<Id>,
    }
}

cpg_schema::query_row! {
    struct DefRow {
        definition_id: Id,
        module_node_id: Id,
        place: String,
        kind: BindingKind,
        scope_kind: LexicalScopeKind,
        start_byte: i64,
        end_byte: i64,
        value_start_byte: Option<i64>,
        value_end_byte: Option<i64>,
        function_node_id: Option<Id>,
        parameter_node_id: Option<Id>,
    }
}

cpg_schema::query_row! {
    struct ReachRow {
        use_id: Id,
        definition_id: Option<Id>,
        condition_id: Id,
    }
}

cpg_schema::query_row! {
    struct ValueRow {
        module_node_id: Id,
        sink: FlowSink,
        sink_start_byte: i64,
        sink_end_byte: i64,
        use_id: Id,
        identity: bool,
        condition_id: Id,
    }
}

cpg_schema::query_row! {
    struct ConditionRow {
        condition_id: Id,
        encoding: String,
    }
}

cpg_schema::query_row! {
    struct RegionRow {
        module_node_id: Id,
        start_byte: i64,
        end_byte: i64,
        condition_id: Id,
    }
}

cpg_schema::query_row! {
    struct ArgumentRow {
        argument_node_id: Id,
        call_node_id: Id,
        module_node_id: Id,
        start_byte: i64,
        end_byte: i64,
    }
}

cpg_schema::query_row! {
    struct CapturedRow {
        module_node_id: Id,
        start_byte: i64,
        end_byte: i64,
        parameter_node_id: Id,
    }
}

cpg_schema::query_row! {
    struct ParameterRow {
        node_id: Id,
        function_node_id: Id,
        name: String,
        ordinal: i64,
    }
}

cpg_schema::query_row! {
    struct FunctionRow {
        node_id: Id,
        module_node_id: Id,
        name: String,
        qualified_name: String,
        kind: DeclarationKind,
        parent_node_id: Option<Id>,
        parent_kind: Option<DeclarationKind>,
    }
}

cpg_schema::query_row! {
    struct ReceiverRow {
        parameter_node_id: Id,
        function_node_id: Id,
    }
}

cpg_schema::query_row! {
    struct NameRootRow {
        module_node_id: Id,
        start_byte: i64,
        end_byte: i64,
        binding_kind: BindingKind,
        binding_name: String,
        binding_module_node_id: Id,
        binding_scope_kind: LexicalScopeKind,
        imported_module: Option<String>,
        imported_name: Option<String>,
        builtin_name: Option<String>,
    }
}

cpg_schema::query_row! {
    struct ModuleRow {
        module_node_id: Id,
        module_name: String,
        text: Option<String>,
    }
}

cpg_schema::query_row! {
    struct CallRow {
        node_id: Id,
        module_node_id: Id,
        owner_node_id: Option<Id>,
        start_byte: i64,
        end_byte: i64,
        callee_start_byte: i64,
        callee_end_byte: i64,
    }
}

cpg_schema::query_row! {
    struct AncestryRow {
        class_node_id: Id,
        ancestor_node_id: Id,
    }
}

cpg_schema::query_row! {
    struct UnreadRow {
        parameter_node_id: Id,
        function_node_id: Id,
        name: String,
        module_node_id: Id,
    }
}

cpg_schema::query_row! {
    struct CoveredRow {
        module_node_id: Id,
    }
}

cpg_schema::query_row! {
    struct RaiseRow {
        module_node_id: Id,
        owner_node_id: Id,
        start_byte: i64,
        end_byte: i64,
    }
}

/// The release modules' scope-naming declaration, joined by the scope's name span.
fn scope_join(alias: &str) -> String {
    format!(
        "LEFT JOIN declarations sd ON sd.module_node_id = {alias}.module_node_id \
           AND sd.name_start_byte = {alias}.scope_start_byte \
           AND sd.name_end_byte = {alias}.scope_end_byte"
    )
}

fn release_modules() -> String {
    format!(
        "SELECT module_node_id FROM source_files WHERE role = {}",
        SourceRole::Release.code()
    )
}

cpg_schema::relations! {
    inventory relations;
    /// Every place load outside annotations, with the function naming its scope.
    uses = "flow_model_uses",
        deps = ["flow_uses", "declarations"],
        sql = format!(
            "SELECT u.use_id, u.module_node_id, u.place, u.scope_kind, u.start_byte, u.end_byte, \
                    sd.node_id AS function_node_id \
             FROM flow_uses u {join} WHERE NOT u.annotation ORDER BY u.use_id",
            join = scope_join("u"),
        );
    /// Every definition, with its function and, for a parameter, the parameter's node.
    definitions = "flow_model_definitions",
        deps = ["flow_definitions", "declarations", "bindings"],
        sql = format!(
            "SELECT d.definition_id, d.module_node_id, d.place, d.kind, d.scope_kind, \
                    d.start_byte, d.end_byte, d.value_start_byte, d.value_end_byte, \
                    sd.node_id AS function_node_id, b.site_node_id AS parameter_node_id \
             FROM flow_definitions d {join} \
             LEFT JOIN bindings b ON b.module_node_id = d.module_node_id \
               AND b.start_byte = d.start_byte AND b.end_byte = d.end_byte \
               AND b.kind = {parameter} AND d.kind = {parameter} \
             ORDER BY d.definition_id",
            join = scope_join("d"),
            parameter = BindingKind::Parameter.code(),
        );
    reaching = "flow_model_reaching",
        deps = ["flow_reaching"],
        sql = "SELECT use_id, definition_id, condition_id FROM flow_reaching \
               ORDER BY use_id, definition_id, condition_id"
            .to_owned();
    values = "flow_model_values",
        deps = ["flow_values"],
        sql = "SELECT module_node_id, sink, sink_start_byte, sink_end_byte, use_id, identity, \
                      condition_id FROM flow_values \
               ORDER BY module_node_id, sink_start_byte, sink_end_byte, use_id, condition_id"
            .to_owned();
    conditions = "flow_model_conditions",
        deps = ["conditions"],
        sql = "SELECT DISTINCT condition_id, encoding FROM conditions ORDER BY condition_id"
            .to_owned();
    regions = "flow_model_regions",
        deps = ["flow_regions"],
        sql = "SELECT module_node_id, start_byte, end_byte, condition_id FROM flow_regions \
               ORDER BY module_node_id, start_byte, end_byte"
            .to_owned();
    /// Every call argument's value span.
    arguments = "flow_model_arguments",
        deps = ["arguments", "edges", "syntax_nodes"],
        sql = format!(
            "SELECT a.node_id AS argument_node_id, a.call_node_id, s.module_node_id, \
                    s.start_byte, s.end_byte \
             FROM arguments a \
             JOIN edges v ON v.edge_kind = {argument_value} AND v.src_node_id = a.node_id \
             JOIN syntax_nodes s ON s.node_id = v.dst_node_id \
             ORDER BY a.node_id",
            argument_value = EdgeKind::ArgumentValue.code(),
        );
    /// Names read in a lambda or comprehension that our resolution binds to an enclosing
    /// function's parameter.
    captured = "flow_model_captured",
        deps = ["references", "reference_resolutions", "bindings"],
        sql = format!(
            "SELECT r.module_node_id, r.start_byte, r.end_byte, b.site_node_id AS parameter_node_id \
             FROM references r \
             JOIN reference_resolutions rr ON rr.reference_id = r.node_id AND rr.captured \
             JOIN bindings b ON b.node_id = rr.binding_id AND b.kind = {parameter} \
             ORDER BY 1, 2, 3, 4",
            parameter = BindingKind::Parameter.code(),
        );
    parameters = "flow_model_parameters",
        deps = ["parameter_syntax"],
        sql = "SELECT node_id, function_node_id, name, ordinal FROM parameter_syntax \
               ORDER BY node_id"
            .to_owned();
    functions = "flow_model_functions",
        deps = ["declarations"],
        sql = "SELECT d.node_id, d.module_node_id, d.name, d.qualified_name, d.kind, \
                      d.parent_node_id, p.kind AS parent_kind \
               FROM declarations d LEFT JOIN declarations p ON p.node_id = d.parent_node_id \
               ORDER BY d.node_id"
            .to_owned();
    receivers = "flow_model_receivers",
        deps = ["parameter_syntax", "provider_node_map", "pysa_functions"],
        sql = cpg_schema::flows::receivers_sql();
    /// Each name reference with the binding our resolution gives it, and an import's source.
    roots = "flow_model_roots",
        deps = ["references", "reference_resolutions", "bindings", "scopes", "export_syntax"],
        sql = format!(
            "SELECT r.module_node_id, r.start_byte, r.end_byte, b.kind AS binding_kind, \
                    b.name AS binding_name, b.module_node_id AS binding_module_node_id, \
                    sc.kind AS binding_scope_kind, \
                    COALESCE(e.resolved_module, e.imported_module) AS imported_module, \
                    e.imported_name, rr.builtin_name \
             FROM references r \
             JOIN ({release}) m ON m.module_node_id = r.module_node_id \
             JOIN reference_resolutions rr ON rr.reference_id = r.node_id \
             JOIN bindings b ON b.node_id = rr.binding_id \
             JOIN scopes sc ON sc.node_id = b.scope_id \
             LEFT JOIN export_syntax e ON e.node_id = b.site_node_id \
             UNION ALL \
             SELECT r.module_node_id, r.start_byte, r.end_byte, {implicit} AS binding_kind, \
                    rr.builtin_name AS binding_name, r.module_node_id AS binding_module_node_id, \
                    {module_scope} AS binding_scope_kind, CAST(NULL AS VARCHAR), \
                    CAST(NULL AS VARCHAR), rr.builtin_name \
             FROM references r \
             JOIN ({release}) m ON m.module_node_id = r.module_node_id \
             JOIN reference_resolutions rr ON rr.reference_id = r.node_id \
             WHERE rr.builtin_name IS NOT NULL \
             ORDER BY 1, 2, 3, 4, 5",
            release = release_modules(),
            implicit = BindingKind::Implicit.code(),
            module_scope = LexicalScopeKind::Module.code(),
        );
    modules = "flow_model_modules",
        deps = ["source_files"],
        sql = format!(
            "SELECT module_node_id, module_name, text FROM source_files WHERE role = {} \
             ORDER BY module_node_id",
            SourceRole::Release.code()
        );
    calls = "flow_model_calls",
        deps = ["call_syntax", "source_files"],
        sql = format!(
            "SELECT c.node_id, c.module_node_id, c.owner_node_id, c.start_byte, c.end_byte, \
                    c.callee_start_byte, c.callee_end_byte \
             FROM call_syntax c JOIN ({release}) m ON m.module_node_id = c.module_node_id \
             ORDER BY c.node_id",
            release = release_modules(),
        );
    /// A class's ancestors, by our MRO edges.
    ancestry = "flow_model_ancestry",
        deps = ["edges"],
        sql = format!(
            "SELECT src_node_id AS class_node_id, dst_node_id AS ancestor_node_id FROM edges \
             WHERE edge_kind = {ancestor} ORDER BY 1, 2",
            ancestor = EdgeKind::MroEntry.code(),
        );
    /// Parameters no reference resolves to (closures included), receivers aside.
    unread = "flow_model_unread",
        deps = ["bindings", "reference_resolutions", "parameter_syntax"],
        sql = format!(
            "SELECT b.site_node_id AS parameter_node_id, ps.function_node_id, b.name, \
                    b.module_node_id \
             FROM bindings b JOIN parameter_syntax ps ON ps.node_id = b.site_node_id \
             LEFT ANTI JOIN reference_resolutions rr ON rr.binding_id = b.node_id \
             WHERE b.kind = {parameter} ORDER BY 1",
            parameter = BindingKind::Parameter.code(),
        );
    /// `raise` statements in release functions.
    raises = "flow_model_raises",
        deps = ["syntax_nodes", "source_files"],
        sql = format!(
            "SELECT s.module_node_id, s.owner_node_id, s.start_byte, s.end_byte \
             FROM syntax_nodes s JOIN ({release}) m ON m.module_node_id = s.module_node_id \
             WHERE s.kind = {raise} AND s.owner_node_id IS NOT NULL ORDER BY 1, 3, 4",
            release = release_modules(),
            raise = SyntaxKind::StmtRaise.code(),
        );
    /// Release modules the flow provider indexed completely.
    flow_complete = "flow_model_complete",
        deps = ["coverage"],
        sql = format!(
            "SELECT DISTINCT scope_node_id AS module_node_id FROM coverage \
             WHERE fact_family = {flow} AND status = {complete} ORDER BY 1",
            flow = cpg_schema::codebook::FactFamily::Flow.code(),
            complete = cpg_schema::codebook::CoverageStatus::CompleteUnderStatedModel.code(),
        );
}

/// The model's digest: its relations, for the compiler digest.
pub fn digest() -> cpg_schema::id::Digest {
    let mut h = IdHasher::new("flow-model");
    for r in relations() {
        h.str(r.name).str(&r.sql);
    }
    h.finish_digest()
}

/// What Stage 2.6 writes, and what the behavior scan reads from it.
#[derive(Debug, Default)]
pub struct FlowModelRows {
    pub value_flows: Vec<ValueFlowsRow>,
    pub field_accesses: Vec<FieldAccessesRow>,
    pub ambient_reads: Vec<AmbientReadsRow>,
    pub dynamic_accesses: Vec<DynamicAccessesRow>,
    pub raise_sites: Vec<RaiseSitesRow>,
    pub singletons: Vec<SingletonsRow>,
    pub premises: Vec<NegativePremisesRow>,
    /// Each release call site's callee as written (not persisted: `call_syntax` spans hold it).
    pub callee_text: BTreeMap<Id, String>,
    /// Each class's relatives: its MRO ancestors and the classes that have it as one.
    pub relatives: BTreeMap<Id, BTreeSet<Id>>,
    /// Each method's class, and each declaration's qualified name.
    pub method_class: BTreeMap<Id, Id>,
    pub qualified: BTreeMap<Id, String>,
}

/// Where a value comes from: a parameter, or a field of a method's receiver read with no local
/// definition (the value any method of a relative stored there).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Origin {
    Parameter(Id),
    Field(Id, String),
}

/// One origin reaching a use.
#[derive(Clone, Debug)]
struct Source {
    identity: bool,
    captured: bool,
    condition: Condition,
}

type Sources = BTreeMap<(Origin, bool), Source>;

fn merge(into: &mut Sources, origin: Origin, s: Source) {
    let key = (origin, s.identity);
    match into.get_mut(&key) {
        Some(e) => {
            e.condition = e.condition.or(&s.condition);
            e.captured |= s.captured;
        }
        None => {
            into.insert(key, s);
        }
    }
}

/// Plain definitions pass a value unchanged; loop, `with` and comprehension targets and
/// augmented assignments compute it.
fn plain(kind: BindingKind) -> bool {
    matches!(
        kind,
        BindingKind::Assignment | BindingKind::Walrus | BindingKind::Parameter
    )
}

struct Model {
    uses: HashMap<Id, UseRow>,
    defs: HashMap<Id, DefRow>,
    reaching: HashMap<Id, Vec<(Option<Id>, Id)>>,
    values: ValueSources,
    conditions: HashMap<Id, Condition>,
    captured: HashMap<(Id, i64, i64), Vec<Id>>,
    /// Receiver parameters: never an origin of their own.
    receivers: HashSet<Id>,
    /// A use naming `<receiver>.<field>` in a method: its class and field.
    fields: HashMap<Id, (Id, String)>,
    memo: HashMap<(Id, bool), Rc<Sources>>,
    /// Receivers count as origins (the dynamic-access check reads through local copies of
    /// `self`).
    keep_receivers: bool,
}

impl Model {
    fn condition(&self, id: Id) -> Condition {
        self.conditions
            .get(&id)
            .cloned()
            .unwrap_or(Condition::OverBudget)
    }

    /// The parameters reaching a use (see the module docs).
    fn reach(&mut self, u: Id, visiting: &mut HashSet<Id>) -> Rc<Sources> {
        if let Some(s) = self.memo.get(&(u, self.keep_receivers)) {
            return s.clone();
        }
        if !visiting.insert(u) {
            return Rc::new(Sources::new());
        }
        let mut out = Sources::new();
        let rows = self.reaching.get(&u).cloned().unwrap_or_default();
        let this = self.uses.get(&u).cloned();
        for (def, condition_id) in rows {
            let at = self.condition(condition_id);
            let Some(def) = def else {
                // A receiver's field with no local definition: the field is the origin.
                if !self.keep_receivers
                    && let Some((class, field)) = self.fields.get(&u).cloned()
                {
                    merge(
                        &mut out,
                        Origin::Field(class, field),
                        Source {
                            identity: true,
                            captured: false,
                            condition: at,
                        },
                    );
                    continue;
                }
                // Unbound in its own scope: a lambda's or comprehension's read of an enclosing
                // function's parameter, by our resolution.
                if let Some(u) = &this
                    && matches!(
                        u.scope_kind,
                        LexicalScopeKind::Lambda | LexicalScopeKind::Comprehension
                    )
                {
                    for &p in self
                        .captured
                        .get(&(u.module_node_id, u.start_byte, u.end_byte))
                        .into_iter()
                        .flatten()
                    {
                        merge(
                            &mut out,
                            Origin::Parameter(p),
                            Source {
                                identity: true,
                                captured: true,
                                condition: Condition::always(),
                            },
                        );
                    }
                }
                continue;
            };
            let Some(d) = self.defs.get(&def).cloned() else {
                continue;
            };
            if let Some(p) = d.parameter_node_id {
                if self.keep_receivers || !self.receivers.contains(&p) {
                    merge(
                        &mut out,
                        Origin::Parameter(p),
                        Source {
                            identity: true,
                            captured: false,
                            condition: at,
                        },
                    );
                }
                continue;
            }
            let (Some(vs), Some(ve)) = (d.value_start_byte, d.value_end_byte) else {
                continue;
            };
            let inner = self
                .values
                .get(&(d.module_node_id, FlowSink::Definition, vs, ve))
                .cloned()
                .unwrap_or_default();
            for (u2, identity, c2) in inner {
                let selected = at.and(&self.condition(c2));
                let sources = self.reach(u2, visiting);
                for ((o, id3), s) in sources.iter() {
                    let id3 = *id3;
                    merge(
                        &mut out,
                        o.clone(),
                        Source {
                            identity: identity && id3 && plain(d.kind),
                            captured: s.captured,
                            condition: selected.and(&s.condition),
                        },
                    );
                }
            }
        }
        visiting.remove(&u);
        let out = Rc::new(out);
        self.memo.insert((u, self.keep_receivers), out.clone());
        out
    }

    /// The parameters reaching a sink: the union over the uses its value reads.
    fn sink(&mut self, key: (Id, FlowSink, i64, i64)) -> Sources {
        let mut out = Sources::new();
        for (u, identity, c) in self.values.get(&key).cloned().unwrap_or_default() {
            let selected = self.condition(c);
            let sources = self.reach(u, &mut HashSet::new());
            for ((o, id2), s) in sources.iter() {
                let id2 = *id2;
                merge(
                    &mut out,
                    o.clone(),
                    Source {
                        identity: identity && id2,
                        captured: s.captured,
                        condition: selected.and(&s.condition),
                    },
                );
            }
        }
        // A derived row adds nothing where the same origin reaches the sink unchanged.
        let unchanged: BTreeSet<Origin> = out
            .keys()
            .filter(|(_, identity)| *identity)
            .map(|(o, _)| o.clone())
            .collect();
        out.retain(|(o, identity), _| *identity || !unchanged.contains(o));
        out
    }
}

/// A sink: its module, kind and span.
type SinkKey = (Id, FlowSink, i64, i64);
/// A sink's value sources: the use, whether it passes unchanged, and the selecting condition.
type ValueSources = HashMap<SinkKey, Vec<(Id, bool, Id)>>;

/// A module's statement regions, for the innermost region around a span.
struct Regions(HashMap<Id, Vec<(i64, i64, Id)>>);

impl Regions {
    fn at(&self, module: Id, start: i64, end: i64) -> Option<Id> {
        self.0
            .get(&module)?
            .iter()
            .filter(|(s, e, _)| *s <= start && end <= *e)
            .min_by_key(|(s, e, _)| e - s)
            .map(|(_, _, c)| *c)
    }
}

/// A resolved root: a module, or a module's global.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Root {
    Module(String),
    Global(String, String),
}

/// Walk a dotted place from its root: a module's own module-level binding of a name wins over a
/// submodule of that name (a package's `settings = Settings()` shadows its `settings` module
/// attribute at runtime); otherwise a submodule; otherwise the first global.
fn resolve(
    root: Root,
    segments: &[&str],
    modules: &BTreeSet<String>,
    globals: &BTreeSet<(String, String)>,
) -> (Root, usize) {
    let mut at = root;
    let mut used = 0;
    while let Root::Module(m) = &at {
        let Some(s) = segments.get(used) else { break };
        let next = format!("{m}.{s}");
        at = if !globals.contains(&(m.clone(), (*s).to_owned())) && modules.contains(&next) {
            Root::Module(next)
        } else {
            Root::Global(m.clone(), (*s).to_owned())
        };
        used += 1;
    }
    (at, used)
}

fn text_of(texts: &HashMap<Id, String>, module: Id, start: i64, end: i64) -> Option<&str> {
    texts.get(&module)?.get(start as usize..end as usize)
}

fn line_of(texts: &HashMap<Id, String>, module: Id, byte: i64) -> i64 {
    texts.get(&module).map_or(0, |t| {
        1 + t.as_bytes()[..(byte as usize).min(t.len())]
            .iter()
            .filter(|&&b| b == b'\n')
            .count() as i64
    })
}

/// Stage 2.6 for one attempt.
pub async fn run(ctx: &SessionContext, snapshot_id: Id) -> Result<FlowModelRows, CoreError> {
    let p = sql::Params::new;
    let uses: Vec<UseRow> = sql::fetch(ctx, &uses(), p()).await?;
    let defs: Vec<DefRow> = sql::fetch(ctx, &definitions(), p()).await?;
    let reaching_rows: Vec<ReachRow> = sql::fetch(ctx, &reaching(), p()).await?;
    let value_rows: Vec<ValueRow> = sql::fetch(ctx, &values(), p()).await?;
    let condition_rows: Vec<ConditionRow> = sql::fetch(ctx, &conditions(), p()).await?;
    let region_rows: Vec<RegionRow> = sql::fetch(ctx, &regions(), p()).await?;
    let argument_rows: Vec<ArgumentRow> = sql::fetch(ctx, &arguments(), p()).await?;
    let captured_rows: Vec<CapturedRow> = sql::fetch(ctx, &captured(), p()).await?;
    let parameter_rows: Vec<ParameterRow> = sql::fetch(ctx, &parameters(), p()).await?;
    let function_rows: Vec<FunctionRow> = sql::fetch(ctx, &functions(), p()).await?;
    let receiver_rows: Vec<ReceiverRow> = sql::fetch(ctx, &receivers(), p()).await?;
    let root_rows: Vec<NameRootRow> = sql::fetch(ctx, &roots(), p()).await?;
    let module_rows: Vec<ModuleRow> = sql::fetch(ctx, &modules(), p()).await?;
    let call_rows: Vec<CallRow> = sql::fetch(ctx, &calls(), p()).await?;
    let ancestry_rows: Vec<AncestryRow> = sql::fetch(ctx, &ancestry(), p()).await?;
    let unread_rows: Vec<UnreadRow> = sql::fetch(ctx, &unread(), p()).await?;
    let raise_rows: Vec<RaiseRow> = sql::fetch(ctx, &raises(), p()).await?;
    let complete: BTreeSet<Id> = sql::fetch::<CoveredRow>(ctx, &flow_complete(), p())
        .await?
        .into_iter()
        .map(|r| r.module_node_id)
        .collect();

    let mut conditions = HashMap::new();
    for r in condition_rows {
        let c = Condition::parse(&r.encoding).map_err(CoreError::Analysis)?;
        conditions.insert(r.condition_id, c);
    }
    let mut reaching_map: HashMap<Id, Vec<(Option<Id>, Id)>> = HashMap::new();
    for r in reaching_rows {
        reaching_map
            .entry(r.use_id)
            .or_default()
            .push((r.definition_id, r.condition_id));
    }
    let mut values_map: ValueSources = HashMap::new();
    for r in &value_rows {
        values_map
            .entry((r.module_node_id, r.sink, r.sink_start_byte, r.sink_end_byte))
            .or_default()
            .push((r.use_id, r.identity, r.condition_id));
    }
    let mut captured_map: HashMap<(Id, i64, i64), Vec<Id>> = HashMap::new();
    for r in captured_rows {
        captured_map
            .entry((r.module_node_id, r.start_byte, r.end_byte))
            .or_default()
            .push(r.parameter_node_id);
    }
    let receiver_set: HashSet<Id> = receiver_rows.iter().map(|r| r.parameter_node_id).collect();
    let receiver_name_of: HashMap<Id, &str> = receiver_rows
        .iter()
        .filter_map(|r| {
            parameter_rows
                .iter()
                .find(|p| p.node_id == r.parameter_node_id)
                .map(|p| (r.function_node_id, p.name.as_str()))
        })
        .collect();
    let class_of_function: HashMap<Id, Id> = function_rows
        .iter()
        .filter(|f| f.parent_kind == Some(DeclarationKind::Class))
        .filter_map(|f| f.parent_node_id.map(|c| (f.node_id, c)))
        .collect();
    let mut field_uses: HashMap<Id, (Id, String)> = HashMap::new();
    for u in &uses {
        let Some(f) = u.function_node_id else {
            continue;
        };
        let (Some(receiver), Some(class)) = (receiver_name_of.get(&f), class_of_function.get(&f))
        else {
            continue;
        };
        if let Some(field) = u.place.strip_prefix(&format!("{receiver}."))
            && !field.contains('.')
            && !field.contains('[')
        {
            field_uses.insert(u.use_id, (*class, field.to_owned()));
        }
    }
    let mut model = Model {
        uses: uses.iter().map(|u| (u.use_id, u.clone())).collect(),
        defs: defs.iter().map(|d| (d.definition_id, d.clone())).collect(),
        reaching: reaching_map,
        values: values_map,
        conditions,
        captured: captured_map,
        receivers: receiver_set,
        fields: field_uses,
        memo: HashMap::new(),
        keep_receivers: false,
    };
    let mut region_map: HashMap<Id, Vec<(i64, i64, Id)>> = HashMap::new();
    for r in region_rows {
        region_map.entry(r.module_node_id).or_default().push((
            r.start_byte,
            r.end_byte,
            r.condition_id,
        ));
    }
    let regions = Regions(region_map);
    let parameter: HashMap<Id, &ParameterRow> =
        parameter_rows.iter().map(|p| (p.node_id, p)).collect();
    let function: HashMap<Id, &FunctionRow> =
        function_rows.iter().map(|f| (f.node_id, f)).collect();
    let receiver_of: HashMap<Id, Id> = receiver_rows
        .iter()
        .map(|r| (r.function_node_id, r.parameter_node_id))
        .collect();
    let argument_at: HashMap<(Id, i64, i64), &ArgumentRow> = argument_rows
        .iter()
        .map(|a| ((a.module_node_id, a.start_byte, a.end_byte), a))
        .collect();
    let texts: HashMap<Id, String> = module_rows
        .iter()
        .filter_map(|m| m.text.clone().map(|t| (m.module_node_id, t)))
        .collect();
    let module_name: HashMap<Id, &str> = module_rows
        .iter()
        .map(|m| (m.module_node_id, m.module_name.as_str()))
        .collect();
    let module_names: BTreeSet<String> =
        module_rows.iter().map(|m| m.module_name.clone()).collect();
    let mut stored_at: HashMap<(Id, i64, i64), Vec<&DefRow>> = HashMap::new();
    for d in &defs {
        if let (Some(s), Some(e)) = (d.value_start_byte, d.value_end_byte) {
            stored_at
                .entry((d.module_node_id, s, e))
                .or_default()
                .push(d);
        }
    }
    let condition_text = |c: &Condition| -> (Id, String) { (c.id(), c.encode()) };

    let mut values_by_use: HashMap<Id, Vec<Id>> = HashMap::new();
    for r in &value_rows {
        values_by_use
            .entry(r.use_id)
            .or_default()
            .push(r.condition_id);
    }
    // The function each sink is in: its uses' scope function.
    let mut sink_function: HashMap<(Id, FlowSink, i64, i64), Id> = HashMap::new();
    for r in &value_rows {
        if let Some(f) = model.uses.get(&r.use_id).and_then(|u| u.function_node_id) {
            sink_function
                .entry((r.module_node_id, r.sink, r.sink_start_byte, r.sink_end_byte))
                .or_insert(f);
        }
    }
    // Value flows.
    let mut out = FlowModelRows::default();
    let mut sinks: BTreeSet<(Id, FlowSink, i64, i64)> = BTreeSet::new();
    for r in &value_rows {
        sinks.insert((r.module_node_id, r.sink, r.sink_start_byte, r.sink_end_byte));
    }
    for key in sinks {
        let (module, sink, start, end) = key;
        let place = match sink {
            FlowSink::Definition => {
                // Only a stored value is a sink of its own: a field or a dict entry.
                let stored: Vec<&DefRow> = stored_at
                    .get(&(module, start, end))
                    .into_iter()
                    .flatten()
                    .copied()
                    .filter(|d| d.place.contains('.') || d.place.contains('['))
                    .collect();
                let Some(d) = stored.first() else { continue };
                Some(d.place.clone())
            }
            _ => None,
        };
        let argument = (sink == FlowSink::Argument)
            .then(|| argument_at.get(&(module, start, end)).copied())
            .flatten();
        for ((origin, identity), s) in model.sink(key) {
            let (function_node_id, source_key, parameter_node_id, source_name, class_node_id) =
                match &origin {
                    Origin::Parameter(p) => {
                        let Some(pr) = parameter.get(p) else { continue };
                        (
                            pr.function_node_id,
                            format!("Parameter[{}]", p.hex()),
                            Some(*p),
                            pr.name.clone(),
                            None,
                        )
                    }
                    Origin::Field(class, field) => {
                        let Some(reader) = sink_function.get(&key).copied() else {
                            continue;
                        };
                        let qualified = function
                            .get(class)
                            .map_or_else(|| class.hex(), |f| f.qualified_name.clone());
                        (
                            reader,
                            format!("Field[{qualified}.{field}]"),
                            None,
                            field.clone(),
                            Some(*class),
                        )
                    }
                };
            let (condition_id, condition) = condition_text(&s.condition);
            out.value_flows.push(ValueFlowsRow {
                snapshot_id,
                function_node_id,
                source_key,
                parameter_node_id,
                source_name,
                class_node_id,
                sink,
                module_node_id: module,
                sink_start_byte: start,
                sink_end_byte: end,
                identity,
                captured: s.captured,
                condition_id,
                condition,
                argument_node_id: argument.map(|a| a.argument_node_id),
                call_site_node_id: argument.map(|a| a.call_node_id),
                place: place.clone(),
            });
        }
    }

    // Field accesses: every `x.f`, read or written, by its last segment.
    let receiver_name = |f: Option<Id>| -> Option<&str> {
        let r = receiver_of.get(&f?)?;
        parameter.get(r).map(|p| p.name.as_str())
    };
    let class_of = |f: Option<Id>| -> Option<Id> {
        let f = function.get(&f?)?;
        (f.parent_kind == Some(DeclarationKind::Class))
            .then_some(f.parent_node_id)
            .flatten()
    };
    let mut field_reads: HashMap<String, usize> = HashMap::new();
    for (write, module, place, start, end, func) in uses
        .iter()
        .map(|u| {
            (
                false,
                u.module_node_id,
                u.place.as_str(),
                u.start_byte,
                u.end_byte,
                u.function_node_id,
            )
        })
        .chain(defs.iter().map(|d| {
            (
                true,
                d.module_node_id,
                d.place.as_str(),
                d.start_byte,
                d.end_byte,
                d.function_node_id,
            )
        }))
    {
        if place.contains('[') {
            continue;
        }
        let Some((receiver, field)) = place.rsplit_once('.') else {
            continue;
        };
        let receiver_self = receiver_name(func) == Some(receiver);
        if !write {
            *field_reads.entry(field.to_owned()).or_default() += 1;
        }
        let condition = regions
            .at(module, start, end)
            .map(|c| model.condition(c))
            .unwrap_or_else(Condition::always);
        let (condition_id, condition) = condition_text(&condition);
        out.field_accesses.push(FieldAccessesRow {
            snapshot_id,
            field: field.to_owned(),
            write,
            receiver_self,
            class_node_id: receiver_self.then(|| class_of(func)).flatten(),
            function_node_id: func,
            module_node_id: module,
            start_byte: start,
            end_byte: end,
            place: place.to_owned(),
            condition_id,
            condition,
        });
    }

    // Every module's own module-level bindings, by name.
    let module_globals: BTreeSet<(String, String)> = defs
        .iter()
        .filter(|d| {
            d.scope_kind == LexicalScopeKind::Module
                && !d.place.contains('.')
                && !d.place.contains('[')
        })
        .filter_map(|d| {
            module_name
                .get(&d.module_node_id)
                .map(|m| (m.to_string(), d.place.clone()))
        })
        .collect();
    // Roots: a name reference's binding, for resolving places to modules and globals.
    let mut root_at: HashMap<(Id, i64, i64), Vec<&NameRootRow>> = HashMap::new();
    for r in &root_rows {
        root_at
            .entry((r.module_node_id, r.start_byte, r.end_byte))
            .or_default()
            .push(r);
    }
    let root_of = |r: &NameRootRow| -> Option<Root> {
        match r.binding_kind {
            BindingKind::Import => {
                let m = r.imported_module.as_deref()?;
                // `import a.b` binds `a`; `import a.b as x` binds `a.b`.
                if m == r.binding_name || !m.starts_with(&format!("{}.", r.binding_name)) {
                    Some(Root::Module(m.to_owned()))
                } else {
                    Some(Root::Module(r.binding_name.clone()))
                }
            }
            BindingKind::FromImport => {
                let m = r.imported_module.as_deref()?;
                let n = r.imported_name.as_deref()?;
                let full = format!("{m}.{n}");
                Some(
                    if !module_globals.contains(&(m.to_owned(), n.to_owned()))
                        && module_names.contains(&full)
                    {
                        Root::Module(full)
                    } else {
                        Root::Global(m.to_owned(), n.to_owned())
                    },
                )
            }
            BindingKind::Assignment | BindingKind::ClassDef | BindingKind::FunctionDef
                if r.binding_scope_kind == LexicalScopeKind::Module =>
            {
                Some(Root::Global(
                    module_name.get(&r.binding_module_node_id)?.to_string(),
                    r.binding_name.clone(),
                ))
            }
            _ => None,
        }
    };
    // Module-level bindings, for following a global re-exported from another module.
    let mut exported: HashMap<(String, String), Root> = HashMap::new();
    for r in &root_rows {
        if r.binding_scope_kind == LexicalScopeKind::Module
            && matches!(
                r.binding_kind,
                BindingKind::FromImport | BindingKind::Import
            )
            && let (Some(m), Some(root)) = (module_name.get(&r.binding_module_node_id), root_of(r))
        {
            exported.insert((m.to_string(), r.binding_name.clone()), root);
        }
    }
    let follow = |mut g: Root| -> Root {
        for _ in 0..8 {
            let Root::Global(m, n) = &g else { break };
            match exported.get(&(m.clone(), n.clone())) {
                Some(next) if next != &g => g = next.clone(),
                _ => break,
            }
        }
        g
    };
    // Classes by qualified name, and singletons: a module-level `N = C(...)`.
    let class_named: HashMap<&str, Id> = function_rows
        .iter()
        .filter(|f| f.kind == DeclarationKind::Class)
        .map(|f| (f.qualified_name.as_str(), f.node_id))
        .collect();
    let calls_at: HashMap<(Id, i64, i64), &CallRow> = call_rows
        .iter()
        .map(|c| ((c.module_node_id, c.start_byte, c.end_byte), c))
        .collect();
    // The root of a dotted expression at a span, resolved through our references.
    let resolve_dotted = |module: Id, start: i64, text: &str| -> Option<(Root, Vec<String>)> {
        let segments: Vec<&str> = text.split('.').map(str::trim).collect();
        let root_end = start + segments.first()?.len() as i64;
        let r = root_at.get(&(module, start, root_end))?.first()?;
        let root = root_of(r)?;
        let (at, used) = resolve(root, &segments[1..], &module_names, &module_globals);
        let rest = segments[1 + used..]
            .iter()
            .map(|s| (*s).to_owned())
            .collect();
        Some((follow(at), rest))
    };
    let mut singletons: HashMap<(String, String), Id> = HashMap::new();
    for d in defs
        .iter()
        .filter(|d| d.scope_kind == LexicalScopeKind::Module && d.kind == BindingKind::Assignment)
    {
        let (Some(vs), Some(ve)) = (d.value_start_byte, d.value_end_byte) else {
            continue;
        };
        let Some(call) = calls_at.get(&(d.module_node_id, vs, ve)) else {
            continue;
        };
        let Some(callee) = text_of(
            &texts,
            call.module_node_id,
            call.callee_start_byte,
            call.callee_end_byte,
        ) else {
            continue;
        };
        let Some((Root::Global(m, n), rest)) =
            resolve_dotted(call.module_node_id, call.callee_start_byte, callee)
        else {
            continue;
        };
        let qualified = std::iter::once(format!("{m}.{n}"))
            .chain(rest)
            .collect::<Vec<_>>()
            .join(".");
        if let (Some(&class), Some(module)) = (
            class_named.get(qualified.as_str()),
            module_name.get(&d.module_node_id),
        ) {
            singletons.insert((module.to_string(), d.place.clone()), class);
        }
    }

    let mut singleton_rows: Vec<SingletonsRow> = defs
        .iter()
        .filter(|d| d.scope_kind == LexicalScopeKind::Module)
        .filter_map(|d| {
            let module = module_name.get(&d.module_node_id)?;
            let class = singletons.get(&(module.to_string(), d.place.clone()))?;
            Some(SingletonsRow {
                snapshot_id,
                global: format!("{module}.{}", d.place),
                class_node_id: *class,
                module_node_id: d.module_node_id,
                start_byte: d.start_byte,
                end_byte: d.end_byte,
            })
        })
        .collect();
    singleton_rows.sort_by(|a, b| (&a.global, a.start_byte).cmp(&(&b.global, b.start_byte)));
    singleton_rows.dedup_by(|a, b| a.global == b.global);
    out.singletons = singleton_rows;

    // Raise sites: every `raise` with its region's condition and the parameters it tests.
    let mut parameters_of: HashMap<Id, Vec<&str>> = HashMap::new();
    for p in &parameter_rows {
        parameters_of
            .entry(p.function_node_id)
            .or_default()
            .push(p.name.as_str());
    }
    for r in &raise_rows {
        let condition = regions
            .at(r.module_node_id, r.start_byte, r.end_byte)
            .map(|c| model.condition(c))
            .unwrap_or_else(Condition::always);
        let names = parameters_of
            .get(&r.owner_node_id)
            .cloned()
            .unwrap_or_default();
        let mut tested: BTreeSet<String> = BTreeSet::new();
        if let Condition::Dnf(d) = &condition {
            for l in d.iter().flatten() {
                // A place's root, or any name an opaque test's text spells.
                let spelled: Vec<&str> = match (l.atom.place(), &l.atom) {
                    (Some(place), _) => vec![place.split('.').next().unwrap_or(place)],
                    (None, cpg_schema::condition::Atom::Opaque { text }) => text
                        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
                        .collect(),
                    (None, _) => Vec::new(),
                };
                for root in spelled {
                    if names.contains(&root) {
                        tested.insert(root.to_owned());
                    }
                }
            }
        }
        let text = text_of(&texts, r.module_node_id, r.start_byte, r.end_byte)
            .and_then(|t| t.lines().next())
            .unwrap_or_default()
            .trim()
            .to_owned();
        let (condition_id, condition) = condition_text(&condition);
        out.raise_sites.push(RaiseSitesRow {
            snapshot_id,
            function_node_id: r.owner_node_id,
            module_node_id: r.module_node_id,
            start_byte: r.start_byte,
            end_byte: r.end_byte,
            line: line_of(&texts, r.module_node_id, r.start_byte),
            text,
            condition_id,
            condition,
            parameters: tested.into_iter().collect(),
        });
    }

    // Each class's methods, by name, and the fields its body declares (a pydantic or dataclass
    // field, a class attribute).
    let methods: BTreeSet<(Id, String)> = function_rows
        .iter()
        .filter(|f| {
            f.parent_kind == Some(DeclarationKind::Class) && f.kind != DeclarationKind::Class
        })
        .filter_map(|f| f.parent_node_id.map(|c| (c, f.name.clone())))
        .collect();
    let declared_fields: BTreeSet<(Id, String)> = defs
        .iter()
        .filter(|d| {
            d.scope_kind == LexicalScopeKind::Class
                && matches!(
                    d.kind,
                    BindingKind::Assignment | BindingKind::AnnotationOnly
                )
                && !d.place.contains('.')
                && !d.place.contains('[')
                && !d.place.starts_with("__")
        })
        .filter_map(|d| d.function_node_id.map(|c| (c, d.place.clone())))
        .collect();

    // Ambient reads: a place whose root resolves to a singleton, and the field after it.
    for u in &uses {
        if !u.place.contains('.') || u.place.contains('[') {
            continue;
        }
        let Some((Root::Global(m, n), rest)) =
            resolve_dotted(u.module_node_id, u.start_byte, &u.place)
        else {
            continue;
        };
        let Some(&class) = singletons.get(&(m.clone(), n.clone())) else {
            continue;
        };
        // One read per field: the use naming exactly `<singleton>.<field>`; a method of its class
        // is not a field.
        if rest.len() != 1 || methods.contains(&(class, rest[0].clone())) {
            continue;
        }
        let field = rest[0].clone();
        let reader = u.function_node_id.filter(|f| {
            function
                .get(f)
                .is_some_and(|d| d.kind != DeclarationKind::Class)
        });
        let phase = match reader
            .and_then(|f| function.get(&f))
            .map(|f| f.name.as_str())
        {
            None => ReadPhase::Import,
            Some("__init__" | "__post_init__") => {
                let stored = stored_at.iter().any(|(&(module, s, e), ds)| {
                    module == u.module_node_id
                        && s <= u.start_byte
                        && u.end_byte <= e
                        && ds.iter().any(|d| {
                            d.function_node_id == reader
                                && receiver_name(reader)
                                    .is_some_and(|r| d.place.starts_with(&format!("{r}.")))
                        })
                });
                if stored {
                    ReadPhase::Snapshot
                } else {
                    ReadPhase::Construction
                }
            }
            Some(_) => ReadPhase::PerCall,
        };
        let region = regions
            .at(u.module_node_id, u.start_byte, u.end_byte)
            .map(|c| model.condition(c))
            .unwrap_or_else(Condition::always);
        // Inside a conditional expression or a boolean operand, the read happens only when the
        // expression selects it.
        let selected = values_by_use
            .get(&u.use_id)
            .map(|cs| {
                cs.iter()
                    .fold(Condition::never(), |acc, c| acc.or(&model.condition(*c)))
            })
            .unwrap_or_else(Condition::always);
        let condition = region.and(&selected);
        let (condition_id, condition) = condition_text(&condition);
        out.ambient_reads.push(AmbientReadsRow {
            snapshot_id,
            global: format!("{m}.{n}"),
            class_node_id: class,
            field,
            reader_node_id: reader,
            phase,
            module_node_id: u.module_node_id,
            start_byte: u.start_byte,
            end_byte: u.end_byte,
            line: line_of(&texts, u.module_node_id, u.start_byte),
            spelled: u.place.clone(),
            condition_id,
            condition,
        });
    }

    for c in &call_rows {
        if let Some(t) = text_of(
            &texts,
            c.module_node_id,
            c.callee_start_byte,
            c.callee_end_byte,
        ) {
            out.callee_text.insert(
                c.node_id,
                t.split_whitespace().collect::<Vec<_>>().join(" "),
            );
        }
    }

    // Dynamic accesses: calls of a builtin in the getattr family, `vars`, `exec`/`eval`,
    // `__import__`, `importlib.import_module`; and `__dict__` loads.
    let builtin_at: HashMap<(Id, i64, i64), &str> = root_rows
        .iter()
        .filter_map(|r| {
            r.builtin_name
                .as_deref()
                .map(|b| ((r.module_node_id, r.start_byte, r.end_byte), b))
        })
        .collect();
    let mut arguments_of: HashMap<Id, Vec<&ArgumentRow>> = HashMap::new();
    for a in &argument_rows {
        arguments_of.entry(a.call_node_id).or_default().push(a);
    }
    for v in arguments_of.values_mut() {
        v.sort_by_key(|a| a.start_byte);
    }
    let ancestors: HashMap<Id, Vec<Id>> = ancestry_rows.iter().fold(HashMap::new(), |mut m, r| {
        m.entry(r.class_node_id)
            .or_default()
            .push(r.ancestor_node_id);
        m
    });
    let is_literal = |a: &ArgumentRow| -> bool {
        text_of(&texts, a.module_node_id, a.start_byte, a.end_byte).is_some_and(|t| {
            let t = t.trim();
            (t.starts_with('"') || t.starts_with('\'')) && !t.starts_with("f")
        })
    };
    for c in &call_rows {
        let callee = text_of(
            &texts,
            c.module_node_id,
            c.callee_start_byte,
            c.callee_end_byte,
        )
        .unwrap_or_default();
        let builtin = builtin_at
            .get(&(c.module_node_id, c.callee_start_byte, c.callee_end_byte))
            .copied();
        let kind = match (builtin, callee) {
            (Some("getattr"), _) => DynamicKind::Getattr,
            (Some("setattr"), _) => DynamicKind::Setattr,
            (Some("hasattr"), _) => DynamicKind::Hasattr,
            (Some("delattr"), _) => DynamicKind::Delattr,
            (Some("vars"), _) => DynamicKind::Vars,
            (Some("exec"), _) => DynamicKind::Exec,
            (Some("eval"), _) => DynamicKind::Eval,
            (Some("__import__"), _) => DynamicKind::DunderImport,
            (_, "importlib.import_module" | "import_module") => DynamicKind::ImportModule,
            _ => continue,
        };
        let args = arguments_of.get(&c.node_id).cloned().unwrap_or_default();
        let name_arg = match kind {
            DynamicKind::Getattr
            | DynamicKind::Setattr
            | DynamicKind::Hasattr
            | DynamicKind::Delattr => args.get(1),
            DynamicKind::ImportModule | DynamicKind::DunderImport => args.first(),
            _ => None,
        };
        if name_arg.is_some_and(|a| is_literal(a)) {
            continue;
        }
        // The receiver's class under the model: `self` (through local copies) in a method of a
        // class, or a singleton global.
        let mut reaches_class = None;
        if matches!(
            kind,
            DynamicKind::Getattr
                | DynamicKind::Setattr
                | DynamicKind::Hasattr
                | DynamicKind::Delattr
                | DynamicKind::Vars
        ) && let Some(receiver) = args.first()
        {
            let key = (
                receiver.module_node_id,
                FlowSink::Argument,
                receiver.start_byte,
                receiver.end_byte,
            );
            let func = c.owner_node_id;
            let own = func.and_then(|f| receiver_of.get(&f).copied());
            model.keep_receivers = true;
            let sources = model.sink(key);
            model.keep_receivers = false;
            if own.is_some_and(|r| {
                sources
                    .keys()
                    .any(|(o, identity)| *o == Origin::Parameter(r) && *identity)
            }) {
                reaches_class = class_of(func);
            } else if let Some(t) = text_of(
                &texts,
                receiver.module_node_id,
                receiver.start_byte,
                receiver.end_byte,
            ) && let Some((Root::Global(m, n), rest)) =
                resolve_dotted(receiver.module_node_id, receiver.start_byte, t.trim())
                && rest.is_empty()
            {
                reaches_class = singletons.get(&(m, n)).copied();
            }
        }
        out.dynamic_accesses.push(DynamicAccessesRow {
            snapshot_id,
            call_site_node_id: c.node_id,
            kind,
            function_node_id: c.owner_node_id,
            module_node_id: c.module_node_id,
            start_byte: c.start_byte,
            end_byte: c.end_byte,
            reaches_class_node_id: reaches_class,
            reaches_modules: matches!(kind, DynamicKind::ImportModule | DynamicKind::DunderImport),
            reaches_all: matches!(kind, DynamicKind::Exec | DynamicKind::Eval),
        });
    }

    out.method_class = class_of_function.iter().map(|(f, c)| (*f, *c)).collect();
    out.qualified = function_rows
        .iter()
        .map(|f| (f.node_id, f.qualified_name.clone()))
        .collect();
    for r in &ancestry_rows {
        out.relatives
            .entry(r.class_node_id)
            .or_default()
            .insert(r.ancestor_node_id);
        out.relatives
            .entry(r.ancestor_node_id)
            .or_default()
            .insert(r.class_node_id);
    }

    // Negative premises (ADR-0022 §Verdicts).
    let dynamic_classes: BTreeSet<Id> = out
        .dynamic_accesses
        .iter()
        .filter_map(|d| d.reaches_class_node_id)
        .collect();
    let reaches_all = out.dynamic_accesses.iter().any(|d| d.reaches_all);
    let all_complete = module_rows
        .iter()
        .all(|m| complete.contains(&m.module_node_id));
    let related = |class: Id| -> bool {
        dynamic_classes.contains(&class)
            || ancestors
                .get(&class)
                .into_iter()
                .flatten()
                .any(|a| dynamic_classes.contains(a))
            || dynamic_classes
                .iter()
                .any(|d| ancestors.get(d).is_some_and(|a| a.contains(&class)))
    };
    let mut premises: BTreeMap<String, NegativePremisesRow> = BTreeMap::new();
    for u in unread_rows
        .iter()
        .filter(|u| module_name.contains_key(&u.module_node_id))
    {
        let (holds, reason, why) = if !complete.contains(&u.module_node_id) {
            (
                false,
                Some(BoundaryReason::MissingEvidence),
                Some("its module has no complete flow IR".to_owned()),
            )
        } else if reaches_all {
            (
                false,
                Some(BoundaryReason::DynamicAccess),
                Some("`exec` or `eval` in the release".to_owned()),
            )
        } else {
            (true, None, None)
        };
        premises.insert(
            format!("Parameter[{}]", u.parameter_node_id.hex()),
            NegativePremisesRow {
                snapshot_id,
                place_key: format!("Parameter[{}]", u.parameter_node_id.hex()),
                kind: PremiseKind::Parameter,
                subject_node_id: Some(u.parameter_node_id),
                holds,
                boundary_reason: reason,
                reason: why,
            },
        );
    }
    let written: BTreeSet<(Id, String)> = out
        .field_accesses
        .iter()
        .filter(|f| f.write && f.receiver_self)
        .filter_map(|f| f.class_node_id.map(|c| (c, f.field.clone())))
        .chain(declared_fields.iter().cloned())
        .collect();
    for (class, field) in &written {
        let qualified = function
            .get(class)
            .map_or_else(|| class.hex(), |f| f.qualified_name.clone());
        let key = format!("Field[{qualified}.{field}]");
        let (holds, reason, why) = if field_reads.contains_key(field) {
            (
                false,
                None,
                Some(format!(
                    "an attribute load named `{field}` exists in the release"
                )),
            )
        } else if !all_complete {
            (
                false,
                Some(BoundaryReason::MissingEvidence),
                Some("a release module has no complete flow IR".to_owned()),
            )
        } else if reaches_all || related(*class) {
            (
                false,
                Some(BoundaryReason::DynamicAccess),
                Some("a name-driven access reaches the class".to_owned()),
            )
        } else {
            (true, None, None)
        };
        premises.insert(
            key.clone(),
            NegativePremisesRow {
                snapshot_id,
                place_key: key,
                kind: PremiseKind::Field,
                subject_node_id: Some(*class),
                holds,
                boundary_reason: reason,
                reason: why,
            },
        );
    }
    // A singleton's fields: the class's fields, each read at the resolved key or not.
    let read_settings: BTreeSet<(String, String)> = out
        .ambient_reads
        .iter()
        .map(|r| (r.global.clone(), r.field.clone()))
        .collect();
    for ((m, n), class) in &singletons {
        let global = format!("{m}.{n}");
        for (c, field) in written.iter().filter(|(c, _)| c == class) {
            let key = format!("Global[{global}].{field}");
            let (holds, reason, why) = if read_settings.contains(&(global.clone(), field.clone())) {
                (false, None, Some("read at the resolved key".to_owned()))
            } else if field_reads.contains_key(field) {
                (
                    false,
                    None,
                    Some(format!(
                        "an attribute load named `{field}` exists in the release"
                    )),
                )
            } else if !all_complete {
                (
                    false,
                    Some(BoundaryReason::MissingEvidence),
                    Some("a release module has no complete flow IR".to_owned()),
                )
            } else if reaches_all || related(*c) {
                (
                    false,
                    Some(BoundaryReason::DynamicAccess),
                    Some("a name-driven access reaches the singleton's class".to_owned()),
                )
            } else {
                (true, None, None)
            };
            premises.insert(
                key.clone(),
                NegativePremisesRow {
                    snapshot_id,
                    place_key: key,
                    kind: PremiseKind::Global,
                    subject_node_id: Some(*class),
                    holds,
                    boundary_reason: reason,
                    reason: why,
                },
            );
        }
    }
    out.premises = premises.into_values().collect();
    Ok(out)
}
