//! Semantic compiler owners consume completed native and normalized streams.
#[path = "fixtures/catalog_runtime.rs"]
mod catalog_runtime;
use lctx_model::domain::{admission::Frontier, stages::Profile, *};
async fn run(profile: Profile) {
    let root = catalog_runtime::root("local_semantics");
    let fixture = catalog_runtime::compile("local_semantics",profile,Frontier::Analysis,catalog_runtime::settings("cases"),None).await;
    let counts:(i64,i64,i64)=catalog_runtime::one(&fixture, "SELECT (SELECT count(*) FROM local_flow_contributions),(SELECT count(*) FROM local_transfer_alternatives),(SELECT count(*) FROM local_flow_assessments WHERE reason IS NOT NULL)").await;
    if profile == Profile::Behavioral {
        assert!(counts.0 > 0);
        assert!(counts.1 >= counts.0);
        assert!(counts.2 > 0);
    } else {
        assert_eq!(counts, (0, 0, 0));
        let statuses: Vec<i16> = catalog_runtime::query(&fixture, "SELECT status FROM local_analysis_outcomes").await;
        assert!(!statuses.is_empty());
        assert!(statuses.iter().all(|s| *s == 3));
        let availabilities: Vec<i16> = catalog_runtime::query(&fixture, "SELECT availability FROM local_analysis_coverage").await;
        assert!(!availabilities.is_empty());
        assert!(
            availabilities
                .iter()
                .all(|s| *s == normalized::coverage::EvidenceAvailability::NotRequested.code())
        );
    }
    let theory:(i64,i64,i64)=catalog_runtime::one(&fixture, "SELECT (SELECT count(*) FROM local_type_domains),(SELECT count(*) FROM local_theory_witnesses),(SELECT count(*) FROM local_type_class_members)").await;
    if profile == Profile::Behavioral {
        assert!(theory.0 > 0 && theory.1 > 0 && theory.2 >= 4);
        let decisions: Vec<(i64, i16)> = catalog_runtime::query(&fixture, "SELECT occurrence.start, decision.outcome FROM local_atom_decisions decision JOIN flow_test_leaf_observations leaf ON leaf.id=decision.leaf JOIN occurrences occurrence ON occurrence.id=leaf.test").await;
        let text = std::fs::read_to_string(root.join("cases.py")).unwrap();
        for (function, expected) in [
            ("finite_zero", 1_i16),
            ("finite_one", 0),
            ("uninhabited", 3),
        ] {
            let start = text.find(&format!("def {function}(")).unwrap();
            let test = start + text[start..].find("if value:").unwrap() + 3;
            assert!(
                decisions.contains(&(test as i64, expected)),
                "missing independent {function} decision at {test}: {decisions:?}"
            );
        }
        let refinements: i64 = catalog_runtime::one(&fixture, "SELECT count(*) FROM local_atom_restrictions r JOIN local_transfer_alternatives original ON original.id=r.original JOIN assertion_qualifications oldq ON oldq.id=original.qualification JOIN assertion_qualifications newq ON newq.id=r.qualification JOIN assumption_sets oldbasis ON oldbasis.id=oldq.assumptions JOIN assumption_sets newbasis ON newbasis.id=newq.assumptions WHERE oldbasis.count=0 AND newbasis.count=1 AND oldq.condition<>newq.condition").await;
        assert!(
            refinements >= 2,
            "true and false refinements must change actual transfers while preserving originals"
        );
        // Independent branch expectations: both a predicate and its negation are restricted.
        // The leaf's provider formula is not an ambient guard that excludes the negative arm.
        let constants: Vec<(i64, i16)> = catalog_runtime::query(&fixture, "SELECT occurrence.start, node.kind FROM local_atom_restrictions r JOIN local_atom_decisions decision ON decision.id=r.decision JOIN flow_test_leaf_observations leaf ON leaf.id=decision.leaf JOIN occurrences occurrence ON occurrence.id=leaf.test JOIN assertion_qualifications q ON q.id=r.qualification JOIN conditions c ON c.id=q.condition JOIN condition_nodes node ON node.id=c.root").await;
        for function in ["finite_zero", "finite_one"] {
            let start = text.find(&format!("def {function}(")).unwrap();
            let test = (start + text[start..].find("if value:").unwrap() + 3) as i64;
            assert!(
                constants.contains(&(test, 0)) && constants.contains(&(test, 1)),
                "both polarities need independent conditional restrictions for {function}: {constants:?}"
            );
        }
        let empty_refinements: i64 = catalog_runtime::one(&fixture, "SELECT count(*) FROM local_atom_restrictions r JOIN local_atom_decisions d ON d.id=r.decision WHERE d.outcome=3").await;
        assert_eq!(empty_refinements, 0, "Never must not certify either branch");
    } else {
        assert_eq!(theory, (0, 0, 0));
    }
    let complete: i64 = catalog_runtime::one(&fixture, "SELECT count(*) FROM local_analysis_coverage WHERE availability=0").await;
    assert_eq!(complete, 0);
}
#[tokio::test]
async fn actual_local_transfers_publish_from_native_completed_sources() {
    run(Profile::Behavioral).await;
}

#[tokio::test]
async fn catalog_local_publishes_not_requested_without_semantic_rows() {
    run(Profile::Catalog).await;
}

#[test]
fn catalog_local_keeps_expected_metadata_out_of_unrequested_domain_state() {
    use lctx_model::domain::{
        attribution::*, calls::*, local_semantics::LocalData, normalized::entities::ClassEntity,
        resources::ResourceBudget, source::*, value::Literal,
    };
    let input = input::InputRevision {
        manifest: ContentHash::of(b"profile retention control"),
    };
    let artifact = SourceArtifact::from_bytes(input.id(), "control.py".into(), b"pass\n").unwrap();
    let scope = CoverageScope::Artifact {
        artifact: artifact.id(),
    };
    let context = AnalysisContext {
        python_version: "3.14".into(),
        python_platform: "test".into(),
        search_path: vec![],
        site_package_path: vec![],
        config_digest: ContentHash::of(b"config"),
        environment_digest: ContentHash::of(b"env"),
        lock_digest: None,
    };
    let coverage = ProviderCoverage {
        scope: scope.id(),
        provider: None,
        context: context.id(),
        family: FactFamily::Flow,
        run: None,
        status: CoverageStatus::NotRequested,
        reason: None,
        diagnostic: None,
    };
    let batches = [
        (
            SourceArtifact::NAME,
            SourceArtifact::encode(&[artifact]).unwrap(),
        ),
        (
            CoverageScope::NAME,
            <CoverageScope as Record>::encode(&[scope]).unwrap(),
        ),
        (
            ProviderCoverage::NAME,
            ProviderCoverage::encode(&[coverage]).unwrap(),
        ),
    ];
    let small = ResourceBudget::fixed(1).unwrap();
    let mut catalog = LocalData::new(&small);
    for (name, batch) in &batches {
        assert!(
            !catalog
                .visit_consumed(Profile::Catalog, name, batch)
                .unwrap()
        );
    }
    assert!(
        catalog.entry.artifacts.is_empty()
            && catalog.entry.scopes.is_empty()
            && catalog.entry.coverage.is_empty()
    );
    assert_eq!(
        small.reserved(),
        0,
        "Catalog does not reserve a second copy of expected-only metadata"
    );
    let budget = ResourceBudget::fixed(1 << 20).unwrap();
    let mut behavioral = LocalData::new(&budget);
    for (name, batch) in &batches {
        assert!(
            behavioral
                .visit_consumed(Profile::Behavioral, name, batch)
                .unwrap()
        );
    }
    assert_eq!(
        (
            behavioral.entry.artifacts.len(),
            behavioral.entry.scopes.len(),
            behavioral.entry.coverage.len()
        ),
        (1, 1, 1)
    );
    let provider = Provider {
        tool: "test".into(),
        revision: "1".into(),
        build_digest: ContentHash::of(b"build"),
    };
    let module = ProviderModule::Bundled {
        provider: provider.id(),
        bundle: ModuleBundle::Typeshed,
        name: "test".into(),
    };
    let symbol = ProviderSymbol {
        provider: provider.id(),
        context: context.id(),
        module: module.id(),
        native_key: "C".into(),
        name: "C".into(),
        kind: SymbolKind::Class,
    };
    let class = ClassEntity::Synthetic {
        symbol: symbol.id(),
    };
    assert!(
        behavioral
            .visit_consumed(
                Profile::Behavioral,
                Literal::NAME,
                &<Literal as Record>::encode(&[Literal::None]).unwrap()
            )
            .unwrap()
    );
    assert!(
        behavioral
            .visit_consumed(
                Profile::Behavioral,
                ClassEntity::NAME,
                &<ClassEntity as Record>::encode(&[class]).unwrap()
            )
            .unwrap()
    );
    assert_eq!(
        (
            behavioral.theory.literals.len(),
            behavioral.fields.class_entities.len()
        ),
        (1, 1),
        "Behavioral visitation reaches both independent inventories"
    );
    let mut catalog = LocalData::new(&budget);
    assert!(
        catalog
            .visit_consumed(
                Profile::Catalog,
                Provider::NAME,
                &Provider::encode(&[provider]).unwrap()
            )
            .unwrap()
    );
    assert_eq!(
        catalog.entry.providers.len(),
        1,
        "Catalog still retains its declared provider metadata"
    );
}

