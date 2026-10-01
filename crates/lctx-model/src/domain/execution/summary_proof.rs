//! Finite proof costs are replayed from earlier occurrences, never producer-selected ranks.
use super::{
    configuration::{self, SummaryLimits},
    summary_path::SummaryPathWitness,
    summary_worklist::ProofCost,
};
use crate::Domain;
use crate::domain::{
    analysis::{AnalysisDefinition, MethodParameters, summary::AnalysisInvocation},
    normalized::Rows,
    resources::ResourceBudget,
    transfer::summary::{SummaryPremise, SummaryWitness},
    *,
};
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="summary_proof_costs",rule="finite_summary_proof_cost",invariants=invariants)]
pub struct SummaryProofCost {
    #[model(key, premise)]
    pub proof: Id<SummaryPremise>,
    pub depth: i64,
    pub steps: i64,
    pub rank: i64,
}
impl SummaryProofCost {
    pub fn new(proof: Id<SummaryPremise>, cost: ProofCost) -> Self {
        Self {
            proof,
            depth: cost.depth.into(),
            steps: cost.steps.into(),
            rank: cost.rank.into(),
        }
    }
    pub fn cost(&self) -> Result<ProofCost, ModelError> {
        Ok(ProofCost {
            depth: self
                .depth
                .try_into()
                .map_err(|_| invalid("invalid summary depth"))?,
            steps: self
                .steps
                .try_into()
                .map_err(|_| invalid("invalid summary cost"))?,
            rank: self
                .rank
                .try_into()
                .map_err(|_| invalid("invalid summary rank"))?,
        })
    }
}
impl ProofCost {
    pub fn through_path(source: Self) -> Result<Self, crate::domain::obligation::ObligationKind> {
        Ok(Self {
            depth: source.depth,
            steps: source
                .steps
                .checked_add(1)
                .ok_or(crate::domain::obligation::ObligationKind::SummaryProofLimit)?,
            rank: source
                .rank
                .checked_add(1)
                .ok_or(crate::domain::obligation::ObligationKind::SummaryProofLimit)?,
        })
    }
}
fn invalid(s: &str) -> ModelError {
    ModelError::Invalid(s.into())
}
pub fn limits(
    parameters: &MethodParameters,
    definition: &AnalysisDefinition,
) -> Result<SummaryLimits, ModelError> {
    let number = |v: Option<i64>| {
        v.and_then(|n| u32::try_from(n).ok())
            .ok_or_else(|| invalid("Summary limit missing or invalid"))
    };
    let limits = SummaryLimits {
        depth: number(parameters.depth)?,
        proof_steps: number(parameters.proof_steps)?,
        work: number(parameters.work)?,
        members: number(parameters.members)?,
    };
    let catalog = parameters
        .model_catalog
        .ok_or_else(|| invalid("Summary catalog absent"))?;
    if configuration::summaries(catalog, limits)? != (parameters.clone(), definition.clone()) {
        return Err(invalid("Summary definition differs from its bound kernel"));
    }
    Ok(limits)
}
pub fn relations() -> Vec<Relation> {
    vec![Relation::of::<SummaryProofCost>()]
}
fn invariants() -> Vec<Invariant> {
    vec![Invariant {
        name: "finite_summary_cost_equations",
        inputs: vec![
            ValidationInput::of::<SummaryProofCost>(&["rank", "id"]),
            ValidationInput::of::<SummaryPremise>(&["id"]),
            ValidationInput::of::<SummaryWitness>(&["id"]),
            ValidationInput::of::<SummaryPathWitness>(&["id"]),
            ValidationInput::of::<AnalysisInvocation>(&["id"]),
            ValidationInput::of::<AnalysisDefinition>(&["id"]),
            ValidationInput::of::<MethodParameters>(&["id"]),
        ],
        create: std::sync::Arc::new(|b| Box::new(CostCheck::new(b))),
    }]
}
struct CostCheck {
    costs: Rows<SummaryProofCost>,
    premises: Rows<SummaryPremise>,
    calls: Rows<SummaryWitness>,
    paths: Rows<SummaryPathWitness>,
    invocations: Rows<AnalysisInvocation>,
    definitions: Rows<AnalysisDefinition>,
    parameters: Rows<MethodParameters>,
    budget: ResourceBudget,
}
impl CostCheck {
    fn new(b: &ResourceBudget) -> Self {
        Self {
            costs: Rows::new(b),
            premises: Rows::new(b),
            calls: Rows::new(b),
            paths: Rows::new(b),
            invocations: Rows::new(b),
            definitions: Rows::new(b),
            parameters: Rows::new(b),
            budget: b.clone(),
        }
    }
}
impl InvariantCheck for CostCheck {
    fn visit(&mut self, n: &str, b: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        macro_rules! row {
            ($t:ty,$f:ident) => {
                if n == <$t>::NAME {
                    self.$f.decode(b)?;
                    return Ok(());
                }
            };
        }
        row!(SummaryProofCost, costs);
        row!(SummaryPremise, premises);
        row!(SummaryWitness, calls);
        row!(SummaryPathWitness, paths);
        row!(AnalysisInvocation, invocations);
        row!(AnalysisDefinition, definitions);
        row!(MethodParameters, parameters);
        Err(invalid("undeclared Summary cost input"))
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        let _order = self.budget.reserve(
            "summary-proof-cost-order",
            self.costs
                .len()
                .saturating_mul(size_of::<&SummaryProofCost>() + 64),
        )?;
        let mut rows = self.costs.iter().collect::<Vec<_>>();
        rows.sort_by_key(|r| (r.rank, r.proof));
        let mut costs = charged::ChargedMap::<Id<SummaryPremise>, ProofCost>::default();
        let mut charge = charged::StateCharge::new(&self.budget, "summary-proof-cost-replay");
        for row in rows {
            let source = |id| -> Result<ProofCost, ModelError> {
                match self
                    .premises
                    .get(id)
                    .ok_or_else(|| invalid("cost parent premise absent"))?
                {
                    SummaryPremise::Local { .. } | SummaryPremise::Model { .. } => {
                        Ok(ProofCost::SOURCE)
                    }
                    SummaryPremise::Witness { .. } | SummaryPremise::Path { .. } => costs
                        .get(&id)
                        .copied()
                        .ok_or_else(|| invalid("summary proof does not precede its consumer")),
                }
            };
            let (invocation, expected) = match self
                .premises
                .get(row.proof)
                .ok_or_else(|| invalid("cost proof absent"))?
            {
                SummaryPremise::Witness { witness } => {
                    let w = self
                        .calls
                        .get(*witness)
                        .ok_or_else(|| invalid("cost call witness absent"))?;
                    (
                        w.invocation,
                        ProofCost::through_call(source(w.caller)?, source(w.callee)?),
                    )
                }
                SummaryPremise::Path { witness } => {
                    let w = self
                        .paths
                        .get(*witness)
                        .ok_or_else(|| invalid("cost path witness absent"))?;
                    (w.invocation, ProofCost::through_path(source(w.source)?))
                }
                _ => return Err(invalid("raw evidence cannot publish a summary proof cost")),
            };
            let expected = expected.map_err(|_| invalid("summary proof cost overflow"))?;
            if expected != row.cost()? {
                return Err(invalid(
                    "summary proof depth/cost/rank differs from finite parents",
                ));
            }
            let invocation = self
                .invocations
                .get(invocation)
                .ok_or_else(|| invalid("cost invocation absent"))?;
            let definition = self
                .definitions
                .get(invocation.definition)
                .ok_or_else(|| invalid("cost definition absent"))?;
            let parameters = self
                .parameters
                .get(definition.parameters)
                .ok_or_else(|| invalid("cost parameters absent"))?;
            expected
                .check(limits(parameters, definition)?)
                .map_err(|_| invalid("stored summary witness exceeds declared limits"))?;
            costs.insert(&mut charge, row.proof, expected)?;
        }
        Ok(())
    }
}
