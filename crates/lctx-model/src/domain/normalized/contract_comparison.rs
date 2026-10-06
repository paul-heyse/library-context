//! Retained callable structure comparison. This operation is neither assignability nor
//! runtime applicability; Source/Stub remain the sole binder's admitted roles.
use crate::domain::{
    calls::*,
    catalog::*,
    normalized::callables::*,
    resources::{Reservation, ResourceBudget},
    selection::{build::need, classification::ClassificationData},
    types::*,
    *,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum Difference {
    Same {},
    DifferentRetainedStructure {},
    Unresolved { reason: String },
}
fn difference<T: PartialEq>(a: &T, b: &T) -> Difference {
    if a == b {
        Difference::Same {}
    } else {
        Difference::DifferentRetainedStructure {}
    }
}
fn unresolved(reason: &str) -> Difference {
    Difference::Unresolved {
        reason: reason.into(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PortComparison {
    pub ordinal: i64,
    pub left: Option<Id<SignatureParameter>>,
    pub right: Option<Id<SignatureParameter>>,
    /// Alignment is positional layout only; it never manufactures a source formal.
    pub layout: Difference,
    pub formal_identity: Difference,
    pub defaults: Difference,
    pub types: Difference,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ContractComparison {
    pub member: Id<CatalogMember>,
    pub analysis: Id<attribution::AnalysisContext>,
    pub left: Id<SignatureVariant>,
    pub right: Id<SignatureVariant>,
    pub left_role: SignatureRole,
    pub right_role: SignatureRole,
    pub form: Difference,
    pub adjustment: Difference,
    pub ports: Vec<PortComparison>,
    pub returns: Difference,
    pub proof: Vec<crate::domain::serving::ProofReference>,
}
/// Charge lives with the complete result, including before pagination and byte trimming.
pub struct ChargedResult<T> {
    pub value: T,
    pub(crate) _charge: Box<dyn Reservation>,
}

/// Only closed retained structure can establish sameness/difference. Inference variables,
/// Any, recursive alias references, unavailable callable slots and opaque leaves remain open.
pub fn closed_term(d: &ClassificationData, root: Id<TypeTerm>) -> bool {
    let mut pending = vec![root];
    let mut visited = std::collections::BTreeSet::new();
    while let Some(id) = pending.pop() {
        if visited.contains(&id) {
            continue;
        }
        if visited.len() >= 4096 {
            return false;
        }
        visited.insert(id);
        let Some(term) = d.facts.type_terms.get(id) else {
            return false;
        };
        let mut sequence = None;
        let mut list = None;
        match term {
            TypeTerm::Any { .. }
            | TypeTerm::Other { .. }
            | TypeTerm::Truncated { .. }
            | TypeTerm::TypeAliasReference { .. }
            | TypeTerm::TypeVar { .. }
            | TypeTerm::ParamSpec { .. }
            | TypeTerm::TypeVarTuple { .. }
            | TypeTerm::VariableForm { .. } => return false,
            TypeTerm::ClassInstance { arguments, .. }
            | TypeTerm::TypedDict { arguments, .. }
            | TypeTerm::SelfType { arguments, .. } => sequence = Some(*arguments),
            TypeTerm::TypeOf { target }
            | TypeTerm::TypeForm { target }
            | TypeTerm::Annotated { target }
            | TypeTerm::Unpack { target }
            | TypeTerm::TypeGuard { target, .. }
            | TypeTerm::TypeAlias { target, .. } => pending.push(*target),
            TypeTerm::Overloaded { alternatives } => sequence = Some(*alternatives),
            TypeTerm::Union { members } | TypeTerm::Intersection { members } => {
                sequence = Some(*members)
            }
            TypeTerm::Tuple { elements } => sequence = Some(*elements),
            TypeTerm::Overload { signatures, .. } => sequence = Some(*signatures),
            TypeTerm::Generic { parameters, body } => {
                sequence = Some(*parameters);
                pending.push(*body);
            }
            TypeTerm::BoundMethod { receiver, function } => pending.extend([*receiver, *function]),
            TypeTerm::Callable {
                form,
                parameters,
                param_spec,
                returns,
                ..
            } => {
                if !matches!(form, CallableForm::List) {
                    return false;
                }
                list = Some(*parameters);
                pending.extend(*param_spec);
                pending.push(*returns);
            }
            TypeTerm::ParamList {
                parameters,
                param_spec,
            } => {
                list = Some(*parameters);
                pending.extend(*param_spec);
            }
            TypeTerm::AnonymousTypedDict { fields, .. } => {
                if d.facts.type_dict_lists.get(*fields).is_none() {
                    return false;
                }
                for field in d
                    .facts
                    .type_dict_fields
                    .iter()
                    .filter(|f| f.list == *fields)
                {
                    if pending.len() >= 4096 {
                        return false;
                    }
                    pending.push(field.term);
                }
            }
            TypeTerm::ClassObject { .. }
            | TypeTerm::EnumLiteral { .. }
            | TypeTerm::Literal { .. }
            | TypeTerm::Module { .. }
            | TypeTerm::Never { .. }
            | TypeTerm::None
            | TypeTerm::LiteralString
            | TypeTerm::SpecialForm { .. } => {}
        }
        if let Some(sequence) = sequence {
            if d.facts.type_sequence_headers.get(sequence).is_none() {
                return false;
            }
            for child in d
                .facts
                .type_sequences
                .iter()
                .filter(|m| m.sequence == sequence)
            {
                if pending.len() >= 4096 {
                    return false;
                }
                pending.push(child.child);
            }
        }
        if let Some(list) = list {
            if d.facts.type_callable_lists.get(list).is_none() {
                return false;
            }
            for slot in d
                .facts
                .type_callable_slots
                .iter()
                .filter(|s| s.list == list)
            {
                if pending.len() >= 4096 {
                    return false;
                }
                pending.push(slot.term);
            }
        }
    }
    true
}
pub fn compare_terms(
    d: &ClassificationData,
    left: &[Id<TypeTerm>],
    right: &[Id<TypeTerm>],
) -> Difference {
    if left.is_empty() || right.is_empty() {
        return unresolved("type observation unavailable");
    }
    if left.iter().chain(right).any(|t| !closed_term(d, *t)) {
        return unresolved("open or unavailable retained type closure");
    }
    difference(&left, &right)
}
fn defaults(
    d: &ClassificationData,
    member: Id<CatalogMember>,
    parameter: Id<SignatureParameter>,
) -> Vec<Id<CatalogDefault>> {
    d.source
        .catalog
        .options
        .iter()
        .filter(|o| o.member == member)
        .filter_map(|o| match d.source.catalog.subjects.get(o.subject)? {
            CatalogOptionSubject::Parameter { slot }
                if d.source.core.slots.get(*slot)?.parameter == parameter =>
            {
                Some(o.default)
            }
            CatalogOptionSubject::SourceParameter { parameter: formal }
                if d.source
                    .core
                    .parameter_links
                    .iter()
                    .any(|l| l.entity == *formal && l.parameter == parameter) =>
            {
                Some(o.default)
            }
            _ => None,
        })
        .collect()
}
fn parameter_terms(
    d: &ClassificationData,
    parameter: Id<SignatureParameter>,
    context: Id<attribution::AnalysisContext>,
) -> Vec<Id<TypeTerm>> {
    let mut ids = d
        .source
        .core
        .slots
        .iter()
        .filter(|s| s.parameter == parameter)
        .flat_map(|slot| {
            d.source
                .core
                .slot_types
                .iter()
                .filter(move |t| t.slot == slot.id())
        })
        .filter_map(|t| d.source.core.signature_types.get(t.observation))
        .filter(|o| {
            d.source
                .core
                .qualifications
                .get(o.qualification)
                .is_some_and(|q| q.context == context)
        })
        .map(|t| t.term)
        .collect::<Vec<_>>();
    for link in d
        .source
        .core
        .parameter_links
        .iter()
        .filter(|l| l.parameter == parameter)
    {
        if let Some(super::entities::ParameterEntity::Source { declaration }) =
            d.source.core.parameters.get(link.entity)
        {
            ids.extend(
                d.facts
                    .type_observations
                    .iter()
                    .filter(|o| {
                        o.subject == *declaration
                            && o.role == TypeRole::Parameter
                            && d.source
                                .core
                                .qualifications
                                .get(o.qualification)
                                .is_some_and(|q| q.context == context)
                    })
                    .map(|o| o.term),
            );
        }
    }
    ids.sort();
    ids.dedup();
    ids
}
pub fn compare(
    d: &ClassificationData,
    member: Id<CatalogMember>,
    context: Id<attribution::AnalysisContext>,
    left: Id<SignatureVariant>,
    right: Id<SignatureVariant>,
    budget: &ResourceBudget,
) -> Result<ChargedResult<ContractComparison>, ModelError> {
    need(&d.source.catalog.members, member)?;
    let l = need(&d.source.core.variants, left)?;
    let r = need(&d.source.core.variants, right)?;
    for variant in [l, r] {
        if variant.context != context
            || !d
                .source
                .catalog
                .callables
                .iter()
                .filter(|c| c.member == member)
                .filter_map(|c| d.source.core.assessments.get(c.assessment))
                .any(|a| Some(a.callable) == variant.callable)
        {
            return Err(ModelError::Invalid(
                "comparison variant belongs to another member or context".into(),
            ));
        }
    }
    let ls = need(&d.facts.signatures, l.signature)?;
    let rs = need(&d.facts.signatures, r.signature)?;
    let count = d
        .facts
        .signature_parameters
        .iter()
        .filter(|p| p.signature == l.signature || p.signature == r.signature)
        .count();
    if count > 8192 {
        return Err(ModelError::Limit {
            owner: "callable-comparison",
            limit: "ports",
            observed: count,
            bound: 8192,
        });
    }
    let mut charge = budget.reserve("callable-comparison-result", (count + 1) * 8192)?;
    let mut lp = d
        .facts
        .signature_parameters
        .iter()
        .filter(|p| p.signature == l.signature)
        .collect::<Vec<_>>();
    let mut rp = d
        .facts
        .signature_parameters
        .iter()
        .filter(|p| p.signature == r.signature)
        .collect::<Vec<_>>();
    let _scratch = budget.reserve("callable-comparison-type-work", 4096 * 192)?;
    lp.sort_by_key(|p| p.ordinal);
    rp.sort_by_key(|p| p.ordinal);
    let mut ports = Vec::new();
    for ordinal in 0..lp.len().max(rp.len()) {
        let a = lp.get(ordinal);
        let b = rp.get(ordinal);
        let mut port = PortComparison {
            ordinal: ordinal as i64,
            left: a.map(|p| p.id()),
            right: b.map(|p| p.id()),
            layout: if ls.form == SignatureForm::List && rs.form == SignatureForm::List {
                Difference::DifferentRetainedStructure {}
            } else {
                unresolved("partial or unavailable port layout")
            },
            formal_identity: unresolved("formal correspondence unavailable"),
            defaults: unresolved("default unavailable"),
            types: unresolved("port absent"),
        };
        if let (Some(a), Some(b)) = (a, b) {
            port.layout = if ls.form == SignatureForm::List && rs.form == SignatureForm::List {
                difference(
                    need(&d.facts.shapes, a.shape)?,
                    need(&d.facts.shapes, b.shape)?,
                )
            } else {
                unresolved("partial or unavailable port layout")
            };
            let mut af = d
                .source
                .core
                .parameter_links
                .iter()
                .filter(|p| p.parameter == a.id())
                .map(|p| p.entity)
                .collect::<Vec<_>>();
            let mut bf = d
                .source
                .core
                .parameter_links
                .iter()
                .filter(|p| p.parameter == b.id())
                .map(|p| p.entity)
                .collect::<Vec<_>>();
            af.sort();
            af.dedup();
            bf.sort();
            bf.dedup();
            if af.len() == 1 && bf.len() == 1 {
                port.formal_identity = difference(&af, &bf);
            }
            let ad = defaults(d, member, a.id());
            let bd = defaults(d, member, b.id());
            if ad.len() == 1
                && bd.len() == 1
                && [ad[0], bd[0]].iter().all(|id| {
                    matches!(
                        d.source.catalog.defaults.get(*id),
                        Some(
                            CatalogDefault::Absent {}
                                | CatalogDefault::Literal { .. }
                                | CatalogDefault::Expression { .. }
                                | CatalogDefault::Factory { .. }
                        )
                    )
                })
            {
                port.defaults = difference(&ad, &bd);
            }
            port.types = compare_terms(
                d,
                &parameter_terms(d, a.id(), context),
                &parameter_terms(d, b.id(), context),
            );
        }
        ports.push(port);
    }
    let returns = |variant: &SignatureVariant| {
        let mut terms = d
            .source
            .core
            .return_types
            .iter()
            .filter(|t| t.variant == variant.id())
            .filter_map(|t| d.source.core.signature_types.get(t.observation))
            .filter(|o| {
                d.source
                    .core
                    .qualifications
                    .get(o.qualification)
                    .is_some_and(|q| q.context == context)
            })
            .map(|t| t.term)
            .collect::<Vec<_>>();
        if variant.role.runtime_source()
            && let Some(super::entities::CallableEntity::Source { declaration, .. }) = variant
                .callable
                .and_then(|c| d.source.core.source_callables.get(c))
        {
            terms.extend(
                d.facts
                    .type_observations
                    .iter()
                    .filter(|o| {
                        o.subject == *declaration
                            && o.role == TypeRole::Return
                            && d.source
                                .core
                                .qualifications
                                .get(o.qualification)
                                .is_some_and(|q| q.context == context)
                    })
                    .map(|o| o.term),
            );
        }
        terms.sort();
        terms.dedup();
        terms
    };
    let mut proof = vec![
        derivation::RowRef::of(left),
        derivation::RowRef::of(right),
        derivation::RowRef::of(ls.id()),
        derivation::RowRef::of(rs.id()),
    ];
    for port in &ports {
        for id in port.left.iter().chain(&port.right) {
            proof.push(derivation::RowRef::of(*id));
            if let Some(parameter) = d.facts.signature_parameters.get(*id) {
                proof.push(derivation::RowRef::of(parameter.shape));
            }
            for default in defaults(d, member, *id) {
                proof.push(derivation::RowRef::of(default));
            }
            for link in d
                .source
                .core
                .parameter_links
                .iter()
                .filter(|l| l.parameter == *id)
            {
                proof.push(derivation::RowRef::of(link.id()));
                proof.push(derivation::RowRef::of(link.entity));
            }
        }
    }
    // Preserve the original typed observations and their supports rather than presenting
    // a term difference without its qualified evidence route.
    let parameter_ids = ports
        .iter()
        .flat_map(|p| p.left.iter().chain(&p.right))
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    let mut source_subjects = std::collections::BTreeSet::new();
    for link in d
        .source
        .core
        .parameter_links
        .iter()
        .filter(|p| parameter_ids.contains(&p.parameter))
    {
        if let Some(super::entities::ParameterEntity::Source { declaration }) =
            d.source.core.parameters.get(link.entity)
        {
            source_subjects.insert(*declaration);
        }
    }
    for variant in [l, r] {
        if variant.role.runtime_source()
            && let Some(super::entities::CallableEntity::Source { declaration, .. }) = variant
                .callable
                .and_then(|c| d.source.core.source_callables.get(c))
        {
            source_subjects.insert(*declaration);
        }
    }
    for observation in d.source.core.signature_types.iter().filter(|o| {
        d.facts
            .signature_subjects
            .get(o.subject)
            .is_some_and(|s| match s {
                SignatureTypeSubject::Parameter { parameter } => parameter_ids.contains(parameter),
                SignatureTypeSubject::Return { signature } => {
                    *signature == l.signature || *signature == r.signature
                }
            })
            && d.source
                .core
                .qualifications
                .get(o.qualification)
                .is_some_and(|q| q.context == context)
    }) {
        charge.try_resize(charge.size().saturating_add(512))?;
        proof.push(derivation::RowRef::of(observation.id()));
        for support in d
            .facts
            .signature_type_supports
            .iter()
            .filter(|s| s.assertion == observation.id())
        {
            charge.try_resize(charge.size().saturating_add(512))?;
            proof.push(derivation::RowRef::of(support.id()));
        }
    }
    for observation in d.facts.type_observations.iter().filter(|o| {
        source_subjects.contains(&o.subject)
            && matches!(o.role, TypeRole::Parameter | TypeRole::Return)
            && d.source
                .core
                .qualifications
                .get(o.qualification)
                .is_some_and(|q| q.context == context)
    }) {
        charge.try_resize(charge.size().saturating_add(512))?;
        proof.push(derivation::RowRef::of(observation.id()));
        for support in d
            .facts
            .type_supports
            .iter()
            .filter(|s| s.assertion == observation.id())
        {
            charge.try_resize(charge.size().saturating_add(512))?;
            proof.push(derivation::RowRef::of(support.id()));
        }
    }
    proof.sort();
    proof.dedup();
    Ok(ChargedResult {
        value: ContractComparison {
            member,
            analysis: context,
            left,
            right,
            left_role: l.role,
            right_role: r.role,
            form: difference(&ls.form, &rs.form),
            adjustment: if l.adjustment == SignatureAdjustment::Unknown
                || r.adjustment == SignatureAdjustment::Unknown
            {
                unresolved("receiver adjustment unavailable")
            } else {
                difference(&l.adjustment, &r.adjustment)
            },
            ports,
            returns: compare_terms(d, &returns(l), &returns(r)),
            proof: proof
                .into_iter()
                .map(crate::domain::serving::ProofReference::from_canonical)
                .collect::<Result<Vec<_>, _>>()?,
        },
        _charge: charge,
    })
}

pub fn definition() -> ContentHash {
    ContentHash::of(b"normalized/contract_comparison/semantic-policy/v1")
}
