//! Immutable native premise inventory over actual attributed assertions/supports.
use super::invalid;
use crate::domain::{
    assertion::{Assertion, AssertionQualification, Support},
    attribution::*,
    *,
};
use crate::{Domain, DomainSum};

// This finite inventory generates its pair enum and both producer/validator adapters together.
// Codes are append-only; the native assertion and support declarations remain their owners.
macro_rules! native_pairs {
    ($($code:literal: $variant:ident => $assertion:ty, $support:ty;)*) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
        #[model(name = "native_analysis_premises", rule = "native_analysis_premise")]
        pub enum NativeAssertionPremise {
            $(#[model(code = $code)] $variant {
                #[model(premise)] assertion: Id<$assertion>,
                #[model(premise)] support: Id<$support>,
            },)*
        }
        impl NativeAssertionPremise {
            pub fn assertion_and_support(&self) -> (derivation::RowRef, derivation::RowRef) {
                match self { $(Self::$variant { assertion, support } =>
                    (derivation::RowRef::of(*assertion), derivation::RowRef::of(*support)),)* }
            }
            pub fn family(&self) -> FactFamily {
                match self { $(Self::$variant { .. } => <$assertion as Assertion>::FAMILY,)* }
            }
        }
        impl NativeInventory {
            pub fn inputs() -> Vec<ValidationInput> {
                let mut inputs = vec![ValidationInput::of::<AssertionQualification>(&["id"]).at_epoch(stages::PublicationBoundary::Facts)];
                $(inputs.extend([ValidationInput::of::<$assertion>(&["id"]),
                    ValidationInput::of::<$support>(&["id"])]);)*
                inputs
            }
            /// Native support validators stay authoritative. The producer must read confirmed
            /// facts with these validators; this projection does not reimplement attribution.
            pub fn stage_inputs(profile: stages::Profile) -> Vec<stages::RelationUse> {
                let mut inputs = vec![stages::RelationUse::stored::<AssertionQualification>().at_epoch(stages::PublicationBoundary::Facts)];
                $(if profile == stages::Profile::Behavioral || <$assertion as Assertion>::FAMILY != FactFamily::Flow {
                    inputs.extend([stages::RelationUse::stored::<$assertion>(),
                        stages::RelationUse::stored::<$support>().validated_by(&[<$support>::NAME])]);
                })*
                inputs
            }
            pub fn visit(&mut self, name: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
                let allowance = batch.get_array_memory_size().checked_mul(4)
                    .and_then(|n| n.checked_add(batch.num_rows().saturating_mul(512)))
                    .ok_or_else(|| invalid("native inventory decode allowance overflow"))?;
                let _decode = self.budget.reserve("native-inventory-decode", allowance)?;
                if name == AssertionQualification::NAME {
                    for row in AssertionQualification::decode(batch)? {
                        self.qualifications.insert(&mut self.charge, row.id())?;
                    }
                    return Ok(());
                }
                $(if name == <$assertion>::NAME {
                    for row in <$assertion>::decode(batch)? {
                        row.validate()?;
                        if self.assertions.insert(&mut self.charge,
                            derivation::RowRef::of(row.id()), row.qualification())?
                            .is_some_and(|previous| previous != row.qualification()) {
                            return Err(ModelError::Conflict(<$assertion>::NAME));
                        }
                    }
                    return Ok(());
                }
                if name == <$support>::NAME {
                    for row in <$support>::decode(batch)? {
                        row.validate()?;
                        let attribution = row.attribution()
                            .ok_or_else(|| invalid("native premise has no native attribution"))?;
                        let premise = NativeAssertionPremise::$variant {
                            assertion: row.assertion(), support: row.id(),
                        };
                        let fidelity = attribution.fidelity;
                        if self.supports.insert(&mut self.charge, premise.id(), (premise, fidelity))?
                            .is_some_and(|(_, previous)| previous != fidelity) {
                            return Err(ModelError::Conflict(<$support>::NAME));
                        }
                    }
                    return Ok(());
                })*
                Err(invalid("undeclared native inventory input"))
            }
        }
    };
}
/// The finite native assertion/support inventory, shared with typed producer adapters.
#[macro_export]
macro_rules! native_analysis_pairs {
    ($callback:ident) => { $callback! {
        0: Use => $crate::domain::flow::FlowUseObservation, $crate::domain::flow::FlowUseSupport;
        1: Definition => $crate::domain::flow::FlowDefinitionObservation, $crate::domain::flow::FlowDefinitionSupport;
        2: Reaching => $crate::domain::flow::FlowReachingObservation, $crate::domain::flow::FlowReachingSupport;
        3: Value => $crate::domain::flow::FlowValueObservation, $crate::domain::flow::FlowValueSupport;
        4: Region => $crate::domain::flow::FlowRegionObservation, $crate::domain::flow::FlowRegionSupport;
        5: Test => $crate::domain::flow::FlowTestObservation, $crate::domain::flow::FlowTestSupport;
        6: Leaf => $crate::domain::flow::FlowTestLeafObservation, $crate::domain::flow::FlowTestLeafSupport;
        7: Signature => $crate::domain::calls::Signature, $crate::domain::calls::SignatureSupport;
        8: CallTarget => $crate::domain::calls::CallTarget, $crate::domain::calls::CallTargetSupport;
        9: ProviderCallSite => $crate::domain::calls::ProviderCallSite, $crate::domain::calls::ProviderCallSiteSupport;
        10: CallSyntax => $crate::domain::calls::CallSyntax, $crate::domain::calls::CallSyntaxSupport;
        11: CallResolution => $crate::domain::calls::CallResolution, $crate::domain::calls::CallResolutionSupport;
        12: SymbolDeclaration => $crate::domain::declarations::SymbolDeclaration, $crate::domain::declarations::SymbolDeclarationSupport;
        13: ParameterDeclaration => $crate::domain::declarations::ParameterDeclaration, $crate::domain::declarations::ParameterDeclarationSupport;
        14: TaskReportObservation => $crate::domain::deployment::TaskReportObservation, $crate::domain::deployment::TaskReportSupport;
        15: DeploymentObservation => $crate::domain::deployment::DeploymentObservation, $crate::domain::deployment::DeploymentSupport;
        16: DocumentObservation => $crate::domain::documents::DocumentObservation, $crate::domain::documents::DocumentSupport;
        17: PassageObservation => $crate::domain::documents::PassageObservation, $crate::domain::documents::PassageSupport;
        18: CodeBlockObservation => $crate::domain::documents::CodeBlockObservation, $crate::domain::documents::CodeBlockSupport;
        19: DocumentLinkObservation => $crate::domain::documents::DocumentLinkObservation, $crate::domain::documents::DocumentLinkSupport;
        20: DocumentMentionObservation => $crate::domain::documents::DocumentMentionObservation, $crate::domain::documents::DocumentMentionSupport;
        21: DocumentComponentObservation => $crate::domain::documents::DocumentComponentObservation, $crate::domain::documents::DocumentComponentSupport;
        22: DocumentAttributeObservation => $crate::domain::documents::DocumentAttributeObservation, $crate::domain::documents::DocumentAttributeSupport;
        23: FlowAttributeLoadObservation => $crate::domain::flow::FlowAttributeLoadObservation, $crate::domain::flow::FlowAttributeLoadSupport;
        24: FlowValuePathObservation => $crate::domain::flow::FlowValuePathObservation, $crate::domain::flow::FlowValuePathSupport;
        25: LexicalScopeObservation => $crate::domain::lexical::LexicalScopeObservation, $crate::domain::lexical::LexicalScopeSupport;
        26: BindingObservation => $crate::domain::lexical::BindingObservation, $crate::domain::lexical::BindingSupport;
        27: ReferenceObservation => $crate::domain::lexical::ReferenceObservation, $crate::domain::lexical::ReferenceSupport;
        28: LexicalResolution => $crate::domain::lexical::LexicalResolution, $crate::domain::lexical::LexicalResolutionSupport;
        29: SyntaxObservation => $crate::domain::source::SyntaxObservation, $crate::domain::source::SyntaxSupport;
        30: SymbolObservation => $crate::domain::symbols::SymbolObservation, $crate::domain::symbols::SymbolSupport;
        31: FunctionTraitObservation => $crate::domain::symbols::FunctionTraitObservation, $crate::domain::symbols::FunctionTraitSupport;
        32: ClassTraitObservation => $crate::domain::symbols::ClassTraitObservation, $crate::domain::symbols::ClassTraitSupport;
        33: ClassAncestryObservation => $crate::domain::symbols::ClassAncestryObservation, $crate::domain::symbols::ClassAncestrySupport;
        34: ParameterAnnotationObservation => $crate::domain::symbols::ParameterAnnotationObservation, $crate::domain::symbols::ParameterAnnotationSupport;
        35: PublicNameObservation => $crate::domain::symbols::PublicNameObservation, $crate::domain::symbols::PublicNameSupport;
        36: ParameterDocObservation => $crate::domain::symbols::ParameterDocObservation, $crate::domain::symbols::ParameterDocSupport;
        37: ModuleResolutionObservation => $crate::domain::symbols::ModuleResolutionObservation, $crate::domain::symbols::ModuleResolutionSupport;
        38: SyntaxPlacement => $crate::domain::syntax::SyntaxPlacement, $crate::domain::syntax::SyntaxPlacementSupport;
        39: SyntaxDetailObservation => $crate::domain::syntax::SyntaxDetailObservation, $crate::domain::syntax::SyntaxDetailSupport;
        40: DeclarationObservation => $crate::domain::syntax::DeclarationObservation, $crate::domain::syntax::DeclarationSupport;
        41: DeclarationDecorator => $crate::domain::syntax::DeclarationDecorator, $crate::domain::syntax::DeclarationDecoratorSupport;
        42: ImportAliasObservation => $crate::domain::syntax::ImportAliasObservation, $crate::domain::syntax::ImportAliasSupport;
        43: DunderAllObservation => $crate::domain::syntax::DunderAllObservation, $crate::domain::syntax::DunderAllSupport;
        44: ParameterSyntaxObservation => $crate::domain::syntax::ParameterSyntaxObservation, $crate::domain::syntax::ParameterSyntaxSupport;
        45: ClassFieldSyntaxObservation => $crate::domain::syntax::ClassFieldSyntaxObservation, $crate::domain::syntax::ClassFieldSyntaxSupport;
        46: TypeObservation => $crate::domain::types::TypeObservation, $crate::domain::types::TypeSupport;
        47: TypePresentation => $crate::domain::types::TypePresentation, $crate::domain::types::TypePresentationSupport;
        48: TypeVariableRestriction => $crate::domain::types::TypeVariableRestriction, $crate::domain::types::TypeRestrictionSupport;
        49: FunctionBodyObservation => $crate::domain::types::FunctionBodyObservation, $crate::domain::types::FunctionBodySupport;
        50: RecordFieldObservation => $crate::domain::types::RecordFieldObservation, $crate::domain::types::RecordFieldSupport;
        51: SignatureEnumerationObservation => $crate::domain::calls::SignatureEnumerationObservation, $crate::domain::calls::SignatureEnumerationSupport;
    } };
}
crate::native_analysis_pairs!(native_pairs);

/// One-shot typed projection of an actual native pair. Every payload is recomputed from
/// the paired assertion/support; it does not select a new attribution or claim.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "native_qualifications", rule = "native_qualification", invariants = inventory_invariants)]
pub struct NativeQualification {
    #[model(key, premise)]
    pub premise: Id<NativeAssertionPremise>,
    pub qualification: Id<AssertionQualification>,
    pub family: FactFamily,
    pub fidelity: Fidelity,
    pub status: super::policy::EvidenceStatus,
}
pub fn relations() -> Vec<Relation> {
    vec![
        Relation::of::<NativeAssertionPremise>(),
        Relation::of::<NativeQualification>(),
    ]
}

/// A charged projection of confirmed native facts. Only compact IDs and original fidelity are
/// retained; callers may stream the resulting pairs instead of retaining another copy.
pub struct NativeInventory {
    budget: resources::ResourceBudget,
    charge: charged::StateCharge,
    qualifications: charged::ChargedSet<Id<AssertionQualification>>,
    assertions: charged::ChargedMap<derivation::RowRef, Id<AssertionQualification>>,
    supports: charged::ChargedMap<Id<NativeAssertionPremise>, (NativeAssertionPremise, Fidelity)>,
}
impl NativeInventory {
    pub fn new(budget: &resources::ResourceBudget) -> Self {
        Self {
            budget: budget.clone(),
            charge: charged::StateCharge::new(budget, "native-inventory"),
            qualifications: Default::default(),
            assertions: Default::default(),
            supports: Default::default(),
        }
    }
    /// Emit exactly one immutable pair/qualification per actual native support. Input order and
    /// repeated identical batches do not affect the domain. An absent assertion/qualification
    /// refuses the complete projection; it never silently removes a required pair.
    pub fn for_each(
        &self,
        mut emit: impl FnMut(&NativeAssertionPremise, NativeQualification) -> Result<(), ModelError>,
    ) -> Result<(), ModelError> {
        for (id, (premise, fidelity)) in self.supports.iter() {
            let (assertion, _) = premise.assertion_and_support();
            let qualification = *self
                .assertions
                .get(&assertion)
                .ok_or_else(|| invalid("native support assertion is absent"))?;
            if !self.qualifications.contains(&qualification) {
                return Err(invalid("native assertion qualification is absent"));
            }
            let family = premise.family();
            emit(
                premise,
                NativeQualification {
                    premise: *id,
                    qualification,
                    family,
                    fidelity: *fidelity,
                    status: super::policy::native_status(family, *fidelity),
                },
            )?;
        }
        Ok(())
    }
    pub fn collect(&self) -> Result<NativeInventoryOutput, ModelError> {
        let mut output = NativeInventoryOutput::new(&self.budget);
        self.for_each(|premise, qualification| {
            output.premises.insert(premise.clone())?;
            output.qualifications.insert(qualification)?;
            Ok(())
        })?;
        Ok(output)
    }
}
pub struct NativeInventoryOutput {
    pub premises: normalized::Rows<NativeAssertionPremise>,
    pub qualifications: normalized::Rows<NativeQualification>,
}
impl NativeInventoryOutput {
    pub fn new(budget: &resources::ResourceBudget) -> Self {
        Self {
            premises: normalized::Rows::new(budget),
            qualifications: normalized::Rows::new(budget),
        }
    }
    pub fn visit(
        &mut self,
        name: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if name == NativeAssertionPremise::NAME {
            self.premises.decode(batch)
        } else if name == NativeQualification::NAME {
            self.qualifications.decode(batch)
        } else {
            Err(invalid("undeclared native inventory output"))
        }
    }
}
fn inventory_invariants() -> Vec<Invariant> {
    let mut inputs = NativeInventory::inputs();
    inputs.extend([
        ValidationInput::of::<NativeAssertionPremise>(&["id"]),
        ValidationInput::of::<NativeQualification>(&["id"]),
    ]);
    vec![Invariant {
        name: "native_inventory_projection",
        inputs,
        create: std::sync::Arc::new(|budget| {
            Box::new(InventoryCheck {
                native: NativeInventory::new(budget),
                observed: NativeInventoryOutput::new(budget),
            })
        }),
    }]
}
struct InventoryCheck {
    native: NativeInventory,
    observed: NativeInventoryOutput,
}
impl InvariantCheck for InventoryCheck {
    fn visit(&mut self, name: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        if name == NativeAssertionPremise::NAME || name == NativeQualification::NAME {
            self.observed.visit(name, batch)
        } else {
            self.native.visit(name, batch)
        }
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        let mut count = 0;
        self.native.for_each(|premise, qualification| {
            if self.observed.premises.get(premise.id()) != Some(premise)
                || self.observed.qualifications.get(qualification.id()) != Some(&qualification)
            {
                return Err(invalid(
                    "native inventory differs from exact assertion/support projection",
                ));
            }
            count += 1;
            Ok(())
        })?;
        if count != self.observed.premises.len() || count != self.observed.qualifications.len() {
            return Err(invalid(
                "native inventory has extraneous or missing members",
            ));
        }
        Ok(())
    }
}
