//! Actual selected native diagnostics and definition answers reach the original evidence owner.
use crate::typed_driver;
use crate::inspector;
use lctx_model::domain::{
    analysis::native::NativeInventory,
    assertion::*,
    attribution::*,
    catalog::evidence::{SourceCharacterization, build::*},
    diagnostics::*,
    resources::ResourceBudget,
    source::*,
    *,
};
use typed_driver::{files, rows};
// Evidence reconstruction includes the independent complete native premise inventory.
inspector!(Facts, complete);
async fn fixture() -> typed_driver::Tables {
    let tables = typed_driver::Tables::default();
    typed_driver::run(&files("native_diagnostics"), Facts(tables.clone()))
        .await
        .unwrap();
    tables
}
fn prepare(
    tables: &typed_driver::Tables,
    omit: Option<&str>,
    budget: &ResourceBudget,
) -> EvidenceData {
    let mut data = EvidenceData::new(budget);
    for (name, batch) in tables.lock().unwrap().iter() {
        if omit != Some(*name) {
            data.visit(name, batch).unwrap();
        }
    }
    let mut native = NativeInventory::new(budget);
    for input in NativeInventory::inputs() {
        if let Some(batch) = tables.lock().unwrap().get(input.name()) {
            native.visit(input.name(), batch).unwrap();
        }
    }
    let native = native.collect().unwrap();
    for p in native.premises.iter() {
        data.facts
            .characterization_native
            .insert(p.clone())
            .unwrap();
    }
    data
}
#[tokio::test]
async fn selected_rules_preserve_actual_suppression_native_errors_and_pytest_answers() {
    let tables = fixture().await;
    let ruff = rows::<RuffDiagnosticObservation>(&tables);
    let native = rows::<PyreflyDiagnosticObservation>(&tables);
    let definitions = rows::<NativeParameterDefinitionObservation>(&tables);
    for rule in [
        SelectedRuffRule::UndefinedName,
        SelectedRuffRule::UnusedImport,
        SelectedRuffRule::UnusedVariable,
    ] {
        assert!(
            ruff.iter()
                .any(|r| r.rule == rule && r.channel == DiagnosticChannel::Emitted),
            "{rule:?}"
        );
    }
    assert!(
        ruff.iter()
            .any(|r| r.rule == SelectedRuffRule::UndefinedName
                && r.channel == DiagnosticChannel::RuffNoqaSuppressed
                && r.message.contains("suppressed_name"))
    );
    assert!(
        native
            .iter()
            .any(|r| r.category == "bad-return" && r.channel == DiagnosticChannel::Emitted)
    );
    assert!(
        native.iter().any(
            |r| r.category == "bad-return" && r.channel == DiagnosticChannel::PyreflySuppressed
        )
    );
    assert!(
        native
            .iter()
            .all(|r| r.baseline == NativeBaselineStatus::NotConfigured)
    );
    let ruff_missing = ruff
        .iter()
        .find(|r| r.message.contains("missing_name") && r.channel == DiagnosticChannel::Emitted)
        .unwrap();
    let captured = rows::<Evidence>(&tables)
        .into_iter()
        .find(|e| e.id() == ruff_missing.primary.unwrap().id())
        .unwrap();
    let Evidence::SourceSpan { start, end, .. } = captured else {
        unreachable!()
    };
    assert_eq!(
        &files("native_diagnostics")["cases.py"][start as usize..end as usize],
        b"missing_name",
        "native UTF8 offsets retain captured source correspondence"
    );
    assert!(
        definitions
            .iter()
            .any(|r| r.role == NativeParameterRole::Fixture
                && r.answer == DefinitionAnswer::Known
                && r.target_name.as_deref() == Some("resource")
                && r.target_location == DiagnosticLocation::Available)
    );
    assert!(definitions.iter().any(|r|r.role==NativeParameterRole::Ordinary&&r.answer==DefinitionAnswer::Known));
    assert!(
        definitions
            .iter()
            .any(|r| r.target_name.as_deref() == Some("no_matching_fixture")
                && r.role == NativeParameterRole::Ordinary
                && r.answer == DefinitionAnswer::Known),
        "no retained fixture uses the native parameter fallback, not an invented fixture absence result"
    );
    let retained = definitions
        .iter()
        .find(|r| {
            r.role == NativeParameterRole::Fixture && r.target_name.as_deref() == Some("ambiguous")
        })
        .unwrap();
    assert_eq!(retained.answer, DefinitionAnswer::Known);
    assert_eq!(retained.answer_count, 1);
    let target = rows::<Evidence>(&tables)
        .into_iter()
        .find(|e| e.id() == retained.target.unwrap().id())
        .unwrap();
    let expected = files("native_diagnostics")["cases.py"]
        .windows(b"def ambiguous() -> str".len())
        .position(|w| w == b"def ambiguous() -> str")
        .unwrap()
        + 4;
    assert!(
        matches!(target,Evidence::SourceSpan{start,..} if start==expected as i64),
        "native duplicate-fixture metadata retains the last exact definition"
    );
    assert!(
        definitions
            .iter()
            .any(|r| r.answer == DefinitionAnswer::Unknown
                && r.reason == Some(obligation::ObligationKind::SyntaxError))
    );
    let artifacts = rows::<SourceArtifact>(&tables);
    let evidence = rows::<Evidence>(&tables);
    let q = rows::<AssertionQualification>(&tables);
    let runs = rows::<ProviderRun>(&tables);
    for row in &ruff {
        let q = q.iter().find(|q| q.id() == row.qualification).unwrap();
        let artifact = artifacts.iter().find(|a| a.id() == row.artifact).unwrap();
        let support = rows::<RuffDiagnosticSupport>(&tables)
            .into_iter()
            .find(|s| s.assertion == row.id())
            .unwrap();
        let run = runs.iter().find(|r| r.id() == support.run).unwrap();
        assert_eq!(run.context, q.context);
        assert_eq!(run.input, artifact.input);
        if let Some(span) = row.primary {
            assert!(evidence.iter().any(|e|e.id()==span.id()&&matches!(e,Evidence::SourceSpan{source,start,end} if *source==artifact.id()&&*start>=0&&*end<=artifact.byte_len)));
        }
    }
}
#[tokio::test]
async fn first_catalog_consumer_retains_proof_links_and_refuses_missing_native_support() {
    let tables = fixture().await;
    let budget = ResourceBudget::fixed(512 << 20).unwrap();
    let data = prepare(&tables, None, &budget);
    let out = build(&data, &budget).unwrap();
    assert!(!out.source_characterizations.is_empty());
    assert!(
        out.source_characterizations.iter().all(|r| data
            .facts
            .characterization_native
            .get(r.native)
            .is_some())
    );
    for omit in [
        RuffDiagnosticSupport::NAME,
        PyreflyDiagnosticSupport::NAME,
        NativeParameterDefinitionSupport::NAME,
    ] {
        let data = prepare(&tables, Some(omit), &budget);
        let declined = build(&data, &budget).unwrap();
        assert!(
            declined.source_characterizations.len() < out.source_characterizations.len(),
            "{omit}"
        );
    }
    let mut malformed = prepare(&tables, None, &budget);
    let parent = malformed
        .facts
        .ruff_diagnostics
        .iter()
        .next()
        .unwrap()
        .clone();
    let foreign = malformed
        .facts
        .canonical_evidence
        .iter()
        .find_map(|e| match e {
            Evidence::SourceSpan { source, .. } if *source != parent.artifact => {
                EvidenceSourceSpanId::of(e).ok()
            }
            _ => None,
        })
        .expect("fixture retains another captured artifact span");
    let subject = DiagnosticSubject::Ruff {
        observation: parent.id(),
    };
    malformed
        .facts
        .diagnostic_annotations
        .insert(DiagnosticAnnotation {
            diagnostic: subject.id(),
            ordinal: 999,
            span: Some(foreign),
            location: DiagnosticLocation::Available,
            label: Some("invalid foreign attribution".into()),
        })
        .unwrap();
    assert!(
        build(&malformed, &budget).is_err(),
        "shared source-characterization validator refuses a foreign parent annotation"
    );
    let invariant = invariants().remove(0);
    for omit in [None, Some(SourceCharacterization::NAME)] {
        let mut check = (invariant.create)(&budget);
        for input in EvidenceData::inputs() {
            if let Some(batch) = tables.lock().unwrap().get(input.name()) {
                check.visit(input.name(), batch).unwrap();
            }
        }
        let batch = <analysis::native::NativeAssertionPremise as Record>::encode(
            &data
                .facts
                .characterization_native
                .iter()
                .cloned()
                .collect::<Vec<_>>(),
        )
        .unwrap();
        check
            .visit(analysis::native::NativeAssertionPremise::NAME, &batch)
            .unwrap();
        macro_rules! outputs {($($field:ident:$ty:ty,)*)=>{$(if omit!=Some(<$ty>::NAME){check.visit(<$ty>::NAME,&<$ty as Record>::encode(&out.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();})*};}
        lctx_model::catalog_evidence_outputs!(outputs);
        assert_eq!(check.finish().is_ok(), omit.is_none());
    }
}
