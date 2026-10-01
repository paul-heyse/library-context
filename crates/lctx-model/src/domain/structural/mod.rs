//! Structural observations retain exact normalized/catalog lineage for synthesis and analytics.
pub mod build;
pub mod frames;
pub mod handoffs;
pub mod controls;
pub mod conclusions;
pub use conclusions::{Conclusion,ConclusionSource};
mod records;
pub use records::*;
use crate::domain::{*,normalized::Rows,resources::ResourceBudget};
#[macro_export]
macro_rules! structural_outputs {($apply:ident)=>{$apply!{
 argument_flows:$crate::domain::structural::controls::ArgumentFlow,
 literal_arguments:$crate::domain::structural::controls::LiteralArgument,
 unfollowed_arguments:$crate::domain::structural::controls::UnfollowedArgument,
 control_traversals:$crate::domain::structural::controls::ControlTraversal,
 control_steps:$crate::domain::structural::controls::ControlStep,
 control_paths:$crate::domain::structural::controls::ControlPath,
 conditional_raises:$crate::domain::structural::controls::ConditionalRaise,
 unfollowed_paths:$crate::domain::structural::controls::UnfollowedPath,
 handoff_values:$crate::domain::structural::handoffs::ValueSource,
 handoff_assessments:$crate::domain::structural::handoffs::Assessment,
 handoffs:$crate::domain::structural::handoffs::Handoff,
 handoff_groups:$crate::domain::structural::handoffs::Group,
 handoff_members:$crate::domain::structural::handoffs::Member,
 conclusions:$crate::domain::structural::Conclusion,
 conclusion_sources:$crate::domain::structural::ConclusionSource,
 frames:$crate::domain::structural::StructuralFrame,
 public:$crate::domain::structural::PublicCandidate,
 configured:$crate::domain::structural::ConfiguredSeed,
 scope:$crate::domain::structural::ScopeMember,
 traversals:$crate::domain::structural::Traversal,
 reaches:$crate::domain::structural::Reach,
 paths:$crate::domain::structural::Path,
 arcs:$crate::domain::structural::ArcSource,
 evidence:$crate::domain::structural::StepEvidence,
 steps:$crate::domain::structural::PathStep,
 unresolved:$crate::domain::structural::UnresolvedEvent,
 unresolved_steps:$crate::domain::structural::UnresolvedStep,
 usage_sites:$crate::domain::structural::UsageSite,
 usage_evidence:$crate::domain::structural::UsageEvidence,
 usage_scores:$crate::domain::structural::UsageScore,
}};}
macro_rules! outputs {($($field:ident:$ty:ty,)*)=>{
 pub struct Output {pub conclusion_qualifications:Rows<assertion::AssertionQualification>,$(pub $field:Rows<$ty>,)*}
 impl Output {
 pub fn new(b:&ResourceBudget)->Self{Self{conclusion_qualifications:Rows::new(b),$($field:Rows::new(b),)*}}
 pub fn visit(&mut self,name:&str,batch:&arrow_array::RecordBatch)->Result<bool,ModelError>{$(if name==<$ty>::NAME{self.$field.decode(batch)?;return Ok(true);})*Ok(false)}
 pub fn validation_inputs()->Vec<ValidationInput>{vec![$(ValidationInput::of::<$ty>(&["id"]),)*]}
 pub fn matches(&self,expected:&Self)->Result<(),ModelError>{$(if !self.$field.same(&expected.$field){return Err(ModelError::Invalid(format!("structural inventory differs: {}",<$ty>::NAME)));})*Ok(())}
 pub fn extend(&mut self,other:Self)->Result<(),ModelError>{for row in other.conclusion_qualifications.iter(){self.conclusion_qualifications.insert(row.clone())?;}$(for row in other.$field.iter(){self.$field.insert(row.clone())?;})*Ok(())}
 }
};}crate::structural_outputs!(outputs);
pub fn relations()->Vec<Relation>{macro_rules! rows{($($field:ident:$ty:ty,)*)=>{vec![$(Relation::of::<$ty>()),*]};}crate::structural_outputs!(rows)}
