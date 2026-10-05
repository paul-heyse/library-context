#[path = "fixtures/lexical.rs"]
mod fixture;
use fixture::Fixture;
use lctx_model::domain::{assertion::Assertion, lexical::*, *};

#[test]
fn lexical_relationships_roundtrip_and_share_source_and_support_validation() {
    let mut fixture = Fixture::new();
    for name in [
        "lexical_source_structure",
        LexicalScopeSupport::NAME,
        BindingSupport::NAME,
        ReferenceSupport::NAME,
        LexicalResolutionSupport::NAME,
    ] {
        let invariant = fixture
            .model
            .invariants()
            .iter()
            .find(|i| i.name == name)
            .unwrap();
        fixture.check(invariant).unwrap();
    }
    assert_eq!(
        fixture.binding.subjects().len(),
        3,
        "present optional value remains a provenance subject"
    );
    let mut absent = fixture.binding.clone();
    absent.value = None;
    assert_eq!(absent.subjects().len(), 2);
    assert_ne!(absent.id(), fixture.binding.id());
    fixture.foreign_value();
    assert!(
        fixture.check(&lctx_model::domain::validation::invariants_for::<BindingSupport>()[0]).is_err(),
        "foreign optional value cannot hide behind local evidence"
    );
    assert!(fixture.check(&lctx_model::domain::validation::invariants_for::<LexicalScope>()[0]).is_err());
}

#[test]
fn lexical_parents_kinds_ordinals_and_resolution_variants_are_checked() {
    let mut fixture = Fixture::new();
    let scopes = fixture.rows::<LexicalScope>();
    let scope = scopes
        .iter()
        .find(|s| s.kind == LexicalScopeKind::Function)
        .unwrap()
        .clone();
    let wrong = LexicalScope {
        kind: LexicalScopeKind::Class,
        ..scope
    };
    fixture.put(vec![wrong]);
    assert!(fixture.check(&lctx_model::domain::validation::invariants_for::<LexicalScope>()[0]).is_err());

    let mut fixture = Fixture::new();
    let mut rows = fixture.rows::<LexicalScopeObservation>();
    rows.iter_mut().find(|r| r.parent.is_some()).unwrap().parent = None;
    fixture.put(rows);
    assert!(fixture.check(&lctx_model::domain::validation::invariants_for::<LexicalScope>()[0]).is_err());

    let fixture = Fixture::new();
    let mut bad = fixture.binding.clone();
    bad.ordinal = -1;
    assert!(bad.validate().is_err());
    bad.ordinal = 0;
    bad.static_branch = Some(StaticBranch::Constant);
    assert!(bad.validate().is_err());

    let mut fixture = Fixture::new();
    let builtin = LexicalTarget::Builtin {
        name: "print".into(),
        variable: false,
    };
    let mut row = fixture.resolution.clone();
    row.target = builtin.id();
    row.captured = true;
    fixture.put(vec![builtin]);
    fixture.put(vec![row]);
    assert!(fixture.check(&lctx_model::domain::validation::invariants_for::<LexicalScope>()[0]).is_err());
}

// Qualification deliberately excludes provider/run: independent raw observations may disagree.
#[test]
fn independent_binding_interpretations_survive_at_the_same_ordinal() {
    use lctx_model::domain::{assertion::ProviderSurface, attribution::*};
    let mut fixture = Fixture::new();
    let mut alternative = fixture.binding.clone();
    alternative.kind = BindingEventKind::AugAssignment;
    let original_run = fixture.rows::<ProviderRun>().pop().unwrap();
    let provider = Provider {
        tool: "other-lexical".into(),
        revision: "1".into(),
        build_digest: ContentHash::of(b"other"),
    };
    let (run, families) = ProviderRun::new(
        provider.id(),
        original_run.context,
        original_run.input,
        original_run.configuration,
        [FactFamily::Lexical],
    )
    .unwrap();
    let surface = ProviderSurface {
        provider: provider.id(),
        family: FactFamily::Lexical,
        name: "independent".into(),
    };
    let support = BindingSupport {
        assertion: alternative.id(),
        run: run.id(),
        surface: surface.id(),
        ..fixture.binding_support.clone()
    };
    macro_rules! append {
        ($ty:ty,$rows:expr) => {
            let mut rows = fixture.rows::<$ty>();
            rows.extend($rows);
            fixture.put(rows);
        };
    }
    append!(Provider, vec![provider]);
    append!(ProviderRun, vec![run]);
    append!(RunFamily, families);
    append!(ProviderSurface, vec![surface]);
    append!(BindingObservation, vec![alternative]);
    append!(BindingSupport, vec![support]);
    fixture.check(&lctx_model::domain::validation::invariants_for::<LexicalScope>()[0]).unwrap();
    fixture.check(&lctx_model::domain::validation::invariants_for::<BindingSupport>()[0]).unwrap();
}

#[test]
fn reference_parent_requires_strict_structural_ancestry_even_for_equal_spans() {
    use lctx_model::domain::source::Occurrence;
    for case in 0..3 {
        let mut fixture = Fixture::new();
        let mut reference = fixture.rows::<ReferenceObservation>().pop().unwrap();
        let mut occurrences = fixture.rows::<Occurrence>();
        let read = occurrences
            .iter()
            .find(|o| o.id() == reference.read)
            .unwrap()
            .clone();
        let mut parent = read.clone();
        match case {
            0 => {}
            1 => parent.structural_path = vec![9],
            _ => {
                parent.structural_path.pop();
            }
        }
        reference.parent = parent.id();
        occurrences.push(parent);
        fixture.put(occurrences);
        fixture.put(vec![reference]);
        assert_eq!(
            fixture.check(&lctx_model::domain::validation::invariants_for::<LexicalScope>()[0]).is_ok(),
            case == 2
        );
    }
}
