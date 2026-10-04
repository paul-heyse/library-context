//! Source containers, formal declarations and native slots have distinct identities.
//! Consumers share this attachment rather than recovering a declaration by name or range.
use super::binding_normalization::BindingData;
use crate::domain::{
    attribution::AnalysisContext,
    calls::SignatureParameter,
    charged::{ChargedSet, StateCharge},
    resources::ResourceBudget,
    source::{Occurrence, SyntaxKind},
    syntax::ParameterSyntaxObservation,
    *,
};

pub struct SourceParameterCorrespondence {
    pub formal: Id<Occurrence>,
    pub parameters: ChargedSet<Id<SignatureParameter>>,
    _charge: StateCharge,
}

/// Missing/ambiguous source attachment is unavailable, never a parent-node fallback.
/// Native slots are admitted only through same-context declarations of this source function.
pub fn source_parameter(
    data: &BindingData,
    parameter: &ParameterSyntaxObservation,
    context: Id<AnalysisContext>,
    budget: &ResourceBudget,
) -> Result<Option<SourceParameterCorrespondence>, ModelError> {
    let in_context = |qualification| {
        data.qualifications
            .get(qualification)
            .is_some_and(|q| q.context == context)
    };
    if !in_context(parameter.qualification) {
        return Ok(None);
    }
    let Some(container) = data.occurrences.get(parameter.parameter) else {
        return Ok(None);
    };
    let formal = match container.syntax_kind {
        SyntaxKind::Parameter => container.id(),
        SyntaxKind::ParameterWithDefault => {
            let mut formal = None;
            for placement in data
                .placements
                .iter()
                .filter(|p| p.parent == Some(container.id()) && in_context(p.qualification))
            {
                let Some(child) = data.occurrences.get(placement.occurrence) else {
                    continue;
                };
                if child.syntax_kind != SyntaxKind::Parameter {
                    continue;
                }
                if child.source != container.source
                    || child.start < container.start
                    || child.end > container.end
                {
                    return Err(ModelError::Invalid(
                        "formal parameter crosses source container".into(),
                    ));
                }
                if formal.is_some_and(|previous| previous != child.id()) {
                    return Ok(None);
                }
                formal = Some(child.id());
            }
            let Some(formal) = formal else {
                return Ok(None);
            };
            formal
        }
        _ => return Ok(None),
    };
    let mut charge = StateCharge::new(budget, "source-parameter-correspondence");
    let mut parameters = ChargedSet::default();
    for declaration in data
        .parameter_declarations
        .iter()
        .filter(|d| d.declaration == formal && in_context(d.qualification))
    {
        let Some(slot) = data.parameters.get(declaration.parameter) else {
            continue;
        };
        let Some(signature) = data.signatures.get(slot.signature) else {
            continue;
        };
        let Some(shape) = data.shapes.get(slot.shape) else {
            continue;
        };
        if !signature.role.runtime_source()
            || slot.ordinal != parameter.ordinal
            || shape.kind != parameter.kind
            || !in_context(signature.qualification)
        {
            continue;
        }
        if data.entity_declarations.iter().any(|d| {
            d.symbol == signature.symbol
                && d.declaration == parameter.function
                && in_context(d.qualification)
        }) {
            parameters.insert(&mut charge, slot.id())?;
        }
    }
    Ok(Some(SourceParameterCorrespondence {
        formal,
        parameters,
        _charge: charge,
    }))
}

/// Descriptor adjustment belongs to the normalized variant of this callable, not its spelling.
pub fn is_bound_receiver(
    data: &BindingData,
    parameter: &SourceParameterCorrespondence,
    callable: Id<super::entities::CallableEntity>,
    context: Id<AnalysisContext>,
) -> bool {
    use super::callables::SignatureAdjustment;
    data.callable_variants
        .iter()
        .filter(|variant| {
            variant.callable == Some(callable)
                && variant.context == context
                && matches!(
                    variant.adjustment,
                    SignatureAdjustment::BindInstanceReceiver
                        | SignatureAdjustment::BindClassReceiver
                        | SignatureAdjustment::PropertyAccess
                )
        })
        .any(|variant| {
            data.callable_slots.iter().any(|slot| {
                slot.variant == variant.id()
                    && slot.ordinal == 0
                    && parameter.parameters.contains(&slot.parameter)
            })
        })
}
