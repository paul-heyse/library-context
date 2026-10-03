//! One latest-Ruff parse of verified captured bytes, shared by syntax and contextual observation.
use lctx_model::domain::{
    ContentHash, ModelError,
    attribution::AnalysisContext,
    resources::{Reservation, ResourceBudget},
    source::SourceArtifact,
};
use ruff_linter::semantic_facts::{Incomplete, Settings, Sink, TraversalStats, observe_parsed};
use ruff_python_ast_latest::{ModModule, PySourceType, PythonVersion};
use ruff_python_parser_latest::{ParseOptions, Parsed, parse_unchecked};
use std::path::{Path, PathBuf};

pub const RUFF_REVISION: &str = "8f01d80020921d3867f255ee5f919dd2d329b730";
pub const RUFF_PATCH_SHA256: &str = "7da3c6617b8979bb5760c59a4def6fdb72fe177eff02b2aa229ff46d9bf0ff74";

/// These are interpretation inputs, never discovered from ambient files or process state.
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
            parsed, source,
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
    pub fn observe(&self, sink: &mut dyn Sink) -> Result<TraversalStats, Incomplete> {
        observe_parsed(self.source, &self.parsed, &self.settings, sink)
    }
}
use lctx_model::domain::Record;
