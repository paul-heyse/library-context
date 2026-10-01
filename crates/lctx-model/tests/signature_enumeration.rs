use lctx_model::domain::{assertion::*, attribution::*, calls::*, resources::ResourceBudget, *};

fn nominal<T>(value: u8) -> Id<T> {
    serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<_, serde::de::value::Error>::new([value; 16].into_iter())).unwrap()
}
fn fixture() -> (AssertionQualification, ProviderSymbol, Vec<Signature>) {
    let q = AssertionQualification {
        context: nominal(1), scope: nominal(2), condition: nominal(3),
        modality: Modality::Definite, approximation: Approximation::Exact,
    };
    let symbol = ProviderSymbol {
        provider: nominal(4), context: q.context, module: nominal(5), native_key: "F:cast".into(),
        name: "cast".into(), kind: SymbolKind::Function,
    };
    let signatures = (0..3).map(|i| Signature::new(&q, symbol.id(), i, SignatureForm::List, &[]).unwrap().0).collect();
    (q, symbol, signatures)
}
fn check(q: &AssertionQualification, symbol: &ProviderSymbol, signatures: &[Signature], headers: &[SignatureEnumerationObservation], members: &[SignatureEnumerationMember]) -> Result<(), ModelError> {
    let budget = ResourceBudget::fixed(1 << 20)?;
    let invariant = SignatureEnumerationObservation::invariants().remove(0);
    let mut check = (invariant.create)(&budget);
    for (name, batch) in [
        (AssertionQualification::NAME, AssertionQualification::encode(std::slice::from_ref(q))?),
        (ProviderSymbol::NAME, ProviderSymbol::encode(std::slice::from_ref(symbol))?),
        (Signature::NAME, Signature::encode(signatures)?),
        (SignatureEnumerationObservation::NAME, SignatureEnumerationObservation::encode(headers)?),
        (SignatureEnumerationMember::NAME, SignatureEnumerationMember::encode(members)?),
    ] { check.visit(name, &batch)?; }
    let result = check.finish();
    assert_eq!(budget.reserved(), 0);
    result
}
#[test]
fn complete_native_export_is_independent_of_signature_bindability() {
    let (q, symbol, mut signatures) = fixture();
    signatures[2] = Signature::new(&q, symbol.id(), 2, SignatureForm::NativeUnavailable, &[]).unwrap().0;
    let (header, members) = SignatureEnumerationObservation::new(&q, symbol.id(), signatures.iter(), true).unwrap();
    check(&q, &symbol, &signatures, &[header], &members).unwrap();
    let (empty, members) = SignatureEnumerationObservation::new(&q, symbol.id(), [].iter(), true).unwrap();
    check(&q, &symbol, &[], &[empty], &members).unwrap();
}
#[test]
fn missing_or_coupled_subset_members_cannot_claim_the_export() {
    let (q, symbol, signatures) = fixture();
    let (header, members) = SignatureEnumerationObservation::new(&q, symbol.id(), signatures.iter(), true).unwrap();
    check(&q, &symbol, &signatures, std::slice::from_ref(&header), &members).unwrap();
    assert!(check(&q, &symbol, &signatures, std::slice::from_ref(&header), &members[..2]).is_err());
    assert!(check(&q, &symbol, &signatures[..2], &[header], &members[..2]).is_err());
    let (subset, members) = SignatureEnumerationObservation::new(&q, symbol.id(), signatures[..2].iter(), true).unwrap();
    assert!(check(&q, &symbol, &signatures, &[subset], &members).is_err());
}
#[test]
fn foreign_owners_order_and_orphan_members_are_refused() {
    let (q, symbol, signatures) = fixture();
    assert!(SignatureEnumerationObservation::new(&q, symbol.id(), signatures[1..].iter(), true).is_err());
    assert!(SignatureEnumerationObservation::new(&q, nominal(9), signatures.iter(), true).is_err());
    let (mut header, members) = SignatureEnumerationObservation::new(&q, symbol.id(), signatures.iter(), true).unwrap();
    assert!(check(&q, &symbol, &signatures, &[], &members).is_err());
    header.scope = nominal(9);
    assert!(check(&q, &symbol, &signatures, &[header], &[]).is_err());
}
