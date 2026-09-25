//! A value source's call ancestry must retain order and operand roles before summary joins.

use std::path::Path;

use cpg_flow::{FlowCallOperandRole, Input, RuntimeBindings, RuntimeContext, Sink};

#[test]
fn value_sources_keep_nested_call_order_and_operand_roles() {
    let text = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/python/flow_call_paths/paths.py"),
    )
    .unwrap();
    let mut modules = cpg_flow::index(
        &[Input {
            path: "paths.py".into(),
            text: text.clone(),
            runtime: RuntimeBindings::default(),
        }],
        &RuntimeContext {
            python_version: (3, 14, 7),
            platform: "linux".into(),
        },
    );
    let flow = modules.pop().unwrap();
    assert_eq!(flow.error, None);
    assert!(
        flow.values
            .iter()
            .all(|v| v.through_call == !v.call_path.is_empty())
    );
    let source = |needle: &str, place: &str| {
        let start = text.find(needle).unwrap() as u32;
        flow.values
            .iter()
            .find(|v| {
                v.sink == Sink::Return
                    && v.span.start == start
                    && flow.uses[v.use_ix as usize].place == place
            })
            .unwrap()
    };
    let direct = source("cast(object, value)", "value");
    assert_eq!(direct.call_path.len(), 1);
    assert_eq!(direct.call_path[0].role, FlowCallOperandRole::Argument);
    assert_eq!(
        &text[direct.call_path[0].operand.start as usize..direct.call_path[0].operand.end as usize],
        "value"
    );
    let nested = source("outer(inner(data=value))", "value");
    assert_eq!(nested.call_path.len(), 2);
    assert!(
        nested
            .call_path
            .iter()
            .all(|step| step.role == FlowCallOperandRole::Argument)
    );
    assert_eq!(
        nested
            .call_path
            .iter()
            .map(|step| &text[step.call.start as usize..step.call.end as usize])
            .collect::<Vec<_>>(),
        vec!["outer(inner(data=value))", "inner(data=value)"]
    );
    assert_eq!(
        &text[nested.call_path[1].operand.start as usize..nested.call_path[1].operand.end as usize],
        "value"
    );
    let callee = source("func(value)", "func");
    assert_eq!(callee.call_path.len(), 1);
    assert_eq!(callee.call_path[0].role, FlowCallOperandRole::Callee);
    let computed = source("outer(value + 1)", "value");
    assert_eq!(computed.call_path.len(), 1);
    assert_eq!(computed.call_path[0].role, FlowCallOperandRole::Argument);
    assert_ne!(
        &text[computed.call_path[0].operand.start as usize
            ..computed.call_path[0].operand.end as usize],
        "value",
        "an argument role does not assert an unchanged value"
    );
}
