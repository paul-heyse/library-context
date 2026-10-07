//! Comparison identities freeze meaning; feedback never becomes oracle truth by opinion.
use crate::contracts::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Meanings {
    pub task_population: String,
    pub split_keys: String,
    pub public_requests: String,
    pub oracle: String,
    pub judgment: String,
    pub observation: String,
    pub wire_schema: String,
    pub completeness_applicability: String,
    pub journey_limits: String,
    pub metrics: String,
    pub numeric_precision_ties: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Realization {
    pub observation_realization: String,
    pub source: String,
    pub native: String,
    pub encoder: String,
    pub scorer: String,
    pub settings: BTreeMap<String, String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Experiment {
    pub revision: String,
    pub split: Split,
    pub meanings: Meanings,
    pub baseline: Realization,
    pub candidate: Realization,
    pub changed_variables: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Freeze {
    pub experiment: Experiment,
    pub digest: String,
}

pub fn freeze(experiment: Experiment) -> Result<Freeze, String> {
    let value = serde_json::to_value(&experiment).map_err(|e| e.to_string())?;
    if experiment.revision.is_empty()
        || serde_json::to_value(&experiment.meanings)
            .map_err(|e| e.to_string())?
            .as_object()
            .is_none_or(|fields| {
                fields
                    .values()
                    .any(|v| v.as_str().is_none_or(str::is_empty))
            })
    {
        return Err("freeze requires every comparison meaning".into());
    }
    let changed: Vec<_> = [
        "observation_realization",
        "source",
        "native",
        "encoder",
        "scorer",
        "settings",
    ]
    .into_iter()
    .filter(|key| value["baseline"][key] != value["candidate"][key])
    .map(str::to_owned)
    .collect();
    let declared: std::collections::BTreeSet<_> =
        experiment.changed_variables.iter().cloned().collect();
    if changed
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>()
        != declared
    {
        return Err(
            "changed variables do not match baseline/candidate realization differences".into(),
        );
    }
    let bytes = serde_json::to_vec(&experiment).map_err(|e| e.to_string())?;
    Ok(Freeze {
        experiment,
        digest: blake3::hash(&bytes).to_hex().to_string(),
    })
}
pub fn admit(frozen: &Freeze, candidate: &Experiment) -> Result<(), String> {
    let expected = freeze(frozen.experiment.clone())?;
    if expected.digest != frozen.digest {
        return Err("comparison freeze digest mismatch".into());
    }
    if &frozen.experiment != candidate {
        return Err(
            "incompatible comparison revision/meaning/realization; new baseline required".into(),
        );
    }
    Ok(())
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackCause {
    System,
    Evaluator,
    Usability,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FeedbackProposal {
    pub task_id: String,
    pub packet_digest: String,
    pub observation: Observation,
    pub independent_basis: OracleBasis,
    pub grounding: String,
    pub causes: Vec<FeedbackCause>,
    pub proposed_change: String,
    pub affected_tasks: Vec<String>,
    pub evaluator_revision: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FeedbackDisposition {
    pub routes: Vec<String>,
    pub regression_tasks: Vec<String>,
    pub new_comparison_required: bool,
    pub retain_old_semantic_result: bool,
    pub reason: String,
}
pub fn triage(
    proposal: &FeedbackProposal,
    current_revision: &str,
) -> Result<FeedbackDisposition, String> {
    if proposal.causes.is_empty()
        || proposal.grounding.is_empty()
        || proposal.proposed_change.is_empty()
        || proposal.task_id.is_empty()
        || proposal.affected_tasks.is_empty()
    {
        return Err("feedback requires grounded cause, proposal and affected tasks".into());
    }
    if proposal.independent_basis.completeness != Completeness::Complete
        || proposal.independent_basis.kind == "agent_opinion"
        || proposal.independent_basis.input_digest.is_empty()
        || proposal.independent_basis.supported_domain.is_empty()
    {
        return Err("agent opinion/incomplete basis cannot revise expected truth".into());
    }
    if crate::observation_digest(&proposal.observation) != proposal.packet_digest {
        return Err("feedback packet digest does not bind exact observation".into());
    }
    let changes_evaluator = proposal.causes.contains(&FeedbackCause::Evaluator);
    if changes_evaluator
        && proposal
            .evaluator_revision
            .as_ref()
            .is_none_or(|revision| revision.is_empty() || revision == current_revision)
    {
        return Err("evaluator meaning change requires a new revision/baseline".into());
    }
    let routes = proposal
        .causes
        .iter()
        .map(|cause| {
            match cause {
                FeedbackCause::System => "system_owner_and_independent_regression",
                FeedbackCause::Evaluator => "private_evaluator_new_meaning_and_baseline",
                FeedbackCause::Usability => {
                    "navigation_presentation_owner_and_optional_new_stratum"
                }
            }
            .to_owned()
        })
        .collect();
    Ok(FeedbackDisposition {
        routes,
        regression_tasks: proposal.affected_tasks.clone(),
        new_comparison_required: changes_evaluator,
        retain_old_semantic_result: true,
        reason: "grounded proposal routed; no automatic oracle or old-result relabeling".into(),
    })
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Rejudgment {
    pub old_revision: String,
    pub new_revision: String,
    pub old_case: Case,
    pub new_case: Case,
}
pub fn rejudge(input: &Rejudgment) -> Result<(Judgment, Judgment), String> {
    if input.old_revision == input.new_revision
        || input.old_revision.is_empty()
        || input.new_revision.is_empty()
    {
        return Err("rejudgment requires distinct explicit revisions".into());
    }
    let old = &input.old_case;
    let new = &input.new_case;
    if old.task.id != new.task.id
        || old.task.family != new.task.family
        || old.task.split != new.task.split
        || serde_json::to_value(&old.task.request).map_err(|e| e.to_string())?
            != serde_json::to_value(&new.task.request).map_err(|e| e.to_string())?
        || serde_json::to_value(&old.observation).map_err(|e| e.to_string())?
            != serde_json::to_value(&new.observation).map_err(|e| e.to_string())?
        || old.mode != new.mode
    {
        return Err(
            "same-packet rejudgment requires unchanged tasks/requests and exact observations"
                .into(),
        );
    }
    Ok((crate::judge(old), crate::judge(new)))
}
