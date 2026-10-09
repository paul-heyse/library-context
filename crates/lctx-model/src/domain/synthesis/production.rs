//! Inputs of the single S0 producer. Every frame requires real completed nominal parents.
use super::{automatic, documentary, frames};
use crate::domain::{
    analysis::{self, settings::AnalyticsConfiguration, synthesis as owner},
    normalized::Rows,
    resources::ResourceBudget,
    stages::*,
    *,
};
pub struct Data {
    pub frames: frames::Data,
    pub documentary: documentary::Data,
    pub automatic: automatic::Data,
    pub observations: super::observations::Data,
    pub controls: super::assertions::ControlData,
    pub summary: super::summary::Data,
    pub terminal: super::terminal::Data,
    pub patterns: super::patterns::Data,
    pub public: Rows<structural::PublicCandidate>,
}
impl Data {
    pub fn new(b: &ResourceBudget) -> Self {
        Self {
            frames: frames::Data::new(b),
            documentary: documentary::Data::new(b),
            automatic: automatic::Data::new(b),
            observations: super::observations::Data::new(b),
            controls: super::assertions::ControlData::new(b),
            summary: super::summary::Data::new(b),
            terminal: super::terminal::Data::new(b),
            patterns: super::patterns::Data::new(b),
            public: Rows::new(b),
        }
    }
    pub fn visit(&mut self, n: &str, b: &arrow_array::RecordBatch) -> Result<bool, ModelError> {
        let f = self.frames.visit(n, b)?;
        let d = self.documentary.visit(n, b)?;
        let a = self.automatic.visit(n, b)?;
        let p = if n == structural::PublicCandidate::NAME {
            self.public.decode(b)?;
            true
        } else {
            false
        };
        let o = self.observations.visit(n, b)?;
        let controls = self.controls.visit(n, b)?;
        let summary = self.summary.visit(n, b)?;
        let terminal = self.terminal.visit(n, b)?;
        let patterns = self.patterns.visit(n, b)?;
        Ok(f || d || a || p || o || summary || terminal || patterns || controls)
    }
    /// Native source checks and retained conclusions have distinct completed vocabulary owners.
    pub fn visit_input(
        &mut self,
        input: &ValidationInput,
        b: &arrow_array::RecordBatch,
    ) -> Result<bool, ModelError> {
        let n = input.name();
        if !is_vocabulary(n) {
            return self.visit(n, b);
        }
        match input.prefix() {
            Some(PublicationBoundary::Facts) => {
                let docs = self.documentary.visit(n, b)?;
                let patterns = self.patterns.visit(n, b)?;
                Ok(docs || patterns)
            }
            Some(PublicationBoundary::Analytic) => {
                let observations = self.observations.visit(n, b)?;
                let terminal = self.terminal.visit(n, b)?;
                Ok(observations || terminal)
            }
            _ => Err(ModelError::Invalid(format!(
                "synthesis input {n} changes its completed vocabulary view"
            ))),
        }
    }
    /// Exact root partition over one already decoded union. Current finite renderers own their
    /// inputs; only the current root is copied, with clone scratch and retained Rows charged.
    pub fn selected_copy(
        &self,
        selected: &mut dyn FnMut(
            std::any::TypeId,
            Option<PublicationBoundary>,
            [u8; 16],
        ) -> Result<bool, ModelError>,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        use std::any::TypeId;
        let mut out = Self::new(budget);
        {
            macro_rules! copy {($($field:ident:$ty:ty,)*)=>{$({
            for row in self.frames.$field.iter(){
                let epoch=if is_vocabulary(<$ty>::NAME){Some(PublicationBoundary::Analytic)}else{None};
                if selected(TypeId::of::<$ty>(),epoch,*row.id().bytes())?{out.frames.$field.insert_borrowed(row)?;}
            }
        })*};}
            crate::synthesis_frame_inputs!(copy);
        }

        for row in self.frames.analytic_parents.frames.iter() {
            if selected(
                TypeId::of::<analytics::AnalyticFrame>(),
                None,
                *row.id().bytes(),
            )? {
                out.frames.analytic_parents.frames.insert_borrowed(row)?;
            }
        }
        for row in self.frames.analytic_parents.invocations.iter() {
            if selected(
                TypeId::of::<analysis::analytic::Invocation>(),
                None,
                *row.id().bytes(),
            )? {
                out.frames
                    .analytic_parents
                    .invocations
                    .insert_borrowed(row)?;
            }
        }
        for row in self.frames.analytic_parents.results.iter() {
            if selected(
                TypeId::of::<analytics::TechniqueResult>(),
                None,
                *row.id().bytes(),
            )? {
                out.frames.analytic_parents.results.insert_borrowed(row)?;
            }
        }
        {
            macro_rules! copy {($($field:ident:$ty:ty,)*)=>{$({
            for row in self.documentary.$field.iter(){
                let epoch=if is_vocabulary(<$ty>::NAME){Some(PublicationBoundary::Facts)}else{None};
                if selected(TypeId::of::<$ty>(),epoch,*row.id().bytes())?{out.documentary.$field.insert_borrowed(row)?;}
            }
        })*};}
            crate::synthesis_documentary_inputs!(copy);
        }

        {
            macro_rules! copy {($($field:ident:$ty:ty,)*)=>{$({
            for row in self.automatic.$field.iter(){
                let epoch=if is_vocabulary(<$ty>::NAME){Some(PublicationBoundary::Analytic)}else{None};
                if selected(TypeId::of::<$ty>(),epoch,*row.id().bytes())?{out.automatic.$field.insert_borrowed(row)?;}
            }
        })*};}
            crate::synthesis_automatic_unique_inputs!(copy);
        }

        {
            macro_rules! copy {($($field:ident:$ty:ty,)*)=>{$({
            for row in self.observations.$field.iter(){
                let epoch=if is_vocabulary(<$ty>::NAME){Some(PublicationBoundary::Analytic)}else{None};
                if selected(TypeId::of::<$ty>(),epoch,*row.id().bytes())?{out.observations.$field.insert_borrowed(row)?;}
            }
        })*};}
            crate::synthesis_observation_inputs!(copy);
        }

        {
            macro_rules! copy {($($field:ident:$ty:ty,)*)=>{$({
            for row in self.controls.$field.iter(){
                let epoch=if is_vocabulary(<$ty>::NAME){Some(PublicationBoundary::Analytic)}else{None};
                if selected(TypeId::of::<$ty>(),epoch,*row.id().bytes())?{out.controls.$field.insert_borrowed(row)?;}
            }
        })*};}
            crate::synthesis_control_text_inputs!(copy);
        }

        {
            macro_rules! copy {($($field:ident:$ty:ty,)*)=>{$({
            for row in self.summary.$field.iter(){
                let epoch=if is_vocabulary(<$ty>::NAME){Some(PublicationBoundary::Analytic)}else{None};
                if selected(TypeId::of::<$ty>(),epoch,*row.id().bytes())?{out.summary.$field.insert_borrowed(row)?;}
            }
        })*};}
            crate::synthesis_summary_inputs!(copy);
        }

        {
            macro_rules! copy {($($field:ident:$ty:ty,)*)=>{$({
            for row in self.terminal.$field.iter(){
                let epoch=if is_vocabulary(<$ty>::NAME){Some(PublicationBoundary::Analytic)}else{None};
                if selected(TypeId::of::<$ty>(),epoch,*row.id().bytes())?{out.terminal.$field.insert_borrowed(row)?;}
            }
        })*};}
            crate::synthesis_terminal_inputs!(copy);
        }

        self.terminal
            .copy_selected_ownership(&mut out.terminal, &mut |kind, key| {
                selected(kind, None, key)
            })?;
        {
            macro_rules! copy {($($field:ident:$ty:ty,)*)=>{$({
            for row in self.patterns.$field.iter(){
                let epoch=if is_vocabulary(<$ty>::NAME){Some(PublicationBoundary::Facts)}else{None};
                if selected(TypeId::of::<$ty>(),epoch,*row.id().bytes())?{out.patterns.$field.insert_borrowed(row)?;}
            }
        })*};}
            crate::synthesis_pattern_inputs!(copy);
        }

        {
            macro_rules! copy {($($field:ident:$ty:ty,)*)=>{$({
            for row in self.patterns.setup.$field.iter(){
                let epoch=if is_vocabulary(<$ty>::NAME){Some(PublicationBoundary::Facts)}else{None};
                if selected(TypeId::of::<$ty>(),epoch,*row.id().bytes())?{out.patterns.setup.$field.insert_borrowed(row)?;}
            }
        })*};}
            crate::synthesis_setup_inputs!(copy);
        }

        {
            macro_rules! copy {($($field:ident:$ty:ty,)*)=>{$({
            for row in self.patterns.flow.$field.iter(){
                let epoch=if is_vocabulary(<$ty>::NAME){Some(PublicationBoundary::Facts)}else{None};
                if selected(TypeId::of::<$ty>(),epoch,*row.id().bytes())?{out.patterns.flow.$field.insert_borrowed(row)?;}
            }
        })*};}
            crate::structural_handoff_inputs!(copy);
        }

        {
            macro_rules! copy {($($field:ident:$ty:ty,)*)=>{$({
            for row in self.patterns.flow.entry.$field.iter(){
                let epoch=if is_vocabulary(<$ty>::NAME){Some(PublicationBoundary::Facts)}else{None};
                if selected(TypeId::of::<$ty>(),epoch,*row.id().bytes())?{out.patterns.flow.entry.$field.insert_borrowed(row)?;}
            }
        })*};}
            crate::entry_value_inputs!(copy);
        }

        // The flow collector's cumulative Local vocabulary is separate from its Facts entry.
        for row in self.patterns.flow.local.qualifications.iter() {
            if selected(
                TypeId::of::<assertion::AssertionQualification>(),
                Some(PublicationBoundary::Local),
                *row.id().bytes(),
            )? {
                out.patterns
                    .flow
                    .local
                    .qualifications
                    .insert_borrowed(row)?;
            }
        }
        for row in self.patterns.flow.local.conditions.iter() {
            if selected(
                TypeId::of::<conditions::Condition>(),
                Some(PublicationBoundary::Local),
                *row.id().bytes(),
            )? {
                out.patterns.flow.local.conditions.insert_borrowed(row)?;
            }
        }
        for row in self.patterns.flow.local.condition_nodes.iter() {
            if selected(
                TypeId::of::<conditions::ConditionNode>(),
                Some(PublicationBoundary::Local),
                *row.id().bytes(),
            )? {
                out.patterns
                    .flow
                    .local
                    .condition_nodes
                    .insert_borrowed(row)?;
            }
        }
        for row in self.public.iter() {
            if selected(
                TypeId::of::<structural::PublicCandidate>(),
                None,
                *row.id().bytes(),
            )? {
                out.public.insert_borrowed(row)?;
            }
        }
        Ok(out)
    }
    /// Physical original chunks are selected from the current partition's exact text demand.
    /// Their identity remains native ArtifactChunkKey; missing chunks remain absent for the
    /// ordinary renderer to refuse when required.
    pub fn copy_text_chunks(
        &mut self,
        union: &Self,
        phase: catalog_scope_program::SynthesisTextPhase,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        let demand = catalog_scope_program::synthesis_text(phase);
        let mut charge = charged::StateCharge::new(budget, "synthesis-partition-text-demand");
        let mut keys = charged::ChargedSet::default();
        let mut range =
            |source: Id<source::SourceArtifact>, start: i64, end: i64| -> Result<(), ModelError> {
                if start < 0 || end < start {
                    return Err(ModelError::Schema("synthesis text range"));
                }
                if start == end {
                    return Ok(());
                }
                for ordinal in start / artifact::ARTIFACT_CHUNK_BYTES as i64
                    ..=(end - 1) / artifact::ARTIFACT_CHUNK_BYTES as i64
                {
                    keys.insert(
                        &mut charge,
                        Id::of(&artifact::ArtifactChunkKey {
                            artifact: source,
                            ordinal,
                        }),
                    )?;
                }
                Ok(())
            };
        for row in self.documentary.occurrences.iter() {
            if demand.syntax_kinds.contains(&(row.syntax_kind as i16)) {
                range(row.source, row.start, row.end)?;
            }
        }
        for row in self.documentary.canonical_evidence.iter() {
            if demand.document_nodes_only&&!self.documentary.nodes.iter().any(|node|matches!(node,documents::DocumentNode::Passage{span,..}|documents::DocumentNode::Component{span,..} if span.id()==row.id())){continue;}
            if let assertion::Evidence::SourceSpan { source, start, end } = row {
                range(*source, *start, *end)?;
            }
        }
        for key in keys.iter() {
            if let Some(row) = union.documentary.chunks.get(*key) {
                self.documentary.chunks.insert_borrowed(row)?;
            }
        }
        Ok(())
    }
    pub fn consumed_inputs(profile: Profile) -> Vec<ValidationInput> {
        let mut inputs = Self::inputs(profile);
        inputs.extend(analysis::expected::inputs(
            analysis::AnalysisMethod::Synthesis,
        ));
        inputs
    }
    pub fn inputs(profile: Profile) -> Vec<ValidationInput> {
        let mut rows = frames::Data::inputs();
        rows.extend(documentary::Data::facts_inputs());
        rows.extend(automatic::Data::inputs());
        rows.extend(frames::AnalyticParents::inputs());
        rows.extend(super::observations::Data::inputs().into_iter().map(|i| {
            if is_vocabulary(i.name()) {
                i.at_epoch(PublicationBoundary::Analytic)
            } else {
                i
            }
        }));
        rows.extend(
            super::assertions::ControlData::inputs()
                .into_iter()
                .filter(|input| {
                    profile == Profile::Behavioral
                        || ![
                            flow::FlowTestLeafObservation::NAME,
                            flow::FlowRegionObservation::NAME,
                        ]
                        .contains(&input.name())
                }),
        );
        rows.extend(super::summary::Data::inputs());
        rows.extend(super::terminal::Data::inputs().into_iter().map(|i| {
            if is_vocabulary(i.name()) {
                i.at_epoch(PublicationBoundary::Analytic)
            } else {
                i
            }
        }));
        rows.extend(super::patterns::Data::inputs(profile).into_iter().map(|i| {
            if is_vocabulary(i.name()) {
                i.at_epoch(PublicationBoundary::Facts)
            } else {
                i
            }
        }));
        rows.push(ValidationInput::of::<structural::PublicCandidate>(&["id"]));
        rows.push(ValidationInput::of::<structural::handoffs::ValueSource>(&[
            "id",
        ]));
        rows.sort_by_key(|r| (r.name(), r.prefix(), r.order().to_vec()));
        rows.dedup_by(|a, b| {
            a.name() == b.name() && a.prefix() == b.prefix() && a.order() == b.order()
        });
        rows
    }
}
/// Shared synthesis state contains only navigation and correspondence topology. Source labels,
/// documentary payloads, assertion text and original chunks are loaded in an actual owner grain.
pub fn topology_inputs() -> Vec<ValidationInput> {
    let mut rows = frames::Data::inputs();
    macro_rules! topology {($($ty:ty),*)=>{$(rows.push(ValidationInput::of::<$ty>(&["id"]));)*};}
    topology!(
        catalog::CatalogMemberInvocation,
        catalog::CatalogExposure,
        catalog::CatalogCandidate,
        catalog::CatalogPath,
        catalog::CatalogAlias,
        normalized::entities::PublicExposure,
        normalized::entities::SymbolEntityCandidate
    );
    rows.sort_by_key(|input| (input.name(), input.prefix()));
    rows.dedup_by(|a, b| a.name() == b.name() && a.prefix() == b.prefix());
    rows
}
/// Incoming membership needed by the actual documentary, proof, control and code owner kernels.
/// Other nominal references are forward dependencies and never open their incoming neighborhood.
pub fn memberships() -> Vec<(std::any::TypeId, &'static str)> {
    use std::any::TypeId;
    let mut rows = vec![];
    macro_rules! own {($ty:ty,$($field:literal),+)=>{$(rows.push((TypeId::of::<$ty>(),$field));)+};}
    own!(catalog::CatalogCandidate, "exposure");
    own!(normalized::entities::ParameterEntityLink, "entity");
    own!(catalog::CatalogPath, "parent");
    own!(catalog::CatalogAlias, "parent");
    own!(syntax::DeclarationObservation, "declaration");
    own!(syntax::SyntaxDetailObservation, "occurrence");
    own!(documents::PassageObservation, "passage");
    own!(documents::DocumentComponentObservation, "passage");
    own!(documents::DocumentAttributeObservation, "component");
    own!(analysis::native::NativeQualification, "premise");
    own!(
        analysis::summary::AnalysisDerivationPremise,
        "derivation",
        "source"
    );
    own!(analysis::summary::AnalysisDerivation, "proposition");
    own!(assumptions::AssumptionSetMember, "set");
    own!(structural::handoffs::Member, "group");
    own!(catalog::evidence::ScenarioSpan, "scenario");
    own!(structural::controls::ControlPath, "traversal");
    own!(structural::controls::ControlStep, "path");
    own!(lexical::BindingObservation, "event");
    own!(lexical::LexicalResolution, "read");
    own!(syntax::ImportAliasObservation, "statement");
    own!(flow::FlowUse, "occurrence");
    own!(flow::FlowUseObservation, "use_");
    own!(flow::FlowReachingObservation, "use_");
    own!(flow_inventory::FlowUseInventoryObservation, "use_");
    own!(flow::FlowDefinitionObservation, "definition");
    own!(flow::FlowUseSupport, "assertion");
    own!(flow::FlowDefinitionSupport, "assertion");
    own!(flow::FlowReachingSupport, "assertion");
    own!(flow::FlowSourceViewSupport, "assertion");
    own!(flow::FlowRegionSupport, "assertion");
    own!(flow_inventory::FlowUseInventorySupport, "assertion");
    own!(flow_inventory::FlowUseCandidate, "inventory");
    own!(flow_inventory::FlowUseInventoryMember, "inventory");
    rows
}
/// Conclusion grains never decode source labels or rendering chunks. Qualification vocabulary
/// remains the exact cumulative Analytic publication used by the retained consequences.
pub fn conclusion_inputs() -> Vec<ValidationInput> {
    let mut inputs = super::observations::Data::inputs()
        .into_iter()
        .map(|input| {
            if stages::is_vocabulary(input.name()) {
                input.at_epoch(PublicationBoundary::Analytic)
            } else {
                input
            }
        })
        .collect::<Vec<_>>();
    inputs.extend(super::summary::Data::inputs());
    inputs.sort_by_key(|input| (input.name(), input.prefix()));
    inputs.dedup_by(|a, b| a.name() == b.name() && a.prefix() == b.prefix());
    inputs
}
/// Independent conclusion roots preserve findings/facets even when no public member links them.
pub fn conclusion_roots() -> [std::any::TypeId; 3] {
    [
        std::any::TypeId::of::<structural::Conclusion>(),
        std::any::TypeId::of::<analytics::Conclusion>(),
        std::any::TypeId::of::<execution::summary_consequences::ClaimConclusion>(),
    ]
}
pub fn stage(
    profile: Profile,
    settings: &AnalyticsConfiguration,
    model: &ValidatedModel,
    order: &stages::PublicationOrder,
) -> Result<Stage, ModelError> {
    use std::collections::BTreeSet;
    settings.validate()?;
    let mut outputs = super::relations()
        .iter()
        .map(RelationUse::of_relation)
        .collect::<Vec<_>>();
    macro_rules! outputs{($($ty:ty),*)=>{$(outputs.push(RelationUse::of::<$ty>());)*};}
    outputs.extend(
        owner::publication_relations()
            .iter()
            .map(RelationUse::of_relation),
    );
    outputs!(assertion::AssertionQualification);
    macro_rules! observation_outputs{($($f:ident:$t:ty,)*)=>{$(outputs.push(RelationUse::of::<$t>());)*};}
    crate::synthesis_observation_outputs!(observation_outputs);
    outputs.sort_by_key(|r| r.name());
    outputs.dedup_by_key(|r| r.name());
    let own = outputs.iter().map(|r| r.name()).collect::<BTreeSet<_>>();
    let mut requested = Data::inputs(profile);
    requested.extend(analysis::expected::inputs(
        analysis::AnalysisMethod::Synthesis,
    ));
    let relation = |name| {
        model
            .relation(name)
            .ok_or_else(|| ModelError::Invalid(format!("S0 relation absent: {name}")))
    };
    let mut inputs = vec![];
    for row in requested {
        if !own.contains(row.name()) || is_vocabulary(row.name()) {
            let mut use_ = RelationUse::of_relation(relation(row.name())?).completed_input();
            if let Some(epoch) = row.prefix() {
                use_ = use_.at_epoch(epoch);
            }
            inputs.push(use_);
        }
    }
    let roots = dependency_closure::DependencyClosure::roots_from_uses(model, &inputs)?;
    let inputs = dependency_closure::DependencyClosure::grants(
        model,
        roots,
        inputs,
        &outputs,
        PublicationBoundary::Analytic,
        dependency_closure::LowerLayerPolicy::OmitInferredOrdinaryFacts,
        order,
    )?;
    Ok(Stage {
        captured_binding: None,
        name: "synthesis",
        inputs,
        outputs,
        contributes: vec![],
        coverage: vec![],
        profiles: vec![profile],
        effect: Effect::Pure,
        code: super::build::definition().1.semantic_version,
        configuration: ContentHash::of(settings.id().bytes()),
    })
}

/// Compact existence demand for documentary first pass; rich conclusion text waits for rendering.
pub fn emission_roots() -> [std::any::TypeId; 5] {
    use std::any::TypeId;
    [
        TypeId::of::<structural::Conclusion>(),
        TypeId::of::<analytics::Conclusion>(),
        TypeId::of::<execution::summary_consequences::ClaimConclusion>(),
        TypeId::of::<structural::handoffs::Group>(),
        TypeId::of::<execution::summary_terminal::SummaryTerminalWitness>(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn final_stage_declares_real_parent_closure_and_one_vocabulary_epoch() {
        let model = crate::domain::model().unwrap();
        let settings = super::super::seeds::tests::settings(vec![], 0);
        for profile in [Profile::Catalog, Profile::Behavioral] {
            let stage = stage(
                profile,
                &settings,
                &model,
                &crate::domain::stages::PublicationOrder::registered(
                    crate::domain::ContentHash::of(b"fixture publication order"),
                    &[
                        (0, crate::domain::stages::PublicationBoundary::Facts),
                        (1, crate::domain::stages::PublicationBoundary::Dispatch),
                        (2, crate::domain::stages::PublicationBoundary::BaseSemantic),
                        (
                            3,
                            crate::domain::stages::PublicationBoundary::ExecutionModel,
                        ),
                        (
                            4,
                            crate::domain::stages::PublicationBoundary::CatalogSynthesis,
                        ),
                        (5, crate::domain::stages::PublicationBoundary::CatalogCore),
                        (
                            6,
                            crate::domain::stages::PublicationBoundary::CatalogEvidence,
                        ),
                        (7, crate::domain::stages::PublicationBoundary::Local),
                        (
                            8,
                            crate::domain::stages::PublicationBoundary::BaseEvaluation,
                        ),
                        (
                            9,
                            crate::domain::stages::PublicationBoundary::BaseCompletion,
                        ),
                        (10, crate::domain::stages::PublicationBoundary::SourceCall),
                        (
                            11,
                            crate::domain::stages::PublicationBoundary::EnrichedExecution,
                        ),
                        (12, crate::domain::stages::PublicationBoundary::Model),
                        (13, crate::domain::stages::PublicationBoundary::Summary),
                        (14, crate::domain::stages::PublicationBoundary::Structural),
                        (
                            15,
                            crate::domain::stages::PublicationBoundary::AnalyticEmbedding,
                        ),
                        (16, crate::domain::stages::PublicationBoundary::Analytic),
                        (17, crate::domain::stages::PublicationBoundary::Selection),
                        (18, crate::domain::stages::PublicationBoundary::Synthesis),
                        (19, crate::domain::stages::PublicationBoundary::Retrieval),
                    ],
                )
                .unwrap(),
            )
            .unwrap();
            assert_eq!(stage.name, "synthesis");
            assert!(
                stage
                    .inputs
                    .iter()
                    .any(|r| r.name() == analysis::summary::AnalysisOutcome::NAME)
            );
            assert!(
                stage
                    .inputs
                    .iter()
                    .any(|r| r.name() == analytics::TechniqueResult::NAME)
            );
            assert!(
                stage
                    .inputs
                    .iter()
                    .any(|r| r.name() == catalog::evidence::DocumentAssociation::NAME)
            );
            assert!(
                stage
                    .outputs
                    .iter()
                    .any(|r| r.name() == super::super::briefs::Brief::NAME)
            );
            assert!(
                stage
                    .outputs
                    .iter()
                    .any(|r| r.name() == owner::SourceReceipt::NAME)
            );
        }
    }
}
