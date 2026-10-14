//! Re-observe an existing product's original boundary; source revision churn is not
//! a reason to manufacture another independent retention root.
use super::*;
use d::{
    completion::{Completion, complete},
    resources::ResourceBudget,
};
use std::sync::Arc;
use surrealdb::{Surreal, engine::remote::grpc::Client, types::Variables};

struct PinTerminal {
    pin: Arc<crate::control::ReaderPin>,
    closed: bool,
}
impl Drop for PinTerminal {
    fn drop(&mut self) {
        if !self.closed {
            self.pin.retain_unknown();
        }
    }
}

/// Returns false only when no product exists. Existing incomplete, released,
/// substituted or ambiguous retention refuses rather than silently replacing it.
pub async fn check_retained_product_replay(
    client: Arc<Surreal<Client>>,
    attempt: ContentHash,
    request: ContentHash,
    physical: ContentHash,
    budget: &ResourceBudget,
) -> Result<bool, ModelError> {
    let product = RecordId::new(
        "native_product",
        format!("{}_{}", request.hex(), attempt.hex()),
    );
    // A terminal point-read absence grants no reuse authority. Only a positive
    // observation needs a receiving pin, whose CAS consumes these original bytes.
    let discovery = crate::NativeReader::private(client.clone()).with_budget(budget);
    let observed = crate::control::inspect_expected_read(&discovery, product.clone(), budget).await;
    let mut discovery_terminal = Completion::default();
    discovery_terminal.step(
        "retained product replay discovery reader close",
        discovery.close().await,
    );
    let Some(expected) = complete(observed, discovery_terminal)? else {
        return Ok(false);
    };
    let pin = crate::control::ReaderPin::acquire(client.clone(), &[]).await?;
    let mut terminal_pin = PinTerminal {
        pin: pin.clone(),
        closed: false,
    };
    let reader = crate::NativeReader::private_pinned(client, pin.clone()).with_budget(budget);
    let result=async {
        let mut row=Object::new();row.insert("id",product.clone());row.insert("request",request.hex());
        row.insert("released",false);row.insert("contribution",RecordId::new("compiler_contribution",physical.hex()));
        if expected.row()!=Value::Object(row){return Err(ModelError::Conflict("retained product replay exact request/contribution/retention"));}
        crate::control::live_view::transfer_expected(&pin,&expected,budget).await?;
        require_hold(&reader,&RecordId::new("native_attempt",attempt.hex()),&product).await?;
        require_hold(&reader,&product,&RecordId::new("compiler_contribution",physical.hex())).await?;
        let mut variables=Variables::new();variables.insert("product",product.clone());
        let parents:Vec<RecordId>=reader.query_native("SELECT VALUE object FROM native_hold WITH INDEX owner_holds WHERE owner=$product AND record::table(object)='native_completed_state' LIMIT 2",variables).await?;
        let [parent]=parents.as_slice() else{return Err(ModelError::Conflict("retained product replay singleton boundary"));};
        let parent_expected=crate::control::inspect_expected_read(&reader,parent.clone(),budget).await?.ok_or(ModelError::Conflict("retained product replay missing boundary"))?;
        let data=FrozenCompletion::from_row(parent_expected.row(),true)?;
        if data.record()?!=*parent || data.attempt!=attempt{return Err(ModelError::Conflict("retained product replay original source"));}
        crate::control::live_view::transfer_expected(&pin,&parent_expected,budget).await?;
        let inventory=inspect_inventory(&reader,&data,budget).await?;
        let copy=inventory.copies().get(&physical).ok_or(ModelError::Conflict("retained product replay exact contributor absent"))?;
        let logical=copy.descriptor.identity()?;
        for (relation,output) in &copy.descriptor.outputs {
            let view=d::completed::CompletedView::new(relation.clone(),[logical].into(),output.rows)?;
            let selector=inventory.installed(view.identity)?;
            if selector.view!=view || !selector.mappings.iter().any(|mapping|mapping.view==view.identity && mapping.physical==physical && mapping.logical==logical && mapping.incarnation==copy.incarnation){return Err(ModelError::Conflict("retained product replay singleton output incidence"));}
        }
        let installation:Vec<Object>=reader.query_native("SELECT generation,validity_epoch FROM native_installation:current",Variables::new()).await?;
        let [installation]=installation.as_slice() else{return Err(ModelError::Conflict("retained product replay installation cardinality"));};
        if installation.get("generation")!=Some(&Value::String(data.generation.hex())) || installation.get("validity_epoch")!=Some(&Value::Number(surrealdb::types::Number::Int(data.validity_epoch))){return Err(ModelError::Conflict("retained product replay installation changed"));}
        for observed in [&expected,&parent_expected] {
            let current=crate::control::inspect_expected_read(&reader,observed.record().clone(),budget).await?.ok_or(ModelError::Conflict("retained product replay record disappeared"))?;
            if current.incarnation()!=observed.incarnation() || current.row()!=observed.row(){return Err(ModelError::Conflict("retained product replay changed during read"));}
        }
        require_hold(&reader,&product,parent).await?;
        Ok(true)
    }.await;
    let mut terminal = Completion::default();
    let closed = reader.close().await;
    if closed.is_err()
        || result
            .as_ref()
            .err()
            .is_some_and(|error: &ModelError| !error.permits_storage_cleanup())
    {
        pin.retain_unknown();
    }
    terminal.step("retained product replay reader close", closed);
    let released = pin.release().await;
    terminal_pin.closed = released.is_ok();
    terminal.step("retained product replay pin release", released);
    complete(result, terminal)
}
