//! Effect-free operation completion. Workflow owners perform and report each finalization.
use super::ModelError;

/// The original thread panic payload remains owned, including non-string payloads.
/// A mutex supplies Sync for the standard library's Send-only payload; no task is run here.
pub struct ThreadPanic {pub owner:&'static str,payload:std::sync::Mutex<Box<dyn std::any::Any+Send>>}
impl ThreadPanic {
    pub fn new(owner:&'static str,payload:Box<dyn std::any::Any+Send>)->Self {Self{owner,payload:std::sync::Mutex::new(payload)}}
}
impl std::fmt::Display for ThreadPanic {
    fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result {
        write!(f,"{} thread panicked",self.owner)?;
        if let Ok(payload)=self.payload.lock() {
            if let Some(message)=payload.downcast_ref::<String>() {write!(f,": {message}")?;}
            else if let Some(message)=payload.downcast_ref::<&'static str>() {write!(f,": {message}")?;}
        }
        Ok(())
    }
}
impl std::fmt::Debug for ThreadPanic {fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result {std::fmt::Display::fmt(self,f)}}
impl std::error::Error for ThreadPanic {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LocalState { #[default] Terminal, Outstanding }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RemoteState { #[default] Confirmed, Unknown }
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageState { Removed(String), Orphan(String) }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedEffect { pub kind: &'static str, pub identity: String }
#[derive(Debug)]
pub struct Finalization { pub step: &'static str, pub error: ModelError }
#[derive(Debug, Default)]
pub struct Completion {
    pub local: LocalState,
    pub remote: RemoteState,
    pub storage: Vec<StorageState>,
    pub committed: Vec<CommittedEffect>,
    pub failures: Vec<Finalization>,
}
impl Completion {
    pub fn step(&mut self, step: &'static str, result: Result<(),ModelError>) {
        if let Err(error)=result { self.observe(&error); self.failures.push(Finalization{step,error}); }
    }
    pub fn cleanup(&mut self, identity: impl Into<String>, result: Result<(),ModelError>) {
        let identity=identity.into();
        if result.is_ok() {self.storage.push(StorageState::Removed(identity));}
        else {self.storage.push(StorageState::Orphan(identity));}
        self.step("private storage cleanup",result);
    }
    pub fn committed(&mut self, kind: &'static str, identity: impl Into<String>) {
        self.committed.push(CommittedEffect{kind,identity:identity.into()});
    }
    fn observe(&mut self,error:&ModelError) {
        if matches!(error,ModelError::Infrastructure{class:super::Infrastructure::Unconfirmed,..}) {self.remote=RemoteState::Unknown;}
        if let ModelError::SharedCause(error)=error {self.observe(error);}
        if let ModelError::Completion(outcome)=error {
            if outcome.completion.local==LocalState::Outstanding {self.local=LocalState::Outstanding;}
            if outcome.completion.remote==RemoteState::Unknown {self.remote=RemoteState::Unknown;}
        }
    }
    fn satisfied(&self)->bool { self.failures.is_empty() && self.local==LocalState::Terminal && self.remote==RemoteState::Confirmed && !self.storage.iter().any(|s|matches!(s,StorageState::Orphan(_))) }
}
#[derive(Debug)]
pub struct OperationFailure { pub primary: Option<Box<ModelError>>, pub completion: Completion }
impl std::fmt::Display for OperationFailure {
    fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result {
        if let Some(primary)=&self.primary {write!(f,"{primary}")?;} else {write!(f,"operation finalization failed")?;}
        for failure in &self.completion.failures {write!(f,"; {}: {}",failure.step,failure.error)?;}
        write!(f,"; local {:?}, remote {:?}",self.completion.local,self.completion.remote)?;
        for storage in &self.completion.storage {write!(f,"; storage {storage:?}")?;}
        for effect in &self.completion.committed {write!(f,"; committed {} {}",effect.kind,effect.identity)?;}
        Ok(())
    }
}
impl std::error::Error for OperationFailure {
    fn source(&self)->Option<&(dyn std::error::Error+'static)> {self.primary.as_deref().map(|e|e as _).or_else(||self.completion.failures.first().map(|e|&e.error as _))}
}
pub fn complete<T>(result:Result<T,ModelError>,mut completion:Completion)->Result<T,ModelError> {
    match result {
        Ok(value) if completion.satisfied()=>Ok(value),
        Err(error) if completion.satisfied() && completion.storage.is_empty() && completion.committed.is_empty()=>Err(error),
        result=>{
            let primary=result.err().map(Box::new);
            if let Some(error)=&primary {completion.observe(error);}
            Err(ModelError::Completion(Box::new(OperationFailure{primary,completion})))
        }
    }
}
impl ModelError {
    pub fn primary(&self)->Option<&ModelError> {
        match self { Self::Completion(outcome)=>outcome.primary.as_deref().and_then(Self::primary),Self::SharedCause(error)=>error.primary(),other=>Some(other) }
    }
    pub fn permits_storage_cleanup(&self)->bool {
        match self {
            Self::Completion(outcome)=>outcome.completion.local==LocalState::Terminal && outcome.completion.remote==RemoteState::Confirmed && outcome.primary.as_deref().is_none_or(Self::permits_storage_cleanup) && outcome.completion.failures.iter().all(|f|f.error.permits_storage_cleanup()),
            Self::SharedCause(error)=>error.permits_storage_cleanup(),
            Self::Infrastructure{class:super::Infrastructure::Unconfirmed,..}=>false,
            _=>true,
        }
    }
    pub fn has_committed_effect(&self)->bool {
        match self {Self::Completion(outcome)=>!outcome.completion.committed.is_empty() || outcome.primary.as_deref().is_some_and(Self::has_committed_effect) || outcome.completion.failures.iter().any(|f|f.error.has_committed_effect()),Self::SharedCause(error)=>error.has_committed_effect(),_=>false}
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn primary_and_all_finalization_causes_survive() {
        let mut finality=Completion::default();
        finality.step("drain",Err(ModelError::Codec("completed failed join".into())));
        finality.cleanup("private-db",Err(ModelError::Schema("remove refusal")));
        let error=complete::<()>(Err(ModelError::Limit{owner:"test",limit:"rows",observed:2,bound:1}),finality).unwrap_err();
        assert!(matches!(error.primary(),Some(ModelError::Limit{..})));
        let ModelError::Completion(outcome)=error else {panic!()};
        assert_eq!(outcome.completion.failures.len(),2);
        assert_eq!(outcome.completion.local,LocalState::Terminal);
        assert_eq!(outcome.completion.remote,RemoteState::Confirmed);
        assert_eq!(outcome.completion.storage,vec![StorageState::Orphan("private-db".into())]);
    }
    #[test]
    fn cleanup_only_and_unknown_acknowledgement_refuse_success() {
        let mut finality=Completion::default();finality.remote=RemoteState::Unknown;
        assert!(complete(Ok(17),finality).is_err());
        let mut finality=Completion::default();finality.step("session",Err(ModelError::Codec("invalidated".into())));
        let error=complete(Ok(17),finality).unwrap_err();assert!(error.primary().is_none());
        assert!(matches!(complete::<()>(Err(ModelError::Schema("primary")),Completion::default()),Err(ModelError::Schema("primary"))));
    }
    #[test]
    fn committed_effect_survives_later_failure() {
        let mut finality=Completion::default();finality.committed("sealed unselected database","owned-handle");
        finality.step("staging removal",Err(ModelError::Codec("cleanup".into())));
        let error=complete(Ok(17),finality).unwrap_err();assert!(error.has_committed_effect());assert!(error.to_string().contains("owned-handle"));
    }
}
