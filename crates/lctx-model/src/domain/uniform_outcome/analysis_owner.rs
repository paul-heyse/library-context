//! Reconstruct a whole unrequested analysis owner from exact selected typed inputs.
use super::*;
use crate::domain::analysis::{self, owner::{AnalysisOwner,SelectedDefinitions}};

/// A named consumer's selected compact premises. Proofs are supplied by lower
/// storage admission; actual current rows independently reconstruct their domains.
pub(crate) struct AnalysisUniformConsumer {
    kind:AnalysisOwner,
    index:Option<analysis::expected::FrontierIndex>,
    definitions:normalized::Rows<analysis::AnalysisDefinition>,
    runs:normalized::Rows<attribution::ProviderRun>,
    profile:Option<stages::Profile>,
    budget:ResourceBudget,
}
impl AnalysisUniformConsumer {
    pub(crate) fn new(kind:AnalysisOwner,budget:&ResourceBudget)->Self{Self{kind,index:None,
        definitions:normalized::Rows::new(budget),runs:normalized::Rows::new(budget),profile:None,budget:budget.clone()}}
    pub(crate) fn inputs(kind:AnalysisOwner,mut inputs:Vec<ValidationInput>)->Vec<ValidationInput>{
        inputs.extend(AnalysisUniformInputs::inputs().into_iter().filter(|input|input.name()!=UniformNotRequested::NAME
            && input.name()!=retrieval::RetrievalDefinition::NAME
            && (input.name()!=embedding::text::TextDefinition::NAME || kind==AnalysisOwner::Embedding)
            && (input.name()!=analysis::settings::AnalyticsConfiguration::NAME || kind==AnalysisOwner::Analytic)
            && (input.name()!=models::ModelCatalog::NAME || matches!(kind,AnalysisOwner::Enriched|AnalysisOwner::Models|AnalysisOwner::Summary))));
        // Consumer inventory carries no epoch authority. Only an admitted same-kind
        // owner plus independent domain reconstruction can resolve a compact premise.
        inputs.push(ValidationInput::of::<UniformNotRequested>(&["id"]));
        let mut seen=std::collections::BTreeSet::new();inputs.retain(|input|seen.insert((input.name(),input.prefix())));inputs
    }
    pub(crate) fn admit(&mut self,owner:&ValidatedUniformOwner)->Result<(),ModelError>{
        if owner.analysis_owner()!=Some(self.kind){return Ok(());}
        if self.profile.is_some_and(|profile|profile!=owner.profile()){return Err(ModelError::Conflict("compact analysis consumer profile"));}
        self.profile=Some(owner.profile());
        self.index.get_or_insert_with(||{let mut index=analysis::expected::FrontierIndex::new(owner.profile(),&self.budget);index.coalesce_selected_copies();index}).admit_uniform_owner(owner)
    }
    pub(crate) fn visit(&mut self,name:&str,batch:&arrow_array::RecordBatch)->Result<(),ModelError>{
        let Some(index)=&mut self.index else{return Ok(());};
        index.visit(name,batch)?;
        if name==analysis::AnalysisDefinition::NAME{self.definitions.decode(batch)?;}
        if name==attribution::ProviderRun::NAME{self.runs.decode(batch)?;}
        Ok(())
    }
    pub(crate) fn frame(&self,input:Id<InputRevision>,context:Id<AnalysisContext>)->Result<Option<ValidatedUniformDomain>,ModelError>{
        self.index.as_ref().map(|index|index.uniform_analysis(self.kind,input,context,&self.definitions,&self.budget)).transpose().map(Option::flatten)
    }
    pub(crate) fn complete_empty(&self,ordinary_empty:bool)->Result<bool,ModelError>{
        let Some(index)=&self.index else{return Ok(false);};
        let mut frames=charged::ChargedSet::default();let mut charge=StateCharge::new(&self.budget,"compact replay actual frames");
        for run in self.runs.iter(){frames.insert(&mut charge,(run.input,run.context))?;}
        index.validate_analysis_frames(&frames)?;
        if frames.is_empty(){return Err(ModelError::Conflict("compact replay proof outside actual frame inventory"));}
        if !ordinary_empty{return Err(ModelError::Conflict("compact analysis owner has ordinary output rows"));}
        for &(input,context) in frames.iter(){if self.frame(input,context)?.is_none(){return Err(ModelError::Conflict("compact analysis owner omitted actual frame"));}}
        Ok(true)
    }
}

/// Bounded typed metadata decoded only from the caller's protected exact selected streams.
/// This type conveys no completed binding or physical read grant.
pub struct AnalysisUniformInputs {
    profile:stages::Profile,
    index:analysis::expected::FrontierIndex,
    runs:normalized::Rows<attribution::ProviderRun>,
    definitions:normalized::Rows<analysis::AnalysisDefinition>,
    parameters:normalized::Rows<analysis::MethodParameters>,
    catalogs:normalized::Rows<models::ModelCatalog>,
    analytics:normalized::Rows<analysis::settings::AnalyticsConfiguration>,
    text:normalized::Rows<embedding::text::TextDefinition>,
    retrieval:normalized::Rows<retrieval::RetrievalDefinition>,
    budget:resources::ResourceBudget,
}
impl AnalysisUniformInputs {
    pub fn new(profile:stages::Profile,budget:&ResourceBudget)->Self{let mut index=analysis::expected::FrontierIndex::new(profile,budget);index.coalesce_selected_copies();Self{
        profile,
        index,
        runs:normalized::Rows::new(budget),definitions:normalized::Rows::new(budget),
        parameters:normalized::Rows::new(budget),catalogs:normalized::Rows::new(budget),analytics:normalized::Rows::new(budget),text:normalized::Rows::new(budget),
        retrieval:normalized::Rows::new(budget),budget:budget.clone(),
    }}
    pub fn inputs()->Vec<ValidationInput>{
        let mut inputs=Vec::new();
        macro_rules! add {($($field:ident:$ty:ty,)*)=>{$(inputs.push(ValidationInput::of::<$ty>(&["id"]));)*};}
        crate::expected_domain_inputs!(add);
        inputs.extend([ValidationInput::of::<attribution::ProviderRun>(&["id"]),
            ValidationInput::of::<analysis::AnalysisDefinition>(&["id"]),
            ValidationInput::of::<analysis::MethodParameters>(&["id"]),
            ValidationInput::of::<models::ModelCatalog>(&["id"]),
            ValidationInput::of::<retrieval::RetrievalDefinition>(&["id"])]);
        inputs.sort_by_key(|input|input.name());inputs.dedup_by_key(|input|input.name());inputs
    }
    pub fn visit(&mut self,name:&str,batch:&arrow_array::RecordBatch)->Result<(),ModelError>{
        // Compact rows are outputs being constructed/validated, never input domain selectors.
        if name!=UniformNotRequested::NAME{self.index.visit(name,batch)?;}
        if name==attribution::ProviderRun::NAME{self.runs.decode(batch)?;}
        if name==analysis::AnalysisDefinition::NAME{self.definitions.decode(batch)?;}
        if name==analysis::MethodParameters::NAME{self.parameters.decode(batch)?;}
        if name==models::ModelCatalog::NAME{self.catalogs.decode(batch)?;}
        if name==analysis::settings::AnalyticsConfiguration::NAME{self.analytics.decode(batch)?;}
        if name==embedding::text::TextDefinition::NAME{self.text.decode(batch)?;}
        if name==retrieval::RetrievalDefinition::NAME{self.retrieval.decode(batch)?;}
        Ok(())
    }
    fn frames(&self)->Result<(charged::ChargedSet<(Id<InputRevision>,Id<AnalysisContext>)>,StateCharge),ModelError>{
        let mut frames=charged::ChargedSet::default();let mut charge=StateCharge::new(&self.budget,"compact analysis actual input contexts");
        for run in self.runs.iter(){frames.insert(&mut charge,(run.input,run.context))?;}
        self.index.validate_analysis_frames(&frames)?;
        Ok((frames,charge))
    }
    fn selected(&self)->Result<SelectedDefinitions<'_>,ModelError>{
        fn optional<R:Record>(rows:&normalized::Rows<R>)->Result<Option<&R>,ModelError>{
            if rows.len()>1{return Err(ModelError::Conflict("ambiguous compact analysis selected configuration"));}
            Ok(rows.iter().next())
        }
        Ok(SelectedDefinitions{definitions:&self.definitions,analytics:optional(&self.analytics)?,text:optional(&self.text)?,retrieval:optional(&self.retrieval)?})
    }
    fn validate_definitions(&self,owner:AnalysisOwner,selected:&SelectedDefinitions<'_>)->Result<(),ModelError>{
        use analysis::AnalysisMethod as M;
        for &(method,_) in owner.methods(){
            let actual=selected.definition(method)?;
            let expected=match method {
                M::LocalTransfers=>local_semantics::definition().1,
                M::Execution=>execution::configuration::base_evaluation().1,
                M::Completion=>execution::configuration::base_completion().1,
                M::SourceCalls=>execution::configuration::source_calls().1,
                M::AnalyticEmbedding=>embedding::analytic::definition().1,
                M::EnrichedExecution|M::Models|M::Summaries=>{
                    let parameters=self.parameters.get(actual.parameters).ok_or(ModelError::Conflict("compact analysis actual method parameters absent"))?;
                    let catalog=parameters.model_catalog.ok_or(ModelError::Conflict("compact analysis authored model catalog absent"))?;
                    if self.catalogs.get(catalog).is_none(){return Err(ModelError::Conflict("compact analysis selected model catalog absent"));}
                    let canonical=match method {
                        M::EnrichedExecution=>execution::configuration::enriched_execution(catalog),
                        M::Models=>execution::configuration::models(catalog),
                        M::Summaries=>{
                            let bound=|value:Option<i64>|value.and_then(|value|u32::try_from(value).ok()).ok_or(ModelError::Conflict("compact summary actual finite bound"));
                            execution::configuration::summaries(catalog,execution::configuration::SummaryLimits{depth:bound(parameters.depth)?,proof_steps:bound(parameters.proof_steps)?,work:bound(parameters.work)?,members:bound(parameters.members)?})?
                        },_=>unreachable!(),
                    };
                    if *parameters!=canonical.0{return Err(ModelError::Conflict("compact analysis method parameter meaning differs"));}canonical.1
                },
                M::PageRank|M::Communities|M::Concepts|M::RelationalConcepts|M::Neighbours=>analytics::build::definition(selected.analytics.ok_or(ModelError::Conflict("compact analytic actual settings absent"))?,method)?.1,
                _=>return Err(ModelError::Conflict("requested analysis owner cannot be compact")),
            };
            if *actual!=expected{return Err(ModelError::Conflict("compact analysis supported method meaning differs"));}
        }Ok(())
    }
    fn domain(&self,owner:AnalysisOwner,stage:&stages::Stage,sources:&[analysis::sources::SourceSnapshot],
        selected:&SelectedDefinitions<'_>,input:Id<InputRevision>,context:Id<AnalysisContext>)->Result<UniformDomain,ModelError>{
        let (entries,_entries)=self.index.unrequested_entries(owner,selected,input,&self.budget)?;
        let mut declaration=KeySink::new("uniform-current-analysis-declaration/v1");
        stage.name.to_owned().encode(&mut declaration);stage.code.encode(&mut declaration);stage.configuration.encode(&mut declaration);
        for role in &stage.inputs{role.name().to_owned().encode(&mut declaration);role.prefix().map(|prefix|prefix.name().to_owned()).encode(&mut declaration);}
        for output in stage.outputs.iter().chain(&stage.contributes){output.name().to_owned().encode(&mut declaration);}
        let mut ordered=KeySink::new("uniform-current-analysis-ordered-sources/v1");
        ordered.part(b"roles",&(sources.len() as u64).to_le_bytes());
        for source in sources{source.identity().encode(&mut ordered);}
        let mut definitions=KeySink::new("uniform-current-analysis-definition-inventory/v1");
        for &(method,capability) in owner.methods(){method.encode(&mut definitions);capability.encode(&mut definitions);selected.definition(method)?.content_digest().encode(&mut definitions);}
        let expected=ExpectedUniform{kind:UniformKind::AnalysisOwner,owner:owner.name().to_owned(),family:None,
            input,context,declaration:declaration.finish(),sources:ordered.finish(),definitions:definitions.finish(),requested:false};
        UniformDomain::new(&expected,entries.iter().cloned(),&self.budget)
    }
    /// Construct descriptors before any per-method kernel executes. All ordinary outputs
    /// must still be explicitly declared empty by the producer.
    pub fn declarations(&self,stage:&stages::Stage,sources:&[analysis::sources::SourceSnapshot])
        ->Result<normalized::Rows<UniformNotRequested>,ModelError>{
        let owner=AnalysisOwner::resolve(stage.name)?;
        if !stage.profiles.contains(&self.profile) || sources.len()!=stage.inputs.len() || sources.iter().zip(&stage.inputs).any(|(source,role)|source.relation()!=role.name()){
            return Err(ModelError::Conflict("compact analysis actual ordered source inventory"));
        }
        let selected=self.selected()?;
        self.validate_definitions(owner,&selected)?;
        self.index.assert_uniformly_unrequested(owner,&selected)?;
        let (frames,_frames)=self.frames()?;let mut rows=normalized::Rows::new(&self.budget);
        for &(input,context) in frames.iter(){rows.insert(self.domain(owner,stage,sources,&selected,input,context)?.declaration()?)?;}
        Ok(rows)
    }
}
impl ValidatedUniformOwner {
    /// Lower consumers call this only after protecting the exact owning contribution and
    /// the selected bindings whose streams populated inputs. Upstream full admission owns
    /// compiler implementation provenance; this checks independently reconstructible meaning.
    pub fn validate_analysis_binding(owner:&completed::CompletedContribution,descriptor:&UniformNotRequested,
        profile:stages::Profile,model:&ValidatedModel,bindings:&[completed::CompletedBinding],
        inputs:&AnalysisUniformInputs,budget:&ResourceBudget)->Result<Self,ModelError>{
        if inputs.profile!=profile || descriptor.kind!=UniformKind::AnalysisOwner || descriptor.family.is_some()
            || descriptor.owner!=owner.spec.producer || owner.outcome!=0 {
            return Err(ModelError::Conflict("compact analysis current complete owner required"));
        }
        owner.spec.validate_profile(profile)?;
        let kind=AnalysisOwner::resolve(&descriptor.owner)?;
        let selected=inputs.selected()?;
        inputs.validate_definitions(kind,&selected)?;
        let order=stages::PublicationOrder::planning(&analysis::owner::publication_groups(Vec::new(),true))?;
        let stage=kind.declaration(profile,&selected,model,&order)?;
        analysis::owner::validate_selected_roles(&stage,owner,bindings,model)?;
        let (frames,_frames)=inputs.frames()?;
        if !frames.contains(&(descriptor.input,descriptor.context))
            || owner.outputs.get(UniformNotRequested::NAME).map(|output|output.rows)!=Some(frames.len() as u64)
            || owner.outputs.iter().any(|(name,output)|name!=UniformNotRequested::NAME && output.rows!=0) {
            return Err(ModelError::Conflict("compact analysis whole output/domain inventory differs"));
        }
        let domain=inputs.domain(kind,&stage,&owner.spec.inputs,&selected,descriptor.input,descriptor.context)?;
        if domain.declaration()?!=*descriptor{return Err(ModelError::Conflict("compact analysis exact declaration/domain differs"));}
        let mut charge=StateCharge::new(budget,"compact analysis semantic owner");charge.grow(size_of::<SemanticOwner>()+descriptor.heap_bytes())?;
        charge.grow(kind.methods().len().saturating_mul(size_of::<analysis::AnalysisDefinition>()))?;
        let mut retained_definitions=Vec::with_capacity(kind.methods().len());
        for &(method,_) in kind.methods(){
            let row=selected.definition(method)?;charge.grow(row.heap_bytes())?;retained_definitions.push(row.clone());
        }
        Ok(Self(std::sync::Arc::new(SemanticOwner{contribution:owner.identity()?,profile,
            descriptor:descriptor.clone(),domain,analysis:Some((kind,retained_definitions)),_charge:charge})))
    }
}

/// Pure kernel fixtures reconstruct the real declaration and selected typed rows.
/// They do not construct a native read capability or establish persisted admission.
#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
    use crate::domain::completed::*;
    pub(crate) struct EmptyOwner {
        pub owner:ValidatedUniformOwner,
        pub domain:ValidatedUniformDomain,
        pub batches:Vec<(&'static str,arrow_array::RecordBatch)>,
        pub _suppliers:Vec<CompletedContribution>,
    }
    /// A pure transport fixture with an explicit terminal supplier, including zero rows.
    /// This does not assert a production supplier recipe or grant physical admission.
    pub(crate) fn supplier_view(model:&ValidatedModel,name:&str,rows:u64)->(CompletedView,CompletedContribution){
        let supplier=CompletedContribution{
            spec:ContributionSpec{captured_binding:None,producer:format!("finite-selected-records/{name}"),
                profile:OutputProfile::Specific(stages::Profile::Catalog),model:model.digest(),
                implementation:ContentHash::of(b"finite-selected-records-fixture/v1"),configuration:None,
                inputs:vec![],outputs:[name.to_owned()].into()},
            assurance:producer_assurance::ProducerAssurance::CurrentProviderInterface,outcome:0,
            outputs:[(name.to_owned(),OutputContent{rows,content:ContentHash::of(b"pure-record-fixture-output")})].into(),
        };
        let view=CompletedView::new(name.to_owned(),[supplier.identity().unwrap()].into(),rows).unwrap();
        (view,supplier)
    }
    pub(crate) fn input_context()->(InputRevision,AnalysisContext){
        let input=InputRevision::from_entries(vec![]).unwrap();
        let context=AnalysisContext{python_version:"3.14.7".into(),python_platform:"linux".into(),search_path:vec![],site_package_path:vec![],config_digest:ContentHash::of(b"compact-kernel-fixture"),environment_digest:input.manifest,lock_digest:None};
        (input,context)
    }
    pub(crate) fn empty_owner(kind:AnalysisOwner,input:&InputRevision,context:&AnalysisContext,budget:&ResourceBudget)->EmptyOwner {
        let model=crate::domain::model().unwrap();
        let mut inputs=AnalysisUniformInputs::new(stages::Profile::Catalog,budget);
        let mut batches=Vec::new();
        fn add<R:Record>(inputs:&mut AnalysisUniformInputs,batches:&mut Vec<(&'static str,arrow_array::RecordBatch)>,row:R){
            let batch=R::encode(&[row]).unwrap();inputs.visit(R::NAME,&batch).unwrap();batches.push((R::NAME,batch));
        }
        add(&mut inputs,&mut batches,input.clone());
        add(&mut inputs,&mut batches,context.clone());
        let provider=attribution::Provider{tool:"finite-compact-owner-control".into(),revision:"1".into(),build_digest:ContentHash::of(b"finite-compact-owner-control")};
        let (run,_)=attribution::ProviderRun::new(provider.id(),context.id(),input.id(),context.config_digest,[attribution::FactFamily::Syntax]).unwrap();
        add(&mut inputs,&mut batches,run);
        add(&mut inputs,&mut batches,CoverageScope::Input{input:input.id()});
        let catalog=models::Catalog::parse("finite-empty-models","version = 7\nmodels = []\n").unwrap();
        let settings=analysis::settings::AnalyticsConfiguration{module_prefixes:vec!["pkg.impl".into()],public_roots:vec!["pkg.api".into()],configured_seeds:vec![],depth:2,vertices:128,arcs:512,witnesses:3,brief_budget:0,communities:false,pagerank:false,fca:false,knn:false,rca:false,type_layer:false,mention_layer:false,knn_layer:false};
        for &(method,_) in kind.methods(){
            use analysis::AnalysisMethod as M;
            let pair=match method {
                M::LocalTransfers=>local_semantics::definition(),M::Execution=>execution::configuration::base_evaluation(),
                M::Completion=>execution::configuration::base_completion(),M::SourceCalls=>execution::configuration::source_calls(),
                M::EnrichedExecution=>execution::configuration::enriched_execution(catalog.declaration().id()),
                M::Models=>execution::configuration::models(catalog.declaration().id()),
                M::Summaries=>execution::configuration::summaries(catalog.declaration().id(),Default::default()).unwrap(),
                M::AnalyticEmbedding=>embedding::analytic::definition(),
                M::PageRank|M::Communities|M::Concepts|M::RelationalConcepts|M::Neighbours=>analytics::build::definition(&settings,method).unwrap(),
                _=>panic!("control only supports genuinely unrequested owners"),
            };
            add(&mut inputs,&mut batches,pair.0);add(&mut inputs,&mut batches,pair.1);
        }
        if matches!(kind,AnalysisOwner::Enriched|AnalysisOwner::Models|AnalysisOwner::Summary){add(&mut inputs,&mut batches,catalog.declaration().clone());}
        if kind==AnalysisOwner::Analytic{add(&mut inputs,&mut batches,settings);}
        if kind==AnalysisOwner::Embedding{add(&mut inputs,&mut batches,embedding::text::TextDefinition::builtin());}
        let order=stages::PublicationOrder::planning(&analysis::owner::publication_groups(Vec::new(),true)).unwrap();
        let stage=kind.declaration(stages::Profile::Catalog,&inputs.selected().unwrap(),&model,&order).unwrap();
        let mut suppliers=Vec::new();
        let bindings=stage.inputs.iter().map(|role|{
            let rows=batches.iter().filter(|(name,_)|*name==role.name()).map(|(_,batch)|batch.num_rows() as u64).sum();
            let (view,supplier)=supplier_view(&model,role.name(),rows);suppliers.push(supplier);
            let source=analysis::sources::SourceSnapshot::of_completed_view(model.relation(role.name()).unwrap(),model.digest(),&view).unwrap();
            CompletedBinding{boundary:role.prefix().map(|boundary|boundary.name().to_owned()),source,view,configuration:None}
        }).collect::<Vec<_>>();
        let sources=bindings.iter().map(|binding|binding.source.clone()).collect::<Vec<_>>();
        let descriptor=inputs.declarations(&stage,&sources).unwrap().iter().next().unwrap().clone();
        assert_eq!(descriptor.members,match kind{AnalysisOwner::Summary=>1,AnalysisOwner::Analytic=>5,_=>0});
        let outputs=stage.outputs.iter().chain(&stage.contributes).map(|output|output.name().to_owned()).collect::<std::collections::BTreeSet<_>>();
        let contribution=CompletedContribution{spec:ContributionSpec{captured_binding:None,producer:stage.name.into(),profile:OutputProfile::Specific(stages::Profile::Catalog),model:model.digest(),implementation:stage.code,configuration:Some(stage.configuration),inputs:sources,outputs:outputs.clone()},assurance:producer_assurance::ProducerAssurance::CurrentProviderInterface,outcome:0,
            outputs:outputs.into_iter().map(|name|{let rows=u64::from(name==UniformNotRequested::NAME);(name,OutputContent{rows,content:ContentHash::of(b"finite-compact-owner-output")})}).collect()};
        let owner=ValidatedUniformOwner::validate_analysis_binding(&contribution,&descriptor,stages::Profile::Catalog,&model,&bindings,&inputs,budget).unwrap();
        let mut consumer=AnalysisUniformConsumer::new(kind,budget);consumer.admit(&owner).unwrap();
        let batch=UniformNotRequested::encode(&[descriptor]).unwrap();batches.push((UniformNotRequested::NAME,batch));
        for (name,batch) in &batches{consumer.visit(name,batch).unwrap();}
        let domain=consumer.frame(input.id(),context.id()).unwrap().unwrap();
        EmptyOwner{owner,domain,batches,_suppliers:suppliers}
    }
}

#[cfg(test)]
mod controls {
    use super::*;
    use crate::domain::{completed::*,stages::*,analysis::sources::SourceSnapshot};
    fn visit<R:Record>(inputs:&mut AnalysisUniformInputs,rows:&[R]){inputs.visit(R::NAME,&R::encode(rows).unwrap()).unwrap();}
    struct Fixture {budget:ResourceBudget,model:ValidatedModel,inputs:AnalysisUniformInputs,stage:Stage,
        bindings:Vec<CompletedBinding>,owner:CompletedContribution,descriptor:UniformNotRequested,input:InputRevision,context:AnalysisContext,_suppliers:Vec<CompletedContribution>}
    fn fixture()->Fixture {
        let budget=ResourceBudget::fixed(64<<20).unwrap();let model=crate::domain::model().unwrap();
        let input=InputRevision::from_entries(vec![]).unwrap();
        let context=AnalysisContext{python_version:"3.14.7".into(),python_platform:"linux".into(),search_path:vec![],site_package_path:vec![],config_digest:ContentHash::of(b"compact-kernel-fixture"),environment_digest:input.manifest,lock_digest:None};
        let provider=attribution::Provider{tool:"finite-model-control".into(),revision:"1".into(),build_digest:ContentHash::of(b"finite-model-control")};
        let (run,_)=attribution::ProviderRun::new(provider.id(),context.id(),input.id(),context.config_digest,[attribution::FactFamily::Syntax]).unwrap();
        let (parameters,definition)=local_semantics::definition();
        let mut inputs=AnalysisUniformInputs::new(Profile::Catalog,&budget);
        visit(&mut inputs,std::slice::from_ref(&input));visit(&mut inputs,std::slice::from_ref(&run));
        visit(&mut inputs,std::slice::from_ref(&definition));visit(&mut inputs,std::slice::from_ref(&parameters));
        visit(&mut inputs,&[CoverageScope::Input{input:input.id()}]);
        let order=PublicationOrder::planning(&analysis::owner::publication_groups(Vec::new(),true)).unwrap();
        let stage=AnalysisOwner::Local.declaration(Profile::Catalog,&inputs.selected().unwrap(),&model,&order).unwrap();
        let mut bindings=Vec::new();let mut suppliers=Vec::new();
        for role in &stage.inputs {
            let rows=match role.name(){InputRevision::NAME|attribution::ProviderRun::NAME|analysis::AnalysisDefinition::NAME|analysis::MethodParameters::NAME|CoverageScope::NAME=>1,_=>0};
            let (view,supplier)=test_support::supplier_view(&model,role.name(),rows);suppliers.push(supplier);
            let source=SourceSnapshot::of_completed_view(model.relation(role.name()).unwrap(),model.digest(),&view).unwrap();
            bindings.push(CompletedBinding{boundary:role.prefix().map(|boundary|boundary.name().to_owned()),source,view,configuration:None});
        }
        let sources=bindings.iter().map(|binding|binding.source.clone()).collect::<Vec<_>>();
        let descriptor=inputs.declarations(&stage,&sources).unwrap().iter().next().unwrap().clone();
        assert_eq!(descriptor.members,0,"an actual empty Python domain has no invented Input member");
        let outputs=stage.outputs.iter().chain(&stage.contributes).map(|output|output.name().to_owned()).collect::<std::collections::BTreeSet<_>>();
        // Pure declaration/kernel evidence only: no native admission or read capability is constructed.
        let owner=CompletedContribution{spec:ContributionSpec{captured_binding:None,producer:stage.name.into(),profile:OutputProfile::Specific(Profile::Catalog),model:model.digest(),implementation:stage.code,configuration:Some(stage.configuration),inputs:sources,outputs:outputs.clone()},assurance:producer_assurance::ProducerAssurance::CurrentProviderInterface,outcome:0,
            outputs:outputs.into_iter().map(|name|{let rows=u64::from(name==UniformNotRequested::NAME);(name,OutputContent{rows,content:ContentHash::of(b"pure-kernel-output-control")})}).collect()};
        Fixture{budget,model,inputs,stage,bindings,owner,descriptor,input,context,_suppliers:suppliers}
    }
    #[test]
    fn compact_analysis_binding_preserves_empty_domain_and_rejects_nonempty_outputs_or_changed_roles(){
        let f=fixture();
        let proof=ValidatedUniformOwner::validate_analysis_binding(&f.owner,&f.descriptor,Profile::Catalog,&f.model,&f.bindings,&f.inputs,&f.budget).unwrap();
        assert_eq!(proof.analysis_owner(),Some(AnalysisOwner::Local));
        let mut consumer=AnalysisUniformConsumer::new(AnalysisOwner::Local,&f.budget);consumer.admit(&proof).unwrap();
        consumer.visit(InputRevision::NAME,&InputRevision::encode(std::slice::from_ref(&f.input)).unwrap()).unwrap();
        consumer.visit(analysis::AnalysisDefinition::NAME,&analysis::AnalysisDefinition::encode(&[local_semantics::definition().1]).unwrap()).unwrap();
        consumer.visit(UniformNotRequested::NAME,&UniformNotRequested::encode(std::slice::from_ref(&f.descriptor)).unwrap()).unwrap();
        let runs=f.inputs.runs.iter().cloned().collect::<Vec<_>>();consumer.visit(attribution::ProviderRun::NAME,&attribution::ProviderRun::encode(&runs).unwrap()).unwrap();
        assert!(consumer.frame(f.input.id(),f.context.id()).unwrap().unwrap().is_explicit_empty());
        assert!(consumer.complete_empty(true).unwrap());assert!(consumer.complete_empty(false).is_err());
        let mut changed=f.owner.clone();let output=changed.outputs.iter_mut().find(|(name,_)|name.as_str()!=UniformNotRequested::NAME).unwrap().1;output.rows=1;
        assert!(ValidatedUniformOwner::validate_analysis_binding(&changed,&f.descriptor,Profile::Catalog,&f.model,&f.bindings,&f.inputs,&f.budget).is_err());
        let mut changed=f.owner.clone();changed.spec.inputs.swap(0,1);
        assert!(ValidatedUniformOwner::validate_analysis_binding(&changed,&f.descriptor,Profile::Catalog,&f.model,&f.bindings,&f.inputs,&f.budget).is_err());
    }
    #[test]
    fn compact_analysis_invariant_inventory_keeps_producer_checkpoint_exact(){
        let f=fixture();
        let uniform=AnalysisUniformConsumer::inputs(AnalysisOwner::Local,vec![]).into_iter()
            .find(|input|input.name()==UniformNotRequested::NAME).unwrap();
        assert_eq!(uniform.prefix(),None);
        let checkpoint=f.stage.inputs.iter().find(|input|input.name()==UniformNotRequested::NAME).unwrap();
        assert_eq!(checkpoint.prefix(),Some(PublicationBoundary::Facts));
        let proof=ValidatedUniformOwner::validate_analysis_binding(&f.owner,&f.descriptor,Profile::Catalog,&f.model,&f.bindings,&f.inputs,&f.budget).unwrap();
        let mut consumer=AnalysisUniformConsumer::new(AnalysisOwner::Local,&f.budget);consumer.admit(&proof).unwrap();
        consumer.visit(InputRevision::NAME,&InputRevision::encode(std::slice::from_ref(&f.input)).unwrap()).unwrap();
        consumer.visit(analysis::AnalysisDefinition::NAME,&analysis::AnalysisDefinition::encode(&[local_semantics::definition().1]).unwrap()).unwrap();
        consumer.visit(UniformNotRequested::NAME,&UniformNotRequested::encode(std::slice::from_ref(&f.descriptor)).unwrap()).unwrap();
        assert!(consumer.frame(f.input.id(),f.context.id()).unwrap().unwrap().is_explicit_empty());
        for boundary in [None,Some(PublicationBoundary::Local.name().to_owned())]{
            let mut bindings=f.bindings.clone();
            for binding in bindings.iter_mut().filter(|binding|binding.source.relation()==UniformNotRequested::NAME){binding.boundary=boundary.clone();}
            assert!(ValidatedUniformOwner::validate_analysis_binding(&f.owner,&f.descriptor,Profile::Catalog,&f.model,&bindings,&f.inputs,&f.budget).is_err());
        }
    }
    #[test]
    fn compact_analysis_requires_selected_owner_and_rejects_changed_actual_scopes_or_requested_profile(){
        let mut f=fixture();
        let mut unprotected=analysis::expected::FrontierIndex::new(Profile::Catalog,&f.budget);
        unprotected.visit(UniformNotRequested::NAME,&UniformNotRequested::encode(std::slice::from_ref(&f.descriptor)).unwrap()).unwrap();
        assert!(unprotected.uniform_analysis(AnalysisOwner::Local,f.input.id(),f.context.id(),&f.inputs.definitions,&f.budget).is_err());
        let artifact=source::SourceArtifact::from_bytes(f.input.id(),"now.py".into(),b"x=1\n").unwrap();
        visit(&mut f.inputs,std::slice::from_ref(&artifact));visit(&mut f.inputs,&[input::ArtifactUse{artifact:artifact.id(),input:f.input.id(),role:input::SourceRole::Release}]);visit(&mut f.inputs,&[CoverageScope::Artifact{artifact:artifact.id()}]);
        assert!(ValidatedUniformOwner::validate_analysis_binding(&f.owner,&f.descriptor,Profile::Catalog,&f.model,&f.bindings,&f.inputs,&f.budget).is_err());
        let mut requested=AnalysisUniformInputs::new(Profile::Behavioral,&f.budget);visit(&mut requested,std::slice::from_ref(&f.input));visit(&mut requested,&[local_semantics::definition().1]);
        assert!(requested.declarations(&f.stage,&f.owner.spec.inputs).is_err());
    }
    #[test]
    fn compact_analysis_closed_nine_owner_inventory_reconstructs_actual_empty_domains(){
        let budget=ResourceBudget::fixed(64<<20).unwrap();let (input,context)=test_support::input_context();
        let mut count=0;
        for kind in AnalysisOwner::ALL.into_iter().filter(|kind|kind.can_be_wholly_unrequested()){
            let actual=test_support::empty_owner(kind,&input,&context,&budget);
            assert_eq!(actual.owner.analysis_owner(),Some(kind));
            // Independent fixed oracle from the declared grains: six execution owners
            // have only Python artifacts, Embedding has Python+Document artifacts,
            // Summary additionally has one InvocationProjection input subject, and
            // each of the five analytic methods additionally has that input subject.
            use analysis::AnalysisMethod as M;
            let input_methods:&[M]=match kind{
                AnalysisOwner::Local|AnalysisOwner::Base|AnalysisOwner::Completion|AnalysisOwner::SourceCalls
                |AnalysisOwner::Enriched|AnalysisOwner::Models|AnalysisOwner::Embedding=>&[],
                AnalysisOwner::Summary=>&[M::Summaries],
                AnalysisOwner::Analytic=>&[M::PageRank,M::Communities,M::Concepts,M::RelationalConcepts,M::Neighbours],
                _=>panic!("closed nine owner fixture"),
            };
            let expected=input_methods.iter().map(|&method|{
                let definition=actual.owner.analysis_definition(method).unwrap();
                (CoverageScope::Input{input:input.id()}.id(),definition.content_digest())
            }).collect::<std::collections::BTreeSet<_>>();
            assert_eq!(actual.domain.members().copied().collect::<std::collections::BTreeSet<_>>(),expected,
                "empty Python inventory retains only the model's explicit input-level subjects for {kind:?}");
            assert_eq!(actual.domain.is_explicit_empty(),expected.is_empty());
            assert_eq!(actual.domain.descriptor_row().unwrap(),*actual.owner.descriptor());count+=1;
        }
        assert_eq!(count,9);
    }
    #[test]
    fn compact_analysis_zero_input_inventory_has_no_placeholder_descriptor_and_requested_analytic_refuses(){
        let budget=ResourceBudget::fixed(64<<20).unwrap();let model=crate::domain::model().unwrap();
        let order=PublicationOrder::planning(&analysis::owner::publication_groups(Vec::new(),true)).unwrap();
        let mut inputs=AnalysisUniformInputs::new(Profile::Catalog,&budget);
        let (parameters,definition)=local_semantics::definition();visit(&mut inputs,&[parameters]);visit(&mut inputs,&[definition]);
        let stage=AnalysisOwner::Local.declaration(Profile::Catalog,&inputs.selected().unwrap(),&model,&order).unwrap();
        let sources=|stage:&Stage,inputs:&AnalysisUniformInputs|stage.inputs.iter().map(|role|{
            let rows=match role.name(){analysis::AnalysisDefinition::NAME=>inputs.definitions.len(),analysis::MethodParameters::NAME=>inputs.parameters.len(),
                analysis::settings::AnalyticsConfiguration::NAME=>inputs.analytics.len(),_=>0} as u64;
            let (view,_supplier)=test_support::supplier_view(&model,role.name(),rows);
            SourceSnapshot::of_completed_view(model.relation(role.name()).unwrap(),model.digest(),&view).unwrap()
        }).collect::<Vec<_>>();
        assert!(inputs.declarations(&stage,&sources(&stage,&inputs)).unwrap().is_empty());
        let settings=analysis::settings::AnalyticsConfiguration{module_prefixes:vec!["pkg".into()],public_roots:vec!["pkg".into()],configured_seeds:vec![],depth:2,vertices:128,arcs:512,witnesses:3,brief_budget:0,communities:false,pagerank:true,fca:false,knn:false,rca:false,type_layer:false,mention_layer:false,knn_layer:false};
        let mut requested=AnalysisUniformInputs::new(Profile::Catalog,&budget);visit(&mut requested,std::slice::from_ref(&settings));
        for method in analytics::build::METHODS{let (parameters,definition)=analytics::build::definition(&settings,method).unwrap();visit(&mut requested,&[parameters]);visit(&mut requested,&[definition]);}
        let stage=AnalysisOwner::Analytic.declaration(Profile::Catalog,&requested.selected().unwrap(),&model,&order).unwrap();
        let error=requested.declarations(&stage,&sources(&stage,&requested)).err().expect("a mixed requested/unrequested analytic owner must retain ordinary outputs");
        assert!(error.to_string().contains("requested"),"{error}");
    }
}
