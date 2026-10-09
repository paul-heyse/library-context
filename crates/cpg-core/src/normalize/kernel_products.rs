//! Selected kernel products substitute only after fresh complete membership/content discovery.
use super::*;
use lctx_model::domain::{compilation_product::*, normalized::{callable_normalization, callable_aspects}};
use lctx_surrealdb::surrealdb::types::Value;

/// Optional acceleration may refuse retention without changing ordinary kernel feasibility.
/// Query/ownership failures still propagate; only a resource-shaped token preparation refusal
/// conservatively falls back to current fresh computation.
pub(super) async fn selected_domain(
    selected: &crate::consumed_rows::PreparedRootBatch, partition: usize,
    root: crate::consumed_rows::PreparedRoot, inputs: &[ValidationInput],
    program: ContentHash, runtime: &Workspace,
) -> Result<Option<ContentHash>, ModelError> {
    match selected.content_domain(partition, root, inputs, program, runtime).await {
        Ok(value) => Ok(value),
        Err(error) if matches!(error.primary(), Some(ModelError::Resource { .. } | ModelError::Limit { .. })) => {
            tracing::debug!(%error, "optional selected product preparation unavailable");
            Ok(None)
        }
        Err(error) => Err(error),
    }
}

pub(super) fn entity_request(output: &ProducerOutput, domain: ContentHash,
    kernel: entity_normalization::EntityKernel) -> Result<ProductRequest, ModelError> {
    let mut request = output.product_request()?;
    let name = match kernel {
        entity_normalization::EntityKernel::Symbol => "symbol",
        entity_normalization::EntityKernel::SyntaxField => "syntax-field",
        entity_normalization::EntityKernel::Public => "public",
        entity_normalization::EntityKernel::Enumeration => "enumeration",
    };
    request.operation = format!("normalized-entity-kernel/{name}");
    request.parameters = name.as_bytes().to_vec();
    request.dependencies = vec![DependencyToken { kind: DependencyKind::CompleteDomain,
        role: "complete-selected-entity-inputs".into(), relation: "entity-scope-domain".into(),
        prefix: None, identity: domain }];
    request.outputs = entity_relations(kernel).into_iter().map(|relation| relation.name().into()).collect();
    request.validate()?;
    Ok(request)
}
fn public_only(kernel: entity_normalization::EntityKernel) -> bool {
    matches!(kernel, entity_normalization::EntityKernel::Public | entity_normalization::EntityKernel::Enumeration)
}
fn included(field: &str, kernel: entity_normalization::EntityKernel) -> bool {
    !public_only(kernel) || matches!(field, "exposures" | "public_enumerations" | "exposure_candidates")
}
fn entity_relations(kernel: entity_normalization::EntityKernel) -> Vec<Relation> {
    let mut relations = Vec::new();
    macro_rules! inventory {($($field:ident:$ty:ty,)*) => {$(if included(stringify!($field), kernel) { relations.push(Relation::of::<$ty>()); })*};}
    lctx_model::normalized_entity_outputs!(inventory);
    relations
}
pub(super) async fn entity_hit(request: &ProductRequest, runtime: &Workspace)
    -> Result<Option<entity_normalization::EntityOutput>, ModelError> {
    let Some(cache) = runtime.product_cache()? else { return Ok(None); };
    let lookup = request.clone(); let read = cache.clone(); let budget = runtime.budget().clone();
    let entry = runtime.native_call(async move { read.lookup(&lookup, &budget).await }).await?;
    let Some(entry) = entry else { return Ok(None); };
    let decode = (|| {
        let product = entry.product();
        crate::workspace::validate_product_rows(product, runtime.model(), runtime.budget(), resources::TRANSFER_ROWS)?;
        if product.outcome != ProductOutcome::Complete {
            return Err(ModelError::Conflict("entity kernel product outcome"));
        }
        let mut rows = entity_normalization::EntityOutput::new(runtime.budget());
        for section in &product.sections {
            let relation = runtime.model().relation(&section.name).ok_or(ModelError::Schema("entity product relation"))?;
            let values: Vec<Value> = serde_json::from_slice(&section.bytes).map_err(ModelError::codec)?;
            if section.rows != values.len() as u64 { return Err(ModelError::Conflict("entity product row count")); }
            for window in values.chunks(resources::TRANSFER_ROWS) {
                let batch = lctx_surrealdb::codec::decode_bodies(relation, window.to_vec(), runtime.budget())?;
                if !rows.visit(&section.name, &batch)? { return Err(ModelError::Schema("entity product output inventory")); }
            }
        }
        Ok(rows)
    })();
    match decode {
        Ok(rows) => { tracing::debug!(operation = %request.operation, "selected entity kernel reused before rich hydration"); Ok(Some(rows)) }
        Err(error) if matches!(error.primary(), Some(ModelError::Resource { .. } | ModelError::Limit { .. })) => {
            tracing::debug!(%error, "optional selected product decoding unavailable");
            Ok(None)
        }
        Err(error) => {
            tracing::warn!(%error, "invalid selected entity product; computing fresh");
            drop(entry); let stale = request.clone();
            runtime.native_call(async move { cache.invalidate(&stale).await }).await?;
            Ok(None)
        }
    }
}
fn section<R: Record>(rows: &Rows<R>) -> Result<ProductSection, ModelError> {
    let mut bodies = Vec::new();
    let mut input = rows.iter();
    loop {
        let window = input.by_ref().take(resources::TRANSFER_ROWS).cloned().collect::<Vec<_>>();
        if window.is_empty() { break; }
        let batch = R::encode(&window)?;
        let relation = Relation::of::<R>();
        let mut encoded = lctx_surrealdb::codec::batch_bodies(&relation, &batch)?;
        for (row, body) in window.iter().zip(&mut encoded) {
            let Value::Object(fields) = body else { return Err(ModelError::Schema("entity product canonical body")); };
            fields.insert("id", hex::encode(row.id().bytes()));
        }
        bodies.extend(encoded);
    }
    Ok(ProductSection { name: R::NAME.into(), rows: bodies.len() as u64,
        bytes: serde_json::to_vec(&bodies).map_err(ModelError::codec)? })
}
pub(super) async fn retain_entity(request: ProductRequest, rows: &entity_normalization::EntityOutput,
    kernel: entity_normalization::EntityKernel, runtime: &Workspace) -> Result<(), ModelError> {
    let Some(cache) = runtime.product_cache()? else { return Ok(()); };
    let mut bytes = 4096usize;
    macro_rules! allowance {($($field:ident:$ty:ty,)*) => {$(if included(stringify!($field), kernel) {
        bytes = rows.$field.iter().fold(bytes, |total, row| total.saturating_add(row.row_bytes().saturating_mul(16))).saturating_add(4096);
    })*};}
    lctx_model::normalized_entity_outputs!(allowance);
    let Ok(charge) = runtime.budget().reserve("optional-selected-entity-product", bytes) else { return Ok(()); };
    let mut sections = Vec::new();
    macro_rules! encode {($($field:ident:$ty:ty,)*) => {$(if included(stringify!($field), kernel) { sections.push(section(&rows.$field)?); })*};}
    lctx_model::normalized_entity_outputs!(encode);
    sections.sort_by(|a, b| a.name.cmp(&b.name));
    let product = PortableProduct { request, outcome: ProductOutcome::Complete, sections };
    let budget = runtime.budget().clone();
    runtime.native_call(async move { let _charge = charge; cache.insert(&product, &budget).await.map(|_| ()) }).await
}

pub(super) fn callable_request(output: &ProducerOutput, domain: ContentHash,
    kernel: callable_scope::Kernel) -> Result<ProductRequest, ModelError> {
    let mut request = output.product_request()?;
    let name = match kernel {
        callable_scope::Kernel::Callable => "callable",
        callable_scope::Kernel::Signature => "signature",
        callable_scope::Kernel::Overload => "overload",
    };
    request.operation = format!("normalized-callable-kernel/{name}");
    request.parameters = name.as_bytes().to_vec();
    request.dependencies = vec![DependencyToken { kind: DependencyKind::CompleteDomain,
        role: "complete-selected-callable-inputs".into(), relation: "callable-scope-domain".into(),
        prefix: None, identity: domain }];
    let mut outputs = std::collections::BTreeSet::new();
    macro_rules! inventory {($($field:ident:$ty:ty,)*) => {$(outputs.insert(<$ty>::NAME.into());)*};}
    lctx_model::normalized_callable_outputs!(inventory);
    request.outputs = outputs;
    request.validate()?;
    Ok(request)
}
pub(super) async fn callable_hit(request: &ProductRequest, runtime: &Workspace)
    -> Result<Option<callable_normalization::CallableOutput>, ModelError> {
    let Some(cache) = runtime.product_cache()? else { return Ok(None); };
    let lookup = request.clone(); let read = cache.clone(); let budget = runtime.budget().clone();
    let entry = runtime.native_call(async move { read.lookup(&lookup, &budget).await }).await?;
    let Some(entry) = entry else { return Ok(None); };
    let decode = (|| {
        let product = entry.product();
        crate::workspace::validate_product_rows(product, runtime.model(), runtime.budget(), resources::TRANSFER_ROWS)?;
        if product.outcome != ProductOutcome::Complete {
            return Err(ModelError::Conflict("callable kernel product outcome"));
        }
        let mut rows = callable_normalization::CallableOutput::new(runtime.budget());
        for section in &product.sections {
            let relation = runtime.model().relation(&section.name).ok_or(ModelError::Schema("callable product relation"))?;
            let values: Vec<Value> = serde_json::from_slice(&section.bytes).map_err(ModelError::codec)?;
            if section.rows != values.len() as u64 { return Err(ModelError::Conflict("callable product row count")); }
            for window in values.chunks(resources::TRANSFER_ROWS) {
                let batch = lctx_surrealdb::codec::decode_bodies(relation, window.to_vec(), runtime.budget())?;
                if !rows.visit(&section.name, &batch)? { return Err(ModelError::Schema("callable product output inventory")); }
            }
        }
        Ok(rows)
    })();
    match decode {
        Ok(rows) => { tracing::debug!(operation = %request.operation, "selected callable kernel reused before rich hydration"); Ok(Some(rows)) }
        Err(error) if matches!(error.primary(), Some(ModelError::Resource { .. } | ModelError::Limit { .. })) => {
            tracing::debug!(%error, "optional selected product decoding unavailable");
            Ok(None)
        }
        Err(error) => {
            tracing::warn!(%error, "invalid selected callable product; computing fresh");
            drop(entry); let stale = request.clone();
            runtime.native_call(async move { cache.invalidate(&stale).await }).await?;
            Ok(None)
        }
    }
}
pub(super) async fn retain_callable(request: ProductRequest, rows: &callable_normalization::CallableOutput,
    runtime: &Workspace) -> Result<(), ModelError> {
    let Some(cache) = runtime.product_cache()? else { return Ok(()); };
    let mut bytes = 4096usize;
    macro_rules! allowance {($($field:ident:$ty:ty,)*) => {$(
        bytes = rows.$field.iter().fold(bytes, |total, row| total.saturating_add(row.row_bytes().saturating_mul(16))).saturating_add(4096);
    )*};}
    lctx_model::normalized_callable_outputs!(allowance);
    let Ok(charge) = runtime.budget().reserve("optional-selected-callable-product", bytes) else { return Ok(()); };
    let mut sections = Vec::new();
    macro_rules! encode {($($field:ident:$ty:ty,)*) => {$(sections.push(section(&rows.$field)?);)*};}
    lctx_model::normalized_callable_outputs!(encode);
    sections.sort_by(|a, b| a.name.cmp(&b.name));
    let product = PortableProduct { request, outcome: ProductOutcome::Complete, sections };
    let budget = runtime.budget().clone();
    runtime.native_call(async move { let _charge = charge; cache.insert(&product, &budget).await.map(|_| ()) }).await
}

pub(super) fn aspect_request(output: &ProducerOutput, domain: ContentHash,
    index: usize) -> Result<ProductRequest, ModelError> {
    let mut request = output.product_request()?;
    let name = match index {
        0 => "assessment", 1 => "field", 2 => "class",
        _ => return Err(ModelError::Schema("aspect product kernel index")),
    };
    request.operation = format!("normalized-aspect-kernel/{name}");
    request.parameters = name.as_bytes().to_vec();
    request.dependencies = vec![DependencyToken { kind: DependencyKind::CompleteDomain,
        role: "complete-selected-aspect-inputs".into(), relation: "aspect-scope-domain".into(),
        prefix: None, identity: domain }];
    let mut outputs = std::collections::BTreeSet::new();
    macro_rules! inventory {($($field:ident:$ty:ty,)*) => {$(outputs.insert(<$ty>::NAME.into());)*};}
    lctx_model::callable_aspect_outputs!(inventory);
    request.outputs = outputs;
    request.validate()?;
    Ok(request)
}
pub(super) async fn aspect_hit(request: &ProductRequest, runtime: &Workspace)
    -> Result<Option<callable_aspects::AspectOutput>, ModelError> {
    let Some(cache) = runtime.product_cache()? else { return Ok(None); };
    let lookup = request.clone(); let read = cache.clone(); let budget = runtime.budget().clone();
    let entry = runtime.native_call(async move { read.lookup(&lookup, &budget).await }).await?;
    let Some(entry) = entry else { return Ok(None); };
    let decode = (|| {
        let product = entry.product();
        crate::workspace::validate_product_rows(product, runtime.model(), runtime.budget(), resources::TRANSFER_ROWS)?;
        if product.outcome != ProductOutcome::Complete {
            return Err(ModelError::Conflict("aspect kernel product outcome"));
        }
        let mut rows = callable_aspects::AspectOutput::new(runtime.budget());
        for section in &product.sections {
            let relation = runtime.model().relation(&section.name).ok_or(ModelError::Schema("aspect product relation"))?;
            let values: Vec<Value> = serde_json::from_slice(&section.bytes).map_err(ModelError::codec)?;
            if section.rows != values.len() as u64 { return Err(ModelError::Conflict("aspect product row count")); }
            for window in values.chunks(resources::TRANSFER_ROWS) {
                let batch = lctx_surrealdb::codec::decode_bodies(relation, window.to_vec(), runtime.budget())?;
                if !rows.visit(&section.name, &batch)? { return Err(ModelError::Schema("aspect product output inventory")); }
            }
        }
        Ok(rows)
    })();
    match decode {
        Ok(rows) => { tracing::debug!(operation = %request.operation, "selected aspect kernel reused before rich hydration"); Ok(Some(rows)) }
        Err(error) if matches!(error.primary(), Some(ModelError::Resource { .. } | ModelError::Limit { .. })) => {
            tracing::debug!(%error, "optional selected product decoding unavailable");
            Ok(None)
        }
        Err(error) => {
            tracing::warn!(%error, "invalid selected aspect product; computing fresh");
            drop(entry); let stale = request.clone();
            runtime.native_call(async move { cache.invalidate(&stale).await }).await?;
            Ok(None)
        }
    }
}
pub(super) async fn retain_aspect(request: ProductRequest, rows: &callable_aspects::AspectOutput,
    runtime: &Workspace) -> Result<(), ModelError> {
    let Some(cache) = runtime.product_cache()? else { return Ok(()); };
    let mut bytes = 4096usize;
    macro_rules! allowance {($($field:ident:$ty:ty,)*) => {$(
        bytes = rows.$field.iter().fold(bytes, |total, row| total.saturating_add(row.row_bytes().saturating_mul(16))).saturating_add(4096);
    )*};}
    lctx_model::callable_aspect_outputs!(allowance);
    let Ok(charge) = runtime.budget().reserve("optional-selected-aspect-product", bytes) else { return Ok(()); };
    let mut sections = Vec::new();
    macro_rules! encode {($($field:ident:$ty:ty,)*) => {$(sections.push(section(&rows.$field)?);)*};}
    lctx_model::callable_aspect_outputs!(encode);
    sections.sort_by(|a, b| a.name.cmp(&b.name));
    let product = PortableProduct { request, outcome: ProductOutcome::Complete, sections };
    let budget = runtime.budget().clone();
    runtime.native_call(async move { let _charge = charge; cache.insert(&product, &budget).await.map(|_| ()) }).await
}
