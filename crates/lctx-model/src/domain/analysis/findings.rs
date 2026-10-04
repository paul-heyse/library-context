//! Synthesis is the single writer of finding records. Earlier owners emit typed conclusions.
//! Status and heuristic restrictions come from the common immutable source projection.
pub use super::policy::*;
use super::{
    invalid,
    synthesis::{
        support::{
            EvidenceIndex, EvidencePremise, QualificationOperation, QualifiedPremise,
            QualifiedResult, qualify_with_basis,
        },
        *,
    },
};
use crate::Domain;
use crate::domain::{
    assertion::AssertionQualification, conditions::Diagram,
    normalized::coverage::EvidenceAvailability, *,
};
pub struct FindingEvidence<'a> {
    pub role: SupportRole,
    premise: EvidencePremise<'a>,
}
impl<'a> FindingEvidence<'a> {
    pub fn new(role: SupportRole, premise: EvidencePremise<'a>) -> Self {
        Self { role, premise }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="findings",invariants=finding_invariants)]
pub struct Finding {
    #[model(key)]
    pub invocation: Id<Invocation>,
    #[model(key)]
    pub kind: FindingKind,
    #[model(key)]
    pub subject: Id<ObligationSubject>,
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub coverage: Id<Coverage>,
    #[model(key)]
    pub members: ContentHash,
    #[model(key)]
    pub supports: ContentHash,
    pub status: EvidenceStatus,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "finding_members")]
pub struct FindingMember {
    #[model(key)]
    pub finding: Id<Finding>,
    #[model(key)]
    pub role: MemberRole,
    #[model(key)]
    pub subject: Id<ObligationSubject>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="finding_supports",rule="finding_support",conclusion=finding)]
pub struct FindingSupport {
    #[model(key)]
    pub finding: Id<Finding>,
    #[model(key)]
    pub role: SupportRole,
    #[model(key, premise)]
    pub source: Id<SupportSource>,
}
fn member_digest(
    members: &std::collections::BTreeSet<(MemberRole, Id<ObligationSubject>)>,
) -> ContentHash {
    let mut sink = KeySink::new("finding-members");
    for (role, subject) in members {
        role.encode(&mut sink);
        subject.encode(&mut sink);
    }
    sink.finish()
}
fn support_digest(
    supports: &std::collections::BTreeSet<(SupportRole, Id<SupportSource>)>,
) -> ContentHash {
    let mut sink = KeySink::new("finding-supports");
    for (role, source) in supports {
        role.encode(&mut sink);
        source.encode(&mut sink);
    }
    sink.finish()
}
/// The emitter never accepts a requested qualification or status. Scope evidence cannot strengthen
/// a finding; any heuristic sibling refuses behavioral kinds even when other evidence is stronger.
pub fn emit(
    invocation: Id<Invocation>,
    kind: FindingKind,
    subject: Id<ObligationSubject>,
    members: &[(MemberRole, Id<ObligationSubject>)],
    coverage: &Coverage,
    evidence: &[FindingEvidence<'_>],
    budget: &resources::ResourceBudget,
) -> Result<
    (
        Finding,
        Vec<FindingMember>,
        Vec<FindingSupport>,
        QualifiedResult,
    ),
    ModelError,
> {
    emit_with_basis(
        invocation, kind, subject, members, coverage, evidence, None, budget,
    )
}
#[allow(
    clippy::too_many_arguments,
    reason = "Finding emission keeps subject membership, closure, evidence and assumption resolution explicit."
)]
pub fn emit_with_basis(
    invocation: Id<Invocation>,
    kind: FindingKind,
    subject: Id<ObligationSubject>,
    members: &[(MemberRole, Id<ObligationSubject>)],
    coverage: &Coverage,
    evidence: &[FindingEvidence<'_>],
    basis: Option<&dyn assumptions::AssumptionResolver>,
    budget: &resources::ResourceBudget,
) -> Result<
    (
        Finding,
        Vec<FindingMember>,
        Vec<FindingSupport>,
        QualifiedResult,
    ),
    ModelError,
> {
    let mut charge = charged::StateCharge::new(budget, "finding_emitter");
    let mut member_set = charged::ChargedSet::default();
    let mut source_set = charged::ChargedSet::default();
    for member in members {
        member_set.insert(&mut charge, *member)?;
    }
    for support in evidence {
        if !source_set.insert(
            &mut charge,
            (support.role, support.premise.premise.source.id()),
        )? {
            return Err(invalid("duplicate finding evidence"));
        }
    }
    let bytes = (members.len() + evidence.len())
        .checked_mul(
            size_of::<FindingMember>()
                + size_of::<FindingSupport>()
                + size_of::<QualifiedPremise<'_>>()
                + size_of::<(SupportRole, EvidenceStatus)>(),
        )
        .ok_or_else(|| invalid("finding allocation overflow"))?;
    let _reservation = budget.reserve("finding_emitter", bytes)?;
    let premises = evidence
        .iter()
        .map(|e| QualifiedPremise {
            source: e.premise.premise.source,
            qualification: e.premise.premise.qualification,
            condition: e.premise.premise.condition,
        })
        .collect::<Vec<_>>();
    let qualification = qualify_with_basis(
        QualificationOperation::Conjunction,
        &premises,
        basis,
        budget,
    )?;
    if (coverage.invocation, coverage.scope, coverage.context)
        != (
            invocation,
            qualification.qualification.scope,
            qualification.qualification.context,
        )
    {
        return Err(invalid("finding coverage changes invocation or frame"));
    }
    let status = if matches!(
        coverage.availability,
        EvidenceAvailability::Unavailable
            | EvidenceAvailability::NotRequested
            | EvidenceAvailability::NoScope
    ) {
        EvidenceStatus::Unresolved
    } else {
        derive_status(
            &evidence
                .iter()
                .map(|e| (e.role, e.premise.facts.status))
                .collect::<Vec<_>>(),
        )
    };
    if evidence.iter().any(|e| e.premise.facts.heuristic) {
        finding_policy(kind, EvidenceStatus::StatisticallyDerived)?;
    }
    finding_policy(kind, status)?;
    let row = Finding {
        invocation,
        kind,
        subject,
        qualification: qualification.qualification.id(),
        coverage: coverage.id(),
        members: member_digest(&member_set),
        supports: support_digest(&source_set),
        status,
    };
    let members = member_set
        .iter()
        .map(|(role, subject)| FindingMember {
            finding: row.id(),
            role: *role,
            subject: *subject,
        })
        .collect();
    let supports = source_set
        .iter()
        .map(|(role, source)| FindingSupport {
            finding: row.id(),
            role: *role,
            source: *source,
        })
        .collect();
    Ok((row, members, supports, qualification))
}
fn finding_invariants() -> Vec<Invariant> {
    let mut inputs = EvidenceIndex::inputs();
    inputs.extend([
        ValidationInput::of::<Finding>(&["id"]),
        ValidationInput::of::<FindingMember>(&["id"]),
        ValidationInput::of::<FindingSupport>(&["id"]),
        ValidationInput::of::<Coverage>(&["id"]),
        ValidationInput::of::<AssertionQualification>(&["id"]),
        ValidationInput::of::<conditions::Condition>(&["id"]),
        ValidationInput::of::<conditions::ConditionNode>(&["id"]),
    ]);
    vec![Invariant {
        name: "finding_emitter",
        inputs,
        create: std::sync::Arc::new(|budget| {
            Box::new(FindingCheck {
                charge: charged::StateCharge::new(budget, "finding_emitter"),
                evidence: EvidenceIndex::new(budget),
                findings: Default::default(),
                members: Default::default(),
                supports: Default::default(),
                coverage: Default::default(),
                qualifications: Default::default(),
                conditions: Default::default(),
                nodes: Default::default(),
            })
        }),
    }]
}
struct FindingCheck {
    charge: charged::StateCharge,
    evidence: EvidenceIndex,
    findings: charged::ChargedMap<Id<Finding>, Finding>,
    members: charged::ChargedMap<
        Id<Finding>,
        std::collections::BTreeSet<(MemberRole, Id<ObligationSubject>)>,
    >,
    supports: charged::ChargedMap<
        Id<Finding>,
        std::collections::BTreeSet<(SupportRole, Id<SupportSource>)>,
    >,
    coverage: charged::ChargedMap<Id<Coverage>, Coverage>,
    qualifications: charged::ChargedMap<Id<AssertionQualification>, AssertionQualification>,
    conditions: charged::ChargedMap<Id<conditions::Condition>, conditions::Condition>,
    nodes: charged::ChargedMap<Id<conditions::ConditionNode>, conditions::ConditionNode>,
}
impl InvariantCheck for FindingCheck {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if self.evidence.visit(relation, batch)? {
            return Ok(());
        }
        macro_rules! insert {
            ($r:ty,$field:ident) => {
                if relation == <$r>::NAME {
                    for row in <$r>::decode(batch)? {
                        if self
                            .$field
                            .insert(&mut self.charge, row.id(), row)?
                            .is_some()
                        {
                            return Err(ModelError::Conflict(<$r>::NAME));
                        }
                    }
                    return Ok(());
                }
            };
        }
        insert!(Finding, findings);
        insert!(Coverage, coverage);
        insert!(AssertionQualification, qualifications);
        insert!(conditions::Condition, conditions);
        insert!(conditions::ConditionNode, nodes);
        if relation == FindingMember::NAME {
            for row in FindingMember::decode(batch)? {
                if !self.members.update(&mut self.charge, row.finding, |m| {
                    m.insert((row.role, row.subject))
                })? {
                    return Err(invalid("duplicate finding member"));
                }
            }
            return Ok(());
        }
        if relation == FindingSupport::NAME {
            for row in FindingSupport::decode(batch)? {
                if !self.supports.update(&mut self.charge, row.finding, |m| {
                    m.insert((row.role, row.source))
                })? {
                    return Err(invalid("duplicate finding support"));
                }
            }
            return Ok(());
        }
        Err(invalid("undeclared finding emitter input"))
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        let budget = self
            .charge
            .budget()
            .ok_or_else(|| invalid("finding budget absent"))?;
        let empty = std::collections::BTreeSet::new();
        let node_bytes = self.nodes.values().try_fold(0usize, |n, row| {
            n.checked_add(size_of::<conditions::ConditionNode>() + row.heap_bytes() + 128)
                .ok_or_else(|| invalid("finding node allocation overflow"))
        })?;
        for (id, row) in self.findings.iter() {
            let members = self.members.get(id).unwrap_or(&empty);
            let supports = self
                .supports
                .get(id)
                .ok_or_else(|| invalid("finding evidence absent"))?;
            if member_digest(members) != row.members || support_digest(supports) != row.supports {
                return Err(invalid("finding exact membership differs"));
            }
            let bytes = node_bytes
                .checked_mul(supports.len() + 1)
                .and_then(|n| {
                    n.checked_add(
                        supports
                            .len()
                            .checked_mul(size_of::<FindingEvidence<'_>>() + size_of::<Diagram>())?,
                    )
                })
                .ok_or_else(|| invalid("finding allocation overflow"))?;
            let _reservation = budget.reserve("finding_emitter_recheck", bytes)?;
            let nodes = self.nodes.values().cloned().collect::<Vec<_>>();
            let mut diagrams = Vec::with_capacity(supports.len());
            let mut observed = Vec::with_capacity(supports.len());
            for (role, source) in supports {
                let (source, facts) = self.evidence.get(*source)?;
                let q = self
                    .qualifications
                    .get(&facts.qualification)
                    .ok_or_else(|| invalid("finding source qualification absent"))?;
                let condition = self
                    .conditions
                    .get(&q.condition)
                    .ok_or_else(|| invalid("finding source condition absent"))?;
                diagrams.push(Diagram::from_records(condition, &nodes)?);
                observed.push((*role, source, q, facts));
            }
            let evidence = observed
                .iter()
                .zip(&diagrams)
                .map(|((role, source, q, facts), condition)| {
                    FindingEvidence::new(
                        *role,
                        EvidencePremise {
                            premise: QualifiedPremise {
                                source,
                                qualification: q,
                                condition,
                            },
                            facts: *facts,
                        },
                    )
                })
                .collect::<Vec<_>>();
            let coverage = self
                .coverage
                .get(&row.coverage)
                .ok_or_else(|| invalid("finding coverage absent"))?;
            if emit_with_basis(
                row.invocation,
                row.kind,
                row.subject,
                &members.iter().copied().collect::<Vec<_>>(),
                coverage,
                &evidence,
                Some(&self.evidence.assumptions),
                budget,
            )?
            .0 != *row
            {
                return Err(invalid(
                    "finding strengthens source lineage or qualification",
                ));
            }
        }
        for id in self.members.keys() {
            if !self.findings.contains_key(id) {
                return Err(invalid("orphan finding member"));
            }
        }
        for id in self.supports.keys() {
            if !self.findings.contains_key(id) {
                return Err(invalid("orphan finding support"));
            }
        }
        Ok(())
    }
}
pub fn relations() -> Vec<Relation> {
    vec![
        Relation::of::<Finding>(),
        Relation::of::<FindingMember>(),
        Relation::of::<FindingSupport>(),
    ]
}
