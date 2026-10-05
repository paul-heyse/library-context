//! Stored authored declarations are the mechanical lowering of a validated, digest-bound catalog.
use super::*;
use crate::domain::{
    Invariant, InvariantCheck, Relation, ValidationInput, normalized::Rows,
    resources::ResourceBudget,
};

pub struct CatalogRecords {
    pub catalogs: Rows<ModelCatalog>,
    pub targets: Rows<AuthoredTarget>,
    pub models: Rows<AuthoredModel>,
    pub protocols: Rows<AuthoredContextProtocol>,
}
impl CatalogRecords {
    pub fn new(budget: &ResourceBudget) -> Self {
        Self {
            catalogs: Rows::new(budget),
            targets: Rows::new(budget),
            models: Rows::new(budget),
            protocols: Rows::new(budget),
        }
    }
    pub fn insert(&mut self, catalog: Catalog) -> Result<(), ModelError> {
        self.insert_borrowed(&catalog)
    }
    /// Lower the admitted immutable selection without cloning its parsed declaration tree.
    pub fn insert_borrowed(&mut self, catalog: &Catalog) -> Result<(), ModelError> {
        for model in &catalog.models {
            self.targets.insert(model.model.target.declaration())?;
            self.models.insert(model.declaration.clone())?;
        }
        for protocol in &catalog.context_protocols {
            for target in [
                &protocol.model.target,
                &protocol.model.allocation,
                &protocol.model.initialization,
            ] {
                self.targets.insert(target.declaration())?;
            }
            self.protocols.insert(protocol.declaration.clone())?;
        }
        self.catalogs.insert(catalog.declaration.clone())?;
        Ok(())
    }
}
pub fn relations() -> Vec<Relation> {
    vec![
        Relation::of::<ModelCatalog>(),
        Relation::of::<AuthoredTarget>(),
        Relation::of::<AuthoredModel>(),
        Relation::of::<AuthoredContextProtocol>(),
    ]
}
pub fn invariants() -> Vec<Invariant> {
    vec![Invariant {
        revision: 1,
        name: "authored_model_catalog",
        inputs: vec![
            ValidationInput::of::<ModelCatalog>(&["id"]),
            ValidationInput::of::<AuthoredTarget>(&["id"]),
            ValidationInput::of::<AuthoredModel>(&["id"]),
            ValidationInput::of::<AuthoredContextProtocol>(&["id"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(Check {
                rows: CatalogRecords::new(budget),
                budget: budget.clone(),
            })
        }),
    }]
}
struct Check {
    rows: CatalogRecords,
    budget: ResourceBudget,
}
impl InvariantCheck for Check {
    fn visit(&mut self, name: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        match name {
            ModelCatalog::NAME => self.rows.catalogs.decode(batch),
            AuthoredTarget::NAME => self.rows.targets.decode(batch),
            AuthoredModel::NAME => self.rows.models.decode(batch),
            AuthoredContextProtocol::NAME => self.rows.protocols.decode(batch),
            _ => Err(ModelError::Invalid(
                "unexpected authored catalog input".into(),
            )),
        }
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        let mut expected = CatalogRecords::new(&self.budget);
        for row in self.rows.catalogs.iter() {
            let _parse = self.budget.reserve(
                "authored-catalog-parse",
                row.source.len().saturating_mul(32).saturating_add(65536),
            )?;
            expected.insert(
                Catalog::parse(&row.source_name, &row.source).map_err(ModelError::Invalid)?,
            )?;
        }
        if !expected.targets.same(&self.rows.targets)
            || !expected.models.same(&self.rows.models)
            || !expected.protocols.same(&self.rows.protocols)
        {
            return Err(ModelError::Invalid(
                "authored declarations differ from their catalog".into(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Batch;
    fn check(rows: &CatalogRecords, budget: &ResourceBudget) -> Result<(), ModelError> {
        let model = crate::domain::ValidatedModel::declared(relations())?;
        let mut check = (invariants()[0].create)(budget);
        macro_rules! feed {
            ($field:ident,$ty:ty) => {
                let batch =
                    Batch::<$ty>::new(&model, rows.$field.iter().cloned().collect(), budget)?;
                check.visit(<$ty>::NAME, batch.arrow())?;
            };
        }
        feed!(catalogs, ModelCatalog);
        feed!(targets, AuthoredTarget);
        feed!(models, AuthoredModel);
        feed!(protocols, AuthoredContextProtocol);
        check.finish()
    }
    #[test]
    fn stored_catalog_refuses_forged_missing_and_foreign_declarations() {
        let budget = ResourceBudget::fixed(16 * 1024 * 1024).unwrap();
        let mut rows = CatalogRecords::new(&budget);
        rows.insert(Catalog::committed().unwrap()).unwrap();
        check(&rows, &budget).unwrap();
        let mut extra = rows.models.iter().next().unwrap().clone();
        extra.revision += 1;
        rows.models.insert(extra).unwrap();
        assert!(check(&rows, &budget).is_err());
        let mut rows = CatalogRecords::new(&budget);
        rows.insert(Catalog::committed().unwrap()).unwrap();
        rows.protocols = Rows::new(&budget);
        assert!(check(&rows, &budget).is_err());
        let mut rows = CatalogRecords::new(&budget);
        rows.insert(Catalog::committed().unwrap()).unwrap();
        rows.targets
            .insert(AuthoredTarget::Stdlib {
                python: "3.14.7".into(),
                module: "builtins".into(),
                callable: "foreign".into(),
            })
            .unwrap();
        assert!(check(&rows, &budget).is_err());
        let mut catalog = Catalog::committed().unwrap().declaration;
        catalog.source.push('\n');
        assert!(catalog.validate().is_err());
    }
}

pub(crate) fn invariants_refs() -> Vec<&'static str> { vec!["authored_model_catalog"] }
