//! Embeddings in analytics (DESIGN §9.7; slice 3.1): the declared relations whose texts are
//! embedded (E0) for exact nearest-neighbour search. Their digest joins the compiler digest and the
//! kNN invocation's record.
//!
//! - **Passages:** every corpus passage with its heading and its document's path.
//! - **API texts:** each given function's docstring and its parameters in order, the receiver aside
//!   (`flows::receivers_sql`); the text itself is assembled in `lctx_analytics::neighbours`.

use crate::flows::receivers_sql;
use crate::id::{Digest, Id, IdHasher};

/// Every corpus passage, ordered by node.
pub fn passages_sql() -> String {
    "SELECT p.node_id, p.heading, d.path, p.text FROM passages p \
     JOIN documents d ON d.node_id = p.document_node_id ORDER BY p.node_id"
        .to_owned()
}

/// The given functions' docstrings and parameter names (in order, the receiver aside), ordered by
/// node.
pub fn api_texts_sql(functions: &[Id]) -> String {
    let list = if functions.is_empty() {
        "NULL".to_owned()
    } else {
        functions
            .iter()
            .map(|f| format!("X'{}'", f.hex()))
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!(
        "WITH receivers AS ({receivers}) \
         SELECT d.node_id, d.docstring, \
                string_agg(ps.name, ', ' ORDER BY ps.ordinal) AS parameters \
         FROM declarations d \
         LEFT JOIN (SELECT ps.* FROM parameter_syntax ps \
                    LEFT ANTI JOIN receivers r ON r.parameter_node_id = ps.node_id) ps \
           ON ps.function_node_id = d.node_id \
         WHERE d.node_id IN ({list}) \
         GROUP BY d.node_id, d.docstring ORDER BY d.node_id",
        receivers = receivers_sql(),
    )
}

/// The relations' identity (the API-text query over no functions stands for its form).
pub fn digest() -> Digest {
    IdHasher::new("neighbour-texts")
        .str(&passages_sql())
        .str(&api_texts_sql(&[]))
        .finish_digest()
}

/// The declared output schemas.
pub mod schemas {
    use std::sync::Arc;

    use arrow_schema::{DataType, Field, Schema, SchemaRef};

    fn id(name: &str) -> Field {
        Field::new(name, DataType::FixedSizeBinary(16), false)
    }

    pub fn passages() -> SchemaRef {
        Arc::new(Schema::new(vec![
            id("node_id"),
            Field::new("heading", DataType::Utf8, true),
            Field::new("path", DataType::Utf8, false),
            Field::new("text", DataType::Utf8, false),
        ]))
    }

    pub fn api_texts() -> SchemaRef {
        Arc::new(Schema::new(vec![
            id("node_id"),
            Field::new("docstring", DataType::Utf8, true),
            Field::new("parameters", DataType::Utf8, true),
        ]))
    }
}
