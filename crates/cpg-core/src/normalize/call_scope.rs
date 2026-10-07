//! Complete event candidate and binding-set grains over immutable typed premises.
use crate::{
    consumed_rows::{ClosureTable, NominalClosure, PreparedClosure, PreparedEdges, identifier},
    workspace::CompletedInputs,
};
use lctx_model::domain::{
    normalized::{binding_normalization::BindingData, event_normalization::EventData},
    *,
};
use std::{any::TypeId, sync::Arc};

pub(crate) struct CallScopes {
    inputs: Vec<ValidationInput>,
    event_roots: Vec<(TypeId, usize)>,
    edges: PreparedEdges,
    _charge: charged::StateCharge,
}
impl CallScopes {
    pub(super) async fn prepare(
        access: &CompletedInputs,
        session: &datafusion::prelude::SessionContext,
        model: &Arc<ValidatedModel>,
        budget: &resources::ResourceBudget,
        binding: bool,
    ) -> Result<Self, ModelError> {
        let inputs: Vec<_> = if binding {
            BindingData::validation_inputs()
        } else {
            EventData::validation_inputs()
        }
        .into_iter()
        .filter(|input| access.table_for(input).is_ok())
        .collect();
        let tables: Vec<_> = inputs
            .iter()
            .map(|input| {
                Ok(ClosureTable {
                    relation: model
                        .relation(input.name())
                        .ok_or(ModelError::Schema(input.name()))?
                        .clone(),
                    alias: access.table_for(input)?,
                })
            })
            .collect::<Result<_, ModelError>>()?;
        Self::from_tables(inputs, tables, session, budget, binding, false).await
    }
    pub(super) async fn from_tables(
        inputs: Vec<ValidationInput>,
        tables: Vec<ClosureTable>,
        session: &datafusion::prelude::SessionContext,
        budget: &resources::ResourceBudget,
        binding: bool,
        admission: bool,
    ) -> Result<Self, ModelError> {
        Self::from_tables_with(
            inputs,
            tables,
            session,
            budget,
            binding,
            admission,
            |_, _| Ok(()),
        )
        .await
    }
    pub(crate) async fn from_tables_with(
        inputs: Vec<ValidationInput>,
        mut tables: Vec<ClosureTable>,
        session: &datafusion::prelude::SessionContext,
        budget: &resources::ResourceBudget,
        binding: bool,
        admission: bool,
        extra: impl FnOnce(&mut NominalClosure, &[ClosureTable]) -> Result<(), ModelError>,
    ) -> Result<Self, ModelError> {
        let mut roots = if binding {
            vec![TypeId::of::<normalized::events::NormalizedCallEvent>()]
        } else {
            vec![
                TypeId::of::<calls::ProviderCallSite>(),
                TypeId::of::<calls::CallTarget>(),
                TypeId::of::<calls::CallResolution>(),
            ]
        };
        if !binding
            && tables.iter().any(|table| {
                table.relation.type_id() == TypeId::of::<flow::FlowValuePathObservation>()
            })
        {
            roots.push(TypeId::of::<flow::FlowValuePathObservation>());
        }
        if !binding && admission {
            roots.push(TypeId::of::<normalized::events::NormalizedCallEvent>());
        }
        let mut event_roots = Vec::new();
        for kind in roots {
            let source = tables
                .iter()
                .position(|table| table.relation.type_id() == kind)
                .ok_or_else(|| ModelError::Invalid("call root stream absent".into()))?;
            event_roots.push((kind, tables.len()));
            tables.push(tables[source].clone());
        }
        let index = |kind: TypeId| {
            tables[..inputs.len()]
                .iter()
                .position(|table| table.relation.type_id() == kind)
        };
        let mut plan = NominalClosure::new(tables.clone())?;
        // Forward dependencies never turn an occurrence or source artifact into additional roots.
        for (source, table) in tables[..inputs.len()].iter().enumerate() {
            for field in table.relation.fields().iter().filter(|field| !field.list()) {
                if let Some((kind, _)) = field.target()
                    && let Some(target) = index(kind)
                {
                    plan.follow(source, field.name(), target)?;
                }
            }
        }
        macro_rules! own {
            ($member:ty, $field:literal, $owner:ty) => {{
                if let (Some(member), Some(owner)) = (
                    index(TypeId::of::<$member>()),
                    index(TypeId::of::<$owner>()),
                ) {
                    plan.own(member, $field, owner)?;
                }
            }};
        }
        macro_rules! own_existing {
            ($member:ty, $field:literal, $owner:ty) => {{
                if let (Some(member),Some(owner))=(index(TypeId::of::<$member>()),index(TypeId::of::<$owner>())) {
                    plan.own_existing(member,$field,owner)?;
                }
            }};
        }
        use calls::*;
        use normalized::{callables::*, entities::*, events::*};
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
            tables[..inputs.len()].iter().position(|table| {
                table.relation.type_id() == TypeId::of::<normalized::receiver::ReceiverAssessment>()
            }),
            tables[..inputs.len()]
                .iter()
                .position(|table| table.relation.type_id() == TypeId::of::<CallTarget>()),
        ) {
            for field in tables[assessments]
                .relation
                .fields()
                .iter()
                .filter(|field| {
                    field.target().map(|(kind, _)| kind) == Some(TypeId::of::<CallTarget>())
                })
            {
                plan.own(assessments, field.name(), targets)?;
            }
        }
        // Candidate bags are defined by semantic context, not merely by a nominal reference.
        let table = |kind: TypeId| index(kind).map(|i| identifier(&tables[i].alias));
        macro_rules! pair {
            ($from:ty, $to:ty, $sql:expr) => {{
                if let (Some(from), Some(to)) =
                    (index(TypeId::of::<$from>()), index(TypeId::of::<$to>()))
                {
                    plan.pairs(from, to, $sql)?;
                }
            }};
        }
        let qualifications = table(TypeId::of::<assertion::AssertionQualification>())
            .ok_or(ModelError::Schema(assertion::AssertionQualification::NAME))?;
        own_existing!(CallSyntax, "site", source::Occurrence);
        if let (Some(syntax), Some(placements)) = (
            table(TypeId::of::<CallSyntax>()),
            table(TypeId::of::<syntax::SyntaxPlacement>()),
        ) {
            pair!(
                CallSyntax,
                syntax::SyntaxPlacement,
                format!(
                    "SELECT s.id AS source_id,p.id AS target_id FROM {syntax} s JOIN {placements} p ON p.parent=s.callee AND p.field={}",
                    lexical::SyntaxField::Value.code()
                )
            );
        }
        if let Some(coverage) = table(TypeId::of::<attribution::ProviderCoverage>()) {
            pair!(
                assertion::AssertionQualification,
                attribution::ProviderCoverage,
                format!(
                    "SELECT q.id AS source_id,c.id AS target_id FROM {qualifications} q JOIN {coverage} c ON c.scope=q.scope AND c.context=q.context WHERE c.family IN ({},{},{})",
                    attribution::FactFamily::Calls.code(),
                    attribution::FactFamily::Syntax.code(),
                    attribution::FactFamily::Signatures.code()
                )
            );
        }
        if let Some(symbols) = table(TypeId::of::<ProviderSymbol>()) {
            // A selected native symbol owns its complete enumeration alternatives. Constructor
            // consumers must borrow the same per-symbol domain captured by the binding owner,
            // including alternatives that have no normalized runtime signature variant.
            own_existing!(Signature, "symbol", ProviderSymbol);
            own_existing!(SignatureEnumerationObservation, "symbol", ProviderSymbol);
            if let Some(resolutions) = table(TypeId::of::<SymbolEntityResolution>()) {
                pair!(
                    ProviderSymbol,
                    SymbolEntityResolution,
                    format!(
                        "SELECT s.id AS source_id,r.id AS target_id FROM {symbols} s JOIN {resolutions} r ON r.symbol=s.id AND r.context=s.context"
                    )
                );
            }
            if let Some(traits) = table(TypeId::of::<symbols::FunctionTraitObservation>()) {
                pair!(
                    ProviderSymbol,
                    symbols::FunctionTraitObservation,
                    format!(
                        "SELECT s.id AS source_id,t.id AS target_id FROM {symbols} s JOIN {traits} t ON t.symbol=s.id JOIN {qualifications} q ON q.id=t.qualification AND q.context=s.context UNION SELECT s.id AS source_id,t.id AS target_id FROM {symbols} s JOIN {traits} t ON t.overrides=s.id JOIN {symbols} candidate ON candidate.id=t.symbol AND candidate.context=s.context AND candidate.provider=s.provider"
                    )
                );
            }
            if let Some(ancestry) = table(TypeId::of::<symbols::ClassAncestryObservation>()) {
                pair!(
                    ProviderSymbol,
                    symbols::ClassAncestryObservation,
                    format!(
                        "SELECT s.id AS source_id,a.id AS target_id FROM {symbols} s JOIN {ancestry} a ON a.class=s.id JOIN {qualifications} q ON q.id=a.qualification AND q.context=s.context"
                    )
                );
            }
            own_existing!(declarations::SymbolDeclaration, "symbol", ProviderSymbol);
        }
        if let (Some(symbols), Some(signatures), Some(variants)) = (
            table(TypeId::of::<ProviderSymbol>()),
            table(TypeId::of::<Signature>()),
            table(TypeId::of::<SignatureVariant>()),
        ) {
            pair!(
                ProviderSymbol,
                SignatureVariant,
                format!(
                    "SELECT s.id AS source_id,v.id AS target_id FROM {symbols} s JOIN {signatures} sig ON sig.symbol=s.id JOIN {variants} v ON v.signature=sig.id"
                )
            );
        }
        if let (Some(signatures), Some(enumerations)) = (
            table(TypeId::of::<Signature>()),
            table(TypeId::of::<SignatureEnumerationObservation>()),
        ) {
            pair!(
                Signature,
                SignatureEnumerationObservation,
                format!(
                    "SELECT s.id AS source_id,e.id AS target_id FROM {signatures} s JOIN {enumerations} e ON e.symbol=s.symbol AND e.qualification=s.qualification AND e.role=s.role"
                )
            );
        }
        for (kind, virtual_root) in &event_roots {
            let alias = identifier(&tables[*virtual_root].alias);
            if *kind == TypeId::of::<flow::FlowValuePathObservation>() {
                let real = index(*kind).expect("actual path root");
                plan.pairs(
                    *virtual_root,
                    real,
                    format!("SELECT id AS source_id,id AS target_id FROM {alias}"),
                )?;
                if let Some(steps) = index(TypeId::of::<flow::FlowCallStep>()) {
                    let step_alias = identifier(&tables[steps].alias);
                    plan.pairs(*virtual_root, steps, format!("SELECT p.id AS source_id,s.id AS target_id FROM {alias} p JOIN {step_alias} s ON s.path=p.path"))?;
                }
                if admission {
                    if let (Some(events), Some(steps)) = (
                        index(TypeId::of::<NormalizedCallEvent>()),
                        table(TypeId::of::<flow::FlowCallStep>()),
                    ) {
                        let event_table = identifier(&tables[events].alias);
                        plan.pairs(*virtual_root, events, format!("SELECT p.id AS source_id,e.id AS target_id FROM {alias} p JOIN {qualifications} q ON q.id=p.qualification JOIN {steps} s ON s.path=p.path JOIN {event_table} e ON e.site=s.call AND e.context=q.context"))?;
                    }
                    if let Some(links) = index(TypeId::of::<FlowCallEventLink>()) {
                        let links_table = identifier(&tables[links].alias);
                        plan.pairs(*virtual_root, links, format!("SELECT p.id AS source_id,l.id AS target_id FROM {alias} p JOIN {links_table} l ON l.observation=p.id"))?;
                    }
                }
            } else if binding {
                let real = index(*kind).expect("actual event root");
                plan.pairs(
                    *virtual_root,
                    real,
                    format!("SELECT id AS source_id,id AS target_id FROM {alias}"),
                )?;
            } else {
                for candidate in [
                    TypeId::of::<ProviderCallSite>(),
                    TypeId::of::<CallTarget>(),
                    TypeId::of::<CallResolution>(),
                ] {
                    let real = index(candidate).expect("actual event candidate");
                    let candidates = identifier(&tables[real].alias);
                    let sql = if *kind == TypeId::of::<NormalizedCallEvent>() {
                        format!(
                            "SELECT r.id AS source_id,c.id AS target_id FROM {alias} r JOIN {candidates} c ON c.site=r.site AND c.origin=r.origin JOIN {qualifications} cq ON cq.id=c.qualification AND cq.context=r.context"
                        )
                    } else {
                        format!(
                            "SELECT r.id AS source_id,c.id AS target_id FROM {alias} r JOIN {candidates} c ON c.site=r.site AND c.origin=r.origin JOIN {qualifications} rq ON rq.id=r.qualification JOIN {qualifications} cq ON cq.id=c.qualification AND cq.context=rq.context"
                        )
                    };
                    plan.pairs(*virtual_root, real, sql)?;
                }
                if admission {
                    let events = index(TypeId::of::<NormalizedCallEvent>())
                        .ok_or(ModelError::Schema(NormalizedCallEvent::NAME))?;
                    let event_table = identifier(&tables[events].alias);
                    let sql = if *kind == TypeId::of::<NormalizedCallEvent>() {
                        format!(
                            "SELECT r.id AS source_id,e.id AS target_id FROM {alias} r JOIN {event_table} e ON e.site=r.site AND e.origin=r.origin AND e.context=r.context"
                        )
                    } else {
                        format!(
                            "SELECT r.id AS source_id,e.id AS target_id FROM {alias} r JOIN {qualifications} q ON q.id=r.qualification JOIN {event_table} e ON e.site=r.site AND e.origin=r.origin AND e.context=q.context"
                        )
                    };
                    plan.pairs(*virtual_root, events, sql)?;
                }
            }
        }
        extra(&mut plan, &tables)?;
        let edges = plan.prepare(session, budget).await?;
        let mut charge = charged::StateCharge::new(budget, "call-scope-descriptors");
        charge.grow(
            inputs.capacity() * size_of::<ValidationInput>()
                + tables.capacity() * size_of::<ClosureTable>(),
        )?;
        Ok(Self {
            inputs,
            event_roots,
            edges,
            _charge: charge,
        })
    }
    pub(crate) fn edges(&self) -> &PreparedEdges {
        &self.edges
    }
    pub(crate) fn inputs(&self) -> &[ValidationInput] {
        &self.inputs
    }
    pub(super) async fn root_grain(
        &self,
        kind: TypeId,
        bytes: &[u8],
        budget: &resources::ResourceBudget,
    ) -> Result<PreparedClosure, ModelError> {
        let root = self
            .event_roots
            .iter()
            .find(|(candidate, _)| *candidate == kind)
            .ok_or_else(|| ModelError::Invalid("call admission root absent".into()))?
            .1;
        let hex = bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        self.edges
            .grain(root, &format!("id=X'{hex}'"), budget)
            .await
    }
    pub(crate) async fn grain<R: Record>(
        &self,
        id: Id<R>,
        budget: &resources::ResourceBudget,
    ) -> Result<PreparedClosure, ModelError> {
        let root = self
            .event_roots
            .iter()
            .find(|(kind, _)| *kind == TypeId::of::<R>())
            .ok_or(ModelError::Schema(R::NAME))?
            .1;
        let hex = id
            .bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        self.edges
            .grain(root, &format!("id=X'{hex}'"), budget)
            .await
    }
    pub(super) async fn event_data<R: Record>(
        &self,
        access: &CompletedInputs,
        root: Id<R>,
        budget: &resources::ResourceBudget,
    ) -> Result<EventData, ModelError> {
        let grain = self.grain(root, budget).await?;
        let mut data = EventData::new(budget);
        macro_rules! read {($($field:ident:$ty:ty,)*) => {$({
            if let Some((table, input)) = self.inputs.iter().enumerate().find(|(_, input)| input.type_id() == TypeId::of::<$ty>()) {
                let permit = access.read_at::<$ty>(input.prefix())?;
                crate::consumed_rows::stream_query_at(&permit, input, grain.session(), &grain.select(table)?, |_, batch| data.$field.decode(batch)).await?;
            }
        })*};}
        lctx_model::normalized_event_inputs!(read);
        Ok(data)
    }
    pub(super) async fn binding_data(
        &self,
        access: &CompletedInputs,
        event: Id<normalized::events::NormalizedCallEvent>,
        budget: &resources::ResourceBudget,
    ) -> Result<BindingData, ModelError> {
        let grain = self.grain(event, budget).await?;
        let mut data = BindingData::new(budget);
        macro_rules! read {($($field:ident:$ty:ty,)*) => {$({
            if let Some((table, input)) = self.inputs.iter().enumerate().find(|(_, input)| input.type_id() == TypeId::of::<$ty>()) {
                let permit = access.read_at::<$ty>(input.prefix())?;
                crate::consumed_rows::stream_query_at(&permit, input, grain.session(), &grain.select(table)?, |_, batch| data.$field.decode(batch)).await?;
            }
        })*};}
        lctx_model::normalized_binding_inputs!(read);
        Ok(data)
    }
}

#[cfg(test)]
mod call_scope_controls {
    use super::*;
    use crate::workspace::{Workspace, WorkspaceOptions};
    use lctx_model::domain::{
        assertion::*,
        attribution::*,
        calls::*,
        normalized::{entities::*, event_normalization, receiver},
        source::*,
        stages::*,
    };
    fn nominal<R>(value: u8) -> Id<R> {
        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
            _,
            serde::de::value::Error,
        >::new([value; 16].into_iter()))
        .unwrap()
    }
    #[tokio::test]
    async fn event_grain_keeps_orphans_and_declared_skew_without_unrelated_source_payload() {
        let model = Arc::new(model().unwrap());
        let runtime = Workspace::new(model.clone(), WorkspaceOptions::default(), crate::test_native::store()
).unwrap();
        let artifact =
            SourceArtifact::from_bytes(nominal(1), "selected.py".into(), b"call").unwrap();
        let unrelated = SourceArtifact::from_bytes(
            nominal(2),
            format!("{}.py", "x".repeat(128 << 10)),
            b"foreign",
        )
        .unwrap();
        let site = Occurrence {
            source: artifact.id(),
            start: 0,
            end: 4,
            syntax_kind: SyntaxKind::ExprCall,
            role: OccurrenceRole::Syntax,
            structural_path: vec![0],
        };
        let scope = CoverageScope::Artifact {
            artifact: artifact.id(),
        };
        let qualification = AssertionQualification {
            context: nominal(3),
            scope: scope.id(),
            condition: conditions::Diagram::always().id(),
            modality: Modality::Definite,
            approximation: Approximation::Exact,
            assumptions: assumptions::AssumptionSet::empty_id(),
        };
        let foreign = AssertionQualification {
            context: nominal(4),
            ..qualification.clone()
        };
        let destination = CallDestination::Unresolved {
            reason: obligation::ObligationKind::UnresolvedTarget,
            native: None,
        };
        let receiver = Receiver::Unknown {
            reason: obligation::ObligationKind::AmbiguousBinding,
        };
        let target = CallTarget {
            qualification: qualification.id(),
            site: site.id(),
            origin: CallOrigin::explicit(),
            destination: destination.id(),
            channel: CallChannel::Direct.id(),
            phase: CallPhase::Call,
            receiver: receiver.id(),
            implicit: false,
            receiver_class: None,
            passing: None,
            class_method: None,
            static_method: None,
        };
        let orphan = CallTarget {
            phase: CallPhase::Init,
            ..target.clone()
        };
        let skew = CallTarget {
            qualification: foreign.id(),
            ..target.clone()
        };
        let (resolution, _) = CallResolution::new(
            &qualification,
            site.id(),
            target.origin,
            target.channel,
            target.phase,
            true,
            std::slice::from_ref(&target),
        )
        .unwrap();
        let member = CallResolutionMember {
            resolution: resolution.id(),
            target: skew.id(),
        };
        let support = CallResolutionSupport {
            assertion: resolution.id(),
            run: nominal(5),
            surface: nominal(6),
            evidence: nominal(7),
            origin: Origin::AnalyzerAssertion,
            mode: ExtractionMode::NativeTraversal,
            fidelity: Fidelity::NativeStructural,
        };
        let entity = EntityRef::Occurrence {
            occurrence: site.id(),
        };
        let owner = OccurrenceOwnership {
            occurrence: site.id(),
            owner: site.id(),
            entity: entity.id(),
        };
        let access = runtime
            .inputs("call-scope-facts", Profile::Catalog, [])
            .unwrap();
        let output = runtime.output(
            "call-scope-facts",
            Profile::Catalog,
            ContentHash::of(b"call-scope-control"),
            access,
        { let mut inventory=Vec::new(); macro_rules! inventory {($($field:ident:$ty:ty,)*) => {$(inventory.push(<$ty>::NAME);)*};} lctx_model::normalized_event_inputs!(inventory); inventory },
        );
        macro_rules! declare {($($field:ident:$ty:ty,)*) => {$(output.declare::<$ty>().unwrap();)*};}
        lctx_model::normalized_event_inputs!(declare);
        macro_rules! push {($($row:expr),* $(,)?) => {$(output.push($row).await.unwrap();)*};}
        push!(
            artifact.clone(),
            unrelated,
            site,
            scope,
            qualification.clone(),
            foreign,
            destination,
            receiver,
            CallChannel::Direct,
            target.clone(),
            orphan.clone(),
            skew.clone(),
            resolution.clone(),
            member.clone(),
            support,
            entity,
            owner
        );
        output.finish(ProviderOutcome::Complete).await.unwrap();
        runtime.freeze_inputs(PublicationBoundary::Facts).unwrap();
        let mut declaration = event_normalization::stage(Profile::Catalog);
        declaration.inputs = EventData::stage_inputs()
            .into_iter()
            .filter(|input| {
                input.name() != flow::FlowValuePathObservation::NAME
                    && input.name() != flow::FlowCallStep::NAME
            })
            .map(|input| {
                if is_vocabulary(input.name()) {
                    input.at_epoch(PublicationBoundary::Facts)
                } else {
                    input
                }
            })
            .collect();
        let access = runtime
            .stage_inputs(&declaration, Profile::Catalog)
            .unwrap();
        let session = access.session(&runtime).await.unwrap();
        let scopes = CallScopes::prepare(&access, &session, &model, runtime.budget(), false)
            .await
            .unwrap();
        let small = resources::ResourceBudget::fixed(96 << 10).unwrap();
        let selected = scopes
            .event_data(&access, target.id(), &small)
            .await
            .unwrap();
        assert_eq!(selected.artifacts.len(), 1);
        assert_eq!(selected.artifacts.get(artifact.id()), Some(&artifact));
        assert!(selected.targets.get(orphan.id()).is_some());
        assert!(selected.targets.get(skew.id()).is_some());
        assert_eq!(selected.resolutions.get(resolution.id()), Some(&resolution));
        assert_eq!(selected.members.get(member.id()), Some(&member));
        let receivers = receiver::normalize_produced(&receiver::ReceiverData::new(&small), &small)
            .unwrap()
            .1;
        let missing = event_normalization::admit_event(
            &selected,
            &event_normalization::EventOutput::new(&small),
            (target.site, target.origin, qualification.context),
            &small,
        )
        .unwrap_err();
        assert!(
            missing
                .to_string()
                .contains("no required normalized outcome"),
            "{missing}"
        );
        let error = event_normalization::normalize_event_produced(
            &selected,
            (target.site, target.origin, qualification.context),
            &receivers,
            &small,
        )
        .err()
        .unwrap();
        assert!(
            error
                .to_string()
                .contains("crosses qualification context or scope"),
            "{error}"
        );
    }
    #[test]
    fn empty_owner_indices_are_not_interchangeable_across_attempt_budgets() {
        let first = resources::ResourceBudget::fixed(1 << 20).unwrap();
        let second = resources::ResourceBudget::fixed(1 << 20).unwrap();
        let mut receiver =
            receiver::normalize_produced(&receiver::ReceiverData::new(&first), &first)
                .unwrap()
                .1;
        let foreign = receiver::normalize_produced(&receiver::ReceiverData::new(&second), &second)
            .unwrap()
            .1;
        assert!(receiver.append(foreign).is_err());
        let mut event = event_normalization::normalize_events_produced(
            &EventData::new(&first),
            &receiver,
            &first,
        )
        .unwrap()
        .1;
        let foreign_receiver =
            receiver::normalize_produced(&receiver::ReceiverData::new(&second), &second)
                .unwrap()
                .1;
        let foreign = event_normalization::normalize_events_produced(
            &EventData::new(&second),
            &foreign_receiver,
            &second,
        )
        .unwrap()
        .1;
        assert!(event.append(foreign).is_err());
    }
}
