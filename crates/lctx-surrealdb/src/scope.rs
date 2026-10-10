//! Mechanical native lowering of model-owned serving scope intent.
use crate::{NativeReader, batches::CanonicalBatches, prepared::PreparedQuery};
use lctx_model::domain::{
    ModelError,
    resources::{Reservation, ResourceBudget},
    serving_scope::{ServingEdge, ServingScopeProgram},
};
use std::{collections::BTreeSet, sync::Arc};
use surrealdb::types::{RecordId, Variables};

pub struct PreparedServingScope {
    program: Arc<ServingScopeProgram>,
    frontier_sql: String,
    bindings: Variables,
    _charge: Box<dyn Reservation>,
}
fn edge_table(edge: ServingEdge) -> &'static str {
    match edge {
        ServingEdge::Reference => "reference",
        ServingEdge::Participant => "participant",
    }
}
fn literal(value: &str) -> String {
    format!("'{}'", value.replace('\\', "\\\\").replace('\'', "\\'"))
}
impl PreparedServingScope {
    /// Prepare once for the selected declaration/ownership shape, independently of root IDs.
    /// The reservation covers retained metadata and its one in-flight request clone.
    pub fn new(
        program: Arc<ServingScopeProgram>,
        budget: &ResourceBudget,
    ) -> Result<Arc<Self>, ModelError> {
        let names = program
            .inputs()
            .iter()
            .chain(program.incoming_inputs())
            .map(|input| input.name().len() + 128)
            .sum::<usize>();
        let fields = program
            .owned_fields()
            .iter()
            .map(|field| field.len() + 128)
            .sum::<usize>();
        let charge = budget.reserve(
            "native-serving-scope-preparation",
            32768usize.saturating_add(names.saturating_add(fields).saturating_mul(8)),
        )?;
        let types: BTreeSet<_> = program
            .inputs()
            .iter()
            .map(|input| input.name().to_owned())
            .collect();
        let mut bindings = Variables::new();
        bindings.insert("types", types.into_iter().collect::<Vec<_>>());
        bindings.insert(
            "incoming_types",
            program
                .incoming_inputs()
                .iter()
                .map(|input| input.name().to_owned())
                .collect::<Vec<_>>(),
        );
        bindings.insert("fields", program.owned_fields().to_vec());
        let capture = program.capture();
        bindings.insert(
            "capture_input_types",
            capture
                .direct_sources
                .iter()
                .map(|source| source.name())
                .collect::<Vec<_>>(),
        );
        bindings.insert(
            "source_input_types",
            capture
                .corpus_sources
                .iter()
                .map(|source| source.name())
                .collect::<Vec<_>>(),
        );
        bindings.insert("corpus_type", capture.corpus_relation);
        bindings.insert("distribution_type", capture.distribution_relation);
        let corpus_keys = crate::prepared::scope_constants(
            "$corpus_type",
            capture.corpus_input,
            "$source_inputs",
        );
        let distribution_keys = crate::prepared::scope_constants(
            "$distribution_type",
            capture.distribution_input,
            &format!(
                "array::concat($capture_inputs,$capture_corpora.map(|$c|$c.{}))",
                capture.corpus_library
            ),
        );
        let mut walks = Vec::new();
        for edge in program.outgoing_edges() {
            walks.push(format!(
                "(SELECT VALUE node FROM compiler_view_member WHERE view IN $lctx_views AND node.semantic_type IN $types AND node.anchor IN (SELECT VALUE out FROM {} WHERE in IN $frontier))",
                edge_table(*edge)
            ));
        }
        for edge in program.outgoing_edges() {
            let mut conditions = vec!["field IN $fields".to_owned()];
            for rule in program
                .incoming_rules()
                .iter()
                .filter(|rule| rule.edge == *edge)
            {
                conditions.push(format!(
                    "(field IN [{}] AND in.semantic_type={})",
                    rule.fields
                        .iter()
                        .map(|field| literal(field))
                        .collect::<Vec<_>>()
                        .join(","),
                    literal(rule.owner.name())
                ));
            }
            walks.push(format!("(SELECT VALUE in FROM {} WHERE out IN $frontier.anchor AND in IN (SELECT VALUE node FROM compiler_view_member WHERE view IN $lctx_views) AND in.semantic_type IN $incoming_types AND ({}))",
                edge_table(*edge), conditions.join(" OR ")));
        }
        walks.push("(SELECT VALUE id FROM assertion WHERE id IN (SELECT VALUE node FROM compiler_view_member WHERE view IN $lctx_views) AND semantic_type=$distribution_type AND semantic_type IN $types AND scope_keys CONTAINSANY $distribution_keys)".to_owned());
        walks.push("$corpus_ids".into());
        let frontier_sql = format!(
            "RETURN {{\
            LET $selected = array::distinct(array::concat((SELECT VALUE node FROM compiler_view_member WHERE view IN $lctx_views),(SELECT VALUE target FROM compiler_alias WHERE source IN (SELECT VALUE node FROM compiler_view_member WHERE view IN $lctx_views))));\
            LET $frontier = SELECT VALUE id FROM $selected WHERE id IN $requested_frontier OR anchor IN $requested_frontier;\
            IF array::len($requested_frontier.filter(|$root| !($root IN $frontier OR $root IN $frontier.anchor)))>0 {{ THROW 'scope root outside exact view'; }};\
            LET $capture_inputs = SELECT VALUE body.{} FROM entity WHERE id IN $frontier AND id IN (SELECT VALUE node FROM compiler_view_member WHERE view IN $lctx_views) AND semantic_type IN $capture_input_types;\
            LET $source_inputs = SELECT VALUE body.{} FROM entity WHERE id IN $frontier AND id IN (SELECT VALUE node FROM compiler_view_member WHERE view IN $lctx_views) AND semantic_type IN $source_input_types;\
            LET $corpus_keys = {corpus_keys};\
            LET $capture_corpora = SELECT id, body.{} AS {} FROM assertion WHERE id IN (SELECT VALUE node FROM compiler_view_member WHERE view IN $lctx_views) AND semantic_type=$corpus_type AND scope_keys CONTAINSANY $corpus_keys;\
            LET $distribution_keys = {distribution_keys};\
            LET $corpus_ids = IF $corpus_type IN $types THEN $capture_corpora.map(|$c|$c.id) ELSE [] END;\
            RETURN array::distinct(array::concat({})); }};",
            capture.source_input,
            capture.source_input,
            capture.corpus_library,
            capture.corpus_library,
            walks.join(",")
        );
        Ok(Arc::new(Self {
            program,
            frontier_sql,
            bindings,
            _charge: charge,
        }))
    }
    pub fn frontier_query(&self, frontier: Vec<RecordId>) -> Result<PreparedQuery, ModelError> {
        let mut bindings = self.bindings.clone();
        bindings.insert("requested_frontier", frontier);
        PreparedQuery::new(bindings, vec![], vec![self.frontier_sql.clone()])
    }
    pub async fn hydrate<Context>(
        &self,
        reader: &NativeReader<Context>,
        nodes: Vec<RecordId>,
        budget: &ResourceBudget,
    ) -> Result<CanonicalBatches, ModelError> {
        let mut bindings = Variables::new();
        bindings.insert("requested_nodes", nodes);
        let types: BTreeSet<_> = self
            .program
            .inputs()
            .iter()
            .map(|input| input.name().to_owned())
            .collect();
        bindings.insert("types", types.into_iter().collect::<Vec<_>>());
        let selected=reader.selected_node_predicate("id");
        reader.canonical_batches(format!("RETURN {{\
            LET $selected=array::distinct(array::concat((SELECT VALUE node FROM compiler_view_member WHERE view IN $lctx_views),(SELECT VALUE target FROM compiler_alias WHERE source IN (SELECT VALUE node FROM compiler_view_member WHERE view IN $lctx_views))));\
            LET $nodes=SELECT VALUE id FROM $selected WHERE id IN $requested_nodes OR anchor IN $requested_nodes;\
            IF array::len($requested_nodes.filter(|$root| !($root IN $nodes OR $root IN $nodes.anchor)))>0 {{ THROW 'scope hydration outside exact view'; }};\
            RETURN array::concat(\
            (SELECT 'entity' AS node_kind, canonical FROM $nodes WHERE record::table(id)='entity' AND semantic_type IN $types AND ({selected})),\
            (SELECT 'assertion' AS node_kind, canonical FROM $nodes WHERE record::table(id)='assertion' AND semantic_type IN $types AND ({selected}))); }};"), bindings, budget).await
    }
}

#[cfg(test)]
mod controls {
    use super::*;
    use lctx_model::domain::{
        ValidationInput, attribution, input, serving_scope::OWNED_FIELDS, source,
    };
    #[test]
    fn serving_template_is_root_independent_and_retains_complete_statement_inventory() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let inputs = [
            ValidationInput::of::<source::SourceArtifact>(&["id"]),
            ValidationInput::of::<input::InputDistribution>(&["id"]),
        ];
        let incoming = [ValidationInput::of::<attribution::ProviderRun>(&["id"])];
        let program = ServingScopeProgram::new(&inputs, &incoming, OWNED_FIELDS, &budget).unwrap();
        let prepared = PreparedServingScope::new(program, &budget).unwrap();
        let a = prepared
            .frontier_query(vec![RecordId::new("entity", "a")])
            .unwrap();
        let b = prepared
            .frontier_query(vec![RecordId::new("entity", "b")])
            .unwrap();
        assert_eq!(a.result_positions(), &[0]);
        assert_eq!(a.expected_terminals(), 1);
        assert_eq!(b.expected_terminals(), 1);
        assert_eq!(a.into_request().0, b.into_request().0);
        assert!(
            prepared.frontier_sql.contains("$corpus_type IN $types"),
            "corpus output inventory cannot gate companion discovery"
        );
        assert!(
            prepared
                .frontier_sql
                .contains("array::concat($capture_inputs,$capture_corpora.map(|$c|$c.library))")
        );
        assert_eq!(
            prepared
                .frontier_sql
                .matches("in.semantic_type IN $incoming_types")
                .count(),
            2
        );
        assert_eq!(
            prepared
                .frontier_sql
                .matches("out.semantic_type IN $types")
                .count(),
            2
        );
        assert!(budget.reserved() > 0);
        drop(prepared);
        assert_eq!(budget.reserved(), 0);
    }
}
