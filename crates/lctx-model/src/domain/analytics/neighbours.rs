//! Bounded exact window similarity over frozen nominal E1 consumption. Scores remain heuristic.
//! The caller selects the query/target universe; this kernel does not infer public API scope.
use crate::domain::{*,analysis::analytic_embedding::{AnalysisInvocation,AnalysisOutcome},embedding::{analytic::{ConsumptionData,AnalysisEmbeddingUse,VectorAvailability},text::{TextSubject,TextWindow,TextAvailability},consumption::Winners,value::DecodedValue},normalized::Rows,resources::{ResourceBudget,Reservation}};

#[derive(Debug,thiserror::Error)]
pub enum Error {
    #[error(transparent)] Model(#[from] ModelError),
    #[error("nearest-neighbour work requires {required} window pairs; limit is {limit}")]
    WorkLimit {required:u64,limit:u64},
}
fn invalid(message:&str)->Error {ModelError::Invalid(message.into()).into()}
#[derive(Debug,Clone,Copy,PartialEq,Eq,PartialOrd,Ord)]
pub struct ItemKey {pub invocation:Id<AnalysisInvocation>,pub subject:Id<TextSubject>}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum Availability {Complete,Partial,Unavailable}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct Item {
    pub key:ItemKey,
    pub context:Id<attribution::AnalysisContext>,
    pub availability:Availability,
    pub available_windows:usize,
    pub expected_windows:usize,
}
struct Window {item:ItemKey,window:Id<TextWindow>,use_:Id<AnalysisEmbeddingUse>,vector:DecodedValue}
/// Admission replays the stored E1 owner first. Callers cannot construct a mismatched-width matrix.
pub struct Prepared {
    items:Vec<Item>,
    windows:Vec<Window>,
    budget:ResourceBudget,
    _reservation:Box<dyn Reservation>,
}
impl Prepared {
    pub fn admit(data:&ConsumptionData,invocations:&Rows<AnalysisInvocation>,outcomes:&Rows<AnalysisOutcome>,uses:&Rows<AnalysisEmbeddingUse>,budget:&ResourceBudget)->Result<Self,Error> {
        data.validate(invocations,outcomes,uses,budget)?;
        if !data.selected()? {return Err(invalid("analytic vectors were not requested"));}
        let item_cap=data.assessments.len();
        let bytes=item_cap.checked_mul(size_of::<Item>()).and_then(|n|n.checked_add(uses.len().checked_mul(size_of::<Window>())?)).and_then(|n|n.checked_add(size_of::<Self>())).ok_or_else(||invalid("analytic vector index overflow"))?;
        let reservation=budget.reserve("native-neighbour-inputs",bytes)?;
        let mut items=Vec::with_capacity(item_cap);let mut windows=Vec::with_capacity(uses.len());
        let selected=data.specification()?;let spec=selected.configuration()?;let mut winners=Winners::new(budget);
        for invocation in invocations.iter() {
            for assessment in data.assessments.iter().filter(|a|a.input==invocation.input && a.context==invocation.context) {
                let key=ItemKey {invocation:invocation.id(),subject:assessment.subject};
                let mut expected=0;let mut available=0;
                for window in data.windows.iter().filter(|w|w.assessment==assessment.id()) {
                    expected+=1;
                    let row=uses.iter().find(|u|u.invocation==invocation.id() && u.window==window.id()).ok_or_else(||invalid("admitted analytic window use disappeared"))?;
                    if row.availability==VectorAvailability::Available {
                        let vector=winners.replay(&spec,window.text.as_str(),row.receipt()?)?;
                        windows.push(Window {item:key,window:window.id(),use_:row.id(),vector});available+=1;
                    }
                }
                let availability=if assessment.availability==TextAvailability::Unavailable || available==0 {Availability::Unavailable}else if available==expected {Availability::Complete}else {Availability::Partial};
                items.push(Item {key,context:assessment.context,availability,available_windows:available,expected_windows:expected});
            }
        }
        items.sort_unstable_by_key(|i|i.key);windows.sort_unstable_by_key(|w|(w.item,w.window,w.use_));
        if items.windows(2).any(|pair|pair[0].key==pair[1].key) {return Err(invalid("duplicate analytic text subject in one invocation"));}
        Ok(Self {items,windows,budget:budget.clone(),_reservation:reservation})
    }
    pub fn items(&self)->&[Item] {&self.items}
    fn item(&self,key:ItemKey)->Result<&Item,Error> {self.items.binary_search_by_key(&key,|i|i.key).map(|n|&self.items[n]).map_err(|_|invalid("selected analytic subject is absent"))}
    fn windows(&self,key:ItemKey)->&[Window] {
        let start=self.windows.partition_point(|w|w.item<key);let end=self.windows.partition_point(|w|w.item<=key);&self.windows[start..end]
    }
    /// Exact best-window cosine, top-k per query, canonical target/window tie-breaking. Self
    /// exclusion is explicit for API layers. Missing windows remain visible in the receipt.
    pub fn nearest(&self,queries:&[ItemKey],targets:&[ItemKey],parameters:Parameters)->Result<Matches,Error> {
        if parameters.k==0 || parameters.min_similarity.get() < -1.0 || parameters.min_similarity.get()>1.0 {return Err(invalid("invalid nearest-neighbour selection parameters"));}
        let count=queries.len().checked_add(targets.len()).ok_or_else(||invalid("neighbour selector overflow"))?;
        let _selectors=self.budget.reserve("native-neighbour-selectors",count.checked_mul(size_of::<ItemKey>()).ok_or_else(||invalid("neighbour selector allocation overflow"))?)?;
        let mut queries=queries.to_vec();let mut targets=targets.to_vec();queries.sort_unstable();targets.sort_unstable();
        if queries.windows(2).any(|p|p[0]==p[1]) || targets.windows(2).any(|p|p[0]==p[1]) {return Err(invalid("duplicate nearest-neighbour selector"));}
        for key in queries.iter().chain(&targets) {self.item(*key)?;}
        let mut pairs=0u64;let mut incomplete=false;
        for query in &queries {
            let q=self.item(*query)?;incomplete|=q.availability!=Availability::Complete;
            for target in &targets {
                let t=self.item(*target)?;
                if q.context!=t.context {return Err(invalid("nearest-neighbour selections have foreign analysis contexts"));}
                if parameters.exclude_self && query==target {continue;}
                incomplete|=t.availability!=Availability::Complete;
                let work=u64::try_from(q.available_windows).ok().and_then(|q|u64::try_from(t.available_windows).ok().and_then(|t|q.checked_mul(t))).ok_or_else(||invalid("neighbour pair work overflow"))?;
                pairs=pairs.checked_add(work).ok_or_else(||invalid("neighbour pair work overflow"))?;
            }
        }
        if pairs>parameters.max_window_pairs {return Err(Error::WorkLimit {required:pairs,limit:parameters.max_window_pairs});}
        let capacity=queries.len().checked_mul(parameters.k.min(targets.len())).ok_or_else(||invalid("neighbour result count overflow"))?;
        let reservation=self.budget.reserve("native-neighbour-results",capacity.checked_mul(size_of::<Match>()).and_then(|n|n.checked_add(size_of::<Matches>())).ok_or_else(||invalid("neighbour result allocation overflow"))?)?;
        let _candidates=self.budget.reserve("native-neighbour-candidates",targets.len().checked_mul(size_of::<Match>()).ok_or_else(||invalid("neighbour candidate allocation overflow"))?)?;
        let mut rows=Vec::with_capacity(capacity);let mut candidates=Vec::with_capacity(targets.len());
        for query in &queries {
            candidates.clear();
            for target in &targets {
                if parameters.exclude_self && query==target {continue;}
                let mut best:Option<Match>=None;
                for left in self.windows(*query) {for right in self.windows(*target) {
                    let a=left.vector.values();let b=right.vector.values();
                    if a.len()!=b.len() {return Err(invalid("admitted analytic vectors have different dimensions"));}
                    let score=FiniteF64::new(a.iter().zip(b).map(|(x,y)|f64::from(*x)*f64::from(*y)).sum())?;
                    let candidate=Match {query:*query,target:*target,query_use:left.use_,target_use:right.use_,query_window:left.window,target_window:right.window,score};
                    if best.as_ref().is_none_or(|prior|score.get()>prior.score.get() || (score==prior.score && (candidate.query_window,candidate.target_window,candidate.query_use,candidate.target_use)<(prior.query_window,prior.target_window,prior.query_use,prior.target_use))) {best=Some(candidate);}
                }}
                if let Some(best)=best.filter(|m|m.score.get()>=parameters.min_similarity.get()) {candidates.push(best);}
            }
            candidates.sort_unstable_by(|a,b|b.score.get().total_cmp(&a.score.get()).then(a.target.cmp(&b.target)));
            rows.extend(candidates.iter().take(parameters.k).copied());
        }
        Ok(Matches {rows,window_pairs:pairs,incomplete,_reservation:reservation})
    }
}
#[derive(Debug,Clone,Copy)]
pub struct Parameters {pub k:usize,pub min_similarity:FiniteF64,pub max_window_pairs:u64,pub exclude_self:bool}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct Match {pub query:ItemKey,pub target:ItemKey,pub query_use:Id<AnalysisEmbeddingUse>,pub target_use:Id<AnalysisEmbeddingUse>,pub query_window:Id<TextWindow>,pub target_window:Id<TextWindow>,pub score:FiniteF64}
pub struct Matches {rows:Vec<Match>,window_pairs:u64,incomplete:bool,_reservation:Box<dyn Reservation>}
impl Matches {
    pub fn rows(&self)->&[Match] {&self.rows}
    pub fn window_pairs(&self)->u64 {self.window_pairs}
    pub fn incomplete(&self)->bool {self.incomplete}
}
/// One normalized centroid of every original admitted member window. The exact source use
/// inventory is retained on the result; callers cannot provide an unverified numeric vector.
pub struct CentroidMatch{pub target:ItemKey,pub target_use:Id<AnalysisEmbeddingUse>,pub score:FiniteF64,pub members:Vec<Id<AnalysisEmbeddingUse>>,_reservation:Box<dyn Reservation>}
impl Prepared{
 pub fn centroid(&self,queries:&[ItemKey],targets:&[ItemKey],floor:FiniteF64,max_work:u64)->Result<Option<CentroidMatch>,Error>{
 if !(-1.0..=1.0).contains(&floor.get()){return Err(invalid("centroid floor outside cosine domain"));}
  let _selectors=self.budget.reserve("centroid-selectors",queries.len().checked_add(targets.len()).and_then(|n|n.checked_mul(size_of::<ItemKey>())).ok_or_else(||invalid("centroid selectors overflow"))?)?;let mut queries=queries.to_vec();let mut targets=targets.to_vec();queries.sort_unstable();targets.sort_unstable();if queries.windows(2).any(|p|p[0]==p[1])||targets.windows(2).any(|p|p[0]==p[1]){return Err(invalid("duplicate centroid selector"));}let mut context=None;let mut count=0usize;let mut dimension=None;
  for key in queries.iter().chain(&targets){let item=self.item(*key)?;if context.replace(item.context).is_some_and(|c|c!=item.context){return Err(invalid("centroid crosses context"));}for w in self.windows(*key){if dimension.replace(w.vector.values().len()).is_some_and(|n|n!=w.vector.values().len()){return Err(invalid("centroid dimensions differ"));}}}
  for key in &queries{count=count.checked_add(self.item(*key)?.available_windows).ok_or_else(||invalid("centroid member overflow"))?;}
  let target_count=targets.iter().try_fold(0usize,|n,key|Ok::<_,Error>(n.checked_add(self.item(*key)?.available_windows).ok_or_else(||invalid("centroid target overflow"))?))?;
  let width=dimension.unwrap_or(0);let work=(count as u64).checked_add(target_count as u64).and_then(|n|n.checked_mul(width as u64)).ok_or_else(||invalid("centroid work overflow"))?;if work>max_work{return Err(Error::WorkLimit{required:work,limit:max_work});}
  let reservation=self.budget.reserve("native-neighbour-centroid",width.checked_mul(4*size_of::<f64>()).and_then(|n|n.checked_add(count.checked_mul(2*size_of::<Id<AnalysisEmbeddingUse>>())?)).and_then(|n|n.checked_add(4096)).ok_or_else(||invalid("centroid allocation overflow"))?)?;
  if count==0||target_count==0{return Ok(None);}let mut center=vec![0.0f64;width];let mut members=Vec::with_capacity(count);for key in &queries{for w in self.windows(*key){for (sum,value) in center.iter_mut().zip(w.vector.values()){*sum+=f64::from(*value);}members.push(w.use_);}}members.sort_unstable();members.dedup();let norm=center.iter().map(|x|x*x).sum::<f64>().sqrt();if !norm.is_finite()||norm==0.0{return Ok(None);}for x in &mut center{*x/=norm;}
  let mut best:Option<(ItemKey,Id<AnalysisEmbeddingUse>,Id<TextWindow>,FiniteF64)>=None;for target in &targets{for w in self.windows(*target){let score=FiniteF64::new(center.iter().zip(w.vector.values()).map(|(a,c)|*a*f64::from(*c)).sum())?;if score.get()<floor.get(){continue;}if best.is_none_or(|(t,u,window,s)|score.get()>s.get()||(score==s&&(*target,w.window,w.use_)<(t,window,u))){best=Some((*target,w.use_,w.window,score));}}}
  Ok(best.map(|(target,target_use,_,score)|CentroidMatch{target,target_use,score,members,_reservation:reservation}))
 }
}
