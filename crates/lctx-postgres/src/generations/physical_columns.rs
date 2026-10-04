//! PostgreSQL-only column conventions. Hidden lifecycle columns are not semantic fields.
use super::{GenerationId, quoted};
use lctx_model::domain::{Relation, Scalar, stages};
use sea_query::{ColumnDef, ColumnType, Expr};

#[derive(Debug, Clone, Copy)]
enum Value {
    Generation,
    Ordinary,
    Epoch,
    Subtype(i16),
}

#[derive(Debug, Clone)]
pub(super) struct Column {
    pub name: String,
    scalar: Scalar,
    list: bool,
    nullable: bool,
    value: Value,
}
impl Column {
    pub fn signature(&self) -> String {
        let base = match self.scalar {
            Scalar::Text => "text",
            Scalar::Bool => "boolean",
            Scalar::Int16 => "smallint",
            Scalar::Int32 => "integer",
            Scalar::Int64 => "bigint",
            Scalar::FiniteF64 => "double precision",
            Scalar::Id | Scalar::Digest | Scalar::Binary => "bytea",
        };
        format!(
            "{}:{base}{}:{}",
            self.name,
            if self.list { "[]" } else { "" },
            !self.nullable
        )
    }
    pub fn definition(&self, generation: GenerationId) -> ColumnDef {
        let ty = match self.scalar {
            Scalar::Text => ColumnType::Text,
            Scalar::Bool => ColumnType::Boolean,
            Scalar::Int16 => ColumnType::SmallInteger,
            Scalar::Int32 => ColumnType::Integer,
            Scalar::Int64 => ColumnType::BigInteger,
            Scalar::FiniteF64 => ColumnType::Double,
            Scalar::Id | Scalar::Digest | Scalar::Binary => ColumnType::Binary(32),
        };
        let mut column = if self.list {
            let mut column = ColumnDef::new(self.name.clone());
            column.array(ty);
            column
        } else {
            ColumnDef::new_with_type(self.name.clone(), ty)
        };
        if !self.nullable {
            column.not_null();
        }
        match self.value {
            Value::Generation => {
                column.default(Expr::cust(format!("decode('{}', 'hex')", generation.hex())));
            }
            Value::Epoch => {
                column.default(0);
            }
            Value::Subtype(code) => {
                column.generated(Expr::val(code), true);
            }
            Value::Ordinary => {}
        }
        column
    }
}
pub(super) fn columns(relation: &Relation) -> Vec<Column> {
    let mut columns = vec![
        Column {
            name: "generation_id".into(),
            scalar: Scalar::Id,
            list: false,
            nullable: false,
            value: Value::Generation,
        },
        Column {
            name: "id".into(),
            scalar: Scalar::Id,
            list: false,
            nullable: false,
            value: Value::Ordinary,
        },
    ];
    if stages::is_vocabulary(relation.name()) {
        columns.push(Column {
            name: "introduced_epoch".into(),
            scalar: Scalar::Int16,
            list: false,
            nullable: false,
            value: Value::Epoch,
        });
    }
    for field in relation.fields() {
        columns.push(Column {
            name: field.name().into(),
            scalar: field.scalar(),
            list: field.list(),
            nullable: field.nullable(),
            value: Value::Ordinary,
        });
        if let (Some(_), Some(code)) = (field.target(), field.subtype()) {
            columns.push(Column {
                name: format!("__{}_tag", field.name()),
                scalar: Scalar::Int16,
                list: false,
                nullable: true,
                value: Value::Subtype(code),
            });
        }
    }
    columns
}
pub(super) fn projection(relation: &Relation) -> String {
    columns(relation)
        .iter()
        .map(|column| quoted(&column.name))
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::*;
    use lctx_model::{
        Domain,
        domain::{Record, source, value},
    };
    #[derive(Debug, Clone, PartialEq, Domain)]
    #[model(name = "physical_column_controls")]
    struct Control {
        #[model(key)]
        key: String,
        optional: Option<i32>,
        names: Vec<String>,
        flag: bool,
        small: i16,
        wide: i64,
        fraction: lctx_model::domain::FiniteF64,
        digest: lctx_model::domain::ContentHash,
        literal: lctx_model::domain::ArmId<value::Literal, 0>,
    }
    #[test]
    fn independent_physical_shapes_preserve_hidden_and_semantic_boundaries() {
        let relation = Relation::of::<Control>();
        assert_eq!(
            columns(&relation)
                .iter()
                .map(Column::signature)
                .collect::<Vec<_>>(),
            [
                "generation_id:bytea:true",
                "id:bytea:true",
                "key:text:true",
                "optional:integer:false",
                "names:text[]:true",
                "flag:boolean:true",
                "small:smallint:true",
                "wide:bigint:true",
                "fraction:double precision:true",
                "digest:bytea:true",
                "literal:bytea:true",
                "__literal_tag:smallint:false",
            ]
        );
        assert_eq!(relation.fields().len(), 9);
        assert_eq!(
            columns(&Relation::of::<source::SourceArtifact>())[0].name,
            "generation_id"
        );
        let literal = columns(&Relation::of::<value::Literal>());
        assert_eq!(literal[2].signature(), "introduced_epoch:smallint:true");
        assert!(
            literal
                .iter()
                .any(|column| column.signature() == "string_value:bytea:false")
        );
        assert!(
            !<Control as Record>::schema()
                .fields()
                .iter()
                .any(|field| field.name() == "generation_id")
        );
    }
}
