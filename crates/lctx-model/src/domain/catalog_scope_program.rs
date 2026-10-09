//! Catalog, documentary and retrieval publication scope meaning, independent of realization.
use super::{resources::ResourceBudget, scope_program::*, *};
use source::{Occurrence, SourceArtifact};
use std::any::TypeId;
pub struct BudgetedCatalogProgram {
    program: ScopeProgram,
    parameters: ScopeParameters,
    _charge: charged::StateCharge,
}
impl BudgetedCatalogProgram {
    pub fn program(&self) -> &ScopeProgram {
        &self.program
    }
    pub fn parameters(&self) -> &ScopeParameters {
        &self.parameters
    }
}
pub(crate) struct Builder {
    pub(crate) program: ScopeProgram,
    charge: charged::StateCharge,
    retained_bytes: usize,
}
impl Builder {
    pub(crate) fn new(
        inputs: Vec<ValidationInput>,
        real: usize,
        relations: &[Relation],
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        let mut charge = charged::StateCharge::new(budget, "catalog-scope-program");
        // Charge a bounded construction arena before allocating owned rules and row/predicate vectors.
        charge.grow(
            262_144usize
                .saturating_add(inputs.len() * 512)
                .saturating_add(relations.iter().map(|r| r.fields().len()).sum::<usize>() * 512),
        )?;
        let ports = (0..inputs.len())
            .map(|input| ScopePort {
                input,
                virtual_owner: input >= real,
            })
            .collect();
        Ok(Self {
            program: ScopeProgram {
                inputs,
                ports,
                rules: vec![],
            },
            charge,
            retained_bytes: 0,
        })
    }
    pub(crate) fn reserve_rules(&mut self, bytes: usize) -> Result<(), ModelError> {
        self.charge.grow(bytes)
    }
    /// Storage outside ScopeProgram that remains owned by the finished factory wrapper.
    pub(crate) fn reserve_retained(&mut self, bytes: usize) -> Result<(), ModelError> {
        self.charge.grow(bytes)?;
        self.retained_bytes = self.retained_bytes.saturating_add(bytes);
        Ok(())
    }
    pub(crate) fn finish(mut self) -> Result<BudgetedCatalogProgram, ModelError> {
        // Replace construction scratch with the metadata that actually survives finish.
        // Columns, names and predicate literals borrow static storage. Input orders are
        // immutable exact-sized to_vec/clone allocations; all other owned vectors expose
        // their capacities here, including each optional join's nested key vector.
        let bytes = size_of::<BudgetedCatalogProgram>() + self.retained_bytes
            + self.program.inputs.capacity() * size_of::<ValidationInput>()
            + self.program.inputs.iter().map(|input| input.order().len() * size_of::<&str>()).sum::<usize>()
            + self.program.ports.capacity() * size_of::<ScopePort>()
            + self.program.rules.capacity() * size_of::<ScopeRule>()
            + self
                .program
                .rules
                .iter()
                .map(|r| match r {
                    ScopeRule::Reference { .. } => 0,
                    ScopeRule::Pairs {
                        rows, predicates, ..
                    }
                    | ScopeRule::FirstPairs {
                        rows, predicates, ..
                    } => {
                        rows.capacity() * size_of::<usize>()
                            + predicates.capacity() * size_of::<ScopePredicate>()
                    }
                    ScopeRule::NearestPairs {
                        rows,
                        predicates,
                        post,
                        ..
                    } => {
                        rows.capacity() * size_of::<usize>()
                            + (predicates.capacity() + post.capacity())
                                * size_of::<ScopePredicate>()
                    }
                    ScopeRule::OptionalPairs {
                        rows,
                        predicates,
                        optional,
                        ..
                    } => {
                        rows.capacity() * size_of::<usize>()
                            + predicates.capacity() * size_of::<ScopePredicate>()
                            + optional.capacity() * size_of::<ScopeOptionalJoin>()
                            + optional
                                .iter()
                                .map(|j| {
                                    j.keys.capacity() * size_of::<(ScopeColumn, ScopeColumn)>()
                                })
                                .sum::<usize>()
                    }
                })
                .sum::<usize>();
        let reserved = self.charge.reserved();
        if bytes > reserved {
            self.charge.grow(bytes - reserved)?;
        } else {
            self.charge.release(reserved - bytes);
        }
        Ok(BudgetedCatalogProgram {
            program: self.program,
            parameters: ScopeParameters(vec![]),
            _charge: self.charge,
        })
    }
    pub(crate) fn follow(&mut self, source: usize, field: &'static str, target: usize, list: bool) {
        self.program.rules.push(ScopeRule::Reference {
            source,
            field,
            target,
            direction: ScopeDirection::Forward,
            list,
        });
    }
    pub(crate) fn reverse(&mut self, source: usize, field: &'static str, target: usize) {
        self.program.rules.push(ScopeRule::Reference {
            source,
            field,
            target,
            direction: ScopeDirection::OwnedReverse,
            list: false,
        });
    }
    pub(crate) fn own(&mut self, source: usize, field: &'static str, target: usize) {
        self.follow(source, field, target, false);
        self.reverse(source, field, target);
    }
    pub(crate) fn pair(
        &mut self,
        source: usize,
        target: usize,
        rows: &[usize],
        predicates: Vec<ScopePredicate>,
        source_key: ScopeColumn,
        target_key: ScopeColumn,
    ) {
        self.program.rules.push(ScopeRule::Pairs {
            source,
            target,
            rows: rows.to_vec(),
            predicates,
            source_key,
            target_key,
        });
    }
}
fn col(row: usize, field: &'static str) -> ScopeColumn {
    ScopeColumn { row, field }
}
fn eq(a: usize, af: &'static str, b: usize, bf: &'static str) -> ScopePredicate {
    ScopePredicate::Equal(col(a, af), col(b, bf))
}
fn membership(source: TypeId, field: &str, kind: TypeId) -> bool {
    use normalized::{callable_aspects::*, callables::*, entities::*, links::*};
    let pair = |member: TypeId, owner: &str| source == member && field == owner;
    pair(TypeId::of::<PublicExposureCandidate>(), "exposure")
        || pair(TypeId::of::<SymbolEntityCandidate>(), "resolution")
        || pair(TypeId::of::<EffectiveCallableAssessment>(), "callable")
        || pair(TypeId::of::<SignatureVariant>(), "callable")
        || pair(TypeId::of::<SignatureSlot>(), "variant")
        || pair(TypeId::of::<SignatureSlotEntity>(), "slot")
        || pair(TypeId::of::<SignatureSlotType>(), "slot")
        || pair(TypeId::of::<SignatureReturnType>(), "variant")
        || pair(TypeId::of::<CallableAspect>(), "assessment")
        || pair(TypeId::of::<FieldEntity>(), "class")
        || pair(TypeId::of::<FieldEntityLink>(), "field")
        || pair(TypeId::of::<FieldDeclarationLink>(), "field")
        || pair(TypeId::of::<FieldDefaultAssessment>(), "declaration")
        || pair(TypeId::of::<ParameterEntityLink>(), "entity")
        || pair(
            TypeId::of::<syntax::ParameterSyntaxObservation>(),
            "function",
        )
        || pair(
            TypeId::of::<syntax::ParameterSyntaxObservation>(),
            "parameter",
        )
        || pair(
            TypeId::of::<syntax::DeclarationObservation>(),
            "declaration",
        )
        || pair(TypeId::of::<syntax::DeclarationObservation>(), "parent")
        || pair(TypeId::of::<OccurrenceOwnership>(), "occurrence")
        || pair(TypeId::of::<lexical::BindingEvent>(), "site")
        || pair(TypeId::of::<lexical::BindingObservation>(), "event")
        || pair(TypeId::of::<lexical::ReferenceObservation>(), "read")
        || pair(TypeId::of::<ReferenceEntityAssessment>(), "reference")
        || pair(TypeId::of::<ReferenceEntityCandidate>(), "assessment")
        || pair(TypeId::of::<AncestryEntityAssessment>(), "class")
        || pair(TypeId::of::<AncestryEntityMember>(), "assessment")
        || pair(TypeId::of::<SymbolEntityResolution>(), "entity")
        || pair(TypeId::of::<SymbolEntityResolution>(), "symbol")
        || pair(TypeId::of::<symbols::FunctionTraitObservation>(), "symbol")
        || pair(
            TypeId::of::<symbols::FunctionTraitObservation>(),
            "defining_class",
        )
        || pair(TypeId::of::<symbols::ClassTraitObservation>(), "symbol")
        || pair(
            TypeId::of::<class_metadata::ClassMetadataObservation>(),
            "class",
        )
        || pair(
            TypeId::of::<class_metadata::ClassMemberObservation>(),
            "class",
        )
        || (kind == TypeId::of::<source::Occurrence>()
            && [
                TypeId::of::<CallableEntity>(),
                TypeId::of::<ClassEntity>(),
                TypeId::of::<ParameterEntity>(),
            ]
            .contains(&source))
        || (source == TypeId::of::<EntityRef>()
            && [
                TypeId::of::<CallableEntity>(),
                TypeId::of::<ClassEntity>(),
                TypeId::of::<ParameterEntity>(),
            ]
            .contains(&kind))
}
fn target(
    inputs: &[ValidationInput],
    source: usize,
    kind: TypeId,
) -> Result<Option<usize>, ModelError> {
    let candidates: Vec<_> = inputs
        .iter()
        .enumerate()
        .filter(|(_, input)| input.type_id() == kind)
        .map(|(index, _)| index)
        .collect();
    if candidates.len() == 1 {
        return Ok(candidates.first().copied());
    }
    if candidates.is_empty() {
        return Ok(None);
    }
    let prefix = if kind == TypeId::of::<assertion::AssertionQualification>() {
        Some(
            if inputs[source].type_id() == TypeId::of::<local_fields::FieldLocation>() {
                stages::PublicationBoundary::Local
            } else {
                stages::PublicationBoundary::Facts
            },
        )
    } else {
        inputs[source].prefix()
    };
    let selected: Vec<_> = candidates
        .into_iter()
        .filter(|index| inputs[*index].prefix() == prefix)
        .collect();
    if selected.len() != 1 {
        return Err(ModelError::Conflict("C1 dependency immutable epoch"));
    }
    Ok(selected.first().copied())
}
fn typed<R: Record>(inputs: &[ValidationInput]) -> Result<usize, ModelError> {
    inputs
        .iter()
        .position(|input| input.type_id() == TypeId::of::<R>())
        .ok_or(ModelError::Schema("C1 root relation absent"))
}
fn evidence_memberships() -> Vec<TypeId> {
    use super::{
        catalog::*,
        documents::*,
        local_fields::*,
        normalized::{bindings::*, callables::*, entities::*, events::*, links::*},
    };
    vec![
        TypeId::of::<CatalogMember>(),
        TypeId::of::<CatalogCallable>(),
        TypeId::of::<CatalogClass>(),
        TypeId::of::<EffectiveCallableAssessment>(),
        TypeId::of::<SignatureVariant>(),
        TypeId::of::<SignatureSlot>(),
        TypeId::of::<PublicExposure>(),
        TypeId::of::<PublicExposureCandidate>(),
        TypeId::of::<SymbolEntityResolution>(),
        TypeId::of::<ReferenceEntityAssessment>(),
        TypeId::of::<AncestryEntityAssessment>(),
        TypeId::of::<ParameterEntity>(),
        TypeId::of::<ClassEntity>(),
        TypeId::of::<FieldEntity>(),
        TypeId::of::<NormalizedCallEvent>(),
        TypeId::of::<NormalizedCallAlternative>(),
        TypeId::of::<CallBindingAttempt>(),
        TypeId::of::<DocumentNode>(),
        TypeId::of::<DocumentObservation>(),
        TypeId::of::<FieldLocation>(),
        TypeId::of::<symbols::SymbolSequence>(),
        TypeId::of::<normalized::symbolic_fields::SourceFieldClass>(),
        TypeId::of::<normalized::symbolic_fields::SourceFieldReader>(),
    ]
}

pub fn core(
    inputs: Vec<ValidationInput>,
    relations: &[Relation],
    model: &ValidatedModel,
    budget: &ResourceBudget,
) -> Result<BudgetedCatalogProgram, ModelError> {
    use normalized::entities::{EntityRef, OccurrenceOwnership, PublicExposure};
    let names = typed::<symbols::PublicNameObservation>(&inputs)?;
    let exposures = typed::<PublicExposure>(&inputs)?;
    let real = inputs.len();
    let root = real;
    let field_bindings = catalog::build::CatalogData::scoped_field_bindings(model, &inputs)?;
    let mut declarations = inputs.clone();
    declarations.push(inputs[names].clone());
    let mut b = Builder::new(declarations, real, relations, budget)?;
    b.pair(root, names, &[names], vec![], col(0, "id"), col(0, "id"));
    b.pair(
        root,
        exposures,
        &[exposures],
        vec![],
        col(0, "observation"),
        col(0, "id"),
    );
    for (source, relation) in relations.iter().enumerate() {
        for (field_index, field) in relation.fields().iter().enumerate() {
            if let Some((kind, _)) = field.target()
                && let Some(to) = field_bindings[source][field_index]
            {
                b.follow(source, field.name(), to, field.list());
                if !field.list() && membership(relation.type_id(), field.name(), kind) {
                    b.reverse(source, field.name(), to);
                }
            }
        }
    }
    let scopes = typed::<lexical::LexicalScope>(&inputs)?;
    let bindings = typed::<lexical::BindingObservation>(&inputs)?;
    let events = typed::<lexical::BindingEvent>(&inputs)?;
    let ownership = typed::<OccurrenceOwnership>(&inputs)?;
    let refs = typed::<EntityRef>(&inputs)?;
    let origins = typed::<symbols::ExportOrigin>(&inputs)?;
    let modules = typed::<calls::ProviderModule>(&inputs)?;
    // Nullable origin/module paths preserve fallback and exclusion. OR is a union of exact rules.
    for predicate in [
        ScopePredicate::IsNull(col(2, "traced_module"), true),
        ScopePredicate::IsNull(col(3, "acquired_module"), false),
    ] {
        b.program.rules.push(ScopeRule::OptionalPairs {
            source: root,
            target: bindings,
            rows: vec![
                names, exposures, origins, modules, refs, ownership, scopes, events, bindings,
            ],
            optional: vec![
                ScopeOptionalJoin {
                    row: 2,
                    keys: vec![(col(2, "id"), col(0, "origin"))],
                },
                ScopeOptionalJoin {
                    row: 3,
                    keys: vec![(col(3, "id"), col(2, "traced_module"))],
                },
            ],
            predicates: vec![
                eq(1, "observation", 0, "id"),
                ScopePredicate::EqualCoalesce {
                    column: col(4, "module_module"),
                    primary: col(3, "acquired_module"),
                    fallback: col(1, "access"),
                },
                eq(5, "entity", 4, "id"),
                eq(6, "owner", 5, "owner"),
                ScopePredicate::Code(col(6, "kind"), lexical::LexicalScopeKind::Module as i16),
                eq(7, "site", 5, "occurrence"),
                ScopePredicate::EqualCoalesce {
                    column: col(7, "name"),
                    primary: col(2, "traced_name"),
                    fallback: col(0, "name"),
                },
                eq(8, "event", 7, "id"),
                eq(8, "scope", 6, "id"),
                predicate,
                ScopePredicate::Code(col(8, "kind"), lexical::BindingEventKind::Assignment as i16),
                ScopePredicate::IsNull(col(8, "static_branch"), true),
            ],
            source_key: col(0, "id"),
            target_key: col(8, "id"),
        });
    }
    let occurrences = typed::<source::Occurrence>(&inputs)?;
    let placements = typed::<syntax::SyntaxPlacement>(&inputs)?;
    b.pair(
        occurrences,
        scopes,
        &[scopes],
        vec![ScopePredicate::Code(
            col(0, "kind"),
            lexical::LexicalScopeKind::Class as i16,
        )],
        col(0, "owner"),
        col(0, "id"),
    );
    b.pair(
        scopes,
        bindings,
        &[scopes, bindings],
        vec![
            eq(1, "scope", 0, "id"),
            ScopePredicate::Code(col(0, "kind"), lexical::LexicalScopeKind::Class as i16),
        ],
        col(0, "id"),
        col(1, "id"),
    );
    b.pair(
        occurrences,
        placements,
        &[placements, occurrences],
        vec![
            eq(1, "id", 0, "occurrence"),
            ScopePredicate::Code(col(1, "syntax_kind"), source::SyntaxKind::Parameter as i16),
        ],
        col(0, "parent"),
        col(0, "id"),
    );
    let qualifications = typed::<assertion::AssertionQualification>(&inputs)?;
    let coverage = typed::<attribution::ProviderCoverage>(&inputs)?;
    b.pair(
        qualifications,
        coverage,
        &[qualifications, coverage],
        vec![
            eq(1, "scope", 0, "scope"),
            eq(1, "context", 0, "context"),
            ScopePredicate::Code(col(1, "family"), attribution::FactFamily::Signatures as i16),
        ],
        col(0, "id"),
        col(1, "id"),
    );
    b.finish()
}
pub fn evidence(
    inputs: Vec<ValidationInput>,
    relations: &[Relation],
    budget: &ResourceBudget,
) -> Result<BudgetedCatalogProgram, ModelError> {
    let real = inputs.len();
    let artifact = typed::<SourceArtifact>(&inputs)?;
    let occurrence = typed::<Occurrence>(&inputs)?;
    let root = real;
    let root_occurrence = real + 1;
    let mut declarations = inputs.clone();
    declarations.extend([inputs[artifact].clone(), inputs[occurrence].clone()]);
    let mut b = Builder::new(declarations, real, relations, budget)?;
    b.pair(
        root,
        artifact,
        &[artifact],
        vec![],
        col(0, "id"),
        col(0, "id"),
    );
    b.pair(
        root_occurrence,
        occurrence,
        &[occurrence],
        vec![],
        col(0, "id"),
        col(0, "id"),
    );
    b.pair(
        root,
        root_occurrence,
        &[occurrence],
        vec![],
        col(0, "source"),
        col(0, "id"),
    );
    let memberships = evidence_memberships();
    for (source, relation) in relations.iter().enumerate() {
        for field in relation.fields() {
            if let Some((kind, _)) = field.target()
                && let Some(to) = target(&inputs, source, kind)?
            {
                b.follow(source, field.name(), to, field.list());
                if field.list() {
                    continue;
                }
                if kind == TypeId::of::<SourceArtifact>() {
                    b.reverse(source, field.name(), root);
                } else if kind == TypeId::of::<Occurrence>() {
                    b.reverse(source, field.name(), root_occurrence);
                }
                if memberships.contains(&kind) {
                    b.reverse(source, field.name(), to);
                }
            }
        }
    }
    let derived = typed::<input::DerivedArtifact>(&inputs)?;
    b.pair(
        root,
        root,
        &[derived],
        vec![ScopePredicate::IsNull(
            col(0, "pythoncodeblock_document"),
            false,
        )],
        col(0, "pythoncodeblock_document"),
        col(0, "pythoncodeblock_artifact"),
    );
    let modules = typed::<source::Module>(&inputs)?;
    let members = typed::<catalog::CatalogMember>(&inputs)?;
    b.pair(
        root,
        members,
        &[modules, members],
        vec![eq(1, "access", 0, "id")],
        col(0, "source"),
        col(1, "id"),
    );
    let owners = [
        catalog::CatalogMemberInvocation::NAME,
        diagnostics::RuffDiagnosticSupport::NAME,
        diagnostics::PyreflyDiagnosticSupport::NAME,
        diagnostics::NativeParameterDefinitionSupport::NAME,
        calls::ProviderCallSiteSupport::NAME,
        analysis::native::NativeAssertionPremise::NAME,
    ];
    for (source, relation) in relations
        .iter()
        .enumerate()
        .filter(|(_, r)| owners.contains(&r.name()))
    {
        for field in relation.fields() {
            if !field.list()
                && (relation.name() == analysis::native::NativeAssertionPremise::NAME
                    || field.name() == "assertion"
                    || field.name() == "member")
                && let Some((kind, _)) = field.target()
                && let Some(to) = target(&inputs, source, kind)?
            {
                b.reverse(source, field.name(), to);
            }
        }
    }
    let coverage = typed::<attribution::ProviderCoverage>(&inputs)?;
    let coverage_scope = typed::<source::CoverageScope>(&inputs)?;
    b.own(coverage, "scope", coverage_scope);
    let nodes = typed::<documents::DocumentNode>(&inputs)?;
    for field in relations[nodes].fields().iter().filter(|field| {
        field
            .target()
            .is_some_and(|(kind, _)| kind == TypeId::of::<assertion::Evidence>())
    }) {
        let owner = target(&inputs, nodes, TypeId::of::<assertion::Evidence>())?
            .ok_or(ModelError::Schema("C1 document span owner"))?;
        b.own(nodes, field.name(), owner);
    }
    for (source, relation) in relations.iter().enumerate() {
        let field = if relation.type_id() == TypeId::of::<syntax::DeclarationObservation>() {
            Some("declaration")
        } else if relation.type_id() == TypeId::of::<syntax::ParameterSyntaxObservation>() {
            Some("function")
        } else {
            None
        };
        if let Some(field) = field {
            b.own(source, field, occurrence);
        }
    }
    let references = typed::<lexical::ReferenceObservation>(&inputs)?;
    let assessments = typed::<normalized::links::ReferenceEntityAssessment>(&inputs)?;
    b.pair(
        root,
        assessments,
        &[assessments, references, occurrence],
        vec![eq(0, "reference", 1, "id"), eq(1, "read", 2, "id")],
        col(2, "source"),
        col(0, "id"),
    );
    let bindings = typed::<lexical::BindingObservation>(&inputs)?;
    let events = typed::<lexical::BindingEvent>(&inputs)?;
    b.pair(
        root,
        bindings,
        &[bindings, events, occurrence],
        vec![eq(0, "event", 1, "id"), eq(1, "site", 2, "id")],
        col(2, "source"),
        col(0, "id"),
    );
    b.finish()
}
fn member_context(
    inputs: &[ValidationInput],
    source: usize,
    row: usize,
    rows: &mut Vec<usize>,
    predicates: &mut Vec<ScopePredicate>,
    context: ScopeColumn,
    extended: bool,
) -> Result<(), ModelError> {
    let kind = inputs[source].type_id();
    let mut qualified = None;
    if kind == TypeId::of::<catalog::CatalogExposure>() {
        let n = rows.len();
        rows.push(typed::<normalized::entities::PublicExposure>(inputs)?);
        predicates.push(eq(n, "id", row, "exposure"));
        qualified = Some(col(n, "context"));
    } else if kind == TypeId::of::<catalog::CatalogCallable>() {
        let n = rows.len();
        rows.push(typed::<normalized::callables::EffectiveCallableAssessment>(
            inputs,
        )?);
        predicates.push(eq(n, "id", row, "assessment"));
        qualified = Some(col(n, "context"));
    } else if extended && kind == TypeId::of::<catalog::evidence::ScenarioAssociation>() {
        let q = target(
            inputs,
            source,
            TypeId::of::<assertion::AssertionQualification>(),
        )?
        .ok_or(ModelError::Schema("catalog scenario context"))?;
        let n = rows.len();
        rows.push(q);
        predicates.push(eq(n, "id", row, "qualification"));
        qualified = Some(col(n, "context"));
    } else if extended && kind == TypeId::of::<catalog::evidence::DocumentAssociation>() {
        let candidate = typed::<normalized::links::MentionEntityCandidate>(inputs)?;
        let assessment = typed::<normalized::links::MentionEntityAssessment>(inputs)?;
        let mention = typed::<documents::DocumentMentionObservation>(inputs)?;
        let q = target(
            inputs,
            mention,
            TypeId::of::<assertion::AssertionQualification>(),
        )?
        .ok_or(ModelError::Schema("catalog mention context"))?;
        let n = rows.len();
        rows.extend([candidate, assessment, mention, q]);
        predicates.extend([
            eq(n, "id", row, "candidate"),
            eq(n + 1, "id", n, "assessment"),
            eq(n + 2, "id", n + 1, "observation"),
            eq(n + 3, "id", n + 2, "qualification"),
        ]);
        qualified = Some(col(n + 3, "context"));
    }
    if let Some(q) = qualified {
        predicates.push(ScopePredicate::Equal(q, context));
    }
    Ok(())
}
pub fn selection(
    inputs: Vec<ValidationInput>,
    relations: &[Relation],
    budget: &ResourceBudget,
) -> Result<BudgetedCatalogProgram, ModelError> {
    let member_table = typed::<catalog::CatalogMember>(&inputs)?;
    let link = typed::<catalog::CatalogMemberInvocation>(&inputs)?;
    let core = typed::<analysis::catalog_core::Invocation>(&inputs)?;
    let occurrence = typed::<source::Occurrence>(&inputs)?;
    let real = inputs.len();
    let member = real;
    let head = real + 1;
    let mut declarations = inputs.clone();
    declarations.extend([inputs[link].clone(), inputs[occurrence].clone()]);
    let mut b = Builder::new(declarations, real, relations, budget)?;
    b.pair(member, link, &[link], vec![], col(0, "id"), col(0, "id"));
    let memberships = selection::build::memberships();
    for (source, relation) in relations.iter().enumerate() {
        for field in relation.fields() {
            if let Some((kind, _)) = field.target()
                && let Some(to) = target(&inputs, source, kind)?
            {
                b.follow(source, field.name(), to, field.list());
                if memberships.contains(&(relation.type_id(), field.name()))
                    && relation.type_id()
                        != TypeId::of::<catalog::evidence::FieldAccessAssessment>()
                {
                    if kind == TypeId::of::<catalog::CatalogMember>() {
                        if relation.type_id() == TypeId::of::<catalog::CatalogMemberInvocation>() {
                            continue;
                        }
                        let mut rows = vec![link, core, source];
                        let mut predicates = vec![
                            eq(1, "id", 0, "invocation"),
                            eq(2, field.name(), 0, "member"),
                        ];
                        member_context(
                            &inputs,
                            source,
                            2,
                            &mut rows,
                            &mut predicates,
                            col(1, "context"),
                            true,
                        )?;
                        b.pair(
                            member,
                            source,
                            &rows,
                            predicates,
                            col(0, "id"),
                            col(2, "id"),
                        );
                    } else {
                        b.reverse(source, field.name(), to);
                    }
                }
            }
        }
    }
    for kind in [
        TypeId::of::<analysis::catalog_core::Invocation>(),
        TypeId::of::<analysis::catalog_evidence::Invocation>(),
    ] {
        let to = inputs
            .iter()
            .position(|input| input.type_id() == kind)
            .ok_or(ModelError::Schema("C2 fixed frame scope"))?;
        b.pair(
            member,
            to,
            &[link, member_table, core, to],
            vec![
                eq(1, "id", 0, "member"),
                eq(2, "id", 0, "invocation"),
                eq(3, "input", 1, "input"),
                eq(3, "context", 2, "context"),
            ],
            col(0, "id"),
            col(3, "id"),
        );
    }
    for kind in [
        TypeId::of::<analysis::catalog_core::AnalysisCoverage>(),
        TypeId::of::<analysis::catalog_evidence::AnalysisCoverage>(),
    ] {
        let from = inputs
            .iter()
            .position(|input| input.type_id() == kind)
            .ok_or(ModelError::Schema("C2 coverage scope"))?;
        let kind = relations[from]
            .fields()
            .iter()
            .find(|field| field.name() == "invocation")
            .and_then(|field| field.target())
            .ok_or(ModelError::Schema("C2 coverage invocation"))?
            .0;
        let to = target(&inputs, from, kind)?.ok_or(ModelError::Schema("C2 coverage owner"))?;
        b.own(from, "invocation", to);
    }
    let access = typed::<catalog::evidence::FieldAccessAssessment>(&inputs)?;
    let option = typed::<catalog::CatalogOption>(&inputs)?;
    let q = target(
        &inputs,
        access,
        TypeId::of::<assertion::AssertionQualification>(),
    )?
    .ok_or(ModelError::Schema("C2 access context"))?;
    b.pair(
        member,
        access,
        &[link, core, option, access, q],
        vec![
            eq(1, "id", 0, "invocation"),
            eq(2, "member", 0, "member"),
            eq(3, "option", 2, "id"),
            eq(4, "id", 3, "qualification"),
            eq(4, "context", 1, "context"),
        ],
        col(0, "id"),
        col(3, "id"),
    );
    let decorator = typed::<syntax::DeclarationDecorator>(&inputs)?;
    let placement = typed::<syntax::SyntaxPlacement>(&inputs)?;
    b.pair(
        decorator,
        head,
        &[decorator],
        vec![],
        col(0, "id"),
        col(0, "decorator"),
    );
    b.pair(
        head,
        occurrence,
        &[occurrence],
        vec![],
        col(0, "id"),
        col(0, "id"),
    );
    const FIELDS: &[i16] = &[
        lexical::SyntaxField::Child as i16,
        lexical::SyntaxField::Callee as i16,
    ];
    b.pair(
        head,
        placement,
        &[placement],
        vec![ScopePredicate::CodeIn(col(0, "field"), FIELDS)],
        col(0, "parent"),
        col(0, "id"),
    );
    b.pair(
        placement,
        head,
        &[placement],
        vec![ScopePredicate::CodeIn(col(0, "field"), FIELDS)],
        col(0, "id"),
        col(0, "occurrence"),
    );
    b.finish()
}
pub fn retrieval(
    inputs: Vec<ValidationInput>,
    relations: &[Relation],
    model: &ValidatedModel,
    budget: &ResourceBudget,
) -> Result<BudgetedCatalogProgram, ModelError> {
    use catalog::evidence as c1;
    use retrieval::build::{self, Data};
    let root_table = typed::<c1::EvidenceRoot>(&inputs)?;
    let brief_table = typed::<synthesis::briefs::Brief>(&inputs)?;
    let real = inputs.len();
    let root = real;
    let brief = real + 1;
    let mut declarations = inputs.clone();
    declarations.extend([inputs[root_table].clone(), inputs[brief_table].clone()]);
    let mut b = Builder::new(declarations, real, relations, budget)?;
    for (from, to) in [(root, root_table), (brief, brief_table)] {
        b.pair(from, to, &[to], vec![], col(0, "id"), col(0, "id"));
    }
    let memberships = build::memberships();
    let subjects = typed::<c1::RootSubject>(&inputs)?;
    let field_bindings = Data::scoped_field_bindings(model, &inputs)?;
    for (source, relation) in relations.iter().enumerate() {
        for (n, field) in relation.fields().iter().enumerate() {
            if let Some((kind, _)) = field.target()
                && let Some(to) = field_bindings[source][n]
            {
                b.follow(source, field.name(), to, field.list());
                if memberships.contains(&(relation.type_id(), field.name())) {
                    if kind == TypeId::of::<catalog::CatalogMember>() {
                        let mut rows = vec![root_table, subjects, source];
                        let mut predicates = vec![
                            eq(1, "id", 0, "subject"),
                            eq(2, field.name(), 1, "member_member"),
                        ];
                        member_context(
                            &inputs,
                            source,
                            2,
                            &mut rows,
                            &mut predicates,
                            col(0, "context"),
                            false,
                        )?;
                        b.pair(root, source, &rows, predicates, col(0, "id"), col(2, "id"));
                    } else {
                        b.reverse(source, field.name(), to);
                    }
                }
            }
        }
    }
    let document = typed::<documents::DocumentObservation>(&inputs)?;
    let passage = typed::<documents::PassageObservation>(&inputs)?;
    let nodes = typed::<documents::DocumentNode>(&inputs)?;
    let evidence = typed::<assertion::Evidence>(&inputs)?;
    let q = inputs
        .iter()
        .position(|input| {
            input.type_id() == TypeId::of::<assertion::AssertionQualification>()
                && input.prefix() == Some(stages::PublicationBoundary::Facts)
        })
        .ok_or(ModelError::Conflict(
            "E0 passage Facts qualification reader absent",
        ))?;
    b.pair(
        root,
        passage,
        &[root_table, subjects, document, evidence, nodes, passage, q],
        vec![
            eq(1, "id", 0, "subject"),
            eq(2, "id", 1, "document_observation"),
            eq(3, "sourcespan_source", 2, "source"),
            eq(4, "passage_span", 3, "id"),
            eq(5, "passage", 4, "id"),
            eq(6, "id", 5, "qualification"),
            eq(6, "context", 0, "context"),
        ],
        col(0, "id"),
        col(5, "id"),
    );
    let mention = typed::<documents::DocumentMentionObservation>(&inputs)?;
    let assessments = typed::<normalized::links::MentionEntityAssessment>(&inputs)?;
    let candidates = typed::<normalized::links::MentionEntityCandidate>(&inputs)?;
    let associations = typed::<c1::DocumentAssociation>(&inputs)?;
    b.own(mention, "passage", nodes);
    b.own(
        typed::<documents::DocumentComponentObservation>(&inputs)?,
        "passage",
        nodes,
    );
    b.own(assessments, "observation", mention);
    b.own(candidates, "assessment", assessments);
    b.own(associations, "candidate", candidates);
    let seeds = typed::<synthesis::seeds::SelectedSeed>(&inputs)?;
    let plans = typed::<synthesis::seeds::SeedPlan>(&inputs)?;
    let invocation = typed::<analysis::synthesis::Invocation>(&inputs)?;
    let member = typed::<catalog::CatalogMemberInvocation>(&inputs)?;
    b.pair(
        brief,
        root_table,
        &[
            brief_table,
            seeds,
            plans,
            invocation,
            member,
            subjects,
            root_table,
        ],
        vec![
            eq(1, "id", 0, "seed"),
            eq(2, "id", 1, "plan"),
            eq(3, "id", 2, "invocation"),
            eq(4, "id", 1, "member"),
            eq(5, "member_member", 4, "member"),
            eq(6, "subject", 5, "id"),
            eq(6, "input", 3, "input"),
            eq(6, "context", 3, "context"),
        ],
        col(0, "id"),
        col(6, "id"),
    );
    b.finish()
}

fn scope_target(
    inputs: &[ValidationInput],
    source: usize,
    kind: TypeId,
) -> Result<Option<usize>, ModelError> {
    use TypeId;
    let candidates: Vec<_> = inputs
        .iter()
        .enumerate()
        .filter(|(_, input)| input.type_id() == kind)
        .map(|(index, _)| index)
        .collect();
    if candidates.len() <= 1 {
        return Ok(candidates.first().copied());
    }
    let analytic = [
        TypeId::of::<structural::Conclusion>(),
        TypeId::of::<structural::ConclusionSource>(),
        TypeId::of::<analytics::Conclusion>(),
        TypeId::of::<analytics::ConclusionSource>(),
        TypeId::of::<execution::summary_consequences::ClaimConclusion>(),
        TypeId::of::<execution::summary_consequences::ClaimProof>(),
        TypeId::of::<execution::summary_consequences::SummaryClaim>(),
        TypeId::of::<analysis::summary::AnalysisDerivation>(),
        TypeId::of::<analysis::summary::AnalysisProposition>(),
        TypeId::of::<analysis::summary::AnalysisDerivationPremise>(),
        TypeId::of::<execution::summary_terminal::SummaryTerminalWitness>(),
        TypeId::of::<execution::protocol_interpretation::ConditionalTerminalFrontier>(),
        TypeId::of::<execution::protocol_interpretation::NormalContinuationRestriction>(),
    ];
    let epoch =
        inputs[source]
            .prefix()
            .unwrap_or(if analytic.contains(&inputs[source].type_id()) {
                stages::PublicationBoundary::Analytic
            } else {
                stages::PublicationBoundary::Facts
            });
    let selected: Vec<_> = candidates
        .into_iter()
        .filter(|index| inputs[*index].prefix() == Some(epoch))
        .collect();
    if selected.len() != 1 {
        return Err(ModelError::Conflict("S0 exact immutable vocabulary"));
    }
    Ok(selected.first().copied())
}

fn pinned(predicates: &mut Vec<ScopePredicate>, column: ScopeColumn, index: usize, selected: bool) {
    if selected {
        predicates.push(ScopePredicate::ParameterIn(column, index));
    }
}
pub fn synthesis(
    inputs: Vec<ValidationInput>,
    relations: &[Relation],
    parents: Option<&[synthesis::frames::Parents]>,
    budget: &ResourceBudget,
) -> Result<BudgetedCatalogProgram, ModelError> {
    let link = typed::<catalog::CatalogMemberInvocation>(&inputs)?;
    let core = typed::<analysis::catalog_core::Invocation>(&inputs)?;
    let occurrence = typed::<source::Occurrence>(&inputs)?;
    let exposure = typed::<catalog::CatalogExposure>(&inputs)?;
    let public = typed::<normalized::entities::PublicExposure>(&inputs)?;
    let placement = typed::<syntax::SyntaxPlacement>(&inputs)?;
    let real = inputs.len();
    let member = real;
    let syntax = real + 1;
    let child = real + 2;
    let mut declarations = inputs.clone();
    declarations.extend([
        inputs[link].clone(),
        inputs[occurrence].clone(),
        inputs[placement].clone(),
    ]);
    let mut b = Builder::new(declarations, real, relations, budget)?;
    b.charge
        .grow(parents.map_or(0, |p| p.len().saturating_mul(96)))?;
    let memberships = synthesis::production::memberships();
    for (from, relation) in relations.iter().enumerate() {
        for field in relation.fields() {
            if let Some((kind, _)) = field.target()
                && let Some(to) = scope_target(&inputs, from, kind)?
            {
                b.follow(from, field.name(), to, field.list());
                if memberships.contains(&(relation.type_id(), field.name()))
                    || relation.type_id()
                        == TypeId::of::<analysis::native::NativeAssertionPremise>()
                    || relation.type_id() == TypeId::of::<analysis::summary::SupportSource>()
                {
                    b.program.rules.push(ScopeRule::Reference {
                        source: from,
                        field: field.name(),
                        target: to,
                        direction: ScopeDirection::OwnedReverse,
                        list: field.list(),
                    });
                }
            }
        }
    }
    b.pair(member, link, &[link], vec![], col(0, "id"), col(0, "id"));
    b.pair(
        member,
        exposure,
        &[link, core, exposure, public],
        vec![
            eq(1, "id", 0, "invocation"),
            eq(2, "member", 0, "member"),
            eq(3, "id", 2, "exposure"),
            eq(3, "context", 1, "context"),
        ],
        col(0, "id"),
        col(2, "id"),
    );
    let option = typed::<catalog::CatalogOption>(&inputs)?;
    b.pair(
        member,
        option,
        &[link, option],
        vec![eq(1, "member", 0, "member")],
        col(0, "id"),
        col(1, "id"),
    );
    let association = typed::<catalog::evidence::DocumentAssociation>(&inputs)?;
    let mention_candidate = typed::<normalized::links::MentionEntityCandidate>(&inputs)?;
    let assessment = typed::<normalized::links::MentionEntityAssessment>(&inputs)?;
    let mention = typed::<documents::DocumentMentionObservation>(&inputs)?;
    let q = scope_target(
        &inputs,
        mention,
        TypeId::of::<assertion::AssertionQualification>(),
    )?
    .ok_or(ModelError::Schema("S0 mention qualification"))?;
    b.pair(
        member,
        association,
        &[
            link,
            core,
            association,
            mention_candidate,
            assessment,
            mention,
            q,
        ],
        vec![
            eq(1, "id", 0, "invocation"),
            eq(2, "member", 0, "member"),
            eq(3, "id", 2, "candidate"),
            eq(4, "id", 3, "assessment"),
            eq(5, "id", 4, "observation"),
            eq(6, "id", 5, "qualification"),
            eq(6, "context", 1, "context"),
        ],
        col(0, "id"),
        col(2, "id"),
    );
    b.pair(
        occurrence,
        placement,
        &[placement],
        vec![],
        col(0, "occurrence"),
        col(0, "id"),
    );
    const STATEMENTS: &[i16] = &[
        source::SyntaxKind::StmtExpr as i16,
        source::SyntaxKind::StmtAssign as i16,
        source::SyntaxKind::StmtAnnAssign as i16,
        source::SyntaxKind::StmtImport as i16,
        source::SyntaxKind::StmtImportFrom as i16,
        source::SyntaxKind::StmtReturn as i16,
    ];
    b.pair(
        occurrence,
        syntax,
        &[occurrence],
        vec![ScopePredicate::CodeIn(col(0, "syntax_kind"), STATEMENTS)],
        col(0, "id"),
        col(0, "id"),
    );
    b.pair(
        syntax,
        occurrence,
        &[occurrence],
        vec![],
        col(0, "id"),
        col(0, "id"),
    );
    b.pair(
        syntax,
        child,
        &[placement],
        vec![ScopePredicate::IsNull(col(0, "parent"), false)],
        col(0, "parent"),
        col(0, "id"),
    );
    b.pair(
        child,
        placement,
        &[placement],
        vec![],
        col(0, "id"),
        col(0, "id"),
    );
    b.pair(
        child,
        syntax,
        &[placement],
        vec![],
        col(0, "id"),
        col(0, "occurrence"),
    );
    for to in [
        typed::<lexical::ReferenceObservation>(&inputs)?,
        typed::<lexical::LexicalResolution>(&inputs)?,
    ] {
        b.pair(occurrence, to, &[to], vec![], col(0, "read"), col(0, "id"));
    }
    let candidate = typed::<catalog::CatalogCandidate>(&inputs)?;
    let path = typed::<catalog::CatalogPath>(&inputs)?;
    let alias = typed::<catalog::CatalogAlias>(&inputs)?;
    let entity_candidate = typed::<normalized::entities::SymbolEntityCandidate>(&inputs)?;
    let sf = typed::<structural::StructuralFrame>(&inputs)?;
    let si = typed::<analysis::structural::Invocation>(&inputs)?;
    let af = typed::<analytics::AnalyticFrame>(&inputs)?;
    let selected_public = typed::<structural::PublicCandidate>(&inputs)?;
    let selected = parents.is_some();
    let mut predicates = vec![
        eq(1, "id", 0, "invocation"),
        eq(2, "member", 0, "member"),
        eq(3, "id", 2, "frame"),
        eq(4, "id", 3, "invocation"),
        eq(4, "input", 1, "input"),
        eq(4, "context", 1, "context"),
    ];
    pinned(&mut predicates, col(2, "frame"), 0, selected);
    b.pair(
        member,
        selected_public,
        &[link, core, selected_public, sf, si],
        predicates,
        col(0, "id"),
        col(2, "id"),
    );
    let claims = typed::<execution::summary_consequences::SummaryClaim>(&inputs)?;
    let transfers = typed::<transfer::summary::TransferKey>(&inputs)?;
    let events = typed::<normalized::events::NormalizedCallEvent>(&inputs)?;
    let ownership = typed::<normalized::entities::OccurrenceOwnership>(&inputs)?;
    let symbolic = typed::<execution::summary_symbolic::SymbolicFieldAlternative>(&inputs)?;
    let conclusion = typed::<execution::summary_consequences::ClaimConclusion>(&inputs)?;
    let subject = typed::<analysis::summary::ObligationSubject>(&inputs)?;
    let summary_inv = typed::<analysis::summary::Invocation>(&inputs)?;
    let witness = typed::<execution::summary_terminal::SummaryTerminalWitness>(&inputs)?;
    let frontier =
        typed::<execution::protocol_interpretation::ConditionalTerminalFrontier>(&inputs)?;
    // Three actual entity correspondence paths, each carrying the exact C0 input and context.
    for variant in 0..3 {
        let mut rows = vec![link, core, exposure, public];
        let mut predicates = vec![
            eq(1, "id", 0, "invocation"),
            eq(2, "member", 0, "member"),
            eq(3, "id", 2, "exposure"),
            eq(3, "context", 1, "context"),
        ];
        let entity = if variant == 1 {
            rows.push(alias);
            predicates.push(eq(4, "parent", 2, "id"));
            col(4, "entity")
        } else {
            rows.extend([
                candidate,
                if variant == 0 { path } else { entity_candidate },
            ]);
            predicates.extend([
                eq(4, "exposure", 2, "id"),
                eq(5, "id", 4, if variant == 0 { "path" } else { "entity" }),
            ]);
            col(5, "entity")
        };
        for (to, field) in [
            (typed::<structural::Conclusion>(&inputs)?, "subject"),
            (typed::<structural::handoffs::Group>(&inputs)?, "seed"),
        ] {
            let mut rs = rows.clone();
            let n = rs.len();
            rs.extend([to, sf, si]);
            let mut ps = predicates.clone();
            ps.extend([
                ScopePredicate::Equal(col(n, field), entity),
                eq(n + 1, "id", n, "frame"),
                eq(n + 2, "id", n + 1, "invocation"),
                eq(n + 2, "input", 1, "input"),
                eq(n + 2, "context", 1, "context"),
            ]);
            pinned(&mut ps, col(n, "frame"), 0, selected);
            b.pair(member, to, &rs, ps, col(0, "id"), col(n, "id"));
        }
        let ac = typed::<analytics::Conclusion>(&inputs)?;
        let mut rs = rows.clone();
        let n = rs.len();
        rs.extend([ac, af, sf, si]);
        let mut ps = predicates.clone();
        ps.extend([
            ScopePredicate::Equal(col(n, "subject"), entity),
            eq(n + 1, "id", n, "frame"),
            eq(n + 2, "id", n + 1, "structural"),
            eq(n + 3, "id", n + 2, "invocation"),
            eq(n + 3, "input", 1, "input"),
            eq(n + 3, "context", 1, "context"),
        ]);
        pinned(&mut ps, col(n, "frame"), 1, selected);
        b.pair(member, ac, &rs, ps, col(0, "id"), col(n, "id"));
        // Four typed SummaryClaim alternatives identify an owner; no generic operation language.
        for claim in 0..4 {
            let mut rs = rows.clone();
            let c = rs.len();
            rs.push(claims);
            let mut ps = predicates.clone();
            match claim {
                0 => ps.push(ScopePredicate::Equal(
                    col(c, "nonormalcontinuation_owner"),
                    entity,
                )),
                1 => {
                    rs.push(transfers);
                    ps.extend([
                        eq(c + 1, "id", c, "finitealternative_transfer"),
                        ScopePredicate::Equal(col(c + 1, "owner"), entity),
                    ]);
                }
                2 => {
                    rs.extend([events, ownership]);
                    ps.extend([
                        eq(c + 1, "id", c, "callclosure_event"),
                        eq(c + 2, "id", c + 1, "owner"),
                        ScopePredicate::Equal(col(c + 2, "entity"), entity),
                    ]);
                }
                _ => {
                    rs.push(symbolic);
                    ps.extend([
                        eq(c + 1, "id", c, "symbolicfieldassociation_alternative"),
                        ScopePredicate::Equal(col(c + 1, "constructor"), entity),
                    ]);
                }
            }
            let n = rs.len();
            rs.extend([subject, conclusion, summary_inv]);
            ps.extend([
                eq(n, "summaryclaim_transfer", c, "id"),
                eq(n + 1, "subject", n, "id"),
                eq(n + 2, "id", n + 1, "invocation"),
                eq(n + 2, "input", 1, "input"),
                eq(n + 2, "context", 1, "context"),
            ]);
            pinned(&mut ps, col(n + 1, "invocation"), 2, selected);
            b.pair(member, conclusion, &rs, ps, col(0, "id"), col(n + 1, "id"));
        }
        let mut rs = rows.clone();
        let n = rs.len();
        rs.extend([frontier, witness, summary_inv]);
        let mut ps = predicates.clone();
        ps.extend([
            ScopePredicate::Equal(col(n, "owner"), entity),
            eq(n + 1, "frontier", n, "id"),
            eq(n + 2, "id", n + 1, "invocation"),
            eq(n + 2, "input", 1, "input"),
            eq(n + 2, "context", 1, "context"),
        ]);
        pinned(&mut ps, col(n + 1, "invocation"), 2, selected);
        b.pair(member, witness, &rs, ps, col(0, "id"), col(n + 1, "id"));
    }
    let scenario = typed::<catalog::evidence::ScenarioAssociation>(&inputs)?;
    let q = scope_target(
        &inputs,
        scenario,
        TypeId::of::<assertion::AssertionQualification>(),
    )?
    .ok_or(ModelError::Schema("S0 scenario qualification"))?;
    b.pair(
        member,
        scenario,
        &[link, core, scenario, q],
        vec![
            eq(1, "id", 0, "invocation"),
            eq(2, "member", 0, "member"),
            eq(3, "id", 2, "qualification"),
            eq(3, "context", 1, "context"),
        ],
        col(0, "id"),
        col(2, "id"),
    );
    if let Some(observation) = inputs
        .iter()
        .position(|input| input.type_id() == TypeId::of::<flow::FlowUseObservation>())
    {
        let q = scope_target(
            &inputs,
            observation,
            TypeId::of::<assertion::AssertionQualification>(),
        )?
        .ok_or(ModelError::Schema("S0 native flow qualification"))?;
        let coverage = typed::<attribution::ProviderCoverage>(&inputs)?;
        b.pair(
            observation,
            coverage,
            &[observation, q, coverage],
            vec![
                eq(1, "id", 0, "qualification"),
                eq(2, "scope", 1, "scope"),
                eq(2, "context", 1, "context"),
                ScopePredicate::Code(col(2, "family"), attribution::FactFamily::Flow as i16),
            ],
            col(0, "id"),
            col(2, "id"),
        );
    }
    let mut result = b.finish()?;
    if let Some(parents) = parents {
        result._charge.grow(3usize.saturating_mul(size_of::<ScopeValue>())
            .saturating_add(parents.len().saturating_mul(3 * size_of::<[u8; 16]>())))?;
        result.parameters = ScopeParameters(vec![
            ScopeValue::Nominals(
                parents
                    .iter()
                    .map(|parent| *parent.structural.bytes())
                    .collect(),
            ),
            ScopeValue::Nominals(
                parents
                    .iter()
                    .map(|parent| *parent.analytic.bytes())
                    .collect(),
            ),
            ScopeValue::Nominals(
                parents
                    .iter()
                    .map(|parent| *parent.summary.bytes())
                    .collect(),
            ),
        ]);
    }
    Ok(result)
}
#[derive(Clone, Copy)]
pub enum SynthesisTextPhase {
    Documentary,
    Member,
    Conclusion,
}
pub struct SynthesisTextDemand {
    pub syntax_kinds: &'static [i16],
    pub document_nodes_only: bool,
}
pub fn synthesis_text(phase: SynthesisTextPhase) -> SynthesisTextDemand {
    const LITERALS: &[i16] = &[source::SyntaxKind::ExprStringLiteral as i16];
    const ALL: &[i16] = &[
        source::SyntaxKind::ExprStringLiteral as i16,
        source::SyntaxKind::StmtExpr as i16,
        source::SyntaxKind::StmtAssign as i16,
        source::SyntaxKind::StmtAnnAssign as i16,
        source::SyntaxKind::StmtImport as i16,
        source::SyntaxKind::StmtImportFrom as i16,
        source::SyntaxKind::StmtReturn as i16,
    ];
    let documentary = matches!(phase, SynthesisTextPhase::Documentary);
    SynthesisTextDemand {
        syntax_kinds: if documentary { LITERALS } else { ALL },
        document_nodes_only: documentary,
    }
}
#[derive(Clone, Copy)]
pub enum RetrievalArtifactRoot {
    Member,
    Document,
    Scenario,
    Deployment,
    Source,
    Empty,
    Brief,
}
impl RetrievalArtifactRoot {
    pub fn subject(subject: &catalog::evidence::RootSubject) -> Self {
        use catalog::evidence::RootSubject as R;
        match subject {
            R::Member { .. } => Self::Member,
            R::Document { .. } => Self::Document,
            R::Scenario { .. } => Self::Scenario,
            R::Deployment { .. } => Self::Deployment,
            R::Source { .. } => Self::Source,
            R::Option { .. } | R::Release { .. } => Self::Empty,
        }
    }
}
pub fn retrieval_artifacts(
    inputs: Vec<ValidationInput>,
    relations: &[Relation],
    kind: RetrievalArtifactRoot,
    budget: &ResourceBudget,
) -> Result<BudgetedCatalogProgram, ModelError> {
    use catalog::evidence as c1;
    let real = inputs.len();
    let artifacts = typed::<source::SourceArtifact>(&inputs)?;
    let roots = typed::<c1::EvidenceRoot>(&inputs)?;
    let subjects = typed::<c1::RootSubject>(&inputs)?;
    let mut b = Builder::new(inputs.clone(), real, relations, budget)?;
    let base = vec![roots, subjects];
    let predicates = vec![
        eq(1, "id", 0, "subject"),
        ScopePredicate::Parameter(col(0, "id"), 0),
    ];
    match kind {
        RetrievalArtifactRoot::Member => {
            let mut rows = base;
            rows.extend([
                typed::<catalog::CatalogMember>(&inputs)?,
                typed::<source::Module>(&inputs)?,
            ]);
            let mut ps = predicates;
            ps.extend([eq(2, "id", 1, "member_member"), eq(3, "id", 2, "access")]);
            b.pair(roots, artifacts, &rows, ps, col(0, "id"), col(3, "source"));
        }
        RetrievalArtifactRoot::Document => {
            let mut rows = base;
            rows.push(typed::<documents::DocumentObservation>(&inputs)?);
            let mut ps = predicates;
            ps.push(eq(2, "id", 1, "document_observation"));
            b.pair(roots, artifacts, &rows, ps, col(0, "id"), col(2, "source"));
        }
        RetrievalArtifactRoot::Source => b.pair(
            roots,
            artifacts,
            &base,
            predicates,
            col(0, "id"),
            col(1, "source_artifact"),
        ),
        RetrievalArtifactRoot::Deployment => {
            let mut rows = base;
            rows.extend([
                typed::<c1::CatalogDeployment>(&inputs)?,
                typed::<deployment::DeploymentObservation>(&inputs)?,
                typed::<assertion::Evidence>(&inputs)?,
            ]);
            let mut ps = predicates;
            ps.extend([
                eq(2, "id", 1, "deployment_deployment"),
                eq(3, "id", 2, "observation"),
                eq(4, "id", 3, "span"),
            ]);
            b.pair(
                roots,
                artifacts,
                &rows,
                ps,
                col(0, "id"),
                col(4, "sourcespan_source"),
            );
        }
        RetrievalArtifactRoot::Scenario => {
            let mut rows = base;
            rows.extend([
                typed::<c1::ScenarioSpan>(&inputs)?,
                typed::<c1::OriginalSource>(&inputs)?,
                typed::<source::Occurrence>(&inputs)?,
                typed::<assertion::Evidence>(&inputs)?,
            ]);
            let mut ps = predicates;
            ps.extend([
                eq(2, "scenario", 1, "scenario_scenario"),
                eq(3, "id", 2, "source"),
            ]);
            let optional = vec![
                ScopeOptionalJoin {
                    row: 4,
                    keys: vec![(col(4, "id"), col(3, "occurrence_occurrence"))],
                },
                ScopeOptionalJoin {
                    row: 5,
                    keys: vec![(col(5, "id"), col(3, "span_span"))],
                },
            ];
            for variant in 0..3 {
                let mut predicates = ps.clone();
                let key = if variant == 0 {
                    predicates.push(ScopePredicate::IsNull(col(3, "artifact_artifact"), false));
                    col(3, "artifact_artifact")
                } else {
                    predicates.push(ScopePredicate::IsNull(col(3, "artifact_artifact"), true));
                    if variant == 1 {
                        predicates.push(ScopePredicate::IsNull(col(4, "source"), false));
                        col(4, "source")
                    } else {
                        predicates.push(ScopePredicate::IsNull(col(4, "source"), true));
                        col(5, "sourcespan_source")
                    }
                };
                b.program.rules.push(ScopeRule::OptionalPairs {
                    source: roots,
                    target: artifacts,
                    rows: rows.clone(),
                    optional: optional.clone(),
                    predicates,
                    source_key: col(0, "id"),
                    target_key: key,
                });
            }
        }
        RetrievalArtifactRoot::Brief => {
            let brief = typed::<synthesis::briefs::Brief>(&inputs)?;
            let rows = vec![
                typed::<synthesis::briefs::BriefSource>(&inputs)?,
                typed::<synthesis::documentary::DocumentaryConclusion>(&inputs)?,
                typed::<synthesis::documentary::ProseSlice>(&inputs)?,
                typed::<synthesis::documentary::ProseSource>(&inputs)?,
                typed::<source::Occurrence>(&inputs)?,
                typed::<assertion::Evidence>(&inputs)?,
            ];
            for literal in [true, false] {
                for occurrence in [true, false] {
                    let mut predicates = vec![
                        eq(1, "id", 0, "documentary"),
                        eq(2, "id", 1, "prose"),
                        eq(3, "id", 2, "source"),
                        ScopePredicate::Parameter(col(0, "brief"), 0),
                        ScopePredicate::IsNull(col(3, "literal_occurrence"), !literal),
                        ScopePredicate::IsNull(col(4, "source"), !occurrence),
                    ];
                    let key = if occurrence {
                        col(4, "source")
                    } else {
                        col(5, "sourcespan_source")
                    };
                    // COALESCE chooses literal identity first even when its optional occurrence row is absent.
                    b.program.rules.push(ScopeRule::OptionalPairs {
                        source: brief,
                        target: artifacts,
                        rows: rows.clone(),
                        optional: vec![
                            ScopeOptionalJoin {
                                row: 4,
                                keys: vec![(
                                    col(4, "id"),
                                    col(
                                        3,
                                        if literal {
                                            "literal_occurrence"
                                        } else {
                                            "occurrence_occurrence"
                                        },
                                    ),
                                )],
                            },
                            ScopeOptionalJoin {
                                row: 5,
                                keys: vec![(col(5, "id"), col(3, "span_span"))],
                            },
                        ],
                        predicates: std::mem::take(&mut predicates),
                        source_key: col(0, "brief"),
                        target_key: key,
                    });
                }
            }
        }
        RetrievalArtifactRoot::Empty => {}
    }
    b.finish()
}
/// Exact E0 render inventories, parameterized by invocation input/context (slots 0/1).
#[derive(Clone, Copy)]
pub enum RetrievalInventory {
    Roots,
    Briefs,
}
pub fn retrieval_inventory(
    inputs: Vec<ValidationInput>,
    relations: &[Relation],
    kind: RetrievalInventory,
    budget: &ResourceBudget,
) -> Result<BudgetedCatalogProgram, ModelError> {
    let mut b = Builder::new(inputs.clone(), inputs.len(), relations, budget)?;
    match kind {
        RetrievalInventory::Roots => {
            let roots = typed::<catalog::evidence::EvidenceRoot>(&inputs)?;
            b.pair(
                roots,
                roots,
                &[roots],
                vec![
                    ScopePredicate::Parameter(col(0, "input"), 0),
                    ScopePredicate::Parameter(col(0, "context"), 1),
                ],
                col(0, "id"),
                col(0, "id"),
            );
        }
        RetrievalInventory::Briefs => {
            let briefs = typed::<synthesis::briefs::Brief>(&inputs)?;
            let seeds = typed::<synthesis::seeds::SelectedSeed>(&inputs)?;
            let plans = typed::<synthesis::seeds::SeedPlan>(&inputs)?;
            let invocations = typed::<analysis::synthesis::Invocation>(&inputs)?;
            b.pair(
                briefs,
                briefs,
                &[briefs, seeds, plans, invocations],
                vec![
                    eq(1, "id", 0, "seed"),
                    eq(2, "id", 1, "plan"),
                    eq(3, "id", 2, "invocation"),
                    ScopePredicate::Parameter(col(3, "input"), 0),
                    ScopePredicate::Parameter(col(3, "context"), 1),
                ],
                col(0, "id"),
                col(0, "id"),
            );
        }
    }
    b.finish()
}
#[cfg(test)]
mod controls {
    use super::*;
    fn relations(inputs: &[ValidationInput], model: &ValidatedModel) -> Vec<Relation> {
        inputs
            .iter()
            .map(|input| model.relation(input.name()).unwrap().clone())
            .collect()
    }
    #[test]
    fn finished_factory_releases_construction_scratch_but_keeps_nested_metadata() {
        let budget = ResourceBudget::fixed(4 << 20).unwrap();
        let inputs = vec![ValidationInput::of::<input::Package>(&["id", "name"])];
        let mut builder = Builder::new(inputs, 1, &[Relation::of::<input::Package>()], &budget).unwrap();
        builder.reserve_rules(2 << 20).unwrap();
        // A selector wrapper owns additional metadata outside the scope program.
        builder.reserve_retained(8192).unwrap();
        let mut keys = Vec::with_capacity(64);
        keys.push((col(0, "id"), col(1, "id")));
        builder.program.rules.push(ScopeRule::OptionalPairs {
            source: 0, target: 0, rows: vec![0, 0],
            optional: vec![ScopeOptionalJoin { row: 1, keys }],
            predicates: vec![ScopePredicate::IsNull(col(1, "id"), true)],
            source_key: col(0, "id"), target_key: col(0, "id"),
        });
        let construction = budget.reserved();
        assert!(construction > 2 << 20);
        let program = builder.finish().unwrap();
        assert!(budget.reserved() < construction);
        assert!(budget.reserved() >= 8192 + 64 * size_of::<(ScopeColumn, ScopeColumn)>());
        let ScopeRule::OptionalPairs { optional, predicates, .. } = &program.program().rules[0] else { panic!("optional scope lost") };
        assert_eq!(optional[0].keys, vec![(col(0, "id"), col(1, "id"))]);
        assert!(matches!(predicates[0], ScopePredicate::IsNull(_, true)));
        // Required execution can use the released arena while the finished factory lives.
        let working = budget.reserve("factory-lifecycle-control", 3 << 20).unwrap();
        drop(working);
        drop(program);
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn catalog_families_validate_and_release_owned_program_metadata() {
        let owner = super::super::model().unwrap();
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        let inputs = catalog::build::CatalogData::validation_inputs();
        let program = core(inputs.clone(), &relations(&inputs, &owner), &owner, &budget).unwrap();
        program.program().validate(&owner).unwrap();
        assert!(budget.reserved() > 0);
        assert_eq!(
            program
                .program()
                .rules
                .iter()
                .filter(|rule| matches!(rule, ScopeRule::OptionalPairs { .. }))
                .count(),
            2
        );
        drop(program);
        assert_eq!(budget.reserved(), 0);
        let inputs = catalog::evidence::build::EvidenceData::inputs();
        let program = evidence(inputs.clone(), &relations(&inputs, &owner), &budget).unwrap();
        program.program().validate(&owner).unwrap();
        drop(program);
        assert_eq!(budget.reserved(), 0);
        let inputs = selection::build::Data::inputs();
        let program = selection(inputs.clone(), &relations(&inputs, &owner), &budget).unwrap();
        program.program().validate(&owner).unwrap();
        drop(program);
        assert_eq!(budget.reserved(), 0);
        let inputs = retrieval::build::Data::inputs();
        let program =
            retrieval(inputs.clone(), &relations(&inputs, &owner), &owner, &budget).unwrap();
        program.program().validate(&owner).unwrap();
        drop(program);
        assert_eq!(budget.reserved(), 0);
        for kind in [
            RetrievalArtifactRoot::Member,
            RetrievalArtifactRoot::Document,
            RetrievalArtifactRoot::Scenario,
            RetrievalArtifactRoot::Deployment,
            RetrievalArtifactRoot::Source,
            RetrievalArtifactRoot::Empty,
            RetrievalArtifactRoot::Brief,
        ] {
            let program =
                retrieval_artifacts(inputs.clone(), &relations(&inputs, &owner), kind, &budget)
                    .unwrap();
            program.program().validate(&owner).unwrap();
            drop(program);
            assert_eq!(budget.reserved(), 0);
        }
    }
    #[test]
    fn synthesis_pins_are_external_and_empty_selected_domain_is_not_unrestricted() {
        let owner = super::super::model().unwrap();
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        let inputs = synthesis::production::Data::inputs(stages::Profile::Behavioral);
        let rs = relations(&inputs, &owner);
        let whole = synthesis(inputs.clone(), &rs, None, &budget).unwrap();
        whole.program().validate(&owner).unwrap();
        assert!(whole.parameters().0.is_empty());
        assert!(!whole.program().rules.iter().any(|r|matches!(r,ScopeRule::Pairs{predicates,..} if predicates.iter().any(|p|matches!(p,ScopePredicate::ParameterIn(..))))));
        drop(whole);
        let selected = synthesis(inputs, &rs, Some(&[]), &budget).unwrap();
        selected.program().validate(&owner).unwrap();
        assert_eq!(selected.parameters().0.len(), 3);
        assert!(
            selected
                .parameters()
                .0
                .iter()
                .all(|value| matches!(value,ScopeValue::Nominals(keys) if keys.is_empty()))
        );
        assert!(selected.program().rules.iter().any(|r|matches!(r,ScopeRule::Pairs{predicates,..} if predicates.iter().any(|p|matches!(p,ScopePredicate::ParameterIn(..))))));
        drop(selected);
        assert_eq!(budget.reserved(), 0);
        let documentary = synthesis_text(SynthesisTextPhase::Documentary);
        assert_eq!(
            documentary.syntax_kinds,
            &[source::SyntaxKind::ExprStringLiteral as i16]
        );
        assert!(documentary.document_nodes_only);
        assert!(!synthesis_text(SynthesisTextPhase::Member).document_nodes_only);
    }
}
