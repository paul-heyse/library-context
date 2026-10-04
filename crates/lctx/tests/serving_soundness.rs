//! Independent generated CPython executions challenge current original native paths.
//! May compatibility is not an established identity claim; refutation is path-local.
#[path = "fixtures/serving_support.rs"]
mod support;
use lctx_model::domain::{
    self,
    assertion::{Approximation, AssertionQualification},
    attribution::Modality,
    execution::summary_path::SummaryPathWitness,
    flow::{FlowSinkKind, FlowTestLeafObservation, FlowValueObservation},
    local_semantics::LocalContribution,
    native_requests::{Assumptions, ExactOutcome, ExactScalar},
    serving::*,
    transfer::{
        self,
        summary::{SummaryContribution, SummaryWitness},
    },
    value::{Place, PlaceRoot},
    *,
};
use lctx_postgres::generations::Error;
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    process::Command,
};
use support::{ServingFixture, path};
#[derive(Debug, Deserialize)]
struct Function {
    name: String,
    flagged: bool,
    decorated: bool,
}
#[derive(Debug, Deserialize)]
struct ReturnTrace {
    function: String,
    line: u64,
    identity_entry: bool,
}
#[derive(Debug, Deserialize)]
struct GuardTrace {
    function: String,
    line: u64,
    value: bool,
}
#[derive(Debug, Deserialize)]
struct Run {
    flag: Option<bool>,
    scalar: bool,
    identity: bool,
    raised: Option<String>,
    returns: Vec<ReturnTrace>,
    guards: Vec<GuardTrace>,
}
#[derive(Debug, Deserialize)]
struct Bundle {
    source: String,
    functions: Vec<Function>,
    observed: BTreeMap<String, Vec<Run>>,
    groups: usize,
}
#[derive(Debug)]
enum NamedPath {
    Guard {
        line: u64,
    },
    Return {
        line: Option<u64>,
        definite_identity: bool,
    },
    Other,
}
fn nominal<R: Record>(reference: &ProofReference) -> Id<R> {
    assert_eq!(reference.relation.as_str(), R::NAME);
    serde_json::from_value(serde_json::json!(reference.row)).unwrap()
}
fn row<R: Record>(batch: &Batch<R>, id: Id<R>) -> R {
    batch
        .rows()
        .iter()
        .find(|r| r.id() == id)
        .expect("required original row")
        .clone()
}
fn line(source: &str, occurrence: &source::Occurrence) -> u64 {
    assert!(
        occurrence.start >= 0
            && occurrence.end >= occurrence.start
            && occurrence.end as usize <= source.len()
    );
    1 + source.as_bytes()[..occurrence.start as usize]
        .iter()
        .filter(|b| **b == b'\n')
        .count() as u64
}
async fn named_path(
    fixture: &ServingFixture,
    packet: &NativeAssessmentPacket,
    source: String,
) -> NamedPath {
    let reference = packet.path.clone();
    let original = packet.original_condition;
    let execution = fixture.service.execution().await.unwrap();
    execution
        .query(move |lease| {
            Box::pin(async move {
                if reference.relation.as_str() == FlowTestLeafObservation::NAME {
                    let id = nominal::<FlowTestLeafObservation>(&reference);
                    let leaf = row(&lease.read_ids::<FlowTestLeafObservation>(&[id]).await?, id);
                    let qualification = row(
                        &lease
                            .read_ids::<AssertionQualification>(&[leaf.qualification])
                            .await?,
                        leaf.qualification,
                    );
                    assert_eq!(qualification.condition, original);
                    let occurrence = row(
                        &lease.read_ids::<source::Occurrence>(&[leaf.test]).await?,
                        leaf.test,
                    );
                    return Ok(NamedPath::Guard {
                        line: line(&source, &occurrence),
                    });
                }
                if reference.relation.as_str() == LocalContribution::NAME {
                    let id = nominal::<LocalContribution>(&reference);
                    let contribution = row(&lease.read_ids::<LocalContribution>(&[id]).await?, id);
                    let value = row(
                        &lease
                            .read_ids::<FlowValueObservation>(&[contribution.value])
                            .await?,
                        contribution.value,
                    );
                    let key = row(
                        &lease
                            .read_ids::<transfer::local::TransferKey>(&[contribution.transfer])
                            .await?,
                        contribution.transfer,
                    );
                    let qualification = row(
                        &lease
                            .read_ids::<AssertionQualification>(&[contribution.qualification])
                            .await?,
                        contribution.qualification,
                    );
                    assert_eq!(qualification.condition, original);
                    if value.kind != FlowSinkKind::Return {
                        return Ok(NamedPath::Other);
                    }
                    let sink = row(
                        &lease.read_ids::<source::Occurrence>(&[value.sink]).await?,
                        value.sink,
                    );
                    return Ok(NamedPath::Return {
                        line: Some(line(&source, &sink)),
                        definite_identity: key.kind == transfer::TransferKind::Identity
                            && key.approximation == Approximation::Exact
                            && key.modality == Modality::Definite,
                    });
                }
                let (transfer, qualification) =
                    if reference.relation.as_str() == SummaryWitness::NAME {
                        let id = nominal::<SummaryWitness>(&reference);
                        let witness = row(&lease.read_ids::<SummaryWitness>(&[id]).await?, id);
                        (witness.transfer, witness.qualification)
                    } else if reference.relation.as_str() == SummaryPathWitness::NAME {
                        let id = nominal::<SummaryPathWitness>(&reference);
                        let witness = row(&lease.read_ids::<SummaryPathWitness>(&[id]).await?, id);
                        (witness.transfer, witness.qualification)
                    } else if reference.relation.as_str() == SummaryContribution::NAME {
                        let id = nominal::<SummaryContribution>(&reference);
                        let contribution =
                            row(&lease.read_ids::<SummaryContribution>(&[id]).await?, id);
                        let witness = row(
                            &lease
                                .read_ids::<SummaryWitness>(&[contribution.witness])
                                .await?,
                            contribution.witness,
                        );
                        (witness.transfer, witness.qualification)
                    } else {
                        panic!("undeclared native path owner: {reference:?}")
                    };
                let key = row(
                    &lease
                        .read_ids::<transfer::summary::TransferKey>(&[transfer])
                        .await?,
                    transfer,
                );
                let qualification = row(
                    &lease
                        .read_ids::<AssertionQualification>(&[qualification])
                        .await?,
                    qualification,
                );
                assert_eq!(qualification.condition, original);
                let output = row(&lease.read_ids::<Place>(&[key.output]).await?, key.output);
                let root = row(
                    &lease.read_ids::<PlaceRoot>(&[output.root]).await?,
                    output.root,
                );
                Ok(if matches!(root, PlaceRoot::Return { .. }) {
                    NamedPath::Return {
                        line: None,
                        definite_identity: key.kind == transfer::TransferKind::Identity
                            && key.approximation == Approximation::Exact
                            && key.modality == Modality::Definite,
                    }
                } else {
                    NamedPath::Other
                })
            })
        })
        .await
        .unwrap()
}
async fn originals_exist(fixture: &ServingFixture, packet: &NativeAssessmentPacket) {
    let model = domain::model().unwrap();
    let generation = fixture.generation;
    for reference in packet.proof.iter().chain(std::iter::once(&packet.path)) {
        assert!(
            model
                .relations()
                .iter()
                .any(|r| r.name() == reference.relation.as_str())
        );
        let sql = format!(
            "SELECT EXISTS(SELECT 1 FROM \"{}\".\"{}\" WHERE generation_id=$1 AND id=$2)",
            generation.schema(),
            reference.relation.as_str()
        );
        let exists: bool = sqlx::query_scalar(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(generation.bytes().as_slice())
            .bind(reference.row.as_slice())
            .fetch_one(&fixture.db.superuser)
            .await
            .unwrap();
        assert!(exists, "{reference:?}");
    }
    let sql = format!(
        "SELECT EXISTS(SELECT 1 FROM \"{}\".\"conditions\" WHERE generation_id=$1 AND id=$2)",
        generation.schema()
    );
    let exists: bool = sqlx::query_scalar(sqlx::AssertSqlSafe(sql.as_str()))
        .bind(generation.bytes().as_slice())
        .bind(packet.original_condition.bytes().as_slice())
        .fetch_one(&fixture.db.superuser)
        .await
        .unwrap();
    assert!(exists);
}
#[tokio::test]
async fn generated_cpython_observations_challenge_original_served_paths() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = Command::new("uv")
        .current_dir(&root)
        .args([
            "run",
            "--no-sync",
            "python",
            "tests/scripts/test_semantic_soundness.py",
            "--bundle",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let bundle: Bundle = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(bundle.groups, 9);
    assert!(bundle.functions.len() >= 33);
    let fixture = ServingFixture::start_profile(bundle.source.as_bytes(), "behavioral").await;
    let native = fixture.service.prepare_native().await.unwrap();
    let mut identity_challenges = 0;
    let mut refuted_guard_challenges = 0;
    let mut refuted_return_challenges = 0;
    let mut composition_paths = 0;
    for function in &bundle.functions {
        let observed = &bundle.observed[&function.name];
        assert!(observed.iter().all(|run| run.raised.is_none()));
        assert!(
            observed
                .iter()
                .all(|run| run.flag.is_some() == function.flagged),
            "captured flag inputs must match the generated function signature"
        );
        let execution = fixture.service.execution().await.unwrap();
        let response = fixture
            .catalog
            .operation(
                &execution,
                &GetOperationRequest {
                    library: Name::new("demo").unwrap(),
                    operation: path(&format!("demo.{}", function.name)),
                    comparison: Optional::default(), reference_parameter:Optional::default(),
                    sections: vec![],
                    page: PageRequest::default(),
                },
            )
            .await
            .unwrap();
        drop(execution);
        let OperationResolution::Unique { packet: operation } = response.operation else {
            panic!("generated public function is ambiguous: {}", function.name)
        };
        let mut requests = BTreeSet::new();
        for signature in &operation.core.signatures {
            for parameter in &signature.effective_parameters {
                let Some(name) = parameter.name.0.as_ref().map(Name::as_str) else {
                    continue;
                };
                if !matches!(name, "value" | "flag") {
                    continue;
                }
                for formal in &parameter.formals {
                    for flag in if name == "flag" {
                        vec![Some(true), Some(false)]
                    } else {
                        vec![None]
                    } {
                        if !requests.insert((signature.analysis, *formal, flag)) {
                            continue;
                        }
                        let value = flag
                            .map(|value| ExactScalar::Bool { value })
                            .unwrap_or_else(|| ExactScalar::String {
                                value: "oracle-sentinel".into(),
                            });
                        let mut request = InspectValuePathsRequest {
                            member: operation.core.member,
                            analysis: signature.analysis,
                            inputs: vec![ExactInputBinding {
                                formal: *formal,
                                value,
                            }],
                            assumptions: Assumptions::default(),
                            page: PageRequest::default(),
                        };
                        loop {
                            let execution = fixture.service.execution().await.unwrap();
                            let response = native.inspect(&execution, request.clone()).await;
                            drop(execution);
                            let response = match response {
                                Ok(response) => response,
                                Err(Error::Contract)
                                    if function.decorated
                                        && operation.core.signature_knowledge
                                            != normalized::callables::Knowledge::Known =>
                                {
                                    break;
                                }
                                Err(error) => panic!("{} {name}: {error:?}", function.name),
                            };
                            for packet in response.paths.items {
                                originals_exist(&fixture, &packet).await;
                                if function.decorated {
                                    assert!(
                                        matches!(
                                            packet.verdict,
                                            obligation::Verdict::Unknown
                                                | obligation::Verdict::NotAnalyzed
                                        ),
                                        "decorator replacement cannot admit the original body: {packet:?}"
                                    );
                                }
                                let named =
                                    named_path(&fixture, &packet, bundle.source.clone()).await;
                                if matches!(
                                    packet.path.relation.as_str(),
                                    SummaryWitness::NAME
                                        | SummaryPathWitness::NAME
                                        | SummaryContribution::NAME
                                ) {
                                    composition_paths += 1;
                                }
                                let runs = observed
                                    .iter()
                                    .filter(|run| {
                                        run.scalar && flag.is_none_or(|flag| run.flag == Some(flag))
                                    })
                                    .collect::<Vec<_>>();
                                assert!(!runs.is_empty());
                                match named {
                                    NamedPath::Guard { line }
                                        if packet.exact == ExactOutcome::RefutedPathUnderModel =>
                                    {
                                        assert_eq!(
                                            packet.verdict,
                                            obligation::Verdict::RefutedUnderModel
                                        );
                                        let guards = runs
                                            .iter()
                                            .flat_map(|run| &run.guards)
                                            .filter(|guard| {
                                                guard.function == function.name
                                                    && guard.line == line
                                            })
                                            .collect::<Vec<_>>();
                                        assert!(
                                            !guards.is_empty(),
                                            "named original guard was not independently challenged: {}:{line}",
                                            function.name
                                        );
                                        assert!(
                                            guards.iter().all(|guard| !guard.value),
                                            "CPython took a refuted guard: {}:{line}\n{}\n{packet:?}",
                                            function.name,
                                            bundle.source
                                        );
                                        refuted_guard_challenges += 1;
                                    }
                                    NamedPath::Return {
                                        line: Some(line), ..
                                    } if packet.exact == ExactOutcome::RefutedPathUnderModel => {
                                        assert!(
                                            runs.iter()
                                                .flat_map(|run| &run.returns)
                                                .all(|trace| trace.function != function.name
                                                    || trace.line != line),
                                            "CPython reached a refuted original return path: {}:{line}\n{}\n{packet:?}",
                                            function.name,
                                            bundle.source
                                        );
                                        refuted_return_challenges += 1;
                                    }
                                    NamedPath::Return {
                                        line,
                                        definite_identity: true,
                                    } if packet.verdict == obligation::Verdict::Established
                                        && name == "value" =>
                                    {
                                        // Challenge the canonical exact identity transfer only when its named
                                        // return executes. The exact DTO's May outcome is not promoted.
                                        let returns = runs
                                            .iter()
                                            .flat_map(|run| &run.returns)
                                            .filter(|trace| {
                                                trace.function == function.name
                                                    && line.is_none_or(|line| trace.line == line)
                                            })
                                            .collect::<Vec<_>>();
                                        assert!(
                                            !returns.is_empty(),
                                            "unconditional identity path lacks a concrete generated challenge"
                                        );
                                        assert!(
                                            returns.iter().all(|trace| trace.identity_entry),
                                            "CPython contradicted an original exact identity return: {}\n{}\n{packet:?}",
                                            function.name,
                                            bundle.source
                                        );
                                        identity_challenges += 1;
                                    }
                                    _ => {}
                                }
                            }
                            if let Some(cursor) = response.paths.continuation.0 {
                                request.page.cursor = Optional::supplied(cursor);
                            } else {
                                break;
                            }
                        }
                    }
                }
            }
        }
        if function.decorated {
            assert!(
                observed.iter().all(|run| !run.identity),
                "CPython replacement must return a different object"
            );
        }
    }
    assert!(
        identity_challenges > 0,
        "no independently challenged exact identity return"
    );
    assert!(
        refuted_guard_challenges > 0,
        "no independently challenged named guard refutation"
    );
    assert!(
        refuted_return_challenges > 0,
        "no independently challenged named return refutation"
    );
    assert!(
        composition_paths > 0,
        "generated acyclic calls must expose original finite composition paths"
    );
    eprintln!(
        "independent generated CPython challenge: groups={}, functions={}, exact_identity={}, refuted_guard={}, refuted_return={}, composed_paths={}",
        bundle.groups,
        bundle.functions.len(),
        identity_challenges,
        refuted_guard_challenges,
        refuted_return_challenges,
        composition_paths
    );
    fixture.finish().await;
}
