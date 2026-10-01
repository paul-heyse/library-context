//! Finite automatic extras are navigation choices over exact earlier public slots.
use super::{documentary, seeds};
use crate::domain::{
    analysis::{self, settings::AnalyticsConfiguration, synthesis as owner},
    catalog::CatalogMemberInvocation,
    normalized::Rows,
    resources::ResourceBudget,
    *,
};
use crate::{Domain, DomainCode};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum DecisionReason {
    Selected = 0,
    Configured = 1,
    OutsideSubsystem = 2,
    NoSummary = 3,
    NoOfficialUsage = 4,
    Budget = 5,
    CommunityCap = 6,
    DuplicateSlot = 7,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum RankBasis {
    OfficialUsage = 0,
    SelectedPageRank = 1,
    PageRankUnavailable = 2,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="synthesis_automatic_seed_decisions",semantic_source=include_bytes!("automatic.rs"))]
pub struct Decision {
    #[model(key)]
    pub plan: Id<seeds::SeedPlan>,
    #[model(key)]
    pub candidate: Id<structural::PublicCandidate>,
    pub member: Id<CatalogMemberInvocation>,
    pub summary: Option<Id<documentary::DocumentaryConclusion>>,
    pub usage: Option<Id<structural::UsageScore>>,
    pub rank: Option<Id<analytics::RankScore>>,
    pub community: Option<Id<analytics::Community>>,
    pub score: FiniteF64,
    pub basis: RankBasis,
    pub reason: DecisionReason,
}
#[macro_export]
macro_rules! synthesis_automatic_inputs{($m:ident)=>{$m!{
 usage:$crate::domain::structural::UsageScore,analytic_frames:$crate::domain::analytics::AnalyticFrame,results:$crate::domain::analytics::TechniqueResult,analytic_invocations:$crate::domain::analysis::analytic::Invocation,ranks:$crate::domain::analytics::RankScore,communities:$crate::domain::analytics::Community,community_members:$crate::domain::analytics::CommunityMember,
}};}
macro_rules! data{($($f:ident:$t:ty,)*)=>{pub struct Data{$(pub $f:Rows<$t>,)*}impl Data{pub fn new(b:&ResourceBudget)->Self{Self{$($f:Rows::new(b),)*}}pub fn visit(&mut self,n:&str,b:&arrow_array::RecordBatch)->Result<bool,ModelError>{$(if n==<$t>::NAME{self.$f.decode(b)?;return Ok(true);})*Ok(false)}pub fn inputs()->Vec<ValidationInput>{vec![$(ValidationInput::of::<$t>(&["id"]),)*]}}};}
crate::synthesis_automatic_inputs!(data);
fn invalid(s: &str) -> ModelError {
    ModelError::Invalid(s.into())
}
fn need<R: Record>(r: &Rows<R>, id: Id<R>) -> Result<&R, ModelError> {
    r.get(id)
        .ok_or_else(|| invalid("automatic seed predecessor absent"))
}
fn result<'a>(
    a: &'a Data,
    s: &AnalyticsConfiguration,
    frame: Id<structural::StructuralFrame>,
    inv: &owner::Invocation,
    method: analysis::AnalysisMethod,
) -> Result<Option<&'a analytics::TechniqueResult>, ModelError> {
    let mut frames = a
        .analytic_frames
        .iter()
        .filter(|f| f.structural == frame && f.configuration == s.id());
    let Some(frame) = frames.next() else {
        return Ok(None);
    };
    if frames.next().is_some() {
        return Err(invalid("automatic seed analytic frame ambiguous"));
    }
    let mut rows = a
        .results
        .iter()
        .filter(|r| r.frame == frame.id() && r.method == method);
    let row = rows.next();
    if rows.next().is_some() {
        return Err(invalid("automatic seed result ambiguous"));
    }
    if let Some(row) = row {
        let parent = need(&a.analytic_invocations, row.invocation)?;
        if (parent.input, parent.context) != (inv.input, inv.context)
            || parent.definition != analytics::build::definition(s, method)?.1.id()
        {
            return Err(invalid(
                "automatic seed result changes exact selected frame",
            ));
        }
    }
    Ok(row)
}
/// The earlier owner validates each supplied row. This operation preserves all candidate
/// decisions, and never turns a rank/community navigation choice into semantic evidence.
pub fn complete(
    d: &documentary::Data,
    docs: &documentary::Output,
    public: &Rows<structural::PublicCandidate>,
    frames: &Rows<structural::StructuralFrame>,
    parents: &Rows<analysis::structural::Invocation>,
    a: &Data,
    s: &AnalyticsConfiguration,
    inv: &owner::Invocation,
    out: &mut seeds::Output,
    b: &ResourceBudget,
) -> Result<(), ModelError> {
    let plan = seeds::SeedPlan {
        invocation: inv.id(),
        configuration: s.id(),
    }
    .id();
    need(&out.plans, plan)?;
    let mut charge = charged::StateCharge::new(b, "automatic-seed-order");
    let mut decisions = Vec::<Decision>::new();
    let mut configured = charged::ChargedSet::default();
    let mut configured_communities = charged::ChargedSet::default();
    let mut held = charged::ChargedMap::<Id<analytics::Community>, usize>::default();
    for selected in out.selected.iter() {
        configured.insert(&mut charge, selected.member)?;
    }
    for candidate in public.iter() {
        let sf = need(frames, candidate.frame)?;
        if sf.configuration != s.id() {
            continue;
        }
        let parent = need(parents, sf.invocation)?;
        if (parent.input, parent.context) != (inv.input, inv.context) {
            continue;
        }
        let mut members = d.member_frames.iter().filter(|m| {
            m.member == candidate.member
                && d.core_invocations
                    .get(m.invocation)
                    .is_some_and(|p| (p.input, p.context) == (inv.input, inv.context))
        });
        let member = members
            .next()
            .ok_or_else(|| invalid("automatic public slot has no C0 frame"))?;
        if members.next().is_some() {
            return Err(invalid("automatic public slot frame ambiguous"));
        }
        let summary=docs.conclusions.iter().filter(|c|c.status()==analysis::policy::EvidenceStatus::Documented&&docs.sources.get(c.source).is_some_and(|source|matches!(source,documentary::DocumentarySource::Literal{member:m,..}if *m==member.id()))).min_by_key(|c|c.id());
        let usage = a.usage.get(
            structural::UsageScore {
                frame: sf.id(),
                target: candidate.entity,
                share: FiniteF64::new(0.0)?,
                contributing_sites: 0,
            }
            .id(),
        );
        let rank_result = result(a, s, sf.id(), inv, analysis::AnalysisMethod::PageRank)?;
        let rank = if s.pagerank {
            rank_result
                .filter(|r| r.selected && r.status != analysis::AnalysisStatus::NotRequested)
                .and_then(|r| {
                    a.ranks
                        .iter()
                        .find(|score| score.result == r.id() && score.target == candidate.entity)
                })
        } else {
            None
        };
        let community_result = result(a, s, sf.id(), inv, analysis::AnalysisMethod::Communities)?;
        let mut communities = a
            .community_members
            .iter()
            .filter(|m| m.entity == candidate.entity)
            .filter_map(|m| a.communities.get(m.community))
            .filter(|c| {
                s.communities
                    && community_result.is_some_and(|r| {
                        r.selected
                            && r.status != analysis::AnalysisStatus::NotRequested
                            && r.id() == c.result
                    })
            });
        let community = communities.next().map(Record::id);
        if communities.next().is_some() {
            return Err(invalid("automatic seed has ambiguous selected community"));
        }
        if configured.contains(&member.id()) {
            if let Some(c) =
                community.filter(|c| !configured_communities.contains(&(*c, member.id())))
            {
                configured_communities.insert(&mut charge, (c, member.id()))?;
                let count = held.get(&c).copied().unwrap_or(0);
                held.insert(
                    &mut charge,
                    c,
                    count
                        + out
                            .selected
                            .iter()
                            .filter(|r| r.member == member.id())
                            .count(),
                )?;
            }
        }
        let reason = if configured.contains(&member.id()) {
            DecisionReason::Configured
        } else if !candidate.in_subsystem {
            DecisionReason::OutsideSubsystem
        } else if summary.is_none() {
            DecisionReason::NoSummary
        } else if usage.is_none_or(|u| u.contributing_sites <= 0) {
            DecisionReason::NoOfficialUsage
        } else {
            DecisionReason::Selected
        };
        charge.grow(2 * size_of::<Decision>())?;
        decisions.reserve_exact(1);
        decisions.push(Decision {
            plan,
            candidate: candidate.id(),
            member: member.id(),
            summary: summary.map(Record::id),
            usage: usage.map(Record::id),
            rank: rank.map(Record::id),
            community,
            score: match rank {
                Some(r) => r.score,
                None => FiniteF64::new(usage.map_or(0.0, |u| u.contributing_sites as f64))?,
            },
            basis: if rank.is_some() {
                RankBasis::SelectedPageRank
            } else if s.pagerank {
                RankBasis::PageRankUnavailable
            } else {
                RankBasis::OfficialUsage
            },
            reason,
        });
    }
    decisions.sort_by(|x, y| {
        y.score
            .get()
            .total_cmp(&x.score.get())
            .then(x.member.cmp(&y.member))
            .then(x.candidate.cmp(&y.candidate))
    });
    let mut selected = configured;
    let mut count = out.selected.len();
    let mut ordinal = out
        .selected
        .iter()
        .map(|r| r.ordinal)
        .max()
        .map_or(0, |n| n + 1);
    let cap = (s.brief_budget as usize).div_ceil(3).max(1);
    for mut row in decisions {
        if row.reason == DecisionReason::Selected {
            row.reason = if selected.contains(&row.member) {
                DecisionReason::DuplicateSlot
            } else if count >= s.brief_budget as usize {
                DecisionReason::Budget
            } else if row
                .community
                .is_some_and(|c| held.get(&c).copied().unwrap_or(0) >= cap)
            {
                DecisionReason::CommunityCap
            } else {
                DecisionReason::Selected
            };
        }
        let id = out.automatic.insert(row.clone())?;
        if row.reason == DecisionReason::Selected {
            selected.insert(&mut charge, row.member)?;
            if let Some(c) = row.community {
                let n = held.get(&c).copied().unwrap_or(0);
                held.insert(&mut charge, c, n + 1)?;
            }
            let source = out
                .sources
                .insert(seeds::SelectedSeedSource::Automatic { decision: id })?;
            out.selected.insert(seeds::SelectedSeed {
                plan,
                ordinal,
                member: row.member,
                source,
            })?;
            ordinal += 1;
            count += 1;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn id<T>(n: u8) -> Id<T> {
        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
            _,
            serde::de::value::Error,
        >::new([n; 16].into_iter()))
        .unwrap()
    }
    // Pure selection assumes earlier C0/A0/A1 rows independently admitted; native store qualification
    // belongs to the final producer controls. Aliases share authored prose and an entity, not a slot.
    fn fixture_with(
        count: usize,
        budget: i64,
        pagerank: bool,
        communities: bool,
        configured: bool,
    ) -> (
        ResourceBudget,
        documentary::Data,
        documentary::Output,
        Rows<structural::PublicCandidate>,
        Rows<structural::StructuralFrame>,
        Rows<analysis::structural::Invocation>,
        Data,
        AnalyticsConfiguration,
        owner::Invocation,
    ) {
        let (b, mut d, original) = documentary::tests::fixture("\"Run.\"", "Run.");
        let mut settings = seeds::tests::settings(
            if configured {
                vec!["pkg.api.run".into()]
            } else {
                vec![]
            },
            budget,
        );
        settings.pagerank = pagerank;
        settings.communities = communities;
        let invocation = seeds::tests::invocation(&d);
        let c0 = d.member_frames.get(original).unwrap().clone();
        let member = d.members.get(c0.member).unwrap().clone();
        let candidate = d.candidates.iter().next().unwrap().clone();
        let exposure = d.exposures.iter().next().unwrap().clone();
        let parent = analysis::structural::Invocation::new(
            invocation.input,
            invocation.context,
            structural::build::definition(&settings, analysis::AnalysisMethod::Delegation)
                .unwrap()
                .1
                .id(),
            None,
            [],
        )
        .0;
        let frame = structural::StructuralFrame {
            invocation: parent.id(),
            usage_invocation: id(11),
            handoff_invocation: id(12),
            control_invocation: id(13),
            controls_requested: false,
            configuration: settings.id(),
            invocation_graph: id(14),
            definition_graph: id(15),
        };
        let mut frames = Rows::new(&b);
        frames.insert(frame.clone()).unwrap();
        let mut parents = Rows::new(&b);
        parents.insert(parent).unwrap();
        let mut public = Rows::new(&b);
        let mut a = Data::new(&b);
        for n in 0..count {
            let m = if n == 0 {
                member.id()
            } else {
                let m = d
                    .members
                    .insert(catalog::CatalogMember {
                        path: vec![format!("run{n}")],
                        name: format!("run{n}").into(),
                        ..member.clone()
                    })
                    .unwrap();
                d.member_frames
                    .insert(CatalogMemberInvocation {
                        member: m,
                        ..c0.clone()
                    })
                    .unwrap();
                let exposure = d
                    .exposures
                    .insert(catalog::CatalogExposure {
                        member: m,
                        ..exposure.clone()
                    })
                    .unwrap();
                d.candidates
                    .insert(catalog::CatalogCandidate {
                        exposure,
                        ..candidate.clone()
                    })
                    .unwrap();
                m
            };
            let entity = id(32 + n as u8);
            public
                .insert(structural::PublicCandidate {
                    frame: frame.id(),
                    callable: id(50 + n as u8),
                    member: m,
                    entity,
                    path: format!("pkg.api.run{n}"),
                    configured_ordinal: None,
                    in_subsystem: true,
                })
                .unwrap();
            a.usage
                .insert(structural::UsageScore {
                    frame: frame.id(),
                    target: entity,
                    share: FiniteF64::new(0.5).unwrap(),
                    contributing_sites: n as i64 + 1,
                })
                .unwrap();
        }
        let docs = documentary::build(&d, &b).unwrap();
        assert_eq!(docs.conclusions.len(), count);
        (b, d, docs, public, frames, parents, a, settings, invocation)
    }
    fn fixture(
        count: usize,
        budget: i64,
    ) -> (
        ResourceBudget,
        documentary::Data,
        documentary::Output,
        Rows<structural::PublicCandidate>,
        Rows<structural::StructuralFrame>,
        Rows<analysis::structural::Invocation>,
        Data,
        AnalyticsConfiguration,
        owner::Invocation,
    ) {
        fixture_with(count, budget, false, false, false)
    }
    fn analytic(
        a: &mut Data,
        s: &AnalyticsConfiguration,
        sf: Id<structural::StructuralFrame>,
        i: &owner::Invocation,
        method: analysis::AnalysisMethod,
        selected: bool,
    ) -> analytics::TechniqueResult {
        let frame = analytics::AnalyticFrame {
            structural: sf,
            configuration: s.id(),
        };
        a.analytic_frames.insert(frame.clone()).unwrap();
        let inv = analysis::analytic::Invocation::new(
            i.input,
            i.context,
            analytics::build::definition(s, method).unwrap().1.id(),
            None,
            [],
        )
        .0;
        a.analytic_invocations.insert(inv.clone()).unwrap();
        let r = analytics::TechniqueResult {
            frame: frame.id(),
            method,
            invocation: inv.id(),
            selected,
            status: if selected {
                analysis::AnalysisStatus::Completed
            } else {
                analysis::AnalysisStatus::NotRequested
            },
            stop: if selected {
                analytics::Stop::Converged
            } else {
                analytics::Stop::NotRequested
            },
            iterations: 0,
            examined: 0,
            residual: None,
            input_partial: false,
        };
        a.results.insert(r.clone()).unwrap();
        r
    }
    fn select(
        d: &documentary::Data,
        docs: &documentary::Output,
        p: &Rows<structural::PublicCandidate>,
        f: &Rows<structural::StructuralFrame>,
        parents: &Rows<analysis::structural::Invocation>,
        a: &Data,
        s: &AnalyticsConfiguration,
        i: &owner::Invocation,
        b: &ResourceBudget,
    ) -> seeds::Output {
        let mut out = seeds::configured(d, p, f, parents, s, i, b).unwrap();
        complete(d, docs, p, f, parents, a, s, i, &mut out, b).unwrap();
        out
    }
    #[test]
    fn eligible_extras_follow_official_usage_and_zero_budget_is_explicit() {
        let (b, d, docs, p, f, parents, a, s, i) = fixture(4, 2);
        let out = select(&d, &docs, &p, &f, &parents, &a, &s, &i, &b);
        assert_eq!(out.selected.len(), 2);
        let selected = out.selected.iter().find(|r| r.ordinal == 0).unwrap();
        let source = out.sources.get(selected.source).unwrap();
        let seeds::SelectedSeedSource::Automatic { decision } = source else {
            panic!("expected automatic extra")
        };
        let decision = out.automatic.get(*decision).unwrap();
        assert_eq!(decision.score.get(), 4.0);
        assert_eq!(decision.basis, RankBasis::OfficialUsage);
        assert_eq!(out.automatic.len(), 4);
        assert_eq!(
            out.automatic
                .iter()
                .filter(|r| r.reason == DecisionReason::Budget)
                .count(),
            2
        );
        let (b, d, docs, p, f, parents, a, s, i) = fixture(2, 0);
        let out = select(&d, &docs, &p, &f, &parents, &a, &s, &i, &b);
        assert!(out.selected.is_empty());
        assert!(
            out.automatic
                .iter()
                .all(|r| r.reason == DecisionReason::Budget)
        );
    }
    #[test]
    fn eligibility_requires_summary_and_official_usage_and_keeps_outside_candidates() {
        let (b, d, mut docs, mut p, f, parents, mut a, s, i) = fixture(3, 3);
        docs.conclusions = Rows::new(&b);
        let out = select(&d, &docs, &p, &f, &parents, &a, &s, &i, &b);
        assert!(out.selected.is_empty());
        assert!(
            out.automatic
                .iter()
                .all(|r| r.reason == DecisionReason::NoSummary)
        );
        let docs = documentary::build(&d, &b).unwrap();
        a.usage = Rows::new(&b);
        let out = select(&d, &docs, &p, &f, &parents, &a, &s, &i, &b);
        assert!(out.selected.is_empty());
        assert!(
            out.automatic
                .iter()
                .all(|r| r.reason == DecisionReason::NoOfficialUsage)
        );
        let old = p.iter().next().unwrap().clone();
        p = Rows::new(&b);
        p.insert(structural::PublicCandidate {
            in_subsystem: false,
            ..old
        })
        .unwrap();
        let out = select(&d, &docs, &p, &f, &parents, &a, &s, &i, &b);
        assert_eq!(
            out.automatic.iter().next().unwrap().reason,
            DecisionReason::OutsideSubsystem
        );
    }
    #[test]
    fn selected_pagerank_orders_extras_and_notrequested_rows_do_not_supply_rank() {
        let (b, d, docs, p, f, parents, mut a, s, i) = fixture_with(4, 1, true, false, false);
        let sf = f.iter().next().unwrap().id();
        let result = analytic(&mut a, &s, sf, &i, analysis::AnalysisMethod::PageRank, true);
        for candidate in p.iter() {
            a.ranks
                .insert(analytics::RankScore {
                    result: result.id(),
                    target: candidate.entity,
                    score: FiniteF64::new(
                        10.0 - a
                            .usage
                            .iter()
                            .find(|u| u.target == candidate.entity)
                            .unwrap()
                            .contributing_sites as f64,
                    )
                    .unwrap(),
                })
                .unwrap();
        }
        let out = select(&d, &docs, &p, &f, &parents, &a, &s, &i, &b);
        let chosen = out
            .automatic
            .iter()
            .find(|r| r.reason == DecisionReason::Selected)
            .unwrap();
        assert_eq!(chosen.score.get(), 9.0);
        assert_eq!(chosen.basis, RankBasis::SelectedPageRank);
        let old = a.results.iter().next().unwrap().clone();
        a.results = Rows::new(&b);
        a.results
            .insert(analytics::TechniqueResult {
                selected: false,
                status: analysis::AnalysisStatus::NotRequested,
                stop: analytics::Stop::NotRequested,
                ..old
            })
            .unwrap();
        let out = select(&d, &docs, &p, &f, &parents, &a, &s, &i, &b);
        let chosen = out
            .automatic
            .iter()
            .find(|r| r.reason == DecisionReason::Selected)
            .unwrap();
        assert_eq!(chosen.score.get(), 4.0);
        assert_eq!(chosen.basis, RankBasis::PageRankUnavailable);
        assert!(chosen.rank.is_none());
    }
    #[test]
    fn configured_seeds_count_toward_selected_community_cap() {
        let (b, d, docs, p, f, parents, mut a, s, i) = fixture_with(5, 4, false, true, true);
        let sf = f.iter().next().unwrap().id();
        let result = analytic(
            &mut a,
            &s,
            sf,
            &i,
            analysis::AnalysisMethod::Communities,
            true,
        );
        let community = a
            .communities
            .insert(analytics::Community {
                result: result.id(),
                representative: p.iter().next().unwrap().entity,
                agreement: FiniteF64::new(1.0).unwrap(),
            })
            .unwrap();
        for candidate in p.iter() {
            a.community_members
                .insert(analytics::CommunityMember {
                    community,
                    entity: candidate.entity,
                })
                .unwrap();
        }
        let out = select(&d, &docs, &p, &f, &parents, &a, &s, &i, &b);
        assert_eq!(out.selected.len(), 2);
        let first = out.selected.iter().find(|r| r.ordinal == 0).unwrap();
        assert!(matches!(
            out.sources.get(first.source).unwrap(),
            seeds::SelectedSeedSource::Configured { .. }
        ));
        assert_eq!(
            out.automatic
                .iter()
                .filter(|r| r.reason == DecisionReason::CommunityCap)
                .count(),
            3
        );
        let expected = out
            .automatic
            .iter()
            .find(|r| r.reason == DecisionReason::Selected)
            .unwrap();
        assert_eq!(expected.score.get(), 5.0);
        let mut erased = out;
        erased.automatic = Rows::new(&b);
        let replay = select(&d, &docs, &p, &f, &parents, &a, &s, &i, &b);
        assert!(erased.matches(&replay).is_err());
    }
    #[test]
    fn foreign_analytic_context_is_refused() {
        let (b, d, docs, p, f, parents, mut a, s, i) = fixture_with(1, 1, true, false, false);
        let sf = f.iter().next().unwrap().id();
        analytic(&mut a, &s, sf, &i, analysis::AnalysisMethod::PageRank, true);
        let old = a.analytic_invocations.iter().next().unwrap().clone();
        a.analytic_invocations = Rows::new(&b);
        a.analytic_invocations
            .insert(analysis::analytic::Invocation {
                context: id(199),
                ..old
            })
            .unwrap();
        let mut out = seeds::configured(&d, &p, &f, &parents, &s, &i, &b).unwrap();
        assert!(complete(&d, &docs, &p, &f, &parents, &a, &s, &i, &mut out, &b).is_err());
    }
}
