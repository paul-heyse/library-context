//! Explicit executable validation provenance, separate from semantic model compatibility.
//! A declaration enables exact premise identity; it never asserts a validation succeeded.
use super::{ContentHash,Key,KeySink,ModelError};
use serde::{Serialize,Deserialize};
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FactoryDependency {
    pub name:String,
    pub identity:ContentHash,
}
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationFactoryContract {
    pub version:u32,
    pub definition:ContentHash,
    pub implementation:ContentHash,
    /// Explicit ordered input contracts, not a set of relation names.
    pub inputs:Vec<ContentHash>,
    /// Additional immutable captures, configuration or vocabulary used by the factory.
    /// Environment/process state outside this declaration makes durable reuse ineligible.
    pub dependencies:Vec<FactoryDependency>,
}
impl ValidationFactoryContract {
    pub fn validate(&self)->Result<(),ModelError>{
        if self.version!=1 || self.dependencies.iter().any(|dependency|dependency.name.is_empty())
            || self.dependencies.windows(2).any(|pair|pair[0].name>=pair[1].name) {
            return Err(ModelError::Conflict("validation factory dependency declaration"));
        }
        Ok(())
    }
    pub fn identity(&self)->Result<ContentHash,ModelError>{
        self.validate()?;let mut sink=KeySink::new("validation-factory-contract/v1");
        self.definition.encode(&mut sink);self.implementation.encode(&mut sink);
        sink.part(b"input-count",&(self.inputs.len() as u64).to_le_bytes());
        for input in &self.inputs {input.encode(&mut sink);}
        sink.part(b"dependency-count",&(self.dependencies.len() as u64).to_le_bytes());
        for dependency in &self.dependencies {dependency.name.encode(&mut sink);dependency.identity.encode(&mut sink);}
        Ok(sink.finish())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn factory_identity_binds_implementation_order_multiplicity_and_captured_dependencies(){
        let base=ValidationFactoryContract{version:1,definition:ContentHash::of(b"semantic definition"),implementation:ContentHash::of(b"executable implementation"),
            inputs:vec![ContentHash::of(b"whole exact input"),ContentHash::of(b"ordered prefix input")],
            dependencies:vec![FactoryDependency{name:"vocabulary".into(),identity:ContentHash::of(b"pinned vocabulary")} ]};
        let identity=base.identity().unwrap();
        for field in ["implementation","input_order","input_multiplicity","capture"] {
            let mut changed=base.clone();match field {
                "implementation"=>changed.implementation=ContentHash::of(b"replacement executable"),
                "input_order"=>changed.inputs.reverse(),"input_multiplicity"=>changed.inputs.push(base.inputs[0]),
                _=>changed.dependencies[0].identity=ContentHash::of(b"replacement vocabulary"),
            }
            assert_ne!(identity,changed.identity().unwrap(),"factory provenance binds {field}");
            assert_eq!(base.definition,changed.definition,"executable provenance does not rename semantic definition");
        }
        let mut duplicate=base.clone();duplicate.dependencies.push(duplicate.dependencies[0].clone());assert!(duplicate.identity().is_err());
    }
}
