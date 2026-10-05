//! Model-declared earlier vocabulary views never acquire authority from a later row's existence.
use super::{Error, GenerationId};
use lctx_model::domain::{
    Relation, ValidationInput,
    resources::ResourceBudget,
    stages::{PrefixOrdinal, is_vocabulary},
};
use sqlx::PgConnection;

pub(super) struct Scope {
    pub upper: Option<PrefixOrdinal>,
    pub candidate: Option<PrefixOrdinal>,
}
pub(super) async fn physical(
    tx: &mut PgConnection,
    g: GenerationId,
    input: &ValidationInput,
    relation: &Relation,
    fallback: &str,
    scope: Scope,
    _budget: &ResourceBudget,
) -> Result<String, Error> {
    let Some(boundary) = input.prefix() else {
        return Ok(fallback.to_owned());
    };
    if !is_vocabulary(input.name()) || input.name() != relation.name() {
        return Err(Error::Contract);
    }
    let order = super::vocabulary::publication_order(tx, g).await?;
    let prefix = order.resolve(boundary)?;
    if let Some(upper) = scope.upper {
        order.validate(upper)?;
        if prefix.ordinal() > upper.ordinal() {
            return Err(Error::Contract);
        }
    }
    let physical = super::vocabulary::physical(input.name(), prefix);
    if scope.candidate != Some(prefix) {
        let closed:Option<bool>=sqlx::query_scalar("SELECT closed FROM lctx_model_store.publication_groups WHERE generation_id=$1 AND epoch=$2")
            .bind(g.0.to_vec()).bind(i16::try_from(prefix.ordinal()).map_err(|_|Error::Contract)?).fetch_optional(&mut *tx).await?;
        if closed != Some(true) {
            return Err(Error::Contract);
        }
        let acknowledged: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM lctx_model_store.epoch_receipts WHERE generation_id=$1 AND epoch=$2 AND relation_name=$3)")
            .bind(g.0.to_vec()).bind(i16::try_from(prefix.ordinal()).map_err(|_|Error::Contract)?)
            .bind(input.name()).fetch_one(&mut *tx).await?;
        if !acknowledged { return Err(Error::Contract); }
    }
    Ok(physical)
}
