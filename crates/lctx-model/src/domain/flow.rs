//! Raw flow observations. A reaching definition is not a value-transfer or call-summary proof.
//! Occurrences and structural places are shared with syntax and conditions; no rendered-place IDs.
use super::charged::{ChargedMap, StateCharge};
use super::{
    assertion::AssertionQualification,
    attribution::FactFamily,
    lexical::{BindingEventKind, LexicalScope},
    source::Occurrence,
    transfer::TransferKind,
    value::Place,
    *,
};
use crate::{Assertion, Domain, DomainCode, DomainSum};

#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "flow_uses", invariants = flow_invariants)]
pub struct FlowUse {
    #[model(key)]
    pub occurrence: Id<Occurrence>,
    #[model(key)]
    pub place: Id<Place>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "flow_definitions")]
pub struct FlowDefinition {
    #[model(key)]
    pub occurrence: Id<Occurrence>,
    #[model(key)]
    pub place: Id<Place>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name = "flow_use_observations")]
#[assertion(support = FlowUseSupport, name = "flow_use_supports", family = FactFamily::Flow, subjects(use_, scope))]
pub struct FlowUseObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub use_: Id<FlowUse>,
    #[model(key)]
    pub scope: Id<LexicalScope>,
    #[model(key)]
    pub annotation: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name = "flow_definition_observations")]
#[assertion(support = FlowDefinitionSupport, name = "flow_definition_supports", family = FactFamily::Flow, subjects(definition, scope, value))]
pub struct FlowDefinitionObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub definition: Id<FlowDefinition>,
    #[model(key)]
    pub scope: Id<LexicalScope>,
    #[model(key)]
    pub kind: BindingEventKind,
    #[model(key)]
    pub value: Option<Id<Occurrence>>,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "reaching_definitions")]
pub enum ReachingDefinition {
    #[model(code = 0)]
    Bound { definition: Id<FlowDefinition> },
    /// The provider observes a possibly unbound place on this path. This is not missing coverage.
    #[model(code = 1)]
    Unbound,
    /// A binding made from another scope may reach this use: a `nonlocal` or `global` write in a
    /// nested function, or a lazy snapshot of such bindings. The writing definition lives in the
    /// other scope, so no definition of this scope is named; it is never a parameter reach.
    #[model(code = 2)]
    Nested,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name = "flow_reaching_observations")]
#[assertion(support = FlowReachingSupport, name = "flow_reaching_supports", family = FactFamily::Flow, subjects(use_, target))]
pub struct FlowReachingObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub use_: Id<FlowUse>,
    #[model(key)]
    pub target: Id<ReachingDefinition>,
    #[model(key)]
    pub loop_carried: bool,
}
/// The provider's type-narrowing formula, never substituted for runtime reachability.
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name = "flow_narrowing_observations")]
#[assertion(support = FlowNarrowingSupport, name = "flow_narrowing_supports", family = FactFamily::Flow, subjects(use_, target))]
pub struct FlowNarrowingObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub use_: Id<FlowUse>,
    #[model(key)]
    pub target: Id<ReachingDefinition>,
    #[model(key)]
    pub precision_lost: bool,
}
/// Exact byte geometry does not make ty's transformed bytes the original source snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name = "flow_source_view_observations", validate = validate_view, invariants = view_invariants)]
#[assertion(support = FlowSourceViewSupport, name = "flow_source_view_supports", family = FactFamily::Flow, subjects(source))]
pub struct FlowSourceViewObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub source: Id<source::SourceArtifact>,
    #[model(key)]
    pub original_content: ContentHash,
    #[model(key)]
    pub view_content: ContentHash,
    #[model(key)]
    pub byte_len: i64,
    #[model(key)]
    pub renamed_type_checking: i64,
}
fn validate_view(row: &FlowSourceViewObservation) -> Result<(),ModelError> {
    if row.byte_len < 0 || row.renamed_type_checking < 0 || row.renamed_type_checking > row.byte_len/13 {
        return Err(ModelError::Invalid("invalid same-length ty source view".into()));
    }
    if (row.renamed_type_checking == 0) != (row.original_content == row.view_content) {
        return Err(ModelError::Invalid("ty view content disagrees with token renames".into()));
    }
    Ok(())
}
fn view_invariants() -> Vec<Invariant> {
    vec![Invariant {
        name:"flow_view_and_narrowing_precision",
        inputs:vec![ValidationInput::of::<source::SourceArtifact>(&["id"]),ValidationInput::of::<AssertionQualification>(&["id"]),ValidationInput::of::<FlowSourceViewObservation>(&["id"]),ValidationInput::of::<FlowNarrowingObservation>(&["id"])],
        create:std::sync::Arc::new(|budget|Box::new(ViewCheck {charge:StateCharge::new(budget,"flow_view_and_narrowing_precision"),..Default::default()})),
    }]
}
#[derive(Default)]
struct ViewCheck {
    charge:StateCharge,
    sources:ChargedMap<Id<source::SourceArtifact>,source::SourceArtifact>,
    qualifications:ChargedMap<Id<AssertionQualification>,AssertionQualification>,
    views:ChargedMap<Id<FlowSourceViewObservation>,FlowSourceViewObservation>,
    narrowing:ChargedMap<Id<FlowNarrowingObservation>,FlowNarrowingObservation>,
}
impl InvariantCheck for ViewCheck {
    fn visit(&mut self,relation:&str,batch:&arrow_array::RecordBatch)->Result<(),ModelError> {
        if relation==source::SourceArtifact::NAME {for row in source::SourceArtifact::decode(batch)? {self.sources.insert(&mut self.charge,row.id(),row)?;}}
        else if relation==AssertionQualification::NAME {for row in AssertionQualification::decode(batch)? {self.qualifications.insert(&mut self.charge,row.id(),row)?;}}
        else if relation==FlowSourceViewObservation::NAME {for row in FlowSourceViewObservation::decode(batch)? {row.validate()?;self.views.insert(&mut self.charge,row.id(),row)?;}}
        else if relation==FlowNarrowingObservation::NAME {for row in FlowNarrowingObservation::decode(batch)? {self.narrowing.insert(&mut self.charge,row.id(),row)?;}}
        else {return Err(ModelError::Invalid("undeclared flow view validation input".into()));}
        Ok(())
    }
    fn finish(self:Box<Self>)->Result<(),ModelError> {
        for row in self.views.values() {
            let source=self.sources.get(&row.source).ok_or_else(||ModelError::Invalid("flow view source missing".into()))?;
            if source.content!=row.original_content || source.byte_len!=row.byte_len {return Err(ModelError::Invalid("flow view source snapshot mismatch".into()));}
        }
        for row in self.narrowing.values() {
            let q=self.qualifications.get(&row.qualification).ok_or_else(||ModelError::Invalid("flow narrowing qualification missing".into()))?;
            if row.precision_lost && matches!(q.approximation,super::assertion::Approximation::Exact|super::assertion::Approximation::Under) {
                return Err(ModelError::Invalid("precision-lost narrowing cannot certify exact or under-approximate truth".into()));
            }
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum FlowSinkKind {
    Definition = 0,
    Argument = 1,
    Return = 2,
    Yield = 3,
    Raise = 4,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name = "flow_value_observations", validate = validate_value)]
#[assertion(support = FlowValueSupport, name = "flow_value_supports", family = FactFamily::Flow, subjects(use_, sink))]
pub struct FlowValueObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub use_: Id<FlowUse>,
    #[model(key)]
    pub sink: Id<Occurrence>,
    #[model(key)]
    pub kind: FlowSinkKind,
    #[model(key)]
    pub transfer: TransferKind,
    /// A raw dependency crossing a call requires later call-transfer evidence.
    #[model(key)]
    pub through_call: bool,
}
fn invalid(message: &str) -> ModelError {
    ModelError::Invalid(message.into())
}
fn validate_value(row: &FlowValueObservation) -> Result<(), ModelError> {
    if row.transfer == TransferKind::Identity && row.through_call {
        return Err(invalid("identity flow cannot cross an unresolved call"));
    }
    Ok(())
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name = "flow_region_observations")]
#[assertion(support = FlowRegionSupport, name = "flow_region_supports", family = FactFamily::Flow, subjects(statement, scope))]
pub struct FlowRegionObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub statement: Id<Occurrence>,
    #[model(key)]
    pub scope: Id<LexicalScope>,
}

fn flow_invariants() -> Vec<Invariant> {
    vec![
        Invariant {
            name: "flow_reaching_places",
            inputs: vec![
                ValidationInput::of::<FlowUse>(&["id"]),
                ValidationInput::of::<FlowDefinition>(&["id"]),
                ValidationInput::of::<ReachingDefinition>(&["id"]),
                ValidationInput::of::<FlowReachingObservation>(&["id"]),
            ],
            create: std::sync::Arc::new(|budget| {
                Box::new(ReachingCheck {
                    charge: StateCharge::new(budget, "flow_reaching_places"),
                    ..Default::default()
                })
            }),
        },
        Invariant {
            name: "flow_source_structure",
            inputs: vec![
                ValidationInput::of::<Occurrence>(&["id"]),
                ValidationInput::of::<LexicalScope>(&["id"]),
                ValidationInput::of::<FlowUse>(&["id"]),
                ValidationInput::of::<FlowDefinition>(&["id"]),
                ValidationInput::of::<FlowUseObservation>(&["id"]),
                ValidationInput::of::<FlowDefinitionObservation>(&["id"]),
                ValidationInput::of::<FlowValueObservation>(&["id"]),
                ValidationInput::of::<FlowRegionObservation>(&["id"]),
                ValidationInput::of::<super::conditions::EvaluationAtom>(&["id"]),
                ValidationInput::of::<FlowTestObservation>(&["id"]),
                ValidationInput::of::<FlowTestLeafObservation>(&["id"]),
                ValidationInput::of::<FlowAttributeLoadObservation>(&["id"]),
            ],
            create: std::sync::Arc::new(|budget| {
                Box::new(FlowStructure {
                    charge: StateCharge::new(budget, "flow_source_structure"),
                    ..Default::default()
                })
            }),
        },
    ]
}
#[derive(Default)]
struct ReachingCheck {
    charge: StateCharge,
    uses: ChargedMap<Id<FlowUse>, Id<Place>>,
    definitions: ChargedMap<Id<FlowDefinition>, Id<Place>>,
    targets: ChargedMap<Id<ReachingDefinition>, ReachingDefinition>,
}
impl InvariantCheck for ReachingCheck {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if relation == FlowUse::NAME {
            for row in FlowUse::decode(batch)? {
                self.uses.insert(&mut self.charge, row.id(), row.place)?;
            }
        } else if relation == FlowDefinition::NAME {
            for row in FlowDefinition::decode(batch)? {
                self.definitions
                    .insert(&mut self.charge, row.id(), row.place)?;
            }
        } else if relation == ReachingDefinition::NAME {
            for row in ReachingDefinition::decode(batch)? {
                self.targets.insert(&mut self.charge, row.id(), row)?;
            }
        } else if relation == FlowReachingObservation::NAME {
            for row in FlowReachingObservation::decode(batch)? {
                let place = self
                    .uses
                    .get(&row.use_)
                    .ok_or_else(|| invalid("reaching use missing"))?;
                if let ReachingDefinition::Bound { definition } = self
                    .targets
                    .get(&row.target)
                    .ok_or_else(|| invalid("reaching target missing"))?
                    && self.definitions.get(definition) != Some(place)
                {
                    return Err(invalid("reaching definition belongs to a different place"));
                }
            }
        } else {
            return Err(invalid("undeclared reaching validation input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        Ok(())
    }
}

#[derive(Default)]
struct FlowStructure {
    charge: StateCharge,
    occurrences: ChargedMap<Id<Occurrence>, Id<super::source::SourceArtifact>>,
    bounds: ChargedMap<Id<Occurrence>, (i64, i64)>,
    atoms: ChargedMap<Id<super::conditions::EvaluationAtom>, Id<Occurrence>>,
    scopes: ChargedMap<Id<LexicalScope>, Id<Occurrence>>,
    uses: ChargedMap<Id<FlowUse>, Id<Occurrence>>,
    definitions: ChargedMap<Id<FlowDefinition>, Id<Occurrence>>,
}
impl FlowStructure {
    fn same_source(&self, left: Id<Occurrence>, right: Id<Occurrence>) -> Result<(), ModelError> {
        let left = self
            .occurrences
            .get(&left)
            .ok_or_else(|| invalid("flow occurrence missing"))?;
        if self.occurrences.get(&right) != Some(left) {
            return Err(invalid("flow source structure crosses source artifacts"));
        }
        Ok(())
    }
    fn within(&self, inner: Id<Occurrence>, outer: Id<Occurrence>) -> Result<(), ModelError> {
        self.same_source(inner, outer)?;
        let (a, b) = self
            .bounds
            .get(&inner)
            .ok_or_else(|| invalid("flow bounds missing"))?;
        let (x, y) = self
            .bounds
            .get(&outer)
            .ok_or_else(|| invalid("flow bounds missing"))?;
        if a < x || b > y {
            return Err(invalid("flow leaf or operand leaves its test"));
        }
        Ok(())
    }
    fn scope(&self, id: Id<LexicalScope>) -> Result<Id<Occurrence>, ModelError> {
        self.scopes
            .get(&id)
            .copied()
            .ok_or_else(|| invalid("flow lexical scope missing"))
    }
    fn use_site(&self, id: Id<FlowUse>) -> Result<Id<Occurrence>, ModelError> {
        self.uses
            .get(&id)
            .copied()
            .ok_or_else(|| invalid("flow use missing"))
    }
}
impl InvariantCheck for FlowStructure {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if relation == Occurrence::NAME {
            for row in Occurrence::decode(batch)? {
                self.occurrences
                    .insert(&mut self.charge, row.id(), row.source)?;
                self.bounds
                    .insert(&mut self.charge, row.id(), (row.start, row.end))?;
            }
        } else if relation == LexicalScope::NAME {
            for row in LexicalScope::decode(batch)? {
                self.scopes.insert(&mut self.charge, row.id(), row.owner)?;
            }
        } else if relation == FlowUse::NAME {
            for row in FlowUse::decode(batch)? {
                self.uses
                    .insert(&mut self.charge, row.id(), row.occurrence)?;
            }
        } else if relation == FlowDefinition::NAME {
            for row in FlowDefinition::decode(batch)? {
                self.definitions
                    .insert(&mut self.charge, row.id(), row.occurrence)?;
            }
        } else if relation == FlowUseObservation::NAME {
            for row in FlowUseObservation::decode(batch)? {
                self.same_source(self.use_site(row.use_)?, self.scope(row.scope)?)?;
            }
        } else if relation == FlowDefinitionObservation::NAME {
            for row in FlowDefinitionObservation::decode(batch)? {
                let site = *self
                    .definitions
                    .get(&row.definition)
                    .ok_or_else(|| invalid("flow definition missing"))?;
                self.same_source(site, self.scope(row.scope)?)?;
                if let Some(value) = row.value {
                    self.same_source(site, value)?;
                }
            }
        } else if relation == FlowValueObservation::NAME {
            for row in FlowValueObservation::decode(batch)? {
                self.same_source(self.use_site(row.use_)?, row.sink)?;
            }
        } else if relation == FlowRegionObservation::NAME {
            for row in FlowRegionObservation::decode(batch)? {
                self.same_source(row.statement, self.scope(row.scope)?)?;
            }
        } else if relation == super::conditions::EvaluationAtom::NAME {
            for row in super::conditions::EvaluationAtom::decode(batch)? {
                self.atoms
                    .insert(&mut self.charge, row.id(), row.evaluation)?;
            }
        } else if relation == FlowTestObservation::NAME {
            for row in FlowTestObservation::decode(batch)? {
                self.same_source(row.test, self.scope(row.scope)?)?;
            }
        } else if relation == FlowTestLeafObservation::NAME {
            for row in FlowTestLeafObservation::decode(batch)? {
                let evaluation = *self
                    .atoms
                    .get(&row.atom)
                    .ok_or_else(|| invalid("flow leaf atom missing"))?;
                self.within(evaluation, row.test)?;
                if let Some(operand) = row.operand {
                    self.within(operand, evaluation)?;
                }
            }
        } else if relation == FlowAttributeLoadObservation::NAME {
            for row in FlowAttributeLoadObservation::decode(batch)? {
                if !self.occurrences.contains_key(&row.occurrence) {
                    return Err(invalid("attribute event missing"));
                }
            }
        } else {
            return Err(invalid("undeclared flow structure input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        Ok(())
    }
}

/// Raw tests retain their source event and scope; P3 owns the leaf-to-type relation.
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name = "flow_test_observations")]
#[assertion(support = FlowTestSupport, name = "flow_test_supports", family = FactFamily::Flow, subjects(test, scope))]
pub struct FlowTestObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub test: Id<Occurrence>,
    #[model(key)]
    pub scope: Id<LexicalScope>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name = "flow_test_leaf_observations")]
#[assertion(support = FlowTestLeafSupport, name = "flow_test_leaf_supports", family = FactFamily::Flow, subjects(test, atom, operand))]
pub struct FlowTestLeafObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub test: Id<Occurrence>,
    #[model(key)]
    pub atom: Id<super::conditions::EvaluationAtom>,
    #[model(key)]
    pub operand: Option<Id<Occurrence>>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name = "flow_attribute_load_observations", validate = validate_attribute)]
#[assertion(support = FlowAttributeLoadSupport, name = "flow_attribute_load_supports", family = FactFamily::Flow, subjects(occurrence))]
pub struct FlowAttributeLoadObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub occurrence: Id<Occurrence>,
    #[model(key)]
    pub name: String,
}
fn validate_attribute(row: &FlowAttributeLoadObservation) -> Result<(), ModelError> {
    if row.name.is_empty() {
        return Err(invalid("attribute load needs its native name"));
    }
    Ok(())
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum FlowCallOperandRole {
    Argument = 0,
    Callee = 1,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "flow_call_paths", invariants = flow_path_invariants)]
pub struct FlowCallPath {
    #[model(key)]
    pub steps: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "flow_call_steps", validate = validate_step)]
pub struct FlowCallStep {
    #[model(key)]
    pub path: Id<FlowCallPath>,
    #[model(key)]
    pub ordinal: i64,
    #[model(key)]
    pub call: Id<Occurrence>,
    #[model(key)]
    pub operand: Id<Occurrence>,
    #[model(key)]
    pub role: FlowCallOperandRole,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name = "flow_value_path_observations")]
#[assertion(support = FlowValuePathSupport, name = "flow_value_path_supports", family = FactFamily::Flow, subjects(value, path))]
pub struct FlowValuePathObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub value: Id<FlowValueObservation>,
    #[model(key)]
    pub path: Id<FlowCallPath>,
}
fn validate_step(row: &FlowCallStep) -> Result<(), ModelError> {
    if !(0..256).contains(&row.ordinal) {
        return Err(invalid("flow call step ordinal is bounded"));
    }
    Ok(())
}
fn path_digest(values: &[(Id<Occurrence>, Id<Occurrence>, FlowCallOperandRole)]) -> ContentHash {
    let mut key = KeySink::new("flow-call-path");
    Key::encode(&(values.len() as i64), &mut key);
    for (call, operand, role) in values {
        Key::encode(call, &mut key);
        Key::encode(operand, &mut key);
        Key::encode(role, &mut key);
    }
    key.finish()
}
impl FlowCallPath {
    pub fn new(
        values: &[(Id<Occurrence>, Id<Occurrence>, FlowCallOperandRole)],
    ) -> Result<(Self, Vec<FlowCallStep>), ModelError> {
        if values.is_empty() || values.len() > 256 {
            return Err(invalid("flow call path contains one to 256 steps"));
        }
        let row = Self {
            steps: path_digest(values),
        };
        let steps = values
            .iter()
            .enumerate()
            .map(|(i, (call, operand, role))| FlowCallStep {
                path: row.id(),
                ordinal: i as i64,
                call: *call,
                operand: *operand,
                role: *role,
            })
            .collect();
        Ok((row, steps))
    }
}
fn flow_path_invariants() -> Vec<Invariant> {
    vec![Invariant {
        name: "flow_call_path_structure",
        inputs: vec![
            ValidationInput::of::<Occurrence>(&["id"]),
            ValidationInput::of::<FlowCallPath>(&["id"]),
            ValidationInput::of::<super::calls::CallSyntax>(&["id"]),
            ValidationInput::of::<super::calls::CallArgument>(&["id"]),
            ValidationInput::of::<FlowCallStep>(&["path", "ordinal"]),
            ValidationInput::of::<FlowUse>(&["id"]),
            ValidationInput::of::<FlowValueObservation>(&["id"]),
            ValidationInput::of::<FlowValuePathObservation>(&["value"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(PathCheck {
                charge: StateCharge::new(budget, "flow_call_path_structure"),
                ..Default::default()
            })
        }),
    }]
}
#[derive(Default)]
struct PathCheck {
    charge: StateCharge,
    occurrences: ChargedMap<Id<Occurrence>, Occurrence>,
    paths: ChargedMap<Id<FlowCallPath>, ContentHash>,
    steps: CallStepIndex,
    callees: ChargedMap<Id<Occurrence>, Id<Occurrence>>,
    call_sites: ChargedMap<Id<super::calls::CallSyntax>, Id<Occurrence>>,
    arguments: super::charged::ChargedSet<(Id<Occurrence>, Id<Occurrence>)>,
    uses: ChargedMap<Id<FlowUse>, Id<Occurrence>>,
    ancestors: ChargedMap<(Id<super::source::SourceArtifact>, Vec<i32>), Id<Occurrence>>,
    values: ChargedMap<Id<FlowValueObservation>, FlowValueObservation>,
    linked: super::charged::ChargedSet<Id<FlowValueObservation>>,
}
impl PathCheck {
    fn within(&self, inner: Id<Occurrence>, outer: Id<Occurrence>) -> Result<(), ModelError> {
        let inner = self
            .occurrences
            .get(&inner)
            .ok_or_else(|| invalid("flow path occurrence missing"))?;
        let outer = self
            .occurrences
            .get(&outer)
            .ok_or_else(|| invalid("flow path occurrence missing"))?;
        if inner.source != outer.source
            || inner.start < outer.start
            || inner.end > outer.end
            || !inner.structural_path.starts_with(&outer.structural_path)
        {
            return Err(invalid("flow call path leaves its call or source"));
        }
        Ok(())
    }
}
impl InvariantCheck for PathCheck {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if relation == Occurrence::NAME {
            for row in Occurrence::decode(batch)? {
                self.occurrences.insert(&mut self.charge, row.id(), row)?;
            }
        } else if relation == FlowCallPath::NAME {
            for row in FlowCallPath::decode(batch)? {
                self.paths.insert(&mut self.charge, row.id(), row.steps)?;
            }
        } else if relation == super::calls::CallSyntax::NAME {
            for row in super::calls::CallSyntax::decode(batch)? {
                let occurrence = self
                    .occurrences
                    .get(&row.site)
                    .ok_or_else(|| invalid("flow call syntax occurrence missing"))?;
                self.ancestors.insert(
                    &mut self.charge,
                    (occurrence.source, occurrence.structural_path.clone()),
                    row.site,
                )?;
                self.callees
                    .insert(&mut self.charge, row.site, row.callee)?;
                self.call_sites
                    .insert(&mut self.charge, row.id(), row.site)?;
            }
        } else if relation == super::calls::CallArgument::NAME {
            for row in super::calls::CallArgument::decode(batch)? {
                self.arguments.insert(
                    &mut self.charge,
                    (
                        *self
                            .call_sites
                            .get(&row.call)
                            .ok_or_else(|| invalid("flow argument call syntax missing"))?,
                        row.value,
                    ),
                )?;
            }
        } else if relation == FlowCallStep::NAME {
            for row in FlowCallStep::decode(batch)? {
                if self
                    .occurrences
                    .get(&row.call)
                    .is_none_or(|o| o.syntax_kind != super::source::SyntaxKind::ExprCall)
                {
                    return Err(invalid("flow step names a call occurrence"));
                }
                self.within(row.operand, row.call)?;
                let matches = match row.role {
                    FlowCallOperandRole::Callee => {
                        self.callees.get(&row.call) == Some(&row.operand)
                    }
                    FlowCallOperandRole::Argument => {
                        self.arguments.contains(&(row.call, row.operand))
                    }
                };
                if !matches {
                    return Err(invalid(
                        "flow step operand disagrees with typed call syntax",
                    ));
                }
                let mut entries = self.steps.get(&row.path).cloned().unwrap_or_default();
                if row.ordinal != entries.len() as i64 {
                    return Err(invalid("flow steps are contiguous and ordered"));
                }
                if entries.iter().any(|(call, _, _)| *call == row.call) {
                    return Err(invalid("flow call path repeats a crossing"));
                }
                if let Some((_, operand, _)) = entries.last() {
                    self.within(row.call, *operand)?;
                }
                entries.push((row.call, row.operand, row.role));
                self.steps.insert(&mut self.charge, row.path, entries)?;
            }
        } else if relation == FlowUse::NAME {
            for row in FlowUse::decode(batch)? {
                self.uses
                    .insert(&mut self.charge, row.id(), row.occurrence)?;
            }
        } else if relation == FlowValueObservation::NAME {
            for row in FlowValueObservation::decode(batch)? {
                self.values.insert(&mut self.charge, row.id(), row)?;
            }
        } else if relation == FlowValuePathObservation::NAME {
            for row in FlowValuePathObservation::decode(batch)? {
                let value = self
                    .values
                    .get(&row.value)
                    .ok_or_else(|| invalid("flow path value missing"))?;
                if value.qualification != row.qualification || !value.through_call {
                    return Err(invalid(
                        "flow path qualification or through-call claim disagrees",
                    ));
                }
                if !self.linked.insert(&mut self.charge, row.value)? {
                    return Err(invalid("flow value has one call path"));
                }
                let steps = self
                    .steps
                    .get(&row.path)
                    .ok_or_else(|| invalid("flow path has no steps"))?;
                self.within(steps[0].0, value.sink)?;
                let origin = *self
                    .uses
                    .get(&value.use_)
                    .ok_or_else(|| invalid("flow path originating use missing"))?;
                self.within(
                    origin,
                    steps.last().ok_or_else(|| invalid("flow path empty"))?.1,
                )?;
                let occurrence = self
                    .occurrences
                    .get(&origin)
                    .ok_or_else(|| invalid("flow use occurrence missing"))?;
                let mut crossing = 0;
                // Syntax paths enumerate every enclosing call from outermost to innermost. Checking
                // prefixes is bounded by syntax depth rather than scanning the whole source for each use.
                for length in 0..occurrence.structural_path.len() {
                    if let Some(call) = self.ancestors.get(&(
                        occurrence.source,
                        occurrence.structural_path[..length].to_vec(),
                    )) && self.within(*call, value.sink).is_ok()
                    {
                        if steps.get(crossing).is_none_or(|step| step.0 != *call) {
                            return Err(invalid("flow call path omits or misorders a crossing"));
                        }
                        crossing += 1;
                    }
                }
                if crossing != steps.len() {
                    return Err(invalid(
                        "flow call path adds a crossing outside its originating use",
                    ));
                }
            }
        } else {
            return Err(invalid("undeclared flow path input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        for (id, expected) in self.paths.iter() {
            let steps = self
                .steps
                .get(id)
                .ok_or_else(|| invalid("flow path has no steps"))?;
            if steps.is_empty() || *expected != path_digest(steps) {
                return Err(invalid("flow steps differ from path identity"));
            }
        }
        for (id, value) in self.values.iter() {
            if value.through_call != self.linked.contains(id) {
                return Err(invalid("through-call flow retains exactly its path"));
            }
        }
        Ok(())
    }
}

type CallStepIndex =
    ChargedMap<Id<FlowCallPath>, Vec<(Id<Occurrence>, Id<Occurrence>, FlowCallOperandRole)>>;
