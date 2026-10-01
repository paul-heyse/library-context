//! Structural observations retain exact normalized/catalog lineage for synthesis and analytics.
pub mod build;
pub mod frames;
mod records;
pub use records::*;
use crate::domain::{*,normalized::Rows,resources::ResourceBudget};
#[macro_export]
macro_rules! structural_outputs {($apply:ident)=>{$apply!{
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
 pub struct Output {$(pub $field:Rows<$ty>,)*}
 impl Output {
 pub fn new(b:&ResourceBudget)->Self{Self{$($field:Rows::new(b),)*}}
 pub fn visit(&mut self,name:&str,batch:&arrow_array::RecordBatch)->Result<bool,ModelError>{$(if name==<$ty>::NAME{self.$field.decode(batch)?;return Ok(true);})*Ok(false)}
 pub fn validation_inputs()->Vec<ValidationInput>{vec![$(ValidationInput::of::<$ty>(&["id"]),)*]}
 pub fn matches(&self,expected:&Self)->Result<(),ModelError>{$(if !self.$field.same(&expected.$field){return Err(ModelError::Invalid(format!("structural inventory differs: {}",<$ty>::NAME)));})*Ok(())}
 pub fn extend(&mut self,other:Self)->Result<(),ModelError>{$(for row in other.$field.iter(){self.$field.insert(row.clone())?;})*Ok(())}
 }
};}crate::structural_outputs!(outputs);
pub fn relations()->Vec<Relation>{macro_rules! rows{($($field:ident:$ty:ty,)*)=>{vec![$(Relation::of::<$ty>()),*]};}crate::structural_outputs!(rows)}
