//! Current FCA attribute observations. SQL preserves structural identities and cited source
//! facts; `concept_attributes` owns meaning and rendering. Unknown types remain excluded by
//! structural type traversal, never by a presentation-string test.

use crate::codebook::{
    Codebook, ConceptAttributeKind as Kind, DeclarationKind, DefinitionKind, ParameterKind,
    TypeRole, TypeTermKind,
};
use crate::flows::{codes, receivers_sql};
use crate::id::{Digest, Id, IdHasher};

crate::query_row! {
    pub struct AttributeObservation {
        function_node_id: Id,
        source_fact_id: Id,
        kind: Kind,
        symbol: Option<String>,
        parameter_kind: Option<ParameterKind>,
        type_term_id: Option<Id>,
        class_module: Option<String>,
        class_key: Option<String>,
        display: String,
    }
}

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
           UNION SELECT a.parent_node_id FROM type_term_args a \
           JOIN unknown u ON u.node_id = a.child_node_id), \
         known AS (SELECT t.* FROM type_terms t LEFT ANTI JOIN unknown u ON u.node_id=t.node_id), \
         class_names AS (SELECT module_name, class_key, min(class_name) AS name FROM ( \
           SELECT module_name, class_key, class_name FROM pysa_classes UNION ALL \
           SELECT module_name, key, name FROM context_definitions WHERE kind={class_definition}) \
           GROUP BY module_name, class_key), \
         receivers AS ({receivers}), \
         wanted AS (SELECT node_id FROM declarations WHERE node_id IN ({list}) AND kind IN ({functions})), \
         params AS (SELECT ps.* FROM parameter_syntax ps JOIN wanted w ON w.node_id=ps.function_node_id \
           LEFT ANTI JOIN receivers r ON r.parameter_node_id=ps.node_id), \
         attributes AS ( \
           SELECT function_node_id, fact_id AS source_fact_id, {param_kind} AS kind, name AS symbol, \
                  kind AS parameter_kind, NULL AS type_term_id, NULL AS class_module, NULL AS class_key, name AS display FROM params \
           UNION ALL \
           SELECT p.function_node_id,o.fact_id,{param_type_kind} AS kind,NULL AS symbol,NULL AS parameter_kind,t.node_id AS type_term_id,NULL AS class_module,NULL AS class_key,t.display \
           FROM params p JOIN type_observations o ON o.subject_node_id=p.node_id AND o.role={parameter} AND o.declared \
           JOIN known t ON t.node_id=o.term_node_id \
           UNION ALL \
           SELECT w.node_id,o.fact_id,{returns_kind} AS kind,NULL AS symbol,NULL AS parameter_kind,t.node_id AS type_term_id,NULL AS class_module,NULL AS class_key,t.display \
           FROM wanted w JOIN type_observations o ON o.subject_node_id=w.node_id AND o.role={returns} AND o.declared \
           JOIN known t ON t.node_id=o.term_node_id \
           UNION ALL \
           SELECT w.node_id,o.fact_id,{raises_kind} AS kind,NULL AS symbol,NULL AS parameter_kind,NULL AS type_term_id,t.class_module,t.class_key,COALESCE(c.name,t.class_key) AS display \
           FROM wanted w JOIN syntax_nodes sn ON sn.owner_node_id=w.node_id \
           JOIN type_observations o ON o.subject_node_id=sn.node_id AND o.role={raised} \
           JOIN known t ON t.node_id=o.term_node_id AND t.kind IN ({class_instance},{class_object}) \
           LEFT JOIN class_names c ON c.module_name=t.class_module AND c.class_key=t.class_key \
           WHERE t.class_module IS NOT NULL AND t.class_key IS NOT NULL \
           UNION ALL \
           SELECT d.node_id,d.fact_id,{decorator_kind} AS kind,d.decorator AS symbol,NULL AS parameter_kind,NULL AS type_term_id,NULL AS class_module,NULL AS class_key,d.decorator AS display FROM ( \
             SELECT node_id,fact_id,unnest(decorators) AS decorator FROM declarations \
             WHERE node_id IN (SELECT node_id FROM wanted)) d) \
         SELECT DISTINCT * FROM attributes ORDER BY function_node_id,kind,source_fact_id,symbol,type_term_id",
        receivers = receivers_sql(),
        functions = codes(&[DeclarationKind::Function, DeclarationKind::AsyncFunction]),
        class_definition = DefinitionKind::Class.code(),
        parameter = TypeRole::Parameter.code(),
        returns = TypeRole::Return.code(),
        raised = TypeRole::Raised.code(),
        any = TypeTermKind::Any.code(),
        class_instance = TypeTermKind::ClassInstance.code(),
        class_object = TypeTermKind::ClassObject.code(),
        param_kind = Kind::Parameter.code(),
        param_type_kind = Kind::ParameterType.code(),
        returns_kind = Kind::Returns.code(),
        raises_kind = Kind::Raises.code(),
        decorator_kind = Kind::Decorator.code(),
    )
}

pub fn digest() -> Digest {
    IdHasher::new("concept-attributes")
        .str(&attributes_sql(&[]))
        .finish_digest()
}

pub mod schemas {
    pub fn attributes() -> arrow_schema::SchemaRef {
        <super::AttributeObservation as crate::query::QueryRow>::schema()
    }
}
