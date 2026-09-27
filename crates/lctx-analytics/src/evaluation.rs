//! Bounded source-expression completion over independently admitted name reads.
//!
//! Pure operators compose with source read certificates. A normal outcome does not assert a
//! normal callee return. Unsupported/effectful operands stop evaluation unless Python skips
//! them. The root syntax fact is a reproducible witness, not a synthetic execution observation.
use std::collections::{HashMap, BTreeMap, BTreeSet};
use cpg_schema::codebook::ReleaseSafety;
use cpg_schema::frame_exit::{ModelFrameExitsRow,ModelFrameExitArgumentsRow};

use cpg_schema::behavior::{ExpressionEvaluationsRow, ExpressionEvaluationStepsRow};
use cpg_schema::summary_contract::{ExpressionRead, PinnedCallTarget, SignatureParameter, BoundArgument, bind_arguments};
use cpg_schema::codebook::{BoundaryReason, ModeledArgumentEvaluationStatus as Status, SyntaxField, SyntaxKind, Codebook, SummaryFlowStepKind as Step, SignatureForm, Modality, ModelTransferKind, ModelTransferEndpointStatus};
use cpg_schema::id::Id;
use cpg_schema::tables::{SyntaxNodesRow, ArgumentsRow, ContextParametersRow};
use cpg_schema::behavior::ModeledTransferSitesRow;

const MAX_DEPTH: usize = 64;
const MAX_WORK: usize = 1024;

#[derive(Clone, Copy)]
enum Value {
    Bool(bool),
    Int(i64),
    Float(f64),
    None,
    Tuple { nonempty: bool, retained: bool },
    /// Literal evaluation is normal even when the bounded evaluator cannot represent its value.
    Literal,
    Retained,
    Unknown,
}

impl Value {
    fn truth(self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(b), Self::Int(i) => Some(i != 0),
            Self::Float(f) => Some(f != 0.0), Self::None => Some(false), Self::Literal | Self::Retained | Self::Unknown => None,
            Self::Tuple { nonempty, .. } => Some(nonempty),
        }
    }
    fn release(self)->ReleaseSafety {
        match self {Self::Retained|Self::Tuple {retained:true,..}=>ReleaseSafety::CallerRetained,
            Self::Unknown=>ReleaseSafety::Unknown,_=>ReleaseSafety::Closed}
    }
    fn number(self) -> Option<Self> {
        match self {
            Self::Bool(b) => Some(Self::Int(i64::from(b))),
            Self::Int(_) | Self::Float(_) => Some(self), _ => None,
        }
    }
}

type Refusal = BoundaryReason;
const UNSUPPORTED: Refusal = BoundaryReason::UnsupportedControlFlow;

struct Evaluator<'a> {
    children: HashMap<(Id, Id, Id), Vec<&'a SyntaxNodesRow>>,
    remaining: usize,
    depth_limit: usize,
    reads: HashMap<(Id, Id), Option<&'a ExpressionRead>>,
    proof: Vec<(Id, Id, Status, Step)>,
    calls: HashMap<(Id, Id), Option<PreparedCall<'a>>>,
    frames:BTreeMap<Id,(ModelFrameExitsRow,Vec<ModelFrameExitArgumentsRow>,Vec<ExpressionEvaluationStepsRow>)>,
}

fn proof_rows(node:&SyntaxNodesRow,proof:&[(Id,Id,Status,Step)],offset:usize)->Vec<ExpressionEvaluationStepsRow> {
    proof.iter().enumerate().map(|(i,&(operand_fact_id,evidence_id,status,kind))|
        ExpressionEvaluationStepsRow {snapshot_id:node.snapshot_id,syntax_fact_id:node.fact_id,
            ordinal:(offset+i) as i64,operand_fact_id,evidence_id,status,kind}).collect()
}

impl Evaluator<'_> {
    /// Evaluate only the callee and its ordered arguments. A successful prefix is independent
    /// of the invoked callable's outcome; whole-expression evaluation checks normality later.
    fn invoke(&mut self,node:&SyntaxNodesRow,depth:usize)->Result<(Value,Vec<ModelFrameExitArgumentsRow>),Refusal> {
        let proof_start=self.proof.len();
        self.remaining=self.remaining.checked_sub(1).ok_or(Refusal::ExpressionWorkLimit)?;
        if depth>self.depth_limit {return Err(Refusal::ExpressionDepthLimit);}
        let call=self.calls.get(&(node.snapshot_id,node.node_id)).and_then(|c|c.clone()).ok_or(UNSUPPORTED)?;
        if !call.defaults_available {return Err(Refusal::DefaultUnavailable);}
        let children=self.children.get(&(node.snapshot_id,node.module_node_id,node.node_id)).cloned().unwrap_or_default();
        self.remaining=self.remaining.checked_sub(children.len()).ok_or(Refusal::ExpressionWorkLimit)?;
        if node.kind!=SyntaxKind::ExprCall || children.len()!=call.arguments.len()+1
            || children.iter().any(|c|c.owner_node_id!=node.owner_node_id || c.start_byte<node.start_byte || c.end_byte>node.end_byte)
            || children.iter().filter(|c|c.field==SyntaxField::Callee && c.kind==SyntaxKind::ExprName).count()!=1 {
            return Err(UNSUPPORTED);
        }
        // These tags describe the successfully evaluated prefix, never the call's return.
        for(kind,evidence)in [(Step::ModuleImportBinding,call.target.import_binding_fact_id),
            (Step::ModuleImportRegion,call.target.import_region_fact_id),(Step::CalleeResolution,call.target.resolution_fact_id)] {
            self.proof.push((node.fact_id,evidence,Status::CalleeEntryNormal,kind));
        }
        if !call.default_formals.is_empty() {
            self.remaining=self.remaining.checked_sub(1+call.default_formals.len()).ok_or(Refusal::ExpressionWorkLimit)?;
            self.proof.push((node.fact_id,call.target.model_id,Status::CalleeEntryNormal,Step::ModelDefaultsAvailable));
            for &formal in &call.default_formals {
                self.proof.push((node.fact_id,formal,Status::CalleeEntryNormal,Step::ModelDefaultFormal));
            }
        }
        let mut returned=Value::Unknown;
        let mut release=Vec::new();
        for(index,argument)in call.arguments.iter().enumerate() {
            let operands:Vec<_>=children.iter().filter(|c|c.field==SyntaxField::Argument
                && c.ordinal==argument.ordinal && c.start_byte==argument.value_start_byte && c.end_byte==argument.value_end_byte).collect();
            let [operand]=operands.as_slice() else {return Err(UNSUPPORTED);};
            let expression_start=self.proof.len();
            let value=self.eval(operand,depth+1)?;
            let expression_count=self.proof.len()-expression_start;
            let expression_digest=cpg_schema::frame_exit::steps_digest(&proof_rows(node,&self.proof[expression_start..],expression_start-proof_start));
            self.remaining=self.remaining.checked_sub(call.parameter_evidence[index].len()).ok_or(Refusal::ExpressionWorkLimit)?;
            for &evidence in &call.parameter_evidence[index] {
                self.proof.push((operand.fact_id,evidence,Status::CalleeEntryNormal,Step::ParameterBinding));
            }
            if call.return_argument==Some(argument.fact_id) {returned=value;}
            release.push(ModelFrameExitArgumentsRow {snapshot_id:node.snapshot_id,frame_exit_id:Id::ZERO,
                ordinal:argument.ordinal,argument_fact_id:argument.fact_id,expression_fact_id:operand.fact_id,safety:value.release(),
                parameter_name:call.parameter_names[index].clone(),expression_offset:(expression_start-proof_start) as i64,
                expression_count:expression_count as i64,expression_digest,
                parameters_digest:cpg_schema::frame_exit::parameters_digest(&call.parameter_evidence[index])});
        }
        for(kind,evidence)in [(Step::CallSite,call.target.call_fact_id),(Step::CallTarget,call.target.pysa_fact_id)] {
            self.proof.push((node.fact_id,evidence,Status::CalleeEntryNormal,kind));
        }
        Ok((returned,release))
    }

    fn eval(&mut self, node: &SyntaxNodesRow, depth: usize) -> Result<Value, Refusal> {
        let value = self.eval_inner(node, depth)?;
        let (evidence, status) = if node.kind == SyntaxKind::ExprName {
            let read = self.reads.get(&(node.snapshot_id, node.fact_id)).and_then(|r| *r).ok_or(UNSUPPORTED)?;
            (read.evidence_id, read.status)
        } else {
            let status = match node.kind {
                SyntaxKind::ExprBooleanLiteral | SyntaxKind::ExprNumberLiteral | SyntaxKind::ExprNoneLiteral
                    | SyntaxKind::ExprStringLiteral | SyntaxKind::ExprBytesLiteral | SyntaxKind::ExprEllipsisLiteral => Status::LiteralNormal,
                SyntaxKind::ExprCall => Status::PinnedCallNormal,
                _ => Status::ComposedExpressionNormal,
            };
            (node.fact_id, status)
        };
        self.proof.push((node.fact_id, evidence, status, if node.kind == SyntaxKind::ExprName {
            Step::ArgumentEvaluation } else { Step::ExpressionSyntax }));
        Ok(value)
    }

    fn eval_inner(&mut self, node: &SyntaxNodesRow, depth: usize) -> Result<Value, Refusal> {
        self.remaining = self.remaining.checked_sub(1).ok_or(Refusal::ExpressionWorkLimit)?;
        if depth > self.depth_limit { return Err(Refusal::ExpressionDepthLimit); }
        // Only selected children are recursively evaluated, but every child of a supported
        // operator must have one unambiguous structural role and belong to the same owner.
        let children = self.children.get(&(node.snapshot_id, node.module_node_id, node.node_id))
            .map(Vec::as_slice).unwrap_or_default();
        if children.len() > self.remaining { return Err(Refusal::ExpressionWorkLimit); }
        self.remaining -= children.len();
        let children = children.to_vec();
        if children.iter().any(|child| child.owner_node_id != node.owner_node_id
            || child.start_byte < node.start_byte || child.end_byte > node.end_byte) {
            return Err(UNSUPPORTED);
        }
        let one = |field| -> Result<&SyntaxNodesRow, Refusal> {
            let mut matching = children.iter().filter(|child| child.field == field);
            let child = matching.next().ok_or(UNSUPPORTED)?;
            if matching.next().is_some() { return Err(UNSUPPORTED); }
            Ok(*child)
        };
        let detail = node.detail.as_deref().unwrap_or("");
        match node.kind {
            SyntaxKind::ExprCall => {
                let call = self.calls.get(&(node.snapshot_id, node.node_id))
                    .and_then(|call| call.clone()).ok_or(UNSUPPORTED)?;
                // Charge this node/children once, through the common invocation owner.
                self.remaining+=1+children.len();
                let proof_start=self.proof.len();
                let (returned,mut arguments)=self.invoke(node,depth)?;
                let invocation=proof_rows(node,&self.proof[proof_start..],0);
                let Some(parameter)=&call.target.body_return_parameter else {return Err(UNSUPPORTED);};
                if !call.target.caller_function_scope {return Err(Refusal::ScopeBoundary);}
                if !call.default_formals.is_empty() || arguments.iter().any(|a|a.safety==ReleaseSafety::Unknown) {
                    return Err(Refusal::FrameExitCleanup);
                }
                let mut frame=ModelFrameExitsRow {snapshot_id:node.snapshot_id,frame_exit_id:Id::ZERO,
                    function_node_id:node.owner_node_id.ok_or(UNSUPPORTED)?,call_node_id:node.node_id,
                    call_fact_id:call.target.call_fact_id,syntax_fact_id:node.fact_id,target_node_id:call.target.target_node_id,
                    pysa_fact_id:call.target.pysa_fact_id,model_id:call.target.model_id,return_parameter:parameter.clone(),
                    return_argument_fact_id:call.return_argument.ok_or(Refusal::FrameExitCleanup)?,
                    argument_count:call.target.argument_count,signature_count:call.target.signature_count,
                    arguments_digest:cpg_schema::frame_exit::arguments_digest(&arguments),
                    invocation_count:invocation.len() as i64,invocation_digest:cpg_schema::frame_exit::steps_digest(&invocation)};
                frame.frame_exit_id=cpg_schema::frame_exit::identity(&frame);
                for argument in &mut arguments {argument.frame_exit_id=frame.frame_exit_id;}
                if invocation.len()+3+usize::from(call.identity.is_some())>cpg_schema::summary_contract::MAX_SUMMARY_PROOF_STEPS {return Err(Refusal::SummaryProofLimit);}
                cpg_schema::frame_exit::admit(&frame,&arguments,&invocation).map_err(|_|Refusal::FrameExitCleanup)?;
                self.proof.push((node.fact_id,frame.frame_exit_id,Status::PinnedCallNormal,Step::ModelFrameExit));
                self.frames.insert(frame.frame_exit_id,(frame,arguments,invocation));
                self.proof.push((node.fact_id,call.target.model_id,Status::PinnedCallNormal,Step::PrecedingCallNormal));
                if let Some((_, rule)) = call.identity {
                    self.proof.push((node.fact_id, rule, Status::PinnedCallNormal, Step::ModelRule));
                }
                Ok(returned)
            },
            SyntaxKind::ExprName if children.is_empty() => {
                let read = self.reads.get(&(node.snapshot_id, node.fact_id)).and_then(|r| *r).ok_or(UNSUPPORTED)?;
                match read.status {
                    Status::BuiltinNameNormal | Status::ParameterNameNormal | Status::AssignmentNameNormal
                        | Status::LexicalParameterNormal => Ok(Value::Retained),
                    _ => Err(UNSUPPORTED),
                }
            },
            SyntaxKind::ExprBooleanLiteral if children.is_empty() => match detail {
                "True" => Ok(Value::Bool(true)), "False" => Ok(Value::Bool(false)), _ => Err(UNSUPPORTED),
            },
            SyntaxKind::ExprNumberLiteral if children.is_empty() => Ok(number(detail)),
            SyntaxKind::ExprNoneLiteral if children.is_empty() => Ok(Value::None),
            SyntaxKind::ExprStringLiteral | SyntaxKind::ExprBytesLiteral
                | SyntaxKind::ExprEllipsisLiteral if children.is_empty() => Ok(Value::Literal),
            SyntaxKind::ExprUnaryOp if children.len() == 1 => {
                let value = self.eval(one(SyntaxField::Operand)?, depth + 1)?;
                match detail {
                    "not" => value.truth().map(|b| Value::Bool(!b)).ok_or(UNSUPPORTED),
                    "+" => value.number().ok_or(UNSUPPORTED),
                    "-" => match value.number() {
                        Some(Value::Int(i)) => i.checked_neg().map(Value::Int).ok_or(UNSUPPORTED),
                        Some(Value::Float(f)) => Ok(Value::Float(-f)), _ => Err(UNSUPPORTED),
                    },
                    _ => Err(UNSUPPORTED),
                }
            },
            SyntaxKind::ExprBinOp if children.len() == 2 && matches!(detail, "+" | "-") => {
                let left = self.eval(one(SyntaxField::Left)?, depth + 1)?.number().ok_or(UNSUPPORTED)?;
                let right = self.eval(one(SyntaxField::Right)?, depth + 1)?.number().ok_or(UNSUPPORTED)?;
                match (left, right) {
                    (Value::Int(a), Value::Int(b)) => if detail == "+" { a.checked_add(b) }
                        else { a.checked_sub(b) }.map(Value::Int).ok_or(UNSUPPORTED),
                    (a, b) => {
                        let float = |value| match value { Value::Int(i) => i as f64,
                            Value::Float(f) => f, _ => unreachable!("numeric operands") };
                        let (a, b) = (float(a), float(b));
                        Ok(Value::Float(if detail == "+" { a + b } else { a - b }))
                    }
                }
            },
            SyntaxKind::ExprTuple => {
                let mut retained=false;
                for (ordinal,child) in children.iter().enumerate() {
                    if child.field!=SyntaxField::Element || child.ordinal!=ordinal as i64 {return Err(UNSUPPORTED);}
                    match self.eval(child,depth+1)?.release() {
                        ReleaseSafety::Closed=>{},ReleaseSafety::CallerRetained=>retained=true,
                        ReleaseSafety::Unknown=>return Err(Refusal::FrameExitCleanup),
                    }
                }
                Ok(Value::Tuple {nonempty:!children.is_empty(),retained})
            },
            SyntaxKind::ExprBoolOp if children.len() >= 2 && matches!(detail, "and" | "or") => {
                for (ordinal, child) in children.iter().enumerate() {
                    if child.field != SyntaxField::Operand || child.ordinal != ordinal as i64 {
                        return Err(UNSUPPORTED);
                    }
                }
                let mut value = self.eval(children[0], depth + 1)?;
                for child in children.iter().skip(1) {
                    let truth = value.truth().ok_or(UNSUPPORTED)?;
                    if (detail == "and" && !truth) || (detail == "or" && truth) { return Ok(value); }
                    value = self.eval(child, depth + 1)?;
                }
                Ok(value)
            },
            SyntaxKind::ExprIf if children.len() == 3 => {
                let test = one(SyntaxField::Test)?;
                let yes = one(SyntaxField::Value)?;
                let no = one(SyntaxField::Orelse)?;
                let choose = self.eval(test, depth + 1)?.truth().ok_or(UNSUPPORTED)?;
                self.eval(if choose { yes } else { no }, depth + 1)
            },
            _ => Err(UNSUPPORTED),
        }
    }
}

/// Parse only bounded, source-provided numeric literals, not Python expressions. Integer
/// overflow stays an opaque literal (normal to read, unknown in arithmetic); in particular we
/// never coerce an unbounded Python integer to float and suppress its possible OverflowError.
fn number(text: &str) -> Value {
    if text.len() > 128 { return Value::Literal; }
    let clean = text.replace('_', "");
    let lower = clean.to_ascii_lowercase();
    let radix = [("0x", 16), ("0o", 8), ("0b", 2)].into_iter()
        .find_map(|(prefix, base)| lower.strip_prefix(prefix).map(|digits| (digits, base)));
    if let Some((digits, base)) = radix {
        return i64::from_str_radix(digits, base).map(Value::Int).unwrap_or(Value::Literal);
    }
    if lower.contains(['.', 'e']) && !lower.ends_with('j') {
        return lower.parse::<f64>().map(Value::Float).unwrap_or(Value::Literal);
    }
    lower.parse::<i64>().map(Value::Int).unwrap_or(Value::Literal)
}

/// A normal expression and the exact operands evaluated to produce it. A refusal has no
/// admitted proof, even if some operands completed before the unsupported operation.
#[derive(Default)]
pub struct EvaluationOutcome {
    pub evaluations: Vec<ExpressionEvaluationsRow>,
    pub steps: Vec<ExpressionEvaluationStepsRow>,
    pub invocations: Vec<CallInvocation>,
    pub frames:Vec<ModelFrameExitsRow>,
    pub frame_arguments:Vec<ModelFrameExitArgumentsRow>,
    pub frame_steps:Vec<cpg_schema::frame_exit::ModelFrameExitStepsRow>,
}

/// Pure invocation prefix, under entry to this expression. Completion must still prove the
/// enclosing function reaches it; a normal callee return is not implied.
#[derive(Clone)]
pub struct CallInvocation {
    pub snapshot_id:Id,pub function_node_id:Id,pub call_node_id:Id,pub call_fact_id:Id,
    pub syntax_fact_id:Id,pub target_node_id:Id,pub pysa_fact_id:Id,pub model_id:Id,
    pub argument_count:i64,pub default_formals:Option<Vec<Id>>,pub reason:Option<BoundaryReason>,pub work:i64,
    pub proof:Vec<(Step,Id)>,
}

#[derive(Clone)]
struct PreparedCall<'a> {
    target: &'a PinnedCallTarget,
    arguments: Vec<&'a ArgumentsRow>,
    parameter_evidence: Vec<Vec<Id>>,
    parameter_names:Vec<String>,
    identity: Option<(Id, Id)>,
    defaults_available:bool,
    default_formals:Vec<Id>,
    return_argument:Option<Id>,
}

#[derive(Default)]
pub struct EvaluationInputs<'a> {
    pub syntax: &'a [SyntaxNodesRow],
    pub reads: &'a [ExpressionRead],
    pub targets: &'a [PinnedCallTarget],
    pub call_arguments: &'a [ArgumentsRow],
    pub parameters: &'a [ContextParametersRow],
    pub transfers: &'a [ModeledTransferSitesRow],
    pub unconditional_conditions: &'a [Id],
}

fn prepare_calls<'a>(inputs: &EvaluationInputs<'a>) -> HashMap<(Id, Id), Option<PreparedCall<'a>>> {
    let normal: BTreeSet<_> = inputs.unconditional_conditions.iter().copied().collect();
    let mut by_call: HashMap<_, Vec<_>> = HashMap::new();
    for argument in inputs.call_arguments {
        by_call.entry((argument.snapshot_id, argument.call_node_id)).or_default().push(argument);
    }
    let mut by_target: HashMap<_, Vec<_>> = HashMap::new();
    for parameter in inputs.parameters {
        by_target.entry((parameter.snapshot_id, parameter.symbol_node_id)).or_default().push(parameter);
    }
    let mut transfers: HashMap<_, Vec<_>> = HashMap::new();
    for transfer in inputs.transfers {
        transfers.entry((transfer.snapshot_id, transfer.call_site_node_id)).or_default().push(transfer);
    }
    let mut out = HashMap::new();
    for target in inputs.targets {
        let key = (target.snapshot_id, target.call_node_id);
        if out.contains_key(&key) { out.insert(key, None); continue; }
        out.insert(key, None);
        if !normal.contains(&target.import_condition_id) || !(1..=128).contains(&target.signature_count) { continue; }
        let mut arguments = by_call.get(&key).cloned().unwrap_or_default();
        if usize::try_from(target.argument_count).ok() != Some(arguments.len()) { continue; }
        arguments.sort_by_key(|argument| argument.ordinal);
        let Some(parameters) = by_target.get(&(target.snapshot_id, target.target_node_id)) else { continue; };
        if parameters.len() > MAX_WORK { continue; }
        let owned_arguments: Vec<_> = arguments.iter().map(|a| (*a).clone()).collect();
        let mut bindings: Option<Vec<BoundArgument>> = None;
        let mut parameter_evidence = vec![Vec::new(); arguments.len()];
        let mut defaults_available=true;
        let mut default_formals=Vec::new();
        let mut default_names:Option<Vec<String>>=None;
        let mut valid = parameters.iter().all(|p| (0..target.signature_count).contains(&p.signature_index));
        for index in 0..target.signature_count {
            let signature: Option<Vec<_>> = parameters.iter().filter(|p| p.signature_index == index).map(|parameter| {
                if parameter.form != SignatureForm::List { return None; }
                SignatureParameter::from_context(parameter).ok()
            }).collect();
            let Some(signature) = signature else { valid = false; break; };
            if signature.is_empty() { valid = false; break; }
            let Ok(bound) = bind_arguments(&signature, &owned_arguments) else { valid = false; break; };
            // Binding owns the complete omitted domain before proof construction. Each
            // signature retains its fact identities; a model promise never resolves drift.
            let names:Vec<_>=bound.defaults.iter().map(|d|d.parameter_name.clone()).collect();
            if default_names.as_ref().is_some_and(|previous|*previous!=names) {valid=false;break;}
            default_names=Some(names);
            default_formals.extend(bound.defaults.iter().map(|d|d.parameter_fact_id));
            defaults_available &= target.call_defaults_available || bound.defaults.is_empty();
            let bound = bound.explicit;
            if bindings.as_ref().is_some_and(|previous| previous.iter().zip(&bound).any(|(a,b)|
                a.argument_fact_id != b.argument_fact_id || a.parameter_name != b.parameter_name)) {
                valid = false; break;
            }
            for (i, binding) in bound.iter().enumerate() { parameter_evidence[i].push(binding.parameter_fact_id); }
            bindings = Some(bound);
        }
        if !valid { continue; }
        if bindings.is_none() { continue; }
        let mut identity = None;
        for transfer in transfers.get(&key).into_iter().flatten() {
            if transfer.model_id == target.model_id && transfer.pysa_fact_id == target.pysa_fact_id
                && transfer.transfer == ModelTransferKind::Identity
                && transfer.target_modality == Modality::Definite && transfer.model_modality == Modality::Definite
                && transfer.candidate_set_complete_under_model && !transfer.has_unresolved_remainder
                && transfer.input_status == ModelTransferEndpointStatus::BoundArgument
                && transfer.output_status == ModelTransferEndpointStatus::CallResult {
                let Some(source) = transfer.input_expression_fact_id else { continue; };
                if arguments.iter().any(|argument| argument.fact_id == source) {
                    if identity.is_some_and(|(previous, _)| previous != source) { identity = None; break; }
                    identity = Some((source, identity.map_or(transfer.rule_id, |(_, rule): (Id, Id)| rule.min(transfer.rule_id))));
                }
            }
        }
        // Resolve the authored body's returned formal independently of transfer rules.
        let mut return_argument=target.body_return_parameter.as_ref().and_then(|name|
            bindings.as_ref()?.iter().find(|b|&b.parameter_name==name).map(|b|b.argument_fact_id));
        if let Some(name)=&target.body_return_parameter {
            if (0..target.signature_count).any(|index|parameters.iter().filter(|p|p.signature_index==index && p.name.as_ref()==Some(name)).count()!=1) {return_argument=None;}
        }
        let parameter_names=bindings.as_ref().unwrap().iter().map(|b|b.parameter_name.clone()).collect();
        out.insert(key, Some(PreparedCall { target, arguments, parameter_evidence,parameter_names, identity,defaults_available,default_formals,return_argument }));
    }
    out
}

pub fn evaluate(inputs: EvaluationInputs<'_>) -> EvaluationOutcome {
    evaluate_with_limits(inputs, MAX_DEPTH, MAX_WORK)
}

fn evaluate_with_limits(inputs: EvaluationInputs<'_>, depth: usize, work: usize) -> EvaluationOutcome {
    let nodes = inputs.syntax;
    let reads = inputs.reads;
    let calls = prepare_calls(&inputs);
    let mut targets_by_call:HashMap<_,Vec<_>>=HashMap::new();
    for target in inputs.targets {targets_by_call.entry((target.snapshot_id,target.call_node_id)).or_default().push(target);}
    let mut children: HashMap<_, Vec<_>> = HashMap::new();
    for node in nodes {
        children.entry((node.snapshot_id, node.module_node_id, node.parent_node_id)).or_default().push(node);
    }
    for group in children.values_mut() {
        group.sort_by_key(|node| (node.ordinal, node.start_byte, node.fact_id));
    }
    let mut read_index = HashMap::new();
    for read in reads {
        read_index.entry((read.snapshot_id, read.syntax_fact_id))
            .and_modify(|r| *r = None).or_insert(Some(read));
    }
    let mut evaluator = Evaluator { children, remaining: work, depth_limit: depth,
        reads: read_index, proof: Vec::new(), calls,frames:BTreeMap::new() };
    let mut out = EvaluationOutcome::default();
    for node in nodes.iter().filter(|node| node.kind.text().starts_with("expr_") && node.field != SyntaxField::Target) {
        evaluator.remaining = work;
        evaluator.proof.clear();
        let result = evaluator.eval(node, 0);
        let (status, evidence_id) = if result.is_ok() {
            let (_, evidence, status, _) = *evaluator.proof.last().expect("normal expression has a root witness");
            out.steps.extend(evaluator.proof.iter().enumerate().map(|(ordinal, &(operand, evidence, status, kind))|
                ExpressionEvaluationStepsRow { snapshot_id: node.snapshot_id, syntax_fact_id: node.fact_id,
                    ordinal: ordinal as i64, operand_fact_id: operand, evidence_id: evidence, status, kind }));
            (status, Some(evidence))
        } else { (Status::Unknown, None) };
        out.evaluations.push(ExpressionEvaluationsRow {
            snapshot_id: node.snapshot_id, syntax_fact_id: node.fact_id,
            normal: result.is_ok(), boolean_value: match result { Ok(Value::Bool(b)) => Some(b), _ => None },
            release_safety:result.as_ref().map_or(ReleaseSafety::Unknown,|v|v.release()),
            status, evidence_id,
            reason: result.err(), work: work.saturating_sub(evaluator.remaining).max(1) as i64,
        });
        if node.kind==SyntaxKind::ExprCall && let Some(function)=node.owner_node_id {
            if let Some(targets)=targets_by_call.get(&(node.snapshot_id,node.node_id)) && let [target]=targets.as_slice() {
                evaluator.remaining=work;evaluator.proof.clear();
                let reason=if target.argument_count>128 {Some(Refusal::InvocationArgumentLimit)}
                    else {evaluator.invoke(node,0).err()};
                let mut proof=if reason.is_none() {evaluator.proof.iter().map(|&(_,id,_,kind)|(kind,id)).collect()} else {Vec::new()};
                if reason.is_none() {proof.push((Step::ModelInvocation,target.model_id));}
                out.invocations.push(CallInvocation {snapshot_id:node.snapshot_id,function_node_id:function,
                    call_node_id:node.node_id,call_fact_id:target.call_fact_id,syntax_fact_id:node.fact_id,
                    target_node_id:target.target_node_id,pysa_fact_id:target.pysa_fact_id,model_id:target.model_id,
                    argument_count:target.argument_count,
                    default_formals:evaluator.calls.get(&(node.snapshot_id,node.node_id)).and_then(|c|c.as_ref()).map(|c|c.default_formals.clone()),
                    reason,work:work.saturating_sub(evaluator.remaining).max(1) as i64,proof});
            }
        }
    }
    out.evaluations.sort_by_key(|row| (row.snapshot_id, row.syntax_fact_id));
    out.steps.sort_by_key(|row| (row.snapshot_id, row.syntax_fact_id, row.ordinal));
    out.invocations.sort_by_key(|row|(row.snapshot_id,row.call_fact_id));
    for (frame,args,steps) in evaluator.frames.into_values() {
        out.frame_steps.extend(steps.into_iter().map(|s|cpg_schema::frame_exit::ModelFrameExitStepsRow {
            snapshot_id:s.snapshot_id,frame_exit_id:frame.frame_exit_id,ordinal:s.ordinal,operand_fact_id:s.operand_fact_id,
            evidence_id:s.evidence_id,status:s.status,kind:s.kind}));
        out.frames.push(frame);out.frame_arguments.extend(args);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinned_defaults_require_an_independent_promise_and_all_signature_agreement() {
        use cpg_schema::codebook::ParameterKind;
        let id=|n|Id([n;16]);let condition=cpg_schema::condition_kernel::Diagram::always().id();
        let syntax=vec![node(10,0,SyntaxKind::ExprCall,SyntaxField::Value,0,""),
            node(11,10,SyntaxKind::ExprName,SyntaxField::Callee,0,"callee")];
        let target=PinnedCallTarget {snapshot_id:id(1),call_node_id:id(10),call_fact_id:id(12),
            target_node_id:id(20),signature_count:2,body_return_parameter:None,caller_function_scope:true,call_defaults_available:true,
            pysa_fact_id:id(21),model_id:id(22),resolution_fact_id:id(23),import_binding_fact_id:id(24),
            import_region_fact_id:id(25),import_condition_id:condition,argument_count:0};
        let parameters:Vec<_>=(0..2).map(|index|ContextParametersRow {snapshot_id:id(1),fact_id:id(30+index as u8),
            symbol_node_id:id(20),module_node_id:id(2),signature_index:index,form:SignatureForm::List,
            ordinal:Some(0),kind:Some(ParameterKind::KeywordOnly),name:Some("optional".to_owned()),required:Some(false)}).collect();
        let run=|target:&PinnedCallTarget,parameters:&[ContextParametersRow],work|evaluate_with_limits(EvaluationInputs {
            syntax:&syntax,targets:std::slice::from_ref(target),parameters,unconditional_conditions:&[condition],
            ..Default::default()},64,work);
        let out=run(&target,&parameters,MAX_WORK);
        assert_eq!(out.invocations[0].reason,None);
        assert_eq!(out.invocations[0].default_formals,Some(vec![id(30),id(31)]));
        assert!(!out.evaluations.iter().find(|row|row.syntax_fact_id==id(10)).unwrap().normal,
            "available defaults never establish normal return");
        let mut no_promise=target.clone();no_promise.call_defaults_available=false;no_promise.body_return_parameter=Some("optional".into());
        let out=run(&no_promise,&parameters,MAX_WORK);
        assert_eq!(out.invocations[0].reason,Some(Refusal::DefaultUnavailable));
        assert_eq!(out.invocations[0].default_formals,Some(vec![id(30),id(31)]));
        assert!(out.invocations[0].proof.is_empty());
        for mutation in 0..3 {
            let mut changed=parameters.clone();
            match mutation {0=>changed[1].name=Some("different".to_owned()),
                1=>changed[1].required=Some(true),_=>changed[1].required=None}
            let out=run(&target,&changed,MAX_WORK);
            assert_eq!(out.invocations[0].reason,Some(UNSUPPORTED));
            assert!(out.invocations[0].default_formals.is_none());
        }
        let out=run(&target,&parameters,4);
        assert_eq!(out.invocations[0].reason,Some(Refusal::ExpressionWorkLimit));
        assert!(out.invocations[0].proof.is_empty());
        assert_eq!(out.invocations[0].default_formals,Some(vec![id(30),id(31)]));
        assert_eq!(run(&target,&parameters,5).invocations[0].reason,None);
        let mut reversed=parameters.clone();reversed.reverse();
        assert_eq!(run(&target,&reversed,MAX_WORK).invocations[0].proof,
            run(&target,&parameters,MAX_WORK).invocations[0].proof);
        let mut available_body=target.clone();available_body.body_return_parameter=Some("optional".into());
        let out=run(&available_body,&parameters,MAX_WORK);
        assert_eq!(out.invocations[0].reason,None);
        assert_eq!(out.evaluations.iter().find(|r|r.syntax_fact_id==id(10)).unwrap().reason,Some(Refusal::FrameExitCleanup));
        assert!(out.frames.is_empty(),"availability supplies no release premise");
    }

    #[test]
    fn direct_body_release_preserves_provenance_and_rejects_deleted_support() {
        use cpg_schema::codebook::{ParameterKind,ArgumentKind};
        let id=|n|Id([n;16]);let condition=cpg_schema::condition_kernel::Diagram::always().id();
        let mut syntax=vec![node(10,0,SyntaxKind::ExprCall,SyntaxField::Value,0,""),
            node(11,10,SyntaxKind::ExprName,SyntaxField::Callee,0,"callee"),
            node(12,10,SyntaxKind::ExprTuple,SyntaxField::Argument,0,""),
            node(13,12,SyntaxKind::ExprName,SyntaxField::Element,0,"value")];
        let target=PinnedCallTarget {snapshot_id:id(1),call_node_id:id(10),call_fact_id:id(14),target_node_id:id(20),
            signature_count:1,body_return_parameter:Some("val".into()),caller_function_scope:true,call_defaults_available:false,
            pysa_fact_id:id(21),model_id:id(22),resolution_fact_id:id(23),import_binding_fact_id:id(24),
            import_region_fact_id:id(25),import_condition_id:condition,argument_count:1};
        let parameter=ContextParametersRow {snapshot_id:id(1),fact_id:id(30),symbol_node_id:id(20),module_node_id:id(2),
            signature_index:0,form:SignatureForm::List,ordinal:Some(0),kind:Some(ParameterKind::PositionalOnly),name:Some("val".into()),required:Some(true)};
        let arg=ArgumentsRow {snapshot_id:id(1),fact_id:id(31),node_id:id(32),call_node_id:id(10),ordinal:0,
            kind:ArgumentKind::Positional,keyword:None,start_byte:0,end_byte:100,value_start_byte:0,value_end_byte:100};
        let read=ExpressionRead {snapshot_id:id(1),syntax_fact_id:id(13),evidence_id:id(33),status:Status::ParameterNameNormal};
        let run=|nodes:&[SyntaxNodesRow],target:&PinnedCallTarget|evaluate(EvaluationInputs {syntax:nodes,
            targets:std::slice::from_ref(target),call_arguments:std::slice::from_ref(&arg),parameters:std::slice::from_ref(&parameter),
            reads:std::slice::from_ref(&read),unconditional_conditions:&[condition],..Default::default()});
        let out=run(&syntax,&target);
        assert_eq!(out.frames.len(),1);assert_eq!(out.frame_arguments[0].safety,ReleaseSafety::CallerRetained);
        assert_eq!(out.evaluations.iter().find(|r|r.syntax_fact_id==id(10)).unwrap().release_safety,ReleaseSafety::CallerRetained);
        assert!(!out.steps.iter().any(|s|s.kind==Step::ModelRule),"body completion is independent of transfer rules");
        let frame=&out.frames[0];
        let proof:Vec<_>=out.frame_steps.iter().map(|s|ExpressionEvaluationStepsRow {snapshot_id:s.snapshot_id,syntax_fact_id:frame.syntax_fact_id,
            ordinal:s.ordinal,operand_fact_id:s.operand_fact_id,evidence_id:s.evidence_id,status:s.status,kind:s.kind}).collect();
        assert!(cpg_schema::frame_exit::admit(frame,&out.frame_arguments,&proof).is_ok());
        for mutation in 0..7 {
            let mut args=out.frame_arguments.clone();let mut steps=proof.clone();let mut f=frame.clone();
            match mutation {0=>args.clear(),1=>args[0].expression_fact_id=id(90),2=>args[0].parameter_name="other".into(),
                3=>args[0].expression_offset=4,4=>{steps.remove(3);},5=>steps[3].evidence_id=id(90),_=>f.argument_count=i64::MAX}
            // Rehash the row itself; independent member/group commitments still matter.
            f.frame_exit_id=cpg_schema::frame_exit::identity(&f);for a in &mut args {a.frame_exit_id=f.frame_exit_id;}
            assert!(cpg_schema::frame_exit::admit(&f,&args,&steps).is_err(),"mutation {mutation}");
        }
        syntax[3].kind=SyntaxKind::ExprNumberLiteral;syntax[3].detail=Some("1".into());
        let out=run(&syntax,&target);assert_eq!(out.frame_arguments[0].safety,ReleaseSafety::Closed);
        let mut class=target.clone();class.caller_function_scope=false;
        let out=run(&syntax,&class);assert!(out.frames.is_empty());
        assert_eq!(out.invocations[0].reason,None,"under-expression invocation remains separate");
        assert_eq!(out.evaluations.iter().find(|r|r.syntax_fact_id==id(10)).unwrap().reason,Some(Refusal::ScopeBoundary));
        // Nested body completion has no transfer rule. Removing both cleanup markers must
        // still fail from the typed PinnedCallNormal expression conclusion.
        let nested=vec![node(10,0,SyntaxKind::ExprCall,SyntaxField::Value,0,""),
            node(11,10,SyntaxKind::ExprName,SyntaxField::Callee,0,"callee"),
            node(12,10,SyntaxKind::ExprCall,SyntaxField::Argument,0,""),
            node(13,12,SyntaxKind::ExprName,SyntaxField::Callee,0,"callee"),
            node(15,12,SyntaxKind::ExprName,SyntaxField::Argument,0,"value")];
        let mut inner=target.clone();inner.call_node_id=id(12);inner.call_fact_id=id(40);
        let mut inner_arg=arg.clone();inner_arg.call_node_id=id(12);inner_arg.fact_id=id(41);inner_arg.node_id=id(42);
        let mut inner_read=read.clone();inner_read.syntax_fact_id=id(15);
        let out=evaluate(EvaluationInputs {syntax:&nested,targets:&[target,inner],call_arguments:&[arg,inner_arg],
            parameters:&[parameter],reads:&[inner_read],unconditional_conditions:&[condition],..Default::default()});
        assert_eq!(out.frames.len(),2);
        let mut frame=out.frames.iter().find(|f|f.call_node_id==id(10)).unwrap().clone();
        let mut args:Vec<_>=out.frame_arguments.iter().filter(|a|a.frame_exit_id==frame.frame_exit_id).cloned().collect();
        let proof:Vec<_>=out.frame_steps.iter().filter(|s|s.frame_exit_id==frame.frame_exit_id)
            .filter(|s|!matches!(s.kind,Step::ModelFrameExit|Step::PrecedingCallNormal))
            .enumerate().map(|(i,s)|ExpressionEvaluationStepsRow {snapshot_id:s.snapshot_id,syntax_fact_id:frame.syntax_fact_id,
                ordinal:i as i64,operand_fact_id:s.operand_fact_id,evidence_id:s.evidence_id,status:s.status,kind:s.kind}).collect();
        assert!(proof.iter().any(|s|s.status==Status::PinnedCallNormal && s.kind==Step::ExpressionSyntax));
        args[0].expression_count-=2;
        args[0].expression_digest=cpg_schema::frame_exit::steps_digest(&proof[3..3+args[0].expression_count as usize]);
        frame.arguments_digest=cpg_schema::frame_exit::arguments_digest(&args);
        frame.invocation_count=proof.len() as i64;frame.invocation_digest=cpg_schema::frame_exit::steps_digest(&proof);
        frame.frame_exit_id=cpg_schema::frame_exit::identity(&frame);args[0].frame_exit_id=frame.frame_exit_id;
        assert!(cpg_schema::frame_exit::admit(&frame,&args,&proof).is_err());
    }

    fn arguments(nodes: &[SyntaxNodesRow], reads: &[ExpressionRead]) -> EvaluationOutcome {
        let mut out = evaluate(EvaluationInputs { syntax: nodes, reads, ..Default::default() });
        let roots: BTreeSet<_> = nodes.iter().filter(|n| n.field == SyntaxField::Argument).map(|n| n.fact_id).collect();
        out.evaluations.retain(|r| roots.contains(&r.syntax_fact_id));
        out.steps.retain(|r| roots.contains(&r.syntax_fact_id));
        out
    }
    fn closed_arguments(nodes: &[SyntaxNodesRow]) -> Vec<ExpressionEvaluationsRow> {
        arguments(nodes, &[]).evaluations
    }
    fn closed_arguments_with_limits(nodes: &[SyntaxNodesRow], depth: usize, work: usize) -> Vec<ExpressionEvaluationsRow> {
        evaluate_with_limits(EvaluationInputs { syntax: nodes, ..Default::default() }, depth, work)
            .evaluations.into_iter().filter(|r| nodes.iter().any(|n| n.fact_id == r.syntax_fact_id && n.field == SyntaxField::Argument)).collect()
    }

    fn node(id: u8, parent: u8, kind: SyntaxKind, field: SyntaxField,
        ordinal: i64, detail: &str) -> SyntaxNodesRow {
        SyntaxNodesRow {
            snapshot_id: Id([1; 16]), fact_id: Id([id; 16]), node_id: Id([id; 16]),
            module_node_id: Id([2; 16]), owner_node_id: Some(Id([3; 16])),
            parent_node_id: Id([parent; 16]), kind, field, ordinal,
            start_byte: 0, end_byte: 100, detail: Some(detail.to_owned()),
        }
    }

    fn nested_condition(selected: bool) -> Vec<SyntaxNodesRow> {
        use SyntaxField as F;
        use SyntaxKind as K;
        // (not False and True and True) if selected else unknown_call()
        vec![node(10, 0, K::ExprIf, F::Argument, 0, ""),
            node(11, 10, K::ExprBooleanLiteral, F::Test, 0, if selected { "True" } else { "False" }),
            node(12, 10, K::ExprBoolOp, F::Value, 0, "and"),
            node(13, 12, K::ExprUnaryOp, F::Operand, 0, "not"),
            node(14, 13, K::ExprBooleanLiteral, F::Operand, 0, "False"),
            node(15, 12, K::ExprBooleanLiteral, F::Operand, 1, "True"),
            node(16, 12, K::ExprBooleanLiteral, F::Operand, 2, "True"),
            node(17, 10, K::ExprCall, F::Orelse, 0, "")]
    }

    #[test]
    fn nested_selected_expressions_preserve_boolean_values_and_refusals() {
        let nodes = nested_condition(true);
        let rows = closed_arguments(&nodes);
        assert_eq!(rows.len(), 1);
        assert!(rows[0].normal);
        assert_eq!(rows[0].boolean_value, Some(true));
        let refused = closed_arguments(&nested_condition(false));
        assert!(!refused[0].normal);
        assert_eq!(refused[0].reason, Some(UNSUPPORTED));
        assert_eq!(refused[0].boolean_value, None);
        let mut shuffled = nodes;
        shuffled.reverse();
        assert_eq!(closed_arguments(&shuffled), rows);
    }

    #[test]
    fn short_circuit_returns_selected_value_and_never_evaluates_skipped_operand() {
        use SyntaxField as F;
        use SyntaxKind as K;
        for (operator, first, normal, boolean) in [("and", "False", true, Some(false)),
            ("or", "True", true, Some(true)), ("and", "True", false, None),
            ("or", "False", false, None)] {
            let nodes = vec![node(10, 0, K::ExprBoolOp, F::Argument, 0, operator),
                node(11, 10, K::ExprBooleanLiteral, F::Operand, 0, first),
                node(12, 10, K::ExprCall, F::Operand, 1, "")];
            let result = closed_arguments(&nodes);
            assert_eq!((result[0].normal, result[0].boolean_value), (normal, boolean));
        }
        let nodes = vec![node(10, 0, K::ExprBoolOp, F::Argument, 0, "and"),
            node(11, 10, K::ExprBooleanLiteral, F::Operand, 0, "True"),
            node(12, 10, K::ExprNumberLiteral, F::Operand, 1, "42")];
        let result = closed_arguments(&nodes);
        assert!(result[0].normal);
        assert_eq!(result[0].boolean_value, None, "normal numeric result is not an exact Boolean");
    }

    #[test]
    fn tuple_creation_preserves_order_and_withholds_unsupported_elements() {
        let mut nodes=vec![node(1,0,SyntaxKind::ExprTuple,SyntaxField::Argument,0,""),
            node(2,1,SyntaxKind::ExprNumberLiteral,SyntaxField::Element,0,"1"),
            node(3,1,SyntaxKind::ExprStringLiteral,SyntaxField::Element,1,"'two'")];
        let out=arguments(&nodes,&[]);
        assert!(out.evaluations[0].normal);
        assert_eq!(out.steps.iter().map(|s|s.operand_fact_id).collect::<Vec<_>>(),nodes[1..].iter().chain(&nodes[..1]).map(|n|n.fact_id).collect::<Vec<_>>());
        nodes[2].kind=SyntaxKind::ExprName;nodes[2].detail=Some("missing".to_owned());
        assert!(!arguments(&nodes,&[]).evaluations[0].normal);
        nodes[2].kind=SyntaxKind::ExprStarred;
        assert!(!arguments(&nodes,&[]).evaluations[0].normal);
    }

    #[test]
    fn numeric_composition_is_closed_and_bounded() {
        use SyntaxField as F;
        use SyntaxKind as K;
        let mut nodes = vec![node(10, 0, K::ExprBinOp, F::Argument, 0, "+"),
            node(11, 10, K::ExprUnaryOp, F::Left, 0, "-"),
            node(12, 11, K::ExprNumberLiteral, F::Operand, 0, "0x10"),
            node(13, 10, K::ExprNumberLiteral, F::Right, 0, "2.5")];
        assert!(closed_arguments(&nodes)[0].normal);
        nodes[3].detail = Some("9".repeat(400));
        assert_eq!(closed_arguments(&nodes)[0].reason, Some(UNSUPPORTED));
        nodes[0].detail = Some("/".to_owned());
        assert_eq!(closed_arguments(&nodes)[0].reason, Some(UNSUPPORTED));
    }

    #[test]
    fn malformed_children_and_each_budget_are_explicit_unknowns() {
        let nodes = nested_condition(true);
        assert_eq!(closed_arguments_with_limits(&nodes, 0, 1024)[0].reason,
            Some(Refusal::ExpressionDepthLimit));
        assert_eq!(closed_arguments_with_limits(&nodes, 64, 1)[0].reason,
            Some(Refusal::ExpressionWorkLimit));
        let mut malformed = nodes.clone();
        malformed[6].ordinal = 1;
        assert_eq!(closed_arguments(&malformed)[0].reason, Some(UNSUPPORTED));
        let mut malformed = nodes;
        malformed[4].owner_node_id = None;
        assert_eq!(closed_arguments(&malformed)[0].reason, Some(UNSUPPORTED));
    }
    #[test]
    fn nested_reads_require_source_certificates_and_preserve_selected_operand_order() {
        use SyntaxField as F;
        use SyntaxKind as K;
        let nodes = vec![node(10, 0, K::ExprIf, F::Argument, 0, ""),
            node(11, 10, K::ExprBooleanLiteral, F::Test, 0, "True"),
            node(12, 10, K::ExprName, F::Value, 0, "value"),
            node(13, 10, K::ExprName, F::Orelse, 0, "unbound")];
        let read = ExpressionRead { snapshot_id: Id([1; 16]), syntax_fact_id: Id([12; 16]),
            evidence_id: Id([90; 16]), status: Status::ParameterNameNormal };
        assert!(!arguments(&nodes, &[]).evaluations[0].normal);
        let outcome = arguments(&nodes, std::slice::from_ref(&read));
        assert!(outcome.evaluations[0].normal);
        assert_eq!(outcome.evaluations[0].boolean_value, None);
        assert_eq!(outcome.steps.iter().map(|s| s.operand_fact_id).collect::<Vec<_>>(),
            vec![Id([11; 16]), Id([12; 16]), Id([10; 16])]);
        assert_eq!(outcome.steps[1].evidence_id, Id([90; 16]));
        let duplicate = vec![read.clone(), read.clone()];
        assert!(!arguments(&nodes, &duplicate).evaluations[0].normal);
        let mut wrong_snapshot = read.clone(); wrong_snapshot.snapshot_id = Id([99; 16]);
        assert!(!arguments(&nodes, &[wrong_snapshot]).evaluations[0].normal);
        let opaque_truth = vec![node(10, 0, K::ExprUnaryOp, F::Argument, 0, "not"),
            node(12, 10, K::ExprName, F::Operand, 0, "value")];
        let refused = arguments(&opaque_truth, &[read]);
        assert!(!refused.evaluations[0].normal, "normal read does not prove arbitrary __bool__ completion");
        assert!(refused.steps.is_empty());
    }

}
