//! Completion certificates describe what happens if a statement is entered, never that it ran.
//! Source observations and expression proofs are explicit inputs. No provider or store is read.
use std::collections::{HashMap, BTreeSet};
use cpg_schema::behavior::{ExpressionEvaluationsRow, ExpressionEvaluationStepsRow, ExitSitesRow, ReturnExitStatusesRow,
    ReturnExitStepsRow, ReturnEntryStatusesRow, ReturnEntryStepsRow, StatementCompletionsRow, StatementCompletionStepsRow};
use cpg_schema::codebook::{BindingKind, BoundaryReason, Codebook, CompletionKind as C,
    ExitSiteKind, LexicalScopeKind, SummaryFlowStepKind as K, SyntaxField as F, SyntaxKind as S};
use cpg_schema::id::Id;
use cpg_schema::context_protocol::{ModelContextProtocolsRow,SourceContextSitesRow,SourceContextArgumentsRow};
use cpg_schema::tables::{SyntaxNodesRow, BindingsRow, ScopesRow, FlowTestsRow, DeclarationsRow, ParameterSyntaxRow};
use cpg_schema::condition_kernel::Diagram;
use crate::summaries::finite::condition_limit;
use cpg_schema::summary_contract::{ExactRuntimeException, CompletionOutcome as Completion};
use cpg_schema::behavior::HandlerTypesRow;
use cpg_schema::tables::{ContextDefinitionsRow,ContextClassMroRow,ContextModulesRow};
use cpg_schema::codebook::{HandlerTypeStatus,DefinitionKind,ModuleOrigin};

const MAX_DEPTH: usize = 128;
const MAX_WORK: usize = 4096;
type Proof = Vec<(K, Id)>;
type Result = std::result::Result<Completion, BoundaryReason>;

pub struct Inputs<'a> {
    pub context_protocols: &'a [ModelContextProtocolsRow],
    pub context_sites: &'a [SourceContextSitesRow],
    pub context_arguments: &'a [SourceContextArgumentsRow],
    pub declarations: &'a [DeclarationsRow],
    pub parameters: &'a [ParameterSyntaxRow],
    pub syntax: &'a [SyntaxNodesRow],
    pub expressions: &'a [ExpressionEvaluationsRow],
    pub expression_steps: &'a [ExpressionEvaluationStepsRow],
    pub bindings: &'a [BindingsRow],
    pub scopes: &'a [ScopesRow],
    pub exits: &'a [ExitSitesRow],
    pub handler_types: &'a [HandlerTypesRow],
    pub classes: &'a [ContextDefinitionsRow],
    pub mro: &'a [ContextClassMroRow],
    pub modules: &'a [ContextModulesRow],
    pub tests: &'a [FlowTestsRow],
    /// Requested (snapshot, return-site fact, candidate condition), never a function cross product.
    pub entry_conditions: &'a [(Id, Id, Id)],
    pub boundaries: &'a HashMap<Id, BoundaryReason>,
    pub diagrams: &'a HashMap<Id,Diagram>,
}

#[derive(Default)]
pub struct Outcome {
    pub certificates: Vec<cpg_schema::completion_proof::ReturnCompletionCertificatesRow>,
    pub statements: Vec<StatementCompletionsRow>,
    pub statement_steps: Vec<StatementCompletionStepsRow>,
    pub returns: Vec<ReturnExitStatusesRow>,
    pub return_steps: Vec<ReturnExitStepsRow>,
    pub entries: Vec<ReturnEntryStatusesRow>,
    pub entry_steps: Vec<ReturnEntryStepsRow>,
}

/// For source-admitted eager synchronous scopes, complete statement outcomes close the completion and escaping-exception domains under
/// entry to this site. A normal/return/break/continue outcome has an exhaustive empty escaping
/// exception domain. This says nothing about whether the enclosing callable reaches the site,
/// exceptions caught within it, other channels, or deferred execution.
pub fn coverage(statements: &[StatementCompletionsRow]) -> Vec<cpg_schema::behavior::SummaryOriginCoverageRow> {
    use cpg_schema::codebook::{InvocationPhase, SummaryChannel};
    statements.iter().flat_map(|statement| {
        let (subject_kind,subject_id,source_fact_id,parameter_node_id)=
            cpg_schema::summary_contract::SummarySubject::ExecutionSite {
                node_id:statement.statement_node_id,syntax_fact_id:statement.source_fact_id,
            }.columns();
        [SummaryChannel::Completion,SummaryChannel::Exception].map(|channel| {
            let complete=statement.kind!=C::Unknown && statement.reason.is_none();
            cpg_schema::behavior::SummaryOriginCoverageRow {
                snapshot_id:statement.snapshot_id,function_node_id:statement.function_node_id,
                subject_kind,subject_id,source_fact_id,parameter_node_id,
                condition_id:Diagram::always().id(),channel,phase:InvocationPhase::Call,
                complete,reason:if complete {None} else {Some(statement.reason.unwrap_or(BoundaryReason::MissingEvidence))},
                witness_count:i64::from(complete && (channel==SummaryChannel::Completion || statement.kind==C::Raise)),
                witnesses_omitted:false,
            }
        })
    }).collect()
}

/// Definition-time header evidence is prepared separately: the body belongs to the new
/// function and is neither evaluated nor checked as part of its enclosing function's suite.
struct DefinitionHeader<'a> {
    binding: &'a BindingsRow,
    defaults: Vec<&'a SyntaxNodesRow>,
    evidence: Vec<Id>,
}

fn definition_headers<'a>(inputs: &Inputs<'a>, children: &HashMap<(Id,Id),Vec<&'a SyntaxNodesRow>>)
    -> HashMap<(Id,Id),DefinitionHeader<'a>> {
    let mut declarations:HashMap<_,Vec<_>>=HashMap::new();
    for row in inputs.declarations {declarations.entry((row.snapshot_id,row.node_id)).or_default().push(row);}
    let mut parameters:HashMap<_,Vec<_>>=HashMap::new();
    for row in inputs.parameters {parameters.entry((row.snapshot_id,row.function_node_id)).or_default().push(row);}
    for rows in parameters.values_mut() {rows.sort_by_key(|p|(p.ordinal,p.fact_id));}
    let scopes:HashMap<_,_>=inputs.scopes.iter().map(|s|((s.snapshot_id,s.node_id),s)).collect();
    let mut bindings:HashMap<_,Vec<_>>=HashMap::new();
    let mut names:HashMap<_,usize>=HashMap::new();
    let mut generic=BTreeSet::new();
    for row in inputs.bindings {
        bindings.entry((row.snapshot_id,row.site_node_id)).or_default().push(row);
        *names.entry((row.snapshot_id,row.scope_id,row.name.as_str())).or_default()+=1;
        if row.kind==BindingKind::TypeParam {
            if let Some(scope)=scopes.get(&(row.snapshot_id,row.scope_id)) {
                generic.insert((row.snapshot_id,scope.owner_node_id));
            }
        }
    }
    let mut out=HashMap::new();
    for node in inputs.syntax.iter().filter(|n|n.kind==S::StmtFunctionDef) {
        let key=(node.snapshot_id,node.node_id);
        let Some(owner)=node.owner_node_id else {continue;};
        if node.parent_node_id!=owner || node.field!=F::Body || generic.contains(&key) {continue;}
        let Some(decls)=declarations.get(&key).filter(|rows|rows.len()==1) else {continue;};
        let declaration=decls[0];
        if declaration.kind!=cpg_schema::codebook::DeclarationKind::Function
            || declaration.parent_node_id!=Some(owner) || !declaration.decorators.is_empty()
            || declaration.module_node_id!=node.module_node_id
            || declaration.start_byte!=node.start_byte || declaration.end_byte!=node.end_byte {continue;}
        let Some(writes)=bindings.get(&key).filter(|rows|rows.len()==1) else {continue;};
        let binding=writes[0];
        let Some(scope)=scopes.get(&(binding.snapshot_id,binding.scope_id)) else {continue;};
        if binding.kind!=BindingKind::FunctionDef || binding.name!=declaration.name
            || binding.module_node_id!=node.module_node_id || scope.kind!=LexicalScopeKind::Function
            || scope.owner_node_id!=owner || names.get(&(binding.snapshot_id,binding.scope_id,binding.name.as_str()))!=Some(&1) {continue;}
        let body=children.get(&key).map_or(&[][..],Vec::as_slice);
        let statements:Vec<_>=body.iter().filter(|n|n.field==F::Body).collect();
        if statements.is_empty() || statements.iter().enumerate().any(|(i,n)|n.ordinal!=i as i64)
            || body.iter().any(|n|n.module_node_id!=node.module_node_id || n.start_byte<node.start_byte
                || n.end_byte>node.end_byte || match n.field {
                    F::Body=>n.owner_node_id!=Some(node.node_id),
                    F::Default=>n.owner_node_id!=node.owner_node_id,
                    _=>true,
                }) {continue;}
        let params=parameters.get(&key).map_or(&[][..],Vec::as_slice);
        if params.len()>128 || params.iter().enumerate().any(|(i,p)|p.ordinal!=i as i64)
            || params.iter().map(|p|&p.name).collect::<BTreeSet<_>>().len()!=params.len()
            || params.iter().map(|p|p.fact_id).collect::<BTreeSet<_>>().len()!=params.len() {continue;}
        let mut defaults=Vec::new();
        let mut valid=true;
        for parameter in params {
            match (parameter.default_start_byte,parameter.default_end_byte,parameter.default_text.as_ref()) {
                (None,None,None)=>{},
                (Some(start),Some(end),Some(_))=>{
                    let found:Vec<_>=body.iter().filter(|n|n.field==F::Default
                        && n.start_byte==start && n.end_byte==end).collect();
                    if found.len()!=1 || found[0].ordinal!=defaults.len() as i64 {valid=false;break;}
                    defaults.push(**found.first().unwrap());
                },
                _=>{valid=false;break;},
            }
        }
        if !valid || defaults.len()!=body.iter().filter(|n|n.field==F::Default).count() {continue;}
        let mut evidence=vec![declaration.fact_id,binding.fact_id,scope.fact_id];
        evidence.extend(params.iter().map(|p|p.fact_id));
        out.insert(key,DefinitionHeader {binding,defaults,evidence});
    }
    out
}

#[derive(Clone)]
struct PreparedContext<'a> {
    site:&'a SourceContextSitesRow,
    protocol:&'a ModelContextProtocolsRow,
    arguments:Vec<(&'a SourceContextArgumentsRow,&'a SyntaxNodesRow)>,
}

struct Kernel<'a> {
    contexts: HashMap<(Id,Id),PreparedContext<'a>>,
    definitions: HashMap<(Id,Id),DefinitionHeader<'a>>,
    children: HashMap<(Id, Id), Vec<&'a SyntaxNodesRow>>,
    expressions: HashMap<(Id, Id), &'a ExpressionEvaluationsRow>,
    expression_steps: HashMap<(Id, Id), Vec<&'a ExpressionEvaluationStepsRow>>,
    writes: HashMap<(Id, Id), Vec<&'a BindingsRow>>,
    initializations: BTreeSet<Id>,
    scopes: HashMap<(Id, Id), &'a ScopesRow>,
    tests: HashMap<(Id,Id,i64,i64),Vec<&'a FlowTestsRow>>,
    diagrams: &'a HashMap<Id,Diagram>,
    boundaries: &'a HashMap<Id, BoundaryReason>,
    handler_types: HashMap<(Id,Id),Vec<&'a HandlerTypesRow>>,
    classes: HashMap<(Id,String,String),Vec<&'a ContextDefinitionsRow>>,
    mro: HashMap<(Id,Id),Vec<&'a ContextClassMroRow>>,
    modules: HashMap<(Id,Id),Vec<&'a ContextModulesRow>>,
    active_exception: Option<ExactRuntimeException>,
    assumption: Option<Id>,
    entry_mode: bool,
    initialized: BTreeSet<(Id,Id,String)>,
    path_initializations: BTreeSet<Id>,
    remaining: usize,
    proof: Proof,
}

impl<'ctx> Kernel<'ctx> {
    fn context_items(&self,node:&SyntaxNodesRow,children:&[&SyntaxNodesRow])
        ->std::result::Result<Vec<PreparedContext<'ctx>>,BoundaryReason> {
        if node.detail.as_deref()==Some("async") || children.iter().any(|n|!matches!(n.field,F::Item|F::Body)) {
            return Err(BoundaryReason::UnsupportedControlFlow);
        }
        let mut items:Vec<_>=children.iter().copied().filter(|n|n.field==F::Item).collect();
        items.sort_by_key(|n|n.ordinal);
        if items.is_empty() || items.len()>128 || items.iter().enumerate().any(|(i,n)|n.ordinal!=i as i64 || n.kind!=S::WithItem) {
            return Err(BoundaryReason::MissingEvidence);
        }
        items.into_iter().map(|item| {
            let context=self.contexts.get(&(node.snapshot_id,item.node_id)).ok_or(BoundaryReason::UnsupportedControlFlow)?;
            if context.site.with_node_id!=node.node_id || context.site.with_fact_id!=node.fact_id
                || context.site.item_fact_id!=item.fact_id || context.site.item_ordinal!=item.ordinal {
                return Err(BoundaryReason::MissingEvidence);
            }
            Ok(context.clone())
        }).collect()
    }

    fn enter_contexts(&mut self,node:&SyntaxNodesRow,children:&[&SyntaxNodesRow])
        ->std::result::Result<(Vec<PreparedContext<'ctx>>,Completion),BoundaryReason> {
        let items=self.context_items(node,children)?;
        let mut entered=Vec::new();
        for context in items {
            self.remaining=self.remaining.checked_sub(1+context.arguments.len()).ok_or(BoundaryReason::CompletionWorkLimit)?;
            for (_,argument) in &context.arguments {self.expression(argument)?;}
            self.proof.push((K::ContextConstruction,context.site.site_id));
            if !context.site.constructor_valid {
                return Ok((entered,Completion::raised(context.site.expression_fact_id,ExactRuntimeException::TypeError)));
            }
            self.proof.push((K::ContextEntry,context.site.site_id));
            // Register before assignment: a failed `as` assignment still runs this exit.
            entered.push(context.clone());
            let pending=self.assign_context(&context)?;
            if pending!=Completion::Normal {return Ok((entered,pending));}
        }
        Ok((entered,Completion::Normal))
    }

    fn assign_context(&mut self,context:&PreparedContext<'_>)->Result {
        let site=context.site;
        let children=self.children.get(&(site.snapshot_id,site.item_node_id)).cloned().unwrap_or_default();
        let targets:Vec<_>=children.iter().copied().filter(|n|n.field==F::Target).collect();
        if targets.is_empty() {return Ok(Completion::Normal);}
        let [target]=targets.as_slice() else {return Err(BoundaryReason::MissingEvidence);};
        self.remaining=self.remaining.checked_sub(1).ok_or(BoundaryReason::CompletionWorkLimit)?;
        self.proof.push((K::ExpressionSyntax,target.fact_id));
        if target.kind==S::ExprName {
            let bindings=self.writes.get(&(site.snapshot_id,target.node_id)).ok_or(BoundaryReason::MissingEvidence)?;
            let [binding]=bindings.as_slice() else {return Err(BoundaryReason::MissingEvidence);};
            let scope=self.scopes.get(&(site.snapshot_id,binding.scope_id)).ok_or(BoundaryReason::MissingEvidence)?;
            if binding.kind!=BindingKind::WithTarget || scope.kind!=LexicalScopeKind::Function
                || scope.owner_node_id!=site.function_node_id || !self.initializations.contains(&binding.fact_id)
                || !self.initialized.insert((site.snapshot_id,binding.scope_id,binding.name.clone())) {
                return Err(BoundaryReason::UnsupportedControlFlow);
            }
            self.proof.push((K::LocalAssignmentBinding,binding.fact_id));
            return Ok(Completion::Normal);
        }
        let none=context.protocol.entry==cpg_schema::codebook::ContextEntryKind::NoneValue
            || context.site.entry_argument_fact_id.is_none()
            || context.arguments.iter().any(|(a,n)|Some(a.argument_fact_id)==site.entry_argument_fact_id && n.kind==S::ExprNoneLiteral);
        if none && matches!(target.kind,S::ExprTuple|S::ExprList) {
            // Iterator acquisition fails before any target assignment, so no old value is
            // released. Other unpacking and attribute/subscript assignment remain open.
            return Ok(Completion::raised(target.fact_id,ExactRuntimeException::TypeError));
        }
        Err(BoundaryReason::UnsupportedControlFlow)
    }

    fn exit_contexts(&mut self,entered:&[PreparedContext<'_>],mut pending:Completion)->Result {
        let previous=self.active_exception;
        let result=(|| {
            for context in entered.iter().rev() {
                self.remaining=self.remaining.checked_sub(1).ok_or(BoundaryReason::CompletionWorkLimit)?;
                self.active_exception=pending.exception().or(previous);
                if let Some(exception)=pending.exception() {
                    if context.protocol.exit==cpg_schema::codebook::ContextExitKind::SuppressClasses {
                        pending=self.suppress_context(context,exception,pending)?;
                    }
                }
                self.proof.push((K::ContextExit,context.site.site_id));
            }
            Ok(pending)
        })();
        self.active_exception=previous;
        result
    }

    /// contextlib.suppress uses issubclass, not exception-handler matching. Only pinned
    /// builtin classes have no user metaclass hook here; nonclasses can raise during exit.
    fn suppress_context(&mut self,context:&PreparedContext<'_>,exception:ExactRuntimeException,pending:Completion)->Result {
        for (argument,node) in &context.arguments {
            self.remaining=self.remaining.checked_sub(1).ok_or(BoundaryReason::CompletionWorkLimit)?;
            let Some(class_id)=argument.exception_class_node_id else {
                if matches!(node.kind,S::ExprNoneLiteral|S::ExprNumberLiteral|S::ExprStringLiteral|S::ExprBytesLiteral
                    |S::ExprBooleanLiteral|S::ExprEllipsisLiteral) {
                    return Ok(Completion::raised(context.site.item_fact_id,ExactRuntimeException::TypeError));
                }
                return Err(BoundaryReason::UnsupportedControlFlow);
            };
            let (module,name)=exception.class();
            let raised=self.pinned_class(context.site.snapshot_id,module,name)?;
            let raised_id=raised.symbol_node_id;
            let mut evidence=vec![raised.fact_id,argument.exception_class_fact_id.ok_or(BoundaryReason::MissingEvidence)?,
                argument.exception_module_fact_id.ok_or(BoundaryReason::MissingEvidence)?,
                argument.reference_fact_id.ok_or(BoundaryReason::MissingEvidence)?,argument.resolution_fact_id.ok_or(BoundaryReason::MissingEvidence)?];
            let classes:Vec<_>=self.classes.values().flatten().filter(|c|c.snapshot_id==context.site.snapshot_id
                && c.symbol_node_id==class_id).collect();
            let [class]=classes.as_slice() else {return Err(BoundaryReason::MissingEvidence);};
            if class.module_name!="builtins" || class.fact_id!=argument.exception_class_fact_id.unwrap() {return Err(BoundaryReason::MissingEvidence);}
            let mut matches=raised_id==class_id;
            if !matches {
                let mro=self.mro.get(&(context.site.snapshot_id,raised_id)).ok_or(BoundaryReason::MissingEvidence)?;
                if mro.is_empty() || mro.iter().enumerate().any(|(i,a)|a.cyclic || !a.linearization_complete
                    || a.ordinal!=Some(i as i64) || a.ancestor_module.is_none() || a.ancestor_key.is_none()) {
                    return Err(BoundaryReason::MissingEvidence);
                }
                matches=mro.iter().any(|a|a.ancestor_module.as_deref()==Some(&class.module_name) && a.ancestor_key.as_deref()==Some(&class.key));
                evidence.extend(mro.iter().map(|a|a.fact_id));
            }
            self.remaining=self.remaining.checked_sub(evidence.len()).ok_or(BoundaryReason::CompletionWorkLimit)?;
            self.proof.extend(evidence.into_iter().map(|id|(K::ContextExceptionEvidence,id)));
            if matches {return Ok(Completion::Normal);}
        }
        Ok(pending)
    }

    fn entry(&mut self, exit:&ExitSitesRow, nodes:&HashMap<(Id,Id),&SyntaxNodesRow>)
        -> std::result::Result<(),BoundaryReason> {
        let mut current=*nodes.get(&(exit.snapshot_id,exit.site_node_id)).ok_or(BoundaryReason::MissingEvidence)?;
        let mut path=Vec::new();
        let mut visited=BTreeSet::new();
        loop {
            if path.len()>=MAX_DEPTH { return Err(BoundaryReason::CompletionDepthLimit); }
            if !visited.insert(current.node_id) { return Err(BoundaryReason::MissingEvidence); }
            let parent=*nodes.get(&(exit.snapshot_id,current.parent_node_id)).ok_or(BoundaryReason::MissingEvidence)?;
            path.push((current,parent));
            if parent.node_id==exit.function_node_id && parent.kind==S::StmtFunctionDef { break; }
            if parent.owner_node_id!=Some(exit.function_node_id) { return Err(BoundaryReason::MissingEvidence); }
            current=parent;
        }
        for (node,parent) in path.into_iter().rev() {
            self.remaining=self.remaining.checked_sub(1).ok_or(BoundaryReason::CompletionWorkLimit)?;
            let children=self.children.get(&(exit.snapshot_id,parent.node_id)).cloned().unwrap_or_default();
            self.remaining=self.remaining.checked_sub(children.len()).ok_or(BoundaryReason::CompletionWorkLimit)?;
            match parent.kind {
                S::StmtFunctionDef if parent.node_id==exit.function_node_id && node.field==F::Body => {},
                S::StmtTry if parent.detail.as_deref()!=Some("except*") => {
                    match node.field {
                        F::Body => {},
                        F::Orelse => {
                            if self.suite(&children,F::Body,0)?.kind()!=C::Normal {
                                return Err(BoundaryReason::RuntimeUnreachable);
                            }
                        },
                        F::Handler => {
                            let pending=self.suite(&children,F::Body,0)?;
                            if pending.kind()!=C::Raise {return Err(BoundaryReason::RuntimeUnreachable);}
                            let exception=pending.exception().ok_or(BoundaryReason::MissingEvidence)?;
                            let handler=self.selected_handler(parent,&children,exception)?
                                .ok_or(BoundaryReason::RuntimeUnreachable)?;
                            if handler.node_id!=node.node_id {return Err(BoundaryReason::RuntimeUnreachable);}
                            self.enter_handler(handler,exception)?;
                        },
                        F::Finalbody => {
                            let pending=self.try_body(parent,&children,0)?;
                            if pending.kind()==C::Raise {self.active_exception=pending.exception();}
                        },
                        _ => return Err(BoundaryReason::UnsupportedControlFlow),
                    }
                },
                S::StmtWith if node.field==F::Body => {
                    let (_,pending)=self.enter_contexts(parent,&children)?;
                    if pending!=Completion::Normal {return Err(BoundaryReason::RuntimeUnreachable);}
                },
                S::ExceptHandlerExceptHandler if node.field==F::Body => {},
                S::ElifElseClause if node.field==F::Body => {},
                S::StmtIf => {
                    let truth=self.truth(Self::field(&children,F::Test)?)?;
                    if truth {
                        if node.field!=F::Body { return Err(BoundaryReason::RuntimeUnreachable); }
                    } else {
                        if node.field!=F::Orelse { return Err(BoundaryReason::RuntimeUnreachable); }
                        let mut selected=None;
                        for clause in children.iter().filter(|n|n.field==F::Orelse) {
                            if clause.kind!=S::ElifElseClause { return Err(BoundaryReason::MissingEvidence); }
                            let body=self.children.get(&(exit.snapshot_id,clause.node_id)).cloned().unwrap_or_default();
                            self.remaining=self.remaining.checked_sub(1+body.len()).ok_or(BoundaryReason::CompletionWorkLimit)?;
                            if !body.iter().any(|n|n.field==F::Test) || self.truth(Self::field(&body,F::Test)?)? {
                                selected=Some(clause.node_id);break;
                            }
                        }
                        if selected!=Some(node.node_id) { return Err(BoundaryReason::RuntimeUnreachable); }
                    }
                },
                _ => return Err(BoundaryReason::UnsupportedControlFlow),
            }
            // Orelse clauses are alternatives, not a preceding sequential suite.
            if (parent.kind==S::StmtIf && node.field==F::Orelse)
                || (parent.kind==S::StmtTry && node.field==F::Handler) { continue; }
            let prefix:Vec<_>=children.iter().copied().filter(|n|n.field==node.field && n.ordinal<node.ordinal).collect();
            if prefix.len()!=usize::try_from(node.ordinal).map_err(|_|BoundaryReason::MissingEvidence)?
                || prefix.iter().enumerate().any(|(i,n)|n.ordinal!=i as i64) {
                return Err(BoundaryReason::MissingEvidence);
            }
            for statement in prefix {
                // A function docstring initializes __doc__ at definition time; it is not
                // evaluated again on invocation.
                if parent.kind==S::StmtFunctionDef && statement.ordinal==0 && statement.kind==S::StmtExpr {
                    let value=self.children.get(&(exit.snapshot_id,statement.node_id));
                    if value.is_some_and(|v|v.len()==1 && v[0].field==F::Value && v[0].kind==S::ExprStringLiteral) {continue;}
                }
                if self.statement(statement,0)?.kind()!=C::Normal { return Err(BoundaryReason::RuntimeUnreachable); }
            }
        }
        Ok(())
    }

    fn selected_handler<'a>(&mut self,node:&SyntaxNodesRow,children:&[&'a SyntaxNodesRow],exception:ExactRuntimeException)
        -> std::result::Result<Option<&'a SyntaxNodesRow>,BoundaryReason> {
        let handlers:Vec<_>=children.iter().copied().filter(|n|n.field==F::Handler).collect();
        if handlers.iter().enumerate().any(|(i,h)|h.ordinal!=i as i64 || h.kind!=S::ExceptHandlerExceptHandler) {
            return Err(BoundaryReason::MissingEvidence);
        }
        for handler in handlers {
            let body=self.children.get(&(node.snapshot_id,handler.node_id)).cloned().unwrap_or_default();
            self.remaining=self.remaining.checked_sub(1+body.len()).ok_or(BoundaryReason::CompletionWorkLimit)?;
            if self.handler_matches(handler,&body,exception)? {return Ok(Some(handler));}
        }
        Ok(None)
    }

    fn enter_handler(&mut self,handler:&SyntaxNodesRow,exception:ExactRuntimeException)
        -> std::result::Result<(),BoundaryReason> {
        // Binding and implicit deletion of a handler name can execute user cleanup.
        if handler.detail.as_deref().is_some_and(|name|!name.is_empty()) {
            return Err(BoundaryReason::HandlerNameCleanup);
        }
        self.proof.push((K::CompletionStatement,handler.fact_id));
        self.active_exception=Some(exception);
        Ok(())
    }

    /// The try/handler/else portion has one owner; entering a finalizer and completing the
    /// whole statement must agree on the pending outcome and ordered evidence.
    fn try_body(&mut self,node:&SyntaxNodesRow,children:&[&SyntaxNodesRow],depth:usize) -> Result {
        let mut pending=self.suite(children,F::Body,depth)?;
        if pending.kind()==C::Raise {
            let exception=pending.exception().ok_or(BoundaryReason::MissingEvidence)?;
            if let Some(handler)=self.selected_handler(node,children,exception)? {
                let previous=self.active_exception;
                self.enter_handler(handler,exception)?;
                let body=self.children.get(&(node.snapshot_id,handler.node_id)).cloned().unwrap_or_default();
                let result=self.suite(&body,F::Body,depth+1);
                self.active_exception=previous;
                pending=result?;
            }
        } else if pending.kind()==C::Normal {
            pending=self.suite(children,F::Orelse,depth)?;
        }
        Ok(pending)
    }

    fn pinned_class(&self,snapshot:Id,module:&str,name:&str)->std::result::Result<&ContextDefinitionsRow,BoundaryReason> {
        let classes=self.classes.get(&(snapshot,module.to_owned(),name.to_owned())).ok_or(BoundaryReason::MissingEvidence)?;
        if classes.len()!=1 || classes[0].kind!=DefinitionKind::Class {return Err(BoundaryReason::MissingEvidence);}
        let class=classes[0];
        let modules=self.modules.get(&(snapshot,class.module_node_id)).ok_or(BoundaryReason::MissingEvidence)?;
        if modules.len()!=1 || modules[0].origin!=ModuleOrigin::BundledTypeshed || modules[0].module_name!=module {
            return Err(BoundaryReason::MissingEvidence);
        }
        Ok(class)
    }

    fn handler_matches(&mut self,handler:&SyntaxNodesRow,body:&[&SyntaxNodesRow],exception:ExactRuntimeException)
        ->std::result::Result<bool,BoundaryReason> {
        if body.iter().all(|n|n.field==F::Body) {return Ok(true);}
        let test=Self::field(body,F::Test)?;
        if body.iter().any(|n|!matches!(n.field,F::Body|F::Test)) {return Err(BoundaryReason::UnsupportedControlFlow);}
        let rows=self.handler_types.get(&(handler.snapshot_id,handler.node_id)).ok_or(BoundaryReason::MissingEvidence)?;
        if rows.len()!=1 {return Err(BoundaryReason::MissingEvidence);}
        let row=rows[0];
        if row.status!=HandlerTypeStatus::PinnedBuiltin || row.reason.is_some() {return Err(row.reason.unwrap_or(BoundaryReason::UnsupportedControlFlow));}
        if Some(row.function_node_id)!=handler.owner_node_id || row.type_node_id!=Some(test.node_id) || row.type_fact_id!=Some(test.fact_id) {
            return Err(BoundaryReason::MissingEvidence);
        }
        let (module,name)=exception.class();
        let raised=self.pinned_class(handler.snapshot_id,module,name)?;
        let caught=self.pinned_class(handler.snapshot_id,"builtins",row.class_name.as_deref().ok_or(BoundaryReason::MissingEvidence)?)?;
        if row.class_node_id!=Some(caught.symbol_node_id) || row.class_fact_id!=Some(caught.fact_id)
            || row.class_module_fact_id!=Some(self.modules[&(handler.snapshot_id,caught.module_node_id)][0].fact_id) {
            return Err(BoundaryReason::MissingEvidence);
        }
        let mut evidence=vec![test.fact_id,row.reference_fact_id.ok_or(BoundaryReason::MissingEvidence)?,
            row.resolution_fact_id.ok_or(BoundaryReason::MissingEvidence)?,caught.fact_id,
            row.class_module_fact_id.ok_or(BoundaryReason::MissingEvidence)?,raised.fact_id];
        let mut matched=raised.symbol_node_id==caught.symbol_node_id;
        let mut work=evidence.len();
        if !matched {
            let ancestors=self.mro.get(&(handler.snapshot_id,raised.symbol_node_id)).ok_or(BoundaryReason::MissingEvidence)?;
            work+=ancestors.len();
            if work>self.remaining {return Err(BoundaryReason::CompletionWorkLimit);}
            if ancestors.iter().enumerate().any(|(i,a)|a.cyclic || a.ordinal!=Some(i as i64) || a.ancestor_module.is_none() || a.ancestor_key.is_none() || a.ancestor_name.is_none()) {
                return Err(BoundaryReason::MissingEvidence);
            }
            matched=ancestors.iter().any(|a|a.ancestor_module.as_deref()==Some(caught.module_name.as_str()) && a.ancestor_key.as_deref()==Some(caught.key.as_str()));
            evidence.extend(ancestors.iter().map(|a|a.fact_id));
            if !matched {
                // Only an exact raised class and a complete resolved MRO justify nonmatch.
                if ancestors.is_empty() || ancestors.iter().any(|a|!a.linearization_complete) {
                    return Err(BoundaryReason::MissingEvidence);
                }
                let caught_mro=self.mro.get(&(handler.snapshot_id,caught.symbol_node_id)).ok_or(BoundaryReason::MissingEvidence)?;
                work+=caught_mro.len();
                if work>self.remaining {return Err(BoundaryReason::CompletionWorkLimit);}
                if caught_mro.iter().any(|a|a.cyclic) || !caught_mro.iter().any(|a|a.ancestor_module.as_deref()==Some("builtins") && a.ancestor_name.as_deref()==Some("BaseException")) {
                    return Err(BoundaryReason::UnsupportedControlFlow);
                }
                evidence.extend(caught_mro.iter().map(|a|a.fact_id));
            }
        }
        self.remaining=self.remaining.checked_sub(work).ok_or(BoundaryReason::CompletionWorkLimit)?;
        self.proof.extend(evidence.into_iter().map(|id|(K::HandlerClassEvidence,id)));
        Ok(matched)
    }

    fn truth(&mut self, node:&SyntaxNodesRow) -> std::result::Result<bool,BoundaryReason> {
        if node.kind==S::ExprCompare && self.assumption.is_some() {
            // Under a source-site predicate assumption, successful comparison/truth testing
            // is part of the premise. Operand evaluation is still independently required.
            // No unconditional expression certificate or stable primitive link is produced.
            let children=self.children.get(&(node.snapshot_id,node.node_id)).cloned().unwrap_or_default();
            self.remaining=self.remaining.checked_sub(1+children.len()).ok_or(BoundaryReason::CompletionWorkLimit)?;
            if children.iter().any(|c|c.owner_node_id!=node.owner_node_id || c.module_node_id!=node.module_node_id
                || c.start_byte<node.start_byte || c.end_byte>node.end_byte) {return Err(BoundaryReason::MissingEvidence);}
            if children.len()!=2 || !matches!(node.detail.as_deref(),Some("=="|"!="|"<"|"<="|">"|">="|"is"|"is not"|"in"|"not in")) {
                return Err(BoundaryReason::UnsupportedControlFlow);
            }
            self.expression(Self::field(&children,F::Left)?)?;
            self.expression(Self::field(&children,F::Right)?)?;
            self.proof.push((K::ExpressionSyntax,node.fact_id));
        } else if let Some(value)=self.expression(node)? { return Ok(value); }
        let id=self.assumption.ok_or(BoundaryReason::UnsupportedControlFlow)?;
        let assumption=self.diagrams.get(&id).ok_or_else(||self.boundaries.get(&id).copied().unwrap_or(BoundaryReason::MissingEvidence))?;
        if assumption.is_false() { return Err(BoundaryReason::RuntimeUnreachable); }
        let tests=self.tests.get(&(node.snapshot_id,node.module_node_id,node.start_byte,node.end_byte)).ok_or(BoundaryReason::MissingEvidence)?;
        if tests.len()!=1 { return Err(BoundaryReason::MissingEvidence); }
        let test=tests[0];
        let predicate=self.diagrams.get(&test.condition_id).ok_or_else(||self.boundaries.get(&test.condition_id).copied().unwrap_or(BoundaryReason::MissingEvidence))?;
        self.remaining=self.remaining.checked_sub(1).ok_or(BoundaryReason::CompletionWorkLimit)?;
        let truth=if assumption.implies(predicate).map_err(condition_limit)? { true }
            else if assumption.and(predicate).map_err(condition_limit)?.is_false() { false }
            else { return Err(BoundaryReason::UnsupportedControlFlow); };
        self.proof.push((K::StatementCondition,test.fact_id));
        Ok(truth)
    }

    fn field<'a>(children: &[&'a SyntaxNodesRow], field: F) -> std::result::Result<&'a SyntaxNodesRow, BoundaryReason> {
        let mut nodes = children.iter().filter(|n| n.field == field);
        let node = *nodes.next().ok_or(BoundaryReason::MissingEvidence)?;
        if nodes.next().is_some() { return Err(BoundaryReason::MissingEvidence); }
        Ok(node)
    }

    fn expression(&mut self, node: &SyntaxNodesRow) -> std::result::Result<Option<bool>, BoundaryReason> {
        let row = self.expressions.get(&(node.snapshot_id, node.fact_id)).ok_or(BoundaryReason::MissingEvidence)?;
        self.remaining = self.remaining.checked_sub(usize::try_from(row.work).map_err(|_| BoundaryReason::MissingEvidence)?)
            .ok_or(BoundaryReason::CompletionWorkLimit)?;
        if !row.normal { return Err(row.reason.unwrap_or(BoundaryReason::MissingEvidence)); }
        let steps=self.expression_steps.get(&(node.snapshot_id,node.fact_id)).ok_or(BoundaryReason::MissingEvidence)?;
        self.remaining=self.remaining.checked_sub(steps.len()).ok_or(BoundaryReason::CompletionWorkLimit)?;
        if steps.is_empty() || steps.iter().enumerate().any(|(i,s)|s.ordinal != i as i64)
            || steps.last().is_none_or(|s|s.operand_fact_id != node.fact_id || Some(s.evidence_id) != row.evidence_id || s.status != row.status) {
            return Err(BoundaryReason::MissingEvidence);
        }
        self.proof.extend(steps.iter().map(|s|(s.kind,s.evidence_id)));
        Ok(row.boolean_value)
    }

    fn suite(&mut self, nodes: &[&SyntaxNodesRow], field: F, depth: usize) -> Result {
        let mut statements: Vec<_> = nodes.iter().copied().filter(|n| n.field == field).collect();
        statements.sort_by_key(|n| (n.ordinal, n.fact_id));
        if statements.iter().enumerate().any(|(i,n)| n.ordinal != i as i64) { return Err(BoundaryReason::MissingEvidence); }
        for statement in statements {
            let outcome = self.statement(statement, depth + 1)?;
            if outcome.kind() != C::Normal { return Ok(outcome); }
        }
        Ok(Completion::Normal)
    }

    fn statement(&mut self, node: &SyntaxNodesRow, depth: usize) -> Result {
        self.remaining = self.remaining.checked_sub(1).ok_or(BoundaryReason::CompletionWorkLimit)?;
        if depth > MAX_DEPTH { return Err(BoundaryReason::CompletionDepthLimit); }
        let children = self.children.get(&(node.snapshot_id, node.node_id)).cloned().unwrap_or_default();
        if children.len() > self.remaining { return Err(BoundaryReason::CompletionWorkLimit); }
        self.remaining -= children.len();
        if node.kind==S::StmtFunctionDef {
            let header=self.definitions.get(&(node.snapshot_id,node.node_id)).ok_or(BoundaryReason::UnsupportedControlFlow)?;
            let binding=header.binding;
            let defaults=header.defaults.clone();
            self.remaining=self.remaining.checked_sub(header.evidence.len()).ok_or(BoundaryReason::CompletionWorkLimit)?;
            let evidence=header.evidence.clone();
            if !self.initialized.insert((binding.snapshot_id,binding.scope_id,binding.name.clone())) {
                return Err(BoundaryReason::UnsupportedControlFlow);
            }
            for default in defaults {self.expression(default)?;}
            self.proof.extend(evidence.into_iter().map(|id|(K::DefinitionHeaderEvidence,id)));
            self.proof.push((K::CompletionStatement,node.fact_id));
            return Ok(Completion::Normal);
        }
        if children.iter().any(|child| child.owner_node_id != node.owner_node_id
            || child.module_node_id != node.module_node_id || child.start_byte < node.start_byte
            || child.end_byte > node.end_byte) { return Err(BoundaryReason::MissingEvidence); }
        let outcome = match node.kind {
            S::StmtPass if children.is_empty() => {
                self.proof.push((K::FinalizerPass, node.fact_id));
                return Ok(Completion::Normal);
            },
            S::StmtExpr if children.len() == 1 => {
                self.expression(Self::field(&children, F::Value)?)?;
                Completion::Normal
            },
            S::StmtAssign => {
                self.expression(Self::field(&children, F::Value)?)?;
                let targets: Vec<_> = children.iter().filter(|n| n.field == F::Target).collect();
                if targets.is_empty() || targets.len() + 1 != children.len() { return Err(BoundaryReason::UnsupportedControlFlow); }
                for target in targets {
                    if target.kind != S::ExprName { return Err(BoundaryReason::UnsupportedControlFlow); }
                    let bindings = self.writes.get(&(node.snapshot_id, target.node_id)).ok_or(BoundaryReason::MissingEvidence)?;
                    if bindings.len() != 1 { return Err(BoundaryReason::MissingEvidence); }
                    let binding = bindings[0];
                    let scope = self.scopes.get(&(node.snapshot_id, binding.scope_id)).ok_or(BoundaryReason::MissingEvidence)?;
                    if binding.kind != BindingKind::Assignment || scope.kind != LexicalScopeKind::Function
                        || Some(scope.owner_node_id) != node.owner_node_id
                        || !(self.initializations.contains(&binding.fact_id) || (self.entry_mode && self.path_initializations.contains(&binding.fact_id))) { return Err(BoundaryReason::UnsupportedControlFlow); }
                    if !self.initialized.insert((binding.snapshot_id,binding.scope_id,binding.name.clone())) {
                        return Err(BoundaryReason::UnsupportedControlFlow);
                    }
                    self.proof.push((K::LocalAssignmentBinding, binding.fact_id));
                }
                Completion::Normal
            },
            S::StmtReturn => {
                if !children.is_empty() {
                    if children.len() != 1 { return Err(BoundaryReason::UnsupportedControlFlow); }
                    self.expression(Self::field(&children, F::Value)?)?;
                }
                Completion::Return(node.fact_id)
            },
            S::StmtRaise if children.is_empty() => {
                let exception=self.active_exception.ok_or(BoundaryReason::UnsupportedControlFlow)?;
                Completion::raised(node.fact_id,exception)
            },
            S::StmtRaise => {
                // An arbitrary exception class is instantiated by `raise` and its constructor
                // may not complete. Only exact non-exception primitive operands establish an
                // immediate TypeError here; an opaque normal read is insufficient.
                let exception=Self::field(&children, F::Exc)?;
                self.expression(exception)?;
                if !matches!(exception.kind, S::ExprNoneLiteral | S::ExprBooleanLiteral |
                    S::ExprNumberLiteral | S::ExprStringLiteral | S::ExprBytesLiteral | S::ExprEllipsisLiteral) {
                    return Err(BoundaryReason::UnsupportedControlFlow);
                }
                if children.iter().any(|n| !matches!(n.field, F::Exc | F::Cause)) { return Err(BoundaryReason::UnsupportedControlFlow); }
                if children.iter().any(|n| n.field == F::Cause) { self.expression(Self::field(&children, F::Cause)?)?; }
                Completion::raised(node.fact_id,ExactRuntimeException::TypeError)
            },
            S::StmtBreak if children.is_empty() => Completion::Break(node.fact_id),
            S::StmtContinue if children.is_empty() => Completion::Continue(node.fact_id),
            S::StmtIf => {
                let truth = self.truth(Self::field(&children, F::Test)?)?;
                if truth { self.suite(&children, F::Body, depth)? }
                else {
                    let mut result = Completion::Normal;
                    for clause in children.iter().filter(|n| n.field == F::Orelse) {
                        if clause.kind != S::ElifElseClause { return Err(BoundaryReason::UnsupportedControlFlow); }
                        let nodes = self.children.get(&(node.snapshot_id, clause.node_id)).cloned().unwrap_or_default();
                        self.remaining = self.remaining.checked_sub(1 + nodes.len()).ok_or(BoundaryReason::CompletionWorkLimit)?;
                        let selected = if nodes.iter().any(|n| n.field == F::Test) {
                            self.truth(Self::field(&nodes, F::Test)?)?
                        } else { true };
                        if selected { result = self.suite(&nodes, F::Body, depth)?; break; }
                    }
                    result
                }
            },
            S::StmtWith => {
                let (entered,mut pending)=self.enter_contexts(node,&children)?;
                if pending==Completion::Normal {pending=self.suite(&children,F::Body,depth)?;}
                self.exit_contexts(&entered,pending)?
            },
            S::StmtTry if node.detail.as_deref()!=Some("except*") => {
                let pending = self.try_body(node,&children,depth)?;
                let previous=self.active_exception;
                if pending.kind()==C::Raise {self.active_exception=pending.exception();}
                let result=self.suite(&children,F::Finalbody,depth);
                self.active_exception=previous;
                let finalizer=result?;
                pending.after_finalizer(finalizer)
            },
            _ => return Err(BoundaryReason::UnsupportedControlFlow),
        };
        self.proof.push((K::CompletionStatement, node.fact_id));
        Ok(outcome)
    }
}

pub fn complete(inputs: Inputs<'_>) -> Outcome {
    let mut children: HashMap<_, Vec<_>> = HashMap::new();
    let mut nodes = HashMap::new();
    for node in inputs.syntax {
        children.entry((node.snapshot_id, node.parent_node_id)).or_default().push(node);
        nodes.insert((node.snapshot_id, node.node_id), node);
    }
    for group in children.values_mut() { group.sort_by_key(|n| (n.ordinal,n.start_byte,n.fact_id)); }
    let mut writes: HashMap<_, Vec<_>> = HashMap::new();
    for binding in inputs.bindings { writes.entry((binding.snapshot_id,binding.site_node_id)).or_default().push(binding); }
    // Rebinding can release the last reference and execute an arbitrary __del__. A unique
    // lexical initialization cannot replace an earlier parameter, assignment or handler value.
    let mut by_name: HashMap<_,Vec<_>> = HashMap::new();
    for binding in inputs.bindings { by_name.entry((binding.snapshot_id,binding.scope_id,&binding.name)).or_default().push(binding); }
    let eligible: BTreeSet<Id>=by_name.values().filter(|group|group.iter().all(|b|matches!(b.kind,BindingKind::Assignment|BindingKind::WithTarget))).flat_map(|group|group.iter()).filter_map(|binding| {
        let mut current=nodes.get(&(binding.snapshot_id,binding.site_node_id)).copied();
        let mut seen=BTreeSet::new();
        for _ in 0..MAX_DEPTH {
            let node=current?;
            if !seen.insert(node.node_id) || matches!(node.kind,S::StmtFor|S::StmtWhile) { return None; }
            if node.owner_node_id.is_none() || node.kind==S::StmtFunctionDef { return Some(binding.fact_id); }
            current=nodes.get(&(binding.snapshot_id,node.parent_node_id)).copied();
        }
        None
    }).collect();
    let initializations=by_name.values().filter(|g|g.len()==1).map(|g|g[0].fact_id).filter(|id|eligible.contains(id)).collect();
    let mut expression_steps: HashMap<_,Vec<_>> = HashMap::new();
    for step in inputs.expression_steps { expression_steps.entry((step.snapshot_id,step.syntax_fact_id)).or_default().push(step); }
    for steps in expression_steps.values_mut() { steps.sort_by_key(|s|s.ordinal); }
    let mut tests:HashMap<_,Vec<_>>=HashMap::new();
    for test in inputs.tests { tests.entry((test.snapshot_id,test.module_node_id,test.start_byte,test.end_byte)).or_default().push(test); }
    let mut handler_types:HashMap<_,Vec<_>>=HashMap::new();
    for handler in inputs.handler_types {handler_types.entry((handler.snapshot_id,handler.handler_node_id)).or_default().push(handler);}
    let mut classes:HashMap<_,Vec<_>>=HashMap::new();
    for class in inputs.classes {classes.entry((class.snapshot_id,class.module_name.clone(),class.qualified_name.clone())).or_default().push(class);}
    let mut modules:HashMap<_,Vec<_>>=HashMap::new();
    for module in inputs.modules {modules.entry((module.snapshot_id,module.module_node_id)).or_default().push(module);}
    let mut mro:HashMap<_,Vec<_>>=HashMap::new();
    for ancestor in inputs.mro {mro.entry((ancestor.snapshot_id,ancestor.class_node_id)).or_default().push(ancestor);}
    for ancestors in mro.values_mut() {ancestors.sort_by_key(|a|(a.ordinal,a.fact_id));}
    let definitions=definition_headers(&inputs,&children);
    let mut contexts=HashMap::new();
    for site in inputs.context_sites {
        let protocols:Vec<_>=inputs.context_protocols.iter().filter(|p|p.snapshot_id==site.snapshot_id
            && p.model_id==site.model_id && p.class_node_id==site.class_node_id).collect();
        let [protocol]=protocols.as_slice() else {continue;};
        let mut arguments:Vec<_>=inputs.context_arguments.iter().filter(|a|a.snapshot_id==site.snapshot_id && a.site_id==site.site_id).cloned().collect();
        arguments.sort_by_key(|a|a.ordinal);
        if !cpg_schema::context_protocol::admits_site(site,&arguments,protocol) {continue;}
        let mut rows:Vec<_>=inputs.context_arguments.iter().filter(|a|a.snapshot_id==site.snapshot_id && a.site_id==site.site_id).collect();
        rows.sort_by_key(|a|a.ordinal);
        let arguments:Option<Vec<_>>=rows.into_iter().map(|a| {
            let found:Vec<_>=inputs.syntax.iter().filter(|n|n.snapshot_id==site.snapshot_id && n.fact_id==a.expression_fact_id
                && n.owner_node_id==Some(site.function_node_id) && n.parent_node_id==site.call_node_id).collect();
            (found.len()==1).then(||(a,found[0]))
        }).collect();
        let Some(arguments)=arguments else {continue;};
        let key=(site.snapshot_id,site.item_node_id);
        if inputs.context_sites.iter().filter(|s|(s.snapshot_id,s.item_node_id)==key).count()!=1 {continue;}
        contexts.insert(key,PreparedContext {site,protocol,arguments});
    }
    let mut kernel = Kernel { contexts, definitions, children, writes, initializations, expression_steps, tests, diagrams:inputs.diagrams, boundaries:inputs.boundaries, assumption:None,
        handler_types,classes,mro,modules,active_exception:None,
        entry_mode:false,initialized:BTreeSet::new(),path_initializations:eligible,
        remaining: MAX_WORK, proof: Vec::new(),
        expressions: inputs.expressions.iter().map(|e| ((e.snapshot_id,e.syntax_fact_id),e)).collect(),
        scopes: inputs.scopes.iter().map(|s| ((s.snapshot_id,s.node_id),s)).collect() };
    let mut out = Outcome::default();
    for node in inputs.syntax.iter().filter(|n| n.owner_node_id.is_some() && n.kind.text().starts_with("stmt_")) {
        kernel.remaining = MAX_WORK; kernel.proof.clear();kernel.initialized.clear();kernel.active_exception=None;
        let result = kernel.statement(node,0);
        let (kind,terminal_fact_id,exception,reason) = match result { Ok(outcome) => (outcome.kind(),outcome.terminal(),outcome.exception(),None),
            Err(reason) => (C::Unknown,None,None,Some(reason)) };
        out.statements.push(StatementCompletionsRow { snapshot_id: node.snapshot_id, source_fact_id: node.fact_id,
            statement_node_id: node.node_id, function_node_id: node.owner_node_id.unwrap(),
            kind,terminal_fact_id,exception,reason,work: (MAX_WORK-kernel.remaining).max(1) as i64 });
        if reason.is_none() {
            out.statement_steps.extend(kernel.proof.iter().enumerate().map(|(ordinal,&(kind,evidence_id))|
                StatementCompletionStepsRow { snapshot_id:node.snapshot_id,source_fact_id:node.fact_id,
                    ordinal:ordinal as i64,evidence_id,kind }));
        }
    }
    let mut requested:HashMap<(Id,Id),BTreeSet<Id>>=HashMap::new();
    for &(snapshot,site,condition) in inputs.entry_conditions {requested.entry((snapshot,site)).or_default().insert(condition);}
    for exit in inputs.exits.iter().filter(|e| e.kind == ExitSiteKind::Return) {
        let mut conditions=requested.remove(&(exit.snapshot_id,exit.source_fact_id)).unwrap_or_default();
        conditions.insert(exit.condition_id);
        for condition_id in conditions {
        kernel.remaining=MAX_WORK;kernel.proof.clear();kernel.initialized.clear();kernel.active_exception=None;kernel.assumption=Some(condition_id);kernel.entry_mode=true;
        let reason=kernel.entry(exit,&nodes).err();
        out.entries.push(ReturnEntryStatusesRow {snapshot_id:exit.snapshot_id,return_site_fact_id:exit.source_fact_id,
            function_node_id:exit.function_node_id,condition_id,reason,
            work:(MAX_WORK-kernel.remaining).max(1) as i64});
        if reason.is_none() {
            out.entry_steps.extend(kernel.proof.iter().enumerate().map(|(ordinal,&(kind,evidence_id))|
                ReturnEntryStepsRow {snapshot_id:exit.snapshot_id,return_site_fact_id:exit.source_fact_id,
                    ordinal:ordinal as i64,evidence_id,kind,condition_id}));
        }
        }
        kernel.assumption=None;kernel.entry_mode=false;
        kernel.remaining = MAX_WORK; kernel.proof.clear();kernel.initialized.clear();kernel.active_exception=None;
        let mut row = ReturnExitStatusesRow { snapshot_id:exit.snapshot_id,function_node_id:exit.function_node_id,
            site_node_id:exit.site_node_id,source_fact_id:exit.source_fact_id,condition_id:exit.condition_id,
            walk_depth:0,frame_node_id:None,frame_fact_id:None,pass_node_id:None,pass_fact_id:None,reason:None };
        let mut current = nodes.get(&(exit.snapshot_id,exit.site_node_id)).copied();
        let mut visited = BTreeSet::new();
        let mut frames = Vec::new();
        while let Some(node) = current {
            if !visited.insert(node.node_id) { row.reason=Some(BoundaryReason::MissingEvidence); break; }
            let Some(parent) = nodes.get(&(exit.snapshot_id,node.parent_node_id)).copied()
                .filter(|p| p.owner_node_id == Some(exit.function_node_id)) else { break; };
            if row.walk_depth as usize >= MAX_DEPTH { row.reason=Some(BoundaryReason::CompletionDepthLimit); break; }
            let actions = kernel.children.get(&(exit.snapshot_id,parent.node_id)).cloned().unwrap_or_default();
            if parent.kind == S::StmtWith || (parent.kind == S::StmtTry && node.field != F::Finalbody
                && actions.iter().any(|n| n.field == F::Finalbody)) {
                if row.frame_node_id.is_none() { row.frame_node_id=Some(parent.node_id); row.frame_fact_id=Some(parent.fact_id); }
                frames.push(parent.node_id);
                if parent.kind == S::StmtWith {
                    let contexts=kernel.context_items(parent,&actions);
                    match contexts.and_then(|items|kernel.exit_contexts(&items,Completion::Return(exit.source_fact_id))) {
                        Ok(Completion::Return(id)) if id==exit.source_fact_id=>{},
                        Ok(_)=>{row.reason=Some(BoundaryReason::UnsupportedControlFlow);break;},
                        Err(reason)=>{row.reason=Some(reason);break;},
                    }
                    row.pass_node_id=None;row.pass_fact_id=None;
                    row.walk_depth+=1;current=Some(parent);continue;
                }
                match kernel.suite(&actions,F::Finalbody,0) {
                    Ok(outcome) if outcome.kind()==C::Normal => {},
                    Ok(_) => { row.reason=Some(BoundaryReason::UnsupportedControlFlow); break; },
                    Err(reason) => { row.reason=Some(reason); break; },
                }
                let passes: Vec<_> = actions.iter().filter(|a| a.field==F::Finalbody).collect();
                if frames.len()==1 && passes.len()==1 && passes[0].kind==S::StmtPass {
                    row.pass_node_id=Some(passes[0].node_id); row.pass_fact_id=Some(passes[0].fact_id);
                } else { row.pass_node_id=None; row.pass_fact_id=None; }
            }
            row.walk_depth+=1; current=Some(parent);
        }
        if current.is_none() { row.reason=Some(BoundaryReason::MissingEvidence); }
        if row.reason.is_none() {
            out.return_steps.extend(kernel.proof.iter().enumerate().map(|(ordinal,&(kind,evidence_id))|
                ReturnExitStepsRow {snapshot_id:exit.snapshot_id,return_site_fact_id:exit.source_fact_id,
                    ordinal:ordinal as i64,evidence_id,kind,condition_id:exit.condition_id}));
        }
        out.returns.push(row);
    }
    out.statements.sort_by_key(|r|(r.snapshot_id,r.source_fact_id));
    out.statement_steps.sort_by_key(|r|(r.snapshot_id,r.source_fact_id,r.ordinal));
    out.returns.sort_by_key(|r|(r.snapshot_id,r.site_node_id));
    out.return_steps.sort_by_key(|r|(r.snapshot_id,r.return_site_fact_id,r.ordinal));
    out.entries.sort_by_key(|r|(r.snapshot_id,r.return_site_fact_id,r.condition_id));
    out.entry_steps.sort_by_key(|r|(r.snapshot_id,r.return_site_fact_id,r.condition_id,r.ordinal));
    out.certificates=cpg_schema::completion_proof::certify(&out.entries,&out.entry_steps,&out.returns,&out.return_steps);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evaluation::{evaluate, EvaluationInputs};
    fn id(value:u8) -> Id { Id([value;16]) }
    fn node(value:u8,parent:u8,kind:S,field:F,ordinal:i64,detail:&str) -> SyntaxNodesRow {
        SyntaxNodesRow { snapshot_id:id(1), fact_id:id(value), node_id:id(value),
            module_node_id:id(2), owner_node_id:Some(id(3)), parent_node_id:id(parent),
            kind,field,ordinal,start_byte:0,end_byte:100,detail:Some(detail.to_owned()) }
    }
    fn run(nodes:&[SyntaxNodesRow], exits:&[ExitSitesRow]) -> Outcome {
        let expressions=evaluate(EvaluationInputs {syntax:nodes,..Default::default()});
        complete(Inputs {context_protocols:&[],context_sites:&[],context_arguments:&[],declarations:&[],parameters:&[],syntax:nodes,expressions:&expressions.evaluations,expression_steps:&expressions.steps,bindings:&[],scopes:&[],exits,
            handler_types:&[],classes:&[],mro:&[],modules:&[],tests:&[],entry_conditions:&[],boundaries:&HashMap::new(),diagrams:&HashMap::new()})
    }
    fn exit(site:u8) -> ExitSitesRow {
        ExitSitesRow { snapshot_id:id(1),function_node_id:id(3),site_node_id:id(site),
            kind:ExitSiteKind::Return,module_node_id:id(2),source_fact_id:id(site),
            region_fact_id:id(90),start_byte:0,end_byte:100,condition_id:id(91),approximated:false }
    }
    #[test]
    fn site_coverage_distinguishes_empty_exception_domains_from_unexamined_and_open() {
        use cpg_schema::codebook::{SummaryChannel,SummarySubjectKind};
        let nodes=vec![node(10,3,S::StmtPass,F::Body,0,""),
            node(11,3,S::StmtRaise,F::Body,1,""),node(12,11,S::ExprNoneLiteral,F::Exc,0,"None"),
            node(13,3,S::StmtExpr,F::Body,2,""),node(14,13,S::ExprCall,F::Value,0,"unknown()")];
        let out=run(&nodes,&[]);
        let rows=coverage(&out.statements);
        let exception=|site|rows.iter().find(|r|r.subject_id==id(site) && r.channel==SummaryChannel::Exception).unwrap();
        let empty=exception(10);
        assert_eq!(empty.subject_kind,SummarySubjectKind::ExecutionSite);
        assert_eq!(empty.parameter_node_id,None);
        assert!(empty.complete && empty.reason.is_none());
        assert_eq!(empty.witness_count,0);
        assert!(exception(11).complete);
        assert_eq!(exception(11).witness_count,1);
        assert!(!exception(13).complete);
        assert!(exception(13).reason.is_some());
        assert_eq!(exception(13).witness_count,0);
        assert!(!rows.iter().any(|r|r.subject_id==id(99)));
        assert_eq!(cpg_schema::summary_contract::coverage_domain(empty.subject_kind,empty.channel),
            Some(cpg_schema::summary_contract::CoverageDomain::EscapingException));
        // A handled exception is activity, but is absent from the escaping domain.
        let caught=run(&[node(20,3,S::StmtTry,F::Body,0,""),node(21,20,S::StmtRaise,F::Body,0,""),
            node(22,21,S::ExprNoneLiteral,F::Exc,0,"None"),node(23,20,S::ExceptHandlerExceptHandler,F::Handler,0,""),
            node(24,23,S::StmtPass,F::Body,0,"")],&[]);
        let caught_rows=coverage(&caught.statements);
        let whole=caught_rows.iter().find(|r|r.subject_id==id(20) && r.channel==SummaryChannel::Exception).unwrap();
        assert!(whole.complete && whole.witness_count==0);
        assert!(caught_rows.iter().any(|r|r.subject_id==id(21) && r.channel==SummaryChannel::Exception && r.witness_count==1));
    }
    #[test]
    fn nested_handler_entry_restores_the_outer_active_exception() {
        let nodes=vec![node(3,2,S::StmtFunctionDef,F::Body,0,"function"),
            node(10,3,S::StmtTry,F::Body,0,""),
            node(11,10,S::StmtRaise,F::Body,0,""),node(12,11,S::ExprNoneLiteral,F::Exc,0,"None"),
            node(13,10,S::ExceptHandlerExceptHandler,F::Handler,0,""),
            node(14,13,S::StmtTry,F::Body,0,""),
            node(15,14,S::StmtRaise,F::Body,0,""),node(16,15,S::ExprNoneLiteral,F::Exc,0,"None"),
            node(17,14,S::ExceptHandlerExceptHandler,F::Handler,0,""),node(18,17,S::StmtPass,F::Body,0,""),
            node(19,13,S::StmtRaise,F::Body,1,""),node(20,13,S::StmtReturn,F::Body,2,"")];
        let out=run(&nodes,&[exit(20)]);
        assert_eq!(out.entries[0].reason,Some(BoundaryReason::RuntimeUnreachable));
        assert_eq!(out.statements.iter().find(|s|s.source_fact_id==id(10)).unwrap().exception,Some(ExactRuntimeException::TypeError));
    }

    #[test]
    fn handler_else_and_finalizer_entry_reconstruct_the_pending_outcome() {
        let mut nodes=vec![node(3,2,S::StmtFunctionDef,F::Body,0,"function"),
            node(10,3,S::StmtTry,F::Body,0,""),
            node(11,10,S::StmtRaise,F::Body,0,""),
            node(12,11,S::ExprNoneLiteral,F::Exc,0,"None"),
            node(13,10,S::ExceptHandlerExceptHandler,F::Handler,0,""),
            node(14,13,S::StmtReturn,F::Body,0,""),
            node(15,10,S::StmtReturn,F::Orelse,0,""),
            node(16,10,S::StmtReturn,F::Finalbody,0,"")];
        let out=run(&nodes,&[exit(14),exit(15),exit(16)]);
        let reason=|out:&Outcome,site|out.entries.iter().find(|e|e.return_site_fact_id==id(site)).unwrap().reason;
        assert_eq!(reason(&out,14),None);
        assert_eq!(reason(&out,15),Some(BoundaryReason::RuntimeUnreachable));
        assert_eq!(reason(&out,16),None);
        assert!(out.entry_steps.iter().any(|s|s.return_site_fact_id==id(14) && s.evidence_id==id(11)));
        assert_eq!(out.statements.iter().find(|s|s.source_fact_id==id(11)).unwrap().exception,Some(ExactRuntimeException::TypeError));
        nodes[4].detail=Some("caught".to_owned());
        let out=run(&nodes,&[exit(14),exit(16)]);
        assert_eq!(reason(&out,14),Some(BoundaryReason::HandlerNameCleanup));
        assert_eq!(reason(&out,16),Some(BoundaryReason::HandlerNameCleanup));
        nodes[4].detail=Some(String::new());
        nodes[2].kind=S::StmtPass;
        nodes.remove(3);
        let out=run(&nodes,&[exit(14),exit(15),exit(16)]);
        assert_eq!(reason(&out,14),Some(BoundaryReason::RuntimeUnreachable));
        assert_eq!(reason(&out,15),None);
        assert_eq!(reason(&out,16),None);
        // An unresolved predecessor is not evidence that a handler or finalizer is entered.
        nodes[2].kind=S::StmtExpr;
        nodes.push(node(17,11,S::ExprName,F::Value,0,"unknown"));
        let out=run(&nodes,&[exit(14),exit(15),exit(16)]);
        for entry in out.entries {assert_eq!(entry.reason,Some(BoundaryReason::UnsupportedControlFlow));}
    }
    #[test]
    fn finalizer_outcome_preserves_or_replaces_pending_return() {
        let mut nodes=vec![node(10,3,S::StmtTry,F::Body,0,""),
            node(11,10,S::StmtReturn,F::Body,0,""),
            node(12,10,S::StmtPass,F::Finalbody,0,"")];
        let out=run(&nodes,&[exit(11)]);
        assert_eq!(out.statements.iter().find(|r|r.source_fact_id==id(10)).map(|r|(r.kind,r.terminal_fact_id)),Some((C::Return,Some(id(11)))));
        assert_eq!(out.returns[0].reason,None);
        assert_eq!(out.return_steps.iter().map(|r|r.evidence_id).collect::<Vec<_>>(),[id(12)]);
        nodes[2].kind=S::StmtReturn;
        let out=run(&nodes,&[exit(11),exit(12)]);
        assert_eq!(out.statements.iter().find(|r|r.source_fact_id==id(10)).map(|r|r.terminal_fact_id),Some(Some(id(12))));
        assert_eq!(out.returns[0].reason,Some(BoundaryReason::UnsupportedControlFlow));
        assert_eq!(out.returns[1].reason,None);
        assert!(out.return_steps.is_empty());
        nodes[2].kind=S::StmtRaise;
        nodes.push(node(13,12,S::ExprNoneLiteral,F::Exc,0,"None"));
        let out=run(&nodes,&[exit(11)]);
        assert_eq!(out.statements.iter().find(|r|r.source_fact_id==id(10)).map(|r|r.kind),Some(C::Raise));
        assert!(out.returns[0].reason.is_some());
    }
    #[test]
    fn selected_branch_skips_unknown_and_nested_frames_unwind_inside_out() {
        let nodes=vec![node(10,3,S::StmtTry,F::Body,0,""),
            node(11,10,S::StmtTry,F::Body,0,""),
            node(12,11,S::StmtReturn,F::Body,0,""),
            node(13,11,S::StmtPass,F::Finalbody,0,""),
            node(14,10,S::StmtIf,F::Finalbody,0,""),
            node(15,14,S::ExprBooleanLiteral,F::Test,0,"True"),
            node(16,14,S::StmtPass,F::Body,0,""),
            node(17,14,S::ElifElseClause,F::Orelse,0,""),
            node(18,17,S::StmtExpr,F::Body,0,""),
            node(19,18,S::ExprName,F::Value,0,"unbound")];
        let out=run(&nodes,&[exit(12)]);
        assert_eq!(out.returns[0].reason,None);
        assert_eq!(out.return_steps.iter().map(|r|r.evidence_id).collect::<Vec<_>>(),[id(13),id(15),id(16),id(14)]);
        let mut reversed=nodes.clone(); reversed.reverse();
        let shuffled=run(&reversed,&[exit(12)]);
        assert_eq!(out.statements,shuffled.statements);
        assert_eq!(out.return_steps,shuffled.return_steps);
        let mut selected_unknown=nodes;
        selected_unknown[5].detail=Some("False".to_owned());
        assert!(run(&selected_unknown,&[exit(12)]).returns[0].reason.is_some());
    }
    #[test]
    fn malformed_suites_and_unproved_writes_are_unknown() {
        let mut nodes=vec![node(10,3,S::StmtTry,F::Body,0,""),
            node(11,10,S::StmtReturn,F::Body,0,""),
            node(12,10,S::StmtPass,F::Finalbody,1,"")];
        assert_eq!(run(&nodes,&[exit(11)]).returns[0].reason,Some(BoundaryReason::MissingEvidence));
        nodes[2]=node(12,10,S::StmtAssign,F::Finalbody,0,"");
        nodes.extend([node(13,12,S::ExprNumberLiteral,F::Value,0,"1"),node(14,12,S::ExprName,F::Target,0,"marker")]);
        assert_eq!(run(&nodes,&[exit(11)]).returns[0].reason,Some(BoundaryReason::MissingEvidence));
    }
    #[test]
    fn a_caught_raise_skips_else_and_bare_reraise_preserves_exception() {
        let mut nodes=vec![node(10,3,S::StmtTry,F::Body,0,""),
            node(11,10,S::StmtRaise,F::Body,0,""),
            node(12,11,S::ExprNoneLiteral,F::Exc,0,"None"),
            node(13,10,S::ExceptHandlerExceptHandler,F::Handler,0,""),
            node(14,13,S::StmtPass,F::Body,0,""),
            node(15,10,S::StmtReturn,F::Orelse,0,""),
            node(16,10,S::StmtPass,F::Finalbody,0,"")];
        let out=run(&nodes,&[]);
        assert_eq!(out.statements.iter().find(|r|r.source_fact_id==id(10)).map(|r|r.kind),Some(C::Normal));
        assert!(!out.statement_steps.iter().any(|r|r.source_fact_id==id(10)&&r.evidence_id==id(15)));
        nodes[4].kind=S::StmtRaise;
        let out=run(&nodes,&[]);
        assert_eq!(out.statements.iter().find(|r|r.source_fact_id==id(10)).map(|r|r.kind),Some(C::Raise));
    }
    #[test]
    fn production_completion_depth_and_work_limits_remain_distinct() {
        let mut nodes:Vec<_>=(10..=140).map(|i|node(i,if i==10 {3} else {i-1},S::StmtTry,F::Body,0,"")).collect();
        nodes.push(node(141,140,S::StmtPass,F::Body,0,""));
        let out=run(&nodes,&[]);
        assert_eq!(out.statements.iter().find(|r|r.source_fact_id==id(10)).unwrap().reason,Some(BoundaryReason::CompletionDepthLimit));
        let mut nodes=vec![node(10,3,S::StmtTry,F::Body,0,"")];
        for ordinal in 0..MAX_WORK {
            let mut statement=node(11,10,S::StmtPass,F::Finalbody,ordinal as i64,"");
            let mut bytes=[0;16];bytes[..8].copy_from_slice(&(ordinal as u64).to_le_bytes());bytes[15]=1;
            statement.fact_id=Id(bytes);statement.node_id=Id(bytes);nodes.push(statement);
        }
        let out=run(&nodes,&[]);
        assert_eq!(out.statements.iter().find(|r|r.source_fact_id==id(10)).unwrap().reason,Some(BoundaryReason::CompletionWorkLimit));
        assert!(!out.statement_steps.iter().any(|r|r.source_fact_id==id(10)));
    }

    #[test]
    fn bare_reraise_uses_active_handler_or_finally_exception_only() {
        let nodes=vec![node(10,3,S::StmtTry,F::Body,0,""),
            node(11,10,S::StmtTry,F::Body,0,""),node(12,11,S::StmtRaise,F::Body,0,""),
            node(13,12,S::ExprNoneLiteral,F::Exc,0,"None"),
            node(14,11,S::ExceptHandlerExceptHandler,F::Handler,0,""),node(15,14,S::StmtRaise,F::Body,0,""),
            node(16,10,S::ExceptHandlerExceptHandler,F::Handler,0,""),node(17,16,S::StmtPass,F::Body,0,"")];
        let out=run(&nodes,&[]);
        assert_eq!(out.statements.iter().find(|s|s.source_fact_id==id(10)).unwrap().kind,C::Normal);
        assert_eq!(out.statements.iter().find(|s|s.source_fact_id==id(15)).unwrap().kind,C::Unknown);
        let mut finalizing=nodes.clone();
        finalizing.retain(|n|n.node_id!=id(14));
        let reraise=finalizing.iter_mut().find(|n|n.node_id==id(15)).unwrap();reraise.parent_node_id=id(11);reraise.field=F::Finalbody;
        assert_eq!(run(&finalizing,&[]).statements.iter().find(|s|s.source_fact_id==id(10)).unwrap().kind,C::Normal);
    }

    #[test]
    fn path_guard_conjunction_preserves_actual_bdd_work_and_node_caps() {
        use cpg_schema::condition::Atom;
        use cpg_schema::summary_contract::ExpressionRead;
        use cpg_schema::codebook::ModeledArgumentEvaluationStatus;
        for (bits,expected) in [(8,BoundaryReason::ConditionNodeLimit),(10,BoundaryReason::ConditionWorkLimit)] {
            let mut halves=[Diagram::always(),Diagram::always()];
            for bit in 0..bits*2 {
                let a=Diagram::from_atom(&Atom::Truthy {place:format!("a{bit:02}")}).unwrap();
                let b=Diagram::from_atom(&Atom::Truthy {place:format!("b{bit:02}")}).unwrap();
                let equal=a.and(&b).unwrap().or(&a.not().unwrap().and(&b.not().unwrap()).unwrap()).unwrap();
                let half=usize::from(bit>=bits);halves[half]=halves[half].and(&equal).unwrap();
            }
            let [path,predicate]=halves;
            let mut function=node(3,2,S::StmtFunctionDef,F::Body,0,"");function.owner_node_id=None;
            let nodes=vec![function,node(10,3,S::StmtIf,F::Body,0,""),
                node(11,10,S::ExprName,F::Test,0,"enabled"),node(12,10,S::StmtReturn,F::Body,0,"")];
            let read=ExpressionRead {snapshot_id:id(1),syntax_fact_id:id(11),evidence_id:id(90),status:ModeledArgumentEvaluationStatus::ParameterNameNormal};
            let evaluations=evaluate(EvaluationInputs {syntax:&nodes,reads:&[read],..Default::default()});
            let mut target=exit(12);target.condition_id=path.id();
            let test=FlowTestsRow {snapshot_id:id(1),fact_id:id(91),module_node_id:id(2),scope_kind:LexicalScopeKind::Function,
                scope_start_byte:None,scope_end_byte:None,start_byte:0,end_byte:100,condition_id:predicate.id()};
            let diagrams=HashMap::from([(path.id(),path),(predicate.id(),predicate)]);
            let exits=[target];let tests=[test];
            let check=|diagrams:&HashMap<Id,Diagram>,boundaries:&HashMap<Id,BoundaryReason>| {
                let out=complete(Inputs {context_protocols:&[],context_sites:&[],context_arguments:&[],declarations:&[],parameters:&[],syntax:&nodes,expressions:&evaluations.evaluations,expression_steps:&evaluations.steps,
                    bindings:&[],scopes:&[],exits:&exits,handler_types:&[],classes:&[],mro:&[],modules:&[],tests:&tests,entry_conditions:&[],boundaries,diagrams});
                assert_eq!(out.entries[0].reason,Some(expected));assert!(out.entry_steps.is_empty());
            };
            check(&diagrams,&HashMap::new());
            for absent in [exits[0].condition_id,tests[0].condition_id] {
                let mut withheld=diagrams.clone();withheld.remove(&absent);
                check(&withheld,&HashMap::from([(absent,expected)]));
            }
        }
    }

    #[test]
    fn comparison_entry_is_conditional_and_requests_are_return_specific() {
        use cpg_schema::condition::Atom;
        use cpg_schema::summary_contract::ExpressionRead;
        use cpg_schema::codebook::ModeledArgumentEvaluationStatus;
        let path=Diagram::from_atom(&Atom::Truthy {place:"comparison event".to_owned()}).unwrap();
        let mut function=node(3,2,S::StmtFunctionDef,F::Body,0,"");function.owner_node_id=None;
        let mut nodes=vec![function,node(10,3,S::StmtIf,F::Body,0,""),
            node(11,10,S::ExprCompare,F::Test,0,"=="),node(12,11,S::ExprName,F::Left,0,"value"),
            node(13,11,S::ExprNumberLiteral,F::Right,0,"2"),node(14,10,S::StmtReturn,F::Body,0,"")];
        let read=ExpressionRead {snapshot_id:id(1),syntax_fact_id:id(12),evidence_id:id(90),status:ModeledArgumentEvaluationStatus::ParameterNameNormal};
        let mut target=exit(14);target.condition_id=path.id();
        let tests=[FlowTestsRow {snapshot_id:id(1),fact_id:id(91),module_node_id:id(2),scope_kind:LexicalScopeKind::Function,
            scope_start_byte:None,scope_end_byte:None,start_byte:0,end_byte:100,condition_id:path.id()}];
        let diagrams=HashMap::from([(path.id(),path)]);
        let check=|nodes:&[SyntaxNodesRow]| {
            let evaluations=evaluate(EvaluationInputs {syntax:nodes,reads:std::slice::from_ref(&read),..Default::default()});
            assert!(!evaluations.evaluations.iter().find(|e|e.syntax_fact_id==id(11)).unwrap().normal);
            complete(Inputs {context_protocols:&[],context_sites:&[],context_arguments:&[],declarations:&[],parameters:&[],syntax:nodes,expressions:&evaluations.evaluations,expression_steps:&evaluations.steps,
                bindings:&[],scopes:&[],exits:std::slice::from_ref(&target),handler_types:&[],classes:&[],mro:&[],modules:&[],tests:&tests,
                entry_conditions:&[(id(1),id(99),Diagram::always().id())],boundaries:&HashMap::new(),diagrams:&diagrams})
        };
        let out=check(&nodes);
        assert_eq!(out.entries.len(),1,"unrelated return requests are not evaluated");
        assert_eq!(out.entries[0].reason,None);
        assert_eq!(out.statements.iter().find(|s|s.source_fact_id==id(10)).unwrap().kind,C::Unknown);
        assert_eq!(out.entry_steps.iter().map(|s|s.evidence_id).collect::<Vec<_>>(),[id(90),id(13),id(11),id(91)]);
        nodes[4].kind=S::ExprName;nodes[4].detail=Some("missing".to_owned());
        assert!(check(&nodes).entries[0].reason.is_some(),"a predicate premise cannot replace operand evidence");
    }

    #[test]
    fn entry_sequence_checks_noncall_statements_and_skips_unselected_branches() {
        let mut function=node(3,2,S::StmtFunctionDef,F::Body,0,"");function.owner_node_id=None;
        let mut nodes=vec![function,node(10,3,S::StmtExpr,F::Body,0,""),
            node(11,10,S::ExprNumberLiteral,F::Value,0,"1"),node(12,3,S::StmtReturn,F::Body,1,"")];
        let out=run(&nodes,&[exit(12)]);
        assert_eq!(out.entries[0].reason,None);
        assert_eq!(out.entry_steps.iter().map(|s|s.evidence_id).collect::<Vec<_>>(),[id(11),id(10)]);
        nodes[2].kind=S::ExprName;nodes[2].detail=Some("missing".to_owned());
        assert_eq!(run(&nodes,&[exit(12)]).entries[0].reason,Some(BoundaryReason::UnsupportedControlFlow));
        nodes[1]=node(10,3,S::StmtIf,F::Body,0,"");
        nodes[2]=node(11,10,S::ExprBooleanLiteral,F::Test,0,"False");
        nodes.extend([node(13,10,S::StmtExpr,F::Body,0,""),node(14,13,S::ExprName,F::Value,0,"missing")]);
        let out=run(&nodes,&[exit(12)]);
        assert_eq!(out.entries[0].reason,None);
        assert!(!out.entry_steps.iter().any(|s|s.evidence_id==id(14)));
        nodes[2].detail=Some("True".to_owned());
        assert!(run(&nodes,&[exit(12)]).entries[0].reason.is_some());
    }
}
