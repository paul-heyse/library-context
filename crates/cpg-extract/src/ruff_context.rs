//! One latest-Ruff parse of verified captured bytes, shared by syntax and contextual observation.
use lctx_model::domain::{
    ContentHash, HeapSize, ModelError,
    attribution::AnalysisContext,
    resources::{Reservation, ResourceBudget},
    source::SourceArtifact,
};
use ruff_linter::semantic_facts::{Incomplete, Settings, Sink, TraversalStats, observe_parsed};
use ruff_python_ast_latest::{ModModule, PySourceType, PythonVersion};
use ruff_python_parser_latest::{ParseOptions, Parsed, parse_unchecked};
use std::path::{Path, PathBuf};

pub const RUFF_REVISION: &str = "f7bdff69e1fb94ab0ed5b340e977aac0d26e9301";
pub const RUFF_PATCH_SHA256: &str = "b7154806d8d5106f02c225d35c4cb8802c869d551af4ce1dd2d391178deaa6f8";

/// These are interpretation inputs, never discovered from ambient files or process state.
#[derive(Clone, serde::Serialize)]
pub struct ContextSettings {
    pub typing_modules: Vec<String>,
    pub custom_builtins: Vec<String>,
    pub maximum_rows: usize,
    pub maximum_node_visits: usize,
}
impl Default for ContextSettings {
    fn default() -> Self {
        Self { typing_modules: vec![], custom_builtins: vec![], maximum_rows: 1_000_000, maximum_node_visits: 8_000_000 }
    }
}
pub struct CanonicalSyntax<'a> {
    source_id: lctx_model::domain::Id<SourceArtifact>,
    context_id: lctx_model::domain::Id<AnalysisContext>,
    parsed: Parsed<ModModule>,
    source: &'a str,
    settings: Settings,
    _parse: Box<dyn Reservation>,
}
impl<'a> CanonicalSyntax<'a> {
    pub fn parse(
        artifact: &SourceArtifact,
        source: &'a str,
        context: &AnalysisContext,
        settings: ContextSettings,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        artifact.validate()?;
        if artifact.byte_len != source.len() as i64 || artifact.content != ContentHash::of(source.as_bytes()) {
            return Err(ModelError::Invalid("canonical Ruff source differs from captured snapshot".into()));
        }
        let mut version = context.python_version.split('.');
        let major = version.next().ok_or_else(|| ModelError::Invalid("missing Python major version".into()))?;
        let minor = version.next().ok_or_else(|| ModelError::Invalid("missing Python minor version".into()))?;
        let python_version = PythonVersion::try_from((major, minor)).map_err(|error| ModelError::Invalid(error.to_string()))?;
        let source_type = if artifact.is_stub() { PySourceType::Stub } else { PySourceType::Python };
        let allocation = source.len().checked_mul(64).and_then(|n| n.checked_add(65536))
            .ok_or_else(|| ModelError::Invalid("canonical Ruff parse allowance overflow".into()))?;
        let reservation = budget.reserve("canonical-ruff-parse", allocation)?;
        let parsed = parse_unchecked(source, ParseOptions::from(source_type).with_target_version(python_version))
            .try_into_module().ok_or_else(|| ModelError::Invalid("canonical Ruff requires module source".into()))?;
        Ok(Self {
            source_id:artifact.id(),context_id:context.id(),parsed, source,
            settings: Settings {
                path: PathBuf::from("/captured").join(&artifact.path),
                package_root: Some(Path::new("/captured").into()),
                source_type, python_version, platform: context.python_platform.clone(),
                typing_modules: settings.typing_modules, custom_builtins: settings.custom_builtins,
                max_rows: settings.maximum_rows, max_node_visits: settings.maximum_node_visits,
            },
            _parse: reservation,
        })
    }
    pub fn module(&self) -> &ModModule { self.parsed.syntax() }
    pub fn parsed(&self) -> &Parsed<ModModule> { &self.parsed }
    pub fn diagnostics(&self,artifact:&SourceArtifact,qualification:&lctx_model::domain::assertion::AssertionQualification,budget:&ResourceBudget)->Result<crate::diagnostic_records::Records,ModelError> {
        if artifact.id()!=self.source_id || qualification.context!=self.context_id {return Err(ModelError::Invalid("selected Ruff diagnostics differ from captured source/context".into()));}
        crate::diagnostic_records::ruff(artifact,self.source,&self.parsed,&self.settings,qualification,budget)
    }
    pub fn observe(&self, sink: &mut dyn Sink) -> Result<TraversalStats, Incomplete> {
        observe_parsed(self.source, &self.parsed, &self.settings, sink)
    }
}
use lctx_model::domain::Record;

pub fn provider() -> lctx_model::domain::attribution::Provider {
    lctx_model::domain::attribution::Provider {
        tool:"ruff".into(), revision:RUFF_REVISION.into(),
        build_digest:crate::bundle::build_digest(&[include_str!("ruff_context.rs"),include_str!("ruff_lexical.rs"),include_str!("diagnostic_records.rs"),include_str!("typed_syntax.rs"),include_str!("syntax_records.rs")]),
    }
}
/// Native integers stay local to one parse. Only canonical occurrence/context identities escape.
pub struct ContextRows {
    source_id:lctx_model::domain::Id<SourceArtifact>,
    context_id:lctx_model::domain::Id<AnalysisContext>,
    pub rows: Vec<lctx_model::domain::ruff::RuffContextObservation>,
    pub unlocated: usize,
    pub incomplete: Option<Incomplete>,
    _charge: lctx_model::domain::charged::StateCharge,
    native: crate::ruff_lexical::NativeRows,
}
impl CanonicalSyntax<'_> {
    pub fn context_rows(&self, spans:&crate::syntax_records::Spans, qualification:&lctx_model::domain::assertion::AssertionQualification, budget:&ResourceBudget) -> Result<ContextRows,ModelError> {
        if spans.source()!=Some(self.source_id)||qualification.context!=self.context_id {return Err(ModelError::Invalid("Ruff contextual attachment differs from captured source/context".into()));}
        use lctx_model::domain::{Id, source::OccurrenceRole, ruff::{ContextPhase,RuffContextObservation}, assertion::AssertionQualification};
        use ruff_linter::semantic_facts::{Fact,NodeOrigin,StopReason};
        use ruff_python_semantic::SemanticModelFlags as Flags;
        use std::collections::{BTreeMap,BTreeSet};
        struct Collect<'a> {
            spans:&'a crate::syntax_records::Spans, qualification:Id<AssertionQualification>,
            output:ContextRows, bindings:BTreeMap<u32,Option<Vec<String>>>, seen:BTreeSet<Id<RuffContextObservation>>, error:Option<ModelError>,
        }
        impl Sink for Collect<'_> {
            fn observe(&mut self,fact:Fact)->Result<(),StopReason> {
                self.output.native.capture(&fact, &mut self.output._charge).map_err(|error| { self.error=Some(error);StopReason::Sink("native lexical resource allowance".into()) })?;
                let row=match fact {
                    Fact::Node(node) if node.origin==NodeOrigin::Source => {
                        let subject=match self.spans.get(node.range,crate::typed_syntax::kind(node.kind)) { Ok(id)=>id,Err(_)=>{self.output.unlocated+=1;return Ok(());} };
                        if let Some(expected)=node.expr_context.map(|role|match role {ruff_python_ast_latest::ExprContext::Load=>OccurrenceRole::Read,ruff_python_ast_latest::ExprContext::Store=>OccurrenceRole::Binding,_=>OccurrenceRole::Syntax}) {
                            if node.kind==ruff_python_ast_latest::NodeKind::ExprName && self.spans.role_of(subject)!=Some(expected) {self.output.unlocated+=1;return Ok(());}
                        }
                        let flags=Flags::from_bits_retain(node.flags);
                        Some(RuffContextObservation {
                            qualification:self.qualification,subject,phase:ContextPhase::ActiveNode,reference_load:None,
                            typing:Some(node.context.is_typing()), typing_only_annotation:Some(flags.contains(Flags::TYPING_ONLY_ANNOTATION)),
                            runtime_annotation:Some(flags.contains(Flags::RUNTIME_EVALUATED_ANNOTATION)),
                            string_annotation:Some(flags.intersects(Flags::SIMPLE_STRING_TYPE_DEFINITION|Flags::COMPLEX_STRING_TYPE_DEFINITION)),
                            type_checking:Some(flags.contains(Flags::TYPE_CHECKING_BLOCK)),qualified_name:node.qualified_name,
                            final_binding:None,final_binding_location:None,unresolved_wildcard:None,unresolved_annotation_binding:None,
                        })
                    }
                    Fact::Binding(binding)=> {
                        let bytes=128+binding.qualified_name.as_ref().map_or(0,|names| names.iter().map(|name|name.len()+64).sum());
                        if let Err(error)=self.output._charge.grow(bytes) { self.error=Some(error);return Err(StopReason::Sink("context resource allowance".into())); }
                        self.bindings.insert(binding.id,binding.qualified_name);None
                    }
                    Fact::Reference(reference)=> {
                        let Some(subject)=self.spans.reference(reference.range,reference.is_load) else {self.output.unlocated+=1;return Ok(());};
                        Some(RuffContextObservation { qualification:self.qualification,subject,phase:ContextPhase::FinalReference,reference_load:Some(reference.is_load),
                            typing:Some(reference.typing_context),typing_only_annotation:Some(reference.typing_only_annotation),
                            runtime_annotation:Some(reference.runtime_annotation),string_annotation:Some(reference.string_annotation),
                            type_checking:Some(reference.type_checking),qualified_name:self.bindings.get(&reference.binding).cloned().flatten(),
                            final_binding:None,final_binding_location:Some(lctx_model::domain::ruff::AttachmentStatus::Unlocated),unresolved_wildcard:None,unresolved_annotation_binding:None,
                        })
                    }
                    _=>None,
                };
                if let Some(row)=row {
                    let bytes=512+row.qualified_name.as_ref().map_or(0,|names|names.iter().map(|name|name.len()+64).sum());
                    if let Err(error)=self.output._charge.grow(bytes) {self.error=Some(error);return Err(StopReason::Sink("context resource allowance".into()));}
                    if self.seen.insert(row.id()) { self.output.rows.push(row); } else { self.output._charge.release(bytes); }
                }
                Ok(())
            }
        }
        let mut sink=Collect { spans,qualification:qualification.id(),output:ContextRows {source_id:self.source_id,context_id:self.context_id,rows:vec![],unlocated:0,incomplete:None,native:Default::default(),_charge:lctx_model::domain::charged::StateCharge::new(budget,"ruff-context-rows")},bindings:BTreeMap::new(),seen:BTreeSet::new(),error:None };
        sink.output.incomplete=self.observe(&mut sink).err();
        if let Some(error)=sink.error {return Err(error);}
        sink.output.rows.sort_by_key(Record::id);
        Ok(sink.output)
    }
}

impl ContextRows {
    /// Borrowed native IDs are translated within this retained traversal before any publication.
    pub fn lower_lexical(&mut self, spans:&crate::syntax_records::Spans, source:&crate::lexical_records::LexicalRecords, qualification:&lctx_model::domain::assertion::AssertionQualification, candidate:lctx_model::domain::Id<lctx_model::domain::assertion::AssertionQualification>, budget:&ResourceBudget) -> Result<crate::ruff_lexical::NativeLexicalRecords,ModelError> {
        if spans.source()!=Some(self.source_id) || qualification.context!=self.context_id {return Err(ModelError::Invalid("final Ruff lexical attachment differs from retained source/context".into()));}
        let lowered=self.native.lower(spans,source,qualification,candidate,budget)?;
        self.rows.retain(|row|row.phase!=lctx_model::domain::ruff::ContextPhase::FinalReference);
        for row in &lowered.contexts { self._charge.grow(1024+row.heap_bytes())?;self.rows.push(row.clone()); }
        self.unlocated+=lowered.unlocated;
        self.rows.sort_by_key(Record::id);self.rows.dedup_by_key(|row|row.id());
        Ok(lowered)
    }
}
