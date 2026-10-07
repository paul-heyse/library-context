//! Session-owned immutable rankings. Continuations replay retained rows without discovery.
use lctx_model::domain::{ContentHash, KeySink, serving::*};
use serde::Serialize;
use std::io::{self, Write};
use std::{collections::BTreeMap, sync::{Mutex, atomic::{AtomicU64, Ordering}}, time::{Duration, Instant, SystemTime}};

const MAX_ENTRIES: usize = RankedContinuationPolicy::MAXIMUM_ENTRIES as usize;
const MAX_BYTES: usize = RankedContinuationPolicy::MAXIMUM_RETAINED_BYTES as usize;
const LIFETIME: Duration = Duration::from_secs(RankedContinuationPolicy::EXPIRES_AFTER_SECONDS);
static SESSION: AtomicU64 = AtomicU64::new(0);

pub(crate) struct RankedResults { session:ContentHash, state:Mutex<State>, lifetime:Duration }
#[derive(Default)]
struct State { entries:BTreeMap<ContentHash,Entry>, next:u64, bytes:usize }
struct Entry { binding:CursorBinding, digest:ContentHash, channels:ChannelState, rows:Vec<Vec<u8>>, template:Option<Vec<u8>>, created:Instant, used:u64, bytes:usize }
#[derive(Serialize,serde::Deserialize)]
struct Row<T> { ranking:ranking::RankedHit, key:ContentHash, item:T }
struct Limited { bytes: Vec<u8>, cap: usize }
impl Write for Limited {
    fn write(&mut self, value: &[u8]) -> io::Result<usize> {
        if value.len()>self.cap.saturating_sub(self.bytes.len()) {return Err(io::Error::other("ranked result retention bytes"));}
        self.bytes.extend_from_slice(value); Ok(value.len())
    }
    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}
fn encode<T:Serialize>(value:&T,cap:usize)->Result<Vec<u8>,WireError> {
    let mut writer=Limited {bytes:Vec::new(),cap};
    serde_json::to_writer(&mut writer,value).map_err(|_|WireError::ResourceRefused("ranked result retention bytes".into()))?;
    Ok(writer.bytes)
}
fn unavailable()->WireError {WireError::Continuation("ranked continuation unavailable in this session".into())}
impl Default for RankedResults {fn default()->Self {
    let mut key=KeySink::new("native-ranked-session/v1");
    key.part(b"process",&std::process::id().to_le_bytes());
    key.part(b"time",&SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_nanos().to_le_bytes());
    key.part(b"sequence",&SESSION.fetch_add(1,Ordering::Relaxed).to_le_bytes());
    Self {session:key.finish(),state:Mutex::new(State::default()),lifetime:LIFETIME}
}}
impl State {
    fn remove(&mut self,id:ContentHash) {if let Some(entry)=self.entries.remove(&id) {self.bytes-=entry.bytes;}}
    fn expire(&mut self,lifetime:Duration) {let old=self.entries.iter().filter(|(_,e)|e.created.elapsed()>=lifetime).map(|(id,_)|*id).collect::<Vec<_>>();for id in old {self.remove(id);}}
    fn make_room(&mut self,bytes:usize,protected:Option<ContentHash>)->Result<(),WireError> {
        if bytes>MAX_BYTES {return Err(WireError::ResourceRefused("ranked result retention bytes".into()));}
        while self.bytes+bytes>MAX_BYTES || (protected.is_none() && self.entries.len()>=MAX_ENTRIES) {
            let id=self.entries.iter().filter(|(id,_)|Some(**id)!=protected).min_by_key(|(_,e)|e.used).map(|(id,_)|*id).ok_or_else(||WireError::ResourceRefused("ranked result retention bytes".into()))?;
            self.remove(id);
        }
        Ok(())
    }
}
impl RankedResults {
    /// Only the first page performs discovery. All rows are serialized once for bounded retention.
    pub(crate) fn page<T:Serialize>(
        &self,values:Vec<(ranking::RankedHit,ContentHash,T)>,request:&Request,snapshot:&SnapshotHandle,channels:&ChannelState,
    )->Result<(SectionPage<T>,Vec<ranking::RankedHit>),WireError> {
        if request.page().cursor.0.is_some() {return Err(unavailable());}
        let expected=crate::pagination::binding(request,snapshot,channels,request.tool().name(),"results",None)?;
        let count=values.len();let size=request.page().size as usize;
        let continuation=if count>size {
            let mut rows=Vec::with_capacity(count);let mut bytes=size_of::<Entry>()+count*size_of::<Vec<u8>>();
            for (ranking,key,item) in &values {
                let encoded=encode(&Row {ranking:ranking.clone(),key:*key,item},MAX_BYTES.saturating_sub(bytes))?;
                bytes=bytes.checked_add(encoded.len()).ok_or_else(||WireError::ResourceRefused("ranked result size overflow".into()))?;
                if bytes>MAX_BYTES {return Err(WireError::ResourceRefused("ranked result retention bytes".into()));}
                rows.push(encoded);
            }
            let mut key=KeySink::new("native-ranked-result/v1");
            for row in &rows {key.part(b"row",row);}
            let digest=key.finish();
            let mut state=self.state.lock().map_err(|_|unavailable())?;
            state.expire(self.lifetime);state.make_room(bytes,None)?;
            state.next+=1;let used=state.next;
            let mut key=KeySink::new("native-ranked-entry/v1");key.part(b"session",&self.session.0);key.part(b"sequence",&used.to_le_bytes());key.part(b"digest",&digest.0);let result=key.finish();
            state.entries.insert(result,Entry {binding:expected.clone(),digest,channels:channels.clone(),rows,template:None,created:Instant::now(),used,bytes});state.bytes+=bytes;
            Optional(Some(Cursor {binding:expected,after:CursorPosition::Ranked {session:self.session,result,digest,offset:size as u64}}.encode()?))
        } else {Optional::default()};
        let (ranking,items)=values.into_iter().take(size).map(|(ranking,_,item)|(ranking,item)).unzip();
        let omitted=count.saturating_sub(size) as u64;
        Ok((SectionPage {availability:Availability::Available {},items,continuation,omitted,truncated:omitted>0},ranking))
    }
    /// Bind the final response's metadata to its retained order; page payloads are not duplicated.
    pub(crate) fn attach(&self,response:&Response)->Result<(),WireError> {
        let mut template:serde_json::Value=serde_json::from_str(&response.to_json()?)?;
        let Some(token)=template.pointer("/results/continuation").and_then(|v|v.as_str()) else{return Ok(());};
        let token=token.to_owned();let cursor:Cursor=serde_json::from_slice(&hex::decode(token).map_err(|_|unavailable())?)?;
        let CursorPosition::Ranked {session,result,digest,..}=cursor.after else{return Ok(());};
        if session!=self.session {return Err(unavailable());}
        template["results"]["items"]=serde_json::json!([]);template["ranking"]=serde_json::json!([]);
        let encoded=encode(&template,MAX_BYTES)?;
        let mut state=self.state.lock().map_err(|_|unavailable())?;
        let entry=state.entries.get(&result).ok_or_else(unavailable)?;
        if entry.digest!=digest || entry.template.is_some() {return Err(unavailable());}
        state.make_room(encoded.len(),Some(result))?;
        let entry=state.entries.get_mut(&result).ok_or_else(unavailable)?;entry.bytes+=encoded.len();entry.template=Some(encoded);let added=entry.template.as_ref().expect("inserted template").len();state.bytes+=added;
        Ok(())
    }
    /// A ranked cursor never re-enters discovery, embedding or channel fallback.
    pub(crate) fn resume(&self,request:&Request,snapshot:&SnapshotHandle,vector:Option<&crate::service::QueryVector>)->Result<Option<Response>,WireError> {
        let Some(token)=&request.page().cursor.0 else{return Ok(None);};
        let cursor:Cursor=serde_json::from_slice(&hex::decode(token.as_str()).map_err(|_|unavailable())?)?;
        let CursorPosition::Ranked {session,result,digest,offset}=cursor.after else {
            if matches!(request,Request::SearchOperations(_) | Request::SearchEvidence(_) | Request::SearchCapabilities(_)) { return Err(unavailable()); }
            return Ok(None);
        };
        if session!=self.session {return Err(unavailable());}
        let mut state=self.state.lock().map_err(|_|unavailable())?;state.expire(self.lifetime);
        state.next+=1;let used=state.next;
        let entry=state.entries.get_mut(&result).ok_or_else(unavailable)?;
        if let Some(v)=vector {
            let query=match request {
                Request::SearchOperations(r)=>r.query.as_str(),
                Request::SearchEvidence(r)=>r.query.as_str(),
                Request::SearchCapabilities(r)=>r.query.as_str(),
                _=>return Err(unavailable()),
            };
            if v.recipe.validate().is_err() || v.input!=lctx_model::domain::embedding::value::input_hash(&v.recipe.text(query)) {return Err(unavailable());}
            let supplied=VectorChannel::Available {spec:v.spec,query_vector:lctx_model::domain::embedding::value::value_digest(&v.vector),query_recipe:v.recipe.identity(),projection:v.projection};
            if supplied!=entry.channels.vector { return Err(unavailable()); }
        }
        let expected=crate::pagination::binding(request,snapshot,&entry.channels,request.tool().name(),"results",None)?;
        Cursor::decode(token,&expected)?;
        if entry.binding!=expected || entry.digest!=digest {return Err(unavailable());}
        let start=usize::try_from(offset).map_err(|_|unavailable())?;
        if start>=entry.rows.len() {return Err(unavailable());}
        let end=start.saturating_add(request.page().size as usize).min(entry.rows.len());
        let mut template:serde_json::Value=serde_json::from_slice(entry.template.as_ref().ok_or_else(unavailable)?)?;
        let rows=entry.rows[start..end].iter().map(|row|serde_json::from_slice::<Row<serde_json::Value>>(row)).collect::<Result<Vec<_>,_>>()?;
        template["results"]["items"]=serde_json::to_value(rows.iter().map(|r|&r.item).collect::<Vec<_>>())?;
        template["ranking"]=serde_json::to_value(rows.iter().map(|r|&r.ranking).collect::<Vec<_>>())?;
        template["results"]["omitted"]=serde_json::json!(entry.rows.len()-end);template["results"]["truncated"]=serde_json::json!(end<entry.rows.len());
        if end<entry.rows.len() {template["results"]["continuation"]=serde_json::json!(Cursor {binding:expected,after:CursorPosition::Ranked {session,result,digest,offset:end as u64}}.encode()?.as_str());}
        else {template["results"].as_object_mut().ok_or_else(unavailable)?.remove("continuation");}
        template["extent"]=serde_json::json!({"extent":"ranked","returned":rows.len()});
        entry.used=used;
        let raw=serde_json::to_string(&template)?;
        decode_response(request.tool().name(),&raw,request.page().expanded,&ResourceLimits::default()).map(Some)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use lctx_model::domain::{Id, retrieval, attribution::AnalysisContext};
    fn id<T>(n:u8)->Id<T> { serde_json::from_value(serde_json::json!(vec![n;16])).unwrap() }
    fn handle()->SnapshotHandle { SnapshotHandle { semantic:ContentHash::of(b"semantic"),realization:ContentHash::of(b"physical"),database:DatabaseIdentity {namespace:Name::new("control").unwrap(),database:Name::new("snapshot").unwrap()} } }
    fn request()->Request { Request::SearchEvidence(SearchEvidenceRequest { library:Optional::default(),query:QueryText::new("original").unwrap(),families:vec![],page:PageRequest {size:1,..PageRequest::default()} }) }
    fn channels()->ChannelState { ChannelState {lexical:true,vector:VectorChannel::Degraded {reason:Name::new("unavailable").unwrap()}} }
    fn response(cache:&RankedResults,request:&Request)->Response {
        response_with_channels(cache,request,&channels())
    }
    fn response_with_channels(cache:&RankedResults,request:&Request,channels:&ChannelState)->Response {
        let values=(1..=3).map(|n| {let unit=id::<retrieval::Unit>(n);let ranking=ranking::RankedHit {target:ranking::Target::Unit {unit},context:id::<AnalysisContext>(n),score:1.0/f64::from(n),promoted:false,witnesses:vec![]};
            (ranking,ContentHash::of(&[n]),EvidenceHit {unit,family:retrieval::Family::Source,title:Name::new(format!("original{n}")).unwrap(),originals:vec![],associated_members:vec![]})}).collect();
        let (results,ranking)=cache.page(values,request,&handle(),channels).unwrap();
        let response=Response::SearchEvidence(SearchEvidenceResponse {
            delivery: Optional::default(),snapshot:handle(),domains:vec![],extent:SelectionExtent::Ranked {returned:1},results,channels:channels.clone(),ranking});cache.attach(&response).unwrap();response
    }
    fn next(request:&mut Request,response:&Response) {
        let Request::SearchEvidence(request)=request else {unreachable!()};let Response::SearchEvidence(response)=response else {unreachable!()};request.page.cursor=response.results.continuation.clone();
    }
    #[test]
    fn continuation_replays_frozen_rows_and_channels_and_checks_context() {
        let cache=RankedResults::default();let mut request=request();let first=response(&cache,&request);next(&mut request,&first);
        let second=cache.resume(&request,&handle(),None).unwrap().unwrap();let Response::SearchEvidence(page)=&second else {unreachable!()};
        assert_eq!(page.results.items[0].title.as_str(),"original2");assert_eq!(page.channels,channels());assert_eq!(page.ranking[0].context,id(2));
        next(&mut request,&second);let third=cache.resume(&request,&handle(),None).unwrap().unwrap();let Response::SearchEvidence(page)=&third else {unreachable!()};assert_eq!(page.results.items[0].title.as_str(),"original3");assert!(page.results.continuation.0.is_none());
        let mut foreign=handle();foreign.semantic=ContentHash::of(b"changed");assert!(cache.resume(&request,&foreign,None).is_err());
        let Request::SearchEvidence(changed)=&mut request else {unreachable!()};changed.query=QueryText::new("other").unwrap();assert!(cache.resume(&request,&handle(),None).is_err());
    }
    #[test]
    fn expiry_foreign_session_and_eviction_refuse_without_rediscovery() {
        let cache=RankedResults::default();let mut request=request();let first=response(&cache,&request);next(&mut request,&first);
        assert!(RankedResults::default().resume(&request,&handle(),None).is_err());
        for _ in 0..MAX_ENTRIES {response(&cache,&self::request());}
        assert!(cache.resume(&request,&handle(),None).is_err());
        let expired=RankedResults {lifetime:Duration::ZERO,..RankedResults::default()};let fresh=response(&expired,&self::request());let mut continuation=self::request();next(&mut continuation,&fresh);assert!(expired.resume(&continuation,&handle(),None).is_err());
    }
    #[test]
    fn retained_writer_refuses_before_oversized_row_allocation() {
        assert!(encode(&vec!["payload";100],16).is_err());
        let mut state=State::default();assert!(state.make_room(MAX_BYTES+1,None).is_err());
    }
    #[test]
    fn continuation_rejects_changed_query_value_or_input_but_needs_no_new_inference() {
        use lctx_model::domain::embedding;
        let recipe=embedding::QueryRecipe {template:"{task_description}: {query}".into(),task:"retrieve".into(),max_tokens:8192};
        let mut vector=vec![0.0;4096];vector[0]=1.0;
        let mut query=crate::service::QueryVector {spec:ContentHash::of(b"encoder"),input:embedding::value::input_hash(&recipe.text("original")),vector,recipe,projection:id(1)};
        let channels=ChannelState {lexical:true,vector:VectorChannel::Available {spec:query.spec,query_vector:embedding::value::value_digest(&query.vector),query_recipe:query.recipe.identity(),projection:query.projection}};
        let cache=RankedResults::default();let mut request=request();let first=response_with_channels(&cache,&request,&channels);next(&mut request,&first);
        assert!(cache.resume(&request,&handle(),None).unwrap().is_some());
        assert!(cache.resume(&request,&handle(),Some(&query)).unwrap().is_some());
        query.input=ContentHash::of(b"foreign input");assert!(cache.resume(&request,&handle(),Some(&query)).is_err());
        query.input=embedding::value::input_hash(&query.recipe.text("original"));query.vector[0]=0.0;query.vector[1]=1.0;
        assert!(cache.resume(&request,&handle(),Some(&query)).is_err());
    }
}
