//! Native roles and ports remain distinct through normalization and explicit typed selection.
use crate::typed_driver;
use crate::inspector;
use lctx_model::domain::{
    calls::*,
    catalog::{self, *},
    normalized::{
        binding_normalization::BindingData, callable_normalization, callables::*,
        entity_normalization, relation_normalization,
    },
    resources::ResourceBudget,
    selection::{self, evaluate::Prepared, *},
    types::*,
    *,
};
use typed_driver::{files, rows};
inspector!(Facts, Signature);
async fn fixture() -> (typed_driver::Tables, BindingData, ResourceBudget) {
    fixture_for("native_callable_variants").await
}
async fn fixture_for(case: &str) -> (typed_driver::Tables, BindingData, ResourceBudget) {
    let tables = typed_driver::Tables::default();
    typed_driver::run(&files(case), Facts(tables.clone()))
        .await
        .unwrap();
    let b = ResourceBudget::fixed(256 << 20).unwrap();
    let mut relations = relation_normalization::RelationData::new(&b);
    macro_rules! facts {($($f:ident:$ty:ty=>$family:ident,)*)=>{$(for row in rows::<$ty>(&tables){relations.facts.$f.insert(row).unwrap();})*};}
    lctx_model::normalized_entity_inputs!(facts);
    relations.entities = entity_normalization::normalize(relations.facts.inputs(), &b).unwrap();
    macro_rules! additional {($($f:ident:$ty:ty=>$family:ident,)*)=>{$(for row in rows::<$ty>(&tables){relations.$f.insert(row).unwrap();})*};}
    lctx_model::normalized_relation_inputs!(additional);
    let links = relation_normalization::normalize(&relations, &b).unwrap();
    let mut data = BindingData::new(&b);
    macro_rules! raw {($($f:ident:$ty:ty,)*)=>{$(for row in rows::<$ty>(&tables){data.$f.insert(row).unwrap();})*};}
    lctx_model::normalized_binding_inputs!(raw);
    macro_rules! entities {($($f:ident:$ty:ty,)*)=>{$(data.visit(<$ty>::NAME,&<$ty as Record>::encode(&relations.entities.$f.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::normalized_entity_outputs!(entities);
    macro_rules! links {($($f:ident:$ty:ty,)*)=>{$(data.visit(<$ty>::NAME,&<$ty as Record>::encode(&links.$f.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::normalized_relation_outputs!(links);
    let mut callables = callable_normalization::CallableData::new(&b);
    macro_rules! inputs {($($f:ident:$ty:ty,)*)=>{$(callables.visit(<$ty>::NAME,&<$ty as Record>::encode(&data.$f.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::normalized_binding_inputs!(inputs);
    let normalized = callable_normalization::normalize(&callables, &b).unwrap();
    macro_rules! outputs {($($f:ident:$ty:ty,)*)=>{$(data.visit(<$ty>::NAME,&<$ty as Record>::encode(&normalized.$f.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::normalized_callable_outputs!(outputs);
    (tables, data, b)
}
fn signatures<'a>(d: &'a BindingData, name: &str, role: SignatureRole) -> Vec<&'a Signature> {
    d.signatures
        .iter()
        .filter(|s| s.role == role && d.symbols.get(s.symbol).is_some_and(|s| s.name == name))
        .collect()
}
fn slot_names(d: &BindingData, s: &Signature) -> Vec<Option<String>> {
    let mut slots: Vec<_> = d
        .parameters
        .iter()
        .filter(|p| p.signature == s.id())
        .collect();
    slots.sort_by_key(|p| p.ordinal);
    slots
        .iter()
        .map(|p| {
            d.shapes
                .get(p.shape)
                .unwrap()
                .name
                .as_ref()
                .map(ToString::to_string)
        })
        .collect()
}
#[tokio::test]
async fn wrappers_opaque_overloads_generated_and_bound_roles_have_native_ports() {
    let (tables, d, _) = fixture().await;
    let bytes = files("native_callable_variants");
    let occurrences = rows::<source::Occurrence>(&tables);
    let artifacts = rows::<source::SourceArtifact>(&tables);
    let text = |id| {
        let occurrence = occurrences.iter().find(|r| r.id() == id).unwrap();
        let artifact = artifacts
            .iter()
            .find(|a| a.id() == occurrence.source)
            .unwrap();
        std::str::from_utf8(
            &bytes[&artifact.path][occurrence.start as usize..occurrence.end as usize],
        )
        .unwrap()
    };
    let terms = rows::<TypeTerm>(&tables);
    let members = rows::<TypeSequenceMember>(&tables);
    assert!(terms.iter().any(|t|matches!(t,TypeTerm::Overloaded {alternatives} if members.iter().filter(|m|m.sequence==*alternatives).count()>=2 && members.iter().filter(|m|m.sequence==*alternatives).all(|m|m.role==TypeChildRole::Member))),"native overloaded result branches retain structure");
    let observations = rows::<TypeObservation>(&tables);
    assert!(
        observations
            .iter()
            .any(|r| r.role == TypeRole::ChosenOverload && text(r.subject) == "parse(1)")
    );
    assert!(
        !observations
            .iter()
            .any(|r| r.role == TypeRole::ChosenOverload && text(r.subject) == "parse(object())"),
        "closest failed alternative is not a chosen overload"
    );
    assert!(observations.iter().any(|r|r.role==TypeRole::OverloadCandidates && text(r.subject)=="parse(object())"));
    let source = signatures(&d, "changed", SignatureRole::Source);
    assert_eq!(source.len(), 1);
    assert_eq!(slot_names(&d, source[0]), vec![Some("value".into())]);
    let effective = signatures(&d, "changed", SignatureRole::EffectiveTyped);
    assert_eq!(effective.len(), 1);
    assert_eq!(
        slot_names(&d, effective[0]),
        vec![Some("text".into()), Some("flag".into())]
    );
    assert_ne!(source[0].id(), effective[0].id());
    assert!(
        d.parameter_links
            .iter()
            .filter(|p| p.declaration.is_some())
            .all(|p| d.parameters.get(p.parameter).is_none_or(|p| d
                .signatures
                .get(p.signature)
                .unwrap()
                .role
                .runtime_source())),
        "native typing never invents source formals"
    );
    let unavailable = signatures(&d, "unavailable", SignatureRole::EffectiveTyped);
    assert_eq!(unavailable.len(), 1);
    assert_eq!(unavailable[0].form, SignatureForm::NativeUnavailable);
    assert!(slot_names(&d, unavailable[0]).is_empty());
    let overloads = signatures(&d, "parse", SignatureRole::EffectiveTyped);
    assert!(overloads.len() >= 2);
    assert!(overloads.iter().all(|s| s.native.is_some()));
    let generated = signatures(&d, "__init__", SignatureRole::Synthesized);
    assert!(!generated.is_empty());
    assert!(
        generated
            .iter()
            .any(|s| slot_names(&d, s).contains(&Some("host".into())))
    );
    assert!(generated.iter().all(|s| {
        d.parameter_links
            .iter()
            .filter(|p| p.declaration.is_some())
            .all(|p| d.parameters.get(p.parameter).unwrap().signature != s.id())
    }));
    let method = signatures(&d, "run", SignatureRole::EffectiveTyped);
    assert_eq!(method.len(), 1);
    let variant = d
        .callable_variants
        .iter()
        .find(|v| v.signature == method[0].id())
        .unwrap();
    assert_eq!(
        variant.adjustment,
        SignatureAdjustment::BindInstanceReceiver
    );
    assert!(d.return_types.iter().any(|r| r.variant == variant.id()));
    assert!(d.slot_types.iter().any(|t| {
        d.callable_slots
            .get(t.slot)
            .is_some_and(|s| s.variant == variant.id())
    }));
}

async fn selection_fixture() -> (selection::build::Data, ResourceBudget) {
    selection_fixture_for("native_callable_variants").await
}
async fn selection_fixture_for(case: &str) -> (selection::build::Data, ResourceBudget) {
    let (tables, normalized, b) = fixture_for(case).await;
    let mut d = selection::build::Data::new(&b);
    for (name, batch) in tables.lock().unwrap().iter() {
        d.visit(name, batch).unwrap();
    }
    macro_rules! binding {($($f:ident:$ty:ty,)*)=>{$(d.visit(<$ty>::NAME,&<$ty as Record>::encode(&normalized.$f.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::normalized_binding_inputs!(binding);
    let mut aspects = lctx_model::domain::normalized::callable_aspects::AspectData::new(&b);
    for (name, batch) in tables.lock().unwrap().iter() {
        aspects.visit(name, batch).unwrap();
    }
    macro_rules! aspect_binding {($($f:ident:$ty:ty,)*)=>{$(aspects.visit(<$ty>::NAME,&<$ty as Record>::encode(&normalized.$f.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::normalized_binding_inputs!(aspect_binding);
    let aspects =
        lctx_model::domain::normalized::callable_aspects::normalize(&aspects, &b).unwrap();
    macro_rules! aspect_output {($($f:ident:$ty:ty,)*)=>{$(d.visit(<$ty>::NAME,&<$ty as Record>::encode(&aspects.$f.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::callable_aspect_outputs!(aspect_output);
    d.source.catalog = lctx_model::domain::catalog::build::build(&d.source.core, &b).unwrap();
    let contexts: Vec<_> = d
        .source
        .core
        .qualifications
        .iter()
        .map(|q| q.context)
        .collect();
    let mut contexts = contexts;
    contexts.sort();
    contexts.dedup();
    for context in contexts {
        let input = d.source.core.artifacts.iter().next().unwrap().input;
        let invocation = analysis::catalog_core::Invocation::new(
            input,
            context,
            catalog::build::definition().1.id(),
            None,
            [],
        )
        .0;
        let id = d.source.facts.core_invocations.insert(invocation).unwrap();
        for member in d.source.catalog.members.iter() {
            d.source
                .facts
                .core_links
                .insert(CatalogMemberInvocation {
                    member: member.id(),
                    invocation: id,
                })
                .unwrap();
        }
    }
    let mut native = analysis::native::NativeInventory::new(&b);
    for input in analysis::native::NativeInventory::inputs() {
        if let Some(batch) = tables.lock().unwrap().get(input.name()) {
            native.visit(input.name(), batch).unwrap();
        }
    }
    for premise in native.collect().unwrap().premises.iter() {
        d.source
            .facts
            .characterization_native
            .insert(premise.clone())
            .unwrap();
    }
    d.evidence = catalog::evidence::build::build(&d.source, &b).unwrap();
    (d, b)
}
#[tokio::test]
async fn catalog_selection_distinguishes_declared_and_effective_wrapper_ports() {
    let (d, b) = selection_fixture().await;
    let output = selection::build::build(&d, &b).unwrap();
    let prepared = Prepared::new(&d, &output, &b).unwrap();
    let member = d
        .source
        .catalog
        .members
        .iter()
        .find(|m| m.name == "changed")
        .unwrap();
    let analysis = d
        .source
        .core
        .native_signatures
        .iter()
        .find_map(|n| {
            let signature = d.facts.signatures.get(n.signature)?;
            d.source
                .core
                .symbols
                .get(signature.symbol)
                .filter(|s| s.name == "changed")?;
            Some(
                d.source
                    .core
                    .qualifications
                    .get(n.qualification)
                    .unwrap()
                    .context,
            )
        })
        .unwrap();
    let classify = |predicate| {
        prepared
            .classify(
                member.id(),
                analysis,
                &Requirement {
                    predicate,
                    quantifier: Quantifier::AnyApplicable,
                },
                &b,
            )
            .unwrap()
    };
    assert_eq!(
        classify(Predicate::VariantParameterType {
            role: SignatureRole::EffectiveTyped,
            name: "text".into(),
            r#type: StructuralType::NominalIdentity {
                module: "builtins".into(),
                name: "str".into()
            }
        })
        .outcome,
        Outcome::Supported
    );
    assert_eq!(
        classify(Predicate::VariantReturnType {
            role: SignatureRole::EffectiveTyped,
            r#type: StructuralType::NominalIdentity {
                module: "builtins".into(),
                name: "bytes".into()
            }
        })
        .outcome,
        Outcome::Supported
    );
    assert_eq!(
        classify(Predicate::ParameterType {
            name: "value".into(),
            r#type: StructuralType::NominalIdentity {
                module: "builtins".into(),
                name: "int".into()
            }
        })
        .outcome,
        Outcome::Supported
    );
    let negative = classify(Predicate::VariantParameterType {
        role: SignatureRole::EffectiveTyped,
        name: "value".into(),
        r#type: StructuralType::NominalIdentity {
            module: "builtins".into(),
            name: "int".into(),
        },
    });
    assert_eq!(
        negative.outcome,
        Outcome::Unresolved,
        "partial enumeration cannot establish absence"
    );
    assert!(
        negative
            .witnesses
            .iter()
            .any(|w| matches!(w, selection::algebra::RequirementWitness::Negative { .. })),
        "the complete observed wrapper still contributes its qualified negative witness"
    );
    let typed = Predicate::FacetMembership {
        facet: Facet::ParameterType,
        value: FacetValue::ParameterType {
            role: SignatureRole::EffectiveTyped,
            name: "text".into(),
            r#type: StructuralType::NominalIdentity {
                module: "builtins".into(),
                name: "str".into(),
            },
        },
    };
    let result = classify(typed.clone());
    assert_eq!(
        result.outcome,
        Outcome::Supported,
        "the effective wrapper exposes str text, without changing declared int value"
    );
    assert_eq!(
        result.requirement.predicate, typed,
        "lowering retains the caller's exact canonical requirement"
    );
    let unavailable = classify(Predicate::FacetMembership {
        facet: Facet::ParameterType,
        value: FacetValue::ParameterType {
            role: SignatureRole::EffectiveTyped,
            name: "value".into(),
            r#type: StructuralType::NominalIdentity {
                module: "builtins".into(),
                name: "int".into(),
            },
        },
    });
    assert_eq!(
        unavailable.outcome,
        Outcome::Unresolved,
        "partial invocation inventory cannot prove absence through a facet either"
    );
    assert_eq!(
        classify(Predicate::FacetMembership {
            facet: Facet::Kind,
            value: FacetValue::MemberKind {
                kind: MemberKind::Function
            }
        })
        .outcome,
        Outcome::Unresolved,
        "a typing callable wrapper does not establish a runtime function descriptor"
    );
    assert_eq!(
        classify(Predicate::FacetMembership {
            facet: Facet::Module,
            value: FacetValue::ModulePath {
                module: "api".into()
            }
        })
        .outcome,
        Outcome::Supported
    );
    assert_eq!(
        classify(Predicate::FacetMembership {
            facet: Facet::Module,
            value: FacetValue::ModulePath {
                module: "other".into()
            }
        })
        .outcome,
        Outcome::Contradicted
    );
    assert_eq!(
        classify(Predicate::FacetMembership {
            facet: Facet::Kind,
            value: FacetValue::MemberKind {
                kind: MemberKind::Class
            }
        })
        .outcome,
        Outcome::Contradicted
    );
}

#[tokio::test]
async fn supported_facets_keep_source_metadata_and_native_role_meanings() {
    let (d, b) = selection_fixture().await;
    let output = selection::build::build(&d, &b).unwrap();
    let prepared = Prepared::new(&d, &output, &b).unwrap();
    let changed = d
        .source
        .catalog
        .members
        .iter()
        .find(|m| m.name == "changed")
        .unwrap();
    let ctx=d.source.core.assessments.iter().find(|a|matches!(d.source.core.source_callables.get(a.callable),Some(lctx_model::domain::normalized::entities::CallableEntity::Source{declaration,..})if d.source.core.declarations.iter().any(|r|r.declaration==*declaration&&d.source.core.occurrences.get(r.name).is_some()))).unwrap().context;
    let query = |member, facet, value| {
        prepared
            .classify(
                member,
                ctx,
                &Requirement {
                    predicate: Predicate::FacetMembership { facet, value },
                    quantifier: Quantifier::AnyApplicable,
                },
                &b,
            )
            .unwrap()
    };
    assert_eq!(
        query(
            changed.id(),
            Facet::Async,
            FacetValue::Async {
                asynchronous: false
            }
        )
        .outcome,
        Outcome::Supported
    );
    let negative = query(
        changed.id(),
        Facet::Async,
        FacetValue::Async { asynchronous: true },
    );
    assert!(
        negative
            .witnesses
            .iter()
            .any(|w| matches!(w, selection::algebra::RequirementWitness::Negative { .. })),
        "known sync source supplies a negative characterization despite partial invocation inventory"
    );
    assert_eq!(
        query(
            changed.id(),
            Facet::Decorator,
            FacetValue::DecoratorQualifiedName {
                module: "api".into(),
                path: vec!["wrap".into()]
            }
        )
        .outcome,
        Outcome::Supported
    );
    let negative = query(
        changed.id(),
        Facet::Decorator,
        FacetValue::DecoratorQualifiedName {
            module: "other".into(),
            path: vec!["wrap".into()],
        },
    );
    assert!(
        negative
            .witnesses
            .iter()
            .any(|w| matches!(w, selection::algebra::RequirementWitness::Negative { .. }))
    );
    let settings = d
        .source
        .catalog
        .members
        .iter()
        .find(|m| m.name == "Settings")
        .unwrap();
    assert_eq!(
        query(
            settings.id(),
            Facet::ClassMetadata,
            FacetValue::ClassMetadata {
                trait_kind: ClassFacet::Enumeration,
                present: false
            }
        )
        .outcome,
        Outcome::Supported
    );
    assert_eq!(
        query(
            settings.id(),
            Facet::ClassMetadata,
            FacetValue::ClassMetadata {
                trait_kind: ClassFacet::AbstractMembers,
                present: false
            }
        )
        .outcome,
        Outcome::Unresolved,
        "native abstract absence is unavailable"
    );
    assert_eq!(
        query(
            changed.id(),
            Facet::Raises,
            FacetValue::RaisedClass {
                r#type: StructuralType::NominalIdentity {
                    module: "builtins".into(),
                    name: "ValueError".into()
                }
            }
        )
        .outcome,
        Outcome::Unresolved,
        "no raised-type record is not a never-raises proof"
    );
    let source = prepared
        .classify(
            changed.id(),
            ctx,
            &Requirement {
                predicate: Predicate::VariantReturnType {
                    role: SignatureRole::Source,
                    r#type: StructuralType::NominalIdentity {
                        module: "builtins".into(),
                        name: "int".into(),
                    },
                },
                quantifier: Quantifier::AnyApplicable,
            },
            &b,
        )
        .unwrap();
    assert_eq!(
        source.outcome,
        Outcome::Supported,
        "source return stays int while effective return is bytes"
    );
    let (d, b) = selection_fixture_for("native_callable_deprecation").await;
    let output = selection::build::build(&d, &b).unwrap();
    let prepared = Prepared::new(&d, &output, &b).unwrap();
    let member = d
        .source
        .catalog
        .members
        .iter()
        .find(|m| m.name == "retired")
        .unwrap();
    let ctx = d
        .source
        .core
        .symbols
        .iter()
        .find(|s| s.name == "retired")
        .unwrap()
        .context;
    let raised = d
        .source
        .catalog
        .members
        .iter()
        .find(|m| m.name == "raises_value_error")
        .unwrap();
    let classify_raise = |name: &str| {
        prepared
            .classify(
                raised.id(),
                ctx,
                &Requirement {
                    predicate: Predicate::FacetMembership {
                        facet: Facet::Raises,
                        value: FacetValue::RaisedClass {
                            r#type: StructuralType::NominalIdentity {
                                module: "builtins".into(),
                                name: name.into(),
                            },
                        },
                    },
                    quantifier: Quantifier::AnyApplicable,
                },
                &b,
            )
            .unwrap()
    };
    assert_eq!(
        classify_raise("ValueError").outcome,
        Outcome::Supported,
        "native raised-class characterization retains its supported type"
    );
    assert!(
        classify_raise("TypeError")
            .witnesses
            .iter()
            .any(|w| matches!(w, selection::algebra::RequirementWitness::Negative { .. })),
        "known raised ValueError differs from TypeError without claiming runtime absence"
    );
    for (role, deprecated, expected) in [
        (SignatureRole::EffectiveTyped, true, Outcome::Supported),
        (SignatureRole::Source, true, Outcome::Unresolved),
    ] {
        let value = prepared
            .classify(
                member.id(),
                ctx,
                &Requirement {
                    predicate: Predicate::FacetMembership {
                        facet: Facet::Deprecation,
                        value: FacetValue::Deprecation { role, deprecated },
                    },
                    quantifier: Quantifier::AnyApplicable,
                },
                &b,
            )
            .unwrap();
        assert_eq!(value.outcome, expected);
    }
}
