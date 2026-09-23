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
        v.extend(cpg_schema::for_each_analysis_table!(all));
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

/// ADR-0019 review O2: an analysis table never has a column named `fact_id`, which would make the
/// generator demand a `facts` row for it; a cited fact is `cited_fact_id`.
#[test]
fn no_analysis_table_has_a_fact_id_column() {
    macro_rules! all {
        ($($t:ty),+) => { vec![$((<$t as Table>::NAME, <$t as Table>::schema())),+] };
    }
    for (name, schema) in cpg_schema::for_each_analysis_table!(all) {
        assert!(schema.index_of("fact_id").is_err(), "{name} has fact_id");
    }
}

/// ADR-0019 review F1: perturbing any identity column of a finding changes its id; the lineage
/// columns (the invocation, the score, a step's edge id, a member's weight) are not inputs.
#[test]
fn a_finding_id_follows_every_identity_column() {
    use cpg_schema::findings::recipe::FindingKey;
    use cpg_schema::findings::{MemberKey, StepKey};
    use cpg_schema::id::Id;
    let step = StepKey {
        call_site: Id([1; 16]),
        callee: Id([2; 16]),
        modality: 0,
        arc_kind: 0,
        phase: Some(0),
    };
    let member = MemberKey {
        role: 0,
        ordinal: 0,
        node: Some(Id([3; 16])),
        cited_fact: None,
        label: Some("pkg.S".to_owned()),
    };
    let paths = vec![vec![step]];
    let members = vec![member.clone()];
    let base = FindingKey {
        finding_kind: 1,
        subject: Id([4; 16]),
        related: Some(Id([5; 16])),
        condition: None,
        evidence_status: 0,
        depth: Some(1),
        stop_reason: None,
        witnesses_omitted: false,
        paths: &paths,
        members: &members,
    };
    let id = base.id();
    let changed = |f: &dyn Fn(&mut FindingKey<'_>)| {
        let mut k = FindingKey { ..base };
        f(&mut k);
        k.id()
    };
    assert_ne!(id, changed(&|k| k.finding_kind = 2));
    assert_ne!(id, changed(&|k| k.subject = Id([9; 16])));
    assert_ne!(id, changed(&|k| k.related = None));
    assert_ne!(id, changed(&|k| k.condition = Some(Id([9; 16]))));
    assert_ne!(id, changed(&|k| k.evidence_status = 2));
    assert_ne!(id, changed(&|k| k.depth = Some(2)));
    assert_ne!(id, changed(&|k| k.stop_reason = Some(0)));
    assert_ne!(id, changed(&|k| k.witnesses_omitted = true));
    for alter in [
        |s: &mut StepKey| s.call_site = Id([9; 16]),
        |s: &mut StepKey| s.callee = Id([9; 16]),
        |s: &mut StepKey| s.modality = 1,
        |s: &mut StepKey| s.arc_kind = 1,
        |s: &mut StepKey| s.phase = Some(4),
        |s: &mut StepKey| s.phase = None,
    ] {
        let mut s = step;
        alter(&mut s);
        let other = vec![vec![s]];
        assert_ne!(
            id,
            FindingKey {
                paths: &other,
                ..base
            }
            .id()
        );
    }
    for alter in [
        |m: &mut MemberKey| m.role = 1,
        |m: &mut MemberKey| m.ordinal = 1,
        |m: &mut MemberKey| m.node = None,
        |m: &mut MemberKey| m.cited_fact = Some(Id([9; 16])),
        |m: &mut MemberKey| m.label = None,
    ] {
        let mut m = member.clone();
        alter(&mut m);
        let other = vec![m];
        assert_ne!(
            id,
            FindingKey {
                members: &other,
                ..base
            }
            .id()
        );
    }
}

/// DESIGN §10.2, slice 1.5 review F1: an assertion's status is a function of its supports alone.
#[test]
fn an_assertion_status_is_a_function_of_its_supports() {
    use cpg_schema::codebook::{EvidenceKind, EvidenceStatus::*, SupportRole::*};
    use cpg_schema::findings::{derive_status, evidence_status};
    assert_eq!(
        derive_status(&[]),
        Unresolved,
        "nothing cited, nothing stated"
    );
    assert_eq!(derive_status(&[(Scope, StructurallyObserved)]), Unresolved);
    assert_eq!(
        derive_status(&[(Support, StructurallyObserved)]),
        StructurallyObserved
    );
    assert_eq!(
        derive_status(&[
            (Support, StructurallyObserved),
            (Support, evidence_status(EvidenceKind::Span))
        ]),
        Documented,
        "the strongest status among the supports"
    );
    assert_eq!(
        derive_status(&[(Support, Documented), (Scope, StatisticallyDerived)]),
        StatisticallyDerived,
        "a statistical scope makes it statistical"
    );
    assert_eq!(
        derive_status(&[(Support, StatisticallyDerived), (Support, Unresolved)]),
        Unresolved
    );
    assert_eq!(
        derive_status(&[(Support, Documented), (Scope, Unresolved)]),
        Documented,
        "only a supporting finding's gap is the assertion's"
    );
    assert_eq!(evidence_status(EvidenceKind::Fact), StructurallyObserved);
    assert_eq!(evidence_status(EvidenceKind::FixtureRun), FixtureChecked);
}

/// Slice 1.5 review F6: the evidence, assertion and brief recipes follow their identity columns.
/// A cited fact id, a run, a model and a template version are lineage: no recipe takes them.
#[test]
fn synthesis_ids_follow_their_identity_columns() {
    use cpg_schema::findings::recipe::{self, AssertionKey};
    use cpg_schema::id::Id;
    let ev = |k: i16, n: u8, m: u8, span: Option<(i64, i64)>, t: Option<&str>| {
        recipe::evidence(k, Some(Id([n; 16])), Some(Id([m; 16])), span, t)
    };
    let base = ev(1, 1, 2, Some((3, 9)), Some("Run it."));
    for other in [
        ev(2, 1, 2, Some((3, 9)), Some("Run it.")),
        ev(1, 9, 2, Some((3, 9)), Some("Run it.")),
        ev(1, 1, 9, Some((3, 9)), Some("Run it.")),
        ev(1, 1, 2, Some((4, 9)), Some("Run it.")),
        ev(1, 1, 2, Some((3, 8)), Some("Run it.")),
        ev(1, 1, 2, None, Some("Run it.")),
        ev(1, 1, 2, Some((3, 9)), Some("Run it!")),
    ] {
        assert_ne!(base, other);
    }

    let supports = [
        (0, Some(Id([1; 16])), None),
        (0, None, Some(Id([2; 16]))),
        (1, Some(Id([3; 16])), None),
    ];
    let key = AssertionKey {
        kind: 0,
        subject: Id([4; 16]),
        applicable_case: None,
        status: 1,
        text: Some("Run it."),
        conditions: None,
        limitations: None,
        supports: &supports,
    };
    let id = key.id();
    let mut reversed = supports;
    reversed.reverse();
    assert_eq!(
        id,
        AssertionKey {
            supports: &reversed,
            ..key
        }
        .id(),
        "supports are sorted (ADR-0019)"
    );
    let fewer = &supports[..2];
    let moved = [
        (0, Some(Id([1; 16])), None),
        (0, None, Some(Id([2; 16]))),
        (0, Some(Id([3; 16])), None),
    ];
    for other in [
        AssertionKey { kind: 1, ..key },
        AssertionKey {
            subject: Id([9; 16]),
            ..key
        },
        AssertionKey {
            applicable_case: Some("stdio"),
            ..key
        },
        AssertionKey { status: 4, ..key },
        AssertionKey { text: None, ..key },
        AssertionKey {
            conditions: Some("c"),
            ..key
        },
        AssertionKey {
            limitations: Some("l"),
            ..key
        },
        AssertionKey {
            supports: fewer,
            ..key
        },
        AssertionKey {
            supports: &moved,
            ..key
        },
    ] {
        assert_ne!(id, other.id());
    }

    let (a, b) = (Id([5; 16]), Id([6; 16]));
    let brief = recipe::brief(Id([7; 16]), None, &[a, b]);
    assert_ne!(brief, recipe::brief(Id([8; 16]), None, &[a, b]));
    assert_ne!(brief, recipe::brief(Id([7; 16]), Some("stdio"), &[a, b]));
    assert_ne!(brief, recipe::brief(Id([7; 16]), None, &[a]));
    assert_ne!(
        brief,
        recipe::brief(Id([7; 16]), None, &[b, a]),
        "presentation order is the brief's"
    );
}
