//! Model-owned capture origin receipt. Native snapshots never supply value authority.
use super::{evaluation::EvaluationData,source_call::{Evidence,HeaderError,SourceCallRequest,unique}};
use crate::domain::{assertion::*,captures::*,conditions::entry::EntryData,
    flow::*,flow_capture::*,lexical::*,normalized::{Rows,binding_normalization::BindingData,entities::*},
    obligation::ObligationKind as K,resources::ResourceBudget,source::*,syntax::*,*};
fn need<R:Record>(rows:&Rows<R>,id:Id<R>)->Result<&R,K> {rows.get(id).ok_or(K::MissingEvidence)}
fn include<R:Record>(data:&EvaluationData,evidence:&mut Evidence<'_>,row:&R,q:Id<AssertionQualification>) ->Result<(),HeaderError> {
    if need(&data.qualifications,q)?.assumptions!=assumptions::AssumptionSet::empty_id() {
        return Err(K::CapturedStateUnavailable.into());
    }
    evidence.include(row,q,None)?;Ok(())
}
/// The value source is explicit; it does not change the nested callable's signature.
#[derive(Debug,Clone,PartialEq,Eq,Hash,crate::DomainSum)]
#[model(name="captured_value_sources")]
pub enum CapturedValueSource {
    #[model(code=0)] Entry {formal:Id<ParameterEntity>,parameter:Id<calls::SignatureParameter>,declaration:Id<Occurrence>},
    #[model(code=1)] Literal {literal:Id<value::Literal>,value:Id<Occurrence>,statement:Id<Occurrence>},
}
/// A checked lower source origin, awaiting activation by the actual caller frame.
pub(super) struct CheckedCaptureOrigin {
    pub read:Id<Occurrence>,
    pub caller:Id<EntityRef>,
    pub callee:Id<EntityRef>,
    pub source:CapturedValueSource,
    pub origin:Id<FlowDefinitionObservation>,
    pub capture:Id<CaptureObservation>,
    pub timing:Id<FlowCaptureTimingObservation>,
    pub qualification:Id<AssertionQualification>,
    _charge:charged::StateCharge,
}
impl CheckedCaptureOrigin {
    #[allow(clippy::too_many_arguments,reason="Independent source, native origin and scope inputs stay explicit.")]
    pub(super) fn derive(data:&EvaluationData,flow:&EntryData,bindings:&BindingData,
        request:SourceCallRequest,caller:Id<EntityRef>,callee:Id<EntityRef>,caller_decl:Id<Occurrence>,callee_decl:Id<Occurrence>,
        resolution:&LexicalResolution,evidence:&mut Evidence<'_>,budget:&ResourceBudget)->Result<Self,HeaderError> {
        let mut charge=charged::StateCharge::new(budget,"bounded_capture_origin");charge.grow(size_of::<Self>())?;
        // The bounded initial lane is the direct synchronous, sole return of an immutable literal or outer Source formal.
        let read=need(&data.occurrences,resolution.read)?;
        if read.syntax_kind!=SyntaxKind::ExprName || !resolution.captured {return Err(K::CapturedStateUnavailable.into());}
        let owner=unique(data.owners.iter().filter(|o|o.occurrence==read.id()))?;
        if owner.entity!=callee || owner.owner!=callee_decl {return Err(K::ScopeBoundary.into());}
        let inner_scope=unique(data.lexical_scopes.iter().filter(|s|s.owner==callee_decl&&s.kind==LexicalScopeKind::Function))?;
        let outer_scope=unique(data.lexical_scopes.iter().filter(|s|s.owner==caller_decl&&s.kind==LexicalScopeKind::Function))?;
        let LexicalTarget::Binding {event}=need(&data.lexical_targets,resolution.target)? else {return Err(K::CapturedStateUnavailable.into())};
        let binding_event=need(&data.binding_events,*event)?;
        let binding=unique(data.bindings.iter().filter(|b|b.event==*event&&data.qualifications.get(b.qualification).is_some_and(|q|q.context==request.context)))?;
        if binding.scope!=outer_scope.id() || !matches!(binding.kind,BindingEventKind::Parameter|BindingEventKind::Assignment) {
            return Err(K::CapturedStateUnavailable.into());
        }
        // Same-name writes anywhere inside either body disqualify; scope is checked before hydration.
        for b in data.bindings.iter().filter(|b| b.event!=*event && data.qualifications.get(b.qualification).is_some_and(|q|q.context==request.context)) {
            let event=need(&data.binding_events,b.event)?;
            let site=need(&data.occurrences,event.site)?;
            let outer=need(&data.occurrences,caller_decl)?;
            if event.name==binding_event.name && site.source==outer.source && site.start>=outer.start&&site.end<=outer.end {
                return Err(K::CapturedStateUnavailable.into());
            }
        }
        let body=bindings.placements.iter().filter(|p|p.parent==Some(callee_decl)&&p.field==SyntaxField::Body).collect::<Vec<_>>();
        charge.grow(body.len()*size_of::<&SyntaxPlacement>()*2)?;
        if body.len()!=1 || body[0].ordinal!=0 || need(&data.occurrences,body[0].occurrence)?.syntax_kind!=SyntaxKind::StmtReturn {
            return Err(K::CapturedStateUnavailable.into());
        }
        let value=unique(data.placements.iter().filter(|p|p.parent==Some(body[0].occurrence)&&p.field==SyntaxField::Value))?;
        if value.occurrence!=read.id() {return Err(K::CapturedStateUnavailable.into());}
        // Every direct reference to the nested function must be this exact call's callee.
        let call=need(&bindings.event_events,request.event)?;
        let syntax=unique(data.call_syntax.iter().filter(|s|s.site==call.site&&data.qualifications.get(s.qualification).is_some_and(|q|q.context==request.context)))?;
        let inner=unique(bindings.declarations.iter().filter(|d|d.declaration==callee_decl&&data.qualifications.get(d.qualification).is_some_and(|q|q.context==request.context)))?;
        if inner.kind!=DeclarationKind::Function {return Err(K::CapturedStateUnavailable.into());}
        let name=unique(data.spellings.iter().filter(|s|s.occurrence==inner.name))?;
        for reference in data.references.iter().filter(|r|r.name==name.spelling&&data.qualifications.get(r.qualification).is_some_and(|q|q.context==request.context)) {
            if reference.scope==outer_scope.id() && reference.read!=syntax.callee {return Err(K::CapturedStateUnavailable.into());}
        }
        // Parameter bindings name the Parameter node; ty definitions name its actual
        // Identifier child. Require the canonical supported edge, never a byte-range join.
        let origin_site=if binding.kind==BindingEventKind::Parameter {
            let parameter_site=need(&data.occurrences,binding_event.site)?;
            if parameter_site.syntax_kind!=SyntaxKind::Parameter || parameter_site.role!=OccurrenceRole::Parameter {
                return Err(K::CapturedStateUnavailable.into());
            }
            let child=unique(data.placements.iter().filter(|p|p.parent==Some(binding_event.site)
                &&p.field==SyntaxField::Child&&p.ordinal==0
                &&data.occurrences.get(p.occurrence).is_some_and(|o|o.syntax_kind==SyntaxKind::Identifier)))?;
            let identifier=need(&data.occurrences,child.occurrence)?;
            if identifier.source!=parameter_site.source || identifier.start!=parameter_site.start || identifier.end!=parameter_site.end
                || identifier.structural_path.len()!=parameter_site.structural_path.len()+1
                || !identifier.structural_path.starts_with(&parameter_site.structural_path) {
                return Err(K::CapturedStateUnavailable.into());
            }
            include(data,evidence,child,child.qualification)?;
            child.occurrence
        } else {binding_event.site};
        let definition=unique(flow.definition_observations.iter().filter(|d|d.scope==outer_scope.id()&&d.kind==binding.kind
            &&flow.definitions.get(d.definition).is_some_and(|f|f.occurrence==origin_site)
            &&flow.qualifications.get(d.qualification).is_some_and(|q|q.context==request.context)))?;
        let outer_symbol=unique(flow.symbol_declarations.iter().filter(|d|d.declaration==caller_decl
            &&flow.qualifications.get(d.qualification).is_some_and(|q|q.context==request.context)))?;
        let source=if binding.kind==BindingEventKind::Parameter {
            if binding.value.is_some() || definition.value.is_some() {return Err(K::CapturedStateUnavailable.into());}
            let identifier=need(&data.occurrences,binding_event.site)?;
            let declared=unique(flow.declarations.iter().filter(|d| {
                let Some(site)=flow.occurrences.get(d.declaration) else {return false};
                site.source==identifier.source && site.start<=identifier.start&&site.end>=identifier.end
                    && identifier.structural_path.starts_with(&site.structural_path)
                    && flow.parameters.get(d.parameter).and_then(|p|flow.signatures.get(p.signature)).is_some_and(|s|s.role==calls::SignatureRole::Source)
                    && flow.qualifications.get(d.qualification).is_some_and(|q|q.context==request.context)
            }))?;
            let formal=ParameterEntity::Source {declaration:declared.declaration};
            let link=unique(flow.links.iter().filter(|l|l.entity==formal.id()&&l.parameter==declared.parameter&&l.declaration==Some(declared.id())))?;
            let parameter=need(&flow.parameters,link.parameter)?;
            if need(&flow.signatures,parameter.signature)?.symbol!=outer_symbol.symbol {return Err(K::ScopeBoundary.into());}
            include(data,evidence,declared,declared.qualification)?;
            CapturedValueSource::Entry {formal:formal.id(),parameter:parameter.id(),declaration:declared.declaration}
        } else {
            let value=binding.value.ok_or(K::CapturedStateUnavailable)?;
            if definition.value!=Some(value) {return Err(K::CapturedStateUnavailable.into());}
            let prefix=unique(bindings.placements.iter().filter(|p|p.parent==Some(caller_decl)&&p.field==SyntaxField::Body&&p.ordinal==0))?;
            if !closed_literal_prefix(data,prefix.occurrence,binding_event.site,value)? {return Err(K::CapturedStateUnavailable.into());}
            let detail=unique(data.details.iter().filter(|d|d.occurrence==value&&data.qualifications.get(d.qualification).is_some_and(|q|q.context==request.context)))?;
            let SyntaxDetail::Literal {literal}=need(&data.detail_values,detail.detail)? else {return Err(K::CapturedStateUnavailable.into())};
            need(&data.literals,*literal)?;
            include(data,evidence,prefix,prefix.qualification)?;
            include(data,evidence,detail,detail.qualification)?;
            CapturedValueSource::Literal {literal:*literal,value,statement:prefix.occurrence}
        };
        let inner_symbol=unique(flow.symbol_declarations.iter().filter(|d|d.declaration==callee_decl
            &&flow.qualifications.get(d.qualification).is_some_and(|q|q.context==request.context)))?;
        let capture=unique(data.captures.iter().filter(|c|c.function==inner_symbol.symbol&&c.declaring==Some(outer_symbol.symbol)
            &&c.name==binding_event.name&&c.origin==CaptureOrigin::OuterFunction
            &&data.qualifications.get(c.qualification).is_some_and(|q|q.context==request.context)))?;
        let use_=unique(flow.uses.iter().filter(|u|u.occurrence==read.id()))?;
        let timing=unique(data.capture_timing.iter().filter(|t|t.use_==use_.id()&&t.nested_scope==inner_scope.id()
            &&t.enclosing_scope==outer_scope.id()&&data.qualifications.get(t.qualification).is_some_and(|q|q.context==request.context)))?;
        if timing.origin!=FlowCaptureOrigin::OuterLocal {return Err(K::CapturedStateUnavailable.into());}
        // Timing is supported characterization only. Candidate/AssumeBound is never included
        // in this value proof's unconditional native premise inventory.
        let support=unique(data.capture_timing_supports.iter().filter(|s|s.assertion==timing.id()))?;
        let run=need(&data.runs,support.run)?;
        if run.input!=request.input || run.context!=request.context {return Err(K::IncompatibleContexts.into());}
        for placement in [body[0],value] {include(data,evidence,placement,placement.qualification)?;}
        include(data,evidence,resolution,resolution.qualification)?;
        include(data,evidence,binding,binding.qualification)?;
        include(data,evidence,definition,definition.qualification)?;
        include(data,evidence,outer_symbol,outer_symbol.qualification)?;
        include(data,evidence,inner_symbol,inner_symbol.qualification)?;
        include(data,evidence,capture,capture.qualification)?;
        Ok(Self {read:read.id(),caller,callee,source,
            origin:definition.id(),capture:capture.id(),timing:timing.id(),qualification:resolution.qualification,_charge:charge})
    }
}

/// Conditional on entry to this exact active caller frame and its direct synchronous call.
/// The header's closed origin proof remains separate from this activation.
#[derive(Debug,Clone,PartialEq,Eq,crate::Domain)]
#[model(name="captured_entry_bindings",rule="bounded_own_frame_capture")]
pub struct CapturedEntryBinding {
    #[model(key,premise)] pub invocation:Id<analysis::enriched_execution::AnalysisInvocation>,
    #[model(key,premise)] pub header:Id<super::source_call_records::SourceCallHeader>,
    #[model(key)] pub read:Id<Occurrence>,
    pub caller:Id<EntityRef>,
    pub callee:Id<EntityRef>,
    pub value_source:Id<CapturedValueSource>,
    #[model(premise)] pub origin:Id<FlowDefinitionObservation>,
    #[model(premise)] pub capture:Id<CaptureObservation>,
    /// Characterization only, including missing/lazy native snapshot states.
    pub timing:Id<FlowCaptureTimingObservation>,
    pub qualification:Id<AssertionQualification>,
    pub under_caller_entry:bool,
}
pub(super) struct CheckedCapturedEntry { pub row:CapturedEntryBinding,pub source:CapturedValueSource }
impl CheckedCapturedEntry {
    pub(super) fn activate(origin:&CheckedCaptureOrigin,header:&super::source_call::CheckedSourceBinding,
        header_row:&super::source_call_records::SourceCallHeader,invocation:&analysis::enriched_execution::AnalysisInvocation,
        data:&EvaluationData)->Result<Self,ModelError> {
        let r=header.request();
        if (r.input,r.context,header.caller(),header.callee())!=(invocation.input,invocation.context,origin.caller,origin.callee)
            || header_row.event!=r.event || header_row.owner!=origin.caller || header_row.callee!=origin.callee
            || data.qualifications.get(origin.qualification)!=data.qualifications.get(header.qualification()) {
            return Err(ModelError::Invalid("captured activation changes exact caller frame".into()));
        }
        Ok(Self {row:CapturedEntryBinding {invocation:invocation.id(),header:header_row.id(),read:origin.read,
            caller:origin.caller,callee:origin.callee,value_source:origin.source.id(),
            origin:origin.origin,capture:origin.capture,timing:origin.timing,qualification:header.qualification(),under_caller_entry:true},source:origin.source.clone()})
    }
}
pub fn relations()->Vec<Relation> {vec![Relation::of::<CapturedEntryBinding>(),Relation::of::<CapturedValueSource>()]}

/// One first local assignment of a primitive literal. No destructuring or setter is admitted.
pub(super) fn closed_literal_prefix(data:&EvaluationData,statement:Id<Occurrence>,target:Id<Occurrence>,value:Id<Occurrence>) ->Result<bool,K> {
    if need(&data.occurrences,statement)?.syntax_kind!=SyntaxKind::StmtAssign || need(&data.occurrences,target)?.syntax_kind!=SyntaxKind::ExprName {return Ok(false);}
    let children=data.placements.iter().filter(|p|p.parent==Some(statement)).collect::<Vec<_>>();
    if children.len()!=2 || !children.iter().any(|p|p.field==SyntaxField::Target&&p.occurrence==target&&p.ordinal==0)
        || !children.iter().any(|p|p.field==SyntaxField::Value&&p.occurrence==value) {return Ok(false);}
    Ok(matches!(need(&data.occurrences,value)?.syntax_kind,SyntaxKind::ExprNoneLiteral|SyntaxKind::ExprBooleanLiteral|SyntaxKind::ExprNumberLiteral|SyntaxKind::ExprStringLiteral|SyntaxKind::ExprBytesLiteral))
}
impl CheckedCaptureOrigin {
    pub(super) fn literal_prefix(&self)->Option<(Id<Occurrence>,Id<Occurrence>)> {
        match &self.source {CapturedValueSource::Literal {statement,value,..}=>Some((*statement,*value)),_=>None}
    }
}
