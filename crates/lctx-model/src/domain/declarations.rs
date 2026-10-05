//! Provider-attributed links from native symbols and signature parameters to the occurrences that
//! declare them. Composition maps a callee's formal places, which are rooted at parameter
//! occurrences, through these links to the binder's signature parameters. A missing link is
//! refused; nothing is recovered by matching names.
use super::charged::{ChargedMap, ChargedSet, StateCharge};
use super::{
    assertion::AssertionQualification,
    attribution::{FactFamily, Provider, ProviderRun},
    calls::{ProviderSymbol, Signature, SignatureParameter, SymbolKind},
    source::{Occurrence, OccurrenceRole, SyntaxKind},
    *,
};
use crate::{Assertion, Domain};

/// A provider asserts that `symbol` is declared by the definition occurrence `declaration`.
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion, serde::Serialize, serde::Deserialize)]
#[model(name = "symbol_declarations", invariant_refs = declaration_invariants_refs)]
#[assertion(support = SymbolDeclarationSupport, name = "symbol_declaration_supports", family = FactFamily::Signatures, subjects(declaration))]
pub struct SymbolDeclaration {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub symbol: Id<ProviderSymbol>,
    #[model(key)]
    pub declaration: Id<Occurrence>,
}
/// A provider asserts that a signature parameter is declared by one parameter occurrence.
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion, serde::Serialize, serde::Deserialize)]
#[model(name = "parameter_declarations")]
#[assertion(support = ParameterDeclarationSupport, name = "parameter_declaration_supports", family = FactFamily::Signatures, subjects(declaration))]
pub struct ParameterDeclaration {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub parameter: Id<SignatureParameter>,
    #[model(key)]
    pub declaration: Id<Occurrence>,
}
fn invalid(message: &str) -> ModelError {
    ModelError::Invalid(message.into())
}

pub(crate) fn declaration_invariants() -> Vec<Invariant> {
    vec![Invariant {
        revision: 1,
        name: "declaration_links",
        inputs: vec![
            ValidationInput::of::<Occurrence>(&["id"]),
            ValidationInput::of::<AssertionQualification>(&["id"]),
            ValidationInput::of::<ProviderSymbol>(&["id"]),
            ValidationInput::of::<Signature>(&["id"]),
            ValidationInput::of::<SignatureParameter>(&["id"]),
            ValidationInput::of::<ProviderRun>(&["id"]),
            ValidationInput::of::<SymbolDeclaration>(&["id"]),
            ValidationInput::of::<ParameterDeclaration>(&["id"]),
            ValidationInput::of::<SymbolDeclarationSupport>(&["id"]),
            ValidationInput::of::<ParameterDeclarationSupport>(&["id"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(DeclarationCheck {
                charge: StateCharge::new(budget, "declaration_links"),
                ..Default::default()
            })
        }),
    }]
}
#[derive(Default)]
struct DeclarationCheck {
    charge: StateCharge,
    occurrences: ChargedMap<Id<Occurrence>, Occurrence>,
    qualifications: ChargedMap<Id<AssertionQualification>, Id<super::attribution::AnalysisContext>>,
    symbols: ChargedMap<Id<ProviderSymbol>, ProviderSymbol>,
    signatures: ChargedMap<Id<Signature>, Id<ProviderSymbol>>,
    parameters: ChargedMap<Id<SignatureParameter>, Id<Signature>>,
    runs: ChargedMap<Id<ProviderRun>, Id<Provider>>,
    /// Declaring occurrences per (qualification, symbol).
    declared: DeclarationIndex,
    symbol_links: ChargedMap<Id<SymbolDeclaration>, Id<ProviderSymbol>>,
    parameter_links: ChargedMap<Id<ParameterDeclaration>, Id<ProviderSymbol>>,
    /// One occurrence per parameter and one parameter per occurrence, within a qualified signature.
    by_parameter: ChargedSet<(Id<AssertionQualification>, Id<SignatureParameter>)>,
    by_occurrence: ChargedSet<(Id<AssertionQualification>, Id<Signature>, Id<Occurrence>)>,
}
impl DeclarationCheck {
    fn occurrence(&self, id: Id<Occurrence>) -> Result<&Occurrence, ModelError> {
        self.occurrences
            .get(&id)
            .ok_or_else(|| invalid("declaration occurrence absent"))
    }
    fn symbol(&self, id: Id<ProviderSymbol>) -> Result<&ProviderSymbol, ModelError> {
        self.symbols
            .get(&id)
            .ok_or_else(|| invalid("declared symbol absent"))
    }
    fn in_context(
        &self,
        qualification: Id<AssertionQualification>,
        symbol: &ProviderSymbol,
    ) -> Result<(), ModelError> {
        if self.qualifications.get(&qualification) != Some(&symbol.context) {
            return Err(invalid(
                "declaration qualification differs from its symbol's context",
            ));
        }
        Ok(())
    }
    fn native(&self, symbol: Id<ProviderSymbol>, run: Id<ProviderRun>) -> Result<(), ModelError> {
        if self.runs.get(&run) != Some(&self.symbol(symbol)?.provider) {
            return Err(invalid("declaration support belongs to another provider"));
        }
        Ok(())
    }
}
impl InvariantCheck for DeclarationCheck {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if relation == Occurrence::NAME {
            for row in Occurrence::decode(batch)? {
                self.occurrences.insert(&mut self.charge, row.id(), row)?;
            }
        } else if relation == AssertionQualification::NAME {
            for row in AssertionQualification::decode(batch)? {
                self.qualifications
                    .insert(&mut self.charge, row.id(), row.context)?;
            }
        } else if relation == ProviderSymbol::NAME {
            for row in ProviderSymbol::decode(batch)? {
                self.symbols.insert(&mut self.charge, row.id(), row)?;
            }
        } else if relation == Signature::NAME {
            for row in Signature::decode(batch)? {
                self.signatures
                    .insert(&mut self.charge, row.id(), row.symbol)?;
            }
        } else if relation == SignatureParameter::NAME {
            for row in SignatureParameter::decode(batch)? {
                self.parameters
                    .insert(&mut self.charge, row.id(), row.signature)?;
            }
        } else if relation == ProviderRun::NAME {
            for row in ProviderRun::decode(batch)? {
                self.runs.insert(&mut self.charge, row.id(), row.provider)?;
            }
        } else if relation == SymbolDeclaration::NAME {
            for row in SymbolDeclaration::decode(batch)? {
                let symbol = self.symbol(row.symbol)?;
                self.in_context(row.qualification, symbol)?;
                let kind = self.occurrence(row.declaration)?.syntax_kind;
                let matches = match symbol.kind {
                    SymbolKind::Function | SymbolKind::Method => {
                        matches!(kind, SyntaxKind::StmtFunctionDef | SyntaxKind::ExprLambda)
                    }
                    SymbolKind::Class | SymbolKind::ClassBody => kind == SyntaxKind::StmtClassDef,
                    SymbolKind::Module | SymbolKind::ModuleBody => kind == SyntaxKind::ModModule,
                    SymbolKind::DecoratorApplication => kind == SyntaxKind::StmtFunctionDef,
                    SymbolKind::Variable | SymbolKind::Unknown => false,
                };
                if !matches {
                    return Err(invalid(
                        "declaring occurrence does not define the symbol's kind",
                    ));
                }
                self.declared.update(
                    &mut self.charge,
                    (row.qualification, row.symbol),
                    |declarations| declarations.push(row.declaration),
                )?;
                self.symbol_links
                    .insert(&mut self.charge, row.id(), row.symbol)?;
            }
        } else if relation == ParameterDeclaration::NAME {
            for row in ParameterDeclaration::decode(batch)? {
                let signature = *self
                    .parameters
                    .get(&row.parameter)
                    .ok_or_else(|| invalid("declared parameter absent"))?;
                let symbol_id = *self
                    .signatures
                    .get(&signature)
                    .ok_or_else(|| invalid("parameter signature absent"))?;
                self.in_context(row.qualification, self.symbol(symbol_id)?)?;
                let parameter = self.occurrence(row.declaration)?;
                if !matches!(
                    parameter.syntax_kind,
                    SyntaxKind::Parameter | SyntaxKind::ParameterWithDefault
                ) || parameter.role != OccurrenceRole::Parameter
                {
                    return Err(invalid(
                        "parameter declaration is not a parameter occurrence",
                    ));
                }
                // The parameter must sit structurally inside a declaration of its own symbol.
                let owners = self
                    .declared
                    .get(&(row.qualification, symbol_id))
                    .ok_or_else(|| invalid("parameter symbol has no declaration"))?;
                let inside = owners.iter().any(|owner| {
                    self.occurrences.get(owner).is_some_and(|owner| {
                        owner.source == parameter.source
                            && parameter.structural_path.len() > owner.structural_path.len()
                            && parameter
                                .structural_path
                                .starts_with(&owner.structural_path)
                    })
                });
                if !inside {
                    return Err(invalid(
                        "parameter occurrence lies outside its symbol's declaration",
                    ));
                }
                if !self
                    .by_parameter
                    .insert(&mut self.charge, (row.qualification, row.parameter))?
                    || !self.by_occurrence.insert(
                        &mut self.charge,
                        (row.qualification, signature, row.declaration),
                    )?
                {
                    return Err(invalid(
                        "parameter declarations must be one-to-one within a signature",
                    ));
                }
                self.parameter_links
                    .insert(&mut self.charge, row.id(), symbol_id)?;
            }
        } else if relation == SymbolDeclarationSupport::NAME {
            for row in SymbolDeclarationSupport::decode(batch)? {
                self.native(
                    *self
                        .symbol_links
                        .get(&row.assertion)
                        .ok_or_else(|| invalid("supported symbol declaration absent"))?,
                    row.run,
                )?;
            }
        } else if relation == ParameterDeclarationSupport::NAME {
            for row in ParameterDeclarationSupport::decode(batch)? {
                self.native(
                    *self
                        .parameter_links
                        .get(&row.assertion)
                        .ok_or_else(|| invalid("supported parameter declaration absent"))?,
                    row.run,
                )?;
            }
        } else {
            return Err(invalid("undeclared declaration validation input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        Ok(())
    }
}

type DeclarationIndex =
    ChargedMap<(Id<AssertionQualification>, Id<ProviderSymbol>), Vec<Id<Occurrence>>>;

pub(crate) fn declaration_invariants_refs() -> Vec<&'static str> {
    vec!["declaration_links"]
}
