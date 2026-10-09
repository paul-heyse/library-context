//! Complete callable-metadata selection meaning. Physical selectors lower these declarations.
use super::{
    callable_aspects::{AspectScope, CallableAspect, FieldDefaultAssessment},
    callables::EffectiveDecoratorMember,
    entities::CallableEntity,
    symbolic_fields::{
        SourceFieldAssociation, SourceFieldClass, SourceFieldReader, SourceFieldReaderLink,
        SourceFieldStore,
    },
};
use crate::domain::{
    assertion::AssertionQualification,
    attribution::{AnalysisContext, ProviderCoverage},
    resources::{Reservation, ResourceBudget},
    scope_program::{
        ScopeColumn, ScopeDirection, ScopePort, ScopePredicate, ScopeProgram, ScopeRule,
        forward_rules, input_binding,
    },
    source::Occurrence,
    syntax::{ClassFieldSyntaxObservation, DeclarationDecorator, DeclarationKind},
    *,
};
use std::any::TypeId;

pub struct AspectProgram {
    pub program: ScopeProgram,
    pub roots: [usize; 3],
    pub admission_roots: Vec<usize>,
    _construction: Box<dyn Reservation>,
}
fn column(row: usize, field: &'static str) -> ScopeColumn {
    ScopeColumn { row, field }
}
fn equal(left: (usize, &'static str), right: (usize, &'static str)) -> ScopePredicate {
    ScopePredicate::Equal(column(left.0, left.1), column(right.0, right.1))
}
fn pairs(
    source: usize,
    target: usize,
    rows: Vec<usize>,
    predicates: Vec<ScopePredicate>,
    source_key: (usize, &'static str),
    target_key: (usize, &'static str),
) -> ScopeRule {
    ScopeRule::Pairs {
        source,
        target,
        rows,
        predicates,
        source_key: column(source_key.0, source_key.1),
        target_key: column(target_key.0, target_key.1),
    }
}
fn optional_type(inputs: &[ValidationInput], kind: TypeId) -> Result<Option<usize>, ModelError> {
    let mut found = inputs
        .iter()
        .enumerate()
        .filter(|(_, input)| input.type_id() == kind);
    let first = found.next().map(|(index, _)| index);
    if found.next().is_some() {
        return Err(ModelError::Conflict("aspect scope immutable binding"));
    }
    Ok(first)
}
fn typed<R: Record>(inputs: &[ValidationInput]) -> Result<usize, ModelError> {
    optional_type(inputs, TypeId::of::<R>())?
        .ok_or(ModelError::Schema("aspect scope relation absent"))
}
fn bound(
    inputs: &[ValidationInput],
    expected: &ValidationInput,
) -> Result<Option<usize>, ModelError> {
    if !inputs
        .iter()
        .any(|input| input.type_id() == expected.type_id() && input.prefix() == expected.prefix())
    {
        return Ok(None);
    }
    input_binding(inputs, expected).map(Some)
}
/// Owned reverse membership comes from the same aspect/support contracts for every lowering.
pub(crate) fn owned_memberships(
    scope: &AspectScope,
    model: &ValidatedModel,
    budget: &ResourceBudget,
) -> Vec<(ValidationInput, &'static str, ValidationInput)> {
    let mut memberships = scope.memberships.clone();
    for invariant in model.invariants() {
        if let Some(support) = (invariant.create)(budget).support_scope() {
            memberships.push((support.support, "assertion", support.assertion));
        }
    }
    memberships
}
/// Coverage is attached to both qualification scope and analysis context.
const COVERAGE_OWNER_FIELDS: [&str; 2] = ["scope", "context"];

/// All physical and inventory scopes compose these same dependencies, owned memberships and
/// exact contextual coverage. Adding a rule here reaches both realizations.
fn dependency_rules(
    scope: &AspectScope,
    inputs: &[ValidationInput],
    model: &ValidatedModel,
    budget: &ResourceBudget,
    body: usize,
) -> Result<Vec<ScopeRule>, ModelError> {
    let qualification = typed::<AssertionQualification>(inputs)?;
    let mut rules = forward_rules(inputs, model)?;
    let mut body_members = Vec::new();
    for rule in &rules {
        if let ScopeRule::Reference {
            source,
            field,
            target,
            list: false,
            ..
        } = rule
            && inputs[*target].type_id() == TypeId::of::<Occurrence>()
        {
            body_members.push(ScopeRule::Reference {
                source: *source,
                field,
                target: body,
                direction: ScopeDirection::OwnedReverse,
                list: false,
            });
        }
    }
    rules.extend(body_members);
    for (member, field, owner) in owned_memberships(scope, model, budget) {
        if let (Some(member), Some(owner)) = (bound(inputs, &member)?, bound(inputs, &owner)?) {
            rules.push(ScopeRule::Reference {
                source: member,
                field,
                target: owner,
                direction: ScopeDirection::OwnedReverse,
                list: false,
            });
        }
    }
    if let Some(coverage) = optional_type(inputs, TypeId::of::<ProviderCoverage>())? {
        rules.push(pairs(
            qualification,
            coverage,
            vec![qualification, coverage],
            COVERAGE_OWNER_FIELDS
                .into_iter()
                .map(|field| equal((0, field), (1, field)))
                .collect(),
            (0, "id"),
            (1, "id"),
        ));
    }
    Ok(rules)
}
fn construction(
    inputs: &[ValidationInput],
    scope: &AspectScope,
    model: &ValidatedModel,
    budget: &ResourceBudget,
) -> Result<Box<dyn Reservation>, ModelError> {
    let field_count = inputs.iter().try_fold(0usize, |count, input| {
        let relation = model
            .relation(input.name())
            .ok_or(ModelError::Schema("aspect scope relation absent"))?;
        count
            .checked_add(relation.fields().len())
            .ok_or(ModelError::Schema("aspect program size overflow"))
    })?;
    let allowance = field_count
        .checked_mul(2048)
        .and_then(|bytes| bytes.checked_add(inputs.len().checked_mul(512)?))
        .and_then(|bytes| bytes.checked_add(model.invariants().len().checked_mul(1024)?))
        .and_then(|bytes| bytes.checked_add(scope.memberships.len().checked_mul(1024)?))
        .and_then(|bytes| bytes.checked_add(65536))
        .ok_or(ModelError::Schema("aspect program size overflow"))?;
    budget.reserve("aspect-scope-program-construction", allowance)
}
pub struct ClassInventoryProgram {
    pub program: ScopeProgram,
    pub class_root: usize,
    pub symbol_root: usize,
    pub declaration_root: usize,
    _construction: Box<dyn Reservation>,
}
/// Inventory has its own owner parameters. References to supporting classes cannot acquire
/// another body, while the class symbol and exact contextual declarations are explicit roots.
pub fn class_inventory_program(
    inputs: Vec<ValidationInput>,
    model: &ValidatedModel,
    budget: &ResourceBudget,
) -> Result<ClassInventoryProgram, ModelError> {
    let scope = super::callable_aspects::aspect_scope();
    let construction = construction(&inputs, &scope, model, budget)?;
    let occurrence = typed::<Occurrence>(&inputs)?;
    let field = typed::<super::entities::FieldDeclarationLink>(&inputs)?;
    let syntax = typed::<ClassFieldSyntaxObservation>(&inputs)?;
    let symbol_root = typed::<crate::domain::calls::ProviderSymbol>(&inputs)?;
    let declaration_root = typed::<crate::domain::syntax::DeclarationObservation>(&inputs)?;
    let mut ports = inputs
        .iter()
        .enumerate()
        .map(|(input, _)| ScopePort {
            input,
            virtual_owner: false,
        })
        .collect::<Vec<_>>();
    let class_root = ports.len();
    ports.push(ScopePort {
        input: occurrence,
        virtual_owner: true,
    });
    let body = ports.len();
    ports.push(ScopePort {
        input: occurrence,
        virtual_owner: true,
    });
    let mut rules = dependency_rules(&scope, &inputs, model, budget, body)?;
    rules.push(pairs(
        class_root,
        occurrence,
        vec![occurrence],
        vec![],
        (0, "id"),
        (0, "id"),
    ));
    rules.push(pairs(
        body,
        occurrence,
        vec![occurrence],
        vec![],
        (0, "id"),
        (0, "id"),
    ));
    rules.push(pairs(
        class_root,
        body,
        vec![occurrence, occurrence],
        vec![ScopePredicate::BodyContains {
            parent: 0,
            child: 1,
        }],
        (0, "id"),
        (1, "id"),
    ));
    rules.push(pairs(
        class_root,
        field,
        vec![occurrence, syntax, field],
        vec![
            equal((0, "id"), (1, "class")),
            equal((1, "id"), (2, "declaration")),
        ],
        (0, "id"),
        (2, "id"),
    ));
    Ok(ClassInventoryProgram {
        program: ScopeProgram {
            inputs,
            ports,
            rules,
        },
        class_root,
        symbol_root,
        declaration_root,
        _construction: construction,
    })
}
pub fn class_declaration_owner(
    declaration: &crate::domain::syntax::DeclarationObservation,
    qualification: Option<&AssertionQualification>,
    class: Id<Occurrence>,
    context: Id<AnalysisContext>,
) -> bool {
    declaration.declaration == class
        && declaration.kind == DeclarationKind::Class
        && qualification.is_some_and(|q| q.context == context)
}

pub fn build(
    scope: &AspectScope,
    inputs: Vec<ValidationInput>,
    model: &ValidatedModel,
    budget: &ResourceBudget,
) -> Result<AspectProgram, ModelError> {
    let construction = construction(&inputs, scope, model, budget)?;
    let mut real_roots = [0; 3];
    for (index, root) in scope.roots.iter().enumerate() {
        real_roots[index] =
            bound(&inputs, root)?.ok_or(ModelError::Conflict("aspect root immutable epoch"))?;
    }
    let occurrence = typed::<Occurrence>(&inputs)?;
    let [assessment, field, declaration] = real_roots;
    let callable = typed::<CallableEntity>(&inputs)?;
    let field_syntax = typed::<ClassFieldSyntaxObservation>(&inputs)?;
    let member = typed::<EffectiveDecoratorMember>(&inputs)?;
    let decorator = typed::<DeclarationDecorator>(&inputs)?;
    let mut ports = inputs
        .iter()
        .enumerate()
        .map(|(input, _)| ScopePort {
            input,
            virtual_owner: false,
        })
        .collect::<Vec<_>>();
    let roots = std::array::from_fn(|index| {
        let port = ports.len();
        ports.push(ScopePort {
            input: real_roots[index],
            virtual_owner: true,
        });
        port
    });
    let body = ports.len();
    ports.push(ScopePort {
        input: occurrence,
        virtual_owner: true,
    });
    let mut rules = Vec::new();
    for (root, real) in roots.iter().zip(real_roots) {
        rules.push(pairs(*root, real, vec![real], vec![], (0, "id"), (0, "id")));
    }
    rules.push(pairs(
        body,
        occurrence,
        vec![occurrence],
        vec![],
        (0, "id"),
        (0, "id"),
    ));
    rules.push(pairs(
        roots[0],
        body,
        vec![assessment, callable, occurrence, occurrence],
        vec![
            equal((0, "callable"), (1, "id")),
            equal((1, "source_declaration"), (2, "id")),
            ScopePredicate::BodyContains {
                parent: 2,
                child: 3,
            },
        ],
        (0, "id"),
        (3, "id"),
    ));
    rules.push(pairs(
        roots[0],
        body,
        vec![member, decorator, occurrence, occurrence],
        vec![
            equal((0, "observation"), (1, "id")),
            equal((1, "decorator"), (2, "id")),
            ScopePredicate::BodyContains {
                parent: 2,
                child: 3,
            },
        ],
        (0, "assessment"),
        (3, "id"),
    ));
    for anchor in ["value", "target", "annotation"] {
        rules.push(pairs(
            roots[1],
            body,
            vec![field, field_syntax, occurrence, occurrence],
            vec![
                equal((0, "declaration"), (1, "id")),
                equal((1, anchor), (2, "id")),
                ScopePredicate::BodyContains {
                    parent: 2,
                    child: 3,
                },
            ],
            (0, "id"),
            (3, "id"),
        ));
    }
    rules.push(pairs(
        roots[2],
        body,
        vec![declaration, occurrence, occurrence],
        vec![
            ScopePredicate::Code(column(0, "kind"), DeclarationKind::Class as i16),
            equal((0, "declaration"), (1, "id")),
            ScopePredicate::BodyContains {
                parent: 1,
                child: 2,
            },
        ],
        (0, "id"),
        (2, "id"),
    ));
    rules.push(pairs(
        roots[2],
        field,
        vec![declaration, field_syntax, field],
        vec![
            ScopePredicate::Code(column(0, "kind"), DeclarationKind::Class as i16),
            equal((1, "class"), (0, "declaration")),
            equal((2, "declaration"), (1, "id")),
        ],
        (0, "id"),
        (2, "id"),
    ));
    rules.extend(dependency_rules(scope, &inputs, model, budget, body)?);
    let optional = |kind| optional_type(&inputs, kind);
    if let Some(index) = optional(TypeId::of::<CallableAspect>())? {
        rules.push(pairs(
            index,
            roots[0],
            vec![index],
            vec![],
            (0, "id"),
            (0, "assessment"),
        ));
    }
    if let Some(index) = optional(TypeId::of::<FieldDefaultAssessment>())? {
        rules.push(pairs(
            index,
            roots[1],
            vec![index],
            vec![],
            (0, "id"),
            (0, "declaration"),
        ));
    }
    for kind in [
        TypeId::of::<SourceFieldClass>(),
        TypeId::of::<SourceFieldStore>(),
        TypeId::of::<SourceFieldReader>(),
    ] {
        if let Some(index) = optional(kind)? {
            rules.push(pairs(
                index,
                roots[2],
                vec![index, declaration],
                vec![
                    equal((1, "declaration"), (0, "class")),
                    ScopePredicate::Code(column(1, "kind"), DeclarationKind::Class as i16),
                ],
                (0, "id"),
                (1, "id"),
            ));
        }
    }
    if let (Some(index), Some(class)) = (
        optional(TypeId::of::<SourceFieldAssociation>())?,
        optional(TypeId::of::<SourceFieldClass>())?,
    ) {
        rules.push(pairs(
            index,
            roots[2],
            vec![index, class, declaration],
            vec![
                equal((1, "id"), (0, "class")),
                equal((2, "declaration"), (1, "class")),
                ScopePredicate::Code(column(2, "kind"), DeclarationKind::Class as i16),
            ],
            (0, "id"),
            (2, "id"),
        ));
    }
    if let (Some(index), Some(association), Some(class)) = (
        optional(TypeId::of::<SourceFieldReaderLink>())?,
        optional(TypeId::of::<SourceFieldAssociation>())?,
        optional(TypeId::of::<SourceFieldClass>())?,
    ) {
        rules.push(pairs(
            index,
            roots[2],
            vec![index, association, class, declaration],
            vec![
                equal((1, "id"), (0, "association")),
                equal((2, "id"), (1, "class")),
                equal((3, "declaration"), (2, "class")),
                ScopePredicate::Code(column(3, "kind"), DeclarationKind::Class as i16),
            ],
            (0, "id"),
            (3, "id"),
        ));
    }
    let mut admission_roots = Vec::new();
    for root in &scope.admission_roots {
        if let Some(index) = bound(&inputs, root)? {
            admission_roots.push(index);
        }
    }
    Ok(AspectProgram {
        program: ScopeProgram {
            inputs,
            ports,
            rules,
        },
        roots,
        admission_roots,
        _construction: construction,
    })
}

/// Eligible production/admission owners are part of the same model operation. Non-class
/// declarations are dependencies, but never class-initializer roots.
pub fn root_inventory(
    scope: &AspectScope,
    inputs: Vec<ValidationInput>,
    index: usize,
) -> Result<crate::domain::scope_program::ScopeProgram, ModelError> {
    let expected = scope
        .roots
        .get(index)
        .ok_or(ModelError::Schema("aspect root operation"))?;
    let root = input_binding(&inputs, expected)?;
    let predicates =
        if expected.type_id() == TypeId::of::<crate::domain::syntax::DeclarationObservation>() {
            vec![ScopePredicate::Code(
                column(0, "kind"),
                DeclarationKind::Class as i16,
            )]
        } else {
            vec![]
        };
    let ports = (0..inputs.len())
        .map(|input| ScopePort {
            input,
            virtual_owner: false,
        })
        .collect();
    Ok(ScopeProgram {
        inputs,
        ports,
        rules: vec![pairs(
            root,
            root,
            vec![root],
            predicates,
            (0, "id"),
            (0, "id"),
        )],
    })
}

#[cfg(test)]
mod controls {
    use super::*;
    use crate::domain::normalized::callable_aspects::{AspectOutput, aspect_scope, scoped_inputs};
    #[test]
    fn aspect_program_declares_complete_virtual_owners_and_contextual_coverage() {
        let budget = ResourceBudget::fixed(32 << 20).unwrap();
        let model = crate::domain::model().unwrap();
        let scope = aspect_scope();
        let mut inputs = scoped_inputs();
        inputs.extend(AspectOutput::inputs());
        let original = inputs
            .iter()
            .map(|input| (input.name(), input.prefix(), input.order().to_vec()))
            .collect::<Vec<_>>();
        let compiled = scope.program(inputs, &model, &budget).unwrap();
        compiled.program.validate(&model).unwrap();
        assert_eq!(
            original,
            compiled
                .program
                .inputs
                .iter()
                .map(|input| (input.name(), input.prefix(), input.order().to_vec()))
                .collect::<Vec<_>>()
        );
        for (root, input) in compiled.roots.iter().zip(&scope.roots) {
            let port = compiled.program.ports[*root];
            assert!(port.virtual_owner);
            assert_eq!(compiled.program.inputs[port.input].name(), input.name());
        }
        assert_eq!(compiled.admission_roots.len(), 7);
        assert_eq!(
            compiled
                .program
                .ports
                .iter()
                .filter(|port| port.virtual_owner)
                .count(),
            4
        );
        let qualification = typed::<AssertionQualification>(&compiled.program.inputs).unwrap();
        let coverage = typed::<ProviderCoverage>(&compiled.program.inputs).unwrap();
        assert!(compiled.program.rules.iter().any(|rule|matches!(rule,
            ScopeRule::Pairs {source,target,predicates,..} if *source==qualification && *target==coverage && *predicates==vec![equal((0,"scope"),(1,"scope")),equal((0,"context"),(1,"context"))]
        )));
        let body_rules=compiled.program.rules.iter().filter(|rule|matches!(rule,ScopeRule::Pairs {predicates,..} if predicates.iter().any(|predicate|matches!(predicate,ScopePredicate::BodyContains {..})))).count();
        assert_eq!(body_rules, 6);
        assert!(budget.reserved() > 0);
        drop(compiled);
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn aspect_program_refuses_missing_ambiguous_and_unfunded_bindings() {
        let budget = ResourceBudget::fixed(32 << 20).unwrap();
        let model = crate::domain::model().unwrap();
        let scope = aspect_scope();
        let mut missing = scoped_inputs();
        missing.retain(|input| input.type_id() != TypeId::of::<Occurrence>());
        assert!(scope.program(missing, &model, &budget).is_err());
        let mut ambiguous = scoped_inputs();
        ambiguous.push(ValidationInput::of::<Occurrence>(&["id"]));
        assert!(scope.program(ambiguous, &model, &budget).is_err());
        let tiny = ResourceBudget::fixed(1).unwrap();
        assert!(matches!(
            scope.program(scoped_inputs(), &model, &tiny),
            Err(ModelError::Resource { .. })
        ));
        assert_eq!(tiny.reserved(), 0);
        assert_eq!(budget.reserved(), 0);
    }
}
