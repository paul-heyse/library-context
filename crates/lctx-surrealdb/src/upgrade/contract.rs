//! Immutable host/native identity. This overlay is deliberately outside the runtime schema hash.
use super::*;
use serde::{Deserialize, Serialize};

pub(super) const OVERLAY_SCHEMA: &str = include_str!("progress.surql");
pub(super) const PROTOCOL: &[u8] = b"native-upgrade-progress/v2:history-page-receipts-v5;source4-exact;locked-installation-and-revision;atomic-read-checkpoints;keyset-bounded;independent-verification;preserving-sealed-publication";
pub(super) const PREFLIGHT: &[u8] = b"history-page-receipts-v5-preflight/v1:source4-history-checkpoint-universe;original-issuance-horizon-counters-phase-cursor;last-page-absent;closed-exclusive-effects;no-data-backfill";
pub(super) const VERIFIER: &[u8] = b"history-page-receipts-v5-verifier/v1:independent-bounded-actual-checkpoint-shape;last-page-absent;final-required-definitions-and-index-readiness;preserve-issuance-horizons-and-outcomes";
pub fn overlay_schema_identity() -> ContentHash { ContentHash::of(OVERLAY_SCHEMA.as_bytes()) }
pub fn transition_protocol_identity() -> ContentHash { ContentHash::of(PROTOCOL) }
pub fn preflight_contract_identity() -> ContentHash { ContentHash::of(PREFLIGHT) }
pub fn verifier_identity() -> ContentHash { ContentHash::of(VERIFIER) }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpgradeExecutionContract {
    pub format: u32,
    pub source_version: u32,
    pub kind: String,
    pub migration: ContentHash,
    pub execution: ContentHash,
    pub source: ContentHash,
    pub target: ContentHash,
    pub generation: ContentHash,
    pub overlay: ContentHash,
    pub protocol: ContentHash,
    pub preflight: ContentHash,
    pub verifier: ContentHash,
    pub namespace: String,
    pub database: String,
}
impl UpgradeExecutionContract {
    pub fn current(migration: ContentHash, execution: ContentHash, source: ContentHash, config: &RuntimeConfig) -> Self {
        Self { format: 2, source_version: SOURCE_VERSION, kind: TRANSITION_KIND.into(), migration, execution, source, target: crate::compiler::base_schema_identity(), generation: config.service_generation, overlay: overlay_schema_identity(), protocol: transition_protocol_identity(), preflight: preflight_contract_identity(), verifier: verifier_identity(), namespace: config.namespace.as_str().into(), database: config.database.as_str().into() }
    }
    pub fn validate(&self, config: &RuntimeConfig) -> Result<(), ModelError> {
        if self != &Self::current(self.migration, self.execution, self.source, config) || self.source.hex() != SOURCE_SCHEMA {
            return Err(ModelError::Conflict("native upgrade execution contract"));
        }
        Ok(())
    }
    pub(super) fn compatible(&self, previous: &Self) -> bool {
        // A verifier correction preserves acknowledged translation but repeats verification.
        let mut previous = previous.clone(); previous.execution = self.execution; previous.verifier = self.verifier;
        self == &previous
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn progress_overlay_is_separate_from_exact_source4_and_target5(){
        assert_eq!(SOURCE_SCHEMA,"0870158e2d5568650845dd61f61884384c44315071f0b0dc402d02994a4798da");
        assert_ne!(crate::compiler::base_schema_identity().hex(),SOURCE_SCHEMA);
        assert_eq!(crate::control::SCHEMA_VERSION,5);
        assert!(!crate::control::schema().contains("native_upgrade_progress_v1"));
        surrealdb_syn::parse(OVERLAY_SCHEMA).unwrap();
    }
    #[test]
    fn only_execution_and_verifier_can_change_without_losing_compatible_prefix(){
        let hash=ContentHash::of(b"identity");
        let contract=UpgradeExecutionContract{format:2,source_version:SOURCE_VERSION,kind:TRANSITION_KIND.into(),migration:hash,execution:hash,source:hash,target:hash,generation:hash,overlay:hash,protocol:hash,preflight:hash,verifier:hash,namespace:"project".into(),database:"validation".into()};
        let mut successor=contract.clone();successor.execution=ContentHash::of(b"successor");assert!(successor.compatible(&contract));
        successor.verifier=ContentHash::of(b"verifier");assert!(successor.compatible(&contract));
        successor.preflight=ContentHash::of(b"different-preflight");assert!(!successor.compatible(&contract));
        successor=contract.clone();successor.database="main".into();assert!(!successor.compatible(&contract));
        let mut value=serde_json::to_value(&contract).unwrap();value["unknown"]=serde_json::json!(true);assert!(serde_json::from_value::<UpgradeExecutionContract>(value).is_err());
    }
    #[test]
    fn current_contract_has_only_the_explicit_adjacent_source4_transition(){
        let hash=ContentHash::of(b"identity");
        let contract=UpgradeExecutionContract{format:2,source_version:SOURCE_VERSION,kind:TRANSITION_KIND.into(),migration:hash,execution:hash,source:hash,target:hash,generation:hash,overlay:hash,protocol:hash,preflight:hash,verifier:hash,namespace:"project".into(),database:"validation".into()};
        assert_eq!(contract.source_version,4);assert_eq!(contract.kind,"history_page_receipts_v5");
        let mut old=serde_json::to_value(contract).unwrap();old["format"]=serde_json::json!(1);old.as_object_mut().unwrap().remove("source_version");old.as_object_mut().unwrap().remove("kind");assert!(serde_json::from_value::<UpgradeExecutionContract>(old).is_err());
    }

}
