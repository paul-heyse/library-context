//! Original call-local native overload membership, independent of structural Callable equality.
use super::*;
use crate::domain::source::SyntaxKind;
use crate::domain::syntax::SyntaxPlacement;
use crate::{Assertion, Domain, DomainCode};
pub const MAX_OVERLOAD_CANDIDATES: usize = 4096;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum OverloadSelection {
    Resolved = 0,
    Selected = 1,
    AmbiguousRepresentative = 2,
    ExpandedRepresentative = 3,
    ClosestOnly = 4,
    Recovered = 5,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name="native_overload_observations", validate=validate_trace, invariant_refs=trace_invariants_refs)]
#[assertion(support=NativeOverloadSupport, name="native_overload_supports", family=crate::domain::attribution::FactFamily::Types, subjects(site, arguments, scope))]
pub struct NativeOverloadObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub site: Id<Occurrence>,
    #[model(key)]
    pub arguments: Id<Occurrence>,
    pub scope: Id<CoverageScope>,
    #[model(key)]
    pub selection: OverloadSelection,
    #[model(key)]
    pub closest_ordinal: i64,
    #[model(key)]
    pub candidate_count: i64,
    #[model(key)]
    pub candidate_digest: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name="native_overload_candidates", validate=validate_candidate)]
#[assertion(support=NativeOverloadCandidateSupport, name="native_overload_candidate_supports", family=crate::domain::attribution::FactFamily::Types, subjects(scope,term), referents(origin))]
pub struct NativeOverloadCandidate {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    pub scope: Id<CoverageScope>,
    #[model(key)]
    pub trace: Id<NativeOverloadObservation>,
    #[model(key)]
    pub ordinal: i64,
    pub term: Id<TypeTerm>,
    pub origin: Option<Id<ProviderSymbol>>,
    /// Declaration binders are not solved call-specific substitutions.
    pub generic: bool,
    /// The supplier does not export a receiver adjustment basis for this trace.
    pub receiver_basis_required: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverloadCandidateInput {
    pub term: Id<TypeTerm>,
    pub origin: Option<Id<ProviderSymbol>>,
    pub generic: bool,
    pub receiver_basis_required: bool,
}
fn digest(values: &[OverloadCandidateInput]) -> ContentHash {
    let mut key = KeySink::new("native-overload-input-vector/v1");
    (values.len() as i64).encode(&mut key);
    for (i, v) in values.iter().enumerate() {
        (i as i64).encode(&mut key);
        v.term.encode(&mut key);
        v.origin.encode(&mut key);
        v.generic.encode(&mut key);
        v.receiver_basis_required.encode(&mut key);
    }
    key.finish()
}
impl NativeOverloadObservation {
    pub fn new(
        qualification: Id<AssertionQualification>,
        scope: Id<CoverageScope>,
        site: Id<Occurrence>,
        arguments: Id<Occurrence>,
        selection: OverloadSelection,
        closest_ordinal: i64,
        values: &[OverloadCandidateInput],
    ) -> Result<(Self, Vec<NativeOverloadCandidate>), ModelError> {
        if values.len() > MAX_OVERLOAD_CANDIDATES {
            return Err(ModelError::Limit {
                owner: "native-overload",
                limit: "candidates",
                observed: values.len(),
                bound: MAX_OVERLOAD_CANDIDATES,
            });
        }
        let row = Self {
            qualification,
            scope,
            site,
            arguments,
            selection,
            closest_ordinal,
            candidate_count: values.len() as i64,
            candidate_digest: digest(values),
        };
        row.validate()?;
        let members = values
            .iter()
            .enumerate()
            .map(|(i, v)| NativeOverloadCandidate {
                qualification: row.qualification,
                scope: row.scope,
                trace: row.id(),
                ordinal: i as i64,
                term: v.term,
                origin: v.origin,
                generic: v.generic,
                receiver_basis_required: v.receiver_basis_required,
            })
            .collect();
        Ok((row, members))
    }
    pub fn verify_candidates(&self, values: &[NativeOverloadCandidate]) -> Result<(), ModelError> {
        self.validate()?;
        if values.len() as i64 != self.candidate_count
            || values.iter().enumerate().any(|(i, v)| {
                v.qualification != self.qualification
                    || v.scope != self.scope
                    || v.trace != self.id()
                    || v.ordinal != i as i64
            })
        {
            return Err(invalid("native overload candidate membership differs"));
        }
        let inputs = values
            .iter()
            .map(|v| OverloadCandidateInput {
                term: v.term,
                origin: v.origin,
                generic: v.generic,
                receiver_basis_required: v.receiver_basis_required,
            })
            .collect::<Vec<_>>();
        if digest(&inputs) != self.candidate_digest {
            return Err(invalid("native overload candidate origin digest differs"));
        }
        Ok(())
    }
}
fn validate_trace(r: &NativeOverloadObservation) -> Result<(), ModelError> {
    if !(1..=MAX_OVERLOAD_CANDIDATES as i64).contains(&r.candidate_count)
        || !(0..r.candidate_count).contains(&r.closest_ordinal)
        || (r.selection == OverloadSelection::Resolved && r.candidate_count != 1)
    {
        return Err(invalid(
            "native overload vector or representative outside its domain",
        ));
    }
    Ok(())
}
fn validate_candidate(r: &NativeOverloadCandidate) -> Result<(), ModelError> {
    if !(0..MAX_OVERLOAD_CANDIDATES as i64).contains(&r.ordinal) {
        return Err(invalid(
            "native overload candidate ordinal outside its domain",
        ));
    }
    Ok(())
}
pub(crate) fn trace_invariants() -> Vec<Invariant> {
    vec![Invariant {
        revision: 1,
        name: "native_overload_original_membership",
        inputs: vec![
            ValidationInput::of::<Occurrence>(&["id"]),
            ValidationInput::of::<AssertionQualification>(&["id"]),
            ValidationInput::of::<SyntaxPlacement>(&["id"]),
            ValidationInput::of::<TypeObservation>(&["id"]),
            ValidationInput::of::<NativeOverloadObservation>(&["id"]),
            ValidationInput::of::<NativeOverloadCandidate>(&["trace", "ordinal", "id"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(Check {
                charge: StateCharge::new(budget, "native-overload-replay"),
                ..Default::default()
            })
        }),
    }]
}
#[derive(Default)]
struct Check {
    charge: StateCharge,
    occurrences: ChargedMap<Id<Occurrence>, Occurrence>,
    qualifications: ChargedMap<Id<AssertionQualification>, AssertionQualification>,
    placements: ChargedMap<Id<SyntaxPlacement>, SyntaxPlacement>,
    observations: ChargedMap<Id<TypeObservation>, TypeObservation>,
    traces: ChargedMap<Id<NativeOverloadObservation>, NativeOverloadObservation>,
    candidates: ChargedMap<(Id<NativeOverloadObservation>, i64), NativeOverloadCandidate>,
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
        rows!(AssertionQualification, qualifications);
        rows!(SyntaxPlacement, placements);
        rows!(TypeObservation, observations);
        rows!(NativeOverloadObservation, traces);
        if name == NativeOverloadCandidate::NAME {
            for row in NativeOverloadCandidate::decode(batch)? {
                row.validate()?;
                self.candidates
                    .insert(&mut self.charge, (row.trace, row.ordinal), row)?;
            }
            return Ok(());
        }
        Err(invalid("undeclared native overload input"))
    }
    fn finish(mut self: Box<Self>) -> Result<(), ModelError> {
        for candidate in self.candidates.values() {
            if !self.traces.contains_key(&candidate.trace) {
                return Err(invalid("orphan native overload candidate"));
            }
        }
        let mut placements: ChargedMap<
            (Id<Occurrence>, Id<AnalysisContext>),
            Vec<&SyntaxPlacement>,
        > = Default::default();
        for placement in self.placements.values() {
            let q = self
                .qualifications
                .get(&placement.qualification)
                .ok_or_else(|| invalid("overload syntax qualification missing"))?;
            placements.update(&mut self.charge, (placement.occurrence, q.context), |v| {
                v.push(placement)
            })?;
        }
        for trace in self.traces.values() {
            let q = self
                .qualifications
                .get(&trace.qualification)
                .ok_or_else(|| invalid("overload qualification missing"))?;
            let site = self
                .occurrences
                .get(&trace.site)
                .ok_or_else(|| invalid("overload call missing"))?;
            let arguments = self
                .occurrences
                .get(&trace.arguments)
                .ok_or_else(|| invalid("overload Arguments missing"))?;
            let matches = placements
                .get(&(trace.arguments, q.context))
                .map(Vec::as_slice)
                .unwrap_or_default();
            if site.syntax_kind != SyntaxKind::ExprCall
                || arguments.syntax_kind != SyntaxKind::Arguments
                || site.source != arguments.source
                || q.scope != trace.scope
                || matches.len() != 1
                || matches[0].parent != Some(trace.site)
                || matches[0].field != crate::domain::lexical::SyntaxField::Child
            {
                return Err(invalid(
                    "native overload Arguments/site/view/context mismatch",
                ));
            }
            self.charge.grow(trace.candidate_count as usize * 512)?;
            let members = self
                .candidates
                .range((trace.id(), 0)..=(trace.id(), i64::MAX))
                .map(|(_, v)| v.clone())
                .collect::<Vec<_>>();
            trace.verify_candidates(&members)?;
        }
        let mut sites: ChargedMap<(Id<Occurrence>, Id<AssertionQualification>), OverloadSelection> =
            Default::default();
        for trace in self.traces.values() {
            if sites
                .insert(
                    &mut self.charge,
                    (trace.site, trace.qualification),
                    trace.selection,
                )?
                .is_some()
            {
                return Err(invalid(
                    "multiple native vectors for one call qualification",
                ));
            }
        }
        for observation in self.observations.values().filter(|o| {
            matches!(
                o.role,
                TypeRole::ChosenOverload | TypeRole::OverloadCandidates
            )
        }) {
            let selection = sites
                .get(&(observation.subject, observation.qualification))
                .ok_or_else(|| invalid("overload typing lacks original native vector"))?;
            if observation.role == TypeRole::ChosenOverload
                && !matches!(
                    selection,
                    OverloadSelection::Resolved | OverloadSelection::Selected
                )
            {
                return Err(invalid(
                    "representative or recovered native result is not a chosen overload",
                ));
            }
        }
        Ok(())
    }
}

pub(crate) fn trace_invariants_refs() -> Vec<&'static str> {
    vec!["native_overload_original_membership"]
}
