use std::collections::BTreeMap;
use arrow_array::RecordBatch;
use lctx_model::domain::{*, assertion::*, attribution::*, source::*, input::*, conditions::*};

fn context(version: &str) -> AnalysisContext {
    AnalysisContext { python_version: version.into(), python_platform: "linux".into(), search_path: vec![],
        site_package_path: vec![], config_digest: ContentHash::of(b"config"), environment_digest: ContentHash::of(b"env"), lock_digest: None }
}
fn insert<R: Record>(model: &ValidatedModel, batches: &mut BTreeMap<&'static str,RecordBatch>, rows: Vec<R>) {
    batches.insert(R::NAME, Batch::new(model, rows, &budget()).unwrap().arrow().clone());
}
fn check(invariant: &Invariant, batches: &BTreeMap<&str,RecordBatch>) -> Result<(),ModelError> {
    let mut check = (invariant.create)(&budget());
    for input in &invariant.inputs { if let Some(batch) = batches.get(input.name()) { check.visit(input.name(),batch)?; } }
    check.finish()
}

#[test]
fn qualified_assertions_preserve_alternatives_and_require_typed_attribution() {
    let model = model().unwrap();
    let input = InputRevision::from_entries(vec![]).unwrap();
    let source = SourceArtifact::from_bytes(input.id(), "a.py".into(), b"x").unwrap();
    let other = SourceArtifact::from_bytes(input.id(), "b.py".into(), b"y").unwrap();
    let occurrence = Occurrence { source: source.id(), start: 0, end: 1, syntax_kind: SyntaxKind::ExprName, role: OccurrenceRole::Read, structural_path: vec![] };
    let provider = Provider { tool: "ruff".into(), revision: "0.0.11".into(), build_digest: ContentHash::of(b"build") };
    let ctx = context("3.14");
    let (run, families) = ProviderRun::new(provider.id(),ctx.id(),input.id(),ctx.config_digest,[FactFamily::Syntax]).unwrap();
    let scope = CoverageScope::Artifact { artifact: source.id() };
    let (condition,nodes) = Diagram::always().records();
    let qualification = AssertionQualification { context: ctx.id(),scope: scope.id(),condition: condition.id(),modality: Modality::Candidate,approximation: Approximation::Over };
    let assertion = SyntaxObservation { qualification: qualification.id(), occurrence: occurrence.id(), spelling: "x".into() };
    for qualified in [AssertionQualification { modality: Modality::Definite,..qualification.clone() },
        AssertionQualification { approximation: Approximation::Exact,..qualification.clone() },
        AssertionQualification { context: context("3.13").id(),..qualification.clone() },
        AssertionQualification { condition: Diagram::never().id(),..qualification.clone() }] {
        assert_ne!(assertion.id(),SyntaxObservation { qualification: qualified.id(),..assertion.clone() }.id());
    }
    let surface = ProviderSurface { provider: provider.id(),family: FactFamily::Syntax,name: "native-tree".into() };
    let evidence = Evidence::Occurrence { occurrence: occurrence.id() };
    let support = SyntaxSupport { assertion: assertion.id(),run: run.id(),surface: surface.id(),evidence: evidence.id(),origin: Origin::SourceObservation,mode: ExtractionMode::NativeTraversal,fidelity: Fidelity::NativeStructural };
    let mut repeated = support.clone(); repeated.mode = ExtractionMode::ReportDecode;
    assert_ne!(support.id(),repeated.id()); assert_eq!(support.assertion,repeated.assertion);
    let roundtrip = Batch::new(&model, vec![support.clone(),repeated.clone()], &budget()).unwrap();
    assert_eq!(SyntaxSupport::decode(roundtrip.arrow()).unwrap(),roundtrip.rows());
    let mut base = BTreeMap::new();
    insert(&model,&mut base,nodes); insert(&model,&mut base,vec![condition]);
    insert(&model,&mut base,vec![source.clone(),other.clone()]); insert(&model,&mut base,vec![occurrence.clone()]);
    insert(&model,&mut base,vec![scope]); insert(&model,&mut base,vec![qualification.clone()]);
    insert(&model,&mut base,vec![run.clone()]); insert(&model,&mut base,families);
    insert(&model,&mut base,vec![surface.clone()]); insert(&model,&mut base,vec![evidence]);
    insert(&model,&mut base,vec![assertion.clone()]); insert(&model,&mut base,vec![support.clone(),repeated]);
    let invariant = SyntaxSupport::invariants().remove(0); check(&invariant,&base).unwrap();
    for case in ["missing-support","wrong-context","wrong-provider","wrong-family","wrong-input","wrong-evidence","wrong-subject","wrong-invocation-evidence"] {
        let mut batches = base.clone();
        let mut changed_support = support.clone();
        match case {
            "missing-support" => { batches.remove(SyntaxSupport::NAME); }
            "wrong-context" => {
                let mut changed = qualification.clone(); changed.context = context("3.13").id();
                let mut assertion = assertion.clone(); assertion.qualification = changed.id(); changed_support.assertion = assertion.id();
                insert(&model,&mut batches,vec![changed]); insert(&model,&mut batches,vec![assertion]);
                insert(&model,&mut batches,vec![changed_support]);
            }
            "wrong-provider" | "wrong-family" => {
                let mut changed = surface.clone();
                if case == "wrong-provider" { changed.provider = Provider { revision: "different".into(),..provider.clone() }.id(); }
                else { changed.family = FactFamily::Calls; }
                changed_support.surface = changed.id(); insert(&model,&mut batches,vec![changed]); insert(&model,&mut batches,vec![changed_support]);
            }
            "wrong-input" => {
                let mut changed = run.clone(); changed.input = InputRevision::from_entries(vec![ManifestEntry { path: "x".into(),content: ContentHash::of(b"x"),byte_len: 1 }]).unwrap().id();
                changed_support.run = changed.id(); insert(&model,&mut batches,vec![RunFamily { run: changed.id(),family: FactFamily::Syntax }]);
                insert(&model,&mut batches,vec![changed]); insert(&model,&mut batches,vec![changed_support]);
            }
            "wrong-evidence" | "wrong-invocation-evidence" => {
                let changed = if case == "wrong-evidence" { Evidence::SourceSpan { source: other.id(),start: 0,end: 1 } }
                else { Evidence::Invocation { run: ProviderRun { configuration: ContentHash::of(b"other"),..run.clone() }.id() } };
                changed_support.evidence = changed.id(); insert(&model,&mut batches,vec![changed]); insert(&model,&mut batches,vec![changed_support]);
            }
            _ => {
                let changed = Occurrence { source: other.id(),..occurrence.clone() };
                let changed_assertion = SyntaxObservation { occurrence: changed.id(),..assertion.clone() };
                changed_support.assertion = changed_assertion.id();
                insert(&model,&mut batches,vec![changed,occurrence.clone()]); insert(&model,&mut batches,vec![changed_assertion]); insert(&model,&mut batches,vec![changed_support]);
            }
        }
        assert!(check(&invariant,&batches).is_err(),"{case}");
    }
    let foreign_input = InputRevision::from_entries(vec![ManifestEntry { path: "foreign.py".into(),content: ContentHash::of(b"z"),byte_len: 1 }]).unwrap();
    let corpus_input = InputRevision::from_entries(vec![ManifestEntry { path: "corpus.txt".into(),content: ContentHash::of(b"corpus"),byte_len: 6 }]).unwrap();
    let foreign = SourceArtifact::from_bytes(foreign_input.id(),"foreign.py".into(),b"z").unwrap();
    for case in ["same-input","unrelated-input","corpus-member","missing-corpus-member","outside-artifact-scope"] {
        let mut batches = base.clone();
        let evaluation_source = match case { "same-input" => &source, "outside-artifact-scope" => &other, _ => &foreign };
        let evaluation = Occurrence { source: evaluation_source.id(),role: OccurrenceRole::Predicate,..occurrence.clone() };
        let atom = EvaluationAtom { evaluation: evaluation.id(),context: ctx.id(),predicate: super_predicate(),operand: None };
        let (condition,nodes) = Diagram::from_atom(atom.id()).records();
        let corpus = matches!(case,"corpus-member"|"missing-corpus-member");
        let scope = if corpus { CoverageScope::Input { input: corpus_input.id() } }
            else if case == "outside-artifact-scope" { CoverageScope::Artifact { artifact: source.id() } }
            else { CoverageScope::Input { input: input.id() } };
        let qualified = AssertionQualification { scope: scope.id(),condition: condition.id(),..qualification.clone() };
        let asserted = SyntaxObservation { qualification: qualified.id(),..assertion.clone() };
        let invocation = ProviderRun { input: if corpus { corpus_input.id() } else { input.id() },..run.clone() };
        let supporting = SyntaxSupport { assertion: asserted.id(),run: invocation.id(),..support.clone() };
        insert(&model,&mut batches,vec![source.clone(),other.clone(),foreign.clone()]);
        insert(&model,&mut batches,vec![occurrence.clone(),evaluation]);
        insert(&model,&mut batches,vec![lctx_model::domain::value::Predicate::Truthy]);
        insert(&model,&mut batches,vec![atom]); insert(&model,&mut batches,nodes); insert(&model,&mut batches,vec![condition]);
        insert(&model,&mut batches,vec![scope]); insert(&model,&mut batches,vec![qualified]); insert(&model,&mut batches,vec![asserted]);
        insert(&model,&mut batches,vec![RunFamily { run: invocation.id(),family: FactFamily::Syntax }]);
        insert(&model,&mut batches,vec![invocation]); insert(&model,&mut batches,vec![supporting]);
        if corpus {
            let mut members = vec![CorpusLibrary { corpus: corpus_input.id(),library: input.id() }];
            if case == "corpus-member" { members.push(CorpusLibrary { corpus: corpus_input.id(),library: foreign_input.id() }); }
            insert(&model,&mut batches,members);
        }
        assert_eq!(check(&invariant,&batches).is_ok(),matches!(case,"same-input"|"corpus-member"),"{case}");
    }
}

#[test]
fn assertion_condition_context_and_evidence_bounds_are_stored_invariants() {
    let model = model().unwrap(); let ctx = context("3.14"); let other = context("3.13");
    let input = InputRevision::from_entries(vec![]).unwrap();
    let source = SourceArtifact::from_bytes(input.id(),"x.py".into(),b"x").unwrap();
    let occurrence = Occurrence { source: source.id(),start: 0,end: 1,syntax_kind: SyntaxKind::ExprName,role: OccurrenceRole::Predicate,structural_path: vec![] };
    let atom = EvaluationAtom { evaluation: occurrence.id(),context: ctx.id(),predicate: super_predicate(),operand: None };
    let (condition,nodes) = Diagram::from_atom(atom.id()).records();
    let scope = CoverageScope::Artifact { artifact: source.id() };
    let mut batches = BTreeMap::new(); insert(&model,&mut batches,vec![atom]); insert(&model,&mut batches,nodes); insert(&model,&mut batches,vec![condition.clone()]);
    for context in [ctx.id(),other.id()] {
        insert(&model,&mut batches,vec![AssertionQualification { context,scope: scope.id(),condition: condition.id(),modality: Modality::Definite,approximation: Approximation::Exact }]);
        assert_eq!(check(&AssertionQualification::invariants()[0],&batches).is_ok(),context == ctx.id());
    }
    insert(&model,&mut batches,vec![source.clone()]);
    for end in [1,2] {
        insert(&model,&mut batches,vec![Evidence::SourceSpan { source: source.id(),start: 0,end }]);
        assert_eq!(check(&Evidence::invariants()[0],&batches).is_ok(),end == 1);
    }
}
fn super_predicate() -> Id<lctx_model::domain::value::Predicate> { lctx_model::domain::value::Predicate::Truthy.id() }

#[test]
fn assertion_model_requires_its_concrete_support_companion() {
    let complete = model().unwrap();
    let without_support = complete.relations().iter().filter(|r| r.name() != SyntaxSupport::NAME).cloned().collect();
    let error = ValidatedModel::validate(without_support).unwrap_err();
    assert!(error.to_string().contains("requires companion relation syntax_supports"));
    ValidatedModel::validate(complete.relations().to_vec()).unwrap();
}

/// A fresh attempt budget; these controls do not share reservations across batches.
fn budget() -> lctx_model::domain::resources::ResourceBudget {
    lctx_model::domain::resources::ResourceBudget::fixed(lctx_model::domain::resources::DEFAULT_MEMORY_BYTES).unwrap()
}
