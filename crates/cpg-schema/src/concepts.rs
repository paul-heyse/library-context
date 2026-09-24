//! Formal concept analysis's declared relation (DESIGN §9.6; ADR-0011): the attributes of each
//! callable, stated before any kernel runs; its digest joins the compiler digest and each FCA
//! invocation's record.
//!
//! One row per (function, attribute), each attribute a string of one of five forms:
//! - `parameter NAME` (`*NAME`, `**NAME` for the catch-alls), the receiver aside (by the
//!   method's kind, `flows::receivers_sql`);
//! - `parameter type T`: a parameter's declared type, as Pyrefly displays it (C4);
//! - `returns T`: the declared return type;
//! - `raises E`: the class of an exception a `raise` directly in the body raises, whether it
//!   raises the class (`raise E`) or an instance (`raise E()`), one attribute either way (C4; a
//!   bare re-raise has no type, so it is no attribute: C4 review O4; C2 O4);
//! - `decorator D`: a decorator as written (its trailing name).
//!
//! A type Pyrefly could not determine is no attribute (the increment-2 review's F2): a term that
//! is, or holds anywhere in its structure, an `Any` of style `error` or `implicit` (displayed
//! `Unknown`), found by walking `type_term_args` up from those terms. Two APIs are never grouped
//! for sharing what the analysis does not know.

use crate::codebook::{Codebook, DeclarationKind, ParameterKind, TypeRole, TypeTermKind};
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
        "WITH RECURSIVE unknown(node_id) AS ( \
           SELECT node_id FROM type_terms WHERE kind = {any} AND detail IN ('error', 'implicit') \
           UNION ALL \
           SELECT a.parent_node_id FROM type_term_args a \
           JOIN unknown u ON u.node_id = a.child_node_id), \
         known AS (SELECT t.node_id, t.kind, t.display FROM type_terms t \
                   LEFT ANTI JOIN unknown u ON u.node_id = t.node_id), \
         receivers AS ({receivers}), \
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
           JOIN known t ON t.node_id = o.term_node_id \
           UNION ALL \
           SELECT w.node_id, 'returns ' || t.display FROM wanted w \
           JOIN type_observations o ON o.subject_node_id = w.node_id \
             AND o.role = {returns} AND o.declared \
           JOIN known t ON t.node_id = o.term_node_id \
           UNION ALL \
           SELECT w.node_id, 'raises ' || CASE \
               WHEN t.kind = {class_object} AND starts_with(t.display, 'type[') \
                 AND ends_with(t.display, ']') \
               THEN substr(t.display, 6, character_length(t.display) - 6) \
               ELSE t.display END FROM wanted w \
           JOIN syntax_nodes sn ON sn.owner_node_id = w.node_id \
           JOIN type_observations o ON o.subject_node_id = sn.node_id AND o.role = {raised} \
           JOIN known t ON t.node_id = o.term_node_id \
             AND t.kind IN ({class_instance}, {class_object}) \
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
        any = TypeTermKind::Any.code(),
        class_instance = TypeTermKind::ClassInstance.code(),
        class_object = TypeTermKind::ClassObject.code(),
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
