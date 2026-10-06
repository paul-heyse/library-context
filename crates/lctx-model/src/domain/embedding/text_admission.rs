//! Independent text membership and original-byte/value admission. Original rich inputs are
//! projected to compact identities and byte receipts; output payloads live only in one window
//! group. This check never calls the producer or its full-output diagnostic replay.
use super::*;
use crate::domain::{*,charged::{ChargedMap,ChargedSet,StateCharge},normalized::{Rows,entities::*},source::*,syntax::*,artifact::*,input::*,documents::*,assertion::*,attribution::AnalysisContext};
use resources::ResourceBudget;
fn invalid(message:&str)->ModelError {ModelError::Invalid(message.into())}
#[derive(Clone,Copy)]
struct Bytes {len:usize,digest:ContentHash}
impl HeapSize for Bytes {fn heap_bytes(&self)->usize {0}}
impl Bytes {fn of(value:&[u8])->Self {Self {len:value.len(),digest:ContentHash::of(value)}}}
#[derive(Clone,Copy)]
struct Artifact {input:Id<InputRevision>,len:i64,class:Option<crate::domain::admission::ArtifactClass>}
impl HeapSize for Artifact {fn heap_bytes(&self)->usize {0}}
#[derive(Clone,Copy)]
struct Span {source:Id<SourceArtifact>,start:i64,end:i64,kind:SyntaxKind}
impl HeapSize for Span {fn heap_bytes(&self)->usize {0}}
#[derive(Clone,Copy)]
struct Passage {qualification:Id<AssertionQualification>,node:DocumentNodePassageId,bytes:Bytes}
impl HeapSize for Passage {fn heap_bytes(&self)->usize {0}}
#[derive(Clone,Copy)]
struct Literal {full:Bytes,trimmed:Bytes}
impl HeapSize for Literal {fn heap_bytes(&self)->usize {0}}
#[derive(Clone,Copy)]
struct SpanReceipt {bytes:Bytes,utf8:bool}
impl HeapSize for SpanReceipt {fn heap_bytes(&self)->usize {0}}
#[derive(Default)]
struct Utf8 {remaining:u8,low:u8,high:u8,invalid:bool}
impl Utf8 {
    fn update(&mut self,bytes:&[u8]) {
        for byte in bytes.iter().copied() {
            if self.remaining>0 {
                if byte<self.low || byte>self.high {self.invalid=true;}
                self.remaining-=1;self.low=0x80;self.high=0xbf;
            } else {
                let (remaining,low,high)=match byte {
                    0..=0x7f=>(0,0x80,0xbf),0xc2..=0xdf=>(1,0x80,0xbf),0xe0=>(2,0xa0,0xbf),0xe1..=0xec|0xee..=0xef=>(2,0x80,0xbf),0xed=>(2,0x80,0x9f),0xf0=>(3,0x90,0xbf),0xf1..=0xf3=>(3,0x80,0xbf),0xf4=>(3,0x80,0x8f),_=>(0,0,0),
                };
                if byte>=0x80 && remaining==0 {self.invalid=true;}
                self.remaining=remaining;self.low=low;self.high=high;
            }
        }
    }
    fn valid(&self)->bool {!self.invalid && self.remaining==0}
}
struct SpanHasher {next:i64,hash:ContentHasher,utf8:Utf8}
impl Default for SpanHasher {fn default()->Self {Self {next:0,hash:ContentHasher::default(),utf8:Default::default()}}}
impl HeapSize for SpanHasher {fn heap_bytes(&self)->usize {0}}
struct Plan {fragments:Vec<Bytes>,charge:StateCharge}
impl Plan {
    fn new(budget:&ResourceBudget)->Self {Self {fragments:vec![],charge:StateCharge::new(budget,"analytic-text-fragments")}}
    fn push(&mut self,fragment:Bytes)->Result<(),ModelError> {self.charge.grow(size_of::<Bytes>()*2)?;self.fragments.push(fragment);Ok(())}
    fn punctuation(&mut self,value:&str)->Result<(),ModelError> {self.push(Bytes::of(value.as_bytes()))}
}
struct WindowCheck {
    assessment:Id<TextAssessment>,plan:Plan,fragment:usize,within:usize,hash:ContentHasher,
    ordinal:i64,offset:i64,cap:usize,pending:Vec<u8>,boundaries:Vec<usize>,charge:StateCharge,
}
impl WindowCheck {
    fn new(assessment:Id<TextAssessment>,plan:Plan,cap:usize,budget:&ResourceBudget)->Result<Self,ModelError> {
        let mut charge=StateCharge::new(budget,"analytic-current-text-window");charge.grow(size_of::<Self>())?;
        Ok(Self {assessment,plan,fragment:0,within:0,hash:ContentHasher::default(),ordinal:0,offset:0,cap,pending:vec![],boundaries:vec![],charge})
    }
    fn empty_fragments(&mut self)->Result<(),ModelError> {
        while self.plan.fragments.get(self.fragment).is_some_and(|fragment|fragment.len==self.within) {
            let hash=std::mem::take(&mut self.hash).finish();
            if hash!=self.plan.fragments[self.fragment].digest {return Err(invalid("analytic text changed original span/value bytes"));}
            self.fragment+=1;self.within=0;
        }
        Ok(())
    }
    fn canonical(&mut self,final_group:bool)->Result<(),ModelError> {
        while !self.boundaries.is_empty() {
            let text=std::str::from_utf8(&self.pending).map_err(ModelError::codec)?;
            let mut windows=stream_windows(text,self.cap)?;
            let Some((_,first))=windows.next() else {return Err(invalid("analytic text canonical window absent"));};
            let stable=final_group || windows.next().is_some();
            if !stable {break;}
            let length=first.len();
            if length!=self.boundaries[0] {return Err(invalid("analytic text changed canonical line/UTF-8 window boundary"));}
            self.boundaries.remove(0);self.pending.drain(..length);
        }
        Ok(())
    }
    fn visit(&mut self,row:TextWindow)->Result<(),ModelError> {
        row.validate()?;
        if row.assessment!=self.assessment || row.ordinal!=self.ordinal || row.start!=self.offset || row.text.len()>self.cap || (row.text.is_empty() && self.ordinal!=0) {return Err(invalid("analytic text window membership/order/cap differs"));}
        let mut bytes=row.text.as_str().as_bytes();self.empty_fragments()?;
        while !bytes.is_empty() {
            let expected=self.plan.fragments.get(self.fragment).ok_or_else(||invalid("unexpected analytic text suffix"))?;
            let take=(expected.len-self.within).min(bytes.len());self.hash.update(&bytes[..take]);self.within+=take;bytes=&bytes[take..];self.empty_fragments()?;
        }
        let capacity=self.pending.len().checked_add(row.text.len()).ok_or_else(||invalid("analytic window buffering overflow"))?;
        if capacity>self.pending.capacity() {self.charge.grow(capacity-self.pending.capacity())?;self.pending.reserve_exact(capacity-self.pending.len());}
        if self.boundaries.len()==self.boundaries.capacity() {self.charge.grow(size_of::<usize>())?;self.boundaries.reserve_exact(1);}
        self.pending.extend_from_slice(row.text.as_str().as_bytes());self.boundaries.push(row.text.len());
        self.offset=row.end;self.ordinal+=1;self.canonical(false)
    }
    fn finish(mut self)->Result<(),ModelError> {
        self.empty_fragments()?;self.canonical(true)?;
        if self.ordinal==0 || self.fragment!=self.plan.fragments.len() || self.within!=0 || !self.pending.is_empty() || !self.boundaries.is_empty() {return Err(invalid("analytic original text has missing windows/bytes"));}Ok(())
    }
}
pub(super) struct Check {
    artifacts:ChargedMap<Id<SourceArtifact>,Artifact>,selected:ChargedSet<Id<SourceArtifact>>,
    modules:ChargedMap<Id<SourceArtifact>,Vec<Bytes>>,qualifications:ChargedMap<Id<AssertionQualification>,Id<AnalysisContext>>,
    declarations:Rows<DeclarationObservation>,parameters:Rows<ParameterSyntaxObservation>,placements:Rows<SyntaxPlacement>,details:Rows<SyntaxDetailObservation>,
    detail_values:ChargedMap<Id<SyntaxDetail>,Id<crate::domain::value::Literal>>,literals:ChargedMap<Id<crate::domain::value::Literal>,Literal>,
    entities:Rows<EntityRef>,callables:Rows<CallableEntity>,classes:Rows<ClassEntity>,
    passages:ChargedMap<Id<PassageObservation>,Passage>,nodes:ChargedMap<Id<DocumentNode>,EvidenceSourceSpanId>,evidence:ChargedMap<Id<Evidence>,Id<SourceArtifact>>,
    wanted:ChargedSet<Id<Occurrence>>,byte_spans:ChargedSet<Id<Occurrence>>,occurrences:ChargedMap<Id<Occurrence>,Span>,span_receipts:ChargedMap<Id<Occurrence>,SpanReceipt>,active_spans:ChargedMap<Id<Occurrence>,SpanHasher>,
    definitions:Rows<TextDefinition>,subjects:Rows<TextSubject>,assessments:Rows<TextAssessment>,assessment_subjects:ChargedSet<Id<TextSubject>>,finished:ChargedSet<Id<TextAssessment>>,
    current:Option<WindowCheck>,previous:Option<Id<TextAssessment>>,charge:StateCharge,budget:ResourceBudget,
}
impl Check {
    pub(super) fn new(budget:&ResourceBudget)->Self {
        Self {artifacts:Default::default(),selected:Default::default(),modules:Default::default(),qualifications:Default::default(),declarations:Rows::new(budget),parameters:Rows::new(budget),placements:Rows::new(budget),details:Rows::new(budget),detail_values:Default::default(),literals:Default::default(),entities:Rows::new(budget),callables:Rows::new(budget),classes:Rows::new(budget),passages:Default::default(),nodes:Default::default(),evidence:Default::default(),wanted:Default::default(),byte_spans:Default::default(),occurrences:Default::default(),span_receipts:Default::default(),active_spans:Default::default(),definitions:Rows::new(budget),subjects:Rows::new(budget),assessments:Rows::new(budget),assessment_subjects:Default::default(),finished:Default::default(),current:None,previous:None,charge:StateCharge::new(budget,"analytic-text-admission-identities"),budget:budget.clone()}
    }
    fn definition(&self)->Result<&TextDefinition,ModelError> {if self.definitions.len()!=1 {return Err(invalid("analytic text needs one immutable definition"));}Ok(self.definitions.iter().next().expect("one definition"))}
    fn context(&self,qualification:Id<AssertionQualification>)->Result<Id<AnalysisContext>,ModelError> {self.qualifications.get(&qualification).copied().ok_or_else(||invalid("analytic text qualification absent"))}
    fn span(&self,id:Id<Occurrence>)->Result<Bytes,TextBoundary> {
        let span=self.occurrences.get(&id).ok_or(TextBoundary::MissingSource)?;
        let artifact=self.artifacts.get(&span.source).ok_or(TextBoundary::MissingSource)?;
        if span.start<0 || span.end<span.start || span.end>artifact.len {return Err(TextBoundary::MissingSource);}
        let receipt=self.span_receipts.get(&id).ok_or(TextBoundary::MissingSource)?;
        if !receipt.utf8 {return Err(TextBoundary::UnsupportedEncoding);}Ok(receipt.bytes)
    }
    fn chunk(&mut self,row:ArtifactChunk)->Result<(),ModelError> {
        row.validate()?;let first=row.ordinal.checked_mul(ARTIFACT_CHUNK_BYTES as i64).ok_or_else(||invalid("analytic source chunk offset overflow"))?;
        let end=first.checked_add(row.body.0.len() as i64).ok_or_else(||invalid("analytic source chunk end overflow"))?;
        for (id,span) in self.occurrences.iter().filter(|(id,span)|self.byte_spans.contains(*id) && span.source==row.artifact && span.start<end && span.end>first) {
            if self.span_receipts.contains_key(id) {continue;}
            if !self.active_spans.contains_key(id) {self.active_spans.insert(&mut self.charge,*id,SpanHasher {next:span.start,hash:ContentHasher::default(),utf8:Default::default()})?;}
            let from=span.start.max(first);let to=span.end.min(end);
            self.active_spans.update(&mut self.charge,*id,|state| {
                if state.next==from {
                    let bytes=&row.body.0[(from-first) as usize..(to-first) as usize];state.hash.update(bytes);state.utf8.update(bytes);state.next=to;
                }
            })?;
            if self.active_spans.get(id).is_some_and(|state|state.next==span.end) {
                let state=self.active_spans.remove(&mut self.charge,id).expect("completed span");
                self.span_receipts.insert(&mut self.charge,*id,SpanReceipt {bytes:Bytes {len:(span.end-span.start) as usize,digest:state.hash.finish()},utf8:state.utf8.valid()})?;
            }
        }
        Ok(())
    }
    fn declaration_plan(&self,row:&DeclarationObservation,source:Id<SourceArtifact>,context:Id<AnalysisContext>)->Result<Result<Plan,TextBoundary>,ModelError> {
        let mut plan=Plan::new(&self.budget);
        let Some(modules)=self.modules.get(&source) else {return Ok(Err(TextBoundary::MissingSource));};
        if modules.len()!=1 {return Ok(Err(TextBoundary::AmbiguousModule));}
        let mut names=vec![];let mut current=row;let mut count=0;
        loop {
            plan.charge.grow(size_of::<Bytes>()*2)?;
            match self.span(current.name) {Ok(bytes)=>names.push(bytes),Err(boundary)=>return Ok(Err(boundary))}
            let Some(parent)=current.parent else {break;};count+=1;
            if count>self.declarations.len() {return Err(invalid("cyclic analytic declaration ancestry"));}
            let mut parents=self.declarations.iter().filter(|candidate|candidate.declaration==parent && self.context(candidate.qualification).ok()==Some(context));
            let Some(parent)=parents.next() else {return Ok(Err(TextBoundary::MissingParent));};
            if parents.next().is_some() {return Ok(Err(TextBoundary::MissingParent));}current=parent;
        }
        let mut parameters=vec![];
        for parameter in self.parameters.iter().filter(|parameter|parameter.function==row.declaration && self.context(parameter.qualification).ok()==Some(context)) {
            plan.charge.grow(size_of::<(&ParameterSyntaxObservation,Bytes)>()*2)?;
            match self.span(parameter.parameter) {Ok(bytes)=>parameters.push((parameter,bytes)),Err(boundary)=>return Ok(Err(boundary))}
        }
        parameters.sort_by_key(|(parameter,_)|(parameter.ordinal,parameter.id()));
        if parameters.windows(2).any(|pair|pair[0].0.ordinal==pair[1].0.ordinal) {return Err(invalid("ambiguous analytic parameter ordinal"));}
        let doc=if let Some(doc)=row.docstring {
            let Some(statement)=self.occurrences.get(&doc).filter(|span|span.kind==SyntaxKind::StmtExpr && span.source==source) else {return Ok(Err(TextBoundary::MissingDocstring));};
            let mut children=self.placements.iter().filter(|placement|placement.parent==Some(doc) && placement.field==crate::domain::lexical::SyntaxField::Value && placement.ordinal==0 && self.context(placement.qualification).ok()==Some(context)).filter_map(|placement|self.occurrences.get(&placement.occurrence).map(|child|(placement.occurrence,child))).filter(|(_,child)|child.kind==SyntaxKind::ExprStringLiteral && child.source==statement.source);
            let Some((literal,_))=children.next() else {return Ok(Err(TextBoundary::MissingDocstring));};
            if children.any(|(other,_)|other!=literal) {return Ok(Err(TextBoundary::MissingDocstring));}
            let mut values=self.details.iter().filter(|detail|detail.occurrence==literal && self.context(detail.qualification).ok()==Some(context)).filter_map(|detail|self.detail_values.get(&detail.detail)).filter_map(|literal|self.literals.get(literal));
            let Some(value)=values.next() else {return Ok(Err(TextBoundary::MissingDocstring));};
            if values.any(|other|other.full.len!=value.full.len || other.full.digest!=value.full.digest) {return Ok(Err(TextBoundary::MissingDocstring));}Some(value.trimmed)
        } else {None};
        plan.push(modules[0])?;for name in names.iter().rev() {plan.punctuation(".")?;plan.push(*name)?;}plan.punctuation("(")?;
        let mut has_star=false;
        for (index,(parameter,bytes)) in parameters.iter().enumerate() {
            let (separator,marker)=parameter_prefix(index,parameter.kind,&mut has_star);plan.punctuation(separator)?;plan.punctuation(marker)?;plan.push(*bytes)?;
            plan.punctuation(parameter_suffix(parameter.kind,parameters.get(index+1).map(|(next,_)|next.kind)))?;
        }
        plan.punctuation(")")?;if let Some(doc)=doc.filter(|doc|doc.len>0) {plan.punctuation("\n")?;plan.push(doc)?;}Ok(Ok(plan))
    }
    fn expected(&self,subject:&TextSubject)->Result<Option<(TextAssessment,Option<Plan>)>,ModelError> {
        let definition=self.definition()?;if !definition.requested {return Ok(None);}
        let (source,context,entity,plan)=match subject {
            TextSubject::Declaration {declaration}=>{
                let row=self.declarations.get(*declaration).ok_or_else(||invalid("analytic declaration subject absent"))?;
                let source=self.occurrences.get(&row.declaration).ok_or_else(||invalid("analytic declaration occurrence absent"))?.source;
                if !self.selected.contains(&source) {return Ok(None);}let context=self.context(row.qualification)?;
                let entity=match row.kind {
                    DeclarationKind::Class=>{let class=ClassEntity::Source {declaration:row.declaration};self.classes.get(class.id()).map(|_|EntityRef::Class {class:class.id()})},
                    _=>{let callable=CallableEntity::Source {declaration:row.declaration,kind:CallableKind::Function};self.callables.get(callable.id()).map(|_|EntityRef::Callable {callable:callable.id()})},
                }.and_then(|entity|self.entities.get(entity.id()).map(Record::id));
                let plan=if entity.is_none() {Err(TextBoundary::MissingEntity)} else {self.declaration_plan(row,source,context)?};(source,context,entity,plan)
            }
            TextSubject::Passage {passage}=>{
                let row=self.passages.get(passage).ok_or_else(||invalid("analytic passage subject absent"))?;
                let span=self.nodes.get(&row.node.id()).ok_or_else(||invalid("analytic passage node absent"))?;
                let source=*self.evidence.get(&span.id()).ok_or_else(||invalid("analytic passage original span absent"))?;
                if !self.selected.contains(&source) {return Ok(None);}let mut plan=Plan::new(&self.budget);plan.push(row.bytes)?;(source,self.context(row.qualification)?,None,Ok(plan))
            }
        };
        let artifact=self.artifacts.get(&source).ok_or_else(||invalid("analytic subject artifact absent"))?;
        let assessment=TextAssessment {subject:subject.id(),definition:definition.id(),input:artifact.input,context,entity,source,availability:if plan.is_ok() {TextAvailability::Available} else {TextAvailability::Unavailable},boundary:plan.as_ref().err().copied()};
        Ok(Some((assessment,plan.ok())))
    }
    fn finish_current(&mut self)->Result<(),ModelError> {
        if let Some(current)=self.current.take() {let id=current.assessment;current.finish()?;self.finished.insert(&mut self.charge,id)?;}Ok(())
    }
}
impl InvariantCheck for Check {
    fn visit(&mut self,name:&str,batch:&arrow_array::RecordBatch)->Result<(),ModelError> {
        macro_rules! each {($ty:ty,$row:ident,$body:block)=>{if name==<$ty>::NAME {let _decode=self.budget.reserve("analytic-text-admission-decode",decode_allowance::<$ty>(batch)?)?;for $row in <$ty>::decode(batch)? $body return Ok(());}};}
        each!(SourceArtifact,row,{self.artifacts.insert(&mut self.charge,row.id(),Artifact {input:row.input,len:row.byte_len,class:crate::domain::admission::ArtifactClass::of(&row.path)})?;});
        each!(ArtifactUse,row,{let artifact=self.artifacts.get(&row.artifact).ok_or_else(||invalid("analytic artifact use absent"))?;if artifact.class.is_some_and(|class|class.requested_by(row.role)) {self.selected.insert(&mut self.charge,row.artifact)?;}});
        each!(Module,row,{self.modules.update(&mut self.charge,row.source,|modules|modules.push(Bytes::of(row.qualified_name.as_bytes())))?;});
        each!(AssertionQualification,row,{self.qualifications.insert(&mut self.charge,row.id(),row.context)?;});
        each!(DeclarationObservation,row,{self.wanted.insert(&mut self.charge,row.declaration)?;self.wanted.insert(&mut self.charge,row.name)?;self.byte_spans.insert(&mut self.charge,row.name)?;if let Some(doc)=row.docstring {self.wanted.insert(&mut self.charge,doc)?;}self.declarations.insert(row)?;});
        each!(ParameterSyntaxObservation,row,{self.wanted.insert(&mut self.charge,row.parameter)?;self.byte_spans.insert(&mut self.charge,row.parameter)?;self.parameters.insert(row)?;});
        each!(SyntaxPlacement,row,{self.wanted.insert(&mut self.charge,row.occurrence)?;self.placements.insert(row)?;});
        each!(SyntaxDetailObservation,row,{self.details.insert(row)?;});
        each!(SyntaxDetail,row,{let id=row.id();if let SyntaxDetail::Literal {literal}=row {self.detail_values.insert(&mut self.charge,id,literal)?;}});
        each!(crate::domain::value::Literal,row,{let id=row.id();if let crate::domain::value::Literal::String {value}=row {self.literals.insert(&mut self.charge,id,Literal {full:Bytes::of(value.as_bytes()),trimmed:Bytes::of(value.trim().as_bytes())})?;}});
        each!(EntityRef,row,{self.entities.insert(row)?;});each!(CallableEntity,row,{self.callables.insert(row)?;});each!(ClassEntity,row,{self.classes.insert(row)?;});
        each!(PassageObservation,row,{self.passages.insert(&mut self.charge,row.id(),Passage {qualification:row.qualification,node:row.passage,bytes:Bytes::of(row.text.as_bytes())})?;});
        each!(DocumentNode,row,{self.nodes.insert(&mut self.charge,row.id(),row.span())?;});
        each!(Evidence,row,{let id=row.id();if let Evidence::SourceSpan {source,..}=row {self.evidence.insert(&mut self.charge,id,source)?;}});
        each!(Occurrence,row,{let id=row.id();if self.wanted.contains(&id) {self.occurrences.insert(&mut self.charge,id,Span {source:row.source,start:row.start,end:row.end,kind:row.syntax_kind})?;if self.byte_spans.contains(&id) && row.start==row.end {self.span_receipts.insert(&mut self.charge,id,SpanReceipt {bytes:Bytes::of(b""),utf8:true})?;}}});
        each!(ArtifactChunk,row,{self.chunk(row)?;});each!(TextDefinition,row,{self.definitions.insert(row)?;});
        each!(TextSubject,row,{if self.expected(&row)?.is_none() {return Err(invalid("unexpected analytic text subject"));}self.subjects.insert(row)?;});
        each!(TextAssessment,row,{
            let subject=self.subjects.get(row.subject).ok_or_else(||invalid("analytic assessment subject absent"))?;
            let (expected,_)=self.expected(subject)?.ok_or_else(||invalid("unexpected analytic text assessment"))?;
            if row!=expected || !self.assessment_subjects.insert(&mut self.charge,row.subject)? {return Err(invalid("analytic text assessment changed source/context/entity/boundary"));}self.assessments.insert(row)?;
        });
        each!(TextWindow,row,{
            if self.current.as_ref().is_none_or(|current|current.assessment!=row.assessment) {
                self.finish_current()?;
                if self.previous.is_some_and(|previous|row.assessment<=previous) {return Err(invalid("analytic text windows are not grouped by assessment"));}
                let assessment=self.assessments.get(row.assessment).ok_or_else(||invalid("analytic text window assessment absent"))?;
                let subject=self.subjects.get(assessment.subject).ok_or_else(||invalid("analytic text window subject absent"))?;
                let (_,plan)=self.expected(subject)?.ok_or_else(||invalid("unexpected analytic text window"))?;
                let plan=plan.ok_or_else(||invalid("unavailable analytic text contains windows"))?;
                self.current=Some(WindowCheck::new(row.assessment,plan,self.definition()?.window_bytes as usize,&self.budget)?);self.previous=Some(row.assessment);
            }
            self.current.as_mut().expect("current window group").visit(row)?;
        });
        Err(invalid("undeclared analytic text admission input"))
    }
    fn finish(mut self:Box<Self>)->Result<(),ModelError> {
        self.finish_current()?;self.definition()?.validate()?;
        let mut expected=0;
        for row in self.declarations.iter() {if self.expected(&TextSubject::Declaration {declaration:row.id()})?.is_some() {expected+=1;}}
        for id in self.passages.keys() {if self.expected(&TextSubject::Passage {passage:*id})?.is_some() {expected+=1;}}
        if expected!=self.subjects.len() || expected!=self.assessments.len() {return Err(invalid("analytic text has missing/unexpected subjects or assessments"));}
        for assessment in self.assessments.iter() {if (assessment.availability==TextAvailability::Available)!=self.finished.contains(&assessment.id()) {return Err(invalid("analytic text has missing/unexpected original windows"));}}Ok(())
    }
}
pub(super) fn invariant()->Invariant {
    let inputs=vec![
        ValidationInput::of::<SourceArtifact>(&["id"]),ValidationInput::of::<ArtifactUse>(&["id"]),ValidationInput::of::<Module>(&["id"]),ValidationInput::of::<AssertionQualification>(&["id"]),
        ValidationInput::of::<DeclarationObservation>(&["id"]),ValidationInput::of::<ParameterSyntaxObservation>(&["id"]),ValidationInput::of::<SyntaxPlacement>(&["id"]),ValidationInput::of::<SyntaxDetailObservation>(&["id"]),ValidationInput::of::<SyntaxDetail>(&["id"]),ValidationInput::of::<crate::domain::value::Literal>(&["id"]),
        ValidationInput::of::<EntityRef>(&["id"]),ValidationInput::of::<CallableEntity>(&["id"]),ValidationInput::of::<ClassEntity>(&["id"]),ValidationInput::of::<PassageObservation>(&["id"]),ValidationInput::of::<DocumentNode>(&["id"]),ValidationInput::of::<Evidence>(&["id"]),
        ValidationInput::of::<Occurrence>(&["id"]),ValidationInput::of::<ArtifactChunk>(&["artifact","ordinal"]),ValidationInput::of::<TextDefinition>(&["id"]),ValidationInput::of::<TextSubject>(&["id"]),ValidationInput::of::<TextAssessment>(&["id"]),ValidationInput::of::<TextWindow>(&["assessment","ordinal"]),
    ];
    Invariant {name:"analytic_text_original_membership",purpose:InvariantPurpose::Admission,revision:1,inputs,create:std::sync::Arc::new(|budget|Box::new(Check::new(budget)))}
}
