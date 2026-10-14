//! Optional E0 computation payloads. Current exact frames, source receipts and embedding uses
//! remain owned by retrieval and are validated against the current selected data on every hit.
use crate::workspace::{ProducerOutput, ReservedProductRequest, Workspace};
use lctx_model::domain::{
    compilation_product::*,
    retrieval::build::{Data, Output},
    *,
};
use lctx_native::surrealdb::types::Value;

pub(super) fn request(
    data: &Data,
    output: &ProducerOutput,
    domain: ContentHash,
    runtime: &Workspace,
) -> Result<Option<ReservedProductRequest>, ModelError> {
    if runtime.product_cache()?.is_none() {
        return Ok(None);
    }
    let hold = match runtime.budget().reserve("retrieval-payload-request", 65536) {
        Ok(hold) => hold,
        Err(ModelError::Resource { .. }) => return Ok(None),
        Err(error) => return Err(error),
    };
    let mut selected = KeySink::new("retrieval-payload-selected-inputs/v1");
    domain.encode(&mut selected);
    let domain = selected.finish();
    let dependencies = match data.value_dependencies(domain, runtime.budget()) {
        Ok(dependencies) => dependencies,
        Err(error)
            if matches!(
                error.primary(),
                Some(ModelError::Resource { .. } | ModelError::Limit { .. })
            ) =>
        {
            return Ok(None);
        }
        Err(error) => return Err(error),
    };
    let mut result = KeySink::new("retrieval-payload-output-contract/v1");
    let mut outputs = std::collections::BTreeSet::new();
    macro_rules! inventory {($($field:ident:$ty:ty,)*)=>{$(result.part(b"record",<$ty>::CONTRACT.as_bytes());outputs.insert(<$ty>::NAME.into());)*};}
    lctx_model::retrieval_outputs!(inventory);
    let (parameters, definition) = lctx_model::domain::retrieval::build::definition();
    let mut configuration = KeySink::new("retrieval-payload-selected-method/v1");
    parameters.content_digest().encode(&mut configuration);
    definition.content_digest().encode(&mut configuration);
    for (parameters, definition) in [
        lctx_model::domain::catalog::evidence::build::definition(),
        lctx_model::domain::synthesis::build::definition(),
    ] {
        parameters.content_digest().encode(&mut configuration);
        definition.content_digest().encode(&mut configuration);
    }
    let request = ProductRequest {
        kind: ProductKind::PureValue,
        operation: "retrieval-render-payload/v1".into(),
        model: runtime.model().digest(),
        implementation: output.implementation_identity(),
        policy: ContentHash::of(include_bytes!("retrieval_products.rs")),
        result_contract: result.finish(),
        configuration: Some(configuration.finish()),
        profile: "neutral-computation".into(),
        parameters: vec![],
        dependencies,
        outputs,
    };
    request.validate()?;
    Ok(Some((request, hold)))
}
pub(super) async fn lookup(
    request: &ProductRequest,
    data: &Data,
    runtime: &Workspace,
) -> Result<Option<Output>, ModelError> {
    if !runtime.product_reuse_enabled() {
        return Ok(None);
    }
    let Some(cache) = runtime.product_cache()? else {
        return Ok(None);
    };
    let hold = match runtime.budget().reserve(
        "retrieval-payload-lookup-request",
        request.retained_bytes()?,
    ) {
        Ok(hold) => hold,
        Err(ModelError::Resource { .. }) => return Ok(None),
        Err(error) => return Err(error),
    };
    let owned = request.clone();
    let budget = runtime.budget().clone();
    let entry = runtime
        .native_call(async move {
            let _hold = hold;
            cache.lookup(&owned, &budget).await
        })
        .await?;
    let Some(entry) = entry else { return Ok(None) };
    let result = (|| {
        if entry.product().outcome != ProductOutcome::Complete {
            return Err(ModelError::Conflict("retrieval payload outcome"));
        }
        let candidate = crate::workspace::decode_product_rows(
            entry.product(),
            runtime.model(),
            runtime.budget(),
            resources::TRANSFER_ROWS,
        )?;
        let mut rows = Output::new(runtime.budget());
        for (relation, batches) in candidate.sections() {
            for batch in batches {
                if !rows.visit(relation.name(), batch)? {
                    return Err(ModelError::Conflict("retrieval payload output inventory"));
                }
            }
        }
        rows.verify_completion(data, runtime.budget())?;
        Ok(rows)
    })();
    drop(entry);
    match result {
        Ok(rows) => Ok(Some(rows)),
        Err(error)
            if matches!(
                error.primary(),
                Some(ModelError::Resource { .. } | ModelError::Limit { .. })
            ) =>
        {
            Ok(None)
        }
        Err(error) => {
            tracing::warn!(%error,"invalid retrieval computation payload");
            let cache = runtime
                .product_cache()?
                .ok_or(ModelError::Conflict("catalog cache owner disappeared"))?;
            let request = request.clone();
            runtime
                .native_call(async move { cache.invalidate(&request).await })
                .await?;
            Ok(None)
        }
    }
}
fn section<R: Record>(rows: &normalized::Rows<R>) -> Result<ProductSection, ModelError> {
    let mut bodies = Vec::new();
    let mut input = rows.iter();
    loop {
        let window = input
            .by_ref()
            .take(resources::TRANSFER_ROWS)
            .cloned()
            .collect::<Vec<_>>();
        if window.is_empty() {
            break;
        }
        let batch = R::encode(&window)?;
        let mut encoded = lctx_native::codec::batch_bodies(&Relation::of::<R>(), &batch)?;
        for (row, body) in window.iter().zip(&mut encoded) {
            let Value::Object(fields) = body else {
                return Err(ModelError::Schema("retrieval payload body"));
            };
            fields.insert("id", hex::encode(row.id().bytes()));
        }
        bodies.extend(encoded);
    }
    Ok(ProductSection {
        name: R::NAME.into(),
        rows: bodies.len() as u64,
        bytes: serde_json::to_vec(&bodies).map_err(ModelError::codec)?,
    })
}
pub(super) async fn retain(
    request: &ProductRequest,
    rows: &Output,
    runtime: &Workspace,
) -> Result<(), ModelError> {
    let Some(cache) = runtime.product_cache()? else {
        return Ok(());
    };
    let mut bytes = request
        .retained_bytes()?
        .saturating_mul(2)
        .saturating_add(65536);
    macro_rules! allowance {($($field:ident:$ty:ty,)*)=>{$(bytes=rows.$field.iter().fold(bytes,|total,row|total.saturating_add(row.row_bytes().saturating_mul(32))).saturating_add(8192);)*};}
    lctx_model::retrieval_outputs!(allowance);
    let hold = match runtime
        .budget()
        .reserve("optional-retrieval-payload-encoding", bytes)
    {
        Ok(hold) => hold,
        Err(ModelError::Resource { .. }) => return Ok(()),
        Err(error) => return Err(error),
    };
    let mut sections = Vec::new();
    macro_rules! encode {($($field:ident:$ty:ty,)*)=>{$(sections.push(section(&rows.$field)?);)*};}
    lctx_model::retrieval_outputs!(encode);
    sections.sort_by(|left, right| left.name.cmp(&right.name));
    let product = PortableProduct {
        request: request.clone(),
        outcome: ProductOutcome::Complete,
        sections,
    };
    product.validate()?;
    let budget = runtime.budget().clone();
    runtime
        .native_call(async move {
            let _hold = hold;
            cache.insert(&product, &budget).await.map(|_| ())
        })
        .await
}
