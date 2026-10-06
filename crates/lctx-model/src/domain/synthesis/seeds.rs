//! Ordered configured requests resolve catalog-owned public slots, independently of a winning entity.
use super::documentary;
use crate::domain::{
    analysis::{self, settings::AnalyticsConfiguration, synthesis as owner},
    catalog::CatalogMemberInvocation,
    normalized::Rows,
    resources::ResourceBudget,
    *,
};
use crate::{Domain, DomainCode, DomainSum};
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name="synthesis_seed_plans",invariant_refs=invariants_refs)]
pub struct SeedPlan {
    #[model(key)]
    pub invocation: Id<owner::Invocation>,
    #[model(key)]
    pub configuration: Id<AnalyticsConfiguration>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum RequestStatus {
    Selected = 0,
    Missing = 1,
    Ambiguous = 2,
    OutsideSubsystem = 3,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum ScopeApplicability {
    Within = 0,
    Outside = 1,
    Unresolved = 2,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "synthesis_configured_seed_decisions")]
pub struct ConfiguredSeedDecision {
    #[model(key)]
    pub plan: Id<SeedPlan>,
    #[model(key)]
    pub ordinal: i64,
    pub requested: Utf8Text,
    pub status: RequestStatus,
    pub scope: ScopeApplicability,
    pub candidates: i64,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "synthesis_configured_seed_candidates")]
pub struct ConfiguredSeedCandidate {
    #[model(key)]
    pub decision: Id<ConfiguredSeedDecision>,
    #[model(key)]
    pub member: Id<CatalogMemberInvocation>,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum, serde::Serialize, serde::Deserialize)]
#[model(name = "synthesis_selected_seed_sources")]
pub enum SelectedSeedSource {
    #[model(code = 0)]
    Configured {
        decision: Id<ConfiguredSeedDecision>,
    },
    #[model(code = 1)]
    Automatic {
        decision: Id<super::automatic::Decision>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "synthesis_selected_seeds")]
pub struct SelectedSeed {
    #[model(key)]
    pub plan: Id<SeedPlan>,
    #[model(key)]
    pub ordinal: i64,
    pub member: Id<CatalogMemberInvocation>,
    pub source: Id<SelectedSeedSource>,
}
macro_rules! outputs{($apply:ident)=>{$apply!{plans:SeedPlan,automatic:super::automatic::Decision,decisions:ConfiguredSeedDecision,candidates:ConfiguredSeedCandidate,sources:SelectedSeedSource,selected:SelectedSeed,}};}
macro_rules! output{($($f:ident:$ty:ty,)*)=>{pub struct Output{$(pub $f:Rows<$ty>,)*}impl Output{pub fn new(b:&ResourceBudget)->Self{Self{$($f:Rows::new(b),)*}}pub fn visit(&mut self,n:&str,b:&arrow_array::RecordBatch)->Result<bool,ModelError>{$(if n==<$ty>::NAME{self.$f.decode(b)?;return Ok(true);})*Ok(false)}pub fn validation_inputs()->Vec<ValidationInput>{vec![$(ValidationInput::of::<$ty>(&["id"]),)*]}pub fn matches(&self,o:&Self)->Result<(),ModelError>{$(if !self.$f.same(&o.$f){return Err(invalid(concat!("synthesis seed inventory differs: ",stringify!($f))));})*Ok(())}}};}
outputs!(output);
fn invalid(s: impl Into<String>) -> ModelError {
    ModelError::Invalid(s.into())
}
fn need<R: Record>(rows: &Rows<R>, id: Id<R>) -> Result<&R, ModelError> {
    rows.get(id)
        .ok_or_else(|| invalid(format!("seed input absent: {}", R::NAME)))
}
pub fn path(
    d: &documentary::Data,
    frame: Id<CatalogMemberInvocation>,
    b: &ResourceBudget,
) -> Result<(String, Box<dyn resources::Reservation>), ModelError> {
    let frame = need(&d.member_frames, frame)?;
    let member = need(&d.members, frame.member)?;
    let module = need(&d.modules, member.access)?;
    let len = module.qualified_name.len()
        + member.path.iter().map(String::len).sum::<usize>()
        + member.path.len();
    let charge = b.reserve("synthesis-public-slot-label", len)?;
    let mut name = String::with_capacity(len);
    name.push_str(&module.qualified_name);
    for segment in &member.path {
        name.push('.');
        name.push_str(segment);
    }
    Ok((name, charge))
}
/// Empty authored seed lists are ordinary plans. Class/unresolved slots are never 'missing'
/// because the earlier callable-only structural selector does not contain them.
pub fn configured(
    d: &documentary::Data,
    public: &Rows<structural::PublicCandidate>,
    structural_frames: &Rows<structural::StructuralFrame>,
    structural_invocations: &Rows<analysis::structural::Invocation>,
    settings: &AnalyticsConfiguration,
    invocation: &owner::Invocation,
    b: &ResourceBudget,
) -> Result<Output, ModelError> {
    settings.validate()?;
    if invocation.definition != super::build::definition().1.id() {
        return Err(invalid("seed plan requires canonical synthesis definition"));
    }
    let mut out = Output::new(b);
    let plan = out.plans.insert(SeedPlan {
        invocation: invocation.id(),
        configuration: settings.id(),
    })?;
    for (ordinal, requested) in settings.configured_seeds.iter().enumerate() {
        let mut candidates = Rows::new(b);
        for frame in d.member_frames.iter() {
            let parent = need(&d.core_invocations, frame.invocation)?;
            if (parent.input, parent.context) != (invocation.input, invocation.context) {
                continue;
            }
            if path(d, frame.id(), b)?.0 == *requested {
                candidates.insert(frame.clone())?;
            }
        }
        let scope = if candidates.len() == 1 {
            let member = candidates.iter().next().unwrap().member;
            let mut observed = 0;
            let mut within = 0;
            for candidate in public.iter().filter(|c| c.member == member) {
                let frame = need(structural_frames, candidate.frame)?;
                if frame.configuration != settings.id() {
                    continue;
                }
                let earlier = need(structural_invocations, frame.invocation)?;
                if (earlier.input, earlier.context) != (invocation.input, invocation.context) {
                    continue;
                }
                observed += 1;
                if candidate.in_subsystem {
                    within += 1;
                }
            }
            if observed == 0 || within > 0 && within < observed {
                ScopeApplicability::Unresolved
            } else if within == 0 {
                ScopeApplicability::Outside
            } else {
                ScopeApplicability::Within
            }
        } else {
            ScopeApplicability::Unresolved
        };
        let status = match candidates.len() {
            0 => RequestStatus::Missing,
            1 if scope == ScopeApplicability::Outside => RequestStatus::OutsideSubsystem,
            1 => RequestStatus::Selected,
            _ => RequestStatus::Ambiguous,
        };
        let decision = out.decisions.insert(ConfiguredSeedDecision {
            plan,
            ordinal: ordinal as i64,
            requested: requested.clone().into(),
            status,
            scope,
            candidates: candidates.len() as i64,
        })?;
        for candidate in candidates.iter() {
            out.candidates.insert(ConfiguredSeedCandidate {
                decision,
                member: candidate.id(),
            })?;
        }
        if status == RequestStatus::Selected {
            let source = out
                .sources
                .insert(SelectedSeedSource::Configured { decision })?;
            out.selected.insert(SelectedSeed {
                plan,
                ordinal: ordinal as i64,
                member: candidates.iter().next().unwrap().id(),
                source,
            })?;
        }
    }
    Ok(out)
}
pub fn relations() -> Vec<Relation> {
    macro_rules! rows{($($f:ident:$ty:ty,)*)=>{vec![$(Relation::of::<$ty>()),*]};}
    outputs!(rows)
}
struct Check {
    automatic: super::automatic::Data,
    analytic: super::frames::AnalyticParents,
    docs: documentary::Output,
    budget: ResourceBudget,
    data: documentary::Data,
    settings: Rows<AnalyticsConfiguration>,
    invocations: Rows<owner::Invocation>,
    public: Rows<structural::PublicCandidate>,
    structural_frames: Rows<structural::StructuralFrame>,
    structural_invocations: Rows<analysis::structural::Invocation>,
    output: Output,
}
pub fn invariants() -> Vec<Invariant> {
    let mut inputs = documentary::replay_inputs(
        documentary::Data::validation_inputs(),
        stages::PublicationBoundary::Facts,
    );
    inputs.extend(Output::validation_inputs());
    inputs.extend(super::automatic::Data::inputs());
    inputs.extend(super::frames::AnalyticParents::inputs());
    inputs.extend(documentary::Output::validation_inputs());
    inputs.extend([
        ValidationInput::of::<AnalyticsConfiguration>(&["id"]),
        ValidationInput::of::<owner::Invocation>(&["id"]),
        ValidationInput::of::<structural::PublicCandidate>(&["id"]),
        ValidationInput::of::<structural::StructuralFrame>(&["id"]),
        ValidationInput::of::<analysis::structural::Invocation>(&["id"]),
    ]);
    inputs.sort_by_key(|r| (r.name(), r.prefix()));
    inputs.dedup_by_key(|r| (r.name(), r.prefix()));
    vec![Invariant {
        revision: 2,
        name: "synthesis_configured_seed_replay",
        inputs,
        create: std::sync::Arc::new(|b| {
            Box::new(Check {
                automatic: super::automatic::Data::new(b),
                analytic: super::frames::AnalyticParents::new(b),
                docs: documentary::Output::new(b),
                budget: b.clone(),
                data: documentary::Data::new(b),
                settings: Rows::new(b),
                invocations: Rows::new(b),
                public: Rows::new(b),
                structural_frames: Rows::new(b),
                structural_invocations: Rows::new(b),
                output: Output::new(b),
            })
        }),
    }]
}
impl InvariantCheck for Check {
    fn visit(&mut self, _name: &str, _batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        Err(ModelError::Invalid(
            "S0 replay requires an explicit completed-input selector".into(),
        ))
    }
    fn visit_input(
        &mut self,
        input: &ValidationInput,
        b: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        documentary::replay_selector(input)?;
        let n = input.name();
        macro_rules! rows{($($f:ident:$ty:ty,)*)=>{$(if n==<$ty>::NAME{self.$f.decode(b)?;return Ok(());})*};}
        rows! {settings:AnalyticsConfiguration,invocations:owner::Invocation,public:structural::PublicCandidate,structural_frames:structural::StructuralFrame,structural_invocations:analysis::structural::Invocation,}
        let d =
            documentary::replay_visit(input, Some(stages::PublicationBoundary::Facts), |name| {
                self.data.visit(name, b)
            })?;
        let o = documentary::replay_visit(input, None, |name| self.output.visit(name, b))?;
        let a = documentary::replay_visit(input, None, |name| self.automatic.visit(name, b))?;
        let analytic = documentary::replay_visit(input, None, |name| self.analytic.visit(name, b))?;
        let docs = documentary::replay_visit(input, None, |name| self.docs.visit(name, b))?;
        if !d && !o && !a && !docs && !analytic {
            return Err(invalid("undeclared seed replay input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        let mut expected = Output::new(&self.budget);
        for inv in self.invocations.iter() {
            let mut settings = self.settings.iter();
            let settings = settings
                .next()
                .ok_or_else(|| invalid("selected synthesis settings absent"))?;
            if self.settings.len() != 1 {
                return Err(invalid("ambiguous synthesis settings"));
            }
            let mut out = configured(
                &self.data,
                &self.public,
                &self.structural_frames,
                &self.structural_invocations,
                settings,
                inv,
                &self.budget,
            )?;
            self.docs
                .matches(&documentary::build(&self.data, &self.budget)?)?;
            super::automatic::complete(
                &self.data,
                &self.docs,
                &self.public,
                &self.structural_frames,
                &self.structural_invocations,
                &self.automatic,
                &self.analytic,
                settings,
                inv,
                &mut out,
                &self.budget,
            )?;
            macro_rules! extend{($($f:ident:$ty:ty,)*)=>{$(for row in out.$f.iter(){expected.$f.insert(row.clone())?;})*};}
            outputs!(extend);
        }
        self.output.matches(&expected)
    }
}

pub(crate) fn invariants_refs() -> Vec<&'static str> {
    vec!["synthesis_configured_seed_replay"]
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    pub(in crate::domain::synthesis) fn settings(
        paths: Vec<String>,
        budget: i64,
    ) -> AnalyticsConfiguration {
        AnalyticsConfiguration {
            module_prefixes: vec!["pkg.impl".into()],
            public_roots: vec!["pkg.api".into()],
            configured_seeds: paths,
            depth: 2,
            vertices: 128,
            arcs: 512,
            witnesses: 3,
            brief_budget: budget,
            communities: false,
            pagerank: false,
            fca: false,
            knn: false,
            rca: false,
            type_layer: false,
            mention_layer: false,
            knn_layer: false,
        }
    }
    pub(in crate::domain::synthesis) fn invocation(d: &documentary::Data) -> owner::Invocation {
        let parent = d.core_invocations.iter().next().unwrap();
        owner::Invocation::new(
            parent.input,
            parent.context,
            super::super::build::definition().1.id(),
            None,
            [],
        )
        .0
    }
    pub(in crate::domain::synthesis) fn select(
        d: &documentary::Data,
        settings: &AnalyticsConfiguration,
        b: &ResourceBudget,
    ) -> Output {
        configured(
            d,
            &Rows::new(b),
            &Rows::new(b),
            &Rows::new(b),
            settings,
            &invocation(d),
            b,
        )
        .unwrap()
    }
    #[test]
    fn configured_order_missing_requests_and_callable_selector_independence() {
        let (b, d, member) = documentary::tests::fixture("\"Run.\"", "Run.");
        let s = settings(vec!["pkg.api.absent".into(), "pkg.api.run".into()], 2);
        let out = select(&d, &s, &b);
        assert_eq!(out.plans.len(), 1);
        assert_eq!(out.decisions.len(), 2);
        let missing = out.decisions.iter().find(|r| r.ordinal == 0).unwrap();
        assert_eq!(
            (missing.status, missing.candidates),
            (RequestStatus::Missing, 0)
        );
        let selected = out.selected.iter().next().unwrap();
        assert_eq!((selected.ordinal, selected.member), (1, member));
        let found = out.decisions.iter().find(|r| r.ordinal == 1).unwrap();
        assert_eq!(
            (found.status, found.scope),
            (RequestStatus::Selected, ScopeApplicability::Unresolved)
        );
    }
    #[test]
    fn explicit_empty_configuration_budget_zero_has_a_plan_and_no_requests() {
        let (b, d, _) = documentary::tests::fixture("\"Run.\"", "Run.");
        let out = select(&d, &settings(vec![], 0), &b);
        assert_eq!(out.plans.len(), 1);
        assert!(out.decisions.is_empty());
        assert!(out.selected.is_empty());
        assert!(out.candidates.is_empty());
    }
    #[test]
    fn equal_public_names_in_distinct_slots_are_ambiguous_and_keep_both_candidates() {
        let (b, mut d, original) = documentary::tests::fixture("\"Run.\"", "Run.");
        let member = d.member_frames.get(original).unwrap().member;
        let old = d.members.get(member).unwrap().clone();
        let module = d.modules.get(old.access).unwrap().clone();
        let source = d.artifacts.get(module.source).unwrap();
        let artifact = source::SourceArtifact::from_bytes(
            source.input,
            "second.py".into(),
            b"different original",
        )
        .unwrap();
        d.artifacts.insert(artifact.clone()).unwrap();
        let access = d
            .modules
            .insert(source::Module {
                source: artifact.id(),
                qualified_name: module.qualified_name,
            })
            .unwrap();
        let member = d
            .members
            .insert(catalog::CatalogMember { access, ..old })
            .unwrap();
        d.member_frames
            .insert(CatalogMemberInvocation {
                member,
                invocation: d.member_frames.get(original).unwrap().invocation,
            })
            .unwrap();
        let out = select(&d, &settings(vec!["pkg.api.run".into()], 1), &b);
        let decision = out.decisions.iter().next().unwrap();
        assert_eq!(
            (decision.status, decision.candidates),
            (RequestStatus::Ambiguous, 2)
        );
        assert_eq!(out.candidates.len(), 2);
        assert!(out.selected.is_empty());
    }
    #[test]
    fn changed_definition_foreign_frame_and_coupled_decision_erasure_are_not_selected() {
        let (b, d, _) = documentary::tests::fixture("\"Run.\"", "Run.");
        let s = settings(vec!["pkg.api.run".into()], 1);
        let expected = select(&d, &s, &b);
        assert!(expected.matches(&Output::new(&b)).is_err());
        let mut inv = invocation(&d);
        inv.definition = super::super::documentary::tests::fixture("\"Other.\"", "Other.")
            .1
            .core_invocations
            .iter()
            .next()
            .unwrap()
            .definition;
        assert!(
            configured(
                &d,
                &Rows::new(&b),
                &Rows::new(&b),
                &Rows::new(&b),
                &s,
                &inv,
                &b
            )
            .is_err()
        );
    }
}
