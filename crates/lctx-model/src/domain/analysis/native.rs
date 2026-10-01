//! Immutable native premise inventory over actual attributed assertions/supports.
use super::invalid;
use crate::domain::{assertion::{Assertion, AssertionQualification, Support}, attribution::*, flow::*, *};
use crate::{Domain, DomainSum};
use crate::domain::calls::{Signature,SignatureSupport};
use crate::domain::calls::{CallTarget,CallTargetSupport};
use crate::domain::calls::{ProviderCallSite,ProviderCallSiteSupport};
use crate::domain::calls::{CallSyntax,CallSyntaxSupport};
use crate::domain::calls::{CallResolution,CallResolutionSupport};
use crate::domain::declarations::{SymbolDeclaration,SymbolDeclarationSupport};
use crate::domain::declarations::{ParameterDeclaration,ParameterDeclarationSupport};
use crate::domain::deployment::{TaskReportObservation,TaskReportSupport};
use crate::domain::deployment::{DeploymentObservation,DeploymentSupport};
use crate::domain::documents::{DocumentObservation,DocumentSupport};
use crate::domain::documents::{PassageObservation,PassageSupport};
use crate::domain::documents::{CodeBlockObservation,CodeBlockSupport};
use crate::domain::documents::{DocumentLinkObservation,DocumentLinkSupport};
use crate::domain::documents::{DocumentMentionObservation,DocumentMentionSupport};
use crate::domain::documents::{DocumentComponentObservation,DocumentComponentSupport};
use crate::domain::documents::{DocumentAttributeObservation,DocumentAttributeSupport};
use crate::domain::flow::{FlowAttributeLoadObservation,FlowAttributeLoadSupport};
use crate::domain::flow::{FlowValuePathObservation,FlowValuePathSupport};
use crate::domain::lexical::{LexicalScopeObservation,LexicalScopeSupport};
use crate::domain::lexical::{BindingObservation,BindingSupport};
use crate::domain::lexical::{ReferenceObservation,ReferenceSupport};
use crate::domain::lexical::{LexicalResolution,LexicalResolutionSupport};
use crate::domain::source::{SyntaxObservation,SyntaxSupport};
use crate::domain::symbols::{SymbolObservation,SymbolSupport};
use crate::domain::symbols::{FunctionTraitObservation,FunctionTraitSupport};
use crate::domain::symbols::{ClassTraitObservation,ClassTraitSupport};
use crate::domain::symbols::{ClassAncestryObservation,ClassAncestrySupport};
use crate::domain::symbols::{ParameterAnnotationObservation,ParameterAnnotationSupport};
use crate::domain::symbols::{PublicNameObservation,PublicNameSupport};
use crate::domain::symbols::{ParameterDocObservation,ParameterDocSupport};
use crate::domain::symbols::{DependencyModuleObservation,DependencyModuleSupport};
use crate::domain::syntax::{SyntaxPlacement,SyntaxPlacementSupport};
use crate::domain::syntax::{SyntaxDetailObservation,SyntaxDetailSupport};
use crate::domain::syntax::{DeclarationObservation,DeclarationSupport};
use crate::domain::syntax::{DeclarationDecorator,DeclarationDecoratorSupport};
use crate::domain::syntax::{ImportAliasObservation,ImportAliasSupport};
use crate::domain::syntax::{DunderAllObservation,DunderAllSupport};
use crate::domain::syntax::{ParameterSyntaxObservation,ParameterSyntaxSupport};
use crate::domain::syntax::{ClassFieldSyntaxObservation,ClassFieldSyntaxSupport};
use crate::domain::types::{TypeObservation,TypeSupport};
use crate::domain::types::{TypePresentation,TypePresentationSupport};
use crate::domain::types::{TypeVariableRestriction,TypeRestrictionSupport};
use crate::domain::types::{FunctionBodyObservation,FunctionBodySupport};
use crate::domain::types::{RecordFieldObservation,RecordFieldSupport};

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
                let mut inputs = vec![ValidationInput::of::<AssertionQualification>(&["id"])];
                $(inputs.extend([ValidationInput::of::<$assertion>(&["id"]),
                    ValidationInput::of::<$support>(&["id"])]);)*
                inputs
            }
            /// Native support validators stay authoritative. The producer must read confirmed
            /// facts with these validators; this projection does not reimplement attribution.
            pub fn stage_inputs() -> Vec<stages::RelationUse> {
                let mut inputs = vec![stages::RelationUse::stored::<AssertionQualification>()];
                $(inputs.extend([stages::RelationUse::stored::<$assertion>(),
                    stages::RelationUse::stored::<$support>().validated_by(&[<$support>::NAME])]);)*
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
native_pairs! {
    0: Use => FlowUseObservation, FlowUseSupport;
    1: Definition => FlowDefinitionObservation, FlowDefinitionSupport;
    2: Reaching => FlowReachingObservation, FlowReachingSupport;
    3: Value => FlowValueObservation, FlowValueSupport;
    4: Region => FlowRegionObservation, FlowRegionSupport;
    5: Test => FlowTestObservation, FlowTestSupport;
    6: Leaf => FlowTestLeafObservation, FlowTestLeafSupport;
    7: Signature => Signature, SignatureSupport;
    8: CallTarget => CallTarget, CallTargetSupport;
    9: ProviderCallSite => ProviderCallSite, ProviderCallSiteSupport;
    10: CallSyntax => CallSyntax, CallSyntaxSupport;
    11: CallResolution => CallResolution, CallResolutionSupport;
    12: SymbolDeclaration => SymbolDeclaration, SymbolDeclarationSupport;
    13: ParameterDeclaration => ParameterDeclaration, ParameterDeclarationSupport;
    14: TaskReportObservation => TaskReportObservation, TaskReportSupport;
    15: DeploymentObservation => DeploymentObservation, DeploymentSupport;
    16: DocumentObservation => DocumentObservation, DocumentSupport;
    17: PassageObservation => PassageObservation, PassageSupport;
    18: CodeBlockObservation => CodeBlockObservation, CodeBlockSupport;
    19: DocumentLinkObservation => DocumentLinkObservation, DocumentLinkSupport;
    20: DocumentMentionObservation => DocumentMentionObservation, DocumentMentionSupport;
    21: DocumentComponentObservation => DocumentComponentObservation, DocumentComponentSupport;
    22: DocumentAttributeObservation => DocumentAttributeObservation, DocumentAttributeSupport;
    23: FlowAttributeLoadObservation => FlowAttributeLoadObservation, FlowAttributeLoadSupport;
    24: FlowValuePathObservation => FlowValuePathObservation, FlowValuePathSupport;
    25: LexicalScopeObservation => LexicalScopeObservation, LexicalScopeSupport;
    26: BindingObservation => BindingObservation, BindingSupport;
    27: ReferenceObservation => ReferenceObservation, ReferenceSupport;
    28: LexicalResolution => LexicalResolution, LexicalResolutionSupport;
    29: SyntaxObservation => SyntaxObservation, SyntaxSupport;
    30: SymbolObservation => SymbolObservation, SymbolSupport;
    31: FunctionTraitObservation => FunctionTraitObservation, FunctionTraitSupport;
    32: ClassTraitObservation => ClassTraitObservation, ClassTraitSupport;
    33: ClassAncestryObservation => ClassAncestryObservation, ClassAncestrySupport;
    34: ParameterAnnotationObservation => ParameterAnnotationObservation, ParameterAnnotationSupport;
    35: PublicNameObservation => PublicNameObservation, PublicNameSupport;
    36: ParameterDocObservation => ParameterDocObservation, ParameterDocSupport;
    37: DependencyModuleObservation => DependencyModuleObservation, DependencyModuleSupport;
    38: SyntaxPlacement => SyntaxPlacement, SyntaxPlacementSupport;
    39: SyntaxDetailObservation => SyntaxDetailObservation, SyntaxDetailSupport;
    40: DeclarationObservation => DeclarationObservation, DeclarationSupport;
    41: DeclarationDecorator => DeclarationDecorator, DeclarationDecoratorSupport;
    42: ImportAliasObservation => ImportAliasObservation, ImportAliasSupport;
    43: DunderAllObservation => DunderAllObservation, DunderAllSupport;
    44: ParameterSyntaxObservation => ParameterSyntaxObservation, ParameterSyntaxSupport;
    45: ClassFieldSyntaxObservation => ClassFieldSyntaxObservation, ClassFieldSyntaxSupport;
    46: TypeObservation => TypeObservation, TypeSupport;
    47: TypePresentation => TypePresentation, TypePresentationSupport;
    48: TypeVariableRestriction => TypeVariableRestriction, TypeRestrictionSupport;
    49: FunctionBodyObservation => FunctionBodyObservation, FunctionBodySupport;
    50: RecordFieldObservation => RecordFieldObservation, RecordFieldSupport;
}

/// One-shot typed projection of an actual native pair. Every payload is recomputed from
/// the paired assertion/support; it does not select a new attribution or claim.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "native_qualifications", rule = "native_qualification", invariants = inventory_invariants)]
pub struct NativeQualification {
    #[model(key, premise)] pub premise: Id<NativeAssertionPremise>,
    pub qualification: Id<AssertionQualification>,
    pub family: FactFamily,
    pub fidelity: Fidelity,
    pub status: super::policy::EvidenceStatus,
}
pub fn relations() -> Vec<Relation> {
    vec![Relation::of::<NativeAssertionPremise>(), Relation::of::<NativeQualification>()]
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
        Self { budget: budget.clone(), charge: charged::StateCharge::new(budget, "native-inventory"),
            qualifications: Default::default(), assertions: Default::default(), supports: Default::default() }
    }
    /// Emit exactly one immutable pair/qualification per actual native support. Input order and
    /// repeated identical batches do not affect the domain. An absent assertion/qualification
    /// refuses the complete projection; it never silently removes a required pair.
    pub fn for_each(&self, mut emit: impl FnMut(&NativeAssertionPremise, NativeQualification) -> Result<(), ModelError>) -> Result<(), ModelError> {
        for (id, (premise, fidelity)) in self.supports.iter() {
            let (assertion, _) = premise.assertion_and_support();
            let qualification = *self.assertions.get(&assertion)
                .ok_or_else(|| invalid("native support assertion is absent"))?;
            if !self.qualifications.contains(&qualification) {
                return Err(invalid("native assertion qualification is absent"));
            }
            let family = premise.family();
            emit(premise, NativeQualification { premise: *id, qualification, family, fidelity: *fidelity,
                status: super::policy::native_status(family, *fidelity) })?;
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
        Self { premises: normalized::Rows::new(budget), qualifications: normalized::Rows::new(budget) }
    }
    pub fn visit(&mut self, name: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        if name == NativeAssertionPremise::NAME { self.premises.decode(batch) }
        else if name == NativeQualification::NAME { self.qualifications.decode(batch) }
        else { Err(invalid("undeclared native inventory output")) }
    }
}
fn inventory_invariants() -> Vec<Invariant> {
    let mut inputs = NativeInventory::inputs();
    inputs.extend([ValidationInput::of::<NativeAssertionPremise>(&["id"]),
        ValidationInput::of::<NativeQualification>(&["id"])]);
    vec![Invariant { name: "native_inventory_projection", inputs,
        create: std::sync::Arc::new(|budget| Box::new(InventoryCheck {
            native: NativeInventory::new(budget), observed: NativeInventoryOutput::new(budget),
        })) }]
}
struct InventoryCheck { native: NativeInventory, observed: NativeInventoryOutput }
impl InvariantCheck for InventoryCheck {
    fn visit(&mut self, name: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        if name == NativeAssertionPremise::NAME || name == NativeQualification::NAME {
            self.observed.visit(name, batch)
        } else { self.native.visit(name, batch) }
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        let mut count = 0;
        self.native.for_each(|premise, qualification| {
            if self.observed.premises.get(premise.id()) != Some(premise)
                || self.observed.qualifications.get(qualification.id()) != Some(&qualification) {
                return Err(invalid("native inventory differs from exact assertion/support projection"));
            }
            count += 1;
            Ok(())
        })?;
        if count != self.observed.premises.len() || count != self.observed.qualifications.len() {
            return Err(invalid("native inventory has extraneous or missing members"));
        }
        Ok(())
    }
}
