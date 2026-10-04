//! Inspection of identity-row serving views/indexes generated from the same finite inventory.
use super::{Error, GenerationLease, quoted};
use lctx_model::domain::{ContentHash, admission::Frontier, serving::mappings};
fn compact(sql: &str) -> String {
    sql.chars()
        .filter(|c| !c.is_whitespace() && *c != '"')
        .collect()
}
impl GenerationLease {
    pub(super) async fn serving_shape(&mut self) -> Result<(), Error> {
        if !Frontier::Catalog
            .descriptor()
            .relations(&self.model)
            .is_ok_and(|catalog| catalog.is_subset(&self.relations))
        {
            return Ok(());
        }
        let schema = self.generation().schema();
        for mapping in mappings::inventory() {
            if !mapping
                .dependencies
                .iter()
                .all(|name| self.relations.contains(name))
            {
                continue;
            }
            let source = mapping.source.name();
            let select = super::physical_columns::projection(&mapping.source);
            let expected = format!(
                "SELECT {select} FROM {}.{};",
                quoted(&schema),
                quoted(source)
            );
            let name = format!("{}.{}", quoted(&schema), quoted(mapping.name));
            let row: Option<(String,bool,bool)> = sqlx::query_as("SELECT pg_get_viewdef(c.oid,false),has_table_privilege(current_user,c.oid,'SELECT'),has_table_privilege(current_user,c.oid,'INSERT,UPDATE,DELETE,TRUNCATE,REFERENCES,TRIGGER') FROM pg_class c WHERE c.oid=to_regclass($1) AND c.relkind='v'")
                .bind(name).fetch_optional(&mut *self.connection).await?;
            let Some((view, readable, writable)) = row else {
                return Err(Error::Contract);
            };
            if compact(&view) != compact(&expected) || !readable || writable {
                return Err(Error::Codec(format!(
                    "serving view {} differs: expected {expected:?}, actual {view:?}, read={readable}, write={writable}",
                    mapping.name
                )));
            }
            for key in mapping.lookup_keys.iter().filter(|key| **key != "id") {
                let index = format!(
                    "serving_{}",
                    &ContentHash::of(format!("{source}/{key}").as_bytes()).hex()[..24]
                );
                let actual: Option<String> = sqlx::query_scalar("SELECT pg_get_indexdef(c.oid) FROM pg_class c WHERE c.oid=to_regclass($1) AND c.relkind='i'")
                    .bind(format!("{}.{}",quoted(&schema),quoted(&index))).fetch_optional(&mut *self.connection).await?;
                let expected = format!(
                    "CREATE INDEX {} ON {}.{} USING btree (generation_id, {})",
                    quoted(&index),
                    quoted(&schema),
                    quoted(source),
                    quoted(key)
                );
                if actual.as_deref().map(compact) != Some(compact(&expected)) {
                    return Err(Error::Codec(format!(
                        "serving index {index} differs: expected {expected:?}, actual {actual:?}"
                    )));
                }
            }
        }
        Ok(())
    }
}
