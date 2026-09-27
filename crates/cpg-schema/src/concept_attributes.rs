//! Current FCA/RCA attribute meaning. Identity excludes object, evidence and display.
//! Incidence records preserve every source occurrence independently of concept membership.

use crate::codebook::{
    Codebook, ConceptAttributeKind as Kind, InvocationPhase, Modality, OperationFacet,
    ParameterKind,
};
use crate::id::{Id, IdHasher};
use crate::table::table;

table!(
    ConceptAttributes, ConceptAttributesRow = "concept_attributes",
    family = Findings,
    key = [snapshot_id, attribute_id],
    checks = [],
    {
        snapshot_id: Id,
        attribute_id: Id,
        kind: Kind,
        symbol: Option<String>,
        parameter_kind: Option<ParameterKind>,
        type_term_id: Option<Id>,
        class_module: Option<String>,
        class_key: Option<String>,
        target_node_id: Option<Id>,
        modality: Option<Modality>,
        phase: Option<InvocationPhase>,
        producer_modality: Option<Modality>,
        producer_phase: Option<InvocationPhase>,
        /// Source-derived display, excluded from semantic identity. Declared facets expose this value.
        display: String,
    }
);

table!(
    /// One attributed observation that puts an attribute on an object. For a call the first
    /// site/edge is the call; for a handoff it is the producer and the second is the consumer.
    ConceptIncidences, ConceptIncidencesRow = "concept_incidences",
    family = Findings,
    key = [snapshot_id, incidence_id],
    checks = [],
    {
        snapshot_id: Id,
        incidence_id: Id,
        object_node_id: Id,
        attribute_id: Id,
        source_fact_id: Option<Id>,
        site_node_id: Option<Id>,
        edge_id: Option<Id>,
        other_site_node_id: Option<Id>,
        other_edge_id: Option<Id>,
        consumer_formal_id: Option<Id>,
    }
);

/// A parameter spelling and kind are syntax. Type terms and targets are structural IDs.
/// A raised class/instance normalizes to the provider's typed class identity, never `type[...]`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum AttributeKey {
    Parameter {
        name: String,
        kind: ParameterKind,
    },
    ParameterType(Id),
    Returns(Id),
    Raises {
        module: String,
        key: String,
    },
    Decorator(String),
    Calls {
        target: Id,
        modality: Modality,
        phase: InvocationPhase,
    },
    Handoff {
        takes: bool,
        target: Id,
        consumer_modality: Modality,
        consumer_phase: InvocationPhase,
        producer_modality: Modality,
        producer_phase: InvocationPhase,
    },
}

impl AttributeKey {
    pub fn row(&self, snapshot_id: Id, display: String) -> ConceptAttributesRow {
        let mut row = ConceptAttributesRow {
            snapshot_id,
            attribute_id: Id::ZERO,
            kind: Kind::Parameter,
            symbol: None,
            parameter_kind: None,
            type_term_id: None,
            class_module: None,
            class_key: None,
            target_node_id: None,
            modality: None,
            phase: None,
            producer_modality: None,
            producer_phase: None,
            display,
        };
        match self {
            Self::Parameter { name, kind } => {
                row.symbol = Some(name.clone());
                row.parameter_kind = Some(*kind);
            }
            Self::ParameterType(id) => {
                row.kind = Kind::ParameterType;
                row.type_term_id = Some(*id);
            }
            Self::Returns(id) => {
                row.kind = Kind::Returns;
                row.type_term_id = Some(*id);
            }
            Self::Raises { module, key } => {
                row.kind = Kind::Raises;
                row.class_module = Some(module.clone());
                row.class_key = Some(key.clone());
            }
            Self::Decorator(syntax) => {
                row.kind = Kind::Decorator;
                row.symbol = Some(syntax.clone());
            }
            Self::Calls {
                target,
                modality,
                phase,
            } => {
                row.kind = Kind::Calls;
                row.target_node_id = Some(*target);
                row.modality = Some(*modality);
                row.phase = Some(*phase);
            }
            Self::Handoff {
                takes,
                target,
                consumer_modality,
                consumer_phase,
                producer_modality,
                producer_phase,
            } => {
                row.kind = if *takes {
                    Kind::TakesFrom
                } else {
                    Kind::HandsOff
                };
                row.target_node_id = Some(*target);
                row.modality = Some(*consumer_modality);
                row.phase = Some(*consumer_phase);
                row.producer_modality = Some(*producer_modality);
                row.producer_phase = Some(*producer_phase);
            }
        }
        row.attribute_id = IdHasher::new("concept-attribute")
            .i64(i64::from(row.kind.code()))
            .opt_str(row.symbol.as_deref())
            .opt_i64(row.parameter_kind.map(|k| i64::from(k.code())))
            .opt_id(row.type_term_id)
            .opt_str(row.class_module.as_deref())
            .opt_str(row.class_key.as_deref())
            .opt_id(row.target_node_id)
            .opt_i64(row.modality.map(|m| i64::from(m.code())))
            .opt_i64(row.phase.map(|p| i64::from(p.code())))
            .opt_i64(row.producer_modality.map(|m| i64::from(m.code())))
            .opt_i64(row.producer_phase.map(|p| i64::from(p.code())))
            .finish_id();
        row
    }
}

impl ConceptAttributesRow {
    /// Validate the discriminated row at any persisted boundary, including unused fields.
    pub fn key(&self) -> Option<AttributeKey> {
        let key = match self.kind {
            Kind::Parameter => AttributeKey::Parameter {
                name: self.symbol.clone()?,
                kind: self.parameter_kind?,
            },
            Kind::ParameterType => AttributeKey::ParameterType(self.type_term_id?),
            Kind::Returns => AttributeKey::Returns(self.type_term_id?),
            Kind::Raises => AttributeKey::Raises {
                module: self.class_module.clone()?,
                key: self.class_key.clone()?,
            },
            Kind::Decorator => AttributeKey::Decorator(self.symbol.clone()?),
            Kind::Calls => AttributeKey::Calls {
                target: self.target_node_id?,
                modality: self.modality?,
                phase: self.phase?,
            },
            Kind::HandsOff | Kind::TakesFrom => AttributeKey::Handoff {
                takes: self.kind == Kind::TakesFrom,
                target: self.target_node_id?,
                consumer_modality: self.modality?,
                consumer_phase: self.phase?,
                producer_modality: self.producer_modality?,
                producer_phase: self.producer_phase?,
            },
        };
        (key.row(self.snapshot_id, self.display.clone()) == *self).then_some(key)
    }

    /// Existing declared facet projection, derived by category rather than parsing English.
    pub fn facet(&self) -> Option<(OperationFacet, String)> {
        Some(match self.key()? {
            AttributeKey::Parameter { name, kind } => {
                (OperationFacet::Parameter, parameter_name(&name, kind))
            }
            AttributeKey::ParameterType(_) => (OperationFacet::ParameterType, self.display.clone()),
            AttributeKey::Returns(_) => (OperationFacet::Returns, self.display.clone()),
            AttributeKey::Raises { .. } => (OperationFacet::Raises, self.display.clone()),
            AttributeKey::Decorator(syntax) => (OperationFacet::Decorator, syntax),
            _ => return None,
        })
    }

    /// The single renderer for labels and assertion predicates. Target resolution is observed;
    /// even a definite target does not assert the call always executes.
    pub fn render(&self) -> Option<String> {
        let display = &self.display;
        Some(match self.key()? {
            AttributeKey::Parameter { name, kind } => {
                format!("declares parameter `{}`", parameter_name(&name, kind))
            }
            AttributeKey::ParameterType(_) => format!("declares a parameter typed `{display}`"),
            AttributeKey::Returns(_) => format!("declares return type `{display}`"),
            AttributeKey::Raises { .. } => format!("has a direct raise of `{display}`"),
            AttributeKey::Decorator(syntax) => format!("has decorator `{syntax}`"),
            AttributeKey::Calls {
                modality, phase, ..
            } => format!(
                "has a {} target `{display}` at phase `{}`",
                modality.text(),
                phase.text()
            ),
            AttributeKey::Handoff {
                takes,
                consumer_modality,
                consumer_phase,
                producer_modality,
                producer_phase,
                ..
            } => {
                let relation = if takes {
                    "receiving from"
                } else {
                    "sending to"
                };
                let certainty = if consumer_modality == Modality::Definite
                    && producer_modality == Modality::Definite
                {
                    "resolved"
                } else {
                    "candidate"
                };
                format!(
                    "has an official-usage {certainty} handoff pairing {relation} `{display}` (producer {}/{}, consumer {}/{})",
                    producer_modality.text(),
                    producer_phase.text(),
                    consumer_modality.text(),
                    consumer_phase.text()
                )
            }
        })
    }
}

fn parameter_name(name: &str, kind: ParameterKind) -> String {
    format!(
        "{}{name}",
        match kind {
            ParameterKind::VarPositional => "*",
            ParameterKind::VarKeyword => "**",
            _ => "",
        }
    )
}

impl ConceptIncidencesRow {
    pub fn identity(&self) -> Id {
        IdHasher::new("concept-incidence")
            .id(self.object_node_id)
            .id(self.attribute_id)
            .opt_id(self.source_fact_id)
            .opt_id(self.site_node_id)
            .opt_id(self.edge_id)
            .opt_id(self.other_site_node_id)
            .opt_id(self.other_edge_id)
            .opt_id(self.consumer_formal_id)
            .finish_id()
    }
}

/// Retained handoff evidence obligations, independent of whether incidences exist.
pub fn handoff_support_requirements_sql() -> String {
    use crate::codebook::MemberRole;
    format!(
        "SELECT DISTINCT m.finding_id,m.attribute_id,f.subject_node_id AS object_node_id, \
        p.node_id AS site_node_id,c.node_id AS other_site_node_id,fm.node_id AS consumer_formal_id \
        FROM finding_members m JOIN findings f ON f.finding_id=m.finding_id \
        JOIN finding_members p ON p.finding_id=m.finding_id AND p.role={producer} \
        JOIN finding_members c ON c.finding_id=m.finding_id AND c.role={consumer} AND c.ordinal=p.ordinal+1 \
        JOIN finding_members fm ON fm.finding_id=m.finding_id AND fm.role={formal} \
        WHERE m.role={handoff}",
        producer = MemberRole::ProducerSite.code(),
        consumer = MemberRole::ConsumerSite.code(),
        formal = MemberRole::Formal.code(),
        handoff = MemberRole::HandoffAttribute.code()
    )
}

/// Exact source occurrences supporting a finding. FCA uses its retained supporter objects;
/// Pass C uses its retained producer/consumer pairs and formal. Never expand by attribute alone.
pub fn finding_incidence_keys_sql() -> String {
    use crate::codebook::MemberRole;
    format!(
        "SELECT DISTINCT m.finding_id,i.incidence_id FROM finding_members m \
        JOIN finding_members g ON g.finding_id=m.finding_id AND g.role={extent} \
        JOIN concept_incidences i ON i.attribute_id=m.attribute_id AND i.object_node_id=g.node_id \
        WHERE m.role IN ({intent},{premise},{conclusion}) \
        UNION \
        SELECT DISTINCT r.finding_id,i.incidence_id FROM ({required}) r \
        JOIN concept_incidences i ON i.attribute_id=r.attribute_id AND i.object_node_id=r.object_node_id \
          AND i.site_node_id=r.site_node_id AND i.other_site_node_id=r.other_site_node_id AND i.consumer_formal_id=r.consumer_formal_id",
        extent = MemberRole::ExtentMember.code(),
        intent = MemberRole::IntentAttribute.code(),
        premise = MemberRole::Premise.code(),
        conclusion = MemberRole::Conclusion.code(),
        required = handoff_support_requirements_sql()
    )
}
