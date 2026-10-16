//! Finite buffered-result ownership, using the same admitted stream items.
use crate::Error;
use crate::types::{Array, Value};
use std::sync::Arc;
use surrealdb_rpc::{
    AdmittedQueryResults, AdmittedQueryStreamEvent, IngressAdmission, IngressReservation,
    IngressStage, QueryResult, QueryStreamItem, QueryType,
};

pub(super) const MAX_STATEMENTS: usize = 1 << 20;

#[derive(Debug)]
struct Ownership {
    // Payload/vector allocations are destroyed by their actual result owner
    // before this lease set. Metadata storage releases after its own Vec.
    leases: Vec<Arc<dyn IngressReservation>>,
    _metadata: Option<Arc<dyn IngressReservation>>,
    _fixed: Arc<dyn IngressReservation>,
}
impl IngressReservation for Ownership {}
// Refused metadata growth transfers its still-owned input grants back to the
// caller. Only the cold error payload is indirect; input ownership stays exact.
struct RetentionFailure {
    error: Box<Error>,
    incoming: Vec<Arc<dyn IngressReservation>>,
}
impl Ownership {
    fn retain(
        &mut self,
        incoming: Vec<Arc<dyn IngressReservation>>,
        owner: &dyn IngressAdmission,
    ) -> Result<(), RetentionFailure> {
        let Some(count) = self.leases.len().checked_add(incoming.len()) else {
            return Err(RetentionFailure {
                error: Box::new(Error::internal("Buffered lease inventory overflow".into())),
                incoming,
            });
        };
        if let Err(error) = grow(&mut self.leases, &mut self._metadata, count, owner) {
            return Err(RetentionFailure {
                error: Box::new(error),
                incoming,
            });
        }
        self.leases.extend(incoming);
        Ok(())
    }
    fn retain_one(
        &mut self,
        lease: Arc<dyn IngressReservation>,
        owner: &dyn IngressAdmission,
    ) -> crate::Result<()> {
        let count = self
            .leases
            .len()
            .checked_add(1)
            .ok_or_else(|| Error::internal("Buffered lease inventory overflow".into()))?;
        grow(&mut self.leases, &mut self._metadata, count, owner)?;
        self.leases.push(lease);
        Ok(())
    }
}

fn grow<T>(
    values: &mut Vec<T>,
    charge: &mut Option<Arc<dyn IngressReservation>>,
    count: usize,
    owner: &dyn IngressAdmission,
) -> crate::Result<()> {
    if count <= values.capacity() {
        return Ok(());
    }
    let capacity = count
        .max(
            values
                .capacity()
                .checked_mul(2)
                .ok_or_else(|| Error::internal("Buffered capacity overflow".into()))?,
        )
        .max(4);
    let bytes = capacity
        .checked_mul(std::mem::size_of::<T>())
        .ok_or_else(|| Error::internal("Buffered allocation overflow".into()))?;
    let next_charge = owner.reserve(IngressStage::ValueConversion, bytes)?;
    let mut next = Vec::with_capacity(capacity);
    next.append(values);
    // Both reservations survive the actual allocation and move; destroy the
    // former vector before releasing the former allocation's authority.
    *values = next;
    *charge = Some(next_charge);
    Ok(())
}

#[derive(Default)]
struct Statement {
    values: Vec<Value>,
    storage: Option<Arc<dyn IngressReservation>>,
    shape: u8,
    finished: bool,
    time: std::time::Duration,
    query_type: QueryType,
    error: Option<Error>,
}
impl Statement {
    fn push(&mut self, item: QueryStreamItem, owner: &dyn IngressAdmission) -> crate::Result<()> {
        if self.finished {
            let error = Error::internal("Buffered item followed its statement terminal".into());
            return Err(match item {
                QueryStreamItem::Finished {
                    error: Some(source),
                    ..
                } => error.append_cause(source),
                item => {
                    drop(item);
                    error
                }
            });
        }
        if let QueryStreamItem::Finished {
            time,
            query_type,
            error,
            ..
        } = item
        {
            self.finished = true;
            self.time = time;
            self.query_type = query_type;
            self.error = error;
            return Ok(());
        }
        let shape = if matches!(item, QueryStreamItem::Rows { .. }) {
            1
        } else {
            2
        };
        if self.shape != 0 && (shape != self.shape || shape == 2) {
            return Err(Error::internal(
                "Buffered mixed/repeated result shape".into(),
            ));
        }
        self.shape = shape;
        match item {
            QueryStreamItem::Rows { values, .. } => {
                let count = self
                    .values
                    .len()
                    .checked_add(values.len())
                    .ok_or_else(|| Error::internal("Buffered row count overflow".into()))?;
                grow(&mut self.values, &mut self.storage, count, owner)?;
                self.values.extend(values);
            }
            QueryStreamItem::Value { value, .. } => {
                grow(&mut self.values, &mut self.storage, 1, owner)?;
                self.values.push(value);
            }
            QueryStreamItem::EmptyValue { .. } => {}
            QueryStreamItem::Finished { .. } => unreachable!(),
        }
        Ok(())
    }
    fn result(self) -> QueryResult {
        let result = match self.error {
            Some(error) => Err(error),
            None if self.shape == 2 => Ok(self.values.into_iter().next().unwrap_or(Value::None)),
            None => Ok(Value::Array(Array::from(self.values))),
        };
        QueryResult {
            time: self.time,
            query_type: self.query_type,
            result,
        }
    }
}

#[derive(Debug)]
struct ErrorOwner {
    _owner: Arc<dyn IngressReservation>,
}
impl crate::types::ErrorResource for ErrorOwner {}

pub(super) struct BufferedResults {
    // Actual buffered values are destroyed before their covering resources.
    statements: Vec<Option<Statement>>,
    statements_storage: Option<Arc<dyn IngressReservation>>,
    // Refusal/cause payloads also die before their covering allocation grants
    // when cancellation drops this owner during continued transport drainage.
    primary: Option<Error>,
    ownership: Ownership,
    owner: Arc<dyn IngressAdmission>,
    last: Option<usize>,
}
impl BufferedResults {
    pub fn new(owner: Arc<dyn IngressAdmission>) -> crate::Result<Self> {
        let fixed = owner.reserve(
            IngressStage::ValueConversion,
            std::mem::size_of::<Self>()
                + std::mem::size_of::<Ownership>()
                + 2 * std::mem::size_of::<usize>()
                + std::mem::size_of::<Error>()
                + Error::resource_allocation_size()
                + 512,
        )?;
        Ok(Self {
            statements: Vec::new(),
            statements_storage: None,
            ownership: Ownership {
                leases: Vec::new(),
                _metadata: None,
                _fixed: fixed,
            },
            owner,
            last: None,
            primary: None,
        })
    }
    pub fn fail(&mut self, error: Error) {
        self.primary = Some(match self.primary.take() {
            Some(primary) => primary.append_cause(error),
            None => error,
        });
    }
    /// Delivery never abandons execution after local refusal: the producer
    /// continues to its checked physical terminal while this owner drops rows.
    pub fn ingest(&mut self, event: AdmittedQueryStreamEvent) {
        let item = match event {
            AdmittedQueryStreamEvent::Failure(failure) => {
                let (error, leases) = failure.into_parts();
                self.fail(super::ingress::retain_error(error, leases));
                return;
            }
            AdmittedQueryStreamEvent::Item(item) => item,
        };
        let (item, leases) = item.into_parts();
        if self.primary.is_some() {
            match item {
                QueryStreamItem::Finished {
                    error: Some(error), ..
                } => self.fail(super::ingress::retain_error(error, leases)),
                item => {
                    drop(item);
                    drop(leases);
                }
            }
            return;
        }
        if let Err(RetentionFailure {
            error,
            incoming: leases,
        }) = self.ownership.retain(leases, self.owner.as_ref())
        {
            let error = *error;
            // Destroy refused payloads while their original frame leases remain
            // live. Preserve a statement's own typed error even if metadata
            // admission was the first refusal.
            let error = if let QueryStreamItem::Finished {
                error: Some(source),
                ..
            } = item
            {
                error.append_cause(source)
            } else {
                drop(item);
                error
            };
            self.fail(super::ingress::retain_error(error, leases));
            return;
        }
        let (item, statement_error) = match item {
            QueryStreamItem::Finished {
                index,
                time,
                query_type,
                error: Some(error),
            } => (
                QueryStreamItem::Finished {
                    index,
                    time,
                    query_type,
                    error: None,
                },
                Some(error),
            ),
            item => (item, None),
        };
        let index = item.index();
        let result = (|| {
            if index >= MAX_STATEMENTS {
                return Err(Error::internal(
                    "Buffered statement inventory exceeds its declared limit".into(),
                ));
            }
            if let Some(last) = self.last
                && (index < last
                    || (index > last
                        && !self.statements[last]
                            .as_ref()
                            .is_some_and(|statement| statement.finished)))
            {
                return Err(Error::internal("Buffered statement order".into()));
            }
            grow(
                &mut self.statements,
                &mut self.statements_storage,
                index + 1,
                self.owner.as_ref(),
            )?;
            self.statements.resize_with(index + 1, || None);
            self.last = Some(index);
            self.statements[index]
                .get_or_insert_with(Statement::default)
                .push(item, self.owner.as_ref())
        })();
        match result {
            Err(error) => self.fail(match statement_error {
                Some(source) => error.append_cause(source),
                None => error,
            }),
            Ok(()) => {
                if let Some(error) = statement_error {
                    self.statements[index]
                        .as_mut()
                        .expect("inserted statement")
                        .error = Some(error);
                }
            }
        }
    }
    pub fn finish(mut self) -> crate::Result<AdmittedQueryResults> {
        if self
            .statements
            .iter()
            .flatten()
            .any(|statement| !statement.finished)
        {
            self.fail(Error::connection(
                "Buffered query missing a statement terminal".into(),
                crate::types::ConnectionError::ConnectionFailed,
            ));
        }
        // Extracted typed errors can outlive the response. Their opaque owner
        // retains this query's finite allocation set, without cloning details.
        if let Some(error) = self.primary.take() {
            let resource: Arc<dyn IngressReservation> = Arc::new(self.ownership);
            return Err(error.with_resource(Arc::new(ErrorOwner { _owner: resource })));
        }
        let count = self.statements.iter().flatten().count();
        let result_bytes = count
            .checked_mul(
                std::mem::size_of::<QueryResult>()
                    + std::mem::size_of::<ErrorOwner>()
                    + 2 * std::mem::size_of::<usize>()
                    + Error::resource_allocation_size(),
            )
            .ok_or_else(|| Error::internal("Buffered result allocation overflow".into()))?;
        let result_charge = self
            .owner
            .reserve(IngressStage::ValueConversion, result_bytes)?;
        self.ownership
            .retain_one(result_charge.clone(), self.owner.as_ref())?;
        if let Some(charge) = &self.statements_storage {
            self.ownership
                .retain_one(charge.clone(), self.owner.as_ref())?;
        }
        self.statements_storage.take();
        for statement in self.statements.iter_mut().flatten() {
            if let Some(charge) = &statement.storage {
                self.ownership
                    .retain_one(charge.clone(), self.owner.as_ref())?;
            }
            statement.storage.take();
        }
        let mut results = Vec::with_capacity(count);
        for statement in std::mem::take(&mut self.statements).into_iter().flatten() {
            results.push(statement.result());
        }
        let resource: Arc<dyn IngressReservation> = Arc::new(self.ownership);
        for result in &mut results {
            if let Err(error) = &mut result.result {
                let error_value = std::mem::replace(error, Error::internal(String::new()));
                *error = error_value.with_resource(Arc::new(ErrorOwner {
                    _owner: resource.clone(),
                }));
            }
        }
        Ok(AdmittedQueryResults {
            results,
            reservations: Some(resource),
            admission: Some(self.owner),
        })
    }
}
