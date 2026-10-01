//! Deployment descriptions and explicitly captured reports; analyzed code is never executed.
use crate::{
    acquisition::{AcquiredInput, Acquisition},
    assembly,
    bundle::{Declared, ProviderStage, StageContext},
};
use lctx_model::domain::{
    assertion::*, attribution::*, conditions::Diagram, deployment::*, input::*, source::*,
    stages::*, *,
};
use sha2::{Digest as _, Sha256};
use std::collections::BTreeMap;
pub const DEPLOYMENT: &str = "deployment";
pub struct Deployment;
pub fn provider() -> Provider {
    Provider {
        tool: "lctx-deployment".into(),
        revision: env!("CARGO_PKG_VERSION").into(),
        build_digest: crate::bundle::build_digest(&[
            include_str!("deployment.rs"),
            include_str!("deployment_parser.rs"),
        ]),
    }
}
macro_rules! outputs {
    ($f:ident) => {
        $f!(
            DeploymentObservation,
            DeploymentSupport,
            ReportValue,
            ReportCollection,
            ReportEntry,
            ReportedEnvironment,
            TaskReport,
            TaskReportObservation,
            TaskReportSupport
        )
    };
}
impl Declared for Deployment {
    fn declaration(&self, _: Profile) -> Stage {
        macro_rules! uses {($($ty:ty),+)=>{vec![$(RelationUse::of::<$ty>()),+]}}
        let provider = provider();
        Stage {
            name: DEPLOYMENT,
            inputs: uses!(
                SourceArtifact,
                ArtifactUse,
                EnvironmentFingerprint,
                Package,
                Release,
                InputAcquisition,
                CorpusLibrary
            ),
            outputs: outputs!(uses),
            contributes: assembly::vocabulary(),
            coverage: vec![FactFamily::Deployment],
            provider: Some(provider.id()),
            profiles: vec![Profile::Catalog, Profile::Behavioral],
            effect: Effect::Extraction,
            code: provider.build_digest,
            configuration: ContentHash::of(
                b"receipt-v1;64KiB;metadata-and-configuration=64MiB;report-only-runtime",
            ),
        }
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReceiptEnvironment {
    pub release_id: Id<Release>,
    pub lock_digest: ContentHash,
    pub environment_digest: ContentHash,
    pub runtime_digest: ContentHash,
    pub interpreter_digest: ContentHash,
    pub python_version: String,
    pub platform: String,
    pub requirement: String,
    pub metadata: BTreeMap<String, ContentHash>,
}
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    pub format: u32,
    pub policy: String,
    pub task: String,
    pub runner_sha256: String,
    pub source_path: String,
    pub source_sha256: String,
    pub environment: ReceiptEnvironment,
    pub command: Vec<String>,
    pub tool: String,
    pub arguments: BTreeMap<String, i64>,
    pub elapsed_ms: u64,
    pub timeout_seconds: u32,
    pub execution: ReceiptStatus,
    pub tools: Vec<String>,
    pub result: Option<String>,
    pub diagnostic: Option<String>,
}
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptStatus {
    Passed,
    Failed,
    NotRun,
    Blocked,
}
impl ReceiptStatus {
    fn typed(&self) -> CheckStatus {
        match self {
            Self::Passed => CheckStatus::Passed,
            Self::Failed => CheckStatus::Failed,
            Self::NotRun => CheckStatus::NotRun,
            Self::Blocked => CheckStatus::Blocked,
        }
    }
}
fn invalid(message: impl Into<String>) -> ModelError {
    ModelError::Invalid(message.into())
}
fn digest_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn metadata(input: &AcquiredInput) -> BTreeMap<String, ContentHash> {
    input
        .captured()
        .artifacts()
        .iter()
        .filter(|a| {
            a.path.ends_with(".dist-info/METADATA")
                || a.path.ends_with(".dist-info/entry_points.txt")
        })
        .map(|a| (a.path.clone(), a.content))
        .collect()
}
/// Captured identity fields which can be checked independently of a reported execution. Native
/// runtime/interpreter digests remain reported fields, as specified by EnvironmentFingerprint.
#[derive(Debug, serde::Serialize)]
pub struct CapturedEnvironment {
    pub release_id: Id<Release>,
    pub lock_digest: ContentHash,
    pub environment_digest: ContentHash,
    pub python_version: String,
    pub platform: String,
    pub requirement: String,
    pub metadata: BTreeMap<String, ContentHash>,
}
pub fn identity(input: &AcquiredInput) -> Result<CapturedEnvironment, ModelError> {
    let Acquisition::Installed(library) = input.acquisition() else {
        return Err(invalid(
            "deployment identity requires a captured installed library",
        ));
    };
    let distribution = library
        .distributions
        .iter()
        .find(|d| d.first_party)
        .ok_or_else(|| invalid("deployment identity has no first-party distribution"))?;
    let package = Package {
        name: distribution.name.clone(),
    };
    let release = Release {
        package: package.id(),
        version: distribution.version.clone(),
    };
    Ok(CapturedEnvironment {
        release_id: release.id(),
        lock_digest: library.lock_digest,
        environment_digest: input.captured().revision().manifest,
        python_version: library.python_version.clone(),
        platform: library.platform.clone(),
        requirement: library.requirement.clone(),
        metadata: metadata(input),
    })
}
fn check_receipt(
    receipt: &Receipt,
    input: &AcquiredInput,
    library: Option<&AcquiredInput>,
    budget: &lctx_model::domain::resources::ResourceBudget,
) -> Result<Id<SourceArtifact>, ModelError> {
    let library = library
        .ok_or_else(|| invalid("receipt association requires a captured installed library"))?;
    let expected = identity(library)?;
    let reported = &receipt.environment;
    if reported.release_id != expected.release_id
        || reported.lock_digest != expected.lock_digest
        || reported.environment_digest != expected.environment_digest
        || reported.python_version != expected.python_version
        || reported.platform != expected.platform
        || reported.requirement != expected.requirement
        || reported.metadata != expected.metadata
    {
        return Err(invalid(
            "receipt environment differs from captured installed input",
        ));
    }
    validate_path(&receipt.source_path)?;
    let uses = input.uses();
    let target = input
        .captured()
        .artifacts()
        .iter()
        .find(|a| {
            a.path == receipt.source_path
                && uses.iter().any(|u| {
                    u.artifact == a.id()
                        && matches!(
                            u.role,
                            SourceRole::Example
                                | SourceRole::Test
                                | SourceRole::Document
                                | SourceRole::Configuration
                        )
                })
        })
        .ok_or_else(|| invalid("receipt source is not selected corpus evidence"))?;
    let _target_charge = budget.reserve(
        "deployment_receipt_target",
        usize::try_from(target.byte_len)
            .map_err(ModelError::codec)?
            .saturating_mul(2)
            .saturating_add(4096),
    )?;
    let bytes =
        std::fs::read(input.captured().root().join(&target.path)).map_err(ModelError::codec)?;
    if ContentHash::of(&bytes) != target.content || digest_hex(&bytes) != receipt.source_sha256 {
        return Err(invalid("receipt source hash differs from captured target"));
    }
    if receipt.format != 1
        || receipt.policy != "fastmcp-stdio-v1"
        || !matches!(receipt.task.as_str(), "programmatic" | "cli")
    {
        return Err(invalid("unsupported task receipt format/policy"));
    }
    let command = if receipt.task == "programmatic" {
        vec!["<environment>/bin/python", "<source>/server.py"]
    } else {
        vec![
            "<environment>/bin/python",
            "-c",
            "from fastmcp.cli import app; app()",
            "run",
            "<source>/server.py:mcp",
            "--transport",
            "stdio",
        ]
    };
    if receipt.source_path != "examples/fastmcp_config/server.py"
        || receipt.command != command
        || receipt.tool != "add"
        || receipt.arguments != BTreeMap::from([("a".into(), 2), ("b".into(), 3)])
    {
        return Err(invalid("receipt does not match interaction policy"));
    }
    if receipt.runner_sha256 != digest_hex(include_bytes!("../../../scripts/deployment_check.py")) {
        return Err(invalid("receipt runner hash differs"));
    }
    if receipt.timeout_seconds != 45
        || receipt.elapsed_ms > 60000
        || receipt.tools.len() > 100
        || receipt.result.as_ref().is_some_and(|r| r.len() > 4096)
        || receipt.diagnostic.as_ref().is_some_and(|r| r.len() > 8192)
    {
        return Err(invalid("receipt limits disagree"));
    }
    if matches!(receipt.execution, ReceiptStatus::Passed)
        && (!receipt.tools.iter().any(|s| s == "add")
            || receipt.result.as_deref() != Some("5")
            || receipt.diagnostic.is_some())
    {
        return Err(invalid("reported success lacks required observation"));
    }
    Ok(target.id())
}
fn collection<S: StageSink + 'static>(
    context: &mut StageContext<S>,
    kind: ReportCollectionKind,
    values: Vec<ReportValue>,
) -> Result<Id<ReportCollection>, ModelError> {
    let (row, values, entries) = ReportCollection::new(kind, values)?;
    let id = row.id();
    context.emit(row)?;
    for value in values {
        context.emit(value)?;
    }
    for entry in entries {
        context.emit(entry)?;
    }
    Ok(id)
}
impl<S: StageSink + 'static> ProviderStage<S> for Deployment {
    fn run(&mut self, context: &mut StageContext<S>) -> Result<ProviderOutcome, ModelError> {
        macro_rules! declare {($($ty:ty),+)=>{$(context.declare::<$ty>()?;)+}}
        outputs!(declare);
        let provider = provider();
        context.contribute(provider.clone())?;
        let captured = context.captured();
        let mut partial = false;
        for input in captured.inputs() {
            let library = match input.acquisition() {
                Acquisition::Corpus { library, .. } => captured.inputs().get(*library),
                _ => None,
            };
            let analysis =
                crate::pyrefly_stage::analysis_context(input, library, captured.config())?;
            let scope = CoverageScope::Input {
                input: input.captured().revision().id(),
            };
            let (condition, nodes) = Diagram::always().records();
            let q = AssertionQualification {
                context: analysis.id(),
                scope: scope.id(),
                condition: condition.id(),
                modality: Modality::Definite,
                approximation: Approximation::Exact,
            };
            let (run, families) = ProviderRun::new(
                provider.id(),
                analysis.id(),
                input.captured().revision().id(),
                analysis.config_digest,
                [FactFamily::Deployment],
            )?;
            let surface = ProviderSurface {
                provider: provider.id(),
                family: FactFamily::Deployment,
                name: "captured metadata, configuration and task reports".into(),
            };
            context.contribute(analysis.clone())?;
            context.contribute(scope.clone())?;
            context.contribute(condition)?;
            for node in nodes {
                context.contribute(node)?;
            }
            context.contribute(q.clone())?;
            context.contribute(run.clone())?;
            for family in families {
                context.contribute(family)?;
            }
            context.contribute(surface.clone())?;
            let uses = input.uses();
            let mut input_partial = false;
            let mut reason = None;
            for artifact in input.captured().artifacts() {
                let role = uses
                    .iter()
                    .find(|u| {
                        u.artifact == artifact.id()
                            && matches!(
                                u.role,
                                SourceRole::DistributionMetadata
                                    | SourceRole::Configuration
                                    | SourceRole::TaskReceipt
                            )
                    })
                    .map(|u| u.role);
                let Some(role) = role else { continue };
                let evidence = Evidence::SourceSpan {
                    source: artifact.id(),
                    start: 0,
                    end: artifact.byte_len,
                };
                let span = EvidenceSourceSpanId::of(&evidence)?;
                context.contribute(evidence.clone())?;
                let limit = if role == SourceRole::TaskReceipt {
                    65536
                } else {
                    64 << 20
                };
                let oversized = artifact.byte_len > limit;
                let _charge = context.budget().reserve(
                    "deployment_native_input",
                    if oversized {
                        4096
                    } else {
                        (artifact.byte_len as usize)
                            .saturating_mul(32)
                            .saturating_add(4096)
                    },
                )?;
                let bytes = if oversized {
                    vec![]
                } else {
                    let bytes = std::fs::read(input.captured().root().join(&artifact.path))
                        .map_err(ModelError::codec)?;
                    if ContentHash::of(&bytes) != artifact.content {
                        return Err(invalid("deployment input differs from capture"));
                    }
                    bytes
                };
                let details = if oversized {
                    vec![crate::deployment_parser::DeploymentDetail {distribution:None,version:None,field:if role==SourceRole::TaskReceipt {"task_receipt".into()} else {"deployment".into()},original:String::new(),name:None,extras:vec![],marker:None,constraint:None,interpretation:CheckStatus::Failed,diagnostic:Some("deployment input exceeds interpretation byte limit; captured bytes retained".into()),environment_digest:None,lock_digest:None,referenced_path:None}]
                } else if role == SourceRole::TaskReceipt {
                    let parsed = serde_json::from_slice::<Receipt>(&bytes)
                        .map_err(ModelError::codec)
                        .and_then(|receipt| {
                            check_receipt(&receipt, input, library, context.budget())
                                .map(|target| (receipt, target))
                        });
                    match parsed {
                        Ok((receipt, target)) => {
                            let metadata = collection(
                                context,
                                ReportCollectionKind::EnvironmentMetadata,
                                receipt
                                    .environment
                                    .metadata
                                    .iter()
                                    .map(|(name, digest)| ReportValue::Metadata {
                                        name: name.clone(),
                                        digest: *digest,
                                    })
                                    .collect(),
                            )?;
                            let mut values = receipt
                                .command
                                .into_iter()
                                .enumerate()
                                .map(|(ordinal, text)| ReportValue::Command {
                                    ordinal: ordinal as i64,
                                    text,
                                })
                                .collect::<Vec<_>>();
                            values.extend(
                                receipt
                                    .arguments
                                    .into_iter()
                                    .map(|(name, value)| ReportValue::Argument { name, value }),
                            );
                            values.extend(receipt.tools.into_iter().enumerate().map(
                                |(ordinal, name)| ReportValue::Tool {
                                    ordinal: ordinal as i64,
                                    name,
                                },
                            ));
                            let invocation =
                                collection(context, ReportCollectionKind::Invocation, values)?;
                            let e = receipt.environment;
                            let environment = ReportedEnvironment {
                                release: e.release_id,
                                lock_digest: e.lock_digest,
                                environment_digest: e.environment_digest,
                                runtime_digest: e.runtime_digest,
                                interpreter_digest: e.interpreter_digest,
                                python_version: e.python_version,
                                platform: e.platform,
                                requirement: e.requirement,
                                metadata,
                            };
                            let report = TaskReport {
                                format: i64::from(receipt.format),
                                policy: receipt.policy,
                                task: receipt.task,
                                runner_sha256: receipt.runner_sha256,
                                source_path: receipt.source_path,
                                source_sha256: receipt.source_sha256,
                                environment: environment.id(),
                                invocation,
                                tool: receipt.tool,
                                elapsed_ms: Milliseconds(receipt.elapsed_ms),
                                timeout_seconds: i64::from(receipt.timeout_seconds),
                                execution: receipt.execution.typed(),
                                result: receipt.result,
                                diagnostic: receipt.diagnostic,
                            };
                            let observation = TaskReportObservation {
                                qualification: q.id(),
                                receipt: span,
                                target,
                                report: report.id(),
                            };
                            context.emit(TaskReportSupport {
                                assertion: observation.id(),
                                run: run.id(),
                                surface: surface.id(),
                                evidence: evidence.id(),
                                origin: Origin::SourceObservation,
                                mode: ExtractionMode::ReportDecode,
                                fidelity: Fidelity::ReportProjection,
                            })?;
                            context.emit(observation)?;
                            context.emit(environment)?;
                            context.emit(report)?;
                            vec![]
                        }
                        Err(
                            error @ (ModelError::Resource { .. }
                            | ModelError::Limit { .. }
                            | ModelError::Infrastructure { .. }),
                        ) => return Err(error),
                        Err(error) => vec![crate::deployment_parser::DeploymentDetail {
                            distribution: None,
                            version: None,
                            field: "task_receipt".into(),
                            original: String::from_utf8_lossy(&bytes).into_owned(),
                            name: None,
                            extras: vec![],
                            marker: None,
                            constraint: None,
                            interpretation: CheckStatus::Failed,
                            diagnostic: Some(error.to_string().chars().take(1024).collect()),
                            environment_digest: None,
                            lock_digest: None,
                            referenced_path: None,
                        }],
                    }
                } else if role == SourceRole::DistributionMetadata {
                    if artifact.path.ends_with("entry_points.txt") {
                        crate::deployment_parser::entry_points(&bytes)
                    } else {
                        crate::deployment_parser::metadata(&bytes)
                    }
                } else {
                    crate::deployment_parser::configuration(&artifact.path, &bytes)
                };
                for (ordinal, d) in details.into_iter().enumerate() {
                    if d.interpretation != CheckStatus::Passed {
                        input_partial = true;
                        reason = Some(if oversized {
                            ObligationKind::ResourceRefused
                        } else {
                            ObligationKind::OutsideProviderModel
                        });
                    }
                    let row = DeploymentObservation {
                        qualification: q.id(),
                        span,
                        ordinal: ordinal as i64,
                        distribution: d.distribution,
                        version: d.version,
                        field: d.field,
                        original: d.original,
                        name: d.name,
                        extras: d.extras,
                        marker: d.marker,
                        constraint: d.constraint,
                        interpretation: d.interpretation,
                        diagnostic: d.diagnostic,
                        environment_digest: d.environment_digest,
                        lock_digest: d.lock_digest,
                        referenced_path: d.referenced_path,
                    };
                    context.emit(DeploymentSupport {
                        assertion: row.id(),
                        run: run.id(),
                        surface: surface.id(),
                        evidence: evidence.id(),
                        origin: Origin::InputContext,
                        mode: if role == SourceRole::TaskReceipt {
                            ExtractionMode::ReportDecode
                        } else {
                            ExtractionMode::Recognizer
                        },
                        fidelity: if role == SourceRole::TaskReceipt {
                            Fidelity::ReportProjection
                        } else {
                            Fidelity::NormalizedStructural
                        },
                    })?;
                    context.emit(row)?;
                }
            }
            partial |= input_partial;
            context.contribute(ProviderCoverage {scope:scope.id(),provider:Some(run.provider),context:analysis.id(),family:FactFamily::Deployment,run:Some(run.id()),status:if input_partial {CoverageStatus::Partial} else {CoverageStatus::CompleteUnderStatedModel},reason,diagnostic:input_partial.then(||"one or more captured deployment descriptions could not be interpreted under the stated model".into())})?;
        }
        Ok(if partial {
            ProviderOutcome::Partial
        } else {
            ProviderOutcome::Complete
        })
    }
}
