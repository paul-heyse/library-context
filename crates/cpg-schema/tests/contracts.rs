//! Table contracts are schema migrations when they change (AGENTS.md): each table's name, family,
//! total key, immutable CHECKs and canonical schema are snapshot-tested.

use cpg_schema::codebook::{DeclarationKind, FactFamily};
use cpg_schema::id::{Id, IdHasher};
use cpg_schema::table::{Table, canonical_sort};
use cpg_schema::tables::{self, Declarations, DeclarationsRow};

#[test]
fn contracts_snapshot() {
    for (name, text) in tables::contracts() {
        insta::assert_snapshot!(name, text);
    }
}

#[test]
fn derivations_snapshot() {
    // The Stage C/D queries are part of the contract: a changed join is a changed table.
    let text: Vec<String> = cpg_schema::derived::derivations()
        .into_iter()
        .map(|(name, sql)| format!("-- {name}\n{sql}\n"))
        .collect();
    insta::assert_snapshot!(text.join("\n"));
}

#[test]
fn rules_snapshot() {
    let text: Vec<String> = cpg_schema::rules::rules()
        .into_iter()
        .map(|r| format!("{}\n  {}", r.name, r.sql))
        .collect();
    insta::assert_snapshot!(text.join("\n"));
}

#[test]
fn references_name_real_columns() {
    let schemas: std::collections::BTreeMap<&str, arrow_schema::SchemaRef> = {
        macro_rules! all {
            ($($t:ty),+) => { vec![$((<$t as Table>::NAME, <$t as Table>::schema())),+] };
        }
        let mut v = cpg_schema::for_each_table!(all);
        v.extend(cpg_schema::for_each_derived_table!(all));
        v.into_iter().collect()
    };
    for r in cpg_schema::rules::REFERENCES {
        assert!(
            schemas[r.table].index_of(r.column).is_ok(),
            "{}.{}",
            r.table,
            r.column
        );
        for (t, c) in r.to {
            assert!(schemas[t].index_of(c).is_ok(), "{t}.{c}");
        }
    }
}

#[test]
fn keys_and_checks_name_real_columns() {
    // Every key column exists; every CHECK mentions only declared columns.
    macro_rules! check {
        ($($t:ty),+) => {$({
            let schema = <$t as Table>::schema();
            for k in <$t as Table>::key() {
                assert!(schema.index_of(k).is_ok(), "{}: key column {k}", <$t as Table>::NAME);
            }
            assert!(<$t as Table>::key().contains(&"snapshot_id"), "{}: key is snapshot-qualified", <$t as Table>::NAME);
            for (_, expr) in <$t as Table>::checks() {
                for word in expr.split(|c: char| !(c.is_alphanumeric() || c == '_')) {
                    if word.contains('_') {
                        assert!(schema.index_of(word).is_ok(), "{}: CHECK column {word}", <$t as Table>::NAME);
                    }
                }
            }
        })+};
    }
    use cpg_schema::derived::*;
    use cpg_schema::tables::*;
    check!(
        Snapshots,
        ProviderNodeMap,
        Exports,
        Signatures,
        Parameters,
        Resolutions,
        CallTargets,
        Facts,
        Runs,
        Contexts,
        Producers,
        Releases,
        Distributions,
        SourceFiles,
        Declarations,
        ExportSyntax,
        PublicNames,
        ParameterSyntax,
        PysaFunctions,
        ParameterSemantics,
        ClassAncestry,
        CallSyntax,
        Arguments,
        PysaCalls,
        Coverage,
        Boundaries
    );
    assert_eq!(Declarations::FAMILY, FactFamily::Exports);
}

fn id(n: i64) -> Id {
    IdHasher::new("test").i64(n).finish_id()
}

fn decl(n: i64, start: i64) -> DeclarationsRow {
    DeclarationsRow {
        snapshot_id: id(0),
        fact_id: id(100 + n),
        node_id: id(200 + n),
        module_node_id: id(1),
        parent_node_id: (n % 2 == 0).then(|| id(300)),
        qualified_name: format!("pkg.f{n}"),
        name: format!("f{n}"),
        kind: DeclarationKind::Function,
        start_byte: start,
        end_byte: start + 10,
        name_start_byte: start + 4,
        name_end_byte: start + 6,
        docstring: None,
        docstring_start_byte: None,
        docstring_end_byte: None,
        is_overload: false,
        decorators: vec!["overload".to_owned(); (n % 3) as usize],
    }
}

#[test]
fn batches_type_check_and_sort_canonically_regardless_of_input_order() {
    let rows: Vec<_> = (0..20).map(|n| decl(n, (n * 37) % 11 * 20)).collect();
    let mut reversed = rows.clone();
    reversed.reverse();
    let a = Declarations::to_sorted_batch(&rows).unwrap();
    let b = Declarations::to_sorted_batch(&reversed).unwrap();
    assert_eq!(a, b, "the declared total key makes the order canonical");
    assert_eq!(a.schema(), Declarations::schema());
    let empty = Declarations::to_sorted_batch(&[]).unwrap();
    assert_eq!(empty.num_rows(), 0);
    // Re-sorting a sorted batch is the identity.
    assert_eq!(canonical_sort(&a, Declarations::key()).unwrap(), a);
}
