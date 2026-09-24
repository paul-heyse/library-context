//! The serving bundle's declared files (DESIGN §6.4; ADR-0019): each file's Arrow schema and sort
//! key, and its **serving schema digest**. The digest is a SHA-256 over a language-neutral
//! canonical form that Python recomputes with its standard library. It is not the store's
//! `canonical_schema` (§6.3), whose Rust type display Python cannot reproduce.
//!
//! Codebook values are served as their text, so the server needs no codebook. Ids are 16-byte
//! fixed-size binaries, and digests 32-byte ones.

use std::sync::Arc;

use arrow_schema::{DataType, Field, Schema, SchemaRef};
use sha2::{Digest as _, Sha256};

/// One served file: its name (`<name>.arrow`), its schema and its total sort key.
#[derive(Debug, Clone)]
pub struct ServingFile {
    pub name: &'static str,
    pub schema: SchemaRef,
    pub key: &'static [&'static str],
}

fn id(name: &str, nullable: bool) -> Field {
    Field::new(name, DataType::FixedSizeBinary(16), nullable)
}

fn digest(name: &str, nullable: bool) -> Field {
    Field::new(name, DataType::FixedSizeBinary(32), nullable)
}

fn utf8(name: &str, nullable: bool) -> Field {
    Field::new(name, DataType::Utf8, nullable)
}

fn int(name: &str, nullable: bool) -> Field {
    Field::new(name, DataType::Int64, nullable)
}

/// A vector column of `dimensions` non-null `float32` items, the child named `item` (pyarrow's
/// name; Delta's list child is `element`, which is why the bundle declares its own).
pub fn vector_type(dimensions: i32) -> DataType {
    DataType::FixedSizeList(
        Arc::new(Field::new("item", DataType::Float32, false)),
        dimensions,
    )
}

/// Every served file, in manifest order; `dimensions` is the snapshot's spec's (0 without one).
pub fn files(dimensions: i32) -> Vec<ServingFile> {
    let file = |name, fields: Vec<Field>, key| ServingFile {
        name,
        schema: Arc::new(Schema::new(fields)),
        key,
    };
    vec![
        file(
            "briefs",
            vec![
                id("brief_id", false),
                id("seed_node_id", false),
                utf8("access_path", false),
                utf8("title", false),
                utf8("applicable_case", true),
                Field::new("documentation_only", DataType::Boolean, false),
                utf8("review_state", false),
                utf8("outcome", true),
                utf8("outcome_status", false),
            ],
            &["brief_id"],
        ),
        file(
            "assertions",
            vec![
                id("brief_id", false),
                int("ordinal", false),
                id("assertion_id", false),
                utf8("kind", false),
                utf8("section", false),
                utf8("status", false),
                utf8("text", true),
                utf8("applicable_case", true),
                utf8("conditions", true),
                utf8("limitations", true),
                int("template_version", false),
            ],
            &["brief_id", "ordinal"],
        ),
        file(
            "supports",
            vec![
                id("assertion_id", false),
                utf8("role", false),
                int("ordinal", false),
                id("finding_id", true),
                utf8("finding_kind", true),
                id("evidence_id", true),
            ],
            &["assertion_id", "role", "ordinal"],
        ),
        file(
            "evidence",
            vec![
                id("evidence_id", false),
                utf8("kind", false),
                id("node_id", true),
                utf8("path", true),
                int("start_byte", true),
                int("end_byte", true),
                utf8("text", true),
            ],
            &["evidence_id"],
        ),
        file(
            "brief_members",
            vec![
                id("brief_id", false),
                utf8("access_path", false),
                id("export_node_id", false),
                id("declaration_node_id", false),
                Field::new("own", DataType::Boolean, false),
            ],
            &["brief_id", "access_path"],
        ),
        file(
            "symbol_map",
            vec![utf8("symbol", false), id("brief_id", false)],
            &["symbol", "brief_id"],
        ),
        // FORMAT 2 (the holistic assessment's A1): every public path of the release, the surface
        // a gold operation or a query spelling resolves against.
        file(
            "public_paths",
            vec![
                id("node_id", false),
                utf8("access_path", false),
                utf8("kind", false),
                Field::new("own", DataType::Boolean, false),
                Field::new("preferred", DataType::Boolean, false),
            ],
            &["node_id", "access_path"],
        ),
        file(
            "lexical_text",
            vec![id("brief_id", false), utf8("text", false)],
            &["brief_id"],
        ),
        file(
            "embedding_spec",
            vec![digest("spec_hash", false), utf8("spec", false)],
            &["spec_hash"],
        ),
        file(
            "vectors",
            vec![
                id("brief_id", false),
                int("chunk", false),
                digest("input_hash", false),
                Field::new("vector", vector_type(dimensions), false),
            ],
            &["brief_id", "chunk"],
        ),
        // FORMAT 3 (ADR-0021; the behavioral-model plan's Stage 1): the whole public surface.
        file(
            "operations",
            vec![
                id("node_id", false),
                utf8("access_path", false),
                utf8("kind", false),
                Field::new("is_method", DataType::Boolean, false),
                utf8("qualified_name", false),
                utf8("module", false),
                utf8("docstring_summary", true),
                utf8("behavior_status", false),
                // FORMAT 4 (increment 3's deep review, F2): the first boundary the scan met.
                utf8("boundary_reason", true),
                utf8("status_reason", true),
                id("brief_id", true),
            ],
            &["node_id"],
        ),
        file(
            "operation_facets",
            vec![
                id("node_id", false),
                utf8("facet", false),
                utf8("value", false),
                // FORMAT 4 (F3): only established and conditional rows match.
                utf8("verdict", false),
            ],
            &["node_id", "facet", "value"],
        ),
        // FORMAT 4 (F3, F4): the served authority for `complete` and the `unknown` list.
        file(
            "operation_facet_status",
            vec![
                id("node_id", false),
                utf8("facet", false),
                utf8("verdict", false),
                utf8("reason", true),
            ],
            &["node_id", "facet"],
        ),
        file(
            "behaviors",
            vec![
                id("behavior_id", false),
                id("operation_node_id", false),
                utf8("kind", false),
                utf8("parameter_name", true),
                id("callee_node_id", true),
                utf8("callee", true),
                utf8("target_name", true),
                utf8("value", true),
                int("depth", false),
                Field::new("conditional", DataType::Boolean, false),
                utf8("verdict", false),
                // FORMAT 4 (F1): why a row is unknown.
                utf8("boundary_reason", true),
                // FORMAT 5 (Stage 2): the condition, a callee outside the release, the read
                // phase, the premise a negative claim rests on.
                utf8("condition", true),
                utf8("callee_text", true),
                utf8("phase", true),
                utf8("premise_key", true),
                int("occurrences", false),
                utf8("path", true),
                int("line", true),
                utf8("site_text", true),
            ],
            &["behavior_id"],
        ),
        // FORMAT 5 (Stage 2; ADR-0022): module-global singletons, their fields' reads at the
        // resolved key, and the field and setting claims with their premises.
        file(
            "singletons",
            vec![utf8("global", false), id("class_node_id", false)],
            &["global"],
        ),
        file(
            "ambient_reads",
            vec![
                utf8("global", false),
                utf8("field", false),
                id("reader_node_id", true),
                utf8("reader", true),
                utf8("phase", false),
                utf8("path", true),
                int("line", false),
                int("start_byte", false),
                utf8("spelled", false),
                utf8("condition", true),
            ],
            &["global", "field", "path", "start_byte"],
        ),
        file(
            "place_claims",
            vec![
                utf8("place_key", false),
                utf8("kind", false),
                id("subject_node_id", true),
                Field::new("holds", DataType::Boolean, false),
                utf8("boundary_reason", true),
                utf8("reason", true),
            ],
            &["place_key"],
        ),
        file(
            "operation_text",
            vec![id("node_id", false), utf8("text", false)],
            &["node_id"],
        ),
        file(
            "operation_vectors",
            vec![
                id("node_id", false),
                utf8("embedding_view", false),
                int("chunk", false),
                digest("input_hash", false),
                Field::new("vector", vector_type(dimensions), false),
            ],
            &["node_id", "embedding_view", "chunk"],
        ),
    ]
}

/// A data type in the declared grammar: `bool`, `int16`, `int32`, `int64`, `float32`, `float64`,
/// `utf8`, `fixed_size_binary(N)`, `list(<child>)` and `fixed_size_list(<child>, N)`, where a
/// child is `<type> null|not null "<name>"`. Any other type is refused: the grammar grows with a
/// served file that needs it.
pub fn type_grammar(t: &DataType) -> Result<String, String> {
    let child = |f: &Field| -> Result<String, String> {
        Ok(format!(
            "{} {} \"{}\"",
            type_grammar(f.data_type())?,
            if f.is_nullable() { "null" } else { "not null" },
            f.name()
        ))
    };
    Ok(match t {
        DataType::Boolean => "bool".to_owned(),
        DataType::Int16 => "int16".to_owned(),
        DataType::Int32 => "int32".to_owned(),
        DataType::Int64 => "int64".to_owned(),
        DataType::Float32 => "float32".to_owned(),
        DataType::Float64 => "float64".to_owned(),
        DataType::Utf8 => "utf8".to_owned(),
        DataType::FixedSizeBinary(n) => format!("fixed_size_binary({n})"),
        DataType::List(f) => format!("list({})", child(f)?),
        DataType::FixedSizeList(f, n) => format!("fixed_size_list({}, {n})", child(f)?),
        other => return Err(format!("{other} is not in the serving type grammar")),
    })
}

/// The canonical form: a JSON array with one object per field in order, each with its keys
/// sorted (`metadata` as sorted `[key, value]` pairs, `name`, `nullable`, `type`), with no
/// whitespace. Python's `json.dumps(fields, sort_keys=True, separators=(",", ":"),
/// ensure_ascii=False)` writes the same bytes.
pub fn canonical_form(schema: &Schema) -> Result<String, String> {
    let fields = schema
        .fields()
        .iter()
        .map(|f| {
            let mut metadata: Vec<(&String, &String)> = f.metadata().iter().collect();
            metadata.sort();
            Ok(format!(
                "{{\"metadata\":{},\"name\":{},\"nullable\":{},\"type\":{}}}",
                serde_json::to_string(&metadata).map_err(|e| e.to_string())?,
                serde_json::to_string(f.name()).map_err(|e| e.to_string())?,
                f.is_nullable(),
                serde_json::to_string(&type_grammar(f.data_type())?).map_err(|e| e.to_string())?,
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(format!("[{}]", fields.join(",")))
}

/// The serving schema digest: SHA-256 of the canonical form, as lowercase hex.
pub fn schema_digest(schema: &Schema) -> Result<String, String> {
    let form = canonical_form(schema)?;
    Ok(Sha256::digest(form.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

/// The serving tokenizer, which Python's `retrieval.tokenize` equals (known answers in
/// `specs/serving/tokens.json`): the text lower-cased (Unicode), then its runs of ASCII letters
/// and digits. `custom_route` is `custom route`; a non-ASCII letter separates tokens.
pub fn tokens(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(str::to_owned)
        .collect()
}

/// A public name's distinct tokens for lexical search, each once in first-seen order (the
/// holistic assessment's A1(c); `FORMAT` 2): the tokens of the path, of each segment, and of each
/// segment's words split at underscores and case changes (`FastMCP` is `fast` and `mcp` too).
pub fn name_tokens(path: &str, out: &mut Vec<String>) {
    let mut push = |w: &str| {
        for t in tokens(w) {
            if !out.contains(&t) {
                out.push(t);
            }
        }
    };
    push(path);
    for segment in path.split('.') {
        push(segment);
        for part in segment.split('_') {
            let chars: Vec<char> = part.chars().collect();
            let mut start = 0;
            for i in 1..chars.len() {
                let (a, b) = (chars[i - 1], chars[i]);
                let next_lower = chars.get(i + 1).is_some_and(|c| c.is_lowercase());
                if (a.is_lowercase() && b.is_uppercase())
                    || (a.is_uppercase() && b.is_uppercase() && next_lower)
                    || (a.is_alphabetic() != b.is_alphabetic())
                {
                    push(&chars[start..i].iter().collect::<String>());
                    start = i;
                }
            }
            if start > 0 {
                push(&chars[start..].iter().collect::<String>());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_grammar_names_each_served_type() {
        assert_eq!(
            type_grammar(&vector_type(4096)).unwrap(),
            "fixed_size_list(float32 not null \"item\", 4096)"
        );
        assert!(type_grammar(&DataType::Utf8View).is_err());
        let form = canonical_form(&Schema::new(vec![
            Field::new("a", DataType::Utf8, true),
            Field::new("b", DataType::FixedSizeBinary(16), false),
        ]))
        .unwrap();
        assert_eq!(
            form,
            "[{\"metadata\":[],\"name\":\"a\",\"nullable\":true,\"type\":\"utf8\"},\
             {\"metadata\":[],\"name\":\"b\",\"nullable\":false,\"type\":\"fixed_size_binary(16)\"}]"
        );
    }
}
