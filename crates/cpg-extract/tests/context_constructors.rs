//! Native constructor contracts; these facts alone do not establish protocol execution.
#[path = "typed_driver/mod.rs"]
mod typed_driver;
use lctx_model::domain::{calls::*, source::*, symbols::SymbolObservation, *};
use typed_driver::{files, rows};
inspector!(Facts, CallTarget);

#[tokio::test]
async fn captured_context_constructors_keep_all_overloads_and_unknown_initializers() {
    let input = files("phase4_context_constructors");
    let tables = typed_driver::Tables::default();
    typed_driver::run_behavioral(&input, Facts(tables.clone()))
        .await
        .unwrap();
    let symbols = rows::<ProviderSymbol>(&tables);
    let modules = rows::<ProviderModule>(&tables);
    let observations = rows::<SymbolObservation>(&tables);
    let destinations = rows::<CallDestination>(&tables);
    let occurrences = rows::<Occurrence>(&tables);
    let receivers = rows::<Receiver>(&tables);
    let signatures = rows::<Signature>(&tables);
    let parameters = rows::<SignatureParameter>(&tables);
    let shapes = rows::<ParameterShape>(&tables);
    let enumerations = rows::<SignatureEnumerationObservation>(&tables);
    let members = rows::<SignatureEnumerationMember>(&tables);
    let targets = rows::<CallTarget>(&tables);
    for (source, class, expected_variants) in [
        (
            "nullcontext(7)",
            "nullcontext",
            vec![vec!["self"], vec!["self", "enter_result"]],
        ),
        (
            "nullcontext()",
            "nullcontext",
            vec![vec!["self"], vec!["self", "enter_result"]],
        ),
        (
            "suppress(TypeError)",
            "suppress",
            vec![vec!["self", "exceptions"]],
        ),
    ] {
        let site = occurrences
            .iter()
            .find(|o| {
                o.syntax_kind == SyntaxKind::ExprCall
                    && &input["cases.py"][o.start as usize..o.end as usize] == source.as_bytes()
            })
            .unwrap();
        for (phase, module, parent, name) in [
            (CallPhase::New, "builtins", "object", "__new__"),
            (CallPhase::Init, "contextlib", class, "__init__"),
        ] {
            let candidates = targets
                .iter()
                .filter(|t| t.site == site.id() && t.phase == phase)
                .collect::<Vec<_>>();
            assert_eq!(candidates.len(), 1, "{source} {phase:?}");
            let target = candidates[0];
            let CallDestination::Resolved { symbol } = destinations
                .iter()
                .find(|d| d.id() == target.destination)
                .unwrap()
            else {
                panic!("exact constructor destination absent")
            };
            let symbol = symbols.iter().find(|s| s.id() == *symbol).unwrap();
            assert_eq!(symbol.name, name);
            assert!(
                matches!(modules.iter().find(|m| m.id() == symbol.module).unwrap(), ProviderModule::Bundled {name, ..} if name == module)
            );
            let ancestry = observations
                .iter()
                .filter(|o| o.symbol == symbol.id())
                .collect::<Vec<_>>();
            assert!(!ancestry.is_empty());
            assert!(
                ancestry
                    .iter()
                    .all(|o| symbols.iter().any(|s| Some(s.id()) == o.parent
                        && s.name == parent
                        && s.module == symbol.module))
            );
            let receiver = receivers
                .iter()
                .find(|r| r.id() == target.receiver)
                .unwrap();
            if phase == CallPhase::Init {
                assert!(matches!(
                    receiver,
                    Receiver::Unknown {
                        reason: obligation::ObligationKind::AmbiguousBinding
                    }
                ));
                assert_eq!(target.passing, Some(ReceiverPassing::Object));
            } else {
                assert!(matches!(receiver, Receiver::None));
                assert_eq!(target.passing, Some(ReceiverPassing::NotPassed));
            }
            let headers = enumerations
                .iter()
                .filter(|e| e.symbol == symbol.id())
                .collect::<Vec<_>>();
            assert_eq!(headers.len(), 1);
            assert!(headers[0].complete);
            let mut variants = members
                .iter()
                .filter(|m| m.enumeration == headers[0].id())
                .collect::<Vec<_>>();
            variants.sort_by_key(|m| m.ordinal);
            let actual = variants
                .iter()
                .map(|m| {
                    let signature = signatures.iter().find(|s| s.id() == m.signature).unwrap();
                    assert_eq!(signature.form, SignatureForm::List);
                    let mut slots = parameters
                        .iter()
                        .filter(|p| p.signature == signature.id())
                        .collect::<Vec<_>>();
                    slots.sort_by_key(|p| p.ordinal);
                    slots
                        .iter()
                        .map(|p| {
                            shapes
                                .iter()
                                .find(|s| s.id() == p.shape)
                                .unwrap()
                                .name
                                .as_ref()
                                .unwrap()
                                .as_str()
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            assert_eq!(
                actual,
                if phase == CallPhase::New {
                    vec![vec!["cls"]]
                } else {
                    expected_variants.clone()
                }
            );
        }
    }
}
