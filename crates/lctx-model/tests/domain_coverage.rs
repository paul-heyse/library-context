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
