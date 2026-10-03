//! Symbol contracts (cutover plan A6): definitions and their nesting, function and class traits,
//! bases and MRO, display-only annotations, public names with traced or untraced origins,
//! docstring parameter documentation and module resolutions, over one real source. Every control
//! states its answer first; each accepted shape has a refused twin.
#[path = "fixtures/symbols.rs"]
#[macro_use]
mod fixture;
use fixture::{Fixture, at, budget};
use lctx_model::domain::{
    assertion::*,
    attribution::{Fidelity, Modality},
    calls::*,
    symbols::*,
    *,
};

fn refused(fixture: &Fixture, why: &str, expected: &str) {
    let result = fixture.validate();
    assert!(
        matches!(&result, Err(ModelError::Invalid(message)) if message.contains(expected)),
        "{why}: expected `{expected}`, got {result:?}"
    );
}
fn row_refused<R: Record + std::fmt::Debug>(row: &R, why: &str, expected: &str) {
    let result = row.validate();
    assert!(
        matches!(&result, Err(ModelError::Invalid(message)) if message.contains(expected)),
        "{why}: expected `{expected}`, got {result:?}"
    );
}
/// The dotted path of `name` through the observed parents: the derivation that replaces a stored
/// qualified name.
fn qualified(f: &Fixture, name: &str) -> String {
    let names: std::collections::BTreeMap<_, _> =
        f.sym.values().map(|s| (s.id(), s.name.clone())).collect();
    let parents: std::collections::BTreeMap<_, _> =
        f.symbols.iter().map(|o| (o.symbol, o.parent)).collect();
    let mut path = vec![];
    let mut current = Some(f.sym[name].id());
    while let Some(symbol) = current {
        path.push(names[&symbol].clone());
        current = parents[&symbol];
    }
    path.reverse();
    path.join(".")
}

#[test]
fn symbols_traits_ancestry_and_names_validate_and_derive_their_presentations() {
    let f = Fixture::new();
    f.validate().unwrap();
    // A qualified name and a signature count are derived, never stored.
    assert_eq!(qualified(&f, "inner"), "Service.run.inner");
    assert_eq!(qualified(&f, "Base.run"), "Base.run");
    assert_eq!(
        f.rows::<Signature>()
            .iter()
            .filter(|s| s.symbol == f.sym["Base.run"].id())
            .count(),
        1
    );
    // Base has no bases: a complete, empty statement, not an absent row.
    let base_bases = f
        .ancestry
        .iter()
        .find(|a| a.class == f.sym["Base"].id() && a.relation == AncestryRelation::Bases)
        .unwrap();
    assert_eq!(base_bases.ancestors, SymbolSequence::empty());
    // Service's MRO names Base, then the bundled typing.Generic, excluding itself and object.
    let mut members: Vec<_> = f
        .rows::<SymbolSequenceMember>()
        .into_iter()
        .filter(|m| m.sequence != SymbolSequence::empty())
        .collect();
    members.sort_by_key(|m| m.ordinal);
    let members: Vec<_> = members.into_iter().map(|m| m.symbol).collect();
    assert_eq!(members, vec![f.sym["Base"].id(), f.sym["Generic"].id()]);
}

#[test]
fn a_cyclic_mro_states_no_ancestors() {
    let f = Fixture::new();
    let service = f.sym["Service"].id();
    let cyclic = |ancestors| ClassAncestryObservation {
        qualification: f.qualification.id(),
        class: service,
        relation: AncestryRelation::Mro,
        ancestors,
        linearization: Some(Linearization::Cyclic),
    };
    row_refused(
        &cyclic(f.sequences[1].0.id()),
        "a cyclic MRO with ancestors",
        "a cyclic MRO states no ancestors",
    );
    let mut twin = Fixture::new();
    twin.ancestry[3] = cyclic(SymbolSequence::empty());
    twin.sync();
    twin.validate()
        .expect("a cyclic MRO with the empty sequence and the flag is accepted");
    row_refused(
        &ClassAncestryObservation {
            linearization: Some(Linearization::Complete),
            ..f.ancestry[0].clone()
        },
        "bases with a linearization",
        "every MRO has one",
    );
    row_refused(
        &ClassAncestryObservation {
            linearization: None,
            ..f.ancestry[1].clone()
        },
        "an MRO without a linearization",
        "every MRO has one",
    );
    // A prefix is accepted, and is distinct from a complete linearization.
    let prefix = ClassAncestryObservation {
        linearization: Some(Linearization::Prefix),
        ..f.ancestry[3].clone()
    };
    prefix.validate().unwrap();
    assert_ne!(prefix, f.ancestry[3]);
}

#[test]
fn an_mro_lists_other_classes_of_its_provider_once() {
    let mut f = Fixture::new();
    let own = SymbolSequence::new(&[f.sym["Base"].id(), f.sym["Service"].id()]).unwrap();
    f.sequences.push(own.clone());
    f.ancestry[3].ancestors = own.0.id();
    f.sync();
    refused(
        &f,
        "a class in its own MRO",
        "a class is not its own ancestor",
    );
    let mut f = Fixture::new();
    let twice = SymbolSequence::new(&[f.sym["Base"].id(), f.sym["Base"].id()]).unwrap();
    f.sequences.push(twice.clone());
    f.ancestry[3].ancestors = twice.0.id();
    f.sync();
    refused(
        &f,
        "an ancestor listed twice",
        "an MRO lists each ancestor once",
    );
    let mut f = Fixture::new();
    let method = SymbolSequence::new(&[f.sym["Base.run"].id()]).unwrap();
    f.sequences.push(method.clone());
    f.ancestry[2].ancestors = method.0.id();
    f.sync();
    refused(&f, "a method as a base", "an ancestor has another kind");
    let mut f = Fixture::new();
    let foreign = f.foreign_symbol("Base");
    f.sym.insert("foreign", foreign.clone());
    let across = SymbolSequence::new(&[foreign.id()]).unwrap();
    f.sequences.push(across.clone());
    f.ancestry[2].ancestors = across.0.id();
    f.sync();
    refused(
        &f,
        "a base from another analysis context",
        "an ancestor belongs to another provider or context",
    );
    let mut f = Fixture::new();
    f.sequences[1].1.pop();
    f.sync();
    refused(
        &f,
        "a sequence missing a member",
        "symbol sequence membership differs",
    );
}

#[test]
fn untraced_never_equals_traced() {
    let f = Fixture::new();
    let traced = &f.origins[0];
    assert_ne!(traced.id(), ExportOrigin::Untraced.id());
    assert_ne!(f.public[0].origin, f.public[1].origin);
    // The same name traced to a bundled module is another origin.
    let bundled = ExportOrigin::Traced {
        module: f.modules["typing"].id(),
        name: "Service".into(),
        kind: Some(ExportKind::Class),
    };
    assert_ne!(bundled.id(), traced.id());
    row_refused(
        &ExportOrigin::Traced {
            module: f.modules["example"].id(),
            name: String::new(),
            kind: None,
        },
        "a trace to no name",
        "a traced export names its origin",
    );
    // One public name has one origin: a second origin for the same name is a conflict.
    let mut both = f.public.clone();
    both.push(PublicNameObservation {
        origin: ExportOrigin::Untraced.id(),
        ..f.public[0].clone()
    });
    assert!(
        Batch::new(&f.model, both, &budget()).is_err(),
        "a public name both traced and untraced is refused"
    );
    // A trace into a module another provider bundles is refused; the fixture's own trace is accepted.
    let mut f = Fixture::new();
    let alien_module = ProviderModule::Bundled {
        provider: f.alien.id(),
        bundle: ModuleBundle::Typeshed,
        name: "typing".into(),
    };
    f.modules.insert("alien", alien_module.clone());
    let alien_origin = ExportOrigin::Traced {
        module: alien_module.id(),
        name: "Generic".into(),
        kind: Some(ExportKind::Class),
    };
    f.origins.push(alien_origin.clone());
    f.public[0].origin = alien_origin.id();
    f.sync();
    refused(
        &f,
        "a trace into another provider's bundle",
        "a provider module is stated only by its own provider and context",
    );
}

#[test]
fn a_display_only_annotation_cannot_establish_structure() {
    let mut f = Fixture::new();
    f.annotation_fidelity = Fidelity::NativeStructural;
    f.sync();
    refused(
        &f,
        "an annotation display supported as structure",
        "assertion requires another support fidelity",
    );
    let mut twin = Fixture::new();
    twin.annotation_fidelity = Fidelity::DisplayOnly;
    twin.sync();
    twin.validate()
        .expect("the display-only annotation is accepted");
    row_refused(
        &ParameterAnnotationObservation {
            display: String::new(),
            ..twin.annotations[0].clone()
        },
        "an empty display",
        "an annotation display is not empty",
    );
    // An annotation is stated under its signature's qualification.
    let mut f = Fixture::new();
    let other = AssertionQualification {
        modality: Modality::Candidate,
        ..f.qualification.clone()
    };
    let mut qualifications = f.rows::<AssertionQualification>();
    qualifications.push(other.clone());
    f.put(qualifications);
    f.annotations[0].qualification = other.id();
    f.sync();
    refused(
        &f,
        "an annotation under another qualification than its signature",
        "an annotation is stated with its signature",
    );
}

#[test]
fn function_traits_follow_the_nesting_they_state() {
    let f = Fixture::new();
    row_refused(
        &FunctionTraitObservation {
            classmethod: true,
            ..f.functions[1].clone()
        },
        "static and class method",
        "not both a static and a class method",
    );
    row_refused(
        &FunctionTraitObservation {
            property_setter: true,
            property_getter: true,
            ..f.functions[0].clone()
        },
        "getter and setter",
        "not both a property getter and setter",
    );
    row_refused(
        &FunctionTraitObservation {
            staticmethod: true,
            ..f.functions[3].clone()
        },
        "a static function outside a class",
        "method traits need a defining class",
    );
    row_refused(
        &FunctionTraitObservation {
            overrides: Some(f.sym["Service.run"].id()),
            ..f.functions[2].clone()
        },
        "a self-override",
        "does not override itself",
    );
    let mut f = Fixture::new();
    f.functions[2].defining_class = Some(f.sym["Base"].id());
    f.sync();
    refused(
        &f,
        "a method defined by a class it does not nest in",
        "a method's defining class is the class it nests in",
    );
    let mut f = Fixture::new();
    f.functions[2].defining_class = None;
    f.functions[2].overrides = None;
    f.sync();
    refused(
        &f,
        "a method without a defining class",
        "a method exactly when it has a defining class",
    );
    let mut f = Fixture::new();
    f.functions[2].overrides = Some(f.sym["Base"].id());
    f.sync();
    refused(
        &f,
        "overriding a class",
        "an overridden method has another kind",
    );
    let mut f = Fixture::new();
    f.symbols.retain(|o| o.symbol != f.sym["Service.make"].id());
    f.sync();
    refused(
        &f,
        "traits of an unobserved function",
        "a symbol's traits need its observation",
    );
    let mut f = Fixture::new();
    f.classes[0].symbol = f.sym["Base.run"].id();
    f.sync();
    refused(
        &f,
        "class traits of a method",
        "a class with class traits has another kind",
    );
}

#[test]
fn symbols_nest_within_one_module_and_end_at_top_level() {
    let mut f = Fixture::new();
    f.symbols[0].parent = Some(f.sym["Base.run"].id());
    f.sync();
    refused(
        &f,
        "Base nested in its own method",
        "symbol nesting is cyclic or beyond its limit",
    );
    let mut f = Fixture::new();
    f.symbols[5].parent = Some(f.sym["Generic"].id());
    f.sync();
    refused(
        &f,
        "a function nested in a bundled class",
        "a symbol nests in another module",
    );
    let mut f = Fixture::new();
    f.symbols.retain(|o| o.symbol != f.sym["Service.run"].id());
    f.functions
        .retain(|t| t.symbol != f.sym["Service.run"].id());
    f.sync();
    refused(
        &f,
        "a parent that is not observed",
        "a symbol's parent is not observed",
    );
    let mut f = Fixture::new();
    let foreign = f.foreign_symbol("Service");
    f.sym.insert("foreign", foreign.clone());
    f.symbols[3].parent = Some(foreign.id());
    f.functions[1].defining_class = Some(foreign.id());
    f.sync();
    refused(
        &f,
        "a parent from another analysis context",
        "a symbol's parent belongs to another provider or context",
    );
}

#[test]
fn a_symbol_is_stated_only_by_its_provider() {
    let mut f = Fixture::new();
    f.alien_support();
    refused(
        &f,
        "the fixture's symbols supported by another provider",
        "stated only by its own provider",
    );
    let mut twin = Fixture::new();
    twin.sync();
    twin.validate()
        .expect("the fixture's own provider states its symbols");
}

#[test]
fn parameter_documentation_lies_inside_its_def() {
    let f = Fixture::new();
    row_refused(
        &ParameterDocObservation {
            name: String::new(),
            ..f.docs[0].clone()
        },
        "an unnamed parameter",
        "a documented parameter has a name",
    );
    let mut f = Fixture::new();
    f.docs[0].declaration = f.occ["Base"].id();
    f.sync();
    refused(
        &f,
        "documentation of a class",
        "parameter documentation belongs to a def",
    );
    let mut f = Fixture::new();
    let outside = Evidence::SourceSpan {
        source: f.module.source,
        start: at("class Service"),
        end: at("class Service") + 5,
    };
    f.description = outside.clone();
    f.docs[0].description = EvidenceSourceSpanId::of(&outside).unwrap();
    f.sync();
    refused(
        &f,
        "a description outside its def",
        "a parameter description lies inside its def",
    );
    // The documented name need not be a parameter: the docstring's claim is kept as written.
    let mut f = Fixture::new();
    f.docs[0].name = "retries".into();
    f.sync();
    f.validate().unwrap();
}

#[test]
fn a_module_resolution_is_located_by_its_origin() {
    let f = Fixture::new();
    // One spelling in two bundles, or found as a namespace versus not found, is two modules.
    let typeshed = ProviderModule::Bundled {
        provider: f.provider.id(),
        bundle: ModuleBundle::Typeshed,
        name: "six".into(),
    };
    let third_party = ProviderModule::Bundled {
        provider: f.provider.id(),
        bundle: ModuleBundle::ThirdParty,
        name: "six".into(),
    };
    assert_ne!(typeshed.id(), third_party.id());
    let namespace = ProviderModule::Namespace {
        provider: f.provider.id(),
        context: f.context.id(),
        name: "gone".into(),
    };
    assert_ne!(namespace.id(), f.modules["gone"].id());
    row_refused(
        &ProviderModule::Namespace {
            provider: f.provider.id(),
            context: f.context.id(),
            name: String::new(),
        },
        "an unnamed namespace",
        "provider module needs a name",
    );
    for (index, location, why) in [
        (1, None, "a bundled stub without its path"),
        (
            0,
            Some("example.py"),
            "an acquired module restating its path",
        ),
        (3, Some("gone"), "an unresolved module with a location"),
        (2, Some("../nspkg"), "a namespace path leaving its root"),
        (1, Some("/abs/typing.pyi"), "an absolute bundle path"),
    ] {
        let mut f = Fixture::new();
        f.module_resolutions[index].location = location.map(str::to_owned);
        f.sync();
        refused(&f, why, "located by a relative path");
    }
    let mut f = Fixture::new();
    let alien = ProviderModule::Namespace {
        provider: f.alien.id(),
        context: f.context.id(),
        name: "nspkg".into(),
    };
    f.modules.insert("alien", alien.clone());
    f.module_resolutions[2].module = alien.id();
    f.sync();
    refused(
        &f,
        "another provider's namespace resolution",
        "a provider module is stated only by its own provider and context",
    );
}

#[test]
fn a_reexport_is_about_its_access_module_and_refers_to_its_origin() {
    use lctx_model::domain::{
        input::InputRevision,
        source::{CoverageScope, SourceArtifact},
    };
    // Under the access module's own scope, a name re-exported from another captured module.
    let f = Fixture::new();
    let artifact = CoverageScope::Artifact {
        artifact: f.module.source,
    };
    let scoped = AssertionQualification {
        scope: artifact.id(),
        ..f.qualification.clone()
    };
    let reexport = |f: &mut Fixture, module: Id<ProviderModule>| {
        let mut qualifications = f.rows::<AssertionQualification>();
        qualifications.push(scoped.clone());
        f.put(qualifications);
        let mut scopes = f.rows::<CoverageScope>();
        scopes.push(artifact.clone());
        f.put(scopes);
        let origin = ExportOrigin::Traced {
            module,
            name: "X".into(),
            kind: Some(ExportKind::Variable),
        };
        f.origins.push(origin.clone());
        f.public.push(PublicNameObservation {
            qualification: scoped.id(),
            access: f.module.id(),
            name: "X".into(),
            via_dunder_all: false,
            origin: origin.id(),
        });
        f.sync();
    };
    let mut accepted = Fixture::new();
    let other = accepted.modules["other"].id();
    reexport(&mut accepted, other);
    accepted
        .validate()
        .expect("a re-export from a module of the same input, under the access module's scope");
    // Its twin: an origin over bytes another input captured.
    let mut foreign = Fixture::new();
    let stray_bytes: &[u8] = b"X = 2\n";
    let elsewhere = InputRevision::from_entries(vec![lctx_model::domain::input::ManifestEntry {
        path: "stray.py".into(),
        content: ContentHash::of(stray_bytes),
        byte_len: stray_bytes.len() as i64,
    }])
    .unwrap();
    let stray = SourceArtifact::from_bytes(elsewhere.id(), "stray.py".into(), b"X = 2\n").unwrap();
    let module = lctx_model::domain::source::Module {
        source: stray.id(),
        qualified_name: "stray".into(),
    };
    let provider_module = ProviderModule::Acquired {
        module: module.id(),
    };
    foreign.modules.insert("stray", provider_module.clone());
    let mut modules = foreign.rows::<lctx_model::domain::source::Module>();
    modules.push(module);
    foreign.put(modules);
    let mut chunks = foreign.rows::<lctx_model::domain::artifact::ArtifactChunk>();
    chunks.extend(lctx_model::domain::artifact::ArtifactChunk::split(&stray, b"X = 2\n").unwrap());
    foreign.put(chunks);
    let mut artifacts = foreign.rows::<SourceArtifact>();
    artifacts.push(stray);
    foreign.put(artifacts);
    let mut inputs = foreign.rows::<InputRevision>();
    inputs.push(elsewhere);
    foreign.put(inputs);
    reexport(&mut foreign, provider_module.id());
    refused(
        &foreign,
        "an origin over bytes this invocation did not capture",
        "invocation input",
    );
}

#[test]
fn module_resolution_belongs_to_the_exact_source_alias_and_qualification() {
    let f = Fixture::new().with_import_alias();
    f.validate().unwrap();
    let mut wrong = Fixture::new().with_import_alias();
    wrong.module_resolutions.last_mut().unwrap().alias = Some(wrong.occ["Base"].id());
    wrong.sync();
    refused(&wrong, "a declaration cannot stand in for an alias", "exact import alias and qualification");
    let mut wrong = Fixture::new().with_import_alias();
    let candidate = AssertionQualification { modality: Modality::Candidate, ..wrong.qualification.clone() };
    wrong.imports[0].qualification = candidate.id();
    wrong.sync();
    let mut qualifications = wrong.rows::<AssertionQualification>();
    qualifications.push(candidate);
    wrong.put(qualifications);
    refused(&wrong, "an alias in another qualification", "exact import alias and qualification");
}
