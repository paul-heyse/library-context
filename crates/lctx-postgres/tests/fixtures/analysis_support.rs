//! Shared store-fixture transport: collect the actual copied native pairs through the model's
//! production projection, then publish its exact inventory before generation validation.
use lctx_model::domain::{
    analysis::native::NativeInventory, normalized, resources::ResourceBudget, *,
};
use lctx_postgres::{generations::Error, testing::Harness};

#[allow(dead_code, reason = "Shared by targets that declare their own model")]
pub fn model() -> ValidatedModel {
    let mut relations = facts_relations();
    relations.extend(analysis_relations());
    relations.extend(normalized::coverage::relations());
    relations.extend([
        Relation::of::<normalized::entities::CallableEntity>(),
        Relation::of::<normalized::entities::ClassEntity>(),
        Relation::of::<normalized::entities::ParameterEntity>(),
        Relation::of::<normalized::entities::FieldEntity>(),
        Relation::of::<normalized::entities::EntityRef>(),
    ]);
    ValidatedModel::validate(relations).unwrap()
}

pub async fn copy<R: Record>(
    harness: &Harness,
    model: &ValidatedModel,
    rows: Vec<R>,
    inventory: &mut NativeInventory,
    budget: &ResourceBudget,
) -> Result<(), Error> {
    let batch = Batch::new(model, rows, budget)?;
    if NativeInventory::inputs()
        .iter()
        .any(|input| input.name() == R::NAME)
    {
        inventory.visit(R::NAME, batch.arrow())?;
    }
    harness.copy(&batch, budget).await
}

pub async fn finish(
    harness: &Harness,
    model: &ValidatedModel,
    inventory: NativeInventory,
    budget: &ResourceBudget,
) -> Result<(), Error> {
    let projected = inventory.collect()?;
    harness
        .copy(
            &Batch::new(model, projected.premises.iter().cloned().collect(), budget)?,
            budget,
        )
        .await?;
    harness
        .copy(
            &Batch::new(
                model,
                projected.qualifications.iter().cloned().collect(),
                budget,
            )?,
            budget,
        )
        .await
}
