//! Relation declarations on a sample model that exercises every declaration feature: identity,
//! provenance, roles, codebooks, nullable and list columns, uniques, checks and lookups.

use arrow_array::RecordBatch;
use lctx_model::ddl::{self, DdlConfig, TableSpec};
use lctx_model::decl::codebook::CodebookEntry;
use lctx_model::decl::identity;
use lctx_model::decl::relation::{
    ColumnClass, ColumnDecl, Exposure, FidelityClass, Layer, Polarity, Relation, RelationDecl,
    validate,
};
use lctx_model::id::{Id, IdKind};

/// Sample recipes for the retained `recipe!` machinery; the production recipe catalog is retired
/// (the typed `domain` owns identity).
mod recipes {
    use lctx_model::id::Id;
    lctx_model::recipe!(occurrence, OCCURRENCE = Occurrence { module: Id, start: i64, end: i64, syntax_kind: &str });
    lctx_model::recipe!(symbol_key, SYMBOL_KEY = SymbolKey { distribution: &str, qualified_path: &str, descriptor: &str });
}
use lctx_model::{model, relation};

relation! {
    /// Release-independent entities.
    Entities, EntitiesRow = "entities" {
        layer: L1,
        family: "sample",
        stage: "normalize",
        fidelity: Resolved,
        polarity: Exact,
        coverage: "release",
        key: [entity_id],
        identity: entity_id = recipes::SYMBOL_KEY [distribution, qualified_path, descriptor],
        serve: { lookups: [[qualified_path]], wire: true },
    }
    row {
        entity_id: Id,
        distribution: String,
        qualified_path: String,
        descriptor: String,
        run_id: Id [provenance],
    }
}

relation! {
    /// Source occurrences within their module entity.
    Occurrences, OccurrencesRow = "occurrences" {
        layer: L0,
        family: "sample",
        stage: "extract",
        fidelity: Extracted,
        polarity: Exact,
        coverage: "module",
        key: [occurrence_id],
        identity: occurrence_id = recipes::OCCURRENCE [module_id, start_byte, end_byte, syntax_kind],
        checks: [("span_ordered", "start_byte <= end_byte")],
    }
    row {
        occurrence_id: Id,
        module_id: Id [ref entities.entity_id],
        start_byte: i64,
        end_byte: i64,
        syntax_kind: String,
        run_id: Id [provenance],
    }
}

relation! {
    /// A relation with a codebook, a nullable column, lists, a float and a second unique key.
    Notes, NotesRow = "notes" {
        layer: L2,
        family: "sample",
        stage: "derive",
        fidelity: Derived,
        polarity: May,
        coverage: "generation",
        key: [note_id],
        unique: [[subject_id, ordinal]],
    }
    row {
        note_id: Id,
        subject_id: Id [ref occurrences.occurrence_id],
        ordinal: i64,
        kind: IdKind,
        label: Option<String>,
        tags: Vec<String>,
        scores: Vec<f64>,
        weight: f64,
    }
}

model! { Entities, Occurrences, Notes }

const CFG: DdlConfig = DdlConfig {
    schema: "lctx",
    reader: "lctx_serving",
};

fn install() -> ddl::Install {
    let tables = DECLS
        .iter()
        .map(|d| TableSpec::from_decl(d).expect("renderable"))
        .collect();
    ddl::install(tables, &[CodebookEntry::of::<IdKind>()], &CFG).expect("installable")
}

#[test]
fn the_sample_model_is_valid() {
    validate(DECLS).expect("valid");
    assert_eq!(RelationId::ALL.len(), 3);
    assert_eq!(RelationId::Occurrences.decl().name, "occurrences");
}

#[test]
fn the_generated_ddl_is_pinned() {
    let install = install();
    let mut text = install.statements.join(";\n\n");
    for spec in &install.tables {
        text.push_str(&format!(
            ";\n\n-- staging templates for {}\n{};\n{};\n{};\n{}",
            spec.name,
            spec.staging_table(),
            spec.staging_indexes().join(";\n"),
            spec.attach(&CFG),
            spec.detach(&CFG),
        ));
    }
    insta::assert_snapshot!(text);
    insta::assert_snapshot!("ddl_digest", install.digest.hex());
}

#[test]
fn referenced_relations_are_created_first() {
    let order: Vec<_> = install().tables.iter().map(|t| t.name.clone()).collect();
    assert_eq!(order, ["entities", "occurrences", "notes"]);
}

#[test]
fn the_store_supplies_the_generation_column() {
    let spec = TableSpec::from_decl(&Occurrences::DECL).unwrap();
    assert_eq!(spec.columns[0].name, "generation_id");
    assert!(spec.partition.supplied);
    assert_eq!(
        spec.copy_columns(),
        ["occurrence_id", "module_id", "start_byte", "end_byte", "syntax_kind", "run_id"]
    );
    assert_eq!(
        spec.primary_key.as_deref(),
        Some(&["generation_id".to_owned(), "occurrence_id".to_owned()][..])
    );
}

#[test]
fn identifiers_are_quoted_and_names_bounded() {
    assert_eq!(ddl::quote("references"), "\"references\"");
    assert_eq!(ddl::quote("a\"b"), "\"a\"\"b\"");
    let long = "x".repeat(49);
    let schema = arrow_schema::Schema::new(vec![arrow_schema::Field::new(
        "snapshot_id",
        arrow_schema::DataType::FixedSizeBinary(16),
        false,
    )]);
    let err = TableSpec::build(
        &long,
        &schema,
        "snapshot_id",
        &ddl::TableOptions {
            key: &["snapshot_id"],
            ..Default::default()
        },
    )
    .unwrap_err();
    assert!(err.0.contains("bytes"), "{err}");
}

#[test]
fn codebook_tables_carry_every_code() {
    let statements = ddl::codebook(&CodebookEntry::of::<IdKind>(), &CFG);
    assert!(statements[1].contains("(0, 'occurrence')"));
    assert!(statements[1].contains("(14, 'ddl')"));
}

fn occurrence(module: Id, start: i64, end: i64, run: Id) -> OccurrencesRow {
    OccurrencesRow {
        occurrence_id: recipes::occurrence(module, start, end, "call"),
        module_id: module,
        start_byte: start,
        end_byte: end,
        syntax_kind: "call".into(),
        run_id: run,
    }
}

#[test]
fn rows_round_trip_through_their_batch() {
    let rows = vec![
        occurrence(Id([1; 16]), 0, 4, Id([9; 16])),
        occurrence(Id([1; 16]), 5, 9, Id([9; 16])),
    ];
    let batch = Occurrences::to_batch(&rows).unwrap();
    assert_eq!(Occurrences::from_batch(&batch).unwrap(), rows);
    let notes = vec![NotesRow {
        note_id: Id([3; 16]),
        subject_id: rows[0].occurrence_id,
        ordinal: 0,
        kind: IdKind::Transfer,
        label: None,
        tags: vec!["a".into(), String::new()],
        scores: vec![],
        weight: -0.0,
    }];
    let batch = Notes::to_batch(&notes).unwrap();
    let back = Notes::from_batch(&batch).unwrap();
    assert_eq!(back, notes);
    assert!(back[0].weight.is_sign_negative());
}

#[test]
fn a_foreign_batch_is_refused() {
    let batch = Occurrences::to_batch(&[occurrence(Id([1; 16]), 0, 1, Id([2; 16]))]).unwrap();
    assert!(Entities::from_batch(&batch).is_err());
}

/// Identity covers exactly the declared inputs: provenance never moves an id, an input always does.
#[test]
fn provenance_never_reaches_identity() {
    let base = occurrence(Id([1; 16]), 0, 4, Id([9; 16]));
    let batch = |row: &OccurrencesRow| Occurrences::to_batch(std::slice::from_ref(row)).unwrap();
    let ids = |b: &RecordBatch| identity::recompute(&Occurrences::DECL, b).unwrap().unwrap();

    let mut other_run = base.clone();
    other_run.run_id = Id([8; 16]);
    assert_eq!(ids(&batch(&base)), ids(&batch(&other_run)));
    assert!(identity::mismatches(&Occurrences::DECL, &batch(&other_run)).unwrap().is_empty());

    let mut other_module = base.clone();
    other_module.module_id = Id([2; 16]);
    assert_ne!(ids(&batch(&base)), ids(&batch(&other_module)));
    // The stored id no longer matches its inputs.
    assert_eq!(
        identity::mismatches(&Occurrences::DECL, &batch(&other_module)).unwrap(),
        [0]
    );
}

#[test]
fn declaration_defects_are_reported() {
    const PROVENANCE_INPUT: RelationDecl = RelationDecl {
        name: "bad",
        layer: Layer::L0,
        family: "sample",
        stage: "extract",
        columns: &[
            ColumnDecl {
                name: "bad_id",
                field: <Id as lctx_model::decl::column::ArrowColumn>::field,
                class: ColumnClass::Payload,
            },
            ColumnDecl {
                name: "run_id",
                field: <Id as lctx_model::decl::column::ArrowColumn>::field,
                class: ColumnClass::Provenance,
            },
            ColumnDecl {
                name: "owner",
                field: <Id as lctx_model::decl::column::ArrowColumn>::field,
                class: ColumnClass::Reference {
                    relation: "missing",
                    column: "missing_id",
                },
            },
            ColumnDecl {
                name: "maybe",
                field: <Option<Id> as lctx_model::decl::column::ArrowColumn>::field,
                class: ColumnClass::Payload,
            },
        ],
        key: &["maybe"],
        unique: &[],
        identity: Some(lctx_model::decl::relation::IdentityDecl {
            column: "bad_id",
            recipe: recipes::SYMBOL_KEY,
            inputs: &["run_id"],
        }),
        checks: &[],
        fidelity: FidelityClass::Extracted,
        polarity: Polarity::Exact,
        coverage: "module",
        exposure: Exposure::INTERNAL,
    };
    let errors = validate(&[&PROVENANCE_INPUT, &PROVENANCE_INPUT]).unwrap_err();
    let text: Vec<String> = errors.iter().map(ToString::to_string).collect();
    for expected in [
        "declared twice",
        "is nullable",
        "takes 1 inputs",
        "identity input run_id is provenance",
        "refers to undeclared relation missing",
    ] {
        assert!(
            text.iter().any(|t| t.contains(expected)),
            "{expected} not in {text:#?}"
        );
    }
}
