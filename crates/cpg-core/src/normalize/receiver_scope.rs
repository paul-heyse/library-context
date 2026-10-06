//! Receiver applicability owns one target and its complete syntax/placement candidate domains.
use crate::{
    consumed_rows::{ClosureTable, NominalClosure, PreparedEdges, identifier},
    workspace::CompletedInputs,
};
use lctx_model::domain::{calls::*, normalized::receiver::ReceiverData, *};
use std::{any::TypeId, sync::Arc};

pub(super) struct ReceiverScopes {
    inputs: Vec<ValidationInput>,
    tables: Vec<ClosureTable>,
    edges: PreparedEdges,
    _charge: charged::StateCharge,
}

impl ReceiverScopes {
    pub(super) async fn prepare(
        access: &CompletedInputs,
        session: &datafusion::prelude::SessionContext,
        model: &Arc<ValidatedModel>,
        budget: &resources::ResourceBudget,
    ) -> Result<Self, ModelError> {
        let inputs: Vec<_> = ReceiverData::validation_inputs()
            .into_iter()
            .filter(|input| {
                access
                    .relations()
                    .any(|relation| relation.name() == input.name())
            })
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
        Self::from_tables(inputs, tables, session, budget).await
    }
    pub(super) async fn from_tables(
        inputs: Vec<ValidationInput>,
        tables: Vec<ClosureTable>,
        session: &datafusion::prelude::SessionContext,
        budget: &resources::ResourceBudget,
    ) -> Result<Self, ModelError> {
        let mut plan = NominalClosure::new(tables.clone())?;
        let index = |kind: TypeId| {
            tables
                .iter()
                .position(|table| table.relation.type_id() == kind)
                .ok_or_else(|| ModelError::Invalid("receiver nominal premise absent".into()))
        };
        // Forward nominal dependencies preserve source membership and the exact qualification.
        for (source, table) in tables.iter().enumerate() {
            for field in table.relation.fields().iter().filter(|field| !field.list()) {
                if let Some((kind, _)) = field.target()
                    && let Some(target) = tables
                        .iter()
                        .position(|table| table.relation.type_id() == kind)
                {
                    plan.follow(source, field.name(), target)?;
                }
            }
        }
        macro_rules! owned {
            ($member:ty, $field:literal, $owner:ty) => {
                plan.own(
                    index(TypeId::of::<$member>())?,
                    $field,
                    index(TypeId::of::<$owner>())?,
                )?;
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
            normalized::callables::EffectiveCallableAssessment,
            "callable",
            normalized::entities::CallableEntity
        );
        owned!(
            normalized::callables::SignatureVariant,
            "assessment",
            normalized::callables::EffectiveCallableAssessment
        );
        if let (Some(assessments), Some(evidence)) = (
            tables.iter().position(|table| {
                table.relation.type_id() == TypeId::of::<normalized::receiver::ReceiverAssessment>()
            }),
            tables.iter().position(|table| {
                table.relation.type_id() == TypeId::of::<normalized::receiver::ReceiverEvidence>()
            }),
        ) {
            for field in tables[assessments]
                .relation
                .fields()
                .iter()
                .filter(|field| {
                    field.target().map(|(kind, _)| kind) == Some(TypeId::of::<CallTarget>())
                })
            {
                plan.own(
                    assessments,
                    field.name(),
                    index(TypeId::of::<CallTarget>())?,
                )?;
            }
            plan.own(evidence, "assessment", assessments)?;
        }
        let table = |kind: TypeId| -> Result<String, ModelError> {
            Ok(identifier(&tables[index(kind)?].alias))
        };
        let targets = table(TypeId::of::<CallTarget>())?;
        let qualifications = table(TypeId::of::<assertion::AssertionQualification>())?;
        let syntax = table(TypeId::of::<CallSyntax>())?;
        let placements = table(TypeId::of::<syntax::SyntaxPlacement>())?;
        let destinations = table(TypeId::of::<CallDestination>())?;
        let resolutions = table(TypeId::of::<normalized::entities::SymbolEntityResolution>())?;
        let coverage = table(TypeId::of::<attribution::ProviderCoverage>())?;
        plan.pairs(index(TypeId::of::<CallTarget>())?, index(TypeId::of::<CallSyntax>())?, format!(
            "SELECT t.id AS source_id,s.id AS target_id FROM {targets} t JOIN {syntax} s ON t.site=s.site JOIN {qualifications} tq ON tq.id=t.qualification JOIN {qualifications} sq ON sq.id=s.qualification AND sq.context=tq.context"))?;
        // Conflicting placement frames remain candidates: uniqueness must see the whole Value domain.
        plan.pairs(index(TypeId::of::<CallSyntax>())?, index(TypeId::of::<syntax::SyntaxPlacement>())?, format!(
            "SELECT s.id AS source_id,p.id AS target_id FROM {syntax} s JOIN {placements} p ON p.parent=s.callee AND p.field={}", lexical::SyntaxField::Value.code()))?;
        plan.pairs(index(TypeId::of::<CallTarget>())?, index(TypeId::of::<normalized::entities::SymbolEntityResolution>())?, format!(
            "SELECT t.id AS source_id,r.id AS target_id FROM {targets} t JOIN {destinations} d ON d.id=t.destination JOIN {qualifications} q ON q.id=t.qualification JOIN {resolutions} r ON r.symbol=COALESCE(d.resolved_symbol,d.overrides_symbol) AND r.context=q.context"))?;
        // Coverage is a scope/context/family premise, never all neighbors of an input or run.
        plan.pairs(index(TypeId::of::<assertion::AssertionQualification>())?, index(TypeId::of::<attribution::ProviderCoverage>())?, format!(
            "SELECT q.id AS source_id,c.id AS target_id FROM {qualifications} q JOIN {coverage} c ON c.scope=q.scope AND c.context=q.context WHERE c.family IN ({},{})", attribution::FactFamily::Calls.code(), attribution::FactFamily::Syntax.code()))?;
        let mut charge = charged::StateCharge::new(budget, "receiver-scope-descriptors");
        charge.grow(
            inputs.capacity() * size_of::<ValidationInput>()
                + tables.capacity() * size_of::<ClosureTable>(),
        )?;
        let edges = plan.prepare(session, budget).await?;
        Ok(Self {
            inputs,
            tables,
            edges,
            _charge: charge,
        })
    }
    pub(super) fn inputs(&self) -> &[ValidationInput] {
        &self.inputs
    }
    pub(super) async fn root_grain(
        &self,
        kind: TypeId,
        bytes: &[u8],
        budget: &resources::ResourceBudget,
    ) -> Result<crate::consumed_rows::PreparedClosure, ModelError> {
        let root = self
            .tables
            .iter()
            .position(|table| table.relation.type_id() == kind)
            .ok_or_else(|| ModelError::Invalid("receiver admission root absent".into()))?;
        let hex = bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        self.edges
            .grain(root, &format!("id=X'{hex}'"), budget)
            .await
    }
    pub(super) async fn data(
        &self,
        access: &CompletedInputs,
        target: Id<CallTarget>,
        budget: &resources::ResourceBudget,
    ) -> Result<ReceiverData, ModelError> {
        let root = self
            .tables
            .iter()
            .position(|table| table.relation.type_id() == TypeId::of::<CallTarget>())
            .ok_or_else(|| ModelError::Invalid("receiver root table absent".into()))?;
        let bytes = target
            .bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let scope = self
            .edges
            .grain(root, &format!("id=X'{bytes}'"), budget)
            .await?;
        let mut data = ReceiverData::new(budget);
        macro_rules! read {($($field:ident: $ty:ty,)*) => {$({
            if let Some((table, input)) = self.inputs.iter().enumerate().find(|(_, input)| input.type_id() == TypeId::of::<$ty>()) {
                let permit = access.read_at::<$ty>(input.prefix())?;
                crate::consumed_rows::stream_query_at(&permit, input, scope.session(), &scope.select(table)?, |_, batch| data.$field.decode(batch)).await?;
            }
        })*};}
        lctx_model::normalized_receiver_inputs!(read);
        Ok(data)
    }
}

#[cfg(test)]
mod receiver_scope_controls {
    use super::*;
    use crate::workspace::{Workspace, WorkspaceOptions};
    use lctx_model::domain::{
        assertion::*, attribution::*, normalized::receiver, source::*, stages::*,
    };
    fn nominal<R>(value: u8) -> Id<R> {
        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
            _,
            serde::de::value::Error,
        >::new([value; 16].into_iter()))
        .unwrap()
    }
    #[tokio::test]
    async fn target_scope_preserves_conflicting_candidates_and_excludes_foreign_source_payload() {
        let model = Arc::new(model().unwrap());
        let runtime = Workspace::new(model.clone(), WorkspaceOptions::default()).unwrap();
        let artifact =
            SourceArtifact::from_bytes(nominal(1), "selected.py".into(), b"value").unwrap();
        let unrelated = SourceArtifact::from_bytes(
            nominal(2),
            format!("{}.py", "x".repeat(128 << 10)),
            b"foreign",
        )
        .unwrap();
        let occurrence = |start, kind| Occurrence {
            source: artifact.id(),
            start,
            end: start + 1,
            syntax_kind: kind,
            role: OccurrenceRole::Syntax,
            structural_path: vec![start as i32],
        };
        let site = occurrence(0, SyntaxKind::ExprCall);
        let callee = occurrence(1, SyntaxKind::ExprAttribute);
        let first = occurrence(2, SyntaxKind::ExprName);
        let second = occurrence(3, SyntaxKind::ExprName);
        let scope = CoverageScope::Artifact {
            artifact: artifact.id(),
        };
        let q = AssertionQualification {
            context: nominal(3),
            scope: scope.id(),
            condition: conditions::Diagram::always().id(),
            modality: Modality::Definite,
            approximation: Approximation::Exact,
            assumptions: assumptions::AssumptionSet::empty_id(),
        };
        let foreign_q = AssertionQualification {
            context: nominal(4),
            ..q.clone()
        };
        let receiver = Receiver::Unknown {
            reason: obligation::ObligationKind::AmbiguousBinding,
        };
        let destination = CallDestination::Unresolved {
            reason: obligation::ObligationKind::UnresolvedTarget,
            native: None,
        };
        let target = CallTarget {
            qualification: q.id(),
            site: site.id(),
            origin: CallOrigin::explicit(),
            destination: destination.id(),
            channel: CallChannel::Direct.id(),
            phase: CallPhase::Call,
            receiver: receiver.id(),
            implicit: false,
            receiver_class: None,
            passing: Some(ReceiverPassing::Object),
            class_method: Some(true),
            static_method: Some(false),
        };
        let (call, _) = CallSyntax::new(q.id(), site.id(), callee.id(), false, &[]).unwrap();
        let other_call = CallSyntax {
            in_annotation: true,
            ..call.clone()
        };
        let foreign_call = CallSyntax {
            qualification: foreign_q.id(),
            ..call.clone()
        };
        let placement = syntax::SyntaxPlacement {
            qualification: q.id(),
            occurrence: first.id(),
            parent: Some(callee.id()),
            field: lexical::SyntaxField::Value,
            ordinal: 0,
        };
        let conflict = syntax::SyntaxPlacement {
            qualification: foreign_q.id(),
            occurrence: second.id(),
            ..placement.clone()
        };
        let input = runtime
            .inputs("receiver-scope-facts", Profile::Catalog, [])
            .unwrap();
        let output = runtime.output(
            "receiver-scope-facts",
            Profile::Catalog,
            ContentHash::of(b"receiver-scope-control"),
            input,
        );
        macro_rules! declare {($($field:ident:$ty:ty,)*) => {$(output.declare::<$ty>().unwrap();)*};}
        lctx_model::normalized_receiver_inputs!(declare);
        macro_rules! push {($($row:expr),* $(,)?) => {$(output.push($row).await.unwrap();)*};}
        push!(
            artifact.clone(),
            unrelated,
            site,
            callee,
            first,
            second,
            scope,
            q,
            foreign_q,
            receiver,
            destination,
            target.clone(),
            call.clone(),
            other_call.clone(),
            foreign_call,
            placement.clone(),
            conflict.clone()
        );
        output.finish(ProviderOutcome::Complete).await.unwrap();
        runtime.freeze_inputs(PublicationBoundary::Facts).unwrap();
        let mut declaration = receiver::stage(Profile::Catalog);
        declaration.inputs = ReceiverData::stage_inputs()
            .into_iter()
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
        let scopes = ReceiverScopes::prepare(&access, &session, &model, runtime.budget())
            .await
            .unwrap();
        let small = resources::ResourceBudget::fixed(32 << 10).unwrap();
        let selected = scopes.data(&access, target.id(), &small).await.unwrap();
        assert_eq!(selected.targets.len(), 1);
        assert_eq!(selected.artifacts.len(), 1);
        assert_eq!(selected.artifacts.get(artifact.id()), Some(&artifact));
        assert_eq!(selected.syntax.len(), 2);
        assert!(selected.syntax.get(call.id()).is_some());
        assert!(selected.syntax.get(other_call.id()).is_some());
        assert_eq!(selected.placements.len(), 2);
        assert!(selected.placements.get(placement.id()).is_some());
        assert!(selected.placements.get(conflict.id()).is_some());
        let records = receiver::normalize_target(&selected, target.id(), &small).unwrap();
        assert_eq!(records.receiver_assessments.len(), 1);
        assert!(matches!(
            records.receiver_assessments.iter().next(),
            Some(receiver::ReceiverAssessment::Unknown {
                reason: receiver::ReceiverReason::DescriptorUnknown,
                ..
            })
        ));
    }
}
