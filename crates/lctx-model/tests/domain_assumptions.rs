#[allow(
    dead_code,
    reason = "Shared native type fixture covers additional contract suites"
)]
#[path = "fixtures/types.rs"]
mod fixture;
use lctx_model::domain::{
    analysis::support::*, assertion::*, assumptions::*, assumptions_universe::*, attribution::*,
    conditions::*, input::*, models::*, types::*, *,
};

fn budget() -> resources::ResourceBudget {
    resources::ResourceBudget::fixed(64 << 20).unwrap()
}
fn second(f: &mut fixture::Fixture) -> Assumption {
    let old = f.base.rows::<TypeObservation>()[0].clone();
    let row = TypeObservation {
        declared: !old.declared,
        ..old
    };
    let support = TypeSupport {
        assertion: row.id(),
        ..f.base.rows::<TypeSupport>()[0].clone()
    };
    let mut rows = f.base.rows::<TypeObservation>();
    rows.push(row.clone());
    f.base.put(rows);
    let mut supports = f.base.rows::<TypeSupport>();
    supports.push(support.clone());
    f.base.put(supports);
    Assumption::TypeConformance {
        observation: row.id(),
        support: support.id(),
    }
}
fn add_basis(
    f: &mut fixture::Fixture,
) -> (Assumption, ResolvedAssumptions, AssertionQualification) {
    let row = f.base.rows::<TypeObservation>()[0].clone();
    let support = f
        .base
        .rows::<TypeSupport>()
        .into_iter()
        .find(|s| s.assertion == row.id())
        .unwrap();
    let definition = Assumption::TypeConformance {
        observation: row.id(),
        support: support.id(),
    };
    let basis = AssumptionSet::new([definition.id()]).unwrap();
    let q = AssertionQualification {
        assumptions: basis.set.id(),
        ..f.base.rows::<AssertionQualification>()[0].clone()
    };
    let mut qs = f.base.rows::<AssertionQualification>();
    qs.push(q.clone());
    f.base.put(qs);
    f.base.put(vec![AssumptionSet::empty(), basis.set.clone()]);
    f.base.put(basis.members.clone());
    f.base.put(vec![definition.clone()]);
    (definition, basis, q)
}
fn index(f: &fixture::Fixture) -> AssumptionIndex {
    let mut index = AssumptionIndex::new(&budget());
    for input in AssumptionIndex::inputs() {
        if let Some(batch) = f.base.batches.get(input.name()) {
            index.visit(input.name(), batch).unwrap();
        }
    }
    index
}
#[test]
fn canonical_sets_are_explicit_sorted_deduplicated_and_bounded() {
    let mut f = fixture::Fixture::new(false);
    let (definition, basis, _) = add_basis(&mut f);
    assert_eq!(
        AssumptionSet::new([]).unwrap(),
        ResolvedAssumptions::empty()
    );
    assert_eq!(
        AssumptionSet::new([definition.id(), definition.id()]).unwrap(),
        basis
    );
    let mut idx = index(&f);
    assert_eq!(
        idx.resolve(AssumptionSet::empty_id()).unwrap(),
        ResolvedAssumptions::empty()
    );
    let missing = AssumptionSet::new([Assumption::TypeConformance {
        observation: f.base.rows::<TypeObservation>()[0].id(),
        support: serde_json::from_value(serde_json::json!(vec![99; 16])).unwrap(),
    }
    .id()])
    .unwrap();
    idx.insert(&missing).unwrap();
    assert!(
        idx.resolve(missing.set.id())
            .unwrap_err()
            .to_string()
            .contains("definition missing")
    );
    let invalid = AssumptionSet {
        count: MAX_ASSUMPTIONS as i64 + 1,
        ..basis.set
    };
    assert!(invalid.validate().is_err());
    let over = (0..=MAX_ASSUMPTIONS).map(|n| {
        let mut id = [0u8; 16];
        id[..8].copy_from_slice(&(n as u64).to_be_bytes());
        serde_json::from_value(serde_json::json!(id)).unwrap()
    });
    assert!(matches!(
        AssumptionSet::new(over),
        Err(ModelError::Limit {
            owner: "claim_assumptions",
            ..
        })
    ));
}
#[test]
fn conjunction_resolves_union_and_missing_distinct_basis_never_becomes_empty() {
    let mut f = fixture::Fixture::new(false);
    let (a, basis, q) = add_basis(&mut f);
    let mut idx = index(&f);
    let other = second(&mut f);
    idx.visit(
        Assumption::NAME,
        &<Assumption as Record>::encode(&[a.clone(), other.clone()]).unwrap(),
    )
    .unwrap();
    let b = AssumptionSet::new([other.id(), a.id()]).unwrap();
    idx.insert(&b).unwrap();
    let condition = Diagram::always();
    let qb = AssertionQualification {
        assumptions: b.set.id(),
        ..q.clone()
    };
    let sources = [a.clone(), other];
    let premises = [
        QualifiedPremise {
            source: &sources[0],
            qualification: &q,
            condition: &condition,
        },
        QualifiedPremise {
            source: &sources[1],
            qualification: &qb,
            condition: &condition,
        },
    ];
    let result = qualify_with_basis(
        QualificationOperation::Conjunction,
        &premises,
        Some(&idx),
        &budget(),
    )
    .unwrap();
    assert_eq!(result.qualification.assumptions, b.set.id());
    let mut expected = sources.iter().map(Record::id).collect::<Vec<_>>();
    expected.sort();
    assert_eq!(
        result
            .assumptions
            .as_ref()
            .unwrap()
            .members
            .iter()
            .map(|m| m.assumption)
            .collect::<Vec<_>>(),
        expected
    );
    assert_eq!(result.assumptions.unwrap(), b);
    assert!(
        qualify(QualificationOperation::Conjunction, &premises, &budget())
            .err()
            .unwrap()
            .to_string()
            .contains("resolved membership")
    );
    assert!(
        qualify(
            QualificationOperation::AlternativeUnion,
            &premises,
            &budget()
        )
        .is_err()
    );
    struct Missing;
    impl AssumptionResolver for Missing {
        fn resolve(&self, _: Id<AssumptionSet>) -> Result<ResolvedAssumptions, ModelError> {
            Ok(ResolvedAssumptions::empty())
        }
    }
    assert!(
        qualify_with_basis(
            QualificationOperation::Conjunction,
            &premises,
            Some(&Missing),
            &budget()
        )
        .is_err()
    );
    let qbad = AssertionQualification {
        context: AnalysisContext {
            python_version: "different".into(),
            ..f.base.rows::<AnalysisContext>()[0].clone()
        }
        .id(),
        ..q.clone()
    };
    let foreign = [
        QualifiedPremise {
            source: &sources[0],
            qualification: &q,
            condition: &condition,
        },
        QualifiedPremise {
            source: &sources[1],
            qualification: &qbad,
            condition: &condition,
        },
    ];
    assert!(
        qualify_with_basis(
            QualificationOperation::Conjunction,
            &foreign,
            Some(&idx),
            &budget()
        )
        .is_err()
    );
    assert_ne!(q.assumptions, AssumptionSet::empty_id());
    assert_eq!(basis.set.count, 1);
}
#[test]
fn lower_native_premises_validate_context_and_missing_is_distinct_from_unknown() {
    let mut f = fixture::Fixture::new(false);
    let (_, _, q) = add_basis(&mut f);
    let invariant = AssumptionSet::invariants()[0].clone();
    f.base.check(&invariant).unwrap();
    let mut qs = f.base.rows::<AssertionQualification>();
    qs.retain(|r| r.id() != q.id());
    qs.push(AssertionQualification {
        context: AnalysisContext {
            python_version: "foreign".into(),
            ..f.base.rows::<AnalysisContext>()[0].clone()
        }
        .id(),
        ..q
    });
    f.base.put(qs);
    assert!(
        f.base
            .check(&invariant)
            .unwrap_err()
            .to_string()
            .contains("context/input")
    );
    let mut f = fixture::Fixture::new(false);
    let row = f.base.rows::<TypeObservation>()[0].clone();
    let unknown = TypeTerm::Any {
        flavor: AnyFlavor::Implicit,
    };
    let row = TypeObservation {
        term: unknown.id(),
        ..row
    };
    let support = TypeSupport {
        assertion: row.id(),
        ..f.base.rows::<TypeSupport>()[0].clone()
    };
    f.base.put(vec![row]);
    f.base.put(vec![support]);
    f.base.put(vec![unknown]);
    add_basis(&mut f);
    assert!(
        f.base
            .check(&invariant)
            .unwrap_err()
            .to_string()
            .contains("premise unknown")
    );
    f.base.put::<TypeTerm>(vec![]);
    assert!(
        f.base
            .check(&invariant)
            .unwrap_err()
            .to_string()
            .contains("term missing")
    );
}
#[test]
fn universe_identity_changes_with_actual_pinned_definition_and_refuses_arbitrary_digest() {
    let mut f = fixture::Fixture::new(false);
    let context = f.base.rows::<AnalysisContext>()[0].clone();
    let input = f.base.rows::<InputRevision>()[0].id();
    let parsed = Catalog::committed().unwrap();
    let catalog = parsed.declaration().clone();
    let model = parsed.models()[0].declaration().clone();
    let universe = AssumptionUniverse {
        context: context.id(),
        input,
        environment: context.environment_digest,
        model_definition: catalog.content,
    };
    let support = AssumptionUniverseSupport::new(&universe, &catalog, &model).unwrap();
    let changed = AssumptionUniverse {
        model_definition: ContentHash::of(b"changed world"),
        ..universe.clone()
    };
    assert_ne!(universe.id(), changed.id());
    assert!(AssumptionUniverseSupport::new(&changed, &catalog, &model).is_err());
    let class = symbols::ClassTraitObservation {
        qualification: f.base.rows::<AssertionQualification>()[0].id(),
        symbol: f.base.rows::<calls::ProviderSymbol>()[0].id(),
        synthesized: false,
        dataclass: false,
        named_tuple: false,
        typed_dict: false,
    };
    let assumption = |u| Assumption::NoExtraOverrides {
        class: class.id(),
        support: serde_json::from_value(serde_json::json!(vec![1; 16])).unwrap(),
        universe: u,
    };
    let basis = AssumptionSet::new([assumption(universe.id()).id()]).unwrap();
    let different = AssumptionSet::new([assumption(changed.id()).id()]).unwrap();
    assert_ne!(basis.set.id(), different.set.id());
    let q = AssertionQualification {
        assumptions: basis.set.id(),
        ..f.base.rows::<AssertionQualification>()[0].clone()
    };
    assert_ne!(
        q.id(),
        AssertionQualification {
            assumptions: different.set.id(),
            ..q
        }
        .id()
    );
    let fabricated = AuthoredModel {
        revision: model.revision + 1,
        ..model.clone()
    };
    assert!(AssumptionUniverseSupport::new(&universe, &catalog, &fabricated).is_err());
    let other = Catalog::parse(
        "other",
        &(catalog.source.clone() + "\n# another pinned universe\n"),
    )
    .unwrap();
    let bad = AssumptionUniverseSupport {
        universe: universe.id(),
        catalog: other.declaration().id(),
        model: other.models()[0].declaration().id(),
    };
    f.base.put(vec![universe]);
    f.base.put(vec![catalog, other.declaration().clone()]);
    f.base
        .put(vec![model, other.models()[0].declaration().clone()]);
    f.base.put(vec![bad, support]);
    assert!(
        f.base
            .check(&AssumptionUniverseSupport::invariants()[0])
            .unwrap_err()
            .to_string()
            .contains("actual pinned definition")
    );
}
#[test]
fn wire_requires_explicit_basis_and_resolved_definition_members() {
    let f = fixture::Fixture::new(false);
    let row = f.base.rows::<TypePresentation>()[0].clone();
    let empty =
        serving::ClaimBasisPacket::from_canonical(&ResolvedAssumptions::empty(), vec![]).unwrap();
    let packet = serving::TypePresentationPacket::from_canonical(&row, empty.clone()).unwrap();
    let encoded = serde_json::to_value(&packet).unwrap();
    assert_eq!(encoded["claim_basis"]["definitions"], serde_json::json!([]));
    assert_eq!(
        serde_json::from_value::<serving::TypePresentationPacket>(encoded.clone()).unwrap(),
        packet
    );
    let mut absent = encoded;
    absent.as_object_mut().unwrap().remove("claim_basis");
    assert!(serde_json::from_value::<serving::TypePresentationPacket>(absent).is_err());
    let q = f.base.rows::<AssertionQualification>()[0].clone();
    let encoded = <AssertionQualification as Record>::encode(&[q]).unwrap();
    let missing = encoded
        .project(
            &encoded
                .schema()
                .fields()
                .iter()
                .enumerate()
                .filter(|(_, f)| f.name() != "assumptions")
                .map(|(i, _)| i)
                .collect::<Vec<_>>(),
        )
        .unwrap();
    assert!(AssertionQualification::decode(&missing).is_err());
    let basis = AssumptionSet::new([Assumption::TypeConformance {
        observation: f.base.rows::<TypeObservation>()[0].id(),
        support: f.base.rows::<TypeSupport>()[0].id(),
    }
    .id()])
    .unwrap();
    assert!(serving::ClaimBasisPacket::from_canonical(&basis, vec![]).is_err());
}

#[test]
fn override_premise_binds_actual_class_support_and_universe_environment() {
    let mut f = fixture::Fixture::new(false);
    let symbol = f
        .base
        .rows::<calls::ProviderSymbol>()
        .into_iter()
        .find(|s| s.kind == calls::SymbolKind::Class)
        .unwrap();
    let context = f.base.rows::<AnalysisContext>()[0].clone();
    let input = f.base.rows::<InputRevision>()[0].id();
    let q = f.base.rows::<AssertionQualification>()[0].clone();
    let class = symbols::ClassTraitObservation {
        qualification: q.id(),
        symbol: symbol.id(),
        synthesized: false,
        dataclass: false,
        named_tuple: false,
        typed_dict: false,
    };
    let (run, families) = ProviderRun::new(
        symbol.provider,
        context.id(),
        input,
        context.config_digest,
        [FactFamily::Signatures],
    )
    .unwrap();
    let surface = ProviderSurface {
        provider: symbol.provider,
        family: FactFamily::Signatures,
        name: "native class".into(),
    };
    let evidence = Evidence::Invocation { run: run.id() };
    let support = symbols::ClassTraitSupport {
        assertion: class.id(),
        run: run.id(),
        surface: surface.id(),
        evidence: evidence.id(),
        origin: Origin::AnalyzerAssertion,
        mode: ExtractionMode::NativeTraversal,
        fidelity: Fidelity::NativeStructural,
    };
    macro_rules! append {
        ($ty:ty,$rows:expr) => {
            let mut rows = f.base.rows::<$ty>();
            rows.extend($rows);
            f.base.put(rows);
        };
    }
    append!(ProviderRun, vec![run]);
    append!(RunFamily, families);
    append!(ProviderSurface, vec![surface]);
    append!(Evidence, vec![evidence]);
    f.base.put(vec![class.clone()]);
    f.base.put(vec![support.clone()]);
    let catalog = Catalog::committed().unwrap();
    let universe = AssumptionUniverse {
        context: context.id(),
        input,
        environment: context.environment_digest,
        model_definition: catalog.declaration().content,
    };
    let definition = Assumption::NoExtraOverrides {
        class: class.id(),
        support: support.id(),
        universe: universe.id(),
    };
    let basis = AssumptionSet::new([definition.id()]).unwrap();
    f.base.put(vec![AssumptionSet::empty(), basis.set]);
    f.base.put(basis.members);
    f.base.put(vec![definition.clone()]);
    f.base.put(vec![universe.clone()]);
    append!(
        AssertionQualification,
        vec![AssertionQualification {
            assumptions: AssumptionSet::new([definition.id()]).unwrap().set.id(),
            ..q
        }]
    );
    let invariant = AssumptionSet::invariants()[0].clone();
    f.base.check(&invariant).unwrap();
    let changed = AssumptionUniverse {
        environment: ContentHash::of(b"other environment"),
        ..universe
    };
    let changed_definition = Assumption::NoExtraOverrides {
        class: class.id(),
        support: support.id(),
        universe: changed.id(),
    };
    f.base.put(vec![changed]);
    f.base.put(vec![definition, changed_definition]);
    assert!(
        f.base
            .check(&invariant)
            .unwrap_err()
            .to_string()
            .contains("environment")
    );
}
