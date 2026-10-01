//! Native graph/numerical kernels over typed, admitted Phase 4 inputs (DESIGN §15).

pub mod native_concepts;
pub mod native_neighbours;
pub mod native_ranking;
pub mod native_schedule;
pub use lctx_model::domain::analysis::delegation as native_delegation;

/// Borrowed normalized snapshots preserve their topology and reservation owner.
pub mod program_projection {
    pub use lctx_model::domain::projection::snapshot::{
        MaterializedGraph, SelectedEntities, hydrate,
    };
    pub use lctx_model::domain::projection::{Arc, ArcId, EndpointRole, ProjectionName};
}
