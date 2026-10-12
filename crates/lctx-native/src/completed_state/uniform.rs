//! Semantic owners resolved from the publication's exact protected physical
//! contribution mappings. No global logical lookup or serialized grant is used.
use super::*;
use d::{Record,uniform_outcome::{UniformNotRequested,UniformKind,ValidatedUniformOwner,AnalysisUniformInputs}};
pub struct PublishedUniformOwners {
    owners:d::charged::ChargedVec<ValidatedUniformOwner>,
    _charge:d::charged::StateCharge,
}
impl PublishedUniformOwners {
    pub fn owners(&self)->&[ValidatedUniformOwner]{&self.owners}
}
impl PublishedCompletedState {
    async fn analysis_uniform_inputs(&self,owner:&d::completed::CompletedContribution,profile:d::stages::Profile,
        model:&d::ValidatedModel,budget:&d::resources::ResourceBudget)->Result<AnalysisUniformInputs,ModelError>{
        let mut inputs=AnalysisUniformInputs::new(profile,budget);
        let required=AnalysisUniformInputs::inputs();let mut loaded=d::charged::ChargedSet::default();
        let mut charge=d::charged::StateCharge::new(budget,"published compact exact metadata selections");
        for source in &owner.spec.inputs {
            if source.relation()==UniformNotRequested::NAME || !required.iter().any(|input|input.name()==source.relation()) || loaded.contains(&source.view()){continue;}
            let binding=self.inventory().bindings().iter().find(|binding|binding.source==*source)
                .ok_or(ModelError::Conflict("published compact exact selected metadata absent"))?;
            binding.validate()?;
            let relation=model.relation(source.relation()).ok_or(ModelError::Schema("published compact metadata relation"))?;
            let reader=self.inner.reader.transport_reader().with_budget(budget);
            let selected=crate::selection::SelectedPayloads::for_published_session(reader.shared_client(),self,&[source.view()],reader.read_cancellation(),budget).await?
                .with_reader_owner(reader);
            let mut variables=surrealdb::types::Variables::new();variables.insert("type",source.relation().to_owned());
            let mut rows=selected.rows_with_budget(crate::schema::ScopeTable::for_relation(source.relation())?.name(),
                "semantic_type=$type",variables,vec![],"semantic_key",None,budget)?;
            // Keep the builder alive through typed consumption: its reservation owns each
            // finished Arrow transfer until the next push/finish, including actual capacities.
            let mut builder=crate::projected_arrow::ProjectedBuilder::new(relation.clone(),relation.schema().clone(),budget)?;
            let mut count=0u64;
            let result=async {
                while let Some(row)=rows.next().await? {
                    count=count.checked_add(1).ok_or(ModelError::Conflict("published compact metadata count overflow"))?;
                    if count>binding.view.rows{return Err(ModelError::Conflict("published compact metadata exceeds exact view"));}
                    let Value::Object(mut row)=row else{return Err(ModelError::Schema("published compact metadata payload"));};
                    let body=row.remove("body").ok_or(ModelError::Schema("published compact metadata body"))?;
                    builder.push(body)?;let batch=builder.finish()?;inputs.visit(source.relation(),&batch)?;
                }
                if count!=binding.view.rows{return Err(ModelError::Conflict("published compact metadata exact EOF count"));}
                Ok(())
            }.await;
            let mut completion=d::completion::Completion::default();completion.step("published compact metadata transport drain",rows.drain_transport().await);
            d::completion::complete(result,completion)?;
            loaded.insert(&mut charge,source.view())?;
        }
        self.inner.reader.ensure_scope_live()?;
        Ok(inputs)
    }
    pub async fn admit_uniform_owners(&self,rows:&[UniformNotRequested],budget:&d::resources::ResourceBudget)
        ->Result<PublishedUniformOwners,ModelError>{
        let mut charge=d::charged::StateCharge::new(budget,"published compact semantic owners");
        let mut owners=d::charged::ChargedVec::default();
        if rows.is_empty(){return Ok(PublishedUniformOwners{owners,_charge:charge});}
        let mut bindings=self.inventory().bindings().iter().filter(|binding|binding.boundary.is_none()
            && binding.view.relation==UniformNotRequested::NAME);
        let binding=bindings.next().ok_or(ModelError::Conflict("published compact outcome view absent"))?;
        if bindings.next().is_some(){return Err(ModelError::Conflict("ambiguous published compact outcome view"));}
        let mappings=self.inventory().contributors(binding.view.identity)?;
        let reader=self.inner.reader.transport_reader().with_budget(budget);
        let mut completed=Vec::new();let mut completed_charge=d::charged::StateCharge::new(budget,"published compact producer decode");
        for mapping in mappings {
            let record=RecordId::new("compiler_contribution",mapping.physical.hex());
            let expected=crate::control::inspect_expected_read(&reader,record,budget).await?
                .ok_or(ModelError::Conflict("published compact physical contributor missing"))?;
            if expected.incarnation()!=mapping.incarnation{return Err(ModelError::Conflict("published compact contributor incarnation changed"));}
            crate::completed_rows::validate_state_row("compiler_contribution",expected.row())?;
            let Value::Object(row)=expected.row() else{return Err(ModelError::Schema("published compact contributor envelope"));};
            let Some(Value::Bytes(bytes))=row.get("descriptor")else{return Err(ModelError::Schema("published compact producer descriptor"));};
            completed_charge.grow(bytes.len().saturating_mul(32).saturating_add(4096))?;
            let contribution:d::completed::CompletedContribution=serde_json::from_slice(bytes).map_err(ModelError::codec)?;
            if contribution.identity()?!=mapping.logical || !binding.view.contributions.contains(&mapping.logical) {
                return Err(ModelError::Conflict("published compact producer exact logical mapping"));
            }
            completed.push(contribution);
        }
        let model=d::model()?;
        for row in rows {
            let mut matching=completed.iter().filter(|owner|owner.spec.producer==row.owner);
            let owner=matching.next().ok_or(ModelError::Conflict("published compact completed producer absent"))?;
            if matching.next().is_some(){return Err(ModelError::Conflict("ambiguous published compact completed producer"));}
            let profile=self.inner.admission.evidence.manifest.profile;
            let owner=match row.kind {
                UniformKind::NativeFamily=>ValidatedUniformOwner::validate_semantic_binding(owner,row,profile,budget)?,
                UniformKind::AnalysisOwner=>{
                    let inputs=self.analysis_uniform_inputs(owner,profile,&model,budget).await?;
                    ValidatedUniformOwner::validate_analysis_binding(owner,row,profile,&model,self.inventory().bindings(),&inputs,budget)?
                },
            };
            if owners.iter().any(|prior:&ValidatedUniformOwner|prior.descriptor().id()==row.id()){
                return Err(ModelError::Conflict("duplicate published compact descriptor"));
            }
            owners.push(&mut charge,owner)?;
        }
        self.inner.reader.check_read_admission()?;
        Ok(PublishedUniformOwners{owners,_charge:charge})
    }
}
