//! Neutral mechanical decoding of canonical graph records. Store and compiler adapters share
//! these closed payload paths; semantic admission remains with the declared model owners.
use super::{Assertion, AssertionValue, Entity};
use crate::domain::{ModelError, Record};
use serde::de::DeserializeOwned;
use serde_json::Value;
macro_rules! entities {
    ($($variant:ident:$ty:ty,)*) => {
        pub fn entity_record<R: Record + DeserializeOwned>(entity: &Entity) -> Result<R, ModelError> {
            let row: R = match entity {
                $(Entity::$variant(row) if R::NAME == <$ty>::NAME =>
                    serde_json::from_value(serde_json::to_value(row).map_err(ModelError::codec)?).map_err(ModelError::codec),)*
                _ => Err(ModelError::Conflict("canonical graph record type")),
            }?;
            row.validate()?;
            Ok(row)
        }
    }
}
crate::graph_entity_records!(entities);
/// AssertionValue's closed wrappers are the only supported payload paths.
fn assertion_value(assertion: &Assertion) -> Result<Value, ModelError> {
    let value = serde_json::to_value(&assertion.value).map_err(ModelError::codec)?;
    match &assertion.value {
        AssertionValue::Acquisition(_) => Ok(value
            .get("Acquisition")
            .ok_or(ModelError::Schema("assertion acquisition payload"))?
            .clone()),
        AssertionValue::Native(_)
        | AssertionValue::Analysis(_)
        | AssertionValue::Support(_)
        | AssertionValue::Provenance(_)
        | AssertionValue::Membership(_)
        | AssertionValue::Claim(_) => {
            let outer = value
                .as_object()
                .and_then(|v| v.values().next())
                .and_then(Value::as_object)
                .ok_or(ModelError::Schema("assertion record payload"))?;
            if outer.len() != 1 {
                return Err(ModelError::Schema("assertion record wrapper"));
            }
            Ok(outer
                .values()
                .next()
                .ok_or(ModelError::Schema("assertion record wrapper"))?
                .clone())
        }
        _ => Err(ModelError::Schema("assertion has no typed record payload")),
    }
}
pub fn assertion_record<R: Record + DeserializeOwned>(
    assertion: &Assertion,
) -> Result<R, ModelError> {
    let source = assertion
        .source
        .as_ref()
        .ok_or(ModelError::Schema("assertion typed source"))?;
    if source.domain() != R::NAME {
        return Err(ModelError::Conflict("canonical assertion record type"));
    }
    let row: R = serde_json::from_value(assertion_value(assertion)?).map_err(ModelError::codec)?;
    row.validate()?;
    if row.id().bytes() != source.bytes() {
        return Err(ModelError::Conflict("canonical assertion record key"));
    }
    Ok(row)
}
