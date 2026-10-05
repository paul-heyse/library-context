//! S0 conclusions preserve exact earlier observation proofs; source paths are not executions.
use super::{documentary, frames};
use crate::domain::{
    analysis::{
        self, findings,
        policy::{FindingKind, MemberRole, SupportRole},
        support::{DerivedEvidence, QualificationOperation},
        synthesis as owner,
    },
    conditions::Diagram,
    normalized::Rows,
    resources::ResourceBudget,
    stages::is_vocabulary,
    *,
};
#[macro_export]
macro_rules! synthesis_observation_inputs{($m:ident)=>{$m!{
 structural_conclusions:$crate::domain::structural::Conclusion,structural_sources:$crate::domain::structural::ConclusionSource,
 analytic_conclusions:$crate::domain::analytics::Conclusion,analytic_sources:$crate::domain::analytics::ConclusionSource,
 qualifications:$crate::domain::assertion::AssertionQualification,conditions:$crate::domain::conditions::Condition,nodes:$crate::domain::conditions::ConditionNode,
}};}
macro_rules! data{($($f:ident:$t:ty,)*)=>{pub struct Data{$(pub $f:Rows<$t>,)*}impl Data{pub fn new(b:&ResourceBudget)->Self{Self{$($f:Rows::new(b),)*}}pub fn visit(&mut self,n:&str,b:&arrow_array::RecordBatch)->Result<bool,ModelError>{$(if n==<$t>::NAME{self.$f.decode(b)?;return Ok(true);})*Ok(false)}pub fn inputs()->Vec<ValidationInput>{vec![$(ValidationInput::of::<$t>(&["id"]),)*]}}};}
crate::synthesis_observation_inputs!(data);
#[macro_export]
macro_rules! synthesis_observation_outputs{($m:ident)=>{$m!{
 subjects:$crate::domain::analysis::synthesis::ObligationSubject,sources:$crate::domain::analysis::synthesis::SupportSource,propositions:$crate::domain::analysis::synthesis::AnalysisProposition,derivations:$crate::domain::analysis::synthesis::AnalysisDerivation,premises:$crate::domain::analysis::synthesis::AnalysisDerivationPremise,
 findings:$crate::domain::analysis::findings::Finding,members:$crate::domain::analysis::findings::FindingMember,supports:$crate::domain::analysis::findings::FindingSupport,
 qualifications:$crate::domain::assertion::AssertionQualification,conditions:$crate::domain::conditions::Condition,nodes:$crate::domain::conditions::ConditionNode,
}};}
macro_rules! output{($($f:ident:$t:ty,)*)=>{pub struct Output{$(pub $f:Rows<$t>,)*}impl Output{pub fn new(b:&ResourceBudget)->Self{Self{$($f:Rows::new(b),)*}}pub fn visit(&mut self,n:&str,b:&arrow_array::RecordBatch)->Result<bool,ModelError>{$(if n==<$t>::NAME{self.$f.decode(b)?;return Ok(true);})*Ok(false)}pub fn inputs()->Vec<ValidationInput>{vec![$(ValidationInput::of::<$t>(&["id"]),)*]}pub fn matches(&self,o:&Self)->Result<(),ModelError>{$(if !self.$f.same(&o.$f){return Err(invalid(concat!("S0 observation closure differs: ",stringify!($f))));})*Ok(())}}};}
crate::synthesis_observation_outputs!(output);
fn invalid(s: &str) -> ModelError {
    ModelError::Invalid(s.into())
}
fn need<R: Record>(r: &Rows<R>, id: Id<R>) -> Result<&R, ModelError> {
    r.get(id)
        .ok_or_else(|| invalid("S0 observation premise absent"))
}
struct FindingSubject {
    source: owner::SupportSource,
    entity: Id<normalized::entities::EntityRef>,
    kind: FindingKind,
}

fn emit<R: DerivedEvidence>(
    d: &Data,
    out: &mut Output,
    invocation: &owner::Invocation,
    row: &R,
    finding_subject: FindingSubject,
    coverage: &owner::AnalysisCoverage,
    b: &ResourceBudget,
) -> Result<(), ModelError> {
    let FindingSubject {
        source,
        entity,
        kind,
    } = finding_subject;
    let q = need(&d.qualifications, row.source_facts().qualification)?;
    if q.context != invocation.context
        || q.scope
            != (source::CoverageScope::Input {
                input: invocation.input,
            })
            .id()
    {
        return Err(invalid(
            "static S0 observation changes exact release/context",
        ));
    }
    let condition = need(&d.conditions, q.condition)?;
    let _nodes = b.reserve(
        "synthesis-observation-condition",
        d.nodes.len() * size_of::<conditions::ConditionNode>(),
    )?;
    let nodes = d.nodes.iter().cloned().collect::<Vec<_>>();
    let diagram = Diagram::from_records(condition, &nodes)?;
    let subject = owner::ObligationSubject::Entity { entity };
    out.subjects.insert(subject.clone())?;
    out.sources.insert(source.clone())?;
    let evidence = owner::support::EvidencePremise::derived(&source, row, q, &diagram)?;
    let (derivation, proposition, premises, qualified) = owner::AnalysisDerivation::emit(
        invocation,
        &super::build::definition().1,
        subject.id(),
        analysis::AnalysisChannel::Catalog,
        calls::CallPhase::Definition,
        QualificationOperation::Conjunction,
        &[evidence],
        b,
    )?;
    out.propositions.insert(proposition)?;
    out.derivations.insert(derivation.clone())?;
    for row in premises {
        out.premises.insert(row)?;
    }
    let own = owner::SupportSource::AnalysisDerivation {
        derivation: derivation.id(),
    };
    out.sources.insert(own.clone())?;
    let evidence = owner::support::EvidencePremise::derived(
        &own,
        &derivation,
        &qualified.qualification,
        &qualified.condition,
    )?;
    let (finding, members, supports, result) = findings::emit(
        invocation.id(),
        kind,
        subject.id(),
        &[(MemberRole::AccessPath, subject.id())],
        coverage,
        &[findings::FindingEvidence::new(
            SupportRole::Support,
            evidence,
        )],
        b,
    )?;
    out.findings.insert(finding)?;
    for row in members {
        out.members.insert(row)?;
    }
    for row in supports {
        out.supports.insert(row)?;
    }
    out.qualifications.insert(result.qualification.clone())?;
    let (condition, nodes) = result.condition.records();
    out.conditions.insert(condition)?;
    for row in nodes {
        out.nodes.insert(row)?;
    }
    Ok(())
}
/// Completeness is reconstructed from all earlier sealed observations in each fixed S0 frame.
pub fn build(
    d: &Data,
    frames: &Rows<frames::Frame>,
    invocations: &Rows<owner::Invocation>,
    coverages: &Rows<owner::AnalysisCoverage>,
    b: &ResourceBudget,
) -> Result<Output, ModelError> {
    let mut out = Output::new(b);
    for frame in frames.iter() {
        let inv = need(invocations, frame.invocation)?;
        let scope = (source::CoverageScope::Input { input: inv.input }).id();
        let mut coverage = coverages.iter().filter(|r| {
            r.invocation == inv.id()
                && r.capability == analysis::AnalysisCapability::Synthesis
                && r.context == inv.context
                && r.scope == scope
        });
        let coverage = coverage
            .next()
            .ok_or_else(|| invalid("S0 static observation requires its admitted Input coverage"))?;
        for row in d
            .structural_conclusions
            .iter()
            .filter(|r| r.frame == frame.structural)
        {
            need(&d.structural_sources, row.source)?;
            emit(
                d,
                &mut out,
                inv,
                row,
                FindingSubject {
                    source: owner::SupportSource::StructuralObservation { witness: row.id() },
                    entity: row.subject,
                    kind: row.kind,
                },
                coverage,
                b,
            )?;
        }
        for row in d
            .analytic_conclusions
            .iter()
            .filter(|r| r.frame == frame.analytic)
        {
            need(&d.analytic_sources, row.source)?;
            emit(
                d,
                &mut out,
                inv,
                row,
                FindingSubject {
                    source: owner::SupportSource::AnalyticObservation { witness: row.id() },
                    entity: row.subject,
                    kind: row.kind,
                },
                coverage,
                b,
            )?;
        }
    }
    Ok(out)
}
pub fn build_all(
    d: &Data,
    summary: &super::summary::Data,
    docs: &documentary::Data,
    frames: &Rows<frames::Frame>,
    invocations: &Rows<owner::Invocation>,
    coverages: &Rows<owner::AnalysisCoverage>,
    b: &ResourceBudget,
) -> Result<Output, ModelError> {
    let mut out = build(d, frames, invocations, coverages, b)?;
    let (facets, _) = super::summary::build(summary, docs, frames, invocations, b)?;
    super::summary::extend_observations(
        summary,
        d,
        &facets,
        frames,
        invocations,
        coverages,
        &mut out,
        b,
    )?;
    Ok(out)
}
/// Documentary candidates are deliberately absent here: authored prose uses its own assertion
/// floor and does not become an observed behavioral Finding.
pub fn invariants() -> Vec<Invariant> {
    let mut inputs = documentary::replay_inputs(Data::inputs(), stages::PublicationBoundary::Analytic);
    inputs.extend(super::summary::Data::inputs());
    inputs.extend(documentary::replay_inputs(documentary::Data::validation_inputs(), stages::PublicationBoundary::Facts));
    inputs.extend(Output::inputs());
    inputs.extend([
        ValidationInput::of::<frames::Frame>(&["id"]),
        ValidationInput::of::<owner::Invocation>(&["id"]),
        ValidationInput::of::<owner::AnalysisCoverage>(&["id"]),
    ]);
    inputs.sort_by_key(|r| (r.name(), r.prefix()));
    inputs.dedup_by_key(|r| (r.name(), r.prefix()));
    vec![Invariant {
        revision: 2,
        name: "synthesis_observation_universe",
        inputs,
        create: std::sync::Arc::new(|b| {
            Box::new(Check {
                data: Data::new(b),
                summary: super::summary::Data::new(b),
                docs: documentary::Data::new(b),
                out: Output::new(b),
                frames: Rows::new(b),
                invocations: Rows::new(b),
                coverage: Rows::new(b),
                budget: b.clone(),
            })
        }),
    }]
}
struct Check {
    data: Data,
    summary: super::summary::Data,
    docs: documentary::Data,
    out: Output,
    frames: Rows<frames::Frame>,
    invocations: Rows<owner::Invocation>,
    coverage: Rows<owner::AnalysisCoverage>,
    budget: ResourceBudget,
}
impl InvariantCheck for Check {
    fn visit(&mut self, _name: &str, _batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        Err(ModelError::Invalid("S0 replay requires an explicit completed-input selector".into()))
    }
    fn visit_input(&mut self, input: &ValidationInput, b: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        documentary::replay_selector(input)?;
        let n = input.name();
        let d = documentary::replay_visit(input, Some(stages::PublicationBoundary::Analytic), |name| self.data.visit(name, b))?;
        let summary = documentary::replay_visit(input, None, |name| self.summary.visit(name, b))?;
        let docs = documentary::replay_visit(input, Some(stages::PublicationBoundary::Facts), |name| self.docs.visit(name, b))?;
        let o = documentary::replay_visit(input, None, |name| self.out.visit(name, b))?;
        macro_rules! row {
            ($t:ty,$f:ident) => {
                if n == <$t>::NAME {
                    self.$f.decode(b)?;
                    return Ok(());
                }
            };
        }
        row!(frames::Frame, frames);
        row!(owner::Invocation, invocations);
        row!(owner::AnalysisCoverage, coverage);
        if !d && !o && !summary && !docs {
            return Err(invalid("undeclared S0 observation input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        let expected = build_all(
            &self.data,
            &self.summary,
            &self.docs,
            &self.frames,
            &self.invocations,
            &self.coverage,
            &self.budget,
        )?;
        macro_rules! compare{($($f:ident:$t:ty,)*)=>{$(if !is_vocabulary(<$t>::NAME)&&!self.out.$f.same(&expected.$f){return Err(invalid(concat!("S0 observation closure differs: ",stringify!($f))));}for row in expected.$f.iter(){if self.out.$f.get(row.id())!=Some(row){return Err(invalid("S0 observation output membership absent"));}})*};}
        crate::synthesis_observation_outputs!(compare);
        Ok(())
    }
}

pub(crate) fn invariants_refs() -> Vec<&'static str> {
    vec!["synthesis_observation_universe"]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::assertion::AssertionQualification;
    fn binary<R: Record>(id: Id<R>) -> arrow_array::ArrayRef {
        std::sync::Arc::new(
            arrow_array::FixedSizeBinaryArray::try_from_iter([id.bytes()].into_iter()).unwrap(),
        )
    }
    fn conclusion(
        frame: Id<structural::StructuralFrame>,
        source: Id<structural::ConclusionSource>,
        invocation: Id<analysis::structural::Invocation>,
        entity: Id<normalized::entities::EntityRef>,
        q: Id<AssertionQualification>,
    ) -> structural::Conclusion {
        let key = structural::conclusions::ConclusionKey { frame, source };
        let columns = vec![
            binary(Id::<structural::Conclusion>::of(&key)),
            binary(frame),
            binary(source),
            binary(invocation),
            binary(entity),
            std::sync::Arc::new(arrow_array::Int16Array::from(vec![
                FindingKind::PublicAlias.code(),
            ])) as arrow_array::ArrayRef,
            binary(q),
        ];
        structural::Conclusion::decode(
            &arrow_array::RecordBatch::try_new(structural::Conclusion::schema(), columns).unwrap(),
        )
        .unwrap()
        .remove(0)
    }
    /// Pure qualification control assumes the earlier A0 source-observation owner admitted this
    /// nominal row. Actual source extraction and completion remain native/store controls.
    fn fixture() -> (
        ResourceBudget,
        Data,
        Rows<frames::Frame>,
        Rows<owner::Invocation>,
        Rows<owner::AnalysisCoverage>,
    ) {
        let (b, earlier) = frames::tests::fixture();
        let parent = frames::parents(&earlier, &b).unwrap().remove(0);
        let inv = owner::Invocation::new(
            parent.input,
            parent.context,
            super::super::build::definition().1.id(),
            None,
            parent.sources.iter().map(Record::id),
        )
        .0;
        let frame = frames::frame(&parent, inv.id());
        let q = assertion::AssertionQualification {
            assumptions: lctx_model::domain::assumptions::AssumptionSet::empty_id(),
            context: inv.context,
            scope: (source::CoverageScope::Input { input: inv.input }).id(),
            condition: Diagram::always().id(),
            modality: attribution::Modality::Candidate,
            approximation: assertion::Approximation::Over,
        };
        let mut data = Data::new(&b);
        data.qualifications.insert(q.clone()).unwrap();
        let (condition, nodes) = Diagram::always().records();
        data.conditions.insert(condition).unwrap();
        for row in nodes {
            data.nodes.insert(row).unwrap();
        }
        let entity: Id<normalized::entities::EntityRef> =
            serde_json::from_value(serde_json::json!(vec![24; 16])).unwrap();
        let source = structural::ConclusionSource::Public {
            candidate: serde_json::from_value(serde_json::json!(vec![25; 16])).unwrap(),
        };
        data.structural_sources.insert(source.clone()).unwrap();
        let conclusion = conclusion(
            frame.structural,
            source.id(),
            *parent
                .sources
                .iter()
                .find_map(|s| {
                    if let owner::InvocationSource::Structural { invocation } = s {
                        Some(invocation)
                    } else {
                        None
                    }
                })
                .unwrap(),
            entity,
            q.id(),
        );
        data.structural_conclusions.insert(conclusion).unwrap();
        let coverage = owner::AnalysisCoverage {
            invocation: inv.id(),
            capability: analysis::AnalysisCapability::Synthesis,
            scope: q.scope,
            context: q.context,
            premises: ContentHash::of(b"earlier-admitted-scope-control"),
            availability: normalized::coverage::EvidenceAvailability::Complete,
            reason: None,
        };
        let mut frames = Rows::new(&b);
        frames.insert(frame).unwrap();
        let mut invocations = Rows::new(&b);
        invocations.insert(inv).unwrap();
        let mut coverages = Rows::new(&b);
        coverages.insert(coverage).unwrap();
        (b, data, frames, invocations, coverages)
    }
    #[test]
    fn static_observation_keeps_candidate_qualification_and_nominal_proof_in_catalog_channel() {
        let (b, d, f, i, c) = fixture();
        let out = build(&d, &f, &i, &c, &b).unwrap();
        assert_eq!(out.findings.len(), 1);
        assert_eq!(out.derivations.len(), 1);
        let row = out.propositions.iter().next().unwrap();
        assert_eq!(
            (row.channel, row.phase),
            (
                analysis::AnalysisChannel::Catalog,
                calls::CallPhase::Definition
            )
        );
        let q = out.qualifications.get(row.qualification).unwrap();
        assert_eq!(q.modality, attribution::Modality::Candidate);
        assert_eq!(q.approximation, assertion::Approximation::Over);
        assert!(out.sources.iter().any(|s|matches!(s,owner::SupportSource::StructuralObservation{witness}if d.structural_conclusions.get(*witness).is_some())));
        assert_eq!(
            out.findings.iter().next().unwrap().status,
            analysis::policy::EvidenceStatus::StructurallyObserved
        );
    }
    #[test]
    fn foreign_scope_and_missing_exact_observation_source_refuse() {
        for case in 0..2 {
            let (b, mut d, f, i, c) = fixture();
            if case == 0 {
                let row = d.structural_conclusions.iter().next().unwrap().clone();
                let mut q = d.qualifications.get(row.qualification()).unwrap().clone();
                q.scope = source::CoverageScope::Artifact {
                    artifact: serde_json::from_value(serde_json::json!(vec![45; 16])).unwrap(),
                }
                .id();
                d.qualifications.insert(q.clone()).unwrap();
                let row = conclusion(row.frame, row.source, row.invocation, row.subject, q.id());
                d.structural_conclusions = Rows::new(&b);
                d.structural_conclusions.insert(row).unwrap();
            } else {
                d.structural_sources = Rows::new(&b);
            }
            assert!(build(&d, &f, &i, &c, &b).is_err());
        }
    }
}

#[cfg(test)]
mod replay_view_tests {
    use super::*;
    #[test]
    fn retained_and_current_qualifications_do_not_enlarge_native_documentary_pool() {
        use stages::PublicationBoundary as View;
        use assertion::AssertionQualification;
        let (budget, docs, _) = documentary::tests::fixture("\"Run.\"", "Run.");
        let native = docs.qualifications.iter().next().unwrap().clone();
        let mut retained = native.clone();
        retained.context = serde_json::from_value(serde_json::json!(vec![31; 16])).unwrap();
        let mut current = native.clone();
        current.context = serde_json::from_value(serde_json::json!(vec![32; 16])).unwrap();
        let mut check = Check {
            data: Data::new(&budget), summary: super::super::summary::Data::new(&budget),
            docs: documentary::Data::new(&budget), out: Output::new(&budget),
            frames: Rows::new(&budget), invocations: Rows::new(&budget), coverage: Rows::new(&budget), budget,
        };
        let facts = ValidationInput::of::<AssertionQualification>(&["id"]).at_epoch(View::Facts);
        let analytic = ValidationInput::of::<AssertionQualification>(&["id"]).at_epoch(View::Analytic);
        let output = ValidationInput::of::<AssertionQualification>(&["id"]);
        check.visit_input(&facts, &AssertionQualification::encode(std::slice::from_ref(&native)).unwrap()).unwrap();
        check.visit_input(&analytic, &AssertionQualification::encode(&[native.clone(), retained.clone()]).unwrap()).unwrap();
        check.visit_input(&output, &AssertionQualification::encode(&[native.clone(), retained.clone(), current.clone()]).unwrap()).unwrap();
        assert_eq!(check.docs.qualifications.len(), 1);
        assert_eq!(check.data.qualifications.len(), 2);
        assert_eq!(check.out.qualifications.len(), 3);
        assert!(check.docs.qualifications.get(retained.id()).is_none());
        assert!(check.data.qualifications.get(current.id()).is_none());
        assert_eq!(check.out.qualifications.get(current.id()), Some(&current));
        let batch = AssertionQualification::encode(&[current]).unwrap();
        assert!(check.visit_input(&ValidationInput::of::<AssertionQualification>(&["id"]).at_epoch(View::Local), &batch).is_err());
        assert!(check.visit(AssertionQualification::NAME, &batch).is_err());
    }
}
