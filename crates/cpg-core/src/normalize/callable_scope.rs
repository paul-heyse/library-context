//! Complete callable/descriptor and native-origin candidate groups selected before rich decode.
use crate::{
    consumed_rows::{
        ClosureTable, NominalClosure, PreparedClosure, PreparedEdges, identifier, stream_batches,
        stream_query_at,
    },
    workspace::CompletedInputs,
};
use arrow_array::Array;
use futures::future::BoxFuture;
use lctx_model::domain::{
    assertion::*,
    attribution::*,
    calls::*,
    lexical::*,
    normalized::{callable_normalization::CallableData, entities::*, links::*},
    source::*,
    syntax::*,
    types::*,
    *,
};
use std::{any::TypeId, sync::Arc};
#[derive(Clone, Copy)]
pub(super) enum Kernel {
    Callable,
    Signature,
    Overload,
}
pub(super) fn nominal<R>(bytes: &[u8]) -> Result<Id<R>, ModelError> {
    serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
        _,
        serde::de::value::Error,
    >::new(bytes.iter().copied()))
    .map_err(ModelError::codec)
}
pub(super) struct CallableScopes {
    inputs: Vec<ValidationInput>,
    tables: Vec<ClosureTable>,
    edges: PreparedEdges,
    kernel: Kernel,
    _charge: charged::StateCharge,
}
fn load_callable_data<'a>(
    descriptor: &'a CallableScopes,
    access: &'a CompletedInputs,
    scope: &'a PreparedClosure,
    data: &'a mut CallableData,
) -> BoxFuture<'a, Result<(), ModelError>> {
    type Loader = for<'a> fn(
        &'a CallableScopes,
        &'a CompletedInputs,
        &'a PreparedClosure,
        &'a mut CallableData,
    ) -> BoxFuture<'a, Result<(), ModelError>>;
    macro_rules! adapters {($($field:ident:$ty:ty,)*) => {
        $(fn $field<'a>(descriptor: &'a CallableScopes, access: &'a CompletedInputs, scope: &'a PreparedClosure, data: &'a mut CallableData) -> BoxFuture<'a, Result<(), ModelError>> {
            Box::pin(async move {
                let (table, input) = descriptor.inputs.iter().enumerate().find(|(_, input)| input.type_id() == TypeId::of::<$ty>()).ok_or(ModelError::Schema("callable typed scope loader"))?;
                let permit = access.read_at::<$ty>(input.prefix())?;
                stream_query_at(&permit, input, access, scope.session(), &scope.select(table)?, |_, batch| data.$field.decode(batch)).await
            })
        })*
        const LOADERS: &[Loader] = &[$($field,)*];
    };}
    lctx_model::normalized_callable_inputs!(adapters);
    Box::pin(async move {
        for load in LOADERS {
            load(descriptor, access, scope, data).await?;
        }
        Ok(())
    })
}
fn load_callable_admission<'a>(
    descriptor: &'a CallableScopes,
    scope: &'a PreparedClosure,
    cancellation: &'a crate::workspace::Cancellation,
    data: &'a mut CallableData,
) -> BoxFuture<'a, Result<(), ModelError>> {
    type Loader = for<'a> fn(
        &'a CallableScopes,
        &'a PreparedClosure,
        &'a crate::workspace::Cancellation,
        &'a mut CallableData,
    ) -> BoxFuture<'a, Result<(), ModelError>>;
    macro_rules! adapters {($($field:ident:$ty:ty,)*) => {
        $(fn $field<'a>(descriptor: &'a CallableScopes, scope: &'a PreparedClosure, cancellation: &'a crate::workspace::Cancellation, data: &'a mut CallableData) -> BoxFuture<'a, Result<(), ModelError>> {
            Box::pin(async move {
                let index = descriptor.inputs.iter().position(|input| input.type_id() == TypeId::of::<$ty>()).ok_or(ModelError::Schema("callable admission typed decoder"))?;
                let input = &descriptor.inputs[index];
                if input.name() != <$ty>::NAME || descriptor.tables[index].relation.type_id() != TypeId::of::<$ty>() {
                    return Err(ModelError::Schema("callable admission typed decoder"));
                }
                let sql = scope.select(index)?;
                let mut visit = |batch: &arrow_array::RecordBatch| { cancellation.check()?; data.$field.decode(batch) };
                stream_batches(input, scope.session(), &sql, None, &mut visit).await
            })
        })*
        const LOADERS: &[Loader] = &[$($field,)*];
    };}
    lctx_model::normalized_callable_inputs!(adapters);
    Box::pin(async move {
        for load in LOADERS {
            load(descriptor, scope, cancellation, data).await?;
        }
        Ok(())
    })
}

impl CallableScopes {
    pub(super) async fn prepare(
        access: &CompletedInputs,
        session: &datafusion::prelude::SessionContext,
        model: &Arc<ValidatedModel>,
        budget: &resources::ResourceBudget,
        kernel: Kernel,
    ) -> Result<Self, ModelError> {
        let inputs = CallableData::validation_inputs();
        let tables = inputs
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
            .collect::<Result<Vec<_>, ModelError>>()?;
        Self::prepare_bound(inputs, tables, session, budget, kernel).await
    }
    async fn prepare_bound(
        inputs: Vec<ValidationInput>,
        tables: Vec<ClosureTable>,
        session: &datafusion::prelude::SessionContext,
        budget: &resources::ResourceBudget,
        kernel: Kernel,
    ) -> Result<Self, ModelError> {
        let index = |kind: TypeId| {
            tables
                .iter()
                .position(|table| table.relation.type_id() == kind)
                .ok_or(ModelError::Schema("callable scope input"))
        };
        let table = |kind: TypeId| -> Result<String, ModelError> {
            Ok(identifier(&tables[index(kind)?].alias))
        };
        let mut plan = NominalClosure::new(tables.clone())?;
        for (source, relation) in tables.iter().enumerate() {
            for field in relation.relation.fields() {
                if let Some((target, _)) = field.target()
                    && let Ok(target) = index(target)
                {
                    if field.list() {
                        plan.pairs(
                            source,
                            target,
                            format!(
                                "SELECT id AS source_id,UNNEST({}) AS target_id FROM {}",
                                identifier(field.name()),
                                identifier(&relation.alias)
                            ),
                        )?;
                    } else {
                        plan.follow(source, field.name(), target)?;
                    }
                }
            }
        }
        macro_rules! own {
            ($member:ty,$field:literal,$owner:ty) => {
                plan.own(
                    index(TypeId::of::<$member>())?,
                    $field,
                    index(TypeId::of::<$owner>())?,
                )?
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
        let resolutions = table(TypeId::of::<SymbolEntityResolution>())?;
        let signatures = table(TypeId::of::<Signature>())?;
        let traits = table(TypeId::of::<symbols::FunctionTraitObservation>())?;
        // The actual model index chooses its final ID-ordered resolution by symbol. Context
        // filtering here would discard uncertainty or change that existing choice.
        plan.pairs(index(TypeId::of::<SymbolEntityResolution>())?,index(TypeId::of::<SymbolEntityResolution>())?,format!("SELECT a.id AS source_id,b.id AS target_id FROM {resolutions} a JOIN {resolutions} b ON a.symbol=b.symbol"))?;
        plan.pairs(index(TypeId::of::<Signature>())?,index(TypeId::of::<SymbolEntityResolution>())?,format!("SELECT s.id AS source_id,r.id AS target_id FROM {signatures} s JOIN {resolutions} r ON s.symbol=r.symbol"))?;
        plan.pairs(index(TypeId::of::<SymbolEntityResolution>())?,index(TypeId::of::<Signature>())?,format!("SELECT r.id AS source_id,s.id AS target_id FROM {resolutions} r JOIN {signatures} s ON s.symbol=r.symbol"))?;
        plan.pairs(index(TypeId::of::<SymbolEntityResolution>())?,index(TypeId::of::<symbols::FunctionTraitObservation>())?,format!("SELECT r.id AS source_id,t.id AS target_id FROM {resolutions} r JOIN {traits} t ON t.symbol=r.symbol"))?;
        let occurrences = table(TypeId::of::<Occurrence>())?;
        let placements = table(TypeId::of::<SyntaxPlacement>())?;
        let references = table(TypeId::of::<ReferenceObservation>())?;
        let coverage = table(TypeId::of::<ProviderCoverage>())?;
        let scopes = table(TypeId::of::<CoverageScope>())?;
        let owners = table(TypeId::of::<OccurrenceOwnership>())?;
        plan.pairs(index(TypeId::of::<Occurrence>())?,index(TypeId::of::<ProviderCoverage>())?,format!("SELECT o.id AS source_id,c.id AS target_id FROM {occurrences} o JOIN {scopes} scope ON scope.artifact_artifact=o.source JOIN {coverage} c ON c.scope=scope.id WHERE c.family={}",FactFamily::Syntax.code()))?;
        // Descriptor children are a complete direct candidate domain. This is deliberately
        // restricted to Decorator nodes: a declaration's whole Body is not a metadata premise.
        plan.pairs(index(TypeId::of::<Occurrence>())?,index(TypeId::of::<SyntaxPlacement>())?,format!("SELECT o.id AS source_id,p.id AS target_id FROM {occurrences} o JOIN {placements} p ON p.parent=o.id WHERE o.syntax_kind={}",SyntaxKind::Decorator.code()))?;
        plan.pairs(index(TypeId::of::<Occurrence>())?,index(TypeId::of::<ReferenceObservation>())?,format!("SELECT o.id AS source_id,r.id AS target_id FROM {occurrences} o JOIN {references} r ON r.read=o.id"))?;
        // Only owned yield forms affect the generator predicate. Ordinary rich body rows and
        // ownership paths must not enter an otherwise tiny callable metadata kernel.
        plan.pairs(index(TypeId::of::<Occurrence>())?,index(TypeId::of::<OccurrenceOwnership>())?,format!("SELECT owner.id AS source_id,m.id AS target_id FROM {occurrences} owner JOIN {owners} m ON m.owner=owner.id JOIN {occurrences} child ON child.id=m.occurrence WHERE child.syntax_kind IN ({},{})",SyntaxKind::ExprYield.code(),SyntaxKind::ExprYieldFrom.code()))?;
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
            let traces = table(TypeId::of::<NativeOverloadObservation>())?;
            let candidates = table(TypeId::of::<NativeOverloadCandidate>())?;
            let native = table(TypeId::of::<NativeSignatureObservation>())?;
            let qualifications = table(TypeId::of::<AssertionQualification>())?;
            plan.pairs(index(TypeId::of::<NativeOverloadObservation>())?,index(TypeId::of::<NativeSignatureObservation>())?,format!("SELECT trace.id AS source_id,n.id AS target_id FROM {traces} trace JOIN {candidates} candidate ON candidate.trace=trace.id JOIN {qualifications} tq ON tq.id=trace.qualification JOIN {native} n ON n.metadata_origin=candidate.origin JOIN {qualifications} nq ON nq.id=n.qualification JOIN {signatures} signature ON signature.id=n.signature WHERE nq.context=tq.context AND signature.role={}",SignatureRole::EffectiveTyped.code()))?;
        }
        let mut charge = charged::StateCharge::new(budget, "callable-scope-descriptors");
        charge.grow(
            inputs.capacity() * size_of::<ValidationInput>()
                + tables.capacity() * size_of::<ClosureTable>()
                + tables
                    .iter()
                    .map(|table| table.alias.capacity())
                    .sum::<usize>(),
        )?;
        let edges = plan.prepare(session, budget).await?;
        Ok(Self {
            inputs,
            tables,
            edges,
            kernel,
            _charge: charge,
        })
    }
    pub(super) fn data<'a, R: Record>(
        &'a self,
        access: &'a CompletedInputs,
        selected: Id<R>,
        budget: &'a resources::ResourceBudget,
    ) -> BoxFuture<'a, Result<CallableData, ModelError>> {
        self.data_selected(access, TypeId::of::<R>(), *selected.bytes(), budget)
    }
    fn data_selected<'a>(
        &'a self,
        access: &'a CompletedInputs,
        kind: TypeId,
        key: [u8; 16],
        budget: &'a resources::ResourceBudget,
    ) -> BoxFuture<'a, Result<CallableData, ModelError>> {
        Box::pin(async move {
            let root = self
                .tables
                .iter()
                .position(|table| table.relation.type_id() == kind)
                .ok_or(ModelError::Schema("callable kernel root"))?;
            let bytes = key
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>();
            let mut roots = vec![(root, format!("id=X'{bytes}'"))];
            if matches!(self.kernel, Kernel::Signature) {
                for (kind, predicate) in [
                    (
                        TypeId::of::<SignatureParameter>(),
                        format!("signature=X'{bytes}'"),
                    ),
                    (
                        TypeId::of::<NativeSignatureObservation>(),
                        format!("signature=X'{bytes}'"),
                    ),
                    (
                        TypeId::of::<SignatureTypeSubject>(),
                        format!("return_signature=X'{bytes}'"),
                    ),
                ] {
                    let table = self
                        .tables
                        .iter()
                        .position(|table| table.relation.type_id() == kind)
                        .ok_or(ModelError::Schema("signature member root"))?;
                    roots.push((table, predicate));
                }
            }
            let mut data = CallableData::new(budget);
            // Member roots are selected explicitly, rather than making all alternative signatures'
            // rich parameter shapes/native messages part of the descriptor assessment scope.
            for (root, predicate) in roots {
                let scope = self.edges.grain(root, &predicate, budget).await?;
                load_callable_data(self, access, &scope, &mut data).await?;
            }
            Ok(data)
        })
    }
}

/// Necessary owner admission uses the same candidate selector as production, but compares only
/// the exact advertised family of each actual callable. Global membership probes prevent an
/// unsupported advertised root or an unowned premise from disappearing from selected work.
pub(super) async fn validate_callables(
    invariant: &Invariant,
    tables: Vec<ClosureTable>,
    session: &datafusion::prelude::SessionContext,
    budget: &resources::ResourceBudget,
    cancellation: &crate::workspace::Cancellation,
) -> Result<(), ModelError> {
    use futures::TryStreamExt;
    use normalized::{callable_normalization, callables::*};
    let index = |kind: TypeId| {
        tables
            .iter()
            .position(|table| table.relation.type_id() == kind)
            .ok_or(ModelError::Schema("callable admission input"))
    };
    let table = |kind: TypeId| -> Result<String, ModelError> {
        Ok(identifier(&tables[index(kind)?].alias))
    };
    let callable = table(TypeId::of::<CallableEntity>())?;
    let assessment = table(TypeId::of::<EffectiveCallableAssessment>())?;
    let decorator = table(TypeId::of::<EffectiveDecoratorMember>())?;
    let premise = table(TypeId::of::<EffectiveCallablePremise>())?;
    let evidence = table(TypeId::of::<EffectiveCallableEvidence>())?;
    for query in [
        format!(
            "SELECT a.id FROM {assessment} a LEFT ANTI JOIN {callable} c ON c.id=a.callable LIMIT 1"
        ),
        format!(
            "SELECT d.id FROM {decorator} d LEFT ANTI JOIN {assessment} a ON a.id=d.assessment LIMIT 1"
        ),
        format!(
            "SELECT e.id FROM {evidence} e LEFT ANTI JOIN {assessment} a ON a.id=e.assessment LIMIT 1"
        ),
        format!(
            "SELECT e.id FROM {evidence} e LEFT ANTI JOIN {premise} p ON p.id=e.premise LIMIT 1"
        ),
        format!(
            "SELECT p.id FROM {premise} p LEFT ANTI JOIN {evidence} e ON e.premise=p.id LIMIT 1"
        ),
    ] {
        let mut rows = crate::sql::query(session, &query)
            .await
            .map_err(ModelError::codec)?
            .execute_stream()
            .await
            .map_err(ModelError::codec)?;
        while let Some(batch) = rows.try_next().await.map_err(ModelError::codec)? {
            cancellation.check()?;
            if batch.num_rows() != 0 {
                return Err(ModelError::Invalid(
                    "callable advertised family lacks actual root or membership".into(),
                ));
            }
        }
    }
    let root = index(TypeId::of::<CallableEntity>())?;
    let prepared = CallableScopes::prepare_bound(
        invariant.inputs.clone(),
        tables,
        session,
        budget,
        Kernel::Callable,
    )
    .await?;
    let mut roots = crate::sql::query(session, &format!("SELECT id FROM {callable} ORDER BY id"))
        .await
        .map_err(ModelError::codec)?
        .execute_stream()
        .await
        .map_err(ModelError::codec)?;
    while let Some(batch) = roots.try_next().await.map_err(ModelError::codec)? {
        let ids = batch
            .column(0)
            .as_any()
            .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
            .ok_or(ModelError::Schema("callable admission roots"))?;
        for row in 0..ids.len() {
            cancellation.check()?;
            let selected: Id<CallableEntity> = nominal(ids.value(row))?;
            let bytes = ids
                .value(row)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>();
            let scope = prepared
                .edges
                .grain(root, &format!("id=X'{bytes}'"), budget)
                .await?;
            let mut data = CallableData::new(budget);
            load_callable_admission(&prepared, &scope, cancellation, &mut data).await?;
            let mut stored = callable_normalization::CallableOutput::new(budget);
            // These predicates select stored membership independently of a producer's claimed
            // premise closure. All expected premises still come from actual source candidates.
            for query in [
                format!("SELECT a.* FROM {assessment} a WHERE a.callable=X'{bytes}' ORDER BY a.id"),
                format!("SELECT d.* FROM {decorator} d JOIN {assessment} a ON a.id=d.assessment WHERE a.callable=X'{bytes}' ORDER BY d.id"),
                format!("SELECT p.* FROM {premise} p JOIN {evidence} e ON e.premise=p.id JOIN {assessment} a ON a.id=e.assessment WHERE a.callable=X'{bytes}' ORDER BY p.id"),
                format!("SELECT e.* FROM {evidence} e JOIN {assessment} a ON a.id=e.assessment WHERE a.callable=X'{bytes}' ORDER BY e.id"),
            ].into_iter().enumerate() {
                let mut rows=crate::sql::query(session,&query.1).await.map_err(ModelError::codec)?.execute_stream().await.map_err(ModelError::codec)?;
                while let Some(batch)=rows.try_next().await.map_err(ModelError::codec)? {cancellation.check()?;match query.0 {0=>stored.assessments.decode(&batch)?,1=>stored.decorators.decode(&batch)?,2=>stored.premises.decode(&batch)?,3=>stored.evidence.decode(&batch)?,_=>unreachable!()};tokio::task::yield_now().await;}
            }
            callable_normalization::admit_callable(&data, &stored, selected, budget)?;
        }
    }
    Ok(())
}
