//! Charged test presentation of the same synchronous events consumed by production.
#![allow(
    dead_code,
    reason = "Each characterization target consumes a different raw family"
)]
use cpg_flow::{
    AttributeLoad, CaptureSnapshot, Def, Input, ModuleEvent, Reach, Region, RuntimeContext,
    SkipCounts, Span, Test, TestLeaf, Use, UseCandidate, ValueSource, VisitError, native,
};
use lctx_model::domain::{
    ContentHash, ModelError, charged::StateCharge, resources::ResourceBudget,
};
use std::{
    collections::BTreeMap,
    ops::{Deref, DerefMut},
};

/// Historical raw shape retained only by tests; conditions retain their admitted backing.
#[derive(Debug)]
pub struct ModuleFlow {
    pub path: String,
    pub uses: Vec<Use>,
    pub defs: Vec<Def>,
    pub reaching: Vec<Reach>,
    pub candidates: Vec<UseCandidate>,
    pub unavailable_uses: Vec<Span>,
    pub captures: Vec<CaptureSnapshot>,
    pub values: Vec<ValueSource>,
    pub regions: Vec<Region>,
    pub tests: Vec<Test>,
    pub test_leaves: Vec<TestLeaf>,
    pub attribute_loads: Vec<AttributeLoad>,
    pub places: BTreeMap<Span, native::Place>,
    pub nodes: Vec<native::Node>,
    pub renamed: u32,
    pub original_content: Option<ContentHash>,
    pub view_content: Option<ContentHash>,
    pub view_byte_len: Option<usize>,
    pub syntax_errors: usize,
    pub error: Option<String>,
    pub skips: SkipCounts,
    charge: StateCharge,
}
impl ModuleFlow {
    fn new(path: &str, budget: &ResourceBudget) -> Result<Self, ModelError> {
        let mut charge = StateCharge::new(budget, "flow-test-module");
        charge.grow(path.len())?;
        Ok(Self {
            path: path.to_owned(),
            uses: vec![],
            defs: vec![],
            reaching: vec![],
            candidates: vec![],
            unavailable_uses: vec![],
            captures: vec![],
            values: vec![],
            regions: vec![],
            tests: vec![],
            test_leaves: vec![],
            attribute_loads: vec![],
            places: BTreeMap::new(),
            nodes: vec![],
            renamed: 0,
            original_content: None,
            view_content: None,
            view_byte_len: None,
            syntax_errors: 0,
            error: None,
            skips: SkipCounts::default(),
            charge,
        })
    }
    fn event(&mut self, event: ModuleEvent<'_>, budget: &ResourceBudget) -> Result<(), ModelError> {
        match event {
            ModuleEvent::Prelude(prelude) => {
                self.original_content = Some(prelude.original_content);
                self.view_content = Some(prelude.view_content);
                self.view_byte_len = Some(prelude.view_byte_len);
                self.renamed = prelude.renamed;
                self.syntax_errors = prelude.syntax_errors;
                let mut scratch = StateCharge::new(budget, "flow-test-node-order");
                let mut nodes = Vec::new();
                for (span, entry) in prelude.raw_nodes() {
                    slots(&mut nodes, &mut scratch, 1)?;
                    nodes.push((
                        entry.ordinal,
                        native::Node {
                            span,
                            kind: entry.kind,
                            role: entry.role,
                        },
                    ));
                }
                nodes.sort_unstable_by_key(|(ordinal, _)| *ordinal);
                slots(&mut self.nodes, &mut self.charge, nodes.len())?;
                for (ordinal, node) in nodes {
                    assert_eq!(ordinal, self.nodes.len(), "raw node encounter ordinal");
                    self.nodes.push(node);
                }
                for (span, place) in prelude.places() {
                    self.charge
                        .grow(size_of::<(Span, native::Place)>() + 32 + place.heap_bytes())?;
                    assert!(self.places.insert(span, place.clone()).is_none());
                }
            }
            ModuleEvent::Definition { ordinal, value } => {
                assert_eq!(ordinal as usize, self.defs.len());
                self.charge.grow(value.place.capacity())?;
                slots(&mut self.defs, &mut self.charge, 1)?;
                self.defs.push(value.clone());
            }
            ModuleEvent::UnavailableUse(span) => {
                slots(&mut self.unavailable_uses, &mut self.charge, 1)?;
                self.unavailable_uses.push(span);
            }
            ModuleEvent::UseGroup { ordinal, value } => {
                assert_eq!(ordinal as usize, self.uses.len());
                self.charge.grow(value.use_.place.capacity())?;
                slots(&mut self.uses, &mut self.charge, 1)?;
                self.uses.push(value.use_.clone());
                let base = self.reaching.len();
                slots(&mut self.reaching, &mut self.charge, value.reaching.len())?;
                self.reaching.extend(value.reaching.iter().cloned());
                slots(
                    &mut self.candidates,
                    &mut self.charge,
                    value.candidates.len(),
                )?;
                for candidate in &value.candidates {
                    assert_eq!(candidate.use_ix, ordinal);
                    assert!(candidate.reaching.end <= value.reaching.len());
                    let mut row = candidate.clone();
                    row.reaching = base
                        .checked_add(row.reaching.start)
                        .ok_or(ModelError::Schema("test reaching offset overflow"))?
                        ..base
                            .checked_add(row.reaching.end)
                            .ok_or(ModelError::Schema("test reaching offset overflow"))?;
                    self.candidates.push(row);
                }
                if let Some(capture) = &value.capture {
                    self.charge.grow(
                        capture
                            .candidates
                            .capacity()
                            .checked_mul(size_of::<cpg_flow::SnapshotCandidate>())
                            .ok_or(ModelError::Schema("test capture allocation overflow"))?,
                    )?;
                    slots(&mut self.captures, &mut self.charge, 1)?;
                    self.captures.push(capture.clone());
                }
            }
            ModuleEvent::Value(value) => {
                self.charge.grow(std::mem::size_of_val(value.call_path))?;
                slots(&mut self.values, &mut self.charge, 1)?;
                self.values.push(ValueSource {
                    sink: value.sink,
                    span: value.span,
                    use_ix: value.use_ix,
                    identity: value.identity,
                    through_call: value.through_call,
                    call_path: value.call_path.to_vec(),
                    condition: value.condition.clone(),
                });
            }
            ModuleEvent::Region(value) => {
                slots(&mut self.regions, &mut self.charge, 1)?;
                self.regions.push(value.clone());
            }
            ModuleEvent::Attribute(value) => {
                self.charge.grow(value.name.len())?;
                slots(&mut self.attribute_loads, &mut self.charge, 1)?;
                self.attribute_loads.push(AttributeLoad {
                    span: value.span,
                    name: value.name.to_owned(),
                });
            }
            ModuleEvent::TraversalComplete => {}
            ModuleEvent::Test(value) => {
                slots(&mut self.tests, &mut self.charge, 1)?;
                self.tests.push(value.clone());
            }
            ModuleEvent::TestLeaf(value) => {
                self.charge.grow(value.atom.heap_bytes())?;
                slots(&mut self.test_leaves, &mut self.charge, 1)?;
                self.test_leaves.push(TestLeaf {
                    scope: value.scope,
                    test_span: value.test_span,
                    condition: value.condition.clone(),
                    atom: value.atom.clone(),
                    leaf_span: value.leaf_span,
                    operand_span: value.operand_span,
                });
            }
            ModuleEvent::End(skips) => self.skips = skips,
            ModuleEvent::ModuleFailure(failure) => {
                self.charge.grow(failure.detail.capacity())?;
                self.error = Some(failure.detail.clone());
            }
        }
        Ok(())
    }
}

/// Collection backing stays admitted even when individual modules are popped and retained.
#[derive(Debug)]
pub struct CollectedModules {
    modules: Vec<ModuleFlow>,
    charge: StateCharge,
}
impl Deref for CollectedModules {
    type Target = Vec<ModuleFlow>;
    fn deref(&self) -> &Self::Target {
        &self.modules
    }
}
impl DerefMut for CollectedModules {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.modules
    }
}

fn slots<T>(
    values: &mut Vec<T>,
    charge: &mut StateCharge,
    additional: usize,
) -> Result<(), ModelError> {
    let wanted = values
        .len()
        .checked_add(additional)
        .ok_or(ModelError::Schema("test flow capacity overflow"))?;
    if wanted > values.capacity() {
        let capacity = wanted.max(values.capacity().saturating_mul(2)).max(4);
        charge.grow(
            (capacity - values.capacity())
                .checked_mul(size_of::<T>())
                .ok_or(ModelError::Schema("test flow allocation overflow"))?,
        )?;
        values.reserve_exact(capacity - values.len());
    }
    Ok(())
}

pub fn index_with_budget(
    inputs: &[Input],
    context: &RuntimeContext,
    budget: &ResourceBudget,
) -> Result<CollectedModules, VisitError<ModelError>> {
    let mut collected = CollectedModules {
        modules: vec![],
        charge: StateCharge::new(budget, "flow-test-collection"),
    };
    slots(&mut collected.modules, &mut collected.charge, inputs.len())
        .map_err(VisitError::Consumer)?;
    for input in inputs {
        collected
            .modules
            .push(ModuleFlow::new(&input.path, budget).map_err(VisitError::Consumer)?);
    }
    cpg_flow::visit_modules(
        inputs,
        context,
        budget,
        &|| Ok(()),
        |_| (),
        |module, event| collected.modules[module].event(event, budget),
    )?;
    Ok(collected)
}

pub fn index(inputs: &[Input], context: &RuntimeContext) -> CollectedModules {
    let budget = ResourceBudget::fixed(256 << 20).unwrap();
    index_with_budget(inputs, context, &budget).unwrap()
}
