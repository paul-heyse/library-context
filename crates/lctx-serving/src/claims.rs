//! Conditional claim bases retain exact native support and the authored universe.
use lctx_model::domain::{*, serving::*, assertion::Support};
use lctx_model::domain::resources;
use lctx_surrealdb::batches::CanonicalBatches;
use std::collections::BTreeMap;
use crate::records::{rows,wire};
macro_rules! claim_rows {($($field:ident:$ty:ty,)*)=>{
    pub struct Claims {$(pub $field:BTreeMap<Id<$ty>,$ty>,)* _charge:charged::StateCharge}
    impl Claims {pub fn new(batches:&CanonicalBatches,budget:&resources::ResourceBudget)->Result<Self,ModelError>{
        let mut charge=charged::StateCharge::new(budget,"native-claim-packet-rows");
        Ok(Self{$($field:{let mut map=BTreeMap::new();for row in rows::<$ty>(batches)? {charge.admit(&row)?;charge.grow(48)?;map.insert(row.id(),row);}map},)*_charge:charge})}}
};}
claim_rows!{
    sets:assumptions::AssumptionSet,
    members:assumptions::AssumptionSetMember,
    assumptions:assumptions::Assumption,
    types:types::TypeObservation,
    type_supports:types::TypeSupport,
    classes:symbols::ClassTraitObservation,
    class_supports:symbols::ClassTraitSupport,
    universes:assumptions::AssumptionUniverse,
    universe_supports:assumptions_universe::AssumptionUniverseSupport,
    models:models::AuthoredModel,
    catalogs:models::ModelCatalog,
    qualifications:assertion::AssertionQualification,
    runs:attribution::ProviderRun,
    contexts:attribution::AnalysisContext,
    providers:attribution::Provider,
    surfaces:assertion::ProviderSurface,
    evidence:assertion::Evidence,
    occurrences:source::Occurrence,
    artifacts:source::SourceArtifact,
}
fn get<R:Record>(rows:&BTreeMap<Id<R>,R>,id:Id<R>)->Result<&R,ModelError>{rows.get(&id).ok_or_else(||ModelError::Invalid(format!("required claim row missing: {}",R::NAME)))}
impl Claims {
    pub fn basis(&self,qid:Id<assertion::AssertionQualification>)->Result<ClaimBasisPacket,ModelError>{
        let q=get(&self.qualifications,qid)?;
        let set=get(&self.sets,q.assumptions)?;
        let members=self.members.values().filter(|m|m.set==q.assumptions).cloned().collect::<Vec<_>>();
        let resolved=assumptions::ResolvedAssumptions{set:set.clone(),members};
        resolved.check(q.assumptions)?;
        let mut definitions=Vec::new();
        for member in &resolved.members {
            let assumption=get(&self.assumptions,member.assumption)?;
            definitions.push(match assumption {
                assumptions::Assumption::TypeConformance{observation,support}=>{
                    let row=get(&self.types,*observation)?;
                    let support=get(&self.type_supports,*support)?;
                    if support.assertion()!=*observation{return Err(ModelError::Conflict("assumption native type support"))}
                    ClaimAssumptionPacket::TypeConformance{assumption:member.assumption,observation:*observation,subject:row.subject,role:row.role,declared:row.declared,term:row.term,support:self.native(support)?}
                },
                assumptions::Assumption::NoExtraOverrides{class,support,universe}=>{
                    let row=get(&self.classes,*class)?;let support=get(&self.class_supports,*support)?;
                    if support.assertion()!=*class{return Err(ModelError::Conflict("assumption native class support"))}
                    let universe_row=get(&self.universes,*universe)?;
                    let mut sources=self.universe_supports.values().filter(|s|s.universe==*universe);
                    let source=sources.next().ok_or(ModelError::Schema("assumption universe support"))?;
                    if sources.next().is_some(){return Err(ModelError::Conflict("assumption universe support ambiguity"))}
                    let catalog=get(&self.catalogs,source.catalog)?;let model=get(&self.models,source.model)?;
                    if model.catalog!=catalog.id() || universe_row.model_definition!=catalog.content{return Err(ModelError::Conflict("assumption universe authored definition"))}
                    ClaimAssumptionPacket::NoExtraOverrides{assumption:member.assumption,class:*class,symbol:row.symbol,synthesized:row.synthesized,dataclass:row.dataclass,named_tuple:row.named_tuple,typed_dict:row.typed_dict,support:self.native(support)?,universe:Box::new(AssumptionUniversePacket{
                        universe:*universe,context:universe_row.context,input:universe_row.input,environment:universe_row.environment,model_definition:universe_row.model_definition,support:source.id(),catalog:catalog.id(),model:model.id(),source_name:Name::new(catalog.source_name.clone()).map_err(wire)?,format:catalog.format,source:Text::new(catalog.source.clone()).map_err(wire)?})}
                }
            });
        }
        ClaimBasisPacket::from_canonical(&resolved,definitions)
    }
    pub fn native<S:Support>(&self,support:&S)->Result<AssumptionNativeSupportPacket,ModelError>{
        let attribution=support.attribution().ok_or(ModelError::Schema("assumption requires native support"))?;
        let run=get(&self.runs,attribution.run)?;let context=get(&self.contexts,run.context)?;
        let provider=get(&self.providers,run.provider)?;let surface=get(&self.surfaces,attribution.surface)?;
        if surface.provider!=run.provider{return Err(ModelError::Conflict("native support provider surface"))}
        let evidence=match get(&self.evidence,attribution.evidence)? {
            assertion::Evidence::Invocation{run}=>AssumptionEvidencePacket::Invocation{run:*run},
            assertion::Evidence::Occurrence{occurrence}=>{
                let span=get(&self.occurrences,*occurrence)?;let artifact=get(&self.artifacts,span.source)?;
                AssumptionEvidencePacket::Source{artifact:artifact.id(),path:Name::new(artifact.path.clone()).map_err(wire)?,content:artifact.content,start:span.start,end:span.end}
            },
            assertion::Evidence::SourceSpan{source,start,end}=>{
                let artifact=get(&self.artifacts,*source)?;
                AssumptionEvidencePacket::Source{artifact:*source,path:Name::new(artifact.path.clone()).map_err(wire)?,content:artifact.content,start:*start,end:*end}
            }
        };
        Ok(AssumptionNativeSupportPacket{support:ProofReference::from_canonical(derivation::RowRef::of(support.id()))?,run:run.id(),input:run.input,context:run.context,environment:context.environment_digest,provider:Name::new(provider.tool.clone()).map_err(wire)?,provider_revision:Name::new(provider.revision.clone()).map_err(wire)?,provider_build:provider.build_digest,surface:Name::new(surface.name.clone()).map_err(wire)?,evidence,fidelity:attribution.fidelity})
    }
}
