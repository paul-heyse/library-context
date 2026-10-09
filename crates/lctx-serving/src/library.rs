//! Exact first-party capture admission and collection metadata, resolved per request.
use lctx_model::domain::{
    ModelError,
    resources::ResourceBudget,
    serving::{LibraryAdmissionData, LibraryDomainPacket, Name, PreparedLibraryDomains},
};
use lctx_surrealdb::NativeReader;
use surrealdb::types::{RecordId, Variables};
pub async fn resolve(
    reader: &NativeReader,
    name: Option<&Name>,
    budget: &ResourceBudget,
) -> Result<Vec<LibraryDomainPacket>, ModelError> {
    let mut vars = Variables::new();
    vars.insert("name", name.map(|n| n.as_str().to_owned()));
    let roots: Vec<RecordId> = reader
        .query("RETURN fn::lctx_library_roots($name);", vars)
        .await?;
    let batches =
        crate::scope::hydrate(reader, roots, &LibraryAdmissionData::inputs(), budget).await?;
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
