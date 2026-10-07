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
/// Supported boundaries are already represented as semantic parts. Every selected primary part
/// retains every mandatory context part. Oversized indivisible primary/context is never truncated.
pub fn construct(d:&Data, out:&mut Output, unit:Id<Unit>, b:&ResourceBudget)->Result<(),ModelError>{
    let definition=d.selected()?;
    if definition.embedding_requested && d.tokenizer().is_none(){return Err(invalid("requested retrieval requires exact acquired local tokenizer assets"));}
    let mut parts:Vec<_>=out.parts.iter().filter(|p|p.unit==unit).cloned().collect();
    parts.sort_by_key(|p|p.ordinal);
    let context:Vec<_>=parts.iter().filter(|p|p.purpose==PartPurpose::Context).cloned().collect();
    let primary:Vec<_>=parts.iter().filter(|p|p.purpose==PartPurpose::Primary).cloned().collect();
    let mut groups:Vec<Vec<ContentPart>>=vec![];
    let mut group=vec![];
    for part in primary {
        let mut tentative=group.clone(); tentative.push(part.clone());
        let text=render(&context,&tentative);
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
        let text=render(&context,&primary);
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
        for (index,part) in context.iter().chain(primary.iter()).enumerate(){
            out.window_parts.insert(WindowPart{window,ordinal:index as i64,part:part.id(),start:0,end:part.text.len() as i64})?;
            let mut maps:Vec<_>=out.part_maps.iter().filter(|m|m.part==part.id()).cloned().collect();maps.sort_by_key(|m|m.ordinal);
            for map in maps {
                out.window_maps.insert(WindowSourceMap{window,ordinal:map_ordinal,start:offset as i64+map.start,end:offset as i64+map.end,part:Some(part.id()),original:map.original,original_start:map.original_start,original_end:map.original_end})?;map_ordinal+=1;
            }
            offset+=part.text.len();
            if index+1<context.len()+primary.len(){synthetic(out,window,&mut map_ordinal,offset,offset+1)?;offset+=1;}
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
fn render(context:&[ContentPart],primary:&[ContentPart])->String{context.iter().chain(primary.iter()).map(|p|p.text.as_str()).collect::<Vec<_>>().join("\n")}
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
