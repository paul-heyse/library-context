//! Coherent topic controls; common drivers compile once in this target.
#[path = "typed_driver/mod.rs"]
mod typed_driver;

#[path = "cases/native_diagnostics.rs"]
mod native_diagnostics;
#[path = "cases/native_exports.rs"]
mod native_exports;
#[path = "cases/native_lexical.rs"]
mod native_lexical;
#[path = "cases/typed_captures.rs"]
mod typed_captures;
#[path = "cases/typed_class_metadata.rs"]
mod typed_class_metadata;
#[path = "cases/typed_comprehension_routes.rs"]
mod typed_comprehension_routes;
#[path = "cases/typed_lexical.rs"]
mod typed_lexical;
#[path = "cases/typed_ruff_context.rs"]
mod typed_ruff_context;
#[path = "cases/typed_symbols.rs"]
mod typed_symbols;
#[path = "cases/typed_syntax_shapes.rs"]
mod typed_syntax_shapes;
