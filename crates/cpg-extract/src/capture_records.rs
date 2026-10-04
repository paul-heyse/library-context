//! Native function-key joins, including globals omitted by Pysa's exported capture projection.
use crate::natives::Natives;
use lctx_model::domain::{
    assertion::AssertionQualification,
    calls::{ProviderModule, ProviderSymbol},
    captures::*,
    charged::StateCharge,
    resources::ResourceBudget,
    *,
};
use pyrefly::report::pysa::{
    PysaModuleDefinitions,
    captured_variable::{CaptureKind, ModuleCapturedVariables},
    function::FunctionRef,
    module::ModuleId,
};
use pyrefly_python::module_name::ModuleName;
pub struct Records {
    charge: StateCharge,
    pub rows: Vec<CaptureObservation>,
}
impl Records {
    fn hold(&mut self, row: &CaptureObservation) -> Result<(), ModelError> {
        self.charge.grow(
            size_of::<CaptureObservation>()
                .saturating_mul(4)
                .saturating_add(row.heap_bytes()),
        )
    }
}
fn symbol(
    natives: &Natives,
    module: Id<ProviderModule>,
    key: &str,
) -> Result<Option<Id<ProviderSymbol>>, ModelError> {
    let mut matches = natives
        .symbols
        .values()
        .filter(|s| s.module == module && s.native_key == key);
    match (matches.next(), matches.next()) {
        (Some(s), None) => Ok(Some(s.id())),
        (None, None) => Ok(None),
        _ => Err(ModelError::Invalid(
            "capture native function key is ambiguous".into(),
        )),
    }
}
#[allow(
    clippy::too_many_arguments,
    reason = "Capture map and function definitions share an exact native module identity"
)]
pub fn records(
    definitions: &PysaModuleDefinitions,
    captured: &ModuleCapturedVariables<FunctionRef>,
    module: Id<ProviderModule>,
    module_id: ModuleId,
    module_name: ModuleName,
    q: &AssertionQualification,
    natives: &Natives,
    budget: &ResourceBudget,
) -> Result<Records, ModelError> {
    let mut out = Records {
        charge: StateCharge::new(budget, "native_capture_records"),
        rows: vec![],
    };
    for (id, definition) in definitions.function_definitions.as_map() {
        let Some(function) = symbol(natives, module, &id.serialize_to_string())? else {
            continue;
        };
        let reference = FunctionRef {
            module_id,
            module_name,
            function_id: id.clone(),
            function_name: definition.base.name.clone(),
        };
        let Some(captures) = captured.get(&reference) else {
            continue;
        };
        for (name, origin) in captures {
            let (origin, declaring) = match origin {
                CaptureKind::Global => (CaptureOrigin::Global, None),
                CaptureKind::Local(outer) => {
                    if outer.module_id != module_id {
                        return Err(ModelError::Invalid(
                            "native local capture crosses declaring module".into(),
                        ));
                    }
                    let declaring =
                        symbol(natives, module, &outer.function_id.serialize_to_string())?
                            .ok_or_else(|| {
                                ModelError::Invalid(
                                    "native declaring capture function was not retained".into(),
                                )
                            })?;
                    (CaptureOrigin::OuterFunction, Some(declaring))
                }
            };
            let row = CaptureObservation {
                qualification: q.id(),
                function,
                name: name.to_string(),
                origin,
                declaring,
                mutable: None,
                timing: CaptureTiming::Unknown,
            };
            row.validate()?;
            out.hold(&row)?;
            out.rows.push(row);
        }
    }
    out.rows.sort_by_key(Record::id);
    out.rows.dedup_by_key(|row| row.id());
    Ok(out)
}
