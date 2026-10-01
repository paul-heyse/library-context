//! Retained declaration metadata. Recognition never admits a body, signature or equivalence.
use crate::domain::{*,normalized::{Rows,callables::*,entities::*,links::*},calls::*,syntax::*,symbols::*,lexical::*,resources::ResourceBudget};
use crate::{Domain,DomainCode,DomainSum};
#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash,DomainCode)]
#[repr(i16)]
pub enum AspectKind {Unknown=0,PropertyGetter=1,PropertySetter=2,PropertyDeleter=3,CachedProperty=4,ContextManager=5,AsyncContextManager=6,Wraps=7,FastMcpTool=8,FastMcpResource=9,FastMcpPrompt=10}
#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash,DomainCode)]
#[repr(i16)]
pub enum AspectAdmission {MetadataOnly=0}
#[derive(Debug,Clone,PartialEq,Eq,Hash,DomainSum)]
#[model(name="callable_aspect_sources")]
pub enum AspectSource {
    #[model(code=0)] Traits {evidence:Id<EffectiveCallableEvidence>,observation:Id<FunctionTraitObservation>},
    #[model(code=2)] Accessor {member:Id<EffectiveDecoratorMember>,candidate:Id<ReferenceEntityCandidate>,declaration:Id<DeclarationObservation>,traits:Id<FunctionTraitObservation>},
    #[model(code=1)] Decorator {member:Id<EffectiveDecoratorMember>,target:Option<Id<CallTarget>>,resolution:Option<Id<SymbolEntityResolution>>,argument:Option<Id<CallArgument>>},
}
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name="callable_aspects",invariants=invariants,semantic_source=include_bytes!("callable_aspects.rs"))]
pub struct CallableAspect {#[model(key)] pub assessment:Id<EffectiveCallableAssessment>,#[model(key)] pub source:Id<AspectSource>,#[model(key)] pub kind:AspectKind,pub admission:AspectAdmission,pub related:Option<Id<EntityRef>>}
#[derive(Debug,Clone,PartialEq,Eq,Hash,DomainSum)]
#[model(name="class_field_defaults")]
pub enum FieldDefault {
    #[model(code=0)] Absent {},
    #[model(code=1)] Unknown {},
    #[model(code=2)] Unavailable {},
    #[model(code=3)] Literal {observation:Id<SyntaxDetailObservation>,literal:Id<value::Literal>},
    #[model(code=4)] Expression {expression:Id<source::Occurrence>},
    #[model(code=5)] Factory {expression:Id<source::Occurrence>,target:Id<CallTarget>,resolution:Id<SymbolEntityResolution>,argument:Id<CallArgument>},
}
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name="class_field_default_assessments")]
pub struct FieldDefaultAssessment {#[model(key)] pub declaration:Id<FieldDeclarationLink>,pub default:Id<FieldDefault>}
#[macro_export]
macro_rules! callable_aspect_inputs {($m:ident)=>{$m! {
    callable_entities:$crate::domain::normalized::entities::CallableEntity,
    declarations:$crate::domain::syntax::DeclarationObservation,
    spellings:$crate::domain::source::SyntaxObservation,
    assessments:$crate::domain::normalized::callables::EffectiveCallableAssessment,
    members:$crate::domain::normalized::callables::EffectiveDecoratorMember,
    evidence:$crate::domain::normalized::callables::EffectiveCallableEvidence,
    premises:$crate::domain::normalized::callables::EffectiveCallablePremise,
    traits:$crate::domain::symbols::FunctionTraitObservation,
    decorators:$crate::domain::syntax::DeclarationDecorator,
    placements:$crate::domain::syntax::SyntaxPlacement,
    occurrences:$crate::domain::source::Occurrence,
    qualifications:$crate::domain::assertion::AssertionQualification,
    targets:$crate::domain::calls::CallTarget,
    destinations:$crate::domain::calls::CallDestination,
    receivers:$crate::domain::calls::Receiver,
    symbols:$crate::domain::calls::ProviderSymbol,
    provider_modules:$crate::domain::calls::ProviderModule,
    modules:$crate::domain::source::Module,
    artifacts:$crate::domain::source::SourceArtifact,
    resolutions:$crate::domain::normalized::entities::SymbolEntityResolution,
    refs:$crate::domain::normalized::entities::EntityRef,
    calls:$crate::domain::calls::CallSyntax,
    arguments:$crate::domain::calls::CallArgument,
    references:$crate::domain::lexical::ReferenceObservation,
    reference_assessments:$crate::domain::normalized::links::ReferenceEntityAssessment,
    reference_candidates:$crate::domain::normalized::links::ReferenceEntityCandidate,
    reference_targets:$crate::domain::normalized::links::ReferenceEntityTarget,
    lexical_resolutions:$crate::domain::lexical::LexicalResolution,
    binding_events:$crate::domain::lexical::BindingEvent,
    bindings:$crate::domain::lexical::BindingObservation,
    ownership:$crate::domain::input::ArtifactOwnership,
    verifications:$crate::domain::input::DistributionVerification,
    packages:$crate::domain::input::Package,
    releases:$crate::domain::input::Release,
    fields:$crate::domain::normalized::entities::FieldDeclarationLink,
    field_syntax:$crate::domain::syntax::ClassFieldSyntaxObservation,
    details:$crate::domain::syntax::SyntaxDetailObservation,
    detail_values:$crate::domain::syntax::SyntaxDetail,
}};}
#[macro_export]
macro_rules! callable_aspect_outputs {($m:ident)=>{$m! {
    sources:$crate::domain::normalized::callable_aspects::AspectSource,
    aspects:$crate::domain::normalized::callable_aspects::CallableAspect,
    defaults:$crate::domain::normalized::callable_aspects::FieldDefault,
    fields:$crate::domain::normalized::callable_aspects::FieldDefaultAssessment,
}};}
macro_rules! data {($($field:ident:$ty:ty,)*)=>{
    pub struct AspectData {$(pub $field:Rows<$ty>,)*}
    impl AspectData {pub fn new(budget:&ResourceBudget)->Self {Self {$($field:Rows::new(budget),)*}}
        pub fn visit(&mut self,name:&str,batch:&arrow_array::RecordBatch)->Result<bool,ModelError> {$(if name==<$ty>::NAME {self.$field.decode(batch)?;return Ok(true);})*Ok(false)}
        pub fn inputs()->Vec<ValidationInput> {vec![$(ValidationInput::of::<$ty>(&["id"]),)*]}
        fn stage_inputs()->Vec<stages::RelationUse> {vec![$(stages::RelationUse::stored::<$ty>()),*]}
    }
};} crate::callable_aspect_inputs!(data);
macro_rules! output {($($field:ident:$ty:ty,)*)=>{
    pub struct AspectOutput {$(pub $field:Rows<$ty>,)*}
    impl AspectOutput {pub fn new(budget:&ResourceBudget)->Self {Self {$($field:Rows::new(budget),)*}}
        pub fn visit(&mut self,name:&str,batch:&arrow_array::RecordBatch)->Result<bool,ModelError> {$(if name==<$ty>::NAME {self.$field.decode(batch)?;return Ok(true);})*Ok(false)}
        pub fn inputs()->Vec<ValidationInput> {vec![$(ValidationInput::of::<$ty>(&["id"]),)*]}
        fn matches(&self,other:&Self)->Result<(),ModelError> {$(if !self.$field.same(&other.$field) {return Err(invalid("callable metadata closure differs"));})*Ok(())}
    }
};} crate::callable_aspect_outputs!(output);
fn invalid(message:&str)->ModelError {ModelError::Invalid(message.into())}
fn need<R:Record>(rows:&Rows<R>,id:Id<R>)->Result<&R,ModelError> {rows.get(id).ok_or_else(||invalid("callable aspect premise absent"))}
fn exact(data:&AspectData,q:Id<assertion::AssertionQualification>,context:Id<attribution::AnalysisContext>)->Result<bool,ModelError> {let q=need(&data.qualifications,q)?;Ok(q.context==context && q.modality==attribution::Modality::Definite && q.approximation==assertion::Approximation::Exact && q.condition==conditions::Diagram::always().id())}
fn module_name<'a>(data:&'a AspectData,symbol:&ProviderSymbol)->Option<&'a str> {match data.provider_modules.get(symbol.module)? {ProviderModule::Acquired {module}=>data.modules.get(*module).map(|r|r.qualified_name.as_str()),ProviderModule::Bundled {name,..}=>Some(name),_=>None}}
fn standard_library(data:&AspectData,symbol:&ProviderSymbol,module:&str)->bool {
    matches!(data.provider_modules.get(symbol.module),Some(ProviderModule::Bundled {provider,bundle:ModuleBundle::Typeshed,name}) if *provider==symbol.provider && name==module)
}
fn same_span(data:&AspectData,left:Id<source::Occurrence>,right:Id<source::Occurrence>)->Result<bool,ModelError> {let l=need(&data.occurrences,left)?;let r=need(&data.occurrences,right)?;Ok((l.source,l.start,l.end)==(r.source,r.start,r.end))}
fn root(data:&AspectData,decorator:&DeclarationDecorator)->Result<Id<source::Occurrence>,ModelError> {
    if need(&data.occurrences,decorator.decorator)?.syntax_kind!=source::SyntaxKind::Decorator {return Ok(decorator.decorator);}
    let mut children=data.placements.iter().filter(|r|r.parent==Some(decorator.decorator) && data.qualifications.get(r.qualification).zip(data.qualifications.get(decorator.qualification)).is_some_and(|(l,r)|l.context==r.context));let Some(first)=children.next() else {return Ok(decorator.decorator)};
    if children.any(|r|r.occurrence!=first.occurrence) {return Ok(decorator.decorator);}Ok(first.occurrence)
}
fn resolution<'a>(data:&'a AspectData,target:&CallTarget,context:Id<attribution::AnalysisContext>)->Result<Option<(&'a ProviderSymbol,&'a SymbolEntityResolution)>,ModelError> {
    if !exact(data,target.qualification,context)? {return Ok(None);}
    let CallDestination::Resolved {symbol}=need(&data.destinations,target.destination)? else {return Ok(None)};
    let symbol=need(&data.symbols,*symbol)?;if symbol.context!=context {return Ok(None);}
    let mut resolutions=data.resolutions.iter().filter(|r|r.symbol==symbol.id() && r.context==context && r.status==ResolutionStatus::Resolved);
    let Some(first)=resolutions.next() else {return Ok(None)};if resolutions.any(|r|r.entity!=first.entity) {return Ok(None);}Ok(Some((symbol,first)))
}
fn emit(out:&mut AspectOutput,assessment:Id<EffectiveCallableAssessment>,source:AspectSource,kind:AspectKind,related:Option<Id<EntityRef>>)->Result<(),ModelError> {let source=out.sources.insert(source)?;out.aspects.insert(CallableAspect {assessment,source,kind,admission:AspectAdmission::MetadataOnly,related})?;Ok(())}
fn related(data:&AspectData,argument:&CallArgument,context:Id<attribution::AnalysisContext>)->Result<Option<Id<EntityRef>>,ModelError> {
    let mut value=None;
    for reference in data.references.iter().filter(|r|r.read==argument.value) {
        for assessment in data.reference_assessments.iter().filter(|r|r.reference==reference.id()) {
            if assessment.status!=ResolutionStatus::Resolved {return Ok(None);}
            for candidate in data.reference_candidates.iter().filter(|r|r.assessment==assessment.id()) {
                if !exact(data,need(&data.lexical_resolutions,candidate.resolution)?.qualification,context)? {return Ok(None);}
                let ReferenceEntityTarget::Binding {entity,..}=need(&data.reference_targets,candidate.target)? else {return Ok(None)};
                if value.is_some_and(|old|old!=*entity) {return Ok(None);}value=Some(*entity);
            }
        }
    }
    Ok(value)
}
fn pinned_fastmcp(data:&AspectData,symbol:&ProviderSymbol)->Result<bool,ModelError> {
    let ProviderModule::Acquired {module}=need(&data.provider_modules,symbol.module)? else {return Ok(false)};
    let artifact=need(&data.modules,*module)?.source;
    for ownership in data.ownership.iter().filter(|r|r.artifact==artifact) {
        let distribution=need(&data.verifications,ownership.distribution)?;
        let release=need(&data.releases,distribution.release)?;let package=need(&data.packages,release.package)?;
        if matches!(package.name.as_str(),"fastmcp"|"fastmcp-slim") && release.version=="4.0.5" {return Ok(true);}
    }
    Ok(false)
}
fn registration(data:&AspectData,target:&CallTarget,symbol:&ProviderSymbol,context:Id<attribution::AnalysisContext>)->Result<bool,ModelError> {
    if !pinned_fastmcp(data,symbol)? {return Ok(false);}
    let Some(class)=target.receiver_class else {return Ok(false)};let class=need(&data.symbols,class)?;
    if class.name!="FastMCP" || !matches!(module_name(data,class),Some("fastmcp"|"fastmcp.server.server")) || !pinned_fastmcp(data,class)? {return Ok(false);}
    let Receiver::Bound {actual}=need(&data.receivers,target.receiver)? else {return Ok(false)};
    let mut references=data.references.iter().filter(|r|r.read==*actual && data.qualifications.get(r.qualification).is_some_and(|q|q.context==context));
    let Some(reference)=references.next() else {return Ok(false)};if references.next().is_some() || !exact(data,reference.qualification,context)? {return Ok(false);}
    let mut assessments=data.reference_assessments.iter().filter(|r|r.reference==reference.id());let Some(assessment)=assessments.next() else {return Ok(false)};
    if assessments.next().is_some() || assessment.status!=ResolutionStatus::Resolved {return Ok(false);}
    let mut candidates=data.reference_candidates.iter().filter(|r|r.assessment==assessment.id());let Some(candidate)=candidates.next() else {return Ok(false)};if candidates.next().is_some() {return Ok(false);}
    if !exact(data,need(&data.lexical_resolutions,candidate.resolution)?.qualification,context)? {return Ok(false);}
    let ReferenceEntityTarget::Binding {event,..}=need(&data.reference_targets,candidate.target)? else {return Ok(false)};
    let mut bindings=data.bindings.iter().filter(|r|r.event==*event && data.qualifications.get(r.qualification).is_some_and(|q|q.context==context));let Some(binding)=bindings.next() else {return Ok(false)};
    if bindings.next().is_some() || binding.kind!=BindingEventKind::Assignment || binding.static_branch.is_some() || !exact(data,binding.qualification,context)? {return Ok(false);}
    let Some(value)=binding.value else {return Ok(false)};let mut found=false;
    for constructor in data.targets.iter() {
        if !same_span(data,constructor.site,value)? {continue;}
        let Some((constructed,_))=resolution(data,constructor,context)? else {return Ok(false)};
        if constructed.id()!=class.id() && constructor.receiver_class!=Some(class.id()) {return Ok(false);}
        found=true;
    }
    Ok(found)
}
fn property_receiver(data:&AspectData,target:&CallTarget)->Result<bool,ModelError> {
    let Some(class)=target.receiver_class else {return Ok(false)};let class=need(&data.symbols,class)?;
    Ok(class.kind==SymbolKind::Class && class.name=="property" && standard_library(data,class,"builtins"))
}
fn accessors(data:&AspectData,out:&mut AspectOutput,member:&EffectiveDecoratorMember,expression:Id<source::Occurrence>,context:Id<attribution::AnalysisContext>)->Result<(),ModelError> {
    if need(&data.occurrences,expression)?.syntax_kind!=source::SyntaxKind::ExprAttribute {return Ok(());}
    let decorator=need(&data.decorators,member.observation)?;
    let Some(current)=data.declarations.iter().find(|r|r.declaration==decorator.declaration && data.qualifications.get(r.qualification).is_some_and(|q|q.context==context)) else {return Ok(())};
    if !exact(data,current.qualification,context)? {return Ok(());}
    let Some(parent)=current.parent else {return Ok(())};
    for placement in data.placements.iter().filter(|r|r.parent==Some(expression)) {
        if !exact(data,placement.qualification,context)? || need(&data.occurrences,placement.occurrence)?.syntax_kind!=source::SyntaxKind::Identifier {continue;}
        for spelling in data.spellings.iter().filter(|r|r.occurrence==placement.occurrence) {
            if !exact(data,spelling.qualification,context)? {continue;}
            let kind=match spelling.spelling.as_str() {"getter"=>AspectKind::PropertyGetter,"setter"=>AspectKind::PropertySetter,"deleter"=>AspectKind::PropertyDeleter,_=>continue};
            for reference in data.references.iter().filter(|r|r.parent==expression && r.field==SyntaxField::Value) {
                if !exact(data,reference.qualification,context)? {continue;}
                for assessment in data.reference_assessments.iter().filter(|r|r.reference==reference.id()) {
                    for candidate in data.reference_candidates.iter().filter(|r|r.assessment==assessment.id()) {
                        let q=need(&data.qualifications,need(&data.lexical_resolutions,candidate.resolution)?.qualification)?;if q.context!=context || q.approximation!=assertion::Approximation::Exact {continue;}
                        let ReferenceEntityTarget::Binding {entity,..}=need(&data.reference_targets,candidate.target)? else {continue};
                        let EntityRef::Callable {callable}=need(&data.refs,*entity)? else {continue};
                        let Some(CallableEntity::Source {declaration,..})=data.callable_entities.get(*callable) else {continue};
                        for previous in data.declarations.iter().filter(|r|r.declaration==*declaration && r.parent==Some(parent)) {
                            if !exact(data,previous.qualification,context)? {continue;}
                            if need(&data.occurrences,previous.declaration)?.start>=need(&data.occurrences,current.declaration)?.start {continue;}
                            for effective in data.assessments.iter().filter(|r|r.callable==*callable && r.context==context) {
                                for evidence in data.evidence.iter().filter(|r|r.assessment==effective.id()) {
                                    let EffectiveCallablePremise::Traits {observation}=need(&data.premises,evidence.premise)? else {continue};let traits=need(&data.traits,*observation)?;
                                    if traits.property_getter || traits.property_setter {emit(out,member.assessment,AspectSource::Accessor {member:member.id(),candidate:candidate.id(),declaration:previous.id(),traits:*observation},kind,Some(*entity))?;}
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
pub fn normalize(data:&AspectData,budget:&ResourceBudget)->Result<AspectOutput,ModelError> {
    let mut out=AspectOutput::new(budget);
    for evidence in data.evidence.iter() {
        let EffectiveCallablePremise::Traits {observation}=need(&data.premises,evidence.premise)? else {continue};
        let traits=need(&data.traits,*observation)?;
        for (present,kind) in [(traits.property_getter,AspectKind::PropertyGetter),(traits.property_setter,AspectKind::PropertySetter)] {
            if present {emit(&mut out,evidence.assessment,AspectSource::Traits {evidence:evidence.id(),observation:*observation},kind,None)?;}
        }
    }
    for member in data.members.iter() {
        let assessment=need(&data.assessments,member.assessment)?;let decorator=need(&data.decorators,member.observation)?;let expression=root(data,decorator)?;let mut found=false;
        for target in data.targets.iter() {
            if !same_span(data,target.site,expression)? {continue;}
            if need(&data.qualifications,target.qualification)?.context!=assessment.context {continue;}
            let Some((symbol,resolved))=resolution(data,target,assessment.context)? else {emit(&mut out,member.assessment,AspectSource::Decorator {member:member.id(),target:Some(target.id()),resolution:None,argument:None},AspectKind::Unknown,None)?;found=true;continue;};
            let module=module_name(data,symbol);let mut argument=None;let mut linked=None;
            let kind=match (module,symbol.name.as_str()) {
                (Some("builtins"),"property") if standard_library(data,symbol,"builtins")=>AspectKind::PropertyGetter,
                (Some("builtins"),"getter") if property_receiver(data,target)?=>AspectKind::PropertyGetter,
                (Some("builtins"),"setter") if property_receiver(data,target)?=>AspectKind::PropertySetter,
                (Some("builtins"),"deleter") if property_receiver(data,target)?=>AspectKind::PropertyDeleter,
                (Some("functools"),"cached_property") if standard_library(data,symbol,"functools")=>AspectKind::CachedProperty,
                (Some("contextlib"),"contextmanager") if standard_library(data,symbol,"contextlib")=>AspectKind::ContextManager,
                (Some("contextlib"),"asynccontextmanager") if standard_library(data,symbol,"contextlib")=>AspectKind::AsyncContextManager,
                (Some("functools"),"wraps") if standard_library(data,symbol,"functools")=>{
                    for call in data.calls.iter().filter(|c|c.site==target.site) {for arg in data.arguments.iter().filter(|a|a.call==call.id() && a.ordinal==0 && a.kind==ArgumentKind::Positional) {argument=Some(arg.id());linked=related(data,arg,assessment.context)?;}}
                    AspectKind::Wraps
                }
                (Some(name),method) if name.starts_with("fastmcp.") && matches!(method,"tool"|"resource"|"prompt") && registration(data,target,symbol,assessment.context)?=>match method {"tool"=>AspectKind::FastMcpTool,"resource"=>AspectKind::FastMcpResource,_=>AspectKind::FastMcpPrompt},
                _=>AspectKind::Unknown,
            };
            emit(&mut out,member.assessment,AspectSource::Decorator {member:member.id(),target:Some(target.id()),resolution:Some(resolved.id()),argument},kind,linked)?;found=true;
        }
        accessors(data,&mut out,member,expression,assessment.context)?;
        if !found {emit(&mut out,member.assessment,AspectSource::Decorator {member:member.id(),target:None,resolution:None,argument:None},AspectKind::Unknown,None)?;}
    }
    for field in data.fields.iter() {
        let syntax=need(&data.field_syntax,field.declaration)?;let q=need(&data.qualifications,syntax.qualification)?;
        let mut default=syntax.value.map_or(FieldDefault::Absent {},|expression|FieldDefault::Expression {expression});
        if let Some(expression)=syntax.value {
            for observation in data.details.iter().filter(|r|r.occurrence==expression) {
                if exact(data,observation.qualification,q.context)? {if let SyntaxDetail::Literal {literal}=need(&data.detail_values,observation.detail)? {default=FieldDefault::Literal {observation:observation.id(),literal:*literal};}}
            }
            for target in data.targets.iter() {
                if !same_span(data,target.site,expression)? {continue;}
                let Some((symbol,resolved))=resolution(data,target,q.context)? else {continue};
                if !standard_library(data,symbol,"dataclasses") || symbol.name!="field" || symbol.kind!=SymbolKind::Function {continue;}
                for call in data.calls.iter().filter(|r|r.site==target.site) {
                    if !exact(data,call.qualification,q.context)? {continue;}
                    let mut factories=data.arguments.iter().filter(|r|r.call==call.id() && r.kind==ArgumentKind::Keyword && r.keyword.as_deref()==Some("default_factory"));let factory=factories.next();let single_factory=factories.next().is_none();
                    let mut explicits=data.arguments.iter().filter(|r|r.call==call.id() && r.kind==ArgumentKind::Keyword && r.keyword.as_deref()==Some("default"));let explicit=explicits.next();let single_explicit=explicits.next().is_none();
                    if let Some(argument)=factory.filter(|_|single_factory && explicit.is_none()) {default=FieldDefault::Factory {expression:argument.value,target:target.id(),resolution:resolved.id(),argument:argument.id()};}
                    else if let Some(argument)=explicit.filter(|_|single_explicit && factory.is_none()) {
                        let expression=argument.value;default=FieldDefault::Expression {expression};
                        for observation in data.details.iter().filter(|r|r.occurrence==expression) {if exact(data,observation.qualification,q.context)? {if let SyntaxDetail::Literal {literal}=need(&data.detail_values,observation.detail)? {default=FieldDefault::Literal {observation:observation.id(),literal:*literal};}}}
                    } else if factory.is_some() || explicit.is_some() {default=FieldDefault::Unknown {};}

                }
            }
        }
        let default=out.defaults.insert(default)?;out.fields.insert(FieldDefaultAssessment {declaration:field.id(),default})?;
    }
    Ok(out)
}
pub fn relations()->Vec<Relation> {macro_rules! declare {($($f:ident:$ty:ty,)*)=>{vec![$(Relation::of::<$ty>()),*]};}crate::callable_aspect_outputs!(declare)}
pub fn invariants()->Vec<Invariant> {let mut inputs=AspectData::inputs();inputs.extend(AspectOutput::inputs());vec![Invariant {name:"normalized_callable_aspects",inputs,create:std::sync::Arc::new(|budget|Box::new(Check {data:AspectData::new(budget),out:AspectOutput::new(budget),budget:budget.clone()}))}]}
struct Check {data:AspectData,out:AspectOutput,budget:ResourceBudget}
impl InvariantCheck for Check {fn visit(&mut self,name:&str,batch:&arrow_array::RecordBatch)->Result<(),ModelError> {if !self.data.visit(name,batch)? && !self.out.visit(name,batch)? {return Err(invalid("undeclared aspect validation input"));}Ok(())}fn finish(self:Box<Self>)->Result<(),ModelError> {self.out.matches(&normalize(&self.data,&self.budget)?)} }
pub fn stage(profile: stages::Profile) -> stages::Stage {
    let mut inputs = super::event_normalization::stage(profile).inputs;
    inputs.extend(AspectData::stage_inputs());
    macro_rules! previous {($($f:ident:$ty:ty,)*)=>{$(inputs.push(stages::RelationUse::stored::<$ty>());)*};}
    crate::normalized_callable_outputs!(previous);
    inputs.sort_by_key(|r| r.name());
    inputs.dedup_by_key(|r| r.name());
    stages::Stage {
        name: "normalize_callable_aspects",
        inputs: crate::domain::normalized::facts_stage_inputs(inputs),
        outputs: relations()
            .iter()
            .map(stages::RelationUse::of_relation)
            .collect(),
        contributes: vec![],
        coverage: vec![],
        provider: None,
        profiles: vec![profile],
        effect: stages::Effect::Pure,
        code: ContentHash::of(include_bytes!("callable_aspects.rs")),
        configuration: ContentHash::of(b"metadata-only/v1"),
    }
}
