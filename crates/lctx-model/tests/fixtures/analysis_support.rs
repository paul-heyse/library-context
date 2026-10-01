//! Native observation plus an actual analysis invocation; never attribute analysis to a provider.
use lctx_model::domain::{analysis::*,analysis::support::*,analysis::obligations::*,assertion::*,attribution::*,conditions::Diagram,flow::*,input::InputRevision,source::Occurrence,transfer::TransferKind,value::Place,*};

pub struct SupportFixture {
    pub parameters: MethodParameters,
    pub definition: AnalysisDefinition,
    pub invocation: AnalysisInvocation,
    pub use_: FlowUse,
    pub observation: FlowValueObservation,
    pub support: FlowValueSupport,
    pub premise: NativeAssertionPremise,
    pub native: SupportSource,
    pub subject:ObligationSubject,
    pub proposition:AnalysisProposition,
    pub derivation: AnalysisDerivation,
    pub members: Vec<AnalysisDerivationPremise>,
    pub derived: SupportSource,
}
impl SupportFixture {
    pub fn new(input:Id<InputRevision>,run:Id<ProviderRun>,surface:Id<ProviderSurface>,evidence:Id<Evidence>,qualification:&AssertionQualification,condition:&Diagram,at:Id<Occurrence>,place:Id<Place>)->Self {
        let parameters=MethodParameters {depth:None,proof_steps:None,work:None,members:None,seed:None,iterations:None,threshold:None,resolution:None,damping:None,model_catalog:None};
        let definition=AnalysisDefinition { method:AnalysisMethod::LocalTransfers,semantic_version:ContentHash::of(b"support fixture"),parameters:parameters.id(),interpretation:Interpretation::Structural };
        let (invocation,_)=AnalysisInvocation::new(input,qualification.context,definition.id(),None,[]);
        let use_=FlowUse {occurrence:at,place};
        let observation=FlowValueObservation {qualification:qualification.id(),use_:use_.id(),sink:at,kind:FlowSinkKind::Definition,transfer:TransferKind::Derived,through_call:false};
        let support=FlowValueSupport { assertion:observation.id(),run,surface,evidence,origin:Origin::AnalyzerAssertion,mode:ExtractionMode::NativeTraversal,fidelity:Fidelity::NativeStructural };
        let premise=NativeAssertionPremise::Value {assertion:observation.id(),support:support.id()};
        let native=SupportSource::NativeAssertion {premise:premise.id()};
        let subject=ObligationSubject::SourceCall {occurrence:at};
        let (derivation,proposition,members,_)=AnalysisDerivation::emit(invocation.id(),subject.id(),AnalysisChannel::Value,calls::CallPhase::Call,QualificationOperation::Conjunction,&[QualifiedPremise {source:&native,qualification,condition}]).unwrap();
        let derived=SupportSource::AnalysisDerivation {derivation:derivation.id()};
        Self {parameters,definition,invocation,use_,observation,support,premise,native,subject,proposition,derivation,members,derived}
    }
}
