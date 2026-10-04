//! F01: actual native raise inventory, disposable PG18 publication, and selection journeys.
#[path = "fixtures/serving_support.rs"]
mod support;

use lctx_model::domain::{
    assertion::AssertionQualification,
    attribution::{AnalysisContext, CoverageStatus, FactFamily, ProviderCoverage},
    calls::{ProviderModule, ProviderSymbol, SignatureRole},
    selection::*,
    serving::*,
    source::{CoverageScope, Occurrence, SourceArtifact, SyntaxKind},
    types::{
        TypeObservation, TypeQueryObservation, TypeQueryStatus, TypeRole, TypeSupport, TypeTerm,
    },
    *,
};
use lctx_postgres::generations::RequestExecution;
use support::{ServingFixture, path};

const MIXED: &[u8] = br#"__all__ = ['api']
def api() -> None:
    if False:
        raise TypeError()
    raise ValueError()
"#;
const COMPLETE: &[u8] = br#"__all__ = ['api']
def api() -> None:
    raise ValueError()
"#;

fn raised(name: &str, quantifier: Quantifier) -> SelectionInput {
    SelectionInput(Selection {
        requirements: vec![Requirement {
            predicate: Predicate::FacetMembership {
                facet: Facet::Raises,
                value: FacetValue::RaisedClass {
                    r#type: StructuralType::NominalIdentity {
                        module: "builtins".into(),
                        name: name.into(),
                    },
                },
            },
            quantifier,
        }],
        mode: Mode::Discovery,
        joint: JointPolicy::IndependentRecords,
    })
}

// These are compiler outputs, not facts manufactured by the test. If the pinned analyzer's
// pruning/trace behavior changes, diagnose that prerequisite instead of weakening selection.
async fn assert_native_inventory(
    execution: &RequestExecution,
    source: &[u8],
    analysis: Id<AnalysisContext>,
    missing_raise: bool,
) -> Vec<ProviderCoverage> {
    let artifacts = execution.read::<SourceArtifact>().await.unwrap();
    let artifact = artifacts
        .rows()
        .iter()
        .find(|a| a.content == ContentHash::of(source))
        .expect("fixture source was captured")
        .id();
    let occurrences = execution.read::<Occurrence>().await.unwrap();
    let qualifications = execution.read::<AssertionQualification>().await.unwrap();
    let queries = execution.read::<TypeQueryObservation>().await.unwrap();
    let observations = execution.read::<TypeObservation>().await.unwrap();
    let supports = execution.read::<TypeSupport>().await.unwrap();
    let terms = execution.read::<TypeTerm>().await.unwrap();
    let symbols = execution.read::<ProviderSymbol>().await.unwrap();
    let modules = execution.read::<ProviderModule>().await.unwrap();
    let raised = queries
        .rows()
        .iter()
        .filter(|q| {
            q.role == TypeRole::Raised
                && !q.declared
                && qualifications
                    .rows()
                    .iter()
                    .any(|a| a.id() == q.qualification && a.context == analysis)
                && occurrences
                    .rows()
                    .iter()
                    .any(|o| o.id() == q.subject && o.source == artifact)
        })
        .collect::<Vec<_>>();
    assert_eq!(
        raised.len(),
        if missing_raise { 2 } else { 1 },
        "native Raised query inventory changed; queries={raised:?}"
    );
    let query_at = |text: &[u8]| {
        raised
            .iter()
            .copied()
            .find(|q| {
                occurrences.rows().iter().any(|o| {
                    o.id() == q.subject
                        && o.syntax_kind == SyntaxKind::StmtRaise
                        && &source[o.start as usize..o.end as usize] == text
                })
            })
            .unwrap_or_else(|| panic!("missing native Raised query at {text:?}: {raised:?}"))
    };
    let available = query_at(b"raise ValueError()");
    assert_eq!(
        available.status,
        TypeQueryStatus::Available,
        "fixture requires a supported native ValueError trace: {available:?}"
    );
    let observed = observations
        .rows()
        .iter()
        .find(|o| Some(o.id()) == available.observation)
        .expect("available query retains its actual TypeObservation");
    assert_eq!(
        (observed.subject, observed.role, observed.qualification),
        (available.subject, TypeRole::Raised, available.qualification)
    );
    assert!(
        supports.rows().iter().any(|s| s.assertion == observed.id()),
        "native raised TypeObservation must have persisted support"
    );
    let term = terms
        .rows()
        .iter()
        .find(|t| t.id() == observed.term)
        .unwrap();
    let TypeTerm::ClassInstance { class, .. } = term else {
        panic!("fixture requires an actual native class instance, observed {term:?}");
    };
    let symbol = symbols.rows().iter().find(|s| s.id() == *class).unwrap();
    let module = modules
        .rows()
        .iter()
        .find(|m| m.id() == symbol.module)
        .unwrap();
    assert_eq!(symbol.name, "ValueError");
    assert!(
        matches!(module, ProviderModule::Bundled { name, .. } if name == "builtins"),
        "native class identity must retain its actual bundled builtins module: {module:?}"
    );
    if missing_raise {
        let unavailable = query_at(b"raise TypeError()");
        assert_eq!(
            unavailable.status,
            TypeQueryStatus::Unavailable,
            "if False pruning is a source-backed fixture hypothesis; changed native output must be investigated: {unavailable:?}"
        );
        assert_eq!(unavailable.observation, None);
        assert_eq!(
            unavailable.reason,
            Some(obligation::ObligationKind::MissingEvidence)
        );
        assert!(
            !observations
                .rows()
                .iter()
                .any(|o| o.subject == unavailable.subject
                    && o.role == TypeRole::Raised
                    && o.qualification == unavailable.qualification),
            "unavailable raise must not acquire a fabricated TypeObservation"
        );
    }
    let coverage = execution.read::<ProviderCoverage>().await.unwrap();
    let scope = CoverageScope::Artifact { artifact }.id();
    let native = coverage
        .rows()
        .iter()
        .filter(|c| c.scope == scope && c.context == analysis && c.family == FactFamily::Types)
        .cloned()
        .collect::<Vec<_>>();
    assert!(
        !native.is_empty(),
        "actual artifact/context Types coverage is required"
    );
    let expected = if missing_raise {
        CoverageStatus::Partial
    } else {
        CoverageStatus::CompleteUnderStatedModel
    };
    assert!(
        native.iter().all(|c| c.status == expected),
        "fixture requires {expected:?} native Types coverage; actual={native:?}; queries={raised:?}"
    );
    native
}

async fn selection_journey(
    fixture: &ServingFixture,
    execution: &RequestExecution,
    name: &str,
    quantifier: Quantifier,
    expected: Outcome,
) -> RequirementResult {
    let selection = raised(name, quantifier);
    let compared = fixture
        .catalog
        .compare(
            execution,
            &CompareOperationsRequest {
                library: Name::new("demo").unwrap(),
                operations: vec![path("demo.api")],
                selection: selection.clone(),
                page: PageRequest {
                    expanded: true,
                    ..Default::default()
                },
            },
        )
        .await
        .unwrap();
    let candidates = &compared.operations[0].candidates;
    assert!(!candidates.is_empty(), "actual api must resolve");
    assert!(
        candidates
            .iter()
            .all(|c| c.requirements[0].outcome == expected),
        "raised {name} {quantifier:?}: expected {expected:?}, actual={candidates:?}"
    );
    let found = fixture
        .catalog
        .find(
            execution,
            &FindOperationsRequest {
                library: Name::new("demo").unwrap(),
                selection,
                page: PageRequest {
                    size: 100,
                    ..Default::default()
                },
            },
        )
        .await
        .unwrap();
    for (outcome, group) in [
        (Outcome::Supported, &found.supported.items),
        (Outcome::Unresolved, &found.unresolved.items),
        (Outcome::Conflicting, &found.conflicting.items),
    ] {
        let api = group
            .iter()
            .filter(|c| c.name.as_str() == "demo.api")
            .collect::<Vec<_>>();
        assert_eq!(
            !api.is_empty(),
            expected == outcome,
            "discovery must place api only in its {expected:?} group: {found:?}"
        );
        for candidate in api {
            assert_eq!(candidate.requirements[0].outcome, expected);
            assert!(
                candidates.iter().any(|c| c == candidate),
                "find and compare must preserve the same native evidence"
            );
        }
    }
    candidates[0].requirements[0].clone()
}

#[tokio::test]
async fn incomplete_native_raise_inventory_keeps_nonmatch_unresolved() {
    let fixture = ServingFixture::start(MIXED).await;
    let execution = fixture.service.execution().await.unwrap();
    let operation = fixture
        .catalog
        .operation(
            &execution,
            &GetOperationRequest {
                library: Name::new("demo").unwrap(),
                operation: path("demo.api"),
                comparison: Optional::default(),
                reference_parameter: Optional::default(),
                sections: vec![],
                page: PageRequest {
                    expanded: true,
                    ..Default::default()
                },
            },
        )
        .await
        .unwrap();
    let OperationResolution::Unique { packet } = operation.operation else {
        panic!("fixture api must resolve uniquely");
    };
    let source = packet
        .core
        .signatures
        .iter()
        .find(|s| s.role == SignatureRole::Source)
        .expect("actual source signature");
    assert!(
        source.complete,
        "complete signatures must not stand in for missing native Types coverage"
    );
    assert_native_inventory(&execution, MIXED, source.analysis, true).await;
    let nonmatch = selection_journey(
        &fixture,
        &execution,
        "TypeError",
        Quantifier::AllApplicable,
        Outcome::Unresolved,
    )
    .await;
    assert!(
        nonmatch.negative.is_empty(),
        "one retained mismatching raise cannot establish a counterexample"
    );
    let matching = selection_journey(
        &fixture,
        &execution,
        "ValueError",
        Quantifier::AnyApplicable,
        Outcome::Supported,
    )
    .await;
    assert!(
        !matching.positive.is_empty(),
        "actual matching native raise remains a positive witness"
    );
    let ids = matching.positive.clone();
    let witnesses = execution
        .query(move |lease| Box::pin(async move { lease.read_ids::<Witness>(&ids).await }))
        .await
        .unwrap();
    assert_eq!(witnesses.rows().len(), matching.positive.len());
    assert!(
        witnesses
            .rows()
            .iter()
            .any(|w| matches!(w, Witness::RaisedType { .. })),
        "matching native raise support must survive publication and serving"
    );
    drop(execution);
    fixture.finish().await;
}

#[tokio::test]
async fn complete_native_raise_inventory_certifies_a_persisted_negative() {
    let fixture = ServingFixture::start(COMPLETE).await;
    let execution = fixture.service.execution().await.unwrap();
    let member = fixture
        .members()
        .await
        .into_iter()
        .find(|m| m.name.as_str() == "demo.api")
        .unwrap();
    let coverage = assert_native_inventory(&execution, COMPLETE, member.analysis, false).await;
    let nonmatch = selection_journey(
        &fixture,
        &execution,
        "TypeError",
        Quantifier::AllApplicable,
        Outcome::Contradicted,
    )
    .await;
    assert!(
        !nonmatch.negative.is_empty(),
        "complete native inventory must retain its negative evidence"
    );
    let ids = nonmatch.negative.clone();
    let witnesses = execution
        .query(move |lease| Box::pin(async move { lease.read_ids::<Witness>(&ids).await }))
        .await
        .unwrap();
    assert_eq!(
        witnesses.rows().len(),
        nonmatch.negative.len(),
        "all negative witnesses are persisted"
    );
    assert!(
        witnesses
            .rows()
            .iter()
            .any(|w| matches!(w, Witness::RaisedType { .. }))
    );
    let closures = witnesses
        .rows()
        .iter()
        .filter_map(|w| match w {
            Witness::NativeTypingCoverage { coverage } => Some(*coverage),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        !closures.is_empty(),
        "negative answer must carry native Types closure, not just signature completeness"
    );
    assert!(
        closures
            .iter()
            .all(|id| coverage.iter().any(|c| c.id() == *id)),
        "native closure must reference this candidate's actual artifact and exact analysis context"
    );
    let persisted = execution
        .query(move |lease| {
            Box::pin(async move { lease.read_ids::<ProviderCoverage>(&closures).await })
        })
        .await
        .unwrap();
    assert!(!persisted.rows().is_empty());
    assert!(
        persisted
            .rows()
            .iter()
            .all(|c| c.status == CoverageStatus::CompleteUnderStatedModel
                && c.context == member.analysis
                && c.family == FactFamily::Types)
    );
    drop(execution);
    fixture.finish().await;
}
