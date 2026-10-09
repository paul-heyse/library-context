//! Scoped production kernels and independent admission; no provider or database prerequisites.
use lctx_model::domain::{
    assertion::*,
    attribution::*,
    normalized::{entities::*, entity_normalization::*},
    resources::ResourceBudget,
    source::*,
    symbols::*,
    *,
};
fn source() -> SourceArtifact {
    SourceArtifact::from_bytes(
        input::InputRevision::from_entries(vec![]).unwrap().id(),
        "scope.py".into(),
        b"pass\n",
    )
    .unwrap()
}
fn occurrence(source: &SourceArtifact, path: Vec<i32>, kind: SyntaxKind) -> Occurrence {
    Occurrence {
        source: source.id(),
        start: 0,
        end: 5,
        syntax_kind: kind,
        role: OccurrenceRole::Syntax,
        structural_path: path,
    }
}
#[test]
fn ordered_ownership_preserves_headers_bodies_and_releases_wide_sources() {
    let source = source();
    let rows = vec![
        occurrence(&source, vec![0], SyntaxKind::ModModule),
        occurrence(&source, vec![0, 0], SyntaxKind::StmtFunctionDef),
        occurrence(&source, vec![0, 0, 0], SyntaxKind::Parameters),
        occurrence(&source, vec![0, 0, 0, 0], SyntaxKind::Parameter),
        occurrence(&source, vec![0, 0, 1], SyntaxKind::StmtReturn),
        occurrence(&source, vec![0, 0, 1, 0], SyntaxKind::ExprName),
        occurrence(&source, vec![0, 1], SyntaxKind::StmtClassDef),
        occurrence(&source, vec![0, 1, 0], SyntaxKind::StmtFunctionDef),
        occurrence(&source, vec![0, 1, 0, 0], SyntaxKind::Parameters),
        occurrence(&source, vec![0, 1, 0, 1], SyntaxKind::StmtReturn),
    ];
    let budget = ResourceBudget::fixed(64 << 10).unwrap();
    let mut sweep = OwnershipSweep::new(&budget);
    for row in &rows {
        let result = sweep.push(row, None, &budget).unwrap();
        assert_eq!(
            result.owners.iter().next().unwrap().owner,
            occurrence_owner::owner_of(row, &rows).unwrap()
        );
    }
    drop(sweep);
    assert_eq!(budget.reserved(), 0);
    let mut sweep = OwnershipSweep::new(&budget);
    drop(sweep.push(&rows[0], None, &budget).unwrap());
    for index in 0..20_000 {
        let row = occurrence(&source, vec![0, index], SyntaxKind::StmtPass);
        let output = sweep.push(&row, None, &budget).unwrap();
        assert_eq!(output.owners.iter().next().unwrap().owner, rows[0].id());
        drop(output);
        assert!(
            budget.reserved() < 4096,
            "wide source must retain only its active path"
        );
    }
    drop(sweep);
    assert_eq!(budget.reserved(), 0);
}
#[test]
fn ordered_ownership_refuses_gaps_duplicates_and_reversed_sources() {
    let source = source();
    let root = occurrence(&source, vec![0], SyntaxKind::ModModule);
    let budget = ResourceBudget::fixed(64 << 10).unwrap();
    let mut sweep = OwnershipSweep::new(&budget);
    drop(sweep.push(&root, None, &budget).unwrap());
    assert!(sweep.push(&root, None, &budget).is_err());
    let mut sweep = OwnershipSweep::new(&budget);
    drop(sweep.push(&root, None, &budget).unwrap());
    assert!(
        sweep
            .push(
                &occurrence(&source, vec![0, 1, 0], SyntaxKind::ExprName),
                None,
                &budget
            )
            .is_err()
    );
    let other = SourceArtifact::from_bytes(source.input, "other.py".into(), b"pass\n").unwrap();
    let (first, last) = if source.id().bytes() < other.id().bytes() {
        (&source, &other)
    } else {
        (&other, &source)
    };
    let mut sweep = OwnershipSweep::new(&budget);
    drop(
        sweep
            .push(
                &occurrence(last, vec![0], SyntaxKind::ModModule),
                None,
                &budget,
            )
            .unwrap(),
    );
    assert!(
        sweep
            .push(
                &occurrence(first, vec![0], SyntaxKind::ModModule),
                None,
                &budget
            )
            .is_err()
    );
}
fn check(
    data: &EntityData,
    output: &EntityOutput,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let invariant = invariants()
        .into_iter()
        .find(|row| row.name == "normalized_entity_membership")
        .unwrap();
    assert_eq!(invariant.purpose, InvariantPurpose::Admission);
    let mut check = (invariant.create)(budget);
    macro_rules! facts { ($($field:ident: $ty:ty => $family:ident,)*) => { $(if invariant.inputs.iter().any(|input| input.name()==<$ty>::NAME) { check.visit(<$ty>::NAME,&<$ty as Record>::encode(&data.$field.iter().cloned().collect::<Vec<_>>())?)?; })* }; }
    lctx_model::normalized_entity_inputs!(facts);
    macro_rules! outputs { ($($field:ident: $ty:ty,)*) => { $(if invariant.inputs.iter().any(|input| input.name()==<$ty>::NAME) { check.visit(<$ty>::NAME,&<$ty as Record>::encode(&output.$field.iter().cloned().collect::<Vec<_>>())?)?; })* }; }
    lctx_model::normalized_entity_outputs!(outputs);
    check.finish()
}
#[test]
fn required_entity_admission_refuses_kind_membership_and_public_fidelity_without_replay() {
    let budget = ResourceBudget::fixed(2 << 20).unwrap();
    let mut data = EntityData::new(&budget);
    let source = source();
    let module = Module {
        source: source.id(),
        qualified_name: "scope".into(),
    };
    let root = occurrence(&source, vec![0], SyntaxKind::ModModule);
    data.modules.insert(module.clone()).unwrap();
    data.occurrences.insert(root.clone()).unwrap();
    let baseline = normalize(data.inputs(), &budget).unwrap();
    check(&data, &baseline, &budget).unwrap();
    let mut output = normalize(data.inputs(), &budget).unwrap();
    let wrong = CallableEntity::Source {
        declaration: root.id(),
        kind: CallableKind::Function,
    };
    output.callables.insert(wrong.clone()).unwrap();
    output
        .refs
        .insert(EntityRef::Callable {
            callable: wrong.id(),
        })
        .unwrap();
    assert!(check(&data, &output, &budget).is_err());
    drop(output);
    let empty = EntityOutput::new(&budget);
    assert!(check(&data, &empty, &budget).is_err());
    drop(empty);
    let context = AnalysisContext {
        python_version: "3.14".into(),
        python_platform: "linux".into(),
        search_path: vec![],
        site_package_path: vec![],
        config_digest: ContentHash::of(b"config"),
        environment_digest: ContentHash::of(b"env"),
        lock_digest: None,
    };
    let q = AssertionQualification {
        assumptions: assumptions::AssumptionSet::empty_id(),
        context: context.id(),
        scope: CoverageScope::Artifact {
            artifact: source.id(),
        }
        .id(),
        condition: conditions::Diagram::always().id(),
        modality: Modality::Candidate,
        approximation: Approximation::Exact,
    };
    data.qualifications.insert(q.clone()).unwrap();
    let public = PublicNameObservation {
        qualification: q.id(),
        access: module.id(),
        name: "uncertain".into(),
        via_dunder_all: false,
        origin: ExportOrigin::Untraced.id(),
    };
    data.public_names.insert(public).unwrap();
    data.export_origins.insert(ExportOrigin::Untraced).unwrap();
    let mut output = normalize(data.inputs(), &budget).unwrap();
    check(&data, &output, &budget).unwrap();
    let mut exposure = output.exposures.iter().next().unwrap().clone();
    exposure.publicity = PublicPathKnowledge::Known;
    // A fresh result avoids Rows correctly refusing an in-place conflicting identity.
    output = EntityOutput::new(&budget);
    output
        .refs
        .insert(EntityRef::Occurrence {
            occurrence: root.id(),
        })
        .unwrap();
    output
        .refs
        .insert(EntityRef::Module {
            module: module.id(),
        })
        .unwrap();
    output.exposures.insert(exposure).unwrap();
    assert!(check(&data, &output, &budget).is_err());
}

fn demand_fixture(budget: &ResourceBudget) -> (EntityData, calls::ProviderSymbol, calls::ProviderSymbol, AssertionQualification, Module, Provider) {
    use calls::*;
    let source = source();
    let module = Module { source: source.id(), qualified_name: "scope".into() };
    let provider = Provider { tool: "demand-control".into(), revision: "1".into(), build_digest: ContentHash::of(b"provider") };
    let context = AnalysisContext { python_version: "3.14".into(), python_platform: "linux".into(), search_path: vec![], site_package_path: vec![], config_digest: ContentHash::of(b"config"), environment_digest: ContentHash::of(b"env"), lock_digest: None };
    let q = AssertionQualification { assumptions: assumptions::AssumptionSet::empty_id(), context: context.id(), scope: CoverageScope::Artifact { artifact: source.id() }.id(), condition: conditions::Diagram::always().id(), modality: Modality::Definite, approximation: Approximation::Exact };
    let provider_module = ProviderModule::Acquired { module: module.id() };
    let class = ProviderSymbol { provider: provider.id(), context: context.id(), module: provider_module.id(), native_key: "class-C".into(), name: "C".into(), kind: SymbolKind::Class };
    let method = ProviderSymbol { native_key: "method-M".into(), name: "M".into(), kind: SymbolKind::Method, ..class.clone() };
    let mut data = EntityData::new(budget);
    data.modules.insert(module.clone()).unwrap();
    data.provider_modules.insert(provider_module).unwrap();
    data.qualifications.insert(q.clone()).unwrap();
    data.symbols.insert(class.clone()).unwrap();
    data.symbols.insert(method.clone()).unwrap();
    data.class_traits.insert(ClassTraitObservation { qualification: q.id(), symbol: class.id(), synthesized: true, dataclass: false, named_tuple: false, typed_dict: false }).unwrap();
    data.function_traits.insert(FunctionTraitObservation { qualification: q.id(), symbol: method.id(), overload: false, staticmethod: false, classmethod: false, property_getter: false, property_setter: false, stub: false, origin: FunctionOrigin::Synthesized, defining_class: Some(class.id()), overrides: None }).unwrap();
    let shape = ParameterShape { name: Some("value".into()), kind: ParameterKind::PositionalOrKeyword, required: true };
    let (signature, parameters) = Signature::new(&q, SignatureRole::Synthesized, None, method.id(), 0, SignatureForm::List, &[shape]).unwrap();
    data.signatures.insert(signature).unwrap();
    for parameter in parameters { data.parameters.insert(parameter).unwrap(); }
    let term = types::TypeTerm::ClassObject { class: class.id() };
    data.terms.insert(term.clone()).unwrap();
    data.fields.insert(types::RecordFieldObservation {
        qualification: q.id(), class: class.id(), name: "field".into(), record: types::RecordKind::NamedTuple,
        ordinal: 0, term: term.id(), declared: true, declaration: None, has_default: Some(false),
        default_term: None, init: None, alias: None, kw_only: None, required: None, read_only: None,
    }).unwrap();
    (data, method, class, q, module, provider)
}

/// Execute the actual finite scope program and borrow exactly its nominal input membership.
fn finite_entity_demand(data: &EntityData, demand: EntityDemand, budget: &ResourceBudget) -> (EntityOutput, Vec<(String, [u8; 16])>) {
    use std::any::TypeId;
    use finite_scope::{FiniteScope, FiniteScopeRow};
    let owner = model().unwrap();
    let inputs = EntityData::validation_inputs();
    let relations = inputs.iter().map(|input| owner.relation(input.name()).unwrap().clone()).collect::<Vec<_>>();
    let program = normalized::normalization_scope_program::entity(inputs.clone(), &relations, budget).unwrap();
    let (kind, key) = match demand {
        EntityDemand::Symbol(id) => (TypeId::of::<calls::ProviderSymbol>(), *id.bytes()),
        EntityDemand::SyntaxField(id) => (TypeId::of::<syntax::ClassFieldSyntaxObservation>(), *id.bytes()),
        EntityDemand::Public(id) => (TypeId::of::<PublicNameObservation>(), *id.bytes()),
        EntityDemand::Enumeration(id) => (TypeId::of::<ExportEnumerationObservation>(), *id.bytes()),
    };
    let root = program.entity_roots().iter().find(|(ty, _)| *ty == kind).unwrap().1;
    let mut rows = Vec::new();
    for input in &program.program().inputs {
        let mut values = Vec::new();
        macro_rules! facts {($($field:ident:$ty:ty => $family:ident,)*) => {$(if input.name() == <$ty>::NAME { values.extend(data.$field.iter().map(FiniteScopeRow::of)); })*};}
        lctx_model::normalized_entity_inputs!(facts);
        if input.type_id() == TypeId::of::<calls::ProviderSymbol>() {
            values = data.symbols.iter().map(|row| FiniteScopeRow::of(row).with_text("name", row.name.clone())).collect();
        }
        if input.type_id() == TypeId::of::<ExportOrigin>() {
            values = data.export_origins.iter().map(|row| match row {
                ExportOrigin::Traced { name, .. } => FiniteScopeRow::of(row).with_text("traced_name", name.clone()),
                ExportOrigin::Untraced => FiniteScopeRow::of(row).with_null("traced_module").with_null("traced_name"),
            }).collect();
        }
        rows.push(values);
    }
    let finite = FiniteScope::new(program.program().clone(), rows, &owner, budget).unwrap();
    let selected = finite.select(&[(root, key)], &scope_program::ScopeParameters(vec![]), budget).unwrap();
    let partition = selected.partition(0).unwrap();
    macro_rules! view {($($field:ident:$ty:ty => $family:ident,)*) => {{
        $(let table = inputs.iter().position(|input| input.type_id() == TypeId::of::<$ty>()).unwrap();
        let $field = data.$field.iter().filter(|row| partition.contains(&(table, *row.id().bytes()))).map(Record::id).collect::<Vec<_>>();)*
        let view = EntityDataView { $($field: normalized::RowsView::selected(&data.$field, &$field).unwrap(),)* };
        normalize_demand_view(&view, &demand, budget).unwrap()
    }};}
    let output = lctx_model::normalized_entity_inputs!(view);
    let membership = partition.iter().map(|(table, key)| (program.program().inputs[*table].name().to_owned(), *key)).collect();
    (output, membership)
}

fn merge_entities(target: &mut EntityOutput, source: &EntityOutput) {
    macro_rules! rows {($($field:ident:$ty:ty,)*) => {$(for row in source.$field.iter() { target.$field.insert(row.clone()).unwrap(); })*};}
    lctx_model::normalized_entity_outputs!(rows);
}

#[test]
fn explicit_entity_demand_keeps_supporting_symbols_without_resolving_them() {
    let budget = ResourceBudget::fixed(16 << 20).unwrap();
    let (data, method, class, _, module, _) = demand_fixture(&budget);
    let expected = normalize(data.inputs(), &budget).unwrap();
    let (method_rows, membership) = finite_entity_demand(&data, EntityDemand::Symbol(method.id()), &budget);
    assert!(membership.contains(&(calls::ProviderSymbol::NAME.into(), *class.id().bytes())));
    let class_traits = data.class_traits.iter().next().unwrap();
    assert!(!membership.contains(&(ClassTraitObservation::NAME.into(), *class_traits.id().bytes())));
    assert_eq!(method_rows.resolutions.len(), 1);
    assert_eq!(method_rows.resolutions.iter().next().unwrap().symbol, method.id());
    assert_eq!(method_rows.parameter_links.len(), 1);
    assert!(method_rows.field_links.is_empty());
    let broad_method = normalize_demand_view(&data.view(), &EntityDemand::Symbol(method.id()), &budget).unwrap();
    broad_method.matches(&method_rows).unwrap();
    assert!(broad_method.field_links.is_empty());
    let (class_rows, _) = finite_entity_demand(&data, EntityDemand::Symbol(class.id()), &budget);
    assert_eq!(class_rows.resolutions.iter().next().unwrap().status, ResolutionStatus::Resolved);
    assert_eq!(class_rows.field_links.len(), 1);
    // A broad lookup view may contain another symbol's parameters; it is not an output demand.
    let broad_class = normalize_demand_view(&data.view(), &EntityDemand::Symbol(class.id()), &budget).unwrap();
    assert!(broad_class.parameter_links.is_empty());
    broad_class.matches(&class_rows).unwrap();
    for reverse in [false, true] {
        let mut merged = EntityOutput::new(&budget);
        // The stage's mechanical vocabulary sweep is separate from these computational roots.
        merged.refs.insert(EntityRef::Module { module: module.id() }).unwrap();
        for term in data.terms.iter() { merged.refs.insert(EntityRef::Type { term: term.id() }).unwrap(); }
        for output in if reverse { [&class_rows, &method_rows] } else { [&method_rows, &class_rows] } { merge_entities(&mut merged, output); }
        merged.matches(&expected).unwrap();
    }
    let absent = calls::ProviderSymbol { native_key: "absent".into(), ..class.clone() };
    assert!(normalize_demand_view(&data.view(), &EntityDemand::Symbol(absent.id()), &budget).is_err());
}

#[test]
fn public_entity_demand_preserves_all_origin_alternatives_and_only_its_output_root() {
    let budget = ResourceBudget::fixed(16 << 20).unwrap();
    let (mut data, _, class, q, module, provider) = demand_fixture(&budget);
    let alternative = calls::ProviderSymbol { native_key: "class-alternative".into(), ..class.clone() };
    data.symbols.insert(alternative.clone()).unwrap();
    data.class_traits.insert(ClassTraitObservation { qualification: q.id(), symbol: alternative.id(), synthesized: true, dataclass: false, named_tuple: false, typed_dict: false }).unwrap();
    let origin = ExportOrigin::Traced { module: class.module, name: class.name.clone(), kind: Some(ExportKind::Class) };
    data.export_origins.insert(origin.clone()).unwrap();
    let public = PublicNameObservation { qualification: q.id(), access: module.id(), name: "C".into(), via_dunder_all: false, origin: origin.id() };
    let other = PublicNameObservation { name: "another_path".into(), ..public.clone() };
    data.public_names.insert(public.clone()).unwrap();
    data.public_names.insert(other.clone()).unwrap();
    let (run, _) = ProviderRun::new(provider.id(), q.context, source().input, ContentHash::of(b"request"), [FactFamily::Exports]).unwrap();
    let evidence = Evidence::Invocation { run: run.id() };
    let surface = ProviderSurface { provider: provider.id(), family: FactFamily::Exports, name: "exports".into() };
    for row in [&public, &other] {
        data.public_supports.insert(PublicNameSupport { assertion: row.id(), run: run.id(), surface: surface.id(), evidence: evidence.id(), origin: Origin::AnalyzerAssertion, mode: ExtractionMode::NativeTraversal, fidelity: Fidelity::NativeStructural }).unwrap();
    }
    data.runs.insert(run).unwrap();
    let expected = normalize(data.inputs(), &budget).unwrap();
    let (output, _) = finite_entity_demand(&data, EntityDemand::Public(public.id()), &budget);
    assert_eq!(output.resolutions.len(), 2);
    assert!(output.resolutions.iter().all(|row| [class.id(), alternative.id()].contains(&row.symbol)));
    assert!(output.parameter_links.is_empty());
    assert!(output.field_links.is_empty());
    assert_eq!(output.exposures.len(), 1);
    let exposure = output.exposures.iter().next().unwrap();
    assert_eq!(exposure.observation, public.id());
    assert_eq!(exposure.status, ResolutionStatus::Ambiguous);
    assert_eq!(output.exposure_candidates.len(), 2);
    assert_eq!(Some(exposure), expected.exposures.get(exposure.id()));
    let broad = normalize_demand_view(&data.view(), &EntityDemand::Public(public.id()), &budget).unwrap();
    assert!(broad.parameter_links.is_empty());
    assert_eq!(broad.exposures.len(), 1);
    assert_eq!(broad.resolutions.len(), 2);
    broad.matches(&output).unwrap();
}

#[test]
fn enumeration_entity_demand_assesses_only_its_root_without_symbol_resolution() {
    let budget = ResourceBudget::fixed(16 << 20).unwrap();
    let (mut data, _, _, q, module, provider) = demand_fixture(&budget);
    let row = ExportEnumerationObservation { qualification: q.id(), access: module.id(), names: value::LiteralSet { members: ContentHash::of(b"names") }.id(), status: ExportEnumerationStatus::Complete, basis: ExportEnumerationBasis::Inferred };
    data.export_enumerations.insert(row.clone()).unwrap();
    let candidate_q = AssertionQualification { modality: Modality::Candidate, ..q.clone() };
    data.qualifications.insert(candidate_q.clone()).unwrap();
    data.export_enumerations.insert(ExportEnumerationObservation { qualification: candidate_q.id(), status: ExportEnumerationStatus::Partial, ..row.clone() }).unwrap();
    let (run, _) = ProviderRun::new(provider.id(), q.context, source().input, ContentHash::of(b"enumeration"), [FactFamily::Exports]).unwrap();
    let evidence = Evidence::Invocation { run: run.id() };
    let surface = ProviderSurface { provider: provider.id(), family: FactFamily::Exports, name: "exports".into() };
    data.export_enumeration_supports.insert(ExportEnumerationSupport { assertion: row.id(), run: run.id(), surface: surface.id(), evidence: evidence.id(), origin: Origin::AnalyzerAssertion, mode: ExtractionMode::NativeTraversal, fidelity: Fidelity::NativeStructural }).unwrap();
    data.runs.insert(run).unwrap();
    let output = normalize_demand_view(&data.view(), &EntityDemand::Enumeration(row.id()), &budget).unwrap();
    assert!(output.resolutions.is_empty());
    assert!(output.exposures.is_empty());
    assert_eq!(output.public_enumerations.len(), 1);
    assert!(output.public_enumerations.iter().next().unwrap().closed);
    let expected = normalize(data.inputs(), &budget).unwrap();
    assert_eq!(expected.public_enumerations.len(), 2);
    let actual = output.public_enumerations.iter().next().unwrap();
    assert_eq!(Some(actual), expected.public_enumerations.get(actual.id()));
    data.qualifications = normalized::Rows::new(&budget);
    assert!(normalize_demand_view(&data.view(), &EntityDemand::Enumeration(row.id()), &budget).is_err(),
        "a demanded enumeration retains its necessary qualification premise");
}
