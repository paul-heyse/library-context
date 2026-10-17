//! A bounded command owner for blocking external ordering. Cancellation keeps submitted
//! work and its acknowledgement in the owner; dropping the owner closes admission and
//! lets already acknowledged blocking work release scratch and guards at its terminal.
use crate::ordered_rows::{
    Candidate, OrderedCandidates, OrderedRows, PreparedCandidates, PreparedRows, SortedCandidates,
    SortedRows,
};
use futures::{
    FutureExt,
    future::{BoxFuture, Shared},
};
use lctx_model::domain::{
    ModelError,
    completed_identity::{CompletedFamilyHasher, CompletedFamilyRoot},
    recovery_closure::RecoveryFamily,
    resources::{Reservation, ResourceBudget},
};
use std::{
    any::Any,
    sync::{Arc, atomic::AtomicBool},
};
use surrealdb::types::{RecordIdKey, Value};
use tokio::sync::{mpsc, oneshot};

type Reply = Result<Output, Arc<ModelError>>;
pub type BlockingTerminal = Shared<BoxFuture<'static, Result<(), Arc<ModelError>>>>;
type BlockingUnit = Box<dyn FnOnce(&ResourceBudget) -> Result<Output, ModelError> + Send>;
enum Input {
    Candidate(Candidate),
    Row(Value),
    RowBatch(Vec<Value>),
    OwnedRowBatch(Vec<(Value, Box<dyn Reservation>)>),
    Unit(BlockingUnit),
    Finish,
    Prepare,
    Next(usize),
    FoldFamily {
        family: RecoveryFamily,
        runs: Vec<PreparedRows>,
        cancelled: Option<Arc<AtomicBool>>,
    },
    UnionCandidates {
        runs: Vec<PreparedCandidates>,
        cancelled: Option<Arc<AtomicBool>>,
    },
    UnionRows {
        runs: Vec<PreparedRows>,
        cancelled: Option<Arc<AtomicBool>>,
    },
    #[cfg(test)]
    Pause(oneshot::Sender<()>, std::sync::mpsc::Receiver<()>),
}
enum Output {
    Ack,
    Prepared(PreparedRows),
    PreparedCandidates(PreparedCandidates),
    Candidates(Vec<Candidate>, Box<dyn Reservation>),
    Rows(Vec<Value>, Box<dyn Reservation>),
    FamilyRoot(CompletedFamilyRoot),
    Unit(Box<dyn Any + Send>, Box<dyn Reservation>),
}
struct Command {
    input: Input,
    reply: oneshot::Sender<Reply>,
    _transfer: Option<Box<dyn Reservation>>,
}
fn candidate_bytes(candidate: &Candidate) -> Result<usize, ModelError> {
    let factor = match &candidate.node.key {
        RecordIdKey::String(_) | RecordIdKey::Number(_) | RecordIdKey::Uuid(_) => 4,
        _ => 2 * std::mem::size_of::<Value>() + 4,
    };
    Ok(serde_json::to_vec(candidate)
        .map_err(ModelError::codec)?
        .len()
        .saturating_mul(factor)
        .saturating_add(std::mem::size_of::<Candidate>()))
}
pub(crate) fn physical_bytes(row: &Value) -> Result<usize, ModelError> {
    // Count without allocating an encoded copy before transfer admission. Actual frame
    // encoding remains in the blocking sorter under its transferred data/work arena.
    struct FrameBytes(usize);
    impl std::io::Write for FrameBytes {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0 = self.0.saturating_add(bytes.len());
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut frame = FrameBytes(0);
    serde_json::to_writer(&mut frame, row).map_err(ModelError::codec)?;
    Ok(crate::ordered_rows::physical_work_bytes(row, frame.0))
}
enum Kernel {
    Candidates(SortedCandidates),
    Rows(SortedRows),
    OrderedCandidates(OrderedCandidates),
    OrderedRows(OrderedRows),
    Taken,
}
struct Owner {
    sender: Option<mpsc::Sender<Command>>,
    pending: Option<oneshot::Receiver<Reply>>,
    terminal: BlockingTerminal,
    budget: ResourceBudget,
    drain_error: Option<Arc<ModelError>>,
}
impl Owner {
    async fn new(
        budget: &ResourceBudget,
        guard: impl Send + 'static,
        physical_row_bytes: Option<usize>,
        prepared: Option<PreparedRows>,
        prepared_candidates: Option<PreparedCandidates>,
        fold_control: Option<Box<dyn Reservation>>,
        register: Option<Box<dyn FnOnce(BlockingTerminal) + Send>>,
    ) -> Result<Self, ModelError> {
        let fold_only = fold_control.is_some();
        let (sender, mut receiver) = mpsc::channel::<Command>(1);
        let budget = budget.clone();
        let worker_budget = budget.clone();
        let (ready_tx, ready) = oneshot::channel();
        let tracing = lctx_tracing::Context::current();
        let worker = tokio::task::spawn_blocking(move || {
            tracing.scope(move || {
                let _guard = guard;
                let _control = match fold_control
                    .map_or_else(|| worker_budget.reserve("ordered-command-owner", 2048), Ok)
                {
                    Ok(reservation) => reservation,
                    Err(error) => {
                        let error = Arc::new(error);
                        let _ = ready_tx.send(Err(error.clone()));
                        return Err(error);
                    }
                };
                let initial = if fold_only {
                    Ok(Kernel::Taken)
                } else if let Some(prepared) = prepared {
                    prepared
                        .cursor_with_budget(&worker_budget)
                        .map(Kernel::OrderedRows)
                } else if let Some(prepared) = prepared_candidates {
                    prepared
                        .cursor_with_budget(&worker_budget)
                        .map(Kernel::OrderedCandidates)
                } else if let Some(row_bytes) = physical_row_bytes {
                    SortedRows::with_budget_and_row_bytes(&worker_budget, row_bytes)
                        .map(Kernel::Rows)
                } else {
                    SortedCandidates::new(&worker_budget).map(Kernel::Candidates)
                };
                let mut kernel = match initial {
                    Ok(kernel) => {
                        let _ = ready_tx.send(Ok(()));
                        kernel
                    }
                    Err(error) => {
                        let error = Arc::new(error);
                        let _ = ready_tx.send(Err(error.clone()));
                        return Err(error);
                    }
                };
                while let Some(command) = receiver.blocking_recv() {
                    let result = (|| match command.input {
                        #[cfg(test)]
                        Input::Pause(entered, gate) => {
                            let _ = entered.send(());
                            gate.recv().map_err(ModelError::codec)?;
                            Ok(Output::Ack)
                        }
                        Input::Candidate(value) => match &mut kernel {
                            Kernel::Candidates(sort) => {
                                sort.push(value)?;
                                Ok(Output::Ack)
                            }
                            _ => Err(ModelError::Conflict("candidate sorting state")),
                        },
                        Input::Row(value) => match &mut kernel {
                            Kernel::Rows(sort) => {
                                sort.push(value)?;
                                Ok(Output::Ack)
                            }
                            _ => Err(ModelError::Conflict("physical sorting state")),
                        },
                        Input::RowBatch(values) => match &mut kernel {
                            Kernel::Rows(sort) => {
                                for value in values {
                                    sort.push(value)?;
                                }
                                Ok(Output::Ack)
                            }
                            _ => Err(ModelError::Conflict("physical sorting state")),
                        },
                        Input::OwnedRowBatch(values) => match &mut kernel {
                            Kernel::Rows(sort) => {
                                for (value, arena) in values {
                                    sort.push_owned(value, arena)?;
                                }
                                Ok(Output::Ack)
                            }
                            _ => Err(ModelError::Conflict("physical sorting state")),
                        },
                        Input::Unit(work) => {
                            if !matches!(kernel, Kernel::Taken) {
                                return Err(ModelError::Conflict("blocking unit worker state"));
                            }
                            work(&worker_budget)
                        }
                        Input::Finish => {
                            kernel = match std::mem::replace(&mut kernel, Kernel::Taken) {
                                Kernel::Candidates(sort) => {
                                    Kernel::OrderedCandidates(sort.finish()?)
                                }
                                Kernel::Rows(sort) => Kernel::OrderedRows(sort.finish()?),
                                ordered @ Kernel::OrderedCandidates(_)
                                | ordered @ Kernel::OrderedRows(_) => ordered,
                                _ => {
                                    return Err(ModelError::Conflict("external sorting state"));
                                }
                            };
                            Ok(Output::Ack)
                        }
                        Input::Prepare => match std::mem::replace(&mut kernel, Kernel::Taken) {
                            Kernel::OrderedCandidates(ordered) => {
                                Ok(Output::PreparedCandidates(ordered.into_prepared()))
                            }
                            Kernel::OrderedRows(ordered) => {
                                Ok(Output::Prepared(ordered.into_prepared()))
                            }
                            other => {
                                kernel = other;
                                Err(ModelError::Conflict("physical preparation state"))
                            }
                        },
                        Input::Next(max) => {
                            if max == 0 || max > crate::loader::NATIVE_WINDOW_ROWS {
                                return Err(ModelError::Schema("ordered transfer window"));
                            }
                            let mut retained =
                                worker_budget.reserve("ordered-command-output", 0)?;
                            match &mut kernel {
                                Kernel::OrderedCandidates(ordered) => {
                                    let mut batch = Vec::new();
                                    let mut bytes = 0usize;
                                    for _ in 0..max {
                                        let Some(value) = ordered.next_candidate()? else {
                                            break;
                                        };
                                        bytes = bytes.saturating_add(candidate_bytes(&value)?);
                                        retained.try_resize(bytes)?;
                                        batch.push(value);
                                    }
                                    Ok(Output::Candidates(batch, retained))
                                }
                                Kernel::OrderedRows(ordered) => {
                                    let mut batch = Vec::new();
                                    let mut bytes = 0usize;
                                    for _ in 0..max {
                                        let Some(value) = ordered.next_row()? else {
                                            break;
                                        };
                                        bytes = bytes.saturating_add(physical_bytes(&value)?);
                                        retained.try_resize(bytes)?;
                                        batch.push(value);
                                    }
                                    Ok(Output::Rows(batch, retained))
                                }
                                _ => Err(ModelError::Conflict("ordered reading state")),
                            }
                        }
                        Input::FoldFamily {
                            family,
                            runs,
                            cancelled,
                        } => {
                            if !matches!(kernel, Kernel::Taken) {
                                return Err(ModelError::Conflict("canonical fold worker state"));
                            }
                            let _hash = worker_budget.reserve(
                                "ordered-canonical-family-hash",
                                std::mem::size_of::<CompletedFamilyHasher>()
                                    + std::mem::size_of::<CompletedFamilyRoot>(),
                            )?;
                            let mut hash = CompletedFamilyHasher::new(family)?;
                            PreparedRows::fold_union(
                                &runs,
                                &worker_budget,
                                || crate::prepared::check_read_cancellation(cancelled.as_ref()),
                                |_, payload| hash.row(payload),
                            )?;
                            crate::prepared::check_read_cancellation(cancelled.as_ref())?;
                            Ok(Output::FamilyRoot(hash.finish()))
                        }
                        Input::UnionCandidates { runs, cancelled } => {
                            if !matches!(kernel, Kernel::Taken) {
                                return Err(ModelError::Conflict("compact union worker state"));
                            }
                            PreparedCandidates::union_prepared(&runs, &worker_budget, || {
                                crate::prepared::check_read_cancellation(cancelled.as_ref())
                            })
                            .map(Output::PreparedCandidates)
                        }
                        Input::UnionRows { runs, cancelled } => {
                            if !matches!(kernel, Kernel::Taken) {
                                return Err(ModelError::Conflict("canonical union worker state"));
                            }
                            PreparedRows::union_prepared(&runs, &worker_budget, || {
                                crate::prepared::check_read_cancellation(cancelled.as_ref())
                            })
                            .map(Output::Prepared)
                        }
                    })();
                    match result {
                        Ok(output) => {
                            let _ = command.reply.send(Ok(output));
                        }
                        Err(error) => {
                            let error = Arc::new(error);
                            let _ = command.reply.send(Err(error.clone()));
                            return Err(error);
                        }
                    }
                }
                Ok(())
            })
        });
        let terminal = async move {
            worker.await.map_err(|error| {
                Arc::new(ModelError::infrastructure(
                    lctx_model::domain::Infrastructure::Unconfirmed,
                    format!("blocking worker terminal unconfirmed: {error}"),
                ))
            })?
        }
        .boxed()
        .shared();
        if let Some(register) = register {
            register(terminal.clone());
        }
        let owner = Self {
            sender: Some(sender),
            pending: None,
            terminal,
            budget,
            drain_error: None,
        };
        match ready.await {
            Ok(Ok(())) => Ok(owner),
            Ok(Err(error)) => Err(ModelError::SharedCause(error)),
            Err(_) => {
                owner
                    .terminal
                    .clone()
                    .await
                    .map_err(ModelError::SharedCause)?;
                Err(ModelError::Conflict("sorting worker initialization"))
            }
        }
    }
    async fn acknowledge(&mut self) -> Result<Option<Output>, ModelError> {
        let Some(pending) = &mut self.pending else {
            return Ok(None);
        };
        let response = pending.await;
        self.pending.take();
        match response {
            Ok(result) => result.map(Some).map_err(ModelError::SharedCause),
            Err(_) => {
                self.terminal
                    .clone()
                    .await
                    .map_err(ModelError::SharedCause)?;
                Err(ModelError::Conflict("sorting worker acknowledgement"))
            }
        }
    }
    fn submit(&mut self, input: Input, bytes: usize) -> Result<(), ModelError> {
        let transfer = self.budget.reserve("ordered-command-input", bytes)?;
        self.submit_reserved(input, transfer)
    }
    fn submit_reserved(
        &mut self,
        input: Input,
        transfer: Box<dyn Reservation>,
    ) -> Result<(), ModelError> {
        let (reply, pending) = oneshot::channel();
        self.sender
            .as_ref()
            .ok_or(ModelError::Conflict("sorting owner closed"))?
            .try_send(Command {
                input,
                reply,
                _transfer: Some(transfer),
            })
            .map_err(|_| ModelError::Conflict("sorting worker admission"))?;
        self.pending = Some(pending);
        Ok(())
    }
    async fn push(&mut self, input: Input, bytes: usize) -> Result<(), ModelError> {
        self.acknowledge().await?;
        self.submit(input, bytes)?;
        self.acknowledge().await?;
        Ok(())
    }
    async fn finish(&mut self) -> Result<Self, ModelError> {
        self.acknowledge().await?;
        self.submit(Input::Finish, 0)?;
        self.acknowledge().await?;
        Ok(Self {
            sender: self.sender.take(),
            pending: None,
            terminal: self.terminal.clone(),
            budget: self.budget.clone(),
            drain_error: None,
        })
    }
    async fn prepare(&mut self) -> Result<PreparedRows, ModelError> {
        self.acknowledge().await?;
        self.submit(Input::Finish, 0)?;
        self.acknowledge().await?;
        self.submit(Input::Prepare, 0)?;
        let prepared = match self.acknowledge().await? {
            Some(Output::Prepared(prepared)) => prepared,
            _ => return Err(ModelError::Schema("prepared physical response")),
        };
        // A ready run cannot retain the creation worker's admission lease. Terminality
        // is acknowledged before the immutable handle escapes to final readers.
        self.drain().await?;
        Ok(prepared)
    }
    async fn prepare_candidates(&mut self) -> Result<PreparedCandidates, ModelError> {
        self.acknowledge().await?;
        self.submit(Input::Finish, 0)?;
        self.acknowledge().await?;
        self.submit(Input::Prepare, 0)?;
        let prepared = match self.acknowledge().await? {
            Some(Output::PreparedCandidates(prepared)) => prepared,
            _ => return Err(ModelError::Schema("prepared candidate response")),
        };
        self.drain().await?;
        Ok(prepared)
    }
    async fn next(&mut self, max: usize) -> Result<Output, ModelError> {
        if self.pending.is_none() {
            self.submit(Input::Next(max), 0)?;
        }
        self.acknowledge()
            .await?
            .ok_or(ModelError::Conflict("ordered worker response"))
    }
    async fn drain(&mut self) -> Result<(), ModelError> {
        self.sender.take();
        if let Err(error) = self.acknowledge().await {
            self.drain_error = Some(Arc::new(error));
        }
        let mut completion = lctx_model::domain::completion::Completion::default();
        completion.step(
            "external sorting worker terminal",
            self.terminal.clone().await.map_err(ModelError::SharedCause),
        );
        let primary = self
            .drain_error
            .as_ref()
            .map_or(Ok(()), |error| Err(ModelError::SharedCause(error.clone())));
        lctx_model::domain::completion::complete(primary, completion)
    }
}

/// Execute an owned synchronous preparation unit on the existing acknowledged worker.
///
/// `input_charge` covers captured input storage not already owned by charged products.
/// The closure must admit its working allocations and return a product that retains its
/// own dynamic allocation reservations. Command/result control storage is admitted here.
/// Registration happens before the initialization await, and both inputs and `guard`
/// remain owned until the actual blocking terminal, even if this future is abandoned.
pub async fn run_registered<T, F>(
    budget: &ResourceBudget,
    guard: impl Send + 'static,
    register: impl FnOnce(BlockingTerminal) + Send + 'static,
    input_charge: Box<dyn Reservation>,
    work: F,
) -> Result<T, ModelError>
where
    T: Send + 'static,
    F: FnOnce(&ResourceBudget) -> Result<T, ModelError> + Send + 'static,
{
    let packet = budget.reserve(
        "ordered-unit-packet",
        std::mem::size_of::<Input>()
            .saturating_add(std::mem::size_of::<F>())
            .saturating_add(std::mem::size_of::<T>()),
    )?;
    let control = budget.reserve("ordered-command-owner", 2048)?;
    let mut owner = Owner::new(
        budget,
        (guard, input_charge),
        None,
        None,
        None,
        Some(control),
        Some(Box::new(register)),
    )
    .await?;
    let result = async {
        let transfer = budget.reserve("ordered-command-input", 0)?;
        owner.submit_reserved(
            Input::Unit(Box::new(move |budget| {
                work(budget).map(|value| Output::Unit(Box::new(value), packet))
            })),
            transfer,
        )?;
        match owner.acknowledge().await? {
            Some(Output::Unit(value, _retained)) => value
                .downcast::<T>()
                .map(|value| *value)
                .map_err(|_| ModelError::Schema("blocking unit response type")),
            _ => Err(ModelError::Schema("blocking unit response")),
        }
    }
    .await;
    let mut completion = lctx_model::domain::completion::Completion::default();
    completion.step("blocking preparation unit drainage", owner.drain().await);
    lctx_model::domain::completion::complete(result, completion)
}

pub struct AsyncCandidateSort(Owner);
impl AsyncCandidateSort {
    /// Exact compact union retains the same registered blocking worker, transfer
    /// and terminality ownership as physical ordering.
    pub async fn union_registered(
        runs: Vec<PreparedCandidates>,
        budget: &ResourceBudget,
        guard: impl Send + 'static,
        register: impl FnOnce(BlockingTerminal) + Send + 'static,
        cancellation: Option<Arc<AtomicBool>>,
    ) -> Result<PreparedCandidates, ModelError> {
        let transfer = budget.reserve(
            "ordered-command-input",
            std::mem::size_of::<Input>().saturating_add(
                runs.capacity()
                    .saturating_mul(std::mem::size_of::<PreparedCandidates>()),
            ),
        )?;
        let control = budget.reserve("ordered-command-owner", 2048)?;
        let mut owner = Owner::new(
            budget,
            guard,
            None,
            None,
            None,
            Some(control),
            Some(Box::new(register)),
        )
        .await?;
        let result = async {
            owner.submit_reserved(
                Input::UnionCandidates {
                    runs,
                    cancelled: cancellation,
                },
                transfer,
            )?;
            match owner.acknowledge().await? {
                Some(Output::PreparedCandidates(rows)) => Ok(rows),
                _ => Err(ModelError::Schema("compact union response")),
            }
        }
        .await;
        let mut completion = lctx_model::domain::completion::Completion::default();
        completion.step("compact union worker drainage", owner.drain().await);
        lctx_model::domain::completion::complete(result, completion)
    }

    pub async fn new(budget: &ResourceBudget) -> Result<Self, ModelError> {
        Self::new_with_owner(budget, ()).await
    }
    pub async fn new_with_owner(
        budget: &ResourceBudget,
        owner: impl Send + 'static,
    ) -> Result<Self, ModelError> {
        Owner::new(budget, owner, None, None, None, None, None)
            .await
            .map(Self)
    }
    /// Register a join observer before the first initialization await. The observer can
    /// retain an operation lease and report late errors after this caller is dropped.
    pub async fn new_registered(
        budget: &ResourceBudget,
        owner: impl Send + 'static,
        register: impl FnOnce(BlockingTerminal) + Send + 'static,
    ) -> Result<Self, ModelError> {
        Owner::new(
            budget,
            owner,
            None,
            None,
            None,
            None,
            Some(Box::new(register)),
        )
        .await
        .map(Self)
    }
    pub async fn push(&mut self, candidate: Candidate) -> Result<(), ModelError> {
        let bytes = candidate_bytes(&candidate)?;
        self.0.push(Input::Candidate(candidate), bytes).await
    }
    pub async fn finish(&mut self) -> Result<AsyncOrderedCandidates, ModelError> {
        self.0.finish().await.map(|owner| AsyncOrderedCandidates {
            owner,
            retained: None,
            exhausted: false,
        })
    }
    pub async fn prepare(&mut self) -> Result<PreparedCandidates, ModelError> {
        self.0.prepare_candidates().await
    }
    pub async fn drain(&mut self) -> Result<(), ModelError> {
        self.0.drain().await
    }
}
pub struct AsyncOrderedCandidates {
    owner: Owner,
    retained: Option<Box<dyn Reservation>>,
    exhausted: bool,
}
impl AsyncOrderedCandidates {
    pub async fn new_registered(
        prepared: PreparedCandidates,
        budget: &ResourceBudget,
        owner: impl Send + 'static,
        register: impl FnOnce(BlockingTerminal) + Send + 'static,
    ) -> Result<Self, ModelError> {
        Owner::new(
            budget,
            owner,
            None,
            None,
            Some(prepared),
            None,
            Some(Box::new(register)),
        )
        .await
        .map(|owner| Self {
            owner,
            retained: None,
            exhausted: false,
        })
    }
    pub async fn next_batch(&mut self, max: usize) -> Result<Vec<Candidate>, ModelError> {
        if self.exhausted {
            self.owner.drain().await?;
            return Ok(Vec::new());
        }
        self.retained.take();
        match self.owner.next(max).await? {
            Output::Candidates(batch, retained) => {
                if batch.is_empty() {
                    self.exhausted = true;
                    drop(retained);
                    self.owner.drain().await?;
                } else {
                    self.retained = Some(retained);
                }
                Ok(batch)
            }
            _ => Err(ModelError::Schema("ordered candidate response")),
        }
    }
    pub async fn drain(&mut self) -> Result<(), ModelError> {
        self.retained.take();
        self.owner.drain().await
    }
}
pub struct AsyncPhysicalSort(Owner);
impl AsyncPhysicalSort {
    /// Build the exact union on the same registered blocking owner used for ordering.
    /// The returned run has no creation-worker lease and remains reusable independently.
    pub async fn union_registered(
        runs: Vec<PreparedRows>,
        budget: &ResourceBudget,
        guard: impl Send + 'static,
        register: impl FnOnce(BlockingTerminal) + Send + 'static,
        cancellation: Option<Arc<AtomicBool>>,
    ) -> Result<PreparedRows, ModelError> {
        let transfer = budget.reserve(
            "ordered-command-input",
            std::mem::size_of::<Input>().saturating_add(
                runs.capacity()
                    .saturating_mul(std::mem::size_of::<PreparedRows>()),
            ),
        )?;
        let control = budget.reserve("ordered-command-owner", 2048)?;
        let mut owner = Owner::new(
            budget,
            guard,
            None,
            None,
            None,
            Some(control),
            Some(Box::new(register)),
        )
        .await?;
        let result = async {
            owner.submit_reserved(
                Input::UnionRows {
                    runs,
                    cancelled: cancellation,
                },
                transfer,
            )?;
            match owner.acknowledge().await? {
                Some(Output::Prepared(rows)) => Ok(rows),
                _ => Err(ModelError::Schema("canonical union response")),
            }
        }
        .await;
        let mut completion = lctx_model::domain::completion::Completion::default();
        completion.step(
            "canonical ordered union worker drainage",
            owner.drain().await,
        );
        lctx_model::domain::completion::complete(result, completion)
    }
    /// Hash an already normalized immutable family on the existing acknowledged
    /// blocking owner. Registration precedes initialization awaits; the supplied guard
    /// remains in the worker through command completion and terminal drainage.
    pub async fn fold_union_registered(
        family: RecoveryFamily,
        runs: Vec<PreparedRows>,
        budget: &ResourceBudget,
        guard: impl Send + 'static,
        register: impl FnOnce(BlockingTerminal) + Send + 'static,
        cancellation: Option<Arc<AtomicBool>>,
    ) -> Result<CompletedFamilyRoot, ModelError> {
        // The caller owns allocation of its incoming Vec. Charge the transferred
        // capacity before creating a command channel, registration box or worker.
        let transfer = budget.reserve(
            "ordered-command-input",
            std::mem::size_of::<Input>().saturating_add(
                runs.capacity()
                    .saturating_mul(std::mem::size_of::<PreparedRows>()),
            ),
        )?;
        let control = budget.reserve("ordered-command-owner", 2048)?;
        let mut owner = Owner::new(
            budget,
            guard,
            None,
            None,
            None,
            Some(control),
            Some(Box::new(register)),
        )
        .await?;
        let result = async {
            owner.submit_reserved(
                Input::FoldFamily {
                    family,
                    runs,
                    cancelled: cancellation,
                },
                transfer,
            )?;
            match owner.acknowledge().await? {
                Some(Output::FamilyRoot(root)) => Ok(root),
                _ => Err(ModelError::Schema("completed family fold response")),
            }
        }
        .await;
        let mut completion = lctx_model::domain::completion::Completion::default();
        completion.step("canonical family fold worker drainage", owner.drain().await);
        lctx_model::domain::completion::complete(result, completion)
    }
    pub async fn new_registered(
        budget: &ResourceBudget,
        owner: impl Send + 'static,
        register: impl FnOnce(BlockingTerminal) + Send + 'static,
    ) -> Result<Self, ModelError> {
        Self::new_registered_with_row_bytes(budget, owner, register, crate::ordered_rows::ROW_BYTES)
            .await
    }
    pub async fn new_registered_with_row_bytes(
        budget: &ResourceBudget,
        owner: impl Send + 'static,
        register: impl FnOnce(BlockingTerminal) + Send + 'static,
        row_bytes: usize,
    ) -> Result<Self, ModelError> {
        Owner::new(
            budget,
            owner,
            Some(row_bytes),
            None,
            None,
            None,
            Some(Box::new(register)),
        )
        .await
        .map(Self)
    }
    pub async fn push(&mut self, row: Value) -> Result<(), ModelError> {
        let bytes = physical_bytes(&row)?;
        self.0.push(Input::Row(row), bytes).await
    }
    /// Transfer one bounded physical page with a single worker acknowledgement.
    pub async fn push_batch(&mut self, rows: Vec<Value>) -> Result<(), ModelError> {
        if rows.len() > crate::loader::NATIVE_WINDOW_ROWS {
            return Err(ModelError::Schema("physical input transfer window"));
        }
        let mut bytes = rows.capacity().saturating_mul(std::mem::size_of::<Value>());
        for row in &rows {
            bytes = bytes.saturating_add(physical_bytes(row)?);
        }
        let retained = self.0.budget.reserve("ordered-command-input", bytes)?;
        self.push_batch_reserved(rows, retained).await
    }
    /// Transfer an already admitted page arena. Its reservation must cover Vec capacity,
    /// decoded rows and encoding workspace until acknowledgement; the sorter separately
    /// owns retained encoded runs. The owner keeps this arena on an abandoned waiter.
    pub async fn push_batch_reserved(
        &mut self,
        rows: Vec<Value>,
        retained: Box<dyn Reservation>,
    ) -> Result<(), ModelError> {
        if rows.len() > crate::loader::NATIVE_WINDOW_ROWS {
            return Err(ModelError::Schema("physical input transfer window"));
        }
        self.0.acknowledge().await?;
        self.0.submit_reserved(Input::RowBatch(rows), retained)?;
        self.0.acknowledge().await?;
        Ok(())
    }
    /// Transfer per-row same-pool data/encoding arenas without readmitting their bytes.
    /// Each arena follows its row into `SortedRows::push_owned`, so dead decoded input
    /// is released before spill/merge admission. Only the batch Vec capacity is new.
    pub async fn push_owned_batch(
        &mut self,
        rows: Vec<(Value, Box<dyn Reservation>)>,
    ) -> Result<(), ModelError> {
        if rows.len() > crate::loader::NATIVE_WINDOW_ROWS {
            return Err(ModelError::Schema("physical input transfer window"));
        }
        let retained = self.0.budget.reserve(
            "ordered-command-input",
            rows.capacity()
                .saturating_mul(std::mem::size_of::<(Value, Box<dyn Reservation>)>()),
        )?;
        self.push_owned_batch_reserved(rows, retained).await
    }
    /// As `push_owned_batch`, with batch Vec capacity already admitted by its producer.
    pub async fn push_owned_batch_reserved(
        &mut self,
        rows: Vec<(Value, Box<dyn Reservation>)>,
        retained: Box<dyn Reservation>,
    ) -> Result<(), ModelError> {
        if rows.len() > crate::loader::NATIVE_WINDOW_ROWS {
            return Err(ModelError::Schema("physical input transfer window"));
        }
        self.0.acknowledge().await?;
        self.0
            .submit_reserved(Input::OwnedRowBatch(rows), retained)?;
        self.0.acknowledge().await?;
        Ok(())
    }
    pub async fn prepare(&mut self) -> Result<PreparedRows, ModelError> {
        self.0.prepare().await
    }
    /// Consume the creation owner and join its worker on every outcome, retaining the
    /// primary preparation error alongside any independent drainage failure.
    pub async fn prepare_terminal(mut self) -> Result<PreparedRows, ModelError> {
        let result = self.0.prepare().await;
        let mut completion = lctx_model::domain::completion::Completion::default();
        completion.step("physical preparation worker drainage", self.0.drain().await);
        lctx_model::domain::completion::complete(result, completion)
    }
    pub async fn finish(&mut self) -> Result<AsyncOrderedRows, ModelError> {
        self.0.finish().await.map(|owner| AsyncOrderedRows {
            owner,
            retained: None,
            exhausted: false,
        })
    }
}
pub struct AsyncOrderedRows {
    owner: Owner,
    retained: Option<Box<dyn Reservation>>,
    exhausted: bool,
}
impl AsyncOrderedRows {
    pub async fn new_registered(
        prepared: PreparedRows,
        budget: &ResourceBudget,
        owner: impl Send + 'static,
        register: impl FnOnce(BlockingTerminal) + Send + 'static,
    ) -> Result<Self, ModelError> {
        Owner::new(
            budget,
            owner,
            Some(crate::ordered_rows::ROW_BYTES),
            Some(prepared),
            None,
            None,
            Some(Box::new(register)),
        )
        .await
        .map(|owner| Self {
            owner,
            retained: None,
            exhausted: false,
        })
    }
    pub async fn next_batch(&mut self, max: usize) -> Result<Vec<Value>, ModelError> {
        if self.exhausted {
            self.owner.drain().await?;
            return Ok(Vec::new());
        }
        self.retained.take();
        match self.owner.next(max).await? {
            Output::Rows(batch, retained) => {
                if batch.is_empty() {
                    self.exhausted = true;
                    drop(retained);
                    self.owner.drain().await?;
                } else {
                    self.retained = Some(retained);
                }
                Ok(batch)
            }
            _ => Err(ModelError::Schema("ordered physical response")),
        }
    }
    pub async fn drain(&mut self) -> Result<(), ModelError> {
        self.retained.take();
        self.owner.drain().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use surrealdb::types::RecordId;
    struct Guard(Arc<AtomicBool>);
    impl Drop for Guard {
        fn drop(&mut self) {
            self.0.store(true, Ordering::Release);
        }
    }
    fn candidate(key: u8) -> Candidate {
        let mut nominal = [0; 16];
        nominal[15] = key;
        Candidate {
            content: None,
            relation: "package".into(),
            key: nominal,
            node: RecordId::new("entity", format!("{:03}", 255 - key)),
        }
    }
    fn physical(key: &str, value: Value) -> Value {
        let mut row = surrealdb::types::Object::new();
        row.insert("id", RecordId::new("entity", key));
        row.insert("value", value);
        Value::Object(row)
    }
    fn prepared(budget: &ResourceBudget, rows: Vec<Value>) -> PreparedRows {
        let mut sorted = SortedRows::with_budget(budget).unwrap();
        for row in rows {
            sorted.push(row).unwrap();
        }
        sorted.finish().unwrap().into_prepared()
    }
    #[tokio::test]
    async fn bounded_physical_batches_keep_order_and_transfer_owned_arenas() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let released = Arc::new(AtomicBool::new(false));
        let mut sort = AsyncPhysicalSort::new_registered(&budget, Guard(released.clone()), |_| {})
            .await
            .unwrap();
        sort.push_batch(vec![physical("c", Value::Null), physical("a", Value::Null)])
            .await
            .unwrap();
        let row = physical("b", Value::Null);
        let arena = budget
            .reserve("test-owned-row", physical_bytes(&row).unwrap())
            .unwrap();
        sort.push_owned_batch(vec![(row, arena)]).await.unwrap();
        let rejected = vec![Value::Null; crate::loader::NATIVE_WINDOW_ROWS + 1];
        assert!(matches!(
            sort.push_batch(rejected).await,
            Err(ModelError::Schema("physical input transfer window"))
        ));
        // A pre-admission refusal cannot poison the already accepted run.
        let rows = sort.prepare_terminal().await.unwrap();
        assert!(released.load(Ordering::Acquire));
        let mut cursor = rows.cursor_with_budget(&budget).unwrap();
        for key in ["a", "b", "c"] {
            assert_eq!(cursor.next_row().unwrap(), Some(physical(key, Value::Null)));
        }
        assert!(cursor.next_row().unwrap().is_none());
        drop(cursor);
        drop(rows);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn physical_batch_capacity_refusal_preserves_previously_accepted_rows() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let mut sort = AsyncPhysicalSort::new_registered(&budget, (), |_| {})
            .await
            .unwrap();
        sort.push_batch(vec![physical("a", Value::Null)])
            .await
            .unwrap();
        let fill = budget
            .reserve("test-fill", budget.limit() - budget.reserved())
            .unwrap();
        assert!(matches!(
            sort.push_batch(vec![physical("b", Value::Null)]).await,
            Err(ModelError::Resource {
                owner: "ordered-command-input",
                ..
            })
        ));
        drop(fill);
        let rows = sort.prepare_terminal().await.unwrap();
        let mut cursor = rows.cursor_with_budget(&budget).unwrap();
        assert_eq!(cursor.next_row().unwrap(), Some(physical("a", Value::Null)));
        assert!(cursor.next_row().unwrap().is_none());
        drop(cursor);
        drop(rows);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn registered_preparation_unit_returns_owned_result_only_after_terminal() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let released = Arc::new(AtomicBool::new(false));
        let charge = budget.reserve("test-unit-input", 128).unwrap();
        let rows = run_registered(
            &budget,
            Guard(released.clone()),
            |_| {},
            charge,
            |budget| {
                let mut sort = SortedRows::with_budget(budget)?;
                sort.push(physical("b", Value::Null))?;
                sort.push(physical("a", Value::Null))?;
                Ok(sort.finish()?.into_prepared())
            },
        )
        .await
        .unwrap();
        assert!(released.load(Ordering::Acquire));
        let mut cursor = rows.cursor_with_budget(&budget).unwrap();
        assert_eq!(cursor.next_row().unwrap(), Some(physical("a", Value::Null)));
        assert_eq!(cursor.next_row().unwrap(), Some(physical("b", Value::Null)));
        assert!(cursor.next_row().unwrap().is_none());
        drop(cursor);
        drop(rows);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn lost_preparation_unit_waiter_retains_guard_and_reports_late_failure() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let released = Arc::new(AtomicBool::new(false));
        let (registered_tx, registered) = oneshot::channel();
        let (entered_tx, entered) = oneshot::channel();
        let (release, gate) = std::sync::mpsc::channel();
        let task_budget = budget.clone();
        let charge = budget.reserve("test-unit-input", 128).unwrap();
        let task_released = released.clone();
        let waiter = tokio::spawn(async move {
            run_registered::<(), _>(
                &task_budget,
                Guard(task_released),
                move |terminal| {
                    let _ = registered_tx.send(terminal);
                },
                charge,
                move |_| {
                    let _ = entered_tx.send(());
                    gate.recv().map_err(ModelError::codec)?;
                    Err(ModelError::Conflict("late preparation failure"))
                },
            )
            .await
        });
        let terminal = registered.await.unwrap();
        entered.await.unwrap();
        // A separate asynchronous task remains runnable while the blocking unit pauses.
        assert_eq!(tokio::spawn(async { 17 }).await.unwrap(), 17);
        waiter.abort();
        assert!(waiter.await.unwrap_err().is_cancelled());
        assert!(!released.load(Ordering::Acquire));
        assert!(budget.reserved() >= 128);
        release.send(()).unwrap();
        let error = terminal.await.unwrap_err();
        assert!(matches!(
            error.as_ref(),
            ModelError::Conflict("late preparation failure")
        ));
        assert!(released.load(Ordering::Acquire));
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn registered_canonical_fold_returns_root_after_drain_and_refuses_late_conflict() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let first = prepared(
            &budget,
            vec![physical("z", Value::Null), physical("a", Value::Null)],
        );
        let overlap = prepared(&budget, vec![physical("z", Value::Null)]);
        let before = budget.reserved();
        let released = Arc::new(AtomicBool::new(false));
        let (registered_tx, registered) = oneshot::channel();
        let root = AsyncPhysicalSort::fold_union_registered(
            RecoveryFamily::Memberships,
            vec![overlap, first.clone()],
            &budget,
            Guard(released.clone()),
            move |terminal| {
                let _ = registered_tx.send(terminal);
            },
            None,
        )
        .await
        .unwrap();
        assert!(released.load(Ordering::Acquire));
        registered.await.unwrap().await.unwrap();
        let mut expected = CompletedFamilyHasher::new(RecoveryFamily::Memberships).unwrap();
        for key in ["a", "z"] {
            expected
                .row(&serde_json::to_vec(&physical(key, Value::Null)).unwrap())
                .unwrap();
        }
        assert_eq!(root, expected.finish());
        assert!(
            budget.reserved() < before,
            "transferred overlap is released at worker terminal"
        );
        let released = Arc::new(AtomicBool::new(false));
        let union = AsyncPhysicalSort::union_registered(
            vec![first.clone(), first.clone()],
            &budget,
            Guard(released.clone()),
            |_| {},
            None,
        )
        .await
        .unwrap();
        assert!(released.load(Ordering::Acquire));
        let mut cursor = union.frame_cursor(&budget).unwrap();
        assert!(cursor.next_frame().unwrap().is_some());
        assert!(cursor.next_frame().unwrap().is_some());
        assert!(cursor.next_frame().unwrap().is_none());
        drop(cursor);
        drop(union);
        let conflict = prepared(&budget, vec![physical("z", Value::None)]);
        let released = Arc::new(AtomicBool::new(false));
        let error = AsyncPhysicalSort::fold_union_registered(
            RecoveryFamily::Memberships,
            vec![first.clone(), conflict],
            &budget,
            Guard(released.clone()),
            |_| {},
            None,
        )
        .await
        .unwrap_err();
        assert!(matches!(
            error.primary(),
            Some(ModelError::Conflict("conflicting physical row identity"))
        ));
        assert!(released.load(Ordering::Acquire));
        let released = Arc::new(AtomicBool::new(false));
        let error = AsyncPhysicalSort::fold_union_registered(
            RecoveryFamily::GraphPayloads,
            vec![],
            &budget,
            Guard(released.clone()),
            |_| {},
            None,
        )
        .await
        .unwrap_err();
        assert!(matches!(
            error.primary(),
            Some(ModelError::Conflict("not a completed logical family"))
        ));
        assert!(released.load(Ordering::Acquire));
        let mut cursor = first.frame_cursor(&budget).unwrap();
        assert!(cursor.next_frame().unwrap().is_some());
        assert!(cursor.next_frame().unwrap().is_some());
        assert!(cursor.next_frame().unwrap().is_none());
        drop(cursor);
        drop(first);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn terminal_physical_preparation_joins_failed_finish_before_returning() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let released = Arc::new(AtomicBool::new(false));
        let mut sort = AsyncPhysicalSort::new_registered(&budget, Guard(released.clone()), |_| {})
            .await
            .unwrap();
        sort.push(physical("a", Value::Null)).await.unwrap();
        sort.push(physical("a", Value::None)).await.unwrap();
        let error = match sort.prepare_terminal().await {
            Ok(_) => panic!("conflicting preparation cannot complete"),
            Err(error) => error,
        };
        assert!(matches!(
            error.primary(),
            Some(ModelError::Conflict("conflicting physical row identity"))
        ));
        assert!(released.load(Ordering::Acquire));
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn abandoned_queued_canonical_fold_keeps_guard_until_cancelled_worker_terminal() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let first = prepared(&budget, vec![physical("a", Value::Null)]);
        let before = budget.reserved();
        let released = Arc::new(AtomicBool::new(false));
        let cancelled = Arc::new(AtomicBool::new(false));
        let control = budget.reserve("ordered-command-owner", 2048).unwrap();
        let mut owner = Owner::new(
            &budget,
            Guard(released.clone()),
            None,
            None,
            None,
            Some(control),
            None,
        )
        .await
        .unwrap();
        let terminal = owner.terminal.clone();
        let (entered_tx, entered) = oneshot::channel();
        let (release, gate) = std::sync::mpsc::channel();
        let (reply, _) = oneshot::channel();
        owner
            .sender
            .as_ref()
            .unwrap()
            .try_send(Command {
                input: Input::Pause(entered_tx, gate),
                reply,
                _transfer: None,
            })
            .unwrap_or_else(|_| panic!("pause canonical worker"));
        entered.await.unwrap();
        owner
            .submit(
                Input::FoldFamily {
                    family: RecoveryFamily::Memberships,
                    runs: vec![first.clone()],
                    cancelled: Some(cancelled.clone()),
                },
                std::mem::size_of::<PreparedRows>(),
            )
            .unwrap();
        let mut wait = Box::pin(owner.acknowledge());
        assert!(futures::poll!(&mut wait).is_pending());
        drop(wait);
        assert!(!released.load(Ordering::Acquire));
        cancelled.store(true, Ordering::Release);
        drop(owner);
        assert!(!released.load(Ordering::Acquire));
        assert!(budget.reserved() > before);
        release.send(()).unwrap();
        let error = terminal.await.unwrap_err();
        assert!(error.to_string().contains("cancelled"));
        assert!(released.load(Ordering::Acquire));
        assert_eq!(budget.reserved(), before);
        let mut cursor = first.frame_cursor(&budget).unwrap();
        assert!(cursor.next_frame().unwrap().is_some());
        assert!(cursor.next_frame().unwrap().is_none());
        drop(cursor);
        drop(first);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn acknowledged_ordering_retains_pending_window_and_owner_until_terminal() {
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let terminal = Arc::new(AtomicBool::new(false));
        let mut sorter = AsyncCandidateSort::new_with_owner(&budget, Guard(terminal.clone()))
            .await
            .unwrap();
        for key in (0..200u8).rev() {
            sorter.push(candidate(key)).await.unwrap();
            sorter.push(candidate(key)).await.unwrap();
        }
        let mut ordered = sorter.finish().await.unwrap();
        assert!(!terminal.load(Ordering::Acquire));
        // Hold the real worker before a file read, cancel its first waiter, then
        // resume the same owner. The acknowledged window cannot be skipped.
        let (entered_tx, entered) = oneshot::channel();
        let (release, gate) = std::sync::mpsc::channel();
        let (reply, _) = oneshot::channel();
        ordered
            .owner
            .sender
            .as_ref()
            .unwrap()
            .try_send(Command {
                input: Input::Pause(entered_tx, gate),
                reply,
                _transfer: None,
            })
            .unwrap_or_else(|_| panic!("pause admission"));
        entered.await.unwrap();
        let mut interrupted = Box::pin(ordered.next_batch(31));
        assert!(futures::poll!(&mut interrupted).is_pending());
        drop(interrupted);
        release.send(()).unwrap();
        let first = ordered.next_batch(31).await.unwrap();
        assert_eq!(first, (0..31).map(candidate).collect::<Vec<_>>());
        let mut output = first;
        loop {
            let batch = ordered.next_batch(31).await.unwrap();
            if batch.is_empty() {
                break;
            }
            output.extend(batch);
        }
        assert_eq!(output, (0..200).map(candidate).collect::<Vec<_>>());
        assert!(terminal.load(Ordering::Acquire));
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn acknowledged_conflict_drain_keeps_typed_primary_and_releases_owner() {
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let terminal = Arc::new(AtomicBool::new(false));
        let mut sorter = AsyncCandidateSort::new_with_owner(&budget, Guard(terminal.clone()))
            .await
            .unwrap();
        sorter.push(candidate(1)).await.unwrap();
        let mut conflict = candidate(1);
        conflict.node = RecordId::new("entity", "other");
        sorter.push(conflict).await.unwrap();
        // Finish is queued without consuming its failed response, as with a cancelled await.
        sorter.0.submit(Input::Finish, 0).unwrap();
        let error = sorter.drain().await.unwrap_err();
        assert!(matches!(
            error.primary(),
            Some(ModelError::Conflict(
                "conflicting nominal candidate pointer"
            ))
        ));
        assert!(terminal.load(Ordering::Acquire));
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn prepared_run_drains_creator_and_independent_cursors_keep_scratch() {
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let terminal = Arc::new(AtomicBool::new(false));
        let mut sorter =
            AsyncPhysicalSort::new_registered(&budget, Guard(terminal.clone()), |_| {})
                .await
                .unwrap();
        for key in ["b", "a"] {
            let mut row = surrealdb::types::Object::new();
            row.insert("id", RecordId::new("entity", key));
            sorter.push(Value::Object(row)).await.unwrap();
        }
        let prepared = sorter.prepare().await.unwrap();
        assert!(terminal.load(Ordering::Acquire));
        let mut first = AsyncOrderedRows::new_registered(prepared.clone(), &budget, (), |_| {})
            .await
            .unwrap();
        let mut second = AsyncOrderedRows::new_registered(prepared.clone(), &budget, (), |_| {})
            .await
            .unwrap();
        assert_eq!(
            first.next_batch(1).await.unwrap(),
            second.next_batch(1).await.unwrap()
        );
        first.drain().await.unwrap();
        drop(first);
        drop(prepared);
        drop(sorter);
        assert_eq!(second.next_batch(1).await.unwrap().len(), 1);
        assert!(second.next_batch(1).await.unwrap().is_empty());
        drop(second);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn dropped_sorter_releases_guard_only_after_queued_work_exits() {
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let terminal = Arc::new(AtomicBool::new(false));
        let mut sorter = AsyncCandidateSort::new_with_owner(&budget, Guard(terminal.clone()))
            .await
            .unwrap();
        sorter
            .0
            .submit(Input::Candidate(candidate(3)), 1024)
            .unwrap();
        let acknowledgement = sorter.0.terminal.clone();
        drop(sorter);
        acknowledgement.await.unwrap();
        assert!(terminal.load(Ordering::Acquire));
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn interrupted_finish_keeps_scratch_and_can_resume_ordered_output() {
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let terminal = Arc::new(AtomicBool::new(false));
        let mut sorter = AsyncCandidateSort::new_with_owner(&budget, Guard(terminal.clone()))
            .await
            .unwrap();
        sorter.push(candidate(9)).await.unwrap();
        sorter.push(candidate(2)).await.unwrap();
        let (entered_tx, entered) = oneshot::channel();
        let (release, gate) = std::sync::mpsc::channel();
        let (reply, _) = oneshot::channel();
        sorter
            .0
            .sender
            .as_ref()
            .unwrap()
            .try_send(Command {
                input: Input::Pause(entered_tx, gate),
                reply,
                _transfer: None,
            })
            .unwrap_or_else(|_| panic!("pause admission"));
        entered.await.unwrap();
        let mut finish = Box::pin(sorter.finish());
        assert!(futures::poll!(&mut finish).is_pending());
        drop(finish);
        assert!(!terminal.load(Ordering::Acquire));
        assert!(budget.reserved() > 0);
        release.send(()).unwrap();
        let mut ordered = sorter.finish().await.unwrap();
        assert_eq!(
            ordered.next_batch(128).await.unwrap(),
            vec![candidate(2), candidate(9)]
        );
        assert!(ordered.next_batch(128).await.unwrap().is_empty());
        assert!(terminal.load(Ordering::Acquire));
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn abandoned_initialization_reports_late_typed_failure_to_registered_owner() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .max_blocking_threads(1)
            .build()
            .unwrap();
        runtime.block_on(async {
            let (entered_tx, entered) = oneshot::channel();
            let (release, gate) = std::sync::mpsc::channel();
            let blocker = tokio::task::spawn_blocking(move || {
                entered_tx.send(()).unwrap();
                gate.recv().unwrap();
            });
            entered.await.unwrap();
            let budget = ResourceBudget::fixed(1024).unwrap();
            let terminal = Arc::new(AtomicBool::new(false));
            let (registered_tx, registered) = oneshot::channel();
            let mut constructor = Box::pin(AsyncCandidateSort::new_registered(
                &budget,
                Guard(terminal.clone()),
                move |terminal| {
                    let _ = registered_tx.send(terminal);
                },
            ));
            assert!(futures::poll!(&mut constructor).is_pending());
            drop(constructor);
            let terminal_result = registered.await.unwrap();
            assert!(!terminal.load(Ordering::Acquire));
            release.send(()).unwrap();
            let error = terminal_result.await.unwrap_err();
            assert!(matches!(
                error.as_ref(),
                ModelError::Resource {
                    owner: "ordered-command-owner",
                    ..
                }
            ));
            assert!(terminal.load(Ordering::Acquire));
            assert_eq!(budget.reserved(), 0);
            blocker.await.unwrap();
        });
    }
}
