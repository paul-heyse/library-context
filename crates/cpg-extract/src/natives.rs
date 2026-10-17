//! The provider-native entities one Pyrefly session names: the modules it resolved and the
//! symbols Pysa keys in them. A module over captured bytes is the typed module of that artifact;
//! a bundled stub is the provider's, per bundle; a namespace package or an unresolved name is the
//! provider's in the session's analysis context. Bytes outside the capture are refused: the
//! analyzer reads only captured inputs.
use lctx_model::domain::{
    HeapSize, Id, ModelError, Record,
    attribution::{AnalysisContext, Provider},
    calls::{ModuleBundle, ProviderModule, ProviderSymbol, SymbolKind},
    charged::StateCharge,
    resources::ResourceBudget,
    source::{Module, SourceArtifact},
};
use pyrefly_python::module_path::{ModulePath, ModulePathDetails};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

fn invalid(message: String) -> ModelError {
    ModelError::Invalid(message)
}

/// Where a resolved module came from, as the provider reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolution {
    pub module: Id<ProviderModule>,
    pub location: Option<String>,
}

#[derive(Clone, Copy)]
struct SymbolCandidates {
    first: Id<ProviderSymbol>,
    ambiguous: bool,
}
impl SymbolCandidates {
    fn include(&mut self, symbol: Id<ProviderSymbol>) {
        self.ambiguous |= self.first != symbol;
        self.first = self.first.min(symbol);
    }
}

/// Derived candidates share the native inventory's lifetime and insertion authority.
#[derive(Default)]
struct SymbolLookup(BTreeMap<Id<ProviderModule>, BTreeMap<String, SymbolCandidates>>);
impl SymbolLookup {
    fn get(&self, module: Id<ProviderModule>, key: &str) -> Option<&SymbolCandidates> {
        self.0.get(&module)?.get(key)
    }
    fn admission(&self, module: Id<ProviderModule>, key: &str) -> usize {
        let module_bytes = if self.0.contains_key(&module) {
            0
        } else {
            size_of::<(Id<ProviderModule>, BTreeMap<String, SymbolCandidates>)>() + 128
        };
        let key_bytes = if self.get(module, key).is_some() {
            0
        } else {
            size_of::<(String, SymbolCandidates)>()
                .saturating_add(key.len())
                .saturating_add(128)
        };
        module_bytes.saturating_add(key_bytes)
    }
    /// The caller admits all growth before either native inventory is changed.
    fn insert_admitted(
        &mut self,
        module: Id<ProviderModule>,
        key: &str,
        symbol: Id<ProviderSymbol>,
    ) {
        let keys = self.0.entry(module).or_default();
        match keys.get_mut(key) {
            Some(candidates) => candidates.include(symbol),
            None => {
                keys.insert(
                    key.to_owned(),
                    SymbolCandidates {
                        first: symbol,
                        ambiguous: false,
                    },
                );
            }
        }
    }
}

/// Interns one session's provider modules and symbols; every entity is emitted once.
pub struct Natives {
    /// Definitions actually retained, distinct from native reference-only symbols.
    pub definitions: BTreeMap<(Id<ProviderModule>, String), Id<ProviderSymbol>>,
    pub mro: BTreeMap<Id<ProviderSymbol>, lctx_model::domain::symbols::Linearization>,
    provider: Id<Provider>,
    context: Id<AnalysisContext>,
    /// The frozen roots, and their captured artifacts by absolute path.
    roots: Vec<PathBuf>,
    artifacts: HashMap<PathBuf, Id<SourceArtifact>>,
    /// Modules over captured artifacts, keyed by their provider module.
    pub modules: BTreeMap<Id<ProviderModule>, (ProviderModule, Option<Module>)>,
    symbols: BTreeMap<Id<ProviderSymbol>, ProviderSymbol>,
    symbol_lookup: SymbolLookup,
    /// Each module's resolution, by provider module.
    pub resolutions: BTreeMap<Id<ProviderModule>, Resolution>,
    /// Each found module's name and path, from which its handle is rebuilt.
    pub paths: BTreeMap<Id<ProviderModule>, (String, ModulePath)>,
    // Retain admission until every owned map and its derived lookup have been dropped.
    charge: StateCharge,
}
impl Natives {
    /// `roots` pairs each frozen root with its captured artifacts (relative paths).
    pub fn new<'a>(
        provider: Id<Provider>,
        context: Id<AnalysisContext>,
        roots: impl IntoIterator<Item = (&'a Path, &'a [SourceArtifact])>,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        let mut charge = StateCharge::new(budget, "pyrefly_native_index");
        let (mut frozen, mut artifacts) = (Vec::new(), HashMap::new());
        for (root, captured) in roots {
            charge.grow(root.as_os_str().len().saturating_mul(2).saturating_add(128))?;
            for artifact in captured {
                charge.grow(
                    root.as_os_str()
                        .len()
                        .saturating_add(artifact.path.len())
                        .saturating_mul(2)
                        .saturating_add(192),
                )?;
            }
            frozen.push(root.to_path_buf());
            artifacts.extend(captured.iter().map(|a| (root.join(&a.path), a.id())));
        }
        Ok(Self {
            definitions: BTreeMap::new(),
            mro: BTreeMap::new(),
            provider,
            context,
            charge,
            roots: frozen,
            artifacts,
            modules: BTreeMap::new(),
            symbols: BTreeMap::new(),
            symbol_lookup: SymbolLookup::default(),
            resolutions: BTreeMap::new(),
            paths: BTreeMap::new(),
        })
    }
    pub fn definition(
        &mut self,
        module: Id<ProviderModule>,
        name: String,
        symbol: Id<ProviderSymbol>,
    ) -> Result<(), ModelError> {
        let key = (module, name);
        if !self.definitions.contains_key(&key) {
            self.charge.grow(key.1.len().saturating_add(96))?;
        }
        self.definitions.insert(key, symbol);
        Ok(())
    }
    pub fn linearization(
        &mut self,
        symbol: Id<ProviderSymbol>,
        value: lctx_model::domain::symbols::Linearization,
    ) -> Result<(), ModelError> {
        if !self.mro.contains_key(&symbol) {
            self.charge.grow(64)?;
        }
        self.mro.insert(symbol, value);
        Ok(())
    }
    /// The provider module of a module the provider found at `path`.
    pub fn module(
        &mut self,
        name: &str,
        path: &ModulePath,
    ) -> Result<Id<ProviderModule>, ModelError> {
        let bundled = |bundle: ModuleBundle| ProviderModule::Bundled {
            provider: self.provider,
            bundle,
            name: name.to_owned(),
        };
        let (module, typed, location) = match path.details() {
            ModulePathDetails::FileSystem(file) => {
                let artifact = *self.artifacts.get(file.as_path()).ok_or_else(|| {
                    invalid(format!(
                        "the analyzer read {} outside the captured inputs",
                        file.display()
                    ))
                })?;
                let module = Module {
                    source: artifact,
                    qualified_name: name.to_owned(),
                };
                (
                    ProviderModule::Acquired {
                        module: module.id(),
                    },
                    Some(module),
                    None,
                )
            }
            ModulePathDetails::Namespace(directory) => {
                let location = self.relative(directory.as_path())?;
                (
                    ProviderModule::Namespace {
                        provider: self.provider,
                        context: self.context,
                        name: name.to_owned(),
                    },
                    None,
                    Some(location),
                )
            }
            ModulePathDetails::BundledTypeshed(p) => (
                bundled(ModuleBundle::Typeshed),
                None,
                Some(p.display().to_string()),
            ),
            ModulePathDetails::BundledTypeshedThirdParty(p) => (
                bundled(ModuleBundle::TypeshedThirdParty),
                None,
                Some(p.display().to_string()),
            ),
            ModulePathDetails::BundledThirdParty(p) => (
                bundled(ModuleBundle::ThirdParty),
                None,
                Some(p.display().to_string()),
            ),
            ModulePathDetails::Memory(p) => {
                return Err(invalid(format!(
                    "an in-memory module {} is outside the captured inputs",
                    p.display()
                )));
            }
        };
        let id = module.id();
        if !self.modules.contains_key(&id) {
            self.charge.grow(
                module
                    .heap_bytes()
                    .saturating_add(typed.as_ref().map_or(0, HeapSize::heap_bytes))
                    .saturating_add(name.len().saturating_mul(2))
                    .saturating_add(location.as_ref().map_or(0, String::len).saturating_mul(2))
                    .saturating_add(768),
            )?;
        }
        self.modules.entry(id).or_insert((module, typed));
        self.resolutions.entry(id).or_insert(Resolution {
            module: id,
            location,
        });
        self.paths
            .entry(id)
            .or_insert_with(|| (name.to_owned(), path.clone()));
        Ok(id)
    }
    /// A module the provider could not find, under its spelling.
    pub fn unresolved(&mut self, name: &str) -> Result<Id<ProviderModule>, ModelError> {
        self.charge
            .grow(name.len().saturating_mul(2).saturating_add(512))?;
        let module = ProviderModule::Unresolved {
            provider: self.provider,
            context: self.context,
            name: name.to_owned(),
        };
        let id = module.id();
        self.modules.entry(id).or_insert((module, None));
        self.resolutions.entry(id).or_insert(Resolution {
            module: id,
            location: None,
        });
        Ok(id)
    }
    /// A directory's path relative to the frozen root holding it.
    fn relative(&self, directory: &Path) -> Result<String, ModelError> {
        self.roots
            .iter()
            .find_map(|root| {
                directory
                    .strip_prefix(root)
                    .ok()
                    .map(|p| p.display().to_string())
            })
            .filter(|p| !p.is_empty())
            .ok_or_else(|| {
                invalid(format!(
                    "the namespace package {} is outside the captured inputs",
                    directory.display()
                ))
            })
    }
    /// The symbol Pysa keys `key` in `module`.
    pub fn symbol(
        &mut self,
        module: Id<ProviderModule>,
        key: String,
        name: String,
        kind: SymbolKind,
    ) -> Result<Id<ProviderSymbol>, ModelError> {
        let symbol = ProviderSymbol {
            provider: self.provider,
            context: self.context,
            module,
            native_key: key,
            name,
            kind,
        };
        self.retain_symbol(symbol)
    }
    pub fn symbols(&self) -> &BTreeMap<Id<ProviderSymbol>, ProviderSymbol> {
        &self.symbols
    }
    /// Preserve the former Id-ordered inventory scan's first candidate.
    pub fn first_symbol(
        &self,
        module: Id<ProviderModule>,
        key: &str,
    ) -> Option<Id<ProviderSymbol>> {
        self.symbol_lookup
            .get(module, key)
            .map(|candidates| candidates.first)
    }
    pub fn unique_symbol(
        &self,
        module: Id<ProviderModule>,
        key: &str,
    ) -> Result<Option<Id<ProviderSymbol>>, ModelError> {
        match self.symbol_lookup.get(module, key) {
            None => Ok(None),
            Some(candidates) if !candidates.ambiguous => Ok(Some(candidates.first)),
            Some(_) => Err(invalid("capture native function key is ambiguous".into())),
        }
    }
    fn retain_symbol(&mut self, symbol: ProviderSymbol) -> Result<Id<ProviderSymbol>, ModelError> {
        let id = symbol.id();
        match self.symbols.get(&id) {
            Some(existing) if *existing != symbol => Err(invalid(format!(
                "Pysa keys two symbols as {}",
                symbol.native_key
            ))),
            Some(_) => Ok(id),
            None => {
                self.charge.grow(
                    symbol.heap_bytes().saturating_add(256).saturating_add(
                        self.symbol_lookup
                            .admission(symbol.module, &symbol.native_key),
                    ),
                )?;
                self.symbol_lookup
                    .insert_admitted(symbol.module, &symbol.native_key, id);
                self.symbols.insert(id, symbol);
                Ok(id)
            }
        }
    }
}

#[cfg(test)]
mod symbol_lookup_controls {
    use super::*;
    fn id<R>(n: u8) -> Id<R> {
        serde_json::from_value(serde_json::json!(vec![n; 16])).unwrap()
    }
    #[test]
    fn native_keys_are_live_reference_only_module_scoped_and_idempotent() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let mut natives = Natives::new(id(1), id(2), std::iter::empty(), &budget).unwrap();
        let module = id(3);
        assert_eq!(natives.first_symbol(module, "F:1"), None);
        assert_eq!(natives.unique_symbol(module, "F:1").unwrap(), None);
        let symbol = natives
            .symbol(
                module,
                "F:1".into(),
                "function".into(),
                SymbolKind::Function,
            )
            .unwrap();
        let before = budget.reserved();
        assert_eq!(natives.first_symbol(module, "F:1"), Some(symbol));
        assert_eq!(natives.unique_symbol(module, "F:1").unwrap(), Some(symbol));
        assert_eq!(
            natives
                .symbol(
                    module,
                    "F:1".into(),
                    "function".into(),
                    SymbolKind::Function
                )
                .unwrap(),
            symbol
        );
        assert_eq!(
            budget.reserved(),
            before,
            "repeated identity adds no index entry"
        );
        assert!(
            matches!(natives.symbol(module, "F:1".into(), "function".into(), SymbolKind::Class),
            Err(ModelError::Invalid(message)) if message == "Pysa keys two symbols as F:1")
        );
        assert_eq!(budget.reserved(), before);
        assert_eq!(natives.symbols()[&symbol].kind, SymbolKind::Function);
        natives
            .symbol(
                id(4),
                "F:1".into(),
                "other module".into(),
                SymbolKind::Class,
            )
            .unwrap();
        assert_eq!(natives.unique_symbol(module, "F:1").unwrap(), Some(symbol));
        assert!(
            natives.definitions.is_empty(),
            "reference-only symbols own the same lookup"
        );
        drop(natives);
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn defensive_candidates_keep_lowest_id_and_ambiguity_across_kinds_and_order() {
        let module = id(3);
        let first = ProviderSymbol {
            provider: id(1),
            context: id(2),
            module,
            native_key: "F:1".into(),
            name: "function".into(),
            kind: SymbolKind::Function,
        };
        // A foreign context gives a distinct genuine identity; kind never filters candidates.
        let second = ProviderSymbol {
            context: id(4),
            kind: SymbolKind::Class,
            ..first.clone()
        };
        assert_ne!(first.id(), second.id());
        for order in [[&first, &second], [&second, &first]] {
            let budget = ResourceBudget::fixed(1 << 20).unwrap();
            let mut natives = Natives::new(id(1), id(2), std::iter::empty(), &budget).unwrap();
            // Private insertion tests the defensive index policy; public interning still rejects
            // a changed name/kind under the same provider/context/module/native-key identity.
            natives.retain_symbol(order[0].clone()).unwrap();
            assert_eq!(
                natives.unique_symbol(module, "F:1").unwrap(),
                Some(order[0].id())
            );
            natives.retain_symbol(order[1].clone()).unwrap();
            let before = budget.reserved();
            natives.retain_symbol(order[1].clone()).unwrap();
            assert_eq!(budget.reserved(), before);
            assert_eq!(
                natives.first_symbol(module, "F:1"),
                Some(first.id().min(second.id()))
            );
            assert!(
                matches!(natives.unique_symbol(module, "F:1"), Err(ModelError::Invalid(message))
                if message == "capture native function key is ambiguous")
            );
            assert!(natives.definitions.is_empty());
            drop(natives);
            assert_eq!(budget.reserved(), 0);
        }
    }
    #[test]
    fn native_key_admission_refusal_keeps_both_inventories_unchanged() {
        let budget = ResourceBudget::fixed(4096).unwrap();
        let mut natives = Natives::new(id(1), id(2), std::iter::empty(), &budget).unwrap();
        let module = id(3);
        let original = natives
            .symbol(
                module,
                "F:1".into(),
                "function".into(),
                SymbolKind::Function,
            )
            .unwrap();
        let before = budget.reserved();
        let refused = "x".repeat(8192);
        assert!(matches!(
            natives.symbol(
                module,
                refused.clone(),
                "refused".into(),
                SymbolKind::Function
            ),
            Err(ModelError::Resource { .. })
        ));
        assert_eq!(budget.reserved(), before);
        assert_eq!(natives.symbols().len(), 1);
        assert_eq!(natives.first_symbol(module, &refused), None);
        assert_eq!(
            natives.unique_symbol(module, "F:1").unwrap(),
            Some(original)
        );
        drop(natives);
        assert_eq!(budget.reserved(), 0);
    }
}
