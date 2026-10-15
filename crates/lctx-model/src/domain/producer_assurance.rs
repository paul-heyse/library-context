//! Current producer provenance and model-owned facts declaration checks.
use super::{
    ContentHash, Id, ModelError,
    analysis::sources::SourceSnapshot,
    attribution::{AnalysisContext, Provider},
    completed::CompletedContribution,
    input::InputRevision,
    stages::Profile,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Required provenance, included in the completed contribution identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "evidence",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ProducerAssurance {
    CurrentProviderInterface,
}
impl ProducerAssurance {
    pub fn is_current(&self) -> bool {
        true
    }
    pub fn identity(&self) -> Result<ContentHash, ModelError> {
        let mut sink = super::KeySink::new("completed-producer-assurance/v1");
        sink.part(b"kind", b"current-provider-interface");
        Ok(sink.finish())
    }
    pub(crate) fn validate_for(
        &self,
        _completed: &CompletedContribution,
    ) -> Result<(), ModelError> {
        Ok(())
    }
    pub fn summary(&self) -> Result<ProducerAssuranceSummary, ModelError> {
        Ok(ProducerAssuranceSummary::CurrentProviderInterface {
            identity: self.identity()?,
        })
    }
}
impl super::HeapSize for ProducerAssurance {}
/// Compact attribution; selected descriptors and actual gates independently establish admission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProducerAssuranceSummary {
    CurrentProviderInterface { identity: ContentHash },
}
impl ProducerAssuranceSummary {
    pub fn validate(&self, _contract: Option<ContentHash>) -> Result<(), ModelError> {
        let Self::CurrentProviderInterface { identity } = self;
        if *identity != ProducerAssurance::CurrentProviderInterface.identity()? {
            return Err(ModelError::Conflict(
                "manifest producer assurance attribution",
            ));
        }
        Ok(())
    }
    pub(crate) fn encode(&self, sink: &mut super::KeySink) {
        use super::Key;
        let Self::CurrentProviderInterface { identity } = self;
        sink.part(b"producer-assurance", b"current-provider-interface");
        identity.encode(sink);
    }
}

/// Checked model-owned facts declaration. This does not certify selected physical rows.
pub struct AdmittedProducerDeclaration<'a> {
    completed: &'a CompletedContribution,
    declaration: &'a super::producer_contract::ProducerContract,
}
#[derive(Clone, Copy)]
pub enum AdmittedSourceBinding<'a> {
    Current(&'a super::producer_contract::CapturedSourceBinding),
}
pub enum AdmittedSourceBindings<'a> {
    Current(std::slice::Iter<'a, super::producer_contract::CapturedSourceBinding>),
}
impl<'a> Iterator for AdmittedSourceBindings<'a> {
    type Item = AdmittedSourceBinding<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        let Self::Current(sources) = self;
        sources.next().map(AdmittedSourceBinding::Current)
    }
}
impl AdmittedSourceBinding<'_> {
    pub fn input(&self) -> Id<InputRevision> {
        let Self::Current(source) = self;
        source.input
    }
    pub fn context(&self) -> Option<Id<AnalysisContext>> {
        let Self::Current(source) = self;
        source.context
    }
    pub fn configuration(&self) -> Option<ContentHash> {
        let Self::Current(source) = self;
        source.configuration
    }
    pub fn complete_acquisition(
        &self,
    ) -> Option<&super::producer_contract::acquisition::CapturedAcquisition> {
        let Self::Current(source) = self;
        Some(&source.acquisition)
    }
}
impl<'a> AdmittedProducerDeclaration<'a> {
    pub fn for_facts(
        completed: &'a CompletedContribution,
        target: &'a super::producer_contract::ProducerContract,
        providers: &[Provider],
        model: ContentHash,
        profile: Profile,
    ) -> Result<Self, ModelError> {
        completed.identity()?;
        completed.spec.validate_profile(profile)?;
        if !super::producer_contract::facts::facts()
            .iter()
            .any(|fixed| fixed.name == target.name && fixed.identity() == target.identity())
        {
            return Err(ModelError::Conflict(
                "facts declaration is not the fixed target recipe",
            ));
        }
        if completed.spec.model != model
            || completed.spec.producer != target.name
            || !target.profiles.contains(&profile)
        {
            return Err(ModelError::Conflict("facts producer target model/profile"));
        }
        let inventory = target
            .outputs
            .iter()
            .chain(&target.contributes)
            .map(|relation| relation.name().to_owned())
            .collect::<BTreeSet<_>>();
        if completed.spec.outputs != inventory
            || completed
                .spec
                .inputs
                .iter()
                .map(|input| input.relation())
                .ne(target.inputs.iter().map(|input| input.name()))
        {
            return Err(ModelError::Conflict(
                "facts producer target relation inventory",
            ));
        }
        let binding = completed
            .spec
            .captured_binding
            .as_ref()
            .ok_or(ModelError::Conflict(
                "current facts producer has no captured binding",
            ))?;
        target.validate_binding(binding, completed.spec.configuration, providers)?;
        target.validate_context_dependency(binding, completed.spec.profile)?;
        Ok(Self {
            completed,
            declaration: target,
        })
    }
    pub fn current_provider_interface(&self) -> bool {
        true
    }
    pub fn complete_request_reuse_eligible(&self) -> bool {
        self.completed.spec.captured_binding.is_some()
    }
    pub fn uniform_owner_eligible(&self) -> bool {
        self.completed.spec.captured_binding.is_some()
    }
    pub fn facts_declaration(
        &self,
    ) -> Result<&super::producer_contract::ProducerContract, ModelError> {
        Ok(self.declaration)
    }
    pub fn completed(&self) -> &CompletedContribution {
        self.completed
    }
    pub fn target_dependency_sources(&self) -> &[SourceSnapshot] {
        &self.completed.spec.inputs
    }
    pub fn source_bindings(&self) -> Result<AdmittedSourceBindings<'_>, ModelError> {
        let binding = self
            .completed
            .spec
            .captured_binding
            .as_ref()
            .ok_or(ModelError::Conflict(
                "facts declaration has no supplier source binding",
            ))?;
        Ok(AdmittedSourceBindings::Current(binding.sources.iter()))
    }
    pub fn name(&self) -> &'static str {
        self.declaration.name
    }
    pub fn declaration_identity(&self) -> ContentHash {
        self.declaration.identity()
    }
    pub fn suppliers(&self) -> Vec<super::producer_contract::SupplierRole> {
        self.declaration.suppliers.clone()
    }
    pub fn supplier(&self, role: &str) -> Result<Id<Provider>, ModelError> {
        self.completed
            .spec
            .captured_binding
            .as_ref()
            .ok_or(ModelError::Conflict("missing current facts binding"))?
            .suppliers
            .get(role)
            .copied()
            .ok_or(ModelError::Conflict("unbound admitted supplier"))
    }
    pub fn producer_settings(&self) -> Result<ContentHash, ModelError> {
        self.completed
            .spec
            .configuration
            .ok_or(ModelError::Conflict("missing admitted producer settings"))
    }
    pub fn not_requested(&self) -> &[super::attribution::FactFamily] {
        &self.declaration.not_requested
    }
    pub fn written_relations(&self) -> Vec<&str> {
        self.declaration.outputs.iter().map(|r| r.name()).collect()
    }
}
