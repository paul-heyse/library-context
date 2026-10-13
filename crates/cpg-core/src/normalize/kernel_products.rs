//! Selected kernel products substitute only after fresh complete membership/content discovery.
use super::*;
use lctx_model::domain::{compilation_product::*, normalized::{callable_normalization, callable_aspects}};
use lctx_native::surrealdb::types::Value;

/// Optional encodings are owned through one insertion window, never through later root windows.
pub(super) struct ProductWindow {
    cache: Option<Arc<lctx_compiler_store::product_cache::NativeProductCache>>,
    products: Vec<PortableProduct>,
    holds: Vec<Box<dyn resources::Reservation>>,
    metadata: Option<Box<dyn resources::Reservation>>,
}
impl ProductWindow {
    pub(super) fn new(count: usize, runtime: &Workspace) -> Result<Self, ModelError> {
        let cache = runtime.product_cache()?;
        let metadata = if cache.is_some() {
            match runtime.budget().reserve("optional-product-retention-window", count.saturating_mul(512)) {
                Ok(charge) => Some(charge),
                Err(ModelError::Resource { .. }) => None,
                Err(error) => return Err(error),
            }
        } else { None };
        let enabled = metadata.is_some();
        Ok(Self { cache: if enabled { cache } else { None },
            products: Vec::with_capacity(if enabled { count } else { 0 }),
            holds: Vec::with_capacity(if enabled { count } else { 0 }), metadata })
    }
    fn abandon(&mut self) {
        self.products = Vec::new();
        self.holds = Vec::new();
        self.metadata = None;
        self.cache = None;
    }
    /// A previous optional encoding must not make an otherwise feasible pure kernel refuse.
    /// All selection/kernel allocations belong inside this closure; no effects may occur here.
    pub(super) fn compute<R>(&mut self, mut compute: impl FnMut() -> Result<R, ModelError>) -> Result<R, ModelError> {
        match compute() {
            Err(error) if (self.metadata.is_some() || !self.holds.is_empty())
                && matches!(error.primary(), Some(ModelError::Resource { .. })) => {
                self.abandon();
                compute()
            }
            result => result,
        }
    }
    pub(super) fn prepare_emission(&mut self, bytes: usize, budget: &resources::ResourceBudget) -> Result<(), ModelError> {
        if (self.metadata.is_some() || !self.holds.is_empty()) && !emission_fits(bytes, budget)? { self.abandon(); }
        Ok(())
    }
    fn push(&mut self, product: PortableProduct, hold: Box<dyn resources::Reservation>) {
        self.products.push(product);
        self.holds.push(hold);
    }
    pub(super) async fn finish(self, runtime: &Workspace) -> Result<(), ModelError> {
        let Some(cache) = self.cache else { return Ok(()); };
        if self.products.is_empty() { return Ok(()); }
        let budget = runtime.budget().clone();
        runtime.native_call(async move {
            let _holds = self.holds;
            let _metadata = self.metadata;
            cache.insert_window(&self.products, &budget).await.map(|_| ())
        }).await
    }
}

/// Reserve before effectful output emission. This covers typed row/Arrow copies, the
/// native 32x logical lowering allowance and per-column native metadata, conservatively
/// over the entire result rather than only the next transfer batch. The probe is released
/// before the existing writer takes its actual reservations; emission is never retried.
fn row_emission_bound<R: Record>(rows: &Rows<R>) -> usize {
    let fields = R::fields().len();
    rows.iter().fold(65536usize, |total, row| total
        .saturating_add(row.row_bytes().saturating_mul(128))
        .saturating_add(fields.saturating_mul(2048).saturating_add(8192)))
}
fn emission_fits(bytes: usize, budget: &resources::ResourceBudget) -> Result<bool, ModelError> {
    match budget.reserve("optional-product-output-headroom", bytes) {
        Ok(_probe) => Ok(true),
        Err(ModelError::Resource { .. }) => Ok(false),
        Err(error) => Err(error),
    }
}
fn release_for_emission<T>(hits: &mut [Option<T>], bytes: usize,
    budget: &resources::ResourceBudget) -> Result<bool, ModelError> {
    if hits.iter().any(Option::is_some) && !emission_fits(bytes, budget)? {
        for entry in hits { *entry = None; }
        return Ok(false);
    }
    Ok(true)
}
pub(super) fn prepare_hits(hits: &mut [Option<lctx_compiler_store::product_cache::LeasedProduct>],
    bytes: usize, budget: &resources::ResourceBudget) -> Result<bool, ModelError> {
    release_for_emission(hits, bytes, budget)
}
pub(super) async fn lookup_one(request: &ProductRequest, runtime: &Workspace)
    -> Result<Option<lctx_compiler_store::product_cache::LeasedProduct>, ModelError> {
    if !runtime.product_reuse_enabled() {return Ok(None);}
    let Some(cache) = runtime.product_cache()? else { return Ok(None); };
    let hold = match runtime.budget().reserve("optional-product-point-request", request.retained_bytes()?) {
        Ok(hold) => hold, Err(ModelError::Resource { .. }) => return Ok(None), Err(error) => return Err(error),
    };
    let owned = request.clone();
    let budget = runtime.budget().clone();
    runtime.native_call(async move { let _hold = hold; cache.lookup(&owned, &budget).await }).await
}
pub(super) fn entity_emission_bound(rows: &entity_normalization::EntityOutput, kernel: entity_normalization::EntityKernel) -> usize {
    let mut bytes = 0usize;
    macro_rules! bounds {($($field:ident:$ty:ty,)*) => {$(if included(stringify!($field), kernel) { bytes = bytes.saturating_add(row_emission_bound(&rows.$field)); })*};}
    lctx_model::normalized_entity_outputs!(bounds);
    bytes
}
pub(super) fn callable_emission_bound(rows: &callable_normalization::CallableOutput) -> usize {
    let mut bytes = 0usize;
    macro_rules! bounds {($($field:ident:$ty:ty,)*) => {$(bytes = bytes.saturating_add(row_emission_bound(&rows.$field));)*};}
    lctx_model::normalized_callable_outputs!(bounds);
    bytes
}
pub(super) fn aspect_emission_bound(rows: &callable_aspects::AspectOutput) -> usize {
    let mut bytes = 0usize;
    macro_rules! bounds {($($field:ident:$ty:ty,)*) => {$(bytes = bytes.saturating_add(row_emission_bound(&rows.$field));)*};}
    lctx_model::callable_aspect_outputs!(bounds);
    bytes
}

/// Optional acceleration may refuse retention without changing ordinary kernel feasibility.
/// Only local token retention may refuse optionally inside `content_domain`.
/// Submitted native failures retain their full ownership/finality outcome.
pub(super) async fn selected_domain(
    selected: &crate::consumed_rows::PreparedRootBatch, partition: usize,
    root: crate::consumed_rows::PreparedRoot, inputs: &[ValidationInput],
    program: ContentHash, runtime: &Workspace,
) -> Result<Option<ContentHash>, ModelError> {
    selected.content_domain(partition, root, inputs, program, runtime).await
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
    request.profile = "neutral-algorithm".into();
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
pub(super) async fn entity_hit(request: &ProductRequest, entry:Option<lctx_compiler_store::product_cache::LeasedProduct>, runtime: &Workspace)
    -> Result<Option<entity_normalization::EntityOutput>, ModelError> {
    let Some(cache) = runtime.product_cache()? else { return Ok(None); };
    let Some(entry) = entry else { return Ok(None); };
    let decode = (|| {
        let product = entry.product();
        let candidate = crate::workspace::decode_product_rows(product, runtime.model(), runtime.budget(), resources::TRANSFER_ROWS)?;
        if product.outcome != ProductOutcome::Complete {
            return Err(ModelError::Conflict("entity kernel product outcome"));
        }
        let mut rows = entity_normalization::EntityOutput::new(runtime.budget());
        for (relation, batches) in candidate.sections() {
            for batch in batches {
                if !rows.visit(relation.name(), batch)? { return Err(ModelError::Schema("entity product output inventory")); }
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
        let mut encoded = lctx_native::codec::batch_bodies(&relation, &batch)?;
        for (row, body) in window.iter().zip(&mut encoded) {
            let Value::Object(fields) = body else { return Err(ModelError::Schema("entity product canonical body")); };
            fields.insert("id", hex::encode(row.id().bytes()));
        }
        bodies.extend(encoded);
    }
    Ok(ProductSection { name: R::NAME.into(), rows: bodies.len() as u64,
        bytes: serde_json::to_vec(&bodies).map_err(ModelError::codec)? })
}
pub(super) fn retain_entity(window: &mut ProductWindow, request: ProductRequest, rows: &entity_normalization::EntityOutput,
    kernel: entity_normalization::EntityKernel, runtime: &Workspace) -> Result<(), ModelError> {
    if window.cache.is_none() { return Ok(()); }
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
    window.push(product, charge);
    Ok(())
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
    request.profile = "neutral-algorithm".into();
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
pub(super) async fn callable_hit(request: &ProductRequest, entry:Option<lctx_compiler_store::product_cache::LeasedProduct>, runtime: &Workspace)
    -> Result<Option<callable_normalization::CallableOutput>, ModelError> {
    let Some(cache) = runtime.product_cache()? else { return Ok(None); };
    let Some(entry) = entry else { return Ok(None); };
    let decode = (|| {
        let product = entry.product();
        let candidate = crate::workspace::decode_product_rows(product, runtime.model(), runtime.budget(), resources::TRANSFER_ROWS)?;
        if product.outcome != ProductOutcome::Complete {
            return Err(ModelError::Conflict("callable kernel product outcome"));
        }
        let mut rows = callable_normalization::CallableOutput::new(runtime.budget());
        for (relation, batches) in candidate.sections() {
            for batch in batches {
                if !rows.visit(relation.name(), batch)? { return Err(ModelError::Schema("callable product output inventory")); }
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
pub(super) fn retain_callable(window: &mut ProductWindow, request: ProductRequest, rows: &callable_normalization::CallableOutput,
    runtime: &Workspace) -> Result<(), ModelError> {
    if window.cache.is_none() { return Ok(()); }
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
    window.push(product, charge);
    Ok(())
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
    request.profile = "neutral-algorithm".into();
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
pub(super) async fn aspect_hit(request: &ProductRequest, entry:Option<lctx_compiler_store::product_cache::LeasedProduct>, runtime: &Workspace)
    -> Result<Option<callable_aspects::AspectOutput>, ModelError> {
    let Some(cache) = runtime.product_cache()? else { return Ok(None); };
    let Some(entry) = entry else { return Ok(None); };
    let decode = (|| {
        let product = entry.product();
        let candidate = crate::workspace::decode_product_rows(product, runtime.model(), runtime.budget(), resources::TRANSFER_ROWS)?;
        if product.outcome != ProductOutcome::Complete {
            return Err(ModelError::Conflict("aspect kernel product outcome"));
        }
        let mut rows = callable_aspects::AspectOutput::new(runtime.budget());
        for (relation, batches) in candidate.sections() {
            for batch in batches {
                if !rows.visit(relation.name(), batch)? { return Err(ModelError::Schema("aspect product output inventory")); }
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
pub(super) fn retain_aspect(window: &mut ProductWindow, request: ProductRequest, rows: &callable_aspects::AspectOutput,
    runtime: &Workspace) -> Result<(), ModelError> {
    if window.cache.is_none() { return Ok(()); }
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
    window.push(product, charge);
    Ok(())
}

/// The existing root window owns one optional cache lookup before rich hydration of misses.
pub(super) async fn lookup_window(requests:&[Option<ProductRequest>],runtime:&Workspace)->Result<Vec<Option<lctx_compiler_store::product_cache::LeasedProduct>>,ModelError>{
    let empty=||std::iter::repeat_with(||None).take(requests.len()).collect::<Vec<_>>();
    if !runtime.product_reuse_enabled(){return Ok(empty());}
    let Some(cache)=runtime.product_cache()? else{return Ok(empty())};
    let metadata=match runtime.budget().reserve("optional-selected-product-window",requests.len().saturating_mul(256)){Ok(charge)=>charge,Err(ModelError::Resource{..})=>return Ok(empty()),Err(error)=>return Err(error)};
    let mut lookups=Vec::with_capacity(requests.len());let mut holds=Vec::with_capacity(requests.len());let mut positions=Vec::with_capacity(requests.len());
    for (index,request) in requests.iter().enumerate(){if let Some(request)=request{
        match runtime.budget().reserve("optional-selected-product-window-request",request.retained_bytes()?){
            Ok(charge)=>{lookups.push(request.clone());holds.push(charge);positions.push(index);},
            Err(ModelError::Resource{..})=>{},Err(error)=>return Err(error),
        }
    }}
    let budget=runtime.budget().clone();
    let entries=runtime.native_call(async move{let _holds=holds;let _metadata=metadata;cache.lookup_window(&lookups,&budget).await}).await?;
    align_entries(requests.len(), positions, entries)

}

#[cfg(test)]
mod window_controls {
    use super::*;
    fn charged_window(budget: &resources::ResourceBudget, bytes: usize) -> ProductWindow {
        ProductWindow { cache: None, products: Vec::new(), holds: Vec::new(),
            metadata: Some(budget.reserve("optional control metadata", bytes).unwrap()) }
    }
    #[test]
    fn aligned_hits_preserve_request_order_multiplicity_and_individual_absence() {
        let aligned = align_entries(5, vec![0, 2, 3, 4], vec![Some("same"), None, Some("other"), Some("same")]).unwrap();
        assert_eq!(aligned, vec![Some("same"), None, None, Some("other"), Some("same")]);
        assert!(align_entries(1, vec![0], Vec::<Option<()>>::new()).is_err());
        assert!(align_entries(1, vec![1], vec![Some(())]).is_err());
    }
    #[test]
    fn later_optional_hits_release_before_effectful_output_under_tight_budget() {
        let budget = resources::ResourceBudget::fixed(100).unwrap();
        let fresh = budget.reserve("fresh output", 70).unwrap();
        drop(fresh);
        let mut hits = vec![Some(budget.reserve("later hit", 40).unwrap()), None];
        assert!(!release_for_emission(&mut hits, 70, &budget).unwrap());
        assert!(hits.iter().all(Option::is_none));
        assert_eq!(budget.reserved(), 0);
        let _fresh = budget.reserve("fresh output", 70).unwrap();
    }
    #[test]
    fn optional_window_cannot_refuse_feasible_pure_kernel_and_is_not_replayed_for_other_errors() {
        let budget = resources::ResourceBudget::fixed(100).unwrap();
        let mut window = charged_window(&budget, 40);
        let mut calls = 0;
        let held = window.compute(|| { calls += 1; budget.reserve("fresh kernel", 70) }).unwrap();
        assert_eq!(calls, 2);
        assert_eq!(held.size(), 70);
        drop(held);
        assert_eq!(budget.reserved(), 0);
        let mut window = charged_window(&budget, 40);
        let mut calls = 0;
        let error = window.compute::<()>(|| { calls += 1; Err(ModelError::Conflict("kernel semantics")) }).unwrap_err();
        assert!(matches!(error, ModelError::Conflict(_)));
        assert_eq!(calls, 1);
        assert_eq!(budget.reserved(), 40);
        drop(window);
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn output_headroom_drops_pending_optional_encodings_without_effect_replay() {
        let budget = resources::ResourceBudget::fixed(100).unwrap();
        let mut window = charged_window(&budget, 40);
        window.prepare_emission(70, &budget).unwrap();
        assert!(window.metadata.is_none());
        assert_eq!(budget.reserved(), 0);
        let _output = budget.reserve("ordinary effectful output", 70).unwrap();
    }
}

fn align_entries<T>(count: usize, positions: Vec<usize>, entries: Vec<Option<T>>) -> Result<Vec<Option<T>>,ModelError> {
    if entries.len() != positions.len() || positions.iter().any(|index| *index >= count)
        || positions.iter().collect::<std::collections::BTreeSet<_>>().len() != positions.len() {
        return Err(ModelError::Conflict("product lookup window cardinality"));
    }
    let mut aligned = std::iter::repeat_with(|| None).take(count).collect::<Vec<_>>();
    for (index, entry) in positions.into_iter().zip(entries) { aligned[index] = entry; }
    Ok(aligned)
}
