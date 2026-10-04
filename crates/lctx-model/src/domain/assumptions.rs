//! Explicit, bounded premises of conditional claims. A premise refers to lower native evidence;
//! it never refers to the assessment whose qualification contains it.
use super::{
    assertion::{Approximation, AssertionQualification, Support},
    attribution::{AnalysisContext, Fidelity, Modality, ProviderRun},
    charged::{ChargedMap, StateCharge},
    conditions::Diagram,
    input::InputRevision,
    symbols::{ClassTraitObservation, ClassTraitSupport},
    types::{TypeObservation, TypeSupport, TypeTerm},
    *,
};
use crate::{Domain, DomainSum};
use std::collections::BTreeSet;

pub const MAX_ASSUMPTIONS: usize = 4096;
fn invalid(message: &str) -> ModelError {
    ModelError::Invalid(message.into())
}

/// Exact identity of the world in which a no-extra-overrides premise is asserted. The digest
/// alone is not closure evidence: the late model owner must publish UniverseSupport.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "assumption_universes")]
pub struct AssumptionUniverse {
    #[model(key)]
    pub context: Id<AnalysisContext>,
    #[model(key)]
    pub input: Id<InputRevision>,
    #[model(key)]
    pub environment: ContentHash,
    #[model(key)]
    pub model_definition: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "assumptions", rule = "conditional_claim_premise")]
pub enum Assumption {
    #[model(code = 0)]
    TypeConformance {
        #[model(premise)]
        observation: Id<TypeObservation>,
        #[model(premise)]
        support: Id<TypeSupport>,
    },
    #[model(code = 1)]
    NoExtraOverrides {
        #[model(premise)]
        class: Id<ClassTraitObservation>,
        #[model(premise)]
        support: Id<ClassTraitSupport>,
        #[model(premise)]
        universe: Id<AssumptionUniverse>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "assumption_sets", invariants = invariants, validate = validate_set)]
pub struct AssumptionSet {
    #[model(key)]
    pub members: ContentHash,
    #[model(key)]
    pub count: i64,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "assumption_set_members")]
pub struct AssumptionSetMember {
    #[model(key)]
    pub set: Id<AssumptionSet>,
    #[model(key)]
    pub assumption: Id<Assumption>,
}
fn digest(values: &BTreeSet<Id<Assumption>>) -> ContentHash {
    let mut sink = KeySink::new("claim-assumption-set");
    for id in values {
        id.encode(&mut sink);
    }
    (values.len() as i64).encode(&mut sink);
    sink.finish()
}
fn validate_set(set: &AssumptionSet) -> Result<(), ModelError> {
    if set.count < 0
        || set.count as usize > MAX_ASSUMPTIONS
        || (set.count == 0 && *set != AssumptionSet::empty())
    {
        return Err(invalid("invalid canonical assumption set"));
    }
    Ok(())
}
impl AssumptionSet {
    pub fn empty() -> Self {
        Self {
            members: digest(&BTreeSet::new()),
            count: 0,
        }
    }
    pub fn empty_id() -> Id<Self> {
        Self::empty().id()
    }
    #[allow(
        clippy::new_ret_no_self,
        reason = "Canonical set construction also returns its required membership rows as one resolved basis."
    )]
    pub fn new(
        values: impl IntoIterator<Item = Id<Assumption>>,
    ) -> Result<ResolvedAssumptions, ModelError> {
        let mut ids = BTreeSet::new();
        for id in values {
            ids.insert(id);
            if ids.len() > MAX_ASSUMPTIONS {
                return Err(ModelError::Limit {
                    owner: "claim_assumptions",
                    limit: "members",
                    observed: ids.len(),
                    bound: MAX_ASSUMPTIONS,
                });
            }
        }
        let set = Self {
            members: digest(&ids),
            count: ids.len() as i64,
        };
        let members = ids
            .into_iter()
            .map(|assumption| AssumptionSetMember {
                set: set.id(),
                assumption,
            })
            .collect();
        Ok(ResolvedAssumptions { set, members })
    }
}
/// A resolved set, including an explicit row for the empty basis. Members are sorted and unique.
/// Definition/source resolution is owned by AssumptionIndex and its stored invariant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedAssumptions {
    pub set: AssumptionSet,
    pub members: Vec<AssumptionSetMember>,
}
impl HeapSize for ResolvedAssumptions {
    fn heap_bytes(&self) -> usize {
        self.members.capacity() * size_of::<AssumptionSetMember>()
    }
}
impl ResolvedAssumptions {
    pub fn empty() -> Self {
        Self {
            set: AssumptionSet::empty(),
            members: Vec::new(),
        }
    }
    pub fn union<'a>(sets: impl IntoIterator<Item = &'a Self>) -> Result<Self, ModelError> {
        AssumptionSet::new(
            sets.into_iter()
                .flat_map(|s| s.members.iter().map(|m| m.assumption)),
        )
    }
    pub fn check(&self, expected: Id<AssumptionSet>) -> Result<(), ModelError> {
        let canonical = AssumptionSet::new(self.members.iter().map(|m| m.assumption))?;
        if self.set.id() != expected || canonical != *self {
            return Err(invalid("assumption membership is not canonical"));
        }
        Ok(())
    }
}
/// Immutable resolution view used by vocabulary owners without copying their charged maps.
pub trait AssumptionResolver {
    fn resolve(&self, id: Id<AssumptionSet>) -> Result<ResolvedAssumptions, ModelError>;
}
pub struct AssumptionCatalog<'a> {
    pub sets: &'a std::collections::BTreeMap<Id<AssumptionSet>, AssumptionSet>,
    pub members: &'a std::collections::BTreeMap<Id<AssumptionSetMember>, AssumptionSetMember>,
    pub definitions: &'a std::collections::BTreeMap<Id<Assumption>, Assumption>,
}
impl AssumptionResolver for AssumptionCatalog<'_> {
    fn resolve(&self, id: Id<AssumptionSet>) -> Result<ResolvedAssumptions, ModelError> {
        let expected = self
            .sets
            .get(&id)
            .ok_or_else(|| invalid("assumption set missing"))?;
        let result = AssumptionSet::new(
            self.members
                .values()
                .filter(|m| m.set == id)
                .map(|m| m.assumption),
        )?;
        if result.set != *expected {
            return Err(invalid("assumption membership missing or digest differs"));
        }
        for member in &result.members {
            if !self.definitions.contains_key(&member.assumption) {
                return Err(invalid("assumption definition missing"));
            }
        }
        Ok(result)
    }
}
/// Shared charged hydration for all qualification consumers. No absent-basis fallback exists.
#[derive(Debug)]
pub struct AssumptionIndex {
    charge: StateCharge,
    pub sets: ChargedMap<Id<AssumptionSet>, AssumptionSet>,
    pub members: ChargedMap<Id<AssumptionSet>, BTreeSet<Id<Assumption>>>,
    pub definitions: ChargedMap<Id<Assumption>, Assumption>,
    pub universes: ChargedMap<Id<AssumptionUniverse>, AssumptionUniverse>,
}
impl AssumptionIndex {
    pub fn new(budget: &resources::ResourceBudget) -> Self {
        Self {
            charge: StateCharge::new(budget, "claim_assumption_index"),
            sets: Default::default(),
            members: Default::default(),
            definitions: Default::default(),
            universes: Default::default(),
        }
    }
    pub fn inputs() -> Vec<ValidationInput> {
        vec![
            ValidationInput::of::<AssumptionSet>(&["id"]),
            ValidationInput::of::<AssumptionSetMember>(&["set", "assumption"]),
            ValidationInput::of::<Assumption>(&["id"]),
            ValidationInput::of::<AssumptionUniverse>(&["id"]),
        ]
    }
    pub fn visit(
        &mut self,
        name: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<bool, ModelError> {
        macro_rules! rows {
            ($ty:ty, $field:ident) => {
                if name == <$ty>::NAME {
                    for row in <$ty>::decode(batch)? {
                        self.$field.insert(&mut self.charge, row.id(), row)?;
                    }
                    return Ok(true);
                }
            };
        }
        rows!(AssumptionSet, sets);
        rows!(Assumption, definitions);
        rows!(AssumptionUniverse, universes);
        if name == AssumptionSetMember::NAME {
            for row in AssumptionSetMember::decode(batch)? {
                if !self
                    .members
                    .update(&mut self.charge, row.set, |ids| ids.insert(row.assumption))?
                {
                    return Err(invalid("duplicate assumption membership"));
                }
                if self.members[&row.set].len() > MAX_ASSUMPTIONS {
                    return Err(invalid("assumption membership work limit"));
                }
            }
            return Ok(true);
        }
        Ok(false)
    }
    pub fn insert(&mut self, basis: &ResolvedAssumptions) -> Result<(), ModelError> {
        basis.check(basis.set.id())?;
        self.sets
            .insert(&mut self.charge, basis.set.id(), basis.set.clone())?;
        for member in &basis.members {
            self.members.update(&mut self.charge, member.set, |ids| {
                ids.insert(member.assumption)
            })?;
        }
        Ok(())
    }
    pub fn resolve(&self, id: Id<AssumptionSet>) -> Result<ResolvedAssumptions, ModelError> {
        let set = self
            .sets
            .get(&id)
            .ok_or_else(|| invalid("assumption set missing"))?;
        if set.count < 0 || set.count as usize > MAX_ASSUMPTIONS {
            return Err(invalid("invalid assumption set bound"));
        }
        let ids = self.members.get(&id).cloned().unwrap_or_default();
        let result = AssumptionSet::new(ids)?;
        if result.set != *set {
            return Err(invalid(
                "assumption set membership missing or digest differs",
            ));
        }
        for member in &result.members {
            if !self.definitions.contains_key(&member.assumption) {
                return Err(invalid("assumption definition missing"));
            }
        }
        Ok(result)
    }
    pub fn finish(&self) -> Result<(), ModelError> {
        for id in self.sets.keys() {
            self.resolve(*id)?;
        }
        for id in self.members.keys() {
            if !self.sets.contains_key(id) {
                return Err(invalid("orphan assumption membership"));
            }
        }
        Ok(())
    }
}

fn invariants() -> Vec<Invariant> {
    let mut inputs = AssumptionIndex::inputs();
    inputs.extend(ownership::ScopeIndex::inputs());
    inputs.push(ValidationInput::of::<calls::ProviderSymbol>(&["id"]));
    inputs.extend([
        ValidationInput::of::<AnalysisContext>(&["id"]),
        ValidationInput::of::<ProviderRun>(&["id"]),
        ValidationInput::of::<AssertionQualification>(&["id"]),
        ValidationInput::of::<TypeObservation>(&["id"]),
        ValidationInput::of::<TypeSupport>(&["id"]),
        ValidationInput::of::<TypeTerm>(&["id"]),
        ValidationInput::of::<ClassTraitObservation>(&["id"]),
        ValidationInput::of::<ClassTraitSupport>(&["id"]),
    ]);
    vec![Invariant {
        name: "claim_assumption_basis",
        inputs,
        create: std::sync::Arc::new(|budget| Box::new(Check::new(budget))),
    }]
}
struct Check {
    charge: StateCharge,
    index: AssumptionIndex,
    scope: ownership::ScopeIndex,
    symbols: ChargedMap<Id<calls::ProviderSymbol>, calls::ProviderSymbol>,
    contexts: ChargedMap<Id<AnalysisContext>, AnalysisContext>,
    runs: ChargedMap<Id<ProviderRun>, ProviderRun>,
    qualifications: ChargedMap<Id<AssertionQualification>, AssertionQualification>,
    types: ChargedMap<Id<TypeObservation>, TypeObservation>,
    type_supports: ChargedMap<Id<TypeSupport>, TypeSupport>,
    terms: ChargedMap<Id<TypeTerm>, TypeTerm>,
    classes: ChargedMap<Id<ClassTraitObservation>, ClassTraitObservation>,
    class_supports: ChargedMap<Id<ClassTraitSupport>, ClassTraitSupport>,
}
impl Check {
    fn new(budget: &resources::ResourceBudget) -> Self {
        Self {
            charge: StateCharge::new(budget, "claim_assumption_validation"),
            index: AssumptionIndex::new(budget),
            scope: ownership::ScopeIndex::new(budget, "claim_assumption_scope"),
            symbols: Default::default(),
            contexts: Default::default(),
            runs: Default::default(),
            qualifications: Default::default(),
            types: Default::default(),
            type_supports: Default::default(),
            terms: Default::default(),
            classes: Default::default(),
            class_supports: Default::default(),
        }
    }
    fn native<S: Support>(
        &self,
        support: &S,
        observation: Id<S::Assertion>,
        q: Id<AssertionQualification>,
    ) -> Result<&ProviderRun, ModelError> {
        let q = self
            .qualifications
            .get(&q)
            .ok_or_else(|| invalid("lower premise qualification missing"))?;
        if q.assumptions != AssumptionSet::empty_id()
            || q.condition != Diagram::always().id()
            || q.modality != Modality::Definite
            || q.approximation != Approximation::Exact
        {
            return Err(invalid(
                "assumption needs lower unconditional native evidence",
            ));
        }
        let attribution = support
            .attribution()
            .ok_or_else(|| invalid("assumption needs actual native support"))?;
        if support.assertion() != observation || attribution.fidelity != Fidelity::NativeStructural
        {
            return Err(invalid(
                "assumption changes native observation/support pair",
            ));
        }
        let run = self
            .runs
            .get(&attribution.run)
            .ok_or_else(|| invalid("assumption native run missing"))?;
        if run.context != q.context {
            return Err(invalid("assumption crosses native context"));
        }
        Ok(run)
    }
    fn frame(&self, definition: &Assumption) -> Result<&ProviderRun, ModelError> {
        match definition {
            Assumption::TypeConformance {
                observation,
                support,
            } => {
                let row = self
                    .types
                    .get(observation)
                    .ok_or_else(|| invalid("type conformance observation missing"))?;
                let support = self
                    .type_supports
                    .get(support)
                    .ok_or_else(|| invalid("type conformance support missing"))?;
                let term = self
                    .terms
                    .get(&row.term)
                    .ok_or_else(|| invalid("type conformance term missing"))?;
                if matches!(
                    term,
                    TypeTerm::Any { .. } | TypeTerm::Other { .. } | TypeTerm::Truncated { .. }
                ) {
                    return Err(invalid("type conformance premise unknown"));
                }
                self.native(support, row.id(), row.qualification)
            }
            Assumption::NoExtraOverrides {
                class,
                support,
                universe,
            } => {
                let row = self
                    .classes
                    .get(class)
                    .ok_or_else(|| invalid("override class observation missing"))?;
                let symbol = self
                    .symbols
                    .get(&row.symbol)
                    .ok_or_else(|| invalid("assumption class symbol missing"))?;
                if symbol.kind != calls::SymbolKind::Class {
                    return Err(invalid("override premise needs provider class"));
                }
                let support = self
                    .class_supports
                    .get(support)
                    .ok_or_else(|| invalid("override class support missing"))?;
                let run = self.native(support, row.id(), row.qualification)?;
                if (symbol.provider, symbol.context) != (run.provider, run.context) {
                    return Err(invalid("override provider class crosses native context"));
                }
                let universe = self
                    .index
                    .universes
                    .get(universe)
                    .ok_or_else(|| invalid("assumption universe missing"))?;
                let context = self
                    .contexts
                    .get(&universe.context)
                    .ok_or_else(|| invalid("assumption universe context missing"))?;
                if universe.context != run.context
                    || universe.input != run.input
                    || universe.environment != context.environment_digest
                {
                    return Err(invalid(
                        "assumption crosses pinned input/context/environment",
                    ));
                }
                Ok(run)
            }
        }
    }
}
impl InvariantCheck for Check {
    fn visit(&mut self, name: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        if self.index.visit(name, batch)? || self.scope.visit(name, batch)? {
            return Ok(());
        }
        macro_rules! rows {
            ($ty:ty, $field:ident) => {
                if name == <$ty>::NAME {
                    for row in <$ty>::decode(batch)? {
                        self.$field.insert(&mut self.charge, row.id(), row)?;
                    }
                    return Ok(());
                }
            };
        }
        rows!(calls::ProviderSymbol, symbols);
        rows!(AnalysisContext, contexts);
        rows!(ProviderRun, runs);
        rows!(AssertionQualification, qualifications);
        rows!(TypeObservation, types);
        rows!(TypeSupport, type_supports);
        rows!(TypeTerm, terms);
        rows!(ClassTraitObservation, classes);
        rows!(ClassTraitSupport, class_supports);
        Err(invalid("undeclared assumption validation input"))
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        self.index.finish()?;
        for universe in self.index.universes.values() {
            let context = self
                .contexts
                .get(&universe.context)
                .ok_or_else(|| invalid("assumption universe context missing"))?;
            if universe.environment != context.environment_digest {
                return Err(invalid("assumption universe environment differs"));
            }
        }
        for definition in self.index.definitions.values() {
            self.frame(definition)?;
        }
        for q in self.qualifications.values() {
            let basis = self.index.resolve(q.assumptions)?;
            let mut input = None;
            for member in &basis.members {
                let run = self.frame(&self.index.definitions[&member.assumption])?;
                if run.context != q.context
                    || input.is_some_and(|id| id != run.input)
                    || !self
                        .scope
                        .owns_scope(run.input, self.scope.scope(q.scope)?)?
                {
                    return Err(invalid("qualification crosses assumption context/input"));
                }
                input = Some(run.input);
            }
        }
        Ok(())
    }
}

impl AssumptionResolver for AssumptionIndex {
    fn resolve(&self, id: Id<AssumptionSet>) -> Result<ResolvedAssumptions, ModelError> {
        AssumptionIndex::resolve(self, id)
    }
}
