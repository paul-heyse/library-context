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

pub const QUERY_TEMPLATE: &str = "Instruct: {task_description}\nQuery:{query}";
pub const QUERY_TASK: &str = "Given a coding task, retrieve relevant Python library APIs, capability briefs, configuration options, source code, usage examples, and documentation.";

/// A launch/input envelope. Encoder, document and query recipes have separate identities.
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
    pub max_query_tokens: u32,
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
        if self.format != 3
            || self.dimensions == 0
            || self.dimensions > 65536
            || self.source_dimensions != self.dimensions
            || self.output_dtype != "float32"
            || self.normalization != "l2"
            || self.max_document_tokens == 0
            || self.max_query_tokens == 0
            || self.model.is_empty()
            || self.revision.is_empty()
            || self.tokenizer_revision.is_empty()
        {
            return Err("incompatible embedding specification".to_owned());
        }
        match self.reduction.as_str() {
            "none" if self.admission.is_none() || self.admission.as_ref().is_some_and(|a| {
                    a.is_matryoshka
                        && a.matryoshka_dimensions == [self.dimensions]
                        && a.use_activation
                }) => {}
            _ => return Err("incompatible embedding reduction/admission".to_owned()),
        }
        self.query_recipe().validate()?;
        if self.document_template.matches("{text}").count()!=1 {return Err("invalid document template".into());}
        Ok(())
    }

    pub fn canonical_json(&self) -> String {
        serde_json::to_string(self).expect("a spec always serializes")
    }

    /// Actual encoder identity. Query/render/projection policy cannot invalidate full winners.
    pub fn hash(&self) -> ContentHash {
        recipe_hash("encoder/v3", &EncoderRecipe {
            source_dimensions: self.source_dimensions,
            admission: &self.admission,
            model: &self.model,
            revision: &self.revision,
            tokenizer_revision: &self.tokenizer_revision,
            server: &self.server,
            served_dtype: &self.served_dtype,
            pooling: &self.pooling,
            dimensions: self.dimensions,
            output_dtype: &self.output_dtype,
            normalization: &self.normalization,
        })
    }
    pub fn document_hash(&self) -> ContentHash {
        recipe_hash("document/v3", &DocumentRecipe {
            template: &self.document_template,
            max_tokens: self.max_document_tokens,
        })
    }
    pub fn query_recipe(&self)->QueryRecipe {
        QueryRecipe {template:self.query_template.clone(),task:self.query_task.clone(),max_tokens:self.max_query_tokens}
    }
    pub fn query_hash(&self)->ContentHash {self.query_recipe().identity()}
    pub fn configuration_hash(&self) -> ContentHash {
        recipe_hash("embedding-envelope/v3", self)
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

#[derive(Serialize)]
struct EncoderRecipe<'a> {
    source_dimensions: u32,
    admission: &'a Option<MatryoshkaAdmission>,
    model: &'a str,
    revision: &'a str,
    tokenizer_revision: &'a str,
    server: &'a str,
    served_dtype: &'a str,
    pooling: &'a str,
    dimensions: u32,
    output_dtype: &'a str,
    normalization: &'a str,
}
#[derive(Serialize)]
struct DocumentRecipe<'a> { template: &'a str, max_tokens: u32 }
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize,schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QueryRecipe {pub template:String,pub task:String,pub max_tokens:u32}
impl QueryRecipe {
    pub fn identity(&self)->ContentHash {recipe_hash("query/v3",self)}
    pub fn text(&self,query:&str)->String {self.template.replace("{task_description}",&self.task).replace("{query}",query)}
    pub fn validate(&self)->Result<(),String> {
        if self.max_tokens==0 || self.template.matches("{query}").count()!=1 || self.template.matches("{task_description}").count()!=1 || self.task.trim().is_empty() {return Err("invalid query recipe".into());} Ok(())
    }
}
pub(crate) fn recipe_hash<T: Serialize>(kind: &str, recipe: &T) -> ContentHash {
    let mut digest = Sha256::new();
    digest.update(kind.as_bytes());
    digest.update([0]);
    digest.update(serde_json::to_vec(recipe).expect("typed recipe serializes"));
    ContentHash(digest.finalize().into())
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
        assert_eq!(spec.dimensions, 4096);
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
            ("format", serde_json::json!(2)),
            ("dimensions", serde_json::json!(1024)),
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
    #[test]
    fn query_document_and_encoder_changes_have_independent_identities() {
        let spec = Spec::parse(LIVE).unwrap();
        let mut query = spec.clone(); query.query_task.push_str(" changed");
        assert_eq!(query.hash(),spec.hash());
        assert_eq!(query.document_hash(),spec.document_hash());
        assert_ne!(query.query_hash(),spec.query_hash());
        let mut document = spec.clone(); document.document_template = "Header\n{text}".into();
        assert_eq!(document.hash(),spec.hash()); assert_ne!(document.document_hash(),spec.document_hash());
        let mut encoder=spec.clone(); encoder.revision.push_str(" changed");
        assert_ne!(encoder.hash(),spec.hash());
    }
}
