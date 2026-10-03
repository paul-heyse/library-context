//! Owned static branch decisions from Pyrefly's native AST adapter.
use lctx_model::domain::{ModelError, charged::StateCharge, resources::ResourceBudget, lexical::StaticBranch};
use pyrefly_python::{ast::Ast, sys_info::SysInfo};
use ruff_python_ast::{Expr, StmtIf, ModModule};
use ruff_python_ast::helpers::any_over_expr;
use ruff_text_size::Ranged;
use std::collections::BTreeMap;

pub struct NativeBranches {
    _charge: StateCharge,
    rows: BTreeMap<(u32,u32), Option<(Vec<Option<(u32,u32)>>, Vec<Option<(StaticBranch,bool)>>)>>,
}
impl NativeBranches {
    pub fn observe(ast: &ModModule, sys: &SysInfo, budget: &ResourceBudget) -> Result<Self,ModelError> {
        use ruff_python_ast::statement_visitor::{StatementVisitor,walk_stmt};
        struct Collect<'a> { output: &'a mut NativeBranches, sys: &'a SysInfo, error: Option<ModelError> }
        impl<'t> StatementVisitor<'t> for Collect<'_> {
            fn visit_stmt(&mut self, stmt: &'t ruff_python_ast::Stmt) {
                if self.error.is_some() { return; }
                if let ruff_python_ast::Stmt::If(branch) = stmt {
                    let tests: Vec<_> = Ast::if_branches(branch).map(|(test,_)| test.map(|t| (t.start().to_u32(),t.end().to_u32()))).collect();
                    if let Err(error) = self.output._charge.grow(256+tests.len()*64) { self.error=Some(error); return; }
                    let marks=clause_marks(self.sys,branch);
                    self.output.rows.entry((branch.start().to_u32(),branch.end().to_u32())).and_modify(|value| *value=None).or_insert(Some((tests,marks)));
                }
                walk_stmt(self,stmt);
            }
        }
        let mut output=Self { _charge:StateCharge::new(budget,"native-branch-adapter"),rows:BTreeMap::new() };
        let mut collect=Collect { output:&mut output,sys,error:None };
        collect.visit_body(&ast.body);
        if let Some(error)=collect.error { return Err(error); }
        Ok(output)
    }
    pub fn marks(&self, branch: &ruff_python_ast_latest::StmtIf) -> Vec<Option<(StaticBranch,bool)>> {
        use ruff_text_size_latest::Ranged;
        let tests: Vec<_> = std::iter::once(Some((branch.test.start().to_u32(),branch.test.end().to_u32())))
            .chain(branch.elif_else_clauses.iter().map(|clause| clause.test.as_ref().map(|t| (t.start().to_u32(),t.end().to_u32())))).collect();
        match self.rows.get(&(branch.start().to_u32(),branch.end().to_u32())) {
            Some(Some((native_tests,marks))) if native_tests == &tests => marks.clone(),
            _ => vec![None;tests.len()],
        }
    }
}

/// Each clause of an `if` statement as Pyrefly decides it (H1 C1): `SysInfo::evaluate_bool` per
/// clause, applied as `SysInfo::pruned_if_branches` applies it. `Some((kind, kept))`: the clause is
/// statically decided, and Pyrefly analyzes it (`kept`) or prunes it; `None`: it depends on the
/// runtime. An `else` is decided when every earlier test was decided false, and every clause after
/// one decided true is pruned. The kind is the deciding test's (the true one's, for the clauses it
/// prunes).
pub(crate) fn clause_marks(sys: &SysInfo, i: &StmtIf) -> Vec<Option<(StaticBranch, bool)>> {
    let mut marks = Vec::new();
    let mut taken: Option<StaticBranch> = None;
    let mut all_false = true;
    let mut last_false: Option<StaticBranch> = None;
    for (test, _) in Ast::if_branches(i) {
        let mark = if let Some(kind) = taken {
            Some((kind, false))
        } else {
            match test {
                None if all_false => last_false.map(|kind| (kind, true)),
                None => None,
                Some(test) => match sys.evaluate_bool(test) {
                    Some(holds) => {
                        let kind = static_kind(test);
                        if holds {
                            taken = Some(kind);
                        } else {
                            last_false = Some(kind);
                        }
                        Some((kind, holds))
                    }
                    None => {
                        all_false = false;
                        None
                    }
                },
            }
        };
        marks.push(mark);
    }
    marks
}

/// What a decided test reads: the kind is from the expression tree, never its text.
fn static_kind(test: &Expr) -> StaticBranch {
    let named = |e: &Expr, module: &str| matches!(e, Expr::Name(n) if n.id.as_str() == module);
    let checking = any_over_expr(test, |e| match e {
        Expr::Name(n) => SysInfo::is_type_checking_constant_name(n.id.as_str()),
        Expr::Attribute(a) => {
            a.value.is_name_expr() && SysInfo::is_type_checking_constant_name(a.attr.as_str())
        }
        _ => false,
    });
    let version = any_over_expr(
        test,
        |e| matches!(e, Expr::Attribute(a) if a.attr.as_str() == "version_info" && named(&a.value, "sys")),
    );
    let platform = any_over_expr(test, |e| {
        matches!(e, Expr::Attribute(a)
            if (a.attr.as_str() == "platform" && named(&a.value, "sys"))
                || (a.attr.as_str() == "name" && named(&a.value, "os")))
    });
    match (checking, version, platform) {
        (true, false, false) => StaticBranch::TypeChecking,
        (false, true, false) => StaticBranch::VersionInfo,
        (false, false, true) => StaticBranch::Platform,
        (false, false, false) => StaticBranch::Constant,
        _ => StaticBranch::Combined,
    }
}

