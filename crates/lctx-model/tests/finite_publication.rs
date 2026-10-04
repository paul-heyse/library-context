use lctx_model::{
    DomainCode,
    domain::{analysis, *},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, DomainCode)]
#[repr(i16)]
#[model(inventory)]
enum SparseCode {
    #[model(wire = "old-wire", label = "first label")]
    First = 0,
    #[model(wire = "appended-wire", label = "second label")]
    Appended = 7,
}
#[test]
fn explicit_finite_definition_owns_inventory_labels_schema_and_codes() {
    assert_eq!(SparseCode::ALL, [SparseCode::First, SparseCode::Appended]);
    assert_eq!(
        SparseCode::codes(),
        &[(0, "old-wire"), (7, "appended-wire")]
    );
    assert_eq!(SparseCode::First.label(), "first label");
    assert_eq!(SparseCode::Appended.label(), "second label");
    assert_eq!(SparseCode::Appended.code(), 7);
    assert_eq!(SparseCode::from_code(7), Some(SparseCode::Appended));
    assert_eq!(SparseCode::from_code(1), None);
    assert_eq!(serde_json::to_string(&SparseCode::Appended).unwrap(), "7");
    assert_eq!(
        serde_json::from_str::<SparseCode>("7").unwrap(),
        SparseCode::Appended
    );
    assert!(serde_json::from_str::<SparseCode>("1").is_err());
}
#[test]
fn common_publication_includes_independently_named_records_for_every_real_owner() {
    macro_rules! check {
        ($($owner:ident),*) => {$( {
            let names = analysis::$owner::publication_relations().iter().map(Relation::name).collect::<std::collections::BTreeSet<_>>();
            assert_eq!(names.len(), 11);
            assert!(names.contains(analysis::$owner::AnalysisInvocation::NAME));
            assert!(names.contains(analysis::$owner::AnalysisOutcome::NAME));
            assert!(names.contains(analysis::$owner::InvocationSource::NAME));
            assert!(names.contains(analysis::$owner::SourceReceipt::NAME));
            assert!(names.contains(analysis::$owner::CoverageRequiredSource::NAME));
            assert!(names.contains(analysis::$owner::AnalysisCoveragePremise::NAME));
            assert!(!names.contains(analysis::$owner::AnalysisDiagnostic::NAME));
            assert!(!names.contains(analysis::$owner::ObligationSource::NAME));
        } )*};
    }
    check!(
        local,
        base_evaluation,
        base_completion,
        source_call,
        enriched_execution,
        model,
        summary,
        structural,
        analytic_embedding,
        analytic,
        catalog_core,
        catalog_evidence,
        selection,
        synthesis,
        retrieval
    );
}
