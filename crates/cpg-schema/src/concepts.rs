//! Formal concept analysis's declared relation (DESIGN §9.6; ADR-0011): the attributes of each
//! callable, stated before any kernel runs; its digest joins the compiler digest and each FCA
//! invocation's record.
//!
//! One row per (function, attribute), each attribute a string of one of five forms:
//! - `parameter NAME` (`*NAME`, `**NAME` for the catch-alls), the receiver aside (by the
//!   method's kind, `flows::receivers_sql`);
//! - `parameter type T`: a parameter's declared type, as Pyrefly displays it (C4);
//! - `returns T`: the declared return type;
//! - `raises E`: the type of an exception a `raise` directly in the body raises (C4; a bare
//!   re-raise has no type, so it is no attribute: C4 review O4);
//! - `decorator D`: a decorator as written (its trailing name).

use crate::codebook::{Codebook, DeclarationKind, ParameterKind, TypeRole};
use crate::flows::{codes, receivers_sql};
use crate::id::{Digest, Id, IdHasher};

/// The attributes of the given functions, ordered by `(function, attribute)`.
pub fn attributes_sql(functions: &[Id]) -> String {
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
        "WITH receivers AS ({receivers}), \
         wanted AS (SELECT node_id FROM declarations \
                    WHERE node_id IN ({list}) AND kind IN ({functions})), \
         params AS ( \
           SELECT ps.function_node_id, ps.node_id, ps.name, ps.kind FROM parameter_syntax ps \
           JOIN wanted w ON w.node_id = ps.function_node_id \
           LEFT ANTI JOIN receivers r ON r.parameter_node_id = ps.node_id), \
         attributes AS ( \
           SELECT function_node_id, 'parameter ' || CASE kind WHEN {var_pos} THEN '*' \
                  WHEN {var_kw} THEN '**' ELSE '' END || name AS attribute FROM params \
           UNION ALL \
           SELECT p.function_node_id, 'parameter type ' || t.display FROM params p \
           JOIN type_observations o ON o.subject_node_id = p.node_id \
             AND o.role = {parameter} AND o.declared \
           JOIN type_terms t ON t.node_id = o.term_node_id \
           UNION ALL \
           SELECT w.node_id, 'returns ' || t.display FROM wanted w \
           JOIN type_observations o ON o.subject_node_id = w.node_id \
             AND o.role = {returns} AND o.declared \
           JOIN type_terms t ON t.node_id = o.term_node_id \
           UNION ALL \
           SELECT w.node_id, 'raises ' || t.display FROM wanted w \
           JOIN syntax_nodes sn ON sn.owner_node_id = w.node_id \
           JOIN type_observations o ON o.subject_node_id = sn.node_id AND o.role = {raised} \
           JOIN type_terms t ON t.node_id = o.term_node_id \
           UNION ALL \
           SELECT d.node_id, 'decorator ' || d.decorator FROM ( \
             SELECT node_id, unnest(decorators) AS decorator FROM declarations \
             WHERE node_id IN (SELECT node_id FROM wanted)) d) \
         SELECT DISTINCT function_node_id, attribute FROM attributes \
         ORDER BY function_node_id, attribute",
        receivers = receivers_sql(),
        functions = codes(&[DeclarationKind::Function, DeclarationKind::AsyncFunction]),
        var_pos = ParameterKind::VarPositional.code(),
        var_kw = ParameterKind::VarKeyword.code(),
        parameter = TypeRole::Parameter.code(),
        returns = TypeRole::Return.code(),
        raised = TypeRole::Raised.code(),
    )
}

/// The relation's identity (the query over no functions stands for its form).
pub fn digest() -> Digest {
    IdHasher::new("concept-attributes")
        .str(&attributes_sql(&[]))
        .finish_digest()
}

/// The declared output schema.
pub mod schemas {
    use std::sync::Arc;

    use arrow_schema::{DataType, Field, Schema, SchemaRef};

    pub fn attributes() -> SchemaRef {
        Arc::new(Schema::new(vec![
            Field::new("function_node_id", DataType::FixedSizeBinary(16), false),
            Field::new("attribute", DataType::Utf8, false),
        ]))
    }
}
