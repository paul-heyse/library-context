//! Finite input contracts. Structural decoding happens before store effects.
use super::*;
use crate::domain::{*};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PageRequest {
    #[serde(default="default_page_size")] pub size:u32,
    #[serde(default,skip_serializing_if="Optional::is_absent")] pub cursor:Optional<CursorToken>,
    #[serde(default)] pub expanded:bool,
}
fn default_page_size()->u32{ResourceLimits::default().default_page_rows}
impl Default for PageRequest{fn default()->Self{Self{size:default_page_size(),cursor:Optional::default(),expanded:false}}}
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
#[serde(tag="kind",rename_all="snake_case",deny_unknown_fields)]
pub enum OperationSelector { Member {member:Id<catalog::CatalogMember>}, PublicPath {path:Vec<Name>} }
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
#[serde(tag="kind",rename_all="snake_case",deny_unknown_fields)]
pub enum BrowseScope { Library {}, Module {module:Id<source::Module>}, Class {member:Id<catalog::CatalogMember>} }
impl Default for BrowseScope{fn default()->Self{Self::Library {}}}
#[derive(Debug,Clone,Copy,PartialEq,Eq,Serialize,Deserialize,JsonSchema,Default)]
#[serde(rename_all="snake_case")]
pub enum BrowseView { #[default] Members, Modules, Classes, Vocabulary }
#[derive(Debug,Clone,Copy,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
#[serde(rename_all="snake_case")]
pub enum OperationSection { Scenarios, Deployment, Relationships, Conflicts, Briefs, Behavior }
pub fn default_selection()->selection::Selection{selection::Selection::default()}
macro_rules! request {($name:ident {$($field:ident:$ty:ty),*$(,)?})=>{
    #[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
    #[serde(deny_unknown_fields)]
    pub struct $name {$ (pub $field:$ty,)* #[serde(default)] pub page:PageRequest}
};}
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SearchOperationsRequest {pub library:Name,pub query:QueryText,#[serde(default)]pub selection:SelectionInput,#[serde(default)]pub page:PageRequest}
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FindOperationsRequest {pub library:Name,#[serde(default)]pub selection:SelectionInput,#[serde(default)]pub page:PageRequest}
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GetOperationRequest {pub library:Name,pub operation:OperationSelector,#[serde(default)]pub sections:Vec<OperationSection>,#[serde(default)]pub page:PageRequest}
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BrowseLibraryRequest {
    pub library:Name, #[serde(default)] pub scope:BrowseScope, #[serde(default)] pub view:BrowseView,
    #[serde(default)] pub selection:SelectionInput,
    #[serde(default)] pub page:PageRequest,
}
/// The finite canonical Selection owns all predicate interpretation.
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
#[serde(transparent)]
pub struct SelectionInput(pub selection::Selection);
impl Default for SelectionInput{fn default()->Self{Self(default_selection())}}
request!(GetEvidenceRequest {source:OriginalReference});
request!(SearchEvidenceRequest {library:Name,query:QueryText,families:Vec<retrieval::Family>});
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CompareOperationsRequest {pub library:Name,pub operations:Vec<OperationSelector>,#[serde(default)]pub selection:SelectionInput,#[serde(default)]pub page:PageRequest}
request!(SearchCapabilitiesRequest {library:Name,query:QueryText});
request!(GetCapabilityRequest {capability:Id<synthesis::briefs::Brief>});
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExactInputBinding {pub formal:Id<normalized::entities::ParameterEntity>,pub value:native_requests::ExactScalar}
request!(InspectValuePathsRequest {member:Id<catalog::CatalogMember>,analysis:Id<attribution::AnalysisContext>,inputs:Vec<ExactInputBinding>,assumptions:native_requests::Assumptions});
