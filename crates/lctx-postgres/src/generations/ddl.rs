//! PostgreSQL is a lowering of the validated domain, never a public table specification.
use lctx_model::domain::{ContentHash, Scalar, ValidatedModel};
use sea_query::{ColumnDef, ColumnType, Expr, ForeignKey, Index, PostgresQueryBuilder, Table};
use super::GenerationId;

pub(super) struct Ddl { pub tables: Vec<String>, pub references: Vec<String> }
pub(super) fn generate(model: &ValidatedModel, generation: GenerationId) -> Ddl {
    let schema = generation.schema();
    let mut tables = Vec::new();
    let mut references = Vec::new();
    for relation in model.relations() {
        let mut table = Table::create();
        table.table((schema.clone(), relation.name()));
        table.col(ColumnDef::new("generation_id").binary().not_null().default(Expr::cust(format!("decode('{}', 'hex')", generation.hex()))));
        table.check(Expr::cust(format!("generation_id = decode('{}', 'hex')", generation.hex())));
        table.col(ColumnDef::new("id").binary().not_null());
        table.check(Expr::cust("octet_length(id) = 16"));
        table.primary_key(Index::create().col("generation_id").col("id"));
        for field in relation.fields() {
            let ty = match field.scalar() {
                Scalar::Text => ColumnType::Text, Scalar::Bool => ColumnType::Boolean,
                Scalar::Int16 => ColumnType::SmallInteger, Scalar::Int32 => ColumnType::Integer,
                Scalar::Int64 => ColumnType::BigInteger, Scalar::Id | Scalar::Digest => ColumnType::Binary(32),
            };
            let mut column = if field.list() {
                let mut column = ColumnDef::new(field.name()); column.array(ty); column
            } else { ColumnDef::new_with_type(field.name(), ty) };
            if !field.nullable() { column.not_null(); }
            table.col(&mut column);
            if !field.codes().is_empty() {
                let values = field.codes().iter().map(|(code, _)| code.to_string()).collect::<Vec<_>>().join(",");
                table.check(Expr::cust(format!("\"{}\" IN ({values})", field.name())));
            }
            if !field.list() && matches!(field.scalar(), Scalar::Id | Scalar::Digest) {
                let size = if field.scalar() == Scalar::Id { 16 } else { 32 };
                table.check(Expr::cust(format!("octet_length(\"{}\") = {size}", field.name())));
            }
            if let Some((_, target)) = field.target() {
                let constraint = format!("ref_{}", &ContentHash::of(format!("{}/{}", relation.name(), field.name()).as_bytes()).hex()[..32]);
                if let Some(code) = field.subtype() {
                    let tag_column = format!("__{}_tag", field.name());
                    table.col(ColumnDef::new(tag_column.clone()).small_integer().generated(Expr::val(code), true));
                    let tag = model.relations().iter().find(|r| r.name() == target).and_then(|r| r.sum()).expect("validated subtype").tag;
                    references.push(ForeignKey::create().name(constraint)
                        .from((schema.clone(), relation.name()), ("generation_id".to_owned(), field.name().to_owned(), tag_column))
                        .to((schema.clone(), target), ("generation_id", "id", tag)).to_string(PostgresQueryBuilder));
                } else {
                    references.push(ForeignKey::create().name(constraint)
                        .from((schema.clone(), relation.name()), ("generation_id", field.name()))
                        .to((schema.clone(), target), ("generation_id", "id")).to_string(PostgresQueryBuilder));
                }
            }
        }
        if let Some(sum) = relation.sum() {
            table.index(Index::create().unique().col("generation_id").col("id").col(sum.tag));
            let checks = sum.arms.iter().map(|arm| {
                let mut clauses = vec![format!("\"{}\" = {}", sum.tag, arm.code)];
                for field in relation.fields().iter().filter(|f| f.name() != sum.tag) {
                    match arm.fields.iter().find(|f| f.name == field.name()) {
                        Some(active) if active.required => clauses.push(format!("\"{}\" IS NOT NULL", field.name())),
                        Some(_) => {},
                        None => clauses.push(format!("\"{}\" IS NULL", field.name())),
                    }
                }
                format!("({})", clauses.join(" AND "))
            }).collect::<Vec<_>>().join(" OR ");
            table.check(Expr::cust(checks));
        }
        tables.push(table.to_string(PostgresQueryBuilder));
    }
    Ddl { tables, references }
}
pub(super) fn digest(model: &ValidatedModel) -> ContentHash {
    let ddl = generate(model, GenerationId([0;16]));
    ContentHash::of(format!("{}\n{}", include_str!("control.sql"), ddl.tables.into_iter().chain(ddl.references).collect::<Vec<_>>().join(";\n")).as_bytes())
}
