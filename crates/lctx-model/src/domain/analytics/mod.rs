//! Optional model-owned finite analytics over captured structural and vector evidence.
pub mod communities;
pub mod concepts;
pub mod neighbours;
pub mod policy;
pub mod ranking;
mod records;
pub use records::*;
pub mod build;
mod conclusions;
pub mod frames;
use crate::domain::{normalized::Rows, resources::ResourceBudget, *};
pub use conclusions::{Conclusion, ConclusionSource};
#[macro_export]
macro_rules! analytic_outputs {
    ($m:ident) => {
        $m! {
        conclusions:$crate::domain::analytics::Conclusion,
        conclusion_sources:$crate::domain::analytics::ConclusionSource,
        document_neighbours:$crate::domain::analytics::DocumentNeighbour,
        layer_results:$crate::domain::analytics::LayerResult,
        layer_neighbours:$crate::domain::analytics::LayerNeighbour,
        label_assessments:$crate::domain::analytics::CommunityLabelAssessment,
        labels:$crate::domain::analytics::CommunityLabel,
        label_members:$crate::domain::analytics::CommunityLabelMember,
        quality_steps:$crate::domain::analytics::QualityStep,
        extents:$crate::domain::analytics::ConceptExtent,
        intents:$crate::domain::analytics::ConceptIntent,
        implication_members:$crate::domain::analytics::ImplicationMember,
        frames:$crate::domain::analytics::AnalyticFrame,
        results:$crate::domain::analytics::TechniqueResult,
        universe:$crate::domain::analytics::UniverseMember,
        selectors:$crate::domain::analytics::PublicSelector,
        arcs:$crate::domain::analytics::GraphArc,
        pair_sources:$crate::domain::analytics::PairSource,
        contributions:$crate::domain::analytics::PairContribution,
        layer_pairs:$crate::domain::analytics::LayerPair,
        combined:$crate::domain::analytics::CombinedPair,
        ranks:$crate::domain::analytics::RankScore,
        runs:$crate::domain::analytics::CommunityRun,
        partitions:$crate::domain::analytics::PartitionMember,
        profiles:$crate::domain::analytics::CommunityProfile,
        communities:$crate::domain::analytics::Community,
        community_members:$crate::domain::analytics::CommunityMember,
        neighbours:$crate::domain::analytics::Neighbour,
        vectors:$crate::domain::analytics::VectorSelection,
        scopes:$crate::domain::analytics::ConceptScope,
        objects:$crate::domain::analytics::ConceptObject,
        attributes:$crate::domain::analytics::Attribute,
        incidence_sources:$crate::domain::analytics::IncidenceSource,
        incidences:$crate::domain::analytics::Incidence,
        concepts:$crate::domain::analytics::Concept,
        implications:$crate::domain::analytics::Implication,
        }
    };
}
macro_rules! output {($($f:ident:$t:ty,)*)=>{
pub struct Output{pub qualifications:Rows<assertion::AssertionQualification>,$(pub $f:Rows<$t>,)*}
impl Output{pub fn new(b:&ResourceBudget)->Self{Self{qualifications:Rows::new(b),$($f:Rows::new(b),)*}}pub fn visit(&mut self,n:&str,b:&arrow_array::RecordBatch)->Result<bool,ModelError>{$(if n==<$t>::NAME{self.$f.decode(b)?;return Ok(true);})*Ok(false)}pub fn validation_inputs()->Vec<ValidationInput>{vec![$(ValidationInput::of::<$t>(&["id"]),)*]}pub fn matches(&self,e:&Self)->Result<(),ModelError>{$(if !self.$f.same(&e.$f){return Err(ModelError::Invalid(format!("analytic output differs: {}",<$t>::NAME)));})*Ok(())}pub fn extend(&mut self,e:Self)->Result<(),ModelError>{for r in e.qualifications.iter(){self.qualifications.insert(r.clone())?;}$(for r in e.$f.iter(){self.$f.insert(r.clone())?;})*Ok(())}}
};}
crate::analytic_outputs!(output);
pub fn relations() -> Vec<Relation> {
    macro_rules! rows {($($f:ident:$t:ty,)*)=>{vec![$(Relation::of::<$t>()),*]};}
    crate::analytic_outputs!(rows)
}
mod attributes;
mod partitions;
mod vectors;
