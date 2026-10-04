//! Complete native undecorated signature exports, independently of runtime callable identity.
use super::*;

pub const MAX_SIGNATURE_VARIANTS: usize = 4096;

/// A provider's exact enumeration for one symbol. `complete` describes this export only;
/// it does not upgrade family coverage or claim that an undecorated signature executes.
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name = "signature_enumeration_observations", invariants = enumeration_invariants)]
#[assertion(support = SignatureEnumerationSupport, name = "signature_enumeration_supports", family = FactFamily::Signatures, subjects(scope))]
pub struct SignatureEnumerationObservation {
    #[model(key)]
    pub role: SignatureRole,
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub scope: Id<CoverageScope>,
    #[model(key)]
    pub symbol: Id<ProviderSymbol>,
    #[model(key)]
    pub members: ContentHash,
    #[model(key)]
    pub complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "signature_enumeration_members", rule = "signature_enumeration_member", conclusion = enumeration, validate = validate_member)]
pub struct SignatureEnumerationMember {
    #[model(key)]
    pub enumeration: Id<SignatureEnumerationObservation>,
    #[model(key)]
    pub ordinal: i64,
    #[model(premise)]
    pub signature: Id<Signature>,
}
fn validate_member(row: &SignatureEnumerationMember) -> Result<(), ModelError> {
    if row.ordinal < 0 || row.ordinal as usize >= MAX_SIGNATURE_VARIANTS {
        return Err(invalid("signature enumeration ordinal outside work bound"));
    }
    Ok(())
}
fn digest(signatures: impl Iterator<Item = Id<Signature>>) -> ContentHash {
    let mut sink = KeySink::new("native-undecorated-signature-enumeration");
    let mut count = 0i64;
    for signature in signatures {
        count.encode(&mut sink);
        signature.encode(&mut sink);
        count += 1;
    }
    count.encode(&mut sink);
    sink.finish()
}
impl SignatureEnumerationObservation {
    pub fn new<'a>(
        qualification: &AssertionQualification,
        symbol: Id<ProviderSymbol>,
        signatures: impl ExactSizeIterator<Item = &'a Signature> + Clone,
        complete: bool,
    ) -> Result<(Self, Vec<SignatureEnumerationMember>), ModelError> {
        if signatures.len() > MAX_SIGNATURE_VARIANTS {
            return Err(invalid("signature enumeration work limit"));
        }
        let role = signatures
            .clone()
            .next()
            .map_or(SignatureRole::Source, |s| s.role);
        for (ordinal, signature) in signatures.clone().enumerate() {
            if signature.role != role {
                return Err(invalid("signature enumeration mixes roles"));
            }
            if signature.qualification != qualification.id()
                || signature.scope != qualification.scope
                || signature.symbol != symbol
                || signature.variant != ordinal as i64
            {
                return Err(invalid("signature enumeration owner/order mismatch"));
            }
        }
        let row = Self {
            role,
            qualification: qualification.id(),
            scope: qualification.scope,
            symbol,
            members: digest(signatures.clone().map(Record::id)),
            complete,
        };
        let members = signatures
            .enumerate()
            .map(|(ordinal, signature)| SignatureEnumerationMember {
                enumeration: row.id(),
                ordinal: ordinal as i64,
                signature: signature.id(),
            })
            .collect();
        Ok((row, members))
    }
}
fn enumeration_invariants() -> Vec<Invariant> {
    vec![Invariant {
        name: "native_signature_enumeration",
        inputs: vec![
            ValidationInput::of::<AssertionQualification>(&["id"]),
            ValidationInput::of::<ProviderSymbol>(&["id"]),
            ValidationInput::of::<Signature>(&["id"]),
            ValidationInput::of::<SignatureEnumerationObservation>(&["id"]),
            ValidationInput::of::<SignatureEnumerationMember>(&["enumeration", "ordinal"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(EnumerationCheck {
                charge: StateCharge::new(budget, "native_signature_enumeration"),
                ..Default::default()
            })
        }),
    }]
}
type SignatureGroups = ChargedMap<
    (
        Id<AssertionQualification>,
        Id<ProviderSymbol>,
        SignatureRole,
    ),
    BTreeMap<i64, Id<Signature>>,
>;

#[derive(Default)]
struct EnumerationCheck {
    charge: StateCharge,
    qualifications: ChargedMap<Id<AssertionQualification>, AssertionQualification>,
    symbols: ChargedMap<Id<ProviderSymbol>, ProviderSymbol>,
    signatures: SignatureGroups,
    headers: ChargedMap<Id<SignatureEnumerationObservation>, SignatureEnumerationObservation>,
    members: ChargedMap<Id<SignatureEnumerationObservation>, BTreeMap<i64, Id<Signature>>>,
}
impl InvariantCheck for EnumerationCheck {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        let bytes = batch
            .get_array_memory_size()
            .checked_mul(4)
            .and_then(|n| n.checked_add(batch.num_rows().saturating_mul(256)))
            .ok_or_else(|| invalid("signature enumeration decode overflow"))?;
        let _decode = self
            .charge
            .budget()
            .ok_or_else(|| invalid("signature enumeration budget absent"))?
            .reserve("signature-enumeration-decode", bytes)?;
        if relation == AssertionQualification::NAME {
            for row in AssertionQualification::decode(batch)? {
                self.qualifications
                    .insert(&mut self.charge, row.id(), row)?;
            }
        } else if relation == ProviderSymbol::NAME {
            for row in ProviderSymbol::decode(batch)? {
                self.symbols.insert(&mut self.charge, row.id(), row)?;
            }
        } else if relation == Signature::NAME {
            for row in Signature::decode(batch)? {
                let key = (row.qualification, row.symbol, row.role);
                if self
                    .signatures
                    .get(&key)
                    .and_then(|rows| rows.get(&row.variant))
                    .is_some_and(|id| *id != row.id())
                {
                    return Err(invalid("conflicting native signature variant"));
                }
                self.signatures.update(&mut self.charge, key, |rows| {
                    rows.insert(row.variant, row.id());
                })?;
            }
        } else if relation == SignatureEnumerationObservation::NAME {
            for row in SignatureEnumerationObservation::decode(batch)? {
                self.headers.insert(&mut self.charge, row.id(), row)?;
            }
        } else if relation == SignatureEnumerationMember::NAME {
            for row in SignatureEnumerationMember::decode(batch)? {
                validate_member(&row)?;
                if self
                    .members
                    .get(&row.enumeration)
                    .and_then(|rows| rows.get(&row.ordinal))
                    .is_some_and(|id| *id != row.signature)
                {
                    return Err(invalid("conflicting signature enumeration member"));
                }
                self.members
                    .update(&mut self.charge, row.enumeration, |rows| {
                        rows.insert(row.ordinal, row.signature);
                    })?;
            }
        } else {
            return Err(invalid("undeclared signature enumeration input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        let empty = BTreeMap::new();
        for row in self.headers.values() {
            let q = self
                .qualifications
                .get(&row.qualification)
                .ok_or_else(|| invalid("signature enumeration qualification absent"))?;
            let symbol = self
                .symbols
                .get(&row.symbol)
                .ok_or_else(|| invalid("signature enumeration symbol absent"))?;
            if q.scope != row.scope
                || q.context != symbol.context
                || !matches!(symbol.kind, SymbolKind::Function | SymbolKind::Method)
            {
                return Err(invalid("signature enumeration scope/context mismatch"));
            }
            let members = self.members.get(&row.id()).unwrap_or(&empty);
            let actual = self
                .signatures
                .get(&(row.qualification, row.symbol, row.role))
                .unwrap_or(&empty);
            if members.len() > MAX_SIGNATURE_VARIANTS
                || members != actual
                || members
                    .keys()
                    .enumerate()
                    .any(|(i, ordinal)| i as i64 != *ordinal)
                || row.members != digest(members.values().copied())
            {
                return Err(invalid("signature enumeration membership/digest mismatch"));
            }
        }
        if self.members.keys().any(|id| !self.headers.contains_key(id)) {
            return Err(invalid("signature enumeration member owner absent"));
        }
        Ok(())
    }
}
