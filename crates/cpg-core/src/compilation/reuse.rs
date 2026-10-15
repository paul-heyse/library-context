//! Exact owner request bindings. Computation payload cutoffs belong to their explicit adapters;
//! an attempt registry only rejects contradictory role/epoch bindings and emits disposition.
use lctx_model::domain::{
    ContentHash, Key, KeySink, ModelError,
    charged::StateCharge,
    compilation_product::{DependencyToken, ProductRequest},
    resources::ResourceBudget,
};
use std::collections::BTreeMap;
#[derive(Clone, Copy)]
pub(super) enum Eligibility {
    Rows,
    Fresh(&'static str),
}
impl Eligibility {
    pub(super) fn reason(self) -> Option<&'static str> {
        match self {
            Self::Rows => None,
            Self::Fresh(reason) => Some(reason),
        }
    }
}
/// Retain the exact binding of each actual owner once, without a second dependency graph.
pub(super) struct SelectedBindings {
    owners: BTreeMap<String, (ContentHash, ContentHash)>,
    charge: StateCharge,
}
impl SelectedBindings {
    pub(super) fn new(budget: &ResourceBudget) -> Result<Self, ModelError> {
        let mut charge = StateCharge::new(budget, "compiler-selected-product-bindings");
        charge.grow(size_of::<Self>())?;
        Ok(Self {
            owners: BTreeMap::new(),
            charge,
        })
    }
    pub(super) fn bind_selected(
        &mut self,
        request: &ProductRequest,
        selected: &[DependencyToken],
    ) -> Result<(), ModelError> {
        if request.dependencies.len() != selected.len()
            || request
                .dependencies
                .iter()
                .zip(selected)
                .any(|(declared, actual)| {
                    declared.kind != actual.kind
                        || declared.role != actual.role
                        || declared.relation != actual.relation
                        || declared.identity != actual.identity
                        || declared
                            .prefix
                            .as_ref()
                            .is_some_and(|prefix| Some(prefix) != actual.prefix.as_ref())
                })
        {
            return Err(ModelError::Conflict("compiler product selected dependency"));
        }
        let identity = request.identity()?;
        let mut binding = KeySink::new("compiler-product-selected-binding/v1");
        identity.encode(&mut binding);
        for token in selected {
            token.role.encode(&mut binding);
            token.prefix.encode(&mut binding);
        }
        let binding = binding.finish();
        if let Some(previous) = self.owners.get(&request.operation) {
            if *previous != (identity, binding) {
                return Err(ModelError::Conflict(
                    "compiler product bound twice with different premises",
                ));
            }
            return Ok(());
        }
        self.charge.grow(
            size_of::<(String, (ContentHash, ContentHash))>() + request.operation.len() + 64,
        )?;
        self.owners
            .insert(request.operation.clone(), (identity, binding));
        Ok(())
    }
}
#[cfg(test)]
mod controls {
    use super::*;
    use lctx_model::domain::compilation_product::{DependencyKind, ProductKind};
    fn request() -> ProductRequest {
        ProductRequest {
            kind: ProductKind::PureRows,
            operation: "actual-owner".into(),
            model: ContentHash::of(b"model"),
            implementation: ContentHash::of(b"implementation"),
            policy: ContentHash::of(b"policy"),
            result_contract: ContentHash::of(b"result"),
            configuration: None,
            profile: "catalog".into(),
            parameters: vec![],
            outputs: std::collections::BTreeSet::from(["output".into()]),
            dependencies: vec![DependencyToken {
                kind: DependencyKind::ExactView,
                role: "input/0".into(),
                relation: "source".into(),
                prefix: None,
                identity: ContentHash::of(b"selected source"),
            }],
        }
    }
    #[test]
    fn actual_selected_epoch_is_bound_once_and_foreign_roles_are_rejected() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let request = request();
        let mut selected = request.dependencies.clone();
        selected[0].prefix = Some("local".into());
        let mut bindings = SelectedBindings::new(&budget).unwrap();
        bindings.bind_selected(&request, &selected).unwrap();
        bindings.bind_selected(&request, &selected).unwrap();
        selected[0].prefix = Some("model".into());
        assert!(bindings.bind_selected(&request, &selected).is_err());
        let mut foreign = request.dependencies.clone();
        foreign[0].role = "input/1".into();
        assert!(bindings.bind_selected(&request, &foreign).is_err());
        let mut explicit = request.clone();
        explicit.dependencies[0].prefix = Some("facts".into());
        assert!(
            SelectedBindings::new(&budget)
                .unwrap()
                .bind_selected(&explicit, &selected)
                .is_err()
        );
    }
}
