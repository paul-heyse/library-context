//! Pure, versioned embedding identity shared by compiler, cache and Python boundaries.
use crate::domain::ContentHash;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MatryoshkaAdmission {
    pub is_matryoshka: bool,
    pub matryoshka_dimensions: Vec<u32>,
    pub use_activation: bool,
}

/// The embedding spec (DESIGN §11.1): everything that can change a vector. Its SHA-256 over the
/// canonical JSON (fields in this order, no whitespace) is the spec hash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    pub format: u32,
    pub source_dimensions: u32,
    pub reduction: String,
    pub admission: Option<MatryoshkaAdmission>,
    pub model: String,
    pub revision: String,
    pub tokenizer_revision: String,
    /// The serving engine and version.
    pub server: String,
    pub served_dtype: String,
    pub pooling: String,
    pub query_template: String,
    pub query_task: String,
    pub document_template: String,
    pub dimensions: u32,
    pub output_dtype: String,
    pub normalization: String,
    /// The §11.1 cap on a document, in the model's tokens.
    pub max_document_tokens: u32,
}

impl crate::domain::HeapSize for Spec {
    fn heap_bytes(&self) -> usize {
        [
            &self.reduction,
            &self.model,
            &self.revision,
            &self.tokenizer_revision,
            &self.server,
            &self.served_dtype,
            &self.pooling,
            &self.query_template,
            &self.query_task,
            &self.document_template,
            &self.output_dtype,
            &self.normalization,
        ]
        .iter()
        .map(|s| s.capacity())
        .sum::<usize>()
            + self
                .admission
                .as_ref()
                .map_or(0, |a| a.matryoshka_dimensions.capacity() * size_of::<u32>())
    }
}

impl Spec {
    pub fn parse(text: &str) -> Result<Self, String> {
        let spec: Self = serde_json::from_str(text).map_err(|e| e.to_string())?;
        spec.validate()?;
        Ok(spec)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.format != 2
            || self.dimensions == 0
            || self.dimensions > 65536
            || self.source_dimensions < self.dimensions
            || self.output_dtype != "float32"
            || self.normalization != "l2"
            || self.max_document_tokens == 0
            || self.model.is_empty()
            || self.revision.is_empty()
            || self.tokenizer_revision.is_empty()
        {
            return Err("incompatible embedding specification".to_owned());
        }
        match self.reduction.as_str() {
            "none" if self.admission.is_none() && self.source_dimensions == self.dimensions => {}
            "mrl-prefix"
                if self.admission.as_ref().is_some_and(|a| {
                    a.is_matryoshka
                        && a.matryoshka_dimensions == [self.dimensions]
                        && a.use_activation
                }) => {}
            _ => return Err("incompatible embedding reduction/admission".to_owned()),
        }
        Ok(())
    }

    pub fn canonical_json(&self) -> String {
        serde_json::to_string(self).expect("a spec always serializes")
    }

    /// SHA-256 of the canonical JSON.
    pub fn hash(&self) -> ContentHash {
        ContentHash(Sha256::digest(self.canonical_json().as_bytes()).into())
    }

    /// The request text of a document (§11.1: documents take no prefix).
    pub fn document_text(&self, text: &str) -> String {
        self.document_template.replace("{text}", text)
    }

    /// The request text of a query: `Instruct: {task}\nQuery:{query}`, with no space after
    /// `Query:` (E1).
    pub fn query_text(&self, query: &str) -> String {
        self.query_template
            .replace("{task_description}", &self.query_task)
            .replace("{query}", query)
    }
}

/// §11.1's rejections of a returned vector: its length, finiteness and unit norm.
pub fn check_vector(v: &[f32], dimensions: u32) -> Result<(), String> {
    if v.len() != dimensions as usize {
        return Err(format!("{} dimensions, not {dimensions}", v.len()));
    }
    if v.iter().any(|x| !x.is_finite()) {
        return Err("a non-finite component".to_owned());
    }
    let norm: f64 = v
        .iter()
        .map(|x| f64::from(*x) * f64::from(*x))
        .sum::<f64>()
        .sqrt();
    if (norm - 1.0).abs() > 1e-3 {
        return Err(format!("norm {norm}, not 1"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    const LIVE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../specs/embedding/qwen3-embedding-8b.json"
    ));
    #[test]
    fn canonical_spec_is_order_independent_and_rejects_incompatible_admission() {
        let spec = Spec::parse(LIVE).unwrap();
        assert_eq!(spec.dimensions, 1024);
        assert_eq!(spec.source_dimensions, 4096);
        assert_eq!(spec.canonical_json() + "\n", LIVE);
        let value: serde_json::Value = serde_json::from_str(LIVE).unwrap();
        let mut reversed = serde_json::Map::new();
        for (key, value) in value.as_object().unwrap().iter().rev() {
            reversed.insert(key.clone(), value.clone());
        }
        assert_eq!(
            Spec::parse(&serde_json::to_string(&reversed).unwrap()).unwrap(),
            spec
        );
        for (key, value) in [
            ("format", serde_json::json!(1)),
            ("dimensions", serde_json::json!(4096)),
            ("admission", serde_json::Value::Null),
            ("reduction", serde_json::json!("unknown")),
        ] {
            let mut invalid: serde_json::Value = serde_json::from_str(LIVE).unwrap();
            invalid[key] = value;
            assert!(Spec::parse(&invalid.to_string()).is_err(), "{key}");
        }
        let mut missing: serde_json::Value = serde_json::from_str(LIVE).unwrap();
        missing.as_object_mut().unwrap().remove("format");
        assert!(Spec::parse(&missing.to_string()).is_err());
        let mut extra: serde_json::Value = serde_json::from_str(LIVE).unwrap();
        extra["unexpected"] = true.into();
        assert!(Spec::parse(&extra.to_string()).is_err());
    }
}
