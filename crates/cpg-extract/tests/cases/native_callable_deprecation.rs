//! Deprecation comes from each actual native callable alternative, with explicit availability.
use crate::typed_driver;
use crate::inspector;
use lctx_model::domain::{calls::*, types::*, *};
use typed_driver::{files, rows};
inspector!(
    Facts,
    lctx_model::domain::calls::Signature,
    lctx_model::domain::types::NativeSignatureObservation,
    lctx_model::domain::calls::ProviderSymbol,
    lctx_model::domain::types::TypeTerm,
    lctx_model::domain::types::TypeSequenceMember
);
#[tokio::test]
async fn native_deprecation_preserves_overload_origin_and_optional_message() {
    let tables = typed_driver::Tables::default();
    typed_driver::run(&files("native_callable_deprecation"), Facts(tables.clone()))
        .await
        .unwrap();
    let signatures = rows::<Signature>(&tables);
    let observations = rows::<NativeSignatureObservation>(&tables);
    let symbols = rows::<ProviderSymbol>(&tables);
    let terms = rows::<TypeTerm>(&tables);
    let members = rows::<TypeSequenceMember>(&tables);
    let signatures_for = |name: &str| -> Vec<&NativeSignatureObservation> {
        observations
            .iter()
            .filter(|observation| {
                let signature = signatures
                    .iter()
                    .find(|s| s.id() == observation.signature)
                    .unwrap();
                signature.role == SignatureRole::EffectiveTyped
                    && symbols
                        .iter()
                        .any(|s| s.id() == signature.symbol && s.name == name)
            })
            .collect()
    };
    let one = |name: &str| {
        let rows = signatures_for(name);
        assert_eq!(rows.len(), 1, "{name}");
        rows[0]
    };
    assert_eq!(one("retired").deprecation, CallableDeprecation::Deprecated);
    assert_eq!(
        one("retired").deprecation_message,
        Some("  use current\n".into())
    );
    assert_eq!(
        one("current").deprecation,
        CallableDeprecation::NotDeprecated
    );
    assert_eq!(
        one("empty_message").deprecation,
        CallableDeprecation::Deprecated
    );
    assert_eq!(
        one("empty_message").deprecation_message,
        Some(String::new())
    );
    assert_eq!(
        one("unavailable_message").deprecation,
        CallableDeprecation::Deprecated
    );
    assert_eq!(one("unavailable_message").deprecation_message, None);
    assert_eq!(one("generic").deprecation, CallableDeprecation::Deprecated);
    assert_eq!(
        one("generic").deprecation_message,
        Some("generic deprecated".into())
    );
    assert_eq!(one("old").deprecation, CallableDeprecation::Deprecated);
    assert_eq!(
        one("old").deprecation_message,
        Some("use the new method".into())
    );
    assert_eq!(
        one("opaque_retired").deprecation,
        CallableDeprecation::Unavailable
    );
    assert!(
        one("opaque_retired").metadata_origin.is_none(),
        "opaque effective value cannot reuse source metadata"
    );
    let convert = signatures_for("convert");
    assert_eq!(convert.len(), 2);
    assert!(
        convert
            .iter()
            .any(|r| r.deprecation == CallableDeprecation::Deprecated
                && r.deprecation_message == Some("use the text overload".into()))
    );
    assert!(
        convert
            .iter()
            .any(|r| r.deprecation == CallableDeprecation::NotDeprecated)
    );
    assert_ne!(
        convert[0].metadata_origin, convert[1].metadata_origin,
        "each alternative has its native function identity"
    );
    assert_eq!(
        convert[0].implementation, convert[1].implementation,
        "runtime implementation remains family identity"
    );
    assert!(convert.iter().all(|r| r.family.is_some()));
    for observation in observations
        .iter()
        .filter(|o| o.deprecation != CallableDeprecation::Unavailable)
    {
        let mut pending = vec![observation.term];
        let mut native_functions = Vec::new();
        while let Some(id) = pending.pop() {
            match terms.iter().find(|t| t.id() == id).unwrap() {
                TypeTerm::Callable { function, .. } => native_functions.extend(function),
                TypeTerm::Generic { body, .. } => pending.push(*body),
                TypeTerm::BoundMethod { function, .. } => pending.push(*function),
                TypeTerm::Overload { signatures, .. } => pending.extend(
                    members
                        .iter()
                        .filter(|m| m.sequence == *signatures)
                        .map(|m| m.child),
                ),
                _ => {}
            }
        }
        if let Some(origin) = observation.metadata_origin {
            assert!(
                native_functions.contains(&origin),
                "metadata belongs to exact native signature origin"
            );
            assert!(symbols.iter().any(|s| s.id() == origin));
        }
    }
}
