//! Four-family deterministic renderer and shared replay, over completed canonical C0/C1 evidence.
// Increment for a meaning/rule change; implementation source bytes live in producer provenance.
const SEMANTIC_RULE_REVISION: i64 = 2;
use super::*;
use crate::domain::{
    catalog::evidence::build::{EvidenceData, EvidenceOutput},
    normalized::Rows,
    resources::ResourceBudget,
    stages::*,
};
pub fn invalid(s: impl Into<String>) -> ModelError {
    ModelError::Invalid(s.into())
}
pub fn need<R: Record>(r: &Rows<R>, id: Id<R>) -> Result<&R, ModelError> {
    r.required(id, || {
        invalid(format!("retrieval premise absent: {}", R::NAME))
    })
}
pub struct Data {
    tokenizer: Option<std::sync::Arc<dyn super::partition::Tokenizer>>,
    pub source: EvidenceData,
    pub evidence: EvidenceOutput,
    pub facts: Facts,
    pub synthesis: SynthesisFacts,
    ruff_headers: charged::ChargedMap<
        Id<diagnostics::RuffDiagnosticObservation>,
        (diagnostics::DiagnosticChannel, ContentHash),
    >,
    pyrefly_headers: charged::ChargedMap<
        Id<diagnostics::PyreflyDiagnosticObservation>,
        diagnostics::DiagnosticChannel,
    >,
    diagnostic_charge: charged::StateCharge,
    completion_members: charged::ChargedMap<
        Id<catalog::CatalogMember>,
        (Id<input::InputRevision>, Id<crate::domain::source::Module>),
    >,
    completion_modules: charged::ChargedMap<
        Id<crate::domain::source::Module>,
        Id<crate::domain::source::SourceArtifact>,
    >,
    completion_briefs: charged::ChargedMap<
        Id<crate::domain::synthesis::briefs::Brief>,
        Id<crate::domain::synthesis::seeds::SelectedSeed>,
    >,
    completion_artifacts: charged::ChargedMap<
        Id<crate::domain::source::SourceArtifact>,
        (Id<input::InputRevision>, i64),
    >,
    completion_documents: charged::ChargedMap<
        Id<documents::DocumentObservation>,
        Id<crate::domain::source::SourceArtifact>,
    >,
    completion_charge: charged::StateCharge,
}
impl Data {
    pub fn new(b: &ResourceBudget) -> Self {
        Self {
            tokenizer: None,
            source: EvidenceData::new(b),
            evidence: EvidenceOutput::new(b),
            facts: Facts::new(b),
            synthesis: SynthesisFacts::new(b),
            ruff_headers: Default::default(),
            pyrefly_headers: Default::default(),
            diagnostic_charge: charged::StateCharge::new(b, "retrieval-diagnostic-properties"),
            completion_members: Default::default(),
            completion_modules: Default::default(),
            completion_briefs: Default::default(),
            completion_artifacts: Default::default(),
            completion_documents: Default::default(),
            completion_charge: charged::StateCharge::new(b, "retrieval-completion-properties"),
        }
    }
    pub fn set_tokenizer(&mut self, t: std::sync::Arc<dyn super::partition::Tokenizer>) {
        self.tokenizer = Some(t);
    }
    pub fn tokenizer(&self) -> Option<&std::sync::Arc<dyn super::partition::Tokenizer>> {
        self.tokenizer.as_ref()
    }
    pub fn visit(&mut self, n: &str, b: &arrow_array::RecordBatch) -> Result<bool, ModelError> {
        if self.visit_diagnostic(n, b)? {
            return Ok(true);
        }
        let source = self.source.visit(n, b)?;
        let evidence = self.evidence.visit(n, b)?;
        let facts = self.facts.visit(n, b)?;
        let synthesis = self.synthesis.visit(n, b)?;
        Ok(source || evidence || facts || synthesis)
    }
    pub fn visit_input(
        &mut self,
        input: &ValidationInput,
        b: &arrow_array::RecordBatch,
    ) -> Result<bool, ModelError> {
        let n = input.name();
        if self.visit_diagnostic(n, b)? {
            return Ok(true);
        }
        if !is_vocabulary(n) {
            return self.visit(n, b);
        }
        if input.prefix() == Some(PublicationBoundary::Facts) {
            let source = self.source.visit_input(input, b)?;
            let facts = self.facts.visit(n, b)?;
            return Ok(source || facts);
        }
        self.source.visit_input(input, b)
    }
    fn visit_diagnostic(
        &mut self,
        name: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<bool, ModelError> {
        use arrow_array::Array;
        use diagnostics::{
            DiagnosticChannel as C, PyreflyDiagnosticObservation as P,
            RuffDiagnosticObservation as R,
        };
        if name != R::NAME && name != P::NAME {
            return Ok(false);
        }
        let ids = batch
            .column_by_name("id")
            .and_then(|array| {
                array
                    .as_any()
                    .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
            })
            .filter(|array| array.value_length() == 16 && array.null_count() == 0)
            .ok_or(ModelError::Schema("retrieval diagnostic identity"))?;
        let channels = batch
            .column_by_name("channel")
            .and_then(|array| array.as_any().downcast_ref::<arrow_array::Int16Array>())
            .filter(|array| array.null_count() == 0)
            .ok_or(ModelError::Schema("retrieval diagnostic channel"))?;
        fn nominal<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, ModelError> {
            serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
                _,
                serde::de::value::Error,
            >::new(bytes.iter().copied()))
            .map_err(ModelError::codec)
        }
        let settings = if name == R::NAME {
            Some(
                batch
                    .column_by_name("settings")
                    .and_then(|array| {
                        array
                            .as_any()
                            .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
                    })
                    .filter(|array| array.value_length() == 32 && array.null_count() == 0)
                    .ok_or(ModelError::Schema("retrieval diagnostic settings"))?,
            )
        } else {
            None
        };
        for row in 0..batch.num_rows() {
            let channel = match channels.value(row) {
                0 => C::Emitted,
                1 => C::RuffNoqaSuppressed,
                2 => C::PyreflyDirective,
                3 => C::PyreflySuppressed,
                4 => C::PyreflyDisabled,
                5 => C::PyreflyBaseline,
                _ => return Err(ModelError::Schema("retrieval diagnostic channel code")),
            };
            if let Some(settings) = settings {
                let id = nominal(ids.value(row))?;
                let value = (channel, nominal(settings.value(row))?);
                if self.ruff_headers.get(&id).is_some_and(|old| *old != value) {
                    return Err(ModelError::Conflict(R::NAME));
                }
                self.ruff_headers
                    .insert(&mut self.diagnostic_charge, id, value)?;
            } else {
                let id = nominal(ids.value(row))?;
                if self
                    .pyrefly_headers
                    .get(&id)
                    .is_some_and(|old| *old != channel)
                {
                    return Err(ModelError::Conflict(P::NAME));
                }
                self.pyrefly_headers
                    .insert(&mut self.diagnostic_charge, id, channel)?;
            }
        }
        Ok(true)
    }
    fn ruff_header(
        &self,
        id: Id<diagnostics::RuffDiagnosticObservation>,
    ) -> Result<(diagnostics::DiagnosticChannel, ContentHash), ModelError> {
        if let Some(row) = self.ruff_headers.get(&id) {
            return Ok(*row);
        }
        let row = need(&self.source.facts.ruff_diagnostics, id)?;
        Ok((row.channel, row.settings))
    }
    fn pyrefly_channel(
        &self,
        id: Id<diagnostics::PyreflyDiagnosticObservation>,
    ) -> Result<diagnostics::DiagnosticChannel, ModelError> {
        if let Some(channel) = self.pyrefly_headers.get(&id) {
            return Ok(*channel);
        }
        Ok(need(&self.source.facts.pyrefly_diagnostics, id)?.channel)
    }
    /// Only ownership columns are retained by completed-unit admission. Rendering still uses
    /// the actual rich records from its own physically selected root.
    pub fn completion_visit(
        &mut self,
        input: &ValidationInput,
        batch: &arrow_array::RecordBatch,
    ) -> Result<bool, ModelError> {
        use arrow_array::Array;
        use std::any::TypeId;
        fn nominal<T: serde::de::DeserializeOwned>(
            batch: &arrow_array::RecordBatch,
            name: &str,
            row: usize,
        ) -> Result<T, ModelError> {
            let array = batch
                .column_by_name(name)
                .and_then(|array| {
                    array
                        .as_any()
                        .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
                })
                .ok_or(ModelError::Schema("retrieval ownership identity"))?;
            if array.is_null(row) {
                return Err(ModelError::Schema("retrieval null ownership identity"));
            }
            serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
                _,
                serde::de::value::Error,
            >::new(array.value(row).iter().copied()))
            .map_err(ModelError::codec)
        }
        let kind = input.type_id();
        for row in 0..batch.num_rows() {
            if kind == TypeId::of::<catalog::CatalogMember>() {
                self.completion_members.insert(
                    &mut self.completion_charge,
                    nominal(batch, "id", row)?,
                    (
                        nominal(batch, "input", row)?,
                        nominal(batch, "access", row)?,
                    ),
                )?;
            } else if kind == TypeId::of::<crate::domain::source::Module>() {
                self.completion_modules.insert(
                    &mut self.completion_charge,
                    nominal(batch, "id", row)?,
                    nominal(batch, "source", row)?,
                )?;
            } else if kind == TypeId::of::<crate::domain::synthesis::briefs::Brief>() {
                self.completion_briefs.insert(
                    &mut self.completion_charge,
                    nominal(batch, "id", row)?,
                    nominal(batch, "seed", row)?,
                )?;
            } else if kind == TypeId::of::<documents::DocumentObservation>() {
                self.completion_documents.insert(
                    &mut self.completion_charge,
                    nominal(batch, "id", row)?,
                    nominal(batch, "source", row)?,
                )?;
            } else if kind == TypeId::of::<crate::domain::source::SourceArtifact>() {
                let lengths = batch
                    .column_by_name("byte_len")
                    .and_then(|array| array.as_any().downcast_ref::<arrow_array::Int64Array>())
                    .ok_or(ModelError::Schema("retrieval original length"))?;
                if lengths.is_null(row) || lengths.value(row) < 0 {
                    return Err(ModelError::Schema("retrieval original length"));
                }
                self.completion_artifacts.insert(
                    &mut self.completion_charge,
                    nominal(batch, "id", row)?,
                    (nominal(batch, "input", row)?, lengths.value(row)),
                )?;
            } else {
                return self.visit_input(input, batch);
            }
        }
        Ok(true)
    }
    pub fn artifact_bounds(
        &self,
        id: Id<crate::domain::source::SourceArtifact>,
    ) -> Result<(Id<input::InputRevision>, i64), ModelError> {
        if let Some(bounds) = self.completion_artifacts.get(&id) {
            return Ok(*bounds);
        }
        let source = need(&self.source.core.artifacts, id)?;
        Ok((source.input, source.byte_len))
    }
    fn member_access(
        &self,
        id: Id<catalog::CatalogMember>,
    ) -> Result<(Id<input::InputRevision>, Id<crate::domain::source::Module>), ModelError> {
        if let Some(owner) = self.completion_members.get(&id) {
            return Ok(*owner);
        }
        let member = need(&self.source.catalog.members, id)?;
        Ok((member.input, member.access))
    }
    pub(super) fn module_source(
        &self,
        id: Id<crate::domain::source::Module>,
    ) -> Result<Id<crate::domain::source::SourceArtifact>, ModelError> {
        if let Some(source) = self.completion_modules.get(&id) {
            return Ok(*source);
        }
        Ok(need(&self.source.core.modules, id)?.source)
    }
    fn brief_seed(
        &self,
        id: Id<crate::domain::synthesis::briefs::Brief>,
    ) -> Result<Id<crate::domain::synthesis::seeds::SelectedSeed>, ModelError> {
        if let Some(seed) = self.completion_briefs.get(&id) {
            return Ok(*seed);
        }
        Ok(need(&self.synthesis.briefs, id)?.seed)
    }
    pub(super) fn document_source(
        &self,
        id: Id<documents::DocumentObservation>,
    ) -> Result<Id<crate::domain::source::SourceArtifact>, ModelError> {
        if let Some(source) = self.completion_documents.get(&id) {
            return Ok(*source);
        }
        Ok(need(&self.source.facts.documents, id)?.source)
    }
    pub(super) fn verify_origin(
        &self,
        unit: &Unit,
        origin: &Origin,
        root: &c1::EvidenceRoot,
    ) -> Result<(), ModelError> {
        use c1::RootSubject as S;
        let subject = need(&self.evidence.subjects, root.subject)?;
        let agrees = match (origin, subject) {
            (Origin::Api { member }, S::Member { member: owner }) => {
                unit.family == Family::ApiOptions
                    && member == owner
                    && self.member_access(*member)?.0 == unit.input
            }
            (Origin::Scenario { scenario }, S::Scenario { scenario: owner }) => {
                unit.family == Family::Scenario && scenario == owner
            }
            (Origin::Document { observation }, S::Document { observation: owner }) => {
                unit.family == Family::DocumentationDeployment
                    && observation == owner
                    && self.artifact_bounds(self.document_source(*observation)?)?.0 == unit.input
            }
            (Origin::Deployment { deployment }, S::Deployment { deployment: owner }) => {
                unit.family == Family::DocumentationDeployment && deployment == owner
            }
            (Origin::Passage { observation }, S::Document { observation: owner }) => {
                let passage = need(&self.source.facts.passages, *observation)?;
                let node = need(&self.source.facts.nodes, passage.passage.id())?;
                let source =
                    super::source::coordinates(self, &AnchorSource::Span { span: node.span() })?.0;
                unit.family == Family::DocumentationDeployment
                    && need(&self.source.core.qualifications, passage.qualification)?.context
                        == unit.context
                    && source == self.document_source(*owner)?
                    && self.artifact_bounds(source)?.0 == unit.input
            }
            (Origin::Definition { member, entity }, S::Member { member: owner }) => {
                member == owner
                    && unit.family == Family::Source
                    && super::construction::definitions(self, *member, unit.context)?
                        .iter()
                        .any(|(e, _, _)| e == entity)
            }
            (Origin::UnavailableDefinition { member }, S::Member { member: owner }) => {
                member == owner
                    && unit.family == Family::Source
                    && super::construction::definitions(self, *member, unit.context)?.is_empty()
            }
            (Origin::Source { artifact }, S::Source { artifact: owner }) => {
                artifact == owner
                    && unit.family == Family::Source
                    && self.artifact_bounds(*artifact)?.0 == unit.input
            }
            (Origin::Option { option }, S::Option { option: owner }) => {
                option == owner && unit.family == Family::ApiOptions
            }
            (Origin::Release { release }, S::Release { release: owner }) => {
                release == owner && unit.family == Family::DocumentationDeployment
            }
            (Origin::Brief { brief }, S::Member { member }) => {
                let seed = need(&self.synthesis.seeds, self.brief_seed(*brief)?)?;
                let plan = need(&self.synthesis.seed_plans, seed.plan)?;
                let invocation = need(&self.synthesis.synthesis_invocations, plan.invocation)?;
                let owner = need(&self.synthesis.member_frames, seed.member)?;
                let core = need(&self.source.facts.core_invocations, owner.invocation)?;
                unit.family == Family::ApiOptions
                    && owner.member == *member
                    && invocation.definition == crate::domain::synthesis::build::definition().1.id()
                    && invocation.input == unit.input
                    && invocation.context == unit.context
                    && core.input == invocation.input
                    && core.context == invocation.context
                    && self.member_access(*member)?.0 == unit.input
            }
            _ => false,
        };
        if !agrees {
            return Err(invalid(
                "retrieval unit origin differs from exact C1 subject owner",
            ));
        }
        Ok(())
    }
    pub fn consumed_inputs(_profile: Profile) -> Vec<ValidationInput> {
        Self::inputs()
    }
    /// Only actual renderer premises are decoded in a root grain. Other earlier C1 properties
    /// remain with their own necessary admission owner.
    pub fn root_inputs() -> Vec<ValidationInput> {
        crate::domain::normalized::facts_inputs(vec![
            ValidationInput::of::<input::Package>(&["id"]),
            ValidationInput::of::<input::Release>(&["id"]),
            ValidationInput::of::<crate::domain::analysis::catalog_core::Invocation>(&["id"]),
            ValidationInput::of::<crate::domain::analysis::native::NativeAssertionPremise>(&["id"]),
            ValidationInput::of::<crate::domain::artifact::ArtifactChunk>(&["id"]),
            ValidationInput::of::<crate::domain::assertion::AssertionQualification>(&["id"]),
            ValidationInput::of::<crate::domain::assertion::Evidence>(&["id"]),
            ValidationInput::of::<crate::domain::attribution::ProviderRun>(&["id"]),
            ValidationInput::of::<crate::domain::calls::ParameterShape>(&["id"]),
            ValidationInput::of::<crate::domain::calls::Signature>(&["id"]),
            ValidationInput::of::<crate::domain::calls::SignatureParameter>(&["id"]),
            ValidationInput::of::<crate::domain::catalog::CatalogCandidate>(&["id"]),
            ValidationInput::of::<normalized::entities::SymbolEntityCandidate>(&["id"]),
            ValidationInput::of::<normalized::entities::SymbolEntityResolution>(&["id"]),
            ValidationInput::of::<normalized::entities::EntityRef>(&["id"]),
            ValidationInput::of::<normalized::entities::CallableEntity>(&["id"]),
            ValidationInput::of::<normalized::entities::ClassEntity>(&["id"]),
            ValidationInput::of::<crate::domain::catalog::CatalogCallable>(&["id"]),
            ValidationInput::of::<crate::domain::catalog::CatalogCallableAspect>(&["id"]),
            ValidationInput::of::<crate::domain::catalog::CatalogClass>(&["id"]),
            ValidationInput::of::<crate::domain::catalog::CatalogConstructor>(&["id"]),
            ValidationInput::of::<crate::domain::catalog::CatalogDefault>(&["id"]),
            ValidationInput::of::<crate::domain::catalog::CatalogExposure>(&["id"]),
            ValidationInput::of::<crate::domain::catalog::CatalogInvocation>(&["id"]),
            ValidationInput::of::<crate::domain::catalog::CatalogMember>(&["id"]),
            ValidationInput::of::<crate::domain::catalog::CatalogMemberInvocation>(&["id"]),
            ValidationInput::of::<catalog::CatalogOptionEvidence>(&["id"]),
            ValidationInput::of::<syntax::ParameterSyntaxObservation>(&["id"]),
            ValidationInput::of::<syntax::ClassFieldSyntaxObservation>(&["id"]),
            ValidationInput::of::<normalized::entities::FieldDeclarationLink>(&["id"]),
            ValidationInput::of::<normalized::entities::ParameterEntity>(&["id"]),
            ValidationInput::of::<crate::domain::catalog::CatalogOption>(&["id"]),
            ValidationInput::of::<crate::domain::catalog::CatalogOptionSubject>(&["id"]),
            ValidationInput::of::<crate::domain::catalog::evidence::CatalogDeployment>(&["id"]),
            ValidationInput::of::<crate::domain::catalog::evidence::CatalogScenario>(&["id"]),
            ValidationInput::of::<crate::domain::catalog::evidence::DiagnosticUseAssessment>(&[
                "id",
            ]),
            ValidationInput::of::<crate::domain::catalog::evidence::DiagnosticUseLink>(&["id"]),
            ValidationInput::of::<crate::domain::catalog::evidence::DiagnosticUseTarget>(&["id"]),
            ValidationInput::of::<crate::domain::catalog::evidence::DocumentAssociation>(&["id"]),
            ValidationInput::of::<crate::domain::catalog::evidence::EvidenceInvocation>(&["id"]),
            ValidationInput::of::<crate::domain::catalog::evidence::EvidenceRoot>(&["id"]),
            ValidationInput::of::<crate::domain::catalog::evidence::OriginalSource>(&["id"]),
            ValidationInput::of::<crate::domain::catalog::evidence::ReleaseDeployment>(&["id"]),
            ValidationInput::of::<crate::domain::catalog::evidence::RootSubject>(&["id"]),
            ValidationInput::of::<normalized::events::NormalizedCallAlternative>(&["id"]),
            ValidationInput::of::<normalized::events::NormalizedCallEvent>(&["id"]),
            ValidationInput::of::<crate::domain::catalog::evidence::ScenarioAssociation>(&["id"]),
            ValidationInput::of::<crate::domain::catalog::evidence::ScenarioDependency>(&["id"]),
            ValidationInput::of::<crate::domain::catalog::evidence::ScenarioSpan>(&["id"]),
            ValidationInput::of::<crate::domain::catalog::evidence::SetupDependency>(&["id"]),
            ValidationInput::of::<crate::domain::catalog::evidence::SourceCharacterization>(&[
                "id",
            ]),
            ValidationInput::of::<crate::domain::deployment::DeploymentObservation>(&["id"]),
            ValidationInput::of::<crate::domain::diagnostics::PyreflyDiagnosticObservation>(&[
                "id",
            ]),
            ValidationInput::of::<crate::domain::diagnostics::PyreflyDiagnosticSupport>(&["id"]),
            ValidationInput::of::<crate::domain::diagnostics::RuffDiagnosticObservation>(&["id"]),
            ValidationInput::of::<crate::domain::documents::DocumentMentionObservation>(&["id"]),
            ValidationInput::of::<crate::domain::documents::DocumentNode>(&["id"]),
            ValidationInput::of::<crate::domain::documents::DocumentObservation>(&["id"]),
            ValidationInput::of::<crate::domain::documents::PassageObservation>(&["id"]),
            ValidationInput::of::<crate::domain::documents::DocumentComponentObservation>(&["id"]),
            ValidationInput::of::<crate::domain::normalized::callable_aspects::CallableAspect>(&[
                "id",
            ]),
            ValidationInput::of::<crate::domain::normalized::callables::EffectiveCallableAssessment>(
                &["id"],
            ),
            ValidationInput::of::<crate::domain::normalized::callables::SignatureSlot>(&["id"]),
            ValidationInput::of::<crate::domain::normalized::callables::SignatureVariant>(&["id"]),
            ValidationInput::of::<crate::domain::normalized::entities::FieldEntity>(&["id"]),
            ValidationInput::of::<crate::domain::normalized::entities::PublicExposure>(&["id"]),
            ValidationInput::of::<crate::domain::normalized::links::MentionEntityAssessment>(&[
                "id",
            ]),
            ValidationInput::of::<crate::domain::normalized::links::MentionEntityCandidate>(&[
                "id",
            ]),
            ValidationInput::of::<crate::domain::source::Module>(&["id"]),
            ValidationInput::of::<crate::domain::source::Occurrence>(&["id"]),
            ValidationInput::of::<syntax::SyntaxPlacement>(&["id"]),
            ValidationInput::of::<crate::domain::source::SourceArtifact>(&["id"]),
            ValidationInput::of::<crate::domain::value::Literal>(&["id"]),
        ])
    }
    /// Rich row membership follows the actual rendering family. Referenced public members in
    /// a document/scenario are nominal subjects, not another API rendering grain.
    pub fn root_types(subject: &c1::RootSubject) -> Vec<std::any::TypeId> {
        use std::any::TypeId;
        let mut types = vec![
            TypeId::of::<Definition>(),
            TypeId::of::<c1::EvidenceRoot>(),
            TypeId::of::<c1::RootSubject>(),
            TypeId::of::<c1::EvidenceInvocation>(),
            TypeId::of::<analysis::catalog_evidence::Invocation>(),
            TypeId::of::<crate::domain::source::SourceArtifact>(),
            TypeId::of::<crate::domain::source::Occurrence>(),
            TypeId::of::<assertion::Evidence>(),
            TypeId::of::<c1::OriginalSource>(),
            TypeId::of::<assertion::AssertionQualification>(),
        ];
        match subject {
            c1::RootSubject::Member { .. } => {
                macro_rules! rows {($($field:ident:$ty:ty,)*)=>{$(types.push(TypeId::of::<$ty>());)*};}
                crate::catalog_inputs!(rows);
                crate::catalog_outputs!(rows);
                crate::retrieval_inputs!(rows);
            }
            c1::RootSubject::Scenario { .. } => types.extend([
                TypeId::of::<normalized::events::NormalizedCallAlternative>(),
                TypeId::of::<normalized::events::NormalizedCallEvent>(),
                TypeId::of::<c1::CatalogScenario>(),
                TypeId::of::<c1::ScenarioSpan>(),
                TypeId::of::<c1::ScenarioDependency>(),
                TypeId::of::<c1::SetupDependency>(),
                TypeId::of::<c1::ScenarioAssociation>(),
                TypeId::of::<c1::DiagnosticUseTarget>(),
                TypeId::of::<c1::DiagnosticUseLink>(),
                TypeId::of::<c1::DiagnosticUseAssessment>(),
                TypeId::of::<c1::SourceCharacterization>(),
                TypeId::of::<analysis::native::NativeAssertionPremise>(),
                TypeId::of::<diagnostics::RuffDiagnosticObservation>(),
                TypeId::of::<diagnostics::PyreflyDiagnosticObservation>(),
                TypeId::of::<diagnostics::PyreflyDiagnosticSupport>(),
                TypeId::of::<attribution::ProviderRun>(),
            ]),
            c1::RootSubject::Document { .. } => types.extend([
                TypeId::of::<documents::DocumentObservation>(),
                TypeId::of::<documents::PassageObservation>(),
                TypeId::of::<documents::DocumentComponentObservation>(),
                TypeId::of::<documents::DocumentNode>(),
                TypeId::of::<c1::DocumentAssociation>(),
                TypeId::of::<normalized::links::MentionEntityCandidate>(),
                TypeId::of::<normalized::links::MentionEntityAssessment>(),
                TypeId::of::<documents::DocumentMentionObservation>(),
            ]),
            c1::RootSubject::Deployment { .. } => types.extend([
                TypeId::of::<c1::CatalogDeployment>(),
                TypeId::of::<deployment::DeploymentObservation>(),
                TypeId::of::<c1::ReleaseDeployment>(),
            ]),
            c1::RootSubject::Option { .. } => {
                macro_rules! rows {($($field:ident:$ty:ty,)*)=>{$(types.push(TypeId::of::<$ty>());)*};}
                crate::catalog_inputs!(rows);
                crate::catalog_outputs!(rows);
                crate::retrieval_inputs!(rows);
            }
            c1::RootSubject::Source { .. } => {
                types.extend([TypeId::of::<syntax::SyntaxPlacement>()])
            }
            c1::RootSubject::Release { .. } => types.extend([
                TypeId::of::<input::Package>(),
                TypeId::of::<input::Release>(),
                TypeId::of::<c1::ReleaseDeployment>(),
                TypeId::of::<c1::CatalogDeployment>(),
                TypeId::of::<deployment::DeploymentObservation>(),
            ]),
        }
        types
    }
    pub fn brief_types() -> Vec<std::any::TypeId> {
        use std::any::TypeId;
        let mut types = vec![
            TypeId::of::<Definition>(),
            TypeId::of::<syntax::SyntaxPlacement>(),
            TypeId::of::<crate::domain::source::SourceArtifact>(),
            TypeId::of::<crate::domain::source::Occurrence>(),
            TypeId::of::<value::Literal>(),
            TypeId::of::<assertion::Evidence>(),
            TypeId::of::<analysis::catalog_core::Invocation>(),
            TypeId::of::<c1::EvidenceRoot>(),
            TypeId::of::<c1::RootSubject>(),
            TypeId::of::<catalog::CatalogMember>(),
        ];
        macro_rules! rows {($($field:ident:$ty:ty,)*)=>{$(types.push(TypeId::of::<$ty>());)*};}
        crate::retrieval_synthesis_inputs!(rows);
        types
    }
    /// Necessary completed corpus/window/anchor properties do not decode API rendering
    /// premises or replay the deterministic renderer.
    pub fn completion_types() -> Vec<std::any::TypeId> {
        use std::any::TypeId;
        vec![
            TypeId::of::<c1::ScenarioSpan>(),
            TypeId::of::<c1::CatalogDeployment>(),
            TypeId::of::<deployment::DeploymentObservation>(),
            TypeId::of::<catalog::CatalogOptionEvidence>(),
            TypeId::of::<syntax::ParameterSyntaxObservation>(),
            TypeId::of::<syntax::ClassFieldSyntaxObservation>(),
            TypeId::of::<normalized::entities::FieldDeclarationLink>(),
            TypeId::of::<catalog::CatalogCandidate>(),
            TypeId::of::<catalog::CatalogExposure>(),
            TypeId::of::<catalog::CatalogOption>(),
            TypeId::of::<catalog::CatalogDefault>(),
            TypeId::of::<normalized::entities::EntityRef>(),
            TypeId::of::<normalized::entities::SymbolEntityCandidate>(),
            TypeId::of::<normalized::entities::SymbolEntityResolution>(),
            TypeId::of::<normalized::entities::PublicExposure>(),
            TypeId::of::<normalized::entities::CallableEntity>(),
            TypeId::of::<normalized::entities::ClassEntity>(),
            TypeId::of::<normalized::entities::ParameterEntity>(),
            TypeId::of::<c1::ScenarioAssociation>(),
            TypeId::of::<normalized::events::NormalizedCallAlternative>(),
            TypeId::of::<normalized::events::NormalizedCallEvent>(),
            TypeId::of::<c1::DocumentAssociation>(),
            TypeId::of::<normalized::links::MentionEntityCandidate>(),
            TypeId::of::<normalized::links::MentionEntityAssessment>(),
            TypeId::of::<documents::DocumentMentionObservation>(),
            TypeId::of::<c1::ReleaseDeployment>(),
            TypeId::of::<syntax::SyntaxPlacement>(),
            TypeId::of::<Definition>(),
            TypeId::of::<crate::domain::source::SourceArtifact>(),
            TypeId::of::<crate::domain::source::Occurrence>(),
            TypeId::of::<value::Literal>(),
            TypeId::of::<assertion::Evidence>(),
            TypeId::of::<c1::OriginalSource>(),
            TypeId::of::<c1::EvidenceRoot>(),
            TypeId::of::<c1::RootSubject>(),
            TypeId::of::<crate::domain::synthesis::documentary::ProseSlice>(),
            TypeId::of::<crate::domain::synthesis::documentary::ProseSource>(),
            TypeId::of::<catalog::CatalogMember>(),
            TypeId::of::<crate::domain::source::Module>(),
            TypeId::of::<crate::domain::synthesis::briefs::Brief>(),
            TypeId::of::<crate::domain::synthesis::seeds::SelectedSeed>(),
            TypeId::of::<crate::domain::synthesis::seeds::SeedPlan>(),
            TypeId::of::<analysis::synthesis::Invocation>(),
            TypeId::of::<catalog::CatalogMemberInvocation>(),
            TypeId::of::<analysis::catalog_core::Invocation>(),
            TypeId::of::<documents::DocumentObservation>(),
            TypeId::of::<documents::PassageObservation>(),
            TypeId::of::<documents::DocumentComponentObservation>(),
            TypeId::of::<documents::DocumentNode>(),
            TypeId::of::<assertion::AssertionQualification>(),
        ]
    }
    /// Small fixed metadata is prepared once, including frames without any retrieval document.
    pub fn frame_inputs() -> Vec<ValidationInput> {
        let mut inputs = crate::domain::selection::frames::Frames::inputs();
        inputs.extend([
            ValidationInput::of::<Definition>(&["id"]),
            ValidationInput::of::<analysis::AnalysisDefinition>(&["id"]),
            ValidationInput::of::<analysis::MethodParameters>(&["id"]),
            ValidationInput::of::<analysis::catalog_evidence::AnalysisOutcome>(&["id"]),
            ValidationInput::of::<analysis::synthesis::Invocation>(&["id"]),
            ValidationInput::of::<analysis::synthesis::AnalysisOutcome>(&["id"]),
            ValidationInput::of::<crate::domain::synthesis::frames::Frame>(&["id"]),
        ]);
        inputs
    }
    pub fn mandatory_consumed_inputs(_profile: Profile) -> Vec<ValidationInput> {
        let mut inputs = Self::root_inputs();
        inputs.extend(Self::frame_inputs());
        inputs.sort_by_key(|input| (input.name(), input.prefix()));
        inputs.dedup_by_key(|input| (input.name(), input.prefix()));
        inputs
    }
    pub fn synthesis_consumed_inputs() -> Vec<ValidationInput> {
        SynthesisFacts::inputs()
    }
    pub fn inputs() -> Vec<ValidationInput> {
        let mut inputs = Self::mandatory_consumed_inputs(Profile::Behavioral);
        inputs.extend(Self::synthesis_consumed_inputs());
        inputs.push(
            ValidationInput::of::<assertion::AssertionQualification>(&["id"])
                .at_epoch(PublicationBoundary::Local),
        );
        inputs.sort_by_key(|input| (input.name(), input.prefix()));
        inputs.dedup_by_key(|input| (input.name(), input.prefix()));
        inputs
    }
    pub fn selected(&self) -> Result<&Definition, ModelError> {
        if self.facts.definitions.len() != 1 {
            return Err(invalid("retrieval needs one completed authored definition"));
        }
        let r = self.facts.definitions.iter().next().unwrap();
        r.validate()?;
        Ok(r)
    }
}
macro_rules! facts {($($f:ident:$ty:ty,)*)=>{pub struct Facts {$(pub $f:Rows<$ty>,)*}impl Facts {pub fn new(b:&ResourceBudget)->Self {Self {$($f:Rows::new(b),)*}}pub fn visit(&mut self,n:&str,b:&arrow_array::RecordBatch)->Result<bool,ModelError> {$(if n==<$ty>::NAME {self.$f.decode(b)?;return Ok(true);})*Ok(false)}pub fn inputs()->Vec<ValidationInput> {vec![$(ValidationInput::of::<$ty>(&["id"])),*]}fn uses()->Vec<RelationUse> {crate::domain::normalized::facts_stage_inputs(vec![$(RelationUse::completed::<$ty>()),*])}}};}
crate::retrieval_inputs!(facts);
macro_rules! synthesis {($($f:ident:$ty:ty,)*)=>{pub struct SynthesisFacts {$(pub $f:Rows<$ty>,)*}impl SynthesisFacts {pub fn new(b:&ResourceBudget)->Self {Self {$($f:Rows::new(b),)*}}pub fn visit(&mut self,n:&str,b:&arrow_array::RecordBatch)->Result<bool,ModelError> {$(if n==<$ty>::NAME {self.$f.decode(b)?;return Ok(true);})*Ok(false)}pub fn inputs()->Vec<ValidationInput> {vec![$(ValidationInput::of::<$ty>(&["id"])),*]}}};}
crate::retrieval_synthesis_inputs!(synthesis);
macro_rules! outputs {($($f:ident:$ty:ty,)*)=>{pub struct Output {$(pub $f:Rows<$ty>,)*}impl Output {pub fn new(b:&ResourceBudget)->Self {Self {$($f:Rows::new(b),)*}}pub fn visit(&mut self,n:&str,b:&arrow_array::RecordBatch)->Result<bool,ModelError> {$(if n==<$ty>::NAME {self.$f.decode(b)?;return Ok(true);})*Ok(false)}pub fn inputs()->Vec<ValidationInput> {vec![$(ValidationInput::of::<$ty>(&["id"])),*]}pub fn matches(&self,o:&Self)->Result<(),ModelError> {$(if !self.$f.same(&o.$f) {return Err(invalid(format!("canonical retrieval closure differs: {}",<$ty>::NAME)));})*Ok(())}}};}
crate::retrieval_outputs!(outputs);
impl Output {
    /// Necessary completion/admission checks operate on immutable semantic records, without
    /// reconstructing upstream producer outputs or replaying the textual renderer.
    pub fn verify_completion(&self, d: &Data, b: &ResourceBudget) -> Result<(), ModelError> {
        super::construction::verify(self, d, b)
    }
}
pub(super) use super::construction::{nominates_part, nominations};
struct Render {
    text: String,
    anchors: Vec<AnchorSource>,
    subjects: Vec<Subject>,
}
fn member_path(d: &Data, m: &catalog::CatalogMember) -> Result<String, ModelError> {
    let module = need(&d.source.core.modules, m.access)?;
    Ok(format!("{}.{}", module.qualified_name, m.path.join(".")))
}
fn api(
    d: &Data,
    m: &catalog::CatalogMember,
    context: Id<AnalysisContext>,
    b: &ResourceBudget,
) -> Result<Render, ModelError> {
    let mut text = member_path(d, m)?;
    text.push('\n');
    for exposure in d
        .source
        .catalog
        .exposures
        .iter()
        .filter(|r| r.member == m.id())
    {
        let exposure = need(&d.source.core.exposures, exposure.exposure)?;
        if exposure.context == context {
            text.push_str(&format!(
                "Exposure status={:?} reason={:?}\n",
                exposure.status, exposure.reason
            ));
        }
    }

    let mut callables = d
        .source
        .catalog
        .callables
        .iter()
        .filter(|r| {
            r.member == m.id()
                && d.source
                    .core
                    .assessments
                    .get(r.assessment)
                    .is_some_and(|a| a.context == context)
        })
        .collect::<Vec<_>>();
    callables.sort_by_key(|r| r.id());
    for c in callables {
        let a = need(&d.source.core.assessments, c.assessment)?;
        text.push_str(&format!("Callable {:?}: identity={:?} signatures={:?} descriptor={:?}/{:?} body={:?} admitted={}\n",c.basis,a.identity,a.signatures,a.descriptor,a.descriptor_kind,a.body,a.body_admitted));
        let mut variants = d
            .source
            .catalog
            .invocations
            .iter()
            .filter(|r| r.callable == c.id())
            .collect::<Vec<_>>();
        variants.sort_by_key(|r| r.variant);
        for i in variants {
            let v = need(&d.source.core.variants, i.variant)?;
            let signature = need(&d.facts.signatures, v.signature)?;
            text.push_str(&format!(
                "Signature role={:?} form={:?} adjustment={:?}\n",
                v.role, signature.form, v.adjustment
            ));
            let mut slots = d
                .source
                .core
                .slots
                .iter()
                .filter(|r| r.variant == v.id())
                .collect::<Vec<_>>();
            slots.sort_by_key(|r| (r.ordinal, r.id()));
            for slot in slots {
                let parameter = need(&d.facts.signature_parameters, slot.parameter)?;
                let shape = need(&d.facts.shapes, parameter.shape)?;
                text.push_str(&format!(
                    "Parameter {:?} {:?} required={} default-slot={:?}\n",
                    shape.name, shape.kind, shape.required, slot.default
                ));
            }
        }
        for aspect in d
            .source
            .catalog
            .aspects
            .iter()
            .filter(|r| r.callable == c.id())
        {
            let a = need(&d.source.core.aspects, aspect.aspect)?;
            text.push_str(&format!(
                "Aspect {:?} admission={:?}\n",
                a.kind, a.admission
            ));
        }
    }
    let mut options = d
        .source
        .catalog
        .options
        .iter()
        .filter(|r| r.member == m.id())
        .collect::<Vec<_>>();
    options.sort_by_key(|r| r.id());
    for o in options {
        text.push_str(&option_text(d, o, b)?);
    }
    for class in d
        .source
        .catalog
        .classes
        .iter()
        .filter(|r| r.member == m.id())
    {
        for c in d
            .source
            .catalog
            .constructors
            .iter()
            .filter(|r| r.class == class.id())
        {
            text.push_str(&format!(
                "Constructor {:?} {:?} applicability={:?} disposition={:?}\n",
                c.origin, c.kind, c.applicability, c.disposition
            ));
        }
    }
    Ok(Render {
        text,
        anchors: vec![],
        subjects: vec![Subject::Member { member: m.id() }],
    })
}
fn option_text(
    d: &Data,
    o: &catalog::CatalogOption,
    b: &ResourceBudget,
) -> Result<String, ModelError> {
    let subject = need(&d.source.catalog.subjects, o.subject)?;
    let default = need(&d.source.catalog.defaults, o.default)?;
    let label = match subject {
        catalog::CatalogOptionSubject::Parameter { slot } => {
            let slot = need(&d.source.core.slots, *slot)?;
            let p = need(&d.facts.signature_parameters, slot.parameter)?;
            let shape = need(&d.facts.shapes, p.shape)?;
            format!(
                "effective parameter {} ({})",
                shape.name.as_deref().unwrap_or("unnamed"),
                shape.kind.label()
            )
        }
        catalog::CatalogOptionSubject::Field { field } => format!(
            "configuration field {}",
            need(&d.source.core.fields, *field)?.name
        ),
        catalog::CatalogOptionSubject::SourceParameter { .. } => "original source parameter".into(),
    };
    let value = match default {
        catalog::CatalogDefault::Absent {} => "Absent".into(),
        catalog::CatalogDefault::Unknown {} => "Unknown".into(),
        catalog::CatalogDefault::Unavailable {} => "Unavailable".into(),
        catalog::CatalogDefault::Literal { literal } => {
            format!(
                "Literal {}",
                value::presentation::render(
                    need(&d.facts.literals, *literal)?,
                    value::presentation::Mode::Human,
                    None,
                    b
                )?
                .map_err(|_| invalid("human literal presentation unavailable"))?
                .text
            )
        }
        catalog::CatalogDefault::Expression { .. } => "Unevaluated expression".into(),
        catalog::CatalogDefault::Factory { .. } => "Factory (not evaluated)".into(),
    };
    Ok(format!("Option {label}: default={value}\n"))
}
struct RenderedIdentity {
    family: Family,
    origin: Origin,
    title: String,
}

fn add(
    d: &Data,
    out: &mut Output,
    root: &c1::EvidenceRoot,
    rendered_identity: RenderedIdentity,
    render: Render,
    b: &ResourceBudget,
) -> Result<(), ModelError> {
    let RenderedIdentity {
        family,
        origin,
        title,
    } = rendered_identity;
    for a in &render.anchors {
        let (artifact, _, _) = super::source::coordinates(d, a)?;
        if !matches!(origin, Origin::Brief { .. })
            && need(&d.source.core.artifacts, artifact)?.input != root.input
        {
            return Err(invalid("retrieval anchor crosses captured input"));
        }
    }
    let corpus = out.corpus.insert(CorpusText {
        family,
        rendering_version: d.selected()?.rendering_version,
        digest: ContentHash::of(render.text.as_bytes()),
        text: Utf8Text::from(render.text),
    })?;
    let origin = out.origins.insert(origin)?;
    let unit = out.units.insert(Unit {
        input: root.input,
        context: root.context,
        family,
        origin,
        corpus,
        title: Utf8Text::from(title),
    })?;
    out.roots.insert(UnitRoot {
        unit,
        root: root.id(),
    })?;
    for subject in render.subjects {
        let subject = out.subjects.insert(subject)?;
        out.unit_subjects.insert(UnitSubject { unit, subject })?;
    }
    for (ordinal, anchor) in render.anchors.into_iter().enumerate() {
        let original = out.anchor_sources.insert(anchor)?;
        out.anchors.insert(OriginalAnchor {
            unit,
            ordinal: ordinal as i64,
            original,
        })?;
    }
    let parts = super::construction::parts(d, out, unit, b)?;
    super::partition::construct(d, out, unit, parts, b)?;
    Ok(())
}
pub fn build(d: &Data, b: &ResourceBudget) -> Result<Output, ModelError> {
    render(d, None, b)
}
pub fn root(
    d: &Data,
    root: Id<c1::EvidenceRoot>,
    b: &ResourceBudget,
) -> Result<Output, ModelError> {
    need(&d.evidence.roots, root)?;
    render(d, Some(root), b)
}
fn render(
    d: &Data,
    selected: Option<Id<c1::EvidenceRoot>>,
    b: &ResourceBudget,
) -> Result<Output, ModelError> {
    d.selected()?;
    let mut out = Output::new(b);
    // Bound render buffers before allocating. Input rows and retained output rows have separate charges.
    let mut payload = 0usize;
    let mut count = 0usize;
    macro_rules! measure {
        ($rows:expr) => {
            for row in $rows.iter() {
                payload = payload
                    .checked_add(row.heap_bytes())
                    .ok_or_else(|| invalid("retrieval render heap overflow"))?;
                count = count
                    .checked_add(1)
                    .ok_or_else(|| invalid("retrieval render count overflow"))?;
            }
        };
    }
    macro_rules! core {($($f:ident:$ty:ty,)*)=>{$(measure!(d.source.core.$f);)*};}
    crate::catalog_inputs!(core);
    macro_rules! catalog {($($f:ident:$ty:ty,)*)=>{$(measure!(d.source.catalog.$f);)*};}
    crate::catalog_outputs!(catalog);
    macro_rules! facts {($($f:ident:$ty:ty,)*)=>{$(measure!(d.source.facts.$f);)*};}
    crate::catalog_evidence_inputs!(facts);
    macro_rules! evidence {($($f:ident:$ty:ty,)*)=>{$(measure!(d.evidence.$f);)*};}
    crate::catalog_evidence_outputs!(evidence);
    macro_rules! extra {($($f:ident:$ty:ty,)*)=>{$(if std::any::TypeId::of::<$ty>()!=std::any::TypeId::of::<artifact::ArtifactChunk>(){measure!(d.facts.$f);} )*};}
    crate::retrieval_inputs!(extra);
    macro_rules! synthesis {($($f:ident:$ty:ty,)*)=>{$(measure!(d.synthesis.$f);)*};}
    crate::retrieval_synthesis_inputs!(synthesis);
    let bound = payload
        .checked_mul(16)
        .and_then(|n| count.checked_mul(4096).and_then(|c| n.checked_add(c)))
        .ok_or_else(|| invalid("retrieval render allocation overflow"))?;
    let _render = b.reserve("retrieval-render-buffers", bound)?;
    for root in d
        .evidence
        .roots
        .iter()
        .filter(|root| selected.is_none_or(|selected| root.id() == selected))
    {
        match need(&d.evidence.subjects, root.subject)? {
            c1::RootSubject::Member { member } => {
                let m = need(&d.source.catalog.members, *member)?;
                if m.input != root.input {
                    return Err(invalid("retrieval member root crosses input"));
                }
                let title = member_path(d, m)?;
                add(
                    d,
                    &mut out,
                    root,
                    RenderedIdentity {
                        family: Family::ApiOptions,
                        origin: Origin::Api { member: *member },
                        title: title.clone(),
                    },
                    api(d, m, root.context, b)?,
                    b,
                )?;
                let definitions = super::construction::definitions(d, *member, root.context)?;
                if definitions.is_empty() {
                    add(
                        d,
                        &mut out,
                        root,
                        RenderedIdentity {
                            family: Family::Source,
                            origin: Origin::UnavailableDefinition { member: *member },
                            title: format!("{title}: defining source unavailable"),
                        },
                        Render {
                            text: format!(
                                "Defining source unavailable for {title}; public access does not identify a source body."
                            ),
                            anchors: vec![],
                            subjects: vec![Subject::Member { member: *member }],
                        },
                        b,
                    )?;
                }
                for (entity, anchor, exact) in definitions {
                    let (text, anchors) = if let Some(anchor) = anchor {
                        let mut text = String::new();
                        let mut anchors = vec![];
                        for (enclosing, branch) in
                            super::construction::enclosing(d, &anchor, root.context)?
                        {
                            text.push_str(&format!("Enclosing syntax branch {branch:?}\n"));
                            text.push_str(&super::source::read(d, &enclosing, b)?.value);
                            text.push('\n');
                            anchors.push(enclosing);
                        }
                        let body = super::source::read(d, &anchor, b)?;
                        if body.value.is_empty() {
                            text.push_str("Defining original source is empty (0 bytes).");
                        } else {
                            text.push_str(&body.value);
                        }
                        anchors.push(anchor);
                        (text, anchors)
                    } else {
                        (
                            format!(
                                "Defining alternative {entity:?}; native/external/synthetic source unavailable; exact={exact}"
                            ),
                            vec![],
                        )
                    };
                    add(
                        d,
                        &mut out,
                        root,
                        RenderedIdentity {
                            family: Family::Source,
                            origin: Origin::Definition {
                                member: *member,
                                entity,
                            },
                            title: title.clone(),
                        },
                        Render {
                            text,
                            anchors,
                            subjects: vec![
                                Subject::Member { member: *member },
                                Subject::Definition { entity },
                            ],
                        },
                        b,
                    )?;
                }
            }
            c1::RootSubject::Scenario { scenario } => {
                let r = need(&d.evidence.scenarios, *scenario)?;
                let mut text = format!(
                    "Scenario intent={:?} extraction={:?} parse={:?} binding={:?} environment={:?} execution={:?}\n",
                    r.intent, r.extraction, r.parse, r.binding, r.environment, r.execution
                );
                let mut anchors = vec![];
                let mut spans = d
                    .evidence
                    .spans
                    .iter()
                    .filter(|r| r.scenario == *scenario)
                    .collect::<Vec<_>>();
                spans.sort_by_key(|r| (r.ordinal, r.id()));
                for span in spans {
                    let anchor = AnchorSource::Original {
                        source: span.source,
                    };
                    let original = super::source::read(d, &anchor, b)?;
                    text.push_str(&format!("Span {:?}\n{}\n", span.role, original.value));
                    anchors.push(anchor);
                }
                for dependency in d
                    .evidence
                    .dependencies
                    .iter()
                    .filter(|r| r.scenario == *scenario)
                {
                    text.push_str(&format!(
                        "Setup {:?} status={:?}\n",
                        need(&d.evidence.setup, dependency.dependency)?,
                        dependency.status
                    ));
                }
                let _diagnostic_notes = b.reserve(
                    "retrieval-diagnostic-relevance",
                    d.evidence.diagnostic_use_targets.len().saturating_mul(256),
                )?;
                let mut diagnostics = std::collections::BTreeSet::new();
                let mut notes = 0usize;
                for target in d.evidence.diagnostic_use_targets.iter() {
                    let association = need(&d.evidence.associations, target.association)?;
                    if association.scenario != *scenario {
                        continue;
                    }
                    let link = need(&d.evidence.diagnostic_use_links, target.link)?;
                    let assessment = need(&d.evidence.diagnostic_use_assessments, link.assessment)?;
                    if assessment.status != c1::DiagnosticUseStatus::UniqueUse
                        || assessment.remainder
                    {
                        continue;
                    }
                    if !diagnostics.insert((assessment.characterization, association.member)) {
                        continue;
                    }
                    if notes == 128 {
                        text.push_str("Additional diagnostic relevance notes omitted by finite rendering bound.\n");
                        break;
                    }
                    notes += 1;
                    let characterization = need(
                        &d.evidence.source_characterizations,
                        assessment.characterization,
                    )?;
                    let native = need(
                        &d.source.facts.characterization_native,
                        characterization.native,
                    )?;
                    let (channel,settings)=match native {
                        crate::domain::analysis::native::NativeAssertionPremise::RuffDiagnosticObservation{assertion,..}=>{d.ruff_header(*assertion)?},
                        crate::domain::analysis::native::NativeAssertionPremise::PyreflyDiagnosticObservation{assertion,support}=>{let channel=d.pyrefly_channel(*assertion)?;let support=need(&d.source.facts.pyrefly_diagnostic_supports,*support)?;(channel,need(&d.source.facts.runs,support.run)?.configuration)},
                        _=>return Err(invalid("diagnostic relevance has a non-diagnostic native premise")),
                    };
                    text.push_str(&format!("Diagnostic source relevance: exact use; target={:?}; channel={:?}; settings={:?}; diagnostic-only, no execution claim.\n",association.basis,channel,settings));
                }
                let mut subjects = vec![];
                for a in d.evidence.associations.iter().filter(|r| {
                    r.scenario == *scenario
                        && d.source
                            .core
                            .qualifications
                            .get(r.qualification)
                            .is_some_and(|q| q.context == root.context)
                }) {
                    text.push_str(&format!(
                        "Association basis={:?} phase={:?} intent={:?}\n",
                        a.basis, a.phase, a.intent
                    ));
                    subjects.push(Subject::Member { member: a.member });
                }
                add(
                    d,
                    &mut out,
                    root,
                    RenderedIdentity {
                        family: Family::Scenario,
                        origin: Origin::Scenario {
                            scenario: *scenario,
                        },
                        title: "Usage scenario".into(),
                    },
                    Render {
                        text,
                        anchors,
                        subjects,
                    },
                    b,
                )?;
            }
            c1::RootSubject::Document { observation } => {
                let document = need(&d.source.facts.documents, *observation)?;
                let artifact = need(&d.source.core.artifacts, document.source)?;
                let mut any = false;
                for passage in d.source.facts.passages.iter().filter(|p| {
                    d.source
                        .core
                        .qualifications
                        .get(p.qualification)
                        .is_some_and(|q| q.context == root.context)
                }) {
                    let node = need(&d.source.facts.nodes, passage.passage.id())?;
                    let anchor = AnchorSource::Span { span: node.span() };
                    if super::source::coordinates(d, &anchor)?.0 != document.source {
                        continue;
                    }
                    any = true;
                    let text = super::source::read(d, &anchor, b)?;
                    let anchors = super::construction::passage_anchors(d, passage, b)?;
                    let mut contextual = String::new();
                    for ancestor in anchors.iter().take(anchors.len().saturating_sub(1)) {
                        contextual.push_str(&super::source::read(d, ancestor, b)?.value);
                    }
                    let mut subjects = vec![];
                    for a in d.evidence.document_associations.iter() {
                        let candidate = need(&d.source.facts.mention_candidates, a.candidate)?;
                        let assessment =
                            need(&d.source.facts.mention_assessments, candidate.assessment)?;
                        let mention = need(&d.source.facts.mentions, assessment.observation)?;
                        if mention.passage == passage.passage
                            && d.source
                                .core
                                .qualifications
                                .get(mention.qualification)
                                .is_some_and(|q| q.context == root.context)
                        {
                            subjects.push(Subject::Member { member: a.member });
                        }
                    }
                    add(
                        d,
                        &mut out,
                        root,
                        RenderedIdentity {
                            family: Family::DocumentationDeployment,
                            origin: Origin::Passage {
                                observation: passage.id(),
                            },
                            title: passage
                                .heading
                                .clone()
                                .unwrap_or_else(|| artifact.path.clone()),
                        },
                        Render {
                            text: format!(
                                "Heading: {}\n{}{}",
                                passage.heading.as_deref().unwrap_or(&artifact.path),
                                contextual,
                                text.value
                            ),
                            anchors,
                            subjects,
                        },
                        b,
                    )?;
                }
                if !any {
                    let anchor = AnchorSource::Artifact {
                        artifact: document.source,
                    };
                    let text = super::source::read(d, &anchor, b)?;
                    add(
                        d,
                        &mut out,
                        root,
                        RenderedIdentity {
                            family: Family::DocumentationDeployment,
                            origin: Origin::Document {
                                observation: *observation,
                            },
                            title: document
                                .title
                                .clone()
                                .unwrap_or_else(|| artifact.path.clone()),
                        },
                        Render {
                            text: format!(
                                "Document: {}\n{}",
                                document.title.as_deref().unwrap_or(&artifact.path),
                                text.value
                            ),
                            anchors: vec![anchor],
                            subjects: vec![],
                        },
                        b,
                    )?;
                }
            }
            c1::RootSubject::Deployment { deployment } => {
                let row = need(&d.evidence.deployments, *deployment)?;
                let observation = need(&d.source.facts.deployment, row.observation)?;
                let anchor = AnchorSource::Span {
                    span: observation.span,
                };
                let text = super::source::read(d, &anchor, b)?;
                let subjects = d
                    .evidence
                    .release_deployments
                    .iter()
                    .filter(|r| r.deployment == *deployment)
                    .map(|r| Subject::Release { release: r.release })
                    .collect();
                add(
                    d,
                    &mut out,
                    root,
                    RenderedIdentity {
                        family: Family::DocumentationDeployment,
                        origin: Origin::Deployment {
                            deployment: *deployment,
                        },
                        title: observation.field.clone(),
                    },
                    Render {
                        text: format!(
                            "Declared {} {:?} interpretation={:?}\n{}",
                            observation.field,
                            observation.name,
                            observation.interpretation,
                            text.value
                        ),
                        anchors: vec![anchor],
                        subjects,
                    },
                    b,
                )?;
            }
            c1::RootSubject::Option { option } => {
                let row = need(&d.source.catalog.options, *option)?;
                let member = need(&d.source.catalog.members, row.member)?;
                let mut text = format!("{}\n{}", member_path(d, member)?, option_text(d, row, b)?);
                let anchors = super::construction::option_anchors(d, row)?;
                for anchor in &anchors {
                    text.push_str(&super::source::read(d, anchor, b)?.value);
                    text.push('\n');
                }

                add(
                    d,
                    &mut out,
                    root,
                    RenderedIdentity {
                        family: Family::ApiOptions,
                        origin: Origin::Option { option: *option },
                        title: format!("{} option", member_path(d, member)?),
                    },
                    Render {
                        text,
                        anchors,
                        subjects: vec![
                            Subject::Option { option: *option },
                            Subject::Member { member: row.member },
                        ],
                    },
                    b,
                )?;
            }
            c1::RootSubject::Source { artifact } => {
                let anchor = AnchorSource::Artifact {
                    artifact: *artifact,
                };
                let text = super::source::read(d, &anchor, b)?;
                let source = need(&d.source.core.artifacts, *artifact)?;
                add(
                    d,
                    &mut out,
                    root,
                    RenderedIdentity {
                        family: Family::Source,
                        origin: Origin::Source {
                            artifact: *artifact,
                        },
                        title: source.path.clone(),
                    },
                    Render {
                        text: if text.value.is_empty() {
                            "Original source is empty (0 bytes).".into()
                        } else {
                            text.value
                        },
                        anchors: vec![anchor],
                        subjects: vec![Subject::Source {
                            artifact: *artifact,
                        }],
                    },
                    b,
                )?;
            }
            c1::RootSubject::Release { release } => {
                let row = need(&d.facts.releases, *release)?;
                let package = need(&d.facts.packages, row.package)?;
                add(
                    d,
                    &mut out,
                    root,
                    RenderedIdentity {
                        family: Family::DocumentationDeployment,
                        origin: Origin::Release { release: *release },
                        title: format!("{} {}", package.name, row.version),
                    },
                    Render {
                        text: format!("Distribution {} version {}", package.name, row.version),
                        anchors: vec![],
                        subjects: vec![Subject::Release { release: *release }],
                    },
                    b,
                )?;
            }
        }
    }
    if selected.is_none() {
        extend_synthesis(d, &mut out, b)?;
    }
    Ok(out)
}
/// Inverse memberships the actual root/brief renderers read, not all incoming references.
pub fn memberships() -> Vec<(std::any::TypeId, &'static str)> {
    use crate::domain::synthesis::briefs::*;
    use catalog::{evidence::*, *};
    use normalized::callables::*;
    use std::any::TypeId;
    vec![
        (TypeId::of::<CatalogExposure>(), "member"),
        (TypeId::of::<CatalogCandidate>(), "exposure"),
        (TypeId::of::<CatalogCallable>(), "member"),
        (TypeId::of::<CatalogOption>(), "member"),
        (TypeId::of::<CatalogClass>(), "member"),
        (TypeId::of::<CatalogInvocation>(), "callable"),
        (TypeId::of::<CatalogCallableAspect>(), "callable"),
        (TypeId::of::<CatalogConstructor>(), "class"),
        (TypeId::of::<SignatureSlot>(), "variant"),
        (TypeId::of::<syntax::SyntaxPlacement>(), "parent"),
        (TypeId::of::<crate::domain::source::Occurrence>(), "source"),
        (TypeId::of::<OriginalSource>(), "artifact_artifact"),
        (TypeId::of::<ScenarioSpan>(), "scenario"),
        (TypeId::of::<ScenarioDependency>(), "scenario"),
        (TypeId::of::<ScenarioAssociation>(), "scenario"),
        (TypeId::of::<DiagnosticUseTarget>(), "association"),
        (TypeId::of::<ReleaseDeployment>(), "deployment"),
        (TypeId::of::<EvidenceInvocation>(), "root"),
        (TypeId::of::<BriefDocument>(), "brief"),
        (TypeId::of::<BriefSource>(), "brief"),
        (TypeId::of::<documents::DocumentNode>(), "passage_span"),
        (TypeId::of::<documents::PassageObservation>(), "passage"),
        (
            TypeId::of::<documents::DocumentComponentObservation>(),
            "passage",
        ),
    ]
}
pub fn invariants() -> Vec<Invariant> {
    let mut inputs = Data::inputs();
    inputs.extend(Output::inputs());
    vec![Invariant {
        purpose: crate::domain::InvariantPurpose::DiagnosticReplay,
        revision: 3,
        name: "retrieval_canonical_rendering",
        inputs,
        create: std::sync::Arc::new(|b| {
            Box::new(Check {
                data: Data::new(b),
                out: Output::new(b),
                budget: b.clone(),
            })
        }),
    }]
}
struct Check {
    data: Data,
    out: Output,
    budget: ResourceBudget,
}
impl InvariantCheck for Check {
    fn visit_input(
        &mut self,
        input: &ValidationInput,
        b: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if is_vocabulary(input.name()) {
            if !self.data.visit_input(input, b)? {
                return Err(invalid("undeclared completed rendering/source view"));
            }
            Ok(())
        } else {
            self.visit(input.name(), b)
        }
    }
    fn visit(&mut self, n: &str, b: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        if !self.data.visit(n, b)? && !self.out.visit(n, b)? {
            return Err(invalid("undeclared retrieval rendering input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        self.out.matches(&build(&self.data, &self.budget)?)
    }
}
pub fn definition() -> (analysis::MethodParameters, analysis::AnalysisDefinition) {
    let (parameters, _) = catalog::build::definition();
    let mut version = KeySink::new("retrieval-final-owner/v2");
    parameters.id().encode(&mut version);
    SEMANTIC_RULE_REVISION.encode(&mut version);
    let definition = analysis::AnalysisDefinition {
        method: analysis::AnalysisMethod::Retrieval,
        interpretation: analysis::Interpretation::Heuristic,
        parameters: parameters.id(),
        semantic_version: version.finish(),
    };
    (parameters, definition)
}

/// Optional briefs enrich ApiOptions without replacing any mandatory family or original source.
pub fn extend_synthesis(d: &Data, out: &mut Output, b: &ResourceBudget) -> Result<(), ModelError> {
    for brief in d.synthesis.briefs.iter() {
        let seed = need(&d.synthesis.seeds, brief.seed)?;
        let plan = need(&d.synthesis.seed_plans, seed.plan)?;
        let invocation = need(&d.synthesis.synthesis_invocations, plan.invocation)?;
        if invocation.definition != crate::domain::synthesis::build::definition().1.id() {
            return Err(invalid("retrieval brief has foreign S0 owner"));
        }
        let member = need(&d.synthesis.member_frames, seed.member)?;
        let core = need(&d.source.facts.core_invocations, member.invocation)?;
        if core.input != invocation.input || core.context != invocation.context {
            return Err(invalid("retrieval brief crosses its public member frame"));
        }
        let subject = c1::RootSubject::Member {
            member: member.member,
        };
        let root = d
            .evidence
            .roots
            .iter()
            .find(|r| {
                r.input == invocation.input
                    && r.context == invocation.context
                    && r.subject == subject.id()
            })
            .ok_or_else(|| invalid("retrieval brief has no exact C1 member root"))?;
        let mut rows = d
            .synthesis
            .brief_documents
            .iter()
            .filter(|r| r.brief == brief.id())
            .collect::<Vec<_>>();
        rows.sort_by_key(|r| r.ordinal);
        let _copy = b.reserve(
            "retrieval-brief-copy",
            usize::try_from(brief.bytes)
                .map_err(ModelError::codec)?
                .checked_mul(2)
                .and_then(|n| n.checked_add(rows.len() * size_of::<AnchorSource>()))
                .ok_or_else(|| invalid("retrieval brief allocation overflow"))?,
        )?;
        let mut text = String::new();
        for (ordinal, row) in rows.into_iter().enumerate() {
            if row.ordinal != ordinal as i64 {
                return Err(invalid("retrieval brief part membership changed"));
            }
            row.validate()?;
            text.push_str(row.text.as_str());
        }
        if text.len() as i64 != brief.bytes || ContentHash::of(text.as_bytes()) != brief.rendered {
            return Err(invalid("retrieval brief differs from canonical S0 bytes"));
        }
        let mut anchors = vec![];
        for source in d
            .synthesis
            .brief_sources
            .iter()
            .filter(|r| r.brief == brief.id())
        {
            let proof = need(&d.synthesis.documentary, source.documentary)?;
            need(&d.synthesis.prose_slices, proof.prose)?;
            anchors.push(AnchorSource::Prose { slice: proof.prose });
        }
        if anchors.is_empty() {
            return Err(invalid("retrieval brief lacks authored original anchors"));
        }
        anchors.sort_by_key(Record::id);
        anchors.dedup();
        add(
            d,
            out,
            root,
            RenderedIdentity {
                family: Family::ApiOptions,
                origin: Origin::Brief { brief: brief.id() },
                title: brief.title.as_str().into(),
            },
            Render {
                text,
                anchors,
                subjects: vec![Subject::Member {
                    member: member.member,
                }],
            },
            b,
        )?;
    }
    Ok(())
}

/// Mandatory helper closure, reused within the single final E0 producer stage.
pub fn mandatory_inputs(
    profile: Profile,
    model: &ValidatedModel,
    order: &stages::PublicationOrder,
) -> Result<Vec<RelationUse>, ModelError> {
    let parent = c1::build::stage(profile, model, order)?;
    let mut inputs = parent.inputs;
    inputs.extend(parent.outputs.into_iter().map(|r| r.completed_input()));
    inputs.extend(Facts::uses());
    inputs.sort_by_key(|r| (r.name(), r.prefix()));
    inputs.dedup_by_key(|r| (r.name(), r.prefix()));
    Ok(inputs)
}

impl Data {
    /// One actual S0 and C1 parent for every native frame, independently of observed retrieval rows.
    pub fn parents(
        &self,
        input: Id<input::InputRevision>,
        context: Id<attribution::AnalysisContext>,
    ) -> Result<[analysis::retrieval::InvocationSource; 2], ModelError> {
        let (params, definition) = crate::domain::synthesis::build::definition();
        if self.synthesis.analysis_definitions.get(definition.id()) != Some(&definition)
            || self.synthesis.parameters.get(params.id()) != Some(&params)
        {
            return Err(invalid("retrieval canonical S0 definition is not authored"));
        }
        let (params, definition) = self::definition();
        if self.synthesis.analysis_definitions.get(definition.id()) != Some(&definition)
            || self.synthesis.parameters.get(params.id()) != Some(&params)
        {
            return Err(invalid("retrieval canonical E0 definition is not authored"));
        }
        let mut synthesis = self.synthesis.synthesis_invocations.iter().filter(|i| {
            i.input == input
                && i.context == context
                && i.definition == crate::domain::synthesis::build::definition().1.id()
                && i.subject.is_none()
        });
        let synthesis = synthesis
            .next()
            .ok_or_else(|| invalid("retrieval completed S0 native frame absent"))?;
        if self
            .synthesis
            .synthesis_invocations
            .iter()
            .filter(|i| {
                i.input == input
                    && i.context == context
                    && i.definition == synthesis.definition
                    && i.subject.is_none()
            })
            .count()
            != 1
        {
            return Err(invalid("retrieval S0 native frame ambiguous"));
        }
        let mut outcomes = self
            .synthesis
            .synthesis_outcomes
            .iter()
            .filter(|r| r.invocation == synthesis.id());
        let outcome = outcomes
            .next()
            .ok_or_else(|| invalid("retrieval S0 outcome absent"))?;
        if outcomes.next().is_some() {
            return Err(invalid("retrieval S0 outcome ambiguous"));
        }
        outcome.validate()?;
        let mut frames = self
            .synthesis
            .synthesis_frames
            .iter()
            .filter(|r| r.invocation == synthesis.id());
        let frame = frames
            .next()
            .ok_or_else(|| invalid("retrieval completed S0 frame closure absent"))?;
        if frames.next().is_some() {
            return Err(invalid("retrieval S0 frame ambiguous"));
        }
        let evidence = need(&self.facts.evidence_invocations, frame.evidence)?;
        if evidence.input != input
            || evidence.context != context
            || evidence.subject.is_some()
            || evidence.definition != c1::build::definition().1.id()
        {
            return Err(invalid("retrieval S0/C1 parent identity changed"));
        }
        let mut outcomes = self
            .synthesis
            .evidence_outcomes
            .iter()
            .filter(|r| r.invocation == evidence.id());
        let outcome = outcomes
            .next()
            .ok_or_else(|| invalid("retrieval C1 outcome absent"))?;
        if outcomes.next().is_some() {
            return Err(invalid("retrieval C1 outcome ambiguous"));
        }
        outcome.validate()?;
        Ok([
            analysis::retrieval::InvocationSource::Synthesis {
                invocation: synthesis.id(),
            },
            analysis::retrieval::InvocationSource::CatalogEvidence {
                invocation: evidence.id(),
            },
        ])
    }
}
/// Final single-writer E0 boundary. Completed E1 rows participate only in shared winner equality.
pub fn stage(
    profile: Profile,
    selected: &Definition,
    model: &ValidatedModel,
    order: &stages::PublicationOrder,
) -> Result<Stage, ModelError> {
    use std::collections::BTreeSet;
    selected.validate()?;
    let mut outputs = super::relations()
        .iter()
        .map(RelationUse::of_relation)
        .collect::<Vec<_>>();
    outputs.extend([
        RelationUse::of::<embedding::value::FullValue>(),
        RelationUse::of::<embedding::projection::ProjectedValue>(),
    ]);
    outputs.extend(
        analysis::retrieval::publication_relations()
            .iter()
            .map(RelationUse::of_relation),
    );
    let own = outputs.iter().map(|r| r.name()).collect::<BTreeSet<_>>();
    let mut requested = super::consumption::ConsumptionData::inputs();
    requested.extend(analysis::expected::inputs(
        analysis::AnalysisMethod::Retrieval,
    ));
    requested.retain(|input| !own.contains(input.name()));
    requested.extend([
        ValidationInput::of::<embedding::value::FullValue>(&["id"])
            .at_epoch(PublicationBoundary::AnalyticEmbedding),
        ValidationInput::of::<embedding::projection::ProjectedValue>(&["id"])
            .at_epoch(PublicationBoundary::AnalyticEmbedding),
    ]);
    let inputs = dependency_closure::DependencyClosure::stage_grants(
        model,
        requested,
        &outputs,
        PublicationBoundary::Synthesis,
        dependency_closure::LowerLayerPolicy::OmitInferredOrdinaryFacts,
        order,
    )?;
    Ok(Stage {
        name: "retrieval",
        inputs,
        outputs,
        contributes: vec![],
        coverage: vec![],
        profiles: vec![profile],
        effect: if selected.embedding_requested {
            Effect::Embedding
        } else {
            Effect::Pure
        },
        code: ContentHash::of(
            &[
                include_bytes!("build.rs").as_slice(),
                include_bytes!("construction.rs").as_slice(),
                include_bytes!("partition.rs").as_slice(),
                include_bytes!("source.rs").as_slice(),
            ]
            .concat(),
        ),
        configuration: ContentHash::of(selected.id().bytes()),
    })
}

pub(crate) fn invariants_refs() -> Vec<&'static str> {
    vec!["retrieval_canonical_rendering"]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::synthesis::{assertions, briefs, documentary, seeds};
    #[test]
    fn optional_canonical_briefs_retain_full_warning_and_original_prose_without_replacing_mandatory_units()
     {
        let prose = "Run carefully.\n\nWarning:\nKeep authentication enabled.\n";
        let raw = format!("\"\"\"{prose}\"\"\"");
        let (b, doc, member) = documentary::tests::fixture(&raw, prose);
        let docs = documentary::build(&doc, &b).unwrap();
        let core = doc.core_invocations.iter().next().unwrap();
        let invocation = analysis::synthesis::Invocation::new(
            core.input,
            core.context,
            crate::domain::synthesis::build::definition().1.id(),
            None,
            [],
        )
        .0;
        let mut invocations = Rows::new(&b);
        invocations.insert(invocation.clone()).unwrap();
        let settings = analysis::settings::AnalyticsConfiguration {
            module_prefixes: vec!["pkg.impl".into()],
            public_roots: vec!["pkg.api".into()],
            configured_seeds: vec!["pkg.api.run".into()],
            depth: 2,
            vertices: 128,
            arcs: 512,
            witnesses: 3,
            brief_budget: 1,
            communities: false,
            pagerank: false,
            fca: false,
            knn: false,
            rca: false,
            type_layer: false,
            mention_layer: false,
            knn_layer: false,
        };
        let seeds = seeds::configured(
            &doc,
            &Rows::new(&b),
            &Rows::new(&b),
            &Rows::new(&b),
            &settings,
            &invocation,
            &b,
        )
        .unwrap();
        let assertions = assertions::build_documentary(&doc, &docs, &invocations, &b).unwrap();
        let briefs = briefs::build(&doc, &docs, &assertions, &seeds, &b).unwrap();
        assert_eq!(briefs.briefs.len(), 1);
        let mut d = Data::new(&b);
        d.facts
            .definitions
            .insert(Definition::builtin(false))
            .unwrap();
        for row in doc.members.iter() {
            d.source.catalog.members.insert(row.clone()).unwrap();
        }
        for row in doc.member_frames.iter() {
            d.synthesis.member_frames.insert(row.clone()).unwrap();
        }
        for row in doc.core_invocations.iter() {
            d.source.facts.core_invocations.insert(row.clone()).unwrap();
        }
        for row in doc.modules.iter() {
            d.source.core.modules.insert(row.clone()).unwrap();
        }
        for row in doc.artifacts.iter() {
            d.source.core.artifacts.insert(row.clone()).unwrap();
        }
        for row in doc.occurrences.iter() {
            d.source.core.occurrences.insert(row.clone()).unwrap();
        }
        for row in doc.chunks.iter() {
            d.facts.chunks.insert(row.clone()).unwrap();
        }
        for row in doc.canonical_evidence.iter() {
            d.source
                .facts
                .canonical_evidence
                .insert(row.clone())
                .unwrap();
        }
        let m = doc.member_frames.get(member).unwrap();
        let module = doc.modules.iter().next().unwrap();
        d.evidence
            .original_sources
            .insert(c1::OriginalSource::Artifact {
                artifact: module.source,
            })
            .unwrap();
        let subject = d
            .evidence
            .subjects
            .insert(c1::RootSubject::Member { member: m.member })
            .unwrap();
        d.evidence
            .roots
            .insert(c1::EvidenceRoot {
                input: core.input,
                context: core.context,
                subject,
            })
            .unwrap();
        let mut extended = build(&d, &b).unwrap();
        let mandatory_units = extended.units.iter().map(Record::id).collect::<Vec<_>>();
        d.synthesis.synthesis_invocations = invocations;
        d.synthesis.seed_plans = seeds.plans;
        d.synthesis.seeds = seeds.selected;
        d.synthesis.briefs = briefs.briefs;
        d.synthesis.brief_documents = briefs.documents;
        d.synthesis.brief_sources = briefs.sources;
        d.synthesis.documentary = docs.conclusions;
        d.synthesis.prose_slices = docs.slices;
        d.synthesis.prose_sources = docs.prose_sources;
        extend_synthesis(&d, &mut extended, &b).unwrap();
        let out = build(&d, &b).unwrap();
        extended.matches(&out).unwrap();
        extended.verify_completion(&d, &b).unwrap();
        assert!(
            mandatory_units
                .iter()
                .all(|id| extended.units.get(*id).is_some())
        );
        // A window with internally valid text/digest still must equal its canonical corpus slice.
        let original = extended.windows.iter().next().unwrap().clone();
        let mut damaged = original.clone();
        damaged.text = "x".repeat(original.text.len()).into();
        damaged.digest = ContentHash::of(damaged.text.as_str().as_bytes());
        damaged.validate().unwrap();
        let mut windows = Rows::new(&b);
        for window in extended.windows.iter() {
            windows
                .insert(if window.id() == original.id() {
                    damaged.clone()
                } else {
                    window.clone()
                })
                .unwrap();
        }
        extended.windows = windows;
        assert!(extended.verify_completion(&d, &b).is_err());
        let unit = out
            .units
            .iter()
            .find(|r| matches!(out.origins.get(r.origin), Some(Origin::Brief { .. })))
            .unwrap();
        let text = need(&out.corpus, unit.corpus).unwrap().text.as_str();
        assert!(text.contains("Keep authentication enabled."));
        assert!(
            out.units
                .iter()
                .any(|r| matches!(out.origins.get(r.origin), Some(Origin::Api { .. })))
        );
        assert!(out.units.iter().any(|r| r.family == Family::Source));
        let anchor = out.anchors.iter().find(|r| r.unit == unit.id()).unwrap();
        let source = need(&out.anchor_sources, anchor.original).unwrap();
        assert!(matches!(source, AnchorSource::Prose { .. }));
        assert_eq!(
            super::super::source::read(&d, source, &b).unwrap().value,
            prose
        );
        let row = d.synthesis.brief_documents.iter().next().unwrap().clone();
        d.synthesis.brief_documents = Rows::new(&b);
        d.synthesis
            .brief_documents
            .insert(crate::domain::synthesis::briefs::BriefDocument {
                text: "Relocated or forged brief".into(),
                ..row
            })
            .unwrap();
        assert!(build(&d, &b).is_err());
    }
}
