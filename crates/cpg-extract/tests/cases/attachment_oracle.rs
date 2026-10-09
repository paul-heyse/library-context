//! Independent scalar geometry controls for the exact typed attachment contract.
use crate::typed_driver;
use crate::inspector;
use lctx_model::domain::{attachment::*, source::*, *};
use typed_driver::{Tables, rows};
inspector!(
    Syntax,
    lctx_model::domain::source::Occurrence
);
fn scalar(rows: &[Occurrence], query: &AttachmentQuery) -> Attachment {
    let mut candidates = rows
        .iter()
        .filter(|r| {
            r.source == query.source
                && r.syntax_kind == query.syntax_kind
                && r.role == query.role
                && r.start <= query.start
                && r.end >= query.end
                && query
                    .structural_path
                    .as_ref()
                    .is_none_or(|path| path == &r.structural_path)
        })
        .collect::<Vec<_>>();
    let Some(width) = candidates.iter().map(|r| r.end - r.start).min() else {
        return Attachment::Unmatched;
    };
    candidates.retain(|r| r.end - r.start == width);
    let mut ids = candidates.iter().map(|r| r.id()).collect::<Vec<_>>();
    ids.sort();
    match ids.as_slice() {
        [] => Attachment::Unmatched,
        [id] if width == query.end - query.start => Attachment::Exact(*id),
        [id] => Attachment::Innermost(*id),
        _ => Attachment::Ambiguous(ids),
    }
}
#[tokio::test]
async fn native_names_attributes_subscripts_wrappers_decorators_fstrings_bom_and_stubs_match_scalar()
 {
    let mut files = typed_driver::files("unicode_bom");
    files.extend(std::collections::BTreeMap::from([
        ("matrix.py".into(),b"@decorate(f'{label}')\ndef f(a: str):\n    return obj.member[a[0]](f'{a!r}', *args, **kw)\n".to_vec()),
        ("matrix.pyi".into(),b"@decorate\ndef f(a: str) -> str: ...\n".to_vec())]));
    let tables = Tables::default();
    typed_driver::run(&files, Syntax(tables.clone()))
        .await
        .unwrap();
    let occurrences = rows::<Occurrence>(&tables);
    let budget = typed_driver::budget();
    let index = OccurrenceIndex::new(&occurrences, budget.clone()).unwrap();
    for row in &occurrences {
        for (start, end) in [
            (row.start, row.end),
            (row.start + 1, row.end),
            (row.start, row.end.saturating_sub(1)),
        ] {
            if start > end {
                continue;
            }
            let query = AttachmentQuery {
                source: row.source,
                start,
                end,
                syntax_kind: row.syntax_kind,
                role: row.role,
                structural_path: None,
            };
            assert_eq!(
                index
                    .attach(&query, AttachmentBudget::default())
                    .unwrap()
                    .value(),
                &scalar(&occurrences, &query)
            );
        }
    }
    let row = occurrences
        .iter()
        .find(|r| r.syntax_kind == SyntaxKind::ExprName)
        .unwrap();
    let query = AttachmentQuery {
        source: row.source,
        start: row.start,
        end: row.end,
        syntax_kind: row.syntax_kind,
        role: row.role,
        structural_path: None,
    };
    assert_eq!(
        index
            .attach(
                &query,
                AttachmentBudget {
                    visited_nodes: 0,
                    alternatives: 256
                }
            )
            .unwrap()
            .value(),
        &Attachment::BudgetExceeded
    );
    let mut duplicate = row.clone();
    duplicate.structural_path.push(999);
    let ambiguous =
        OccurrenceIndex::new(&[row.clone(), duplicate.clone()], budget.clone()).unwrap();
    let mut ids = vec![row.id(), duplicate.id()];
    ids.sort();
    assert_eq!(
        ambiguous
            .attach(&query, AttachmentBudget::default())
            .unwrap()
            .value(),
        &Attachment::Ambiguous(ids)
    );
    let mut exact = query.clone();
    exact.structural_path = Some(row.structural_path.clone());
    assert_eq!(
        ambiguous
            .attach(&exact, AttachmentBudget::default())
            .unwrap()
            .value(),
        &Attachment::Exact(row.id())
    );
    let mut unmatched = query;
    unmatched.start = 1_000_000;
    unmatched.end = 1_000_001;
    assert_eq!(
        index
            .attach(&unmatched, AttachmentBudget::default())
            .unwrap()
            .value(),
        &Attachment::Unmatched
    );
    drop(index);
    drop(ambiguous);
    assert_eq!(budget.reserved(), 0);
}
