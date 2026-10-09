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
                "(SELECT VALUE out FROM $frontier->{} WHERE out.semantic_type IN $types)",
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
            walks.push(format!("(SELECT VALUE in FROM $frontier<-{} WHERE in.semantic_type IN $incoming_types AND ({}))",
                edge_table(*edge), conditions.join(" OR ")));
        }
        walks.push("(SELECT VALUE id FROM assertion WHERE semantic_type=$distribution_type AND semantic_type IN $types AND scope_keys CONTAINSANY $distribution_keys)".to_owned());
        walks.push("$corpus_ids".into());
        let frontier_sql = format!(
            "RETURN {{\
            LET $capture_inputs = SELECT VALUE body.{} FROM entity WHERE id IN $frontier AND semantic_type IN $capture_input_types;\
            LET $source_inputs = SELECT VALUE body.{} FROM entity WHERE id IN $frontier AND semantic_type IN $source_input_types;\
            LET $corpus_keys = {corpus_keys};\
            LET $capture_corpora = SELECT id, body.{} AS {} FROM assertion WHERE semantic_type=$corpus_type AND scope_keys CONTAINSANY $corpus_keys;\
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
        bindings.insert("frontier", frontier);
        PreparedQuery::new(bindings, vec![], vec![self.frontier_sql.clone()])
    }
    pub async fn hydrate(
        &self,
        reader: &NativeReader,
        nodes: Vec<RecordId>,
        budget: &ResourceBudget,
    ) -> Result<CanonicalBatches, ModelError> {
        let mut bindings = Variables::new();
        bindings.insert("nodes", nodes);
        let types: BTreeSet<_> = self
            .program
            .inputs()
            .iter()
            .map(|input| input.name().to_owned())
            .collect();
        bindings.insert("types", types.into_iter().collect::<Vec<_>>());
        reader.canonical_batches("RETURN array::concat(\
            (SELECT 'entity' AS node_kind, canonical FROM $nodes WHERE record::table(id)='entity' AND semantic_type IN $types),\
            (SELECT 'assertion' AS node_kind, canonical FROM $nodes WHERE record::table(id)='assertion' AND semantic_type IN $types));".into(), bindings, budget).await
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
