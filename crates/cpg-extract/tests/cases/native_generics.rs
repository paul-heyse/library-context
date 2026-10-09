//! Native roles and ports remain distinct through normalization and explicit typed selection.
use crate::typed_driver;
use crate::inspector;
use lctx_model::domain::{
    calls::*,
    catalog::{self, *},
    normalized::{
        binding_normalization::BindingData, callable_normalization, entity_normalization,
        relation_normalization,
    },
    resources::ResourceBudget,
    selection::{self, evaluate::Prepared},
    types::*,
    *,
};
use typed_driver::{files, rows};
// This control reconstructs the complete native inventory or compares complete output.
inspector!(Facts, complete);
async fn fixture() -> (typed_driver::Tables, BindingData, ResourceBudget) {
    let tables = typed_driver::Tables::default();
    typed_driver::run(&files("native_generics"), Facts(tables.clone()))
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
#[tokio::test]
async fn native_receiver_specializations_retain_site_binder_and_declaration() {
    let (tables, d, _) = fixture().await;
    let specializations = rows::<GenericSpecializationObservation>(&tables);
    assert!(
        !specializations.is_empty(),
        "actual bound-method type trace must provide generic bindings"
    );
    let files = files("native_generics");
    let occurrences = rows::<source::Occurrence>(&tables);
    let artifacts = rows::<source::SourceArtifact>(&tables);
    let text = |id| {
        let o = occurrences.iter().find(|r| r.id() == id).unwrap();
        let a = artifacts.iter().find(|a| a.id() == o.source).unwrap();
        std::str::from_utf8(&files[&a.path][o.start as usize..o.end as usize]).unwrap()
    };
    let int = specializations
        .iter()
        .find(|r| text(r.site) == "integer_box.get")
        .expect("int receiver");
    let string = specializations
        .iter()
        .find(|r| text(r.site) == "string_box.get")
        .expect("str receiver");
    let other = specializations
        .iter()
        .find(|r| text(r.site) == "Other[bytes](b\"x\").get")
        .expect("other declaring class");
    assert_eq!(int.variable, string.variable);
    assert_eq!(int.declaration, string.declaration);
    assert_ne!(int.argument, string.argument);
    assert_ne!(int.receiver, string.receiver);
    assert_ne!(
        int.variable, other.variable,
        "same T spelling has different native binder identity"
    );
    assert!(
        d.signatures
            .iter()
            .all(|s| s.role != SignatureRole::Specialized),
        "no global specialized aliases"
    );
    let variables = typed_driver::rows::<TypeVariable>(&tables);
    assert!(
        variables
            .iter()
            .any(|v| v.declared_variance == Some(TypeVariance::Covariant))
    );
    assert!(
        variables
            .iter()
            .any(|v| v.kind == TypeVariableKind::ParamSpec)
    );
    assert!(
        typed_driver::rows::<TypeTerm>(&tables)
            .iter()
            .any(|t| matches!(t,TypeTerm::TypeAliasReference {name,..} if name=="Recursive"))
    );
    assert!(
        variables.iter().all(|v| v.inferred_variance.is_none()),
        "no inferred variance claim without native interface"
    );
    let restrictions = typed_driver::rows::<TypeVariableRestriction>(&tables);
    assert!(
        restrictions
            .iter()
            .any(|r| r.kind == TypeRestrictionKind::Constraint)
    );
    assert!(
        restrictions
            .iter()
            .any(|r| r.kind == TypeRestrictionKind::Default)
    );
}

async fn selection_fixture() -> (selection::build::Data, ResourceBudget) {
    let (tables, normalized, b) = fixture().await;
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
async fn located_query_specializes_two_receivers_without_changing_declared_catalog_types() {
    use lctx_model::domain::normalized::generic_specialization::SpecializationStatus;
    let (d, b) = selection_fixture().await;
    let output = selection::build::build(&d, &b).unwrap();
    let prepared = Prepared::new(&d, &output, &b).unwrap();
    let bindings: Vec<_> = d.facts.generic_specializations.iter().collect();
    assert!(!bindings.is_empty());
    let mut observed = std::collections::BTreeSet::new();
    for binding in bindings {
        let native = d
            .source
            .core
            .native_signatures
            .get(binding.declaration)
            .unwrap();
        let subject = SignatureTypeSubject::Return {
            signature: native.signature,
        };
        let context = d
            .source
            .core
            .qualifications
            .get(binding.qualification)
            .unwrap()
            .context;
        let result = prepared
            .specialize_port(binding.site, binding.declaration, subject.id(), context, &b)
            .unwrap();
        assert_eq!(
            result.specialized.status,
            SpecializationStatus::Resolved,
            "{:?}",
            result.specialized.boundaries
        );
        assert_eq!(result.specialized.site, binding.site);
        assert_eq!(result.specialized.receiver, Some(binding.receiver));
        assert_eq!(result.specialized.declaration, binding.declaration);
        assert!(result.specialized.support.contains(&binding.id()));
        let term = result
            .specialized
            .structure
            .terms
            .get(result.specialized.term)
            .unwrap();
        let TypeTerm::ClassInstance { class, .. } = term else {
            panic!("specialized return not nominal: {term:?}")
        };
        observed.insert(d.source.core.symbols.get(*class).unwrap().name.clone());
        let invocation = d
            .source
            .catalog
            .invocations
            .iter()
            .find(|i| {
                d.source
                    .core
                    .variants
                    .get(i.variant)
                    .is_some_and(|v| v.native == Some(binding.declaration))
            })
            .expect("native role catalog invocation");
        let member = d
            .source
            .catalog
            .callables
            .get(invocation.callable)
            .unwrap()
            .member;
        let occurrence = d.source.core.occurrences.get(binding.site).unwrap();
        let artifact = d.source.core.artifacts.get(occurrence.source).unwrap();
        let inputs = files("native_generics");
        let source = std::str::from_utf8(
            &inputs[&artifact.path][occurrence.start as usize..occurrence.end as usize],
        )
        .unwrap();
        let expected = match source {
            "integer_box.get" => "int",
            "string_box.get" => "str",
            "Other[bytes](b\"x\").get" => "bytes",
            _ => panic!("unexpected native receiver query {source}"),
        };
        for name in ["int", "str", "bytes"] {
            let requirement = selection::Requirement {
                predicate: selection::Predicate::SpecializedType {
                    site: binding.site,
                    declaration: binding.declaration,
                    subject: subject.id(),
                    r#type: selection::StructuralType::NominalIdentity {
                        module: "builtins".into(),
                        name: name.into(),
                    },
                },
                quantifier: selection::Quantifier::AnyApplicable,
            };
            let classified = prepared
                .classify(member, context, &requirement, &b)
                .unwrap();
            assert_eq!(
                classified.outcome,
                if name == expected {
                    selection::Outcome::Supported
                } else {
                    selection::Outcome::Contradicted
                }
            );
            assert_eq!(classified.requirement, requirement);
            let facet = selection::Requirement {
                predicate: selection::Predicate::FacetMembership {
                    facet: selection::Facet::Returns,
                    value: selection::FacetValue::SpecializedReturnType {
                        site: binding.site,
                        declaration: binding.declaration,
                        signature: native.signature,
                        r#type: selection::StructuralType::NominalIdentity {
                            module: "builtins".into(),
                            name: name.into(),
                        },
                    },
                },
                quantifier: selection::Quantifier::AnyApplicable,
            };
            let facet_result = prepared.classify(member, context, &facet, &b).unwrap();
            assert_eq!(facet_result.outcome, classified.outcome);
            assert_eq!(facet_result.requirement, facet);

            assert!(classified.closure.iter().any(|w|matches!(w,selection::Witness::GenericSpecialization {observation} if *observation==binding.id())));
            assert!(
                prepared
                    .classify(
                        serde_json::from_value(serde_json::to_value([255u8; 16]).unwrap()).unwrap(),
                        context,
                        &requirement,
                        &b
                    )
                    .is_err()
            );
        }
        assert!(
            selection::Predicate::VariantReturnType {
                role: SignatureRole::Specialized,
                r#type: selection::StructuralType::Category { kind: 0 }
            }
            .validate()
            .is_err()
        );

        let source = d
            .source
            .core
            .signature_types
            .get(result.observation)
            .unwrap();
        assert_ne!(
            source.term, result.specialized.term,
            "declaration stays generic"
        );
        let mut wrong = subject.clone();
        let SignatureTypeSubject::Return { signature } = &mut wrong else {
            unreachable!()
        };
        *signature = d
            .facts
            .signatures
            .iter()
            .find(|s| s.role == SignatureRole::Source)
            .unwrap()
            .id();
        if d.facts.signature_subjects.get(wrong.id()).is_some() {
            assert!(
                prepared
                    .specialize_port(binding.site, binding.declaration, wrong.id(), context, &b)
                    .is_err()
            );
        }
    }
    assert!(
        observed.contains("int") && observed.contains("str") && observed.contains("bytes"),
        "{observed:?}"
    );
    assert!(
        d.source
            .core
            .variants
            .iter()
            .all(|v| v.role != SignatureRole::Specialized)
    );
}

#[tokio::test]
async fn missing_function_binder_proofs_and_recursive_ports_return_qualified_unknown() {
    use lctx_model::domain::normalized::generic_specialization::{Boundary, SpecializationStatus};
    let (d, b) = selection_fixture().await;
    let output = selection::build::build(&d, &b).unwrap();
    let prepared = Prepared::new(&d, &output, &b).unwrap();
    let files = files("native_generics");
    let occurrences = typed_driver::rows::<source::Occurrence>(&{
        let tables = typed_driver::Tables::default();
        typed_driver::run(&files, Facts(tables.clone()))
            .await
            .unwrap();
        tables
    });
    for name in ["retained", "recursive"] {
        let native = d
            .source
            .core
            .native_signatures
            .iter()
            .find(|n| {
                d.facts
                    .signatures
                    .get(n.signature)
                    .and_then(|s| d.source.core.symbols.get(s.symbol))
                    .is_some_and(|s| s.name == name)
            })
            .unwrap();
        let context = d
            .source
            .core
            .qualifications
            .get(native.qualification)
            .unwrap()
            .context;
        let site = d
            .facts
            .call_syntax
            .iter()
            .find_map(|c| {
                let o = occurrences.iter().find(|o| o.id() == c.callee)?;
                let artifact = d.source.core.artifacts.get(o.source)?;
                (std::str::from_utf8(&files[&artifact.path][o.start as usize..o.end as usize]).ok()
                    == Some(name))
                .then_some(c.callee)
            })
            .unwrap();
        let subject = SignatureTypeSubject::Return {
            signature: native.signature,
        };
        let result = prepared
            .specialize_port(site, native.id(), subject.id(), context, &b)
            .unwrap();
        let invocation = d
            .source
            .catalog
            .invocations
            .iter()
            .find(|i| {
                d.source
                    .core
                    .variants
                    .get(i.variant)
                    .is_some_and(|v| v.native == Some(native.id()))
            })
            .expect("native role catalog invocation");
        let member = d
            .source
            .catalog
            .callables
            .get(invocation.callable)
            .unwrap()
            .member;
        let requirement = selection::Requirement {
            predicate: selection::Predicate::SpecializedType {
                site,
                declaration: native.id(),
                subject: subject.id(),
                r#type: selection::StructuralType::Category { kind: 0 },
            },
            quantifier: selection::Quantifier::AnyApplicable,
        };
        assert_eq!(
            prepared
                .classify(member, context, &requirement, &b)
                .unwrap()
                .outcome,
            selection::Outcome::Unresolved
        );
        assert_eq!(result.specialized.status, SpecializationStatus::Unknown);
        assert!(
            result
                .specialized
                .boundaries
                .contains(&Boundary::MissingNativeBinding)
        );
        if name == "recursive" {
            assert!(
                result
                    .specialized
                    .boundaries
                    .iter()
                    .any(|b| matches!(b, Boundary::AliasReference(_)))
            );
        }
        assert_eq!(
            d.source
                .core
                .signature_types
                .get(result.observation)
                .unwrap()
                .subject,
            subject.id()
        );
    }
}
