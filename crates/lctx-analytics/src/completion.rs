//! Completion certificates describe what happens if a statement is entered, never that it ran.
//! Source observations and expression proofs are explicit inputs. No provider or store is read.
use std::collections::{HashMap, BTreeSet};
use cpg_schema::behavior::{ExpressionEvaluationsRow, ExpressionEvaluationStepsRow, ExitSitesRow, ReturnExitStatusesRow,
    ReturnExitStepsRow, StatementCompletionsRow, StatementCompletionStepsRow};
use cpg_schema::codebook::{BindingKind, BoundaryReason, Codebook, CompletionKind as C,
    ExitSiteKind, LexicalScopeKind, SummaryFlowStepKind as K, SyntaxField as F, SyntaxKind as S};
use cpg_schema::id::Id;
use cpg_schema::tables::{SyntaxNodesRow, BindingsRow, ScopesRow};

const MAX_DEPTH: usize = 128;
const MAX_WORK: usize = 4096;
type Proof = Vec<(K, Id)>;
type Result = std::result::Result<(C, Option<Id>), BoundaryReason>;

pub struct Inputs<'a> {
    pub syntax: &'a [SyntaxNodesRow],
    pub expressions: &'a [ExpressionEvaluationsRow],
    pub expression_steps: &'a [ExpressionEvaluationStepsRow],
    pub bindings: &'a [BindingsRow],
    pub scopes: &'a [ScopesRow],
    pub exits: &'a [ExitSitesRow],
}

#[derive(Default)]
pub struct Outcome {
    pub statements: Vec<StatementCompletionsRow>,
    pub statement_steps: Vec<StatementCompletionStepsRow>,
    pub returns: Vec<ReturnExitStatusesRow>,
    pub return_steps: Vec<ReturnExitStepsRow>,
}

struct Kernel<'a> {
    children: HashMap<(Id, Id), Vec<&'a SyntaxNodesRow>>,
    expressions: HashMap<(Id, Id), &'a ExpressionEvaluationsRow>,
    expression_steps: HashMap<(Id, Id), Vec<&'a ExpressionEvaluationStepsRow>>,
    writes: HashMap<(Id, Id), Vec<&'a BindingsRow>>,
    initializations: BTreeSet<Id>,
    scopes: HashMap<(Id, Id), &'a ScopesRow>,
    remaining: usize,
    proof: Proof,
}

impl Kernel<'_> {
    fn field<'a>(children: &[&'a SyntaxNodesRow], field: F) -> std::result::Result<&'a SyntaxNodesRow, BoundaryReason> {
        let mut nodes = children.iter().filter(|n| n.field == field);
        let node = *nodes.next().ok_or(BoundaryReason::MissingEvidence)?;
        if nodes.next().is_some() { return Err(BoundaryReason::MissingEvidence); }
        Ok(node)
    }

    fn expression(&mut self, node: &SyntaxNodesRow) -> std::result::Result<Option<bool>, BoundaryReason> {
        let row = self.expressions.get(&(node.snapshot_id, node.fact_id)).ok_or(BoundaryReason::MissingEvidence)?;
        self.remaining = self.remaining.checked_sub(usize::try_from(row.work).map_err(|_| BoundaryReason::MissingEvidence)?)
            .ok_or(BoundaryReason::CompletionWorkLimit)?;
        if !row.normal { return Err(row.reason.unwrap_or(BoundaryReason::MissingEvidence)); }
        let steps=self.expression_steps.get(&(node.snapshot_id,node.fact_id)).ok_or(BoundaryReason::MissingEvidence)?;
        self.remaining=self.remaining.checked_sub(steps.len()).ok_or(BoundaryReason::CompletionWorkLimit)?;
        if steps.is_empty() || steps.iter().enumerate().any(|(i,s)|s.ordinal != i as i64)
            || steps.last().is_none_or(|s|s.operand_fact_id != node.fact_id || Some(s.evidence_id) != row.evidence_id || s.status != row.status) {
            return Err(BoundaryReason::MissingEvidence);
        }
        self.proof.extend(steps.iter().map(|s|(s.kind,s.evidence_id)));
        Ok(row.boolean_value)
    }

    fn suite(&mut self, nodes: &[&SyntaxNodesRow], field: F, depth: usize) -> Result {
        let mut statements: Vec<_> = nodes.iter().copied().filter(|n| n.field == field).collect();
        statements.sort_by_key(|n| (n.ordinal, n.fact_id));
        if statements.iter().enumerate().any(|(i,n)| n.ordinal != i as i64) { return Err(BoundaryReason::MissingEvidence); }
        for statement in statements {
            let outcome = self.statement(statement, depth + 1)?;
            if outcome.0 != C::Normal { return Ok(outcome); }
        }
        Ok((C::Normal, None))
    }

    fn statement(&mut self, node: &SyntaxNodesRow, depth: usize) -> Result {
        self.remaining = self.remaining.checked_sub(1).ok_or(BoundaryReason::CompletionWorkLimit)?;
        if depth > MAX_DEPTH { return Err(BoundaryReason::CompletionDepthLimit); }
        let children = self.children.get(&(node.snapshot_id, node.node_id)).cloned().unwrap_or_default();
        if children.len() > self.remaining { return Err(BoundaryReason::CompletionWorkLimit); }
        self.remaining -= children.len();
        if children.iter().any(|child| child.owner_node_id != node.owner_node_id
            || child.module_node_id != node.module_node_id || child.start_byte < node.start_byte
            || child.end_byte > node.end_byte) { return Err(BoundaryReason::MissingEvidence); }
        let outcome = match node.kind {
            S::StmtPass if children.is_empty() => {
                self.proof.push((K::FinalizerPass, node.fact_id));
                return Ok((C::Normal, None));
            },
            S::StmtExpr if children.len() == 1 => {
                self.expression(Self::field(&children, F::Value)?)?;
                (C::Normal, None)
            },
            S::StmtAssign => {
                self.expression(Self::field(&children, F::Value)?)?;
                let targets: Vec<_> = children.iter().filter(|n| n.field == F::Target).collect();
                if targets.is_empty() || targets.len() + 1 != children.len() { return Err(BoundaryReason::UnsupportedControlFlow); }
                for target in targets {
                    if target.kind != S::ExprName { return Err(BoundaryReason::UnsupportedControlFlow); }
                    let bindings = self.writes.get(&(node.snapshot_id, target.node_id)).ok_or(BoundaryReason::MissingEvidence)?;
                    if bindings.len() != 1 { return Err(BoundaryReason::MissingEvidence); }
                    let binding = bindings[0];
                    let scope = self.scopes.get(&(node.snapshot_id, binding.scope_id)).ok_or(BoundaryReason::MissingEvidence)?;
                    if binding.kind != BindingKind::Assignment || scope.kind != LexicalScopeKind::Function
                        || Some(scope.owner_node_id) != node.owner_node_id
                        || !self.initializations.contains(&binding.fact_id) { return Err(BoundaryReason::UnsupportedControlFlow); }
                    self.proof.push((K::LocalAssignmentBinding, binding.fact_id));
                }
                (C::Normal, None)
            },
            S::StmtReturn => {
                if !children.is_empty() {
                    if children.len() != 1 { return Err(BoundaryReason::UnsupportedControlFlow); }
                    self.expression(Self::field(&children, F::Value)?)?;
                }
                (C::Return, Some(node.fact_id))
            },
            S::StmtRaise => {
                // An arbitrary exception class is instantiated by `raise` and its constructor
                // may not complete. Only exact non-exception primitive operands establish an
                // immediate TypeError here; an opaque normal read is insufficient.
                let exception=Self::field(&children, F::Exc)?;
                self.expression(exception)?;
                if !matches!(exception.kind, S::ExprNoneLiteral | S::ExprBooleanLiteral |
                    S::ExprNumberLiteral | S::ExprStringLiteral | S::ExprBytesLiteral | S::ExprEllipsisLiteral) {
                    return Err(BoundaryReason::UnsupportedControlFlow);
                }
                if children.iter().any(|n| !matches!(n.field, F::Exc | F::Cause)) { return Err(BoundaryReason::UnsupportedControlFlow); }
                if children.iter().any(|n| n.field == F::Cause) { self.expression(Self::field(&children, F::Cause)?)?; }
                (C::Raise, Some(node.fact_id))
            },
            S::StmtBreak if children.is_empty() => (C::Break, Some(node.fact_id)),
            S::StmtContinue if children.is_empty() => (C::Continue, Some(node.fact_id)),
            S::StmtIf => {
                let truth = self.expression(Self::field(&children, F::Test)?)?.ok_or(BoundaryReason::UnsupportedControlFlow)?;
                if truth { self.suite(&children, F::Body, depth)? }
                else {
                    let mut result = (C::Normal, None);
                    for clause in children.iter().filter(|n| n.field == F::Orelse) {
                        if clause.kind != S::ElifElseClause { return Err(BoundaryReason::UnsupportedControlFlow); }
                        let nodes = self.children.get(&(node.snapshot_id, clause.node_id)).cloned().unwrap_or_default();
                        self.remaining = self.remaining.checked_sub(1 + nodes.len()).ok_or(BoundaryReason::CompletionWorkLimit)?;
                        let selected = if nodes.iter().any(|n| n.field == F::Test) {
                            self.expression(Self::field(&nodes, F::Test)?)?.ok_or(BoundaryReason::UnsupportedControlFlow)?
                        } else { true };
                        if selected { result = self.suite(&nodes, F::Body, depth)?; break; }
                    }
                    result
                }
            },
            S::StmtTry => {
                let mut pending = self.suite(&children, F::Body, depth)?;
                if pending.0 == C::Raise && children.iter().any(|n| n.field == F::Handler) {
                    // A bare first handler catches every ordinary explicit raise. A typed
                    // handler requires independently resolved exception matching; spelling
                    // alone does not establish it. Exception groups remain unsupported.
                    let handlers: Vec<_> = children.iter().filter(|n| n.field == F::Handler).collect();
                    if handlers.iter().enumerate().any(|(i,h)|h.ordinal != i as i64)
                        || handlers[0].kind != S::ExceptHandlerExceptHandler {
                        return Err(BoundaryReason::MissingEvidence);
                    }
                    let handler = handlers[0];
                    let body = self.children.get(&(node.snapshot_id,handler.node_id)).cloned().unwrap_or_default();
                    if body.iter().any(|n| n.field != F::Body) { return Err(BoundaryReason::UnsupportedControlFlow); }
                    self.remaining=self.remaining.checked_sub(1+body.len()).ok_or(BoundaryReason::CompletionWorkLimit)?;
                    self.proof.push((K::CompletionStatement,handler.fact_id));
                    pending=self.suite(&body,F::Body,depth+1)?;
                } else if pending.0 == C::Normal {
                    pending = self.suite(&children, F::Orelse, depth)?;
                }
                let finalizer = self.suite(&children, F::Finalbody, depth)?;
                if finalizer.0 == C::Normal { pending } else { finalizer }
            },
            _ => return Err(BoundaryReason::UnsupportedControlFlow),
        };
        self.proof.push((K::CompletionStatement, node.fact_id));
        Ok(outcome)
    }
}

pub fn complete(inputs: Inputs<'_>) -> Outcome {
    let mut children: HashMap<_, Vec<_>> = HashMap::new();
    let mut nodes = HashMap::new();
    for node in inputs.syntax {
        children.entry((node.snapshot_id, node.parent_node_id)).or_default().push(node);
        nodes.insert((node.snapshot_id, node.node_id), node);
    }
    for group in children.values_mut() { group.sort_by_key(|n| (n.ordinal,n.start_byte,n.fact_id)); }
    let mut writes: HashMap<_, Vec<_>> = HashMap::new();
    for binding in inputs.bindings { writes.entry((binding.snapshot_id,binding.site_node_id)).or_default().push(binding); }
    // Rebinding can release the last reference and execute an arbitrary __del__. A unique
    // lexical initialization cannot replace an earlier parameter, assignment or handler value.
    let mut by_name: HashMap<_,Vec<_>> = HashMap::new();
    for binding in inputs.bindings { by_name.entry((binding.snapshot_id,binding.scope_id,&binding.name)).or_default().push(binding); }
    let initializations=by_name.values().filter(|group|group.len()==1).filter_map(|group| {
        let binding=group[0];
        let mut current=nodes.get(&(binding.snapshot_id,binding.site_node_id)).copied();
        let mut seen=BTreeSet::new();
        for _ in 0..MAX_DEPTH {
            let node=current?;
            if !seen.insert(node.node_id) || matches!(node.kind,S::StmtFor|S::StmtWhile) { return None; }
            if node.owner_node_id.is_none() || node.kind==S::StmtFunctionDef { return Some(binding.fact_id); }
            current=nodes.get(&(binding.snapshot_id,node.parent_node_id)).copied();
        }
        None
    }).collect();
    let mut expression_steps: HashMap<_,Vec<_>> = HashMap::new();
    for step in inputs.expression_steps { expression_steps.entry((step.snapshot_id,step.syntax_fact_id)).or_default().push(step); }
    for steps in expression_steps.values_mut() { steps.sort_by_key(|s|s.ordinal); }
    let mut kernel = Kernel { children, writes, initializations, expression_steps, remaining: MAX_WORK, proof: Vec::new(),
        expressions: inputs.expressions.iter().map(|e| ((e.snapshot_id,e.syntax_fact_id),e)).collect(),
        scopes: inputs.scopes.iter().map(|s| ((s.snapshot_id,s.node_id),s)).collect() };
    let mut out = Outcome::default();
    for node in inputs.syntax.iter().filter(|n| n.owner_node_id.is_some() && n.kind.text().starts_with("stmt_")) {
        kernel.remaining = MAX_WORK; kernel.proof.clear();
        let result = kernel.statement(node,0);
        let (kind,terminal_fact_id,reason) = match result { Ok((kind,terminal)) => (kind,terminal,None),
            Err(reason) => (C::Unknown,None,Some(reason)) };
        out.statements.push(StatementCompletionsRow { snapshot_id: node.snapshot_id, source_fact_id: node.fact_id,
            statement_node_id: node.node_id, function_node_id: node.owner_node_id.unwrap(),
            kind,terminal_fact_id,reason,work: (MAX_WORK-kernel.remaining).max(1) as i64 });
        if reason.is_none() {
            out.statement_steps.extend(kernel.proof.iter().enumerate().map(|(ordinal,&(kind,evidence_id))|
                StatementCompletionStepsRow { snapshot_id:node.snapshot_id,source_fact_id:node.fact_id,
                    ordinal:ordinal as i64,evidence_id,kind }));
        }
    }
    for exit in inputs.exits.iter().filter(|e| e.kind == ExitSiteKind::Return) {
        kernel.remaining = MAX_WORK; kernel.proof.clear();
        let mut row = ReturnExitStatusesRow { snapshot_id:exit.snapshot_id,function_node_id:exit.function_node_id,
            site_node_id:exit.site_node_id,source_fact_id:exit.source_fact_id,condition_id:exit.condition_id,
            walk_depth:0,frame_node_id:None,frame_fact_id:None,pass_node_id:None,pass_fact_id:None,reason:None };
        let mut current = nodes.get(&(exit.snapshot_id,exit.site_node_id)).copied();
        let mut visited = BTreeSet::new();
        let mut frames = Vec::new();
        while let Some(node) = current {
            if !visited.insert(node.node_id) { row.reason=Some(BoundaryReason::MissingEvidence); break; }
            let Some(parent) = nodes.get(&(exit.snapshot_id,node.parent_node_id)).copied()
                .filter(|p| p.owner_node_id == Some(exit.function_node_id)) else { break; };
            if row.walk_depth as usize >= MAX_DEPTH { row.reason=Some(BoundaryReason::CompletionDepthLimit); break; }
            let actions = kernel.children.get(&(exit.snapshot_id,parent.node_id)).cloned().unwrap_or_default();
            if parent.kind == S::StmtWith || (parent.kind == S::StmtTry && node.field != F::Finalbody
                && actions.iter().any(|n| n.field == F::Finalbody)) {
                if row.frame_node_id.is_none() { row.frame_node_id=Some(parent.node_id); row.frame_fact_id=Some(parent.fact_id); }
                frames.push(parent.node_id);
                if parent.kind == S::StmtWith { row.reason=Some(BoundaryReason::UnsupportedControlFlow); break; }
                match kernel.suite(&actions,F::Finalbody,0) {
                    Ok((C::Normal,_)) => {},
                    Ok(_) => { row.reason=Some(BoundaryReason::UnsupportedControlFlow); break; },
                    Err(reason) => { row.reason=Some(reason); break; },
                }
                let passes: Vec<_> = actions.iter().filter(|a| a.field==F::Finalbody).collect();
                if frames.len()==1 && passes.len()==1 && passes[0].kind==S::StmtPass {
                    row.pass_node_id=Some(passes[0].node_id); row.pass_fact_id=Some(passes[0].fact_id);
                } else { row.pass_node_id=None; row.pass_fact_id=None; }
            }
            row.walk_depth+=1; current=Some(parent);
        }
        if current.is_none() { row.reason=Some(BoundaryReason::MissingEvidence); }
        if row.reason.is_none() {
            out.return_steps.extend(kernel.proof.iter().enumerate().map(|(ordinal,&(kind,evidence_id))|
                ReturnExitStepsRow {snapshot_id:exit.snapshot_id,return_site_fact_id:exit.source_fact_id,
                    ordinal:ordinal as i64,evidence_id,kind,condition_id:exit.condition_id}));
        }
        out.returns.push(row);
    }
    out.statements.sort_by_key(|r|(r.snapshot_id,r.source_fact_id));
    out.statement_steps.sort_by_key(|r|(r.snapshot_id,r.source_fact_id,r.ordinal));
    out.returns.sort_by_key(|r|(r.snapshot_id,r.site_node_id));
    out.return_steps.sort_by_key(|r|(r.snapshot_id,r.return_site_fact_id,r.ordinal));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evaluation::{evaluate, EvaluationInputs};
    fn id(value:u8) -> Id { Id([value;16]) }
    fn node(value:u8,parent:u8,kind:S,field:F,ordinal:i64,detail:&str) -> SyntaxNodesRow {
        SyntaxNodesRow { snapshot_id:id(1), fact_id:id(value), node_id:id(value),
            module_node_id:id(2), owner_node_id:Some(id(3)), parent_node_id:id(parent),
            kind,field,ordinal,start_byte:0,end_byte:100,detail:Some(detail.to_owned()) }
    }
    fn run(nodes:&[SyntaxNodesRow], exits:&[ExitSitesRow]) -> Outcome {
        let expressions=evaluate(EvaluationInputs {syntax:nodes,..Default::default()});
        complete(Inputs {syntax:nodes,expressions:&expressions.evaluations,expression_steps:&expressions.steps,bindings:&[],scopes:&[],exits})
    }
    fn exit(site:u8) -> ExitSitesRow {
        ExitSitesRow { snapshot_id:id(1),function_node_id:id(3),site_node_id:id(site),
            kind:ExitSiteKind::Return,module_node_id:id(2),source_fact_id:id(site),
            region_fact_id:id(90),start_byte:0,end_byte:100,condition_id:id(91),approximated:false }
    }
    #[test]
    fn finalizer_outcome_preserves_or_replaces_pending_return() {
        let mut nodes=vec![node(10,3,S::StmtTry,F::Body,0,""),
            node(11,10,S::StmtReturn,F::Body,0,""),
            node(12,10,S::StmtPass,F::Finalbody,0,"")];
        let out=run(&nodes,&[exit(11)]);
        assert_eq!(out.statements.iter().find(|r|r.source_fact_id==id(10)).map(|r|(r.kind,r.terminal_fact_id)),Some((C::Return,Some(id(11)))));
        assert_eq!(out.returns[0].reason,None);
        assert_eq!(out.return_steps.iter().map(|r|r.evidence_id).collect::<Vec<_>>(),[id(12)]);
        nodes[2].kind=S::StmtReturn;
        let out=run(&nodes,&[exit(11),exit(12)]);
        assert_eq!(out.statements.iter().find(|r|r.source_fact_id==id(10)).map(|r|r.terminal_fact_id),Some(Some(id(12))));
        assert_eq!(out.returns[0].reason,Some(BoundaryReason::UnsupportedControlFlow));
        assert_eq!(out.returns[1].reason,None);
        assert!(out.return_steps.is_empty());
        nodes[2].kind=S::StmtRaise;
        nodes.push(node(13,12,S::ExprNoneLiteral,F::Exc,0,"None"));
        let out=run(&nodes,&[exit(11)]);
        assert_eq!(out.statements.iter().find(|r|r.source_fact_id==id(10)).map(|r|r.kind),Some(C::Raise));
        assert!(out.returns[0].reason.is_some());
    }
    #[test]
    fn selected_branch_skips_unknown_and_nested_frames_unwind_inside_out() {
        let nodes=vec![node(10,3,S::StmtTry,F::Body,0,""),
            node(11,10,S::StmtTry,F::Body,0,""),
            node(12,11,S::StmtReturn,F::Body,0,""),
            node(13,11,S::StmtPass,F::Finalbody,0,""),
            node(14,10,S::StmtIf,F::Finalbody,0,""),
            node(15,14,S::ExprBooleanLiteral,F::Test,0,"True"),
            node(16,14,S::StmtPass,F::Body,0,""),
            node(17,14,S::ElifElseClause,F::Orelse,0,""),
            node(18,17,S::StmtExpr,F::Body,0,""),
            node(19,18,S::ExprName,F::Value,0,"unbound")];
        let out=run(&nodes,&[exit(12)]);
        assert_eq!(out.returns[0].reason,None);
        assert_eq!(out.return_steps.iter().map(|r|r.evidence_id).collect::<Vec<_>>(),[id(13),id(15),id(16),id(14)]);
        let mut reversed=nodes.clone(); reversed.reverse();
        let shuffled=run(&reversed,&[exit(12)]);
        assert_eq!(out.statements,shuffled.statements);
        assert_eq!(out.return_steps,shuffled.return_steps);
        let mut selected_unknown=nodes;
        selected_unknown[5].detail=Some("False".to_owned());
        assert!(run(&selected_unknown,&[exit(12)]).returns[0].reason.is_some());
    }
    #[test]
    fn malformed_suites_and_unproved_writes_are_unknown() {
        let mut nodes=vec![node(10,3,S::StmtTry,F::Body,0,""),
            node(11,10,S::StmtReturn,F::Body,0,""),
            node(12,10,S::StmtPass,F::Finalbody,1,"")];
        assert_eq!(run(&nodes,&[exit(11)]).returns[0].reason,Some(BoundaryReason::MissingEvidence));
        nodes[2]=node(12,10,S::StmtAssign,F::Finalbody,0,"");
        nodes.extend([node(13,12,S::ExprNumberLiteral,F::Value,0,"1"),node(14,12,S::ExprName,F::Target,0,"marker")]);
        assert_eq!(run(&nodes,&[exit(11)]).returns[0].reason,Some(BoundaryReason::MissingEvidence));
    }
    #[test]
    fn a_caught_raise_does_not_execute_else_and_bare_reraise_is_unknown() {
        let mut nodes=vec![node(10,3,S::StmtTry,F::Body,0,""),
            node(11,10,S::StmtRaise,F::Body,0,""),
            node(12,11,S::ExprNoneLiteral,F::Exc,0,"None"),
            node(13,10,S::ExceptHandlerExceptHandler,F::Handler,0,""),
            node(14,13,S::StmtPass,F::Body,0,""),
            node(15,10,S::StmtReturn,F::Orelse,0,""),
            node(16,10,S::StmtPass,F::Finalbody,0,"")];
        let out=run(&nodes,&[]);
        assert_eq!(out.statements.iter().find(|r|r.source_fact_id==id(10)).map(|r|r.kind),Some(C::Normal));
        assert!(!out.statement_steps.iter().any(|r|r.source_fact_id==id(10)&&r.evidence_id==id(15)));
        nodes[4].kind=S::StmtRaise;
        let out=run(&nodes,&[]);
        assert_eq!(out.statements.iter().find(|r|r.source_fact_id==id(10)).map(|r|r.kind),Some(C::Unknown));
    }
    #[test]
    fn production_completion_depth_and_work_limits_remain_distinct() {
        let mut nodes:Vec<_>=(10..=140).map(|i|node(i,if i==10 {3} else {i-1},S::StmtTry,F::Body,0,"")).collect();
        nodes.push(node(141,140,S::StmtPass,F::Body,0,""));
        let out=run(&nodes,&[]);
        assert_eq!(out.statements.iter().find(|r|r.source_fact_id==id(10)).unwrap().reason,Some(BoundaryReason::CompletionDepthLimit));
        let mut nodes=vec![node(10,3,S::StmtTry,F::Body,0,"")];
        for ordinal in 0..MAX_WORK {
            let mut statement=node(11,10,S::StmtPass,F::Finalbody,ordinal as i64,"");
            let mut bytes=[0;16];bytes[..8].copy_from_slice(&(ordinal as u64).to_le_bytes());bytes[15]=1;
            statement.fact_id=Id(bytes);statement.node_id=Id(bytes);nodes.push(statement);
        }
        let out=run(&nodes,&[]);
        assert_eq!(out.statements.iter().find(|r|r.source_fact_id==id(10)).unwrap().reason,Some(BoundaryReason::CompletionWorkLimit));
        assert!(!out.statement_steps.iter().any(|r|r.source_fact_id==id(10)));
    }
}
