//! Read-only native operation session. The Python adapter never receives writer credentials.
use lctx_model::domain::serving::ResourceLimits;
use lctx_surrealdb::{NativeReader,config::ViewerConfig};
use lctx_serving::NativeService;
use pyo3::{prelude::*,exceptions::{PyValueError,PyRuntimeError}};
use std::sync::{Arc,Mutex,Condvar};
struct State {service:Option<Arc<NativeService>>,active:usize,closing:bool}
struct Shared {state:Mutex<State>,drained:Condvar}
#[pyclass(module="lctx_semantics",frozen)]
pub struct NativeSession {runtime:Arc<tokio::runtime::Runtime>,shared:Arc<Shared>,reader:NativeReader}
#[pymethods]
impl NativeSession {
    #[new]
    fn new(py:Python<'_>,serving_config:String)->PyResult<Self>{
        py.detach(move||{
            let config=ViewerConfig::read(std::path::Path::new(&serving_config)).map_err(|e|PyValueError::new_err(e.to_string()))?;
            let runtime=Arc::new(tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().map_err(|e|PyRuntimeError::new_err(e.to_string()))?);
            let reader=runtime.block_on(NativeReader::connect(&config.endpoint,&config.credentials(),config.snapshot)).map_err(|e|PyRuntimeError::new_err(e.to_string()))?;
            let service=Arc::new(NativeService::new(reader.clone(),ResourceLimits::default()).map_err(|e|PyRuntimeError::new_err(e.to_string()))?);
            Ok(Self{runtime,shared:Arc::new(Shared{state:Mutex::new(State{service:Some(service),active:0,closing:false}),drained:Condvar::new()}),reader})
        })
    }
    #[pyo3(signature=(tool,request_json,query_vector_json=None))]
    fn execute(&self,py:Python<'_>,tool:String,request_json:String,query_vector_json:Option<String>)->PyResult<String>{
        py.detach(||{
            let service={let mut state=self.shared.state.lock().map_err(|_|PyRuntimeError::new_err("native session state"))?;
                if state.closing{return Err(PyRuntimeError::new_err("native session closed"))}
                if state.active>=2{return Err(PyValueError::new_err("resource_refused: native request slots"))}
                let service=state.service.as_ref().ok_or_else(||PyRuntimeError::new_err("native session closed"))?.clone();state.active+=1;service};
            let _active=Active(self.shared.clone());
            if query_vector_json.as_deref()==Some("unavailable"){
                self.runtime.block_on(service.execute_unavailable(&tool,&request_json)).map_err(|e|PyValueError::new_err(e.to_string()))
            }else if let Some(vector)=query_vector_json{
                let vector=serde_json::from_str(&vector).map_err(|e|PyValueError::new_err(e.to_string()))?;
                self.runtime.block_on(service.execute_with_vector(&tool,&request_json,Some(vector))).map_err(|e|PyValueError::new_err(e.to_string()))
            }else{self.runtime.block_on(service.execute(&tool,&request_json)).map_err(|e|PyValueError::new_err(e.to_string()))}
        })
    }
    fn handle_json(&self)->PyResult<String>{serde_json::to_string(self.reader.handle()).map_err(|e|PyValueError::new_err(e.to_string()))}
    /// Drain admitted calls before invalidating the one server session.
    fn close(&self,py:Python<'_>)->PyResult<()>{py.detach(||{
        let mut state=self.shared.state.lock().map_err(|_|PyRuntimeError::new_err("native session state"))?;
        state.closing=true;
        while state.active>0{state=self.shared.drained.wait(state).map_err(|_|PyRuntimeError::new_err("native session drain"))?;}
        if state.service.take().is_some(){drop(state);self.runtime.block_on(self.reader.client().invalidate()).map_err(|e|PyRuntimeError::new_err(e.to_string()))?;}
        Ok(())
    })}
}
struct Active(Arc<Shared>);
impl Drop for Active{fn drop(&mut self){if let Ok(mut state)=self.0.state.lock(){state.active=state.active.saturating_sub(1);self.0.drained.notify_all();}}}
