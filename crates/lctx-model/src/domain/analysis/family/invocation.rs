use super::*;
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = owner_table!("analysis_invocations"), invariant_refs = invocation_invariants_refs, publication_refs=source_publication_checks_refs)]
pub struct AnalysisInvocation {
    #[model(key)]
    pub input: Id<InputRevision>,
    #[model(key)]
    pub context: Id<AnalysisContext>,
    #[model(key)]
    pub definition: Id<AnalysisDefinition>,
    #[model(key)]
    pub subject: Option<Id<EntityRef>>,
    /// Exact typed parent membership is validated; its digest participates in invocation identity.
    #[model(key)]
    pub inputs: ContentHash,
    #[model(key)] pub sources:ContentHash,
    #[model(key)] pub projections:ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = owner_table!("analysis_inputs"), rule = "analysis_input", conclusion = invocation)]
pub struct AnalysisInput {
    #[model(key)]
    pub invocation: Id<AnalysisInvocation>,
    #[model(key, premise)]
    pub parent: Id<InvocationSource>,
}
impl AnalysisInvocation {
    pub fn new(
        input: Id<InputRevision>,
        context: Id<AnalysisContext>,
        definition: Id<AnalysisDefinition>,
        subject: Option<Id<EntityRef>>,
        parents: impl IntoIterator<Item = Id<InvocationSource>>,
    ) -> (Self, Vec<AnalysisInput>) {
        let parents = parents
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>();
        let row = Self {
            input,
            context,
            definition,
            subject,
            inputs: parent_digest(&parents),
            sources:crate::domain::analysis::sources::empty_digest(),
            projections:projection_digest(&std::collections::BTreeSet::new()),
        };
        let inputs = parents
            .into_iter()
            .map(|parent| AnalysisInput {
                invocation: row.id(),
                parent,
            })
            .collect();
        (row, inputs)
    }
}
fn parent_digest(parents: &std::collections::BTreeSet<Id<InvocationSource>>) -> ContentHash {
    let mut sink = KeySink::new("analysis-invocation-inputs");
    for parent in parents {
        parent.encode(&mut sink);
    }
    sink.finish()
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = owner_table!("analysis_outcomes"), validate = validate_outcome)]
pub struct AnalysisOutcome {
    #[model(key)]
    pub invocation: Id<AnalysisInvocation>,
    pub status: AnalysisStatus,
    pub reason: Option<ObligationKind>,
}
fn validate_outcome(row: &AnalysisOutcome) -> Result<(), ModelError> {
    if match row.status {
        AnalysisStatus::Completed => row.reason.is_some(),
        AnalysisStatus::Partial | AnalysisStatus::Unavailable => {
            row.reason.is_none() || row.reason == Some(ObligationKind::NotRequested)
        }
        AnalysisStatus::NotRequested => row.reason != Some(ObligationKind::NotRequested),
    } {
        return Err(invalid("analysis outcome disagrees with boundary"));
    }
    Ok(())
}
/// Measurement payloads never distinguish the semantic invocation or outcome.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = owner_table!("analysis_diagnostics"), validate = validate_diagnostic)]
pub struct AnalysisDiagnostic {
    #[model(key)]
    pub invocation: Id<AnalysisInvocation>,
    pub elapsed_micros: Option<i64>,
    pub iterations: Option<i64>,
    pub examined_members: Option<i64>,
    pub residual: Option<FiniteF64>,
    pub converged: Option<bool>,
}
fn validate_diagnostic(row: &AnalysisDiagnostic) -> Result<(), ModelError> {
    if [row.elapsed_micros, row.iterations, row.examined_members]
        .into_iter()
        .flatten()
        .any(|v| v < 0)
    {
        return Err(invalid("negative analysis diagnostic"));
    }
    Ok(())
}
pub(crate) fn invocation_invariants() -> Vec<Invariant> {
    vec![Invariant {
        revision: 1,
        name: owner_table!("analysis_invocation_inputs"),
        inputs: invocation_inputs(),
        create: std::sync::Arc::new(|budget| {
            Box::new(InvocationCheck {
                charge: charged::StateCharge::new(budget, "analysis_invocation_inputs"),
                rows: Default::default(),
                edges: Default::default(),
                snapshots:Default::default(),projections:Default::default(),
                sources: Default::default(),frames:Default::default(),
            })
        }),
    }]
}
fn invocation_inputs()->Vec<ValidationInput> { let mut inputs=vec![ValidationInput::of::<SourceReceipt>(&["id"]),ValidationInput::of::<ProjectionInput>(&["id"]),ValidationInput::of::<AnalysisInvocation>(&["id"]),ValidationInput::of::<AnalysisInput>(&["id"]),ValidationInput::of::<InvocationSource>(&["id"])]; predecessor_invocation_inputs(&mut inputs); inputs }
struct InvocationCheck {
    snapshots:charged::ChargedMap<Id<AnalysisInvocation>,std::collections::BTreeMap<String,crate::domain::analysis::sources::SourceSnapshot>>,
    projections:charged::ChargedMap<Id<AnalysisInvocation>,std::collections::BTreeSet<Id<ProjectionDefinition>>>,
    sources:charged::ChargedMap<Id<InvocationSource>,InvocationSource>,
    frames:charged::ChargedMap<derivation::RowRef,(Id<input::InputRevision>,Id<attribution::AnalysisContext>)>,
    charge: charged::StateCharge,
    rows: charged::ChargedMap<Id<AnalysisInvocation>, AnalysisInvocation>,
    edges: charged::ChargedMap<
        Id<AnalysisInvocation>,
        std::collections::BTreeSet<Id<InvocationSource>>,
    >,
}
impl InvariantCheck for InvocationCheck {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if relation == AnalysisInvocation::NAME {
            for row in AnalysisInvocation::decode(batch)? {
                if self.rows.insert(&mut self.charge, row.id(), row)?.is_some() {
                    return Err(ModelError::Conflict(AnalysisInvocation::NAME));
                }
            }
        } else if relation == AnalysisInput::NAME {
            for row in AnalysisInput::decode(batch)? {
                if !self
                    .edges
                    .update(&mut self.charge, row.invocation, |edges| {
                        edges.insert(row.parent)
                    })?
                {
                    return Err(invalid("duplicate analysis parent"));
                }
            }
        } else if relation==SourceReceipt::NAME {for row in SourceReceipt::decode(batch)? {let snapshot=row.snapshot();let mut duplicate=false;self.snapshots.update(&mut self.charge,row.invocation,|sources|{duplicate=sources.insert(snapshot.relation.clone(),snapshot).is_some();})?;if duplicate {return Err(invalid("duplicate invocation source receipt"));}}
        } else if relation==ProjectionInput::NAME {for row in ProjectionInput::decode(batch)? {if !self.projections.update(&mut self.charge,row.invocation,|p|p.insert(row.projection))? {return Err(invalid("duplicate invocation projection"));}}}
        else if relation==InvocationSource::NAME { for row in InvocationSource::decode(batch)? { self.sources.insert(&mut self.charge,row.id(),row)?; }
        } else if !visit_predecessor_invocation(relation,batch,&mut self.frames,&mut self.charge)? {
            return Err(invalid("undeclared analysis invocation input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        let empty = std::collections::BTreeSet::new();
        for (id,edges) in self.edges.iter() { let invocation=self.rows.get(id).ok_or_else(||invalid("analysis input invocation absent"))?; for parent in edges {let source=self.sources.get(parent).ok_or_else(||invalid("analysis parent source absent"))?; let reference=source.reference(); let frame=if reference.relation()==AnalysisInvocation::NAME {let row=self.rows.get(&source.current().ok_or_else(||invalid("parent is not current"))?).ok_or_else(||invalid("analysis parent absent"))?; (row.input,row.context)} else {*self.frames.get(&reference).ok_or_else(||invalid("predecessor invocation absent"))?}; if reference==derivation::RowRef::of(*id) || frame!=(invocation.input,invocation.context) {return Err(invalid("analysis parent crosses input/context or self"));} } }
        let no_sources=std::collections::BTreeMap::new();
        let no_projections=std::collections::BTreeSet::new();
        for (id, row) in self.rows.iter() {
            if crate::domain::analysis::sources::digest(self.snapshots.get(id).unwrap_or(&no_sources))!=row.sources || projection_digest(self.projections.get(id).unwrap_or(&no_projections))!=row.projections {return Err(invalid("invocation source or projection membership differs"));}
            if parent_digest(self.edges.get(id).unwrap_or(&empty)) != row.inputs {
                return Err(invalid(
                    "analysis input membership differs from invocation identity",
                ));
            }
        }
        for id in self.snapshots.keys() {if !self.rows.contains_key(id) {return Err(invalid("orphan invocation source receipt"));}}
        for id in self.projections.keys() {if !self.rows.contains_key(id) {return Err(invalid("orphan invocation projection"));}}
        // Global declared derivation validation owns cross-relation cycle checking.
        Ok(())
    }
}

include!("source_receipts.rs");

pub(crate) fn invocation_invariants_refs() -> Vec<&'static str> { vec![owner_table!("analysis_invocation_inputs")] }
