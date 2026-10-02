//! Continuations bind actual admitted channels and representation independently of row count.
use super::{*,identity::*};
use crate::domain::{catalog::CatalogMember, ContentHash, Id, KeySink};
use schemars::JsonSchema;
use serde::{Deserialize,Serialize};
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
#[serde(tag="status",rename_all="snake_case",deny_unknown_fields)]
pub enum VectorChannel {Disabled {},Available {spec:ContentHash,query_vector:ContentHash},Degraded {reason:Name}}
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ChannelState {pub lexical:bool,pub vector:VectorChannel}
impl ChannelState {pub fn identity(&self)->ChannelIdentity {
    let mut sink=KeySink::new("serving-channels/v1");
    sink.part(b"channels",&postcard::to_allocvec(self).expect("finite channel encoding"));
    ChannelIdentity(sink.finish())
}}
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CursorBinding {
    pub generation:GenerationKey,pub request:RequestIdentity,pub policy:PolicyIdentity,pub wire:WireIdentity,
    pub channels:ChannelIdentity,pub group:Name,pub section:Name,pub member:Option<Id<CatalogMember>>,
    pub ordering:ContentHash,
}
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Cursor {pub binding:CursorBinding,pub offset:u64}
impl Cursor {
    pub fn identity(&self)->CursorIdentity {let mut sink=KeySink::new("serving-cursor/v1");sink.part(b"cursor",&postcard::to_allocvec(self).expect("finite cursor encoding"));CursorIdentity(sink.finish())}
    pub fn encode(&self)->Result<CursorToken,WireError> {
        let bytes=serde_json::to_vec(self)?; let mut text=String::with_capacity(bytes.len()*2);
        use std::fmt::Write;
        for byte in bytes {write!(&mut text,"{byte:02x}").expect("writing string");}
        CursorToken::new(text)
    }
    pub fn decode(token:&CursorToken,expected:&CursorBinding)->Result<Self,WireError> {
        let text=token.as_str();
        if text.len()%2!=0 {return Err(WireError::Invalid("cursor encoding".into()));}
        let bytes:Result<Vec<_>,_>=text.as_bytes().chunks_exact(2).map(|c|std::str::from_utf8(c).ok().and_then(|s|u8::from_str_radix(s,16).ok()).ok_or_else(||WireError::Invalid("cursor encoding".into()))).collect();
        let cursor:Self=serde_json::from_slice(&bytes?)?;
        if &cursor.binding!=expected {return Err(WireError::Continuation("generation, request, policy, representation, group, section, member, ordering or channel changed".into()));}
        Ok(cursor)
    }
}
