//! Source-anchored lexical vocabulary and separately attributed scoping/resolution assertions.
//! No normalized entity, display-name join or provider-local index identifies a source event.
use crate::{Assertion,Domain,DomainCode,DomainSum};
use super::{*, assertion::AssertionQualification, attribution::FactFamily, source::Occurrence};

#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash,DomainCode)]
#[repr(i16)]
pub enum LexicalScopeKind { Module = 0, Class = 1, Function = 2, Lambda = 3, Comprehension = 4 }
#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash,DomainCode)]
#[repr(i16)]
pub enum BindingEventKind {
    FunctionDef = 0, ClassDef = 1, Parameter = 2, Assignment = 3, AugAssignment = 4,
    AnnotationOnly = 5, ForTarget = 6, WithTarget = 7, ExceptHandler = 8, Import = 9,
    FromImport = 10, StarImport = 11, Walrus = 12, ComprehensionTarget = 13,
    MatchCapture = 14, Del = 15, Global = 16, Nonlocal = 17, TypeAlias = 18,
    TypeParam = 19, Implicit = 20, ImportFromSubmodule = 21,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash,DomainCode)]
#[repr(i16)]
pub enum StaticBranch { TypeChecking = 0, VersionInfo = 1, Platform = 2, Constant = 3, Combined = 4 }
#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash,DomainCode)]
#[repr(i16)]
pub enum SyntaxField {
    Body = 0, Orelse = 1, Test = 2, Handler = 3, Finalbody = 4, Target = 5,
    Value = 6, Exc = 7, Cause = 8, Msg = 9, Subject = 10, Case = 11, Guard = 12,
    Iter = 13, Item = 14, Annotation = 15, Left = 16, Right = 17, Operand = 18,
    Slice = 19, Callee = 20, Argument = 21, Decorator = 22, Element = 23, Child = 24, Default = 25,
}

#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name = "lexical_scopes", invariants = lexical_invariants)]
pub struct LexicalScope {
    #[model(key)] pub owner: Id<Occurrence>,
    #[model(key)] pub kind: LexicalScopeKind,
}
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name = "binding_events", validate = validate_event)]
pub struct BindingEvent {
    #[model(key)] pub site: Id<Occurrence>,
    #[model(key)] pub name: String,
}
fn validate_event(row: &BindingEvent) -> Result<(),ModelError> {
    if row.name.is_empty() { return Err(ModelError::Invalid("binding event needs a name".into())); }
    Ok(())
}

#[derive(Debug,Clone,PartialEq,Eq,Domain,Assertion)]
#[model(name = "lexical_scope_observations", validate = validate_scope)]
#[assertion(support = LexicalScopeSupport, name = "lexical_scope_supports", family = FactFamily::Lexical, subjects(scope, parent))]
pub struct LexicalScopeObservation {
    #[model(key)] pub qualification: Id<AssertionQualification>,
    #[model(key)] pub scope: Id<LexicalScope>,
    #[model(key)] pub parent: Option<Id<LexicalScope>>,
}
fn validate_scope(row: &LexicalScopeObservation) -> Result<(),ModelError> {
    if row.parent == Some(row.scope) { return Err(ModelError::Invalid("lexical scope cannot enclose itself".into())); }
    Ok(())
}

#[derive(Debug,Clone,PartialEq,Eq,Domain,Assertion)]
#[model(name = "binding_observations", validate = validate_binding)]
#[assertion(support = BindingSupport, name = "binding_supports", family = FactFamily::Lexical, subjects(event, scope, value))]
pub struct BindingObservation {
    #[model(key)] pub qualification: Id<AssertionQualification>,
    #[model(key)] pub event: Id<BindingEvent>,
    #[model(key)] pub scope: Id<LexicalScope>,
    #[model(key)] pub kind: BindingEventKind,
    #[model(key)] pub ordinal: i64,
    #[model(key)] pub value: Option<Id<Occurrence>>,
    #[model(key)] pub static_branch: Option<StaticBranch>,
    #[model(key)] pub static_polarity: Option<bool>,
}
fn validate_binding(row: &BindingObservation) -> Result<(),ModelError> {
    if row.ordinal < 0 || row.static_branch.is_some() != row.static_polarity.is_some() {
        return Err(ModelError::Invalid("invalid binding ordinal or static branch qualification".into()));
    }
    Ok(())
}

#[derive(Debug,Clone,PartialEq,Eq,Domain,Assertion)]
#[model(name = "reference_observations", validate = validate_reference)]
#[assertion(support = ReferenceSupport, name = "reference_supports", family = FactFamily::Lexical, subjects(read, scope, parent))]
pub struct ReferenceObservation {
    #[model(key)] pub qualification: Id<AssertionQualification>,
    #[model(key)] pub read: Id<Occurrence>,
    #[model(key)] pub scope: Id<LexicalScope>,
    #[model(key)] pub parent: Id<Occurrence>,
    #[model(key)] pub field: SyntaxField,
    #[model(key)] pub name: String,
}
fn validate_reference(row: &ReferenceObservation) -> Result<(),ModelError> {
    if row.name.is_empty() { return Err(ModelError::Invalid("reference needs its observed spelling".into())); }
    Ok(())
}

#[derive(Debug,Clone,PartialEq,Eq,Hash,DomainSum)]
#[model(name = "lexical_targets", validate = validate_target)]
pub enum LexicalTarget {
    #[model(code = 0)] Binding { event: Id<BindingEvent> },
    /// A provider's builtin classification, not a cross-provider symbol equivalence.
    #[model(code = 1)] Builtin { name: String, variable: bool },
    #[model(code = 2)] Unresolved { reason: super::obligation::ObligationKind },
}
fn validate_target(row: &LexicalTarget) -> Result<(),ModelError> {
    if matches!(row,LexicalTarget::Builtin { name,.. } if name.is_empty()) {
        return Err(ModelError::Invalid("builtin target needs its observed spelling".into()));
    }
    Ok(())
}
#[derive(Debug,Clone,PartialEq,Eq,Domain,Assertion)]
#[model(name = "lexical_resolutions")]
#[assertion(support = LexicalResolutionSupport, name = "lexical_resolution_supports", family = FactFamily::Lexical, subjects(read, target))]
pub struct LexicalResolution {
    #[model(key)] pub qualification: Id<AssertionQualification>,
    #[model(key)] pub read: Id<Occurrence>,
    #[model(key)] pub target: Id<LexicalTarget>,
    #[model(key)] pub captured: bool,
}

fn lexical_invariants() -> Vec<Invariant> {
    vec![Invariant { name: "lexical_source_structure", inputs: vec![
        ValidationInput::of::<Occurrence>(&["id"]), ValidationInput::of::<LexicalScope>(&["id"]),
        ValidationInput::of::<BindingEvent>(&["id"]), ValidationInput::of::<LexicalTarget>(&["id"]),
        ValidationInput::of::<LexicalScopeObservation>(&["id"]), ValidationInput::of::<BindingObservation>(&["id"]),
        ValidationInput::of::<ReferenceObservation>(&["id"]), ValidationInput::of::<LexicalResolution>(&["id"]),
    ], create: std::sync::Arc::new(|| Box::new(LexicalCheck::default())) }]
}
#[derive(Default)]
struct LexicalCheck {
    occurrences: std::collections::BTreeMap<Id<Occurrence>,Occurrence>,
    scopes: std::collections::BTreeMap<Id<LexicalScope>,LexicalScope>,
    events: std::collections::BTreeMap<Id<BindingEvent>,BindingEvent>,
    targets: std::collections::BTreeMap<Id<LexicalTarget>,LexicalTarget>,
}
fn invalid(message: &str) -> ModelError { ModelError::Invalid(message.into()) }
impl LexicalCheck {
    fn occurrence(&self, id: Id<Occurrence>) -> Result<&Occurrence,ModelError> {
        self.occurrences.get(&id).ok_or_else(|| invalid("lexical occurrence missing"))
    }
    fn scope(&self, id: Id<LexicalScope>) -> Result<&LexicalScope,ModelError> {
        self.scopes.get(&id).ok_or_else(|| invalid("lexical scope missing"))
    }
    fn event(&self, id: Id<BindingEvent>) -> Result<&BindingEvent,ModelError> {
        self.events.get(&id).ok_or_else(|| invalid("lexical binding event missing"))
    }
    fn same_source(&self, a: Id<Occurrence>, b: Id<Occurrence>) -> Result<(),ModelError> {
        if self.occurrence(a)?.source != self.occurrence(b)?.source { return Err(invalid("lexical relationship crosses source artifacts")); }
        Ok(())
    }
}
impl InvariantCheck for LexicalCheck {
    fn visit(&mut self, relation: &str, batch: &arrow_array::RecordBatch) -> Result<(),ModelError> {
        if relation == Occurrence::NAME { for row in Occurrence::decode(batch)? { self.occurrences.insert(row.id(),row); } }
        else if relation == LexicalScope::NAME { for row in LexicalScope::decode(batch)? {
            use super::source::SyntaxKind;
            let owner = self.occurrence(row.owner)?;
            let valid = match row.kind {
                LexicalScopeKind::Module => owner.syntax_kind == SyntaxKind::ModModule,
                LexicalScopeKind::Class => owner.syntax_kind == SyntaxKind::StmtClassDef,
                LexicalScopeKind::Function => owner.syntax_kind == SyntaxKind::StmtFunctionDef,
                LexicalScopeKind::Lambda => owner.syntax_kind == SyntaxKind::ExprLambda,
                LexicalScopeKind::Comprehension => matches!(owner.syntax_kind,SyntaxKind::ExprListComp | SyntaxKind::ExprSetComp | SyntaxKind::ExprDictComp | SyntaxKind::ExprGenerator),
            };
            if !valid { return Err(invalid("lexical scope kind differs from its opening occurrence")); }
            self.scopes.insert(row.id(),row);
        } }
        else if relation == BindingEvent::NAME { for row in BindingEvent::decode(batch)? { self.occurrence(row.site)?; self.events.insert(row.id(),row); } }
        else if relation == LexicalTarget::NAME { for row in LexicalTarget::decode(batch)? {
            if let LexicalTarget::Binding { event } = &row { self.event(*event)?; }
            self.targets.insert(row.id(),row);
        } }
        else if relation == LexicalScopeObservation::NAME { for row in LexicalScopeObservation::decode(batch)? {
            let scope = self.scope(row.scope)?;
            if (scope.kind == LexicalScopeKind::Module) != row.parent.is_none() { return Err(invalid("only a module lexical scope has no parent")); }
            if let Some(parent) = row.parent {
                let parent = self.occurrence(self.scope(parent)?.owner)?;
                let child = self.occurrence(scope.owner)?;
                if parent.source != child.source || parent.start > child.start || parent.end < child.end
                    || parent.structural_path.len() >= child.structural_path.len() || !child.structural_path.starts_with(&parent.structural_path) {
                    return Err(invalid("lexical parent is not a structural source ancestor"));
                }
            }
        } }
        else if relation == BindingObservation::NAME { for row in BindingObservation::decode(batch)? {
            let site = self.event(row.event)?.site;
            self.same_source(site,self.scope(row.scope)?.owner)?;
            if let Some(value) = row.value { self.same_source(site,value)?; }
        } }
        else if relation == ReferenceObservation::NAME { for row in ReferenceObservation::decode(batch)? {
            self.same_source(row.read,self.scope(row.scope)?.owner)?;
            self.same_source(row.read,row.parent)?;
            let read = self.occurrence(row.read)?; let parent = self.occurrence(row.parent)?;
            if read.start < parent.start || read.end > parent.end
                || parent.structural_path.len() >= read.structural_path.len() || !read.structural_path.starts_with(&parent.structural_path) {
                return Err(invalid("lexical read parent is not a structural ancestor"));
            }
        } }
        else if relation == LexicalResolution::NAME { for row in LexicalResolution::decode(batch)? {
            self.occurrence(row.read)?;
            match self.targets.get(&row.target).ok_or_else(|| invalid("lexical target missing"))? {
                LexicalTarget::Binding { event } => self.same_source(row.read,self.event(*event)?.site)?,
                LexicalTarget::Builtin { .. } | LexicalTarget::Unresolved { .. } if row.captured => return Err(invalid("only a lexical binding can be captured")),
                LexicalTarget::Builtin { .. } | LexicalTarget::Unresolved { .. } => {},
            }
        } }
        else { return Err(invalid("undeclared lexical validation input")); }
        if self.occurrences.len()+self.scopes.len()+self.events.len()+self.targets.len() > 1_000_000 {
            return Err(invalid("lexical validation cardinality limit"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(),ModelError> { Ok(()) }
}
