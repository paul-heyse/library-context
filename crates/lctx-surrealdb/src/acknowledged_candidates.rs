//! A bounded command owner for blocking external ordering. Cancellation keeps submitted
//! work and its acknowledgement in the owner; dropping the owner closes admission and
//! lets already acknowledged blocking work release scratch and guards at its terminal.
use crate::ordered_rows::{Candidate, SortedCandidates, OrderedCandidates, SortedRows, OrderedRows};
use futures::{FutureExt, future::{BoxFuture, Shared}};
use lctx_model::domain::{ModelError, resources::{ResourceBudget, Reservation}};
use std::sync::Arc;
use surrealdb::types::{Value,RecordIdKey};
use tokio::sync::{mpsc, oneshot};

type Reply = Result<Output, Arc<ModelError>>;
pub type BlockingTerminal = Shared<BoxFuture<'static, Result<(), Arc<ModelError>>>>;
enum Input { Candidate(Candidate), Row(Value), Finish, Next(usize),
    #[cfg(test)] Pause(oneshot::Sender<()>,std::sync::mpsc::Receiver<()>),
}
enum Output { Ack, Candidates(Vec<Candidate>, Box<dyn Reservation>), Rows(Vec<Value>, Box<dyn Reservation>) }
struct Command { input: Input, reply: oneshot::Sender<Reply>, _transfer: Option<Box<dyn Reservation>> }
fn candidate_bytes(candidate:&Candidate)->Result<usize,ModelError>{
    let factor=match &candidate.node.key {RecordIdKey::String(_)|RecordIdKey::Number(_)|RecordIdKey::Uuid(_)=>4,_=>2*std::mem::size_of::<Value>()+4};
    Ok(serde_json::to_vec(candidate).map_err(ModelError::codec)?.len().saturating_mul(factor).saturating_add(std::mem::size_of::<Candidate>()))
}
fn physical_bytes(row:&Value)->Result<usize,ModelError>{
    Ok(serde_json::to_vec(row).map_err(ModelError::codec)?.len().saturating_mul(2*std::mem::size_of::<Value>()+4).saturating_add(std::mem::size_of::<Value>()))
}
enum Kernel { Candidates(SortedCandidates), Rows(SortedRows), OrderedCandidates(OrderedCandidates), OrderedRows(OrderedRows), Taken }
struct Owner {
    sender: Option<mpsc::Sender<Command>>, pending: Option<oneshot::Receiver<Reply>>,
    terminal: BlockingTerminal, budget: ResourceBudget, drain_error:Option<Arc<ModelError>>,
}
impl Owner {
    async fn new(budget:&ResourceBudget, guard:impl Send+'static, physical:bool,register:Option<Box<dyn FnOnce(BlockingTerminal)+Send>>)->Result<Self,ModelError> {
        let (sender,mut receiver)=mpsc::channel::<Command>(1);
        let budget=budget.clone();let worker_budget=budget.clone();
        let (ready_tx,ready)=oneshot::channel();
        let worker=tokio::task::spawn_blocking(move || {
            let _guard=guard;
            let _control=match worker_budget.reserve("ordered-command-owner",2048) {Ok(reservation)=>reservation,Err(error)=>{let error=Arc::new(error);let _=ready_tx.send(Err(error.clone()));return Err(error);}};
            let initial=if physical {SortedRows::with_budget(&worker_budget).map(Kernel::Rows)}else{SortedCandidates::new(&worker_budget).map(Kernel::Candidates)};
            let mut kernel=match initial {Ok(kernel)=>{let _=ready_tx.send(Ok(()));kernel},Err(error)=>{let error=Arc::new(error);let _=ready_tx.send(Err(error.clone()));return Err(error);}};
            while let Some(command)=receiver.blocking_recv() {
                let result=(||{
                    match command.input {
                        #[cfg(test)]
                        Input::Pause(entered,gate)=>{let _=entered.send(());gate.recv().map_err(ModelError::codec)?;Ok(Output::Ack)},
                        Input::Candidate(value)=>match &mut kernel {Kernel::Candidates(sort)=>{sort.push(value)?;Ok(Output::Ack)},_=>Err(ModelError::Conflict("candidate sorting state"))},
                        Input::Row(value)=>match &mut kernel {Kernel::Rows(sort)=>{sort.push(value)?;Ok(Output::Ack)},_=>Err(ModelError::Conflict("physical sorting state"))},
                        Input::Finish=>{
                            kernel=match std::mem::replace(&mut kernel,Kernel::Taken) {Kernel::Candidates(sort)=>Kernel::OrderedCandidates(sort.finish()?),Kernel::Rows(sort)=>Kernel::OrderedRows(sort.finish()?),ordered@Kernel::OrderedCandidates(_)|ordered@Kernel::OrderedRows(_)=>ordered,_=>return Err(ModelError::Conflict("external sorting state"))};
                            Ok(Output::Ack)
                        },
                        Input::Next(max)=>{
                            if max==0 || max>crate::loader::NATIVE_WINDOW_ROWS {return Err(ModelError::Schema("ordered transfer window"));}
                            let mut retained=worker_budget.reserve("ordered-command-output",0)?;
                            match &mut kernel {
                                Kernel::OrderedCandidates(ordered)=>{
                                    let mut batch=Vec::new();let mut bytes=0usize;
                                    for _ in 0..max {let Some(value)=ordered.next_candidate()? else{break;};bytes=bytes.saturating_add(candidate_bytes(&value)?);retained.try_resize(bytes)?;batch.push(value);}
                                    Ok(Output::Candidates(batch,retained))
                                },
                                Kernel::OrderedRows(ordered)=>{
                                    let mut batch=Vec::new();let mut bytes=0usize;
                                    for _ in 0..max {let Some(value)=ordered.next_row()? else{break;};bytes=bytes.saturating_add(physical_bytes(&value)?);retained.try_resize(bytes)?;batch.push(value);}
                                    Ok(Output::Rows(batch,retained))
                                },
                                _=>Err(ModelError::Conflict("ordered reading state")),
                            }
                        },
                    }
                })();
                match result {Ok(output)=>{let _=command.reply.send(Ok(output));},Err(error)=>{let error=Arc::new(error);let _=command.reply.send(Err(error.clone()));return Err(error);}}
            }
            Ok(())
        });
        let terminal=async move {worker.await.map_err(|error|Arc::new(ModelError::Cause(Box::new(error))))?}.boxed().shared();
        if let Some(register)=register {register(terminal.clone());}
        let owner=Self{sender:Some(sender),pending:None,terminal,budget,drain_error:None};
        match ready.await {Ok(Ok(()))=>Ok(owner),Ok(Err(error))=>Err(ModelError::SharedCause(error)),Err(_)=>{owner.terminal.clone().await.map_err(ModelError::SharedCause)?;Err(ModelError::Conflict("sorting worker initialization"))}}
    }
    async fn acknowledge(&mut self)->Result<Option<Output>,ModelError> {
        let Some(pending)=&mut self.pending else{return Ok(None);};
        let response=pending.await;self.pending.take();
        match response {Ok(result)=>result.map(Some).map_err(ModelError::SharedCause),Err(_)=>{self.terminal.clone().await.map_err(ModelError::SharedCause)?;Err(ModelError::Conflict("sorting worker acknowledgement"))}}
    }
    fn submit(&mut self,input:Input,bytes:usize)->Result<(),ModelError> {
        let transfer=self.budget.reserve("ordered-command-input",bytes)?;
        let (reply,pending)=oneshot::channel();
        self.sender.as_ref().ok_or(ModelError::Conflict("sorting owner closed"))?.try_send(Command{input,reply,_transfer:Some(transfer)}).map_err(|_|ModelError::Conflict("sorting worker admission"))?;
        self.pending=Some(pending);Ok(())
    }
    async fn push(&mut self,input:Input,bytes:usize)->Result<(),ModelError> {
        self.acknowledge().await?;
        self.submit(input,bytes)?;self.acknowledge().await?;Ok(())
    }
    async fn finish(&mut self)->Result<Self,ModelError> {
        self.acknowledge().await?;
        self.submit(Input::Finish,0)?;self.acknowledge().await?;
        Ok(Self{sender:self.sender.take(),pending:None,terminal:self.terminal.clone(),budget:self.budget.clone(),drain_error:None})
    }
    async fn next(&mut self,max:usize)->Result<Output,ModelError> {
        if self.pending.is_none(){self.submit(Input::Next(max),0)?;}
        self.acknowledge().await?.ok_or(ModelError::Conflict("ordered worker response"))
    }
    async fn drain(&mut self)->Result<(),ModelError> {
        self.sender.take();if let Err(error)=self.acknowledge().await {self.drain_error=Some(Arc::new(error));}
        let mut completion=lctx_model::domain::completion::Completion::default();
        completion.step("external sorting worker terminal",self.terminal.clone().await.map_err(ModelError::SharedCause));
        let primary=self.drain_error.as_ref().map_or(Ok(()),|error|Err(ModelError::SharedCause(error.clone())));
        lctx_model::domain::completion::complete(primary,completion)
    }
}

pub struct AsyncCandidateSort(Owner);
impl AsyncCandidateSort {
    pub async fn new(budget:&ResourceBudget)->Result<Self,ModelError>{Self::new_with_owner(budget,()).await}
    pub async fn new_with_owner(budget:&ResourceBudget,owner:impl Send+'static)->Result<Self,ModelError>{Owner::new(budget,owner,false,None).await.map(Self)}
    /// Register a join observer before the first initialization await. The observer can
    /// retain an operation lease and report late errors after this caller is dropped.
    pub async fn new_registered(budget:&ResourceBudget,owner:impl Send+'static,register:impl FnOnce(BlockingTerminal)+Send+'static)->Result<Self,ModelError>{Owner::new(budget,owner,false,Some(Box::new(register))).await.map(Self)}
    pub async fn push(&mut self,candidate:Candidate)->Result<(),ModelError>{let bytes=candidate_bytes(&candidate)?;self.0.push(Input::Candidate(candidate),bytes).await}
    pub async fn finish(&mut self)->Result<AsyncOrderedCandidates,ModelError>{self.0.finish().await.map(|owner|AsyncOrderedCandidates{owner,retained:None,exhausted:false})}
    pub async fn drain(&mut self)->Result<(),ModelError>{self.0.drain().await}
}
pub struct AsyncOrderedCandidates {owner:Owner,retained:Option<Box<dyn Reservation>>,exhausted:bool}
impl AsyncOrderedCandidates {
    pub async fn next_batch(&mut self,max:usize)->Result<Vec<Candidate>,ModelError>{
        if self.exhausted {self.owner.drain().await?;return Ok(Vec::new());}
        self.retained.take();match self.owner.next(max).await? {Output::Candidates(batch,retained)=>{if batch.is_empty(){self.exhausted=true;drop(retained);self.owner.drain().await?;}else{self.retained=Some(retained);}Ok(batch)},_=>Err(ModelError::Schema("ordered candidate response"))}
    }
    pub async fn drain(&mut self)->Result<(),ModelError>{self.retained.take();self.owner.drain().await}
}
pub struct AsyncPhysicalSort(Owner);
impl AsyncPhysicalSort {
    pub async fn new_registered(budget:&ResourceBudget,owner:impl Send+'static,register:impl FnOnce(BlockingTerminal)+Send+'static)->Result<Self,ModelError>{Owner::new(budget,owner,true,Some(Box::new(register))).await.map(Self)}
    pub async fn push(&mut self,row:Value)->Result<(),ModelError>{let bytes=physical_bytes(&row)?;self.0.push(Input::Row(row),bytes).await}
    pub async fn finish(&mut self)->Result<AsyncOrderedRows,ModelError>{self.0.finish().await.map(|owner|AsyncOrderedRows{owner,retained:None,exhausted:false})}
}
pub struct AsyncOrderedRows {owner:Owner,retained:Option<Box<dyn Reservation>>,exhausted:bool}
impl AsyncOrderedRows {
    pub async fn next_batch(&mut self,max:usize)->Result<Vec<Value>,ModelError>{
        if self.exhausted {self.owner.drain().await?;return Ok(Vec::new());}
        self.retained.take();match self.owner.next(max).await? {Output::Rows(batch,retained)=>{if batch.is_empty(){self.exhausted=true;drop(retained);self.owner.drain().await?;}else{self.retained=Some(retained);}Ok(batch)},_=>Err(ModelError::Schema("ordered physical response"))}
    }
    pub async fn drain(&mut self)->Result<(),ModelError>{self.retained.take();self.owner.drain().await}
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool,Ordering};
    use surrealdb::types::RecordId;
    struct Guard(Arc<AtomicBool>);
    impl Drop for Guard {fn drop(&mut self){self.0.store(true,Ordering::Release);}}
    fn candidate(key:u8)->Candidate {let mut nominal=[0;16];nominal[15]=key;Candidate{relation:"package".into(),key:nominal,node:RecordId::new("entity",format!("{:03}",255-key))}}
    #[tokio::test]
    async fn acknowledged_ordering_retains_pending_window_and_owner_until_terminal() {
        let budget=ResourceBudget::fixed(8<<20).unwrap();let terminal=Arc::new(AtomicBool::new(false));
        let mut sorter=AsyncCandidateSort::new_with_owner(&budget,Guard(terminal.clone())).await.unwrap();
        for key in (0..200u8).rev(){sorter.push(candidate(key)).await.unwrap();sorter.push(candidate(key)).await.unwrap();}
        let mut ordered=sorter.finish().await.unwrap();
        assert!(!terminal.load(Ordering::Acquire));
        // Hold the real worker before a file read, cancel its first waiter, then
        // resume the same owner. The acknowledged window cannot be skipped.
        let (entered_tx,entered)=oneshot::channel();let (release,gate)=std::sync::mpsc::channel();let (reply,_)=oneshot::channel();
        ordered.owner.sender.as_ref().unwrap().try_send(Command{input:Input::Pause(entered_tx,gate),reply,_transfer:None}).unwrap_or_else(|_|panic!("pause admission"));
        entered.await.unwrap();
        let mut interrupted=Box::pin(ordered.next_batch(31));assert!(futures::poll!(&mut interrupted).is_pending());drop(interrupted);release.send(()).unwrap();
        let first=ordered.next_batch(31).await.unwrap();
        assert_eq!(first,(0..31).map(candidate).collect::<Vec<_>>());
        let mut output=first;
        loop {let batch=ordered.next_batch(31).await.unwrap();if batch.is_empty(){break;}output.extend(batch);}
        assert_eq!(output,(0..200).map(candidate).collect::<Vec<_>>());
        assert!(terminal.load(Ordering::Acquire));assert_eq!(budget.reserved(),0);
    }
    #[tokio::test]
    async fn acknowledged_conflict_drain_keeps_typed_primary_and_releases_owner() {
        let budget=ResourceBudget::fixed(8<<20).unwrap();let terminal=Arc::new(AtomicBool::new(false));
        let mut sorter=AsyncCandidateSort::new_with_owner(&budget,Guard(terminal.clone())).await.unwrap();
        sorter.push(candidate(1)).await.unwrap();let mut conflict=candidate(1);conflict.node=RecordId::new("entity","other");sorter.push(conflict).await.unwrap();
        // Finish is queued without consuming its failed response, as with a cancelled await.
        sorter.0.submit(Input::Finish,0).unwrap();let error=sorter.drain().await.unwrap_err();
        assert!(matches!(error.primary(),Some(ModelError::Conflict("conflicting nominal candidate pointer"))));
        assert!(terminal.load(Ordering::Acquire));assert_eq!(budget.reserved(),0);
    }
    #[tokio::test]
    async fn dropped_sorter_releases_guard_only_after_queued_work_exits() {
        let budget=ResourceBudget::fixed(8<<20).unwrap();let terminal=Arc::new(AtomicBool::new(false));
        let mut sorter=AsyncCandidateSort::new_with_owner(&budget,Guard(terminal.clone())).await.unwrap();
        sorter.0.submit(Input::Candidate(candidate(3)),1024).unwrap();
        let acknowledgement=sorter.0.terminal.clone();drop(sorter);
        acknowledgement.await.unwrap();assert!(terminal.load(Ordering::Acquire));assert_eq!(budget.reserved(),0);
    }
    #[tokio::test]
    async fn interrupted_finish_keeps_scratch_and_can_resume_ordered_output() {
        let budget=ResourceBudget::fixed(8<<20).unwrap();let terminal=Arc::new(AtomicBool::new(false));
        let mut sorter=AsyncCandidateSort::new_with_owner(&budget,Guard(terminal.clone())).await.unwrap();
        sorter.push(candidate(9)).await.unwrap();sorter.push(candidate(2)).await.unwrap();
        let (entered_tx,entered)=oneshot::channel();let (release,gate)=std::sync::mpsc::channel();let (reply,_)=oneshot::channel();
        sorter.0.sender.as_ref().unwrap().try_send(Command{input:Input::Pause(entered_tx,gate),reply,_transfer:None}).unwrap_or_else(|_|panic!("pause admission"));entered.await.unwrap();
        let mut finish=Box::pin(sorter.finish());assert!(futures::poll!(&mut finish).is_pending());drop(finish);
        assert!(!terminal.load(Ordering::Acquire));assert!(budget.reserved()>0);release.send(()).unwrap();
        let mut ordered=sorter.finish().await.unwrap();assert_eq!(ordered.next_batch(128).await.unwrap(),vec![candidate(2),candidate(9)]);
        assert!(ordered.next_batch(128).await.unwrap().is_empty());assert!(terminal.load(Ordering::Acquire));assert_eq!(budget.reserved(),0);
    }
    #[test]
    fn abandoned_initialization_reports_late_typed_failure_to_registered_owner() {
        let runtime=tokio::runtime::Builder::new_current_thread().enable_all().max_blocking_threads(1).build().unwrap();
        runtime.block_on(async {
            let (entered_tx,entered)=oneshot::channel();let (release,gate)=std::sync::mpsc::channel();
            let blocker=tokio::task::spawn_blocking(move ||{entered_tx.send(()).unwrap();gate.recv().unwrap();});entered.await.unwrap();
            let budget=ResourceBudget::fixed(1024).unwrap();let terminal=Arc::new(AtomicBool::new(false));let (registered_tx,registered)=oneshot::channel();
            let mut constructor=Box::pin(AsyncCandidateSort::new_registered(&budget,Guard(terminal.clone()),move |terminal|{let _=registered_tx.send(terminal);}));
            assert!(futures::poll!(&mut constructor).is_pending());drop(constructor);
            let terminal_result=registered.await.unwrap();assert!(!terminal.load(Ordering::Acquire));release.send(()).unwrap();
            let error=terminal_result.await.unwrap_err();assert!(matches!(error.as_ref(),ModelError::Resource{owner:"ordered-command-owner",..}));
            assert!(terminal.load(Ordering::Acquire));assert_eq!(budget.reserved(),0);blocker.await.unwrap();
        });
    }
}
