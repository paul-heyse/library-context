//! C0 caches computation payload only. Current exact invocation/coverage/source receipts and
//! member-to-invocation links are constructed by catalog_core for every execution.
use crate::workspace::{ReservedProductRequest, Workspace};
use lctx_model::domain::{
    catalog::build::{CatalogData, CatalogOutput},
    compilation_product::*,
    *,
};
use lctx_native::surrealdb::types::Value;

/// One optional interpretation belongs to one root window. Native calls move it with its
/// grant; pure preparation may abandon it, but neither request construction nor effects replay.
pub(super) struct ProductWindow {
    request: Option<ReservedProductRequest>,
}

impl ProductWindow {
    pub(super) fn new(
        data: &CatalogData,
        implementation: ContentHash,
        domain: ContentHash,
        runtime: &Workspace,
    ) -> Result<Self, ModelError> {
        Ok(Self {
            request: optional_request(
                runtime.product_cache()?.is_some(),
                runtime.budget(),
                || {
                    request(
                        data,
                        implementation,
                        domain,
                        runtime.model().digest(),
                        runtime.budget(),
                    )
                },
            )?,
        })
    }
    /// Retry only the refused pure step, after releasing optional metadata. Existing rows
    /// remain owned by the caller; this must never wrap output emission or native effects.
    pub(super) fn compute<R>(
        &mut self,
        mut compute: impl FnMut() -> Result<R, ModelError>,
    ) -> Result<R, ModelError> {
        match compute() {
            Err(error)
                if self.request.is_some()
                    && matches!(error.primary(), Some(ModelError::Resource { .. })) =>
            {
                self.request = None;
                compute()
            }
            result => result,
        }
    }
    pub(super) fn prepare_emission(
        &mut self,
        rows: &CatalogOutput,
        links: &normalized::Rows<catalog::CatalogMemberInvocation>,
        budget: &resources::ResourceBudget,
    ) -> Result<(), ModelError> {
        if self.request.is_none() {
            return Ok(());
        }
        let mut bytes = row_emission_bound(links);
        macro_rules! bounds {($($field:ident:$ty:ty,)*)=>{$(bytes=bytes.saturating_add(row_emission_bound(&rows.$field));)*};}
        lctx_model::catalog_outputs!(bounds);
        self.release_for_emission(bytes, budget)
    }
    fn release_for_emission(
        &mut self,
        bytes: usize,
        budget: &resources::ResourceBudget,
    ) -> Result<(), ModelError> {
        if self.request.is_some() {
            match budget.reserve("optional-catalog-output-headroom", bytes) {
                Ok(_probe) => {}
                Err(ModelError::Resource { .. }) => self.request = None,
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }
}
fn row_emission_bound<R: Record>(rows: &normalized::Rows<R>) -> usize {
    rows.iter().fold(65536usize, |total, row| {
        total
            .saturating_add(row.row_bytes().saturating_mul(128))
            .saturating_add(R::fields().len().saturating_mul(2048).saturating_add(8192))
    })
}
fn optional_request(
    enabled: bool,
    budget: &resources::ResourceBudget,
    construct: impl FnOnce() -> Result<ProductRequest, ModelError>,
) -> Result<Option<ReservedProductRequest>, ModelError> {
    if !enabled {
        return Ok(None);
    }
    let mut hold = match budget.reserve("catalog-payload-request", 65536) {
        Ok(hold) => hold,
        Err(ModelError::Resource { .. }) => return Ok(None),
        Err(error) => return Err(error),
    };
    let request = match construct() {
        Ok(request) => request,
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
    match hold.try_resize(request.retained_bytes()?) {
        Ok(()) => Ok(Some((request, hold))),
        Err(ModelError::Resource { .. }) => Ok(None),
        Err(error) => Err(error),
    }
}
fn request(
    data: &CatalogData,
    implementation: ContentHash,
    domain: ContentHash,
    model: ContentHash,
    budget: &resources::ResourceBudget,
) -> Result<ProductRequest, ModelError> {
    let mut selected = KeySink::new("catalog-payload-selected-inputs/v1");
    domain.encode(&mut selected);
    let domain = selected.finish();
    let dependencies = data.value_dependencies(domain, budget)?;
    let mut result = KeySink::new("catalog-payload-output-contract/v1");
    let mut outputs = std::collections::BTreeSet::new();
    macro_rules! inventory {($($field:ident:$ty:ty,)*)=>{$(result.part(b"record",<$ty>::CONTRACT.as_bytes());outputs.insert(<$ty>::NAME.into());)*};}
    lctx_model::catalog_outputs!(inventory);
    let (parameters, definition) = lctx_model::domain::catalog::build::definition();
    let mut configuration = KeySink::new("catalog-payload-selected-method/v1");
    parameters.content_digest().encode(&mut configuration);
    definition.content_digest().encode(&mut configuration);
    let request = ProductRequest {
        kind: ProductKind::PureValue,
        operation: "catalog-core-payload/v1".into(),
        model,
        implementation,
        policy: ContentHash::of(include_bytes!("catalog_products.rs")),
        result_contract: result.finish(),
        configuration: Some(configuration.finish()),
        profile: "neutral-computation".into(),
        parameters: vec![],
        dependencies,
        outputs,
    };
    request.validate()?;
    Ok(request)
}
impl ProductWindow {
    pub(super) async fn lookup(
        &mut self,
        runtime: &Workspace,
    ) -> Result<Option<CatalogOutput>, ModelError> {
        if !runtime.product_reuse_enabled() {
            return Ok(None);
        }
        let Some(cache) = runtime.product_cache()? else {
            return Ok(None);
        };
        let Some(reserved) = self.request.take() else {
            return Ok(None);
        };
        let budget = runtime.budget().clone();
        let (reserved, entry) = runtime
            .cache_call(async move {
                let entry = cache.lookup(&reserved.0, &budget).await?;
                Ok((reserved, entry))
            })
            .await?;
        self.request = Some(reserved);
        let Some(entry) = entry else { return Ok(None) };
        let result = (|| {
            if entry.product().outcome != ProductOutcome::Complete {
                return Err(ModelError::Conflict("catalog payload outcome"));
            }
            let candidate = crate::workspace::decode_product_rows(
                entry.product(),
                runtime.model(),
                runtime.budget(),
                resources::TRANSFER_ROWS,
            )?;
            let mut rows = CatalogOutput::new(runtime.budget());
            for (relation, batches) in candidate.sections() {
                for batch in batches {
                    if !rows.visit(relation.name(), batch)? {
                        return Err(ModelError::Conflict("catalog payload output inventory"));
                    }
                }
            }
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
                tracing::warn!(%error,"invalid catalog computation payload");
                let cache = runtime
                    .product_cache()?
                    .ok_or(ModelError::Conflict("catalog cache owner disappeared"))?;
                let reserved = self
                    .request
                    .take()
                    .ok_or(ModelError::Conflict("catalog request owner disappeared"))?;
                let reserved = runtime
                    .cache_call(async move {
                        cache.invalidate(&reserved.0).await?;
                        Ok(reserved)
                    })
                    .await?;
                self.request = Some(reserved);
                Ok(None)
            }
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
                return Err(ModelError::Schema("catalog payload body"));
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
impl ProductWindow {
    pub(super) async fn retain(
        mut self,
        rows: &CatalogOutput,
        runtime: &Workspace,
    ) -> Result<(), ModelError> {
        let Some(cache) = runtime.product_cache()? else {
            return Ok(());
        };
        let Some(reserved) = self.request.take() else {
            return Ok(());
        };
        let mut bytes = 65536usize;
        macro_rules! allowance {($($field:ident:$ty:ty,)*)=>{$(bytes=rows.$field.iter().fold(bytes,|total,row|total.saturating_add(row.row_bytes().saturating_mul(32))).saturating_add(8192);)*};}
        lctx_model::catalog_outputs!(allowance);
        let hold = match runtime
            .budget()
            .reserve("optional-catalog-payload-encoding", bytes)
        {
            Ok(hold) => hold,
            Err(ModelError::Resource { .. }) => return Ok(()),
            Err(error) => return Err(error),
        };
        let mut sections = Vec::new();
        macro_rules! encode {($($field:ident:$ty:ty,)*)=>{$(sections.push(section(&rows.$field)?);)*};}
        lctx_model::catalog_outputs!(encode);
        sections.sort_by(|left, right| left.name.cmp(&right.name));
        let (request, request_hold) = reserved;
        let product = PortableProduct {
            request,
            outcome: ProductOutcome::Complete,
            sections,
        };
        product.validate()?;
        let budget = runtime.budget().clone();
        // Keep field drop order explicit on cancellation as well as normal completion:
        // encoded payload and request disappear before either backing reservation.
        let owned = (product, hold, request_hold);
        runtime
            .cache_call(async move {
                let owned = owned;
                cache.insert(&owned.0, &budget).await.map(|_| ())
            })
            .await
    }
}

#[cfg(test)]
mod window_controls {
    use super::*;
    use lctx_model::domain::catalog::build;

    fn empty_request(budget: &resources::ResourceBudget) -> ProductRequest {
        request(
            &CatalogData::new(budget),
            ContentHash::of(b"implementation"),
            ContentHash::of(b"roots"),
            ContentHash::of(b"model"),
            budget,
        )
        .unwrap()
    }
    fn window(request: ProductRequest, budget: &resources::ResourceBudget) -> ProductWindow {
        let hold = budget
            .reserve("control request", request.retained_bytes().unwrap())
            .unwrap();
        ProductWindow {
            request: Some((request, hold)),
        }
    }
    #[test]
    fn disabled_or_unadmitted_catalog_cache_does_not_interpret_a_request() {
        let budget = resources::ResourceBudget::fixed(65535).unwrap();
        assert!(
            optional_request(false, &budget, || panic!(
                "disabled cache interpreted inputs"
            ))
            .unwrap()
            .is_none()
        );
        assert!(
            optional_request(true, &budget, || panic!(
                "unadmitted cache interpreted inputs"
            ))
            .unwrap()
            .is_none()
        );
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn one_catalog_request_survives_fresh_preparation_with_its_actual_charge() {
        let budget = resources::ResourceBudget::fixed(1 << 20).unwrap();
        let mut constructions = 0;
        let request = optional_request(true, &budget, || {
            constructions += 1;
            Ok(empty_request(&budget))
        })
        .unwrap();
        let mut window = ProductWindow { request };
        let identity = window.request.as_ref().unwrap().0.identity().unwrap();
        let actual = window.request.as_ref().unwrap().0.retained_bytes().unwrap();
        assert_eq!(budget.reserved(), actual);
        // A miss needs both fresh payload and current invocation links; neither rebuilds keys.
        window
            .compute(|| build::build(&CatalogData::new(&budget), &budget))
            .unwrap();
        window.compute(|| Ok::<_, ModelError>(())).unwrap();
        let (request, hold) = window.request.take().unwrap();
        assert_eq!(request.identity().unwrap(), identity);
        assert_eq!(hold.size(), actual);
        assert_eq!(constructions, 1);
        drop((request, hold));
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn catalog_optional_request_releases_for_only_the_refused_pure_step() {
        let source_budget = resources::ResourceBudget::fixed(1 << 20).unwrap();
        let request = empty_request(&source_budget);
        let request_bytes = request.retained_bytes().unwrap();
        let budget = resources::ResourceBudget::fixed(request_bytes + 100).unwrap();
        let completed_rows = budget.reserve("already completed rows", 30).unwrap();
        let mut window = window(request, &budget);
        let mut calls = 0;
        let links = window
            .compute(|| {
                calls += 1;
                budget.reserve("fresh invocation links", 80)
            })
            .unwrap();
        assert_eq!(calls, 2);
        assert!(window.request.is_none());
        assert_eq!(completed_rows.size(), 30);
        assert_eq!(budget.reserved(), 110);
        drop((completed_rows, links));
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn catalog_pure_failures_are_not_replayed_without_releasable_request() {
        let budget = resources::ResourceBudget::fixed(1 << 20).unwrap();
        let mut window = window(empty_request(&budget), &budget);
        let mut calls = 0;
        let error = window
            .compute::<()>(|| {
                calls += 1;
                Err(ModelError::Conflict("current witness mismatch"))
            })
            .unwrap_err();
        assert!(matches!(error, ModelError::Conflict(_)));
        assert_eq!(calls, 1);
        assert!(window.request.is_some());
        let mut calls = 0;
        let error = window
            .compute(|| {
                calls += 1;
                budget.reserve("infeasible pure step", budget.limit() + 1)
            })
            .unwrap_err();
        assert!(matches!(error.primary(), Some(ModelError::Resource { .. })));
        assert_eq!(calls, 2);
        let mut calls = 0;
        assert!(
            window
                .compute(|| {
                    calls += 1;
                    budget.reserve("still infeasible", budget.limit() + 1)
                })
                .is_err()
        );
        assert_eq!(calls, 1);
    }
    #[test]
    fn catalog_output_headroom_abandons_retention_before_effects() {
        let source_budget = resources::ResourceBudget::fixed(1 << 20).unwrap();
        let request = empty_request(&source_budget);
        let budget =
            resources::ResourceBudget::fixed(request.retained_bytes().unwrap() + 100).unwrap();
        let mut window = window(request, &budget);
        window
            .release_for_emission(budget.limit(), &budget)
            .unwrap();
        assert!(window.request.is_none());
        assert_eq!(budget.reserved(), 0);
        let _output = budget.reserve("effectful output", budget.limit()).unwrap();
    }
    fn nominal<R>(byte: u8) -> Id<R> {
        serde_json::from_value(serde_json::to_value([byte; 16]).unwrap()).unwrap()
    }
    #[test]
    fn catalog_request_keeps_exact_root_context_and_witness_dependencies() {
        let budget = resources::ResourceBudget::fixed(1 << 20).unwrap();
        let mut data = CatalogData::new(&budget);
        let implementation = ContentHash::of(b"implementation");
        let domain = ContentHash::of(b"roots");
        let model = ContentHash::of(b"model");
        let identity = |data: &CatalogData, domain, model, implementation| {
            request(data, implementation, domain, model, &budget)
                .unwrap()
                .identity()
                .unwrap()
        };
        let empty = identity(&data, domain, model, implementation);
        assert_ne!(
            empty,
            identity(
                &data,
                ContentHash::of(b"other roots"),
                model,
                implementation
            )
        );
        assert_ne!(
            empty,
            identity(
                &data,
                domain,
                ContentHash::of(b"other model"),
                implementation
            )
        );
        assert_ne!(
            empty,
            identity(
                &data,
                domain,
                model,
                ContentHash::of(b"other implementation")
            )
        );
        let mut name = symbols::PublicNameObservation {
            qualification: nominal(1),
            access: nominal(2),
            name: "same_name".into(),
            via_dunder_all: false,
            origin: nominal(3),
        };
        data.names.insert(name.clone()).unwrap();
        let first = identity(&data, domain, model, implementation);
        assert_ne!(empty, first);
        // Equal public spelling in another qualified context has its own exact witness.
        name.qualification = nominal(4);
        let mut other = CatalogData::new(&budget);
        other.names.insert(name.clone()).unwrap();
        assert_ne!(first, identity(&other, domain, model, implementation));
        name.origin = nominal(5);
        let mut changed = CatalogData::new(&budget);
        changed.names.insert(name).unwrap();
        assert_ne!(
            identity(&other, domain, model, implementation),
            identity(&changed, domain, model, implementation)
        );
    }
}
