//! The lexical records of one module (cutover plan A5): the recognizer runs over the typed
//! occurrences the traversal emitted, and its facts become the model's scopes, binding events,
//! binding and reference observations, targets and resolutions. A resolution with several
//! candidates, or one an unknown star import may bind, is qualified as a candidate.
use crate::native_branches::NativeBranches;
use crate::{
    lexical::{Lexical, LexicalFacts, Outside, Stars, Target},
    syntax_records::Spans,
};
use lctx_model::domain::{
    Id, ModelError, Record, assertion::AssertionQualification, lexical::*, source::Occurrence,
};
use ruff_python_ast_latest::visitor::source_order::{self, SourceOrderVisitor, TraversalSignal};
use ruff_python_ast_latest::{AnyNodeRef, Expr, ModModule};
use ruff_text_size_latest::Ranged;

/// Run the recognizer over one module's typed occurrences.
pub fn facts(
    ast: &ModModule,
    spans: &Spans,
    decisions: &NativeBranches,
    outside: &Outside,
    stars: &Stars,
) -> Result<LexicalFacts<Id<Occurrence>>, ModelError> {
    let module = spans.get(
        ast.range(),
        lctx_model::domain::source::SyntaxKind::ModModule,
    )?;
    let mut driver = Driver {
        spans,
        lexical: Lexical::new(module, ast.range(), decisions, outside, stars),
        entered: vec![],
        annotation: 0,
        error: None,
    };
    driver.visit_body(&ast.body);
    match driver.error {
        Some(error) => Err(error),
        None => Ok(driver.lexical.finish()),
    }
}
struct Driver<'a, 'b> {
    spans: &'a Spans,
    lexical: Lexical<'b, Id<Occurrence>>,
    entered: Vec<bool>,
    annotation: usize,
    error: Option<ModelError>,
}
impl<'t> SourceOrderVisitor<'t> for Driver<'_, '_> {
    fn enter_node(&mut self, node: AnyNodeRef<'t>) -> TraversalSignal {
        if self.error.is_some() {
            self.entered.push(false);
            return TraversalSignal::Skip;
        }
        let found = self
            .spans
            .get(node.range(), crate::typed_syntax::kind(node.kind()))
            .and_then(|id| {
                self.spans
                    .parent(id)
                    .map(|parent| (id, parent))
                    .ok_or_else(|| {
                        ModelError::Invalid(
                            "an occurrence was emitted without its placement".into(),
                        )
                    })
            });
        match found {
            Ok((id, parent)) => {
                let own = matches!(
                    node,
                    AnyNodeRef::StmtFunctionDef(_) | AnyNodeRef::StmtClassDef(_)
                )
                .then_some(id);
                self.lexical
                    .enter(node, id, own, None, parent, self.annotation > 0);
                self.entered.push(true);
                TraversalSignal::Traverse
            }
            Err(error) => {
                self.error = Some(error);
                self.entered.push(false);
                TraversalSignal::Skip
            }
        }
    }
    fn leave_node(&mut self, node: AnyNodeRef<'t>) {
        if self.entered.pop() == Some(true) {
            self.lexical.leave(node);
        }
    }
    fn visit_annotation(&mut self, expr: &'t Expr) {
        self.annotation += 1;
        source_order::walk_annotation(self, expr);
        self.annotation -= 1;
    }
}

/// One module's lexical records, qualified definitely or as candidates.
#[derive(Debug, Default)]
pub struct LexicalRecords {
    pub scopes: Vec<LexicalScope>,
    pub scope_observations: Vec<LexicalScopeObservation>,
    pub events: Vec<BindingEvent>,
    pub bindings: Vec<BindingObservation>,
    pub references: Vec<ReferenceObservation>,
    pub targets: Vec<LexicalTarget>,
    pub resolutions: Vec<LexicalResolution>,
}
/// The records of `facts`, under `definite`, or `candidate` for a candidate resolution.
pub fn records(
    facts: &LexicalFacts<Id<Occurrence>>,
    spans: &Spans,
    definite: Id<AssertionQualification>,
    candidate: Id<AssertionQualification>,
) -> Result<LexicalRecords, ModelError> {
    let scopes: Vec<LexicalScope> = facts
        .scopes
        .iter()
        .map(|s| LexicalScope {
            owner: s.owner,
            kind: s.kind,
        })
        .collect();
    let events: Vec<BindingEvent> = facts
        .binds
        .iter()
        .map(|b| BindingEvent {
            site: b.site,
            name: b.name.clone(),
        })
        .collect();
    let mut out = LexicalRecords::default();
    for (scope, fact) in scopes.iter().zip(&facts.scopes) {
        out.scope_observations.push(LexicalScopeObservation {
            qualification: definite,
            scope: scope.id(),
            parent: fact.parent.map(|p| scopes[p].id()),
        });
    }
    for (event, bind) in events.iter().zip(&facts.binds) {
        out.bindings.push(BindingObservation {
            qualification: definite,
            event: event.id(),
            scope: scopes[bind.scope].id(),
            kind: bind.kind,
            ordinal: bind.ordinal,
            value: bind
                .value
                .map(|(range, kind)| spans.get(range, kind))
                .transpose()?,
            static_branch: bind.branch.map(|b| b.0),
            static_polarity: bind.branch.map(|b| b.1),
        });
    }
    for (reference, (resolutions, is_candidate)) in facts.refs.iter().zip(&facts.resolutions) {
        out.references.push(ReferenceObservation {
            qualification: definite,
            read: reference.read,
            scope: scopes[reference.scope].id(),
            parent: reference.parent.0,
            field: reference.parent.1,
            name: reference.name.clone(),
        });
        for resolution in resolutions {
            let target = match &resolution.target {
                Target::Binding(index) => LexicalTarget::Binding {
                    event: events[*index].id(),
                },
                Target::Builtin { name, variable } => LexicalTarget::Builtin {
                    name: name.clone(),
                    variable: *variable,
                },
                Target::Unresolved(reason) => LexicalTarget::Unresolved { reason: *reason },
            };
            out.resolutions.push(LexicalResolution {
                qualification: if *is_candidate { candidate } else { definite },
                read: reference.read,
                target: target.id(),
                captured: resolution.captured,
            });
            out.targets.push(target);
        }
    }
    out.scopes = scopes;
    out.events = events;
    Ok(out)
}
