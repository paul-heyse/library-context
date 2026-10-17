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
    captured_variable::{CaptureKind, ModuleCapturedVariables},
    function::{FunctionBaseDefinition, FunctionId, FunctionRef, ModuleFunctionDefinitions},
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
    definitions: &ModuleFunctionDefinitions<FunctionBaseDefinition>,
    captured: &ModuleCapturedVariables<FunctionRef>,
    module: Id<ProviderModule>,
    module_id: ModuleId,
    module_name: ModuleName,
    q: &AssertionQualification,
    natives: &Natives,
    budget: &ResourceBudget,
) -> Result<Records, ModelError> {
    records_from_functions(
        definitions
            .as_map()
            .iter()
            .map(|(id, base)| (id, &base.name)),
        captured,
        module,
        module_id,
        module_name,
        q,
        natives,
        budget,
    )
}

#[allow(
    clippy::too_many_arguments,
    reason = "Exact module identity and native capture inventory are separate inputs"
)]
fn records_from_functions<'a>(
    definitions: impl IntoIterator<Item = (&'a FunctionId, &'a ruff_python_ast::name::Name)>,
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
    for (id, name) in definitions {
        let Some(function) = symbol(natives, module, &id.serialize_to_string())? else {
            continue;
        };
        let reference = FunctionRef {
            module_id,
            module_name,
            function_id: id.clone(),
            function_name: name.clone(),
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

#[cfg(test)]
mod selected_inventory_controls {
    use super::*;
    use lctx_model::domain::{assertion::Approximation, attribution::Modality, calls::SymbolKind};
    use pyrefly::{
        report::pysa::{
            PysaFormat, PysaReporter,
            captured_variable::collect_captured_variables_for_module,
            context::{ModuleAnswersContext, ModuleContext, PysaResolver},
            export_module_definitions, export_module_definitions_selected,
            module::ModuleIds,
            override_graph::create_reversed_override_graph_for_module,
        },
        state::{require::Require, state::State},
    };
    use pyrefly_config::{
        config::{ConfigFile, ConfigSource},
        finder::ConfigFinder,
    };
    use pyrefly_python::{
        module_path::ModulePath,
        sys_info::{PythonPlatform, PythonVersion},
    };
    use pyrefly_util::{arc_id::ArcId, thread_pool::ThreadCount};

    fn id<R>(n: u8) -> Id<R> {
        serde_json::from_value(serde_json::json!(vec![n; 16])).unwrap()
    }

    #[test]
    fn selected_payloads_keep_full_report_capture_eligibility_for_reference_only_functions() {
        std::thread::Builder::new().stack_size(64 << 20).spawn(|| {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../fixtures/python/capture_observations/api.py").canonicalize().unwrap();
            let root = path.parent().unwrap();
            let text = std::fs::read_to_string(&path).unwrap();
            let mut config = ConfigFile {
                source: ConfigSource::File(root.join("pyrefly.toml")),
                search_path_from_args: vec![root.to_path_buf()],
                disable_search_path_heuristics: true, disable_project_excludes_heuristics: true,
                enable_fallback_search_path: false, ..ConfigFile::default()
            };
            config.python_environment.python_version = Some(PythonVersion::new(3, 14, 7));
            config.python_environment.python_platform = Some(PythonPlatform::new("linux"));
            config.python_environment.site_package_path = Some(vec![]);
            config.interpreters.skip_interpreter_query = true;
            assert!(config.configure().is_empty());
            let handle = config.handle_from_module_path(ModulePath::filesystem(path.clone()));
            let state = State::new(ConfigFinder::new_constant(ArcId::new(config)), ThreadCount::Inline);
            let mut transaction = state.new_transaction(Require::Exports, None);
            transaction.set_pysa_reporter(Some(Box::new(PysaReporter {
                module_ids: ModuleIds::new(std::slice::from_ref(&handle)),
                pysa_directory: Default::default(), definitions_directory: Default::default(),
                type_of_expressions_directory: Default::default(), call_graphs_directory: Default::default(),
                format: PysaFormat::Json, write_files: false,
            })));
            transaction.run(std::slice::from_ref(&handle), Require::Everything, None);
            let module_ids = &transaction.pysa_reporter().unwrap().module_ids;
            let resolver = PysaResolver::new(&transaction, module_ids, handle.clone());
            let context = ModuleContext {
                answers_context: ModuleAnswersContext::create(handle.clone(), &transaction, module_ids),
                resolver: &resolver,
            };
            let captured = collect_captured_variables_for_module(&context);
            let overrides = create_reversed_override_graph_for_module(&context);
            let budget = ResourceBudget::fixed(64 << 20).unwrap();
            let mut selection = None;
            let selected = export_module_definitions_selected(&context, &captured, &overrides, |classes, bases| {
                let (kept, functions) = crate::symbol_records::select_definitions(classes, bases,
                    &std::collections::BTreeSet::new(), &["stable.inner".into()], false, &budget)?;
                selection = Some(kept);
                Ok::<_, ModelError>(functions)
            }).unwrap();
            let full = export_module_definitions(&context, &captured, &overrides);
            let bases = &resolver.current_module_solutions().function_base_definitions;
            assert_eq!(bases.as_map().len(), full.function_definitions.as_map().len());
            for (key, definition) in full.function_definitions.as_map() {
                assert_eq!(bases.get(key), Some(&definition.base));
            }
            let qualification = AssertionQualification {
                context: id(1), scope: id(2), condition: id(3), modality: Modality::Definite,
                approximation: Approximation::Exact,
                assumptions: lctx_model::domain::assumptions::AssumptionSet::empty_id(),
            };
            let input = lctx_model::domain::input::InputRevision::from_entries(vec![]).unwrap();
            let artifacts = [lctx_model::domain::source::SourceArtifact::from_bytes(input.id(), "api.py".into(), text.as_bytes()).unwrap()];
            let mut natives = Natives::new(id(4), qualification.context, [(root, artifacts.as_slice())], &budget).unwrap();
            let module = natives.module("api", handle.path()).unwrap();
            // An earlier reference (or override lowering) may already have registered ANY function.
            // These are deliberately not all selected payloads, nor retained definitions.
            for (key, base) in bases.as_map() {
                natives.symbol(module, key.serialize_to_string(), base.name.to_string(), SymbolKind::Function).unwrap();
            }
            assert!(natives.definitions.is_empty());
            let actual = records(bases, &captured, module, full.module_id, full.module_name,
                &qualification, &natives, &budget).unwrap();
            let baseline = records_from_functions(full.function_definitions.as_map().iter()
                .map(|(key, definition)| (key, &definition.base.name)), &captured,
                module, full.module_id, full.module_name, &qualification, &natives, &budget).unwrap();
            assert_eq!(actual.rows, baseline.rows);
            let payload_only = records_from_functions(selected.function_definitions.as_map().iter()
                .map(|(key, definition)| (key, &definition.base.name)), &captured,
                module, full.module_id, full.module_name, &qualification, &natives, &budget).unwrap();
            assert!(actual.rows.len() > payload_only.rows.len(), "selected heavy payloads cannot own capture eligibility");
            assert!(actual.rows.iter().any(|row| row.name == "GLOBAL" && row.origin == CaptureOrigin::Global && row.declaring.is_none()));
            assert!(actual.rows.iter().any(|row| row.name == "value" && row.origin == CaptureOrigin::OuterFunction && row.declaring.is_some()));
            assert!(actual.rows.iter().any(|row| row.name == "cell" && row.origin == CaptureOrigin::OuterFunction));
            drop(payload_only); drop(baseline); drop(actual); drop(natives); drop(selection);
            assert_eq!(budget.reserved(), 0);
        }).unwrap().join().unwrap();
    }
}
