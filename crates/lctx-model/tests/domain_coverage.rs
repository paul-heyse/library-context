#[allow(dead_code)] // Shared fixture has controls used by the lexical suites.
#[path = "fixtures/lexical.rs"] mod fixture;
use fixture::Fixture;
use lctx_model::domain::{*,attribution::*,input::*,source::*};

#[test]
fn attempted_coverage_is_bound_to_the_invocation_input_for_every_scope_kind() {
    for corpus in [false,true] {
        for owned in [false,true] {
            let mut f = Fixture::new();
            let original = f.rows::<ProviderCoverage>().pop().unwrap();
            let original_run = f.rows::<ProviderRun>().pop().unwrap();
            let owner = if owned { original_run.input } else { InputRevision::from_entries(vec![]).unwrap().id() };
            let source = SourceArtifact::from_bytes(owner,"foreign.py".into(),b"x").unwrap();
            let module = Module { source: source.id(),qualified_name: "foreign".into() };
            let package = Package { name: "fixture".into() };
            let release = Release { package: package.id(),version: "1".into() };
            f.put(vec![source.clone()]); f.put(vec![module.clone()]);
            f.put(vec![InputDistribution { input: owner,release: release.id(),role: DistributionRole::FirstParty }]);
            let mut run = original_run.clone();
            if corpus {
                // A corpus may own member artifact/module/release scopes; input scope is exact.
                run.input = InputRevision { manifest: ContentHash::of(b"corpus") }.id();
                f.put(vec![CorpusLibrary { corpus: run.input,library: original_run.input }]);
            }
            f.put(vec![run.clone()]);
            let scopes = [
                CoverageScope::Artifact { artifact: source.id() },CoverageScope::Module { module: module.id() },
                CoverageScope::Release { release: release.id() },CoverageScope::Input { input: owner },
            ];
            for scope in scopes {
                let is_input = matches!(&scope,CoverageScope::Input { .. });
                f.put(vec![ProviderCoverage { scope: scope.id(),run: Some(run.id()),..original.clone() }]);
                f.put(vec![scope]);
                let expected = owned && !(corpus && is_input);
                assert_eq!(f.check(&ProviderCoverage::invariants()[0]).is_ok(),expected);
            }
            f.put(vec![CoverageScope::Input { input: run.input }]);
            f.put(vec![ProviderCoverage { scope: CoverageScope::Input { input: run.input }.id(),run: Some(run.id()),..original }]);
            f.check(&ProviderCoverage::invariants()[0]).unwrap();
        }
    }
}

/// Plan A3 (T7): a provider stop is a boundary under non-complete coverage of its scope, and an
/// event that did not attach keeps exactly its candidates.
#[path = "fixtures/syntax.rs"] #[macro_use] mod syntax_fixture;
mod boundaries {
    use super::syntax_fixture::Fixture;
    use lctx_model::domain::{*, attribution::*, source::*, syntax::*};
    fn refused(fixture: &Fixture, why: &str, expected: &str) {
        let result = fixture.validate();
        assert!(matches!(&result, Err(ModelError::Invalid(message)) if message.contains(expected)), "{why}: expected `{expected}`, got {result:?}");
    }
    const CANDIDATES: &str = "keeps exactly its candidates";

    #[test]
    fn attachment_outcomes_keep_exactly_their_candidates() {
        for (kind, candidates, admitted) in [(AttachmentKind::Ambiguous, &["f.name", "f"][..], true), (AttachmentKind::Ambiguous, &["f.name"], false),
            (AttachmentKind::Innermost, &["f"], true), (AttachmentKind::Innermost, &[], false), (AttachmentKind::Unmatched, &[], true),
            (AttachmentKind::Unmatched, &["f"], false), (AttachmentKind::BudgetExceeded, &["f"], true)] {
            let mut f = Fixture::new();
            f.attachment(kind, candidates);
            if admitted { f.validate().unwrap_or_else(|e| panic!("{kind:?} with {candidates:?}: {e}")); } else { refused(&f, &format!("{kind:?} with {candidates:?}"), CANDIDATES); }
        }
    }

    #[test]
    fn a_candidate_or_subject_outside_the_scope_is_refused() {
        let foreign = |f: &Fixture| Occurrence { source: f.other.id(), start: 0, end: 1, syntax_kind: SyntaxKind::ExprName, role: OccurrenceRole::Read, structural_path: vec![9] };
        let mut f = Fixture::new();
        let occurrence = foreign(&f); f.add("foreign", occurrence);
        f.attachment(AttachmentKind::Ambiguous, &["f.name", "foreign"]);
        refused(&f, "a candidate in another source", "a candidate lies in its event's source");
        let mut f = Fixture::new();
        let occurrence = foreign(&f); f.add("foreign", occurrence.clone());
        f.attachment(AttachmentKind::Unmatched, &[]);
        f.boundaries[0].subject = Some(occurrence.id());
        f.outcomes[0].boundary = f.boundaries[0].id(); f.sync();
        refused(&f, "a boundary subject in another artifact", "a boundary's subject lies outside its scope");
    }

    #[test]
    fn a_boundary_needs_non_complete_coverage_and_its_matching_reason() {
        let mut f = Fixture::new();
        f.attachment(AttachmentKind::Unmatched, &[]);
        f.coverage[0].status = CoverageStatus::CompleteUnderStatedModel; f.coverage[0].reason = None; f.sync();
        refused(&f, "a boundary under complete coverage", "a boundary needs its provider's partial");
        let mut f = Fixture::new();
        f.attachment(AttachmentKind::Unmatched, &[]);
        f.boundaries[0].family = FactFamily::Calls; f.outcomes[0].boundary = f.boundaries[0].id(); f.sync();
        refused(&f, "a boundary in a family its provider did not partially cover", "a boundary needs its provider's partial");
        let mut f = Fixture::new();
        f.attachment(AttachmentKind::Unmatched, &[]);
        f.outcomes[0].outcome = AttachmentKind::Ambiguous; f.sync();
        refused(&f, "an ambiguous event disclosed as unmatched", "disclosed by its matching reason");
        let mut f = Fixture::new();
        f.attachment(AttachmentKind::Unmatched, &[]);
        f.put::<CoverageScope>(vec![]);
        assert!(f.validate().is_err(), "a boundary without its scope is refused");
    }
}
