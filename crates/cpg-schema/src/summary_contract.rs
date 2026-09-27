//! Shared admission of linked, fixed Boolean controls in finite summary proofs.
//! These checks do not prove argument-to-formal binding; the source producer must also
//! reconstruct the ordered argument group and the actual substitution result.
use std::collections::{BTreeSet, HashMap};

use crate::codebook::{BoundaryReason, SummaryFlowStepKind};
use crate::condition::Atom;
use crate::condition_kernel::{Diagram, KernelBoundary};
use crate::id::{Id, recipe::SummaryFlowProofStep};

/// Expected occurrence of a returned source value, independently supplied by its consumer.
/// Grouping the scope prevents confusing these IDs with certificate evidence IDs.
#[derive(Clone, Copy)]
pub struct ReturnValueScope {
    pub function: Id,
    pub parameter: Id,
    pub source: Id,
    pub origin: Id,
    pub condition: Id,
    pub return_site: Id,
}

/// A source subject retains its own identity. Site channels require neither a fabricated
/// formal nor a successful return. Additional subject variants require actual producers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SummarySubject {
    ValueOrigin {
        origin_id: Id,
        flow_fact_id: Id,
        parameter_node_id: Id,
    },
    ExecutionSite {
        node_id: Id,
        syntax_fact_id: Id,
    },
}

/// Claim-specific domains; a consumer must match this meaning before using completeness.
/// In particular, no escaping exception says nothing about caught/converted/suppressed
/// exception activity or about whether an operation reaches the subject.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoverageDomain {
    ValueTransfer,
    CompletionOutcome,
    EscapingException,
}

pub fn coverage_domain(
    subject: crate::codebook::SummarySubjectKind,
    channel: crate::codebook::SummaryChannel,
) -> Option<CoverageDomain> {
    use crate::codebook::{SummaryChannel as C, SummarySubjectKind as S};
    match (subject, channel) {
        (S::ValueOrigin, C::Value) => Some(CoverageDomain::ValueTransfer),
        (S::ExecutionSite, C::Completion) => Some(CoverageDomain::CompletionOutcome),
        (S::ExecutionSite, C::Exception) => Some(CoverageDomain::EscapingException),
        _ => None,
    }
}

impl SummarySubject {
    pub fn columns(self) -> (crate::codebook::SummarySubjectKind, Id, Id, Option<Id>) {
        use crate::codebook::SummarySubjectKind as K;
        match self {
            Self::ValueOrigin {
                origin_id,
                flow_fact_id,
                parameter_node_id,
            } => (
                K::ValueOrigin,
                origin_id,
                flow_fact_id,
                Some(parameter_node_id),
            ),
            Self::ExecutionSite {
                node_id,
                syntax_fact_id,
            } => (K::ExecutionSite, node_id, syntax_fact_id, None),
        }
    }
}

crate::query_row! {
    pub struct LocalCallArgument {
        call_fact_id: Id,
        callee_node_id: Id,
        argument_fact_id: Id,
        ordinal: i64,
        formal_node_id: Option<Id>,
        evaluation_fact_id: Option<Id>,
        evaluation_reason: Option<BoundaryReason>,
        boolean_value: Option<bool>,
        source_formal_node_id: Option<Id>,
    }
}

crate::query_row! {
    /// A direct entry-value link, admitted by the source/value-link boundary before use here.
    pub struct DirectValueLink {
        operation_node_id: Id,
        formal_node_id: Id,
        link_id: Id,
        atom: String,
    }
}

pub fn fixed_truth(condition: &Diagram, atom: &str) -> Result<Option<bool>, KernelBoundary> {
    let atom = Atom::parse_encoded(atom).map_err(|_| KernelBoundary::TransferUnsupported)?;
    if !matches!(&atom, Atom::Evaluated { atom, .. } if matches!(**atom, Atom::Truthy { .. })) {
        return Err(KernelBoundary::TransferUnsupported);
    }
    let predicate = Diagram::from_atom(&atom)?;
    if condition.implies(&predicate)? {
        return Ok(Some(true));
    }
    if condition.implies(&predicate.not()?)? {
        return Ok(Some(false));
    }
    Ok(None)
}

/// Validate the complete contiguous control group immediately before a conditional callee
/// reference. The same rule serves source composition and immutable native admission.
pub fn validate_fixed_control_proof(
    caller: Id,
    caller_condition: &Diagram,
    callee: Id,
    callee_condition: &Diagram,
    steps: &[SummaryFlowProofStep],
    links: &HashMap<Id, DirectValueLink>,
) -> Result<(), ProofAdmissionError> {
    if steps.is_empty() {
        return Err("conditional callee lacks a test link".into());
    }
    let mut atoms = BTreeSet::new();
    let mut pending_caller = false;
    for step in steps {
        match step.kind {
            SummaryFlowStepKind::CallerConditionLink => {
                let link = links
                    .get(&step.evidence_id)
                    .ok_or("caller condition link does not fix its direct formal")?;
                if pending_caller
                    || step.condition_id != caller_condition.id()
                    || link.operation_node_id != caller
                    || !caller_condition.support().contains(&link.atom)
                    || fixed_truth(caller_condition, &link.atom)?.is_none()
                {
                    return Err("caller condition link does not fix its direct formal".into());
                }
                pending_caller = true;
            }
            SummaryFlowStepKind::CalleeConditionLink => {
                let Some(link) = links.get(&step.evidence_id) else {
                    return Err("conditional callee lacks a cited exact test link".into());
                };
                if link.operation_node_id != callee
                    || step.condition_id != callee_condition.id()
                    || !callee_condition.support().contains(&link.atom)
                    || !matches!(Atom::parse_encoded(&link.atom), Ok(Atom::Evaluated { atom, .. })
                        if matches!(*atom, Atom::Truthy { .. }))
                    || !atoms.insert(link.atom.as_str())
                {
                    return Err("conditional callee lacks a cited exact test link".into());
                }
                pending_caller = false;
            }
            _ => return Err("unexpected step in conditional callee control proof".into()),
        }
    }
    if pending_caller {
        return Err("caller condition link does not fix its direct formal".into());
    }
    if atoms.len() != callee_condition.support().len() {
        return Err("conditional callee control proof has incomplete atom coverage".into());
    }
    Ok(())
}

/// The admitted semantic target of a callee reference. Both source composition and immutable
/// native loading supply this view; neither consumer interprets proof adjacency itself.
pub struct CalleeProofTarget<'a> {
    pub function_node_id: Id,
    pub condition: &'a Diagram,
    pub verdict: crate::codebook::Verdict,
    pub path_depth: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProofAdmissionError {
    pub reason: BoundaryReason,
    pub message: &'static str,
}

impl From<&'static str> for ProofAdmissionError {
    fn from(message: &'static str) -> Self {
        Self {
            reason: BoundaryReason::MissingEvidence,
            message,
        }
    }
}

/// One summary-facing classification for a bounded condition operation or stored root.
pub fn condition_limit(reason: KernelBoundary) -> BoundaryReason {
    match reason {
        KernelBoundary::AtomLimit => BoundaryReason::ConditionAtomLimit,
        KernelBoundary::WorkPreflight => BoundaryReason::ConditionWorkLimit,
        KernelBoundary::NodeLimit => BoundaryReason::ConditionNodeLimit,
        KernelBoundary::SourceOverBudget => BoundaryReason::BudgetReached,
        KernelBoundary::TransferUnsupported => BoundaryReason::OutsideProviderModel,
        KernelBoundary::AtomNameCollision => BoundaryReason::MissingEvidence,
    }
}

impl From<KernelBoundary> for ProofAdmissionError {
    fn from(boundary: KernelBoundary) -> Self {
        Self {
            reason: condition_limit(boundary),
            message: boundary.code(),
        }
    }
}

/// A local base return requires a value witness in addition to its completion obligations.
/// Completion alone proves no relationship between an entry formal and the returned value.
pub fn admit_base_value(
    source: Id,
    condition: Id,
    steps: &[SummaryFlowProofStep],
) -> Result<(), ProofAdmissionError> {
    let values: Vec<_> = steps
        .iter()
        .enumerate()
        .filter(|(_, s)| {
            matches!(
                s.kind,
                SummaryFlowStepKind::RawIdentity | SummaryFlowStepKind::ContextEntryValueIdentity
            )
        })
        .collect();
    let [(at, value)] = values.as_slice() else {
        return Err("base return requires exactly one value witness".into());
    };
    if value.condition_id != condition
        || !steps[*at + 1..]
            .iter()
            .any(|s| s.kind == SummaryFlowStepKind::ReturnExit)
    {
        return Err("base value witness has a foreign condition or follows its return exit".into());
    }
    if value.kind == SummaryFlowStepKind::RawIdentity && value.evidence_id != source {
        return Err("base identity cites a different source flow".into());
    }
    if value.kind == SummaryFlowStepKind::ContextEntryValueIdentity
        && steps
            .iter()
            .any(|s| s.kind == SummaryFlowStepKind::SourceParameterIdentity)
    {
        return Err("context entry value is not a bare parameter read".into());
    }
    Ok(())
}

/// Admit the complete structural callee obligations in a proof. Source reconstruction still
/// owns binding/evaluation evidence. A link belongs to exactly one following callee reference;
/// references decrease depth, cite the callee's actual condition, and cover every control.
pub fn admit_callee_proof<'a>(
    caller: Id,
    condition: &Diagram,
    path_depth: i64,
    steps: &[SummaryFlowProofStep],
    links: &HashMap<Id, DirectValueLink>,
    resolve: impl Fn(Id) -> Option<CalleeProofTarget<'a>>,
) -> Result<(), ProofAdmissionError> {
    use crate::codebook::Verdict;
    if steps.len() > MAX_SUMMARY_PROOF_STEPS {
        return Err(ProofAdmissionError {
            reason: BoundaryReason::SummaryProofLimit,
            message: "summary proof exceeds step limit",
        });
    }
    let mut start = None;
    for (index, step) in steps.iter().enumerate() {
        match step.kind {
            SummaryFlowStepKind::CallerConditionLink | SummaryFlowStepKind::CalleeConditionLink => {
                start.get_or_insert(index);
            }
            SummaryFlowStepKind::CalleeSummary => {
                let callee = resolve(step.evidence_id).ok_or("missing cited callee summary")?;
                if callee.path_depth < 0 || callee.path_depth >= path_depth {
                    return Err("cyclic or unordered callee proof".into());
                }
                if step.condition_id != callee.condition.id() {
                    return Err("callee reference condition mismatch".into());
                }
                let controls = &steps[start.take().unwrap_or(index)..index];
                match callee.verdict {
                    Verdict::Established if callee.condition.is_true() => {
                        if !controls.is_empty() {
                            return Err("unexpected unconditional callee controls".into());
                        }
                    }
                    Verdict::Conditional
                        if !callee.condition.is_true() && !callee.condition.is_false() =>
                    {
                        validate_fixed_control_proof(
                            caller,
                            condition,
                            callee.function_node_id,
                            callee.condition,
                            controls,
                            links,
                        )?;
                    }
                    _ => return Err("callee proof has no admitted verdict".into()),
                }
            }
            _ if start.is_some() => return Err("orphan condition proof link".into()),
            _ => {}
        }
    }
    if start.is_some() {
        return Err("orphan condition proof link".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::condition::EvaluationIdentity;

    #[test]
    fn callee_reference_structure_rejects_orphans_wrong_conditions_and_cycles() {
        use crate::codebook::Verdict;
        let root = Diagram::always();
        let reference = SummaryFlowProofStep {
            kind: SummaryFlowStepKind::CalleeSummary,
            evidence_id: Id([2; 16]),
            condition_id: root.id(),
        };
        let check = |steps: &[SummaryFlowProofStep], depth| {
            admit_callee_proof(Id([1; 16]), &root, depth, steps, &HashMap::new(), |id| {
                (id == Id([2; 16])).then_some(CalleeProofTarget {
                    function_node_id: Id([3; 16]),
                    condition: &root,
                    verdict: Verdict::Established,
                    path_depth: 0,
                })
            })
            .map_err(|error| error.message)
        };
        assert!(check(std::slice::from_ref(&reference), 1).is_ok());
        assert_eq!(
            check(std::slice::from_ref(&reference), 0),
            Err("cyclic or unordered callee proof")
        );
        let mut wrong = reference;
        wrong.condition_id = Diagram::never().id();
        assert_eq!(
            check(&[wrong], 1),
            Err("callee reference condition mismatch")
        );
        let mut missing = reference;
        missing.evidence_id = Id([4; 16]);
        assert_eq!(check(&[missing], 1), Err("missing cited callee summary"));
        let link = SummaryFlowProofStep {
            kind: SummaryFlowStepKind::CalleeConditionLink,
            evidence_id: Id([5; 16]),
            condition_id: root.id(),
        };
        assert_eq!(
            check(std::slice::from_ref(&link), 1),
            Err("orphan condition proof link")
        );
        assert_eq!(
            check(&[link, reference], 1),
            Err("unexpected unconditional callee controls")
        );
        let other = SummaryFlowProofStep {
            kind: SummaryFlowStepKind::ReturnExit,
            ..reference
        };
        assert_eq!(
            check(&[link, other, reference], 1),
            Err("orphan condition proof link")
        );
    }

    #[test]
    fn every_callee_atom_needs_its_own_scoped_link() {
        let caller = Id([1; 16]);
        let callee = Id([2; 16]);
        let atoms: Vec<_> = ["a", "b"]
            .into_iter()
            .enumerate()
            .map(|(index, name)| {
                Atom::Truthy {
                    place: name.to_owned(),
                }
                .evaluated(EvaluationIdentity::Synthetic {
                    module: "00".repeat(16),
                    predicate: format!("{:02x}", index + 1).repeat(16),
                })
            })
            .collect();
        let condition = Diagram::from_atom(&atoms[0])
            .unwrap()
            .and(&Diagram::from_atom(&atoms[1]).unwrap())
            .unwrap();
        let links: HashMap<_, _> = atoms
            .iter()
            .enumerate()
            .map(|(index, atom)| {
                let id = Id([index as u8 + 10; 16]);
                (
                    id,
                    DirectValueLink {
                        operation_node_id: callee,
                        formal_node_id: id,
                        link_id: id,
                        atom: atom.encode(),
                    },
                )
            })
            .collect();
        let steps: Vec<_> = (10..12)
            .map(|byte| SummaryFlowProofStep {
                kind: SummaryFlowStepKind::CalleeConditionLink,
                evidence_id: Id([byte; 16]),
                condition_id: condition.id(),
            })
            .collect();
        assert!(
            validate_fixed_control_proof(
                caller,
                &Diagram::always(),
                callee,
                &condition,
                &steps,
                &links
            )
            .is_ok()
        );
        assert!(
            validate_fixed_control_proof(
                caller,
                &Diagram::always(),
                callee,
                &condition,
                &steps[1..],
                &links
            )
            .is_err()
        );
        let mut wrong_scope = links.clone();
        wrong_scope
            .get_mut(&Id([10; 16]))
            .unwrap()
            .operation_node_id = caller;
        assert!(
            validate_fixed_control_proof(
                caller,
                &Diagram::always(),
                callee,
                &condition,
                &steps,
                &wrong_scope
            )
            .is_err()
        );
        let duplicate = vec![steps[0], steps[0]];
        assert!(
            validate_fixed_control_proof(
                caller,
                &Diagram::always(),
                callee,
                &condition,
                &duplicate,
                &links
            )
            .is_err()
        );
    }
}

crate::query_row! {
    /// A source-admitted normal name read. It does not provide an exact value or establish
    /// evaluation of the enclosing expression. Duplicate certificates are refused by the kernel.
    pub struct ExpressionRead {
        snapshot_id: Id,
        syntax_fact_id: Id,
        evidence_id: Id,
        status: crate::codebook::ModeledArgumentEvaluationStatus,
    }
}

/// A complete source or pinned signature, independent of evaluation order.
#[derive(Clone, Debug)]
pub struct SignatureParameter {
    pub evidence_id: Id,
    pub ordinal: i64,
    pub name: String,
    pub kind: crate::codebook::ParameterKind,
    pub required: bool,
}

impl SignatureParameter {
    /// Pinned provider metadata omits requiredness for variadic slots: an empty collection
    /// needs no argument. Missing requiredness for an ordinary formal remains unknown.
    pub fn from_context(p: &crate::tables::ContextParametersRow) -> Result<Self, BoundaryReason> {
        use crate::codebook::{ParameterKind as P, SignatureForm};
        let absent = BoundaryReason::MissingEvidence;
        if p.form != SignatureForm::List {
            return Err(BoundaryReason::OutsideProviderModel);
        }
        let kind = p.kind.ok_or(absent)?;
        let required = match kind {
            P::VarPositional | P::VarKeyword if p.required.is_none() => false,
            _ => p.required.ok_or(absent)?,
        };
        Ok(Self {
            evidence_id: p.fact_id,
            ordinal: p.ordinal.ok_or(absent)?,
            name: p
                .name
                .as_ref()
                .filter(|n| !n.is_empty())
                .ok_or(absent)?
                .clone(),
            kind,
            required,
        })
    }
}

/// One local proof budget, shared by the producer and native generation admission.
/// Recursive callee support is bounded independently by the summary path-depth limit.
pub const MAX_SUMMARY_PROOF_STEPS: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundArgument {
    pub argument_fact_id: Id,
    pub parameter_fact_id: Id,
    pub parameter_name: String,
}

/// An omitted default is a definition-time obligation, never an invented call argument.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DefaultRequirement {
    pub parameter_fact_id: Id,
    pub parameter_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallBinding {
    pub explicit: Vec<BoundArgument>,
    pub defaults: Vec<DefaultRequirement>,
}

/// Bind an argument group without evaluating it. Source callers must discharge every omitted
/// default through definition-time availability/stability evidence; a pinned model must
/// author availability independently of any normal-return promise. No default expression is evaluated at the call site.
/// Unpacking and variadic collection remain unsupported rather than partially bound.
pub fn bind_arguments(
    parameters: &[SignatureParameter],
    arguments: &[crate::tables::ArgumentsRow],
) -> Result<CallBinding, BoundaryReason> {
    use crate::codebook::{ArgumentKind as A, ParameterKind as P};
    let unsupported = BoundaryReason::UnsupportedControlFlow;
    if arguments.len() > 128 || parameters.len() > 128 {
        return Err(unsupported);
    }
    let mut formals: Vec<_> = parameters.iter().collect();
    formals.sort_by_key(|p| p.ordinal);
    if formals
        .iter()
        .enumerate()
        .any(|(index, p)| p.ordinal != index as i64)
        || formals
            .iter()
            .map(|p| &p.name)
            .collect::<BTreeSet<_>>()
            .len()
            != formals.len()
        || formals
            .iter()
            .map(|p| p.evidence_id)
            .collect::<BTreeSet<_>>()
            .len()
            != formals.len()
    {
        return Err(BoundaryReason::MissingEvidence);
    }
    let mut args: Vec<_> = arguments.iter().collect();
    args.sort_by_key(|a| a.ordinal);
    if args
        .iter()
        .enumerate()
        .any(|(index, a)| a.ordinal != index as i64)
        || args
            .iter()
            .map(|a| a.fact_id)
            .collect::<BTreeSet<_>>()
            .len()
            != args.len()
        || args.windows(2).any(|pair| {
            pair[0].snapshot_id != pair[1].snapshot_id
                || pair[0].call_node_id != pair[1].call_node_id
        })
    {
        return Err(BoundaryReason::MissingEvidence);
    }
    let positional: Vec<_> = formals
        .iter()
        .copied()
        .filter(|p| matches!(p.kind, P::PositionalOnly | P::PositionalOrKeyword))
        .collect();
    let mut next_positional = 0;
    let mut used = BTreeSet::new();
    let mut out = Vec::new();
    for argument in args {
        let parameter = match argument.kind {
            A::Positional if argument.keyword.is_none() => {
                let parameter = positional
                    .get(next_positional)
                    .copied()
                    .ok_or(unsupported)?;
                next_positional += 1;
                parameter
            }
            A::Keyword => {
                let name = argument.keyword.as_deref().ok_or(unsupported)?;
                formals
                    .iter()
                    .copied()
                    .find(|p| {
                        p.name == name && matches!(p.kind, P::PositionalOrKeyword | P::KeywordOnly)
                    })
                    .ok_or(unsupported)?
            }
            _ => return Err(unsupported),
        };
        if !used.insert(parameter.evidence_id) {
            return Err(unsupported);
        }
        out.push(BoundArgument {
            argument_fact_id: argument.fact_id,
            parameter_fact_id: parameter.evidence_id,
            parameter_name: parameter.name.clone(),
        });
    }
    if formals
        .iter()
        .any(|p| p.required && !used.contains(&p.evidence_id))
    {
        return Err(unsupported);
    }
    let defaults = formals
        .iter()
        .filter(|p| {
            !p.required
                && !used.contains(&p.evidence_id)
                && !matches!(p.kind, P::VarPositional | P::VarKeyword)
        })
        .map(|p| DefaultRequirement {
            parameter_fact_id: p.evidence_id,
            parameter_name: p.name.clone(),
        })
        .collect();
    Ok(CallBinding {
        explicit: out,
        defaults,
    })
}

#[cfg(test)]
mod binding_tests {
    use super::*;
    use crate::codebook::{ArgumentKind as A, ParameterKind as P};
    fn argument(id: u8, ordinal: i64, keyword: Option<&str>) -> crate::tables::ArgumentsRow {
        crate::tables::ArgumentsRow {
            snapshot_id: Id([1; 16]),
            fact_id: Id([id; 16]),
            node_id: Id([id; 16]),
            call_node_id: Id([2; 16]),
            ordinal,
            kind: if keyword.is_some() {
                A::Keyword
            } else {
                A::Positional
            },
            keyword: keyword.map(str::to_owned),
            start_byte: 0,
            end_byte: 10,
            value_start_byte: 0,
            value_end_byte: 10,
        }
    }
    #[test]
    fn binding_is_independent_of_source_order_and_rejects_missing_or_duplicate_formals() {
        let parameters = vec![
            SignatureParameter {
                evidence_id: Id([3; 16]),
                ordinal: 0,
                name: "first".to_owned(),
                kind: P::PositionalOrKeyword,
                required: true,
            },
            SignatureParameter {
                evidence_id: Id([4; 16]),
                ordinal: 1,
                name: "second".to_owned(),
                kind: P::KeywordOnly,
                required: false,
            },
        ];
        let args = vec![
            argument(10, 0, Some("second")),
            argument(11, 1, Some("first")),
        ];
        let bound = bind_arguments(&parameters, &args).unwrap();
        assert_eq!(
            bound
                .explicit
                .iter()
                .map(|b| b.parameter_name.as_str())
                .collect::<Vec<_>>(),
            vec!["second", "first"]
        );
        assert!(bind_arguments(&parameters, &args[..1]).is_err());
        assert!(bound.defaults.is_empty());
        let omitted = bind_arguments(&parameters, &[argument(10, 0, None)]).unwrap();
        assert_eq!(omitted.explicit.len(), 1);
        assert_eq!(
            omitted.defaults,
            vec![DefaultRequirement {
                parameter_fact_id: Id([4; 16]),
                parameter_name: "second".to_owned()
            }]
        );
        assert!(
            bind_arguments(
                &parameters,
                &[argument(10, 0, None), argument(11, 1, Some("first"))]
            )
            .is_err()
        );
        let mut unpacked = argument(10, 0, None);
        unpacked.kind = A::Starred;
        assert!(bind_arguments(&parameters, &[unpacked]).is_err());
    }
}

crate::query_row! {
    /// A sole pinned target with an independently resolved module-import callee.
    /// Invocation and total normal completion are separate promises.
    /// The expression kernel still checks the signature and every evaluated argument.
    pub struct PinnedCallTarget {
        snapshot_id: Id,
        call_node_id: Id,
        call_fact_id: Id,
        target_node_id: Id,
        signature_count: i64,
        body_return_parameter: Option<String>,
        caller_function_scope: bool,
        call_defaults_available: bool,
        pysa_fact_id: Id,
        model_id: Id,
        resolution_fact_id: Id,
        import_binding_fact_id: Id,
        import_region_fact_id: Id,
        import_condition_id: Id,
        argument_count: i64,
    }
}

/// Exact exception outcomes established by the bounded Python completion rules. This list
/// also owns which builtin classes extraction retains for their pinned hierarchy evidence.
pub use crate::codebook::ExactRuntimeException;
impl ExactRuntimeException {
    pub const ALL: &[Self] = &[Self::TypeError];
    pub const fn class(self) -> (&'static str, &'static str) {
        match self {
            Self::TypeError => ("builtins", "TypeError"),
        }
    }
}

/// An admitted outcome under statement entry. Unknown outcomes are represented by the
/// producer's typed refusal, not by a fictitious normal or raised completion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompletionOutcome {
    Normal,
    Return(Id),
    Raise {
        terminal: Id,
        exception: ExactRuntimeException,
    },
    Break(Id),
    Continue(Id),
}

impl CompletionOutcome {
    pub fn kind(self) -> crate::codebook::CompletionKind {
        use crate::codebook::CompletionKind as C;
        match self {
            Self::Normal => C::Normal,
            Self::Return(_) => C::Return,
            Self::Raise { .. } => C::Raise,
            Self::Break(_) => C::Break,
            Self::Continue(_) => C::Continue,
        }
    }

    pub fn terminal(self) -> Option<Id> {
        match self {
            Self::Normal => None,
            Self::Return(id)
            | Self::Break(id)
            | Self::Continue(id)
            | Self::Raise { terminal: id, .. } => Some(id),
        }
    }

    pub fn exception(self) -> Option<ExactRuntimeException> {
        match self {
            Self::Raise { exception, .. } => Some(exception),
            _ => None,
        }
    }

    pub fn raised(terminal: Id, exception: ExactRuntimeException) -> Self {
        Self::Raise {
            terminal,
            exception,
        }
    }

    /// Python executes a finalizer even for an abrupt pending outcome. Only normal completion
    /// preserves that outcome; return/raise/break/continue replace it.
    pub fn after_finalizer(self, finalizer: Self) -> Self {
        if finalizer == Self::Normal {
            self
        } else {
            finalizer
        }
    }
}
