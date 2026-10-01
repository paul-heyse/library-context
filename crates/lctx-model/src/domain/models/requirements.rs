//! Dependency capture requests derived from authored models, before either native provider runs.
use super::*;
use crate::domain::{HeapSize, charged::{ChargedSet, StateCharge}, execution::ExactRuntimeException,
    resources::ResourceBudget};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RequirementPin {
    Python(String),
    Distribution { name: String, version: String },
    Release,
    /// A class mentioned by a rule must resolve within the frozen context. Its captured
    /// distribution and version remain evidence, never an inferred pin from its spelling.
    CapturedEnvironment,
}
impl HeapSize for RequirementPin {
    fn heap_bytes(&self) -> usize {
        match self {
            Self::Python(v) => v.heap_bytes(),
            Self::Distribution { name, version } => name.heap_bytes() + version.heap_bytes(),
            Self::Release | Self::CapturedEnvironment => 0,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RequiredBy {
    Model(Id<AuthoredModel>),
    Protocol(Id<AuthoredContextProtocol>),
    RuntimeException(ExactRuntimeException),
}
impl HeapSize for RequiredBy {}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RequiredDefinition {
    pub owner: RequiredBy,
    pub pin: RequirementPin,
    pub module: String,
    pub qualified_name: String,
    /// Retain complete ancestry when this definition is a class. A containing scope request
    /// does not itself assert that the provider will report a class rather than a function.
    pub require_mro: bool,
}
impl HeapSize for RequiredDefinition {
    fn heap_bytes(&self) -> usize {
        self.pin.heap_bytes() + self.module.heap_bytes() + self.qualified_name.heap_bytes()
    }
}
/// An exact expected domain. Failed/mismatched resolutions remain required entries; consumers
/// cannot derive this domain from the definitions that happened to be observed.
pub struct ModelContextRequirements {
    pub catalog: Id<ModelCatalog>,
    pub catalog_digest: ContentHash,
    entries: ChargedSet<RequiredDefinition>,
    _charge: StateCharge,
}
impl ModelContextRequirements {
    pub fn derive(catalog: &Catalog, python: &str, budget: &ResourceBudget) -> Result<Self, ModelError> {
        let mut entries = ChargedSet::default();
        let mut charge = StateCharge::new(budget, "model-context-requirements");
        let mut add = |entry| entries.insert(&mut charge, entry).map(|_| ());
        for model in &catalog.models {
            let owner = RequiredBy::Model(model.declaration.id());
            add_target(&model.model.target, owner, false, &mut add)?;
            for rule in &model.model.rules {
                for class in rule.context_classes() {
                    add_class(class, owner, python, &mut add)?;
                }
            }
        }
        for protocol in &catalog.context_protocols {
            let owner = RequiredBy::Protocol(protocol.declaration.id());
            add_target(&protocol.model.target, owner, true, &mut add)?;
            add_target(&protocol.model.allocation, owner, false, &mut add)?;
            add_target(&protocol.model.initialization, owner, false, &mut add)?;
            // Protocol members are class members of the same pinned target.
            for suffix in ["__enter__", "__exit__"] {
                let mut entry = target_requirement(&protocol.model.target, owner, false);
                entry.qualified_name.push('.');
                entry.qualified_name.push_str(suffix);
                add(entry)?;
            }
        }
        for exception in ExactRuntimeException::ALL {
            let (module, name) = exception.class();
            add(RequiredDefinition {
                owner: RequiredBy::RuntimeException(*exception),
                pin: RequirementPin::Python(python.into()), module: module.into(),
                qualified_name: name.into(), require_mro: true,
            })?;
        }
        Ok(Self { catalog: catalog.declaration.id(), catalog_digest: catalog.digest(),
            entries, _charge: charge })
    }
    pub fn entries(&self) -> impl Iterator<Item=&RequiredDefinition> { self.entries.iter() }
}
fn target_requirement(target: &Target, owner: RequiredBy, require_mro: bool) -> RequiredDefinition {
    let (pin, module, name) = match target {
        Target::Stdlib { python, module, callable } => (RequirementPin::Python(python.clone()), module, callable),
        Target::Dependency { distribution, version, module, callable } => (
            RequirementPin::Distribution { name: distribution.clone(), version: version.clone() }, module, callable),
        Target::Release { module, callable } => (RequirementPin::Release, module, callable),
    };
    RequiredDefinition { owner, pin, module: module.clone(), qualified_name: name.clone(), require_mro }
}
fn add_target(target: &Target, owner: RequiredBy, mro: bool,
    add: &mut impl FnMut(RequiredDefinition)->Result<(),ModelError>) -> Result<(),ModelError> {
    let entry = target_requirement(target, owner, mro);
    if let Some((class, _)) = entry.qualified_name.rsplit_once('.') {
        add(RequiredDefinition { qualified_name: class.into(), require_mro: true, ..entry.clone() })?;
    }
    add(entry)
}
fn add_class(class: &str, owner: RequiredBy, python: &str,
    add: &mut impl FnMut(RequiredDefinition)->Result<(),ModelError>) -> Result<(),ModelError> {
    let (module, name) = class.rsplit_once('.').ok_or_else(|| ModelError::Invalid("model class has no module".into()))?;
    add(RequiredDefinition {
        owner, pin: if module == "builtins" { RequirementPin::Python(python.into()) } else { RequirementPin::CapturedEnvironment },
        module: module.into(), qualified_name: name.into(), require_mro: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn requests_include_pins_protocol_members_and_exact_exception_hierarchies() {
        let catalog = Catalog::committed().unwrap();
        let budget = ResourceBudget::fixed(1024 * 1024).unwrap();
        let required = ModelContextRequirements::derive(&catalog, "3.14.7", &budget).unwrap();
        let entries = required.entries().collect::<Vec<_>>();
        for (module, name, mro) in [("builtins","TypeError",true), ("builtins","OSError",true),
            ("contextlib","nullcontext.__enter__",false), ("contextlib","suppress.__exit__",false),
            ("builtins","object.__new__",false), ("logging","Logger",true)] {
            assert!(entries.iter().any(|e| e.module == module && e.qualified_name == name && e.require_mro == mro));
        }
        assert!(entries.iter().any(|e| e.module == "pydantic.type_adapter" &&
            matches!(&e.pin, RequirementPin::Distribution { name, version } if name == "pydantic" && version == "2.13.5")));
        assert!(budget.reserved() > 0);
        drop(entries);
        drop(required);
        assert_eq!(budget.reserved(), 0);
        assert!(ModelContextRequirements::derive(&catalog, "3.14.7", &ResourceBudget::fixed(1).unwrap()).is_err());
    }
    #[test]
    fn changing_catalog_version_changes_the_expected_domain_without_dropping_targets() {
        let source = include_str!("../../../models/external.toml");
        let before = Catalog::parse("external.toml", source).unwrap();
        let after = Catalog::parse("external.toml", &source.replace("2.13.5", "2.13.6")).unwrap();
        let budget = ResourceBudget::fixed(1024 * 1024).unwrap();
        let before = ModelContextRequirements::derive(&before, "3.14.7", &budget).unwrap();
        let after = ModelContextRequirements::derive(&after, "3.14.7", &budget).unwrap();
        assert_ne!(before.catalog, after.catalog);
        assert_ne!(before.catalog_digest, after.catalog_digest);
        assert_eq!(before.entries().count(), after.entries().count());
        assert!(after.entries().any(|e| matches!(&e.pin, RequirementPin::Distribution { version, .. } if version == "2.13.6")));
    }
}
