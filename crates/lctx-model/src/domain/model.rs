use std::{any::TypeId, collections::HashSet};
use arrow_schema::SchemaRef;
use super::{ContentHash, Field, KeySink, ModelError, Record};

#[derive(Debug, Clone)]
pub struct Relation {
    type_id: TypeId,
    name: &'static str,
    fields: Vec<Field>,
    sum: Option<super::Sum>,
    schema: SchemaRef,
    contract: &'static str,
    owner: &'static str,
    semantic_source: &'static [u8],
    validate: fn(&arrow_array::RecordBatch) -> Result<arrow_array::RecordBatch, ModelError>,
}
impl Relation {
    pub fn of<R: Record>() -> Self { Self { type_id: TypeId::of::<R>(), name: R::NAME, fields: R::fields(), sum: R::sum(), schema: R::schema(), contract: R::CONTRACT, owner: R::OWNER, semantic_source: R::SEMANTIC_SOURCE, validate: canonical::<R> } }
    pub fn name(&self) -> &'static str { self.name }
    pub fn sum(&self) -> Option<&super::Sum> { self.sum.as_ref() }
    pub fn fields(&self) -> &[Field] { &self.fields }
    pub fn schema(&self) -> &SchemaRef { &self.schema }
    pub fn canonical(&self, batch: &arrow_array::RecordBatch) -> Result<arrow_array::RecordBatch, ModelError> { (self.validate)(batch) }
    pub fn type_id(&self) -> TypeId { self.type_id }
}

/// The only model accepted by storage or execution. Validation supports reference cycles.
#[derive(Debug)]
pub struct ValidatedModel { relations: Vec<Relation>, digest: ContentHash }
impl ValidatedModel {
    pub fn validate(mut relations: Vec<Relation>) -> Result<Self, ModelError> {
        if relations.is_empty() { return Err(ModelError::Invalid("empty model".into())); }
        relations.sort_by_key(Relation::name);
        let mut types = HashSet::new();
        let mut names = HashSet::new();
        for relation in &relations {
            if !identifier(relation.name) || !names.insert(relation.name) || !types.insert(relation.type_id) {
                return Err(ModelError::Invalid(format!("invalid or duplicate relation {}", relation.name)));
            }
        }
        let mut digest = KeySink::new("model");
        digest.part(b"owned-semantics", &ContentHash::of(include_bytes!(concat!(env!("OUT_DIR"), "/semantic-contract.bin"))).0);
        for relation in &relations {
            if relation.owner != "lctx-model" && relation.semantic_source.is_empty() {
                return Err(ModelError::Invalid(format!("{} is outside the captured semantic owner; declare semantic_source", relation.name)));
            }
            digest.part(b"semantic-source", relation.semantic_source);
            digest.part(b"declaration", relation.contract.as_bytes());
            digest.part(b"relation", relation.name.as_bytes());
            let mut fields = HashSet::from(["id", "generation_id"]);
            let mut has_key = false;
            for field in &relation.fields {
                if !identifier(field.name()) || field.name().starts_with("__") || !fields.insert(field.name()) {
                    return Err(ModelError::Invalid(format!("invalid field {}.{}", relation.name, field.name())));
                }
                has_key |= field.is_key();
                if let Some((target, name)) = field.target() {
                    if !types.contains(&target) { return Err(ModelError::Invalid(format!("{}.{} targets absent {}", relation.name, field.name(), name))); }
                    if let Some(code) = field.subtype() {
                        let target = relations.iter().find(|r| r.type_id == target).expect("checked member");
                        if !target.sum().is_some_and(|sum| sum.arms.iter().any(|arm| arm.code == code)) {
                            return Err(ModelError::Invalid("reference to missing sum subtype".into()));
                        }
                        digest.part(b"subtype", &code.to_le_bytes());
                    }
                    digest.part(b"reference", name.as_bytes());
                }
                for (code, label) in field.codes() {
                    digest.part(b"code", &code.to_le_bytes());
                    digest.part(b"label", label.as_bytes());
                }
                digest.part(b"field", field.name().as_bytes());
                digest.part(b"type", format!("{:?}", field.arrow().data_type()).as_bytes());
                digest.part(b"roles", &[u8::from(field.is_key()), u8::from(field.is_provenance()), u8::from(field.nullable())]);
            }
            if let Some(sum) = relation.sum() {
                let tag = relation.fields.iter().find(|field| field.name() == sum.tag)
                    .ok_or_else(|| ModelError::Invalid("missing sum discriminant".into()))?;
                if tag.scalar() != super::Scalar::Int16 || tag.nullable() || sum.arms.is_empty() {
                    return Err(ModelError::Invalid("invalid sum discriminant".into()));
                }
                let mut codes = HashSet::new();
                for arm in &sum.arms {
                    if !codes.insert(arm.code) { return Err(ModelError::Invalid("duplicate sum arm".into())); }
                    let mut active = HashSet::new();
                    for field in &arm.fields {
                        if field.name == sum.tag || !fields.contains(field.name) || !active.insert(field.name) {
                            return Err(ModelError::Invalid("invalid sum payload field".into()));
                        }
                    }
                }
                digest.part(b"sum", format!("{sum:?}").as_bytes());
            }
            if !has_key { return Err(ModelError::Invalid(format!("{} has no key", relation.name))); }
        }
        Ok(Self { relations, digest: digest.finish() })
    }
    pub fn relations(&self) -> &[Relation] { &self.relations }
    pub fn digest(&self) -> ContentHash { self.digest }
    pub fn require<R: Record>(&self) -> Result<&Relation, ModelError> {
        self.relations.iter().find(|r| r.type_id == TypeId::of::<R>())
            .ok_or_else(|| ModelError::Invalid(format!("{} is not in this model", R::NAME)))
    }
}
fn identifier(name: &str) -> bool {
    !name.is_empty() && name.len() <= 48 && name.bytes().enumerate().all(|(i, c)| c == b'_' || c.is_ascii_lowercase() || (i > 0 && c.is_ascii_digit()))
}

fn canonical<R: Record>(batch: &arrow_array::RecordBatch) -> Result<arrow_array::RecordBatch, ModelError> {
    let mut rows = R::decode(batch)?;
    rows.sort_by_key(Record::id);
    for pair in rows.windows(2) {
        if pair[0].id() == pair[1].id() { return Err(ModelError::Conflict(R::NAME)); }
    }
    R::encode(&rows)
}
