//! The current linked consumer reuses a verified snapshot, not a scratch-only fork API.
use cpg_extract::ruff_context::{CanonicalSyntax, ContextSettings};
use lctx_model::domain::{
    attribution::AnalysisContext,
    resources::ResourceBudget,
    source::{CoverageScope, SourceArtifact},
    *,
};
use ruff_linter::semantic_facts::{Fact, Sink, StopReason};
use ruff_text_size_latest::Ranged;
#[derive(Default)]
struct Rows(Vec<Fact>);
impl Sink for Rows {
    fn observe(&mut self, row: Fact) -> Result<(), StopReason> {
        self.0.push(row);
        Ok(())
    }
}
fn context() -> AnalysisContext {
    AnalysisContext {
        python_version: "3.14.7".into(),
        python_platform: "linux".into(),
        search_path: vec!["release".into()],
        site_package_path: vec![],
        config_digest: ContentHash::of(b"explicit"),
        environment_digest: ContentHash::of(b"frozen"),
        lock_digest: None,
    }
}
fn artifact(text: &str) -> SourceArtifact {
    SourceArtifact::from_bytes(
        serde_json::from_value(serde_json::json!(vec![1; 16])).unwrap(),
        "pkg/api.py".into(),
        text.as_bytes(),
    )
    .unwrap()
}
#[test]
fn populated_context_keeps_alias_resolution_final_tables_and_exact_snapshot() {
    let source = "from typing import TYPE_CHECKING as TC\nfrom typing import final as fin\nif TC:\n    annotation_only = 1\n    annotation_only\n@fin\ndef f():\n    return missing\nfin = 0\nfin\n__all__ = ['f']\n";
    let b = ResourceBudget::fixed(16 << 20).unwrap();
    let canonical = CanonicalSyntax::parse(
        &artifact(source),
        source,
        &context(),
        ContextSettings::default(),
        &b,
    )
    .unwrap();
    let syntax = canonical.prepare(&b).unwrap();
    assert_eq!(syntax.module().body.len(), 7);
    let mut rows = Rows::default();
    let stats = syntax.observe(&mut rows).unwrap();
    assert_eq!(stats.rows, rows.0.len());
    assert!(rows.0.iter().any(|r| matches!(r, Fact::Reference(r)
        if &source[r.range] == "annotation_only" && r.type_checking && !r.typing_only_annotation)));
    assert!(rows.0.iter().any(|r| matches!(r, Fact::Node(n)
        if &source[n.range] == "annotation_only"
        && n.flags & ruff_python_semantic::SemanticModelFlags::TYPE_CHECKING_BLOCK.bits() != 0)));
    assert!(rows.0.iter().any(|r| matches!(r, Fact::Node(n) if n.qualified_name.as_deref() == Some(&["typing".to_owned(), "final".to_owned()]))));
    let imported = rows
        .0
        .iter()
        .find_map(|r| match r {
            Fact::Binding(b) if b.name == "fin" && b.kind == "FromImport" => Some(b.id),
            _ => None,
        })
        .unwrap();
    assert!(
        rows.0
            .iter()
            .any(|r| matches!(r, Fact::Reference(r) if r.binding == imported))
    );
    assert!(rows.0.iter().any(
        |r| matches!(r, Fact::Binding(b) if b.kind == "Assignment" && b.shadowed == Some(imported))
    ));
    assert!(
        rows.0
            .iter()
            .any(|r| matches!(r, Fact::Unresolved { name, .. } if name == "missing"))
    );
    assert!(
        rows.0
            .iter()
            .any(|r| matches!(r, Fact::Binding(b) if b.name == "__all__" && b.kind == "Export"))
    );
    assert!(syntax.module().range().end().to_u32() <= source.len() as u32);
    assert!(
        CanonicalSyntax::parse(
            &artifact(source),
            "different",
            &context(),
            ContextSettings::default(),
            &b
        )
        .is_err()
    );
    drop(syntax);
    drop(canonical);
    assert_eq!(b.reserved(), 0);
}
#[test]
fn cancellation_and_limits_never_certify_complete_context() {
    struct Cancel;
    impl Sink for Cancel {
        fn checkpoint(&mut self) -> Result<(), StopReason> {
            Err(StopReason::Cancelled)
        }
        fn observe(&mut self, _: Fact) -> Result<(), StopReason> {
            panic!("cancelled before emission")
        }
    }
    let source = "x = 1\n";
    let b = ResourceBudget::fixed(1 << 20).unwrap();
    let canonical = CanonicalSyntax::parse(
        &artifact(source),
        source,
        &context(),
        ContextSettings {
            maximum_rows: 1,
            ..Default::default()
        },
        &b,
    )
    .unwrap();
    let syntax = canonical.prepare(&b).unwrap();
    assert_eq!(
        syntax.observe(&mut Cancel).unwrap_err().reason,
        StopReason::Cancelled
    );
    assert_eq!(
        syntax.observe(&mut Rows::default()).unwrap_err().reason,
        StopReason::RowBudget
    );
}

#[test]
fn contextual_attachment_refuses_foreign_source_and_context() {
    use cpg_extract::{
        syntax_records::Spans,
        typed_syntax::{self, SyntaxInvocation, SyntaxLimits},
    };
    use lctx_model::domain::{assertion::*, attribution::*, conditions::Diagram, source::*};
    let source = "x = 1\nx\n";
    let artifact = artifact(source);
    let context = context();
    let budget = ResourceBudget::fixed(16 << 20).unwrap();
    let canonical = CanonicalSyntax::parse(
        &artifact,
        source,
        &context,
        ContextSettings::default(),
        &budget,
    )
    .unwrap();
    let syntax = canonical.prepare(&budget).unwrap();
    let provider = cpg_extract::ruff_context::provider();
    let (run, _) = ProviderRun::new(
        provider.id(),
        context.id(),
        artifact.input,
        context.config_digest,
        [FactFamily::Syntax],
    )
    .unwrap();
    let surface = ProviderSurface {
        provider: provider.id(),
        family: FactFamily::Syntax,
        name: "control".into(),
    };
    let qualification = AssertionQualification {
        assumptions: lctx_model::domain::assumptions::AssumptionSet::empty_id(),
        context: context.id(),
        scope: CoverageScope::Artifact {
            artifact: artifact.id(),
        }
        .id(),
        condition: Diagram::always().id(),
        modality: Modality::Definite,
        approximation: Approximation::Exact,
    };
    let mut spans = Spans::new(&budget);
    typed_syntax::emit(
        syntax.module(),
        source,
        SyntaxInvocation {
            source: &artifact,
            qualification: &qualification,
            run: &run,
            surface: &surface,
        },
        SyntaxLimits::default(),
        |event| spans.insert(&event.occurrence),
    )
    .unwrap();
    let contexts = syntax
        .context_rows(&spans, &qualification, &budget, &|| Ok(()))
        .unwrap();
    assert!(contexts.incomplete.is_none());
    assert_eq!(contexts.unlocated, 0);
    assert!(!contexts.rows.is_empty());
    let checkpoints = std::cell::Cell::new(0usize);
    let stopped = syntax.context_rows(&spans, &qualification, &budget, &|| {
        let count = checkpoints.get() + 1;
        checkpoints.set(count);
        if count >= 3 {
            Err(ModelError::Invalid("provider cancellation control".into()))
        } else {
            Ok(())
        }
    });
    assert!(matches!(stopped, Err(ModelError::Invalid(ref detail))
        if detail == "provider cancellation control"));
    assert!(
        checkpoints.get() >= 3,
        "actual production collector must check during traversal"
    );
    let mut foreign = artifact.clone();
    foreign.path = "foreign.py".into();
    let foreign_canonical = CanonicalSyntax::parse(
        &foreign,
        source,
        &context,
        ContextSettings::default(),
        &budget,
    )
    .unwrap();
    let foreign_syntax = foreign_canonical.prepare(&budget).unwrap();
    assert!(
        foreign_syntax
            .context_rows(&spans, &qualification, &budget, &|| Ok(()))
            .is_err()
    );
    let mut other = qualification.clone();
    other.context = serde_json::from_value(serde_json::json!(vec![2; 16])).unwrap();
    assert!(
        syntax
            .context_rows(&spans, &other, &budget, &|| Ok(()))
            .is_err()
    );
    let occurrence = Occurrence {
        source: foreign.id(),
        start: 0,
        end: 1,
        syntax_kind: SyntaxKind::ExprName,
        role: OccurrenceRole::Read,
        structural_path: vec![999],
    };
    assert!(spans.insert(&occurrence).is_err());
}

#[test]
fn shared_source_preparation_is_explicit_admitted_and_released() {
    let source = "x = 1\nx\n";
    let budget = ResourceBudget::fixed(1 << 20).unwrap();
    let canonical = CanonicalSyntax::parse(
        &artifact(source),
        source,
        &context(),
        ContextSettings::default(),
        &budget,
    )
    .unwrap();
    let parsed_bytes = budget.reserved();
    assert_eq!(
        canonical.module().body.len(),
        2,
        "syntax-only use needs no source preparation"
    );
    let refused = ResourceBudget::fixed(1).unwrap();
    assert!(matches!(
        canonical.prepare(&refused),
        Err(ModelError::Resource { .. })
    ));
    assert_eq!(refused.reserved(), 0);
    assert_eq!(budget.reserved(), parsed_bytes);
    let prepared = canonical.prepare(&budget).unwrap();
    assert!(budget.reserved() > parsed_bytes);
    drop(prepared);
    assert_eq!(budget.reserved(), parsed_bytes);
    drop(canonical);
    assert_eq!(budget.reserved(), 0);
}

#[test]
fn shared_preparation_keeps_both_diagnostic_channels_after_observer_exhaustion() {
    use lctx_model::domain::{assertion::*, attribution::*, conditions::Diagram, diagnostics::*};
    for (source, expected_stop) in [
        (
            "import os  # noqa: F401\nmissing\ndef f():\n    unused = 1\n    return absent\n",
            StopReason::RowBudget,
        ),
        (
            "from os import (\n    path,  # noqa: F401\n    environ,\n)\n",
            StopReason::RowBudget,
        ),
        ("def broken(:\n    missing\n", StopReason::InvalidSyntax),
    ] {
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        let artifact = artifact(source);
        let context = context();
        let canonical = CanonicalSyntax::parse(
            &artifact,
            source,
            &context,
            ContextSettings {
                maximum_rows: 1,
                ..ContextSettings::default()
            },
            &budget,
        )
        .unwrap();
        let prepared = canonical.prepare(&budget).unwrap();
        let qualification = AssertionQualification {
            assumptions: lctx_model::domain::assumptions::AssumptionSet::empty_id(),
            context: context.id(),
            scope: CoverageScope::Artifact {
                artifact: artifact.id(),
            }
            .id(),
            condition: Diagram::always().id(),
            modality: Modality::Definite,
            approximation: Approximation::Exact,
        };
        let before = prepared
            .diagnostics(&artifact, &qualification, &budget, &|| Ok(()))
            .unwrap();
        let stopped = prepared.observe(&mut Rows::default()).unwrap_err();
        assert_eq!(stopped.reason, expected_stop);
        let after = prepared
            .diagnostics(&artifact, &qualification, &budget, &|| Ok(()))
            .unwrap();
        assert_eq!(
            (
                &before.ruff,
                &before.subjects,
                &before.annotations,
                &before.evidence
            ),
            (
                &after.ruff,
                &after.subjects,
                &after.annotations,
                &after.evidence
            )
        );
        if source.starts_with("import os") {
            assert!(
                after
                    .ruff
                    .iter()
                    .any(|row| row.rule == SelectedRuffRule::UnusedImport
                        && row.channel == DiagnosticChannel::RuffNoqaSuppressed)
            );
            assert!(
                after
                    .ruff
                    .iter()
                    .any(|row| row.rule == SelectedRuffRule::UndefinedName
                        && row.message.contains("absent")
                        && row.channel == DiagnosticChannel::Emitted)
            );
            assert!(
                after
                    .ruff
                    .iter()
                    .any(|row| row.rule == SelectedRuffRule::UnusedVariable
                        && row.message.contains("unused")
                        && row.channel == DiagnosticChannel::Emitted)
            );
        }
        drop(after);
        drop(before);
        drop(prepared);
        drop(canonical);
        assert_eq!(budget.reserved(), 0);
    }
}

#[test]
fn diagnostic_cancellation_preserves_cause_and_releases_each_channel_boundary() {
    use lctx_model::domain::{assertion::*, attribution::*, conditions::Diagram};
    let source = "import os  # noqa: F401\nmissing\n";
    let budget = ResourceBudget::fixed(16 << 20).unwrap();
    let artifact = artifact(source);
    let context = context();
    let canonical = CanonicalSyntax::parse(
        &artifact,
        source,
        &context,
        ContextSettings::default(),
        &budget,
    )
    .unwrap();
    let prepared = canonical.prepare(&budget).unwrap();
    let qualification = AssertionQualification {
        assumptions: lctx_model::domain::assumptions::AssumptionSet::empty_id(),
        context: context.id(),
        scope: CoverageScope::Artifact {
            artifact: artifact.id(),
        }
        .id(),
        condition: Diagram::always().id(),
        modality: Modality::Definite,
        approximation: Approximation::Exact,
    };
    let retained = budget.reserved();
    // Before diagnostic preparation, after the emitted channel (so the second
    // traversal must not start), and after the unsuppressed channel. None may
    // return partial Records or retain the native diagnostic allowance.
    for stop_at in [1, 3, 5] {
        let checks = std::cell::Cell::new(0);
        let result = prepared.diagnostics(&artifact, &qualification, &budget, &|| {
            checks.set(checks.get() + 1);
            if checks.get() == stop_at {
                Err(ModelError::Invalid("provider run cancelled".into()))
            } else {
                Ok(())
            }
        });
        assert!(matches!(result, Err(ModelError::Invalid(ref detail))
            if detail == "provider run cancelled"));
        assert_eq!(checks.get(), stop_at, "no work boundary may follow refusal");
        assert_eq!(budget.reserved(), retained);
    }
    drop(prepared);
    drop(canonical);
    assert_eq!(budget.reserved(), 0);
}
