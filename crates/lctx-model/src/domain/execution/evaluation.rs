//! Bounded closed-expression rules over captured, attributed syntax. A normal evaluation is
//! conditional on entry to the expression; it never proves its owner reaches the expression.
use crate::domain::{*, analysis::{native::{NativeAssertionPremise, NativeQualification}}, assertion::*, attribution::*, lexical::SyntaxField, normalized::{Rows, entities::*}, obligation::ObligationKind, resources::ResourceBudget, source::*, syntax::*, value::Literal};
use std::collections::BTreeSet;
use crate::DomainCode;

macro_rules! inputs {($($field:ident:$ty:ty,)*)=>{
    pub struct EvaluationData {$(pub $field:Rows<$ty>,)*}
    impl EvaluationData {
        pub fn new(budget:&ResourceBudget)->Self {Self{$($field:Rows::new(budget),)*}}
        pub fn visit(&mut self,name:&str,batch:&arrow_array::RecordBatch)->Result<bool,ModelError> {$(if name==<$ty>::NAME {self.$field.decode(batch)?;return Ok(true);})* Ok(false)}
        pub fn validation_inputs()->Vec<ValidationInput> {vec![$(ValidationInput::of::<$ty>(&["id"]),)*].into_iter().map(|input|if stages::is_vocabulary(input.name()){input.at_epoch(stages::PublicationBoundary::Facts)}else{input}).collect()}
    }
};}
#[macro_export]
macro_rules! execution_evaluation_inputs {($apply:ident)=>{$apply! {
    occurrences:$crate::domain::source::Occurrence,
    artifacts:$crate::domain::source::SourceArtifact,
    uses:$crate::domain::input::ArtifactUse,
    modules:$crate::domain::source::Module,
    scopes:$crate::domain::source::CoverageScope,
    qualifications:$crate::domain::assertion::AssertionQualification,
    placements:$crate::domain::syntax::SyntaxPlacement,
    declarations:$crate::domain::syntax::DeclarationObservation,
    details:$crate::domain::syntax::SyntaxDetailObservation,
    detail_values:$crate::domain::syntax::SyntaxDetail,
    literals:$crate::domain::value::Literal,
    spellings:$crate::domain::source::SyntaxObservation,
    owners:$crate::domain::normalized::entities::OccurrenceOwnership,
    refs:$crate::domain::normalized::entities::EntityRef,
    callables:$crate::domain::normalized::entities::CallableEntity,
    runs:$crate::domain::attribution::ProviderRun,
    premises:$crate::domain::analysis::native::NativeAssertionPremise,
    native:$crate::domain::analysis::native::NativeQualification,
}};}
crate::execution_evaluation_inputs!(inputs);

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct ExpressionRequest {
    pub input:Id<input::InputRevision>, pub context:Id<AnalysisContext>,
    pub owner:Id<EntityRef>, pub expression:Id<Occurrence>,
}

/// Frame disposal is independent of value flow. CallerRetained is an unresolved obligation
/// until the exact source-call frame proves the relevant outside reference survives the call.
#[derive(Debug,Clone,Copy,PartialEq,Eq,DomainCode)]
#[repr(i16)]
pub enum ReleaseSafety { Closed=0, CallerRetained=1, Unknown=2 }

#[derive(Debug,Clone,Copy)]
enum Value { None, Bool(bool), Int(i64), Float(f64), Literal, Tuple{nonempty:bool,retained:bool}, Retained, OpaqueClosed }
impl Value {
    fn truth(self)->Option<bool> {match self {Self::None=>Some(false),Self::Bool(b)=>Some(b),Self::Int(i)=>Some(i!=0),Self::Float(f)=>Some(f!=0.0),Self::Tuple{nonempty,..}=>Some(nonempty),Self::Literal|Self::Retained|Self::OpaqueClosed=>None}}
    fn number(self)->Option<Self> {match self {Self::Bool(b)=>Some(Self::Int(i64::from(b))),Self::Int(_)|Self::Float(_)=>Some(self),_=>None}}
    fn release(self)->ReleaseSafety {if matches!(self,Self::Retained|Self::Tuple{retained:true,..}){ReleaseSafety::CallerRetained}else{ReleaseSafety::Closed}}
}

/// Only the shared evaluator constructs this token. Records/replay must preserve the request
/// and exact ordered evidence; consumers cannot declare arbitrary operands normal.
pub struct CheckedEvaluation {
    request:ExpressionRequest, value:Value, release:ReleaseSafety,
    call_source:Option<Id<super::source_call_records::SourceInvocation>>, exception:Option<(Id<Occurrence>,super::ExactRuntimeException)>,
    native:Vec<Id<NativeAssertionPremise>>, operands:Vec<Id<Occurrence>>,
    entries:Vec<Id<conditions::entry::EntryValueWitness>>, _entry_charges:Vec<std::sync::Arc<charged::StateCharge>>,
    qualification:Id<AssertionQualification>, status:analysis::policy::EvidenceStatus,
    _charge:charged::StateCharge,
}
impl CheckedEvaluation {
    pub fn request(&self)->ExpressionRequest {self.request}
    pub fn truth(&self)->Option<bool> {self.value.truth()}
    pub fn release(&self)->ReleaseSafety {self.release}
    pub fn native_premises(&self)->&[Id<NativeAssertionPremise>] {&self.native}
    pub fn evaluated_operands(&self)->&[Id<Occurrence>] {&self.operands}
    pub fn entry_premises(&self)->&[Id<conditions::entry::EntryValueWitness>] {&self.entries}
    pub fn qualification(&self)->Id<AssertionQualification> {self.qualification}
    pub fn status(&self)->analysis::policy::EvidenceStatus {self.status}
    pub(crate) fn call_source(&self)->Option<Id<super::source_call_records::SourceInvocation>>{self.call_source}
    pub(crate) fn exception(&self)->Option<(Id<Occurrence>,super::ExactRuntimeException)>{self.exception}
}

pub const EXPRESSION_DEPTH_LIMIT:usize=64;
pub const EXPRESSION_WORK_LIMIT:usize=1024;
const UNSUPPORTED:ObligationKind=ObligationKind::UnsupportedControlFlow;
struct PreparedSyntax {
    placements:charged::ChargedMap<Id<Occurrence>,Id<SyntaxPlacement>>,
    children:charged::ChargedMap<Id<Occurrence>,Vec<Id<SyntaxPlacement>>>,
    details:charged::ChargedMap<Id<Occurrence>,Vec<Id<SyntaxDetailObservation>>>,
    native:charged::ChargedMap<derivation::RowRef,Vec<Id<NativeQualification>>>,
    ownership:charged::ChargedMap<Id<Occurrence>,Vec<Id<EntityRef>>>,
    declarations:charged::ChargedMap<Id<Occurrence>,Vec<Id<syntax::DeclarationObservation>>>,
    lazy_owners:charged::ChargedSet<Id<EntityRef>>,
    _charge:charged::StateCharge,
}
impl PreparedSyntax {
    fn new(data:&EvaluationData,context:Id<AnalysisContext>,budget:&ResourceBudget)->Result<Self,ModelError> {
        let mut index=Self {placements:Default::default(),children:Default::default(),details:Default::default(),native:Default::default(),ownership:Default::default(),declarations:Default::default(),lazy_owners:Default::default(),_charge:charged::StateCharge::new(budget,"execution-syntax-index")};
        for row in data.placements.iter() {
            if data.qualifications.get(row.qualification).is_none_or(|q|q.context!=context) {continue;}
            if index.placements.insert(&mut index._charge,row.occurrence,row.id())?.is_some_and(|old|old!=row.id()) {return Err(ModelError::Invalid("execution syntax has ambiguous placement".into()));}
            if let Some(parent)=row.parent {index.children.update(&mut index._charge,parent,|rows|rows.push(row.id()))?;}
        }
        for row in data.details.iter() {if data.qualifications.get(row.qualification).is_some_and(|q|q.context==context) {index.details.update(&mut index._charge,row.occurrence,|rows|rows.push(row.id()))?;}}
        for row in data.native.iter() {let premise=data.premises.get(row.premise).ok_or_else(||ModelError::Invalid("execution native pair missing".into()))?;index.native.update(&mut index._charge,premise.assertion_and_support().0,|rows|rows.push(row.id()))?;}
        for row in data.owners.iter() {index.ownership.update(&mut index._charge,row.occurrence,|rows|rows.push(row.entity))?;}
        for row in data.declarations.iter(){if data.qualifications.get(row.qualification).is_some_and(|q|q.context==context){index.declarations.update(&mut index._charge,row.declaration,|rows|rows.push(row.id()))?;}}
        for row in data.occurrences.iter().filter(|row|matches!(row.syntax_kind,SyntaxKind::ExprYield|SyntaxKind::ExprYieldFrom)){for owner in index.ownership.get(&row.id()).into_iter().flatten(){index.lazy_owners.insert(&mut index._charge,*owner)?;}}
        Ok(index)
    }
}
pub(crate) struct Evaluator<'a> {
    index:&'a PreparedSyntax,
    data:&'a EvaluationData, request:ExpressionRequest, remaining:usize,
    available_entries:&'a [&'a conditions::entry::DerivedEntryValue], entries:Vec<Id<conditions::entry::EntryValueWitness>>, entry_charges:Vec<std::sync::Arc<charged::StateCharge>>,
    native:Vec<Id<NativeAssertionPremise>>, operands:Vec<Id<Occurrence>>,
    charge:charged::StateCharge, work_reason:ObligationKind, status:analysis::policy::EvidenceStatus,
}
fn need<R:Record>(rows:&Rows<R>,id:Id<R>)->Result<&R,ObligationKind> {rows.get(id).ok_or(ObligationKind::MissingEvidence)}
impl Evaluator<'_> {
    pub(crate) fn tick(&mut self,n:usize)->Result<(),ObligationKind> {self.remaining=self.remaining.checked_sub(n).ok_or(self.work_reason)?;Ok(())}
    pub(crate) fn observe(&mut self,id:Id<Occurrence>)->Result<(),EvaluationError> {
        self.tick(1).map_err(boundary)?;
        if !self.index.ownership.get(&id).is_some_and(|owners|owners.contains(&self.request.owner)) {return Err(boundary(ObligationKind::ScopeBoundary));}
        let placement_id=*self.index.placements.get(&id).ok_or_else(||boundary(ObligationKind::MissingEvidence))?;
        let placement=need(&self.data.placements,placement_id).map_err(boundary)?;
        self.support(placement,placement.qualification,id)
    }
    pub(crate) fn status(&self)->analysis::policy::EvidenceStatus {self.status}
    pub(crate) fn qualification(&self,id:Id<Occurrence>)->Result<Id<AssertionQualification>,ObligationKind> {let placement=need(&self.data.placements,*self.index.placements.get(&id).ok_or(ObligationKind::MissingEvidence)?)?;Ok(placement.qualification)}
    /// A source callable's declaration is lexically owned by its enclosing scope. The
    /// normalized callable anchor, rather than lexical containment, binds this body operator.
    pub(crate) fn observe_callable(&mut self,id:Id<Occurrence>)->Result<(),EvaluationError> {
        self.tick(1).map_err(boundary)?;
        let entity=need(&self.data.refs,self.request.owner).map_err(boundary)?;
        let EntityRef::Callable{callable}=entity else{return Err(boundary(ObligationKind::ScopeBoundary));};
        if !matches!(need(&self.data.callables,*callable).map_err(boundary)?,CallableEntity::Source{declaration,kind:CallableKind::Function} if *declaration==id){return Err(boundary(ObligationKind::MissingEvidence));}
        let declarations=self.index.declarations.get(&id).ok_or_else(||boundary(ObligationKind::MissingEvidence))?;self.tick(declarations.len()).map_err(boundary)?;
        if declarations.len()!=1{return Err(boundary(ObligationKind::MissingEvidence));}let declaration=need(&self.data.declarations,declarations[0]).map_err(boundary)?;
        if declaration.kind!=syntax::DeclarationKind::Function||self.index.lazy_owners.contains(&self.request.owner){return Err(boundary(ObligationKind::ScopeBoundary));}
        self.support(declaration,declaration.qualification,id)?;
        let placement_id=*self.index.placements.get(&id).ok_or_else(||boundary(ObligationKind::MissingEvidence))?;
        let placement=need(&self.data.placements,placement_id).map_err(boundary)?;
        self.support(placement,placement.qualification,id)
    }
    pub(crate) fn take_admission(&mut self)->(Vec<Id<NativeAssertionPremise>>,charged::StateCharge) {(std::mem::take(&mut self.native),std::mem::take(&mut self.charge))}
    fn frame(&self,q:Id<AssertionQualification>,occurrence:Id<Occurrence>)->Result<(),ObligationKind> {
        let q=need(&self.data.qualifications,q)?;
        let source=need(&self.data.occurrences,occurrence)?.source;
        let artifact=need(&self.data.artifacts,source)?;
        if artifact.input!=self.request.input || q.context!=self.request.context {return Err(ObligationKind::IncompatibleContexts);}
        if q.modality!=Modality::Definite || q.approximation!=Approximation::Exact || q.condition!=conditions::Diagram::always().id() {return Err(ObligationKind::Approximation);}
        let covered=match need(&self.data.scopes,q.scope)? {CoverageScope::Input{input}=>*input==self.request.input,CoverageScope::Artifact{artifact}=>*artifact==source,CoverageScope::Module{module}=>need(&self.data.modules,*module)?.source==source,_=>false};
        if !covered {return Err(ObligationKind::MissingEvidence);}Ok(())
    }
    fn support<R:Record>(&mut self,row:&R,q:Id<AssertionQualification>,occurrence:Id<Occurrence>)->Result<(),EvaluationError> {
        self.frame(q,occurrence).map_err(boundary)?;
        let mut any=false;
        self.tick(self.index.native.get(&derivation::RowRef::of(row.id())).map_or(0,Vec::len)).map_err(boundary)?;
        for native_id in self.index.native.get(&derivation::RowRef::of(row.id())).into_iter().flatten() {
            let native=need(&self.data.native,*native_id).map_err(boundary)?;
            if native.qualification!=q || analysis::policy::behavioral_support(native.status,false).is_err() {return Err(boundary(ObligationKind::MissingEvidence));}
            if !self.native.contains(&native.premise) {if self.native.len()==64 {return Err(boundary(ObligationKind::SummaryProofLimit));}self.charge.grow(size_of::<Id<NativeAssertionPremise>>() * 2)?;self.native.push(native.premise);self.status=analysis::support::inferred_status(analysis::Interpretation::Structural,[self.status,native.status]);}
            any=true;
        }
        if !any {return Err(boundary(ObligationKind::MissingEvidence));}Ok(())
    }
    pub(crate) fn children(&mut self,parent:Id<Occurrence>)->Result<Vec<SyntaxPlacement>,EvaluationError> {
        let parent_row=need(&self.data.occurrences,parent).map_err(boundary)?;
        let mut children=Vec::new();
        self.tick(self.index.children.get(&parent).map_or(0,Vec::len)).map_err(boundary)?;
        for placement_id in self.index.children.get(&parent).into_iter().flatten() {
            let placement=need(&self.data.placements,*placement_id).map_err(boundary)?;
            let q=need(&self.data.qualifications,placement.qualification).map_err(boundary)?;
            if q.context!=self.request.context {continue;}
            let child=need(&self.data.occurrences,placement.occurrence).map_err(boundary)?;
            if child.source!=parent_row.source || child.start<parent_row.start || child.end>parent_row.end {return Err(boundary(ObligationKind::MissingEvidence));}
            self.support(placement,placement.qualification,placement.occurrence)?;
            self.charge.grow(size_of::<SyntaxPlacement>() * 2 + 128)?;
            children.push(placement.clone());
        }
        children.sort_by_key(|row|(row.field as i16,row.ordinal,row.occurrence));
        let mut seen=BTreeSet::new();
        for child in &children {if !seen.insert((child.field as i16,child.ordinal)){return Err(boundary(ObligationKind::MissingEvidence));}}
        self.tick(children.len()).map_err(boundary)?;
        Ok(children)
    }
    fn detail(&mut self,occurrence:Id<Occurrence>)->Result<Option<SyntaxDetail>,EvaluationError> {
        let mut found=None;
        self.tick(self.index.details.get(&occurrence).map_or(0,Vec::len)).map_err(boundary)?;
        for row_id in self.index.details.get(&occurrence).into_iter().flatten() {
            let row=need(&self.data.details,*row_id).map_err(boundary)?;
            if row.ordinal!=0 || found.is_some() {return Err(boundary(UNSUPPORTED));}
            self.support(row,row.qualification,occurrence)?;
            found=Some(need(&self.data.detail_values,row.detail).map_err(boundary)?.clone());
        }
        Ok(found)
    }
    fn eval(&mut self,id:Id<Occurrence>,depth:usize)->Result<Value,EvaluationError> {
        self.tick(1).map_err(boundary)?;
        if depth>EXPRESSION_DEPTH_LIMIT {return Err(boundary(ObligationKind::ExpressionDepthLimit));}
        let occurrence=need(&self.data.occurrences,id).map_err(boundary)?;
        if !self.index.ownership.get(&id).is_some_and(|owners|owners.contains(&self.request.owner)) {return Err(boundary(ObligationKind::ScopeBoundary));}
        let placement_id=*self.index.placements.get(&id).ok_or_else(||boundary(ObligationKind::MissingEvidence))?;
        let placement=need(&self.data.placements,placement_id).map_err(boundary)?;
        self.support(placement,placement.qualification,id)?;
        let children=self.children(id)?;
        let detail=self.detail(id)?;
        let one=|field|->Result<Id<Occurrence>,EvaluationError> {let mut matching=children.iter().filter(|row|row.field==field);let child=matching.next().ok_or_else(||boundary(UNSUPPORTED))?;if matching.next().is_some(){return Err(boundary(UNSUPPORTED));}Ok(child.occurrence)};
        let value=match occurrence.syntax_kind {
            SyntaxKind::ExprNoneLiteral if children.is_empty()=>Value::None,
            SyntaxKind::ExprBooleanLiteral|SyntaxKind::ExprNumberLiteral|SyntaxKind::ExprStringLiteral|SyntaxKind::ExprBytesLiteral=>{
                let Some(SyntaxDetail::Literal{literal})=detail else{
                    if occurrence.syntax_kind==SyntaxKind::ExprNumberLiteral && children.is_empty(){self.charge.grow(size_of::<Id<Occurrence>>() * 2)?;self.operands.push(id);return Ok(Value::Literal);}
                    return Err(boundary(UNSUPPORTED));
                };
                match need(&self.data.literals,literal).map_err(boundary)? {Literal::None=>Value::None,Literal::Bool{value}=>Value::Bool(*value),Literal::Integer{decimal}=>decimal.parse::<i64>().map(Value::Int).unwrap_or(Value::Literal),Literal::Float{bits}=>Value::Float(f64::from_bits(*bits as u64)),Literal::String{..}|Literal::Bytes{..}=>Value::Literal}
            }
            SyntaxKind::ExprEllipsisLiteral if children.is_empty()=>Value::Literal,
            SyntaxKind::ExprAttribute=>return Err(boundary(ObligationKind::HeapFieldStateUnavailable)),
            SyntaxKind::ExprName if children.is_empty()=>{
                self.tick(self.available_entries.len()).map_err(boundary)?;
                let mut entries=self.available_entries.iter().filter(|entry|entry.witness().access==id);
                let entry=*entries.next().ok_or_else(||boundary(UNSUPPORTED))?;
                if entries.next().is_some() {return Err(boundary(ObligationKind::MissingEvidence));}
                if !matches!(entry.source(),conditions::entry::EntryAccessSource::Use{..}) {return Err(boundary(ObligationKind::EntryValueUnknown));}
                analysis::policy::behavioral_support(entry.evidence_status(),false).map_err(|_|boundary(ObligationKind::MissingEvidence))?;
                self.status=analysis::support::inferred_status(analysis::Interpretation::Structural,[self.status,entry.evidence_status()]);
                let witness=entry.witness();
                if witness.owner!=self.request.owner || witness.context!=self.request.context || need(&self.data.runs,witness.run).map_err(boundary)?.input!=self.request.input {return Err(boundary(ObligationKind::IncompatibleContexts));}
                self.charge.grow(size_of::<Id<conditions::entry::EntryValueWitness>>() * 2+size_of::<std::sync::Arc<charged::StateCharge>>() * 2)?;
                self.entries.push(witness.id());self.entry_charges.push(entry.allowance());
                Value::Retained
            },
            SyntaxKind::ExprUnaryOp if children.len()==1=>{
                let value=self.eval(one(SyntaxField::Operand)?,depth+1)?;
                match detail {Some(SyntaxDetail::Operator{operator:OperatorKind::Not})=>Value::Bool(!value.truth().ok_or_else(||boundary(UNSUPPORTED))?),Some(SyntaxDetail::Operator{operator:OperatorKind::UAdd})=>value.number().ok_or_else(||boundary(UNSUPPORTED))?,Some(SyntaxDetail::Operator{operator:OperatorKind::USub})=>match value.number(){Some(Value::Int(i))=>Value::Int(i.checked_neg().ok_or_else(||boundary(UNSUPPORTED))?),Some(Value::Float(f))=>Value::Float(-f),_=>return Err(boundary(UNSUPPORTED))},_=>return Err(boundary(UNSUPPORTED))}
            }
            SyntaxKind::ExprBinOp if children.len()==2=>{
                let Some(SyntaxDetail::Operator{operator})=detail else{return Err(boundary(UNSUPPORTED));};
                if !matches!(operator,OperatorKind::Add|OperatorKind::Sub){return Err(boundary(UNSUPPORTED));}
                let left=self.eval(one(SyntaxField::Left)?,depth+1)?.number().ok_or_else(||boundary(UNSUPPORTED))?;
                let right=self.eval(one(SyntaxField::Right)?,depth+1)?.number().ok_or_else(||boundary(UNSUPPORTED))?;
                match (left,right) {(Value::Int(a),Value::Int(b))=>Value::Int(if operator==OperatorKind::Add {a.checked_add(b)}else{a.checked_sub(b)}.ok_or_else(||boundary(UNSUPPORTED))?), (a,b)=>{let number=|value|match value {Value::Int(i)=>i as f64,Value::Float(f)=>f,_=>unreachable!()};Value::Float(if operator==OperatorKind::Add{number(a)+number(b)}else{number(a)-number(b)})}}
            }
            SyntaxKind::ExprTuple=>{
                let mut retained=false;
                for (ordinal,child) in children.iter().enumerate(){if child.field!=SyntaxField::Element || child.ordinal!=ordinal as i64{return Err(boundary(UNSUPPORTED));}retained|=self.eval(child.occurrence,depth+1)?.release()==ReleaseSafety::CallerRetained;}
                Value::Tuple{nonempty:!children.is_empty(),retained}
            }
            SyntaxKind::ExprBoolOp if children.len()>=2=>{
                let Some(SyntaxDetail::Operator{operator})=detail else{return Err(boundary(UNSUPPORTED));};
                if !matches!(operator,OperatorKind::And|OperatorKind::Or){return Err(boundary(UNSUPPORTED));}
                for (ordinal,child) in children.iter().enumerate(){if child.field!=SyntaxField::Operand || child.ordinal!=ordinal as i64{return Err(boundary(UNSUPPORTED));}}
                let mut value=self.eval(children[0].occurrence,depth+1)?;
                for child in children.iter().skip(1){let truth=value.truth().ok_or_else(||boundary(UNSUPPORTED))?;if operator==OperatorKind::And && !truth || operator==OperatorKind::Or && truth {break;}value=self.eval(child.occurrence,depth+1)?;}
                value
            }
            SyntaxKind::ExprIf if children.len()==3=>{let truth=self.eval(one(SyntaxField::Test)?,depth+1)?.truth().ok_or_else(||boundary(UNSUPPORTED))?;self.eval(one(if truth{SyntaxField::Value}else{SyntaxField::Orelse})?,depth+1)?}
            _=>return Err(boundary(UNSUPPORTED)),
        };
        self.charge.grow(size_of::<Id<Occurrence>>() * 2)?;self.operands.push(id);
        Ok(value)
    }
}
pub(crate) enum EvaluationError { Boundary(ObligationKind), Model(ModelError) }
impl From<ModelError> for EvaluationError {fn from(error:ModelError)->Self {Self::Model(error)}}
pub(crate) fn boundary(reason:ObligationKind)->EvaluationError {EvaluationError::Boundary(reason)}

/// Pure closed-expression admission. Refusals retain a typed semantic reason; resource refusal
/// remains a ModelError so a producer cannot publish an empty successful result after it.
pub fn evaluate(data:&EvaluationData,request:ExpressionRequest,budget:&ResourceBudget)->Result<Result<CheckedEvaluation,ObligationKind>,ModelError> {
    evaluate_with_entries(data,request,&[],budget)
}
pub fn evaluate_with_entries(data:&EvaluationData,request:ExpressionRequest,entries:&[&conditions::entry::DerivedEntryValue],budget:&ResourceBudget)->Result<Result<CheckedEvaluation,ObligationKind>,ModelError> {
    let prepared=PreparedExecution::new(data,request.input,request.context,budget)?;prepared.evaluate(request,entries)
}

pub(crate) fn with_completion_syntax<T>(data:&EvaluationData,request:ExpressionRequest,budget:&ResourceBudget,visit:impl FnOnce(&mut Evaluator<'_>)->Result<T,EvaluationError>)->Result<Result<T,ObligationKind>,ModelError> {
    let index=PreparedSyntax::new(data,request.context,budget)?;
    let mut syntax=Evaluator {index:&index,data,request,remaining:super::completion::COMPLETION_WORK_LIMIT,native:Vec::new(),operands:Vec::new(),available_entries:&[],entries:Vec::new(),entry_charges:Vec::new(),charge:charged::StateCharge::new(budget,"base-completion-syntax"),work_reason:ObligationKind::CompletionWorkLimit,status:analysis::policy::EvidenceStatus::StructurallyObserved};
    match visit(&mut syntax) {Ok(value)=>Ok(Ok(value)),Err(EvaluationError::Boundary(reason))=>Ok(Err(reason)),Err(EvaluationError::Model(error))=>Err(error)}
}





/// Source name reads require the exact shared, privately derived entry proof. Merely naming a
/// parameter or observing a value transfer does not establish availability or harmless disposal.
/// Reuse the admitted syntax index for one captured frame. Result tokens retain their own
/// charged proof state; the preparation keeps its index charged for the full producer lifetime.
pub struct PreparedExecution<'a> {data:&'a EvaluationData,input:Id<input::InputRevision>,context:Id<AnalysisContext>,index:PreparedSyntax,budget:ResourceBudget}
impl<'a> PreparedExecution<'a> {
    pub fn new(data:&'a EvaluationData,input:Id<input::InputRevision>,context:Id<AnalysisContext>,budget:&ResourceBudget)->Result<Self,ModelError> {Ok(Self{data,input,context,index:PreparedSyntax::new(data,context,budget)?,budget:budget.clone()})}
    pub fn evaluate(&self,request:ExpressionRequest,entries:&[&conditions::entry::DerivedEntryValue])->Result<Result<CheckedEvaluation,ObligationKind>,ModelError> {
        if (request.input,request.context)!=(self.input,self.context){return Ok(Err(ObligationKind::IncompatibleContexts));}
        evaluate_prepared(self,request,entries)
    }
}
fn evaluate_prepared(prepared:&PreparedExecution<'_>,request:ExpressionRequest,entries:&[&conditions::entry::DerivedEntryValue])->Result<Result<CheckedEvaluation,ObligationKind>,ModelError> {
    let data=prepared.data;let budget=&prepared.budget;
    let mut evaluator=Evaluator {index:&prepared.index,data,request,remaining:EXPRESSION_WORK_LIMIT,native:Vec::new(),operands:Vec::new(),available_entries:entries,entries:Vec::new(),entry_charges:Vec::new(),charge:charged::StateCharge::new(budget,"closed-expression"),work_reason:ObligationKind::ExpressionWorkLimit,status:analysis::policy::EvidenceStatus::StructurallyObserved};
    match evaluator.eval(request.expression,0) {
        Ok(value)=>{
            let placement=data.placements.get(*prepared.index.placements.get(&request.expression).expect("evaluated placement")).expect("indexed placement");
            let status=evaluator.status;
            Ok(Ok(CheckedEvaluation {request,value,release:value.release(),call_source:None,exception:None,qualification:placement.qualification,status,native:evaluator.native,operands:evaluator.operands,entries:evaluator.entries,_entry_charges:evaluator.entry_charges,_charge:evaluator.charge}))
        },
        Err(EvaluationError::Boundary(reason))=>Ok(Err(reason)),
        Err(EvaluationError::Model(error))=>Err(error),
    }
}

/// Only the actual SourceCall replay callback invokes this lowering. A normal callee outcome
/// proves completion of the call under entry; it gives no truth, returned shape or caller reach.
pub(crate) fn source_call_evaluation(data:&EvaluationData,request:ExpressionRequest,proof:&super::source_invocation::CheckedSourceInvocation,source:Id<super::source_call_records::SourceInvocation>,budget:&ResourceBudget)->Result<Result<CheckedEvaluation,ObligationKind>,ModelError>{
 with_completion_syntax(data,request,budget,|syntax|{
  syntax.observe(request.expression)?;
  if data.occurrences.get(request.expression).is_none_or(|row|row.syntax_kind!=SyntaxKind::ExprCall){return Err(boundary(ObligationKind::MissingEvidence));}
  let qualification=syntax.qualification(request.expression).map_err(boundary)?;
  if data.qualifications.get(qualification)!=data.qualifications.get(proof.qualification()){return Err(boundary(ObligationKind::IncompatibleContexts));}
  let exception=match proof.outcome(){super::source_invocation::InvocationOutcome::Normal=>None,super::source_invocation::InvocationOutcome::Raised{site,exception}=>Some((site,exception))};
  let status=analysis::support::inferred_status(analysis::Interpretation::Structural,[syntax.status(),proof.status()]);
  let(native,charge)=syntax.take_admission();
  Ok(CheckedEvaluation{request,value:Value::OpaqueClosed,release:ReleaseSafety::Closed,call_source:Some(source),exception,native,operands:Vec::new(),entries:Vec::new(),_entry_charges:Vec::new(),qualification,status,_charge:charge})
 })
}
