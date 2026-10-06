//! Located native protocol observations. Diagnostic-free typing is not runtime applicability.
use super::{
    assertion::AssertionQualification,
    attribution::FactFamily,
    charged::{ChargedMap, StateCharge},
    lexical::SyntaxField,
    source::{Occurrence, SourceArtifact, SyntaxKind},
    syntax::SyntaxPlacement,
    types::TypeTerm,
    *,
};
use crate::{Assertion, Domain, DomainCode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum NativeCallStatus {
    NoHardDiagnostics = 0,
    HardDiagnostics = 1,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum ExitAwaitability {
    Synchronous = 0,
    Awaitable = 1,
    NotAwaitable = 2,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum TerminalDecision {
    DeclaredOrInferredDivergence = 0,
    NarrowedNeverCallee = 1,
    InferredMethodReturn = 2,
    NotImplementedBody = 3,
    NonDivergentCallable = 4,
    NoNativeCallableSignature = 5,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum ExitDiagnosticPhase {
    Member = 0,
    Normal = 1,
    Exceptional = 2,
    Await = 3,
}

#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion, serde::Serialize, serde::Deserialize)]
#[model(name = "native_exit_observations", validate = validate_exit, invariant_refs = invariants_refs)]
#[assertion(support = NativeExitSupport, name = "native_exit_supports", family = FactFamily::Types, subjects(subject, receiver, member, normal_result, exceptional_result))]
pub struct NativeExitObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    /// Exact context expression under a WithItem; not a scope-wide suppression result.
    #[model(key)]
    pub subject: Id<Occurrence>,
    #[model(key)]
    pub asynchronous: bool,
    #[model(key)]
    pub receiver: Id<TypeTerm>,
    #[model(key)]
    pub member: Id<TypeTerm>,
    #[model(key)]
    pub member_name: String,
    #[model(key)]
    pub normal_result: Id<TypeTerm>,
    #[model(key)]
    pub exceptional_result: Id<TypeTerm>,
    #[model(key)]
    pub normal_status: NativeCallStatus,
    #[model(key)]
    pub exceptional_status: NativeCallStatus,
    #[model(key)]
    pub normal_awaitability: ExitAwaitability,
    #[model(key)]
    pub exceptional_awaitability: ExitAwaitability,
}
fn invalid(message: &str) -> ModelError {
    ModelError::Invalid(message.into())
}
fn validate_exit(row: &NativeExitObservation) -> Result<(), ModelError> {
    if row.member_name
        != if row.asynchronous {
            "__aexit__"
        } else {
            "__exit__"
        }
    {
        return Err(invalid("native exit member disagrees with source protocol"));
    }
    if !row.asynchronous
        && (row.normal_awaitability != ExitAwaitability::Synchronous
            || row.exceptional_awaitability != ExitAwaitability::Synchronous)
    {
        return Err(invalid("synchronous exit has asynchronous await status"));
    }
    Ok(())
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion, serde::Serialize, serde::Deserialize)]
#[model(name = "native_terminal_observations")]
#[assertion(support = NativeTerminalSupport, name = "native_terminal_supports", family = FactFamily::Types, subjects(subject, callee, return_type))]
pub struct NativeTerminalObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub subject: Id<Occurrence>,
    #[model(key)]
    pub callee: Id<TypeTerm>,
    #[model(key)]
    pub return_type: Option<Id<TypeTerm>>,
    #[model(key)]
    pub return_is_inferred: Option<bool>,
    #[model(key)]
    pub is_bound_method: bool,
    #[model(key)]
    pub decision: TerminalDecision,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion, serde::Serialize, serde::Deserialize)]
#[model(name = "native_exit_diagnostics", validate = validate_diagnostic)]
#[assertion(support = NativeExitDiagnosticSupport, name = "native_exit_diagnostic_supports", family = FactFamily::Types, subjects(subject, source))]
pub struct NativeExitDiagnostic {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub exit: Id<NativeExitObservation>,
    #[model(key)]
    pub subject: Id<Occurrence>,
    #[model(key)]
    pub phase: ExitDiagnosticPhase,
    #[model(key)]
    pub ordinal: i64,
    #[model(key)]
    pub source: Id<SourceArtifact>,
    #[model(key)]
    pub start: i64,
    #[model(key)]
    pub end: i64,
    #[model(key)]
    pub kind: String,
    #[model(key)]
    pub message: String,
    #[model(key)]
    pub details: Option<String>,
}
fn validate_diagnostic(row: &NativeExitDiagnostic) -> Result<(), ModelError> {
    if row.start < 0
        || row.end < row.start
        || row.ordinal < 0
        || row.kind.is_empty()
        || row.message.is_empty()
    {
        return Err(invalid("invalid native exit diagnostic geometry"));
    }
    Ok(())
}
pub(crate) fn invariants() -> Vec<Invariant> {
    vec![Invariant {
        purpose: crate::domain::InvariantPurpose::Admission,
        revision: 1,
        name: "located_native_protocols",
        inputs: vec![
            ValidationInput::of::<Occurrence>(&["id"]),
            ValidationInput::of::<SyntaxPlacement>(&["id"]),
            ValidationInput::of::<SourceArtifact>(&["id"]),
            ValidationInput::of::<NativeExitObservation>(&["id"]),
            ValidationInput::of::<NativeTerminalObservation>(&["id"]),
            ValidationInput::of::<NativeExitDiagnostic>(&["id"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(Check {
                charge: StateCharge::new(budget, "located_native_protocols"),
                occurrences: Default::default(),
                placements: Default::default(),
                sources: Default::default(),
                exits: Default::default(),
                terminals: Default::default(),
                diagnostics: Default::default(),
            })
        }),
    }]
}
struct Check {
    charge: StateCharge,
    occurrences: ChargedMap<Id<Occurrence>, Occurrence>,
    placements: ChargedMap<Id<SyntaxPlacement>, SyntaxPlacement>,
    sources: ChargedMap<Id<SourceArtifact>, SourceArtifact>,
    exits: ChargedMap<Id<NativeExitObservation>, NativeExitObservation>,
    terminals: ChargedMap<Id<NativeTerminalObservation>, NativeTerminalObservation>,
    diagnostics: ChargedMap<Id<NativeExitDiagnostic>, NativeExitDiagnostic>,
}
impl Check {
    fn parent(&self, child: Id<Occurrence>) -> Result<&SyntaxPlacement, ModelError> {
        let mut candidates = self.placements.values().filter(|p| p.occurrence == child);
        match (candidates.next(), candidates.next()) {
            (Some(row), None) => Ok(row),
            _ => Err(invalid("native protocol needs unique source placement")),
        }
    }
}
impl InvariantCheck for Check {
    fn visit(&mut self, name: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        macro_rules! rows {
            ($ty:ty,$field:ident) => {
                if name == <$ty>::NAME {
                    for row in <$ty>::decode(batch)? {
                        self.$field.insert(&mut self.charge, row.id(), row)?;
                    }
                    return Ok(());
                }
            };
        }
        rows!(Occurrence, occurrences);
        rows!(SyntaxPlacement, placements);
        rows!(SourceArtifact, sources);
        rows!(NativeExitObservation, exits);
        rows!(NativeTerminalObservation, terminals);
        rows!(NativeExitDiagnostic, diagnostics);
        Err(invalid("undeclared native protocol validation input"))
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        for row in self.exits.values() {
            row.validate()?;
            let subject = self
                .occurrences
                .get(&row.subject)
                .ok_or_else(|| invalid("exit expression missing"))?;
            let placement = self.parent(row.subject)?;
            let parent = placement
                .parent
                .and_then(|id| self.occurrences.get(&id))
                .ok_or_else(|| invalid("exit WithItem missing"))?;
            if placement.field != SyntaxField::Value
                || parent.syntax_kind != SyntaxKind::WithItem
                || parent.source != subject.source
            {
                return Err(invalid("exit expression is not a canonical WithItem value"));
            }
        }
        for row in self.terminals.values() {
            let subject = self
                .occurrences
                .get(&row.subject)
                .ok_or_else(|| invalid("terminal call missing"))?;
            let placement = self.parent(row.subject)?;
            let parent = placement
                .parent
                .and_then(|id| self.occurrences.get(&id))
                .ok_or_else(|| invalid("terminal statement missing"))?;
            if subject.syntax_kind != SyntaxKind::ExprCall
                || placement.field != SyntaxField::Value
                || parent.syntax_kind != SyntaxKind::StmtExpr
                || parent.source != subject.source
            {
                return Err(invalid(
                    "native terminal decision is not a direct call statement",
                ));
            }
            if row.decision == TerminalDecision::DeclaredOrInferredDivergence
                && row.return_type.is_none()
            {
                return Err(invalid("terminal divergence signature missing"));
            }
        }
        for row in self.diagnostics.values() {
            row.validate()?;
            let exit = self
                .exits
                .get(&row.exit)
                .ok_or_else(|| invalid("diagnostic exit missing"))?;
            let subject = self
                .occurrences
                .get(&exit.subject)
                .ok_or_else(|| invalid("diagnostic subject missing"))?;
            let source = self
                .sources
                .get(&row.source)
                .ok_or_else(|| invalid("diagnostic source missing"))?;
            if row.qualification != exit.qualification
                || row.subject != exit.subject
                || row.source != subject.source
                || row.end > source.byte_len
            {
                return Err(invalid("exit diagnostic crosses source/context"));
            }
        }
        Ok(())
    }
}

pub(crate) fn invariants_refs() -> Vec<&'static str> {
    vec!["located_native_protocols"]
}
