//! Selected SourceCall dependency grains and private attempt-owned result payloads.
use lctx_model::domain::{*,declarations::{ParameterDeclaration,SymbolDeclaration},analysis::source_call::AnalysisInvocation,normalized::events::NormalizedCallEvent};
use std::{fs::File,io::{Read,Seek,SeekFrom,Write},sync::Mutex};
#[derive(Clone,Copy)]
struct Slot {offset:u64,length:usize}
impl HeapSize for Slot {fn heap_bytes(&self)->usize{0}}
/// Anonymous bytes are a private compiler physical resource. The actual model issuer verifies
/// their immutable frame/event digest before accepting any rehydrated private value.
pub(super) struct SourcePayloadSpool {
    file:Option<Mutex<File>>,
    slots:charged::ChargedMap<(Id<AnalysisInvocation>,Id<NormalizedCallEvent>),Slot>,
    charge:charged::StateCharge,
}
impl SourcePayloadSpool {
    pub(super) fn new(budget:&resources::ResourceBudget)->Self{Self {file:None,slots:Default::default(),charge:charged::StateCharge::new(budget,"source-call-private-payload-index")}}
    pub(super) fn insert(&mut self,frame:Id<AnalysisInvocation>,event:Id<NormalizedCallEvent>,bytes:&[u8])->Result<(),ModelError>{
        if self.slots.contains_key(&(frame,event)){return Err(ModelError::Conflict("SourceCall private payload duplicated"));}
        if self.file.is_none(){self.file=Some(Mutex::new(tempfile::tempfile().map_err(ModelError::codec)?));}
        let file=self.file.as_mut().expect("opened private spool").get_mut().map_err(|_|ModelError::Conflict("SourceCall private spool lock poisoned"))?;
        let offset=file.seek(SeekFrom::End(0)).map_err(ModelError::codec)?;
        offset.checked_add(u64::try_from(bytes.len()).map_err(ModelError::codec)?).ok_or(ModelError::Conflict("SourceCall private payload offset overflow"))?;
        file.write_all(bytes).map_err(ModelError::codec)?;
        self.slots.insert(&mut self.charge,(frame,event),Slot {offset,length:bytes.len()})?;Ok(())
    }
    pub(super) fn read(&self,frame:Id<AnalysisInvocation>,event:Id<NormalizedCallEvent>,budget:&resources::ResourceBudget)->Result<(Vec<u8>,Box<dyn resources::Reservation>),ModelError>{
        if !self.charge.budget().expect("bound spool").shares_pool(budget){return Err(ModelError::Conflict("SourceCall private spool budget changed"));}
        let slot=self.slots.get(&(frame,event)).ok_or(ModelError::Conflict("SourceCall private payload absent"))?;
        let reservation=budget.reserve("source-call-private-payload-read",slot.length)?;
        let mut bytes=vec![0;slot.length];let mut file=self.file.as_ref().ok_or(ModelError::Conflict("SourceCall private spool absent"))?.lock().map_err(|_|ModelError::Conflict("SourceCall private spool lock poisoned"))?;
        file.seek(SeekFrom::Start(slot.offset)).map_err(ModelError::codec)?;file.read_exact(&mut bytes).map_err(ModelError::codec)?;
        Ok((bytes,reservation))
    }
}

use crate::{consumed_rows::{ClosureTable,NominalClosure,PreparedEdges,PreparedClosure,identifier},workspace::CompletedInputs};
use lctx_model::domain::{execution::{source_call_records::SourceCallData,body_records::*,records::*},normalized::{binding_normalization::*,bindings::*,callables::*,entities::*,events::*},source::*,syntax::*,flow::*,calls::*,lexical::*,symbols::*};
use std::{any::TypeId,sync::Arc};
use futures::TryStreamExt;
use super::execution_scope::{nominal,predicate};
pub(super) struct SourceCallScopes {inputs:Vec<ValidationInput>,tables:Vec<ClosureTable>,edges:PreparedEdges,event:usize,owner:Option<usize>,statement:Option<usize>,_charge:charged::StateCharge}
impl SourceCallScopes {
    pub(super) async fn prepare(access:&CompletedInputs,session:&datafusion::prelude::SessionContext,model:&Arc<ValidatedModel>,budget:&resources::ResourceBudget)->Result<Self,ModelError>{
        let inputs=SourceCallData::inputs();let tables=inputs.iter().map(|input|Ok(ClosureTable {relation:model.relation(input.name()).ok_or(ModelError::Schema(input.name()))?.clone(),alias:access.table_for(input)?})).collect::<Result<Vec<_>,ModelError>>()?;
        Self::prepare_bound(inputs,tables,session,budget).await
    }
    pub(super) async fn prepare_enriched(access:&CompletedInputs,session:&datafusion::prelude::SessionContext,model:&Arc<ValidatedModel>,budget:&resources::ResourceBudget)->Result<Self,ModelError>{
        let inputs=execution::enriched_production::EnrichedData::inputs();let tables=inputs.iter().map(|input|Ok(ClosureTable {relation:model.relation(input.name()).ok_or(ModelError::Schema(input.name()))?.clone(),alias:access.table_for(input)?})).collect::<Result<Vec<_>,ModelError>>()?;
        Self::prepare_bound(inputs,tables,session,budget).await
    }
    async fn prepare_bound(inputs:Vec<ValidationInput>,mut tables:Vec<ClosureTable>,session:&datafusion::prelude::SessionContext,budget:&resources::ResourceBudget)->Result<Self,ModelError>{
        let index=|kind:TypeId|inputs.iter().position(|input|input.type_id()==kind).ok_or(ModelError::Schema("SourceCall dependency input"));
        let event=index(TypeId::of::<NormalizedCallEvent>())?;let occurrence=index(TypeId::of::<Occurrence>())?;
        let callee=tables.len();tables.push(tables[index(TypeId::of::<NormalizedCallAlternative>())?].clone());
        let header=tables.len();tables.push(tables[occurrence].clone());
        let payload=tables.len();tables.push(tables[occurrence].clone());
        let children=tables.len();tables.push(tables[index(TypeId::of::<SyntaxPlacement>())?].clone());
        let enriched=inputs.iter().any(|input|input.type_id()==TypeId::of::<execution::source_call_records::SourceCallHeader>());
        let (owner,statement)=if enriched {let owner=tables.len();tables.push(tables[index(TypeId::of::<EntityRef>())?].clone());let statement=tables.len();tables.push(tables[occurrence].clone());(Some(owner),Some(statement))}else{(None,None)};
        let mut plan=NominalClosure::new(tables.clone())?;
        for (source,table) in tables.iter().take(inputs.len()).enumerate(){for field in table.relation.fields(){
            // An ordinary declaration carries metadata. Its docstring body is used only by
            // the exact caller-prefix closure, and values are private dependency roots.
            if (table.relation.type_id()==TypeId::of::<DeclarationObservation>() && field.name()=="docstring") || (table.relation.type_id()==TypeId::of::<SyntaxDetail>() && field.name()=="literal_literal") {continue;}
            let Some((target,_))=field.target() else{continue;};let Some(target)=crate::scoped_admission::field_target(&inputs,source,target)? else{continue;};
            if field.list(){plan.pairs(source,target,format!("SELECT id AS source_id,UNNEST({}) AS target_id FROM {}",identifier(field.name()),identifier(&table.alias)))?;}else{plan.follow(source,field.name(),target)?;}
            if field.name()=="assertion"||field.name().ends_with("_assertion")||(table.relation.type_id()==TypeId::of::<analysis::native::NativeQualification>()&&field.name()=="premise"){plan.own(source,field.name(),target)?;}
        }}
        macro_rules! own {($member:ty,$field:literal,$owner:ty)=>{plan.own(index(TypeId::of::<$member>())?,$field,index(TypeId::of::<$owner>())?)?};}
        own!(input::ArtifactUse,"artifact",SourceArtifact);own!(OccurrenceOwnership,"occurrence",Occurrence);own!(SyntaxPlacement,"occurrence",Occurrence);own!(DeclarationObservation,"declaration",Occurrence);own!(SyntaxObservation,"occurrence",Occurrence);
        own!(NormalizedCallAlternative,"event",NormalizedCallEvent);own!(CallBindingAttempt,"event",NormalizedCallEvent);own!(CallBinding,"attempt",CallBindingAttempt);own!(BindingSetAssessment,"event",NormalizedCallEvent);own!(BindingVariantAssessment,"set",BindingSetAssessment);own!(BindingSetMember,"variant",BindingVariantAssessment);own!(BindingSetCoverage,"set",BindingSetAssessment);
        own!(EffectiveCallableEvidence,"assessment",EffectiveCallableAssessment);own!(EffectiveCallableAssessment,"callable",CallableEntity);own!(SignatureVariant,"signature",Signature);own!(SignatureSlot,"variant",SignatureVariant);own!(SignatureSlotEntity,"slot",SignatureSlot);own!(SignatureParameter,"signature",Signature);own!(ParameterEntityLink,"parameter",SignatureParameter);own!(ParameterDeclaration,"parameter",SignatureParameter);own!(SignatureEnumerationMember,"enumeration",SignatureEnumerationObservation);
        own!(CallArgument,"call",CallSyntax);own!(CallSyntax,"site",Occurrence);own!(CallTarget,"site",Occurrence);own!(ReferenceObservation,"read",Occurrence);own!(LexicalResolution,"read",Occurrence);own!(BindingEvent,"site",Occurrence);own!(BindingObservation,"event",BindingEvent);own!(FlowUse,"occurrence",Occurrence);own!(FlowUseObservation,"use_",FlowUse);own!(FlowReachingObservation,"use_",FlowUse);own!(FlowValueObservation,"use_",FlowUse);own!(FlowDefinition,"occurrence",Occurrence);own!(FlowDefinitionObservation,"definition",FlowDefinition);
        own!(ExpressionEvaluation,"expression",Occurrence);own!(EvaluationMember,"evaluation",ExpressionEvaluation);own!(EvaluationOperand,"evaluation",ExpressionEvaluation);own!(BodyMember,"body",SourceBodyCompletion);own!(BodyReleaseInput,"body",SourceBodyCompletion);
        own!(LexicalScope,"owner",Occurrence);own!(SymbolDeclaration,"declaration",Occurrence);own!(SymbolDeclaration,"symbol",ProviderSymbol);own!(SymbolObservation,"symbol",ProviderSymbol);own!(SymbolSequenceMember,"sequence",SymbolSequence);
        // A docstring contributes exact source spans to fresh-header availability. Its
        // rich UTF-8 payload never contributes to that predicate.
        plan.follow(index(TypeId::of::<DeclarationObservation>())?,"docstring",occurrence)?;
        let t=|kind:TypeId|Ok::<_,ModelError>(identifier(&tables[index(kind)?].alias));macro_rules! table {($ty:ty)=>{t(TypeId::of::<$ty>())?};}
        macro_rules! pair {($from:ty,$to:ty,$sql:expr)=>{plan.pairs(index(TypeId::of::<$from>())?,index(TypeId::of::<$to>())?,$sql)?};}
        let alternatives=table!(NormalizedCallAlternative);let refs=table!(EntityRef);let callables=table!(CallableEntity);let occurrences=table!(Occurrence);let placements=table!(SyntaxPlacement);let declarations=table!(DeclarationObservation);let bodies=table!(SourceBodyCompletion);let normalized_events=table!(NormalizedCallEvent);let base_frames=table!(analysis::base_completion::AnalysisInvocation);
        // Only actual alternative callees become body roots. Merely mentioning the caller
        // entity must not pull that caller's whole earlier body into a binding kernel.
        plan.pairs(index(TypeId::of::<NormalizedCallAlternative>())?,callee,format!("SELECT id AS source_id,id AS target_id FROM {alternatives} WHERE entity IS NOT NULL"))?;
        plan.pairs(callee,index(TypeId::of::<EntityRef>())?,format!("SELECT id AS source_id,entity AS target_id FROM {alternatives} WHERE entity IS NOT NULL"))?;
        plan.pairs(callee,index(TypeId::of::<SourceBodyCompletion>())?,format!("SELECT a.id AS source_id,b.id AS target_id FROM {alternatives} a JOIN {normalized_events} e ON e.id=a.event JOIN {bodies} b ON b.owner=a.entity JOIN {base_frames} f ON f.id=b.invocation AND f.context=e.context"))?;
        plan.pairs(callee,header,format!("SELECT a.id AS source_id,c.source_declaration AS target_id FROM {alternatives} a JOIN {refs} r ON r.id=a.entity JOIN {callables} c ON c.id=r.callable_callable WHERE c.source_declaration IS NOT NULL"))?;
        plan.pairs(header,occurrence,format!("SELECT id AS source_id,id AS target_id FROM {occurrences}"))?;
        plan.pairs(header,children,format!("SELECT d.id AS source_id,p.id AS target_id FROM {occurrences} d JOIN {placements} p ON p.parent=d.id"))?;
        // Captures are a complete range/path competition within the actual declaration. The
        // SQL closure discovers them before any rich source row is decoded.
        let resolutions=table!(LexicalResolution);
        plan.pairs(header,index(TypeId::of::<LexicalResolution>())?,format!("SELECT d.id AS source_id,r.id AS target_id FROM {occurrences} d JOIN {occurrences} o ON o.source=d.source AND o.start>=d.start AND o.end<=d.end AND array_slice(o.structural_path,1,CAST(array_length(d.structural_path) AS BIGINT))=d.structural_path JOIN {resolutions} r ON r.read=o.id WHERE r.captured=true"))?;
        plan.pairs(children,index(TypeId::of::<SyntaxPlacement>())?,format!("SELECT id AS source_id,id AS target_id FROM {placements}"))?;
        plan.pairs(children,occurrence,format!("SELECT id AS source_id,occurrence AS target_id FROM {placements}"))?;
        plan.pairs(children,children,format!("SELECT p.id AS source_id,c.id AS target_id FROM {placements} p JOIN {placements} c ON c.parent=p.occurrence WHERE p.field!={}",SyntaxField::Body.code()))?;
        // Caller suite ordinal zero may be the exact literal prefix or a docstring. Ancestor
        // ownership itself never expands other sibling body rows.
        pair!(Occurrence,SyntaxPlacement,format!("SELECT o.id AS source_id,p.id AS target_id FROM {occurrences} o JOIN {placements} p ON p.parent=o.id WHERE o.syntax_kind={} AND p.field={} AND p.ordinal=0",SyntaxKind::StmtFunctionDef.code(),SyntaxField::Body.code()));
        plan.pairs(index(TypeId::of::<SyntaxPlacement>())?,payload,format!("SELECT p.id AS source_id,p.occurrence AS target_id FROM {placements} p JOIN {occurrences} o ON o.id=p.parent WHERE o.syntax_kind={} AND p.field={} AND p.ordinal=0",SyntaxKind::StmtFunctionDef.code(),SyntaxField::Body.code()))?;
        plan.pairs(payload,occurrence,format!("SELECT id AS source_id,id AS target_id FROM {occurrences}"))?;
        plan.pairs(payload,children,format!("SELECT o.id AS source_id,p.id AS target_id FROM {occurrences} o JOIN {placements} p ON p.parent=o.id WHERE o.syntax_kind NOT IN ({},{},{})",SyntaxKind::StmtFunctionDef.code(),SyntaxKind::StmtClassDef.code(),SyntaxKind::ExprLambda.code()))?;
        plan.pairs(children,payload,format!("SELECT p.id AS source_id,p.occurrence AS target_id FROM {placements} p JOIN {occurrences} o ON o.id=p.parent WHERE o.syntax_kind NOT IN ({},{},{})",SyntaxKind::StmtFunctionDef.code(),SyntaxKind::StmtClassDef.code(),SyntaxKind::ExprLambda.code()))?;
        let details=table!(SyntaxDetailObservation);let values=table!(SyntaxDetail);let literals=table!(value::Literal);
        plan.pairs(payload,index(TypeId::of::<SyntaxDetailObservation>())?,format!("SELECT o.id AS source_id,d.id AS target_id FROM {occurrences} o JOIN {details} d ON d.occurrence=o.id"))?;
        plan.pairs(payload,index(TypeId::of::<value::Literal>())?,format!("SELECT o.id AS source_id,l.id AS target_id FROM {occurrences} o JOIN {details} d ON d.occurrence=o.id JOIN {values} v ON v.id=d.detail JOIN {literals} l ON l.id=v.literal_literal"))?;
        // Native callee uses can have corresponding canonical occurrences; identity and all
        // reaching/region alternatives are selected together rather than one winning row.
        let uses=table!(FlowUseObservation);let regions=table!(FlowRegionObservation);let q=table!(assertion::AssertionQualification);
        pair!(FlowUseObservation,FlowRegionObservation,format!("SELECT u.id AS source_id,r.id AS target_id FROM {uses} u JOIN {q} uq ON uq.id=u.qualification JOIN {regions} r ON r.scope=u.scope JOIN {q} rq ON rq.id=r.qualification AND rq.context=uq.context"));
        let bindings=table!(BindingObservation);let events=table!(BindingEvent);
        pair!(BindingObservation,BindingObservation,format!("SELECT a.id AS source_id,b.id AS target_id FROM {bindings} a JOIN {events} ae ON ae.id=a.event JOIN {bindings} b ON b.scope=a.scope JOIN {events} be ON be.id=b.event AND be.name=ae.name"));
        let references=table!(ReferenceObservation);let spellings=table!(SyntaxObservation);let lexical_scopes=table!(LexicalScope);
        pair!(DeclarationObservation,ReferenceObservation,format!("SELECT d.id AS source_id,r.id AS target_id FROM {declarations} d JOIN {q} dq ON dq.id=d.qualification JOIN {spellings} s ON s.occurrence=d.name JOIN {lexical_scopes} outer_scope ON outer_scope.owner=d.parent JOIN {references} r ON r.name=s.spelling AND r.scope=outer_scope.id JOIN {q} rq ON rq.id=r.qualification AND rq.context=dq.context"));
        let artifacts=table!(SourceArtifact);let scopes=table!(CoverageScope);let coverage=table!(attribution::ProviderCoverage);let modules=table!(Module);
        pair!(SourceArtifact,attribution::ProviderCoverage,format!("SELECT a.id AS source_id,c.id AS target_id FROM {artifacts} a JOIN {scopes} s ON s.input_input=a.input OR s.artifact_artifact=a.id JOIN {coverage} c ON c.scope=s.id UNION SELECT a.id AS source_id,c.id AS target_id FROM {artifacts} a JOIN {modules} m ON m.source=a.id JOIN {scopes} s ON s.module_module=m.id JOIN {coverage} c ON c.scope=s.id"));
        if let (Some(owner),Some(statement))=(owner,statement) {
            use execution::source_call_records::{SourceCallHeader,HeaderMember,SourceFrameRelease,SourceFrameArgument,SourceInvocation};
            own!(NormalizedCallEvent,"owner",OccurrenceOwnership);own!(SourceCallHeader,"event",NormalizedCallEvent);own!(HeaderMember,"header",SourceCallHeader);own!(SourceFrameRelease,"header",SourceCallHeader);own!(SourceFrameArgument,"release",SourceFrameRelease);own!(SourceInvocation,"release",SourceFrameRelease);
            let owners=table!(OccurrenceOwnership);
            plan.pairs(owner,index(TypeId::of::<EntityRef>())?,format!("SELECT id AS source_id,id AS target_id FROM {refs}"))?;
            plan.pairs(owner,payload,format!("SELECT r.id AS source_id,m.occurrence AS target_id FROM {refs} r JOIN {owners} m ON m.entity=r.id"))?;
            plan.pairs(owner,index(TypeId::of::<CallableEntity>())?,format!("SELECT r.id AS source_id,c.id AS target_id FROM {refs} r JOIN {callables} c ON c.id=r.callable_callable"))?;
            // A selected callable body's lexical children belong to this actual owner. A
            // referenced callable reached through a value remains ordinary metadata.
            plan.pairs(owner,payload,format!("SELECT r.id AS source_id,p.occurrence AS target_id FROM {refs} r JOIN {callables} c ON c.id=r.callable_callable JOIN {placements} p ON p.parent=c.source_declaration WHERE p.field={}",SyntaxField::Body.code()))?;
            plan.pairs(statement,payload,format!("SELECT id AS source_id,id AS target_id FROM {occurrences}"))?;
        }
        let mut charge=charged::StateCharge::new(budget,"SourceCall-scope-descriptors");charge.grow(inputs.capacity()*size_of::<ValidationInput>()+tables.capacity()*size_of::<ClosureTable>()+tables.iter().map(|table|table.alias.capacity()).sum::<usize>())?;
        let edges=plan.prepare(session,budget).await?;Ok(Self {inputs,tables,edges,event,owner,statement,_charge:charge})
    }
    fn index(&self,kind:TypeId)->Result<usize,ModelError>{self.inputs.iter().position(|input|input.type_id()==kind).ok_or(ModelError::Schema("SourceCall dependency type"))}
    fn table<R:Record>(&self)->Result<String,ModelError>{Ok(identifier(&self.tables[self.index(TypeId::of::<R>())?].alias))}
    pub(super) fn roots(&self,inv:&AnalysisInvocation)->Result<String,ModelError>{
        let events=self.table::<NormalizedCallEvent>()?;let occurrences=self.table::<Occurrence>()?;let artifacts=self.table::<SourceArtifact>()?;let uses=self.table::<input::ArtifactUse>()?;
        let context=predicate(inv.context).replacen("id IN","e.context IN",1);let input=predicate(inv.input).replacen("id IN","a.input IN",1);
        Ok(format!("SELECT e.id FROM {events} e LEFT JOIN {occurrences} o ON o.id=e.site LEFT JOIN {artifacts} a ON a.id=o.source WHERE {context} AND (o.id IS NULL OR a.id IS NULL OR ({input} AND (a.path LIKE '%.py' OR a.path LIKE '%.pyi') AND EXISTS (SELECT 1 FROM {uses} u WHERE u.artifact=a.id AND u.role IN ({},{},{},{})))) ORDER BY e.id",input::SourceRole::Release.code(),input::SourceRole::Example.code(),input::SourceRole::Test.code(),input::SourceRole::DocBlock.code()))
    }
    pub(super) async fn scope(&self,event:Id<NormalizedCallEvent>,budget:&resources::ResourceBudget)->Result<PreparedClosure,ModelError>{self.edges.grain(self.event,&predicate(event),budget).await}
    pub(super) fn owner_roots(&self,inv:&analysis::enriched_execution::AnalysisInvocation,unowned:bool)->Result<String,ModelError>{
        let occurrences=self.table::<Occurrence>()?;let owners=self.table::<OccurrenceOwnership>()?;let artifacts=self.table::<SourceArtifact>()?;let uses=self.table::<input::ArtifactUse>()?;let refs=self.table::<EntityRef>()?;let callables=self.table::<CallableEntity>()?;
        let input=predicate(inv.input).replacen("id IN","a.input IN",1);
        let selected=format!("{input} AND (a.path LIKE '%.py' OR a.path LIKE '%.pyi') AND EXISTS (SELECT 1 FROM {uses} u WHERE u.artifact=a.id AND u.role IN ({},{},{},{}))",input::SourceRole::Release.code(),input::SourceRole::Example.code(),input::SourceRole::Test.code(),input::SourceRole::DocBlock.code());
        let statements=[SyntaxKind::StmtFunctionDef,SyntaxKind::StmtClassDef,SyntaxKind::StmtReturn,SyntaxKind::StmtDelete,SyntaxKind::StmtTypeAlias,SyntaxKind::StmtAssign,SyntaxKind::StmtAugAssign,SyntaxKind::StmtAnnAssign,SyntaxKind::StmtFor,SyntaxKind::StmtWhile,SyntaxKind::StmtIf,SyntaxKind::StmtWith,SyntaxKind::StmtMatch,SyntaxKind::StmtRaise,SyntaxKind::StmtTry,SyntaxKind::StmtAssert,SyntaxKind::StmtImport,SyntaxKind::StmtImportFrom,SyntaxKind::StmtGlobal,SyntaxKind::StmtNonlocal,SyntaxKind::StmtExpr,SyntaxKind::StmtPass,SyntaxKind::StmtBreak,SyntaxKind::StmtContinue,SyntaxKind::StmtIpyEscapeCommand].iter().map(|kind|kind.code().to_string()).collect::<Vec<_>>().join(",");
        if unowned {return Ok(format!("SELECT o.id FROM {occurrences} o JOIN {artifacts} a ON a.id=o.source WHERE {selected} AND o.syntax_kind IN ({statements}) AND NOT EXISTS (SELECT 1 FROM {owners} m WHERE m.occurrence=o.id) ORDER BY o.id"));}
        let attempts=self.table::<CallBindingAttempt>()?;let events=self.table::<NormalizedCallEvent>()?;let context=predicate(inv.context).replacen("id IN","e.context IN",1);
        Ok(format!("SELECT m.entity AS id FROM {owners} m JOIN {occurrences} o ON o.id=m.occurrence JOIN {artifacts} a ON a.id=o.source WHERE {selected} AND o.syntax_kind IN ({statements},{}) UNION SELECT r.id FROM {refs} r JOIN {callables} c ON c.id=r.callable_callable JOIN {occurrences} o ON o.id=c.source_declaration JOIN {artifacts} a ON a.id=o.source WHERE {selected} UNION SELECT m.entity AS id FROM {attempts} attempt JOIN {events} e ON e.id=attempt.event JOIN {owners} m ON m.id=e.owner JOIN {occurrences} o ON o.id=e.site JOIN {artifacts} a ON a.id=o.source WHERE {selected} AND {context} ORDER BY id",SyntaxKind::ExprName.code()))
    }
    pub(super) async fn owner_scope(&self,owner:Id<EntityRef>,budget:&resources::ResourceBudget)->Result<PreparedClosure,ModelError>{self.edges.grain(self.owner.ok_or(ModelError::Schema("Enriched owner namespace"))?,&predicate(owner),budget).await}
    pub(super) async fn unowned_scope(&self,statement:Id<Occurrence>,budget:&resources::ResourceBudget)->Result<PreparedClosure,ModelError>{self.edges.grain(self.statement.ok_or(ModelError::Schema("Enriched statement namespace"))?,&predicate(statement),budget).await}
    async fn docstring_literals(&self,scope:&PreparedClosure,sql:&str,data:&mut execution::evaluation::EvaluationData,budget:&resources::ResourceBudget)->Result<String,ModelError>{
        let declarations=self.table::<DeclarationObservation>()?;let occurrences=self.table::<Occurrence>()?;let details=self.table::<SyntaxDetailObservation>()?;let values=self.table::<SyntaxDetail>()?;
        let docstrings=format!("SELECT DISTINCT v.literal_literal AS id FROM {declarations} d JOIN {occurrences} span ON span.id=d.docstring JOIN {occurrences} o ON o.source=span.source AND o.start<=span.start AND o.end>=span.end AND o.syntax_kind={} JOIN {details} detail ON detail.occurrence=o.id JOIN {values} v ON v.id=detail.detail WHERE v.literal_literal IS NOT NULL",SyntaxKind::ExprStringLiteral.code());
        // Literal IDs are content based. A shared ID used by a selected capture/default
        // still requires its exact payload even if a docstring happens to have equal bytes.
        let selected_occurrences=scope.select(self.index(TypeId::of::<Occurrence>())?)?;
        let non_docstring_use=format!("EXISTS (SELECT 1 FROM ({selected_occurrences}) o JOIN {details} detail ON detail.occurrence=o.id JOIN {values} v ON v.id=detail.detail WHERE v.literal_literal=l.id AND NOT EXISTS (SELECT 1 FROM {declarations} d JOIN {occurrences} span ON span.id=d.docstring WHERE o.source=span.source AND o.start<=span.start AND o.end>=span.end AND o.syntax_kind={}))",SyntaxKind::ExprStringLiteral.code());
        let opaque=format!("l.kind=3 AND EXISTS (SELECT 1 FROM ({docstrings}) docstring WHERE docstring.id=l.id) AND NOT ({non_docstring_use})");
        let projection=format!("SELECT l.id,l.kind FROM ({sql}) l WHERE {opaque} ORDER BY l.id");
        let mut stream=crate::sql::query(scope.session(),&projection).await.map_err(ModelError::codec)?.execute_stream().await.map_err(ModelError::codec)?;
        while let Some(batch)=stream.try_next().await.map_err(ModelError::codec)? {let _transfer=budget.reserve("execution-docstring-kind-transfer",logical_batch_bytes(&batch)?)?;
            for row in 0..batch.num_rows(){let key=crate::scoped_admission::column(&batch,"id",row)?.ok_or(ModelError::Schema("docstring literal projection ID"))?;data.project_opaque_literal(nominal(&key)?,3)?;}tokio::task::yield_now().await;
        }
        Ok(format!("SELECT l.* FROM ({sql}) l WHERE NOT ({opaque})"))
    }
    pub(super) async fn enriched_data(&self,scope:&PreparedClosure,budget:&resources::ResourceBudget)->Result<execution::enriched_production::EnrichedData,ModelError>{
        let mut data=execution::enriched_production::EnrichedData::new(budget);
        for (table,input) in self.inputs.iter().enumerate(){let sql=scope.select(table)?;
            let sql=if input.type_id()==TypeId::of::<value::Literal>() {self.docstring_literals(scope,&sql,&mut data.source.evaluation,budget).await?}else{sql};
            let mut stream=crate::sql::query(scope.session(),&sql).await.map_err(ModelError::codec)?.execute_stream().await.map_err(ModelError::codec)?;
            while let Some(batch)=stream.try_next().await.map_err(ModelError::codec)? {let _transfer=budget.reserve("Enriched-selected-input-transfer",lctx_model::domain::logical_batch_bytes(&batch)?)?;data.visit_input(input,&batch)?;tokio::task::yield_now().await;}
        }Ok(data)
    }
    pub(super) async fn data(&self,scope:&PreparedClosure,budget:&resources::ResourceBudget)->Result<SourceCallData,ModelError>{
        let mut data=SourceCallData::new(budget);
        for (table,input) in self.inputs.iter().enumerate(){let sql=scope.select(table)?;
            let sql=if input.type_id()==TypeId::of::<value::Literal>() {self.docstring_literals(scope,&sql,&mut data.evaluation,budget).await?}else{sql};
            let mut stream=crate::sql::query(scope.session(),&sql).await.map_err(ModelError::codec)?.execute_stream().await.map_err(ModelError::codec)?;
            while let Some(batch)=stream.try_next().await.map_err(ModelError::codec)? {let _transfer=budget.reserve("SourceCall-selected-input-transfer",lctx_model::domain::logical_batch_bytes(&batch)?)?;data.visit_input(input,&batch)?;tokio::task::yield_now().await;}
        }Ok(data)
    }
}

/// Share only the immutable configuration/frame domain; no source, proof or output row is
/// copied from an earlier rich kernel. Rows validates duplicate identities mechanically.
pub(super) fn enriched_configuration(from:&execution::enriched_production::EnrichedData,to:&mut execution::enriched_production::EnrichedData)->Result<(),ModelError>{
    for row in from.parameters.iter(){to.parameters.insert(row.clone())?;}
    for row in from.catalogs.iter(){to.catalogs.insert(row.clone())?;}
    for row in from.source_invocations.iter(){to.source_invocations.insert(row.clone())?;}
    for row in from.source.definitions.iter(){to.source.definitions.insert(row.clone())?;}
    Ok(())
}
pub(super) fn hydrate_selected(values:&execution::source_call_records::ProducedSourceCalls,spool:&SourcePayloadSpool,parent:&AnalysisInvocation,data:&execution::enriched_production::EnrichedData,budget:&resources::ResourceBudget)->Result<execution::source_call_records::HydratedSourceCalls,ModelError>{
    let mut hydrated=values.hydrate_empty(parent,budget)?;
    let mut events=charged::ChargedSet::default();let mut charge=charged::StateCharge::new(budget,"Enriched-selected-private-events");
    for header in data.source_headers.iter().filter(|row|row.invocation==parent.id()) {
        if events.insert(&mut charge,header.event)? {
            let (bytes,_read)=spool.read(parent.id(),header.event,budget)?;
            hydrated.append(values.hydrate(parent,header.event,&bytes,budget)?)?;
        }
    }
    Ok(hydrated)
}

#[cfg(test)]
mod controls {
    use super::*;
    use datafusion::{prelude::SessionContext,datasource::MemTable};
    use assertion::*;
    fn id<R>(n:u8)->Id<R>{nominal(&[n;16]).unwrap()}
    fn register<R:Record>(session:&SessionContext,rows:&[R]){let batch=<R as Record>::encode(rows).unwrap();session.deregister_table(R::NAME).unwrap();session.register_table(R::NAME,Arc::new(MemTable::try_new(batch.schema(),vec![vec![batch]]).unwrap())).unwrap();}
    async fn prepare(session:&SessionContext,budget:&resources::ResourceBudget,enriched:bool)->SourceCallScopes{
        let inputs=if enriched{execution::enriched_production::EnrichedData::inputs()}else{SourceCallData::inputs()};let model=model().unwrap();
        for input in &inputs{let schema=model.relation(input.name()).unwrap().schema().clone();session.deregister_table(input.name()).unwrap();session.register_table(input.name(),Arc::new(MemTable::try_new(schema.clone(),vec![vec![arrow_array::RecordBatch::new_empty(schema)]]).unwrap())).unwrap();}
        let tables=inputs.iter().map(|input|ClosureTable{relation:model.relation(input.name()).unwrap().clone(),alias:input.name().into()}).collect();SourceCallScopes::prepare_bound(inputs,tables,session,budget).await.unwrap()
    }
    // Preparation is repeated after fixture writes because the edge index is immutable.
    async fn bound(session:&SessionContext,budget:&resources::ResourceBudget,enriched:bool)->SourceCallScopes{
        let inputs=if enriched{execution::enriched_production::EnrichedData::inputs()}else{SourceCallData::inputs()};let model=model().unwrap();let tables=inputs.iter().map(|input|ClosureTable{relation:model.relation(input.name()).unwrap().clone(),alias:input.name().into()}).collect();SourceCallScopes::prepare_bound(inputs,tables,session,budget).await.unwrap()
    }
    fn fixture(session:&SessionContext)->(NormalizedCallEvent,EntityRef,Occurrence,[NormalizedCallAlternative;2],value::Literal){
        let artifact=SourceArtifact::from_bytes(id(1),"scope.py".into(),b"def caller():\n  pass\n").unwrap();let scope=CoverageScope::Artifact{artifact:artifact.id()};let q=AssertionQualification{context:id(2),scope:scope.id(),assumptions:assumptions::AssumptionSet::empty_id(),condition:conditions::Diagram::always().id(),modality:attribution::Modality::Definite,approximation:Approximation::Exact};
        let caller=Occurrence{source:artifact.id(),start:0,end:100,syntax_kind:SyntaxKind::StmtFunctionDef,role:OccurrenceRole::Declaration,structural_path:vec![0]};
        let call=Occurrence{start:80,end:90,syntax_kind:SyntaxKind::ExprCall,role:OccurrenceRole::Syntax,structural_path:vec![0,3],..caller.clone()};let caller_entity=CallableEntity::Source{declaration:caller.id(),kind:CallableKind::Function};let owner=EntityRef::Callable{callable:caller_entity.id()};let membership=OccurrenceOwnership{occurrence:call.id(),owner:caller.id(),entity:owner.id()};
        let rich=Occurrence{start:110,end:120,syntax_kind:SyntaxKind::StmtPass,structural_path:vec![9;2_000_000],..caller.clone()};
        let literal=value::Literal::String{value:"λ".repeat(3<<20).into()};let doc=Occurrence{start:5,end:15,syntax_kind:SyntaxKind::ExprStringLiteral,role:OccurrenceRole::Syntax,structural_path:vec![0,0,0],..caller.clone()};let doc_stmt=Occurrence{start:4,end:16,syntax_kind:SyntaxKind::StmtExpr,structural_path:vec![0,0],..doc.clone()};let detail=SyntaxDetail::Literal{literal:literal.id()};let observation=SyntaxDetailObservation{qualification:q.id(),occurrence:doc.id(),ordinal:0,detail:detail.id()};
        let event=NormalizedCallEvent{site:call.id(),origin:id(8),context:id(2),owner:membership.id()};let mut callees=Vec::new();let mut entities=vec![caller_entity];let mut references=vec![owner.clone()];let mut occurrences=vec![caller.clone(),call.clone(),doc.clone(),doc_stmt.clone(),rich.clone()];let mut declarations=vec![DeclarationObservation{qualification:q.id(),declaration:caller.id(),name:call.id(),kind:DeclarationKind::Function,parent:None,overload:false,docstring:Some(doc.id())}];
        for n in [1,2]{let declaration=Occurrence{start:20*n,end:20*n+10,structural_path:vec![0,n as i32],..caller.clone()};let callable=CallableEntity::Source{declaration:declaration.id(),kind:CallableKind::Function};let reference=EntityRef::Callable{callable:callable.id()};callees.push(NormalizedCallAlternative{event:event.id(),source:id(n as u8+10),resolution:None,correspondence:None,entity:Some(reference.id()),status:ResolutionStatus::Resolved,reason:normalized::links::LinkReason::ExplicitIdentity});entities.push(callable);references.push(reference);occurrences.push(declaration.clone());declarations.push(DeclarationObservation{qualification:q.id(),declaration:declaration.id(),name:call.id(),kind:DeclarationKind::Function,parent:Some(caller.id()),overload:false,docstring:None});}
        register(session,&[artifact.clone()]);register(session,&[input::ArtifactUse{artifact:artifact.id(),input:artifact.input,role:input::SourceRole::Release}]);register(session,&[scope]);register(session,std::slice::from_ref(&q));register(session,&occurrences);register(session,&entities);register(session,&references);register(session,&[membership]);register(session,&[event.clone()]);register(session,&callees);register(session,&declarations);register(session,&[SyntaxPlacement{qualification:q.id(),occurrence:call.id(),parent:Some(caller.id()),field:SyntaxField::Body,ordinal:3},SyntaxPlacement{qualification:q.id(),occurrence:doc_stmt.id(),parent:Some(caller.id()),field:SyntaxField::Body,ordinal:0},SyntaxPlacement{qualification:q.id(),occurrence:doc.id(),parent:Some(doc_stmt.id()),field:SyntaxField::Value,ordinal:0}]);register(session,&[literal.clone()]);register(session,&[detail]);register(session,&[observation]);
        (event,owner,rich,callees.try_into().unwrap(),literal)
    }
    #[tokio::test]
    async fn event_and_owner_closures_keep_all_alternatives_and_project_unused_docstring(){
        let session=SessionContext::new();let budget=resources::ResourceBudget::fixed(4<<20).unwrap();drop(prepare(&session,&budget,true).await);let (event,owner,rich,alternatives,literal)=fixture(&session);drop(literal);
        let prepared=bound(&session,&budget,true).await;let retained=budget.reserved();
        for scope in [prepared.scope(event.id(),&budget).await.unwrap(),prepared.owner_scope(owner.id(),&budget).await.unwrap()] {
            let data=prepared.enriched_data(&scope,&budget).await.unwrap();
            for alternative in &alternatives{assert_eq!(data.source.bindings.event_alternatives.get(alternative.id()),Some(alternative));}
            assert!(data.source.evaluation.occurrences.get(rich.id()).is_none());assert!(data.source.evaluation.literals.is_empty(),"unused rich docstring bytes must not be decoded");
            drop(data);drop(scope);
        }
        assert_eq!(budget.reserved(),retained);drop(prepared);assert_eq!(budget.reserved(),0);
    }
    #[tokio::test]
    async fn empty_root_stream_and_private_spool_preserve_exact_issuer_and_release_resources(){
        let session=SessionContext::new();let budget=resources::ResourceBudget::fixed(4<<20).unwrap();let prepared=prepare(&session,&budget,false).await;
        let definition=execution::configuration::source_calls().1;let frame=AnalysisInvocation::new(id(1),id(2),definition.id(),None,[]).0;
        let mut stream=crate::sql::query(&session,&prepared.roots(&frame).unwrap()).await.unwrap().execute_stream().await.unwrap();while let Some(batch)=stream.try_next().await.unwrap(){assert_eq!(batch.num_rows(),0);}drop(stream);drop(prepared);
        let data=SourceCallData::new(&budget);let (_,issuer,payload)=execution::source_call_records::prepare_event_produced(&data,&frame,&definition,stages::Profile::Catalog,&budget,None,None,None,id(3)).unwrap();let mut spool=SourcePayloadSpool::new(&budget);spool.insert(frame.id(),id(3),payload.bytes()).unwrap();assert!(spool.insert(frame.id(),id(3),payload.bytes()).is_err());drop(payload);
        let retained=budget.reserved();let (bytes,reservation)=spool.read(frame.id(),id(3),&budget).unwrap();let hydrated=issuer.hydrate(&frame,id(3),&bytes,&budget).unwrap();assert!(issuer.hydrate(&frame,id(4),&bytes,&budget).is_err());assert!(spool.read(frame.id(),id(4),&budget).is_err());let foreign=resources::ResourceBudget::fixed(4<<20).unwrap();assert!(spool.read(frame.id(),id(3),&foreign).is_err());drop(hydrated);drop(bytes);drop(reservation);assert_eq!(budget.reserved(),retained);drop(spool);drop(issuer);drop(data);assert_eq!(budget.reserved(),0);
    }
}
