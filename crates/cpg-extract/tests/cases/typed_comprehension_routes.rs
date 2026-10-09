//! Actual retained Ruff parse emits semantic comprehension child fields, without another parse.
use crate::typed_driver;
use crate::inspector;
use lctx_model::domain::{lexical::SyntaxField, source::*, syntax::*, *};
use typed_driver::{files, rows, run};
inspector!(Routes, Occurrence, SyntaxPlacement, SyntaxObservation);
#[tokio::test]
async fn native_comprehensions_keep_element_clause_target_iter_and_filter_routes() {
    let tables = typed_driver::Tables::default();
    run(&files("catalog_context"), Routes(tables.clone()))
        .await
        .unwrap();
    let occurrences = rows::<Occurrence>(&tables);
    let placements = rows::<SyntaxPlacement>(&tables);
    for kind in [
        SyntaxKind::ExprGenerator,
        SyntaxKind::ExprListComp,
        SyntaxKind::ExprSetComp,
        SyntaxKind::ExprDictComp,
    ] {
        let parents = occurrences
            .iter()
            .filter(|r| r.syntax_kind == kind)
            .collect::<Vec<_>>();
        assert!(!parents.is_empty());
        for parent in parents {
            let children = placements
                .iter()
                .filter(|r| r.parent == Some(parent.id()))
                .collect::<Vec<_>>();
            assert!(children.iter().any(|r| {
                r.field == SyntaxField::Item
                    && occurrences.iter().any(|o| {
                        o.id() == r.occurrence && o.syntax_kind == SyntaxKind::Comprehension
                    })
            }));
            assert!(children.iter().any(|r| r.field
                == if kind == SyntaxKind::ExprDictComp {
                    SyntaxField::Value
                } else {
                    SyntaxField::Element
                }));
        }
    }
    for clause in occurrences
        .iter()
        .filter(|r| r.syntax_kind == SyntaxKind::Comprehension)
    {
        let children = placements
            .iter()
            .filter(|r| r.parent == Some(clause.id()))
            .collect::<Vec<_>>();
        assert!(children.iter().any(|r| r.field == SyntaxField::Target));
        assert!(children.iter().any(|r| r.field == SyntaxField::Iter));
    }
    assert!(placements.iter().any(|r| r.field == SyntaxField::Test
        && r.parent.is_some_and(|parent| {
            occurrences
                .iter()
                .any(|o| o.id() == parent && o.syntax_kind == SyntaxKind::Comprehension)
        })));
    let multiple = occurrences
        .iter()
        .find(|r| {
            r.syntax_kind == SyntaxKind::ExprGenerator
                && placements
                    .iter()
                    .filter(|p| p.parent == Some(r.id()) && p.field == SyntaxField::Item)
                    .count()
                    == 2
        })
        .unwrap();
    let ordinals = placements
        .iter()
        .filter(|r| r.parent == Some(multiple.id()) && r.field == SyntaxField::Item)
        .map(|r| r.ordinal)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(ordinals, [0, 1].into());
}
