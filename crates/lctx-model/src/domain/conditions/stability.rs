//! Stability witnesses and witnessed guard substitution (DESIGN §15.6, core review C06).
//!
//! A callee guard such as `timeout is None` on a formal may be restated at a call only when the
//! guard's read of the formal is reached by the parameter's own definition alone, under complete
//! flow coverage. The witness is a derivation over those flow facts; a substitution names the
//! caller-side `BoundGuard` atom, its witness and the call argument whose value it tests.
use super::super::charged::{ChargedMap, ChargedSet, StateCharge};
use super::super::{
    assertion::AssertionQualification,
    attribution::{CoverageStatus, FactFamily, ProviderCoverage},
    calls::{CallArgument, CallSyntax},
    flow::{
        FlowDefinition, FlowDefinitionObservation, FlowReachingObservation, FlowUse,
        ReachingDefinition,
    },
    lexical::BindingEventKind,
    ownership::ScopeIndex,
    source::Occurrence,
    value::{AccessPath, Place, PlaceRoot, Predicate},
    *,
};
use super::EvaluationAtom;
use crate::{Domain, DomainCode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum StabilityBasis {
    ParameterOnlyReaching = 0,
}

/// The guard's read of its formal is reached only by the parameter definition, not loop-carried,
/// under complete flow coverage of the read. Any other reach refuses: an assignment, an unbound
/// path, a binding from a nested scope (`nonlocal`), or a second reaching observation. Writes
/// through frame objects (`sys._getframe().f_locals`, PEP 667) are `DynamicAccess`, outside the
/// stated model.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "stability_witnesses", rule = "parameter_only_reaching", conclusion = atom, invariants = stability_invariants)]
pub struct StabilityWitness {
    #[model(key)]
    pub atom: Id<EvaluationAtom>,
    #[model(key, premise)]
    pub reaching: Id<FlowReachingObservation>,
    #[model(key, premise)]
    pub definition: Id<FlowDefinitionObservation>,
    #[model(key, premise)]
    pub coverage: Id<ProviderCoverage>,
    #[model(key)]
    pub basis: StabilityBasis,
}
/// The caller-side `BoundGuard` atom, justified by a witness and the argument it tests.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "guard_substitutions", rule = "witnessed_guard_substitution", conclusion = atom)]
pub struct GuardSubstitution {
    #[model(key)]
    pub atom: Id<EvaluationAtom>,
    #[model(key, premise)]
    pub witness: Id<StabilityWitness>,
    #[model(key, premise)]
    pub argument: Id<CallArgument>,
}
/// Guards whose truth depends only on the tested value's identity with `None` or a literal.
pub fn substitutable(predicate: &Predicate) -> bool {
    matches!(predicate, Predicate::IsNone | Predicate::IsValue { .. })
}
fn invalid(message: &str) -> ModelError {
    ModelError::Invalid(message.into())
}

fn stability_invariants() -> Vec<Invariant> {
    let mut inputs = ScopeIndex::inputs();
    inputs.extend([
        ValidationInput::of::<Occurrence>(&["id"]),
        ValidationInput::of::<AssertionQualification>(&["id"]),
        ValidationInput::of::<Predicate>(&["id"]),
        ValidationInput::of::<PlaceRoot>(&["id"]),
        ValidationInput::of::<AccessPath>(&["id"]),
        ValidationInput::of::<Place>(&["id"]),
        ValidationInput::of::<EvaluationAtom>(&["id"]),
        ValidationInput::of::<FlowUse>(&["id"]),
        ValidationInput::of::<FlowDefinition>(&["id"]),
        ValidationInput::of::<ReachingDefinition>(&["id"]),
        ValidationInput::of::<FlowDefinitionObservation>(&["id"]),
        ValidationInput::of::<FlowReachingObservation>(&["id"]),
        ValidationInput::of::<ProviderCoverage>(&["id"]),
        ValidationInput::of::<CallSyntax>(&["id"]),
        ValidationInput::of::<CallArgument>(&["id"]),
        ValidationInput::of::<StabilityWitness>(&["id"]),
        ValidationInput::of::<GuardSubstitution>(&["id"]),
    ]);
    vec![Invariant {
        name: "witnessed_guard_substitution",
        inputs,
        create: std::sync::Arc::new(|budget| {
            Box::new(StabilityCheck {
                charge: StateCharge::new(budget, "witnessed_guard_substitution"),
                scopes: ScopeIndex::new(budget, "witnessed_guard_substitution"),
                ..Default::default()
            })
        }),
    }]
}
#[derive(Default)]
struct StabilityCheck {
    charge: StateCharge,
    scopes: ScopeIndex,
    occurrences: ChargedMap<Id<Occurrence>, Occurrence>,
    contexts:
        ChargedMap<Id<AssertionQualification>, Id<super::super::attribution::AnalysisContext>>,
    predicates: ChargedMap<Id<Predicate>, Predicate>,
    roots: ChargedMap<Id<PlaceRoot>, PlaceRoot>,
    empty_paths: ChargedSet<Id<AccessPath>>,
    places: ChargedMap<Id<Place>, Place>,
    atoms: ChargedMap<Id<EvaluationAtom>, EvaluationAtom>,
    uses: ChargedMap<Id<FlowUse>, FlowUse>,
    definitions: ChargedMap<Id<FlowDefinition>, FlowDefinition>,
    targets: ChargedMap<Id<ReachingDefinition>, ReachingDefinition>,
    definition_observations: ChargedMap<Id<FlowDefinitionObservation>, FlowDefinitionObservation>,
    reaching: ChargedMap<Id<FlowReachingObservation>, FlowReachingObservation>,
    /// Reaching observations per (use, analysis context).
    reaching_count: ChargedMap<(Id<FlowUse>, Id<super::super::attribution::AnalysisContext>), i64>,
    coverage: ChargedMap<Id<ProviderCoverage>, ProviderCoverage>,
    calls: ChargedMap<Id<CallSyntax>, Id<Occurrence>>,
    arguments: ChargedMap<Id<CallArgument>, CallArgument>,
    witnesses: ChargedMap<Id<StabilityWitness>, Id<EvaluationAtom>>,
    substituted: ChargedSet<Id<EvaluationAtom>>,
}
impl StabilityCheck {
    fn get<'a, K: Ord, V>(
        map: &'a ChargedMap<K, V>,
        key: &K,
        what: &str,
    ) -> Result<&'a V, ModelError> {
        map.get(key).ok_or_else(|| invalid(what))
    }
    fn inside(&self, child: Id<Occurrence>, parent: Id<Occurrence>) -> Result<bool, ModelError> {
        let (child, parent) = (
            Self::get(&self.occurrences, &child, "witness occurrence absent")?,
            Self::get(&self.occurrences, &parent, "witness occurrence absent")?,
        );
        Ok(child.source == parent.source
            && child.structural_path.len() > parent.structural_path.len()
            && child.structural_path.starts_with(&parent.structural_path))
    }
    /// The atom's operand, as (root, empty path?).
    fn operand(&self, atom: &EvaluationAtom) -> Result<(Id<Place>, &PlaceRoot, bool), ModelError> {
        let place_id = atom
            .operand
            .ok_or_else(|| invalid("witnessed guard has no operand"))?;
        let place = Self::get(&self.places, &place_id, "witness place absent")?;
        Ok((
            place_id,
            Self::get(&self.roots, &place.root, "witness place root absent")?,
            self.empty_paths.contains(&place.path),
        ))
    }
    fn witness(&self, row: &StabilityWitness) -> Result<(), ModelError> {
        let atom = Self::get(&self.atoms, &row.atom, "witnessed atom absent")?;
        if !substitutable(Self::get(
            &self.predicates,
            &atom.predicate,
            "witnessed predicate absent",
        )?) {
            return Err(invalid("guard predicate is not substitutable"));
        }
        let (place, root, empty) = self.operand(atom)?;
        let PlaceRoot::Formal { declaration } = root else {
            return Err(invalid("witnessed operand is not a formal"));
        };
        if !empty {
            return Err(invalid("witnessed operand must be the whole formal"));
        }
        let reaching = Self::get(
            &self.reaching,
            &row.reaching,
            "witness reaching observation absent",
        )?;
        if reaching.loop_carried
            || Self::get(
                &self.contexts,
                &reaching.qualification,
                "reaching qualification absent",
            )? != &atom.context
        {
            return Err(invalid(
                "witness reaching is loop-carried or in another context",
            ));
        }
        let read = Self::get(&self.uses, &reaching.use_, "witness use absent")?;
        if read.place != place || !self.inside(read.occurrence, atom.evaluation)? {
            return Err(invalid(
                "witness read is not the guard's read of its formal",
            ));
        }
        if self.reaching_count.get(&(reaching.use_, atom.context)) != Some(&1) {
            return Err(invalid(
                "the formal read has more than one reaching definition",
            ));
        }
        let ReachingDefinition::Bound { definition } = Self::get(
            &self.targets,
            &reaching.target,
            "witness reaching target absent",
        )?
        else {
            return Err(invalid(
                "witness reaching target is unbound or from a nested scope",
            ));
        };
        let observed = Self::get(
            &self.definition_observations,
            &row.definition,
            "witness definition observation absent",
        )?;
        let reached = Self::get(&self.definitions, definition, "witness definition absent")?;
        if observed.definition != *definition
            || observed.kind != BindingEventKind::Parameter
            || reached.occurrence != *declaration
            || reached.place != place
        {
            return Err(invalid(
                "the reaching definition is not the formal's parameter definition",
            ));
        }
        let coverage = Self::get(&self.coverage, &row.coverage, "witness coverage absent")?;
        let source = Self::get(&self.occurrences, &read.occurrence, "witness read absent")?.source;
        if coverage.family != FactFamily::Flow
            || coverage.status != CoverageStatus::CompleteUnderStatedModel
            || coverage.context != atom.context
            || !self
                .scopes
                .within(source, self.scopes.scope(coverage.scope)?)?
        {
            return Err(invalid("witness needs complete flow coverage of the read"));
        }
        Ok(())
    }
    fn substitution(&self, row: &GuardSubstitution) -> Result<(), ModelError> {
        let bound = Self::get(&self.atoms, &row.atom, "substituted atom absent")?;
        let witnessed = Self::get(&self.witnesses, &row.witness, "substitution witness absent")?;
        let Predicate::BoundGuard { source } = Self::get(
            &self.predicates,
            &bound.predicate,
            "substituted predicate absent",
        )?
        else {
            return Err(invalid("a substitution names a bound guard"));
        };
        if source != witnessed
            || Self::get(&self.atoms, source, "bound guard source absent")?.context != bound.context
        {
            return Err(invalid(
                "bound guard and witness name different guards or contexts",
            ));
        }
        let argument = Self::get(
            &self.arguments,
            &row.argument,
            "substituted argument absent",
        )?;
        let site = Self::get(&self.calls, &argument.call, "substituted call absent")?;
        let (_, root, empty) = self.operand(bound)?;
        if bound.evaluation != *site
            || !empty
            || *root
                != (PlaceRoot::Occurrence {
                    occurrence: argument.value,
                })
        {
            return Err(invalid(
                "bound guard must test the argument's value at its call",
            ));
        }
        Ok(())
    }
}
impl InvariantCheck for StabilityCheck {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if self.scopes.visit(relation, batch)? {
            return Ok(());
        }
        let c = &mut self.charge;
        if relation == Occurrence::NAME {
            for r in Occurrence::decode(batch)? {
                self.occurrences.insert(c, r.id(), r)?;
            }
        } else if relation == AssertionQualification::NAME {
            for r in AssertionQualification::decode(batch)? {
                self.contexts.insert(c, r.id(), r.context)?;
            }
        } else if relation == Predicate::NAME {
            for r in Predicate::decode(batch)? {
                self.predicates.insert(c, r.id(), r)?;
            }
        } else if relation == PlaceRoot::NAME {
            for r in PlaceRoot::decode(batch)? {
                self.roots.insert(c, r.id(), r)?;
            }
        } else if relation == AccessPath::NAME {
            for r in AccessPath::decode(batch)? {
                if r.first.is_none() && !r.unknown_suffix {
                    self.empty_paths.insert(c, r.id())?;
                }
            }
        } else if relation == Place::NAME {
            for r in Place::decode(batch)? {
                self.places.insert(c, r.id(), r)?;
            }
        } else if relation == EvaluationAtom::NAME {
            for r in EvaluationAtom::decode(batch)? {
                self.atoms.insert(c, r.id(), r)?;
            }
        } else if relation == FlowUse::NAME {
            for r in FlowUse::decode(batch)? {
                self.uses.insert(c, r.id(), r)?;
            }
        } else if relation == FlowDefinition::NAME {
            for r in FlowDefinition::decode(batch)? {
                self.definitions.insert(c, r.id(), r)?;
            }
        } else if relation == ReachingDefinition::NAME {
            for r in ReachingDefinition::decode(batch)? {
                self.targets.insert(c, r.id(), r)?;
            }
        } else if relation == FlowDefinitionObservation::NAME {
            for r in FlowDefinitionObservation::decode(batch)? {
                self.definition_observations.insert(c, r.id(), r)?;
            }
        } else if relation == FlowReachingObservation::NAME {
            for r in FlowReachingObservation::decode(batch)? {
                let context = *self
                    .contexts
                    .get(&r.qualification)
                    .ok_or_else(|| invalid("reaching qualification absent"))?;
                self.reaching_count
                    .update(c, (r.use_, context), |count| *count += 1)?;
                self.reaching.insert(c, r.id(), r)?;
            }
        } else if relation == ProviderCoverage::NAME {
            for r in ProviderCoverage::decode(batch)? {
                self.coverage.insert(c, r.id(), r)?;
            }
        } else if relation == CallSyntax::NAME {
            for r in CallSyntax::decode(batch)? {
                self.calls.insert(c, r.id(), r.site)?;
            }
        } else if relation == CallArgument::NAME {
            for r in CallArgument::decode(batch)? {
                self.arguments.insert(c, r.id(), r)?;
            }
        } else if relation == StabilityWitness::NAME {
            for r in StabilityWitness::decode(batch)? {
                self.witness(&r)?;
                self.witnesses.insert(&mut self.charge, r.id(), r.atom)?;
            }
        } else if relation == GuardSubstitution::NAME {
            for r in GuardSubstitution::decode(batch)? {
                self.substitution(&r)?;
                self.substituted.insert(&mut self.charge, r.atom)?;
            }
        } else {
            return Err(invalid("undeclared guard substitution input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        for atom in self.atoms.values() {
            if matches!(
                self.predicates.get(&atom.predicate),
                Some(Predicate::BoundGuard { .. })
            ) && !self.substituted.contains(&atom.id())
            {
                return Err(invalid("a bound guard needs a witnessed substitution"));
            }
        }
        Ok(())
    }
}
