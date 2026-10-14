//! Collection is a test presentation; the provider emits each module synchronously.
pub fn index(
    inputs: &[cpg_flow::Input],
    context: &cpg_flow::RuntimeContext,
) -> Vec<cpg_flow::ModuleFlow> {
    let mut modules = Vec::new();
    cpg_flow::visit_modules(inputs, context, |_, flow| {
        modules.push(flow);
        Ok::<_, std::convert::Infallible>(())
    })
    .unwrap();
    modules
}
