//! A compact unrequested outcome is total only over its independently reconstructed domain.
//! This record is not authority by itself: lower admission also binds it to the exact owning
//! completed producer declaration and held source inventory.
use crate::domain::{*,attribution::{AnalysisContext,ProviderCoverage,FactFamily},input::InputRevision,
    source::CoverageScope,resources::ResourceBudget,charged::{ChargedSet,StateCharge}};
use crate::{Domain,DomainCode};
#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash,DomainCode)]
#[repr(i16)]
pub enum UniformKind {NativeFamily=0,AnalysisOwner=1}
#[derive(Debug,Clone,PartialEq,Eq,Domain,serde::Serialize,serde::Deserialize)]
#[model(name="uniform_not_requested",validate=validate)]
pub struct UniformNotRequested {
    #[model(key)] pub kind:UniformKind,
    #[model(key)] pub owner:String,
    #[model(key)] pub family:Option<FactFamily>,
    #[model(key)] pub input:Id<InputRevision>,
    #[model(key)] pub context:Id<AnalysisContext>,
    #[model(key)] pub declaration:ContentHash,
    #[model(key)] pub sources:ContentHash,
    #[model(key)] pub definitions:ContentHash,
    #[model(key)] pub domain:ContentHash,
    #[model(key)] pub members:i64,
}
fn validate(row:&UniformNotRequested)->Result<(),ModelError>{
    if row.owner.is_empty() || row.members<0 || (row.kind==UniformKind::NativeFamily)!=row.family.is_some(){return Err(ModelError::Conflict("invalid uniform unrequested descriptor"));}Ok(())
}
/// This is a reference to actual persisted authority, not a synthesized ProviderCoverage ID.
#[derive(Debug,Clone,Copy,PartialEq,Eq,PartialOrd,Ord,serde::Serialize,serde::Deserialize,schemars::JsonSchema)]
#[serde(tag="kind",rename_all="snake_case",deny_unknown_fields)]
pub enum CoverageAuthority {Scoped{coverage:Id<ProviderCoverage>},Uniform{descriptor:Id<UniformNotRequested>}}
/// Obtained from the actual bound stage/owning completed Spec and current exact sources;
/// method definitions and request policy are selected independently of the reported outcome.
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct ExpectedUniform {
    pub kind:UniformKind,pub owner:String,pub family:Option<FactFamily>,pub input:Id<InputRevision>,pub context:Id<AnalysisContext>,
    pub declaration:ContentHash,pub sources:ContentHash,pub definitions:ContentHash,pub requested:bool,
}
/// One entry is a real scope under one exact method/family definition. Domain identity is
/// independent of iteration order; duplicate entries coalesce, conflicting scope bodies refuse.
#[derive(Debug)]
pub struct UniformDomain {
    members:ChargedSet<(Id<CoverageScope>,ContentHash)>,digest:ContentHash,expected:ExpectedUniform,_charge:StateCharge,
}
impl UniformDomain {
    pub fn new(expected:&ExpectedUniform,entries:impl IntoIterator<Item=(CoverageScope,ContentHash)>,budget:&ResourceBudget)->Result<Self,ModelError>{
        let mut members=ChargedSet::default();let mut scopes=charged::ChargedMap::default();let mut charge=StateCharge::new(budget,"uniform-not-requested-domain");
        charge.grow(size_of::<Self>()+expected.owner.len())?;
        let mut scope_charge=StateCharge::new(budget,"uniform-not-requested-scope-order");
        for (scope,definition) in entries {
            let id=scope.id();let content=scope.content_digest();
            if scopes.get(&id).is_some_and(|prior|*prior!=content){return Err(ModelError::Conflict("uniform domain scope payload"));}
            if !scopes.contains_key(&id){scopes.insert(&mut scope_charge,id,content)?;}
            members.insert(&mut charge,(id,definition))?;
        }
        let mut digest=KeySink::new("uniform-not-requested-domain/v1");
        (expected.kind as i16).encode(&mut digest);expected.family.encode(&mut digest);expected.input.encode(&mut digest);expected.context.encode(&mut digest);expected.definitions.encode(&mut digest);
        digest.part(b"members",&(members.len() as u64).to_le_bytes());
        for (scope,definition) in members.iter(){scope.encode(&mut digest);scopes[scope].encode(&mut digest);definition.encode(&mut digest);}
        // Scope scratch is no longer retained; the domain owns only its exact membership set.
        drop(scopes);drop(scope_charge);
        Ok(Self{members,digest:digest.finish(),expected:expected.clone(),_charge:charge})
    }
    pub fn declaration(&self)->Result<UniformNotRequested,ModelError>{
        let expected=&self.expected;
        if expected.requested{return Err(ModelError::Conflict("requested domain cannot report uniform NotRequested"));}
        Ok(UniformNotRequested{kind:expected.kind,owner:expected.owner.clone(),family:expected.family,input:expected.input,context:expected.context,
            declaration:expected.declaration,sources:expected.sources,definitions:expected.definitions,domain:self.digest,
            members:i64::try_from(self.members.len()).map_err(ModelError::codec)?})
    }
    pub fn validate(self,descriptor:&UniformNotRequested,owner:&ValidatedUniformOwner)->Result<ValidatedUniformDomain,ModelError>{
        owner.accepts(descriptor)?;
        if self.expected.requested || *descriptor!=self.declaration()?{return Err(ModelError::Conflict("uniform unrequested domain/declaration/request differs"));}
        descriptor.validate()?;Ok(ValidatedUniformDomain{descriptor:descriptor.id(),domain:self})
    }
}
/// Semantic binding between a compact descriptor and its completed producer declaration.
/// This does not prove persistence, selected membership or a read grant. Lower admission must
/// first establish those properties from its exact selected physical contribution, then pass
/// this capability explicitly to each consumer. It cannot be deserialized from source tags.
#[derive(Debug,Clone)]
pub struct ValidatedUniformOwner(std::sync::Arc<SemanticOwner>);
// The shared allocation has its own retained charge; cloning the capability shares that lease.
impl HeapSize for ValidatedUniformOwner {}
#[derive(Debug)]
struct SemanticOwner {
    contribution:ContentHash,
    profile:stages::Profile,
    descriptor:UniformNotRequested,
    domain:UniformDomain,
    _charge:StateCharge,
}
impl ValidatedUniformOwner {
    pub fn validate_semantic_binding(owner:&completed::CompletedContribution,descriptor:&UniformNotRequested,profile:stages::Profile,budget:&ResourceBudget)->Result<Self,ModelError>{
        let binding=owner.spec.captured_binding.as_ref().ok_or(ModelError::Conflict("uniform outcome owner lacks captured declaration"))?;
        let _hash=budget.reserve("uniform-completed-owner-admission",binding.heap_bytes().saturating_mul(8).saturating_add(owner.spec.inputs.iter().fold(0usize,|bytes,input|bytes.saturating_add(input.heap_bytes()))).saturating_add(owner.outputs.len().saturating_mul(256)).saturating_add(65536))?;
        let identity=owner.identity()?;
        owner.spec.validate_profile(profile)?;
        if descriptor.kind!=UniformKind::NativeFamily || owner.spec.producer!=producer_contract::facts::ASSEMBLE || descriptor.owner!=owner.spec.producer || owner.outcome!=0 {
            return Err(ModelError::Conflict("uniform outcome completed producer owner"));
        }
        let contract=producer_contract::facts::assembly();
        contract.validate_binding(binding,owner.spec.configuration,&[])?;
        contract.validate_context_dependency(binding,owner.spec.profile)?;
        if owner.spec.inputs.iter().map(|input|input.relation()).ne(contract.inputs.iter().map(|input|input.name())) || owner.spec.outputs!=contract.outputs.iter().chain(&contract.contributes).map(|relation|relation.name().to_owned()).collect() || !owner.outputs.get(UniformNotRequested::NAME).is_some_and(|output|output.rows>0) {
            return Err(ModelError::Conflict("uniform outcome owner output inventory"));
        }
        let family=descriptor.family.ok_or(ModelError::Conflict("uniform native family absent"))?;
        let (_,domain)=ExpectedUniform::captured_binding(binding,profile,descriptor.input,descriptor.context,family,budget)?;
        if domain.declaration()?!=*descriptor {return Err(ModelError::Conflict("uniform outcome differs from actual completed declaration"));}
        let mut charge=StateCharge::new(budget,"uniform-semantic-owner");
        charge.grow(size_of::<SemanticOwner>()+descriptor.heap_bytes())?;
        Ok(Self(std::sync::Arc::new(SemanticOwner{contribution:identity,profile,descriptor:descriptor.clone(),domain,_charge:charge})))
    }
    pub fn contribution(&self)->ContentHash{self.0.contribution}
    pub fn profile(&self)->stages::Profile{self.0.profile}
    pub fn descriptor(&self)->&UniformNotRequested{&self.0.descriptor}
    pub fn scoped_evidence(&self,scope:Id<CoverageScope>,definition:ContentHash)->Option<UniformScopedEvidence>{
        self.0.domain.evidence(self.0.descriptor.id(),scope,definition)
    }
    fn accepts(&self,descriptor:&UniformNotRequested)->Result<(),ModelError>{
        if self.0.descriptor!=*descriptor{return Err(ModelError::Conflict("uniform outcome semantic owner differs"));}Ok(())
    }
}
/// Consumer-owned registry of semantic owners supplied by exact selected storage admission.
/// Missing owners are an error for compact outcomes; absence never means NotRequested.
#[derive(Debug)]
pub struct UniformOwners {
    owners:charged::ChargedMap<Id<UniformNotRequested>,ValidatedUniformOwner>,
    charge:StateCharge,
}
impl UniformOwners {
    pub fn new(budget:&ResourceBudget)->Self{Self{owners:Default::default(),charge:StateCharge::new(budget,"uniform-consumer-owners")}}
    pub fn include(&mut self,owner:&ValidatedUniformOwner)->Result<(),ModelError>{
        let id=owner.descriptor().id();
        if let Some(prior)=self.owners.get(&id){
            if prior.contribution()!=owner.contribution() || prior.descriptor()!=owner.descriptor(){return Err(ModelError::Conflict("ambiguous uniform completed owner"));}
            return Ok(());
        }
        self.owners.insert(&mut self.charge,id,owner.clone())?;Ok(())
    }
    pub fn require(&self,descriptor:&UniformNotRequested)->Result<&ValidatedUniformOwner,ModelError>{
        let owner=self.owners.get(&descriptor.id()).ok_or(ModelError::Conflict("uniform outcome has no selected admitted owner"))?;
        owner.accepts(descriptor)?;Ok(owner)
    }
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum ScopedUniformOutcome {NotRequested(CoverageAuthority),OutsideDomain}
pub struct ValidatedUniformDomain {descriptor:Id<UniformNotRequested>,domain:UniformDomain}
// Membership storage is retained under UniformDomain's own lease.
impl HeapSize for ValidatedUniformDomain {}
impl ValidatedUniformDomain {
    pub fn evidence(&self,scope:Id<CoverageScope>,definition:ContentHash)->Option<UniformScopedEvidence>{
        self.domain.evidence(self.descriptor,scope,definition)
    }
    pub fn scoped(&self,scope:Id<CoverageScope>,definition:ContentHash)->ScopedUniformOutcome{
        if self.domain.members.contains(&(scope,definition)){ScopedUniformOutcome::NotRequested(CoverageAuthority::Uniform{descriptor:self.descriptor})}else{ScopedUniformOutcome::OutsideDomain}
    }
    pub fn is_explicit_empty(&self)->bool{self.domain.members.is_empty()}
    pub fn authority(&self)->CoverageAuthority{CoverageAuthority::Uniform{descriptor:self.descriptor}}
    pub fn members(&self)->impl Iterator<Item=&(Id<CoverageScope>,ContentHash)>{self.domain.members.iter()}
}
impl UniformDomain {
    fn evidence(&self,descriptor:Id<UniformNotRequested>,scope:Id<CoverageScope>,definition:ContentHash)->Option<UniformScopedEvidence>{
        (!self.expected.requested && self.members.contains(&(scope,definition))).then(||UniformScopedEvidence{descriptor,scope,context:self.expected.context,family:self.expected.family,definition})
    }
}
/// A scoped result minted by the shared resolver. It retains the real compact authority;
/// it is neither a canonical ProviderCoverage row nor a substitute row identity.
#[derive(Debug,Clone,Copy)]
pub struct UniformScopedEvidence {descriptor:Id<UniformNotRequested>,scope:Id<CoverageScope>,context:Id<AnalysisContext>,family:Option<FactFamily>,definition:ContentHash}
impl UniformScopedEvidence {
    pub fn descriptor(&self)->Id<UniformNotRequested>{self.descriptor}
    pub fn scope(&self)->Id<CoverageScope>{self.scope}
    pub fn context(&self)->Id<AnalysisContext>{self.context}
    pub fn family(&self)->Option<FactFamily>{self.family}
    pub fn definition(&self)->ContentHash{self.definition}
}

impl ExpectedUniform {
    /// The caller enumerates the entire family domain from its independently prepared source
    /// classifier. The declaration's definition/request policy is always selected here.
    pub fn native_scopes(owner:&ValidatedUniformOwner,input:Id<InputRevision>,context:Id<AnalysisContext>,scopes:impl IntoIterator<Item=CoverageScope>,profile:stages::Profile,budget:&ResourceBudget)->Result<ValidatedUniformDomain,ModelError>{
        let descriptor=owner.descriptor();
        if descriptor.kind!=UniformKind::NativeFamily || descriptor.input!=input || descriptor.context!=context{return Err(ModelError::Conflict("uniform consumer input/context differs"));}
        let family=descriptor.family.ok_or(ModelError::Conflict("uniform native family absent"))?;
        let (requirement,definition)=native_definition(family)?;
        let expected=Self{kind:UniformKind::NativeFamily,owner:descriptor.owner.clone(),family:Some(family),input,context,declaration:descriptor.declaration,sources:descriptor.sources,definitions:definition,requested:requirement.requested_in.contains(&profile)};
        UniformDomain::new(&expected,scopes.into_iter().map(|scope|(scope,definition)),budget)?.validate(descriptor,owner)
    }
    /// Reconstruct the domain from a consumer's exact admitted native inputs. The owner's
    /// declaration/source hashes are accepted only through the explicit semantic capability;
    /// family policy, definition inventory and scope membership are recomputed here.
    pub fn native_inputs(owner:&ValidatedUniformOwner,input:&InputRevision,context:&AnalysisContext,artifacts:&[source::SourceArtifact],uses:&[input::ArtifactUse],profile:stages::Profile,budget:&ResourceBudget)->Result<ValidatedUniformDomain,ModelError>{
        let descriptor=owner.descriptor();
        if descriptor.kind!=UniformKind::NativeFamily || descriptor.input!=input.id() || descriptor.context!=context.id(){return Err(ModelError::Conflict("uniform consumer input/context differs"));}
        let family=descriptor.family.ok_or(ModelError::Conflict("uniform native family absent"))?;
        let (requirement,definition)=native_definition(family)?;
        let expected=Self{kind:UniformKind::NativeFamily,owner:descriptor.owner.clone(),family:Some(family),input:input.id(),context:context.id(),declaration:descriptor.declaration,sources:descriptor.sources,definitions:definition,requested:requirement.requested_in.contains(&profile)};
        let _roots=budget.reserve("uniform-consumer-analysis-roots",artifacts.len().saturating_mul(128).saturating_add(uses.len().saturating_mul(64)))?;
        let roots=admission::analysis_roots(artifacts,uses)?;
        let mut scratch=StateCharge::new(budget,"uniform-consumer-scope-preparation");let mut entries=Vec::new();
        for grain in requirement.grains {match grain {
            admission::Grain::Input=>{scratch.grow(size_of::<(CoverageScope,ContentHash)>())?;entries.push((CoverageScope::Input{input:input.id()},definition));}
            admission::Grain::Artifact(class)=>for artifact in artifacts.iter().filter(|artifact|artifact.input==input.id() && admission::ArtifactClass::of(&artifact.path)==Some(*class) && roots.contains(&artifact.id())){
                scratch.grow(size_of::<(CoverageScope,ContentHash)>())?;entries.push((CoverageScope::Artifact{artifact:artifact.id()},definition));
            },
        }}
        UniformDomain::new(&expected,entries,budget)?.validate(descriptor,owner)
    }
    pub fn captured_family(stage:&stages::Stage,profile:stages::Profile,input:Id<InputRevision>,context:Id<AnalysisContext>,family:FactFamily,budget:&ResourceBudget)->Result<(Self,UniformDomain),ModelError>{
        use producer_contract::facts;
        if stage.name!=facts::ASSEMBLE{return Err(ModelError::Conflict("uniform native outcome has foreign producer"));}
        let contract=facts::assembly();let binding=stage.captured_binding.as_ref().ok_or(ModelError::Conflict("uniform native outcome lacks actual bound declaration"))?;
        if binding.contract!=contract.identity() || binding.semantic_revision!=contract.semantic_revision || binding.producer_settings!=stage.configuration{return Err(ModelError::Conflict("uniform native bound declaration differs"));}
        Self::captured_binding(binding,profile,input,context,family,budget)
    }
    pub fn captured_binding(binding:&producer_contract::CapturedProducerBinding,profile:stages::Profile,input:Id<InputRevision>,context:Id<AnalysisContext>,family:FactFamily,budget:&ResourceBudget)->Result<(Self,UniformDomain),ModelError>{
        use producer_contract::facts;
        let _hash=budget.reserve("uniform-captured-declaration-hash",binding.heap_bytes().saturating_mul(8).saturating_add(65536))?;
        let contract=facts::assembly();
        if binding.contract!=contract.identity() || binding.semantic_revision!=contract.semantic_revision {return Err(ModelError::Conflict("uniform native captured semantic contract differs"));}
        contract.validate_context_dependency(binding,completed::OutputProfile::Specific(profile))?;
        let (requirement,definition)=native_definition(family)?;
        let declaration=binding.identity()?;let mut sources=KeySink::new("uniform-captured-sources/v1");sources.part(b"ordered-sources",&(binding.sources.len() as u64).to_le_bytes());
        for source in &binding.sources {source.input.encode(&mut sources);source.context.encode(&mut sources);source.configuration.encode(&mut sources);source.acquisition.identity()?.encode(&mut sources);}
        let expected=Self{kind:UniformKind::NativeFamily,owner:facts::ASSEMBLE.into(),family:Some(family),input,context,declaration,sources:sources.finish(),definitions:definition,requested:requirement.requested_in.contains(&profile)};
        let mut entries=Vec::new();let mut scratch=StateCharge::new(budget,"uniform-native-family-scope-preparation");let mut selected=false;
        for source in binding.sources.iter().filter(|source|source.input==input && source.context==Some(context)) {
            selected=true;let acquisition=source.acquisition.as_ref();
            let _roots=budget.reserve("uniform-captured-analysis-roots",acquisition.artifacts.len().saturating_mul(128).saturating_add(acquisition.uses.len().saturating_mul(64)))?;
            let roots=admission::analysis_roots(&acquisition.artifacts,&acquisition.uses)?;
            for grain in requirement.grains {match grain {
                admission::Grain::Input=>{scratch.grow(size_of::<(CoverageScope,ContentHash)>())?;entries.push((CoverageScope::Input{input},definition));}
                admission::Grain::Artifact(class)=>for artifact in acquisition.artifacts.iter().filter(|artifact|admission::ArtifactClass::of(&artifact.path)==Some(*class) && roots.contains(&artifact.id())) {
                    scratch.grow(size_of::<(CoverageScope,ContentHash)>())?;entries.push((CoverageScope::Artifact{artifact:artifact.id()},definition));
                },
            }}
        }
        if !selected{return Err(ModelError::Conflict("uniform family source outside bound capture"));}
        let domain=UniformDomain::new(&expected,entries,budget)?;Ok((expected,domain))
    }
}

fn native_definition(family:FactFamily)->Result<(&'static admission::FamilyRequirement,ContentHash),ModelError>{
    let requirement=admission::FACTS_REQUIREMENTS.iter().find(|requirement|requirement.family==family).ok_or(ModelError::Conflict("unknown native uniform family"))?;
    let reporters=producer_contract::facts::facts();let mut reporters=reporters.iter().filter(|contract|contract.not_requested.contains(&family));
        let reporter=reporters.next().ok_or(ModelError::Conflict("uniform family lacks declared reporting policy"))?;
        if reporters.next().is_some(){return Err(ModelError::Conflict("uniform family reporting policy ambiguous"));}
        let mut definitions=KeySink::new("uniform-native-family-contract/v1");family.encode(&mut definitions);reporter.identity().encode(&mut definitions);
        for grain in requirement.grains {match grain {admission::Grain::Input=>definitions.part(b"grain",b"input"),admission::Grain::Artifact(class)=>definitions.part(b"artifact-class",&[*class as u8])};}
        for profile in requirement.requested_in {definitions.part(b"requested-profile",profile.name().as_bytes());}
        definitions.part(b"required",&[u8::from(requirement.required)]);let definition=definitions.finish();
    Ok((requirement,definition))

}
