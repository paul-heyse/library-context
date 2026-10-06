//! Finite authored structural vocabulary uses the existing predicate evaluator.
use lctx_model::domain::{selection::*, *};
pub fn questions(data: &classification::ClassificationData) -> Vec<(Facet, String, Predicate)> {
    let mut values = std::collections::BTreeMap::new();
    let mut add = |facet: Facet, text: String, predicate: Predicate| {
        values.insert((facet, text), predicate);
    };
    for module in data.source.core.modules.iter() {
        add(
            Facet::Module,
            module.qualified_name.clone(),
            Predicate::PublicModule {
                module: module.qualified_name.clone(),
            },
        );
    }
    for shape in data.facts.shapes.iter() {
        if let Some(name) = &shape.name {
            add(
                Facet::Parameter,
                name.as_str().to_owned(),
                Predicate::DeclaresParameter {
                    name: name.as_str().to_owned(),
                },
            );
        }
    }
    for kind in [
        MemberKind::Function,
        MemberKind::Method,
        MemberKind::Class,
        MemberKind::Property,
        MemberKind::Module,
        MemberKind::Variable,
        MemberKind::Unknown,
    ] {
        add(
            Facet::Kind,
            format!("{kind:?}"),
            Predicate::MemberKind { kind },
        );
    }
    for observation in data.source.core.signature_types.iter() {
        let Some(subject) = data.facts.signature_subjects.get(observation.subject) else {
            continue;
        };
        match subject {
            types::SignatureTypeSubject::Parameter { parameter } => {
                if let Some(p) = data.facts.signature_parameters.get(*parameter)
                    && let Some(shape) = data.facts.shapes.get(p.shape)
                    && let Some(name) = &shape.name
                {
                    add(
                        Facet::ParameterType,
                        format!("{}:{}", name.as_str(), observation.term.hex()),
                        Predicate::ParameterType {
                            name: name.as_str().to_owned(),
                            r#type: StructuralType::CanonicalTerm {
                                term: observation.term,
                            },
                        },
                    );
                }
            }
            types::SignatureTypeSubject::Return { signature } => {
                if let Some(signature) = data.facts.signatures.get(*signature) {
                    add(
                        Facet::Returns,
                        format!("{:?}:{}", signature.role, observation.term.hex()),
                        Predicate::VariantReturnType {
                            role: signature.role,
                            r#type: StructuralType::CanonicalTerm {
                                term: observation.term,
                            },
                        },
                    );
                }
            }
        }
    }
    for flag in [true, false] {
        add(
            Facet::Async,
            flag.to_string(),
            Predicate::FacetMembership {
                facet: Facet::Async,
                value: FacetValue::Async { asynchronous: flag },
            },
        );
    }
    for kind in [
        ClassFacet::FinalDeclaration,
        ClassFacet::Protocol,
        ClassFacet::RuntimeCheckable,
        ClassFacet::Enumeration,
        ClassFacet::ExplicitlyAbstract,
        ClassFacet::AbstractMembers,
        ClassFacet::ExplicitSlots,
    ] {
        for present in [true, false] {
            add(
                Facet::ClassMetadata,
                format!("{kind:?}:{present}"),
                Predicate::FacetMembership {
                    facet: Facet::ClassMetadata,
                    value: FacetValue::ClassMetadata {
                        trait_kind: kind,
                        present,
                    },
                },
            );
        }
    }
    for observation in data
        .facts
        .type_observations
        .iter()
        .filter(|o| o.role == types::TypeRole::Raised)
    {
        add(
            Facet::Raises,
            observation.term.hex(),
            Predicate::FacetMembership {
                facet: Facet::Raises,
                value: FacetValue::RaisedClass {
                    r#type: StructuralType::CanonicalTerm {
                        term: observation.term,
                    },
                },
            },
        );
    }
    for role in [
        calls::SignatureRole::Source,
        calls::SignatureRole::EffectiveTyped,
        calls::SignatureRole::Synthesized,
        calls::SignatureRole::Stub,
    ] {
        for deprecated in [true, false] {
            add(
                Facet::Deprecation,
                format!("{role:?}:{deprecated}"),
                Predicate::FacetMembership {
                    facet: Facet::Deprecation,
                    value: FacetValue::Deprecation { role, deprecated },
                },
            );
        }
    }
    values
        .into_iter()
        .map(|((facet, text), predicate)| (facet, text, predicate))
        .collect()
}
