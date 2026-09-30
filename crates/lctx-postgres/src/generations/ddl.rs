//! PostgreSQL is a lowering of the validated domain, never a public table specification. The
//! lowering is pure: one model, generation, schema name and control schema name always give the
//! same statements, so a shadow install is a faithful reference for `store check` (plan P1.6).
use super::failure::FailureClass;
use super::{GenerationId, quoted};
use lctx_model::domain::{
    ContentHash, FieldValue, Relation, Scalar, ValidatedModel,
    admission::{Availability, Frontier, FrontierContract},
    attribution::FactFamily,
    stages::{Profile, ProviderOutcome},
};
use sea_query::{ColumnDef, ColumnType, Expr, ForeignKey, Index, PostgresQueryBuilder, Table};
use std::collections::{BTreeMap, BTreeSet};

/// The stable control schema of an installed store.
pub(super) const CONTROL: &str = "lctx_model_store";
/// Lifecycle states in the order a generation passes through them.
pub(super) const STATES: [&str; 4] = ["staging", "sealed", "validated", "published"];
/// The terminal state of a refused attempt. It keeps the physical shape of the state it failed
/// from, with the writer revoked, and permits only abort.
pub(super) const FAILED: &str = "failed";

/// The relations one frontier's generations lower, and the physical digest of that lowering.
#[derive(Debug, Clone)]
pub(super) struct Scope {
    pub relations: BTreeSet<&'static str>,
    pub physical: ContentHash,
    pub columns: String,
}
/// A conformance generation lowers the whole model; a facts generation lowers only the facts
/// relations, so nothing above its frontier exists to read as empty (P0 exit F02). A model without
/// every facts relation has no facts scope.
pub(super) fn scopes(model: &ValidatedModel) -> BTreeMap<Frontier, Scope> {
    let scope = |relations: BTreeSet<&'static str>| Scope {
        physical: physical_digest(model, &relations),
        columns: column_signature(model, &relations),
        relations,
    };
    let mut scopes = BTreeMap::from([(
        Frontier::Conformance,
        scope(model.relations().iter().map(Relation::name).collect()),
    )]);
    if let Ok(contract) = FrontierContract::facts(model, Profile::Catalog) {
        scopes.insert(
            Frontier::Facts,
            scope(
                model
                    .relations()
                    .iter()
                    .map(Relation::name)
                    .filter(|name| contract.contains(name))
                    .collect(),
            ),
        );
    }
    scopes
}
/// The live-column signature of a lowering: `relation.column:type:not-null`, in relation-name
/// and column order, exactly as `pg_attribute` and `format_type` report the tables `lower` creates.
fn column_signature(model: &ValidatedModel, relations: &BTreeSet<&str>) -> String {
    let mut held: Vec<&Relation> = model
        .relations()
        .iter()
        .filter(|r| relations.contains(r.name()))
        .collect();
    held.sort_by_key(|r| r.name());
    let mut parts = Vec::new();
    for relation in held {
        let name = relation.name();
        parts.push(format!("{name}.generation_id:bytea:true"));
        parts.push(format!("{name}.id:bytea:true"));
        for field in relation.fields() {
            let base = match field.scalar() {
                Scalar::Text => "text",
                Scalar::Bool => "boolean",
                Scalar::Int16 => "smallint",
                Scalar::Int32 => "integer",
                Scalar::Int64 => "bigint",
                Scalar::Id | Scalar::Digest | Scalar::Binary => "bytea",
            };
            parts.push(format!(
                "{name}.{}:{base}{}:{}",
                field.name(),
                if field.list() { "[]" } else { "" },
                !field.nullable()
            ));
            if field.target().is_some() && field.subtype().is_some() {
                parts.push(format!("{name}.__{}_tag:smallint:false", field.name()));
            }
        }
    }
    parts.join(",")
}
/// The live-column signature of one generation schema, in the same form.
pub(super) const LIVE_COLUMNS: &str = "SELECT COALESCE(string_agg(c.relname || '.' || a.attname || ':' || format_type(a.atttypid, a.atttypmod) || ':' || a.attnotnull::text, \
    ',' ORDER BY c.relname COLLATE \"C\", a.attnum), '') FROM pg_attribute a JOIN pg_class c ON c.oid = a.attrelid JOIN pg_namespace n ON n.oid = c.relnamespace \
    WHERE n.nspname = $1 AND c.relkind = 'r' AND a.attnum > 0 AND NOT a.attisdropped";

/// The frontier a registry row names.
pub(super) fn frontier(name: &str) -> Option<Frontier> {
    Frontier::ALL.into_iter().find(|f| f.name() == name)
}

/// The statements that give a generation its physical form, grouped by the lifecycle transition
/// that executes them.
pub(super) struct Lowering {
    pub schema: String,
    pub tables: Vec<String>,
    pub views: Vec<String>,
    pub grant_staging: Vec<String>,
    pub revoke_writer: Vec<String>,
    pub references: Vec<String>,
    pub grant_reader: Vec<String>,
}
impl Lowering {
    /// Entering `state`: staging creates the schema, tables, views and writer grants; sealing
    /// revokes the writer; validation adds references; publication grants the reader.
    pub fn phase(&self, state: &str) -> Vec<String> {
        match state {
            "staging" => std::iter::once(format!("CREATE SCHEMA {}", quoted(&self.schema)))
                .chain(
                    self.tables
                        .iter()
                        .chain(&self.views)
                        .chain(&self.grant_staging)
                        .cloned(),
                )
                .collect(),
            "sealed" => self.revoke_writer.clone(),
            "validated" => self.references.clone(),
            "published" => self.grant_reader.clone(),
            _ => Vec::new(),
        }
    }
    /// Every statement a generation in `state` has executed, in order.
    pub fn through(&self, state: &str) -> Vec<String> {
        let end = STATES.iter().position(|s| *s == state).map_or(0, |i| i + 1);
        STATES[..end].iter().flat_map(|s| self.phase(s)).collect()
    }
}

/// The control schema DDL named `control`, with CHECK lists rendered from the lifecycle and the
/// model's enums.
pub(super) fn control(control: &str) -> String {
    let list = |names: &mut dyn Iterator<Item = &str>| {
        names
            .map(|n| format!("'{n}'"))
            .collect::<Vec<_>>()
            .join(",")
    };
    let codes = |codes: &mut dyn Iterator<Item = i16>| {
        codes.map(|c| c.to_string()).collect::<Vec<_>>().join(",")
    };
    include_str!("control.sql")
        .replace("{control}", &quoted(control))
        .replace(
            "{lifecycle}",
            &list(&mut STATES.into_iter().chain([FAILED])),
        )
        .replace(
            "{outcomes}",
            &ProviderOutcome::ALL
                .iter()
                .map(|o| o.code().to_string())
                .collect::<Vec<_>>()
                .join(","),
        )
        .replace(
            "{classes}",
            &list(&mut FailureClass::ALL.iter().map(|c| c.name())),
        )
        .replace(
            "{families}",
            &codes(
                &mut <FactFamily as FieldValue>::codes()
                    .iter()
                    .map(|(code, _)| *code),
            ),
        )
        .replace(
            "{availabilities}",
            &codes(&mut Availability::ALL.iter().map(|a| a.code())),
        )
        .replace(
            "{profiles}",
            &list(&mut Profile::ALL.iter().map(|p| p.name())),
        )
        .replace(
            "{frontiers}",
            &list(&mut Frontier::ALL.iter().map(|f| f.name())),
        )
}

pub(super) fn lower(
    model: &ValidatedModel,
    relations: &BTreeSet<&str>,
    generation: GenerationId,
    schema: &str,
    control: &str,
) -> Lowering {
    let schema = schema.to_owned();
    let mut tables = Vec::new();
    let mut references = Vec::new();
    for relation in model
        .relations()
        .iter()
        .filter(|r| relations.contains(r.name()))
    {
        let mut table = Table::create();
        let mut wire_sizes = vec!["42::bigint".to_owned()];
        table.table((schema.clone(), relation.name()));
        table.col(
            ColumnDef::new("generation_id")
                .binary()
                .not_null()
                .default(Expr::cust(format!("decode('{}', 'hex')", generation.hex()))),
        );
        table.check(Expr::cust(format!(
            "generation_id = decode('{}', 'hex')",
            generation.hex()
        )));
        table.col(ColumnDef::new("id").binary().not_null());
        table.check(Expr::cust("octet_length(id) = 16"));
        table.primary_key(Index::create().col("generation_id").col("id"));
        for field in relation.fields() {
            let ty = match field.scalar() {
                Scalar::Text => ColumnType::Text,
                Scalar::Bool => ColumnType::Boolean,
                Scalar::Int16 => ColumnType::SmallInteger,
                Scalar::Int32 => ColumnType::Integer,
                Scalar::Int64 => ColumnType::BigInteger,
                Scalar::Id | Scalar::Digest | Scalar::Binary => ColumnType::Binary(32),
            };
            let mut column = if field.list() {
                let mut column = ColumnDef::new(field.name());
                column.array(ty);
                column
            } else {
                ColumnDef::new_with_type(field.name(), ty)
            };
            if !field.nullable() {
                column.not_null();
            }
            table.col(&mut column);
            let column_name = format!("\"{}\"", field.name());
            let scalar_width = match field.scalar() {
                Scalar::Bool => 1,
                Scalar::Int16 => 2,
                Scalar::Int32 => 4,
                Scalar::Int64 => 8,
                Scalar::Id => 16,
                Scalar::Digest => 32,
                Scalar::Text | Scalar::Binary => 0,
            };
            let size = if field.list() {
                table.check(Expr::cust(format!("CASE WHEN array_ndims({column_name}) > 1 THEN FALSE ELSE (COALESCE(array_lower({column_name},1),1)=1 AND array_position({column_name},NULL) IS NULL) END")));
                if field.scalar() == Scalar::Text {
                    format!("{}.text_array_wire_bytes({column_name})", quoted(control))
                } else {
                    format!(
                        "20::bigint + COALESCE(cardinality({column_name}),0)::bigint * {}",
                        scalar_width + 4
                    )
                }
            } else if matches!(field.scalar(), Scalar::Text | Scalar::Binary) {
                format!("COALESCE(octet_length({column_name}),0)::bigint")
            } else {
                scalar_width.to_string()
            };
            wire_sizes.push(format!("4::bigint + ({size})"));
            if !field.codes().is_empty() {
                let values = field
                    .codes()
                    .iter()
                    .map(|(code, _)| code.to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                table.check(Expr::cust(format!("\"{}\" IN ({values})", field.name())));
            }
            if !field.list() && matches!(field.scalar(), Scalar::Id | Scalar::Digest) {
                let size = if field.scalar() == Scalar::Id { 16 } else { 32 };
                table.check(Expr::cust(format!(
                    "octet_length(\"{}\") = {size}",
                    field.name()
                )));
            }
            if let Some((_, target)) = field.target() {
                let constraint = format!(
                    "ref_{}",
                    &ContentHash::of(format!("{}/{}", relation.name(), field.name()).as_bytes())
                        .hex()[..32]
                );
                if let Some(code) = field.subtype() {
                    let tag_column = format!("__{}_tag", field.name());
                    table.col(
                        ColumnDef::new(tag_column.clone())
                            .small_integer()
                            .generated(Expr::val(code), true),
                    );
                    let tag = model
                        .relations()
                        .iter()
                        .find(|r| r.name() == target)
                        .and_then(|r| r.sum())
                        .expect("validated subtype")
                        .tag;
                    references.push(
                        ForeignKey::create()
                            .name(constraint)
                            .from(
                                (schema.clone(), relation.name()),
                                (
                                    "generation_id".to_owned(),
                                    field.name().to_owned(),
                                    tag_column,
                                ),
                            )
                            .to((schema.clone(), target), ("generation_id", "id", tag))
                            .to_string(PostgresQueryBuilder),
                    );
                } else {
                    references.push(
                        ForeignKey::create()
                            .name(constraint)
                            .from(
                                (schema.clone(), relation.name()),
                                ("generation_id", field.name()),
                            )
                            .to((schema.clone(), target), ("generation_id", "id"))
                            .to_string(PostgresQueryBuilder),
                    );
                }
            }
        }
        table.check(Expr::cust(format!(
            "({}) <= {}",
            wire_sizes.join(" + "),
            super::MAX_ROW_BYTES
        )));
        if let Some(sum) = relation.sum() {
            table.index(
                Index::create()
                    .unique()
                    .col("generation_id")
                    .col("id")
                    .col(sum.tag),
            );
            let checks = sum
                .arms
                .iter()
                .map(|arm| {
                    let mut clauses = vec![format!("\"{}\" = {}", sum.tag, arm.code)];
                    for field in relation.fields().iter().filter(|f| f.name() != sum.tag) {
                        match arm.fields.iter().find(|f| f.name == field.name()) {
                            Some(active) if active.required => {
                                clauses.push(format!("\"{}\" IS NOT NULL", field.name()))
                            }
                            Some(_) => {}
                            None => clauses.push(format!("\"{}\" IS NULL", field.name())),
                        }
                    }
                    format!("({})", clauses.join(" AND "))
                })
                .collect::<Vec<_>>()
                .join(" OR ");
            table.check(Expr::cust(checks));
        }
        tables.push(table.to_string(PostgresQueryBuilder));
    }
    let s = quoted(&schema);
    Lowering {
        views: derivation_views(model, relations, &schema),
        grant_staging: vec![
            format!("GRANT USAGE ON SCHEMA {s} TO lctx_importer"),
            format!("GRANT INSERT ON ALL TABLES IN SCHEMA {s} TO lctx_importer"),
        ],
        revoke_writer: vec![
            format!("REVOKE ALL ON ALL TABLES IN SCHEMA {s} FROM lctx_importer"),
            format!("REVOKE USAGE ON SCHEMA {s} FROM lctx_importer"),
        ],
        grant_reader: vec![
            format!("GRANT USAGE ON SCHEMA {s} TO lctx_serving"),
            format!("GRANT SELECT ON ALL TABLES IN SCHEMA {s} TO lctx_serving"),
        ],
        schema,
        tables,
        references,
    }
}
/// The physical digest covers the control DDL and every phase of a generation's lowering of
/// `relations`.
fn physical_digest(model: &ValidatedModel, relations: &BTreeSet<&str>) -> ContentHash {
    let g = GenerationId([0; 16]);
    let lowering = lower(model, relations, g, &g.schema(), CONTROL);
    ContentHash::of(
        format!(
            "{}\n{}",
            control(CONTROL),
            lowering.through("published").join(";\n")
        )
        .as_bytes(),
    )
}

fn literal(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}
/// Explanation targets and premise roles are projected from nominal model fields. No separately
/// maintained target registry, copied evidence table, or executable relation-name payload is used.
fn derivation_views(
    model: &ValidatedModel,
    relations: &BTreeSet<&str>,
    schema: &str,
) -> Vec<String> {
    let mut derivations = Vec::new();
    let mut premises = Vec::new();
    for source in model
        .relations()
        .iter()
        .filter(|r| relations.contains(r.name()))
    {
        let Some(rule) = source.derivation() else {
            continue;
        };
        let (target, column) = rule
            .conclusion
            .as_ref()
            .map_or((source.name(), "id"), |c| (c.target().1, c.name()));
        let table = format!("{}.{}", quoted(schema), quoted(source.name()));
        let source_name = literal(source.name());
        derivations.push(format!("SELECT generation_id,id AS derivation_id,{source_name}::text AS source_relation,{}::text AS rule,{}::text AS conclusion_relation,{} AS conclusion_id FROM {table}",literal(rule.rule),literal(target),quoted(column)));
        for premise in &rule.premises {
            let column = quoted(premise.name());
            premises.push(format!("SELECT generation_id,id AS derivation_id,{source_name}::text AS source_relation,{}::text AS role,{}::text AS premise_relation,{column} AS premise_id FROM {table} WHERE {column} IS NOT NULL",literal(premise.name()),literal(premise.target().1)));
        }
    }
    if derivations.is_empty() {
        return Vec::new();
    }
    [
        ("derivations", derivations),
        ("derivation_premises", premises),
    ]
    .into_iter()
    .map(|(name, parts)| {
        format!(
            "CREATE VIEW {}.{} AS {}",
            quoted(schema),
            quoted(name),
            parts.join(" UNION ALL ")
        )
    })
    .collect()
}
