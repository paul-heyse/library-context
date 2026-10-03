//! The current linked consumer reuses a verified snapshot, not a scratch-only fork API.
use cpg_extract::ruff_context::{CanonicalSyntax, ContextSettings};
use lctx_model::domain::{attribution::AnalysisContext, resources::ResourceBudget, source::SourceArtifact, *};
use ruff_linter::semantic_facts::{Fact, Sink, StopReason};
use ruff_text_size_latest::Ranged;
#[derive(Default)]
struct Rows(Vec<Fact>);
impl Sink for Rows {
    fn observe(&mut self, row: Fact) -> Result<(), StopReason> { self.0.push(row); Ok(()) }
}
fn context() -> AnalysisContext {
    AnalysisContext { python_version: "3.14.7".into(), python_platform: "linux".into(), search_path: vec!["release".into()], site_package_path: vec![], config_digest: ContentHash::of(b"explicit"), environment_digest: ContentHash::of(b"frozen"), lock_digest: None }
}
fn artifact(text: &str) -> SourceArtifact {
    SourceArtifact::from_bytes(serde_json::from_value(serde_json::json!(vec![1;16])).unwrap(), "pkg/api.py".into(), text.as_bytes()).unwrap()
}
#[test]
fn populated_context_keeps_alias_resolution_final_tables_and_exact_snapshot() {
    let source = "from typing import TYPE_CHECKING as TC\nfrom typing import final as fin\nif TC:\n    annotation_only = 1\n@fin\ndef f():\n    return missing\nfin = 0\nfin\n__all__ = ['f']\n";
    let b = ResourceBudget::fixed(16 << 20).unwrap();
    let syntax = CanonicalSyntax::parse(&artifact(source), source, &context(), ContextSettings::default(), &b).unwrap();
    assert_eq!(syntax.module().body.len(), 7);
    let mut rows = Rows::default();
    let stats = syntax.observe(&mut rows).unwrap();
    assert_eq!(stats.rows, rows.0.len());
    assert!(rows.0.iter().any(|r| matches!(r, Fact::Branch { type_checking: true, .. })));
    assert!(rows.0.iter().any(|r| matches!(r, Fact::Node(n) if n.qualified_name.as_deref() == Some(&["typing".to_owned(), "final".to_owned()]))));
    let imported = rows.0.iter().find_map(|r| match r { Fact::Binding(b) if b.name == "fin" && b.kind == "FromImport" => Some(b.id), _ => None }).unwrap();
    assert!(rows.0.iter().any(|r| matches!(r, Fact::Reference(r) if r.binding == imported)));
    assert!(rows.0.iter().any(|r| matches!(r, Fact::Binding(b) if b.kind == "Assignment" && b.shadowed == Some(imported))));
    assert!(rows.0.iter().any(|r| matches!(r, Fact::Unresolved { name, .. } if name == "missing")));
    assert!(rows.0.iter().any(|r| matches!(r, Fact::Export { name, .. } if name == "f")));
    assert!(syntax.module().range().end().to_u32() <= source.len() as u32);
    assert!(CanonicalSyntax::parse(&artifact(source), "different", &context(), ContextSettings::default(), &b).is_err());
    drop(syntax);
    assert_eq!(b.reserved(), 0);
}
#[test]
fn cancellation_and_limits_never_certify_complete_context() {
    struct Cancel;
    impl Sink for Cancel {
        fn checkpoint(&mut self) -> Result<(), StopReason> { Err(StopReason::Cancelled) }
        fn observe(&mut self, _: Fact) -> Result<(), StopReason> { panic!("cancelled before emission") }
    }
    let source = "x = 1\n";
    let b = ResourceBudget::fixed(1 << 20).unwrap();
    let syntax = CanonicalSyntax::parse(&artifact(source), source, &context(), ContextSettings { maximum_rows: 1, ..Default::default() }, &b).unwrap();
    assert_eq!(syntax.observe(&mut Cancel).unwrap_err().reason, StopReason::Cancelled);
    assert_eq!(syntax.observe(&mut Rows::default()).unwrap_err().reason, StopReason::RowBudget);
}
