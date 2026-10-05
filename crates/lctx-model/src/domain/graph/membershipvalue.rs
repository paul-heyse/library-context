use super::*;
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub enum MembershipValue{
    TypeSequenceMember(crate::domain::types::TypeSequenceMember),
    SymbolSequenceMember(crate::domain::symbols::SymbolSequenceMember),
    AssumptionMember(crate::domain::assumptions::AssumptionSetMember),
    LiteralMember(crate::domain::value::LiteralSetMember),
}
impl Key for MembershipValue{fn encode(&self,sink:&mut KeySink){match self{
    Self::TypeSequenceMember(row)=>{sink.part(b"variant",&(0u16).to_le_bytes());row.content_digest().encode(sink);},
    Self::SymbolSequenceMember(row)=>{sink.part(b"variant",&(1u16).to_le_bytes());row.content_digest().encode(sink);},
    Self::AssumptionMember(row)=>{sink.part(b"variant",&(2u16).to_le_bytes());row.content_digest().encode(sink);},
    Self::LiteralMember(row)=>{sink.part(b"variant",&(3u16).to_le_bytes());row.content_digest().encode(sink);},
}}}
impl MembershipValue{pub fn validate(&self)->Result<(),ModelError>{match self{
    Self::TypeSequenceMember(row)=>row.validate(),
    Self::SymbolSequenceMember(row)=>row.validate(),
    Self::AssumptionMember(row)=>row.validate(),
    Self::LiteralMember(row)=>row.validate(),
}}pub fn references(&self)->Vec<super::super::SemanticReference>{match self{
    Self::TypeSequenceMember(row)=>row.references(),
    Self::SymbolSequenceMember(row)=>row.references(),
    Self::AssumptionMember(row)=>row.references(),
    Self::LiteralMember(row)=>row.references(),
}}pub fn semantic_key(&self)->SemanticKey{match self{
    Self::TypeSequenceMember(row)=>SemanticKey::of(row.id()),
    Self::SymbolSequenceMember(row)=>SemanticKey::of(row.id()),
    Self::AssumptionMember(row)=>SemanticKey::of(row.id()),
    Self::LiteralMember(row)=>SemanticKey::of(row.id()),
}}}
