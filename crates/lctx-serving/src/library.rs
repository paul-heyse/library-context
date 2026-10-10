//! Exact first-party capture admission and collection metadata, resolved per request.
use lctx_model::domain::{
    ModelError,
    resources::ResourceBudget,
    serving::{LibraryAdmissionData, LibraryDomainPacket, Name, PreparedLibraryDomains},
};
use lctx_surrealdb::NativeReader;
use surrealdb::types::RecordId;
pub async fn resolve(
    reader: &NativeReader,
    name: Option<&Name>,
    budget: &ResourceBudget,
) -> Result<Vec<LibraryDomainPacket>, ModelError> {
    resolve_inner(reader, name, budget, None).await
}
pub(crate) async fn resolve_prepared(
    reader: &NativeReader,
    name: Option<&Name>,
    budget: &ResourceBudget,
    preparation: &crate::preparation::Preparation<'_>,
) -> Result<Vec<LibraryDomainPacket>, ModelError> {
    resolve_inner(reader, name, budget, Some(preparation)).await
}
async fn resolve_inner(
    reader: &NativeReader,
    name: Option<&Name>,
    budget: &ResourceBudget,
    preparation: Option<&crate::preparation::Preparation<'_>>,
) -> Result<Vec<LibraryDomainPacket>, ModelError> {
    let mut vars = reader.view_bindings();
    vars.insert("name", name.map(|n| n.as_str().to_owned()));
    let roots: Vec<RecordId> = reader
        .query(
            format!(
                "RETURN {}($name, $lctx_views);",
                reader.handle().library_roots_function()
            ),
            vars,
        )
        .await?;
    let cached;
    let fresh;
    let batches = if let Some(preparation) = preparation {
        cached = preparation
            .hydrate(
                reader,
                roots,
                &LibraryAdmissionData::inputs(),
                &LibraryAdmissionData::inputs(),
                crate::scope::OWNED_FIELDS,
            )
            .await?;
        &*cached
    } else {
        fresh =
            crate::scope::hydrate(reader, roots, &LibraryAdmissionData::inputs(), budget).await?;
        &fresh
    };
    let mut data = LibraryAdmissionData::new(budget);
    for (name, batch) in &batches.batches {
        data.visit(name, batch)?;
    }
    let prepared = PreparedLibraryDomains::prepare(&data, budget)?;
    let resolved = prepared.resolve(name).map_err(|_| {
        ModelError::Serving(lctx_model::domain::serving::FailureKind::UnknownLibrary)
    })?;
    Ok(resolved.metadata(budget)?.domains)
}
pub fn native_definitions() -> &'static str {
    lctx_surrealdb::materialization::library_definitions()
}

#[cfg(test)]
mod blueprint_controls {
    #[test]
    fn actual_serving_blueprint_is_accepted_by_native_publication() {
        lctx_surrealdb::materialization::validate_native_definitions(&crate::native_definitions())
            .unwrap();
    }
}
