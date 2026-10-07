//! Offline complete-input tokenization and semantic whole-part partitioning.
use super::*;
use build::{Data, Output, invalid, need};
use crate::domain::resources::ResourceBudget;


pub struct EncodedInput {
    pub text: String,
    pub body_start: usize,
    pub body_end: usize,
    pub offsets: Vec<(usize, usize)>,
    pub specials: Vec<bool>,
}
pub trait Tokenizer: Send + Sync {
    fn identity(&self) -> ContentHash;
    fn encode(&self, text: &str) -> Result<EncodedInput, ModelError>;
}
/// Supported boundaries are semantic parts. Only the selected primaries' interpretation
/// dependencies participate in admission. Oversized indivisible material is never truncated.
#[derive(Default)]
pub(super) struct UnitParts{pub parts:Vec<ContentPart>,pub maps:Vec<PartSourceMap>,pub dependencies:Vec<PartContext>}
pub(super) fn construct(d:&Data, out:&mut Output, unit:Id<Unit>, cohort:UnitParts, b:&ResourceBudget)->Result<(),ModelError>{
    let definition=d.selected()?;
    if definition.embedding_requested && d.tokenizer().is_none(){return Err(invalid("requested retrieval requires exact acquired local tokenizer assets"));}
    let mut parts=cohort.parts;
    parts.sort_by_key(|p|p.ordinal);
    let dependencies=context_index(cohort.dependencies.iter());
    let mut maps=std::collections::BTreeMap::<_,Vec<_>>::new();
    for map in cohort.maps{maps.entry(map.part).or_default().push(map);}
    for maps in maps.values_mut(){maps.sort_by_key(|m|m.ordinal);}
    let part_refs=parts.iter().collect::<Vec<_>>();
    let context:Vec<_>=parts.iter().filter(|p|p.purpose==PartPurpose::Context).cloned().collect();
    let primary:Vec<_>=parts.iter().filter(|p|p.purpose==PartPurpose::Primary).cloned().collect();
    let mut groups:Vec<Vec<ContentPart>>=vec![];
    let mut group:Vec<ContentPart>=vec![];
    for part in primary {
        if !group.is_empty() && required_context(&dependencies,&[part.id()].into_iter().collect())!=required_context(&dependencies,&group.iter().map(Record::id).collect()){
            // Sibling interpretation scopes remain independent even when both are short.
            groups.push(std::mem::take(&mut group));
        }
        let mut tentative=group.clone(); tentative.push(part.clone());
        let text=render(&selected_parts(&part_refs,&dependencies,unit,&tentative.iter().map(Record::id).collect())?);
        let tokens=d.tokenizer().map(|t|t.encode(&text).map(|e|e.offsets.len())).transpose()?;
        if !group.is_empty() && tokens.is_some_and(|n|n>definition.preferred_tokens as usize) {
            groups.push(std::mem::take(&mut group));
        }
        group.push(part);
    }
    if !group.is_empty(){groups.push(group);}
    // A setup-only grain remains discoverable but has no applicability binding.
    if groups.is_empty() && !context.is_empty(){groups.push(vec![]);}
    for (ordinal,primary) in groups.into_iter().enumerate(){
        let selected=selected_parts(&part_refs,&dependencies,unit,&primary.iter().map(Record::id).collect())?;
        let text=render(&selected);
        let _copy=b.reserve("retrieval-semantic-window",text.len().saturating_mul(4))?;
        let encoded=d.tokenizer().map(|t|t.encode(&text)).transpose()?;
        let (input_text,body_start,tokens,availability)=match &encoded {
            Some(e)=>(e.text.clone(),e.body_start,Some(e.offsets.len() as i64),if e.offsets.len()>definition.hard_tokens as usize{WindowAvailability::LexicalOnly}else{WindowAvailability::Ready}),
            None=>(text.clone(),0,None,WindowAvailability::TokenizerUnavailable),
        };
        let owner=need(&out.units,unit)?;
        let corpus=out.corpus.insert(CorpusText{family:owner.family,rendering_version:definition.rendering_version,digest:ContentHash::of(text.as_bytes()),text:text.clone().into()})?;
        let window=out.windows.insert(SearchWindow{definition:definition.id(),unit,ordinal:ordinal as i64,corpus,digest:ContentHash::of(text.as_bytes()),text:text.clone().into(),encoded_digest:embedding::value::input_hash(&input_text),input_text:input_text.clone().into(),tokenizer:d.tokenizer().map(|t|t.identity()),tokens,availability})?;
        let mut offset=body_start;
        let mut map_ordinal=0;
        if body_start>0{synthetic(out,window,&mut map_ordinal,0,body_start)?;}
        for (index,part) in selected.iter().enumerate(){
            out.window_parts.insert(WindowPart{window,ordinal:index as i64,part:part.id(),start:0,end:part.text.len() as i64})?;
            for map in maps.get(&part.id()).into_iter().flatten() {
                out.window_maps.insert(WindowSourceMap{window,ordinal:map_ordinal,start:offset as i64+map.start,end:offset as i64+map.end,part:Some(part.id()),original:map.original,original_start:map.original_start,original_end:map.original_end})?;map_ordinal+=1;
            }
            offset+=part.text.len();
            if index+1<selected.len(){synthetic(out,window,&mut map_ordinal,offset,offset+1)?;offset+=1;}
        }
        if offset<input_text.len(){synthetic(out,window,&mut map_ordinal,offset,input_text.len())?;}
        // Nomination is separately produced from primary evidence, not unit parent membership.
        let nominations=super::build::nominations(d,out,unit)?;
        for part in &primary {
            for nomination in &nominations {
                if super::build::nominates_part(d,out,part,nomination)? {
                    out.bindings.insert(WindowBinding{window,part:part.id(),subject:nomination.subject,basis:nomination.basis,qualification:nomination.qualification})?;
                }
            }
        }
    }
    Ok(())
}
pub(super) fn context_index<'a>(rows:impl Iterator<Item=&'a PartContext>)->std::collections::BTreeMap<Id<ContentPart>,std::collections::BTreeSet<Id<ContentPart>>>{
    let mut index=std::collections::BTreeMap::<_,std::collections::BTreeSet<_>>::new();
    for row in rows{index.entry(row.primary).or_default().insert(row.context);}index
}
fn required_context(dependencies:&std::collections::BTreeMap<Id<ContentPart>,std::collections::BTreeSet<Id<ContentPart>>>,primary:&std::collections::BTreeSet<Id<ContentPart>>)->std::collections::BTreeSet<Id<ContentPart>>{primary.iter().flat_map(|id|dependencies.get(id).into_iter().flatten()).copied().collect()}
pub(super) fn selected_parts<'a>(parts:&[&'a ContentPart],dependencies:&std::collections::BTreeMap<Id<ContentPart>,std::collections::BTreeSet<Id<ContentPart>>>,unit:Id<Unit>,primary:&std::collections::BTreeSet<Id<ContentPart>>)->Result<Vec<&'a ContentPart>,ModelError>{
    let mut selected=primary.clone();selected.extend(required_context(dependencies,primary));
    if primary.is_empty(){selected.extend(parts.iter().filter(|p|p.purpose==PartPurpose::Context).map(|p|p.id()));}
    let mut result=vec![];
    for part in parts{if !selected.remove(&part.id()){continue;}if part.unit!=unit || (primary.contains(&part.id())&&part.purpose!=PartPurpose::Primary)||(!primary.contains(&part.id())&&part.purpose!=PartPurpose::Context){return Err(invalid("retrieval interpretation closure crosses unit/domain or role"));}result.push(*part);}
    if !selected.is_empty(){return Err(invalid("retrieval interpretation closure crosses unit/domain or role"));}
    result.sort_by_key(|p|p.ordinal);Ok(result)
}
fn render(parts:&[&ContentPart])->String{parts.iter().map(|p|p.text.as_str()).collect::<Vec<_>>().join("\n")}
fn synthetic(out:&mut Output,window:Id<SearchWindow>,ordinal:&mut i64,start:usize,end:usize)->Result<(),ModelError>{out.window_maps.insert(WindowSourceMap{window,ordinal:*ordinal,start:start as i64,end:end as i64,part:None,original:None,original_start:None,original_end:None})?;*ordinal+=1;Ok(())}

/// Compose actual Rust byte offsets with canonical complete-input maps. Empty postprocessor
/// special ranges have no source; a token crossing synthetic/original material retains only
/// each real intersection, never an invented enclosing source span.
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct TokenSource { pub token:usize, pub original:Id<AnchorSource>, pub start:i64, pub end:i64 }
pub fn token_sources(encoded:&EncodedInput,maps:&[WindowSourceMap])->Result<Vec<TokenSource>,ModelError>{
    if encoded.offsets.len()!=encoded.specials.len(){return Err(invalid("tokenizer special mask differs from offsets"));}
    let mut result=vec![];
    for (token,((start,end),special)) in encoded.offsets.iter().zip(&encoded.specials).enumerate(){
        if *special||start==end{continue;}
        if start>end || *end>encoded.text.len(){return Err(invalid("token offset exceeds complete input"));}
        for map in maps {
            let Some(original)=map.original else{continue;};
            let a=(*start as i64).max(map.start);let z=(*end as i64).min(map.end);
            if a<z{let base=map.original_start.ok_or_else(||invalid("token source map missing original coordinates"))?;result.push(TokenSource{token,original,start:base+a-map.start,end:base+z-map.start});}
        }
    }
    Ok(result)
}
