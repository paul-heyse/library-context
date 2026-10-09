//! Single S0 writer; rendering never reconstructs graphs or invents completed parents.
use crate::producer_operations::{self, Declaration};
use crate::{
    synthesis_preparation,
    workspace::{CompletedInputs, ProducerOutput, Workspace},
};
use futures::future::BoxFuture;
use lctx_model::domain::{
    analysis::{self, synthesis::*},
    normalized::Rows,
    stages::*,
    synthesis::{self, production::Data},
    *,
};
use std::sync::Arc;
// Decoder reachability is separate from the model-owned consumed source inventory.
macro_rules! decoder_inputs {
    ($apply:ident) => {
        lctx_model::synthesis_frame_inputs!($apply);
        lctx_model::synthesis_documentary_inputs!($apply);
        lctx_model::synthesis_automatic_inputs!($apply);
        lctx_model::synthesis_observation_inputs!($apply);
        lctx_model::synthesis_summary_inputs!($apply);
        lctx_model::synthesis_terminal_inputs!($apply);
        lctx_model::synthesis_pattern_inputs!($apply);
        lctx_model::synthesis_setup_inputs!($apply);
        lctx_model::synthesis_pattern_named_inputs!($apply);
        lctx_model::synthesis_control_text_inputs!($apply);
        lctx_model::ownership_scope_inputs!($apply);
        lctx_model::expected_domain_inputs!($apply);
        $apply! {public:structural::PublicCandidate,handoff_values:structural::handoffs::ValueSource,}
    };
}
#[derive(Clone, Copy)]
enum LoadPhase {
    Documentary,
    Member,
    Conclusion,
}
struct SynthesisScopes {
    inputs: Vec<ValidationInput>,
    tables: Vec<crate::consumed_rows::ClosureTable>,
    edges: crate::consumed_rows::PreparedEdges,
    member: usize,
}
fn typed<R: Record>(inputs: &[ValidationInput]) -> Result<usize, ModelError> {
    inputs
        .iter()
        .position(|input| input.type_id() == std::any::TypeId::of::<R>())
        .ok_or(ModelError::Schema("S0 scope relation absent"))
}
fn scope_target(
    inputs: &[ValidationInput],
    source: usize,
    kind: std::any::TypeId,
) -> Result<Option<usize>, ModelError> {
    use std::any::TypeId;
    let candidates: Vec<_> = inputs
        .iter()
        .enumerate()
        .filter(|(_, input)| input.type_id() == kind)
        .map(|(index, _)| index)
        .collect();
    if candidates.len() <= 1 {
        return Ok(candidates.first().copied());
    }
    let analytic = [
        TypeId::of::<structural::Conclusion>(),
        TypeId::of::<structural::ConclusionSource>(),
        TypeId::of::<analytics::Conclusion>(),
        TypeId::of::<analytics::ConclusionSource>(),
        TypeId::of::<execution::summary_consequences::ClaimConclusion>(),
        TypeId::of::<execution::summary_consequences::ClaimProof>(),
        TypeId::of::<execution::summary_consequences::SummaryClaim>(),
        TypeId::of::<analysis::summary::AnalysisDerivation>(),
        TypeId::of::<analysis::summary::AnalysisProposition>(),
        TypeId::of::<analysis::summary::AnalysisDerivationPremise>(),
        TypeId::of::<execution::summary_terminal::SummaryTerminalWitness>(),
        TypeId::of::<execution::protocol_interpretation::ConditionalTerminalFrontier>(),
        TypeId::of::<execution::protocol_interpretation::NormalContinuationRestriction>(),
    ];
    let epoch =
        inputs[source]
            .prefix()
            .unwrap_or(if analytic.contains(&inputs[source].type_id()) {
                PublicationBoundary::Analytic
            } else {
                PublicationBoundary::Facts
            });
    let selected: Vec<_> = candidates
        .into_iter()
        .filter(|index| inputs[*index].prefix() == Some(epoch))
        .collect();
    if selected.len() != 1 {
        return Err(ModelError::Conflict("S0 exact immutable vocabulary"));
    }
    Ok(selected.first().copied())
}
type MetadataLoader = for<'a, 'sources> fn(
    &'a CompletedInputs,
    &'a datafusion::prelude::SessionContext,
    &'a [ValidationInput],
    &'a mut crate::consumed_rows::ConsumedInputs,
    &'a mut analysis::expected::CoverageAdmission<'sources>,
    &'a mut Data,
) -> BoxFuture<'a, Result<(), ModelError>>;
fn load_metadata<'a, 'sources, R: Record>(
    access: &'a CompletedInputs,
    session: &'a datafusion::prelude::SessionContext,
    topology: &'a [ValidationInput],
    consumed: &'a mut crate::consumed_rows::ConsumedInputs,
    admission: &'a mut analysis::expected::CoverageAdmission<'sources>,
    data: &'a mut Data,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        while let Some((input, permit)) = consumed.next::<R>(access)? {
            if crate::consumed_rows::stream_artifact_admission(access, &input, session, admission)
                .await?
            {
                continue;
            }
            crate::consumed_rows::stream_at(&permit, &input, access, session, |permit, batch| {
                admission.visit_if_expected(permit, batch)?;
                if topology.iter().any(|item| {
                    item.type_id() == input.type_id() && item.prefix() == input.prefix()
                }) {
                    data.visit_input(&input, batch)?;
                }
                Ok(())
            })
            .await?;
        }
        Ok(())
    })
}
fn read_metadata<'a, 'sources>(
    access: &'a CompletedInputs,
    session: &'a datafusion::prelude::SessionContext,
    topology: &'a [ValidationInput],
    consumed: &'a mut crate::consumed_rows::ConsumedInputs,
    admission: &'a mut analysis::expected::CoverageAdmission<'sources>,
    data: &'a mut Data,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        let mut loaders: Vec<MetadataLoader> = Vec::new();
        macro_rules! read {($($field:ident:$ty:ty,)*) => {$(loaders.push(load_metadata::<$ty>);)*};}
        decoder_inputs!(read);
        for loader in loaders {
            loader(access, session, topology, consumed, admission, data).await?;
        }
        Ok(())
    })
}
type ScopedLoader = for<'a> fn(
    &'a SynthesisScopes,
    &'a CompletedInputs,
    &'a crate::consumed_rows::PreparedClosure,
    LoadPhase,
    &'a mut crate::consumed_rows::ConsumedInputs,
    &'a mut Data,
) -> BoxFuture<'a, Result<(), ModelError>>;
fn load_scoped<'a, R: Record>(
    scopes: &'a SynthesisScopes,
    access: &'a CompletedInputs,
    scope: &'a crate::consumed_rows::PreparedClosure,
    phase: LoadPhase,
    consumed: &'a mut crate::consumed_rows::ConsumedInputs,
    data: &'a mut Data,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        while let Some((input, permit)) = consumed.next::<R>(access)? {
            let table = scopes
                .inputs
                .iter()
                .position(|candidate| {
                    candidate.type_id() == input.type_id() && candidate.prefix() == input.prefix()
                })
                .ok_or(ModelError::Conflict("S0 scoped declaration"))?;
            let selected = if input.type_id() == std::any::TypeId::of::<artifact::ArtifactChunk>() {
                scopes.chunks(scope, phase)?
            } else {
                scope.select(table)?
            };
            crate::consumed_rows::stream_query_at(
                &permit,
                &input,
                access,
                scope.session(),
                &selected,
                |_, batch| {
                    data.visit_input(&input, batch)?;
                    Ok(())
                },
            )
            .await?;
        }
        Ok(())
    })
}
impl SynthesisScopes {
    async fn prepare(
        access: &CompletedInputs,
        model: &ValidatedModel,
        session: &datafusion::prelude::SessionContext,
        parents: &[synthesis::frames::Parents],
        budget: &resources::ResourceBudget,
    ) -> Result<Self, ModelError> {
        let inputs = Data::inputs(access.profile());
        let tables = inputs
            .iter()
            .map(|input| {
                Ok(crate::consumed_rows::ClosureTable {
                    relation: model
                        .relation(input.name())
                        .ok_or(ModelError::Schema("S0 scope model relation"))?
                        .clone(),
                    alias: access.table_for(input)?,
                })
            })
            .collect::<Result<Vec<_>, ModelError>>()?;
        Self::prepare_bound(inputs, tables, session, Some(parents), budget).await
    }
    async fn prepare_bound(
        inputs: Vec<ValidationInput>,
        tables: Vec<crate::consumed_rows::ClosureTable>,
        session: &datafusion::prelude::SessionContext,
        parents: Option<&[synthesis::frames::Parents]>,
        budget: &resources::ResourceBudget,
    ) -> Result<Self, ModelError> {
        use crate::consumed_rows::{NominalClosure, identifier};
        use std::any::TypeId;
        let index = |kind| {
            inputs
                .iter()
                .position(|input| input.type_id() == kind)
                .ok_or(ModelError::Schema("S0 scoped owner absent"))
        };
        let link = typed::<catalog::CatalogMemberInvocation>(&inputs)?;
        let core = typed::<analysis::catalog_core::Invocation>(&inputs)?;
        let occurrence = typed::<source::Occurrence>(&inputs)?;
        let exposure = typed::<catalog::CatalogExposure>(&inputs)?;
        let public = typed::<normalized::entities::PublicExposure>(&inputs)?;
        let mut bindings = tables.clone();
        let member = bindings.len();
        bindings.push(tables[link].clone());
        let syntax = bindings.len();
        bindings.push(tables[occurrence].clone());
        let child = bindings.len();
        bindings.push(tables[typed::<syntax::SyntaxPlacement>(&inputs)?].clone());
        let mut plan = NominalClosure::new(bindings)?;
        let memberships = synthesis::production::memberships();
        for (from, table) in tables.iter().enumerate() {
            for field in table.relation.fields() {
                let Some((kind, _)) = field.target() else {
                    continue;
                };
                let Some(to) = scope_target(&inputs, from, kind)? else {
                    continue;
                };
                let values = if field.list() {
                    format!("UNNEST({})", identifier(field.name()))
                } else {
                    identifier(field.name())
                };
                plan.pairs(
                    from,
                    to,
                    format!(
                        "SELECT id AS source_id,{values} AS target_id FROM {}",
                        identifier(&table.alias)
                    ),
                )?;
                if memberships.contains(&(table.relation.type_id(), field.name()))
                    || table.relation.type_id()
                        == TypeId::of::<analysis::native::NativeAssertionPremise>()
                    || table.relation.type_id() == TypeId::of::<analysis::summary::SupportSource>()
                {
                    plan.pairs(
                        to,
                        from,
                        format!(
                            "SELECT {values} AS source_id,id AS target_id FROM {}",
                            identifier(&table.alias)
                        ),
                    )?;
                }
            }
        }
        let a = |table: usize| identifier(&tables[table].alias);
        let selected = |column: &str, field: fn(&synthesis::frames::Parents) -> [u8; 16]| {
            parents.map_or("TRUE".into(), |parents| {
                if parents.is_empty() {
                    "FALSE".into()
                } else {
                    format!(
                        "{column} IN ({})",
                        parents
                            .iter()
                            .map(|parent| format!(
                                "X'{}'",
                                field(parent)
                                    .iter()
                                    .map(|byte| format!("{byte:02x}"))
                                    .collect::<String>()
                            ))
                            .collect::<Vec<_>>()
                            .join(",")
                    )
                }
            })
        };
        let selected_public_frames = selected("p.frame", |parent| *parent.structural.bytes());
        let selected_structural = selected("r.frame", |parent| *parent.structural.bytes());
        let selected_analytic = selected("r.frame", |parent| *parent.analytic.bytes());
        let selected_summary = selected("r.invocation", |parent| *parent.summary.bytes());
        let selected_terminal = selected("w.invocation", |parent| *parent.summary.bytes());

        plan.pairs(
            member,
            link,
            format!("SELECT id AS source_id,id AS target_id FROM {}", a(link)),
        )?;
        plan.pairs(member,exposure,format!("SELECT l.id AS source_id,e.id AS target_id FROM {} l JOIN {} i ON l.invocation=i.id JOIN {} e ON e.member=l.member JOIN {} p ON e.exposure=p.id WHERE p.context=i.context",a(link),a(core),a(exposure),a(public)))?;
        let option = typed::<catalog::CatalogOption>(&inputs)?;
        plan.pairs(member,option,format!("SELECT l.id AS source_id,o.id AS target_id FROM {} l JOIN {} o ON o.member=l.member",a(link),a(option)))?;
        let association = typed::<catalog::evidence::DocumentAssociation>(&inputs)?;
        let mention_candidate = typed::<normalized::links::MentionEntityCandidate>(&inputs)?;
        let assessment = typed::<normalized::links::MentionEntityAssessment>(&inputs)?;
        let mention = typed::<documents::DocumentMentionObservation>(&inputs)?;
        let q = scope_target(
            &inputs,
            mention,
            TypeId::of::<assertion::AssertionQualification>(),
        )?
        .ok_or(ModelError::Schema("S0 mention qualification"))?;
        plan.pairs(member,association,format!("SELECT l.id AS source_id,d.id AS target_id FROM {} l JOIN {} i ON l.invocation=i.id JOIN {} d ON d.member=l.member JOIN {} c ON d.candidate=c.id JOIN {} r ON c.assessment=r.id JOIN {} o ON r.observation=o.id JOIN {} q ON o.qualification=q.id WHERE q.context=i.context",a(link),a(core),a(association),a(mention_candidate),a(assessment),a(mention),a(q)))?;
        // A source subtree is a separate namespace. Ordinary ancestor/declaration references
        // never open the complete function or module body.
        let placement = typed::<syntax::SyntaxPlacement>(&inputs)?;
        plan.pairs(
            occurrence,
            placement,
            format!(
                "SELECT occurrence AS source_id,id AS target_id FROM {}",
                a(placement)
            ),
        )?;
        plan.pairs(occurrence,syntax,format!("SELECT id AS source_id,id AS target_id FROM {} WHERE syntax_kind IN ({},{},{},{},{},{})",a(occurrence),source::SyntaxKind::StmtExpr as i16,source::SyntaxKind::StmtAssign as i16,source::SyntaxKind::StmtAnnAssign as i16,source::SyntaxKind::StmtImport as i16,source::SyntaxKind::StmtImportFrom as i16,source::SyntaxKind::StmtReturn as i16))?;
        plan.pairs(
            syntax,
            occurrence,
            format!(
                "SELECT id AS source_id,id AS target_id FROM {}",
                a(occurrence)
            ),
        )?;
        plan.pairs(
            syntax,
            child,
            format!(
                "SELECT parent AS source_id,id AS target_id FROM {} WHERE parent IS NOT NULL",
                a(placement)
            ),
        )?;
        plan.pairs(
            child,
            placement,
            format!(
                "SELECT id AS source_id,id AS target_id FROM {}",
                a(placement)
            ),
        )?;
        plan.pairs(
            child,
            syntax,
            format!(
                "SELECT id AS source_id,occurrence AS target_id FROM {}",
                a(placement)
            ),
        )?;
        let reference = typed::<lexical::ReferenceObservation>(&inputs)?;
        let resolution = typed::<lexical::LexicalResolution>(&inputs)?;
        plan.pairs(
            occurrence,
            reference,
            format!(
                "SELECT read AS source_id,id AS target_id FROM {}",
                a(reference)
            ),
        )?;
        plan.pairs(
            occurrence,
            resolution,
            format!(
                "SELECT read AS source_id,id AS target_id FROM {}",
                a(resolution)
            ),
        )?;
        let candidate = typed::<catalog::CatalogCandidate>(&inputs)?;
        let path = typed::<catalog::CatalogPath>(&inputs)?;
        let alias = typed::<catalog::CatalogAlias>(&inputs)?;
        let entity_candidate = typed::<normalized::entities::SymbolEntityCandidate>(&inputs)?;
        // Complete compact entity correspondence includes path and alias targets, not just the
        // optional candidate.entity column. Input/context is carried by the actual C0 owner.
        let entities = format!(
            "SELECT l.id AS member,i.input,i.context,p.entity FROM {} l JOIN {} i ON l.invocation=i.id JOIN {} e ON e.member=l.member JOIN {} x ON e.exposure=x.id JOIN {} c ON c.exposure=e.id JOIN {} p ON c.path=p.id WHERE x.context=i.context UNION ALL SELECT l.id AS member,i.input,i.context,t.entity FROM {} l JOIN {} i ON l.invocation=i.id JOIN {} e ON e.member=l.member JOIN {} x ON e.exposure=x.id JOIN {} t ON t.parent=e.id WHERE x.context=i.context UNION ALL SELECT l.id AS member,i.input,i.context,n.entity FROM {} l JOIN {} i ON l.invocation=i.id JOIN {} e ON e.member=l.member JOIN {} x ON e.exposure=x.id JOIN {} c ON c.exposure=e.id JOIN {} n ON c.entity=n.id WHERE x.context=i.context",
            a(link),
            a(core),
            a(exposure),
            a(public),
            a(candidate),
            a(path),
            a(link),
            a(core),
            a(exposure),
            a(public),
            a(alias),
            a(link),
            a(core),
            a(exposure),
            a(public),
            a(candidate),
            a(entity_candidate)
        );
        let structural_frame = typed::<structural::StructuralFrame>(&inputs)?;
        let structural_inv = typed::<analysis::structural::Invocation>(&inputs)?;
        let analytic_frame = typed::<analytics::AnalyticFrame>(&inputs)?;
        let selected_public = typed::<structural::PublicCandidate>(&inputs)?;
        plan.pairs(member,selected_public,format!("SELECT l.id AS source_id,p.id AS target_id FROM {} l JOIN {} i ON l.invocation=i.id JOIN {} p ON p.member=l.member JOIN {} f ON p.frame=f.id JOIN {} s ON f.invocation=s.id WHERE s.input=i.input AND s.context=i.context AND {selected_public_frames}",a(link),a(core),a(selected_public),a(structural_frame),a(structural_inv)))?;
        for kind in [
            TypeId::of::<structural::Conclusion>(),
            TypeId::of::<structural::handoffs::Group>(),
        ] {
            let to = index(kind)?;
            let entity = if kind == TypeId::of::<structural::Conclusion>() {
                "subject"
            } else {
                "seed"
            };
            plan.pairs(member,to,format!("SELECT m.member AS source_id,r.id AS target_id FROM ({entities}) m JOIN {} r ON r.{entity}=m.entity JOIN {} f ON r.frame=f.id JOIN {} i ON f.invocation=i.id WHERE i.input=m.input AND i.context=m.context AND {selected_structural}",a(to),a(structural_frame),a(structural_inv)))?;
        }
        let analytic_conclusion = typed::<analytics::Conclusion>(&inputs)?;
        plan.pairs(member,analytic_conclusion,format!("SELECT m.member AS source_id,r.id AS target_id FROM ({entities}) m JOIN {} r ON r.subject=m.entity JOIN {} f ON r.frame=f.id JOIN {} s ON f.structural=s.id JOIN {} i ON s.invocation=i.id WHERE i.input=m.input AND i.context=m.context AND {selected_analytic}",a(analytic_conclusion),a(analytic_frame),a(structural_frame),a(structural_inv)))?;
        let claims = typed::<execution::summary_consequences::SummaryClaim>(&inputs)?;
        let transfers = typed::<transfer::summary::TransferKey>(&inputs)?;
        let events = typed::<normalized::events::NormalizedCallEvent>(&inputs)?;
        let ownership = typed::<normalized::entities::OccurrenceOwnership>(&inputs)?;
        let symbolic = typed::<execution::summary_symbolic::SymbolicFieldAlternative>(&inputs)?;
        let claim_owners = format!(
            "SELECT id,NoNormalContinuation_owner AS entity FROM {} WHERE NoNormalContinuation_owner IS NOT NULL UNION ALL SELECT c.id,t.owner AS entity FROM {} c JOIN {} t ON c.FiniteAlternative_transfer=t.id UNION ALL SELECT c.id,o.entity FROM {} c JOIN {} e ON c.CallClosure_event=e.id JOIN {} o ON e.owner=o.id UNION ALL SELECT c.id,s.constructor AS entity FROM {} c JOIN {} s ON c.SymbolicFieldAssociation_alternative=s.id",
            a(claims),
            a(claims),
            a(transfers),
            a(claims),
            a(events),
            a(ownership),
            a(claims),
            a(symbolic)
        );
        let conclusion = typed::<execution::summary_consequences::ClaimConclusion>(&inputs)?;
        let subject = typed::<analysis::summary::ObligationSubject>(&inputs)?;
        let summary_inv = typed::<analysis::summary::Invocation>(&inputs)?;
        plan.pairs(member,conclusion,format!("SELECT m.member AS source_id,r.id AS target_id FROM ({entities}) m JOIN ({claim_owners}) c ON c.entity=m.entity JOIN {} s ON s.SummaryClaim_transfer=c.id JOIN {} r ON r.subject=s.id JOIN {} i ON r.invocation=i.id WHERE i.input=m.input AND i.context=m.context AND {selected_summary}",a(subject),a(conclusion),a(summary_inv)))?;
        let witness = typed::<execution::summary_terminal::SummaryTerminalWitness>(&inputs)?;
        let frontier =
            typed::<execution::protocol_interpretation::ConditionalTerminalFrontier>(&inputs)?;
        plan.pairs(member,witness,format!("SELECT m.member AS source_id,w.id AS target_id FROM ({entities}) m JOIN {} f ON f.owner=m.entity JOIN {} w ON w.frontier=f.id JOIN {} i ON w.invocation=i.id WHERE i.input=m.input AND i.context=m.context AND {selected_terminal}",a(frontier),a(witness),a(summary_inv)))?;
        let scenario = typed::<catalog::evidence::ScenarioAssociation>(&inputs)?;
        let q = scope_target(
            &inputs,
            scenario,
            TypeId::of::<assertion::AssertionQualification>(),
        )?
        .ok_or(ModelError::Schema("S0 scenario qualification"))?;
        plan.pairs(member,scenario,format!("SELECT l.id AS source_id,s.id AS target_id FROM {} l JOIN {} i ON l.invocation=i.id JOIN {} s ON s.member=l.member JOIN {} q ON s.qualification=q.id WHERE q.context=i.context",a(link),a(core),a(scenario),a(q)))?;
        if inputs
            .iter()
            .any(|input| input.type_id() == TypeId::of::<flow::FlowUseObservation>())
        {
            let observation = typed::<flow::FlowUseObservation>(&inputs)?;
            let qualification = scope_target(
                &inputs,
                observation,
                TypeId::of::<assertion::AssertionQualification>(),
            )?
            .ok_or(ModelError::Schema("S0 native flow qualification"))?;
            let coverage = typed::<attribution::ProviderCoverage>(&inputs)?;
            plan.pairs(observation,coverage,format!("SELECT o.id AS source_id,c.id AS target_id FROM {} o JOIN {} q ON o.qualification=q.id JOIN {} c ON c.scope=q.scope AND c.context=q.context WHERE c.family={}",a(observation),a(qualification),a(coverage),attribution::FactFamily::Flow as i16))?;
        }
        let edges = plan.prepare(session, budget).await?;
        Ok(Self {
            inputs,
            tables,
            edges,
            member,
        })
    }
    async fn load(
        &self,
        access: &CompletedInputs,
        scope: &crate::consumed_rows::PreparedClosure,
        phase: LoadPhase,
        budget: &resources::ResourceBudget,
    ) -> Result<Data, ModelError> {
        let declarations = match phase {
            LoadPhase::Documentary => synthesis::documentary::Data::facts_inputs(),
            LoadPhase::Member => self.inputs.clone(),
            LoadPhase::Conclusion => synthesis::production::conclusion_inputs(),
        };
        let mut consumed = crate::consumed_rows::ConsumedInputs::new(declarations, budget)?;
        let mut data = Data::new(budget);
        let mut loaders: Vec<ScopedLoader> = Vec::new();
        macro_rules! scoped {($($field:ident:$ty:ty,)*) => {$(loaders.push(load_scoped::<$ty>);)*};}
        decoder_inputs!(scoped);
        for loader in loaders {
            loader(self, access, scope, phase, &mut consumed, &mut data).await?;
        }
        consumed.finish(access.name())?;
        Ok(data)
    }
    fn chunks(
        &self,
        scope: &crate::consumed_rows::PreparedClosure,
        phase: LoadPhase,
    ) -> Result<String, ModelError> {
        use crate::consumed_rows::identifier;
        let chunks = typed::<artifact::ArtifactChunk>(&self.inputs)?;
        let occurrence = typed::<source::Occurrence>(&self.inputs)?;
        let evidence = typed::<assertion::Evidence>(&self.inputs)?;
        // The scope already selects actual documentary spans and authored statement syntax.
        // Module/function declarations are not text ranges requested by these renderers.
        let nodes = typed::<documents::DocumentNode>(&self.inputs)?;
        let occurrences = match phase {
            LoadPhase::Documentary => format!(
                "SELECT source,start,\"end\" FROM ({}) WHERE syntax_kind={}",
                scope.select(occurrence)?,
                source::SyntaxKind::ExprStringLiteral as i16
            ),
            _ => format!(
                "SELECT source,start,\"end\" FROM ({}) WHERE syntax_kind IN ({},{},{},{},{},{},{})",
                scope.select(occurrence)?,
                source::SyntaxKind::ExprStringLiteral as i16,
                source::SyntaxKind::StmtExpr as i16,
                source::SyntaxKind::StmtAssign as i16,
                source::SyntaxKind::StmtAnnAssign as i16,
                source::SyntaxKind::StmtImport as i16,
                source::SyntaxKind::StmtImportFrom as i16,
                source::SyntaxKind::StmtReturn as i16
            ),
        };
        let spans = match phase {
            LoadPhase::Documentary => format!(
                "SELECT e.sourcespan_source AS source,e.sourcespan_start AS start,e.sourcespan_end AS \"end\" FROM ({}) e LEFT SEMI JOIN ({}) n ON e.id=n.passage_span OR e.id=n.component_span WHERE e.sourcespan_source IS NOT NULL",
                scope.select(evidence)?,
                scope.select(nodes)?
            ),
            _ => format!(
                "SELECT sourcespan_source AS source,sourcespan_start AS start,sourcespan_end AS \"end\" FROM ({}) WHERE sourcespan_source IS NOT NULL",
                scope.select(evidence)?
            ),
        };
        Ok(format!(
            "SELECT c.* FROM {} c LEFT SEMI JOIN ({occurrences} UNION ALL {spans}) r ON c.artifact=r.source AND c.ordinal*{}<r.\"end\" AND (c.ordinal+1)*{}>r.start",
            identifier(&self.tables[chunks].alias),
            artifact::ARTIFACT_CHUNK_BYTES,
            artifact::ARTIFACT_CHUNK_BYTES
        ))
    }
    async fn has_emission(
        &self,
        scope: &crate::consumed_rows::PreparedClosure,
    ) -> Result<bool, ModelError> {
        use futures::TryStreamExt;
        for kind in synthesis::production::conclusion_roots()
            .into_iter()
            .chain([
                std::any::TypeId::of::<structural::handoffs::Group>(),
                std::any::TypeId::of::<execution::summary_terminal::SummaryTerminalWitness>(),
            ])
        {
            let table = self
                .inputs
                .iter()
                .position(|input| input.type_id() == kind)
                .ok_or(ModelError::Schema("S0 emission owner"))?;
            let mut stream = crate::sql::query(
                scope.session(),
                &format!("SELECT id FROM ({}) r LIMIT 1", scope.select(table)?),
            )
            .await
            .map_err(ModelError::codec)?
            .execute_stream()
            .await
            .map_err(ModelError::codec)?;
            while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
                if batch.num_rows() != 0 {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
}
fn nominal<T>(bytes: &[u8]) -> Result<Id<T>, ModelError> {
    serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
        _,
        serde::de::value::Error,
    >::new(bytes.iter().copied()))
    .map_err(ModelError::codec)
}

struct RankingTables<'a> {
    public: &'a str,
    result: &'a str,
    community: &'a str,
}
type RankingLoader = for<'a> fn(
    &'a CompletedInputs,
    &'a datafusion::prelude::SessionContext,
    &'a synthesis::frames::Parents,
    &'a RankingTables<'a>,
    &'a mut crate::consumed_rows::ConsumedInputs,
    &'a mut synthesis::automatic::Data,
) -> BoxFuture<'a, Result<(), ModelError>>;
fn load_ranking<'a, R: Record>(
    access: &'a CompletedInputs,
    session: &'a datafusion::prelude::SessionContext,
    parent: &'a synthesis::frames::Parents,
    tables: &'a RankingTables<'a>,
    consumed: &'a mut crate::consumed_rows::ConsumedInputs,
    automatic: &'a mut synthesis::automatic::Data,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        use crate::consumed_rows::identifier;
        while let Some((input, permit)) = consumed.next::<R>(access)? {
            let alias = identifier(&access.table_for(&input)?);
            let frame = parent.structural.hex();
            let analytic = parent.analytic.hex();
            let selected = match input.type_id() {
                kind if kind == std::any::TypeId::of::<structural::UsageScore>() => format!(
                    "SELECT r.* FROM {alias} r LEFT SEMI JOIN {} p ON p.entity=r.target AND p.frame=r.frame WHERE r.frame=X'{frame}'",
                    identifier(tables.public)
                ),
                kind if kind == std::any::TypeId::of::<analytics::RankScore>() => format!(
                    "SELECT r.* FROM {alias} r JOIN {} t ON r.result=t.id LEFT SEMI JOIN {} p ON p.entity=r.target AND p.frame=X'{frame}' WHERE t.frame=X'{analytic}'",
                    identifier(tables.result),
                    identifier(tables.public)
                ),
                kind if kind == std::any::TypeId::of::<analytics::Community>() => format!(
                    "SELECT r.* FROM {alias} r JOIN {} t ON r.result=t.id WHERE t.frame=X'{analytic}'",
                    identifier(tables.result)
                ),
                kind if kind == std::any::TypeId::of::<analytics::CommunityMember>() => format!(
                    "SELECT r.* FROM {alias} r JOIN {} c ON r.community=c.id JOIN {} t ON c.result=t.id LEFT SEMI JOIN {} p ON p.entity=r.entity AND p.frame=X'{frame}' WHERE t.frame=X'{analytic}'",
                    identifier(tables.community),
                    identifier(tables.result),
                    identifier(tables.public)
                ),
                _ => {
                    return Err(ModelError::Schema(
                        "S0 automatic input property declaration",
                    ));
                }
            };
            crate::consumed_rows::stream_query_at(
                &permit,
                &input,
                access,
                session,
                &selected,
                |_, batch| {
                    automatic.visit(input.name(), batch)?;
                    Ok(())
                },
            )
            .await?;
        }
        Ok(())
    })
}
async fn ranking(
    access: &CompletedInputs,
    session: &datafusion::prelude::SessionContext,
    parent: &synthesis::frames::Parents,
    budget: &resources::ResourceBudget,
) -> Result<(synthesis::seeds::PublicSlots, synthesis::automatic::Data), ModelError> {
    use crate::consumed_rows::identifier;
    use arrow_array::Array;
    use futures::TryStreamExt;
    let public = access.table_for(&ValidationInput::of::<structural::PublicCandidate>(&["id"]))?;
    let mut slots = synthesis::seeds::PublicSlots::new(budget);
    let mut stream = crate::sql::query(
        session,
        &format!(
            "SELECT id,frame,member,entity,in_subsystem FROM {} WHERE frame=X'{}' ORDER BY id",
            identifier(&public),
            parent.structural.hex()
        ),
    )
    .await
    .map_err(ModelError::codec)?
    .execute_stream()
    .await
    .map_err(ModelError::codec)?;
    while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
        let ids = |name| {
            batch
                .column_by_name(name)
                .and_then(|column| {
                    column
                        .as_any()
                        .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
                })
                .filter(|column| column.null_count() == 0)
                .ok_or(ModelError::Schema("S0 public slot projection"))
        };
        let (id, frame, member, entity) =
            (ids("id")?, ids("frame")?, ids("member")?, ids("entity")?);
        let within = batch
            .column_by_name("in_subsystem")
            .and_then(|column| column.as_any().downcast_ref::<arrow_array::BooleanArray>())
            .filter(|column| column.null_count() == 0)
            .ok_or(ModelError::Schema("S0 public slot scope property"))?;
        for index in 0..batch.num_rows() {
            slots.observe(synthesis::seeds::PublicSlot {
                id: nominal(id.value(index))?,
                frame: nominal(frame.value(index))?,
                member: nominal(member.value(index))?,
                entity: nominal(entity.value(index))?,
                in_subsystem: within.value(index),
            })?;
        }
    }
    drop(stream);
    let mut automatic = synthesis::automatic::Data::new(budget);
    let result = access.table_for(&ValidationInput::of::<analytics::TechniqueResult>(&["id"]))?;
    let community = access.table_for(&ValidationInput::of::<analytics::Community>(&["id"]))?;
    let mut consumed =
        crate::consumed_rows::ConsumedInputs::new(synthesis::automatic::Data::inputs(), budget)?;
    macro_rules! read {($($field:ident:$ty:ty,)*) => {const LOADERS: &[RankingLoader] = &[$(load_ranking::<$ty>,)*];};}
    lctx_model::synthesis_automatic_unique_inputs!(read);
    let tables = RankingTables {
        public: &public,
        result: &result,
        community: &community,
    };
    for loader in LOADERS {
        loader(
            access,
            session,
            parent,
            &tables,
            &mut consumed,
            &mut automatic,
        )
        .await?;
    }
    consumed.finish(access.name())?;
    Ok((slots, automatic))
}

fn declare_outputs(output: &ProducerOutput) -> BoxFuture<'_, Result<(), ModelError>> {
    Box::pin(async move {
        let mut declarations: Vec<Declaration> = Vec::new();
        macro_rules! common_publication {($($record:ident,)*)=>{$(declarations.push(producer_operations::declare::<analysis::synthesis::$record>);)*};}
        lctx_model::analysis_publication!(common_publication);
        macro_rules! declare {($($ty:ty),*)=>{$(declarations.push(producer_operations::declare::<$ty>);)*};}
        declare!(
            synthesis::seeds::SeedPlan,
            synthesis::seeds::ConfiguredSeedDecision,
            synthesis::seeds::ConfiguredSeedCandidate,
            synthesis::seeds::SelectedSeedSource,
            synthesis::seeds::SelectedSeed,
            synthesis::automatic::Decision,
            synthesis::summary::SummaryFacet,
            synthesis::frames::Frame,
            synthesis::assertions::ProgrammaticAssertion,
            synthesis::assertions::AssertionTemplate,
            synthesis::assertions::AssertionSource,
            synthesis::assertions::ProgrammaticAssertionSupport,
            synthesis::briefs::Brief,
            synthesis::briefs::BriefAssertion,
            synthesis::briefs::BriefSource,
            synthesis::briefs::BriefSummary,
            synthesis::briefs::BriefCodeBoundary,
            synthesis::briefs::BriefDocument,
            synthesis::briefs::BriefOmission
        );
        macro_rules! declare_rows {($($field:ident:$ty:ty,)*)=>{$(if <$ty>::NAME!=assertion::AssertionQualification::NAME{declarations.push(producer_operations::declare::<$ty>);})*};}
        lctx_model::synthesis_pattern_outputs!(declare_rows);
        lctx_model::synthesis_observation_outputs!(declare_rows);
        producer_operations::declare_ordered(output, &declarations).await
    })
}
fn publish_seed<'a>(
    rows: &'a synthesis::seeds::Output,
    output: &'a ProducerOutput,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        type Emit = for<'a> fn(
            &'a synthesis::seeds::Output,
            &'a ProducerOutput,
        ) -> BoxFuture<'a, Result<(), ModelError>>;
        macro_rules! entries {($($field:ident),*) => {const EMITTERS: &[Emit] = &[$(|rows, output| producer_operations::emit(&rows.$field, output),)*];};}
        entries!(plans, automatic, decisions, candidates, sources, selected);
        for emit in EMITTERS {
            emit(rows, output).await?;
        }
        Ok(())
    })
}
fn publish_assertions<'a>(
    rows: &'a synthesis::assertions::Output,
    output: &'a ProducerOutput,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        type Emit = for<'a> fn(
            &'a synthesis::assertions::Output,
            &'a ProducerOutput,
        ) -> BoxFuture<'a, Result<(), ModelError>>;
        macro_rules! entries {($($field:ident),*) => {const EMITTERS: &[Emit] = &[$(|rows, output| producer_operations::emit(&rows.$field, output),)*];};}
        entries!(assertions, templates, sources, supports);
        for emit in EMITTERS {
            emit(rows, output).await?;
        }
        Ok(())
    })
}
fn publish_briefs<'a>(
    rows: &'a synthesis::briefs::Output,
    output: &'a ProducerOutput,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        type Emit = for<'a> fn(
            &'a synthesis::briefs::Output,
            &'a ProducerOutput,
        ) -> BoxFuture<'a, Result<(), ModelError>>;
        macro_rules! entries {($($field:ident),*) => {const EMITTERS: &[Emit] = &[$(|rows, output| producer_operations::emit(&rows.$field, output),)*];};}
        entries!(
            briefs,
            assertions,
            sources,
            summary,
            code_boundaries,
            documents,
            omissions
        );
        for emit in EMITTERS {
            emit(rows, output).await?;
        }
        Ok(())
    })
}
fn publish_patterns<'a>(
    rows: &'a synthesis::patterns::Output,
    output: &'a ProducerOutput,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        type Emit = for<'a> fn(
            &'a synthesis::patterns::Output,
            &'a ProducerOutput,
        ) -> BoxFuture<'a, Result<(), ModelError>>;
        macro_rules! entries {($($field:ident:$ty:ty,)*) => {const EMITTERS: &[Emit] = &[$(|rows, output| producer_operations::emit(&rows.$field, output),)*];};}
        lctx_model::synthesis_pattern_outputs!(entries);
        for emit in EMITTERS {
            emit(rows, output).await?;
        }
        Ok(())
    })
}
fn publish_observations<'a>(
    rows: &'a synthesis::observations::Output,
    output: &'a ProducerOutput,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        type Emit = for<'a> fn(
            &'a synthesis::observations::Output,
            &'a ProducerOutput,
        ) -> BoxFuture<'a, Result<(), ModelError>>;
        macro_rules! entries {($($field:ident:$ty:ty,)*) => {const EMITTERS: &[Emit] = &[$(|rows, output| producer_operations::emit(&rows.$field, output),)*];};}
        lctx_model::synthesis_observation_outputs!(entries);
        for emit in EMITTERS {
            emit(rows, output).await?;
        }
        Ok(())
    })
}
fn publish_frames<'a>(
    frames: &'a Rows<synthesis::frames::Frame>,
    sources: &'a Rows<InvocationSource>,
    inputs: &'a Rows<AnalysisInput>,
    receipts: &'a Rows<SourceReceipt>,
    output: &'a ProducerOutput,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        producer_operations::emit(frames, output).await?;
        producer_operations::emit(sources, output).await?;
        producer_operations::emit(inputs, output).await?;
        producer_operations::emit(receipts, output).await
    })
}
fn publish_coverage<'a, 'sources>(
    invocations: &'a Rows<Invocation>,
    definition: &'a analysis::AnalysisDefinition,
    admission: &'a analysis::expected::CoverageAdmission<'sources>,
    budget: &'a resources::ResourceBudget,
    output: &'a ProducerOutput,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        for invocation in invocations.iter() {
            let admitted = coverage::admit(
                invocation,
                definition,
                analysis::AnalysisCapability::Synthesis,
                admission,
                budget,
            )?;
            for scope in admitted.scopes() {
                let (requirement, members) = scope.expectation().records()?;
                output.push(requirement).await?;
                for row in members {
                    output.push(row).await?;
                }
                for row in scope.observations() {
                    output.push(row.source().clone()).await?;
                }
                let (coverage, members) = coverage::assess(
                    scope.expectation(),
                    scope.observations(),
                    analysis::AnalysisStatus::Completed,
                    None,
                    budget,
                )?;
                output.push(coverage).await?;
                for row in members {
                    output.push(row).await?;
                }
            }
            output
                .push(AnalysisOutcome {
                    invocation: invocation.id(),
                    status: analysis::AnalysisStatus::Completed,
                    reason: None,
                })
                .await?;
            output.push(invocation.clone()).await?;
        }
        Ok(())
    })
}
pub async fn produce(
    access: CompletedInputs,
    mut output: ProducerOutput,
    runtime: &Workspace,
    model: &Arc<ValidatedModel>,
) -> Result<(), ModelError> {
    let captured = analysis::sources::CapturedSources::capture(
        access.profile(),
        access.snapshots(),
        runtime.budget(),
    )?;
    let mut admission = analysis::expected::CoverageAdmission::new(&captured, runtime.budget())?;
    let mut data = Data::new(runtime.budget());
    let session = access.session(runtime).await?;
    let topology = synthesis::production::topology_inputs();
    let mut consumed = crate::consumed_rows::ConsumedInputs::new(
        {
            let mut inputs = topology.clone();
            inputs.extend(analysis::expected::inputs(
                analysis::AnalysisMethod::Synthesis,
            ));
            inputs
        },
        runtime.budget(),
    )?;
    read_metadata(
        &access,
        &session,
        &topology,
        &mut consumed,
        &mut admission,
        &mut data,
    )
    .await?;
    consumed.finish(access.name())?;
    let (_, definition) = synthesis::build::definition();
    let settings = data.frames.configuration()?;
    let parents = synthesis::frames::parents(&data.frames, runtime.budget())?;
    declare_outputs(&output).await?;
    let scopes =
        SynthesisScopes::prepare(&access, model, &session, &parents, runtime.budget()).await?;
    let mut documentary_spool = crate::documentary_spool::DocumentarySpool::new(runtime.budget())?;
    let mut requests = synthesis::seeds::ConfiguredRequests::new(runtime.budget());
    let mut summaries = synthesis::documentary::LiteralSummaries::new(runtime.budget());
    let mut emission = charged::ChargedSet::default();
    let mut emission_charge =
        charged::StateCharge::new(runtime.budget(), "synthesis-emission-member-identities");
    // First documentary pass publishes every boundary/conclusion and retains only exact matches,
    // literal identities and members needing later text. No upstream producer is replayed.
    synthesis_preparation::publish_documentary(
        &mut output,
        &synthesis::documentary::Output::new(runtime.budget()),
    )
    .await?;
    let links = access.table_for(&ValidationInput::of::<catalog::CatalogMemberInvocation>(&[
        "id",
    ]))?;
    let mut roots = crate::sql::query(
        &session,
        &format!(
            "SELECT id FROM {} ORDER BY id",
            crate::consumed_rows::identifier(&links)
        ),
    )
    .await
    .map_err(ModelError::codec)?
    .execute_stream()
    .await
    .map_err(ModelError::codec)?;
    use futures::TryStreamExt;
    while let Some(batch) = roots.try_next().await.map_err(ModelError::codec)? {
        let ids = batch
            .column_by_name("id")
            .and_then(|column| {
                column
                    .as_any()
                    .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
            })
            .ok_or(ModelError::Schema("S0 member root identity"))?;
        for index in 0..batch.num_rows() {
            runtime.cancellation().check()?;
            let id: Id<catalog::CatalogMemberInvocation> = nominal(ids.value(index))?;
            let scope = scopes
                .edges
                .grain(
                    scopes.member,
                    &format!("id=X'{}'", id.hex()),
                    runtime.budget(),
                )
                .await?;
            let grain = scopes
                .load(&access, &scope, LoadPhase::Documentary, runtime.budget())
                .await?;
            let docs = synthesis::documentary::build(&grain.documentary, runtime.budget())?;
            requests.observe(&grain.documentary, settings, runtime.budget())?;
            summaries.observe(&docs)?;
            documentary_spool.append(id, &docs)?;
            if docs
                .conclusions
                .iter()
                .any(|row| row.status() == analysis::policy::EvidenceStatus::Documented)
                || scopes.has_emission(&scope).await?
            {
                emission.insert(&mut emission_charge, id)?;
            }
            synthesis_preparation::publish_documentary_grain(&output, &docs).await?;
            drop(docs);
            drop(grain);
            drop(scope);
        }
    }
    drop(roots);
    let mut invocations = Rows::new(runtime.budget());
    let mut frames = Rows::new(runtime.budget());
    let mut sources = Rows::new(runtime.budget());
    let mut inputs = Rows::new(runtime.budget());
    let mut receipts = Rows::new(runtime.budget());
    let mut seeds = synthesis::seeds::Output::new(runtime.budget());
    for parent in &parents {
        let mut source_ids = charged::ChargedSet::default();
        let mut charge = charged::StateCharge::new(runtime.budget(), "synthesis-parent-identities");
        for source in &parent.sources {
            source_ids.insert(&mut charge, sources.insert(source.clone())?)?;
        }
        let (invocation, members, source_receipts, projections) = Invocation::admitted(
            parent.input,
            parent.context,
            definition.id(),
            None,
            source_ids.iter().copied(),
            &captured,
            [],
            runtime.budget(),
        )?;
        if !projections.is_empty() {
            return Err(ModelError::Invalid("S0 has no projection parameter".into()));
        }
        for row in members {
            inputs.insert(row)?;
        }
        for row in source_receipts {
            receipts.insert(row)?;
        }
        frames.insert(synthesis::frames::frame(parent, invocation.id()))?;
        let (public, automatic) = ranking(&access, &session, parent, runtime.budget()).await?;
        let mut selected = synthesis::seeds::configured_indexed(
            &requests,
            &public,
            &data.frames.structural,
            &data.frames.structural_invocations,
            settings,
            &invocation,
            runtime.budget(),
        )?;
        synthesis::automatic::complete_indexed(
            &data.documentary,
            &summaries,
            &public,
            &data.frames.structural,
            &data.frames.structural_invocations,
            &automatic,
            &data.frames.analytic_parents,
            settings,
            &invocation,
            &mut selected,
            runtime.budget(),
        )?;
        // Every candidate and ranking decision is emitted; only selected navigation identities
        // and their plan survive until the member render pass.
        publish_seed(&selected, &output).await?;
        for row in selected.plans.iter() {
            seeds.plans.insert(row.clone())?;
        }
        for row in selected.selected.iter() {
            emission.insert(&mut emission_charge, row.member)?;
            seeds.selected.insert(row.clone())?;
        }
        drop(selected);
        drop(automatic);
        drop(public);
        invocations.insert(invocation)?;
    }
    synthesis::frames::verify(
        &data.frames,
        &frames,
        &invocations,
        &sources,
        &inputs,
        runtime.budget(),
    )?;
    let mut coverages = Rows::new(runtime.budget());
    for invocation in invocations.iter() {
        let admitted = coverage::admit(
            invocation,
            &definition,
            analysis::AnalysisCapability::Synthesis,
            &admission,
            runtime.budget(),
        )?;
        for scope in admitted.scopes() {
            coverages.insert(
                coverage::assess(
                    scope.expectation(),
                    scope.observations(),
                    analysis::AnalysisStatus::Completed,
                    None,
                    runtime.budget(),
                )?
                .0,
            )?;
        }
    }
    // Second pass loads only members needed by mandatory assertions/code or selected briefs.
    // Compact correspondence remains shared for independent Summary roots below.
    for id in emission.iter().copied() {
        runtime.cancellation().check()?;
        let scope = scopes
            .edges
            .grain(
                scopes.member,
                &format!("id=X'{}'", id.hex()),
                runtime.budget(),
            )
            .await?;
        let grain = scopes
            .load(&access, &scope, LoadPhase::Member, runtime.budget())
            .await?;
        drop(scope);
        let docs = documentary_spool.read(id)?;
        let assertions = synthesis::assertions::build_all(
            &grain.documentary,
            &docs,
            &grain.observations,
            &grain.controls,
            &grain.summary,
            &grain.terminal,
            &grain.patterns,
            &grain.public,
            &frames,
            &invocations,
            runtime.budget(),
        )?;
        let (facets, _) = synthesis::summary::build(
            &grain.summary,
            &grain.documentary,
            &frames,
            &invocations,
            runtime.budget(),
        )?;
        let patterns = synthesis::patterns::build(
            &grain.patterns,
            &grain.documentary,
            &frames,
            &invocations,
            runtime.budget(),
        )?;
        let mut selected = synthesis::seeds::Output::new(runtime.budget());
        for row in seeds.selected.iter().filter(|row| row.member == id) {
            selected.selected.insert(row.clone())?;
            selected.plans.insert(
                seeds
                    .plans
                    .get(row.plan)
                    .ok_or(ModelError::Schema("S0 selected plan absent"))?
                    .clone(),
            )?;
        }
        let briefs = synthesis::briefs::build_with_summary(
            &grain.documentary,
            &docs,
            &assertions,
            &selected,
            &grain.summary,
            &grain.observations.qualifications,
            &facets,
            &frames,
            &patterns,
            runtime.budget(),
        )?;
        publish_assertions(&assertions, &output).await?;
        publish_briefs(&briefs, &output).await?;
        publish_patterns(&patterns, &output).await?;
        drop(briefs);
        drop(selected);
        drop(patterns);
        drop(facets);
        drop(assertions);
        drop(docs);
        drop(grain);
    }
    drop(documentary_spool);
    drop(emission);
    drop(emission_charge);
    drop(requests);
    drop(summaries);
    drop(seeds);
    // Independent conclusion roots keep unlinked consequences. Summary linkage uses complete
    // compact member/entity topology, so a local missing member never invents an unlinked facet.
    for frame in frames.iter() {
        for kind in synthesis::production::conclusion_roots() {
            let root = scopes
                .inputs
                .iter()
                .position(|input| input.type_id() == kind)
                .ok_or(ModelError::Schema("S0 conclusion root"))?;
            let predicate = if kind == std::any::TypeId::of::<structural::Conclusion>() {
                format!("frame=X'{}'", frame.structural.hex())
            } else if kind == std::any::TypeId::of::<analytics::Conclusion>() {
                format!("frame=X'{}'", frame.analytic.hex())
            } else {
                format!("invocation=X'{}'", frame.summary.hex())
            };
            let mut roots = crate::sql::query(
                &session,
                &format!(
                    "SELECT id FROM {} WHERE {predicate} ORDER BY id",
                    crate::consumed_rows::identifier(&scopes.tables[root].alias)
                ),
            )
            .await
            .map_err(ModelError::codec)?
            .execute_stream()
            .await
            .map_err(ModelError::codec)?;
            while let Some(batch) = roots.try_next().await.map_err(ModelError::codec)? {
                let ids = batch
                    .column_by_name("id")
                    .and_then(|column| {
                        column
                            .as_any()
                            .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
                    })
                    .ok_or(ModelError::Schema("S0 conclusion identity"))?;
                for index in 0..batch.num_rows() {
                    runtime.cancellation().check()?;
                    let key = ids
                        .value(index)
                        .iter()
                        .map(|byte| format!("{byte:02x}"))
                        .collect::<String>();
                    let scope = scopes
                        .edges
                        .grain(root, &format!("id=X'{key}'"), runtime.budget())
                        .await?;
                    let grain = scopes
                        .load(&access, &scope, LoadPhase::Conclusion, runtime.budget())
                        .await?;
                    drop(scope);
                    let observations = synthesis::observations::build_all(
                        &grain.observations,
                        &grain.summary,
                        &data.documentary,
                        &frames,
                        &invocations,
                        &coverages,
                        runtime.budget(),
                    )?;
                    let (facets, _) = synthesis::summary::build(
                        &grain.summary,
                        &data.documentary,
                        &frames,
                        &invocations,
                        runtime.budget(),
                    )?;
                    producer_operations::emit(&facets, &output).await?;
                    publish_observations(&observations, &output).await?;
                    drop(facets);
                    drop(observations);
                    drop(grain);
                }
            }
        }
    }
    publish_frames(&frames, &sources, &inputs, &receipts, &output).await?;
    publish_coverage(
        &invocations,
        &definition,
        &admission,
        runtime.budget(),
        &output,
    )
    .await?;
    drop(coverages);
    drop(scopes);
    drop(receipts);
    drop(inputs);
    drop(sources);
    drop(frames);
    drop(invocations);
    drop(parents);
    drop(data);
    drop(admission);
    drop(captured);
    output.finish(ProviderOutcome::Complete).await
}

#[cfg(test)]
mod decoder_tests {
    use super::*;
    #[test]
    fn declared_sources_have_decoder_reachability_in_both_profiles() {
        let mut decoders = std::collections::BTreeSet::new();
        macro_rules! collect {($($field:ident:$ty:ty,)*)=>{$(decoders.insert(std::any::TypeId::of::<$ty>());)*};}
        decoder_inputs!(collect);
        for profile in Profile::ALL {
            crate::consumed_rows::assert_decoder_reachability(
                Data::consumed_inputs(profile),
                &decoders,
            );
        }
    }
    fn id<T>(byte: u8) -> Id<T> {
        nominal(&[byte; 16]).unwrap()
    }
    fn fixture_data(
        b: &resources::ResourceBudget,
    ) -> (
        synthesis::documentary::Data,
        Id<catalog::CatalogMemberInvocation>,
    ) {
        let mut data = synthesis::documentary::Data::new(b);
        let text = "\"Run.\"";
        let artifact =
            source::SourceArtifact::from_bytes(id(1), "api.py".into(), text.as_bytes()).unwrap();
        data.artifacts.insert(artifact.clone()).unwrap();
        for row in artifact::ArtifactChunk::split(&artifact, text.as_bytes()).unwrap() {
            data.chunks.insert(row).unwrap();
        }
        let module = data
            .modules
            .insert(source::Module {
                source: artifact.id(),
                qualified_name: "pkg.api".into(),
            })
            .unwrap();
        let q = assertion::AssertionQualification {
            assumptions: assumptions::AssumptionSet::empty_id(),
            context: id(2),
            scope: source::CoverageScope::Artifact {
                artifact: artifact.id(),
            }
            .id(),
            condition: conditions::Diagram::always().id(),
            modality: attribution::Modality::Definite,
            approximation: assertion::Approximation::Exact,
        };
        data.qualifications.insert(q.clone()).unwrap();
        let mut occurrence = |kind, path| {
            data.occurrences
                .insert(source::Occurrence {
                    source: artifact.id(),
                    start: 0,
                    end: text.len() as i64,
                    syntax_kind: kind,
                    role: source::OccurrenceRole::Syntax,
                    structural_path: path,
                })
                .unwrap()
        };
        let declaration = occurrence(source::SyntaxKind::StmtFunctionDef, vec![]);
        let docstring = occurrence(source::SyntaxKind::StmtExpr, vec![0]);
        let literal_occurrence = occurrence(source::SyntaxKind::ExprStringLiteral, vec![0, 0]);
        let member = data
            .members
            .insert(catalog::CatalogMember {
                input: artifact.input,
                access: module,
                path: vec!["run".into()],
                name: "run".into(),
            })
            .unwrap();
        let core =
            analysis::catalog_core::Invocation::new(artifact.input, q.context, id(3), None, []).0;
        data.core_invocations.insert(core.clone()).unwrap();
        let frame = data
            .member_frames
            .insert(catalog::CatalogMemberInvocation {
                member,
                invocation: core.id(),
            })
            .unwrap();
        let public = data
            .public_exposures
            .insert(normalized::entities::PublicExposure {
                access: module,
                context: q.context,
                observation: id(4),
                origin: id(5),
                enumeration: None,
                publicity: normalized::entities::PublicPathKnowledge::Known,
                status: normalized::entities::ResolutionStatus::Unresolved,
                reason: normalized::entities::EntityReason::UntracedExposure,
            })
            .unwrap();
        let exposure = data
            .exposures
            .insert(catalog::CatalogExposure {
                member,
                exposure: public,
            })
            .unwrap();
        let callable = data
            .callables
            .insert(normalized::entities::CallableEntity::Source {
                declaration,
                kind: normalized::entities::CallableKind::Function,
            })
            .unwrap();
        let entity = data
            .refs
            .insert(normalized::entities::EntityRef::Callable { callable })
            .unwrap();
        let candidate = data
            .entity_candidates
            .insert(normalized::entities::SymbolEntityCandidate {
                resolution: id(6),
                entity,
            })
            .unwrap();
        data.candidates
            .insert(catalog::CatalogCandidate {
                exposure,
                candidate: Some(id(7)),
                entity: Some(candidate),
                path: None,
                alias: None,
            })
            .unwrap();
        let row = syntax::DeclarationObservation {
            qualification: q.id(),
            declaration,
            name: id(8),
            kind: syntax::DeclarationKind::Function,
            parent: None,
            overload: false,
            docstring: Some(docstring),
        };
        data.declarations.insert(row.clone()).unwrap();
        let placement = syntax::SyntaxPlacement {
            qualification: q.id(),
            occurrence: literal_occurrence,
            parent: Some(docstring),
            field: lexical::SyntaxField::Value,
            ordinal: 0,
        };
        data.placements.insert(placement.clone()).unwrap();
        let literal = data
            .literals
            .insert(value::Literal::String {
                value: "Run.".into(),
            })
            .unwrap();
        let detail = data
            .detail_values
            .insert(syntax::SyntaxDetail::Literal { literal })
            .unwrap();
        let detail = syntax::SyntaxDetailObservation {
            qualification: q.id(),
            occurrence: literal_occurrence,
            ordinal: 0,
            detail,
        };
        data.details.insert(detail.clone()).unwrap();
        for premise in [
            analysis::native::NativeAssertionPremise::DeclarationObservation {
                assertion: row.id(),
                support: id(9),
            },
            analysis::native::NativeAssertionPremise::SyntaxPlacement {
                assertion: placement.id(),
                support: id(10),
            },
            analysis::native::NativeAssertionPremise::SyntaxDetailObservation {
                assertion: detail.id(),
                support: id(11),
            },
        ] {
            data.native_qualifications
                .insert(analysis::native::NativeQualification {
                    premise: premise.id(),
                    qualification: q.id(),
                    family: premise.family(),
                    fidelity: attribution::Fidelity::NativeStructural,
                    status: analysis::policy::native_status(
                        premise.family(),
                        attribution::Fidelity::NativeStructural,
                    ),
                })
                .unwrap();
            data.native.insert(premise).unwrap();
        }
        (data, frame)
    }
    #[tokio::test]
    async fn member_documentary_grain_preserves_actual_oracle_and_excludes_unrelated_labels_and_chunks()
     {
        use crate::consumed_rows::ClosureTable;
        use datafusion::{datasource::MemTable, prelude::SessionContext};
        use futures::TryStreamExt;
        let budget = resources::ResourceBudget::fixed(8 << 20).unwrap();
        let model = lctx_model::domain::model().unwrap();
        let inputs = Data::inputs(Profile::Catalog);
        let session = SessionContext::new();
        let tables = inputs
            .iter()
            .enumerate()
            .map(|(index, input)| {
                let relation = model.relation(input.name()).unwrap().clone();
                let alias = format!("synthesis_fixture_{index}");
                let batch = arrow_array::RecordBatch::new_empty(relation.schema().clone());
                session
                    .register_table(
                        alias.as_str(),
                        Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap()),
                    )
                    .unwrap();
                ClosureTable { relation, alias }
            })
            .collect::<Vec<_>>();
        let finite_budget = resources::ResourceBudget::fixed(8 << 20).unwrap();
        let (data, member) = fixture_data(&finite_budget);
        let expected = synthesis::documentary::build(&data, &finite_budget).unwrap();
        assert_eq!(expected.conclusions.len(), 1);
        macro_rules! install {($($field:ident:$ty:ty,)*)=>{$(
            let input=synthesis::documentary::Data::facts_inputs().into_iter().find(|input|input.type_id()==std::any::TypeId::of::<$ty>()).unwrap();let index=inputs.iter().position(|candidate|candidate.type_id()==input.type_id()&&candidate.prefix()==input.prefix()).unwrap();
            let mut rows=data.$field.iter().cloned().collect::<Vec<_>>();
            let batch=<$ty as Record>::encode(&rows).unwrap();session.deregister_table(tables[index].alias.as_str()).unwrap();session.register_table(tables[index].alias.as_str(),Arc::new(MemTable::try_new(batch.schema(),vec![vec![batch]]).unwrap())).unwrap();rows.clear();
        )*};}
        lctx_model::synthesis_documentary_inputs!(install);
        let actual = data.members.iter().next().unwrap();
        let foreign = catalog::CatalogMember {
            path: vec!["x".repeat(16 << 20)],
            name: "y".repeat(16 << 20),
            ..actual.clone()
        };
        let index = typed::<catalog::CatalogMember>(&inputs).unwrap();
        let batch =
            <catalog::CatalogMember as Record>::encode(&[actual.clone(), foreign.clone()]).unwrap();
        session
            .deregister_table(tables[index].alias.as_str())
            .unwrap();
        session
            .register_table(
                tables[index].alias.as_str(),
                Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap()),
            )
            .unwrap();
        let source = data.artifacts.iter().next().unwrap();
        let foreign_source =
            source::SourceArtifact::from_bytes(source.input, "z".repeat(16 << 20), b"unrelated")
                .unwrap();
        let index = typed::<source::SourceArtifact>(&inputs).unwrap();
        let batch =
            <source::SourceArtifact as Record>::encode(&[source.clone(), foreign_source.clone()])
                .unwrap();
        session
            .deregister_table(tables[index].alias.as_str())
            .unwrap();
        session
            .register_table(
                tables[index].alias.as_str(),
                Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap()),
            )
            .unwrap();
        let index = typed::<artifact::ArtifactChunk>(&inputs).unwrap();
        let mut chunks = data.chunks.iter().cloned().collect::<Vec<_>>();
        chunks.push(artifact::ArtifactChunk {
            artifact: foreign_source.id(),
            ordinal: 0,
            body: EvidenceBytes(b"not a requested original".to_vec()),
        });
        let batch = <artifact::ArtifactChunk as Record>::encode(&chunks).unwrap();
        session
            .deregister_table(tables[index].alias.as_str())
            .unwrap();
        session
            .register_table(
                tables[index].alias.as_str(),
                Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap()),
            )
            .unwrap();
        let scopes = SynthesisScopes::prepare_bound(inputs, tables, &session, None, &budget)
            .await
            .unwrap();
        let scope = scopes
            .edges
            .grain(scopes.member, &format!("id=X'{}'", member.hex()), &budget)
            .await
            .unwrap();
        let mut selected = synthesis::documentary::Data::new(&budget);
        for input in synthesis::documentary::Data::facts_inputs() {
            let index = scopes
                .inputs
                .iter()
                .position(|candidate| {
                    candidate.type_id() == input.type_id() && candidate.prefix() == input.prefix()
                })
                .unwrap();
            let query = if input.type_id() == std::any::TypeId::of::<artifact::ArtifactChunk>() {
                scopes.chunks(&scope, LoadPhase::Documentary).unwrap()
            } else {
                scope.select(index).unwrap()
            };
            let mut stream = crate::sql::query(scope.session(), &query)
                .await
                .unwrap()
                .execute_stream()
                .await
                .unwrap();
            while let Some(batch) = stream.try_next().await.unwrap() {
                selected.visit(input.name(), &batch).unwrap();
            }
        }
        assert_eq!(selected.members.len(), 1);
        assert!(selected.members.get(foreign.id()).is_none());
        assert_eq!(selected.artifacts.len(), 1);
        assert!(selected.artifacts.get(foreign_source.id()).is_none());
        assert_eq!(selected.chunks.len(), 1);
        let actual = synthesis::documentary::build(&selected, &budget).unwrap();
        actual.matches(&expected).unwrap();
        drop(actual);
        drop(selected);
        drop(scope);
        drop(scopes);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn control_formal_grain_selects_named_parameter_links_without_unrelated_parameters() {
        use crate::consumed_rows::ClosureTable;
        use datafusion::{datasource::MemTable, prelude::SessionContext};
        use futures::TryStreamExt;
        use normalized::entities::{ParameterEntity, ParameterEntityLink};
        fn install<R: Record>(
            session: &SessionContext,
            tables: &[ClosureTable],
            inputs: &[ValidationInput],
            rows: &[R],
        ) {
            let index = inputs
                .iter()
                .position(|input| input.type_id() == std::any::TypeId::of::<R>())
                .unwrap();
            let batch = R::encode(rows).unwrap();
            session
                .deregister_table(tables[index].alias.as_str())
                .unwrap();
            session
                .register_table(
                    tables[index].alias.as_str(),
                    Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap()),
                )
                .unwrap();
        }
        let budget = resources::ResourceBudget::fixed(8 << 20).unwrap();
        let model = lctx_model::domain::model().unwrap();
        let inputs = Data::inputs(Profile::Catalog);
        let session = SessionContext::new();
        let tables = inputs
            .iter()
            .enumerate()
            .map(|(index, input)| {
                let relation = model.relation(input.name()).unwrap().clone();
                let alias = format!("control_parameter_fixture_{index}");
                let batch = arrow_array::RecordBatch::new_empty(relation.schema().clone());
                session
                    .register_table(
                        alias.as_str(),
                        Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap()),
                    )
                    .unwrap();
                ClosureTable { relation, alias }
            })
            .collect::<Vec<_>>();
        let formal = ParameterEntity::Source {
            declaration: nominal(&[1; 16]).unwrap(),
        };
        let unrelated = ParameterEntity::Source {
            declaration: nominal(&[2; 16]).unwrap(),
        };
        let shapes = [
            calls::ParameterShape {
                name: Some("flag".into()),
                kind: calls::ParameterKind::KeywordOnly,
                required: false,
            },
            calls::ParameterShape {
                name: Some("unrelated".into()),
                kind: calls::ParameterKind::KeywordOnly,
                required: false,
            },
        ];
        let parameters = [
            calls::SignatureParameter {
                signature: nominal(&[3; 16]).unwrap(),
                ordinal: 0,
                shape: shapes[0].id(),
            },
            calls::SignatureParameter {
                signature: nominal(&[4; 16]).unwrap(),
                ordinal: 0,
                shape: shapes[1].id(),
            },
        ];
        let links = [
            ParameterEntityLink {
                parameter: parameters[0].id(),
                entity: formal.id(),
                declaration: None,
            },
            ParameterEntityLink {
                parameter: parameters[1].id(),
                entity: unrelated.id(),
                declaration: None,
            },
        ];
        let path = structural::controls::ControlPath {
            traversal: nominal(&[5; 16]).unwrap(),
            target: nominal(&[6; 16]).unwrap(),
            formal: formal.id(),
            may_suppress: false,
            length: 0,
        };
        install(&session, &tables, &inputs, &[formal, unrelated]);
        install(&session, &tables, &inputs, &shapes);
        install(&session, &tables, &inputs, &parameters);
        install(&session, &tables, &inputs, &links);
        install(&session, &tables, &inputs, std::slice::from_ref(&path));
        let scopes = SynthesisScopes::prepare_bound(inputs, tables, &session, None, &budget)
            .await
            .unwrap();
        let root = typed::<structural::controls::ControlPath>(&scopes.inputs).unwrap();
        let scope = scopes
            .edges
            .grain(root, &format!("id=X'{}'", path.id().hex()), &budget)
            .await
            .unwrap();
        let mut selected_links = Rows::<ParameterEntityLink>::new(&budget);
        let mut selected_shapes = Rows::<calls::ParameterShape>::new(&budget);
        for (index, is_link) in [
            (typed::<ParameterEntityLink>(&scopes.inputs).unwrap(), true),
            (
                typed::<calls::ParameterShape>(&scopes.inputs).unwrap(),
                false,
            ),
        ] {
            let mut stream = crate::sql::query(scope.session(), &scope.select(index).unwrap())
                .await
                .unwrap()
                .execute_stream()
                .await
                .unwrap();
            while let Some(batch) = stream.try_next().await.unwrap() {
                if is_link {
                    selected_links.decode(&batch).unwrap();
                } else {
                    selected_shapes.decode(&batch).unwrap();
                }
            }
        }
        assert_eq!(selected_links.len(), 1);
        assert_eq!(selected_links.get(links[0].id()), Some(&links[0]));
        assert!(selected_links.get(links[1].id()).is_none());
        assert_eq!(selected_shapes.len(), 1);
        assert_eq!(selected_shapes.get(shapes[0].id()), Some(&shapes[0]));
        drop(selected_links);
        drop(selected_shapes);
        drop(scope);
        drop(scopes);
        assert_eq!(budget.reserved(), 0);
    }

    #[tokio::test]
    async fn expected_artifact_property_query_preserves_exact_extension_policy() {
        use datafusion::{datasource::MemTable, prelude::SessionContext};
        use futures::TryStreamExt;
        let session = SessionContext::new();
        let paths = [
            "x.py",
            "x.pyi",
            "x.md",
            "x.mdx",
            "x.rst",
            "x.PY",
            "no_extension",
            ".py",
            "..md",
            "Unicode_π.md",
            "wrong.py.long",
            "dot.py/folder",
        ];
        let mut rows = paths
            .into_iter()
            .map(|path| source::SourceArtifact::from_bytes(id(1), path.into(), b"x").unwrap())
            .collect::<Vec<_>>();
        rows.push(
            source::SourceArtifact::from_bytes(id(1), format!("{}.pyi", "λ".repeat(1 << 20)), b"x")
                .unwrap(),
        );
        let batch = <source::SourceArtifact as Record>::encode(&rows).unwrap();
        session
            .register_table(
                "captured_artifacts",
                Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap()),
            )
            .unwrap();
        let mut stream = crate::sql::query(
            &session,
            &format!(
                "SELECT {} FROM captured_artifacts ORDER BY id",
                analysis::expected::CoverageAdmission::artifact_property_columns()
            ),
        )
        .await
        .unwrap()
        .execute_stream()
        .await
        .unwrap();
        let mut checked = 0;
        while let Some(batch) = stream.try_next().await.unwrap() {
            let ids = batch
                .column_by_name("id")
                .unwrap()
                .as_any()
                .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
                .unwrap();
            let suffix = batch
                .column_by_name("path_suffix")
                .unwrap()
                .as_any()
                .downcast_ref::<arrow_array::StringArray>()
                .unwrap();
            for index in 0..batch.num_rows() {
                let id: Id<source::SourceArtifact> = nominal(ids.value(index)).unwrap();
                let original = rows.iter().find(|row| row.id() == id).unwrap();
                assert!(suffix.value(index).chars().count() <= 4);
                assert_eq!(
                    admission::ArtifactClass::of(suffix.value(index)),
                    admission::ArtifactClass::of(&original.path)
                );
                checked += 1;
            }
        }
        assert_eq!(checked, rows.len());
    }
}
