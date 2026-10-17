use super::{ContentHash, Field, KeySink, ModelError, Record};
use arrow_schema::SchemaRef;
use std::{any::TypeId, collections::HashSet, sync::Arc};

/// Typed nominal identity and complete content in the supplied row order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowFingerprint {
    pub nominal: [u8; 16],
    pub content: ContentHash,
}

/// Immutable executable declaration metadata is shared by physical bindings and compiled scopes.
/// Cloning a relation acquires another owner; it never copies field/arm/proof inventories.
#[derive(Debug, Clone)]
pub struct Relation {
    type_id: TypeId,
    name: &'static str,
    fields: Arc<[Field]>,
    invariant_refs: Arc<[&'static str]>,
    publication_refs: Arc<[&'static str]>,
    required: Arc<[(TypeId, &'static str)]>,
    sum: Option<Arc<super::Sum>>,
    derivation: Option<Arc<super::derivation::Derivation>>,
    schema: SchemaRef,
    family: Option<super::attribution::FactFamily>,
    projection_roles: Arc<[super::projection::EndpointRole]>,
    validate: fn(&arrow_array::RecordBatch) -> Result<arrow_array::RecordBatch, ModelError>,
    proofs: fn(&arrow_array::RecordBatch) -> Result<Vec<super::derivation::Proof>, ModelError>,
    hash_rows: fn(&arrow_array::RecordBatch, &mut RelationContent) -> Result<(), ModelError>,
    row_fingerprints: fn(&arrow_array::RecordBatch) -> Result<Vec<RowFingerprint>, ModelError>,
    ordered_canonical:
        fn(&arrow_array::RecordBatch) -> Result<arrow_array::RecordBatch, ModelError>,
}
impl Relation {
    pub fn of<R: Record>() -> Self {
        Self {
            type_id: TypeId::of::<R>(),
            name: R::NAME,
            fields: R::fields().into(),
            invariant_refs: R::invariant_refs().into(),
            publication_refs: R::publication_refs().into(),
            required: R::required_relations().into(),
            sum: R::sum().map(Arc::new),
            derivation: R::derivation().map(Arc::new),
            schema: R::schema(),
            family: R::family(),
            projection_roles: R::projection_roles().into(),
            validate: canonical::<R>,
            hash_rows: hash_rows::<R>,
            row_fingerprints: row_fingerprints::<R>,
            ordered_canonical: ordered_canonical::<R>,
            proofs: proofs::<R>,
        }
    }
    pub fn derivation(&self) -> Option<&super::derivation::Derivation> {
        self.derivation.as_deref()
    }
    pub fn name(&self) -> &'static str {
        self.name
    }
    pub fn family(&self) -> Option<super::attribution::FactFamily> {
        self.family
    }
    pub fn projection_roles(&self) -> &[super::projection::EndpointRole] {
        &self.projection_roles
    }
    pub fn invariant_refs(&self) -> &[&'static str] {
        &self.invariant_refs
    }
    pub fn publication_refs(&self) -> &[&'static str] {
        &self.publication_refs
    }
    pub fn resolved_invariants<'m>(
        &self,
        model: &'m ValidatedModel,
    ) -> Result<Vec<&'m Invariant>, ModelError> {
        self.invariant_refs
            .iter()
            .map(|id| model.invariant(id))
            .collect()
    }
    pub fn resolved_publications<'m>(
        &self,
        model: &'m ValidatedModel,
    ) -> Result<Vec<&'m PublicationInvariant>, ModelError> {
        self.publication_refs
            .iter()
            .map(|id| model.publication_check(id))
            .collect()
    }
    pub fn sum(&self) -> Option<&super::Sum> {
        self.sum.as_deref()
    }
    pub fn fields(&self) -> &[Field] {
        &self.fields
    }
    pub fn schema(&self) -> &SchemaRef {
        &self.schema
    }
    pub fn canonical(
        &self,
        batch: &arrow_array::RecordBatch,
    ) -> Result<arrow_array::RecordBatch, ModelError> {
        (self.validate)(batch)
    }
    pub fn proofs(
        &self,
        batch: &arrow_array::RecordBatch,
    ) -> Result<Vec<super::derivation::Proof>, ModelError> {
        (self.proofs)(batch)
    }
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }
    pub fn content(&self) -> RelationContent {
        RelationContent {
            relation: self.name,
            sink: KeySink::new(self.name),
            previous: None,
            count: 0,
        }
    }
    /// Validate and reencode complete typed rows while preserving their supplied order.
    /// `canonical` separately sorts IDs for canonical relation storage and is not suitable
    /// for enforcing a validator's ordered-input contract.
    pub fn ordered_canonical(
        &self,
        batch: &arrow_array::RecordBatch,
    ) -> Result<arrow_array::RecordBatch, ModelError> {
        (self.ordered_canonical)(batch)
    }
    /// Full typed nominal/content fingerprints without imposing a second input order.
    pub fn row_fingerprints(
        &self,
        batch: &arrow_array::RecordBatch,
    ) -> Result<Vec<RowFingerprint>, ModelError> {
        (self.row_fingerprints)(batch)
    }
    pub fn hash_rows(
        &self,
        batch: &arrow_array::RecordBatch,
        content: &mut RelationContent,
    ) -> Result<(), ModelError> {
        if content.relation != self.name {
            return Err(ModelError::Schema(self.name));
        }
        (self.hash_rows)(batch, content)
    }
}

/// Semantic validation identity is distinct from domain refusal/coverage obligation kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValidationKind {
    Invariant,
    Publication,
}
impl ValidationKind {
    pub const fn code(self) -> u8 {
        match self {
            Self::Invariant => 0,
            Self::Publication => 1,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ValidationIdentity {
    pub kind: ValidationKind,
    pub id: &'static str,
    pub revision: u32,
}
impl ValidationIdentity {
    pub fn encode(self, sink: &mut KeySink) {
        sink.part(b"validation-kind", &[self.kind.code()]);
        sink.part(b"validation-id", self.id.as_bytes());
        sink.part(b"validation-revision", &self.revision.to_le_bytes());
    }
    pub fn digest(self) -> ContentHash {
        let mut sink = KeySink::new("validation-identity/v1");
        self.encode(&mut sink);
        sink.finish()
    }
}
/// Definitions are supplied once by the semantic owner; relation/stage declarations only refer.
#[derive(Debug, Clone, Default)]
pub struct ValidationDefinitions {
    pub invariants: Vec<Invariant>,
    pub publication_checks: Vec<PublicationInvariant>,
}
impl ValidationDefinitions {
    fn contains(&self, kind: ValidationKind, id: &str) -> bool {
        match kind {
            ValidationKind::Invariant => self.invariants.iter().any(|i| i.name == id),
            ValidationKind::Publication => self.publication_checks.iter().any(|i| i.name == id),
        }
    }
    fn check_unique(&self) -> Result<(), ModelError> {
        let mut seen = HashSet::new();
        for identity in self.invariants.iter().map(Invariant::identity).chain(
            self.publication_checks
                .iter()
                .map(PublicationInvariant::identity),
        ) {
            if !identifier(identity.id)
                || identity.revision == 0
                || (identity.kind == ValidationKind::Invariant
                    && identity.id == "derivation_acyclic")
                || !seen.insert((identity.kind.code(), identity.id))
            {
                return Err(ModelError::Invalid(format!(
                    "invalid or conflicting validation definition {}",
                    identity.id
                )));
            }
        }
        Ok(())
    }
    /// Select declared definitions by explicit IDs, never by whether their premises happen to fit.
    pub fn required_for(&self, relations: &[Relation]) -> Result<Self, ModelError> {
        self.check_unique()?;
        let mut invariant_ids = HashSet::new();
        let mut publication_ids = HashSet::new();
        for relation in relations {
            for id in relation.invariant_refs() {
                if !self.contains(ValidationKind::Invariant, id) {
                    return Err(ModelError::Invalid(format!(
                        "unresolved validation definition {id}"
                    )));
                }
                invariant_ids.insert(*id);
            }
            for id in relation.publication_refs() {
                if !self.contains(ValidationKind::Publication, id) {
                    return Err(ModelError::Invalid(format!(
                        "unresolved publication definition {id}"
                    )));
                }
                publication_ids.insert(*id);
            }
        }
        Ok(Self {
            invariants: self
                .invariants
                .iter()
                .filter(|i| invariant_ids.contains(i.name))
                .cloned()
                .collect(),
            publication_checks: self
                .publication_checks
                .iter()
                .filter(|i| publication_ids.contains(i.name))
                .cloned()
                .collect(),
        })
    }
}
fn validate_definition(
    identity: ValidationIdentity,
    inputs: &[ValidationInput],
    relations: &[Relation],
) -> Result<(), ModelError> {
    if inputs.is_empty() {
        return Err(ModelError::Invalid(format!(
            "{} has no validation premises",
            identity.id
        )));
    }
    let mut frames = HashSet::new();
    for input in inputs {
        if !frames.insert((input.type_id, input.prefix.map(|p| p.code()))) || input.order.is_empty()
        {
            return Err(ModelError::Invalid(format!(
                "{} needs distinct ordered input frames",
                identity.id
            )));
        }
        let relation = relations
            .iter()
            .find(|r| r.type_id == input.type_id && r.name == input.name)
            .ok_or_else(|| {
                ModelError::Invalid(format!(
                    "{} requires absent validation premise {}",
                    identity.id, input.name
                ))
            })?;
        input.validate_prefix()?;
        let mut order_fields = HashSet::new();
        for order in &input.order {
            if !order_fields.insert(*order)
                || (*order != "id" && !relation.fields.iter().any(|f| f.name() == *order))
            {
                return Err(ModelError::Invalid(format!(
                    "{} has invalid input order {}.{order}",
                    identity.id, relation.name
                )));
            }
        }
        if !input.order.contains(&"id")
            && !relation
                .fields
                .iter()
                .filter(|f| f.is_key())
                .all(|f| input.order.contains(&f.name()))
        {
            return Err(ModelError::Invalid(format!(
                "{} needs the complete nominal key or ID for a deterministic input order",
                identity.id
            )));
        }
    }
    Ok(())
}

/// Revise when a semantic policy changes without changing a declaration or a named invariant.
pub const SEMANTIC_POLICY_REVISION: u32 = 1;

/// Implementation provenance is deliberately independent of semantic model compatibility.
/// This may invalidate reuse after an implementation/dependency change without renaming entities.
pub fn implementation_digest() -> ContentHash {
    ContentHash::of(include_bytes!(concat!(
        env!("OUT_DIR"),
        "/model-implementation.bin"
    )))
}

/// The only model accepted by storage or execution. Validation supports reference cycles.
#[derive(Debug)]
pub struct ValidatedModel {
    relations: Vec<Relation>,
    invariants: Vec<Invariant>,
    publication_checks: Vec<PublicationInvariant>,
    digest: ContentHash,
    factory_contracts: std::collections::BTreeMap<
        ValidationIdentity,
        super::validation_factory::ValidationFactoryContract,
    >,
}
impl ValidatedModel {
    /// Resolve the explicitly referenced canonical domain definitions. No missing premise is omitted.
    pub fn declared(relations: Vec<Relation>) -> Result<Self, ModelError> {
        let definitions = super::validation::definitions().required_for(&relations)?;
        let mut model = Self::validate(relations, definitions)?;
        // This route instantiates only the model-owned canonical factory registry.
        // Custom caller-supplied closures enter through validate() and remain undeclared.
        let declarations = model
            .invariants
            .iter()
            .map(|check| check.identity())
            .chain(
                model
                    .publication_checks
                    .iter()
                    .map(|check| check.identity()),
            )
            .collect::<Vec<_>>();
        let implementation = implementation_digest();
        let vocabulary = model.digest;
        for identity in declarations {
            model.register_factory(
                identity,
                implementation,
                vec![super::validation_factory::FactoryDependency {
                    name: "model-vocabulary".into(),
                    identity: vocabulary,
                }],
            )?;
        }
        Ok(model)
    }
    pub fn validate(
        mut relations: Vec<Relation>,
        mut definitions: ValidationDefinitions,
    ) -> Result<Self, ModelError> {
        if relations.is_empty() {
            return Err(ModelError::Invalid("empty model".into()));
        }
        relations.sort_by_key(Relation::name);
        let mut types = HashSet::new();
        let mut names = HashSet::new();
        for relation in &relations {
            if super::normalized::events::CallPolicy::ALL
                .iter()
                .any(|policy| policy.view_name() == relation.name)
            {
                return Err(ModelError::Invalid(
                    "relation name reserved for generated call policy view".into(),
                ));
            }
            if !identifier(relation.name)
                || !names.insert(relation.name)
                || !types.insert(relation.type_id)
            {
                return Err(ModelError::Invalid(format!(
                    "invalid or duplicate relation {}",
                    relation.name
                )));
            }
        }
        let mut digest = KeySink::new("model/v3");
        // Declaration fields, codebooks, projection meanings and explicit invariant revisions
        // define compatibility. Source text, documentation, implementation and the lockfile do not.
        digest.part(
            b"semantic-policy-revision",
            &SEMANTIC_POLICY_REVISION.to_le_bytes(),
        );
        for relation in &relations {
            digest.part(b"relation", relation.name.as_bytes());
            let mut roles = HashSet::new();
            for role in relation.projection_roles.iter() {
                if role.relation() != relation.name || !roles.insert(*role) {
                    return Err(ModelError::Invalid(format!(
                        "invalid projection role on {}",
                        relation.name
                    )));
                }
                digest.part(b"projection-role", &(*role as i16).to_le_bytes());
            }
            for (type_id, name) in relation.required.iter() {
                if !relations
                    .iter()
                    .any(|r| r.type_id == *type_id && r.name == *name)
                {
                    return Err(ModelError::Invalid(format!(
                        "{} requires companion relation {name}",
                        relation.name
                    )));
                }
                digest.part(b"required-relation", name.as_bytes());
            }
            let mut fields = HashSet::from(["id", "generation_id"]);
            let mut has_key = false;
            for field in relation.fields.iter() {
                if !identifier(field.name())
                    || field.name().starts_with("__")
                    || !fields.insert(field.name())
                {
                    return Err(ModelError::Invalid(format!(
                        "invalid field {}.{}",
                        relation.name,
                        field.name()
                    )));
                }
                has_key |= field.is_key();
                if let Some((target, name)) = field.target() {
                    if !types.contains(&target) {
                        return Err(ModelError::Invalid(format!(
                            "{}.{} targets absent {}",
                            relation.name,
                            field.name(),
                            name
                        )));
                    }
                    if let Some(code) = field.subtype() {
                        let target = relations
                            .iter()
                            .find(|r| r.type_id == target)
                            .expect("checked member");
                        if !target
                            .sum()
                            .is_some_and(|sum| sum.arms.iter().any(|arm| arm.code == code))
                        {
                            return Err(ModelError::Invalid(
                                "reference to missing sum subtype".into(),
                            ));
                        }
                    }
                }
                field.encode_contract(&mut digest);
            }
            if let Some(proof) = relation.derivation() {
                if !identifier(proof.rule) || proof.premises.is_empty() {
                    return Err(ModelError::Invalid("invalid derivation rule".into()));
                }
                let mut roles = HashSet::new();
                for column in proof.conclusion.iter().chain(&proof.premises) {
                    let field = relation
                        .fields
                        .iter()
                        .find(|f| f.name() == column.name())
                        .ok_or_else(|| ModelError::Invalid("derivation column absent".into()))?;
                    if field.target() != Some(column.target())
                        || field.nullable() != column.nullable()
                        || field.list()
                    {
                        return Err(ModelError::Invalid(
                            "derivation target differs from nominal field".into(),
                        ));
                    }
                }
                if proof.conclusion.as_ref().is_some_and(|c| c.nullable()) {
                    return Err(ModelError::Invalid(
                        "derivation conclusion cannot be optional".into(),
                    ));
                }
                for premise in &proof.premises {
                    if !roles.insert(premise.name()) {
                        return Err(ModelError::Invalid(
                            "duplicate derivation premise role".into(),
                        ));
                    }
                    digest.part(b"premise", premise.name().as_bytes());
                    digest.part(b"premise-target", premise.target().1.as_bytes());
                }
                digest.part(b"rule", proof.rule.as_bytes());
                digest.part(
                    b"conclusion",
                    proof
                        .conclusion
                        .as_ref()
                        .map_or("id", |c| c.name())
                        .as_bytes(),
                );
            }
            if let Some(sum) = relation.sum() {
                let tag = relation
                    .fields
                    .iter()
                    .find(|field| field.name() == sum.tag)
                    .ok_or_else(|| ModelError::Invalid("missing sum discriminant".into()))?;
                if tag.scalar() != super::Scalar::Int16 || tag.nullable() || sum.arms.is_empty() {
                    return Err(ModelError::Invalid("invalid sum discriminant".into()));
                }
                let mut codes = HashSet::new();
                for arm in &sum.arms {
                    if !codes.insert(arm.code) {
                        return Err(ModelError::Invalid("duplicate sum arm".into()));
                    }
                    let mut active = HashSet::new();
                    for field in &arm.fields {
                        if field.name == sum.tag
                            || !fields.contains(field.name)
                            || !active.insert(field.name)
                        {
                            return Err(ModelError::Invalid("invalid sum payload field".into()));
                        }
                    }
                }
                sum.encode_contract(&mut digest);
            }
            if !has_key {
                return Err(ModelError::Invalid(format!("{} has no key", relation.name)));
            }
        }
        definitions.check_unique()?;
        for relation in &relations {
            for (kind, references) in [
                (ValidationKind::Invariant, relation.invariant_refs()),
                (ValidationKind::Publication, relation.publication_refs()),
            ] {
                let mut seen = HashSet::new();
                for reference in references {
                    if !seen.insert(*reference) || !definitions.contains(kind, reference) {
                        return Err(ModelError::Invalid(format!(
                            "{} has duplicate or unresolved {kind:?} reference {reference}",
                            relation.name()
                        )));
                    }
                    digest.part(b"validation-reference-relation", relation.name().as_bytes());
                    digest.part(b"validation-reference-kind", &[kind.code()]);
                    digest.part(b"validation-reference-id", reference.as_bytes());
                }
            }
        }
        definitions.invariants.sort_by_key(|check| check.name);
        definitions
            .publication_checks
            .sort_by_key(|check| check.name);
        let mut invariants = definitions.invariants;
        let publication_checks = definitions.publication_checks;
        let mut invariant_names: HashSet<_> = invariants.iter().map(|i| i.name).collect();
        for (identity, inputs) in invariants
            .iter()
            .map(|i| (i.identity(), i.inputs.as_slice()))
            .chain(
                publication_checks
                    .iter()
                    .map(|i| (i.identity(), i.inputs.as_slice())),
            )
        {
            validate_definition(identity, inputs, &relations)?;
            identity.encode(&mut digest);
            for input in inputs {
                input.encode_contract(&mut digest);
            }
        }
        if types.contains(&TypeId::of::<super::projection::ProjectionSourceAssessment>()) {
            for name in super::projection::ProjectionName::ALL {
                for role in super::projection::ProjectionSpec::builtin(name).roles() {
                    if !relations.iter().any(|r| r.projection_roles.contains(role)) {
                        return Err(ModelError::Invalid(format!(
                            "projection {name:?} lacks declared role {role:?}"
                        )));
                    }
                }
            }
        }
        let sources: Vec<_> = relations
            .iter()
            .filter(|r| r.derivation().is_some())
            .cloned()
            .collect();
        if let Some(invariant) = generated_derivation(sources) {
            if names.contains("derivations") || names.contains("derivation_premises") {
                return Err(ModelError::Invalid(
                    "relation name reserved for generated derivation view".into(),
                ));
            }
            if !invariant_names.insert(invariant.name) {
                return Err(ModelError::Invalid(
                    "reserved generated invariant name".into(),
                ));
            }
            invariant.identity().encode(&mut digest);
            for input in &invariant.inputs {
                input.encode_contract(&mut digest);
            }
            invariants.push(invariant);
        }
        invariants.sort_by_key(|v| v.name);
        Ok(Self {
            relations,
            invariants,
            publication_checks,
            digest: digest.finish(),
            factory_contracts: std::collections::BTreeMap::new(),
        })
    }
    /// Explicitly declares executable provenance and every immutable extra capture.
    /// The caller owns the declaration's truth; mutable ambient/process state makes a
    /// factory ineligible for durable premise reuse and must remain unregistered.
    pub fn register_factory(
        &mut self,
        identity: ValidationIdentity,
        implementation: ContentHash,
        dependencies: Vec<super::validation_factory::FactoryDependency>,
    ) -> Result<(), ModelError> {
        let (definition, inputs) = match identity.kind {
            ValidationKind::Invariant => {
                let check = self.invariant(identity.id)?;
                if check.identity() != identity {
                    return Err(ModelError::Conflict("factory invariant revision"));
                }
                (check.digest(), &check.inputs)
            }
            ValidationKind::Publication => {
                let check = self.publication_check(identity.id)?;
                if check.identity() != identity {
                    return Err(ModelError::Conflict("factory publication revision"));
                }
                (check.digest(), &check.inputs)
            }
        };
        let inputs = inputs
            .iter()
            .map(|input| {
                let mut sink = KeySink::new("validation-factory-input/v1");
                input.encode_contract(&mut sink);
                sink.finish()
            })
            .collect();
        let contract = super::validation_factory::ValidationFactoryContract {
            version: 1,
            definition,
            implementation,
            inputs,
            dependencies,
        };
        contract.validate()?;
        if let Some(existing) = self.factory_contracts.get(&identity) {
            if existing != &contract {
                return Err(ModelError::Conflict(
                    "factory executable registration collision",
                ));
            }
        } else {
            self.factory_contracts.insert(identity, contract);
        }
        Ok(())
    }
    pub fn factory_contract(
        &self,
        identity: ValidationIdentity,
    ) -> Option<&super::validation_factory::ValidationFactoryContract> {
        self.factory_contracts.get(&identity)
    }
    /// Relations are canonicalized by name during validation.
    pub fn relation(&self, name: &str) -> Option<&Relation> {
        self.relations
            .binary_search_by_key(&name, |relation| relation.name())
            .ok()
            .map(|index| &self.relations[index])
    }
    pub fn relations(&self) -> &[Relation] {
        &self.relations
    }
    pub fn invariants(&self) -> &[Invariant] {
        &self.invariants
    }
    pub fn publication_checks(&self) -> &[PublicationInvariant] {
        &self.publication_checks
    }
    /// Every reference in the scope is required, even if another referring relation is absent.
    pub fn invariants_for_scope(
        &self,
        names: &std::collections::BTreeSet<&str>,
    ) -> Result<Vec<Invariant>, ModelError> {
        self.invariants_for_scope_with_premises(names, &std::collections::BTreeSet::new())
    }
    /// Referrers select required checks; separately acknowledged premises only complete inputs.
    /// The effect owner must bind those premises to actual acknowledgement receipts. Naming a
    /// premise here creates no read authority and does not select its own validation references.
    pub fn invariants_for_scope_with_premises(
        &self,
        referrers: &std::collections::BTreeSet<&str>,
        acknowledged_premises: &std::collections::BTreeSet<&str>,
    ) -> Result<Vec<Invariant>, ModelError> {
        self.checks_for_scope(referrers, acknowledged_premises, false)
    }
    /// Select necessary properties at their model owners, excluding differential reconstruction.
    pub fn admission_for_scope_with_premises(
        &self,
        referrers: &std::collections::BTreeSet<&str>,
        acknowledged_premises: &std::collections::BTreeSet<&str>,
    ) -> Result<Vec<Invariant>, ModelError> {
        self.checks_for_scope(referrers, acknowledged_premises, true)
    }
    /// Select model-owned admission checks before an executor resolves their premises.
    /// This grants no validity: the executor must enforce every applicable premise, allowing
    /// omission only after an actual declared owner stream is confirmed complete-empty.
    pub fn admission_candidates_for_scope(
        &self,
        referrers: &std::collections::BTreeSet<&str>,
    ) -> Result<Vec<Invariant>, ModelError> {
        self.select_checks_for_scope(referrers, &std::collections::BTreeSet::new(), true, false)
    }
    /// Shared vocabulary selects a check at each actual referrer's exact publication
    /// role. Its unqualified vocabulary premises refer to that same role, rather than
    /// to whichever cumulative stream happens to be available to the consumer.
    pub fn admission_candidates_for_views(
        &self,
        referrers: &[ValidationInput],
    ) -> Result<Vec<Invariant>, ModelError> {
        self.checks_for_views(referrers, &std::collections::BTreeSet::new(), true, false)
    }
    pub fn invariants_for_views_with_premises(
        &self,
        referrers: &[ValidationInput],
        acknowledged_premises: &std::collections::BTreeSet<&str>,
    ) -> Result<Vec<Invariant>, ModelError> {
        self.checks_for_views(referrers, acknowledged_premises, false, true)
    }
    fn checks_for_views(
        &self,
        referrers: &[ValidationInput],
        acknowledged_premises: &std::collections::BTreeSet<&str>,
        admission: bool,
        require_premises: bool,
    ) -> Result<Vec<Invariant>, ModelError> {
        let names = referrers.iter().map(ValidationInput::name).collect();
        let checks = self.select_checks_for_scope(
            &names,
            acknowledged_premises,
            admission,
            require_premises,
        )?;
        let mut by_check = std::collections::BTreeMap::<_, std::collections::BTreeSet<_>>::new();
        let mut by_relation = std::collections::BTreeMap::<_, std::collections::BTreeSet<_>>::new();
        for root in referrers {
            root.validate_prefix()?;
            let relation = self
                .relation(root.name())
                .ok_or(ModelError::Schema("validation referrer relation"))?;
            let role = if super::stages::is_vocabulary(root.name()) {
                root.prefix()
            } else {
                None
            };
            by_relation
                .entry(root.name())
                .or_default()
                .insert(root.prefix());
            for id in relation.invariant_refs() {
                by_check.entry(*id).or_default().insert(role);
            }
        }
        let mut instances = Vec::new();
        for check in checks {
            let Some(roles) = by_check.remove(check.name) else {
                // Generated graph validation remains one cross-relation check. Each
                // shared proof stream retains every actual selected role, while
                // ordinary proof sources are visited once. Nominal graph nodes keep
                // their relation/row identity, independently of publication copies.
                let mut generated = check;
                let mut expanded = Vec::new();
                for input in generated.inputs {
                    if super::stages::is_epoch_shared(input.name()) {
                        for role in &by_relation[&input.name()] {
                            expanded.push(if let Some(epoch) = role {
                                input.clone().at_epoch(*epoch)
                            } else {
                                input.clone()
                            });
                        }
                    } else {
                        expanded.push(input);
                    }
                }
                generated.inputs = expanded;
                instances.push(generated);
                continue;
            };
            for role in roles {
                let mut instance = check.clone();
                for input in &mut instance.inputs {
                    *input = input.clone().with_vocabulary_epoch(role);
                }
                instances.push(instance);
            }
        }
        Ok(instances)
    }
    fn checks_for_scope(
        &self,
        referrers: &std::collections::BTreeSet<&str>,
        acknowledged_premises: &std::collections::BTreeSet<&str>,
        admission: bool,
    ) -> Result<Vec<Invariant>, ModelError> {
        self.select_checks_for_scope(referrers, acknowledged_premises, admission, true)
    }
    fn select_checks_for_scope(
        &self,
        referrers: &std::collections::BTreeSet<&str>,
        acknowledged_premises: &std::collections::BTreeSet<&str>,
        admission: bool,
        require_premises: bool,
    ) -> Result<Vec<Invariant>, ModelError> {
        for name in acknowledged_premises {
            self.relation(name).ok_or_else(|| {
                ModelError::Invalid(format!("unknown validation premise relation {name}"))
            })?;
        }
        let mut ids = std::collections::BTreeSet::new();
        for name in referrers {
            let relation = self.relation(name).ok_or_else(|| {
                ModelError::Invalid(format!("unknown validation scope relation {name}"))
            })?;
            ids.extend(relation.invariant_refs().iter().copied());
        }
        let mut checks = Vec::new();
        for id in ids {
            let check = self.invariant(id)?;
            if !admission || check.purpose == InvariantPurpose::Admission {
                checks.push(check.clone());
            }
        }
        if let Some(check) = self.generated_invariant_for_scope(referrers)? {
            checks.push(check);
        }
        for check in checks.iter().filter(|_| require_premises) {
            if let Some(input) = check.inputs.iter().find(|input| {
                !referrers.contains(input.name()) && !acknowledged_premises.contains(input.name())
            }) {
                return Err(ModelError::Invalid(format!(
                    "{} requires validation premise {} outside scope",
                    check.name,
                    input.name()
                )));
            }
        }
        Ok(checks)
    }
    /// The generated checker is over exactly the declared derivation sources in this scope.
    /// A lower frontier must not silently omit the rule because upper inputs are unavailable.
    pub fn generated_invariant_for_scope(
        &self,
        names: &std::collections::BTreeSet<&str>,
    ) -> Result<Option<Invariant>, ModelError> {
        let mut sources = Vec::new();
        for name in names {
            let relation = self.relation(name).ok_or_else(|| {
                ModelError::Invalid(format!(
                    "unknown generated validation scope relation {name}"
                ))
            })?;
            if relation.derivation().is_some() {
                sources.push(relation.clone());
            }
        }
        sources.sort_by_key(Relation::name);
        Ok(generated_derivation(sources))
    }
    pub fn invariant(&self, id: &str) -> Result<&Invariant, ModelError> {
        self.invariants
            .iter()
            .find(|i| i.name == id)
            .ok_or_else(|| ModelError::Invalid(format!("unresolved invariant {id}")))
    }
    pub fn publication_check(&self, id: &str) -> Result<&PublicationInvariant, ModelError> {
        self.publication_checks
            .iter()
            .find(|i| i.name == id)
            .ok_or_else(|| ModelError::Invalid(format!("unresolved publication check {id}")))
    }
    pub fn definitions(&self) -> ValidationDefinitions {
        ValidationDefinitions {
            invariants: self
                .invariants
                .iter()
                .filter(|i| i.name != "derivation_acyclic")
                .cloned()
                .collect(),
            publication_checks: self.publication_checks.clone(),
        }
    }
    pub fn digest(&self) -> ContentHash {
        self.digest
    }
    pub fn require<R: Record>(&self) -> Result<&Relation, ModelError> {
        self.relations
            .iter()
            .find(|r| r.type_id == TypeId::of::<R>())
            .ok_or_else(|| ModelError::Invalid(format!("{} is not in this model", R::NAME)))
    }
}
fn generated_derivation(sources: Vec<Relation>) -> Option<Invariant> {
    if sources.is_empty() {
        return None;
    }
    let inputs = sources
        .iter()
        .map(|r| ValidationInput::of_relation(r, &["id"]))
        .collect();
    Some(Invariant {
        purpose: InvariantPurpose::Admission,
        name: "derivation_acyclic",
        revision: 2,
        inputs,
        create: std::sync::Arc::new(move |budget| {
            Box::new(super::derivation::Check::new(sources.clone(), budget))
        }),
    })
}
fn identifier(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 48
        && name
            .bytes()
            .enumerate()
            .all(|(i, c)| c == b'_' || c.is_ascii_lowercase() || (i > 0 && c.is_ascii_digit()))
}

fn canonical<R: Record>(
    batch: &arrow_array::RecordBatch,
) -> Result<arrow_array::RecordBatch, ModelError> {
    let mut rows = R::decode(batch)?;
    rows.sort_by_key(Record::id);
    for pair in rows.windows(2) {
        if pair[0].id() == pair[1].id() {
            return Err(ModelError::Conflict(R::NAME));
        }
    }
    R::encode(&rows)
}

/// Streaming content validation. Chunks must be globally ordered by ID; duplicate IDs refuse.
pub struct RelationContent {
    relation: &'static str,
    sink: KeySink,
    previous: Option<[u8; 16]>,
    count: u64,
}
impl RelationContent {
    pub fn finish(mut self) -> (u64, ContentHash) {
        self.sink.part(b"row-count", &self.count.to_le_bytes());
        (self.count, self.sink.finish())
    }
}
fn ordered_canonical<R: Record>(
    batch: &arrow_array::RecordBatch,
) -> Result<arrow_array::RecordBatch, ModelError> {
    let rows = R::decode(batch)?;
    for row in &rows {
        row.validate()?;
    }
    R::encode(&rows)
}
fn row_fingerprints<R: Record>(
    batch: &arrow_array::RecordBatch,
) -> Result<Vec<RowFingerprint>, ModelError> {
    R::decode(batch)?
        .into_iter()
        .map(|row| {
            row.validate()?;
            Ok(RowFingerprint {
                nominal: *row.id().bytes(),
                content: row.content_digest(),
            })
        })
        .collect()
}
fn hash_rows<R: Record>(
    batch: &arrow_array::RecordBatch,
    content: &mut RelationContent,
) -> Result<(), ModelError> {
    for row in R::decode(batch)? {
        let id = *row.id().bytes();
        if content.previous.is_some_and(|previous| previous >= id) {
            return Err(ModelError::Invalid(format!(
                "{} content rows must have strictly increasing IDs",
                R::NAME
            )));
        }
        content.sink.part(b"id", &id);
        content.sink.part(b"payload", &row.content_digest().0);
        content.previous = Some(id);
        content.count = content
            .count
            .checked_add(1)
            .ok_or_else(|| ModelError::Invalid("row count overflow".into()))?;
    }
    Ok(())
}

/// Admission checks necessary semantic properties. DiagnosticReplay independently compares a
/// producer against its reconstruction and is never selected by production artifact admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvariantPurpose {
    Admission,
    DiagnosticReplay,
}

/// A model-owned cross-relation invariant. The store supplies declared ordered inputs;
/// semantic validation itself remains a pure, independently testable state machine.
#[derive(Clone)]
pub struct Invariant {
    pub purpose: InvariantPurpose,
    pub revision: u32,
    pub name: &'static str,
    pub inputs: Vec<ValidationInput>,
    /// Checks retain state only through the supplied attempt budget (see `charged`).
    pub create: InvariantFactory,
}
impl Invariant {
    pub fn identity(&self) -> ValidationIdentity {
        ValidationIdentity {
            kind: ValidationKind::Invariant,
            id: self.name,
            revision: self.revision,
        }
    }
    pub fn digest(&self) -> ContentHash {
        let mut sink = KeySink::new("validation-definition/v1");
        self.identity().encode(&mut sink);
        sink.part(b"purpose", &[self.purpose as u8]);
        for input in &self.inputs {
            input.encode_contract(&mut sink);
        }
        sink.finish()
    }
}
impl std::fmt::Debug for Invariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Invariant")
            .field("name", &self.name)
            .field("inputs", &self.inputs)
            .finish_non_exhaustive()
    }
}
#[derive(Debug, Clone)]
pub struct ValidationInput {
    type_id: TypeId,
    name: &'static str,
    order: Vec<&'static str>,
    prefix: Option<super::stages::PublicationBoundary>,
}
impl ValidationInput {
    /// Shared vocabulary premises follow their exact referrer's role. An explicit
    /// premise role remains authoritative, and ordinary inputs remain unqualified.
    pub(crate) fn with_vocabulary_epoch(
        self,
        epoch: Option<super::stages::PublicationBoundary>,
    ) -> Self {
        if super::stages::is_vocabulary(self.name)
            && self.prefix.is_none()
            && let Some(epoch) = epoch
        {
            self.at_epoch(epoch)
        } else {
            self
        }
    }
    /// An earlier semantic owner replays against its immutable vocabulary, even when a later
    /// consumer also validates current vocabulary of the same nominal relation.
    pub fn at_epoch(mut self, prefix: super::stages::PublicationBoundary) -> Self {
        self.prefix = Some(prefix);
        self
    }
    pub fn prefix(&self) -> Option<super::stages::PublicationBoundary> {
        self.prefix
    }
    fn validate_prefix(&self) -> Result<(), ModelError> {
        if self.prefix.is_some() && !super::stages::is_epoch_shared(self.name) {
            return Err(ModelError::Invalid(
                "validation prefix applies only to shared relations".into(),
            ));
        }
        Ok(())
    }
    pub fn encode_contract(&self, sink: &mut KeySink) {
        sink.part(b"input-role", self.name.as_bytes());
        sink.part(b"input-prefix", &self.prefix_code());
        sink.part(b"order-count", &(self.order.len() as u64).to_le_bytes());
        for order in &self.order {
            sink.part(b"input-order", order.as_bytes());
        }
    }
    fn prefix_code(&self) -> [u8; 2] {
        self.prefix.map_or([0, 0], |prefix| [1, prefix.code()])
    }
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }
    pub fn name(&self) -> &'static str {
        self.name
    }
    pub fn order(&self) -> &[&'static str] {
        &self.order
    }
    pub fn of_relation(relation: &Relation, order: &[&'static str]) -> Self {
        Self {
            type_id: relation.type_id(),
            name: relation.name(),
            order: order.to_vec(),
            prefix: None,
        }
    }
    pub fn of<R: Record>(order: &[&'static str]) -> Self {
        Self {
            type_id: TypeId::of::<R>(),
            name: R::NAME,
            order: order.to_vec(),
            prefix: None,
        }
    }
}
pub trait InvariantCheck: Send {
    /// Lower selected storage admission supplies semantic compact owners explicitly.
    fn admit_uniform_owner(
        &mut self,
        _owner: &super::uniform_outcome::ValidatedUniformOwner,
    ) -> Result<(), ModelError> {
        Ok(())
    }
    /// Model-owned dependency declaration for private scoped reuse. False is
    /// conservative: when compact owners exist, this check earns fresh execution.
    /// True declares that admit_uniform_owner has no semantic effect for this
    /// kernel. This declaration never establishes execution or native authority.
    fn scoped_uniform_independent(&self) -> bool {
        false
    }
    fn retrieval_scope(&self) -> Option<super::retrieval::consumption::Scope> {
        None
    }
    fn aspect_scope(&self) -> Option<super::normalized::callable_aspects::AspectScope> {
        None
    }
    fn inventory_scope(&self) -> Option<super::flow_inventory::InventoryScope> {
        None
    }
    fn support_scope(&self) -> Option<super::assertion::SupportScope> {
        None
    }

    fn projection_scope(&self) -> Option<super::projection::normalization::AdmissionScope> {
        None
    }
    fn normalization_scope(&self) -> Option<super::normalized::admission::Scope> {
        None
    }

    fn execution_scope(&self) -> Option<super::execution::fidelity::ExecutionScope> {
        None
    }
    fn visit_input(
        &mut self,
        input: &ValidationInput,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        self.visit(input.name(), batch)
    }
    fn visit(&mut self, relation: &str, batch: &arrow_array::RecordBatch)
    -> Result<(), ModelError>;
    fn finish(self: Box<Self>) -> Result<(), ModelError>;
}

fn proofs<R: Record>(
    batch: &arrow_array::RecordBatch,
) -> Result<Vec<super::derivation::Proof>, ModelError> {
    R::decode(batch)?
        .into_iter()
        .map(|row| {
            row.proof()
                .ok_or_else(|| ModelError::Invalid("declared derivation has no proof".into()))
        })
        .collect()
}

type InvariantFactory = std::sync::Arc<
    dyn Fn(&super::resources::ResourceBudget) -> Box<dyn InvariantCheck> + Send + Sync,
>;

type PublicationFactory = std::sync::Arc<
    dyn Fn(&super::resources::ResourceBudget) -> Box<dyn PublicationCheck> + Send + Sync,
>;

/// Checks ordinary output consistency against effect-owner acknowledged source metadata.
/// The effect owner validates live authority before supplying snapshots; snapshots grant no reads.
#[derive(Clone)]
pub struct PublicationInvariant {
    pub revision: u32,
    pub name: &'static str,
    pub inputs: Vec<ValidationInput>,
    pub create: PublicationFactory,
}
impl PublicationInvariant {
    pub fn identity(&self) -> ValidationIdentity {
        ValidationIdentity {
            kind: ValidationKind::Publication,
            id: self.name,
            revision: self.revision,
        }
    }
    pub fn digest(&self) -> ContentHash {
        let mut sink = KeySink::new("validation-definition/v1");
        self.identity().encode(&mut sink);
        for input in &self.inputs {
            input.encode_contract(&mut sink);
        }
        sink.finish()
    }
}
impl std::fmt::Debug for PublicationInvariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PublicationInvariant")
            .field("name", &self.name)
            .field("inputs", &self.inputs)
            .finish_non_exhaustive()
    }
}
pub trait PublicationCheck: Send + Sync {
    /// A lower effect owner supplies this only after exact selected native membership admission.
    /// Checks that consume compact outcomes retain it explicitly; it is not reconstructed from
    /// deserialized source metadata or the compact row's own hashes.
    fn admit_uniform_owner(
        &mut self,
        _owner: &super::uniform_outcome::ValidatedUniformOwner,
    ) -> Result<(), ModelError> {
        Ok(())
    }
    fn visit_input(
        &mut self,
        input: &ValidationInput,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        self.visit(input.name(), batch)
    }
    fn visit(&mut self, relation: &str, batch: &arrow_array::RecordBatch)
    -> Result<(), ModelError>;
    fn finish(
        self: Box<Self>,
        sources: &[super::analysis::sources::SourceSnapshot],
        profile: super::stages::Profile,
    ) -> Result<(), ModelError>;
}

#[cfg(test)]
mod relation_metadata_controls {
    use super::*;
    #[test]
    fn immutable_relation_clones_share_all_declaration_storage_and_keep_model_identity() {
        let relation = Relation::of::<super::super::types::TypeTerm>();
        let clone = relation.clone();
        assert!(Arc::ptr_eq(&relation.fields, &clone.fields));
        assert!(Arc::ptr_eq(&relation.invariant_refs, &clone.invariant_refs));
        assert!(Arc::ptr_eq(
            &relation.publication_refs,
            &clone.publication_refs
        ));
        assert!(Arc::ptr_eq(&relation.required, &clone.required));
        assert!(Arc::ptr_eq(
            &relation.projection_roles,
            &clone.projection_roles
        ));
        assert!(Arc::ptr_eq(
            relation.sum.as_ref().unwrap(),
            clone.sum.as_ref().unwrap()
        ));
        assert!(Arc::ptr_eq(&relation.schema, &clone.schema));
        let owner = super::super::model().unwrap();
        let before = owner.digest();
        let proof = owner
            .relations()
            .iter()
            .find(|r| r.derivation.is_some())
            .expect("model has declared proofs");
        let copy = proof.clone();
        assert!(Arc::ptr_eq(
            proof.derivation.as_ref().unwrap(),
            copy.derivation.as_ref().unwrap()
        ));
        drop(relation);
        drop(owner);
        assert!(!clone.fields().is_empty());
        assert!(copy.derivation().is_some());
        assert_eq!(before, super::super::model().unwrap().digest());
    }
}

#[cfg(test)]
mod validation_factory_registration_controls {
    use super::*;
    #[test]
    fn custom_opaque_factories_are_ineligible_until_explicit_registration_without_changing_semantic_model()
     {
        let canonical = crate::domain::model().unwrap();
        assert!(!canonical.invariants().is_empty());
        assert!(
            canonical
                .invariants()
                .iter()
                .all(|check| canonical.factory_contract(check.identity()).is_some())
        );
        let mut custom =
            ValidatedModel::validate(canonical.relations().to_vec(), canonical.definitions())
                .unwrap();
        assert_eq!(
            canonical.digest(),
            custom.digest(),
            "factory provenance remains separate from model compatibility"
        );
        assert!(
            custom
                .invariants()
                .iter()
                .all(|check| custom.factory_contract(check.identity()).is_none()),
            "same names/definitions cannot silently qualify opaque caller factories"
        );
        let check = custom.invariants()[0].identity();
        let implementation = ContentHash::of(b"declared custom executable");
        custom
            .register_factory(check, implementation, Vec::new())
            .unwrap();
        assert_eq!(
            custom.factory_contract(check).unwrap().implementation,
            implementation
        );
        custom
            .register_factory(check, implementation, Vec::new())
            .unwrap();
        assert!(
            custom
                .register_factory(
                    check,
                    ContentHash::of(b"another factory behind same registry slot"),
                    Vec::new()
                )
                .is_err(),
            "a different executable cannot mutate an existing eligible registry slot"
        );
    }
}
