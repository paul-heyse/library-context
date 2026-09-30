//! The owner rule (DESIGN §15.4, core review C12): an occurrence belongs to the innermost
//! declaration whose body holds it, otherwise to its module. This is the only definition of
//! "caller". `OwnerTable` is the batch form used by producing stages; `owner_of` is its per-item
//! oracle. Decorators, default values, parameter and return annotations, base classes and the
//! declared name are not in the body, so they belong to the enclosing owner. Comprehensions are
//! not declarations.
use super::charged::{ChargedMap, StateCharge};
use super::resources::ResourceBudget;
use super::source::{Occurrence, SourceArtifact, SyntaxKind};
use super::{Id, ModelError, Record};
use std::collections::BTreeMap;

/// Declarations that own a body: the module, functions, classes and lambdas.
pub fn owns_body(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::ModModule
            | SyntaxKind::StmtFunctionDef
            | SyntaxKind::StmtClassDef
            | SyntaxKind::ExprLambda
    )
}
fn statement(kind: SyntaxKind) -> bool {
    use SyntaxKind::*;
    matches!(
        kind,
        StmtFunctionDef
            | StmtClassDef
            | StmtReturn
            | StmtDelete
            | StmtTypeAlias
            | StmtAssign
            | StmtAugAssign
            | StmtAnnAssign
            | StmtFor
            | StmtWhile
            | StmtIf
            | StmtWith
            | StmtMatch
            | StmtRaise
            | StmtTry
            | StmtAssert
            | StmtImport
            | StmtImportFrom
            | StmtGlobal
            | StmtNonlocal
            | StmtExpr
            | StmtPass
            | StmtBreak
            | StmtContinue
            | StmtIpyEscapeCommand
    )
}
/// Whether a direct child of an owning declaration lies in its body.
pub fn is_body_child(parent: SyntaxKind, child: SyntaxKind) -> bool {
    match parent {
        SyntaxKind::ModModule => true,
        SyntaxKind::StmtFunctionDef | SyntaxKind::StmtClassDef => statement(child),
        SyntaxKind::ExprLambda => child != SyntaxKind::Parameters,
        _ => false,
    }
}
fn gap() -> ModelError {
    ModelError::Invalid("occurrence structural path has a gap".into())
}

/// Per-item oracle: walk the structural ancestors of `occurrence` inside `all`.
pub fn owner_of(occurrence: &Occurrence, all: &[Occurrence]) -> Result<Id<Occurrence>, ModelError> {
    let by_path: BTreeMap<&[i32], &Occurrence> = all
        .iter()
        .filter(|o| o.source == occurrence.source)
        .map(|o| (o.structural_path.as_slice(), o))
        .collect();
    let path = &occurrence.structural_path;
    if path.len() <= 1 {
        return Ok(occurrence.id());
    }
    for depth in (1..path.len()).rev() {
        let parent = by_path.get(&path[..depth]).ok_or_else(gap)?;
        let child = by_path.get(&path[..depth + 1]).ok_or_else(gap)?;
        if owns_body(parent.syntax_kind) && is_body_child(parent.syntax_kind, child.syntax_kind) {
            return Ok(parent.id());
        }
    }
    Err(ModelError::Invalid(
        "occurrence has no owning module".into(),
    ))
}

/// The owner of every occurrence in a batch, computed by one sorted sweep per source.
pub struct OwnerTable {
    charge: StateCharge,
    owners: ChargedMap<Id<Occurrence>, Id<Occurrence>>,
}
impl OwnerTable {
    pub fn build(occurrences: &[Occurrence], budget: &ResourceBudget) -> Result<Self, ModelError> {
        let mut charge = StateCharge::new(budget, "occurrence-owners");
        charge.grow(occurrences.len().saturating_mul(size_of::<u32>()))?;
        let mut order: Vec<u32> =
            (0..u32::try_from(occurrences.len()).map_err(ModelError::codec)?).collect();
        order.sort_by(|a, b| {
            let (a, b) = (&occurrences[*a as usize], &occurrences[*b as usize]);
            (a.source, &a.structural_path).cmp(&(b.source, &b.structural_path))
        });
        let mut owners = ChargedMap::default();
        // Each frame: (source, structural path, kind, id, the frame's own owner).
        let mut stack: Vec<OwnerFrame<'_>> = Vec::new();
        for index in order {
            let occurrence = &occurrences[index as usize];
            let path = occurrence.structural_path.as_slice();
            while stack.last().is_some_and(|(source, frame, ..)| {
                *source != occurrence.source
                    || !path.starts_with(frame)
                    || frame.len() >= path.len()
            }) {
                if let Some((source, frame, ..)) = stack.last()
                    && *source == occurrence.source
                    && *frame == path
                {
                    return Err(ModelError::Conflict(Occurrence::NAME));
                }
                stack.pop();
            }
            let owner = match stack.last() {
                None if path.len() == 1 => occurrence.id(),
                None => return Err(gap()),
                Some((_, frame, kind, parent, parent_owner)) => {
                    if frame.len() + 1 != path.len() {
                        return Err(gap());
                    }
                    if owns_body(*kind) && is_body_child(*kind, occurrence.syntax_kind) {
                        *parent
                    } else {
                        *parent_owner
                    }
                }
            };
            owners.insert(&mut charge, occurrence.id(), owner)?;
            stack.push((
                occurrence.source,
                path,
                occurrence.syntax_kind,
                occurrence.id(),
                owner,
            ));
        }
        charge.release(occurrences.len().saturating_mul(size_of::<u32>()));
        Ok(Self { charge, owners })
    }
    pub fn owner(&self, occurrence: Id<Occurrence>) -> Option<Id<Occurrence>> {
        self.owners.get(&occurrence).copied()
    }
    pub fn len(&self) -> usize {
        self.owners.len()
    }
    pub fn is_empty(&self) -> bool {
        self.owners.is_empty()
    }
    pub fn reserved(&self) -> usize {
        self.charge.reserved()
    }
}

type OwnerFrame<'a> = (
    Id<SourceArtifact>,
    &'a [i32],
    SyntaxKind,
    Id<Occurrence>,
    Id<Occurrence>,
);
