//! Native callable origins and unlocated typed ports. Source occurrence types keep their owner.
use super::*;
use crate::domain::calls::{Signature, SignatureParameter};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum NativeReceiver {
    Unbound = 0,
    Instance = 1,
    Class = 2,
    Property = 3,
    Unknown = 4,
}
/// Availability comes from live native function metadata, never source decorator spelling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum CallableDeprecation {
    Unavailable = 0,
    NotDeprecated = 1,
    Deprecated = 2,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion, serde::Serialize, serde::Deserialize)]
#[model(name = "native_signature_observations", invariant_refs = signature_port_invariants_refs)]
#[assertion(support = NativeSignatureSupport, name = "native_signature_supports", family = FactFamily::Types, subjects(scope, term, family), referents(implementation, metadata_origin))]
pub struct NativeSignatureObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub signature: Id<Signature>,
    pub scope: Id<CoverageScope>,
    pub term: Id<TypeTerm>,
    pub family: Option<Id<TypeTerm>>,
    /// Native function identity, when present. This never grants runtime body admission.
    pub implementation: Option<Id<ProviderSymbol>>,
    /// Exact native metadata function identity, independent of overload runtime implementation.
    /// Intrinsic metadata can be available without a named provider symbol.
    pub metadata_origin: Option<Id<ProviderSymbol>>,
    pub deprecation: CallableDeprecation,
    /// Raw native optional message: empty, whitespace and unavailable messages are distinct.
    pub deprecation_message: Option<String>,
    pub receiver: NativeReceiver,
    pub complete: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum, serde::Serialize, serde::Deserialize)]
#[model(name = "signature_type_subjects")]
pub enum SignatureTypeSubject {
    #[model(code = 0)]
    Parameter { parameter: Id<SignatureParameter> },
    #[model(code = 1)]
    Return { signature: Id<Signature> },
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion, serde::Serialize, serde::Deserialize)]
#[model(name = "signature_type_observations")]
#[assertion(support = SignatureTypeSupport, name = "signature_type_supports", family = FactFamily::Types, subjects(scope, term))]
pub struct SignatureTypeObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub subject: Id<SignatureTypeSubject>,
    #[model(key)]
    pub term: Id<TypeTerm>,
    pub scope: Id<CoverageScope>,
}
pub(crate) fn signature_port_invariants() -> Vec<Invariant> {
    vec![Invariant {
        revision: 1,
        name: "native_signature_ports",
        inputs: vec![
            ValidationInput::of::<Signature>(&["id"]),
            ValidationInput::of::<SignatureParameter>(&["id"]),
            ValidationInput::of::<SignatureTypeSubject>(&["id"]),
            ValidationInput::of::<NativeSignatureObservation>(&["id"]),
            ValidationInput::of::<SignatureTypeObservation>(&["id"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(PortCheck {
                charge: StateCharge::new(budget, "native_signature_ports"),
                ..Default::default()
            })
        }),
    }]
}
#[derive(Default)]
struct PortCheck {
    charge: StateCharge,
    signatures: ChargedMap<Id<Signature>, Signature>,
    parameters: ChargedMap<Id<SignatureParameter>, Id<Signature>>,
    subjects: ChargedMap<Id<SignatureTypeSubject>, Id<Signature>>,
    native: ChargedSet<Id<Signature>>,
}
impl InvariantCheck for PortCheck {
    fn visit(&mut self, name: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        if name == Signature::NAME {
            for r in Signature::decode(batch)? {
                self.signatures.insert(&mut self.charge, r.id(), r)?;
            }
        } else if name == SignatureParameter::NAME {
            for r in SignatureParameter::decode(batch)? {
                self.parameters
                    .insert(&mut self.charge, r.id(), r.signature)?;
            }
        } else if name == SignatureTypeSubject::NAME {
            for r in SignatureTypeSubject::decode(batch)? {
                let signature = match &r {
                    SignatureTypeSubject::Parameter { parameter } => *self
                        .parameters
                        .get(parameter)
                        .ok_or_else(|| invalid("typed slot parameter absent"))?,
                    SignatureTypeSubject::Return { signature } => *signature,
                };
                self.subjects.insert(&mut self.charge, r.id(), signature)?;
            }
        } else if name == NativeSignatureObservation::NAME {
            for r in NativeSignatureObservation::decode(batch)? {
                let s = self
                    .signatures
                    .get(&r.signature)
                    .ok_or_else(|| invalid("native signature absent"))?;
                if s.qualification != r.qualification
                    || s.scope != r.scope
                    || s.native != Some(r.term)
                    || s.role.runtime_source()
                    || (r.complete && s.form != crate::domain::calls::SignatureForm::List)
                {
                    return Err(invalid(
                        "native signature role, origin or qualification mismatch",
                    ));
                }
                if (r.deprecation == CallableDeprecation::Unavailable
                    && r.metadata_origin.is_some())
                    || (r.deprecation != CallableDeprecation::Deprecated
                        && r.deprecation_message.is_some())
                {
                    return Err(invalid(
                        "callable deprecation availability contradicts origin or message",
                    ));
                }
                self.native.insert(&mut self.charge, r.signature)?;
            }
        } else if name == SignatureTypeObservation::NAME {
            for r in SignatureTypeObservation::decode(batch)? {
                let s = self
                    .subjects
                    .get(&r.subject)
                    .and_then(|s| self.signatures.get(s))
                    .ok_or_else(|| invalid("typed port signature absent"))?;
                if s.qualification != r.qualification || s.scope != r.scope {
                    return Err(invalid("typed port crosses signature qualification"));
                }
            }
        } else {
            return Err(invalid("undeclared signature port input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        for (id, signature) in self.signatures.iter() {
            if !signature.role.runtime_source() && !self.native.contains(id) {
                return Err(invalid(
                    "native signature role lacks native origin observation",
                ));
            }
        }
        Ok(())
    }
}

pub(crate) fn signature_port_invariants_refs() -> Vec<&'static str> {
    vec!["native_signature_ports"]
}
