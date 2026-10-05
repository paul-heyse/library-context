//! Late validation of the actual authored definition supporting a pinned assumption universe.
//! This relation is absent from facts/preflight. The producing model publication owns it.
use super::{
    assumptions::{Assumption, AssumptionUniverse},
    charged::{ChargedMap, StateCharge},
    models::{AuthoredModel, ModelCatalog},
    *,
};
use crate::Domain;
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "assumption_universe_supports", rule = "pinned_assumption_universe", conclusion = universe, invariant_refs = invariants_refs)]
pub struct AssumptionUniverseSupport {
    #[model(key)]
    pub universe: Id<AssumptionUniverse>,
    #[model(key, premise)]
    pub catalog: Id<ModelCatalog>,
    #[model(key, premise)]
    pub model: Id<AuthoredModel>,
}
impl AssumptionUniverseSupport {
    /// Caller has the actual late declaration, not just a producer-supplied digest.
    pub fn new(
        universe: &AssumptionUniverse,
        catalog: &ModelCatalog,
        model: &AuthoredModel,
    ) -> Result<Self, ModelError> {
        catalog.validate()?;
        if model.catalog != catalog.id() || universe.model_definition != catalog.content {
            return Err(invalid(
                "assumption universe differs from actual pinned definition",
            ));
        }
        let parsed = super::models::Catalog::parse(&catalog.source_name, &catalog.source)
            .map_err(ModelError::Invalid)?;
        if parsed.declaration() != catalog
            || !parsed.models().iter().any(|m| m.declaration() == model)
        {
            return Err(invalid(
                "universe model differs from actual definition source",
            ));
        }
        Ok(Self {
            universe: universe.id(),
            catalog: catalog.id(),
            model: model.id(),
        })
    }
}
fn invalid(s: &str) -> ModelError {
    ModelError::Invalid(s.into())
}
pub(crate) fn invariants() -> Vec<Invariant> {
    vec![Invariant {
        revision: 1,
        name: "actual_pinned_assumption_universe",
        inputs: vec![
            ValidationInput::of::<AssumptionUniverse>(&["id"]),
            ValidationInput::of::<Assumption>(&["id"]),
            ValidationInput::of::<ModelCatalog>(&["id"]),
            ValidationInput::of::<AuthoredModel>(&["id"]),
            ValidationInput::of::<AssumptionUniverseSupport>(&["id"]),
        ],
        create: std::sync::Arc::new(|b| {
            Box::new(Check {
                charge: StateCharge::new(b, "actual_assumption_universe"),
                universes: Default::default(),
                definitions: Default::default(),
                catalogs: Default::default(),
                models: Default::default(),
                supports: Default::default(),
            })
        }),
    }]
}
struct Check {
    charge: StateCharge,
    universes: ChargedMap<Id<AssumptionUniverse>, AssumptionUniverse>,
    definitions: ChargedMap<Id<Assumption>, Assumption>,
    catalogs: ChargedMap<Id<ModelCatalog>, ModelCatalog>,
    models: ChargedMap<Id<AuthoredModel>, AuthoredModel>,
    supports: ChargedMap<Id<AssumptionUniverseSupport>, AssumptionUniverseSupport>,
}
impl InvariantCheck for Check {
    fn visit(&mut self, n: &str, b: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        macro_rules! rows {
            ($ty:ty, $f:ident) => {
                if n == <$ty>::NAME {
                    for row in <$ty>::decode(b)? {
                        self.$f.insert(&mut self.charge, row.id(), row)?;
                    }
                    return Ok(());
                }
            };
        }
        rows!(AssumptionUniverse, universes);
        rows!(Assumption, definitions);
        rows!(ModelCatalog, catalogs);
        rows!(AuthoredModel, models);
        if n == AssumptionUniverseSupport::NAME {
            for row in AssumptionUniverseSupport::decode(b)? {
                self.supports.insert(&mut self.charge, row.id(), row)?;
            }
            return Ok(());
        }
        Err(invalid("undeclared late universe input"))
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        for support in self.supports.values() {
            let u = self
                .universes
                .get(&support.universe)
                .ok_or_else(|| invalid("pinned universe missing"))?;
            let c = self
                .catalogs
                .get(&support.catalog)
                .ok_or_else(|| invalid("actual universe catalog missing"))?;
            let m = self
                .models
                .get(&support.model)
                .ok_or_else(|| invalid("actual universe model missing"))?;
            let _parse = self
                .charge
                .budget()
                .ok_or_else(|| invalid("universe validation budget missing"))?
                .reserve(
                    "universe-source-parse",
                    c.source.len().saturating_mul(32).saturating_add(65536),
                )?;
            if AssumptionUniverseSupport::new(u, c, m)? != *support {
                return Err(invalid("pinned universe support differs"));
            }
        }
        for assumption in self.definitions.values() {
            if let Assumption::NoExtraOverrides { universe, .. } = assumption
                && !self
                    .supports
                    .values()
                    .any(|support| support.universe == *universe)
            {
                return Err(invalid("actual pinned-universe support missing"));
            }
        }
        Ok(())
    }
}

pub(crate) fn invariants_refs() -> Vec<&'static str> {
    vec!["actual_pinned_assumption_universe"]
}
