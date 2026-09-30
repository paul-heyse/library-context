use arrow_array::RecordBatch;
use lctx_model::domain::{
    artifact::*, assertion::*, attribution::*, conditions::*, deployment::*, input::*, source::*, *,
};
use std::collections::BTreeMap;
pub struct Fixture {
    pub model: ValidatedModel,
    pub batches: BTreeMap<&'static str, RecordBatch>,
    pub report: TaskReport,
    pub observation: TaskReportObservation,
    pub support: TaskReportSupport,
}
impl Fixture {
    pub fn new(foreign: bool) -> Self {
        let model = model().unwrap();
        let receipt_bytes =
            include_bytes!("../../../../fixtures/python/semantic_deployment/receipt.json");
        let target_bytes =
            include_bytes!("../../../../fixtures/python/semantic_deployment/example.py");
        let input = InputRevision::from_entries(vec![
            ManifestEntry {
                path: "receipt.json".into(),
                content: ContentHash::of(receipt_bytes),
                byte_len: receipt_bytes.len() as i64,
            },
            ManifestEntry {
                path: "example.py".into(),
                content: ContentHash::of(target_bytes),
                byte_len: target_bytes.len() as i64,
            },
        ])
        .unwrap();
        let alien = InputRevision::from_entries(vec![ManifestEntry {
            path: "other.py".into(),
            content: ContentHash::of(b"x"),
            byte_len: 1,
        }])
        .unwrap();
        let receipt =
            SourceArtifact::from_bytes(input.id(), "receipt.json".into(), receipt_bytes).unwrap();
        let target =
            SourceArtifact::from_bytes(input.id(), "example.py".into(), target_bytes).unwrap();
        let other = SourceArtifact::from_bytes(alien.id(), "other.py".into(), b"x").unwrap();
        let origin = InputOrigin::Tree {
            label: "deployment contract".into(),
        };
        let acquisitions = vec![
            InputAcquisition {
                input: input.id(),
                origin: origin.id(),
            },
            InputAcquisition {
                input: alien.id(),
                origin: origin.id(),
            },
        ];
        let package = Package {
            name: "example".into(),
        };
        let release = Release {
            package: package.id(),
            version: "1.0".into(),
        };
        let context = AnalysisContext {
            python_version: "3.14.7".into(),
            python_platform: "linux".into(),
            search_path: vec![],
            site_package_path: vec![],
            config_digest: ContentHash::of(b"cfg"),
            environment_digest: input.manifest,
            lock_digest: None,
        };
        let provider = Provider {
            tool: "report-contract".into(),
            revision: "1".into(),
            build_digest: ContentHash::of(b"report"),
        };
        let (run, families) = ProviderRun::new(
            provider.id(),
            context.id(),
            input.id(),
            context.config_digest,
            [FactFamily::Deployment],
        )
        .unwrap();
        let surface = ProviderSurface {
            provider: provider.id(),
            family: FactFamily::Deployment,
            name: "captured reports".into(),
        };
        let scope = CoverageScope::Input { input: input.id() };
        let (condition, nodes) = Diagram::always().records();
        let q = AssertionQualification {
            context: context.id(),
            scope: scope.id(),
            condition: condition.id(),
            modality: Modality::Definite,
            approximation: Approximation::Exact,
        };
        let evidence = Evidence::SourceSpan {
            source: receipt.id(),
            start: 0,
            end: receipt.byte_len,
        };
        let span = EvidenceSourceSpanId::of(&evidence).unwrap();
        let (metadata, mut values, mut entries) = ReportCollection::new(
            ReportCollectionKind::EnvironmentMetadata,
            vec![ReportValue::Metadata {
                name: "pyproject".into(),
                digest: ContentHash::of(b"metadata"),
            }],
        )
        .unwrap();
        let (invocation, items, members) = ReportCollection::new(
            ReportCollectionKind::Invocation,
            vec![
                ReportValue::Command {
                    ordinal: 0,
                    text: "python".into(),
                },
                ReportValue::Command {
                    ordinal: 1,
                    text: "example.py".into(),
                },
                ReportValue::Argument {
                    name: "count".into(),
                    value: i64::MIN,
                },
                ReportValue::Tool {
                    ordinal: 0,
                    name: "python".into(),
                },
            ],
        )
        .unwrap();
        values.extend(items);
        entries.extend(members);
        let environment = ReportedEnvironment {
            release: release.id(),
            lock_digest: ContentHash::of(b"reported-lock"),
            environment_digest: ContentHash::of(b"reported-environment"),
            runtime_digest: ContentHash::of(b"reported-runtime"),
            interpreter_digest: ContentHash::of(b"reported-interpreter"),
            python_version: "3.13.0".into(),
            platform: "reported-platform".into(),
            requirement: "example==1.0".into(),
            metadata: metadata.id(),
        };
        let report = TaskReport {
            format: 1,
            policy: "capture-only".into(),
            task: "fixture-report".into(),
            runner_sha256: "a".repeat(64),
            source_path: "example.py".into(),
            source_sha256: "b".repeat(64),
            environment: environment.id(),
            invocation: invocation.id(),
            tool: "python".into(),
            elapsed_ms: Milliseconds(u64::MAX),
            timeout_seconds: i64::from(u32::MAX),
            execution: CheckStatus::Passed,
            result: Some("reported result".into()),
            diagnostic: None,
        };
        let observation = TaskReportObservation {
            qualification: q.id(),
            receipt: span,
            target: if foreign { other.id() } else { target.id() },
            report: report.id(),
        };
        let support = TaskReportSupport {
            assertion: observation.id(),
            run: run.id(),
            surface: surface.id(),
            evidence: evidence.id(),
            origin: Origin::SourceObservation,
            mode: ExtractionMode::ReportDecode,
            fidelity: Fidelity::ReportProjection,
        };
        let deployment = DeploymentObservation {
            qualification: q.id(),
            span,
            ordinal: 0,
            distribution: Some("example".into()),
            version: Some("1.0".into()),
            field: "Requires-Dist".into(),
            original: "example[extra]>=1; python_version >= '3.13'".into(),
            name: Some("example".into()),
            extras: vec!["extra".into()],
            marker: Some("python_version >= '3.13'".into()),
            constraint: Some(">=1".into()),
            interpretation: CheckStatus::NotRun,
            diagnostic: None,
            environment_digest: None,
            lock_digest: None,
            referenced_path: Some("../reported-only-path".into()),
        };
        let deployment_support = DeploymentSupport {
            assertion: deployment.id(),
            run: run.id(),
            surface: surface.id(),
            evidence: evidence.id(),
            origin: support.origin,
            mode: support.mode,
            fidelity: support.fidelity,
        };
        let coverage = ProviderCoverage {
            scope: scope.id(),
            provider: Some(provider.id()),
            context: context.id(),
            family: FactFamily::Deployment,
            run: Some(run.id()),
            status: CoverageStatus::CompleteUnderStatedModel,
            reason: None,
            diagnostic: None,
        };
        let mut f = Self {
            model,
            batches: BTreeMap::new(),
            report,
            observation,
            support,
        };
        macro_rules! one { ($($row:expr),+ $(,)?) => { $(f.put(vec![$row.clone()]);)+ }; }
        one!(
            origin,
            package,
            release,
            context,
            provider,
            run,
            surface,
            scope,
            condition,
            q,
            evidence,
            environment,
            f.report,
            f.observation,
            f.support,
            deployment,
            deployment_support,
            coverage
        );
        f.put(vec![input, alien]);
        f.put(acquisitions);
        f.put(families);
        f.put(nodes);
        f.put(vec![metadata, invocation]);
        f.put(values);
        f.put_entries(entries);
        f.put(
            ArtifactChunk::split(&receipt, receipt_bytes)
                .unwrap()
                .chain(ArtifactChunk::split(&target, target_bytes).unwrap())
                .chain(ArtifactChunk::split(&other, b"x").unwrap())
                .collect(),
        );
        f.put(vec![receipt, target, other]);
        f
    }
    pub fn put<R: Record>(&mut self, rows: Vec<R>) {
        self.batches.insert(
            R::NAME,
            Batch::new(&self.model, rows, &budget())
                .unwrap()
                .arrow()
                .clone(),
        );
    }
    pub fn rows<R: Record>(&self) -> Vec<R> {
        self.batches
            .get(R::NAME)
            .map(|b| R::decode(b).unwrap())
            .unwrap_or_default()
    }
    pub fn put_entries(&mut self, mut rows: Vec<ReportEntry>) {
        rows.sort_by_key(|r| (r.collection, r.ordinal));
        self.batches
            .insert(ReportEntry::NAME, ReportEntry::encode(&rows).unwrap());
    }
    pub fn check(&self, invariant: &Invariant) -> Result<(), ModelError> {
        let mut check = (invariant.create)(&budget());
        for input in &invariant.inputs {
            if let Some(batch) = self.batches.get(input.name()) {
                check.visit(input.name(), batch)?;
            }
        }
        check.finish()
    }
}

/// A fresh attempt budget; these controls do not share reservations across batches.
fn budget() -> lctx_model::domain::resources::ResourceBudget {
    lctx_model::domain::resources::ResourceBudget::fixed(
        lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
    )
    .unwrap()
}
