//! Exact selected producer binding for compact outcomes, shared by all compiler
//! validation consumers. These semantic owners do not grant physical read access.
use super::*;
use lctx_model::domain::uniform_outcome::{UniformNotRequested,UniformKind,ValidatedUniformOwner,AnalysisUniformInputs};
pub struct AdmittedUniformOwners {
    owners:lctx_model::domain::charged::ChargedVec<ValidatedUniformOwner>,
    _charge:StateCharge,
}
impl AdmittedUniformOwners {
    pub fn owners(&self)->&[ValidatedUniformOwner]{&self.owners}
}
impl AdmissionRuntime {
    async fn analysis_uniform_inputs(&self,owner:&CompletedContribution,bindings:&[CompletedBinding],profile:Profile)
        ->Result<AnalysisUniformInputs,ModelError>{
        let mut inputs=AnalysisUniformInputs::new(profile,self.budget());
        let required=AnalysisUniformInputs::inputs();let mut loaded=lctx_model::domain::charged::ChargedSet::default();
        let mut charge=StateCharge::new(self.budget(),"compact analysis selected metadata views");
        for source in &owner.spec.inputs {
            if source.relation()==UniformNotRequested::NAME || !required.iter().any(|input|input.name()==source.relation()){continue;}
            if loaded.contains(&source.view()){continue;}
            let binding=bindings.iter().find(|binding|binding.source==*source)
                .ok_or(ModelError::Conflict("compact analysis actual selected metadata binding absent"))?;
            binding.validate()?;
            let relation=self.model().relation(source.relation()).ok_or(ModelError::Schema("compact analysis metadata relation"))?;
            let mut rows=self.native.scan_batches(&binding.view,relation,None,None,self.budget(),self.options.batch_rows).await?;
            let mut count=0u64;
            while let Some(batch)=rows.try_next().await.map_err(ModelError::codec)? {
                self.cancellation.check()?;
                count=count.checked_add(batch.num_rows() as u64).ok_or(ModelError::Conflict("compact analysis metadata count overflow"))?;
                if count>binding.view.rows{return Err(ModelError::Conflict("compact analysis metadata exceeds exact view"));}
                inputs.visit(source.relation(),&batch)?;
            }
            if count!=binding.view.rows{return Err(ModelError::Conflict("compact analysis metadata missing physical EOF rows"));}
            loaded.insert(&mut charge,source.view())?;
        }
        Ok(inputs)
    }
    pub async fn uniform_owners_async(&self,profile:Profile)->Result<Arc<AdmittedUniformOwners>,ModelError>{
        let mut charge=StateCharge::new(self.budget(),"selected compact producer owners");
        let source=self.completed.lock().map_err(|_|poisoned())?.get(UniformNotRequested::NAME).cloned();
        let key=source.as_ref().map(|source|(profile,source.view_identity()));
        if let Some(key)=key {
            if let Some((known,owners))=&*self.memo.uniform.lock().map_err(|_|poisoned())? {
                if *known==key{return Ok(owners.clone());}
            }
        }
        let mut owners=lctx_model::domain::charged::ChargedVec::default();
        if let Some(source)=source {
            if source.view().rows!=0 {
                let (rows,_rows)=self.coverage_rows_async::<UniformNotRequested>(&source).await?;
                let (completed,bindings,_transfer)=self.completed_inventory()?;
                for row in rows {
                    let mut matching=completed.iter().filter(|owner|owner.spec.producer==row.owner
                        && owner.identity().is_ok_and(|identity|source.view().contributions.contains(&identity)));
                    let owner=matching.next().ok_or(ModelError::Conflict("compact outcome has no exact selected producer"))?;
                    if matching.next().is_some(){return Err(ModelError::Conflict("ambiguous selected compact outcome producer"));}
                    let owner=match row.kind {
                        UniformKind::NativeFamily=>ValidatedUniformOwner::validate_semantic_binding(owner,&row,profile,self.budget())?,
                        UniformKind::AnalysisOwner=>{
                            let inputs=self.analysis_uniform_inputs(owner,&bindings,profile).await?;
                            ValidatedUniformOwner::validate_analysis_binding(owner,&row,profile,self.model(),&bindings,&inputs,self.budget())?
                        },
                    };
                    if owners.iter().any(|prior:&ValidatedUniformOwner|prior.descriptor().id()==row.id()) {
                        return Err(ModelError::Conflict("duplicate selected compact outcome"));
                    }
                    owners.push(&mut charge,owner)?;
                }
            }
        }
        let owners=Arc::new(AdmittedUniformOwners{owners,_charge:charge});
        if let Some(key)=key{*self.memo.uniform.lock().map_err(|_|poisoned())?=Some((key,owners.clone()));}
        Ok(owners)
    }
}
