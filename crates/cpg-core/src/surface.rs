//! Pure ordered surface normalization and bounded record associations (ADR-0074).
use crate::{
    CoreError,
    catalog::{CatalogFacts, Contracts},
};
use cpg_schema::{
    Codebook, Id, IdHasher,
    catalog::*,
    codebook::{BindingKind, RecordKind, SyntaxField, SyntaxKind},
};
use ruff_python_ast::{Expr, Number, UnaryOp};
use ruff_text_size::Ranged;
use std::collections::{BTreeMap, BTreeSet};

/// Decode Python scalar syntax, without evaluating any expression or importing a library.
pub fn literal(text: &str) -> Option<serde_json::Value> {
    fn value(e: &Expr) -> Option<serde_json::Value> {
        match e {
            Expr::NoneLiteral(_) => Some(serde_json::Value::Null),
            Expr::BooleanLiteral(b) => Some(b.value.into()),
            Expr::StringLiteral(s) => Some(s.value.to_str().into()),
            Expr::NumberLiteral(n) => match &n.value {
                Number::Int(i) => i.as_i64().map(Into::into),
                Number::Float(f) => serde_json::Number::from_f64(*f).map(Into::into),
                Number::Complex { .. } => None,
            },
            Expr::UnaryOp(u) if matches!(u.op, UnaryOp::USub | UnaryOp::UAdd) => {
                let n = value(&u.operand)?;
                if u.op == UnaryOp::UAdd {
                    return n.is_number().then_some(n);
                }
                if let Some(i) = n.as_i64() {
                    i.checked_neg().map(Into::into)
                } else {
                    serde_json::Number::from_f64(-n.as_f64()?).map(Into::into)
                }
            }
            _ => None,
        }
    }
    value(
        &ruff_python_parser::parse_expression(text.trim())
            .ok()?
            .syntax()
            .body,
    )
}

/// One admission policy shared with the behavioral owner. Metadata recognition is insufficient.
pub fn descriptor_preserves_body(targets: &[Option<&str>], flags: &[(bool, bool, bool)]) -> bool {
    if flags.is_empty() {
        return false;
    }
    match targets {
        [Some("builtins.classmethod")] => flags.iter().all(|f| f.0),
        [Some("builtins.staticmethod")] => flags.iter().all(|f| f.1),
        [Some("builtins.property")] => flags.iter().all(|f| f.2),
        [] => true,
        _ => false,
    }
}

/// Deliberately bounded body admission: a single bare builtin, lexical and provider agreement.
/// Richer alias/protocol metadata is retained independently of this execution claim.
pub fn admitted_descriptor(
    kind: SyntaxKind,
    module: Id,
    start: i64,
    end: i64,
    roots: &[crate::flow_model::NameRootRow],
    flags: &[(bool, bool, bool)],
) -> bool {
    if kind != SyntaxKind::ExprName {
        return false;
    }
    let matches: Vec<_> = roots
        .iter()
        .filter(|r| r.module_node_id == module && r.start_byte == start && r.end_byte == end)
        .collect();
    let target = matches.first().and_then(|r| r.builtin_name.as_deref());
    if matches.is_empty()
        || matches
            .iter()
            .any(|r| r.binding_kind != BindingKind::Implicit || r.builtin_name.as_deref() != target)
    {
        return false;
    }
    let target = target.map(|name| format!("builtins.{name}"));
    descriptor_preserves_body(&[target.as_deref()], flags)
}

/// Resolve the root through lexical observations, then append syntactic attribute segments.
/// Any conflicting root or local variable keeps the effective target unresolved.
pub(crate) fn resolved(expr: &Expr, module: Id, base: i64, facts: &CatalogFacts) -> Option<String> {
    match expr {
        Expr::Call(c) => resolved(&c.func, module, base, facts),
        Expr::Attribute(a) => Some(format!(
            "{}.{}",
            resolved(&a.value, module, base, facts)?,
            a.attr
        )),
        Expr::Name(n) => {
            let start = base + i64::from(n.start().to_u32());
            let end = base + i64::from(n.end().to_u32());
            let roots: Vec<_> = facts
                .roots
                .iter()
                .filter(|r| {
                    r.module_node_id == module && r.start_byte == start && r.end_byte == end
                })
                .collect();
            if roots.is_empty() {
                return None;
            }
            let mut targets = BTreeSet::new();
            for r in roots {
                let target = if r.binding_kind == BindingKind::Implicit {
                    format!("builtins.{}", r.builtin_name.as_deref()?)
                } else if let Some(module) = &r.imported_module {
                    match &r.imported_name {
                        Some(name) => format!("{module}.{name}"),
                        None => module.clone(),
                    }
                } else {
                    return None;
                };
                targets.insert(target);
            }
            (targets.len() == 1)
                .then(|| targets.into_iter().next())
                .flatten()
        }
        _ => None,
    }
}

fn name_binding<'a>(
    expr: &Expr,
    module: Id,
    base: i64,
    facts: &'a CatalogFacts,
) -> Option<&'a cpg_schema::tables::BindingsRow> {
    let Expr::Name(name) = expr else { return None };
    let references: Vec<_> = facts
        .references
        .iter()
        .filter(|r| {
            r.module_node_id == module
                && r.start_byte == base + i64::from(name.start().to_u32())
                && r.end_byte == base + i64::from(name.end().to_u32())
        })
        .collect();
    let ids: BTreeSet<_> = references.iter().map(|r| r.node_id).collect();
    let resolutions: Vec<_> = facts
        .resolutions
        .iter()
        .filter(|r| ids.contains(&r.reference_id))
        .collect();
    let id = resolutions.first()?.binding_id?;
    if resolutions
        .iter()
        .any(|r| r.binding_id != Some(id) || r.captured || r.reason.is_some())
    {
        return None;
    }
    facts.lexical_bindings.iter().find(|b| b.node_id == id)
}
fn local_declaration(expr: &Expr, module: Id, base: i64, facts: &CatalogFacts) -> Option<Id> {
    let binding = name_binding(expr, module, base, facts)?;
    matches!(
        binding.kind,
        BindingKind::FunctionDef | BindingKind::ClassDef
    )
    .then_some(binding.site_node_id)
}
/// Class bodies execute in source order. A self-rebinding property accessor can therefore
/// name its preceding direct class definition even when lexical candidates include the new def.
fn preceding_class_member(class: Id, name: &str, before: i64, facts: &CatalogFacts) -> Option<Id> {
    if facts
        .nodes
        .iter()
        .any(|n| n.parent_node_id == class && n.field == SyntaxField::Argument)
    {
        return None;
    }
    let scopes: BTreeSet<_> = facts
        .scopes
        .iter()
        .filter(|s| {
            s.owner_node_id == class && s.kind == cpg_schema::codebook::LexicalScopeKind::Class
        })
        .map(|s| s.node_id)
        .collect();
    let binding = facts
        .lexical_bindings
        .iter()
        .filter(|b| scopes.contains(&b.scope_id) && b.name == name && b.start_byte < before)
        .max_by_key(|b| (b.start_byte, b.ordinal))?;
    if binding.kind != BindingKind::FunctionDef {
        return None;
    }
    facts
        .nodes
        .iter()
        .any(|n| {
            n.node_id == binding.site_node_id
                && n.parent_node_id == class
                && n.field == SyntaxField::Body
        })
        .then_some(binding.site_node_id)
}
/// Only an unambiguous direct construction of the pinned registration owner is recognized.
fn registration_target(expr: &Expr, module: Id, base: i64, facts: &CatalogFacts) -> Option<String> {
    let expr = if let Expr::Call(c) = expr {
        c.func.as_ref()
    } else {
        expr
    };
    let Expr::Attribute(a) = expr else {
        return None;
    };
    if !matches!(a.attr.as_str(), "tool" | "resource" | "prompt") {
        return None;
    }
    let binding = name_binding(&a.value, module, base, facts)?;
    let (start, end) = (binding.value_start_byte?, binding.value_end_byte?);
    let file = facts
        .source
        .iter()
        .find(|f| f.module_node_id == binding.module_node_id)?;
    let source = file.text.as_deref()?.get(start as usize..end as usize)?;
    let expression = ruff_python_parser::parse_expression(source).ok()?;
    let Expr::Call(call) = expression.syntax().body.as_ref() else {
        return None;
    };
    let target = resolved(&call.func, binding.module_node_id, start, facts)?;
    matches!(
        target.as_str(),
        "fastmcp.FastMCP" | "fastmcp.server.server.FastMCP"
    )
    .then(|| format!("fastmcp.server.server.FastMCP.{}", a.attr))
}

struct SurfaceAspects {
    binding: Option<&'static str>,
    accessor: Option<&'static str>,
    protocol: Option<&'static str>,
    registration: Option<&'static str>,
    reason: Option<&'static str>,
}
fn aspects(target: Option<&str>, call: bool, pilot: bool) -> SurfaceAspects {
    let (binding, accessor, protocol, registration, reason) = match target {
        Some("builtins.classmethod") if !call => (Some("class"), None, None, None, None),
        Some("builtins.staticmethod") if !call => (Some("static"), None, None, None, None),
        Some("builtins.property") if !call => (Some("property"), Some("getter"), None, None, None),
        Some("builtins.property.getter") if !call => (
            Some("property"),
            Some("getter"),
            None,
            None,
            Some("accessor binding; body admission unmodeled"),
        ),
        Some("builtins.property.setter") if !call => (
            Some("property"),
            Some("setter"),
            None,
            None,
            Some("accessor binding; body admission unmodeled"),
        ),
        Some("builtins.property.deleter") if !call => (
            Some("property"),
            Some("deleter"),
            None,
            None,
            Some("accessor binding; body admission unmodeled"),
        ),
        Some("functools.cached_property") if !call => (
            Some("property"),
            Some("cached_getter"),
            None,
            None,
            Some("descriptor body admission unmodeled"),
        ),
        Some("contextlib.contextmanager") if !call => (
            None,
            None,
            Some("context_manager"),
            None,
            Some("protocol form does not prove cleanup"),
        ),
        Some("contextlib.asynccontextmanager") if !call => (
            None,
            None,
            Some("async_context_manager"),
            None,
            Some("protocol form does not prove cleanup"),
        ),
        Some(
            "typing.overload"
            | "typing.final"
            | "typing.override"
            | "typing_extensions.overload"
            | "typing_extensions.final"
            | "typing_extensions.override",
        ) if !call => (None, None, None, None, Some("declaration metadata only")),
        Some("functools.wraps") if call => (
            None,
            None,
            None,
            None,
            Some("wrapping relationship; effective signature unresolved"),
        ),
        Some(
            "fastmcp.tool"
            | "fastmcp.tools.function_tool.tool"
            | "fastmcp.server.server.FastMCP.tool",
        ) if pilot => (
            None,
            None,
            None,
            Some("tool"),
            Some("registration metadata; body admission remains separate"),
        ),
        Some("fastmcp.server.server.FastMCP.resource") if pilot => (
            None,
            None,
            None,
            Some("resource"),
            Some("registration metadata; body admission remains separate"),
        ),
        Some("fastmcp.server.server.FastMCP.prompt") if pilot => (
            None,
            None,
            None,
            Some("prompt"),
            Some("registration metadata; body admission remains separate"),
        ),
        Some("dataclasses.dataclass" | "attrs.define" | "attrs.frozen" | "attr.s") => {
            (None, None, None, None, Some("record declaration metadata"))
        }
        Some("pydantic.field_validator" | "pydantic.functional_validators.field_validator")
            if call =>
        {
            (
                None,
                None,
                None,
                None,
                Some("validator declaration; conversion semantics unmodeled"),
            )
        }
        _ => (
            None,
            None,
            None,
            None,
            Some("unresolved or unsupported decorator"),
        ),
    };
    SurfaceAspects {
        binding,
        accessor,
        protocol,
        registration,
        reason,
    }
}

fn simple_record_field(s: &cpg_schema::tables::RecordFieldSyntaxRow, facts: &CatalogFacts) -> bool {
    let Some(text) = s.value_text.as_deref() else {
        return true;
    };
    if literal(text).is_some() {
        return true;
    }
    let Ok(parsed) = ruff_python_parser::parse_expression(text) else {
        return false;
    };
    let Expr::Call(call) = parsed.syntax().body.as_ref() else {
        return false;
    };
    let base = s.value_start_byte.unwrap_or(0);
    if resolved(&call.func, s.module_node_id, base, facts).as_deref() != Some("dataclasses.field")
        || !call.arguments.args.is_empty()
    {
        return false;
    }
    call.arguments.keywords.iter().all(|k| {
        let Some(name) = &k.arg else { return false };
        if name.as_str() == "default_factory" {
            matches!(
                resolved(&k.value, s.module_node_id, base, facts).as_deref(),
                Some("builtins.list" | "builtins.dict" | "builtins.set" | "builtins.tuple")
            )
        } else {
            literal(&text[k.value.range()]).is_some()
        }
    })
}
fn simple_record_decorator(expression: &str) -> bool {
    let Ok(parsed) = ruff_python_parser::parse_expression(expression) else {
        return false;
    };
    match parsed.syntax().body.as_ref() {
        Expr::Call(c) => {
            c.arguments.args.is_empty()
                && c.arguments.keywords.iter().all(|k| {
                    k.arg.as_ref().is_some_and(|n| {
                        matches!(
                            n.as_str(),
                            "init"
                                | "repr"
                                | "eq"
                                | "order"
                                | "unsafe_hash"
                                | "frozen"
                                | "match_args"
                                | "kw_only"
                                | "slots"
                                | "weakref_slot"
                        )
                    }) && matches!(k.value, Expr::BooleanLiteral(_))
                })
        }
        Expr::Name(_) | Expr::Attribute(_) => true,
        _ => false,
    }
}

/// A deliberately small straight-line constructor proof. No calls, branches, conversions,
/// rebinding or descriptor hooks are interpreted as identity storage.
fn direct_store(facts: &CatalogFacts, function: Id, formal: Id, field: &str) -> Option<Id> {
    let descriptors: Vec<_> = facts
        .descriptors
        .iter()
        .filter(|d| d.function_node_id == function)
        .collect();
    if descriptors.is_empty()
        || descriptors
            .iter()
            .any(|d| d.is_classmethod || d.is_staticmethod)
    {
        return None;
    }
    let parameters: Vec<_> = facts
        .syntax
        .iter()
        .filter(|p| p.function_node_id == function)
        .collect();
    let receiver = parameters.iter().find(|p| p.ordinal == 0)?;
    let formal = parameters.iter().find(|p| p.node_id == formal)?;
    if receiver.node_id == formal.node_id {
        return None;
    }
    let nodes: Vec<_> = facts
        .nodes
        .iter()
        .filter(|n| n.owner_node_id == Some(function))
        .collect();
    if nodes.iter().any(|n| {
        n.kind.text().starts_with("stmt_")
            && !matches!(
                n.kind,
                SyntaxKind::StmtAssign
                    | SyntaxKind::StmtAnnAssign
                    | SyntaxKind::StmtExpr
                    | SyntaxKind::StmtPass
            )
    }) {
        return None;
    }
    if nodes.iter().any(|n| {
        n.kind == SyntaxKind::StmtExpr
            && !nodes
                .iter()
                .any(|c| c.parent_node_id == n.node_id && c.kind == SyntaxKind::ExprStringLiteral)
    }) {
        return None;
    }
    if nodes.iter().any(|n| {
        matches!(
            n.kind,
            SyntaxKind::ExprCall
                | SyntaxKind::StmtIf
                | SyntaxKind::StmtFor
                | SyntaxKind::StmtWhile
                | SyntaxKind::StmtTry
                | SyntaxKind::StmtWith
                | SyntaxKind::StmtRaise
                | SyntaxKind::StmtReturn
                | SyntaxKind::StmtAugAssign
                | SyntaxKind::StmtDelete
                | SyntaxKind::ExprYield
                | SyntaxKind::ExprYieldFrom
                | SyntaxKind::ExprAwait
        )
    }) {
        return None;
    }
    let class = facts
        .declarations
        .iter()
        .find(|d| d.node_id == function)?
        .parent_node_id?;
    // Every effect in the constructor must fit the supported storage shape. An unrelated
    // descriptor assignment or overloaded operator can otherwise mutate the selected field.
    for statement in nodes
        .iter()
        .filter(|n| matches!(n.kind, SyntaxKind::StmtAssign | SyntaxKind::StmtAnnAssign))
    {
        if statement.kind != SyntaxKind::StmtAssign {
            return None;
        }
        let targets: Vec<_> = nodes
            .iter()
            .filter(|n| n.parent_node_id == statement.node_id && n.field == SyntaxField::Target)
            .collect();
        let [target] = targets.as_slice() else {
            return None;
        };
        if target.kind != SyntaxKind::ExprAttribute
            || !facts.record_fields.iter().any(|f| {
                f.class_node_id == class && Some(f.name.as_str()) == target.detail.as_deref()
            })
        {
            return None;
        }
        let base = nodes.iter().find(|n| {
            n.parent_node_id == target.node_id
                && n.field == SyntaxField::Value
                && n.kind == SyntaxKind::ExprName
        })?;
        let roots: Vec<_> = facts
            .roots
            .iter()
            .filter(|r| {
                r.module_node_id == base.module_node_id
                    && r.start_byte == base.start_byte
                    && r.end_byte == base.end_byte
            })
            .collect();
        if roots.is_empty()
            || roots.iter().any(|r| {
                r.binding_kind != BindingKind::Parameter || r.binding_name != receiver.name
            })
        {
            return None;
        }
        let value = nodes
            .iter()
            .find(|n| n.parent_node_id == statement.node_id && n.field == SyntaxField::Value)?;
        let source = facts
            .source
            .iter()
            .find(|f| f.module_node_id == value.module_node_id)?
            .text
            .as_deref()?;
        if literal(source.get(value.start_byte as usize..value.end_byte as usize)?).is_none() {
            if value.kind != SyntaxKind::ExprName {
                return None;
            }
            let roots: Vec<_> = facts
                .roots
                .iter()
                .filter(|r| {
                    r.module_node_id == value.module_node_id
                        && r.start_byte == value.start_byte
                        && r.end_byte == value.end_byte
                })
                .collect();
            if roots.is_empty()
                || roots.iter().any(|r| {
                    r.binding_kind != BindingKind::Parameter
                        || !parameters.iter().any(|p| p.name == r.binding_name)
                })
            {
                return None;
            }
        }
    }
    let mut stores = Vec::new();
    for target in nodes.iter().filter(|n| {
        n.kind == SyntaxKind::ExprAttribute
            && n.detail.as_deref() == Some(field)
            && n.field == SyntaxField::Target
    }) {
        let statement = nodes
            .iter()
            .find(|n| n.node_id == target.parent_node_id && n.kind == SyntaxKind::StmtAssign)?;
        let targets: Vec<_> = nodes
            .iter()
            .filter(|n| n.parent_node_id == statement.node_id && n.field == SyntaxField::Target)
            .collect();
        if targets.len() != 1 {
            return None;
        }
        let value = nodes.iter().find(|n| {
            n.parent_node_id == statement.node_id
                && n.field == SyntaxField::Value
                && n.kind == SyntaxKind::ExprName
        })?;
        let base = nodes.iter().find(|n| {
            n.parent_node_id == target.node_id
                && n.field == SyntaxField::Value
                && n.kind == SyntaxKind::ExprName
        })?;
        for (node, param) in [(value, *formal), (base, *receiver)] {
            if node.detail.as_deref() != Some(&param.name) {
                return None;
            }
            let roots: Vec<_> = facts
                .roots
                .iter()
                .filter(|r| {
                    r.module_node_id == node.module_node_id
                        && r.start_byte == node.start_byte
                        && r.end_byte == node.end_byte
                })
                .collect();
            if roots.is_empty()
                || roots.iter().any(|r| {
                    r.binding_kind != BindingKind::Parameter || r.binding_name != param.name
                })
            {
                return None;
            }
        }
        stores.push(statement.fact_id);
    }
    (stores.len() == 1).then(|| stores[0])
}

pub fn derive(
    facts: &CatalogFacts,
    selected: &BTreeSet<Id>,
    snapshot_id: Id,
    out: &mut Contracts,
) -> Result<(), CoreError> {
    let files: BTreeMap<_, _> = facts.source.iter().map(|r| (r.module_node_id, r)).collect();
    let declarations: BTreeMap<_, _> = facts.declarations.iter().map(|r| (r.node_id, r)).collect();
    let pilot = facts
        .releases
        .iter()
        .any(|r| r.distributions.iter().any(|d| d == "fastmcp==4.0.5"));
    let mut ordered: Vec<_> = facts
        .declarations
        .iter()
        .filter(|d| selected.contains(&d.node_id))
        .collect();
    ordered.sort_by_key(|d| (d.module_node_id, d.start_byte, d.node_id));
    for decl in ordered {
        let mut nodes: Vec<_> = facts
            .nodes
            .iter()
            .filter(|n| n.parent_node_id == decl.node_id && n.field == SyntaxField::Decorator)
            .collect();
        nodes.sort_by_key(|n| (n.start_byte, n.end_byte, n.node_id));
        let mut observations = Vec::new();
        for (ordinal, n) in nodes.iter().enumerate() {
            let Some(source) = files.get(&n.module_node_id).and_then(|f| f.text.as_deref()) else {
                continue;
            };
            let expression = source
                .get(n.start_byte as usize..n.end_byte as usize)
                .ok_or_else(|| CoreError::Analysis("decorator outside source bytes".into()))?;
            let parsed = ruff_python_parser::parse_expression(expression).ok();
            let mut target = parsed.as_ref().and_then(|p| {
                resolved(&p.syntax().body, n.module_node_id, n.start_byte, facts).or_else(|| {
                    pilot
                        .then(|| {
                            registration_target(
                                &p.syntax().body,
                                n.module_node_id,
                                n.start_byte,
                                facts,
                            )
                        })
                        .flatten()
                })
            });
            let call = parsed
                .as_ref()
                .is_some_and(|p| matches!(p.syntax().body.as_ref(), Expr::Call(_)));
            // Accessors refer to the preceding property binding in the same class, not a suffix.
            let mut accessor_link = None;
            if target.as_deref() == Some("functools.wraps")
                && let Some(parsed) = &parsed
                && let Expr::Call(c) = parsed.syntax().body.as_ref()
                && let Some(wrapped) = c.arguments.args.first()
            {
                accessor_link = local_declaration(wrapped, n.module_node_id, n.start_byte, facts);
            }

            if let Some(parsed) = &parsed
                && let Expr::Attribute(a) = parsed.syntax().body.as_ref()
                && matches!(a.attr.as_str(), "getter" | "setter" | "deleter")
                && let Expr::Name(name) = a.value.as_ref()
            {
                let previous = decl
                    .parent_node_id
                    .and_then(|class| {
                        preceding_class_member(class, name.id.as_str(), n.start_byte, facts)
                    })
                    .or_else(|| local_declaration(&a.value, n.module_node_id, n.start_byte, facts))
                    .and_then(|id| {
                        facts.declarations.iter().find(|d| {
                            d.node_id == id
                                && d.parent_node_id == decl.parent_node_id
                                && d.name == name.id.as_str()
                                && d.start_byte < decl.start_byte
                        })
                    });
                if let Some(previous) = previous
                    && {
                        let aspects: Vec<_> = out
                            .surfaces
                            .iter()
                            .filter(|s| s.declaration_node_id == previous.node_id)
                            .collect();
                        matches!(aspects.as_slice(),[s] if s.binding_mode.as_deref()==Some("property")&&matches!(s.resolved_target.as_deref(),Some("builtins.property"|"builtins.property.getter"|"builtins.property.setter"|"builtins.property.deleter")))
                    }
                {
                    target = Some(format!("builtins.property.{}", a.attr));
                    accessor_link = Some(previous.node_id);
                }
            }
            let SurfaceAspects {
                binding,
                accessor,
                protocol,
                registration,
                reason,
            } = aspects(target.as_deref(), call, pilot);

            observations.push(CatalogSurfacesRow {
                snapshot_id,
                declaration_node_id: decl.node_id,
                ordinal: ordinal as i64,
                related_node_id: accessor_link,
                source_fact_id: n.fact_id,
                expression: expression.into(),
                resolved_target: target.clone(),
                binding_mode: binding.map(Into::into),
                accessor_role: accessor.map(Into::into),
                protocol: protocol.map(Into::into),
                registration: registration.map(Into::into),
                admission: "withheld".into(),
                reason: reason.map(Into::into),
            });
        }
        let flags: Vec<_> = facts
            .descriptors
            .iter()
            .filter(|f| f.function_node_id == decl.node_id)
            .map(|f| (f.is_classmethod, f.is_staticmethod, f.is_property_getter))
            .collect();
        let preserved = match nodes.as_slice() {
            [n] => admitted_descriptor(
                n.kind,
                n.module_node_id,
                n.start_byte,
                n.end_byte,
                &facts.roots,
                &flags,
            ),
            _ => false,
        };
        for r in &mut observations {
            if preserved {
                r.admission = "body_preserved".into()
            } else if r.reason.is_none() {
                r.reason = Some("decorator composition or provider agreement unresolved".into())
            }
        }
        out.surfaces.extend(observations);
    }
    let fields: Vec<_> = facts
        .record_fields
        .iter()
        .filter(|f| selected.contains(&f.class_node_id))
        .collect();
    for field in fields {
        let syntaxes: Vec<_> = facts
            .field_syntax
            .iter()
            .filter(|s| s.field_node_id == field.node_id)
            .collect();
        // Multiple source observations remain explicit uncertainty, never an arbitrary winner.
        let syntax = if syntaxes.len() == 1 {
            syntaxes.first().copied()
        } else {
            None
        };
        let mut default = syntax.and_then(|s| s.value_text.clone());
        let mut factory = None;
        let mut reason = (syntaxes.len() > 1).then(|| "conflicting field syntax".to_owned());
        if let Some(s) = syntax
            && let Some(text) = &s.value_text
            && let Ok(parsed) = ruff_python_parser::parse_expression(text)
            && let Expr::Call(call) = parsed.syntax().body.as_ref()
        {
            let target = resolved(
                &call.func,
                s.module_node_id,
                s.value_start_byte.unwrap_or(0),
                facts,
            );
            if matches!(
                target.as_deref(),
                Some(
                    "dataclasses.field"
                        | "attrs.field"
                        | "attr.ib"
                        | "pydantic.Field"
                        | "pydantic.fields.Field"
                )
            ) {
                let defaults: Vec<_> = call
                    .arguments
                    .keywords
                    .iter()
                    .filter(|k| k.arg.as_ref().is_some_and(|a| a.as_str() == "default"))
                    .collect();
                let factories: Vec<_> = call
                    .arguments
                    .keywords
                    .iter()
                    .filter(|k| {
                        k.arg
                            .as_ref()
                            .is_some_and(|a| matches!(a.as_str(), "default_factory" | "factory"))
                    })
                    .collect();
                if defaults.len() <= 1
                    && factories.len() <= 1
                    && !(defaults.len() == 1 && factories.len() == 1)
                {
                    default = defaults.first().map(|k| text[k.value.range()].to_owned());
                    factory = factories.first().map(|k| text[k.value.range()].to_owned());
                    if default.is_none() && factory.is_none() && !call.arguments.args.is_empty() {
                        reason = Some("positional field default is not normalized".into());
                        default = Some(text.clone());
                    }
                } else {
                    reason = Some("conflicting field default and factory".into())
                }
            }
        }
        let (default_state, literal_json) = if factory.is_some() {
            ("factory_expression".into(), None)
        } else {
            crate::catalog::default_value(default.as_deref(), field.has_default.map(|v| !v))
        };
        out.configurations.push(CatalogConfigurationsRow {
            snapshot_id,
            field_id: field.node_id,
            class_node_id: field.class_node_id,
            source_fact_id: field.fact_id,
            syntax_fact_id: syntax.map(|s| s.fact_id),
            record_kind: field.record_kind.text().into(),
            name: field.name.clone(),
            ordinal: field.ordinal,
            term_id: field.term_node_id,
            alias: field.alias.clone(),
            init: field.init,
            kw_only: field.kw_only,
            required: field.required,
            read_only: field.read_only,
            annotation_text: syntax.and_then(|s| s.annotation_text.clone()),
            default_state,
            default_text: default,
            literal_json,
            factory_text: factory,
            reason,
        });
        if field.init == Some(false) || field.record_kind == RecordKind::TypedDict {
            continue;
        }
        let name = field.alias.as_deref().unwrap_or(&field.name);
        for constructor in out
            .constructors
            .iter()
            .filter(|c| c.class_node_id == field.class_node_id)
        {
            for p in out
                .parameters
                .iter()
                .filter(|p| p.signature_id == constructor.signature_id)
            {
                let signature = out
                    .signatures
                    .iter()
                    .find(|s| s.signature_id == p.signature_id);
                let direct = signature.filter(|s| s.role == "source").and_then(|s| {
                    p.formal_node_id.and_then(|formal| {
                        direct_store(facts, s.callable_node_id, formal, &field.name)
                    })
                });
                if p.name.as_deref() != Some(name) && direct.is_none() {
                    continue;
                }

                let hooks = facts.declarations.iter().any(|d| {
                    d.parent_node_id == Some(field.class_node_id)
                        && matches!(
                            d.name.as_str(),
                            "__new__"
                                | "__post_init__"
                                | "__setattr__"
                                | "__getattribute__"
                                | "__getattr__"
                        )
                });
                let plain_base = facts
                    .ancestry_targets
                    .iter()
                    .filter(|a| a.class_node_id == Some(field.class_node_id))
                    .all(|a| {
                        a.ancestor_node_id == Some(field.class_node_id)
                            || a.ancestor_node_id.is_none()
                                && facts.ancestry.iter().any(|r| {
                                    r.fact_id == a.ancestry_fact_id
                                        && r.ancestor_module.as_deref() == Some("builtins")
                                        && r.ancestor_key.as_deref() == Some("object")
                                })
                    });
                let generated = signature.is_some_and(|s| s.role == "provider_constructor")
                    && !facts.declarations.iter().any(|d| {
                        d.parent_node_id == Some(field.class_node_id) && d.name == "__init__"
                    });
                let class_decorators: Vec<_> = out
                    .surfaces
                    .iter()
                    .filter(|s| s.declaration_node_id == field.class_node_id)
                    .collect();
                let dataclass_only = matches!(class_decorators.as_slice(),[s] if s.resolved_target.as_deref()==Some("dataclasses.dataclass")&&simple_record_decorator(&s.expression));
                let class_arguments = facts.nodes.iter().any(|n| {
                    n.parent_node_id == field.class_node_id && n.field == SyntaxField::Argument
                });
                let field_descriptors = facts.declarations.iter().any(|d| {
                    d.parent_node_id == Some(field.class_node_id)
                        && facts
                            .record_fields
                            .iter()
                            .any(|f| f.class_node_id == field.class_node_id && f.name == d.name)
                });
                let simple_fields = facts
                    .field_syntax
                    .iter()
                    .filter(|s| s.class_node_id == field.class_node_id)
                    .all(|s| simple_record_field(s, facts));
                let simple_methods = !out.surfaces.iter().any(|s| {
                    s.admission != "body_preserved"
                        && signature
                            .is_some_and(|sig| sig.callable_node_id == s.declaration_node_id)
                });
                let exact = simple_fields
                    && simple_methods
                    && dataclass_only
                    && !class_arguments
                    && field.record_kind == RecordKind::Dataclass
                    && (generated || direct.is_some())
                    && !hooks
                    && !field_descriptors
                    && plain_base
                    && syntax.is_some()
                    && out.configurations.last().is_some_and(|f| {
                        f.literal_json.is_some()
                            || f.factory_text.is_some()
                            || f.default_text.is_none()
                    });
                let kind = if exact {
                    "exact_storage"
                } else {
                    "declared_parameter"
                };
                out.field_links.push(CatalogFieldLinksRow {
                    snapshot_id,
                    link_id: IdHasher::new("catalog-field-link")
                        .id(field.fact_id)
                        .id(p.signature_id)
                        .i64(p.ordinal)
                        .str(kind)
                        .finish_id(),
                    field_id: field.node_id,
                    class_node_id: field.class_node_id,
                    signature_id: p.signature_id,
                    ordinal: p.ordinal,
                    formal_node_id: p.formal_node_id,
                    reader_node_id: None,
                    source_fact_id: if exact {
                        direct.unwrap_or(field.fact_id)
                    } else {
                        field.fact_id
                    },
                    kind: kind.into(),
                    reason: (!exact)
                        .then(|| "declaration association; runtime storage unproved".into()),
                });
            }
        }
        // Reader association names a field access, not unchanged value through time.
        for node in facts.nodes.iter().filter(|n| {
            n.kind == SyntaxKind::ExprAttribute
                && n.detail.as_deref() == Some(&field.name)
                && n.field != SyntaxField::Target
        }) {
            let Some(reader) = node.owner_node_id.and_then(|id| declarations.get(&id)) else {
                continue;
            };
            if reader.parent_node_id != Some(field.class_node_id) {
                continue;
            }
            let descriptors: Vec<_> = facts
                .descriptors
                .iter()
                .filter(|d| d.function_node_id == reader.node_id)
                .collect();
            if descriptors.is_empty()
                || descriptors
                    .iter()
                    .any(|d| d.is_classmethod || d.is_staticmethod)
            {
                continue;
            }
            if out
                .surfaces
                .iter()
                .any(|s| s.declaration_node_id == reader.node_id && s.admission != "body_preserved")
            {
                continue;
            }

            let receivers: Vec<_> = facts
                .syntax
                .iter()
                .filter(|p| p.function_node_id == reader.node_id && p.ordinal == 0)
                .collect();
            let Some(receiver) = receivers.first() else {
                continue;
            };
            let Some(child) = facts.nodes.iter().find(|n| {
                n.parent_node_id == node.node_id
                    && n.field == SyntaxField::Value
                    && n.kind == SyntaxKind::ExprName
            }) else {
                continue;
            };
            if child.detail.as_deref() != Some(&receiver.name) {
                continue;
            }
            let roots: Vec<_> = facts
                .roots
                .iter()
                .filter(|r| {
                    r.module_node_id == child.module_node_id
                        && r.start_byte == child.start_byte
                        && r.end_byte == child.end_byte
                })
                .collect();
            if roots.is_empty()
                || roots.iter().any(|r| {
                    r.binding_kind != BindingKind::Parameter || r.binding_name != receiver.name
                })
            {
                continue;
            }
            let stores: Vec<_> = out
                .field_links
                .iter()
                .filter(|l| l.field_id == field.node_id && l.kind == "exact_storage")
                .cloned()
                .collect();
            for mut link in stores {
                link.link_id = IdHasher::new("catalog-field-reader")
                    .id(link.link_id)
                    .id(node.fact_id)
                    .finish_id();
                link.reader_node_id = Some(reader.node_id);
                link.source_fact_id = node.fact_id;
                link.kind = "exact_reader".into();
                link.reason = Some(
                    "exact receiver field association; intervening mutation is not excluded".into(),
                );
                out.field_links.push(link);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn python_defaults_preserve_spelling_and_refuse_execution() {
        for (text, expected) in [
            ("'http'", serde_json::json!("http")),
            ("0x10", 16.into()),
            ("1_024", 1024.into()),
            ("-3", (-3).into()),
            ("None", serde_json::Value::Null),
            ("True", true.into()),
        ] {
            assert_eq!(literal(text), Some(expected), "{text}")
        }
        for text in [
            "factory()",
            "b'bytes'",
            "[1,2]",
            "2 ** 4",
            "18446744073709551616",
        ] {
            assert_eq!(literal(text), None, "{text}")
        }
    }
    #[test]
    fn admission_requires_one_resolved_descriptor_and_provider_agreement() {
        assert!(descriptor_preserves_body(
            &[Some("builtins.classmethod")],
            &[(true, false, false)]
        ));
        assert!(!descriptor_preserves_body(
            &[Some("local.classmethod")],
            &[(true, false, false)]
        ));
        assert!(!descriptor_preserves_body(
            &[Some("builtins.classmethod"), None],
            &[(true, false, false)]
        ));
        assert!(!descriptor_preserves_body(
            &[Some("builtins.classmethod")],
            &[(true, false, false), (false, false, false)]
        ));
    }
}
