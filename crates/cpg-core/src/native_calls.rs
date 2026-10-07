//! Submitted asynchronous native calls retain task ownership through caller cancellation.
use crate::workspace::Cancellation;
use lctx_model::domain::{ModelError,charged::StateCharge,resources::ResourceBudget};
use lctx_surrealdb::compiler::NativeCompilerStore;
use std::sync::{Arc,Mutex,atomic::{AtomicBool,Ordering}};
use futures::{FutureExt,future::{BoxFuture,Shared}};
use tokio::sync::{Semaphore,oneshot};

pub(crate) struct NativeCalls {
    native:Arc<NativeCompilerStore>,
    cancellation:Cancellation,
    limit:Arc<Semaphore>,
    tasks:Mutex<(Vec<NativeCallTask>,StateCharge)>,
}
struct NativeCallTask {joined:Shared<BoxFuture<'static,Result<(),Arc<tokio::task::JoinError>>>>,finished:Arc<AtomicBool>}
struct CallGuard {native:Arc<NativeCompilerStore>,cancellation:Cancellation,finished:bool,done:Option<Arc<AtomicBool>>}
impl Drop for CallGuard {
    fn drop(&mut self){if !self.finished{self.native.fail();self.cancellation.cancel();}
        if let Some(done)=&self.done{done.store(true,Ordering::Release);}}
}
impl NativeCalls {
    pub(crate) fn new(native:Arc<NativeCompilerStore>,cancellation:Cancellation,budget:&ResourceBudget)->Self{
        Self{native,cancellation,limit:Arc::new(Semaphore::new(8)),tasks:Mutex::new((Vec::new(),StateCharge::new(budget,"native-call-ownership")))}
    }
    pub(crate) async fn call<T:Send+'static>(&self,future:impl Future<Output=Result<T,ModelError>>+Send+'static)->Result<T,ModelError>{
        self.cancellation.check()?;
        let mut waiting=CallGuard{native:self.native.clone(),cancellation:self.cancellation.clone(),finished:false,done:None};
        let permit=tokio::select!{
            ()=self.cancellation.cancelled()=>return Err(ModelError::Invalid("compilation cancelled".into())),
            permit=self.limit.clone().acquire_owned()=>permit.map_err(ModelError::codec)?,
        };
        let (answer,acknowledgement)=oneshot::channel();
        {
            // Serialize registration with drainage. No call is submitted after cancellation.
            let mut tasks=self.tasks.lock().map_err(|_|ModelError::Conflict("native call ownership"))?;
            self.cancellation.check()?;
            tasks.0.retain(|task|!task.finished.load(Ordering::Acquire));
            if tasks.0.len()==tasks.0.capacity(){tasks.1.grow(256)?;tasks.0.reserve_exact(1);}
            let native=self.native.clone();let cancellation=self.cancellation.clone();
            let finished=Arc::new(AtomicBool::new(false));let done=finished.clone();
            let task=tokio::spawn(async move{
                let mut submitted=CallGuard{native,cancellation,finished:false,done:Some(done)};
                let result=future.await;
                if result.is_err(){submitted.native.fail();submitted.cancellation.cancel();}
                submitted.finished=true;
                let _=answer.send(result);
                drop(permit);
            });
            let joined=async move{task.await.map_err(Arc::new)}.boxed().shared();
            tasks.0.push(NativeCallTask{joined,finished});
        }
        let result=acknowledgement.await.map_err(|_|ModelError::Invalid("native call ended without acknowledgement".into()))?;
        waiting.finished=true;
        result
    }
    pub(crate) async fn drain(&self)->Result<(),ModelError>{
        self.cancellation.cancel();
        // Shared join ownership stays registered if drainage itself is cancelled and retried.
        let tasks=self.tasks.lock().map_err(|_|ModelError::Conflict("native call ownership"))?.0.iter().map(|task|task.joined.clone()).collect::<Vec<_>>();
        let mut error=None;
        for task in tasks{if let Err(failure)=task.await{error.get_or_insert_with(||ModelError::codec(failure));}}
        self.tasks.lock().map_err(|_|ModelError::Conflict("native call ownership"))?.0.retain(|task|!task.finished.load(Ordering::Acquire));
        error.map_or(Ok(()),Err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn cancelled_caller_stops_submissions_but_submitted_native_call_finishes(){
        let native=crate::test_native::store();let budget=ResourceBudget::fixed(1<<20).unwrap();
        let cancellation=Cancellation::default();let calls=Arc::new(NativeCalls::new(native.clone(),cancellation.clone(),&budget));
        let entered=Arc::new(tokio::sync::Notify::new());let completed=Arc::new(AtomicBool::new(false));
        let call_owner=calls.clone();let call_entered=entered.clone();let call_completed=completed.clone();
        let caller=tokio::spawn(async move{call_owner.call(async move{call_entered.notify_one();tokio::time::sleep(std::time::Duration::from_millis(80)).await;call_completed.store(true,Ordering::Release);Ok(17)}).await});
        entered.notified().await;caller.abort();assert!(caller.await.is_err());
        calls.drain().await.unwrap();assert!(completed.load(Ordering::Acquire));assert!(cancellation.check().is_err());assert!(native.check().is_err());
        assert!(calls.call(async{Ok(18)}).await.is_err());
    }
}
