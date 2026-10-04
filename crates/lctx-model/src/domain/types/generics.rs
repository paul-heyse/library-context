//! Native receiver specialization is located evidence, never a global callable alias fact.
use super::*;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum TypeVariance {
    Covariant = 0,
    Contravariant = 1,
    Invariant = 2,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name="generic_specialization_observations",invariants=generic_invariants)]
#[assertion(support=GenericSpecializationSupport,name="generic_specialization_supports",family=FactFamily::Types,subjects(scope,site,receiver,variable,argument))]
pub struct GenericSpecializationObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub site: Id<Occurrence>,
    #[model(key)]
    pub declaration: Id<NativeSignatureObservation>,
    #[model(key)]
    pub variable: Id<TypeVariable>,
    #[model(key)]
    pub argument: Id<TypeTerm>,
    pub scope: Id<CoverageScope>,
    #[model(key)]
    pub receiver: Id<TypeTerm>,
}
fn generic_invariants() -> Vec<Invariant> {
    vec![Invariant {
        name: "generic_specialization_basis",
        inputs: vec![
            ValidationInput::of::<AssertionQualification>(&["id"]),
            ValidationInput::of::<calls::CallSyntax>(&["id"]),
            ValidationInput::of::<calls::Signature>(&["id"]),
            ValidationInput::of::<calls::ProviderSymbol>(&["id"]),
            ValidationInput::of::<TypeVariable>(&["id"]),
            ValidationInput::of::<NativeSignatureObservation>(&["id"]),
            ValidationInput::of::<GenericSpecializationObservation>(&["id"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(GenericCheck {
                charge: StateCharge::new(budget, "generic_specialization_basis"),
                ..Default::default()
            })
        }),
    }]
}
#[derive(Default)]
struct GenericCheck {
    charge: StateCharge,
    sites: ChargedSet<(Id<Occurrence>, Id<AnalysisContext>)>,
    signatures: ChargedMap<Id<calls::Signature>, calls::Signature>,
    symbols: ChargedMap<Id<calls::ProviderSymbol>, calls::ProviderSymbol>,
    variables: ChargedMap<Id<TypeVariable>, TypeVariable>,
    qualifications: ChargedMap<Id<AssertionQualification>, AssertionQualification>,
    declarations: ChargedMap<Id<NativeSignatureObservation>, NativeSignatureObservation>,
}
impl InvariantCheck for GenericCheck {
    fn visit(&mut self, name: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        if name == AssertionQualification::NAME {
            for q in AssertionQualification::decode(batch)? {
                self.qualifications.insert(&mut self.charge, q.id(), q)?;
            }
        } else if name == calls::CallSyntax::NAME {
            for row in calls::CallSyntax::decode(batch)? {
                let q = self
                    .qualifications
                    .get(&row.qualification)
                    .ok_or_else(|| invalid("generic call qualification absent"))?;
                self.sites
                    .insert(&mut self.charge, (row.callee, q.context))?;
            }
        } else if name == calls::Signature::NAME {
            for row in calls::Signature::decode(batch)? {
                self.signatures.insert(&mut self.charge, row.id(), row)?;
            }
        } else if name == calls::ProviderSymbol::NAME {
            for row in calls::ProviderSymbol::decode(batch)? {
                self.symbols.insert(&mut self.charge, row.id(), row)?;
            }
        } else if name == TypeVariable::NAME {
            for row in TypeVariable::decode(batch)? {
                self.variables.insert(&mut self.charge, row.id(), row)?;
            }
        } else if name == NativeSignatureObservation::NAME {
            for row in NativeSignatureObservation::decode(batch)? {
                self.declarations.insert(&mut self.charge, row.id(), row)?;
            }
        } else if name == GenericSpecializationObservation::NAME {
            for row in GenericSpecializationObservation::decode(batch)? {
                let q = self
                    .qualifications
                    .get(&row.qualification)
                    .ok_or_else(|| invalid("specialization qualification absent"))?;
                let declaration = self
                    .declarations
                    .get(&row.declaration)
                    .ok_or_else(|| invalid("specialization native signature origin absent"))?;
                let dq = self
                    .qualifications
                    .get(&declaration.qualification)
                    .ok_or_else(|| invalid("generic declaration qualification absent"))?;
                let signature = self
                    .signatures
                    .get(&declaration.signature)
                    .ok_or_else(|| invalid("generic native signature absent"))?;
                let symbol = self
                    .symbols
                    .get(&signature.symbol)
                    .ok_or_else(|| invalid("generic declaration symbol absent"))?;
                let variable = self
                    .variables
                    .get(&row.variable)
                    .ok_or_else(|| invalid("generic native binder absent"))?;
                if (variable.provider, variable.context) != (symbol.provider, symbol.context) {
                    return Err(invalid(
                        "specialization binder crosses native declaration provider/context",
                    ));
                }
                if !self.sites.contains(&(row.site, q.context)) {
                    return Err(invalid("specialization site is not an actual callee"));
                }
                if row.scope != q.scope || q.context != dq.context {
                    return Err(invalid("specialization crosses native signature context"));
                }
            }
        } else {
            return Err(invalid("undeclared generic specialization input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        Ok(())
    }
}
