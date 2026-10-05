use super::*;
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub enum MembershipValue{
    TypeSequenceMember(crate::domain::types::TypeSequenceMember),
    SymbolSequenceMember(crate::domain::symbols::SymbolSequenceMember),
    AssumptionMember(crate::domain::assumptions::AssumptionSetMember),
    LiteralMember(crate::domain::value::LiteralSetMember),
    AnalyticUniverseMember(crate::domain::analytics::UniverseMember),
    AnalyticPublicSelector(crate::domain::analytics::PublicSelector),
    AnalyticPartitionMember(crate::domain::analytics::PartitionMember),
    AnalyticCommunityMember(crate::domain::analytics::CommunityMember),
    AnalyticConceptObject(crate::domain::analytics::ConceptObject),
    AnalyticConceptExtent(crate::domain::analytics::ConceptExtent),
    AnalyticConceptIntent(crate::domain::analytics::ConceptIntent),
    AnalyticImplicationMember(crate::domain::analytics::ImplicationMember),
}
impl Key for MembershipValue{fn encode(&self,sink:&mut KeySink){match self{
    Self::TypeSequenceMember(row)=>{sink.part(b"variant",&(0u16).to_le_bytes());row.content_digest().encode(sink);},
    Self::SymbolSequenceMember(row)=>{sink.part(b"variant",&(1u16).to_le_bytes());row.content_digest().encode(sink);},
    Self::AssumptionMember(row)=>{sink.part(b"variant",&(2u16).to_le_bytes());row.content_digest().encode(sink);},
    Self::LiteralMember(row)=>{sink.part(b"variant",&(3u16).to_le_bytes());row.content_digest().encode(sink);},

    Self::AnalyticUniverseMember(row)=>{sink.part(b"variant",&(4u16).to_le_bytes());row.content_digest().encode(sink);},
    Self::AnalyticPublicSelector(row)=>{sink.part(b"variant",&(5u16).to_le_bytes());row.content_digest().encode(sink);},
    Self::AnalyticPartitionMember(row)=>{sink.part(b"variant",&(6u16).to_le_bytes());row.content_digest().encode(sink);},
    Self::AnalyticCommunityMember(row)=>{sink.part(b"variant",&(7u16).to_le_bytes());row.content_digest().encode(sink);},
    Self::AnalyticConceptObject(row)=>{sink.part(b"variant",&(8u16).to_le_bytes());row.content_digest().encode(sink);},
    Self::AnalyticConceptExtent(row)=>{sink.part(b"variant",&(9u16).to_le_bytes());row.content_digest().encode(sink);},
    Self::AnalyticConceptIntent(row)=>{sink.part(b"variant",&(10u16).to_le_bytes());row.content_digest().encode(sink);},
    Self::AnalyticImplicationMember(row)=>{sink.part(b"variant",&(11u16).to_le_bytes());row.content_digest().encode(sink);},
}}}
impl MembershipValue{pub fn validate(&self)->Result<(),ModelError>{match self{
    Self::TypeSequenceMember(row)=>row.validate(),
    Self::SymbolSequenceMember(row)=>row.validate(),
    Self::AssumptionMember(row)=>row.validate(),
    Self::LiteralMember(row)=>row.validate(),

    Self::AnalyticUniverseMember(row)=>row.validate(),
    Self::AnalyticPublicSelector(row)=>row.validate(),
    Self::AnalyticPartitionMember(row)=>row.validate(),
    Self::AnalyticCommunityMember(row)=>row.validate(),
    Self::AnalyticConceptObject(row)=>row.validate(),
    Self::AnalyticConceptExtent(row)=>row.validate(),
    Self::AnalyticConceptIntent(row)=>row.validate(),
    Self::AnalyticImplicationMember(row)=>row.validate(),
}}pub fn references(&self)->Vec<super::super::SemanticReference>{match self{
    Self::TypeSequenceMember(row)=>row.references(),
    Self::SymbolSequenceMember(row)=>row.references(),
    Self::AssumptionMember(row)=>row.references(),
    Self::LiteralMember(row)=>row.references(),

    Self::AnalyticUniverseMember(row)=>row.references(),
    Self::AnalyticPublicSelector(row)=>row.references(),
    Self::AnalyticPartitionMember(row)=>row.references(),
    Self::AnalyticCommunityMember(row)=>row.references(),
    Self::AnalyticConceptObject(row)=>row.references(),
    Self::AnalyticConceptExtent(row)=>row.references(),
    Self::AnalyticConceptIntent(row)=>row.references(),
    Self::AnalyticImplicationMember(row)=>row.references(),
}}pub fn semantic_key(&self)->SemanticKey{match self{
    Self::TypeSequenceMember(row)=>SemanticKey::of(row.id()),
    Self::SymbolSequenceMember(row)=>SemanticKey::of(row.id()),
    Self::AssumptionMember(row)=>SemanticKey::of(row.id()),
    Self::LiteralMember(row)=>SemanticKey::of(row.id()),

    Self::AnalyticUniverseMember(row)=>SemanticKey::of(row.id()),
    Self::AnalyticPublicSelector(row)=>SemanticKey::of(row.id()),
    Self::AnalyticPartitionMember(row)=>SemanticKey::of(row.id()),
    Self::AnalyticCommunityMember(row)=>SemanticKey::of(row.id()),
    Self::AnalyticConceptObject(row)=>SemanticKey::of(row.id()),
    Self::AnalyticConceptExtent(row)=>SemanticKey::of(row.id()),
    Self::AnalyticConceptIntent(row)=>SemanticKey::of(row.id()),
    Self::AnalyticImplicationMember(row)=>SemanticKey::of(row.id()),
}}}
