//! Complete normalization candidate domains, independent of native or finite realization.
use super::super::{
    catalog_scope_program::{BudgetedCatalogProgram, Builder},
    scope_program::*,
    *,
};
use super::{callables::*, entities::*, events::*, links::*};
use assertion::*;
use attribution::*;
use calls::*;
use lexical::*;
use source::*;
use std::any::TypeId;
use syntax::*;
use types::*;
#[derive(Clone, Copy)]
pub enum CallableKernel {
    Callable,
    Signature,
    Overload,
}
pub struct NormalizationProgram {
    inner: BudgetedCatalogProgram,
    signature_root: Option<usize>,
    entity_roots: Vec<(TypeId, usize)>,
}
impl NormalizationProgram {
    pub fn program(&self) -> &ScopeProgram {
        self.inner.program()
    }
    pub fn signature_root(&self) -> Option<usize> {
        self.signature_root
    }
    pub fn entity_roots(&self) -> &[(TypeId, usize)] {
        &self.entity_roots
    }
}
fn col(row: usize, field: &'static str) -> ScopeColumn {
    ScopeColumn { row, field }
}
fn eq(a: usize, af: &'static str, b: usize, bf: &'static str) -> ScopePredicate {
    ScopePredicate::Equal(col(a, af), col(b, bf))
}
pub fn callable(
    inputs: Vec<ValidationInput>,
    input_relations: &[Relation],
    kernel: CallableKernel,
    budget: &resources::ResourceBudget,
) -> Result<NormalizationProgram, ModelError> {
    use CallableKernel as Kernel;
    let index = |kind: TypeId| {
        input_relations
            .iter()
            .position(|relation| relation.type_id() == kind)
            .ok_or(ModelError::Schema("callable scope input"))
    };
    let real = inputs.len();
    let signature_root = if matches!(kernel, Kernel::Signature) {
        Some(real)
    } else {
        None
    };
    let mut declarations = inputs.clone();
    if signature_root.is_some() {
        declarations.push(inputs[index(TypeId::of::<Signature>())?].clone());
    }
    let mut b = Builder::new(declarations, real, input_relations, budget)?;
    for (source, relation) in input_relations.iter().enumerate() {
        for field in relation.fields() {
            if let Some((kind, _)) = field.target()
                && let Ok(to) = index(kind)
            {
                b.follow(source, field.name(), to, field.list());
            }
        }
    }
    macro_rules! own {
        ($member:ty,$field:literal,$owner:ty) => {
            b.own(
                index(TypeId::of::<$member>())?,
                $field,
                index(TypeId::of::<$owner>())?,
            )
        };
    }
    own!(EntityRef, "callable_callable", CallableEntity);
    own!(SymbolEntityResolution, "entity", EntityRef);
    own!(CallableEntity, "source_declaration", Occurrence);
    own!(DeclarationObservation, "declaration", Occurrence);
    own!(DeclarationDecorator, "declaration", Occurrence);
    own!(FunctionBodyObservation, "declaration", Occurrence);
    own!(ReferenceEntityAssessment, "reference", ReferenceObservation);
    own!(
        ReferenceEntityCandidate,
        "assessment",
        ReferenceEntityAssessment
    );
    if index(TypeId::of::<
        normalized::callables::EffectiveCallableAssessment,
    >())
    .is_ok()
    {
        own!(
            normalized::callables::EffectiveCallableAssessment,
            "callable",
            CallableEntity
        );
        own!(
            normalized::callables::EffectiveDecoratorMember,
            "assessment",
            normalized::callables::EffectiveCallableAssessment
        );
        own!(
            normalized::callables::EffectiveCallableEvidence,
            "assessment",
            normalized::callables::EffectiveCallableAssessment
        );
    }

    let resolutions = index(TypeId::of::<SymbolEntityResolution>())?;
    let signatures = index(TypeId::of::<Signature>())?;
    let traits = index(TypeId::of::<symbols::FunctionTraitObservation>())?;
    b.pair(
        resolutions,
        resolutions,
        &[resolutions, resolutions],
        vec![eq(1, "symbol", 0, "symbol")],
        col(0, "id"),
        col(1, "id"),
    );
    b.pair(
        signatures,
        resolutions,
        &[signatures, resolutions],
        vec![eq(1, "symbol", 0, "symbol")],
        col(0, "id"),
        col(1, "id"),
    );
    b.pair(
        resolutions,
        signatures,
        &[resolutions, signatures],
        vec![eq(1, "symbol", 0, "symbol")],
        col(0, "id"),
        col(1, "id"),
    );
    b.pair(
        resolutions,
        traits,
        &[resolutions, traits],
        vec![eq(1, "symbol", 0, "symbol")],
        col(0, "id"),
        col(1, "id"),
    );
    let occurrences = index(TypeId::of::<Occurrence>())?;
    let placements = index(TypeId::of::<SyntaxPlacement>())?;
    let references = index(TypeId::of::<ReferenceObservation>())?;
    let coverage = index(TypeId::of::<ProviderCoverage>())?;
    let scopes = index(TypeId::of::<CoverageScope>())?;
    let owners = index(TypeId::of::<OccurrenceOwnership>())?;
    b.pair(
        occurrences,
        coverage,
        &[occurrences, scopes, coverage],
        vec![
            eq(1, "artifact_artifact", 0, "source"),
            eq(2, "scope", 1, "id"),
            ScopePredicate::Code(col(2, "family"), FactFamily::Syntax.code()),
        ],
        col(0, "id"),
        col(2, "id"),
    );
    b.pair(
        occurrences,
        placements,
        &[occurrences, placements],
        vec![
            eq(1, "parent", 0, "id"),
            ScopePredicate::Code(col(0, "syntax_kind"), SyntaxKind::Decorator.code()),
        ],
        col(0, "id"),
        col(1, "id"),
    );
    b.pair(
        occurrences,
        references,
        &[occurrences, references],
        vec![eq(1, "read", 0, "id")],
        col(0, "id"),
        col(1, "id"),
    );
    const YIELDS: &[i16] = &[
        SyntaxKind::ExprYield as i16,
        SyntaxKind::ExprYieldFrom as i16,
    ];
    b.pair(
        occurrences,
        owners,
        &[occurrences, owners, occurrences],
        vec![
            eq(1, "owner", 0, "id"),
            eq(2, "id", 1, "occurrence"),
            ScopePredicate::CodeIn(col(2, "syntax_kind"), YIELDS),
        ],
        col(0, "id"),
        col(1, "id"),
    );
    if matches!(kernel, Kernel::Overload) {
        own!(NativeSignatureObservation, "signature", Signature);
    }
    if matches!(kernel, Kernel::Signature) {
        own!(ParameterEntityLink, "parameter", SignatureParameter);
        own!(
            SignatureTypeSubject,
            "parameter_parameter",
            SignatureParameter
        );
        own!(SignatureTypeObservation, "subject", SignatureTypeSubject);
    }
    if matches!(kernel, Kernel::Overload) {
        own!(NativeOverloadCandidate, "trace", NativeOverloadObservation);
        own!(
            NativeOverloadSupport,
            "assertion",
            NativeOverloadObservation
        );
        own!(
            NativeSignatureSupport,
            "assertion",
            NativeSignatureObservation
        );
        let traces = index(TypeId::of::<NativeOverloadObservation>())?;
        let candidates = index(TypeId::of::<NativeOverloadCandidate>())?;
        let native = index(TypeId::of::<NativeSignatureObservation>())?;
        let q = index(TypeId::of::<AssertionQualification>())?;
        b.pair(
            traces,
            native,
            &[traces, candidates, q, native, q, signatures],
            vec![
                eq(1, "trace", 0, "id"),
                eq(2, "id", 0, "qualification"),
                eq(3, "metadata_origin", 1, "origin"),
                eq(4, "id", 3, "qualification"),
                eq(5, "id", 3, "signature"),
                eq(4, "context", 2, "context"),
                ScopePredicate::Code(col(5, "role"), SignatureRole::EffectiveTyped.code()),
            ],
            col(0, "id"),
            col(3, "id"),
        );
    }
    if let Some(root) = signature_root {
        b.pair(
            root,
            signatures,
            &[signatures],
            vec![],
            col(0, "id"),
            col(0, "id"),
        );
        for (kind, field) in [
            (TypeId::of::<SignatureParameter>(), "signature"),
            (TypeId::of::<NativeSignatureObservation>(), "signature"),
            (TypeId::of::<SignatureTypeSubject>(), "return_signature"),
        ] {
            b.reverse(index(kind)?, field, root);
        }
    }
    Ok(NormalizationProgram {
        inner: b.finish()?,
        signature_root,
        entity_roots: vec![],
    })
}
pub fn receiver(
    inputs: Vec<ValidationInput>,
    input_relations: &[Relation],
    budget: &resources::ResourceBudget,
) -> Result<NormalizationProgram, ModelError> {
    let index = |kind: TypeId| {
        input_relations
            .iter()
            .position(|relation| relation.type_id() == kind)
            .ok_or(ModelError::Schema("receiver nominal premise absent"))
    };
    let mut b = Builder::new(inputs.clone(), inputs.len(), input_relations, budget)?;
    for (source, relation) in input_relations.iter().enumerate() {
        for field in relation.fields().iter().filter(|field| !field.list()) {
            if let Some((kind, _)) = field.target()
                && let Ok(to) = index(kind)
            {
                b.follow(source, field.name(), to, false);
            }
        }
    }
    macro_rules! owned {
        ($member:ty,$field:literal,$owner:ty) => {
            b.own(
                index(TypeId::of::<$member>())?,
                $field,
                index(TypeId::of::<$owner>())?,
            )
        };
    }
    owned!(CallTargetSupport, "assertion", CallTarget);
    owned!(CallSyntaxSupport, "assertion", CallSyntax);
    owned!(
        syntax::SyntaxPlacementSupport,
        "assertion",
        syntax::SyntaxPlacement
    );
    owned!(
        super::callables::EffectiveCallableAssessment,
        "callable",
        CallableEntity
    );
    owned!(
        super::callables::SignatureVariant,
        "assessment",
        super::callables::EffectiveCallableAssessment
    );
    if let (Ok(assessments), Ok(evidence)) = (
        index(TypeId::of::<super::receiver::ReceiverAssessment>()),
        index(TypeId::of::<super::receiver::ReceiverEvidence>()),
    ) {
        for field in input_relations[assessments]
            .fields()
            .iter()
            .filter(|field| {
                field.target().map(|(kind, _)| kind) == Some(TypeId::of::<CallTarget>())
            })
        {
            b.own(
                assessments,
                field.name(),
                index(TypeId::of::<CallTarget>())?,
            );
        }
        b.own(evidence, "assessment", assessments);
    }
    let targets = index(TypeId::of::<CallTarget>())?;
    let q = index(TypeId::of::<AssertionQualification>())?;
    let syntax = index(TypeId::of::<CallSyntax>())?;
    let placements = index(TypeId::of::<SyntaxPlacement>())?;
    let destinations = index(TypeId::of::<CallDestination>())?;
    let resolutions = index(TypeId::of::<SymbolEntityResolution>())?;
    let coverage = index(TypeId::of::<ProviderCoverage>())?;
    b.pair(
        targets,
        syntax,
        &[targets, syntax, q, q],
        vec![
            eq(1, "site", 0, "site"),
            eq(2, "id", 0, "qualification"),
            eq(3, "id", 1, "qualification"),
            eq(3, "context", 2, "context"),
        ],
        col(0, "id"),
        col(1, "id"),
    );
    b.pair(
        syntax,
        placements,
        &[syntax, placements],
        vec![
            eq(1, "parent", 0, "callee"),
            ScopePredicate::Code(col(1, "field"), SyntaxField::Value.code()),
        ],
        col(0, "id"),
        col(1, "id"),
    );
    b.pair(
        targets,
        resolutions,
        &[targets, destinations, q, resolutions],
        vec![
            eq(1, "id", 0, "destination"),
            eq(2, "id", 0, "qualification"),
            ScopePredicate::EqualCoalesce {
                column: col(3, "symbol"),
                primary: col(1, "resolved_symbol"),
                fallback: col(1, "overrides_symbol"),
            },
            eq(3, "context", 2, "context"),
        ],
        col(0, "id"),
        col(3, "id"),
    );
    const FAMILIES: &[i16] = &[FactFamily::Calls as i16, FactFamily::Syntax as i16];
    b.pair(
        q,
        coverage,
        &[q, coverage],
        vec![
            eq(1, "scope", 0, "scope"),
            eq(1, "context", 0, "context"),
            ScopePredicate::CodeIn(col(1, "family"), FAMILIES),
        ],
        col(0, "id"),
        col(1, "id"),
    );
    Ok(NormalizationProgram {
        inner: b.finish()?,
        signature_root: None,
        entity_roots: vec![],
    })
}

pub fn calls(
    inputs: Vec<ValidationInput>,
    input_relations: &[Relation],
    real: usize,
    event_roots: &[(TypeId, usize)],
    binding: bool,
    admission: bool,
    budget: &resources::ResourceBudget,
) -> Result<NormalizationProgram, ModelError> {
    let index = |kind: TypeId| {
        input_relations[..real]
            .iter()
            .position(|relation| relation.type_id() == kind)
    };
    let mut b = Builder::new(inputs, real, input_relations, budget)?;
    for (source, relation) in input_relations[..real].iter().enumerate() {
        for field in relation.fields().iter().filter(|field| !field.list()) {
            if let Some((kind, _)) = field.target()
                && let Some(to) = index(kind)
            {
                b.follow(source, field.name(), to, false);
            }
        }
    }
    macro_rules! own {
        ($member:ty,$field:literal,$owner:ty) => {{
            if let (Some(member), Some(owner)) = (
                index(TypeId::of::<$member>()),
                index(TypeId::of::<$owner>()),
            ) {
                b.own(member, $field, owner);
            }
        }};
    }
    macro_rules! own_existing {
        ($member:ty,$field:literal,$owner:ty) => {{
            if let (Some(member), Some(owner)) = (
                index(TypeId::of::<$member>()),
                index(TypeId::of::<$owner>()),
            ) {
                b.follow(member, $field, owner, false);
                b.pair(
                    owner,
                    member,
                    &[member, owner],
                    vec![eq(0, $field, 1, "id")],
                    col(1, "id"),
                    col(0, "id"),
                );
            }
        }};
    }
    own!(ProviderCallSiteSupport, "assertion", ProviderCallSite);
    own!(CallTargetSupport, "assertion", CallTarget);
    own!(CallResolutionSupport, "assertion", CallResolution);
    own!(CallResolutionMember, "resolution", CallResolution);
    own!(OccurrenceOwnership, "occurrence", source::Occurrence);
    own!(CallSyntaxSupport, "assertion", CallSyntax);
    own!(CallArgument, "call", CallSyntax);
    own!(
        syntax::SyntaxPlacementSupport,
        "assertion",
        syntax::SyntaxPlacement
    );
    own!(EffectiveCallableAssessment, "callable", CallableEntity);
    own!(
        EffectiveDecoratorMember,
        "assessment",
        EffectiveCallableAssessment
    );
    own!(
        EffectiveCallableEvidence,
        "assessment",
        EffectiveCallableAssessment
    );
    own!(SignatureVariant, "callable", CallableEntity);
    own!(SignatureVariant, "assessment", EffectiveCallableAssessment);
    own!(SignatureSlot, "variant", SignatureVariant);
    own!(SignatureSlotEntity, "slot", SignatureSlot);
    own!(SignatureParameter, "signature", Signature);
    own!(SignatureSupport, "assertion", Signature);
    own!(
        SignatureEnumerationMember,
        "enumeration",
        SignatureEnumerationObservation
    );
    own!(
        SignatureEnumerationSupport,
        "assertion",
        SignatureEnumerationObservation
    );
    own!(
        symbols::FunctionTraitSupport,
        "assertion",
        symbols::FunctionTraitObservation
    );
    own!(
        symbols::ClassAncestrySupport,
        "assertion",
        symbols::ClassAncestryObservation
    );
    own!(
        symbols::SymbolSequenceMember,
        "sequence",
        symbols::SymbolSequence
    );
    own!(
        declarations::SymbolDeclarationSupport,
        "assertion",
        declarations::SymbolDeclaration
    );
    own!(CallEventSource, "event", NormalizedCallEvent);
    own!(CallEventSourceEvidence, "source", CallEventSource);
    own!(CallEventResolution, "event", NormalizedCallEvent);
    own!(
        CallEventResolutionEvidence,
        "resolution",
        CallEventResolution
    );
    own!(NormalizedCallAlternative, "event", NormalizedCallEvent);
    own!(
        CallAlternativeEvidence,
        "alternative",
        NormalizedCallAlternative
    );
    own!(EventAssessment, "event", NormalizedCallEvent);
    own!(EventPhaseTarget, "assessment", EventAssessment);
    own!(CallPolicyAssessment, "event", NormalizedCallEvent);
    own!(CallPolicyAdmission, "assessment", CallPolicyAssessment);
    own!(
        normalized::dispatch::DispatchAssessment,
        "event",
        NormalizedCallEvent
    );
    own!(
        normalized::dispatch::DispatchMember,
        "assessment",
        normalized::dispatch::DispatchAssessment
    );
    own!(
        normalized::dispatch::DispatchEvidence,
        "assessment",
        normalized::dispatch::DispatchAssessment
    );
    own!(
        normalized::bindings::CallBindingAttempt,
        "event",
        NormalizedCallEvent
    );
    own!(
        normalized::bindings::CallBinding,
        "attempt",
        normalized::bindings::CallBindingAttempt
    );
    own!(
        normalized::bindings::BindingSetAssessment,
        "event",
        NormalizedCallEvent
    );
    own!(
        normalized::bindings::BindingVariantAssessment,
        "set",
        normalized::bindings::BindingSetAssessment
    );
    own!(
        normalized::bindings::BindingSetMember,
        "variant",
        normalized::bindings::BindingVariantAssessment
    );
    own!(
        normalized::bindings::BindingSetCoverage,
        "set",
        normalized::bindings::BindingSetAssessment
    );
    own!(
        normalized::receiver::ReceiverEvidence,
        "assessment",
        normalized::receiver::ReceiverAssessment
    );
    if let (Some(assessments), Some(targets)) = (
        input_relations[..real].iter().position(|table| {
            table.type_id() == TypeId::of::<normalized::receiver::ReceiverAssessment>()
        }),
        input_relations[..real]
            .iter()
            .position(|table| table.type_id() == TypeId::of::<CallTarget>()),
    ) {
        for field in input_relations[assessments]
            .fields()
            .iter()
            .filter(|field| {
                field.target().map(|(kind, _)| kind) == Some(TypeId::of::<CallTarget>())
            })
        {
            b.own(assessments, field.name(), targets);
        }
    }

    let q = index(TypeId::of::<AssertionQualification>())
        .ok_or(ModelError::Schema(AssertionQualification::NAME))?;
    own_existing!(CallSyntax, "site", Occurrence);
    if let (Some(syntax), Some(placements)) = (
        index(TypeId::of::<CallSyntax>()),
        index(TypeId::of::<SyntaxPlacement>()),
    ) {
        b.pair(
            syntax,
            placements,
            &[syntax, placements],
            vec![
                eq(1, "parent", 0, "callee"),
                ScopePredicate::Code(col(1, "field"), SyntaxField::Value.code()),
            ],
            col(0, "id"),
            col(1, "id"),
        );
    }
    if let Some(coverage) = index(TypeId::of::<ProviderCoverage>()) {
        const FAMILIES: &[i16] = &[
            FactFamily::Calls as i16,
            FactFamily::Syntax as i16,
            FactFamily::Signatures as i16,
        ];
        b.pair(
            q,
            coverage,
            &[q, coverage],
            vec![
                eq(1, "scope", 0, "scope"),
                eq(1, "context", 0, "context"),
                ScopePredicate::CodeIn(col(1, "family"), FAMILIES),
            ],
            col(0, "id"),
            col(1, "id"),
        );
    }
    if let Some(symbols) = index(TypeId::of::<ProviderSymbol>()) {
        own_existing!(Signature, "symbol", ProviderSymbol);
        own_existing!(SignatureEnumerationObservation, "symbol", ProviderSymbol);
        own_existing!(declarations::SymbolDeclaration, "symbol", ProviderSymbol);
        if let Some(resolutions) = index(TypeId::of::<SymbolEntityResolution>()) {
            b.pair(
                symbols,
                resolutions,
                &[symbols, resolutions],
                vec![eq(1, "symbol", 0, "id"), eq(1, "context", 0, "context")],
                col(0, "id"),
                col(1, "id"),
            );
        }
        if let Some(traits) = index(TypeId::of::<symbols::FunctionTraitObservation>()) {
            b.pair(
                symbols,
                traits,
                &[symbols, traits, q],
                vec![
                    eq(1, "symbol", 0, "id"),
                    eq(2, "id", 1, "qualification"),
                    eq(2, "context", 0, "context"),
                ],
                col(0, "id"),
                col(1, "id"),
            );
            b.pair(
                symbols,
                traits,
                &[symbols, traits, symbols],
                vec![
                    eq(1, "overrides", 0, "id"),
                    eq(2, "id", 1, "symbol"),
                    eq(2, "context", 0, "context"),
                    eq(2, "provider", 0, "provider"),
                ],
                col(0, "id"),
                col(1, "id"),
            );
        }
        if let Some(ancestry) = index(TypeId::of::<symbols::ClassAncestryObservation>()) {
            b.pair(
                symbols,
                ancestry,
                &[symbols, ancestry, q],
                vec![
                    eq(1, "class", 0, "id"),
                    eq(2, "id", 1, "qualification"),
                    eq(2, "context", 0, "context"),
                ],
                col(0, "id"),
                col(1, "id"),
            );
        }
    }
    if let (Some(symbols), Some(signatures), Some(variants)) = (
        index(TypeId::of::<ProviderSymbol>()),
        index(TypeId::of::<Signature>()),
        index(TypeId::of::<SignatureVariant>()),
    ) {
        b.pair(
            symbols,
            variants,
            &[symbols, signatures, variants],
            vec![eq(1, "symbol", 0, "id"), eq(2, "signature", 1, "id")],
            col(0, "id"),
            col(2, "id"),
        );
    }
    if let (Some(signatures), Some(enumerations)) = (
        index(TypeId::of::<Signature>()),
        index(TypeId::of::<SignatureEnumerationObservation>()),
    ) {
        b.pair(
            signatures,
            enumerations,
            &[signatures, enumerations],
            vec![
                eq(1, "symbol", 0, "symbol"),
                eq(1, "qualification", 0, "qualification"),
                eq(1, "role", 0, "role"),
            ],
            col(0, "id"),
            col(1, "id"),
        );
    }
    for (kind, root) in event_roots {
        if *kind == TypeId::of::<flow::FlowValuePathObservation>() {
            let actual = index(*kind).ok_or(ModelError::Schema("actual path root"))?;
            b.pair(*root, actual, &[*root], vec![], col(0, "id"), col(0, "id"));
            if let Some(steps) = index(TypeId::of::<flow::FlowCallStep>()) {
                b.pair(
                    *root,
                    steps,
                    &[*root, steps],
                    vec![eq(1, "path", 0, "path")],
                    col(0, "id"),
                    col(1, "id"),
                );
                if admission && let Some(events) = index(TypeId::of::<NormalizedCallEvent>()) {
                    b.pair(
                        *root,
                        events,
                        &[*root, q, steps, events],
                        vec![
                            eq(1, "id", 0, "qualification"),
                            eq(2, "path", 0, "path"),
                            eq(3, "site", 2, "call"),
                            eq(3, "context", 1, "context"),
                        ],
                        col(0, "id"),
                        col(3, "id"),
                    );
                }
            }
            if admission && let Some(links) = index(TypeId::of::<FlowCallEventLink>()) {
                b.pair(
                    *root,
                    links,
                    &[*root, links],
                    vec![eq(1, "observation", 0, "id")],
                    col(0, "id"),
                    col(1, "id"),
                );
            }
        } else if binding {
            let actual = index(*kind).ok_or(ModelError::Schema("actual event root"))?;
            b.pair(*root, actual, &[*root], vec![], col(0, "id"), col(0, "id"));
        } else {
            for candidate in [
                TypeId::of::<ProviderCallSite>(),
                TypeId::of::<CallTarget>(),
                TypeId::of::<CallResolution>(),
            ] {
                let actual =
                    index(candidate).ok_or(ModelError::Schema("actual event candidate"))?;
                let mut rows = vec![*root, actual, q];
                let mut ps = vec![
                    eq(1, "site", 0, "site"),
                    eq(1, "origin", 0, "origin"),
                    eq(2, "id", 1, "qualification"),
                ];
                if *kind == TypeId::of::<NormalizedCallEvent>() {
                    ps.push(eq(2, "context", 0, "context"));
                } else {
                    rows.push(q);
                    ps.extend([
                        eq(3, "id", 0, "qualification"),
                        eq(2, "context", 3, "context"),
                    ]);
                }
                b.pair(*root, actual, &rows, ps, col(0, "id"), col(1, "id"));
            }
            if admission {
                let events = index(TypeId::of::<NormalizedCallEvent>())
                    .ok_or(ModelError::Schema(NormalizedCallEvent::NAME))?;
                let mut rows = vec![*root, events];
                let mut ps = vec![eq(1, "site", 0, "site"), eq(1, "origin", 0, "origin")];
                if *kind == TypeId::of::<NormalizedCallEvent>() {
                    ps.push(eq(1, "context", 0, "context"));
                } else {
                    rows.push(q);
                    ps.extend([
                        eq(2, "id", 0, "qualification"),
                        eq(1, "context", 2, "context"),
                    ]);
                }
                b.pair(*root, events, &rows, ps, col(0, "id"), col(1, "id"));
            }
        }
    }
    Ok(NormalizationProgram {
        inner: b.finish()?,
        signature_root: None,
        entity_roots: vec![],
    })
}
#[cfg(test)]
mod controls {
    use super::*;
    fn input_relations(inputs: &[ValidationInput], owner: &ValidatedModel) -> Vec<Relation> {
        inputs
            .iter()
            .map(|input| owner.relation(input.name()).unwrap().clone())
            .collect()
    }
    #[test]
    fn callable_signature_demand_is_virtual_and_never_forward_activated() {
        let owner = super::super::super::model().unwrap();
        let budget = resources::ResourceBudget::fixed(8 << 20).unwrap();
        let inputs = super::super::callable_normalization::CallableData::validation_inputs();
        let rs = input_relations(&inputs, &owner);
        for kernel in [
            CallableKernel::Callable,
            CallableKernel::Overload,
            CallableKernel::Signature,
        ] {
            let program = callable(inputs.clone(), &rs, kernel, &budget).unwrap();
            program.program().validate(&owner).unwrap();
            if matches!(kernel, CallableKernel::Signature) {
                let root = program.signature_root().unwrap();
                assert_eq!(root, inputs.len());
                assert!(program.program().ports[root].virtual_owner);
                let owned=program.program().rules.iter().filter(|rule|matches!(rule,ScopeRule::Reference{target,direction:ScopeDirection::OwnedReverse,..} if *target==root)).count();
                assert_eq!(owned, 3);
                assert!(!program.program().rules.iter().any(|rule|matches!(rule,ScopeRule::Reference{target,direction:ScopeDirection::Forward,..} if *target==root)));
            } else {
                assert!(program.signature_root().is_none());
                assert!(
                    program
                        .program()
                        .ports
                        .iter()
                        .all(|port| !port.virtual_owner)
                );
            }
            drop(program);
            assert_eq!(budget.reserved(), 0);
        }
    }
    #[test]
    fn receiver_and_call_candidate_programs_validate_with_charged_metadata() {
        let owner = super::super::super::model().unwrap();
        let budget = resources::ResourceBudget::fixed(8 << 20).unwrap();
        let inputs = super::super::receiver::ReceiverData::validation_inputs();
        let program = receiver(inputs.clone(), &input_relations(&inputs, &owner), &budget).unwrap();
        program.program().validate(&owner).unwrap();
        drop(program);
        let mut inputs = super::super::binding_normalization::BindingData::validation_inputs();
        let real = inputs.len();
        let event = inputs
            .iter()
            .position(|input| input.type_id() == TypeId::of::<NormalizedCallEvent>())
            .unwrap();
        inputs.push(inputs[event].clone());
        let rs = input_relations(&inputs, &owner);
        let program = calls(
            inputs,
            &rs,
            real,
            &[(TypeId::of::<NormalizedCallEvent>(), real)],
            true,
            false,
            &budget,
        )
        .unwrap();
        program.program().validate(&owner).unwrap();
        assert!(program.program().ports[real].virtual_owner);
        drop(program);
        assert_eq!(budget.reserved(), 0);
    }
}

/// Entity roots seed only their explicit candidate families. Supporting references stay in the
/// ordinary nominal ports, so an origin-matched symbol never activates a second Symbol demand.
pub fn entity(
    inputs: Vec<ValidationInput>,
    input_relations: &[Relation],
    budget: &resources::ResourceBudget,
) -> Result<NormalizationProgram, ModelError> {
    let index = |kind: TypeId| {
        input_relations
            .iter()
            .position(|relation| relation.type_id() == kind)
    };
    let real = inputs.len();
    let mut declarations = inputs.clone();
    let mut roots = Vec::new();
    for kind in [
        TypeId::of::<ProviderSymbol>(),
        TypeId::of::<symbols::PublicNameObservation>(),
        TypeId::of::<symbols::ExportEnumerationObservation>(),
        TypeId::of::<ClassFieldSyntaxObservation>(),
    ] {
        if let Some(source) = index(kind) {
            roots.push((kind, declarations.len()));
            declarations.push(inputs[source].clone());
        }
    }
    let mut b = Builder::new(declarations, real, input_relations, budget)?;
    for (from, relation) in input_relations.iter().enumerate() {
        for field in relation.fields() {
            if let Some((kind, _)) = field.target()
                && let Some(to) = index(kind)
            {
                b.follow(from, field.name(), to, field.list());
            }
        }
    }
    for (kind, root) in &roots {
        let actual = index(*kind).ok_or(ModelError::Schema("entity root"))?;
        b.pair(*root, actual, &[*root], vec![], col(0, "id"), col(0, "id"));
        if *kind == TypeId::of::<ProviderSymbol>()
            || *kind == TypeId::of::<symbols::PublicNameObservation>()
        {
            let public = *kind == TypeId::of::<symbols::PublicNameObservation>();
            let symbols = index(TypeId::of::<ProviderSymbol>())
                .ok_or(ModelError::Schema("entity symbols"))?;
            let (base, ps, symbol) = if public {
                let q = index(TypeId::of::<AssertionQualification>())
                    .ok_or(ModelError::Schema("entity public qualification"))?;
                let origin = index(TypeId::of::<symbols::ExportOrigin>())
                    .ok_or(ModelError::Schema("entity public origin"))?;
                (
                    vec![*root, q, origin, symbols],
                    vec![
                        eq(1, "id", 0, "qualification"),
                        eq(2, "id", 0, "origin"),
                        eq(3, "context", 1, "context"),
                        eq(3, "module", 2, "traced_module"),
                        eq(3, "name", 2, "traced_name"),
                    ],
                    3,
                )
            } else {
                (vec![*root], vec![], 0)
            };
            if public {
                b.pair(
                    *root,
                    symbols,
                    &base,
                    ps.clone(),
                    col(0, "id"),
                    col(symbol, "id"),
                );
            }
            for (ty, field) in [
                (TypeId::of::<declarations::SymbolDeclaration>(), "symbol"),
                (TypeId::of::<symbols::FunctionTraitObservation>(), "symbol"),
                (TypeId::of::<symbols::ClassTraitObservation>(), "symbol"),
            ] {
                if let Some(to) = index(ty) {
                    let mut rows = base.clone();
                    let n = rows.len();
                    rows.push(to);
                    let mut predicates = ps.clone();
                    predicates.push(eq(n, field, symbol, "id"));
                    b.pair(*root, to, &rows, predicates, col(0, "id"), col(n, "id"));
                }
            }
            if let (Some(declaration), Some(support)) = (
                index(TypeId::of::<declarations::SymbolDeclaration>()),
                index(TypeId::of::<declarations::SymbolDeclarationSupport>()),
            ) {
                let mut rows = base.clone();
                let n = rows.len();
                rows.extend([declaration, support]);
                let mut predicates = ps.clone();
                predicates.extend([
                    eq(n, "symbol", symbol, "id"),
                    eq(n + 1, "assertion", n, "id"),
                ]);
                b.pair(
                    *root,
                    support,
                    &rows,
                    predicates,
                    col(0, "id"),
                    col(n + 1, "id"),
                );
            }
            if !public {
                if let Some(signature) = index(TypeId::of::<Signature>()) {
                    let mut rows = base.clone();
                    let n = rows.len();
                    rows.push(signature);
                    let mut predicates = ps.clone();
                    predicates.push(eq(n, "symbol", symbol, "id"));
                    b.pair(
                        *root,
                        signature,
                        &rows,
                        predicates.clone(),
                        col(0, "id"),
                        col(n, "id"),
                    );
                    if let Some(parameter) = index(TypeId::of::<SignatureParameter>()) {
                        rows.push(parameter);
                        predicates.push(eq(n + 1, "signature", n, "id"));
                        b.pair(
                            *root,
                            parameter,
                            &rows,
                            predicates.clone(),
                            col(0, "id"),
                            col(n + 1, "id"),
                        );
                        if let Some(declaration) =
                            index(TypeId::of::<declarations::ParameterDeclaration>())
                        {
                            rows.push(declaration);
                            predicates.push(eq(n + 2, "parameter", n + 1, "id"));
                            b.pair(
                                *root,
                                declaration,
                                &rows,
                                predicates,
                                col(0, "id"),
                                col(n + 2, "id"),
                            );
                        }
                    }
                }
                if let Some(fields) = index(TypeId::of::<types::RecordFieldObservation>()) {
                    let mut rows = base.clone();
                    let n = rows.len();
                    rows.push(fields);
                    let mut predicates = ps.clone();
                    predicates.push(eq(n, "class", symbol, "id"));
                    b.pair(*root, fields, &rows, predicates, col(0, "id"), col(n, "id"));
                }
            } else {
                if let Some(support) = index(TypeId::of::<symbols::PublicNameSupport>()) {
                    b.pair(
                        *root,
                        support,
                        &[*root, support],
                        vec![eq(1, "assertion", 0, "id")],
                        col(0, "id"),
                        col(1, "id"),
                    );
                }
                if let Some(observation) = index(TypeId::of::<symbols::SymbolObservation>()) {
                    let mut rows = base.clone();
                    let n = rows.len();
                    rows.push(observation);
                    let mut predicates = ps.clone();
                    predicates.push(eq(n, "symbol", symbol, "id"));
                    b.pair(
                        *root,
                        observation,
                        &rows,
                        predicates.clone(),
                        col(0, "id"),
                        col(n, "id"),
                    );
                    if let Some(support) = index(TypeId::of::<symbols::SymbolSupport>()) {
                        rows.push(support);
                        predicates.push(eq(n + 1, "assertion", n, "id"));
                        b.pair(
                            *root,
                            support,
                            &rows,
                            predicates,
                            col(0, "id"),
                            col(n + 1, "id"),
                        );
                    }
                }
                if let (Some(enumeration), Some(q)) = (
                    index(TypeId::of::<symbols::ExportEnumerationObservation>()),
                    index(TypeId::of::<AssertionQualification>()),
                ) {
                    let mut rows = vec![*root, enumeration, q, q];
                    let mut predicates = vec![
                        eq(1, "access", 0, "access"),
                        eq(2, "id", 1, "qualification"),
                        eq(3, "id", 0, "qualification"),
                        eq(2, "context", 3, "context"),
                        eq(2, "scope", 3, "scope"),
                    ];
                    b.pair(
                        *root,
                        enumeration,
                        &rows,
                        predicates.clone(),
                        col(0, "id"),
                        col(1, "id"),
                    );
                    if let Some(support) = index(TypeId::of::<symbols::ExportEnumerationSupport>())
                    {
                        rows.push(support);
                        predicates.push(eq(4, "assertion", 1, "id"));
                        b.pair(
                            *root,
                            support,
                            &rows,
                            predicates,
                            col(0, "id"),
                            col(4, "id"),
                        );
                    }
                }
            }
        } else if *kind == TypeId::of::<symbols::ExportEnumerationObservation>() {
            if let Some(support) = index(TypeId::of::<symbols::ExportEnumerationSupport>()) {
                b.pair(
                    *root,
                    support,
                    &[support],
                    vec![],
                    col(0, "assertion"),
                    col(0, "id"),
                );
            }
        } else if let Some(events) = index(TypeId::of::<BindingEvent>()) {
            b.pair(
                *root,
                events,
                &[*root, events],
                vec![eq(1, "site", 0, "target")],
                col(0, "id"),
                col(1, "id"),
            );
        }
    }
    b.reserve_retained(roots.capacity() * size_of::<(TypeId, usize)>())?;
    Ok(NormalizationProgram {
        inner: b.finish()?,
        signature_root: None,
        entity_roots: roots,
    })
}
/// Every relation operation starts at an explicit demand namespace. Supporting references
/// retain ordinary forward closure and cannot activate another operation's candidate domain.
pub fn relation(
    inputs: Vec<ValidationInput>,
    input_relations: &[Relation],
    kernel: super::relation_normalization::RelationKernel,
    budget: &resources::ResourceBudget,
) -> Result<NormalizationProgram, ModelError> {
    use super::relation_normalization::RelationKernel as K;
    use documents::*;
    use flow::*;
    use ruff::*;
    use symbols::*;
    use value::*;
    let index = |kind: TypeId| {
        input_relations
            .iter()
            .position(|r| r.type_id() == kind)
            .ok_or(ModelError::Schema("relation scope nominal premise"))
    };
    let root_kind = match kernel {
        K::Reference => TypeId::of::<ReferenceObservation>(),
        K::NativeDefinition => TypeId::of::<RuffDefinitionObservation>(),
        K::Import => TypeId::of::<ImportAliasObservation>(),
        K::Ancestry => TypeId::of::<ClassAncestryObservation>(),
        K::Mention => TypeId::of::<DocumentMentionObservation>(),
        K::Type => TypeId::of::<TypeTerm>(),
        K::Binder => TypeId::of::<TypeVariable>(),
        K::Place => TypeId::of::<Place>(),
        K::TestOperand => TypeId::of::<FlowTestLeafObservation>(),
    };
    let physical = index(root_kind)?;
    let root = inputs.len();
    let mut declarations = inputs;
    declarations.push(declarations[physical].clone());
    let mut b = Builder::new(declarations, root, input_relations, budget)?;
    for (source, relation) in input_relations.iter().enumerate() {
        for field in relation.fields() {
            if let Some((kind, _)) = field.target()
                && let Ok(target) = index(kind)
            {
                b.follow(source, field.name(), target, field.list());
            }
        }
    }
    b.pair(
        root,
        physical,
        &[physical],
        vec![],
        col(0, "id"),
        col(0, "id"),
    );
    if let (Ok(symbols), Ok(resolutions)) = (
        index(TypeId::of::<ProviderSymbol>()),
        index(TypeId::of::<SymbolEntityResolution>()),
    ) {
        b.pair(
            symbols,
            resolutions,
            &[symbols, resolutions],
            vec![eq(1, "symbol", 0, "id")],
            col(0, "id"),
            col(1, "id"),
        );
    }
    // A candidate's qualification context is compared to the operation root, never merely
    // to its supporting native symbol or a globally selected qualification.
    let contextual = |candidate: usize, q: usize, field: &'static str, root_field: &'static str| {
        (
            vec![physical, candidate, q, q],
            vec![
                eq(1, field, 0, root_field),
                eq(2, "id", 0, "qualification"),
                eq(3, "id", 1, "qualification"),
                eq(3, "context", 2, "context"),
            ],
        )
    };
    match kernel {
        K::Reference => {
            if let Ok(q) = index(TypeId::of::<AssertionQualification>()) {
                if let Ok(lexical) = index(TypeId::of::<LexicalResolution>()) {
                    let (rows, p) = contextual(lexical, q, "read", "read");
                    b.pair(root, lexical, &rows, p, col(0, "id"), col(1, "id"));
                }
                if let Ok(contexts) = index(TypeId::of::<RuffContextObservation>()) {
                    let (rows, p) = contextual(contexts, q, "subject", "read");
                    b.pair(root, contexts, &rows, p, col(0, "id"), col(1, "id"));
                    if let Ok(support) = index(TypeId::of::<RuffContextSupport>()) {
                        b.reverse(support, "assertion", contexts);
                    }
                    if let Ok(bindings) = index(TypeId::of::<RuffBindingObservation>()) {
                        b.pair(
                            contexts,
                            bindings,
                            &[contexts, bindings, q, q],
                            vec![
                                eq(1, "event", 0, "final_binding"),
                                eq(2, "id", 0, "qualification"),
                                eq(3, "id", 1, "qualification"),
                                eq(3, "context", 2, "context"),
                            ],
                            col(0, "id"),
                            col(1, "id"),
                        );
                        if let Ok(support) = index(TypeId::of::<RuffBindingSupport>()) {
                            b.reverse(support, "assertion", bindings);
                        }
                    }
                }
            }
        }
        K::NativeDefinition => {
            if let (Ok(candidate), Ok(q)) = (
                index(TypeId::of::<DeclarationObservation>()),
                index(TypeId::of::<AssertionQualification>()),
            ) {
                let (rows, p) = contextual(candidate, q, "declaration", "declaration");
                b.pair(root, candidate, &rows, p, col(0, "id"), col(1, "id"));
            }
            if let Ok(support) = index(TypeId::of::<RuffDefinitionSupport>()) {
                b.reverse(support, "assertion", root);
            }
        }
        K::Import => {
            if let (Ok(candidate), Ok(q)) = (
                index(TypeId::of::<ModuleResolutionObservation>()),
                index(TypeId::of::<AssertionQualification>()),
            ) {
                let (rows, p) = contextual(candidate, q, "alias", "alias");
                b.pair(root, candidate, &rows, p, col(0, "id"), col(1, "id"));
            }
        }
        K::Ancestry => {
            if let Ok(members) = index(TypeId::of::<SymbolSequenceMember>()) {
                b.pair(
                    root,
                    members,
                    &[physical, members],
                    vec![eq(1, "sequence", 0, "ancestors")],
                    col(0, "id"),
                    col(1, "id"),
                );
            }
        }
        K::TestOperand => {
            if let Ok(q) = index(TypeId::of::<AssertionQualification>()) {
                if let Ok(candidate) = index(TypeId::of::<TypeObservation>()) {
                    let (rows, p) = contextual(candidate, q, "subject", "operand");
                    b.pair(root, candidate, &rows, p, col(0, "id"), col(1, "id"));
                }
                if let (Ok(scope), Ok(coverage), Ok(occurrences)) = (
                    index(TypeId::of::<CoverageScope>()),
                    index(TypeId::of::<ProviderCoverage>()),
                    index(TypeId::of::<Occurrence>()),
                ) {
                    b.pair(
                        root,
                        coverage,
                        &[physical, q, occurrences, scope, coverage],
                        vec![
                            eq(1, "id", 0, "qualification"),
                            eq(2, "id", 0, "test"),
                            eq(3, "artifact_artifact", 2, "source"),
                            eq(4, "scope", 3, "id"),
                            eq(4, "context", 1, "context"),
                        ],
                        col(0, "id"),
                        col(4, "id"),
                    );
                }
            }
        }
        K::Binder => {
            const ELIGIBLE: &[i16] = &[
                SyntaxKind::StmtAssign as i16,
                SyntaxKind::StmtAnnAssign as i16,
                SyntaxKind::StmtTypeAlias as i16,
                SyntaxKind::StmtFunctionDef as i16,
                SyntaxKind::StmtClassDef as i16,
            ];
            const BINDINGS: &[i16] = &[
                BindingEventKind::Assignment as i16,
                BindingEventKind::AnnotationOnly as i16,
                BindingEventKind::TypeAlias as i16,
                BindingEventKind::TypeParam as i16,
            ];
            if let (Ok(pm), Ok(module), Ok(occurrences)) = (
                index(TypeId::of::<ProviderModule>()),
                index(TypeId::of::<Module>()),
                index(TypeId::of::<Occurrence>()),
            ) {
                let span = |ancestor: usize| ScopePredicate::SpanContains {
                    outer_start: col(ancestor, "start"),
                    outer_end: col(ancestor, "end"),
                    inner_start: col(0, "anchor_start"),
                    inner_end: col(0, "anchor_end"),
                };
                let p = vec![
                    eq(1, "id", 0, "module"),
                    eq(2, "id", 1, "acquired_module"),
                    eq(3, "source", 2, "source"),
                    span(3),
                    ScopePredicate::CodeIn(col(3, "syntax_kind"), ELIGIBLE),
                ];
                b.pair(
                    root,
                    occurrences,
                    &[physical, pm, module, occurrences],
                    p.clone(),
                    col(0, "id"),
                    col(3, "id"),
                );
                if let (Ok(declarations), Ok(q)) = (
                    index(TypeId::of::<DeclarationObservation>()),
                    index(TypeId::of::<AssertionQualification>()),
                ) {
                    let mut declarations_p = p;
                    declarations_p.extend([
                        eq(4, "declaration", 3, "id"),
                        eq(5, "id", 4, "qualification"),
                        eq(5, "context", 0, "context"),
                    ]);
                    b.pair(
                        root,
                        declarations,
                        &[physical, pm, module, occurrences, declarations, q],
                        declarations_p,
                        col(0, "id"),
                        col(4, "id"),
                    );
                }
                if let (Ok(events), Ok(observations), Ok(q)) = (
                    index(TypeId::of::<BindingEvent>()),
                    index(TypeId::of::<BindingObservation>()),
                    index(TypeId::of::<AssertionQualification>()),
                ) {
                    let rows = vec![
                        physical,
                        pm,
                        module,
                        events,
                        occurrences,
                        occurrences,
                        observations,
                        q,
                    ];
                    let pre = vec![
                        eq(1, "id", 0, "module"),
                        eq(2, "id", 1, "acquired_module"),
                        eq(4, "id", 3, "site"),
                        eq(4, "source", 2, "source"),
                        ScopePredicate::PathPrefix {
                            parent: 5,
                            child: 4,
                        },
                        ScopePredicate::CanonicalOccurrence { row: 5 },
                        ScopePredicate::CodeIn(col(5, "syntax_kind"), ELIGIBLE),
                    ];
                    let post = vec![
                        span(5),
                        eq(6, "event", 3, "id"),
                        ScopePredicate::CodeIn(col(6, "kind"), BINDINGS),
                        eq(7, "id", 6, "qualification"),
                        eq(7, "context", 0, "context"),
                    ];
                    b.program.rules.push(ScopeRule::NearestPairs {
                        source: root,
                        target: observations,
                        rows,
                        rank_rows: 6,
                        predicates: pre,
                        post,
                        source_key: col(0, "id"),
                        target_key: col(6, "id"),
                        event_key: col(3, "id"),
                        ancestor: 5,
                    });
                    // All structural ancestors of selected binding sites allow the finite kernel to repeat
                    // its own deterministic nearest decision without loading another binding body.
                    b.pair(
                        observations,
                        occurrences,
                        &[observations, events, occurrences, occurrences],
                        vec![
                            eq(1, "id", 0, "event"),
                            eq(2, "id", 1, "site"),
                            ScopePredicate::PathPrefix {
                                parent: 3,
                                child: 2,
                            },
                        ],
                        col(0, "id"),
                        col(3, "id"),
                    );
                }
            }
        }
        K::Mention => relation_mention(&mut b, &index, root, physical)?,
        K::Type | K::Place => {}
    }
    if matches!(kernel, K::Reference | K::Place)
        && let (Ok(occurrences), Ok(refs)) = (
            index(TypeId::of::<Occurrence>()),
            index(TypeId::of::<EntityRef>()),
        )
    {
        b.pair(
            occurrences,
            refs,
            &[occurrences, refs],
            vec![eq(1, "occurrence_occurrence", 0, "id")],
            col(0, "id"),
            col(1, "id"),
        );
        for (kind, field) in [
            (TypeId::of::<CallableEntity>(), "callable_callable"),
            (TypeId::of::<ClassEntity>(), "class_class"),
            (TypeId::of::<ParameterEntity>(), "parameter_parameter"),
        ] {
            if let Ok(entity) = index(kind) {
                b.pair(
                    occurrences,
                    refs,
                    &[occurrences, entity, refs],
                    vec![eq(1, "source_declaration", 0, "id"), eq(2, field, 1, "id")],
                    col(0, "id"),
                    col(2, "id"),
                );
            }
        }
    }
    if matches!(kernel, K::Place)
        && let (Ok(roots), Ok(refs)) = (
            index(TypeId::of::<PlaceRoot>()),
            index(TypeId::of::<EntityRef>()),
        )
    {
        b.pair(
            roots,
            refs,
            &[roots, refs],
            vec![eq(1, "module_module", 0, "global_module")],
            col(0, "id"),
            col(1, "id"),
        );
        if let (Ok(classes), Ok(fields)) = (
            index(TypeId::of::<ClassEntity>()),
            index(TypeId::of::<FieldEntity>()),
        ) {
            b.pair(
                roots,
                refs,
                &[roots, classes, fields, refs],
                vec![
                    eq(1, "source_declaration", 0, "field_class"),
                    eq(2, "class", 1, "id"),
                    eq(2, "name", 0, "field_name"),
                    eq(3, "field_field", 2, "id"),
                ],
                col(0, "id"),
                col(3, "id"),
            );
        }
    }
    b.reserve_retained(size_of::<(TypeId, usize)>())?;
    Ok(NormalizationProgram {
        inner: b.finish()?,
        signature_root: None,
        entity_roots: vec![(root_kind, root)],
    })
}
fn relation_mention(
    b: &mut Builder,
    index: &dyn Fn(TypeId) -> Result<usize, ModelError>,
    root: usize,
    physical: usize,
) -> Result<(), ModelError> {
    b.reserve_rules(2 << 20)?;
    use symbols::*;
    macro_rules! port {
        ($ty:ty) => {
            index(TypeId::of::<$ty>())?
        };
    }
    let q = port!(AssertionQualification);
    let scope = port!(CoverageScope);
    let artifact = port!(SourceArtifact);
    let module = port!(Module);
    let libraries = port!(input::CorpusLibrary);
    let symbols = port!(ProviderSymbol);
    let pm = port!(ProviderModule);
    let observations = port!(SymbolObservation);
    let exposures = port!(PublicExposure);
    let public = port!(PublicNameObservation);
    let candidates = port!(PublicExposureCandidate);
    let resolutions = port!(SymbolEntityResolution);
    let entity_candidates = port!(SymbolEntityCandidate);
    // Capture qualification input through the same ordered scope/artifact/module fallbacks as
    // the owner kernel. These finite branches encode COALESCE and contain no physical query text.
    let captured = |base: usize, branch: usize| -> (ScopeColumn, Vec<ScopePredicate>) {
        match branch {
            0 => (col(base + 2, "input_input"), vec![]),
            1 => (
                col(base + 3, "input"),
                vec![ScopePredicate::IsNull(col(base + 2, "input_input"), true)],
            ),
            _ => (
                col(base + 5, "input"),
                vec![
                    ScopePredicate::IsNull(col(base + 2, "input_input"), true),
                    ScopePredicate::IsNull(col(base + 3, "input"), true),
                ],
            ),
        }
    };
    let base_rows = vec![physical, q, scope, artifact, module, artifact];
    let base_predicates = vec![eq(1, "id", 0, "qualification"), eq(2, "id", 1, "scope")];
    let base_optional = vec![
        ScopeOptionalJoin {
            row: 3,
            keys: vec![(col(3, "id"), col(2, "artifact_artifact"))],
        },
        ScopeOptionalJoin {
            row: 4,
            keys: vec![(col(4, "id"), col(2, "module_module"))],
        },
        ScopeOptionalJoin {
            row: 5,
            keys: vec![(col(5, "id"), col(4, "source"))],
        },
    ];
    for own_branch in 0..3 {
        let (own, own_predicates) = captured(0, own_branch);
        let mut p = base_predicates.clone();
        p.extend(own_predicates.clone());
        p.push(ScopePredicate::Equal(col(6, "corpus"), own));
        let mut rows = base_rows.clone();
        rows.push(libraries);
        b.program.rules.push(ScopeRule::OptionalPairs {
            source: root,
            target: libraries,
            rows,
            predicates: p,
            optional: base_optional.clone(),
            source_key: col(0, "id"),
            target_key: col(6, "id"),
        });
        for linked in [false, true] {
            for name in ["access_path", "qualified_name"] {
                let mut rows = base_rows.clone();
                rows.extend([exposures, module, artifact, public]);
                let mut p = base_predicates.clone();
                p.extend(own_predicates.clone());
                p.extend([
                    eq(7, "id", 6, "access"),
                    eq(8, "id", 7, "source"),
                    eq(9, "id", 6, "observation"),
                    ScopePredicate::QualifiedName {
                        wanted: col(0, name),
                        module: col(7, "qualified_name"),
                        leaf: col(9, "name"),
                        leaf_match: false,
                    },
                ]);
                if linked {
                    rows.push(libraries);
                    p.extend([
                        ScopePredicate::Equal(col(10, "corpus"), own),
                        eq(8, "input", 10, "library"),
                    ]);
                } else {
                    p.push(ScopePredicate::Equal(col(8, "input"), own));
                }
                b.program.rules.push(ScopeRule::OptionalPairs {
                    source: root,
                    target: exposures,
                    rows,
                    predicates: p,
                    optional: base_optional.clone(),
                    source_key: col(0, "id"),
                    target_key: col(6, "id"),
                });
            }
            for target_branch in 0..3 {
                for module_branch in 0..4 {
                    // 6 observation,7 symbol,8 provider module,9 acquired module;10 qualification,
                    // 11 target scope,12 artifact,13 scope module,14 module artifact.
                    let mut rows = base_rows.clone();
                    rows.extend([
                        observations,
                        symbols,
                        pm,
                        module,
                        q,
                        scope,
                        artifact,
                        module,
                        artifact,
                    ]);
                    let mut p = base_predicates.clone();
                    p.extend(own_predicates.clone());
                    p.extend([
                        eq(7, "id", 6, "symbol"),
                        eq(8, "id", 7, "module"),
                        eq(10, "id", 6, "qualification"),
                        eq(11, "id", 10, "scope"),
                    ]);
                    let mut optional = base_optional.clone();
                    optional.extend([
                        ScopeOptionalJoin {
                            row: 9,
                            keys: vec![(col(9, "id"), col(8, "acquired_module"))],
                        },
                        ScopeOptionalJoin {
                            row: 12,
                            keys: vec![(col(12, "id"), col(11, "artifact_artifact"))],
                        },
                        ScopeOptionalJoin {
                            row: 13,
                            keys: vec![(col(13, "id"), col(11, "module_module"))],
                        },
                        ScopeOptionalJoin {
                            row: 14,
                            keys: vec![(col(14, "id"), col(13, "source"))],
                        },
                    ]);
                    let target = match target_branch {
                        0 => col(11, "input_input"),
                        1 => {
                            p.push(ScopePredicate::IsNull(col(11, "input_input"), true));
                            col(12, "input")
                        }
                        _ => {
                            p.extend([
                                ScopePredicate::IsNull(col(11, "input_input"), true),
                                ScopePredicate::IsNull(col(12, "input"), true),
                            ]);
                            col(14, "input")
                        }
                    };
                    let module_columns = [
                        col(9, "qualified_name"),
                        col(8, "bundled_name"),
                        col(8, "namespace_name"),
                        col(8, "unresolved_name"),
                    ];
                    for previous in &module_columns[..module_branch] {
                        p.push(ScopePredicate::IsNull(*previous, true));
                    }
                    if module_branch < 3 {
                        p.push(ScopePredicate::IsNull(module_columns[module_branch], false));
                    }
                    p.push(ScopePredicate::QualifiedName {
                        wanted: col(0, "qualified_name"),
                        module: module_columns[module_branch],
                        leaf: col(7, "name"),
                        leaf_match: true,
                    });
                    if linked {
                        rows.push(libraries);
                        p.extend([
                            ScopePredicate::Equal(col(15, "corpus"), own),
                            ScopePredicate::Equal(target, col(15, "library")),
                        ]);
                    } else {
                        p.push(ScopePredicate::Equal(target, own));
                    }
                    b.program.rules.push(ScopeRule::OptionalPairs {
                        source: root,
                        target: observations,
                        rows,
                        predicates: p,
                        optional,
                        source_key: col(0, "id"),
                        target_key: col(6, "id"),
                    });
                }
            }
        }
    }
    b.reverse(candidates, "exposure", exposures);
    // Parent observations retain every alternative; ambiguous parents remain a model verdict.
    b.pair(
        observations,
        observations,
        &[observations, observations],
        vec![eq(1, "symbol", 0, "parent")],
        col(0, "id"),
        col(1, "id"),
    );
    b.reverse(entity_candidates, "resolution", resolutions);
    Ok(())
}
#[cfg(test)]
mod relation_program_controls {
    use super::*;
    #[test]
    fn all_relation_candidate_domains_validate_and_keep_demand_isolated() {
        use super::super::relation_normalization::{RelationData, RelationKernel as K};
        let owner = super::super::super::model().unwrap();
        let budget = resources::ResourceBudget::fixed(16 << 20).unwrap();
        let inputs = RelationData::validation_inputs();
        let records = inputs
            .iter()
            .map(|input| owner.relation(input.name()).unwrap().clone())
            .collect::<Vec<_>>();
        for kernel in [
            K::Reference,
            K::NativeDefinition,
            K::Import,
            K::Ancestry,
            K::Mention,
            K::Type,
            K::Binder,
            K::Place,
            K::TestOperand,
        ] {
            let program = relation(inputs.clone(), &records, kernel, &budget).unwrap();
            program.program().validate(&owner).unwrap();
            let root = program.entity_roots()[0].1;
            assert!(program.program().ports[root].virtual_owner);
            assert!(!program.program().rules.iter().any(|rule|matches!(rule,ScopeRule::Reference{target,direction:ScopeDirection::Forward,..} if *target==root)));
            if matches!(kernel, K::Binder) {
                assert!(
                    program
                        .program()
                        .rules
                        .iter()
                        .any(|r| matches!(r, ScopeRule::NearestPairs { rank_rows: 6, .. }))
                );
            }
            drop(program);
            assert_eq!(budget.reserved(), 0);
        }
    }
}
#[cfg(test)]
mod missing_candidate_controls {
    use super::*;
    #[test]
    fn absent_optional_candidate_namespaces_keep_explicit_relation_roots() {
        use super::super::relation_normalization::RelationKernel as K;
        let owner = super::super::super::model().unwrap();
        let budget = resources::ResourceBudget::fixed(8 << 20).unwrap();
        for (input, record, kernel) in [
            (
                ValidationInput::of::<ReferenceObservation>(&["id"]),
                Relation::of::<ReferenceObservation>(),
                K::Reference,
            ),
            (
                ValidationInput::of::<TypeTerm>(&["id"]),
                Relation::of::<TypeTerm>(),
                K::Type,
            ),
            (
                ValidationInput::of::<TypeVariable>(&["id"]),
                Relation::of::<TypeVariable>(),
                K::Binder,
            ),
        ] {
            let program = relation(vec![input], &[record], kernel, &budget).unwrap();
            program.program().validate(&owner).unwrap();
            assert_eq!(program.program().ports.len(), 2);
            assert!(program.program().ports[1].virtual_owner);
            assert_eq!(program.program().rules.iter().filter(|rule|matches!(rule,ScopeRule::Pairs{source:1,target:0,source_key:ScopeColumn{row:0,field:"id"},target_key:ScopeColumn{row:0,field:"id"},rows,predicates} if rows==&[0]&&predicates.is_empty())).count(),1);
            assert!(program.program().rules.iter().all(|rule| matches!(
                rule,
                ScopeRule::Pairs {
                    source: 1,
                    target: 0,
                    ..
                } | ScopeRule::Reference {
                    source: 0,
                    target: 0,
                    direction: ScopeDirection::Forward,
                    ..
                }
            )));
            if matches!(kernel, K::Type) {
                // Type terms retain their recursive nominal children even when every optional candidate
                // namespace is absent. Those ordinary references are not additional root selectors.
                let _fixture = budget.reserve("absent-relation-fixture", 8192).unwrap();
                let child = TypeTerm::None;
                let selected = TypeTerm::TypeOf { target: child.id() };
                let unrelated = TypeTerm::Any {
                    flavor: AnyFlavor::Explicit,
                };
                assert_eq!(program.program().inputs.len(), 2);
                assert_eq!(program.program().ports[0].input, 0);
                assert_eq!(program.program().ports[1].input, 1);
                // The virtual port repeats its exact physical declaration. The finite realization must
                // provide the same immutable typed inventory for each declared input slot, just as native
                // binding supplies the same relation alias twice; a port is not an extra missing namespace.
                let adapter = || {
                    vec![
                        super::super::super::finite_scope::FiniteScopeRow::of(&child),
                        super::super::super::finite_scope::FiniteScopeRow::of(&selected),
                        super::super::super::finite_scope::FiniteScopeRow::of(&unrelated),
                    ]
                };
                let rows = program
                    .program()
                    .inputs
                    .iter()
                    .map(|input| {
                        assert_eq!(input.type_id(), TypeId::of::<TypeTerm>());
                        assert_eq!(input.prefix(), program.program().inputs[0].prefix());
                        assert_eq!(input.order(), program.program().inputs[0].order());
                        adapter()
                    })
                    .collect();
                let finite = super::super::super::finite_scope::FiniteScope::new(
                    program.program().clone(),
                    rows,
                    &owner,
                    &budget,
                )
                .unwrap();
                let result = finite
                    .select(
                        &[(1, *selected.id().bytes())],
                        &ScopeParameters(vec![]),
                        &budget,
                    )
                    .unwrap();
                let partition = result.partition(0).unwrap();
                assert!(partition.contains(&(1, *selected.id().bytes())));
                assert!(partition.contains(&(0, *selected.id().bytes())));
                assert!(partition.contains(&(0, *child.id().bytes())));
                assert!(!partition.contains(&(0, *unrelated.id().bytes())));
                assert_eq!(partition.len(), 3);
            }
            drop(program);
            assert_eq!(budget.reserved(), 0);
        }
    }
}
