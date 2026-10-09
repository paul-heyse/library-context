//! Upper compiler scope intent. Physical aliases and SQL belong to mechanical lowerings.
use super::{scope_program::*, *};
use std::any::TypeId;

/// Reserve before copying declarations or constructing an upper compiler program. The bounded
/// families add at most a fixed group of rules per descriptor/reference field; each Model target
/// adds four pair rules and their small owned row/predicate vectors. This arena also covers
/// temporary type sets, candidate vectors and canonical preparation while the raw program lives.
pub fn reserve_construction(
    input_count: usize,
    field_count: usize,
    target_count: usize,
    budget: &resources::ResourceBudget,
) -> Result<Box<dyn resources::Reservation>, ModelError> {
    let bytes = 524_288usize
        .saturating_add(input_count.saturating_mul(8192))
        .saturating_add(field_count.saturating_mul(4096))
        .saturating_add(target_count.saturating_mul(32_768));
    budget.reserve("compiler-scope-construction", bytes)
}

struct Builder {
    program: ScopeProgram,
}
impl Builder {
    fn new(inputs: Vec<ValidationInput>, real: usize) -> Self {
        let ports = (0..inputs.len())
            .map(|input| ScopePort {
                input,
                virtual_owner: input >= real,
            })
            .collect();
        Self {
            program: ScopeProgram {
                inputs,
                ports,
                rules: Vec::new(),
            },
        }
    }
    fn follow(
        &mut self,
        source: usize,
        field: &'static str,
        target: usize,
    ) -> Result<(), ModelError> {
        self.program.rules.push(ScopeRule::Reference {
            source,
            field,
            target,
            direction: ScopeDirection::Forward,
            list: false,
        });
        Ok(())
    }
    fn own(&mut self, source: usize, field: &'static str, target: usize) -> Result<(), ModelError> {
        // Ownership of a virtual publication namespace does not let an ordinary supporting
        // reference activate that namespace. Its outgoing dependencies come from the real row.
        if !self.program.ports[target].virtual_owner {
            self.follow(source, field, target)?;
        }
        self.program.rules.push(ScopeRule::Reference {
            source,
            field,
            target,
            direction: ScopeDirection::OwnedReverse,
            list: false,
        });
        Ok(())
    }
    fn pair(
        &mut self,
        source: usize,
        target: usize,
        rows: &[usize],
        predicates: Vec<ScopePredicate>,
        source_row: usize,
        target_row: usize,
    ) {
        self.program.rules.push(ScopeRule::Pairs {
            source,
            target,
            rows: rows.to_vec(),
            predicates,
            source_key: col(source_row, "id"),
            target_key: col(target_row, "id"),
        });
    }
}
fn col(row: usize, field: &'static str) -> ScopeColumn {
    ScopeColumn { row, field }
}
fn eq(a: usize, af: &'static str, b: usize, bf: &'static str) -> ScopePredicate {
    ScopePredicate::Equal(col(a, af), col(b, bf))
}

/// Summary selects one complete actual frame. Its transfer/SCC kernel cannot be partitioned
/// into publication members; compatible discovery stays within this explicit frame demand.
pub fn summary(
    inputs: Vec<ValidationInput>,
    relations: &[Relation],
    real: usize,
    frame: usize,
    source: usize,
    parents: &[(TypeId, usize, usize)],
    event: usize,
) -> Result<ScopeProgram, ModelError> {
    let idx = |kind: TypeId| {
        relations[..real]
            .iter()
            .position(|relation| relation.type_id() == kind)
    };
    let owner_idx = |kind: TypeId| {
        parents
            .iter()
            .find(|(parent, _, _)| *parent == kind)
            .map(|(_, _, root)| *root)
            .or_else(|| idx(kind))
    };
    let native: std::collections::BTreeSet<_> =
        normalized::binding_normalization::BindingData::validation_inputs()
            .into_iter()
            .chain(conditions::entry::EntryData::facts_inputs())
            .chain(execution::summary_path::PathData::inputs())
            .map(|input| input.type_id())
            .collect();
    let mut b = Builder::new(inputs, real);
    for (from, relation) in relations[..real].iter().enumerate() {
        let epoch = if stages::is_vocabulary(relation.name()) {
            b.program.inputs[from].prefix()
        } else if native.contains(&relation.type_id()) {
            Some(stages::PublicationBoundary::Facts)
        } else {
            Some(stages::PublicationBoundary::Model)
        };
        for field in relation.fields().iter().filter(|field| !field.list()) {
            if let Some((kind, _)) = field.target() {
                let target = relations[..real]
                    .iter()
                    .enumerate()
                    .find(|(i, t)| {
                        t.type_id() == kind
                            && (!stages::is_vocabulary(t.name())
                                || b.program.inputs[*i].prefix() == epoch)
                    })
                    .map(|(i, _)| i);
                if let Some(to) = target {
                    b.follow(from, field.name(), to)?;
                }
            }
        }
    }
    macro_rules! own {
        ($member:ty,$field:literal,$owner:ty) => {{
            if let (Some(member), Some(owner)) = (
                idx(TypeId::of::<$member>()),
                owner_idx(TypeId::of::<$owner>()),
            ) {
                b.own(member, $field, owner)?;
            }
        }};
    }
    macro_rules! owned_fields {
        ($member:ty,$owner:ty) => {{
            if let (Some(member), Some(owner)) =
                (idx(TypeId::of::<$member>()), idx(TypeId::of::<$owner>()))
            {
                for field in relations[member].fields().iter().filter(|field| {
                    !field.list()
                        && field.target().map(|(kind, _)| kind) == Some(TypeId::of::<$owner>())
                }) {
                    b.own(member, field.name(), owner)?;
                }
            }
        }};
    }
    for kind in [
        TypeId::of::<analysis::MethodParameters>(),
        TypeId::of::<analysis::AnalysisDefinition>(),
    ] {
        if let Some(target) = idx(kind) {
            b.pair(frame, target, &[frame, target], vec![], 0, 1);
        }
    }
    b.pair(
        frame,
        source,
        &[frame, source],
        vec![eq(1, "input", 0, "input"), eq(1, "context", 0, "context")],
        0,
        1,
    );
    for (_, original, root) in parents {
        b.pair(
            frame,
            *root,
            &[frame, *root],
            vec![eq(1, "input", 0, "input"), eq(1, "context", 0, "context")],
            0,
            1,
        );
        b.pair(*root, *original, &[*root], vec![], 0, 0);
    }
    own!(
        local_semantics::LocalContribution,
        "invocation",
        analysis::local::AnalysisInvocation
    );
    own!(
        local_semantics::LocalGuardContribution,
        "invocation",
        analysis::local::AnalysisInvocation
    );
    own!(
        local_symbolic::SymbolicFieldStore,
        "invocation",
        analysis::local::AnalysisInvocation
    );
    own!(
        atom_decision::AtomRestriction,
        "invocation",
        analysis::local::AnalysisInvocation
    );
    own!(
        analysis::local::AnalysisDerivation,
        "invocation",
        analysis::local::AnalysisInvocation
    );
    own!(
        analysis::local::AnalysisOutcome,
        "invocation",
        analysis::local::AnalysisInvocation
    );
    own!(
        analysis::model::AnalysisDerivation,
        "invocation",
        analysis::model::AnalysisInvocation
    );
    own!(
        analysis::model::AnalysisOutcome,
        "invocation",
        analysis::model::AnalysisInvocation
    );
    own!(
        execution::protocol_interpretation::ConditionalTerminalFrontier,
        "invocation",
        analysis::model::AnalysisInvocation
    );
    own!(
        execution::protocol_interpretation::NormalContinuationRestriction,
        "frontier",
        execution::protocol_interpretation::ConditionalTerminalFrontier
    );
    own!(
        execution::capture_bridge::CapturedEntryBinding,
        "invocation",
        analysis::enriched_execution::AnalysisInvocation
    );
    own!(
        execution::enriched_records::SourceExecutionInvocation,
        "invocation",
        analysis::enriched_execution::AnalysisInvocation
    );
    own!(
        execution::enriched_records::BodyExecution,
        "invocation",
        analysis::enriched_execution::AnalysisInvocation
    );
    own!(
        analysis::enriched_execution::AnalysisOutcome,
        "invocation",
        analysis::enriched_execution::AnalysisInvocation
    );
    own!(
        analysis::source_call::AnalysisOutcome,
        "invocation",
        analysis::source_call::AnalysisInvocation
    );
    owned_fields!(
        analysis::local::SupportSource,
        analysis::local::AnalysisDerivation
    );
    owned_fields!(
        analysis::model::SupportSource,
        analysis::model::AnalysisDerivation
    );
    own!(
        transfer::local::TransferAlternative,
        "transfer",
        transfer::local::TransferKey
    );
    own!(
        transfer::local::TransferSupport,
        "source",
        analysis::local::SupportSource
    );
    own!(
        transfer::model::TransferSupport,
        "source",
        analysis::model::SupportSource
    );
    own!(
        transfer::local::TransferSupport,
        "assertion",
        transfer::local::TransferAlternative
    );
    own!(
        transfer::model::TransferSupport,
        "assertion",
        transfer::model::TransferAlternative
    );
    own!(
        execution::source_call_records::SourceCallHeader,
        "attempt",
        normalized::bindings::CallBindingAttempt
    );
    let occurrences = idx(TypeId::of::<source::Occurrence>())
        .ok_or(ModelError::Schema(source::Occurrence::NAME))?;
    let artifacts = idx(TypeId::of::<source::SourceArtifact>())
        .ok_or(ModelError::Schema(source::SourceArtifact::NAME))?;
    let qualifications = idx(TypeId::of::<assertion::AssertionQualification>())
        .ok_or(ModelError::Schema(assertion::AssertionQualification::NAME))?;
    b.pair(
        frame,
        event,
        &[frame, event, occurrences, artifacts],
        vec![
            eq(1, "context", 0, "context"),
            eq(2, "id", 1, "site"),
            eq(3, "id", 2, "source"),
            eq(3, "input", 0, "input"),
        ],
        0,
        1,
    );
    for (kind, subject) in [
        (TypeId::of::<flow::FlowValueObservation>(), "sink"),
        (
            TypeId::of::<normalized::symbolic_fields::SourceFieldClass>(),
            "class",
        ),
        (
            TypeId::of::<normalized::symbolic_fields::SourceFieldStore>(),
            "target",
        ),
        (
            TypeId::of::<normalized::symbolic_fields::SourceFieldReader>(),
            "access",
        ),
    ] {
        if let Some(target) = idx(kind) {
            b.pair(
                frame,
                target,
                &[frame, target, qualifications, occurrences, artifacts],
                vec![
                    eq(2, "id", 1, "qualification"),
                    eq(2, "context", 0, "context"),
                    eq(3, "id", 1, subject),
                    eq(4, "id", 3, "source"),
                    eq(4, "input", 0, "input"),
                ],
                0,
                1,
            );
        }
    }
    own!(
        normalized::symbolic_fields::SourceFieldAssociation,
        "class",
        normalized::symbolic_fields::SourceFieldClass
    );
    own!(
        normalized::symbolic_fields::SourceFieldReaderLink,
        "association",
        normalized::symbolic_fields::SourceFieldAssociation
    );
    own!(
        normalized::symbolic_fields::SourceFieldReaderLink,
        "reader",
        normalized::symbolic_fields::SourceFieldReader
    );
    own!(
        flow::FlowValuePathObservation,
        "value",
        flow::FlowValueObservation
    );
    own!(flow::FlowCallStep, "path", flow::FlowCallPath);
    b.pair(
        occurrences,
        occurrences,
        &[occurrences, occurrences],
        vec![
            eq(1, "source", 0, "source"),
            eq(1, "start", 0, "start"),
            eq(1, "end", 0, "end"),
            eq(1, "syntax_kind", 0, "syntax_kind"),
        ],
        0,
        1,
    );
    own!(flow::FlowUse, "occurrence", source::Occurrence);
    own!(flow::FlowUseObservation, "use_", flow::FlowUse);
    own!(flow::FlowReachingObservation, "use_", flow::FlowUse);
    own!(
        flow_inventory::FlowUseInventoryObservation,
        "use_",
        flow::FlowUse
    );
    own!(
        flow_inventory::FlowUseCandidate,
        "inventory",
        flow_inventory::FlowUseInventoryObservation
    );
    own!(
        flow_inventory::FlowUseInventoryMember,
        "inventory",
        flow_inventory::FlowUseInventoryObservation
    );
    own!(
        flow::FlowDefinitionObservation,
        "definition",
        flow::FlowDefinition
    );
    own!(flow::FlowRegionObservation, "scope", lexical::LexicalScope);
    own!(syntax::SyntaxPlacement, "occurrence", source::Occurrence);
    own!(
        normalized::entities::ParameterEntityLink,
        "entity",
        normalized::entities::ParameterEntity
    );
    own!(
        declarations::ParameterDeclaration,
        "parameter",
        calls::SignatureParameter
    );
    own!(
        declarations::SymbolDeclaration,
        "symbol",
        calls::ProviderSymbol
    );
    owned_fields!(normalized::entities::CallableEntity, source::Occurrence);
    owned_fields!(normalized::entities::CallableEntity, calls::ProviderSymbol);
    owned_fields!(
        normalized::entities::EntityRef,
        normalized::entities::CallableEntity
    );
    own!(types::TypeSequenceMember, "sequence", types::TypeSequence);
    // Set memberships are independent in each selected epoch; condition nodes and finite path
    // segments already have forward nominal edges to their exact roots/children.
    for epoch in [
        stages::PublicationBoundary::Facts,
        stages::PublicationBoundary::Model,
    ] {
        let at = |kind: TypeId| {
            relations[..real].iter().enumerate().position(|(i, t)| {
                t.type_id() == kind && b.program.inputs[i].prefix() == Some(epoch)
            })
        };
        if let (Some(member), Some(owner)) = (
            at(TypeId::of::<assumptions::AssumptionSetMember>()),
            at(TypeId::of::<assumptions::AssumptionSet>()),
        ) {
            b.own(member, "set", owner)?;
        }
    }
    // All independently attributed supports of a selected assertion participate; there is no
    // generic reverse source/input edge that would turn evidence into a whole-input collector.
    for (member, table) in relations[..real].iter().enumerate() {
        if let Some(field) = table
            .fields()
            .iter()
            .find(|field| field.name() == "assertion")
            && let Some((kind, _)) = field.target()
            && let Some(owner) = idx(kind)
        {
            b.own(member, field.name(), owner)?;
        }
    }
    if let Some(premises) = idx(TypeId::of::<analysis::native::NativeAssertionPremise>()) {
        for field in relations[premises]
            .fields()
            .iter()
            .filter(|field| !field.list())
        {
            if let Some((kind, _)) = field.target()
                && let Some(owner) = idx(kind)
            {
                b.own(premises, field.name(), owner)?;
            }
        }
        if let Some(native) = idx(TypeId::of::<analysis::native::NativeQualification>()) {
            b.own(native, "premise", premises)?;
        }
    }
    if let (Some(values), Some(placements)) = (
        idx(TypeId::of::<source::Occurrence>()),
        idx(TypeId::of::<syntax::SyntaxPlacement>()),
    ) {
        for code in [
            source::SyntaxKind::StmtReturn.code(),
            source::SyntaxKind::StmtRaise.code(),
            source::SyntaxKind::StmtTry.code(),
        ] {
            b.pair(
                values,
                placements,
                &[values, placements],
                vec![
                    eq(1, "parent", 0, "id"),
                    ScopePredicate::Code(col(0, "syntax_kind"), code),
                ],
                0,
                1,
            );
        }
    }
    Ok(b.program)
}

#[derive(Clone, Copy)]
pub enum FrameKind {
    Structural,
    Analytic,
}
/// Structural and Analytic retain their actual complete frame universes. Graph algorithms,
/// projections and configured analytics consume those universes without root-member slicing.
pub fn analytical(
    inputs: Vec<ValidationInput>,
    relations: &[Relation],
    real: usize,
    source: usize,
    kind: FrameKind,
) -> Result<ScopeProgram, ModelError> {
    use FrameKind as Kind;
    let mut structural_types = std::collections::BTreeSet::new();
    macro_rules! structural_types {($($field:ident:$ty:ty,)*)=>{$(structural_types.insert(TypeId::of::<$ty>());)*};}
    lctx_model::structural_outputs!(structural_types);
    let target_for = |from: usize, target: TypeId| -> Result<Option<usize>, ModelError> {
        let candidates = inputs[..real]
            .iter()
            .enumerate()
            .filter(|(_, input)| input.type_id() == target)
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            return Ok(None);
        }
        if let [only] = candidates.as_slice() {
            return Ok(Some(*only));
        }
        let epoch = inputs[from].prefix().unwrap_or_else(|| {
            if [
                TypeId::of::<local_semantics::LocalContribution>(),
                TypeId::of::<local_semantics::LocalAssessment>(),
            ]
            .contains(&inputs[from].type_id())
            {
                stages::PublicationBoundary::Local
            } else if structural_types.contains(&inputs[from].type_id()) {
                stages::PublicationBoundary::Structural
            } else {
                stages::PublicationBoundary::Facts
            }
        });
        let matches = candidates
            .into_iter()
            .filter(|i| inputs[*i].prefix() == Some(epoch))
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [only] => Ok(Some(*only)),
            [] => Err(ModelError::Conflict("analytical dependency epoch absent")),
            _ => Err(ModelError::Conflict(
                "analytical dependency epoch ambiguous",
            )),
        }
    };
    let root = real;
    let idx = |kind: TypeId| {
        relations[..real]
            .iter()
            .position(|relation| relation.type_id() == kind)
    };
    let mut b = Builder::new(inputs.clone(), real);
    for (from, relation) in relations[..real].iter().enumerate() {
        for field in relation.fields() {
            if let Some((target, _)) = field.target()
                && let Some(to) = target_for(from, target)?
            {
                b.program.rules.push(ScopeRule::Reference {
                    source: from,
                    field: field.name(),
                    target: to,
                    direction: ScopeDirection::Forward,
                    list: field.list(),
                });
            }
        }
    }
    b.pair(root, source, &[root], vec![], 0, 0);
    macro_rules! own {
        ($member:ty,$field:literal,$owner:ty) => {
            for (member, input) in inputs[..real]
                .iter()
                .enumerate()
                .filter(|(_, input)| input.type_id() == TypeId::of::<$member>())
            {
                let _ = input;
                if let Some(owner) = target_for(member, TypeId::of::<$owner>())? {
                    b.own(member, $field, owner)?;
                }
            }
        };
    }
    // Native supports and ordered list members are semantic ownership, not arbitrary incoming refs.
    for (member, table) in relations[..real].iter().enumerate() {
        if let Some(field) = table
            .fields()
            .iter()
            .find(|field| field.name() == "assertion")
            && let Some((target, _)) = field.target()
            && let Some(owner) = target_for(member, target)?
        {
            b.own(member, field.name(), owner)?;
        }
    }
    own!(input::ArtifactUse, "artifact", source::SourceArtifact);
    own!(
        normalized::entities::OccurrenceOwnership,
        "occurrence",
        source::Occurrence
    );
    own!(
        normalized::entities::SymbolEntityResolution,
        "symbol",
        calls::ProviderSymbol
    );
    own!(
        normalized::entities::ParameterEntityLink,
        "entity",
        normalized::entities::ParameterEntity
    );
    own!(
        normalized::entities::ParameterEntityLink,
        "parameter",
        calls::SignatureParameter
    );
    own!(types::TypeSequenceMember, "sequence", types::TypeSequence);
    own!(types::TypedDictField, "list", types::TypedDictFieldList);
    own!(
        types::CallableParameter,
        "list",
        types::CallableParameterList
    );
    own!(
        symbols::SymbolSequenceMember,
        "sequence",
        symbols::SymbolSequence
    );
    own!(
        assumptions::AssumptionSetMember,
        "set",
        assumptions::AssumptionSet
    );
    own!(calls::SignatureParameter, "signature", calls::Signature);
    own!(
        normalized::callables::SignatureSlot,
        "variant",
        normalized::callables::SignatureVariant
    );
    own!(
        normalized::callables::SignatureSlotEntity,
        "slot",
        normalized::callables::SignatureSlot
    );
    own!(
        normalized::callables::EffectiveCallableEvidence,
        "assessment",
        normalized::callables::EffectiveCallableAssessment
    );
    own!(
        normalized::callables::EffectiveDecoratorMember,
        "assessment",
        normalized::callables::EffectiveCallableAssessment
    );
    own!(
        syntax::ParameterSyntaxObservation,
        "function",
        source::Occurrence
    );
    own!(
        declarations::ParameterDeclaration,
        "declaration",
        source::Occurrence
    );
    own!(
        declarations::ParameterDeclaration,
        "parameter",
        calls::SignatureParameter
    );
    own!(
        normalized::entities::EntityRef,
        "callable_callable",
        normalized::entities::CallableEntity
    );
    own!(
        syntax::DeclarationDecorator,
        "declaration",
        source::Occurrence
    );
    own!(types::TypeObservation, "subject", source::Occurrence);
    own!(
        types::FunctionBodyObservation,
        "declaration",
        source::Occurrence
    );
    own!(
        types::NativeSignatureObservation,
        "signature",
        calls::Signature
    );
    own!(
        types::SignatureTypeObservation,
        "subject",
        types::SignatureTypeSubject
    );
    own!(
        class_metadata::ClassMetadataObservation,
        "class",
        calls::ProviderSymbol
    );
    own!(
        class_metadata::ClassMemberObservation,
        "class",
        calls::ProviderSymbol
    );
    own!(
        captures::CaptureObservation,
        "function",
        calls::ProviderSymbol
    );
    own!(
        protocols::NativeExitObservation,
        "subject",
        source::Occurrence
    );
    own!(
        protocols::NativeTerminalObservation,
        "subject",
        source::Occurrence
    );
    own!(
        normalized::events::CallEventSource,
        "event",
        normalized::events::NormalizedCallEvent
    );
    own!(
        normalized::events::CallEventSourceEvidence,
        "source",
        normalized::events::CallEventSource
    );
    own!(
        normalized::events::CallEventResolution,
        "event",
        normalized::events::NormalizedCallEvent
    );
    own!(
        normalized::events::CallEventResolutionEvidence,
        "resolution",
        normalized::events::CallEventResolution
    );
    own!(
        normalized::events::NormalizedCallAlternative,
        "event",
        normalized::events::NormalizedCallEvent
    );
    own!(
        normalized::events::CallAlternativeEvidence,
        "alternative",
        normalized::events::NormalizedCallAlternative
    );
    own!(
        normalized::events::EventAssessment,
        "event",
        normalized::events::NormalizedCallEvent
    );
    own!(
        normalized::events::EventPhaseTarget,
        "assessment",
        normalized::events::EventAssessment
    );
    own!(
        normalized::events::CallPolicyAssessment,
        "event",
        normalized::events::NormalizedCallEvent
    );
    own!(
        normalized::events::CallPolicyAdmission,
        "assessment",
        normalized::events::CallPolicyAssessment
    );
    own!(
        normalized::bindings::CallBindingAttempt,
        "event",
        normalized::events::NormalizedCallEvent
    );
    own!(
        normalized::bindings::CallBinding,
        "attempt",
        normalized::bindings::CallBindingAttempt
    );
    own!(flow::FlowUseObservation, "use_", flow::FlowUse);
    own!(flow::FlowReachingObservation, "use_", flow::FlowUse);
    own!(
        flow::FlowDefinitionObservation,
        "definition",
        flow::FlowDefinition
    );
    own!(
        flow_inventory::FlowUseInventoryMember,
        "inventory",
        flow_inventory::FlowUseInventoryObservation
    );
    own!(
        flow_inventory::FlowUseInventoryObservation,
        "use_",
        flow::FlowUse
    );
    own!(
        flow_inventory::FlowUseCandidate,
        "inventory",
        flow_inventory::FlowUseInventoryObservation
    );
    own!(
        conditions::entry::EntryValueWitness,
        "use_observation",
        flow::FlowUseObservation
    );
    own!(flow::FlowUse, "occurrence", source::Occurrence);
    own!(flow::FlowDefinition, "occurrence", source::Occurrence);
    own!(catalog::CatalogCallable, "member", catalog::CatalogMember);
    own!(calls::CallArgument, "call", calls::CallSyntax);
    own!(
        calls::CallResolutionMember,
        "resolution",
        calls::CallResolution
    );
    own!(
        syntax::DeclarationObservation,
        "declaration",
        source::Occurrence
    );
    own!(
        declarations::SymbolDeclaration,
        "declaration",
        source::Occurrence
    );
    own!(lexical::ReferenceObservation, "read", source::Occurrence);
    own!(
        normalized::links::ReferenceEntityAssessment,
        "reference",
        lexical::ReferenceObservation
    );
    own!(
        normalized::links::ReferenceEntityCandidate,
        "assessment",
        normalized::links::ReferenceEntityAssessment
    );
    own!(
        symbols::FunctionTraitObservation,
        "symbol",
        calls::ProviderSymbol
    );
    own!(
        symbols::ClassAncestryObservation,
        "class",
        calls::ProviderSymbol
    );
    own!(
        calls::SignatureEnumerationObservation,
        "symbol",
        calls::ProviderSymbol
    );
    own!(
        calls::SignatureEnumerationMember,
        "enumeration",
        calls::SignatureEnumerationObservation
    );
    own!(
        normalized::callables::SignatureSlotType,
        "slot",
        normalized::callables::SignatureSlot
    );
    for (member, table) in relations[..real]
        .iter()
        .enumerate()
        .filter(|(_, table)| table.type_id() == TypeId::of::<types::SignatureTypeSubject>())
    {
        for field in table.fields() {
            if let Some((kind, _)) = field.target()
                && let Some(owner) = idx(kind)
            {
                b.own(member, field.name(), owner)?;
            }
        }
    }

    let frame_rows = match kind {
        Kind::Structural => vec![root],
        Kind::Analytic => vec![
            root,
            idx(TypeId::of::<analysis::structural::Invocation>())
                .ok_or(ModelError::Schema("analytic parent"))?,
        ],
    };
    let f = frame_rows.len() - 1;
    let n = frame_rows.len();
    // The real frame's input/context come from its actual parent, not another provider run.
    let frame_pair = |b: &mut Builder,
                      target: usize,
                      extra: &[usize],
                      mut predicates: Vec<ScopePredicate>,
                      target_row: usize| {
        let mut rows = frame_rows.clone();
        rows.extend_from_slice(extra);
        if f == 1 {
            predicates.push(eq(1, "id", 0, "invocation"));
        }
        b.pair(root, target, &rows, predicates, 0, target_row);
    };
    for ty in [
        TypeId::of::<analysis::settings::AnalyticsConfiguration>(),
        TypeId::of::<analysis::AnalysisDefinition>(),
        TypeId::of::<analysis::MethodParameters>(),
        TypeId::of::<embedding::text::TextDefinition>(),
        TypeId::of::<embedding::EmbeddingSpec>(),
        TypeId::of::<embedding::configuration::ServiceConfiguration>(),
    ] {
        if let Some(target) = idx(ty) {
            frame_pair(&mut b, target, &[target], vec![], n);
        }
    }
    for ty in [
        TypeId::of::<attribution::ProviderRun>(),
        TypeId::of::<analysis::local::Invocation>(),
        TypeId::of::<analysis::analytic_embedding::Invocation>(),
        TypeId::of::<projection::ProjectionSourceAssessment>(),
    ] {
        if let Some(target) = idx(ty) {
            frame_pair(
                &mut b,
                target,
                &[target],
                vec![eq(n, "input", f, "input"), eq(n, "context", f, "context")],
                n,
            );
        }
    }
    for ty in [
        TypeId::of::<analysis::local::AnalysisOutcome>(),
        TypeId::of::<analysis::local::AnalysisCoverage>(),
        TypeId::of::<local_semantics::LocalContribution>(),
        TypeId::of::<local_semantics::LocalAssessment>(),
    ] {
        if let (Some(target), Some(local)) =
            (idx(ty), idx(TypeId::of::<analysis::local::Invocation>()))
        {
            frame_pair(
                &mut b,
                target,
                &[local, target],
                vec![
                    eq(n, "input", f, "input"),
                    eq(n, "context", f, "context"),
                    eq(n + 1, "invocation", n, "id"),
                ],
                n + 1,
            );
        }
    }
    let a = idx(TypeId::of::<source::SourceArtifact>());
    let o = idx(TypeId::of::<source::Occurrence>());
    let c = idx(TypeId::of::<normalized::entities::CallableEntity>());
    let refs = idx(TypeId::of::<normalized::entities::EntityRef>());
    if let (Some(artifacts), Some(occurrences), Some(entities)) = (a, o, c) {
        if matches!(kind, Kind::Structural)
            && let Some(refs) = refs
        {
            frame_pair(
                &mut b,
                refs,
                &[artifacts, occurrences, entities, refs],
                vec![
                    eq(n, "input", f, "input"),
                    eq(n + 1, "source", n, "id"),
                    eq(n + 2, "source_declaration", n + 1, "id"),
                    eq(n + 3, "callable_callable", n + 2, "id"),
                ],
                n + 3,
            );
        }
        if let Some(events) = idx(TypeId::of::<normalized::events::NormalizedCallEvent>()) {
            frame_pair(
                &mut b,
                events,
                &[events, occurrences, artifacts],
                vec![
                    eq(n, "context", f, "context"),
                    eq(n + 1, "id", n, "site"),
                    eq(n + 2, "id", n + 1, "source"),
                    eq(n + 2, "input", f, "input"),
                ],
                n,
            );
        }
        if let Some(modules) = idx(TypeId::of::<source::Module>()) {
            b.pair(
                artifacts,
                modules,
                &[artifacts, modules],
                vec![eq(1, "source", 0, "id")],
                0,
                1,
            );
        }
    }
    if let (Some(refs), Some(callables), Some(occurrences), Some(artifacts)) = (refs, c, o, a) {
        for ty in [
            TypeId::of::<normalized::callables::SignatureVariant>(),
            TypeId::of::<normalized::callables::EffectiveCallableAssessment>(),
        ] {
            if let Some(target) = idx(ty) {
                match kind {
                    Kind::Structural => frame_pair(
                        &mut b,
                        target,
                        &[artifacts, occurrences, callables, refs, target],
                        vec![
                            eq(n, "input", f, "input"),
                            eq(n + 1, "source", n, "id"),
                            eq(n + 2, "source_declaration", n + 1, "id"),
                            eq(n + 3, "callable_callable", n + 2, "id"),
                            eq(n + 4, "callable", n + 3, "callable_callable"),
                            eq(n + 4, "context", f, "context"),
                        ],
                        n + 4,
                    ),
                    Kind::Analytic => {
                        let members = idx(TypeId::of::<structural::ScopeMember>())
                            .ok_or(ModelError::Schema("analytic scope members"))?;
                        frame_pair(
                            &mut b,
                            target,
                            &[members, refs, target],
                            vec![
                                eq(n, "frame", 0, "id"),
                                eq(n + 1, "id", n, "entity"),
                                eq(n + 2, "callable", n + 1, "callable_callable"),
                                eq(n + 2, "context", f, "context"),
                            ],
                            n + 2,
                        );
                    }
                }
            }
        }
    }
    if matches!(kind, Kind::Structural) {
        if let (Some(occurrences), Some(artifacts), Some(quals)) =
            (o, a, idx(TypeId::of::<assertion::AssertionQualification>()))
        {
            for (ty, field) in [
                (TypeId::of::<flow::FlowRegionObservation>(), "statement"),
                (TypeId::of::<flow::FlowValueObservation>(), "sink"),
                (TypeId::of::<flow::FlowTestLeafObservation>(), "test"),
            ] {
                if let Some(target) = idx(ty) {
                    frame_pair(
                        &mut b,
                        target,
                        &[target, quals, occurrences, artifacts],
                        vec![
                            eq(n + 1, "id", n, "qualification"),
                            eq(n + 1, "context", f, "context"),
                            eq(n + 2, "id", n, field),
                            eq(n + 3, "id", n + 2, "source"),
                            eq(n + 3, "input", f, "input"),
                        ],
                        n,
                    );
                }
            }
        }
        if let Some(premises) = idx(TypeId::of::<analysis::native::NativeAssertionPremise>()) {
            for field in relations[premises]
                .fields()
                .iter()
                .filter(|field| !field.list())
            {
                if let Some((target, _)) = field.target()
                    && let Some(owner) = idx(target)
                {
                    b.own(premises, field.name(), owner)?;
                }
            }
            own!(
                analysis::native::NativeQualification,
                "premise",
                analysis::native::NativeAssertionPremise
            );
        }
        if let (Some(occurrences), Some(details)) =
            (o, idx(TypeId::of::<syntax::SyntaxDetailObservation>()))
        {
            b.pair(
                occurrences,
                details,
                &[occurrences, occurrences, details],
                vec![
                    eq(1, "source", 0, "source"),
                    eq(1, "start", 0, "start"),
                    eq(1, "end", 0, "end"),
                    eq(1, "syntax_kind", 0, "syntax_kind"),
                    eq(1, "structural_path", 0, "structural_path"),
                    eq(2, "occurrence", 1, "id"),
                ],
                0,
                2,
            );
        }
        if let (Some(links), Some(_)) = (
            idx(TypeId::of::<catalog::CatalogMemberInvocation>()),
            idx(TypeId::of::<catalog::CatalogMember>()),
        ) {
            b.pair(
                root,
                links,
                &[root, links],
                vec![eq(1, "invocation", 0, "id")],
                0,
                1,
            );
        }
    }
    if matches!(kind, Kind::Analytic) {
        for (member, relation) in relations[..real].iter().enumerate() {
            for field in relation.fields().iter().filter(|field| {
                field.target().map(|(target, _)| target)
                    == Some(TypeId::of::<structural::StructuralFrame>())
                    && !field.list()
            }) {
                b.own(member, field.name(), root)?;
            }
        }
        if let (Some(occurrences), Some(placements), Some(quals)) = (
            o,
            idx(TypeId::of::<syntax::SyntaxPlacement>()),
            idx(TypeId::of::<assertion::AssertionQualification>()),
        ) {
            b.pair(
                occurrences,
                placements,
                &[occurrences, placements, occurrences, quals],
                vec![
                    eq(1, "parent", 0, "id"),
                    eq(2, "id", 1, "occurrence"),
                    eq(3, "id", 1, "qualification"),
                    ScopePredicate::Code(
                        col(0, "syntax_kind"),
                        source::SyntaxKind::ParameterWithDefault.code(),
                    ),
                    ScopePredicate::Code(
                        col(2, "syntax_kind"),
                        source::SyntaxKind::Parameter.code(),
                    ),
                ],
                0,
                1,
            );
        }
        if let (Some(scope), Some(ownership), Some(quals)) = (
            idx(TypeId::of::<structural::ScopeMember>()),
            idx(TypeId::of::<normalized::entities::OccurrenceOwnership>()),
            idx(TypeId::of::<assertion::AssertionQualification>()),
        ) {
            for ty in [
                TypeId::of::<protocols::NativeExitObservation>(),
                TypeId::of::<protocols::NativeTerminalObservation>(),
            ] {
                if let Some(target) = idx(ty) {
                    let mut rows = frame_rows.clone();
                    rows.extend([scope, ownership, target, quals]);
                    let mut predicates = vec![
                        eq(0, "id", n, "frame"),
                        eq(n + 1, "entity", n, "entity"),
                        eq(n + 2, "subject", n + 1, "occurrence"),
                        eq(n + 3, "id", n + 2, "qualification"),
                        eq(n + 3, "context", f, "context"),
                    ];
                    if f == 1 {
                        predicates.push(eq(1, "id", 0, "invocation"));
                    }
                    b.pair(scope, ownership, &rows, predicates.clone(), n, n + 1);
                    b.pair(scope, target, &rows, predicates, n, n + 2);
                }
            }
        }
        own!(
            embedding::text::TextWindow,
            "assessment",
            embedding::text::TextAssessment
        );
        own!(
            normalized::links::MentionEntityCandidate,
            "assessment",
            normalized::links::MentionEntityAssessment
        );
        own!(
            normalized::entities::PublicExposureCandidate,
            "exposure",
            normalized::entities::PublicExposure
        );
        if let (Some(target), Some(candidates), Some(exposures), Some(resolutions), Some(scope)) = (
            idx(TypeId::of::<normalized::links::MentionEntityAssessment>()),
            idx(TypeId::of::<normalized::links::MentionEntityCandidate>()),
            idx(TypeId::of::<normalized::entities::PublicExposureCandidate>()),
            idx(TypeId::of::<normalized::entities::SymbolEntityResolution>()),
            idx(TypeId::of::<structural::ScopeMember>()),
        ) {
            frame_pair(
                &mut b,
                target,
                &[scope, resolutions, exposures, candidates, target],
                vec![
                    eq(n, "frame", 0, "id"),
                    eq(n + 1, "entity", n, "entity"),
                    eq(n + 1, "context", f, "context"),
                    eq(n + 2, "resolution", n + 1, "id"),
                    eq(n + 3, "exposure", n + 2, "exposure"),
                    eq(n + 4, "id", n + 3, "assessment"),
                ],
                n + 4,
            );
        }
        own!(
            embedding::analytic::AnalysisEmbeddingUse,
            "window",
            embedding::text::TextWindow
        );
        own!(
            analysis::analytic_embedding::AnalysisOutcome,
            "invocation",
            analysis::analytic_embedding::Invocation
        );
        if let Some(target) = idx(TypeId::of::<embedding::text::TextAssessment>()) {
            frame_pair(
                &mut b,
                target,
                &[target],
                vec![eq(n, "input", f, "input"), eq(n, "context", f, "context")],
                n,
            );
        }
        if let Some(target) = idx(TypeId::of::<input::CorpusLibrary>()) {
            frame_pair(
                &mut b,
                target,
                &[target],
                vec![eq(n, "library", f, "input")],
                n,
            );
        }
    }
    Ok(b.program)
}

/// Local dependency graph keeps actual source roots separate from reached artifact references.
pub fn local(
    inputs: Vec<ValidationInput>,
    relations: &[Relation],
    real: usize,
    artifact: usize,
) -> Result<ScopeProgram, ModelError> {
    let source = real;
    let idx = |kind: TypeId| {
        relations[..real]
            .iter()
            .position(|relation| relation.type_id() == kind)
    };
    let mut b = Builder::new(inputs, real);
    for (from, relation) in relations[..real].iter().enumerate() {
        for field in relation.fields().iter().filter(|field| !field.list()) {
            if let Some((kind, _)) = field.target()
                && let Some(to) = idx(kind)
            {
                b.follow(from, field.name(), to)?;
            }
        }
    }
    b.pair(source, artifact, &[source], vec![], 0, 0);
    let occurrences = idx(TypeId::of::<source::Occurrence>())
        .ok_or(ModelError::Schema(source::Occurrence::NAME))?;
    macro_rules! own {
        ($member:ty,$field:literal,$owner:ty) => {{
            if let (Some(member), Some(owner)) =
                (idx(TypeId::of::<$member>()), idx(TypeId::of::<$owner>()))
            {
                b.own(member, $field, owner)?;
            }
        }};
    }
    // Exact native and normalized candidate bags required by the Entry/Theory/Field kernels.
    own!(
        normalized::entities::OccurrenceOwnership,
        "occurrence",
        source::Occurrence
    );
    own!(flow::FlowUse, "occurrence", source::Occurrence);
    own!(flow::FlowUseObservation, "use_", flow::FlowUse);
    own!(flow::FlowReachingObservation, "use_", flow::FlowUse);
    own!(
        flow_inventory::FlowUseInventoryObservation,
        "use_",
        flow::FlowUse
    );
    own!(
        flow_inventory::FlowUseCandidate,
        "inventory",
        flow_inventory::FlowUseInventoryObservation
    );
    own!(
        flow_inventory::FlowUseInventoryMember,
        "inventory",
        flow_inventory::FlowUseInventoryObservation
    );
    own!(
        flow::FlowDefinitionObservation,
        "definition",
        flow::FlowDefinition
    );
    own!(flow::FlowValueObservation, "sink", source::Occurrence);
    own!(flow::FlowTestLeafObservation, "test", source::Occurrence);
    own!(
        flow::FlowAttributeLoadObservation,
        "occurrence",
        source::Occurrence
    );
    own!(flow::FlowRegionObservation, "scope", lexical::LexicalScope);
    own!(syntax::SyntaxPlacement, "occurrence", source::Occurrence);
    own!(
        normalized::entities::ParameterEntityLink,
        "entity",
        normalized::entities::ParameterEntity
    );
    own!(
        normalized::entities::ParameterEntityLink,
        "parameter",
        calls::SignatureParameter
    );
    own!(
        declarations::SymbolDeclaration,
        "symbol",
        calls::ProviderSymbol
    );
    own!(
        declarations::ParameterDeclaration,
        "parameter",
        calls::SignatureParameter
    );
    own!(
        declarations::ParameterDeclaration,
        "declaration",
        source::Occurrence
    );
    own!(
        syntax::ParameterSyntaxObservation,
        "parameter",
        source::Occurrence
    );
    for (member, table) in relations[..real].iter().enumerate().filter(|(_, table)| {
        table.type_id() == TypeId::of::<normalized::entities::ParameterEntity>()
    }) {
        for field in table.fields().iter().filter(|field| {
            !field.list()
                && field.target().map(|(kind, _)| kind) == Some(TypeId::of::<source::Occurrence>())
        }) {
            b.own(member, field.name(), occurrences)?;
        }
    }

    if let Some(placements) = idx(TypeId::of::<syntax::SyntaxPlacement>()) {
        for code in [
            source::SyntaxKind::ParameterWithDefault.code(),
            source::SyntaxKind::ExprAttribute.code(),
            source::SyntaxKind::ExprCompare.code(),
            source::SyntaxKind::ExprCall.code(),
        ] {
            b.pair(
                occurrences,
                placements,
                &[occurrences, placements],
                vec![
                    eq(1, "parent", 0, "id"),
                    ScopePredicate::Code(col(0, "syntax_kind"), code),
                ],
                0,
                1,
            );
        }
    }
    own!(
        symbols::ClassAncestryObservation,
        "class",
        calls::ProviderSymbol
    );
    own!(
        symbols::SymbolSequenceMember,
        "sequence",
        symbols::SymbolSequence
    );
    own!(types::TypeSequenceMember, "sequence", types::TypeSequence);
    own!(value::LiteralSetMember, "set", value::LiteralSet);
    own!(types::TypeObservation, "subject", source::Occurrence);
    own!(types::TypeQueryObservation, "subject", source::Occurrence);
    own!(
        normalized::entities::SymbolEntityResolution,
        "symbol",
        calls::ProviderSymbol
    );
    own!(
        normalized::entities::FieldEntity,
        "class",
        normalized::entities::ClassEntity
    );
    own!(
        normalized::entities::FieldDeclarationLink,
        "field",
        normalized::entities::FieldEntity
    );
    own!(
        normalized::symbolic_fields::SourceFieldStore,
        "target",
        source::Occurrence
    );
    own!(lexical::LexicalResolution, "read", source::Occurrence);
    own!(calls::CallSyntax, "site", source::Occurrence);
    own!(calls::CallArgument, "call", calls::CallSyntax);
    own!(
        normalized::links::TestOperandTypeAssessment,
        "leaf",
        flow::FlowTestLeafObservation
    );
    own!(
        normalized::links::TestOperandTypeLink,
        "assessment",
        normalized::links::TestOperandTypeAssessment
    );
    own!(
        normalized::links::TestOperandCoverage,
        "assessment",
        normalized::links::TestOperandTypeAssessment
    );
    for (member, table) in relations[..real].iter().enumerate() {
        if let Some(field) = table
            .fields()
            .iter()
            .find(|field| field.name() == "assertion")
            && let Some((kind, _)) = field.target()
            && let Some(owner) = idx(kind)
        {
            b.own(member, field.name(), owner)?;
        }
    }
    if let Some(native) = idx(TypeId::of::<analysis::native::NativeAssertionPremise>()) {
        for field in relations[native]
            .fields()
            .iter()
            .filter(|field| !field.list())
        {
            if let Some((kind, _)) = field.target()
                && let Some(owner) = idx(kind)
            {
                b.own(native, field.name(), owner)?;
            }
        }
        if let Some(qualifications) = idx(TypeId::of::<analysis::native::NativeQualification>()) {
            b.own(qualifications, "premise", native)?;
        }
    }

    if let (Some(qualifications), Some(coverage)) = (
        idx(TypeId::of::<assertion::AssertionQualification>()),
        idx(TypeId::of::<attribution::ProviderCoverage>()),
    ) {
        for family in [
            attribution::FactFamily::Flow,
            attribution::FactFamily::Signatures,
            attribution::FactFamily::Syntax,
        ] {
            b.pair(
                qualifications,
                coverage,
                &[qualifications, coverage],
                vec![
                    eq(1, "scope", 0, "scope"),
                    eq(1, "context", 0, "context"),
                    ScopePredicate::Code(col(1, "family"), family.code()),
                ],
                0,
                1,
            );
        }
    }
    if let (Some(coverage), Some(scopes), Some(modules)) = (
        idx(TypeId::of::<attribution::ProviderCoverage>()),
        idx(TypeId::of::<source::CoverageScope>()),
        idx(TypeId::of::<source::Module>()),
    ) {
        for family in [
            attribution::FactFamily::Flow,
            attribution::FactFamily::Signatures,
            attribution::FactFamily::Syntax,
            attribution::FactFamily::Types,
        ] {
            for field in ["input_input", "artifact_artifact"] {
                b.pair(
                    source,
                    coverage,
                    &[source, scopes, coverage],
                    vec![
                        eq(
                            1,
                            field,
                            0,
                            if field == "input_input" {
                                "input"
                            } else {
                                "id"
                            },
                        ),
                        eq(2, "scope", 1, "id"),
                        ScopePredicate::Code(col(2, "family"), family.code()),
                    ],
                    0,
                    2,
                );
            }
            b.pair(
                source,
                coverage,
                &[source, modules, scopes, coverage],
                vec![
                    eq(1, "source", 0, "id"),
                    eq(2, "module_module", 1, "id"),
                    eq(3, "scope", 2, "id"),
                    ScopePredicate::Code(col(3, "family"), family.code()),
                ],
                0,
                3,
            );
        }
    }
    b.pair(
        occurrences,
        occurrences,
        &[occurrences, occurrences],
        vec![
            eq(1, "source", 0, "source"),
            eq(1, "start", 0, "start"),
            eq(1, "end", 0, "end"),
            eq(1, "syntax_kind", 0, "syntax_kind"),
            eq(1, "structural_path", 0, "structural_path"),
            ScopePredicate::Code(col(1, "role"), source::OccurrenceRole::Read.code()),
        ],
        0,
        1,
    );
    Ok(b.program)
}

/// Qualified Local primary roots. Inventory selects actual artifacts by input; selected-source
/// demand keeps dangling artifact references as explicit primary obligations, just like the
/// consumer's nominal query joins. Parameters are context then input/source, never program ID.
pub fn local_roots(
    inputs: Vec<ValidationInput>,
    real: usize,
    selected_source: bool,
) -> Result<ScopeProgram, ModelError> {
    let source = real;
    let idx = |kind: TypeId| {
        inputs[..real]
            .iter()
            .position(|input| input.type_id() == kind)
            .ok_or(ModelError::Schema("Local primary scope input absent"))
    };
    let occurrence = idx(TypeId::of::<source::Occurrence>())?;
    let qualification = idx(TypeId::of::<assertion::AssertionQualification>())?;
    let uses = idx(TypeId::of::<flow::FlowUse>())?;
    let families = [
        (TypeId::of::<flow::FlowValueObservation>(), "sink", false),
        (TypeId::of::<flow::FlowTestLeafObservation>(), "test", false),
        (
            TypeId::of::<flow::FlowAttributeLoadObservation>(),
            "occurrence",
            false,
        ),
        (TypeId::of::<types::TypeObservation>(), "subject", false),
        (
            TypeId::of::<normalized::symbolic_fields::SourceFieldStore>(),
            "target",
            false,
        ),
        (TypeId::of::<flow::FlowUseObservation>(), "use_", true),
    ];
    let mut b = Builder::new(inputs.clone(), real);
    for (kind, field, through_use) in families {
        let target = idx(kind)?;
        let mut rows = if selected_source {
            vec![]
        } else {
            vec![source]
        };
        let observation = rows.len();
        rows.push(target);
        let q = rows.len();
        rows.push(qualification);
        let u = rows.len();
        if through_use {
            rows.push(uses);
        }
        let o = rows.len();
        rows.push(occurrence);
        let mut predicates = vec![
            eq(q, "id", observation, "qualification"),
            ScopePredicate::Parameter(col(q, "context"), 0),
        ];
        if through_use {
            predicates.extend([
                eq(u, "id", observation, field),
                eq(o, "id", u, "occurrence"),
            ]);
        } else {
            predicates.push(eq(o, "id", observation, field));
        }
        let source_key = if selected_source {
            predicates.push(ScopePredicate::Parameter(col(o, "source"), 1));
            col(o, "source")
        } else {
            predicates.extend([
                eq(0, "id", o, "source"),
                ScopePredicate::Parameter(col(0, "input"), 1),
            ]);
            col(0, "id")
        };
        b.program.rules.push(ScopeRule::Pairs {
            source,
            target,
            rows,
            predicates,
            source_key,
            target_key: col(observation, "id"),
        });
    }
    Ok(b.program)
}

/// Model publication owners extend the complete callable program. Parameters are captured
/// catalog then each target's module and final callable name, outside immutable program identity.
pub struct ModelPorts<'a> {
    pub real: usize,
    pub frame: usize,
    pub contexts: usize,
    pub terminals: usize,
    pub exits: usize,
    pub targets: &'a [usize],
    pub event: usize,
}
pub fn model(
    inputs: Vec<ValidationInput>,
    relations: &[Relation],
    ports: ModelPorts<'_>,
) -> Result<ScopeProgram, ModelError> {
    let ModelPorts {
        real,
        frame,
        contexts,
        terminals,
        exits,
        targets,
        event,
    } = ports;
    let idx = |kind: TypeId| {
        relations[..real]
            .iter()
            .position(|relation| relation.type_id() == kind)
    };
    let required = |kind: TypeId| idx(kind).ok_or(ModelError::Schema("Model scope input absent"));
    let mut b = Builder::new(inputs, real);
    macro_rules! own {
        ($member:ty,$field:literal,$owner:ty) => {{
            if let (Some(member), Some(owner)) =
                (idx(TypeId::of::<$member>()), idx(TypeId::of::<$owner>()))
            {
                b.own(member, $field, owner)?;
            }
        }};
    }
    own!(types::TypeObservation, "subject", source::Occurrence);
    own!(types::TypeSequenceMember, "sequence", types::TypeSequence);
    own!(
        class_metadata::ClassMemberObservation,
        "class",
        calls::ProviderSymbol
    );
    own!(
        class_metadata::ClassMetadataObservation,
        "class",
        calls::ProviderSymbol
    );
    own!(
        symbols::FunctionTraitObservation,
        "defining_class",
        calls::ProviderSymbol
    );
    own!(calls::CallOriginStep, "origin", calls::CallOrigin);
    own!(input::ArtifactOwnership, "artifact", source::SourceArtifact);
    own!(input::ArtifactUse, "artifact", source::SourceArtifact);
    own!(input::InputAcquisition, "input", input::InputRevision);
    own!(
        input::EnvironmentFingerprint,
        "acquisition",
        input::InputAcquisition
    );
    own!(
        execution::context_execution::ContextItem,
        "execution",
        execution::context_execution::ContextExecution
    );
    own!(
        execution::context_execution::ContextMember,
        "execution",
        execution::context_execution::ContextExecution
    );
    own!(
        execution::context_binding::BindingMember,
        "binding",
        execution::context_binding::ContextEntryBinding
    );
    own!(
        execution::modeled_call::ModeledCallEvaluation,
        "attempt",
        normalized::bindings::CallBindingAttempt
    );
    own!(
        execution::modeled_call::ModeledCallArgument,
        "call",
        execution::modeled_call::ModeledCallEvaluation
    );
    own!(
        execution::modeled_call::ModeledCallNative,
        "call",
        execution::modeled_call::ModeledCallEvaluation
    );
    own!(
        execution::records::EvaluationMember,
        "evaluation",
        execution::records::ExpressionEvaluation
    );
    own!(
        execution::records::EvaluationOperand,
        "evaluation",
        execution::records::ExpressionEvaluation
    );
    own!(
        conditions::entry::EntryValueWitness,
        "access_source",
        conditions::entry::EntryAccessSource
    );
    // These source-memberships are model-owned finite families, not arbitrary incoming references.
    if let (Some(native), Some(premises)) = (
        idx(TypeId::of::<analysis::native::NativeQualification>()),
        idx(TypeId::of::<analysis::native::NativeAssertionPremise>()),
    ) {
        b.own(native, "premise", premises)?;
    }
    if let Some(premises) = idx(TypeId::of::<analysis::native::NativeAssertionPremise>()) {
        for field in relations[premises].fields().iter().filter(|f| !f.list()) {
            if let Some((kind, _)) = field.target()
                && let Some(assertion) = idx(kind)
            {
                b.own(premises, field.name(), assertion)?;
            }
        }
    }

    let symbols = required(TypeId::of::<calls::ProviderSymbol>())?;
    let qualifications = required(TypeId::of::<assertion::AssertionQualification>())?;
    let occurrences = required(TypeId::of::<source::Occurrence>())?;
    let artifacts = required(TypeId::of::<source::SourceArtifact>())?;
    if let Some(observations) = idx(TypeId::of::<symbols::SymbolObservation>()) {
        b.pair(
            symbols,
            observations,
            &[symbols, observations, qualifications],
            vec![
                eq(1, "symbol", 0, "id"),
                eq(2, "id", 1, "qualification"),
                eq(2, "context", 0, "context"),
            ],
            0,
            1,
        );
    }
    for (member, relation) in relations[..real].iter().enumerate() {
        if let Some(field) = relation
            .fields()
            .iter()
            .find(|field| field.name() == "assertion")
            && let Some((kind, _)) = field.target()
            && let Some(owner) = idx(kind)
        {
            b.own(member, field.name(), owner)?;
        }
    }
    if let (Some(items), Some(bindings)) = (
        idx(TypeId::of::<execution::context_execution::ContextItem>()),
        idx(TypeId::of::<execution::context_binding::ContextEntryBinding>()),
    ) {
        b.pair(
            items,
            bindings,
            &[items, bindings],
            vec![eq(1, "item", 0, "item"), eq(1, "site", 0, "site")],
            0,
            1,
        );
    }
    if let (Some(bindings), Some(evaluations), Some(base)) = (
        idx(TypeId::of::<execution::context_binding::ContextEntryBinding>()),
        idx(TypeId::of::<execution::records::ExpressionEvaluation>()),
        idx(TypeId::of::<analysis::base_evaluation::AnalysisInvocation>()),
    ) {
        b.pair(
            bindings,
            evaluations,
            &[bindings, qualifications, evaluations, base],
            vec![
                eq(1, "id", 0, "qualification"),
                eq(2, "expression", 0, "entry_actual"),
                eq(2, "owner", 0, "owner"),
                eq(3, "id", 2, "invocation"),
                eq(3, "context", 1, "context"),
            ],
            0,
            2,
        );
    }
    for kind in [
        TypeId::of::<analysis::MethodParameters>(),
        TypeId::of::<analysis::AnalysisDefinition>(),
    ] {
        if let Some(target) = idx(kind) {
            b.pair(frame, target, &[frame, target], vec![], 0, 1);
        }
    }
    if let Some(target) = idx(TypeId::of::<models::ModelCatalog>()) {
        b.pair(
            frame,
            target,
            &[frame, target],
            vec![ScopePredicate::Parameter(col(1, "id"), 0)],
            0,
            1,
        );
    }
    for kind in [
        TypeId::of::<attribution::ProviderRun>(),
        TypeId::of::<analysis::enriched_execution::AnalysisInvocation>(),
        TypeId::of::<analysis::source_call::AnalysisInvocation>(),
        TypeId::of::<analysis::local::AnalysisInvocation>(),
    ] {
        if let Some(target) = idx(kind) {
            b.pair(
                frame,
                target,
                &[frame, target],
                vec![eq(1, "input", 0, "input"), eq(1, "context", 0, "context")],
                0,
                1,
            );
        }
    }
    for (root, kind) in [
        (
            contexts,
            TypeId::of::<execution::context_execution::ContextExecution>(),
        ),
        (
            terminals,
            TypeId::of::<protocols::NativeTerminalObservation>(),
        ),
        (exits, TypeId::of::<protocols::NativeExitObservation>()),
    ] {
        let source = required(kind)?;
        b.pair(root, source, &[root], vec![], 0, 0);
        if root == contexts {
            let enriched =
                required(TypeId::of::<analysis::enriched_execution::AnalysisInvocation>())?;
            b.pair(
                root,
                frame,
                &[root, enriched, frame],
                vec![
                    eq(1, "id", 0, "invocation"),
                    eq(2, "input", 1, "input"),
                    eq(2, "context", 1, "context"),
                ],
                0,
                2,
            );
        } else {
            b.pair(
                root,
                frame,
                &[root, qualifications, occurrences, artifacts, frame],
                vec![
                    eq(1, "id", 0, "qualification"),
                    eq(2, "id", 0, "subject"),
                    eq(3, "id", 2, "source"),
                    eq(4, "context", 1, "context"),
                    eq(4, "input", 3, "input"),
                ],
                0,
                4,
            );
        }
    }
    let events = required(TypeId::of::<normalized::events::NormalizedCallEvent>())?;
    b.pair(
        event,
        frame,
        &[events, occurrences, artifacts, frame],
        vec![
            eq(1, "id", 0, "site"),
            eq(2, "id", 1, "source"),
            eq(3, "input", 2, "input"),
            eq(3, "context", 0, "context"),
        ],
        0,
        3,
    );
    if let Some(term) = idx(TypeId::of::<protocols::NativeTerminalObservation>()) {
        // The native explicit-origin key is an execution parameter, following target strings.
        b.pair(
            event,
            term,
            &[events, term, qualifications],
            vec![
                eq(1, "subject", 0, "site"),
                eq(2, "id", 1, "qualification"),
                eq(2, "context", 0, "context"),
                ScopePredicate::Parameter(col(0, "origin"), 1 + targets.len() * 2),
            ],
            0,
            1,
        );
    }
    for (number, root) in targets.iter().enumerate() {
        b.pair(*root, frame, &[*root], vec![], 0, 0);
        if let (Some(modules), Some(acquired)) = (
            idx(TypeId::of::<calls::ProviderModule>()),
            idx(TypeId::of::<source::Module>()),
        ) {
            let base = vec![
                eq(1, "context", 0, "context"),
                eq(2, "id", 1, "module"),
                ScopePredicate::Parameter(col(1, "name"), 2 + number * 2),
            ];
            // Union preserves the nullable acquired-module OR bundled-module domain exactly.
            let mut bundled = base.clone();
            bundled.push(ScopePredicate::Parameter(
                col(2, "bundled_name"),
                1 + number * 2,
            ));
            b.pair(*root, symbols, &[*root, symbols, modules], bundled, 0, 1);
            let mut actual = base;
            actual.extend([
                eq(3, "id", 2, "acquired_module"),
                ScopePredicate::Parameter(col(3, "qualified_name"), 1 + number * 2),
            ]);
            b.pair(
                *root,
                symbols,
                &[*root, symbols, modules, acquired],
                actual,
                0,
                1,
            );
        }
        if let Some(uses) = idx(TypeId::of::<input::ArtifactUse>()) {
            // MIN is over one eligible union; splitting role or suffix branches changes cardinality.
            const ROLES: &[i16] = &[
                input::SourceRole::Release as i16,
                input::SourceRole::Example as i16,
                input::SourceRole::Test as i16,
                input::SourceRole::DocBlock as i16,
            ];
            b.program.rules.push(ScopeRule::FirstPairs {
                source: *root,
                target: artifacts,
                rows: vec![*root, artifacts, uses],
                predicates: vec![
                    eq(1, "input", 0, "input"),
                    eq(2, "artifact", 1, "id"),
                    eq(2, "input", 1, "input"),
                    ScopePredicate::TextSuffixIn(col(1, "path"), &[".py", ".pyi"]),
                    ScopePredicate::CodeIn(col(2, "role"), ROLES),
                ],
                source_key: col(0, "id"),
                target_key: col(1, "id"),
            });
        }
    }
    Ok(b.program)
}
/// Captured selected catalog rows; references to other language epochs stay dependencies.
pub fn model_catalog(inputs: Vec<ValidationInput>) -> Result<ScopeProgram, ModelError> {
    let target = inputs
        .iter()
        .position(|input| input.type_id() == TypeId::of::<models::ModelCatalog>())
        .ok_or(ModelError::Schema(models::ModelCatalog::NAME))?;
    let mut b = Builder::new(inputs.clone(), inputs.len());
    b.pair(
        target,
        target,
        &[target],
        vec![ScopePredicate::Parameter(col(0, "id"), 0)],
        0,
        0,
    );
    Ok(b.program)
}

#[cfg(test)]
mod controls {
    use super::*;
    fn relations(inputs: &[ValidationInput], owner: &ValidatedModel) -> Vec<Relation> {
        inputs
            .iter()
            .map(|input| owner.relation(input.name()).unwrap().clone())
            .collect()
    }
    #[test]
    fn construction_reservation_covers_dynamic_model_targets_and_releases_on_refusal() {
        let owner = super::super::model().unwrap();
        let original = execution::model_production::ModelData::inputs();
        let records = relations(&original, &owner);
        let budget = resources::ResourceBudget::fixed(64 << 20).unwrap();
        let fields = records.iter().map(|r| r.fields().len()).sum();
        let construction = reserve_construction(original.len() + 37, fields, 32, &budget).unwrap();
        let real = original.len();
        let mut inputs = original.clone();
        let mut target_ports = Vec::new();
        for kind in [
            TypeId::of::<attribution::ProviderRun>(),
            TypeId::of::<execution::context_execution::ContextExecution>(),
            TypeId::of::<protocols::NativeTerminalObservation>(),
            TypeId::of::<protocols::NativeExitObservation>(),
        ] {
            let i = original
                .iter()
                .position(|input| input.type_id() == kind)
                .unwrap();
            inputs.push(original[i].clone());
        }
        let frame_source = original
            .iter()
            .position(|input| input.type_id() == TypeId::of::<attribution::ProviderRun>())
            .unwrap();
        for _ in 0..32 {
            target_ports.push(inputs.len());
            inputs.push(original[frame_source].clone());
        }
        let event = inputs.len();
        let event_source = original
            .iter()
            .position(|input| {
                input.type_id() == TypeId::of::<normalized::events::NormalizedCallEvent>()
            })
            .unwrap();
        inputs.push(original[event_source].clone());
        let program = model(
            inputs.clone(),
            &relations(&inputs, &owner),
            ModelPorts {
                real,
                frame: real,
                contexts: real + 1,
                terminals: real + 2,
                exits: real + 3,
                targets: &target_ports,
                event,
            },
        )
        .unwrap();
        program.validate(&owner).unwrap();
        assert!(program.allowance() <= construction.size());
        let bounded = resources::ResourceBudget::fixed(construction.size() - 1).unwrap();
        assert!(reserve_construction(original.len() + 37, fields, 32, &bounded).is_err());
        assert_eq!(bounded.reserved(), 0);
        drop(program);
        drop(construction);
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn local_program_retains_nominal_virtual_source_and_distinct_root_domains() {
        let owner = super::super::model().unwrap();
        let mut inputs = local_semantics::LocalData::validation_inputs();
        inputs.push(ValidationInput::of::<
            analysis::native::NativeAssertionPremise,
        >(&["id"]));
        inputs.sort_by_key(|input| (input.name(), input.prefix()));
        inputs.dedup_by_key(|input| (input.name(), input.prefix()));
        let real = inputs.len();
        let artifact = inputs
            .iter()
            .position(|input| input.type_id() == TypeId::of::<source::SourceArtifact>())
            .unwrap();
        inputs.push(inputs[artifact].clone());
        local(inputs.clone(), &relations(&inputs, &owner), real, artifact)
            .unwrap()
            .validate(&owner)
            .unwrap();
        let inventory = local_roots(inputs.clone(), real, false).unwrap();
        let primary = local_roots(inputs, real, true).unwrap();
        inventory.validate(&owner).unwrap();
        primary.validate(&owner).unwrap();
        assert!(primary.ports[real].virtual_owner);
        assert_eq!(inventory.rules.len(), 6);
        assert_eq!(primary.rules.len(), 6);
        for rule in &primary.rules {
            let ScopeRule::Pairs {
                rows, source_key, ..
            } = rule
            else {
                panic!("primary pair")
            };
            assert!(!rows.contains(&real));
            assert_eq!(source_key.field, "source");
        }
        for rule in &inventory.rules {
            let ScopeRule::Pairs {
                rows, source_key, ..
            } = rule
            else {
                panic!("inventory pair")
            };
            assert_eq!(rows[0], real);
            assert_eq!(*source_key, col(0, "id"));
        }
    }
    #[test]
    fn model_program_has_one_minimum_eligible_pool_and_explicit_virtual_ports() {
        let owner = super::super::model().unwrap();
        let mut inputs = execution::model_production::ModelData::inputs();
        let real = inputs.len();
        let kinds = [
            TypeId::of::<attribution::ProviderRun>(),
            TypeId::of::<execution::context_execution::ContextExecution>(),
            TypeId::of::<protocols::NativeTerminalObservation>(),
            TypeId::of::<protocols::NativeExitObservation>(),
            TypeId::of::<attribution::ProviderRun>(),
            TypeId::of::<normalized::events::NormalizedCallEvent>(),
        ];
        for kind in kinds {
            let original = inputs[..real]
                .iter()
                .position(|input| input.type_id() == kind)
                .unwrap();
            inputs.push(inputs[original].clone());
        }
        let program = model(
            inputs.clone(),
            &relations(&inputs, &owner),
            ModelPorts {
                real,
                frame: real,
                contexts: real + 1,
                terminals: real + 2,
                exits: real + 3,
                targets: &[real + 4],
                event: real + 5,
            },
        )
        .unwrap();
        program.validate(&owner).unwrap();
        let minima = program
            .rules
            .iter()
            .filter_map(|rule| {
                if let ScopeRule::FirstPairs { predicates, .. } = rule {
                    Some(predicates)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(minima.len(), 1);
        assert!(minima[0].iter().any(
            |predicate| matches!(predicate,ScopePredicate::CodeIn(_,codes) if codes.len()==4)
        ));
        assert!(minima[0].iter().any(|predicate|matches!(predicate,ScopePredicate::TextSuffixIn(_,suffixes) if *suffixes==[".py",".pyi"])));
        assert!(program.ports[real..].iter().all(|port| port.virtual_owner));
        assert!(program.rules.iter().all(|rule|!matches!(rule,ScopeRule::Reference{target,direction:ScopeDirection::Forward,..} if *target>=real)));
        model_catalog(inputs[..real].to_vec())
            .unwrap()
            .validate(&owner)
            .unwrap();
    }
}
/// Actual Model operation inventories. Terminal exclusion is an independent positive event
/// relation; physical adapters subtract it after lowering without changing negative coverage.
#[derive(Clone, Copy)]
pub enum ModelRootInventory {
    Context,
    Event,
    Terminal,
    TerminalEvent,
    Exit,
}
impl ModelRootInventory {
    pub fn exclusion(self) -> Option<Self> {
        if matches!(self, Self::Terminal) {
            Some(Self::TerminalEvent)
        } else {
            None
        }
    }
}
pub fn model_roots(
    inputs: Vec<ValidationInput>,
    records: &[Relation],
    kind: ModelRootInventory,
    budget: &resources::ResourceBudget,
) -> Result<super::catalog_scope_program::BudgetedCatalogProgram, ModelError> {
    // This inventory always emits one pair (at most five rows/eight predicates), independent of
    // schema field count and catalog targets. Reserve descriptor copies before cloning; Builder
    // retains the complete resulting descriptor/port/rule arena. No schema fields are copied here.
    let descriptor_bytes = inputs.iter().fold(65_536usize, |bytes, input| {
        bytes
            .saturating_add(512)
            .saturating_add(input.name().len())
            .saturating_add(
                input
                    .order()
                    .iter()
                    .map(|field| field.len().saturating_add(128))
                    .sum::<usize>(),
            )
    });
    let _construction = budget.reserve("model-root-inventory-construction", descriptor_bytes)?;
    let idx = |kind: TypeId| {
        inputs
            .iter()
            .position(|input| input.type_id() == kind)
            .ok_or(ModelError::Schema(
                "Model root inventory nominal input absent",
            ))
    };
    let mut b =
        super::catalog_scope_program::Builder::new(inputs.clone(), inputs.len(), records, budget)?;
    let (root, rows, predicates) = match kind {
        ModelRootInventory::Context => {
            let root = idx(TypeId::of::<execution::context_execution::ContextExecution>())?;
            (
                root,
                vec![root],
                vec![ScopePredicate::Parameter(col(0, "invocation"), 0)],
            )
        }
        ModelRootInventory::Event => {
            let root = idx(TypeId::of::<normalized::events::NormalizedCallEvent>())?;
            (
                root,
                vec![
                    root,
                    idx(TypeId::of::<source::Occurrence>())?,
                    idx(TypeId::of::<source::SourceArtifact>())?,
                ],
                vec![
                    eq(1, "id", 0, "site"),
                    eq(2, "id", 1, "source"),
                    ScopePredicate::Parameter(col(2, "input"), 1),
                    ScopePredicate::Parameter(col(0, "context"), 2),
                ],
            )
        }
        ModelRootInventory::Terminal
        | ModelRootInventory::TerminalEvent
        | ModelRootInventory::Exit => {
            let root = idx(if matches!(kind, ModelRootInventory::Exit) {
                TypeId::of::<protocols::NativeExitObservation>()
            } else {
                TypeId::of::<protocols::NativeTerminalObservation>()
            })?;
            let q = inputs
                .iter()
                .position(|input| {
                    input.type_id() == TypeId::of::<assertion::AssertionQualification>()
                        && input.prefix() == Some(stages::PublicationBoundary::Facts)
                })
                .ok_or(ModelError::Schema("Model root Facts qualification absent"))?;
            let mut rows = vec![
                root,
                q,
                idx(TypeId::of::<source::Occurrence>())?,
                idx(TypeId::of::<source::SourceArtifact>())?,
            ];
            let mut predicates = vec![
                eq(1, "id", 0, "qualification"),
                eq(2, "id", 0, "subject"),
                eq(3, "id", 2, "source"),
                ScopePredicate::Parameter(col(3, "input"), 1),
                ScopePredicate::Parameter(col(1, "context"), 2),
            ];
            if matches!(kind, ModelRootInventory::TerminalEvent) {
                rows.push(idx(TypeId::of::<normalized::events::NormalizedCallEvent>())?);
                predicates.extend([
                    eq(4, "site", 0, "subject"),
                    eq(4, "context", 1, "context"),
                    ScopePredicate::Parameter(col(4, "origin"), 3),
                ]);
            }
            (root, rows, predicates)
        }
    };
    b.pair(root, root, &rows, predicates, col(0, "id"), col(0, "id"));
    b.finish()
}
#[cfg(test)]
mod model_inventory_controls {
    use super::*;
    #[test]
    fn model_operation_inventories_validate_and_terminal_exclusion_is_independent() {
        let owner = super::super::model().unwrap();
        let budget = resources::ResourceBudget::fixed(8 << 20).unwrap();
        let inputs = execution::model_production::ModelData::inputs();
        let records = inputs
            .iter()
            .map(|input| owner.relation(input.name()).unwrap().clone())
            .collect::<Vec<_>>();
        for kind in [
            ModelRootInventory::Context,
            ModelRootInventory::Event,
            ModelRootInventory::Terminal,
            ModelRootInventory::TerminalEvent,
            ModelRootInventory::Exit,
        ] {
            let p = model_roots(inputs.clone(), &records, kind, &budget).unwrap();
            p.program().validate(&owner).unwrap();
            if let ScopeRule::Pairs {
                rows, predicates, ..
            } = &p.program().rules[0]
            {
                if matches!(kind, ModelRootInventory::TerminalEvent) {
                    assert_eq!(rows.len(), 5);
                    assert!(predicates.iter().any(|p|matches!(p,ScopePredicate::Parameter(column,3) if column.field=="origin")));
                } else if matches!(kind, ModelRootInventory::Terminal) {
                    assert_eq!(rows.len(), 4);
                    assert!(
                        !predicates
                            .iter()
                            .any(|p| matches!(p, ScopePredicate::Parameter(_, 3)))
                    );
                }
            }
            drop(p);
            assert_eq!(budget.reserved(), 0);
        }
    }
}
/// Context-bound Local demand shares its immutable physical graph across selected source roots.
/// Source identity is the pair source_key and is supplied by traversal, outside program shape.
pub fn local_context_roots(
    inputs: Vec<ValidationInput>,
    real: usize,
    budget: &resources::ResourceBudget,
) -> Result<super::catalog_scope_program::BudgetedCatalogProgram, ModelError> {
    let mut b = super::catalog_scope_program::Builder::new(vec![], 0, &[], budget)?;
    b.reserve_rules(inputs.len() * 1024)?;
    b.program = local_roots(inputs, real, true)?;
    for rule in &mut b.program.rules {
        if let ScopeRule::Pairs { predicates, .. } = rule {
            predicates.retain(|predicate| !matches!(predicate, ScopePredicate::Parameter(_, 1)));
        }
    }
    b.finish()
}
