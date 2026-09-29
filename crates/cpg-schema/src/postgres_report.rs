//! Finite PostgreSQL reporting domain. Wire inference never declares these identities.
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use std::{collections::HashMap, sync::Arc};
#[derive(Clone, Copy, Debug)]
pub enum View {
    GenerationRelations,
    OperationOutline,
    Events,
    Publications,
    Imports,
    ProjectionState,
    Profiles,
    Selections,
}
impl View {
    pub fn name(self) -> &'static str {
        match self {
            Self::GenerationRelations => "generation_relations",
            Self::OperationOutline => "operation_outline",
            Self::Events => "captured_events",
            Self::Publications => "captured_publications",
            Self::Imports => "captured_imports",
            Self::ProjectionState => "captured_projection_state",
            Self::Profiles => "captured_profiles",
            Self::Selections => "captured_selections",
        }
    }
    pub const MUTABLE: [Self; 6] = [
        Self::Events,
        Self::Publications,
        Self::Imports,
        Self::ProjectionState,
        Self::Profiles,
        Self::Selections,
    ];
    pub fn immutable(self) -> bool {
        matches!(self, Self::GenerationRelations | Self::OperationOutline)
    }
    pub fn schema(self) -> SchemaRef {
        use DataType::{Boolean as Bool, FixedSizeBinary as Id, Int64 as Int, Utf8 as Text};
        let columns = match self {
            Self::GenerationRelations => vec![
                ("generation_digest", Id(32), false),
                ("snapshot_id", Id(16), false),
                ("library", Text, false),
                ("relation_name", Text, false),
                ("rows", Int, false),
            ],
            Self::OperationOutline => vec![
                ("generation_digest", Id(32), false),
                ("node_id", Id(16), false),
                ("access_path", Text, false),
                ("kind", Text, false),
                ("is_method", Bool, false),
                ("docstring_summary", Text, true),
            ],
            Self::Events => vec![
                ("attempt_id", Id(16), false),
                ("compiler_digest", Id(32), false),
                ("library", Text, true),
                ("started_at_us", Int, true),
                ("event_key", Text, true),
                ("kind", Text, true),
                ("detail", Text, true),
                ("recorded_at_us", Int, true),
            ],
            Self::Publications => vec![
                ("snapshot_id", Id(16), false),
                ("content_digest", Id(32), false),
                ("compiler_digest", Id(32), false),
                ("available", Bool, false),
            ],
            Self::ProjectionState => vec![
                ("generation_digest", Id(32), false),
                ("snapshot_id", Id(16), false),
                ("content_digest", Id(32), false),
                ("compiler_digest", Id(32), false),
                ("library", Text, false),
                ("state", Text, false),
            ],
            Self::Profiles => vec![
                ("generation_digest", Id(32), false),
                ("profile_digest", Id(32), false),
                ("ready", Bool, false),
                ("policy", Text, true),
                ("qualification", Text, false),
            ],
            Self::Selections => vec![
                ("library", Text, false),
                ("generation_digest", Id(32), false),
                ("profile_digest", Id(32), true),
            ],
            Self::Imports => vec![
                ("generation_digest", Id(32), false),
                ("attempt_id", Int, false),
                ("outcome", Text, false),
                ("started_at_us", Int, false),
                ("finished_at_us", Int, true),
                ("failure_code", Text, true),
            ],
        };
        Arc::new(Schema::new_with_metadata(
            columns
                .into_iter()
                .map(|(name, ty, nullable)| {
                    let mut metadata = HashMap::new();
                    if let Id(width) = ty {
                        metadata.insert("lctx.identity_bytes".into(), width.to_string());
                    }
                    if ty == Text {
                        metadata.insert("lctx.collation".into(), "C".into());
                    }
                    if name.ends_with("_us") {
                        metadata.insert("lctx.time".into(), "unix-microseconds-UTC".into());
                    }
                    Field::new(name, ty, nullable).with_metadata(metadata)
                })
                .collect::<Vec<_>>(),
            HashMap::from([("lctx.report".into(), format!("1/{}", self.name()))]),
        ))
    }
}
