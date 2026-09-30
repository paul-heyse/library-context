//! The provider-native entities one Pyrefly session names: the modules it resolved and the
//! symbols Pysa keys in them. A module over captured bytes is the typed module of that artifact;
//! a bundled stub is the provider's, per bundle; a namespace package or an unresolved name is the
//! provider's in the session's analysis context. Bytes outside the capture are refused: the
//! analyzer reads only captured inputs.
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use lctx_model::domain::{Id, ModelError, Record, attribution::{AnalysisContext, Provider},
    calls::{ModuleBundle, ProviderModule, ProviderSymbol, SymbolKind}, source::{Module, SourceArtifact}};
use pyrefly_python::module_path::{ModulePath, ModulePathDetails};

fn invalid(message: String) -> ModelError { ModelError::Invalid(message) }

/// Where a resolved module came from, as the provider reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolution { pub module: Id<ProviderModule>, pub location: Option<String> }

/// Interns one session's provider modules and symbols; every entity is emitted once.
pub struct Natives {
    provider: Id<Provider>,
    context: Id<AnalysisContext>,
    /// The frozen roots, and their captured artifacts by absolute path.
    roots: Vec<PathBuf>,
    artifacts: HashMap<PathBuf, Id<SourceArtifact>>,
    /// Modules over captured artifacts, keyed by their provider module.
    pub modules: BTreeMap<Id<ProviderModule>, (ProviderModule, Option<Module>)>,
    pub symbols: BTreeMap<Id<ProviderSymbol>, ProviderSymbol>,
    /// Each non-analyzed module's resolution, by provider module.
    pub resolutions: BTreeMap<Id<ProviderModule>, Resolution>,
}
impl Natives {
    /// `roots` pairs each frozen root with its captured artifacts (relative paths).
    pub fn new<'a>(provider: Id<Provider>, context: Id<AnalysisContext>, roots: impl IntoIterator<Item = (&'a Path, &'a [SourceArtifact])>) -> Self {
        let (mut frozen, mut artifacts) = (Vec::new(), HashMap::new());
        for (root, captured) in roots {
            frozen.push(root.to_path_buf());
            artifacts.extend(captured.iter().map(|a| (root.join(&a.path), a.id())));
        }
        Self { provider, context, roots: frozen, artifacts, modules: BTreeMap::new(), symbols: BTreeMap::new(), resolutions: BTreeMap::new() }
    }
    /// The provider module of a module the provider found at `path`.
    pub fn module(&mut self, name: &str, path: &ModulePath) -> Result<Id<ProviderModule>, ModelError> {
        let bundled = |bundle: ModuleBundle| ProviderModule::Bundled { provider: self.provider, bundle, name: name.to_owned() };
        let (module, typed, location) = match path.details() {
            ModulePathDetails::FileSystem(file) => {
                let artifact = *self.artifacts.get(file.as_path())
                    .ok_or_else(|| invalid(format!("the analyzer read {} outside the captured inputs", file.display())))?;
                let module = Module { source: artifact, qualified_name: name.to_owned() };
                (ProviderModule::Acquired { module: module.id() }, Some(module), None)
            }
            ModulePathDetails::Namespace(directory) => {
                let location = self.relative(directory.as_path())?;
                (ProviderModule::Namespace { provider: self.provider, context: self.context, name: name.to_owned() }, None, Some(location))
            }
            ModulePathDetails::BundledTypeshed(p) => (bundled(ModuleBundle::Typeshed), None, Some(p.display().to_string())),
            ModulePathDetails::BundledTypeshedThirdParty(p) => (bundled(ModuleBundle::TypeshedThirdParty), None, Some(p.display().to_string())),
            ModulePathDetails::BundledThirdParty(p) => (bundled(ModuleBundle::ThirdParty), None, Some(p.display().to_string())),
            ModulePathDetails::Memory(p) => return Err(invalid(format!("an in-memory module {} is outside the captured inputs", p.display()))),
        };
        let id = module.id();
        self.modules.entry(id).or_insert((module, typed));
        self.resolutions.entry(id).or_insert(Resolution { module: id, location });
        Ok(id)
    }
    /// A module the provider could not find, under its spelling.
    pub fn unresolved(&mut self, name: &str) -> Id<ProviderModule> {
        let module = ProviderModule::Unresolved { provider: self.provider, context: self.context, name: name.to_owned() };
        let id = module.id();
        self.modules.entry(id).or_insert((module, None));
        self.resolutions.entry(id).or_insert(Resolution { module: id, location: None });
        id
    }
    /// A directory's path relative to the frozen root holding it.
    fn relative(&self, directory: &Path) -> Result<String, ModelError> {
        self.roots.iter().find_map(|root| directory.strip_prefix(root).ok().map(|p| p.display().to_string())).filter(|p| !p.is_empty())
            .ok_or_else(|| invalid(format!("the namespace package {} is outside the captured inputs", directory.display())))
    }
    /// The symbol Pysa keys `key` in `module`.
    pub fn symbol(&mut self, module: Id<ProviderModule>, key: String, name: String, kind: SymbolKind) -> Result<Id<ProviderSymbol>, ModelError> {
        let symbol = ProviderSymbol { provider: self.provider, context: self.context, module, native_key: key, name, kind };
        let id = symbol.id();
        match self.symbols.get(&id) {
            Some(existing) if *existing != symbol => Err(invalid(format!("Pysa keys two symbols as {}", symbol.native_key))),
            Some(_) => Ok(id),
            None => { self.symbols.insert(id, symbol); Ok(id) }
        }
    }
}
