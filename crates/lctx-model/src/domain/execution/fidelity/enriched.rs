//! Portable Enriched owner and ordered membership properties. No evaluator is invoked.
use super::*;
use crate::domain::execution::{enriched_records as e, modeled_call as m, definition as d, context_binding as b, context_execution as c, capture_bridge as capture};

pub(super) fn name(kind:Kind)->&'static str {match kind {
    Kind::EnrichedStatement=>"enriched_statement_fidelity",Kind::EnrichedBody=>"enriched_body_fidelity",
    Kind::Fresh=>"enriched_fresh_fidelity",Kind::Modeled=>"enriched_modeled_fidelity",
    Kind::Definition=>"enriched_definition_fidelity",Kind::ContextBinding=>"enriched_binding_fidelity",
    Kind::Context=>"enriched_context_fidelity",Kind::Capture=>"enriched_capture_fidelity",Kind::EnrichedFrames=>"enriched_frame_fidelity",_=>unreachable!(),
}}
pub(super) fn root(kind:Kind)->ValidationInput {match kind {
    Kind::EnrichedStatement=>ValidationInput::of::<e::StatementExecution>(&["id"]),Kind::EnrichedBody=>ValidationInput::of::<e::BodyExecution>(&["id"]),
    Kind::Fresh=>ValidationInput::of::<e::SourceExecutionInvocation>(&["id"]),Kind::Modeled=>ValidationInput::of::<m::ModeledCallEvaluation>(&["id"]),
    Kind::Definition=>ValidationInput::of::<d::DefinitionEvaluation>(&["id"]),Kind::ContextBinding=>ValidationInput::of::<b::ContextEntryBinding>(&["id"]),
    Kind::Context=>ValidationInput::of::<c::ContextExecution>(&["id"]),Kind::Capture=>ValidationInput::of::<capture::CapturedEntryBinding>(&["id"]),Kind::EnrichedFrames=>ValidationInput::of::<analysis::enriched_execution::AnalysisInvocation>(&["id"]),_=>unreachable!(),
}}
pub(super) fn scope(kind:Kind)->ExecutionScope {
    let mut memberships=vec![
        (ValidationInput::of::<OccurrenceOwnership>(&["id"]),"occurrence",ValidationInput::of::<Occurrence>(&["id"])),
        (ValidationInput::of::<input::ArtifactUse>(&["id"]),"artifact",ValidationInput::of::<SourceArtifact>(&["id"])),
    ];
    let mut joins=Vec::new();
    macro_rules! own {($member:ty,$field:literal,$owner:ty)=>{memberships.push((ValidationInput::of::<$member>(&["id"]),$field,ValidationInput::of::<$owner>(&["id"])));};}
    macro_rules! join {($source:ty,$source_key:literal,$member:ty,$member_key:literal,$context:ident)=>{joins.push(ExecutionMembership {source:ValidationInput::of::<$source>(&["id"]),source_key:$source_key,member:ValidationInput::of::<$member>(&["id"]),member_key:$member_key,context:Some(ExecutionMembershipContext::$context)});};}
    match kind {
        Kind::EnrichedStatement=>{own!(e::ExecutionMember,"execution",e::StatementExecution);own!(e::EnteredStatement,"execution",e::StatementExecution);}
        Kind::EnrichedBody=>{own!(e::BodyMember,"body",e::BodyExecution);own!(e::BodyReleaseInput,"body",e::BodyExecution);}
        Kind::Fresh=>{own!(e::SourceExecutionArgument,"call",e::SourceExecutionInvocation);own!(CallBinding,"attempt",CallBindingAttempt);own!(CallArgument,"call",CallSyntax);}
        Kind::Modeled=>{own!(m::ModeledCallArgument,"call",m::ModeledCallEvaluation);own!(m::ModeledCallNative,"call",m::ModeledCallEvaluation);own!(CallBinding,"attempt",CallBindingAttempt);own!(CallArgument,"call",CallSyntax);}
        Kind::Definition=>{own!(d::DefinitionMember,"definition",d::DefinitionEvaluation);join!(d::DefinitionEvaluation,"statement",syntax::ParameterSyntaxObservation,"function",EnrichedInvocation);}
        Kind::ContextBinding=>{own!(b::BindingMember,"binding",b::ContextEntryBinding);join!(b::ContextEntryBinding,"item",lexical::SyntaxPlacement,"parent",EnrichedInvocation);}
        Kind::Context=>{own!(c::ContextMember,"execution",c::ContextExecution);own!(c::ContextItem,"execution",c::ContextExecution);join!(c::ContextExecution,"statement",lexical::SyntaxPlacement,"parent",EnrichedInvocation);join!(c::ContextItem,"item",lexical::SyntaxPlacement,"parent",EnrichedContextItem);}
        Kind::Capture=>{},_=>unreachable!(),
    }
    ExecutionScope {root:root(kind),memberships,joins}
}
pub(super) fn inputs(kind:Kind)->Vec<ValidationInput> {
    let mut inputs=vec![ValidationInput::of::<analysis::enriched_execution::AnalysisInvocation>(&["id"]),ValidationInput::of::<analysis::MethodParameters>(&["id"]),ValidationInput::of::<OccurrenceOwnership>(&["id"])];
    macro_rules! add {($($ty:ty),* $(,)?)=>{{$(inputs.push(ValidationInput::of::<$ty>(&["id"]));)*}};}
    match kind {
        Kind::EnrichedStatement=>add!(NormalizedCallEvent,e::ExecutionSource,e::ExecutionMember,e::EnteredStatement,ExpressionEvaluation,analysis::base_evaluation::AnalysisInvocation,SourceInvocation,SourceFrameRelease,SourceCallHeader,analysis::source_call::AnalysisInvocation,e::SourceExecutionInvocation,m::ModeledCallEvaluation,d::DefinitionEvaluation,c::ContextExecution,b::ContextEntryBinding,capture::CapturedEntryBinding),
        Kind::EnrichedBody=>add!(EntityRef,CallableEntity,e::BodySource,e::BodyMember,e::BodyReleaseInput,e::StatementExecution),
        Kind::Fresh=>add!(e::SourceExecutionArgument,e::BodyExecution,SourceCallHeader,analysis::source_call::AnalysisInvocation,NormalizedCallEvent,EntityRef,CallableEntity,CallBindingAttempt,CallBinding,SignatureSlot,SignatureParameter,BindingSource,BindingProjection,CallSyntax,CallArgument,ExpressionEvaluation,analysis::base_evaluation::AnalysisInvocation),
        Kind::Modeled=>add!(m::ModeledCallArgument,m::ModeledCallNative,models::AuthoredModel,CallBindingAttempt,CallBinding,SignatureSlot,SignatureParameter,BindingSource,BindingProjection,NormalizedCallEvent,CallSyntax,CallArgument,ExpressionEvaluation,analysis::base_evaluation::AnalysisInvocation),
        Kind::Definition=>add!(d::DefinitionSource,d::DefinitionMember,syntax::ParameterSyntaxObservation,ExpressionEvaluation,analysis::base_evaluation::AnalysisInvocation),
        Kind::ContextBinding=>add!(b::BindingSource,b::BindingMember,models::AuthoredContextProtocol,ProviderSymbol,lexical::SyntaxPlacement,ExpressionEvaluation,analysis::base_evaluation::AnalysisInvocation),
        Kind::Context=>add!(CallTarget,CallDestination,SignatureEnumerationObservation,c::ContextSource,c::ContextMember,c::ContextItem,models::AuthoredContextProtocol,ProviderSymbol,lexical::SyntaxPlacement,ExpressionEvaluation,analysis::base_evaluation::AnalysisInvocation,e::StatementExecution),
        Kind::Capture=>add!(NormalizedCallEvent,SourceCallHeader,analysis::source_call::AnalysisInvocation,capture::CapturedValueSource,ParameterEntity),_=>unreachable!(),
    }
    inputs
}
impl Check {
    fn enriched_frame(&self,id:Id<analysis::enriched_execution::AnalysisInvocation>)->Result<(&analysis::enriched_execution::AnalysisInvocation,Id<models::ModelCatalog>),ModelError> {
        // The independent compact enriched_frame_fidelity admission owns complete parent
        // membership, including absent Enriched frames. Per-value checks only borrow that frame.
        let frame=need(&self.enriched_frames,id)?;let definition=need(&self.definitions,frame.definition)?;
        let parameters=need(&self.method_parameters,definition.parameters)?;
        let catalog=parameters.model_catalog.ok_or_else(||invalid("Enriched catalog absent"))?;
        if frame.subject.is_some() || crate::domain::execution::configuration::enriched_execution(catalog)!=(parameters.clone(),definition.clone()) {return Err(invalid("Enriched selected definition/frame"));}
        Ok((frame,catalog))
    }
    fn enriched_site(&self,invocation:Id<analysis::enriched_execution::AnalysisInvocation>,qualification:Id<AssertionQualification>,site:Id<Occurrence>,owner:Id<EntityRef>)->Result<&analysis::enriched_execution::AnalysisInvocation,ModelError> {
        let (frame,_)=self.enriched_frame(invocation)?;
        self.frame(frame.input,frame.context,frame.subject,qualification,site)?;self.owner(site,owner)?;Ok(frame)
    }
    fn base_source(&self,id:Id<ExpressionEvaluation>,frame:&analysis::enriched_execution::AnalysisInvocation,owner:Id<EntityRef>)->Result<&ExpressionEvaluation,ModelError> {
        let evaluation=need(&self.evaluations,id)?;let parent=need(&self.base,evaluation.invocation)?;
        if parent.subject.is_some() || (parent.input,parent.context,evaluation.owner)!=(frame.input,frame.context,owner) {return Err(invalid("Enriched Base source frame/owner"));}
        self.frame(parent.input,parent.context,parent.subject,evaluation.qualification,evaluation.expression)?;self.owner(evaluation.expression,owner)?;Ok(evaluation)
    }
    fn same_enriched(&self,id:Id<analysis::enriched_execution::AnalysisInvocation>,frame:&analysis::enriched_execution::AnalysisInvocation)->Result<(),ModelError> {
        if id!=frame.id() {return Err(invalid("Enriched source belongs to another invocation"));}Ok(())
    }
    fn source_header_frame(&self,header:&SourceCallHeader,frame:&analysis::enriched_execution::AnalysisInvocation)->Result<(),ModelError> {
        let parent=need(&self.source,header.invocation)?;
        if parent.subject.is_some() || (parent.input,parent.context)!=(frame.input,frame.context) {return Err(invalid("Enriched SourceCall parent frame"));}
        self.definition(parent.definition,super::super::configuration::source_calls().1)?;
        let event=need(&self.events,header.event)?;if event.context!=frame.context {return Err(invalid("Enriched header event context"));}
        self.frame(frame.input,frame.context,frame.subject,header.qualification,event.site)?;self.owner(event.site,header.owner)?;Ok(())
    }
    fn enriched_body(&self,row:&e::BodyExecution)->Result<&analysis::enriched_execution::AnalysisInvocation,ModelError> {
        let (frame,_)=self.enriched_frame(row.invocation)?;
        self.frame(frame.input,frame.context,frame.subject,row.qualification,row.declaration)?;self.declaration(row.owner,row.declaration)?;Ok(frame)
    }
    fn argument_domain(&self,attempt_id:Id<CallBindingAttempt>,event_id:Id<NormalizedCallEvent>,owner:Id<EntityRef>,site:Id<Occurrence>,frame:&analysis::enriched_execution::AnalysisInvocation,args:impl Iterator<Item=(i64,Id<SignatureParameter>,Id<Occurrence>,Id<ExpressionEvaluation>)>,digest:ContentHash)->Result<Vec<(Id<SignatureParameter>,Id<Occurrence>)>,ModelError> {
        let attempt=need(&self.attempts,attempt_id)?;if attempt.outcome!=BindingOutcome::Bound {return Err(invalid("Enriched argument binding is not Bound"));}let event=need(&self.events,event_id)?;
        let event_owner=need(&self.owners,event.owner)?;
        if attempt.event!=event.id() || event.site!=site || event.context!=frame.context || event_owner.occurrence!=site || event_owner.entity!=owner {return Err(invalid("Enriched argument event/owner"));}
        let syntax=need(&self.syntax,attempt.syntax.ok_or_else(||invalid("Enriched argument syntax"))?)?;
        if syntax.site!=site || syntax.in_annotation {return Err(invalid("Enriched argument call site"));}
        let count=self.bindings.len()+self.call_arguments.len();let _domain=self.budget.reserve("enriched-fidelity-arguments",count.checked_mul(256).ok_or_else(||invalid("argument allowance"))?)?;
        let mut expected=std::collections::BTreeSet::new();
        for binding in self.bindings.iter().filter(|binding|binding.attempt==attempt.id()) {
            let parameter=need(&self.parameters,need(&self.slots,binding.slot)?.parameter)?;
            if Some(parameter.signature)!=attempt.signature {return Err(invalid("Enriched formal belongs to another signature"));}
            match need(&self.binding_sources,binding.source)? {
                BindingSource::Actual {occurrence}=>{if need(&self.projections,binding.projection)?!=&BindingProjection::Whole || !expected.insert((parameter.id(),*occurrence)) {return Err(invalid("Enriched formal/actual projection or duplicate"));}}
                BindingSource::EmptyVarargs|BindingSource::EmptyKwargs=>{},_=>return Err(invalid("Enriched argument is not actual")),
            }
        }
        let syntax_args=ordered(self.call_arguments.iter().filter(|argument|argument.call==syntax.id()),|argument|argument.ordinal)?;
        let mut actual=std::collections::BTreeSet::new();let mut pairs=Vec::new();let mut sink=KeySink::new("source-frame-arguments");
        for (ordinal,(index,formal,occurrence,evaluation)) in args.enumerate() {
            if index!=ordinal as i64 || !actual.insert((formal,occurrence)) || syntax_args.get(ordinal).is_none_or(|argument|argument.value!=occurrence || !matches!(argument.kind,ArgumentKind::Positional|ArgumentKind::Keyword)) {return Err(invalid("Enriched ordered actual membership"));}
            if self.base_source(evaluation,frame,owner)?.expression!=occurrence {return Err(invalid("Enriched argument evaluation targets another occurrence"));}
            formal.encode(&mut sink);occurrence.encode(&mut sink);evaluation.encode(&mut sink);pairs.push((formal,occurrence));
        }
        if actual!=expected || pairs.len()!=syntax_args.len() || sink.finish()!=digest {return Err(invalid("Enriched complete argument domain/digest"));}Ok(pairs)
    }
    fn member_source(&self,source:&e::ExecutionSource,frame:&analysis::enriched_execution::AnalysisInvocation,owner:Id<EntityRef>)->Result<(),ModelError> {
        match source {
            e::ExecutionSource::Native {..}=>{},
            e::ExecutionSource::BaseEvaluation {evaluation}=>{self.base_source(*evaluation,frame,owner)?;}
            e::ExecutionSource::SourceInvocation {invocation}=>{let call=need(&self.invocations,*invocation)?;let header=need(&self.headers,need(&self.releases,call.release)?.header)?;self.source_header_frame(header,frame)?;if header.owner!=owner || call.invocation!=header.invocation {return Err(invalid("Enriched source invocation owner/frame"));}}
            e::ExecutionSource::FreshHeader {header}=>{let header=need(&self.headers,*header)?;self.source_header_frame(header,frame)?;if header.owner!=owner {return Err(invalid("Enriched header owner"));}}
            e::ExecutionSource::ModeledCall {call}=>{let call=need(&self.modeled,*call)?;self.same_enriched(call.invocation,frame)?;if call.owner!=owner {return Err(invalid("Enriched modeled source owner"));}}
            e::ExecutionSource::FreshSource {call}=>{let call=need(&self.fresh,*call)?;self.same_enriched(call.invocation,frame)?;if need(&self.headers,call.header)?.owner!=owner {return Err(invalid("Enriched fresh source owner"));}}
            e::ExecutionSource::Definition {definition}=>{let row=need(&self.definition_values,*definition)?;self.same_enriched(row.invocation,frame)?;if row.owner!=owner {return Err(invalid("Enriched definition source owner"));}}
            e::ExecutionSource::ContextBinding {binding}=>{let row=need(&self.context_bindings,*binding)?;self.same_enriched(row.invocation,frame)?;if row.owner!=owner {return Err(invalid("Enriched binding source owner"));}}
            e::ExecutionSource::CapturedEntry {binding}=>{let row=need(&self.captured,*binding)?;self.same_enriched(row.invocation,frame)?;if row.callee!=owner {return Err(invalid("Enriched captured source callee"));}}
            e::ExecutionSource::Context {context}=>{let row=need(&self.contexts,*context)?;self.same_enriched(row.invocation,frame)?;if row.owner!=owner {return Err(invalid("Enriched context source owner"));}}
        }Ok(())
    }
    pub(super) fn finish_enriched(&self)->Result<(),ModelError> {
        let _scratch=self.budget.reserve("enriched-fidelity-members",self.member_allowance()?)?;
        match self.kind {
            Kind::EnrichedFrames=>self.finish_enriched_frames()?,
            Kind::EnrichedStatement=>for row in self.enriched_statements.iter() {
                let frame=self.enriched_site(row.invocation,row.qualification,row.statement,row.owner)?;
                if !crate::domain::execution::completion_production::is_statement(need(&self.occurrences,row.statement)?.syntax_kind) {return Err(invalid("Enriched statement source kind"));}
                let members=ordered(self.enriched_members.iter().filter(|member|member.execution==row.id()),|member|member.ordinal)?;let mut sources=Vec::new();
                for member in members {let source=need(&self.enriched_sources,member.source)?;self.member_source(source,frame,row.owner)?;sources.push(source.id());}
                let entered=ordered(self.enriched_entered.iter().filter(|member|member.execution==row.id()),|member|member.ordinal)?;
                for member in &entered {self.owner(member.statement,row.owner)?;if need(&self.occurrences,member.statement)?.source!=need(&self.occurrences,row.statement)?.source {return Err(invalid("Enriched entered statement source"));}}
                if ordered_digest("execution-sources",sources)!=row.sources || ordered_digest("execution-entered",entered.iter().map(|member|member.statement))!=row.entered {return Err(invalid("Enriched statement ordered member digest"));}
            },
            Kind::EnrichedBody=>for row in self.enriched_bodies.iter() {
                self.enriched_body(row)?;let members=ordered(self.enriched_body_members.iter().filter(|member|member.body==row.id()),|member|member.ordinal)?;let mut sources=Vec::new();
                for member in members {let source=need(&self.enriched_body_sources,member.source)?;if let e::BodySource::Statement {execution}=source {let statement=need(&self.enriched_statements,*execution)?;if statement.invocation!=row.invocation || statement.owner!=row.owner {return Err(invalid("Enriched body member frame/owner"));}}sources.push(source.id());}
                let releases=ordered(self.enriched_releases.iter().filter(|member|member.body==row.id()),|member|member.ordinal)?;let mut digest=KeySink::new("execution-body-releases");
                for release in releases {self.owner(release.expression,row.owner)?;if need(&self.occurrences,release.expression)?.source!=need(&self.occurrences,row.declaration)?.source {return Err(invalid("Enriched release expression source"));}release.expression.encode(&mut digest);release.safety.encode(&mut digest);}
                if ordered_digest("execution-body-sources",sources)!=row.sources || digest.finish()!=row.releases {return Err(invalid("Enriched body ordered member/release digest"));}
            },
            Kind::Fresh=>for row in self.fresh.iter() {
                let (frame,_)=self.enriched_frame(row.invocation)?;let header=need(&self.headers,row.header)?;let body=need(&self.enriched_bodies,row.body)?;
                self.source_header_frame(header,frame)?;self.enriched_body(body)?;
                if body.invocation!=row.invocation || (body.owner,body.declaration,body.qualification)!=(header.callee,header.declaration,header.qualification) || row.event!=header.event || row.qualification!=header.qualification {return Err(invalid("Enriched fresh header/body/frame correspondence"));}
                let args=ordered(self.fresh_arguments.iter().filter(|argument|argument.call==row.id()),|argument|argument.ordinal)?;
                self.argument_domain(header.attempt,row.event,header.owner,need(&self.events,row.event)?.site,frame,args.into_iter().map(|argument|(argument.ordinal,argument.formal,argument.actual,argument.evaluation)),row.arguments)?;
            },
            Kind::Modeled=>for row in self.modeled.iter() {
                let frame=self.enriched_site(row.invocation,row.qualification,row.expression,row.owner)?;
                let (_,catalog)=self.enriched_frame(row.invocation)?;if row.catalog!=catalog || need(&self.authored_models,row.model)?.catalog!=catalog {return Err(invalid("Enriched modeled catalog ownership"));}
                let args=ordered(self.modeled_arguments.iter().filter(|argument|argument.call==row.id()),|argument|argument.ordinal)?;
                // The modeled owner uses the same ordered tuple framing under its own namespace.
                let mut digest=KeySink::new("source-frame-arguments");let mut modeled=KeySink::new("modeled-call-arguments");
                for argument in &args {argument.formal.encode(&mut digest);argument.actual.encode(&mut digest);argument.evaluation.encode(&mut digest);argument.formal.encode(&mut modeled);argument.actual.encode(&mut modeled);argument.evaluation.encode(&mut modeled);}
                let domain=self.argument_domain(row.attempt,row.event,row.owner,row.expression,frame,args.into_iter().map(|argument|(argument.ordinal,argument.formal,argument.actual,argument.evaluation)),digest.finish())?;
                if modeled.finish()!=row.arguments || !domain.contains(&(row.returned_formal,row.returned_actual)) {return Err(invalid("Enriched returned parameter is not an actual member"));}
            },
            Kind::Definition=>for row in self.definition_values.iter() {self.definition_membership(row)?;},
            Kind::ContextBinding=>for row in self.context_bindings.iter() {self.context_binding_membership(row)?;},
            Kind::Context=>for row in self.contexts.iter() {self.context_membership(row)?;},
            Kind::Capture=>for row in self.captured.iter() {
                let (frame,_)=self.enriched_frame(row.invocation)?;let header=need(&self.headers,row.header)?;self.source_header_frame(header,frame)?;
                self.frame(frame.input,frame.context,frame.subject,row.qualification,row.read)?;self.owner(row.read,row.callee)?;
                if !row.under_caller_entry || (row.caller,row.callee,row.qualification)!=(header.owner,header.callee,header.qualification) {return Err(invalid("Enriched captured entry owner/header"));}
                match need(&self.captured_values,row.value_source)? {
                    capture::CapturedValueSource::Entry {formal,declaration,..}=>{if need(&self.parameter_entities,*formal)!=&(ParameterEntity::Source {declaration:*declaration}) {return Err(invalid("Enriched captured entry parameter declaration"));}self.owner(*declaration,row.caller)?;}
                    capture::CapturedValueSource::Literal {value,statement,..}=>{self.owner(*value,row.caller)?;self.owner(*statement,row.caller)?;if need(&self.occurrences,*value)?.source!=need(&self.occurrences,row.read)?.source || need(&self.occurrences,*statement)?.source!=need(&self.occurrences,row.read)?.source {return Err(invalid("Enriched captured literal source"));}}
                }
            },_=>unreachable!(),
        }Ok(())
    }
    fn member_allowance(&self)->Result<usize,ModelError> {
        [self.enriched_members.len(),self.enriched_entered.len(),self.enriched_body_members.len(),self.enriched_releases.len(),self.fresh_arguments.len(),self.modeled_arguments.len(),self.definition_members.len(),self.parameter_syntax.len(),self.context_binding_members.len(),self.context_members.len(),self.context_items.len(),self.placements.len()].into_iter().try_fold(0usize,|n,count|n.checked_add(count)).and_then(|n|n.checked_mul(256)).ok_or_else(||invalid("Enriched member allowance"))
    }
    fn definition_membership(&self,row:&d::DefinitionEvaluation)->Result<(),ModelError> {
        let frame=self.enriched_site(row.invocation,row.qualification,row.statement,row.owner)?;
        if need(&self.occurrences,row.statement)?.syntax_kind!=SyntaxKind::StmtFunctionDef {return Err(invalid("Enriched definition source kind"));}
        let parameters=ordered(self.parameter_syntax.iter().filter(|parameter|parameter.function==row.statement && self.qualifications.get(parameter.qualification).is_some_and(|q|q.context==frame.context)),|parameter|parameter.ordinal)?;
        let members=ordered(self.definition_members.iter().filter(|member|member.definition==row.id()),|member|member.ordinal)?;let mut defaults=std::collections::BTreeSet::new();
        for member in members {if let d::DefinitionSource::Default {evaluation}=need(&self.definition_sources,member.source)? {if self.base_source(*evaluation,frame,row.owner)?.qualification!=row.qualification {return Err(invalid("Enriched default qualification differs"));}if !defaults.insert(*evaluation) {return Err(invalid("duplicate Enriched default source"));}}}
        let mut expected=std::collections::BTreeSet::new();let mut digest=KeySink::new("definition-default-evaluations");
        for parameter in parameters {if let Some(default)=parameter.default {let mut matches=defaults.iter().filter(|evaluation|self.evaluations.get(**evaluation).is_some_and(|evaluation|evaluation.expression==default));let evaluation=matches.next().ok_or_else(||invalid("Enriched default evaluation absent"))?;if matches.next().is_some() {return Err(invalid("Enriched default evaluation ambiguous"));}expected.insert(*evaluation);default.encode(&mut digest);evaluation.encode(&mut digest);}}
        if expected!=defaults || digest.finish()!=row.defaults {return Err(invalid("Enriched complete default member domain"));}Ok(())
    }
    fn context_binding_membership(&self,row:&b::ContextEntryBinding)->Result<(),ModelError> {
        let frame=self.enriched_site(row.invocation,row.qualification,row.access,row.owner)?;let (_,catalog)=self.enriched_frame(row.invocation)?;
        if need(&self.occurrences,row.access)?.syntax_kind!=SyntaxKind::ExprName || need(&self.protocols,row.protocol)?.catalog!=catalog || (need(&self.provider_symbols,row.class)?.context,need(&self.provider_symbols,row.class)?.kind)!=(frame.context,SymbolKind::Class) {return Err(invalid("Enriched context binding native/catalog owner"));}
        self.item_sites(row.item,row.site,Some(row.target),frame,row.owner,row.qualification)?;
        let members=ordered(self.context_binding_members.iter().filter(|member|member.binding==row.id()),|member|member.ordinal)?;let mut sources=Vec::new();let mut actuals=std::collections::BTreeSet::new();
        for member in members {let source=need(&self.context_binding_sources,member.source)?;if let b::BindingSource::Actual {evaluation}=source {actuals.insert(self.base_source(*evaluation,frame,row.owner)?.expression);}sources.push(source.id());}
        if ordered_digest("context-binding-sources",sources)!=row.sources || row.entry_actual.is_some_and(|actual|!actuals.contains(&actual)) {return Err(invalid("Enriched context binding member domain"));}Ok(())
    }
    fn item_sites(&self,item:Id<Occurrence>,site:Id<Occurrence>,target:Option<Id<Occurrence>>,frame:&analysis::enriched_execution::AnalysisInvocation,owner:Id<EntityRef>,qualification:Id<AssertionQualification>)->Result<(),ModelError> {
        if need(&self.occurrences,item)?.syntax_kind!=SyntaxKind::WithItem {return Err(invalid("Enriched context item source kind"));}
        self.owner(item,owner)?;self.owner(site,owner)?;self.frame(frame.input,frame.context,frame.subject,qualification,site)?;
        let values=self.placements.iter().filter(|placement|placement.parent==Some(item) && self.placement_context(placement,frame.context) && placement.field==lexical::SyntaxField::Value).collect::<Vec<_>>();
        if values.len()!=1 || values[0].occurrence!=site {return Err(invalid("Enriched context item value correspondence"));}
        if let Some(target)=target {let targets=self.placements.iter().filter(|placement|placement.parent==Some(item) && self.placement_context(placement,frame.context) && placement.field==lexical::SyntaxField::Target).collect::<Vec<_>>();if targets.len()!=1 || targets[0].occurrence!=target {return Err(invalid("Enriched context target correspondence"));}self.owner(target,owner)?;}
        Ok(())
    }
    fn placement_context(&self,placement:&lexical::SyntaxPlacement,context:Id<AnalysisContext>)->bool {
        self.qualifications.get(placement.qualification).is_some_and(|qualification|qualification.context==context)
    }
    fn context_membership(&self,row:&c::ContextExecution)->Result<(),ModelError> {
        let frame=self.enriched_site(row.invocation,row.qualification,row.statement,row.owner)?;let (_,catalog)=self.enriched_frame(row.invocation)?;
        if need(&self.occurrences,row.statement)?.syntax_kind!=SyntaxKind::StmtWith {return Err(invalid("Enriched context source kind"));}
        let syntax_items=ordered(self.placements.iter().filter(|placement|placement.parent==Some(row.statement) && self.placement_context(placement,frame.context) && placement.field==lexical::SyntaxField::Item),|placement|placement.ordinal)?;
        let items=ordered(self.context_items.iter().filter(|item|item.execution==row.id()),|item|item.ordinal)?;
        if items.len()!=syntax_items.len() || items.is_empty() {return Err(invalid("Enriched complete context item domain"));}
        for (item,syntax) in items.iter().zip(syntax_items) {if item.item!=syntax.occurrence || need(&self.protocols,item.protocol)?.catalog!=catalog || (need(&self.provider_symbols,item.class)?.context,need(&self.provider_symbols,item.class)?.kind)!=(frame.context,SymbolKind::Class) {return Err(invalid("Enriched context item catalog/source"));}self.item_sites(item.item,item.site,None,frame,row.owner,row.qualification)?;
            let allocation=need(&self.call_targets,item.allocation)?;let initialization=need(&self.call_targets,item.initialization)?;let enumeration=need(&self.signature_enumerations,item.enumeration)?;
            let symbol=need(&self.call_destinations,initialization.destination)?.symbol().ok_or_else(||invalid("Enriched initializer destination"))?;
            if allocation.site!=item.site || initialization.site!=item.site || allocation.phase!=CallPhase::New || initialization.phase!=CallPhase::Init || initialization.receiver_class!=Some(item.class)
                || !enumeration.complete || enumeration.symbol!=symbol || need(&self.qualifications,allocation.qualification)?.context!=frame.context || need(&self.qualifications,initialization.qualification)?.context!=frame.context || need(&self.qualifications,enumeration.qualification)?.context!=frame.context {return Err(invalid("Enriched context item target/enumeration ownership"));}}
        let suite=ordered(self.placements.iter().filter(|placement|placement.parent==Some(row.statement) && self.placement_context(placement,frame.context) && placement.field==lexical::SyntaxField::Body),|placement|placement.ordinal)?;
        let mut actual_body=std::collections::BTreeMap::new();let mut actuals=std::collections::BTreeSet::new();
        let members=ordered(self.context_members.iter().filter(|member|member.execution==row.id()),|member|member.ordinal)?;let mut sources=Vec::new();
        for member in members {let source=need(&self.context_sources,member.source)?;match source {c::ContextSource::Actual {evaluation}=>{actuals.insert(self.base_source(*evaluation,frame,row.owner)?.expression);}c::ContextSource::Body {statement}=>{let body=need(&self.enriched_statements,*statement)?;if body.invocation!=row.invocation || body.owner!=row.owner {return Err(invalid("Enriched context body member frame/owner"));}if actual_body.insert(body.statement,body.outcome).is_some() {return Err(invalid("duplicate Enriched context body statement"));}}c::ContextSource::Native {..}=>{}}sources.push(source.id());}
        let mut expected_body=std::collections::BTreeSet::new();
        if suite.is_empty() {return Err(invalid("Enriched context body empty"));}
        for placement in suite {let outcome=actual_body.get(&placement.occurrence).ok_or_else(||invalid("Enriched context entered body member absent"))?;expected_body.insert(placement.occurrence);if *outcome!=e::ExecutionOutcome::Normal.id() {break;}}
        if actual_body.keys().copied().collect::<std::collections::BTreeSet<_>>()!=expected_body || items.iter().any(|item|item.entry_actual.is_some_and(|actual|!actuals.contains(&actual))) {return Err(invalid("Enriched complete context body/entry domain"));}
        let mut items_digest=KeySink::new("context-items");
        for item in items {item.item.encode(&mut items_digest);item.site.encode(&mut items_digest);item.class.encode(&mut items_digest);item.protocol.encode(&mut items_digest);item.allocation.encode(&mut items_digest);item.initialization.encode(&mut items_digest);item.enumeration.encode(&mut items_digest);item.entry_actual.encode(&mut items_digest);item.exit_input.encode(&mut items_digest);item.exit_output.encode(&mut items_digest);item.suppressed.encode(&mut items_digest);}
        if items_digest.finish()!=row.items || ordered_digest("context-sources",sources)!=row.sources {return Err(invalid("Enriched context ordered member digest"));}Ok(())
    }
}

/// Complete frame topology is compact; it must run even with no advertised Enriched rows.
pub(super) fn frame_inputs()->Vec<ValidationInput> {
    vec![ValidationInput::of::<analysis::enriched_execution::AnalysisInvocation>(&["id"]),ValidationInput::of::<analysis::source_call::AnalysisInvocation>(&["id"]),ValidationInput::of::<analysis::enriched_execution::AnalysisInput>(&["id"]),ValidationInput::of::<analysis::enriched_execution::InvocationSource>(&["id"]),ValidationInput::of::<analysis::AnalysisDefinition>(&["id"]),ValidationInput::of::<analysis::MethodParameters>(&["id"])]
}
impl Check {
    fn finish_enriched_frames(&self)->Result<(),ModelError> {
        let count=self.source.len().checked_add(self.enriched_frames.len()).and_then(|n|n.checked_add(self.enriched_inputs.len())).ok_or_else(||invalid("Enriched frame allowance"))?;
        let _scratch=self.budget.reserve("enriched-fidelity-frame-topology",count.checked_mul(256).ok_or_else(||invalid("Enriched frame allowance"))?)?;
        let mut expected=std::collections::BTreeMap::<_,std::collections::BTreeSet<_>>::new();
        for source in self.source.iter() {
            self.definition(source.definition,super::super::configuration::source_calls().1)?;
            if source.subject.is_some() {return Err(invalid("Enriched predecessor is not whole SourceCall frame"));}
            expected.entry((source.input,source.context)).or_default().insert(analysis::enriched_execution::InvocationSource::SourceCallAnalysis {invocation:source.id()}.id());
        }
        let mut actual=std::collections::BTreeSet::new();
        for frame in self.enriched_frames.iter() {
            self.enriched_frame(frame.id())?;
            let key=(frame.input,frame.context);
            if !actual.insert(key) {return Err(invalid("Enriched frame domain duplicate"));}
            let parents=expected.get(&key).ok_or_else(||invalid("Enriched frame has no SourceCall frame"))?;
            let mut observed=std::collections::BTreeSet::new();
            for input in self.enriched_inputs.iter().filter(|input|input.invocation==frame.id()) {
                let analysis::enriched_execution::InvocationSource::SourceCallAnalysis {invocation}=need(&self.enriched_parents,input.parent)? else {return Err(invalid("Enriched parent is not SourceCallAnalysis"));};
                if need(&self.source,*invocation).is_err() || !observed.insert(input.parent) {return Err(invalid("Enriched parent missing or duplicate"));}
            }
            let mut digest=KeySink::new("analysis-invocation-inputs");for parent in parents {parent.encode(&mut digest);}
            if &observed!=parents || digest.finish()!=frame.inputs {return Err(invalid("Enriched exact complete SourceCall parent domain"));}
        }
        if actual.len()!=expected.len() || expected.keys().any(|key|!actual.contains(key)) {return Err(invalid("Enriched complete actual frame domain"));}
        if self.enriched_inputs.iter().any(|input|self.enriched_frames.get(input.invocation).is_none()) {return Err(invalid("Enriched parent membership has no invocation"));}
        Ok(())
    }
}
