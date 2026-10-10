//! Bounded grammar-owned decoding. Imported SQL is data and is never submitted to a server.
use lctx_model::domain::{
    ModelError,
    resources::{MAX_ROW_BYTES, TRANSFER_BYTES},
};
use lctx_surrealdb::surrealdb::types::{
    Bytes, Number, Object, RecordId, RecordIdKey, ToSql, Value,
};
use std::io::Read;
use surrealdb_sql::{Data, Expr, Literal, TopLevelExpr};

/// Canonical row limits count decoded bytes. Native SQL also carries the canonical byte
/// envelope and escaped typed fields. The installed exporter emits one record per statement;
/// this allowance covers those encodings rather than multiplying by a whole export batch.
pub(crate) const MAX_DUMP_RECORD_BYTES: usize = MAX_ROW_BYTES * 16;

/// The native adapter owns the entire immutable dump inventory, including inert actual
/// integrity claims. Runtime controls and regenerated secondary indexes have no dump codec.
pub(crate) fn dump_tables() -> impl Iterator<Item = lctx_surrealdb::compiler::RecoveryTable> {
    lctx_surrealdb::compiler::recovery_table_inventory()
}
pub(crate) fn table_codec(name: &str) -> Result<lctx_surrealdb::compiler::RecoveryTable, ModelError> {
    lctx_surrealdb::compiler::classify_recovery_table(name)
        .ok_or(ModelError::Schema("dump table outside immutable content inventory"))
}

#[derive(Debug)]
pub(crate) enum Item {
    /// Definition text is retained only for comparison with installed definitions.
    Definition(String),
    Rows(Vec<Value>),
}

/// Compare declarations as immutable metadata, never as executable import instructions.
/// SurrealDB 3.3 exports fields with OVERWRITE for idempotent SQL re-import, while
/// INFO renders their installed definitions without that application modifier.
/// Only the modifier is discarded; every schema/body/permission attribute remains.
pub(crate) fn definition_metadata(
    definition: &surrealdb_sql::statements::DefineStatement,
) -> Result<String, ModelError> {
    use surrealdb_sql::statements::{DefineStatement, define::DefineKind};
    let mut definition = definition.clone();
    let kind = match &mut definition {
        DefineStatement::Table(value) => &mut value.kind,
        DefineStatement::Field(value) => &mut value.kind,
        DefineStatement::Index(value) => &mut value.kind,
        DefineStatement::Analyzer(value) => &mut value.kind,
        DefineStatement::Function(value) => &mut value.kind,
        _ => {
            return Err(ModelError::Schema(
                "dump executable administrative definition",
            ));
        }
    };
    *kind = DefineKind::Default;
    Ok(definition.to_sql())
}

fn require_literal_value(value: &Value) -> Result<(), ModelError> {
    match value {
        Value::None | Value::Null | Value::Bool(_) | Value::String(_) | Value::Bytes(_) => Ok(()),
        Value::Number(Number::Float(value)) if !value.is_finite() => Err(ModelError::Schema("dump unsupported literal family")),
        Value::Number(_) => Ok(()),
        Value::RecordId(id) if matches!(id.key, RecordIdKey::String(_) | RecordIdKey::Number(_)) => Ok(()),
        Value::Array(values) => values.iter().try_for_each(require_literal_value),
        Value::Object(values) => values.values().try_for_each(require_literal_value),
        _ => Err(ModelError::Schema("dump unsupported literal family")),
    }
}
/// The single typed data-only SQL lowering used by selected export and local compaction.
pub(crate) fn write_row(
    writer: &mut impl std::io::Write, row: Value,
    charge: &mut dyn lctx_model::domain::resources::Reservation,
) -> Result<(), ModelError> {
    let Value::Object(object) = &row else { return Err(ModelError::Schema("dump record object")); };
    let Some(Value::RecordId(id)) = object.get("id") else { return Err(ModelError::Schema("dump literal record identity")); };
    let relation = table_codec(id.table.as_str())?.relation;
    require_literal_value(&row)?;
    // Admit the actual value before SQL escaping and geometric String growth. A
    // tiny record must not reserve the maximum legal dump statement allowance.
    let bound = lctx_surrealdb::loader::native_bytes(&row).saturating_mul(32).saturating_add(512);
    charge.try_resize(bound)?;
    let encoded = Value::Array(vec![row].into()).to_sql(); charge.try_resize(encoded.capacity())?;
    if encoded.len().saturating_add(8) > MAX_DUMP_RECORD_BYTES { return Err(ModelError::Schema("data dump statement bound")); }
    writeln!(writer, "INSERT {}{encoded};", if relation { "RELATION " } else { "" }).map_err(ModelError::codec)?; charge.try_resize(0)?; Ok(())
}
struct Units<R> {
    input: R,
    parser: surrealdb_syn::parser::StatementStream,
    buffer: bytes::BytesMut,
    eof: bool,
    initial_import: bool,
    max_bytes: usize,
}

impl<R: Read> Units<R> {
    fn new(input: R, max_bytes: usize) -> Self {
        Self {
            input,
            parser: surrealdb_syn::parser::StatementStream::new(),
            buffer: bytes::BytesMut::new(),
            eof: false,
            initial_import: false,
            max_bytes,
        }
    }
    fn limit(&self, limit: &'static str, observed: usize) -> ModelError {
        ModelError::Limit {
            owner: "restore-import",
            limit,
            observed,
            bound: self.max_bytes,
        }
    }
    fn statement(&mut self) -> Result<Option<(surrealdb_sql::TopLevelExpr, usize)>, ModelError> {
        loop {
            let before = self.buffer.len();
            let statement = if self.eof {
                self.parser.parse_complete(&mut self.buffer)
            } else {
                self.parser.parse_partial(&mut self.buffer)
            }
            .map_err(|error| ModelError::codec(format!("restore dump parse: {error}")))?;
            if let Some(statement) = statement {
                return Ok(Some((statement, before - self.buffer.len())));
            }
            if self.eof {
                return Ok(None);
            }
            let remaining = self.max_bytes.saturating_sub(self.buffer.len());
            if remaining == 0 {
                return Err(self.limit("statement bytes", self.buffer.len() + 1));
            }
            // The pinned parser reparses unfinished input: retain geometric read growth.
            let amount = self
                .buffer
                .len()
                .clamp(65536, TRANSFER_BYTES)
                .min(remaining);
            let mut window = [0u8; 65536];
            let mut added = 0;
            while added < amount {
                let count = self
                    .input
                    .read(&mut window[..(amount - added).min(65536)])
                    .map_err(ModelError::codec)?;
                if count == 0 {
                    self.eof = true;
                    break;
                }
                self.buffer.extend_from_slice(&window[..count]);
                added += count;
            }
        }
    }
}

pub(crate) struct DataDump<R> {
    units: Units<R>,
    transaction: bool,
    finished: bool,
}
impl<R: Read> DataDump<R> {
    pub(crate) fn new(input: R) -> Self {
        Self {
            units: Units::new(input, MAX_DUMP_RECORD_BYTES),
            transaction: false,
            finished: false,
        }
    }
    pub(crate) fn next(&mut self) -> Result<Option<Item>, ModelError> {
        if self.finished {
            return Ok(None);
        }
        loop {
            let Some((statement, _)) = self.units.statement()? else {
                self.finished = true;
                if !self.units.initial_import {
                    return Err(ModelError::Schema("dump initial OPTION IMPORT"));
                }
                if self.transaction {
                    return Err(ModelError::Schema("dump incomplete transaction"));
                }
                return Ok(None);
            };
            if !self.units.initial_import {
                if !matches!(&statement, TopLevelExpr::Option(option) if option.name.as_str()=="IMPORT" && option.what)
                {
                    return Err(ModelError::Schema("dump initial OPTION IMPORT"));
                }
                self.units.initial_import = true;
                continue;
            }
            return match statement {
                // Export transactions confer no transaction or runtime authority. Their grammar is
                // checked locally; every imported value is still admitted independently.
                TopLevelExpr::Begin if !self.transaction => {
                    self.transaction = true;
                    continue;
                }
                TopLevelExpr::Commit if self.transaction => {
                    self.transaction = false;
                    continue;
                }
                TopLevelExpr::Expr(Expr::Define(definition)) => {
                    definition_metadata(definition.as_ref())
                        .map(|metadata| Some(Item::Definition(metadata)))
                }
                TopLevelExpr::Expr(Expr::Insert(insert)) => {
                    if insert.into.is_some()
                        || insert.ignore
                        || insert.update.is_some()
                        || insert.output.is_some()
                        || !matches!(insert.timeout, Expr::Literal(Literal::None))
                    {
                        return Err(ModelError::Schema("dump unsupported INSERT form"));
                    }
                    let Data::SingleExpression(Expr::Literal(Literal::Array(rows))) = insert.data
                    else {
                        return Err(ModelError::Schema("dump literal INSERT array"));
                    };
                    let mut result = Vec::with_capacity(rows.len());
                    for row in rows {
                        let value = literal(row)?;
                        let Value::Object(object) = &value else {
                            return Err(ModelError::Schema("dump record object"));
                        };
                        let Some(Value::RecordId(id)) = object.get("id") else {
                            return Err(ModelError::Schema("dump literal record identity"));
                        };
                        let codec = table_codec(id.table.as_str())?;
                        if insert.relation != codec.relation {
                            return Err(ModelError::Schema("dump record relation kind"));
                        }
                        // Derived rows remain inert claims for independent cold comparison.
                        // Ordinary restore never executes or imports these physical records.
                        result.push(value);
                    }
                    Ok(Some(Item::Rows(result)))
                }
                _ => Err(ModelError::Schema("dump unsupported executable statement")),
            };
        }
    }
}

fn literal(expr: Expr) -> Result<Value, ModelError> {
    let value = match expr {
        Expr::Literal(value) => value,
        Expr::Prefix { op, expr } => {
            let Expr::Literal(value) = *expr else {
                return Err(ModelError::Schema("dump nonliteral numeric prefix"));
            };
            match (op, value) {
                (
                    surrealdb_sql::PrefixOperator::Positive,
                    value @ (Literal::Integer(_) | Literal::Float(_) | Literal::Decimal(_)),
                ) => value,
                (surrealdb_sql::PrefixOperator::Negate, Literal::Integer(value)) => {
                    Literal::Integer(
                        value
                            .checked_neg()
                            .ok_or(ModelError::Schema("dump integer overflow"))?,
                    )
                }
                (surrealdb_sql::PrefixOperator::Negate, Literal::Float(value)) => {
                    Literal::Float(-value)
                }
                (surrealdb_sql::PrefixOperator::Negate, Literal::Decimal(value)) => {
                    Literal::Decimal(-value)
                }
                _ => return Err(ModelError::Schema("dump unsupported prefix")),
            }
        }
        _ => return Err(ModelError::Schema("dump nonliteral expression")),
    };
    Ok(match value {
        Literal::None => Value::None,
        Literal::Null => Value::Null,
        Literal::Bool(v) => Value::Bool(v),
        Literal::Integer(v) => Value::Number(Number::Int(v)),
        Literal::Float(v) if v.is_finite() => Value::Number(Number::Float(v)),
        Literal::Decimal(v) => Value::Number(Number::Decimal(v)),
        Literal::String(v) => Value::String(v.to_string()),
        Literal::Bytes(v) => Value::Bytes(Bytes::from(v.to_vec())),
        Literal::Array(v) => Value::Array(
            v.into_iter()
                .map(literal)
                .collect::<Result<Vec<_>, _>>()?
                .into(),
        ),
        Literal::Object(entries) => {
            let mut object = Object::new();
            for entry in entries {
                let key = entry.key.to_string();
                if object.insert(key, literal(entry.value)?).is_some() {
                    return Err(ModelError::Schema("dump duplicate object field"));
                }
            }
            Value::Object(object)
        }
        Literal::RecordId(id) => {
            let key = match id.key {
                surrealdb_sql::RecordIdKeyLit::String(v) => RecordIdKey::String(v.to_string()),
                surrealdb_sql::RecordIdKeyLit::Number(v) => RecordIdKey::Number(v),
                _ => return Err(ModelError::Schema("dump noncanonical record key")),
            };
            Value::RecordId(RecordId::new(id.table.to_string(), key))
        }
        // The current native export contract needs no executable casts, dynamic keys,
        // closures or nonfinite numeric values. Unsupported literal families are refused.
        _ => return Err(ModelError::Schema("dump unsupported literal family")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn decode(sql: &str) -> Result<Vec<Item>, ModelError> {
        let mut dump = DataDump::new(sql.as_bytes());
        let mut result = Vec::new();
        while let Some(item) = dump.next()? {
            result.push(item);
        }
        Ok(result)
    }
    fn metadata(sql: &str) -> String {
        let items = decode(&format!("OPTION IMPORT; {sql};")).unwrap();
        let [Item::Definition(metadata)] = items.as_slice() else {
            panic!("one declaration metadata item");
        };
        metadata.clone()
    }
    #[test]
    fn literal_export_admits_actual_record_size_and_refuses_before_writing() {
        let budget = lctx_model::domain::resources::ResourceBudget::fixed(128 * 1024).unwrap();
        let mut charge = budget.reserve("literal-export-control", 0).unwrap();
        let mut object = Object::new();
        object.insert("id", RecordId::new("entity", "quoted-key"));
        object.insert("escaped", "line\n'\\\"\0".repeat(16));
        object.insert("binary", Bytes::from(vec![0, 255, 42]));
        let row = Value::Object(object.clone());
        let mut output = b"OPTION IMPORT;\n".to_vec();
        write_row(&mut output, row.clone(), charge.as_mut()).unwrap();
        let mut decoded = DataDump::new(output.as_slice());
        let Some(Item::Rows(rows)) = decoded.next().unwrap() else { panic!("literal rows"); };
        assert_eq!(rows, [row]);
        assert_eq!(charge.size(), 0);
        object.insert("escaped", "large".repeat(4096));
        let mut refused_output = Vec::new();
        assert!(matches!(write_row(&mut refused_output, Value::Object(object), charge.as_mut()), Err(ModelError::Resource { .. })));
        assert!(refused_output.is_empty(), "resource refusal precedes any dump effect");
    }
    #[test]
    fn every_recovery_family_uses_the_actual_data_only_codec_and_relation_kind() {
        let budget = lctx_model::domain::resources::ResourceBudget::fixed(MAX_DUMP_RECORD_BYTES * 2).unwrap();
        let mut charge = budget.reserve("family-codec-control", 0).unwrap();
        for table in dump_tables() {
            let mut object = Object::new(); object.insert("id", RecordId::new(table.name, "claim"));
            object.insert("unexpected_actual_field", "preserve; quoted ' source anomaly");
            object.insert("bytes", Bytes::from(vec![0, 255, 59])); object.insert("explicit_null", Value::Null);
            let expected = Value::Object(object);
            let mut output = b"OPTION IMPORT;\n".to_vec(); write_row(&mut output, expected.clone(), charge.as_mut()).unwrap();
            let text = String::from_utf8(output).unwrap();
            let decoded = decode(&text).unwrap(); let [Item::Rows(rows)] = decoded.as_slice() else { panic!("one family row"); };
            assert_eq!(rows, &vec![expected], "typed source preservation: {:?}", table.kind);
            let wrong = if table.relation { text.replace("INSERT RELATION ", "INSERT ") } else { text.replace("INSERT ", "INSERT RELATION ") };
            assert!(decode(&wrong).is_err(), "wrong native codec kind: {}", table.name);
        }
        for excluded in ["native_pin", "compiler_attempt", "entity_anchor", "lexical_document"] {
            assert!(table_codec(excluded).is_err(), "no unowned cache/control codec: {excluded}");
        }
    }
    #[test]
    fn export_definition_metadata_normalizes_only_application_kind() {
        for (kind, body) in [
            ("TABLE", "entity SCHEMAFULL PERMISSIONS FULL"),
            (
                "FIELD",
                "payload ON entity TYPE string DEFAULT 'OVERWRITE; IF NOT EXISTS' READONLY ASSERT $value != '' PERMISSIONS FULL COMMENT 'OVERWRITE'",
            ),
            ("INDEX", "payload ON entity FIELDS payload UNIQUE"),
            ("ANALYZER", "search TOKENIZERS class FILTERS lowercase"),
            (
                "FUNCTION",
                "fn::never_execute($input: string) { RETURN 'OVERWRITE; IF NOT EXISTS'; } PERMISSIONS FULL",
            ),
        ] {
            let ordinary = metadata(&format!("DEFINE {kind} {body}"));
            for modifier in ["OVERWRITE", "IF NOT EXISTS"] {
                let exported = metadata(&format!("DEFINE {kind} {modifier} {body}"));
                assert_eq!(exported, ordinary, "application modifier on {kind}");
                assert_eq!(
                    metadata(&exported),
                    ordinary,
                    "metadata roundtrip on {kind}"
                );
            }
        }
    }
    #[test]
    fn export_definition_metadata_preserves_semantic_drift() {
        for (installed, changed) in [
            (
                "DEFINE FIELD value ON entity TYPE string",
                "DEFINE FIELD OVERWRITE value ON entity TYPE int",
            ),
            (
                "DEFINE FIELD value ON entity TYPE string PERMISSIONS FULL",
                "DEFINE FIELD OVERWRITE value ON entity TYPE string PERMISSIONS NONE",
            ),
            (
                "DEFINE FIELD value ON entity TYPE string DEFAULT 'original'",
                "DEFINE FIELD OVERWRITE value ON entity TYPE string DEFAULT 'changed'",
            ),
            (
                "DEFINE FIELD value ON entity TYPE string ASSERT $value != ''",
                "DEFINE FIELD OVERWRITE value ON entity TYPE string ASSERT $value = ''",
            ),
            (
                "DEFINE FIELD value ON entity TYPE string READONLY",
                "DEFINE FIELD OVERWRITE value ON entity TYPE string",
            ),
            (
                "DEFINE TABLE entity SCHEMAFULL",
                "DEFINE TABLE OVERWRITE entity SCHEMALESS",
            ),
            (
                "DEFINE INDEX value ON entity FIELDS value UNIQUE",
                "DEFINE INDEX OVERWRITE value ON entity FIELDS value",
            ),
            (
                "DEFINE INDEX value ON entity FIELDS value",
                "DEFINE INDEX OVERWRITE value ON entity FIELDS other",
            ),
            (
                "DEFINE ANALYZER search TOKENIZERS class FILTERS lowercase",
                "DEFINE ANALYZER OVERWRITE search TOKENIZERS class FILTERS uppercase",
            ),
            (
                "DEFINE FUNCTION fn::answer() { RETURN 'original'; }",
                "DEFINE FUNCTION OVERWRITE fn::answer() { RETURN 'changed'; }",
            ),
        ] {
            assert_ne!(
                metadata(installed),
                metadata(changed),
                "changed declaration: {changed}"
            );
        }
    }
    #[test]
    fn export_definition_metadata_refuses_administrative_variants() {
        for sql in [
            "DEFINE DATABASE other",
            "DEFINE NAMESPACE other",
            "DEFINE USER root ON ROOT PASSWORD 'x' ROLES OWNER",
            "DEFINE PARAM $secret VALUE 'x'",
            "DEFINE EVENT mutation ON entity WHEN true THEN (DELETE entity)",
        ] {
            surrealdb_syn::parse(&format!("{sql};"))
                .expect("valid administrative declaration grammar");
            assert!(
                decode(&format!("OPTION IMPORT; {sql};")).is_err(),
                "accepted {sql}"
            );
        }
    }
    #[test]
    fn restore_decodes_native_literals_without_evaluating_definitions() {
        let items=decode(r#"OPTION IMPORT; DEFINE FUNCTION fn::never_execute() { THROW 'sentinel'; }; INSERT [{id:entity:abc,canonical:b"61623B63",body:{text:'é;literal',values:[1,2.5f,-1,-0.5f,+2,NULL,NONE]}}];"#).unwrap();
        assert!(matches!(&items[0],Item::Definition(v) if v.contains("sentinel")));
        let Item::Rows(rows) = &items[1] else {
            panic!("rows")
        };
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].to_sql().contains("é;literal"), true);
    }
    #[test]
    fn restore_rejects_arbitrary_effects_and_control_identity_injection() {
        for sql in [
            "USE NS other DB other;",
            "RETURN 1;",
            "THROW 'x';",
            "DEFINE USER root ON ROOT PASSWORD 'x' ROLES OWNER;",
            "INSERT [{id:native_pin:borrowed}];",
            "INSERT [{id:entity:x,body:rand::uuid()}];",
            "INSERT IGNORE [{id:entity:x}];",
            "INSERT INTO entity [{id:entity:x}];",
            "INSERT [{id:entity:rand()}];",
            "INSERT [{id:entity:x,body:{x:1,x:2}}];",
            "BEGIN;",
            "COMMIT;",
            "CANCEL;",
        ] {
            assert!(
                decode(&format!("OPTION IMPORT; {sql}")).is_err(),
                "accepted {sql}"
            );
        }
    }
    #[test]
    fn malformed_tail_is_refused_after_provisional_data_without_remote_effects() {
        let mut dump =
            DataDump::new(b"OPTION IMPORT; INSERT [{id:entity:x}]; INSERT [{bad: }];".as_slice());
        assert!(matches!(dump.next().unwrap(), Some(Item::Rows(_))));
        assert!(dump.next().is_err());
    }
    #[test]
    fn chunked_parser_preserves_quoted_semicolons_and_utf8() {
        struct Chunks<'a>(&'a [u8]);
        impl Read for Chunks<'_> {
            fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
                let n = out.len().min(3).min(self.0.len());
                out[..n].copy_from_slice(&self.0[..n]);
                self.0 = &self.0[n..];
                Ok(n)
            }
        }
        let mut dump = DataDump::new(Chunks(
            r#"OPTION IMPORT; INSERT [{id:entity:x,canonical:b"00",body:{text:'é;é'}}];"#
                .as_bytes(),
        ));
        assert!(matches!(dump.next().unwrap(), Some(Item::Rows(_))));
        assert!(dump.next().unwrap().is_none());
    }
}
