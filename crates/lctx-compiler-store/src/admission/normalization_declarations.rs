//! Normalization completion names the actual selected producer declaration.
use super::*;
use lctx_model::domain::{self as d,normalized::coverage::NormalizationComputation,
 producer_assurance::ProducerAssurance};
impl AdmissionRuntime {
 pub(super) async fn validate_normalization_declarations(&self,profile:Profile,selected:&[Arc<ExactRelation>])->Result<(),ModelError>{
  let Some(source)=selected.iter().find(|source|source.name()==NormalizationComputation::NAME)else{return Ok(());};
  let (owners,_,_charge)=self.completed_inventory()?;
  let mut stream=self.native.scan_batches(source.view(),&Relation::of::<NormalizationComputation>(),None,None,self.budget(),self.options.batch_rows).await?;
  while let Some(batch)=stream.try_next().await.map_err(ModelError::codec)? {
   self.cancellation.check()?;
   for row in NormalizationComputation::decode(&batch)? {
    if row.profile!=profile.name()||row.policy!=d::normalized::policy_revision(){return Err(ModelError::Conflict("normalization selected profile/policy differs"));}
    let producer=row.capability.producer(profile);
    if row.producer!=producer.name{return Err(ModelError::Conflict("normalization capability original producer differs"));}
    let mut declaration=None;
    for owner in owners.iter().filter(|owner|owner.spec.producer==row.producer) {
     owner.spec.validate_profile(profile)?;
     let expected=match &owner.assurance {
      ProducerAssurance::CurrentProviderInterface=>producer.digest(),
     };
     if declaration.replace(expected).is_some_and(|prior|prior!=expected){return Err(ModelError::Conflict("normalization selected producer declaration ambiguous"));}
    }
    if declaration!=Some(row.declaration){return Err(ModelError::Conflict("normalization computation has no exact selected producer declaration"));}
   }
  }Ok(())
 }
}
