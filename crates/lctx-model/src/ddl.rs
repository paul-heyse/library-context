//! Generated PostgreSQL DDL (DESIGN §15.2, §15.11; cutover plan §4.1 D3–D4).
//!
//! Every canonical table is rendered from a [`TableSpec`], which is built from a relation
//! declaration ([`TableSpec::from_decl`]) or, during the cutover, from a legacy contract
//! ([`TableSpec::build`]). Nothing here is written by hand a second time:
//! - a list-partitioned **parent** per relation in the canonical schema, with its key,
//!   NOT NULL, CHECKs, fixed-width checks, generation-qualified references and codebook
//!   references;
//! - **templates** for a generation's staging table, its key indexes, the partition attach and
//!   detach. The store's SECURITY DEFINER lifecycle functions substitute only `{schema}` and
//!   `{generation}` into them, so a writer never supplies DDL;
//! - codebook tables with their rows, reader grants and view text.
//!
//! The rendered install script has one digest ([`Install::digest`]) that the store records and
//! checks before every generation.

use std::fmt::Write as _;

use arrow_schema::{DataType, Schema};

use crate::decl::codebook::CodebookEntry;
use crate::decl::column::CODEBOOK_KEY;
use crate::decl::relation::{ColumnClass, GENERATION_COLUMN, RelationDecl};
use crate::id::{Digest, Id};

/// Placeholder for the generation's schema, substituted by the store.
pub const SCHEMA_SLOT: &str = "{schema}";
/// Placeholder for the generation id's 32 lowercase hex digits, substituted by the store.
pub const GENERATION_SLOT: &str = "{generation}";

/// Where the rendered objects live and who reads them.
#[derive(Debug, Clone, Copy)]
pub struct DdlConfig {
    /// The canonical schema holding the parents and codebook tables.
    pub schema: &'static str,
    /// The role granted read access to published relations.
    pub reader: &'static str,
}

/// A rendering failure: an unsupported type, an unknown column or an over-long name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DdlError(pub String);

impl std::fmt::Display for DdlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for DdlError {}

/// The PostgreSQL column type of an Arrow type. The mapping is the binary COPY encoder's
/// (pgpq): the store asserts they agree for every declared type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SqlType {
    Bytea,
    Text,
    SmallInt,
    BigInt,
    Boolean,
    Double,
    Real,
    Array(Box<SqlType>),
}

impl SqlType {
    /// The type of `data_type`, and the fixed byte width a `FixedSizeBinary` must keep.
    pub fn of(data_type: &DataType) -> Result<(Self, Option<i32>), DdlError> {
        Ok(match data_type {
            DataType::FixedSizeBinary(n) => (Self::Bytea, Some(*n)),
            DataType::Binary => (Self::Bytea, None),
            DataType::Utf8 => (Self::Text, None),
            DataType::Int16 => (Self::SmallInt, None),
            DataType::Int64 => (Self::BigInt, None),
            DataType::Boolean => (Self::Boolean, None),
            DataType::Float64 => (Self::Double, None),
            DataType::Float32 => (Self::Real, None),
            DataType::List(item) => match Self::of(item.data_type())? {
                (Self::Array(_), _) => {
                    return Err(DdlError(format!("nested list {data_type} has no column type")));
                }
                (_, Some(_)) => {
                    return Err(DdlError(format!("list of fixed-width binary {data_type}")));
                }
                (inner, None) => (Self::Array(Box::new(inner)), None),
            },
            other => return Err(DdlError(format!("Arrow type {other} has no column type"))),
        })
    }

    pub fn sql(&self) -> String {
        match self {
            Self::Bytea => "bytea".into(),
            Self::Text => "text".into(),
            Self::SmallInt => "smallint".into(),
            Self::BigInt => "bigint".into(),
            Self::Boolean => "boolean".into(),
            Self::Double => "double precision".into(),
            Self::Real => "real".into(),
            Self::Array(inner) => format!("{}[]", inner.sql()),
        }
    }
}

/// One column of a canonical table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnSpec {
    pub name: String,
    pub sql_type: SqlType,
    pub fixed_width: Option<i32>,
    pub nullable: bool,
    pub codebook: Option<String>,
}

/// A unique key. `nulls_not_distinct` makes two nulls collide, as a key requires.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UniqueSpec {
    pub columns: Vec<String>,
    pub nulls_not_distinct: bool,
}

/// A generation-qualified reference from `columns` to `relation`'s `target` columns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForeignKeySpec {
    pub columns: Vec<String>,
    pub relation: String,
    pub target: Vec<String>,
}

/// The partition-key column. `supplied` means the store fills it (a default on the staging
/// table) rather than the producer's batch carrying it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Partition {
    pub column: String,
    pub supplied: bool,
}

/// One canonical table, ready to render.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableSpec {
    pub name: String,
    pub partition: Partition,
    /// Every column in table order; the partition column first when supplied.
    pub columns: Vec<ColumnSpec>,
    /// Includes the partition column.
    pub primary_key: Option<Vec<String>>,
    pub unique: Vec<UniqueSpec>,
    pub checks: Vec<(String, String)>,
    pub foreign_keys: Vec<ForeignKeySpec>,
    /// Whether codebook columns reference their codebook tables.
    pub codebook_references: bool,
    /// Lookup indexes beyond the keys.
    pub indexes: Vec<Vec<String>>,
    pub serve: bool,
}

/// The inputs of [`TableSpec::build`] beyond the schema.
#[derive(Debug, Clone, Default)]
pub struct TableOptions<'a> {
    /// The table key; the partition column must be in it.
    pub key: &'a [&'a str],
    pub unique: &'a [&'a [&'a str]],
    pub checks: &'a [(&'a str, &'a str)],
    pub foreign_keys: Vec<ForeignKeySpec>,
    pub codebook_references: bool,
    pub indexes: &'a [&'a [&'a str]],
    pub serve: bool,
}

impl TableSpec {
    /// A table from an Arrow schema whose fields include `partition` (a legacy contract, whose
    /// batches carry their envelope column). A key with a nullable column becomes a unique key
    /// with `NULLS NOT DISTINCT`, since a primary key admits no null.
    pub fn build(
        name: &str,
        schema: &Schema,
        partition: &str,
        options: &TableOptions<'_>,
    ) -> Result<Self, DdlError> {
        check_name(name)?;
        let columns = schema
            .fields()
            .iter()
            .map(|f| column(f.name(), f.data_type(), f.is_nullable(), f.metadata().get(CODEBOOK_KEY)))
            .collect::<Result<Vec<_>, _>>()?;
        let spec_columns = |cols: &[&str]| -> Result<Vec<String>, DdlError> {
            cols.iter()
                .map(|c| {
                    columns
                        .iter()
                        .any(|s| s.name == *c)
                        .then(|| (*c).to_owned())
                        .ok_or_else(|| DdlError(format!("{name}: no column {c}")))
                })
                .collect()
        };
        if !columns.iter().any(|c| c.name == partition && !c.nullable) {
            return Err(DdlError(format!("{name}: no non-null partition column {partition}")));
        }
        if !options.key.contains(&partition) {
            return Err(DdlError(format!("{name}: the key omits {partition}")));
        }
        let key = spec_columns(options.key)?;
        let nullable_key = key
            .iter()
            .any(|k| columns.iter().any(|c| &c.name == k && c.nullable));
        let mut unique = Vec::new();
        let primary_key = if nullable_key {
            unique.push(UniqueSpec {
                columns: key,
                nulls_not_distinct: true,
            });
            None
        } else {
            Some(key)
        };
        for u in options.unique {
            let mut cols = spec_columns(u)?;
            if !cols.iter().any(|c| c == partition) {
                cols.insert(0, partition.to_owned());
            }
            unique.push(UniqueSpec {
                columns: cols,
                nulls_not_distinct: false,
            });
        }
        let indexes = options
            .indexes
            .iter()
            .map(|i| spec_columns(i))
            .collect::<Result<_, _>>()?;
        Ok(Self {
            name: name.to_owned(),
            partition: Partition {
                column: partition.to_owned(),
                supplied: false,
            },
            columns,
            primary_key,
            unique,
            checks: options
                .checks
                .iter()
                .map(|(n, e)| ((*n).to_owned(), (*e).to_owned()))
                .collect(),
            foreign_keys: options.foreign_keys.clone(),
            codebook_references: options.codebook_references,
            indexes,
            serve: options.serve,
        })
    }

    /// A model relation's table: the store-supplied generation column first and in every key,
    /// generation-qualified references from its roles, and codebook references.
    pub fn from_decl(decl: &RelationDecl) -> Result<Self, DdlError> {
        let mut fields = vec![arrow_schema::Field::new(
            GENERATION_COLUMN,
            DataType::FixedSizeBinary(16),
            false,
        )];
        fields.extend(decl.columns.iter().map(|c| c.arrow()));
        let schema = Schema::new(fields);
        let key: Vec<&str> = std::iter::once(GENERATION_COLUMN)
            .chain(decl.key.iter().copied())
            .collect();
        let foreign_keys = decl
            .columns
            .iter()
            .filter_map(|c| match c.class {
                ColumnClass::Reference { relation, column } => Some(ForeignKeySpec {
                    columns: vec![GENERATION_COLUMN.to_owned(), c.name.to_owned()],
                    relation: relation.to_owned(),
                    target: vec![GENERATION_COLUMN.to_owned(), column.to_owned()],
                }),
                _ => None,
            })
            .collect();
        let mut spec = Self::build(
            decl.name,
            &schema,
            GENERATION_COLUMN,
            &TableOptions {
                key: &key,
                unique: decl.unique,
                checks: decl.checks,
                foreign_keys,
                codebook_references: true,
                indexes: decl.exposure.lookups,
                serve: decl.exposure.serve,
            },
        )?;
        spec.partition.supplied = true;
        Ok(spec)
    }

    /// The columns a producer's batch carries, in COPY order.
    pub fn copy_columns(&self) -> Vec<&str> {
        self.columns
            .iter()
            .filter(|c| !(self.partition.supplied && c.name == self.partition.column))
            .map(|c| c.name.as_str())
            .collect()
    }

    fn column_definitions(&self, staging: bool) -> Vec<String> {
        self.columns
            .iter()
            .map(|c| {
                let mut out = format!("{} {}", quote(&c.name), c.sql_type.sql());
                if staging && self.partition.supplied && c.name == self.partition.column {
                    let _ = write!(out, " DEFAULT '\\x{GENERATION_SLOT}'::bytea");
                }
                if !c.nullable {
                    out.push_str(" NOT NULL");
                }
                out
            })
            .collect()
    }

    /// Row-local constraints shared by the parent and the staging table.
    fn row_checks(&self) -> Vec<String> {
        let mut out = Vec::new();
        for (i, c) in self.columns.iter().enumerate() {
            if let Some(width) = c.fixed_width {
                out.push(format!(
                    "CONSTRAINT {} CHECK (octet_length({}) = {width})",
                    quote(&format!("width_{i}")),
                    quote(&c.name)
                ));
            }
        }
        for (name, expr) in &self.checks {
            out.push(format!("CONSTRAINT {} CHECK ({expr})", quote(name)));
        }
        out
    }

    /// The partitioned parent, its key, references and lookup indexes.
    pub fn parent(&self, cfg: &DdlConfig) -> Vec<String> {
        let table = qualified(cfg.schema, &self.name);
        let mut parts = self.column_definitions(false);
        parts.extend(self.row_checks());
        if let Some(pk) = &self.primary_key {
            parts.push(format!(
                "CONSTRAINT {} PRIMARY KEY ({})",
                quote(&format!("{}_pk", self.name)),
                list(pk)
            ));
        }
        for (i, u) in self.unique.iter().enumerate() {
            parts.push(format!(
                "CONSTRAINT {} UNIQUE{} ({})",
                quote(&format!("{}_u{i}", self.name)),
                if u.nulls_not_distinct {
                    " NULLS NOT DISTINCT"
                } else {
                    ""
                },
                list(&u.columns)
            ));
        }
        for (i, fk) in self.foreign_keys.iter().enumerate() {
            parts.push(format!(
                "CONSTRAINT {} FOREIGN KEY ({}) REFERENCES {} ({})",
                quote(&format!("{}_fk{i}", self.name)),
                list(&fk.columns),
                qualified(cfg.schema, &fk.relation),
                list(&fk.target)
            ));
        }
        if self.codebook_references {
            for (i, c) in self.columns.iter().enumerate() {
                if let Some(codebook) = &c.codebook {
                    parts.push(format!(
                        "CONSTRAINT {} FOREIGN KEY ({}) REFERENCES {} (\"code\")",
                        quote(&format!("{}_cb{i}", self.name)),
                        quote(&c.name),
                        qualified(cfg.schema, &codebook_table(codebook))
                    ));
                }
            }
        }
        let mut out = vec![format!(
            "CREATE TABLE {table} (\n    {}\n) PARTITION BY LIST ({})",
            parts.join(",\n    "),
            quote(&self.partition.column)
        )];
        for (i, index) in self.indexes.iter().enumerate() {
            out.push(format!(
                "CREATE INDEX {} ON {table} ({})",
                quote(&format!("{}_i{i}", self.name)),
                list(index)
            ));
        }
        if self.serve {
            out.push(format!("GRANT SELECT ON {table} TO {}", quote(cfg.reader)));
        }
        out
    }

    /// Template: the generation's staging table, with the partition CHECK that lets the attach
    /// skip its validation scan. Keys and references are left to [`Self::staging_indexes`] and
    /// the attach.
    pub fn staging_table(&self) -> String {
        let mut parts = self.column_definitions(true);
        parts.extend(self.row_checks());
        parts.push(format!(
            "CONSTRAINT \"partition\" CHECK ({} = '\\x{GENERATION_SLOT}'::bytea)",
            quote(&self.partition.column)
        ));
        format!(
            "CREATE TABLE {SCHEMA_SLOT}.{} (\n    {}\n)",
            quote(&self.name),
            parts.join(",\n    ")
        )
    }

    /// Templates: the key and lookup indexes, built on the staging table before validation, so
    /// the attach finds them instead of building them inside the publish transaction.
    pub fn staging_indexes(&self) -> Vec<String> {
        let table = format!("{SCHEMA_SLOT}.{}", quote(&self.name));
        let mut out = Vec::new();
        if let Some(pk) = &self.primary_key {
            out.push(format!(
                "CREATE UNIQUE INDEX {} ON {table} ({})",
                quote(&format!("{}_pk", self.name)),
                list(pk)
            ));
        }
        for (i, u) in self.unique.iter().enumerate() {
            out.push(format!(
                "CREATE UNIQUE INDEX {} ON {table} ({}){}",
                quote(&format!("{}_u{i}", self.name)),
                list(&u.columns),
                if u.nulls_not_distinct {
                    " NULLS NOT DISTINCT"
                } else {
                    ""
                }
            ));
        }
        for (i, index) in self.indexes.iter().enumerate() {
            out.push(format!(
                "CREATE INDEX {} ON {table} ({})",
                quote(&format!("{}_i{i}", self.name)),
                list(index)
            ));
        }
        out
    }

    /// Template: attach the staging table as the generation's partition.
    pub fn attach(&self, cfg: &DdlConfig) -> String {
        format!(
            "ALTER TABLE {} ATTACH PARTITION {SCHEMA_SLOT}.{} FOR VALUES IN ('\\x{GENERATION_SLOT}'::bytea)",
            qualified(cfg.schema, &self.name),
            quote(&self.name)
        )
    }

    /// Template: detach the generation's partition without blocking readers of the parent.
    pub fn detach(&self, cfg: &DdlConfig) -> String {
        format!(
            "ALTER TABLE {} DETACH PARTITION {SCHEMA_SLOT}.{} CONCURRENTLY",
            qualified(cfg.schema, &self.name),
            quote(&self.name)
        )
    }
}

fn column(
    name: &str,
    data_type: &DataType,
    nullable: bool,
    codebook: Option<&String>,
) -> Result<ColumnSpec, DdlError> {
    if name.len() > 63 {
        return Err(DdlError(format!("column {name} exceeds 63 bytes")));
    }
    let (sql_type, fixed_width) = SqlType::of(data_type)?;
    Ok(ColumnSpec {
        name: name.to_owned(),
        sql_type,
        fixed_width,
        nullable,
        codebook: codebook.cloned(),
    })
}

/// The longest table name, so every derived constraint and index name (`<name>_cb12`) stays
/// within 63 bytes.
fn check_name(name: &str) -> Result<(), DdlError> {
    if name.is_empty() || name.len() > crate::decl::relation::MAX_NAME {
        return Err(DdlError(format!(
            "table name {name:?} must be 1..={} bytes",
            crate::decl::relation::MAX_NAME
        )));
    }
    Ok(())
}

/// A double-quoted identifier.
pub fn quote(ident: &str) -> String {
    format!("\"{}\"", ident.replace('"', "\"\""))
}

fn qualified(schema: &str, name: &str) -> String {
    format!("{}.{}", quote(schema), quote(name))
}

fn list(columns: &[String]) -> String {
    columns
        .iter()
        .map(|c| quote(c))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The table holding one codebook's rows.
pub fn codebook_table(name: &str) -> String {
    format!("codebook_{name}")
}

/// A codebook table and its rows. Codes are append-only, so rows are only ever added.
pub fn codebook(entry: &CodebookEntry, cfg: &DdlConfig) -> Vec<String> {
    let table = qualified(cfg.schema, &codebook_table(entry.name));
    let mut out = vec![format!(
        "CREATE TABLE {table} (\"code\" smallint PRIMARY KEY, \"text\" text NOT NULL UNIQUE)"
    )];
    if !entry.values.is_empty() {
        let rows: Vec<String> = entry
            .values
            .iter()
            .map(|(code, text)| format!("({code}, '{}')", text.replace('\'', "''")))
            .collect();
        out.push(format!(
            "INSERT INTO {table} (\"code\", \"text\") VALUES {}",
            rows.join(", ")
        ));
    }
    out.push(format!("GRANT SELECT ON {table} TO {}", quote(cfg.reader)));
    out
}

/// A read-only view over canonical relations, evaluated with the reader's privileges.
pub fn view(name: &str, sql: &str, cfg: &DdlConfig) -> String {
    format!(
        "CREATE VIEW {} WITH (security_invoker = true) AS\n{sql}",
        qualified(cfg.schema, name)
    )
}

/// A generation's schema: `lctx_g` and the id's 32 hex digits.
pub fn generation_schema(generation: Id) -> String {
    format!("lctx_g{}", generation.hex())
}

/// A complete, ordered install script and its digest.
#[derive(Debug, Clone)]
pub struct Install {
    pub statements: Vec<String>,
    pub tables: Vec<TableSpec>,
    pub digest: Digest,
}

/// Render the whole canonical schema: codebook tables first, then parents in dependency order
/// (a referenced relation before its referrer). The digest covers every statement and template.
pub fn install(
    tables: Vec<TableSpec>,
    codebooks: &[CodebookEntry],
    cfg: &DdlConfig,
) -> Result<Install, DdlError> {
    let ordered = dependency_order(tables)?;
    let mut statements = Vec::new();
    for entry in codebooks {
        statements.extend(codebook(entry, cfg));
    }
    for spec in &ordered {
        if let Some(missing) = spec.columns.iter().find_map(|c| {
            c.codebook
                .as_ref()
                .filter(|cb| spec.codebook_references && !codebooks.iter().any(|e| &e.name == cb))
        }) {
            return Err(DdlError(format!(
                "{} refers to undeclared codebook {missing}",
                spec.name
            )));
        }
        statements.extend(spec.parent(cfg));
    }
    let mut h = crate::id::IdHasher::v2(crate::id::IdKind::Ddl);
    for s in &statements {
        h.str(s);
    }
    for spec in &ordered {
        h.str(&spec.staging_table());
        for index in spec.staging_indexes() {
            h.str(&index);
        }
        h.str(&spec.attach(cfg));
        h.str(&spec.detach(cfg));
    }
    Ok(Install {
        statements,
        tables: ordered,
        digest: h.finish_digest(),
    })
}

/// Referenced relations before their referrers, otherwise in the given order.
fn dependency_order(tables: Vec<TableSpec>) -> Result<Vec<TableSpec>, DdlError> {
    let mut remaining = tables;
    let mut out: Vec<TableSpec> = Vec::new();
    while !remaining.is_empty() {
        let before = remaining.len();
        let mut next = Vec::new();
        for spec in remaining {
            let ready = spec.foreign_keys.iter().all(|fk| {
                fk.relation == spec.name || out.iter().any(|o| o.name == fk.relation)
            });
            if ready {
                out.push(spec);
            } else {
                next.push(spec);
            }
        }
        if next.len() == before {
            return Err(DdlError(format!(
                "references among {} form a cycle or name an undeclared relation",
                next.iter().map(|s| s.name.as_str()).collect::<Vec<_>>().join(", ")
            )));
        }
        remaining = next;
    }
    Ok(out)
}
